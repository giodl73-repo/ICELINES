use std::{collections::BTreeMap, env, fs, path::PathBuf};

use chrono::{NaiveDate, TimeZone, Utc};
use icelines_core::view_model::{FANTASY_INJURY_PLAN_SCHEMA, FANTASY_SCHEDULE_SCHEMA};
use icelines_core::{
    build_fantasy_daily_lineup, build_fantasy_roster_card,
    model::{Position, Season},
    season_stats::SeasonType,
    FantasyAssistantRules, FantasyInjuryPlanView, FantasyLineupPlayerInput,
    FantasyPlayerAvailabilityStatus, FantasyRosterCardInput, FantasyRosterScheduleView,
    FantasyScheduleClassRow, FantasyScheduleComplementRow, FantasyScheduleView, ViewContext,
    ViewWindow,
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = env::args_os().nth(1).map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from("examples/fantasy-roster-card-dexters-dawgs-2026-10-05.json")
    });
    let healthy = FantasyPlayerAvailabilityStatus::Healthy;
    // Names come from the user's 2025 historical workbook. Status and projected
    // values are deterministic fixture inputs, not current injury or ranking claims.
    let players = vec![
        player(
            "nathan-mackinnon",
            "Nathan MacKinnon",
            "COL",
            &[Position::Center],
            7.88,
            healthy,
        ),
        player(
            "macklin-celebrini",
            "Macklin Celebrini",
            "SJS",
            &[Position::Center],
            7.10,
            healthy,
        ),
        player(
            "drake-batherson",
            "Drake Batherson",
            "OTT",
            &[Position::LeftWing, Position::RightWing],
            6.13,
            healthy,
        ),
        player(
            "emmitt-finnie",
            "Emmitt Finnie",
            "DET",
            &[Position::LeftWing],
            5.85,
            healthy,
        ),
        player(
            "wyatt-johnston",
            "Wyatt Johnston",
            "DAL",
            &[Position::Center, Position::RightWing],
            6.40,
            healthy,
        ),
        player(
            "mackie-samoskevich",
            "Mackie Samoskevich",
            "FLA",
            &[Position::RightWing],
            5.40,
            healthy,
        ),
        player(
            "nikita-zadorov",
            "Nikita Zadorov",
            "BOS",
            &[Position::Defense],
            5.20,
            healthy,
        ),
        player(
            "vince-dunn",
            "Vince Dunn",
            "SEA",
            &[Position::Defense],
            5.90,
            healthy,
        ),
        player(
            "luke-hughes",
            "Luke Hughes",
            "NJD",
            &[Position::Defense],
            5.75,
            healthy,
        ),
        player(
            "alexander-romanov",
            "Alexander Romanov",
            "NYI",
            &[Position::Defense],
            4.85,
            healthy,
        ),
        player(
            "juuse-saros",
            "Juuse Saros",
            "NSH",
            &[Position::Goalie],
            6.90,
            healthy,
        ),
        player(
            "igor-shesterkin",
            "Igor Shesterkin",
            "NYR",
            &[Position::Goalie],
            6.55,
            healthy,
        ),
        player(
            "kyle-palmieri",
            "Kyle Palmieri",
            "NYI",
            &[Position::RightWing],
            4.95,
            healthy,
        ),
        player(
            "noah-hanifin",
            "Noah Hanifin",
            "VGK",
            &[Position::Defense],
            5.05,
            healthy,
        ),
        player(
            "spencer-knight",
            "Spencer Knight",
            "CHI",
            &[Position::Goalie],
            4.70,
            healthy,
        ),
        player(
            "alex-lyon",
            "Alex Lyon",
            "BUF",
            &[Position::Goalie],
            4.45,
            healthy,
        ),
        player(
            "elias-lindholm",
            "Elias Lindholm",
            "BOS",
            &[Position::Center],
            5.98,
            FantasyPlayerAvailabilityStatus::InjuredReserve,
        ),
        player(
            "boone-jenner",
            "Boone Jenner",
            "CBJ",
            &[Position::Center, Position::LeftWing],
            5.35,
            FantasyPlayerAvailabilityStatus::LongTermInjuredReserve,
        ),
        player(
            "justin-brazeau",
            "Justin Brazeau",
            "MIN",
            &[Position::RightWing],
            4.60,
            FantasyPlayerAvailabilityStatus::DayToDay,
        ),
        player(
            "ben-meyers",
            "Ben Meyers",
            "SEA",
            &[Position::Center],
            4.20,
            FantasyPlayerAvailabilityStatus::Out,
        ),
    ];
    let lineup = build_fantasy_daily_lineup(FantasyAssistantRules::configured_2026(), players)?;
    let date = NaiveDate::from_ymd_opt(2026, 10, 5).unwrap();
    let timestamp = Utc.with_ymd_and_hms(2026, 10, 5, 14, 0, 0).unwrap();
    let roster_teams = [
        "BOS", "BUF", "CHI", "COL", "DAL", "DET", "FLA", "MIN", "NJD", "NSH", "NYI", "NYR", "OTT",
        "SEA", "SJS", "VGK",
    ];
    let schedule = FantasyScheduleView {
        schema: FANTASY_SCHEDULE_SCHEMA.to_string(),
        season: 20262027,
        game_count: 1344,
        season_start: date,
        season_end: NaiveDate::from_ymd_opt(2027, 4, 18).unwrap(),
        off_night_max_games: 4,
        daily_slates: Vec::new(),
        weeks: Vec::new(),
        teams: Vec::new(),
        equivalence_classes: vec![
            FantasyScheduleClassRow { class_id: 1, teams: vec!["BOS".into(), "COL".into(), "DET".into(), "NYR".into()], average_within_overlap_pct: 47.5 },
            FantasyScheduleClassRow { class_id: 2, teams: vec!["ANA".into(), "MIN".into(), "SEA".into(), "VAN".into()], average_within_overlap_pct: 43.2 },
            FantasyScheduleClassRow { class_id: 3, teams: vec!["BUF".into(), "DAL".into(), "FLA".into(), "VGK".into()], average_within_overlap_pct: 45.1 },
            FantasyScheduleClassRow { class_id: 4, teams: vec!["CBJ".into(), "EDM".into(), "NJD".into(), "OTT".into()], average_within_overlap_pct: 44.4 },
            FantasyScheduleClassRow { class_id: 5, teams: vec!["CAR".into(), "CHI".into(), "NSH".into(), "SJS".into()], average_within_overlap_pct: 42.8 },
            FantasyScheduleClassRow { class_id: 6, teams: vec!["CGY".into(), "LAK".into(), "NYI".into(), "TOR".into()], average_within_overlap_pct: 46.0 },
            FantasyScheduleClassRow { class_id: 7, teams: vec!["MTL".into(), "PHI".into(), "PIT".into(), "UTA".into()], average_within_overlap_pct: 41.9 },
            FantasyScheduleClassRow { class_id: 8, teams: vec!["STL".into(), "TBL".into(), "WPG".into(), "WSH".into()], average_within_overlap_pct: 40.7 },
        ],
        roster: Some(FantasyRosterScheduleView {
            teams: roster_teams.iter().map(|team| (*team).to_string()).collect(),
            team_player_counts: BTreeMap::from([("BOS".to_string(), 2), ("SEA".to_string(), 2)]),
            roster_player_slots: 20,
            collision_dates: 71,
            total_team_games: 1312,
            distinct_active_dates: 182,
            utilization_pct: 84.6,
            highest_overlap_pairs: Vec::new(),
            best_complements: vec![FantasyScheduleComplementRow { team: "WSH".to_string(), average_roster_overlap_pct: 36.8, quiet_slate_games: 14, equivalence_class: 8 }],
        }),
        disclosures: vec!["Fixture schedule classes are deterministic contract data for renderer parity; regenerate from the official season schedule for live decisions.".to_string()],
    };
    let mut view = ViewContext::new(ViewWindow::new(Season(20262027), SeasonType::Regular));
    view.generated_at = Some(timestamp);
    let card = build_fantasy_roster_card(FantasyRosterCardInput {
        league_id: "dexters-dawgs-league".to_string(),
        league_name: "Dexter's 2026-27 League".to_string(),
        fantasy_team_id: "dexters-dawgs".to_string(),
        fantasy_team_name: "Dexter's Dawgs".to_string(),
        scoring_scheme_id: "dexters-dawgs".to_string(),
        scoring_scheme_name: "Dexter's Dawgs league scoring".to_string(),
        roster_snapshot_id: Some("historical-workbook-names-fixture-v1".to_string()),
        acquisitions_used_this_week: 2,
        injury_plan: FantasyInjuryPlanView {
            schema: FANTASY_INJURY_PLAN_SCHEMA.to_string(),
            date,
            lineup,
            statuses: Vec::new(),
            warnings: vec![
                "Historical roster names were read from the user's 2025 workbook; fixture statuses and projected values are deterministic examples, not current claims.".to_string(),
                "Refresh platform injuries, locks, eligibility, and free-agent state before acting.".to_string(),
            ],
        },
        schedule: Some(schedule),
        view,
        evidence_at: Some(timestamp),
    })?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        &output,
        format!("{}\n", serde_json::to_string_pretty(&card)?),
    )?;
    println!("WROTE {} {}", output.display(), card.fingerprint);
    Ok(())
}
