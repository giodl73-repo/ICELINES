//! Shared fantasy week-plan assembly boundary.
//!
//! Source adapters resolve ownership, schedule, projections, waivers, and locks
//! into the versioned core input. Every interactive surface invokes this
//! boundary so the optimizer is never reimplemented by a renderer.

use icelines_core::{
    build_fantasy_pickup_sequence, FantasyPickupSequenceError, FantasyPickupSequenceInput,
    FantasyPickupSequenceView,
};

#[derive(Debug, thiserror::Error)]
pub enum FantasyWeekPlanAssemblyError {
    #[error(transparent)]
    Planner(#[from] FantasyPickupSequenceError),
    #[error(transparent)]
    Today(#[from] crate::fantasy_today_service::FantasyTodayAssemblyError),
    #[error("daily assembly did not produce a week plan")]
    MissingWeekPlan,
}

pub fn assemble_fantasy_week_plan_from_sources(
    request: crate::fantasy_today_service::FantasyTodayAssemblyRequest,
) -> Result<FantasyPickupSequenceView, FantasyWeekPlanAssemblyError> {
    crate::fantasy_today_service::assemble_fantasy_today(request)?
        .week_plan
        .ok_or(FantasyWeekPlanAssemblyError::MissingWeekPlan)
}

pub fn assemble_fantasy_week_plan(
    input: FantasyPickupSequenceInput,
) -> Result<FantasyPickupSequenceView, FantasyWeekPlanAssemblyError> {
    build_fantasy_pickup_sequence(input).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l1_service_preserves_typed_planner_errors() {
        let error = assemble_fantasy_week_plan(FantasyPickupSequenceInput {
            context: icelines_core::FantasyPickupSequenceContext {
                league_id: "league".to_owned(),
                league_name: "League".to_owned(),
                fantasy_team_id: "team".to_owned(),
                fantasy_team_name: "Team".to_owned(),
                stats_season: "20252026".to_owned(),
                season_type: icelines_core::season_stats::SeasonType::Regular,
                competition_mode: "points".to_owned(),
                week_start: chrono::NaiveDate::from_ymd_opt(2026, 11, 10).unwrap(),
                week_end: chrono::NaiveDate::from_ymd_opt(2026, 11, 16).unwrap(),
                timezone: "America/Los_Angeles".to_owned(),
                generated_at: chrono::Utc::now(),
                evaluated_at: chrono::Utc::now(),
            },
            rules: icelines_core::FantasyAssistantRules::configured_2026(),
            budget: icelines_core::FantasyWeekBudgetView {
                schema: icelines_core::view_model::fantasy_assistant::FANTASY_WEEK_BUDGET_SCHEMA
                    .to_owned(),
                timezone: "America/Los_Angeles".to_owned(),
                week_start: chrono::NaiveDate::from_ymd_opt(2026, 11, 10).unwrap(),
                week_end: chrono::NaiveDate::from_ymd_opt(2026, 11, 16).unwrap(),
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
            beam_width: 10,
            alternative_limit: 2,
            readiness: Vec::new(),
            evidence: Vec::new(),
        })
        .unwrap_err();
        assert!(matches!(
            error,
            FantasyWeekPlanAssemblyError::Planner(FantasyPickupSequenceError::InvalidWeek)
        ));
    }
}
