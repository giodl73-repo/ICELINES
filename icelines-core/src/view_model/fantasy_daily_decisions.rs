//! Pure league-aware prioritization for the daily fantasy cockpit.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::fantasy_assistant::FantasyMorningActionKind;
use super::fantasy_today::{
    FantasyTodayAction, FantasyTodayFirmness, FantasyTodayState, FantasyTodayView,
};

pub const FANTASY_DAILY_DECISIONS_SCHEMA: &str = "fantasy_daily_decisions.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyDailyTransactionCandidate {
    pub add_player_key: String,
    pub add_player: String,
    pub drop_player_key: String,
    pub drop_player: String,
    pub modeled_value_delta: f64,
    pub incremental_usable_starts: f64,
    pub legal_at_evaluation: bool,
    pub waiver_clears_at: Option<DateTime<Utc>>,
    pub acquisition_cost: u8,
    pub acquisitions_remaining_before: u8,
    pub candidates_considered: usize,
    pub candidate_limit: usize,
    pub truncated: bool,
    pub elapsed_ms: u64,
    pub evidence_observed_at: Option<DateTime<Utc>>,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyDailyDecisionRow {
    pub action: FantasyTodayAction,
    pub legal_at_evaluation: bool,
    pub matchup_impact: Option<String>,
    pub deadline_utc: Option<DateTime<Utc>>,
    pub evidence_age_seconds: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyDailyDecisionsView {
    pub schema: String,
    pub state: FantasyTodayState,
    pub primary_decision: Option<FantasyDailyDecisionRow>,
    pub alternatives: Vec<FantasyDailyDecisionRow>,
    pub transaction_candidate: Option<FantasyDailyTransactionCandidate>,
    pub candidate_state: FantasyTodayState,
    pub candidate_recovery_command: Option<String>,
    pub material_fingerprint: String,
}

#[derive(Debug, Clone)]
pub struct FantasyDailyDecisionsInput {
    pub today: FantasyTodayView,
    pub transaction_candidate: Option<FantasyDailyTransactionCandidate>,
    pub candidate_state: FantasyTodayState,
    pub candidate_recovery_command: Option<String>,
}

pub fn build_fantasy_daily_decisions(
    input: FantasyDailyDecisionsInput,
) -> FantasyDailyDecisionsView {
    let matchup_impact = input.today.matchup.as_ref().map(|matchup| {
        format!(
            "{} against {}: {}",
            matchup.matchup_state, matchup.opponent, matchup.recommendation
        )
    });
    let deadline = input.today.next_decision_deadline_utc;
    let mut rows = input
        .today
        .actions
        .iter()
        .cloned()
        .map(|action| FantasyDailyDecisionRow {
            legal_at_evaluation: action.firmness == FantasyTodayFirmness::Firm,
            matchup_impact: matchup_impact.clone(),
            deadline_utc: deadline,
            evidence_age_seconds: None,
            action,
        })
        .collect::<Vec<_>>();

    if let Some(candidate) = &input.transaction_candidate {
        let evidence_age_seconds = candidate.evidence_observed_at.map(|observed| {
            (input.today.context.evaluated_at - observed)
                .num_seconds()
                .max(0)
        });
        let action = FantasyTodayAction {
            id: transaction_action_id(candidate),
            priority: 50,
            kind: FantasyMorningActionKind::PickupReview,
            firmness: if candidate.legal_at_evaluation {
                FantasyTodayFirmness::Firm
            } else {
                FantasyTodayFirmness::Conditional
            },
            player_key: Some(candidate.add_player_key.clone()),
            player: Some(candidate.add_player.clone()),
            message: format!(
                "Add {} for {} ({:+.1} modeled value; {:+.1} usable starts)",
                candidate.add_player,
                candidate.drop_player,
                candidate.modeled_value_delta,
                candidate.incremental_usable_starts
            ),
            constraint_summary:
                "Verify waiver timing, roster fit, game lock, and weekly acquisition budget"
                    .to_owned(),
        };
        rows.push(FantasyDailyDecisionRow {
            action,
            legal_at_evaluation: candidate.legal_at_evaluation,
            matchup_impact: matchup_impact.clone(),
            deadline_utc: candidate.waiver_clears_at.or(deadline),
            evidence_age_seconds,
        });
    }

    rows.sort_by(|a, b| {
        decision_precedence(&a.action)
            .cmp(&decision_precedence(&b.action))
            .then_with(|| b.legal_at_evaluation.cmp(&a.legal_at_evaluation))
            .then_with(|| a.action.priority.cmp(&b.action.priority))
            .then_with(|| a.action.id.cmp(&b.action.id))
    });
    let primary_index = rows.iter().position(|row| {
        row.legal_at_evaluation
            || matches!(row.action.firmness, FantasyTodayFirmness::RefreshRequired)
    });
    let primary_decision = primary_index.map(|index| rows.remove(index));
    let state = if input.today.state == FantasyTodayState::Blocked {
        FantasyTodayState::Blocked
    } else if input.today.state == FantasyTodayState::Provisional
        || input.candidate_state != FantasyTodayState::Ready
        || primary_decision
            .as_ref()
            .is_some_and(|row| row.action.firmness != FantasyTodayFirmness::Firm)
    {
        FantasyTodayState::Provisional
    } else {
        FantasyTodayState::Ready
    };

    let material = serde_json::to_vec(&(
        state,
        &primary_decision,
        &rows,
        &input.transaction_candidate,
        input.candidate_state,
        &input.candidate_recovery_command,
        &input.today.material_fingerprint,
    ))
    .expect("daily fantasy decision fields are serializable");

    FantasyDailyDecisionsView {
        schema: FANTASY_DAILY_DECISIONS_SCHEMA.to_owned(),
        state,
        primary_decision,
        alternatives: rows,
        transaction_candidate: input.transaction_candidate,
        candidate_state: input.candidate_state,
        candidate_recovery_command: input.candidate_recovery_command,
        material_fingerprint: format!("{:x}", Sha256::digest(material)),
    }
}

fn decision_precedence(action: &FantasyTodayAction) -> u8 {
    match action.kind {
        FantasyMorningActionKind::RefreshStatus | FantasyMorningActionKind::RefreshGoalie => 0,
        FantasyMorningActionKind::MoveToIr | FantasyMorningActionKind::MoveToIrPlus => 10,
        FantasyMorningActionKind::StartGoalie
        | FantasyMorningActionKind::BenchGoalie
        | FantasyMorningActionKind::GoalieLocked
        | FantasyMorningActionKind::GoalieFallback
        | FantasyMorningActionKind::GoalieStreamReview => 20,
        FantasyMorningActionKind::Start => 30,
        FantasyMorningActionKind::PickupReview => 50,
        FantasyMorningActionKind::SleeperWatch => 60,
    }
}

fn transaction_action_id(candidate: &FantasyDailyTransactionCandidate) -> String {
    let material = serde_json::to_vec(&(
        &candidate.add_player_key,
        &candidate.drop_player_key,
        candidate.modeled_value_delta,
        candidate.incremental_usable_starts,
        candidate.legal_at_evaluation,
        candidate.waiver_clears_at,
    ))
    .expect("transaction candidate is serializable");
    let digest = format!("{:x}", Sha256::digest(material));
    format!("today-transaction-{}", &digest[..12])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decision_precedence_keeps_refresh_ahead_of_transactions() {
        let refresh = FantasyTodayAction {
            id: "refresh".to_owned(),
            priority: 30,
            kind: FantasyMorningActionKind::RefreshStatus,
            firmness: FantasyTodayFirmness::RefreshRequired,
            player_key: None,
            player: None,
            message: "refresh".to_owned(),
            constraint_summary: String::new(),
        };
        let pickup = FantasyTodayAction {
            kind: FantasyMorningActionKind::PickupReview,
            ..refresh.clone()
        };
        assert!(decision_precedence(&refresh) < decision_precedence(&pickup));
    }
}
