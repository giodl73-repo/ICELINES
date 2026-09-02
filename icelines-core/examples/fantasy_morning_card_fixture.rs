use std::{env, fs, path::PathBuf};

use chrono::{NaiveDate, TimeZone, Utc};
use icelines_core::view_model::{
    FANTASY_INJURY_PLAN_SCHEMA, FANTASY_WEEKLY_PICKUP_SCHEMA, FANTASY_WEEK_BUDGET_SCHEMA,
};
use icelines_core::{
    build_fantasy_daily_lineup, build_fantasy_goalie_plan, build_fantasy_morning_briefing,
    build_fantasy_morning_card,
    model::{Position, Season},
    season_stats::SeasonType,
    FantasyAssistantRules, FantasyCompetitionMode, FantasyGoalieGameInput, FantasyGoaliePlanInput,
    FantasyGoaliePlanPlayerInput, FantasyGoalieStartObservation, FantasyGoalieStartState,
    FantasyInjuryPlanView, FantasyLineupPlayerInput, FantasyMatchupStrategy,
    FantasyMorningCardInput, FantasyObservationConfidence, FantasyObservationFreshness,
    FantasyPlayerAvailabilityStatus, FantasyResolvedPlayerStatus, FantasyWeekBudgetView,
    FantasyWeeklyMoveRow, FantasyWeeklyPickupView, ViewContext, ViewWindow,
};

fn player(
    key: &str,
    name: &str,
    team: &str,
    positions: &[Position],
    value: f64,
    status: FantasyPlayerAvailabilityStatus,
) -> FantasyLineupPlayerInput {
    FantasyLineupPlayerInput {
        player_key: key.to_string(),
        display_name: name.to_string(),
        nhl_team: team.to_string(),
        platform_positions: positions.to_vec(),
        projected_value: value,
        has_game: true,
        status,
        locked_slot: None,
        locked: false,
    }
}

#[allow(clippy::too_many_arguments)]
fn goalie(
    key: &str,
    name: &str,
    team: &str,
    opponent: &str,
    date: NaiveDate,
    start: chrono::DateTime<Utc>,
    points: f64,
    probability: f64,
) -> FantasyGoaliePlanPlayerInput {
    FantasyGoaliePlanPlayerInput {
        player_key: key.to_string(),
        player: name.to_string(),
        nhl_team: team.to_string(),
        rostered: true,
        acquisition_eligible: false,
        games: vec![FantasyGoalieGameInput {
            date,
            start_time_utc: Some(start),
            opponent: opponent.to_string(),
            home: true,
            team_back_to_back: false,
            opponent_offense_index: 1.0,
        }],
        projected_points_per_start: points,
        historical_start_probability: probability,
        expected_save_percentage: Some(0.912),
        expected_goals_against_average: Some(2.61),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = env::args_os().nth(1).map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from("examples/fantasy-morning-card-sample-squad-2026-10-08.json")
    });
    let date = NaiveDate::from_ymd_opt(2026, 10, 8).unwrap();
    let evaluated_at = Utc.with_ymd_and_hms(2026, 10, 8, 14, 0, 0).unwrap();
    let game_start = Utc.with_ymd_and_hms(2026, 10, 9, 2, 0, 0).unwrap();
    let rules = FantasyAssistantRules::configured_2026();
    let healthy = FantasyPlayerAvailabilityStatus::Healthy;
    let lineup = build_fantasy_daily_lineup(
        rules.clone(),
        vec![
            player(
                "sample-player-002",
                "Sample Player 002",
                "COL",
                &[Position::Center],
                8.4,
                healthy,
            ),
            player(
                "sample-player-007",
                "Sample Player 007",
                "SJS",
                &[Position::Center],
                7.2,
                healthy,
            ),
            player(
                "sample-player-026",
                "Sample Player 026",
                "NYR",
                &[Position::LeftWing],
                7.8,
                healthy,
            ),
            player(
                "sample-player-009",
                "Sample Player 009",
                "OTT",
                &[Position::LeftWing, Position::RightWing],
                6.2,
                healthy,
            ),
            player(
                "sample-player-027",
                "Sample Player 027",
                "DAL",
                &[Position::RightWing],
                7.6,
                healthy,
            ),
            player(
                "sample-player-011",
                "Sample Player 011",
                "DAL",
                &[Position::Center, Position::RightWing],
                6.7,
                healthy,
            ),
            player(
                "sample-player-028",
                "Sample Player 028",
                "NYR",
                &[Position::Defense],
                6.9,
                healthy,
            ),
            player(
                "sample-player-014",
                "Sample Player 014",
                "SEA",
                &[Position::Defense],
                5.9,
                healthy,
            ),
            player(
                "sample-player-015",
                "Sample Player 015",
                "NJD",
                &[Position::Defense],
                5.8,
                healthy,
            ),
            player(
                "sample-player-006",
                "Sample Player 006",
                "NYR",
                &[Position::Goalie],
                6.7,
                healthy,
            ),
            player(
                "sample-player-017",
                "Sample Player 017",
                "NSH",
                &[Position::Goalie],
                6.5,
                healthy,
            ),
            player(
                "sample-player-024",
                "Sample Player 024",
                "MIN",
                &[Position::RightWing],
                4.6,
                FantasyPlayerAvailabilityStatus::DayToDay,
            ),
        ],
    )?;
    let injury_plan = FantasyInjuryPlanView {
        schema: FANTASY_INJURY_PLAN_SCHEMA.to_string(),
        date,
        lineup,
        statuses: vec![FantasyResolvedPlayerStatus {
            player_key: "sample-player-024".to_string(),
            reported_status: FantasyPlayerAvailabilityStatus::DayToDay,
            effective_status: FantasyPlayerAvailabilityStatus::DayToDay,
            freshness: FantasyObservationFreshness::Fresh,
            confidence: FantasyObservationConfidence::Reported,
            source: Some("deterministic-fixture".to_string()),
            source_url: None,
            observed_at: Some(evaluated_at),
            age_minutes: Some(0),
            requires_pregame_refresh: true,
            detail: Some("Refresh before lineup lock.".to_string()),
        }],
        warnings: vec![
            "Player availability is deterministic fixture evidence, not a current injury claim."
                .to_string(),
        ],
    };
    let budget = FantasyWeekBudgetView {
        schema: FANTASY_WEEK_BUDGET_SCHEMA.to_string(),
        timezone: rules.timezone.clone(),
        week_start: NaiveDate::from_ymd_opt(2026, 10, 5).unwrap(),
        week_end: NaiveDate::from_ymd_opt(2026, 10, 11).unwrap(),
        acquisition_limit: 4,
        acquisitions_used: 3,
        acquisitions_remaining: 1,
        can_add: true,
        injury_reserve: 1,
        injury_reserve_active: 1,
        proactive_acquisitions_remaining: 0,
        can_proactively_add: false,
        injury_reserve_releases_on: Some(NaiveDate::from_ymd_opt(2026, 10, 10).unwrap()),
    };
    let pickup_plan = FantasyWeeklyPickupView {
        schema: FANTASY_WEEKLY_PICKUP_SCHEMA.to_string(),
        budget: budget.clone(),
        rows: vec![FantasyWeeklyMoveRow {
            rank: 1,
            add_player_key: "sample-player-029".to_string(),
            add_player: "Sample Player 029".to_string(),
            drop_player_key: "bench-replacement".to_string(),
            drop_player: "Bench replacement".to_string(),
            incremental_usable_starts: 2.0,
            projected_value_delta: 6.4,
            reasons: vec!["Two usable off-night starts and defense scoring fit.".to_string()],
        }],
        blocked_waiver_candidates: 1,
        warnings: vec!["The final pickup remains protected for injury replacement.".to_string()],
    };
    let goalie_plan = build_fantasy_goalie_plan(FantasyGoaliePlanInput {
        league: "Sample 2026-27 League".to_string(),
        team: "Sample Multicategory".to_string(),
        week_start: budget.week_start,
        week_end: budget.week_end,
        focus_date: Some(date),
        strategy: FantasyMatchupStrategy::Balanced,
        competition_mode: FantasyCompetitionMode::Points,
        goalie_slots: 2,
        minimum_goalie_appearances: 3,
        current_goalie_appearances: 1.0,
        evaluated_at,
        max_age_minutes: 360,
        acquisitions_remaining: budget.acquisitions_remaining,
        goalies: vec![
            goalie(
                "sample-player-006",
                "Sample Player 006",
                "NYR",
                "BOS",
                date,
                game_start,
                6.7,
                0.72,
            ),
            goalie(
                "sample-player-017",
                "Sample Player 017",
                "NSH",
                "STL",
                date,
                game_start,
                6.5,
                0.66,
            ),
        ],
        observations: vec![FantasyGoalieStartObservation {
            player_key: "sample-player-006".to_string(),
            game_date: date,
            state: FantasyGoalieStartState::ReportedStarting,
            source: "deterministic-fixture".to_string(),
            source_url: None,
            observed_at: evaluated_at,
            fetched_at: evaluated_at,
            detail: Some("Reported starter; confirmation still required.".to_string()),
        }],
        warnings: vec![
            "Goalie starts are deterministic fixture evidence, not a current starter claim."
                .to_string(),
        ],
    })?;
    let briefing = build_fantasy_morning_briefing(
        evaluated_at,
        evaluated_at,
        rules.timezone,
        injury_plan,
        Some(goalie_plan),
        budget,
        Some(pickup_plan),
        None,
    );
    let mut view = ViewContext::new(ViewWindow::new(Season(20262027), SeasonType::Regular));
    view.generated_at = Some(evaluated_at);
    let card = build_fantasy_morning_card(FantasyMorningCardInput {
        league_id: "sample-multicategory-league".to_string(),
        league_name: "Sample 2026-27 League".to_string(),
        fantasy_team_id: "sample-multicategory".to_string(),
        fantasy_team_name: "Sample Multicategory".to_string(),
        scoring_scheme_id: "sample-multicategory".to_string(),
        scoring_scheme_name: "Sample Multicategory league scoring".to_string(),
        roster_snapshot_id: Some("morning-fixture-v1".to_string()),
        briefing,
        view,
    })?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        output,
        format!("{}\n", serde_json::to_string_pretty(&card)?),
    )?;
    Ok(())
}
