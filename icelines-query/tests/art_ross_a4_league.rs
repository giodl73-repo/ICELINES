//! Phase Art Ross A.4 — cross-league career-atom end-to-end tests.
//!
//! Exercises `parse_query` → `Constraint::matches` for league
//! atoms (`league=OHL`, `league IN (...)`, `league.tier=Junior`,
//! `p.career.junior>=200`) against a `MockProvider` that returns
//! canned career-history records.

use std::sync::Mutex;

use chrono::NaiveDate;
use icelines_core::career_history::{
    CareerGameType, CareerHistory, CareerStint, LeagueAbbrev,
};
use icelines_core::model::Season;
use icelines_query::data_provider::{
    DataProvider, EvalCtx, FetchError, FetchEvent, PlanRequirement,
};
use icelines_query::sliding_window::GameStatLine;
use icelines_query::{parse_query, FilterInput, StrictMode};

struct MockProvider {
    history: Mutex<Option<CareerHistory>>,
}
impl MockProvider {
    fn with_history(history: CareerHistory) -> Self {
        Self {
            history: Mutex::new(Some(history)),
        }
    }
    fn empty() -> Self {
        Self {
            history: Mutex::new(None),
        }
    }
}
impl DataProvider for MockProvider {
    fn ensure(
        &self,
        _req: &PlanRequirement,
        _events: &mut dyn FnMut(FetchEvent),
    ) -> Result<(), FetchError> {
        Ok(())
    }
    fn fetch_game_lines(&self, _pid: u32, _season: u32) -> Vec<GameStatLine> {
        Vec::new()
    }
    fn fetch_career_history(&self, _pid: u32) -> Option<CareerHistory> {
        self.history.lock().unwrap().clone()
    }
}

fn fixed_today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 5, 7).unwrap()
}

fn skater_stint(season: u32, league: &str, gp: u32, points: u32) -> CareerStint {
    CareerStint {
        season: Season(season),
        league: LeagueAbbrev::new(league),
        team: "TEST".to_string(),
        game_type: CareerGameType::Regular,
        sequence: 0,
        gp,
        goals: Some(points / 2),
        assists: Some(points - points / 2),
        points: Some(points),
        pim: None,
        plus_minus: None,
        power_play_goals: None,
        power_play_points: None,
        shorthanded_goals: None,
        shorthanded_points: None,
        game_winning_goals: None,
        ot_goals: None,
        shots: None,
        shooting_pct: None,
        avg_toi_sec: None,
        faceoff_win_pct: None,
        games_started: None,
        wins: None,
        losses: None,
        ot_losses: None,
        goals_against: None,
        goals_against_avg: None,
        save_pct: None,
        shots_against: None,
        shutouts: None,
        time_on_ice_sec: None,
    }
}

fn synthetic_repo() -> icelines_core::stats_repository::StatsRepository {
    use icelines_core::identity::{PlayerBio, PlayerId, PlayerIdentity};
    use icelines_core::model::{Position, TeamAbbr};
    use icelines_core::season_stats::{SeasonStats, SeasonType, StatTotals, TeamStint};
    use icelines_core::stats_repository::StatsRepository;

    let mut repo = StatsRepository::with_lru_cap(8);
    let pid = PlayerId(8478402);

    repo.upsert_identity(PlayerIdentity {
        id: pid,
        full_name: "Test McSkater".to_string(),
        name_normalized: "test mcskater".to_string(),
        headshot_canonical_url: None,
        bio: PlayerBio {
            birth_date: Some("2003-04-15".to_string()),
            birth_country: Some("CAN".to_string()),
            nationality_code: Some("CAN".to_string()),
            birth_city: Some("Toronto".to_string()),
            birth_state_province: Some("ON".to_string()),
            height_in_inches: Some(72),
            weight_lbs: Some(190),
            draft_year: Some(2021),
            draft_round: Some(1),
            draft_overall: Some(5),
            shoots_catches: Some("L".to_string()),
            rookie_season: Some("20212022".to_string()),
        },
    })
    .unwrap();

    repo.upsert_stats(SeasonStats {
        player_id: pid,
        season: Season(20252026),
        season_type: SeasonType::Regular,
        position: Position::Center,
        sweater_number: Some(13),
        team_stints: vec![TeamStint {
            team: TeamAbbr("EDM".to_string()),
            started: None,
            ended: None,
            gp: 50,
            goals: 25,
            assists: 35,
            points: 60,
            goalie: None,
        }],
        totals: StatTotals::default(),
        realtime: None,
        advanced: None,
        goalie: None,
        time_on_ice: None,
        goals_for_against: None,
        goalie_advanced: None,
        goalie_saves_by_strength: None,
        goalie_bios: None,
    })
    .unwrap();

    repo
}

fn view_for_repo(
    repo: &icelines_core::stats_repository::StatsRepository,
) -> icelines_core::stats_repository::PlayerView<'_> {
    use icelines_core::identity::PlayerId;
    use icelines_core::season_stats::SeasonType;
    repo.view(PlayerId(8478402), Season(20252026), SeasonType::Regular)
        .expect("view")
}

/// `league=OHL` matches a player with at least one OHL stint.
#[test]
fn l1_a4_league_eq_matches_when_player_played_in_league() {
    let repo = synthetic_repo();
    let history = CareerHistory {
        player_id: 8478402,
        stints: vec![
            skater_stint(20192020, "OHL", 60, 95),
            skater_stint(20202021, "OHL", 50, 80),
            skater_stint(20212022, "NHL", 82, 60),
        ],
    };
    let provider = MockProvider::with_history(history);
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), 20252026);

    let plan = parse_query(FilterInput::Cli("league=OHL".to_string())).unwrap();
    assert!(plan.root.matches(&view_for_repo(&repo), &ctx));
}

#[test]
fn l1_a4_league_eq_misses_when_no_stint() {
    let repo = synthetic_repo();
    let history = CareerHistory {
        player_id: 8478402,
        stints: vec![skater_stint(20212022, "NHL", 82, 60)],
    };
    let provider = MockProvider::with_history(history);
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), 20252026);

    let plan = parse_query(FilterInput::Cli("league=OHL".to_string())).unwrap();
    assert!(!plan.root.matches(&view_for_repo(&repo), &ctx));
}

/// `league IN (OHL, WHL, QMJHL)` matches CHL-three players.
#[test]
fn l1_a4_league_in_set_matches_any() {
    let repo = synthetic_repo();
    let history = CareerHistory {
        player_id: 8478402,
        stints: vec![skater_stint(20192020, "WHL", 60, 90)],
    };
    let provider = MockProvider::with_history(history);
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), 20252026);

    let plan = parse_query(FilterInput::Cli(
        "league IN (OHL, WHL, QMJHL)".to_string(),
    ))
    .unwrap();
    assert!(plan.root.matches(&view_for_repo(&repo), &ctx));
}

/// `league NOT IN (NHL)` matches a player who never played NHL
/// (in this fixture, the player only played OHL).
#[test]
fn l1_a4_league_not_in_matches_when_no_match() {
    let repo = synthetic_repo();
    let history = CareerHistory {
        player_id: 8478402,
        stints: vec![skater_stint(20192020, "OHL", 60, 95)],
    };
    let provider = MockProvider::with_history(history);
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), 20252026);

    let plan = parse_query(FilterInput::Cli("league NOT IN (NHL)".to_string())).unwrap();
    assert!(plan.root.matches(&view_for_repo(&repo), &ctx));

    let plan_neg = parse_query(FilterInput::Cli("league NOT IN (OHL)".to_string())).unwrap();
    assert!(!plan_neg.root.matches(&view_for_repo(&repo), &ctx));
}

/// `league.tier=Junior` matches players with junior stints.
#[test]
fn l1_a4_league_tier_junior_matches() {
    let repo = synthetic_repo();
    let history = CareerHistory {
        player_id: 8478402,
        stints: vec![
            skater_stint(20192020, "OHL", 60, 95),
            skater_stint(20202021, "USHL", 50, 70),
        ],
    };
    let provider = MockProvider::with_history(history);
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), 20252026);

    let plan = parse_query(FilterInput::Cli("league.tier=Junior".to_string())).unwrap();
    assert!(plan.root.matches(&view_for_repo(&repo), &ctx));
}

/// `p.career.junior>=200` sums points across all junior-tier
/// stints.
#[test]
fn l1_a4_career_junior_sum_matches() {
    let repo = synthetic_repo();
    let history = CareerHistory {
        player_id: 8478402,
        stints: vec![
            skater_stint(20182019, "OHL", 60, 90), // junior
            skater_stint(20192020, "OHL", 60, 95), // junior
            skater_stint(20202021, "WHL", 60, 100), // junior
            skater_stint(20212022, "NHL", 82, 50), // pro — excluded
        ],
    };
    let provider = MockProvider::with_history(history);
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), 20252026);

    // Junior career sum = 90 + 95 + 100 = 285. Threshold 200.
    let plan = parse_query(FilterInput::Cli("p.career.junior>=200".to_string())).unwrap();
    assert!(plan.root.matches(&view_for_repo(&repo), &ctx));
}

#[test]
fn l1_a4_career_junior_sum_misses_above_threshold() {
    let repo = synthetic_repo();
    let history = CareerHistory {
        player_id: 8478402,
        stints: vec![skater_stint(20192020, "OHL", 60, 50)],
    };
    let provider = MockProvider::with_history(history);
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), 20252026);

    let plan = parse_query(FilterInput::Cli("p.career.junior>=200".to_string())).unwrap();
    assert!(!plan.root.matches(&view_for_repo(&repo), &ctx));
}

/// `p.career.nhl>=500` excludes non-NHL stints from the sum.
#[test]
fn l1_a4_career_nhl_only_excludes_other_leagues() {
    let repo = synthetic_repo();
    let history = CareerHistory {
        player_id: 8478402,
        stints: vec![
            skater_stint(20192020, "OHL", 60, 100), // not NHL
            skater_stint(20212022, "NHL", 82, 60), // NHL
            skater_stint(20222023, "NHL", 82, 70), // NHL
        ],
    };
    let provider = MockProvider::with_history(history);
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), 20252026);

    // NHL-only career = 60 + 70 = 130. Threshold 500 → no.
    let plan_high = parse_query(FilterInput::Cli("p.career.nhl>=500".to_string())).unwrap();
    assert!(!plan_high.root.matches(&view_for_repo(&repo), &ctx));

    // Threshold 100 → yes (130 >= 100).
    let plan_low = parse_query(FilterInput::Cli("p.career.nhl>=100".to_string())).unwrap();
    assert!(plan_low.root.matches(&view_for_repo(&repo), &ctx));
}

/// Empty career-history provider returns false (fail-closed).
#[test]
fn l1_a4_no_history_returns_false() {
    let repo = synthetic_repo();
    let provider = MockProvider::empty();
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), 20252026);

    let plan = parse_query(FilterInput::Cli("league=OHL".to_string())).unwrap();
    assert!(!plan.root.matches(&view_for_repo(&repo), &ctx));
}

/// Compound: junior elite cohort.
#[test]
fn l1_a4_compound_junior_cohort() {
    let repo = synthetic_repo();
    let history = CareerHistory {
        player_id: 8478402,
        stints: vec![skater_stint(20192020, "OHL", 60, 95)],
    };
    let provider = MockProvider::with_history(history);
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), 20252026);

    let plan = parse_query(FilterInput::Cli(
        "league.tier=Junior AND country=CAN".to_string(),
    ))
    .unwrap();
    assert!(plan.root.matches(&view_for_repo(&repo), &ctx));
}

/// `needs_provider()` returns true for CareerLeague atoms.
#[test]
fn l0_a4_needs_provider_for_career_league() {
    let plan = parse_query(FilterInput::Cli("league=OHL".to_string())).unwrap();
    assert!(plan.root.needs_provider());

    let plan_career = parse_query(FilterInput::Cli("p.career.junior>=200".to_string())).unwrap();
    assert!(plan_career.root.needs_provider());
}
