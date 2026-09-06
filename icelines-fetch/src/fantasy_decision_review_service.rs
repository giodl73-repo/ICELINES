//! Read-only assembly for the shared fantasy decision-review projection.

use std::collections::BTreeMap;

use anyhow::Context;
use chrono::{DateTime, NaiveDate, Utc};
use icelines_core::{
    build_fantasy_decision_review, FantasyDecisionOutcome, FantasyDecisionReviewBuildInput,
    FantasyDecisionReviewView, FantasyDecisionStoredInput, FantasyDecisionStoredOutcomeInput,
    FANTASY_DECISION_OUTCOME_SCHEMA,
};

use crate::fantasy_db::{FantasyDb, LeagueRow};

pub fn assemble_fantasy_decision_review(
    db: &FantasyDb,
    league: &LeagueRow,
    limit: usize,
    week: Option<NaiveDate>,
    season: Option<String>,
    include_private: bool,
) -> anyhow::Result<FantasyDecisionReviewView> {
    let decisions = db.list_decisions(&league.id, limit, include_private)?;
    let mut outcomes_by_decision = BTreeMap::<String, Vec<_>>::new();
    for row in db.list_decision_outcomes_for_league(&league.id, include_private)? {
        if row.outcome_kind != FANTASY_DECISION_OUTCOME_SCHEMA {
            continue;
        }
        let outcome: FantasyDecisionOutcome = serde_json::from_str(&row.outcome_json)
            .with_context(|| format!("decode typed outcome '{}'", row.id))?;
        outcomes_by_decision
            .entry(row.decision_id.clone())
            .or_default()
            .push(FantasyDecisionStoredOutcomeInput {
                id: row.id,
                observed_at: parse_utc(&row.observed_at, "outcome observed_at")?,
                outcome,
                correction_of: row.correction_of,
                private_notes: row.private_notes,
            });
    }
    let decisions = decisions
        .into_iter()
        .map(|row| {
            Ok(FantasyDecisionStoredInput {
                outcomes: outcomes_by_decision.remove(&row.id).unwrap_or_default(),
                id: row.id,
                kind: row.kind,
                recommendation_id: row.recommendation_id,
                recommendation_fingerprint: row.recommendation_fingerprint,
                recorded_at: parse_utc(&row.recorded_at, "decision recorded_at")?,
                evaluated_at: parse_utc(&row.evaluated_at, "decision evaluated_at")?,
                chosen_alternative: row.chosen_alternative,
                manager_rationale: row.manager_rationale,
                projection_json: row.projection_json,
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    build_fantasy_decision_review(FantasyDecisionReviewBuildInput {
        league_id: league.id.clone(),
        league_name: league.name.clone(),
        generated_at: Utc::now(),
        week,
        season,
        include_private,
        decisions,
    })
    .map_err(anyhow::Error::msg)
}

fn parse_utc(value: &str, field: &str) -> anyhow::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .with_context(|| format!("parse {field} '{value}'"))
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone};
    use icelines_core::season_stats::SeasonType;
    use icelines_core::{
        build_fantasy_pickup_sequence, FantasyAssistantRules, FantasyDecisionOutcomeCompleteness,
        FantasyDecisionOutcomeLane, FantasyDecisionOutcomeSource, FantasyPickupSequenceContext,
        FantasyPickupSequenceInput, FantasyTodayState, FantasyWeekBudgetView,
        FANTASY_DECISION_OUTCOME_SCHEMA,
    };

    use super::*;

    fn at(day: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, day, 12, 0, 0)
            .single()
            .expect("fixture instant")
    }

    fn frozen_projection(league_id: &str, team_id: &str) -> String {
        let monday = NaiveDate::from_ymd_opt(2026, 11, 9).expect("fixture Monday");
        let view = build_fantasy_pickup_sequence(FantasyPickupSequenceInput {
            context: FantasyPickupSequenceContext {
                league_id: league_id.to_owned(),
                league_name: "Neutral League".to_owned(),
                fantasy_team_id: team_id.to_owned(),
                fantasy_team_name: "Neutral Team".to_owned(),
                stats_season: "20262027".to_owned(),
                season_type: SeasonType::Regular,
                competition_mode: "points".to_owned(),
                week_start: monday,
                week_end: monday + Duration::days(6),
                timezone: "UTC".to_owned(),
                generated_at: at(9),
                evaluated_at: at(9),
            },
            rules: FantasyAssistantRules::configured_2026(),
            budget: FantasyWeekBudgetView {
                schema: "fantasy_week_budget.v1".to_owned(),
                timezone: "UTC".to_owned(),
                week_start: monday,
                week_end: monday + Duration::days(6),
                acquisition_limit: 4,
                acquisitions_used: 0,
                acquisitions_remaining: 4,
                can_add: true,
                injury_reserve: 1,
                injury_reserve_active: 1,
                proactive_acquisitions_remaining: 3,
                can_proactively_add: true,
                injury_reserve_releases_on: None,
            },
            players: Vec::new(),
            transitions: Vec::new(),
            max_moves: 3,
            beam_width: 20,
            alternative_limit: 3,
            readiness: Vec::new(),
            evidence: Vec::new(),
        })
        .expect("build frozen plan");
        serde_json::to_string(&view).expect("serialize frozen plan")
    }

    #[test]
    fn l1_empty_review_uses_versioned_shared_contract() {
        let db = FantasyDb::open_in_memory().expect("in-memory fantasy db");
        db.create_league("Neutral League", "yahoo-standard")
            .expect("create league");
        db.set_active_league("Neutral League")
            .expect("activate league");
        let league = db
            .get_active_league()
            .expect("read league")
            .expect("active league");
        let view = assemble_fantasy_decision_review(&db, &league, 20, None, None, false)
            .expect("build review");
        assert_eq!(view.schema, icelines_core::FANTASY_DECISION_REVIEW_SCHEMA);
        assert!(view.items.is_empty());
        assert_eq!(view.summary.decisions, 0);
    }

    #[test]
    fn l1_review_joins_typed_outcome_without_exposing_private_fields() {
        let db = FantasyDb::open_in_memory().expect("in-memory fantasy db");
        let league_id = db
            .create_league("Neutral League", "yahoo-standard")
            .expect("create league");
        db.set_active_league("Neutral League")
            .expect("activate league");
        let team_id = db
            .create_team(&league_id, "Neutral Team", "Owner")
            .expect("create team");
        let projection = frozen_projection(&league_id, &team_id);
        let (decision_id, _) = db
            .record_decision(
                &league_id,
                &team_id,
                "week_plan",
                "hold",
                "fingerprint",
                at(9),
                0,
                Some("private rationale"),
                &projection,
            )
            .expect("record decision");
        let outcome = FantasyDecisionOutcome {
            schema: FANTASY_DECISION_OUTCOME_SCHEMA.to_owned(),
            decision_id: decision_id.clone(),
            lane: FantasyDecisionOutcomeLane::ActiveValue,
            completeness: FantasyDecisionOutcomeCompleteness::Provisional,
            source: FantasyDecisionOutcomeSource::Manager,
            source_observed_at: Some(at(16)),
            executed: None,
            actual_active_points_delta: Some(2.0),
            actual_usable_starts_delta: Some(0),
            matchup_result: None,
            user_final_points: None,
            opponent_final_points: None,
            reserve_needed: None,
            reserve_used: None,
        };
        db.record_typed_decision_outcome(&outcome, Some("private note"), None)
            .expect("record outcome");
        let league = db.get_active_league().unwrap().unwrap();
        let view = assemble_fantasy_decision_review(
            &db,
            &league,
            20,
            Some(NaiveDate::from_ymd_opt(2026, 11, 9).unwrap()),
            None,
            false,
        )
        .expect("assemble review");
        assert_eq!(view.items.len(), 1);
        assert_eq!(
            view.items[0].process,
            icelines_core::FantasyDecisionProcessAssessment::Supported
        );
        assert_eq!(
            view.items[0].result,
            icelines_core::FantasyDecisionResultAssessment::Positive
        );
        assert_eq!(view.items[0].manager_rationale, None);
        assert!(view.items[0].private_outcome_notes.is_empty());
        let public_json = serde_json::to_string(&view).unwrap();
        assert!(!public_json.contains("private rationale"));
        assert!(!public_json.contains("private note"));
        assert_eq!(view.items[0].decision_state, Some(FantasyTodayState::Ready));
    }
}
