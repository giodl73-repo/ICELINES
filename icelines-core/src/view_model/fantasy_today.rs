//! Daily fantasy cockpit composed from existing fantasy decision ViewModels.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::fantasy_assistant::{FantasyMorningActionKind, FantasyMorningBriefingView};
use super::{
    FantasyBenchCoverageView, FantasyCategoryMatchupView, FantasyMatchupStrategyView,
    FantasyProviderStatusView,
};
use crate::season_stats::SeasonType;

pub const FANTASY_TODAY_SCHEMA: &str = "fantasy_today.v1";
pub const FANTASY_TODAY_V2_SCHEMA: &str = "fantasy_today.v2";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyTodayState {
    Ready,
    Provisional,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyTodayFirmness {
    Firm,
    Conditional,
    RefreshRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyTodayContext {
    pub league_id: String,
    pub league_name: String,
    pub fantasy_team_id: String,
    pub fantasy_team_name: String,
    pub stats_season: String,
    pub season_type: SeasonType,
    pub competition_mode: String,
    pub date: NaiveDate,
    pub week_start: NaiveDate,
    pub week_end: NaiveDate,
    pub timezone: String,
    pub generated_at: DateTime<Utc>,
    pub evaluated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyTodayReadinessRow {
    pub workflow: String,
    pub state: FantasyTodayState,
    pub reason_code: Option<String>,
    pub message: String,
    pub recovery_command: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyTodayAction {
    pub id: String,
    pub priority: u8,
    pub kind: FantasyMorningActionKind,
    pub firmness: FantasyTodayFirmness,
    pub player_key: Option<String>,
    pub player: Option<String>,
    pub message: String,
    pub constraint_summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyTodayEvidenceRow {
    pub source_family: String,
    pub authority_scope: String,
    pub state: FantasyTodayState,
    pub observed_at: Option<String>,
    pub fetched_at: Option<String>,
    pub detail: String,
    pub recovery_command: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyTodayMatchupSummary {
    pub competition_mode: String,
    pub opponent: String,
    pub matchup_state: String,
    pub expected_margin: Option<f64>,
    pub downside_margin: Option<f64>,
    pub upside_margin: Option<f64>,
    pub modeled_win_probability: f64,
    pub projected_category_wins: Option<usize>,
    pub projected_category_ties: Option<usize>,
    pub projected_category_losses: Option<usize>,
    pub recommendation: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyTodayQuietNightSummary {
    pub bench_players: usize,
    pub usable_substitute_starts: usize,
    pub quiet_night_starts: usize,
    pub bench_collisions: usize,
    pub projected_substitute_value: f64,
    pub best_substitute: Option<String>,
    pub best_substitute_team: Option<String>,
}

#[derive(Debug, Clone)]
pub enum FantasyTodayMatchupInput {
    Points(Box<FantasyMatchupStrategyView>),
    Categories(Box<FantasyCategoryMatchupView>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyTodayLineupSummary {
    pub active_players: usize,
    pub usable_starts: usize,
    pub bench_players: usize,
    pub bench_players_with_games: usize,
    pub open_active_slots: usize,
    pub ir_moves: usize,
    pub ir_plus_moves: usize,
    pub projected_active_value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyTodayGoalieSummary {
    pub minimum_appearances: u8,
    pub current_appearances: f64,
    pub expected_total_appearances: f64,
    pub confirmed_floor_total_appearances: f64,
    pub minimum_at_risk: bool,
    pub refreshes_due_now: usize,
    pub safety_checks_due_now: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyTodayAcquisitionSummary {
    pub limit: u8,
    pub used: u8,
    pub remaining: u8,
    pub proactive_remaining: u8,
    pub can_add: bool,
    pub can_proactively_add: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyTodayView {
    pub schema: String,
    pub context: FantasyTodayContext,
    pub state: FantasyTodayState,
    pub primary_decision: Option<FantasyTodayAction>,
    pub actions: Vec<FantasyTodayAction>,
    pub alternatives: Vec<FantasyTodayAction>,
    pub matchup: Option<FantasyTodayMatchupSummary>,
    pub lineup: FantasyTodayLineupSummary,
    pub goalies: Option<FantasyTodayGoalieSummary>,
    pub acquisitions: FantasyTodayAcquisitionSummary,
    pub quiet_nights: Option<FantasyTodayQuietNightSummary>,
    pub readiness: Vec<FantasyTodayReadinessRow>,
    pub evidence: Vec<FantasyTodayEvidenceRow>,
    pub next_decision_deadline_utc: Option<DateTime<Utc>>,
    pub material_fingerprint: String,
    pub warnings: Vec<String>,
    pub morning: FantasyMorningBriefingView,
}

#[derive(Debug, Clone)]
pub struct FantasyTodayInput {
    pub context: FantasyTodayContext,
    pub morning: FantasyMorningBriefingView,
    pub matchup: Option<FantasyTodayMatchupInput>,
    pub bench_coverage: Option<FantasyBenchCoverageView>,
    pub provider_status: Option<FantasyProviderStatusView>,
    pub readiness: Vec<FantasyTodayReadinessRow>,
    pub evidence: Vec<FantasyTodayEvidenceRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyTodayV2View {
    #[serde(flatten)]
    pub today: FantasyTodayView,
    pub decisions: super::fantasy_daily_decisions::FantasyDailyDecisionsView,
}

/// Decision-critical fields rendered by every interactive surface.
///
/// Keeping this projection in core prevents CLI, TUI, and Web from deriving
/// different answers from the same sealed `fantasy_today.v2` contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyTodaySurfaceDecision {
    pub primary_message: Option<String>,
    pub alternative_messages: Vec<String>,
    pub deadline_utc: Option<DateTime<Utc>>,
    pub firmness: Option<FantasyTodayFirmness>,
    pub legal_at_evaluation: Option<bool>,
    pub matchup_impact: Option<String>,
    pub evidence_age_seconds: Option<i64>,
    pub material_fingerprint: String,
}

impl FantasyTodaySurfaceDecision {
    pub fn primary_display_message(&self) -> String {
        let message = self
            .primary_message
            .as_deref()
            .unwrap_or("No action recommended.");
        let Some(firmness) = self.firmness else {
            return message.to_owned();
        };
        let firmness = match firmness {
            FantasyTodayFirmness::Firm => "firm",
            FantasyTodayFirmness::Conditional => "conditional",
            FantasyTodayFirmness::RefreshRequired => "refresh_required",
        };
        format!(
            "{} [{}; legal now: {}; evidence age: {}]{}",
            message,
            firmness,
            self.legal_at_evaluation.unwrap_or(false),
            self.evidence_age_seconds
                .map(|seconds| format!("{seconds}s"))
                .unwrap_or_else(|| "not timestamped".to_owned()),
            self.matchup_impact
                .as_ref()
                .map(|impact| format!(" | {impact}"))
                .unwrap_or_default()
        )
    }
}

impl FantasyTodayV2View {
    pub fn v1_projection(&self) -> FantasyTodayView {
        let mut view = self.today.clone();
        view.schema = FANTASY_TODAY_SCHEMA.to_owned();
        view
    }

    pub fn surface_decision(&self) -> FantasyTodaySurfaceDecision {
        let primary = self.decisions.primary_decision.as_ref();
        FantasyTodaySurfaceDecision {
            primary_message: primary.map(|row| row.action.message.clone()),
            alternative_messages: self
                .decisions
                .alternatives
                .iter()
                .map(|row| row.action.message.clone())
                .collect(),
            deadline_utc: primary
                .and_then(|row| row.deadline_utc)
                .or(self.today.next_decision_deadline_utc),
            firmness: primary.map(|row| row.action.firmness),
            legal_at_evaluation: primary.map(|row| row.legal_at_evaluation),
            matchup_impact: primary.and_then(|row| row.matchup_impact.clone()),
            evidence_age_seconds: primary.and_then(|row| row.evidence_age_seconds),
            material_fingerprint: self.decisions.material_fingerprint.clone(),
        }
    }
}

pub fn build_fantasy_today_v2(
    mut today: FantasyTodayView,
    transaction_candidate: Option<super::fantasy_daily_decisions::FantasyDailyTransactionCandidate>,
    candidate_state: FantasyTodayState,
    candidate_recovery_command: Option<String>,
) -> FantasyTodayV2View {
    let decisions = super::fantasy_daily_decisions::build_fantasy_daily_decisions(
        super::fantasy_daily_decisions::FantasyDailyDecisionsInput {
            today: today.clone(),
            transaction_candidate,
            candidate_state,
            candidate_recovery_command,
        },
    );
    today.schema = FANTASY_TODAY_V2_SCHEMA.to_owned();
    FantasyTodayV2View { today, decisions }
}

pub fn build_fantasy_today(input: FantasyTodayInput) -> FantasyTodayView {
    let FantasyTodayInput {
        context,
        morning,
        matchup,
        bench_coverage,
        provider_status,
        mut readiness,
        mut evidence,
    } = input;
    let mut actions = morning
        .actions
        .iter()
        .map(|action| FantasyTodayAction {
            id: stable_action_id(action),
            priority: action.priority,
            kind: action.kind,
            firmness: if matches!(
                action.kind,
                FantasyMorningActionKind::RefreshStatus | FantasyMorningActionKind::RefreshGoalie
            ) {
                FantasyTodayFirmness::RefreshRequired
            } else if action.conditional {
                FantasyTodayFirmness::Conditional
            } else {
                FantasyTodayFirmness::Firm
            },
            player_key: action.player_key.clone(),
            player: action.player.clone(),
            message: action.message.clone(),
            constraint_summary: morning_action_constraint(action.kind),
        })
        .collect::<Vec<_>>();
    actions.sort_by(|a, b| a.priority.cmp(&b.priority).then_with(|| a.id.cmp(&b.id)));
    let primary_decision = actions.first().cloned();
    let alternatives = actions.iter().skip(1).cloned().collect::<Vec<_>>();

    if matchup.is_none() {
        readiness.push(FantasyTodayReadinessRow {
            workflow: "matchup".to_owned(),
            state: FantasyTodayState::Provisional,
            reason_code: Some("matchup_unavailable".to_owned()),
            message: "No matchup strategy was supplied; no margin is inferred".to_owned(),
            recovery_command: Some("icelines fantasy matchup-plan".to_owned()),
        });
    }
    if bench_coverage.is_none() {
        readiness.push(FantasyTodayReadinessRow {
            workflow: "quiet_nights".to_owned(),
            state: FantasyTodayState::Provisional,
            reason_code: Some("bench_coverage_unavailable".to_owned()),
            message: "Quiet-night substitution coverage was not supplied".to_owned(),
            recovery_command: Some("icelines fantasy bench-coverage".to_owned()),
        });
    }
    if let Some(provider) = &provider_status {
        for capability in &provider.capabilities {
            use super::FantasyProviderCapabilityState as State;
            let state = match capability.state {
                State::Fresh => FantasyTodayState::Ready,
                State::Blocked | State::Failed | State::Inconsistent => FantasyTodayState::Blocked,
                State::Unknown | State::Stale | State::Partial => FantasyTodayState::Provisional,
            };
            readiness.push(FantasyTodayReadinessRow {
                workflow: format!("provider.{:?}", capability.capability).to_ascii_lowercase(),
                state,
                reason_code: (state != FantasyTodayState::Ready)
                    .then(|| format!("provider_{:?}", capability.state).to_ascii_lowercase()),
                message: capability.detail.clone(),
                recovery_command: capability.recovery_command.clone(),
            });
        }
    }
    readiness.sort_by(|a, b| a.workflow.cmp(&b.workflow));
    readiness.dedup_by(|a, b| a.workflow == b.workflow && a.reason_code == b.reason_code);

    let state = if readiness
        .iter()
        .any(|row| row.state == FantasyTodayState::Blocked)
    {
        FantasyTodayState::Blocked
    } else if readiness
        .iter()
        .any(|row| row.state == FantasyTodayState::Provisional)
        || primary_decision
            .as_ref()
            .is_some_and(|action| action.firmness != FantasyTodayFirmness::Firm)
    {
        FantasyTodayState::Provisional
    } else {
        FantasyTodayState::Ready
    };

    let daily = &morning.injury_plan.lineup;
    let lineup = FantasyTodayLineupSummary {
        active_players: daily.active.len(),
        usable_starts: daily.usable_starts,
        bench_players: daily.bench_assignments.len().max(daily.bench.len()),
        bench_players_with_games: daily
            .bench_assignments
            .iter()
            .filter(|row| row.has_game)
            .count(),
        open_active_slots: daily.missing_active_slots.len(),
        ir_moves: daily.injured_reserve.len(),
        ir_plus_moves: daily.injured_reserve_plus.len(),
        projected_active_value: daily.projected_active_value,
    };
    let goalies = morning
        .goalie_plan
        .as_ref()
        .map(|plan| FantasyTodayGoalieSummary {
            minimum_appearances: plan.minimum_goalie_appearances,
            current_appearances: plan.current_goalie_appearances,
            expected_total_appearances: plan.expected_total_appearances,
            confirmed_floor_total_appearances: plan.confirmed_floor_total_appearances,
            minimum_at_risk: plan.minimum_at_risk,
            refreshes_due_now: plan.refreshes_due_now,
            safety_checks_due_now: plan.safety_checks_due_now,
        });
    let budget = &morning.budget;
    let acquisitions = FantasyTodayAcquisitionSummary {
        limit: budget.acquisition_limit,
        used: budget.acquisitions_used,
        remaining: budget.acquisitions_remaining,
        proactive_remaining: budget.proactive_acquisitions_remaining,
        can_add: budget.can_add,
        can_proactively_add: budget.can_proactively_add,
    };
    let next_decision_deadline_utc = [
        morning.next_goalie_refresh_utc,
        morning.next_goalie_safety_check_utc,
        morning.next_goalie_lock_utc,
    ]
    .into_iter()
    .flatten()
    .filter(|instant| *instant >= context.evaluated_at)
    .min();

    let matchup_summary = matchup.as_ref().map(project_matchup_summary);
    let quiet_nights = bench_coverage.as_ref().map(project_quiet_nights);
    let mut warnings = morning.warnings.clone();
    match &matchup {
        Some(FantasyTodayMatchupInput::Points(view)) => warnings.extend(view.warnings.clone()),
        Some(FantasyTodayMatchupInput::Categories(view)) => warnings.extend(view.warnings.clone()),
        None => {}
    }
    if let Some(provider) = &provider_status {
        warnings.extend(provider.warnings.clone());
    }
    warnings.sort();
    warnings.dedup();
    evidence.sort_by(|a, b| {
        a.source_family
            .cmp(&b.source_family)
            .then_with(|| a.authority_scope.cmp(&b.authority_scope))
    });
    evidence.dedup_by(|a, b| {
        a.source_family == b.source_family && a.authority_scope == b.authority_scope
    });

    let material = serde_json::to_vec(&(
        (
            &context.league_id,
            &context.fantasy_team_id,
            &context.stats_season,
            context.season_type,
            &context.competition_mode,
            context.date,
            context.week_start,
            context.week_end,
            context.evaluated_at,
        ),
        (
            state,
            &primary_decision,
            &alternatives,
            &matchup_summary,
            &quiet_nights,
            &readiness,
            &evidence,
            next_decision_deadline_utc,
            &morning.material_fingerprint,
        ),
    ))
    .expect("fantasy today decision fields are serializable");
    let material_fingerprint = format!("{:x}", Sha256::digest(material));

    FantasyTodayView {
        schema: FANTASY_TODAY_SCHEMA.to_owned(),
        context,
        state,
        primary_decision,
        actions,
        alternatives,
        matchup: matchup_summary,
        lineup,
        goalies,
        acquisitions,
        quiet_nights,
        readiness,
        evidence,
        next_decision_deadline_utc,
        material_fingerprint,
        warnings,
        morning,
    }
}

fn stable_action_id(action: &super::FantasyMorningAction) -> String {
    let material = serde_json::to_vec(&(
        action.priority,
        action.kind,
        &action.player_key,
        &action.message,
        action.conditional,
    ))
    .expect("fantasy morning action is serializable");
    let digest = format!("{:x}", Sha256::digest(material));
    format!("today-{:02}-{}", action.priority, &digest[..12])
}

fn morning_action_constraint(kind: FantasyMorningActionKind) -> String {
    match kind {
        FantasyMorningActionKind::MoveToIr | FantasyMorningActionKind::MoveToIrPlus => {
            "Reserve-slot eligibility and capacity must remain valid".to_owned()
        }
        FantasyMorningActionKind::RefreshStatus | FantasyMorningActionKind::RefreshGoalie => {
            "Refresh evidence before the next applicable lineup lock".to_owned()
        }
        FantasyMorningActionKind::Start
        | FantasyMorningActionKind::StartGoalie
        | FantasyMorningActionKind::BenchGoalie
        | FantasyMorningActionKind::GoalieLocked => {
            "Saved eligibility, slot capacity, and game lock govern execution".to_owned()
        }
        FantasyMorningActionKind::GoalieStreamReview
        | FantasyMorningActionKind::GoalieFallback
        | FantasyMorningActionKind::PickupReview => {
            "Verify availability, waiver timing, roster legality, and acquisition budget".to_owned()
        }
        FantasyMorningActionKind::SleeperWatch => {
            "Watch-only evidence; no transaction is implied".to_owned()
        }
    }
}

fn project_matchup_summary(input: &FantasyTodayMatchupInput) -> FantasyTodayMatchupSummary {
    match input {
        FantasyTodayMatchupInput::Points(view) => FantasyTodayMatchupSummary {
            competition_mode: view.competition_mode.clone(),
            opponent: view.opponent.team.clone(),
            matchup_state: view.matchup_state.clone(),
            expected_margin: Some(view.expected_margin),
            downside_margin: Some(view.downside_margin),
            upside_margin: Some(view.upside_margin),
            modeled_win_probability: view.modeled_win_probability,
            projected_category_wins: None,
            projected_category_ties: None,
            projected_category_losses: None,
            recommendation: view.recommendation.clone(),
        },
        FantasyTodayMatchupInput::Categories(view) => FantasyTodayMatchupSummary {
            competition_mode: view.competition_mode.clone(),
            opponent: view.opponent_team.clone(),
            matchup_state: view.matchup_state.clone(),
            expected_margin: None,
            downside_margin: None,
            upside_margin: None,
            modeled_win_probability: view.modeled_matchup_win_probability,
            projected_category_wins: Some(view.projected_category_wins),
            projected_category_ties: Some(view.projected_category_ties),
            projected_category_losses: Some(view.projected_category_losses),
            recommendation: view.recommendation.clone(),
        },
    }
}

fn project_quiet_nights(view: &FantasyBenchCoverageView) -> FantasyTodayQuietNightSummary {
    FantasyTodayQuietNightSummary {
        bench_players: view.rows.len(),
        usable_substitute_starts: view
            .rows
            .iter()
            .map(|row| row.usable_substitute_starts)
            .sum(),
        quiet_night_starts: view.rows.iter().map(|row| row.quiet_night_starts).sum(),
        bench_collisions: view.rows.iter().map(|row| row.bench_collisions).sum(),
        projected_substitute_value: view
            .rows
            .iter()
            .map(|row| row.projected_substitute_value)
            .sum(),
        best_substitute: view.rows.first().map(|row| row.player.clone()),
        best_substitute_team: view.rows.first().map(|row| row.nhl_team.clone()),
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::view_model::fantasy_assistant::{
        FantasyAssistantRules, FantasyDailyLineupView, FantasyInjuryPlanView, FantasyMorningAction,
        FantasyWeekBudgetView, FANTASY_DAILY_LINEUP_SCHEMA, FANTASY_INJURY_PLAN_SCHEMA,
        FANTASY_MORNING_BRIEFING_SCHEMA, FANTASY_WEEK_BUDGET_SCHEMA,
    };
    use crate::view_model::fantasy_matchup_strategy::FANTASY_MATCHUP_STRATEGY_SCHEMA;
    use crate::{
        FantasyBenchCoveragePlayerRow, FantasyMatchupStrategy, FantasyMatchupTeamProjection,
        FANTASY_BENCH_COVERAGE_SCHEMA, FANTASY_CATEGORY_MATCHUP_SCHEMA,
    };

    fn morning() -> FantasyMorningBriefingView {
        let date = NaiveDate::from_ymd_opt(2026, 9, 8).unwrap();
        let instant = Utc.with_ymd_and_hms(2026, 9, 8, 14, 0, 0).unwrap();
        let lineup = FantasyDailyLineupView {
            schema: FANTASY_DAILY_LINEUP_SCHEMA.to_owned(),
            rules: FantasyAssistantRules::configured_2026(),
            active: Vec::new(),
            bench: vec!["Bench Player".to_owned()],
            bench_assignments: Vec::new(),
            injured_reserve: Vec::new(),
            injured_reserve_plus: Vec::new(),
            overflow: Vec::new(),
            missing_active_slots: Vec::new(),
            projected_active_value: 12.5,
            usable_starts: 3,
            warnings: Vec::new(),
        };
        FantasyMorningBriefingView {
            schema: FANTASY_MORNING_BRIEFING_SCHEMA.to_owned(),
            date,
            generated_at: instant,
            evaluated_at: instant,
            timezone: "America/Los_Angeles".to_owned(),
            injury_plan: FantasyInjuryPlanView {
                schema: FANTASY_INJURY_PLAN_SCHEMA.to_owned(),
                date,
                lineup,
                statuses: Vec::new(),
                warnings: Vec::new(),
            },
            goalie_plan: None,
            budget: FantasyWeekBudgetView {
                schema: FANTASY_WEEK_BUDGET_SCHEMA.to_owned(),
                timezone: "America/Los_Angeles".to_owned(),
                week_start: NaiveDate::from_ymd_opt(2026, 9, 7).unwrap(),
                week_end: NaiveDate::from_ymd_opt(2026, 9, 13).unwrap(),
                acquisition_limit: 4,
                acquisitions_used: 1,
                acquisitions_remaining: 3,
                can_add: true,
                injury_reserve: 1,
                injury_reserve_active: 1,
                proactive_acquisitions_remaining: 2,
                can_proactively_add: true,
                injury_reserve_releases_on: None,
            },
            pickup_plan: None,
            sleeper_plan: None,
            next_goalie_refresh_utc: None,
            next_goalie_safety_check_utc: None,
            next_goalie_lock_utc: None,
            goalie_refreshes_due_now: 0,
            goalie_safety_checks_due_now: 0,
            actions: vec![FantasyMorningAction {
                priority: 30,
                kind: FantasyMorningActionKind::RefreshStatus,
                player_key: Some("8470001".to_owned()),
                player: Some("Test Player".to_owned()),
                message: "Refresh Test Player before lock".to_owned(),
                conditional: true,
            }],
            material_fingerprint: "morning-fixture".to_owned(),
            suppressed_unchanged: false,
            warnings: vec!["Briefing is advisory".to_owned()],
        }
    }

    fn input() -> FantasyTodayInput {
        let morning = morning();
        FantasyTodayInput {
            context: FantasyTodayContext {
                league_id: "league-1".to_owned(),
                league_name: "Fixture League".to_owned(),
                fantasy_team_id: "team-1".to_owned(),
                fantasy_team_name: "Fixture Team".to_owned(),
                stats_season: "20262027".to_owned(),
                season_type: SeasonType::Regular,
                competition_mode: "points".to_owned(),
                date: morning.date,
                week_start: morning.budget.week_start,
                week_end: morning.budget.week_end,
                timezone: morning.timezone.clone(),
                generated_at: morning.generated_at,
                evaluated_at: morning.evaluated_at,
            },
            morning,
            matchup: None,
            bench_coverage: None,
            provider_status: None,
            readiness: Vec::new(),
            evidence: Vec::new(),
        }
    }

    #[test]
    fn refresh_action_makes_today_provisional_without_inventing_missing_values() {
        let view = build_fantasy_today(input());

        assert_eq!(view.schema, FANTASY_TODAY_SCHEMA);
        assert_eq!(view.state, FantasyTodayState::Provisional);
        assert_eq!(
            view.primary_decision.as_ref().map(|row| row.firmness),
            Some(FantasyTodayFirmness::RefreshRequired)
        );
        assert!(view.goalies.is_none());
        assert_eq!(view.lineup.bench_players, 1);
        assert_eq!(view.lineup.bench_players_with_games, 0);
    }

    #[test]
    fn decision_fingerprint_is_stable_when_only_generation_time_changes() {
        let first = build_fantasy_today(input());
        let mut changed = input();
        changed.context.generated_at += chrono::Duration::minutes(5);
        changed.morning.generated_at += chrono::Duration::minutes(5);
        let second = build_fantasy_today(changed);

        assert_eq!(first.material_fingerprint, second.material_fingerprint);
    }

    #[test]
    fn blocked_readiness_overrides_action_firmness() {
        let mut input = input();
        input.readiness.push(FantasyTodayReadinessRow {
            workflow: "roster".to_owned(),
            state: FantasyTodayState::Blocked,
            reason_code: Some("illegal_roster".to_owned()),
            message: "Roster legality is unknown".to_owned(),
            recovery_command: Some("icelines fantasy roster-shape validate".to_owned()),
        });

        assert_eq!(build_fantasy_today(input).state, FantasyTodayState::Blocked);
    }

    #[test]
    fn points_matchup_and_quiet_nights_are_distilled_without_recalculation() {
        let mut input = input();
        let date = input.context.date;
        let team = |name: &str, projected: f64| FantasyMatchupTeamProjection {
            team: name.to_owned(),
            current_points: 0.0,
            remaining_projected_points: projected,
            projected_points: projected,
            floor_points: projected - 5.0,
            upside_points: projected + 5.0,
            usable_starts: 8,
            scheduled_player_games: 9,
            benched_player_games: 1,
            bench_collision_value: 2.0,
            daily: Vec::new(),
        };
        input.matchup = Some(FantasyTodayMatchupInput::Points(Box::new(
            FantasyMatchupStrategyView {
                schema: FANTASY_MATCHUP_STRATEGY_SCHEMA.to_owned(),
                competition_mode: "points".to_owned(),
                league: "Fixture League".to_owned(),
                scoring_scheme: "fixture".to_owned(),
                week_start: input.context.week_start,
                week_end: input.context.week_end,
                matchup_state: "pre_week".to_owned(),
                current_through_date: None,
                current_totals_source: None,
                strategy: FantasyMatchupStrategy::Balanced,
                user: team("Fixture Team", 100.0),
                opponent: team("Opponent", 94.0),
                expected_margin: 6.0,
                downside_margin: -4.0,
                upside_margin: 16.0,
                modeled_win_probability: 0.64,
                largest_legal_swing: None,
                recommendation: "Protect the floor".to_owned(),
                model_notes: Vec::new(),
                warnings: vec!["shared warning".to_owned()],
            },
        )));
        input.bench_coverage = Some(FantasyBenchCoverageView {
            schema: FANTASY_BENCH_COVERAGE_SCHEMA.to_owned(),
            fantasy_team: "Fixture Team".to_owned(),
            start: date,
            end: date,
            off_night_max_games: 4,
            baseline_starters: 10,
            rows: vec![FantasyBenchCoveragePlayerRow {
                player_key: "bench-player".to_owned(),
                player: "Bench Player".to_owned(),
                nhl_team: "WSH".to_owned(),
                positions: Vec::new(),
                baseline_bench_slot: "BN1".to_owned(),
                scheduled_games: 3,
                usable_substitute_starts: 2,
                quiet_night_starts: 2,
                bench_collisions: 1,
                projected_substitute_value: 8.5,
                covers: Vec::new(),
            }],
            uncovered_starter_dates: Default::default(),
            disclosures: Vec::new(),
        });

        let view = build_fantasy_today(input);
        assert_eq!(
            view.matchup.as_ref().and_then(|row| row.expected_margin),
            Some(6.0)
        );
        assert_eq!(
            view.quiet_nights
                .as_ref()
                .map(|row| (row.best_substitute.as_deref(), row.usable_substitute_starts)),
            Some((Some("Bench Player"), 2))
        );
        assert!(!view
            .readiness
            .iter()
            .any(|row| row.reason_code.as_deref() == Some("matchup_unavailable")));
    }

    #[test]
    fn category_matchup_never_serializes_a_points_margin() {
        let mut input = input();
        input.matchup = Some(FantasyTodayMatchupInput::Categories(Box::new(
            FantasyCategoryMatchupView {
                schema: FANTASY_CATEGORY_MATCHUP_SCHEMA.to_owned(),
                competition_mode: "categories".to_owned(),
                league: "Fixture League".to_owned(),
                week_start: input.context.week_start,
                week_end: input.context.week_end,
                matchup_state: "pre_week".to_owned(),
                current_through_date: None,
                current_totals_source: None,
                strategy: FantasyMatchupStrategy::Balanced,
                user_team: "Fixture Team".to_owned(),
                opponent_team: "Opponent".to_owned(),
                user_goalie_appearances: 3.0,
                opponent_goalie_appearances: 2.0,
                user_current_goalie_appearances: 0.0,
                opponent_current_goalie_appearances: 0.0,
                user_remaining_goalie_appearances: 3.0,
                opponent_remaining_goalie_appearances: 2.0,
                minimum_goalie_appearances: 2,
                user_meets_goalie_minimum: true,
                opponent_meets_goalie_minimum: true,
                projected_category_wins: 6,
                projected_category_ties: 1,
                projected_category_losses: 4,
                projected_matchup_result: crate::FantasyCategoryProjectedResult::Win,
                modeled_matchup_win_probability: 0.71,
                categories: Vec::new(),
                recommendation: "Protect six categories".to_owned(),
                model_notes: Vec::new(),
                warnings: Vec::new(),
            },
        )));

        let matchup = build_fantasy_today(input).matchup.unwrap();
        assert_eq!(matchup.competition_mode, "categories");
        assert_eq!(matchup.expected_margin, None);
        assert_eq!(matchup.projected_category_wins, Some(6));
    }

    #[test]
    fn next_deadline_is_the_earliest_future_goalie_checkpoint() {
        let mut input = input();
        let later = input.context.evaluated_at + chrono::Duration::hours(4);
        let earlier = input.context.evaluated_at + chrono::Duration::hours(2);
        input.morning.next_goalie_lock_utc = Some(later);
        input.morning.next_goalie_refresh_utc = Some(earlier);
        assert_eq!(
            build_fantasy_today(input).next_decision_deadline_utc,
            Some(earlier)
        );
    }

    #[test]
    fn action_order_and_fingerprint_are_stable_for_equivalent_inputs() {
        let mut first = input();
        first.morning.actions.push(FantasyMorningAction {
            priority: 10,
            kind: FantasyMorningActionKind::Start,
            player_key: Some("8470002".to_owned()),
            player: Some("Earlier Player".to_owned()),
            message: "Start Earlier Player".to_owned(),
            conditional: false,
        });
        let mut reversed = first.clone();
        reversed.morning.actions.reverse();

        let first = build_fantasy_today(first);
        let reversed = build_fantasy_today(reversed);
        assert_eq!(first.actions, reversed.actions);
        assert_eq!(first.material_fingerprint, reversed.material_fingerprint);
        assert_eq!(
            first.primary_decision.as_ref().map(|row| row.priority),
            Some(10)
        );
    }

    #[test]
    fn decision_projection_matches_the_versioned_json_golden() {
        let view = build_fantasy_today(input());
        let projection = serde_json::json!({
            "schema": view.schema,
            "state": view.state,
            "primary_decision": view.primary_decision,
            "matchup": view.matchup,
            "quiet_nights": view.quiet_nights,
            "next_decision_deadline_utc": view.next_decision_deadline_utc,
            "readiness": view.readiness,
        });
        let actual = serde_json::to_value(&projection).unwrap();
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/fantasy_today_decision.golden.json"
        ))
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn v2_serializes_new_decisions_and_projects_exact_v1_schema() {
        let v1 = build_fantasy_today(input());
        let v2 = build_fantasy_today_v2(v1.clone(), None, FantasyTodayState::Ready, None);
        let json = serde_json::to_value(&v2).unwrap();

        assert_eq!(json["schema"], FANTASY_TODAY_V2_SCHEMA);
        assert_eq!(
            json["decisions"]["schema"],
            super::super::fantasy_daily_decisions::FANTASY_DAILY_DECISIONS_SCHEMA
        );
        assert_eq!(v2.v1_projection(), v1);
    }

    #[test]
    fn v2_decision_projection_matches_its_independent_golden() {
        let view = build_fantasy_today_v2(
            build_fantasy_today(input()),
            None,
            FantasyTodayState::Ready,
            None,
        );
        let primary = view.decisions.primary_decision.as_ref().unwrap();
        let actual = serde_json::json!({
            "schema": view.today.schema,
            "decision_schema": view.decisions.schema,
            "decision_state": view.decisions.state,
            "primary_kind": primary.action.kind,
            "primary_firmness": primary.action.firmness,
            "primary_legal_at_evaluation": primary.legal_at_evaluation,
            "candidate_state": view.decisions.candidate_state,
            "transaction_candidate": view.decisions.transaction_candidate,
        });
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/fantasy_today_v2_decision.golden.json"
        ))
        .unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn v2_decision_fingerprint_is_deterministic() {
        let first = build_fantasy_today_v2(
            build_fantasy_today(input()),
            None,
            FantasyTodayState::Ready,
            None,
        );
        let second = build_fantasy_today_v2(
            build_fantasy_today(input()),
            None,
            FantasyTodayState::Ready,
            None,
        );

        assert_eq!(
            first.decisions.material_fingerprint,
            second.decisions.material_fingerprint
        );
        assert_eq!(first.surface_decision(), second.surface_decision());
        assert_eq!(
            first.surface_decision().material_fingerprint,
            first.decisions.material_fingerprint
        );
    }

    #[test]
    fn sealed_surface_projection_preserves_cross_surface_decision_fields() {
        let fixture: FantasyTodaySurfaceDecision = serde_json::from_str(include_str!(
            "../../tests/fixtures/fantasy_today_surface_decision.v1.json"
        ))
        .unwrap();

        assert_eq!(
            fixture.primary_display_message(),
            "Start Fixture Player [firm; legal now: true; evidence age: 90s] | projected +4.5 points against Fixture Rival"
        );
        assert_eq!(fixture.alternative_messages.len(), 2);
        assert_eq!(
            fixture.deadline_utc.unwrap().to_rfc3339(),
            "2026-09-08T23:00:00+00:00"
        );
        assert_eq!(fixture.material_fingerprint.len(), 64);
    }

    #[test]
    fn v2_keeps_refresh_primary_and_discloses_bounded_transaction() {
        let candidate = super::super::fantasy_daily_decisions::FantasyDailyTransactionCandidate {
            add_player_key: "add".to_owned(),
            add_player: "Add Player".to_owned(),
            drop_player_key: "drop".to_owned(),
            drop_player: "Drop Player".to_owned(),
            modeled_value_delta: 4.5,
            incremental_usable_starts: 1.0,
            legal_at_evaluation: true,
            waiver_clears_at: None,
            acquisition_cost: 1,
            acquisitions_remaining_before: 4,
            candidates_considered: 12,
            candidate_limit: 12,
            truncated: true,
            elapsed_ms: 8,
            evidence_observed_at: None,
            reasons: vec!["quiet-night fit".to_owned()],
        };
        let view = build_fantasy_today_v2(
            build_fantasy_today(input()),
            Some(candidate),
            FantasyTodayState::Ready,
            None,
        );

        assert_eq!(
            view.decisions
                .primary_decision
                .as_ref()
                .map(|row| row.action.kind),
            Some(FantasyMorningActionKind::RefreshStatus)
        );
        assert!(view.decisions.alternatives.iter().any(|row| {
            row.action.kind == FantasyMorningActionKind::PickupReview && row.legal_at_evaluation
        }));
        assert_eq!(
            view.decisions
                .transaction_candidate
                .as_ref()
                .map(|row| row.elapsed_ms),
            Some(8)
        );
    }
}
