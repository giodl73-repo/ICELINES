//! Phase Art Ross A.2.6 — end-to-end executor coverage.
//!
//! A.2.6 review (bench) — `sliding_window_matches` in
//! executor.rs was untested with a real `PlayerView`. The 5
//! integration tests in `art_ross_a2_sliding.rs` exercised
//! either the parser-only path or `aggregate_window` directly;
//! none drove `Constraint::matches` against a populated
//! StatsRepository carrying a SlidingWindow constraint.
//!
//! This file closes that gap: builds a synthetic StatsRepository
//! with one player, provides a MockProvider that returns canned
//! GameStatLine entries, and exercises the killer-query end-to-
//! end path.

use chrono::NaiveDate;
use icelines_core::identity::PlayerId;
use icelines_core::model::Position;
use icelines_core::stats_repository::StatsRepository;
use icelines_query::data_provider::{
    DataProvider, EvalCtx, FetchError, FetchEvent, PlanRequirement,
};
use icelines_query::sliding_window::GameStatLine;
use icelines_query::{parse_query, FilterInput, StrictMode};

use std::sync::Mutex;

/// MockProvider that returns a canned set of game lines.
struct CannedProvider {
    lines: Mutex<Vec<GameStatLine>>,
}
impl CannedProvider {
    fn new(lines: Vec<GameStatLine>) -> Self {
        Self {
            lines: Mutex::new(lines),
        }
    }
}
impl DataProvider for CannedProvider {
    fn ensure(
        &self,
        _req: &PlanRequirement,
        _events: &mut dyn FnMut(FetchEvent),
    ) -> Result<(), FetchError> {
        Ok(())
    }
    fn fetch_game_lines(&self, _pid: u32, _season: u32) -> Vec<GameStatLine> {
        self.lines.lock().unwrap().clone()
    }
}

fn line(date: &str, team: &str, goals: u32, assists: u32) -> GameStatLine {
    GameStatLine {
        player_id: 8478402,
        date: NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap(),
        game_id: 0,
        team_abbrev: team.to_string(),
        goals,
        assists,
        plus_minus: 0,
        sog: goals + 2,
        hits: 1,
        blocked_shots: 0,
        takeaways: 0,
        giveaways: 0,
        pim: 0,
        toi_seconds: 1200,
    }
}

/// Build a synthetic StatsRepository with one Center for testing.
fn synthetic_repo_with_player() -> StatsRepository {
    use icelines_core::identity::{PlayerBio, PlayerIdentity};
    use icelines_core::model::{Season, TeamAbbr};
    use icelines_core::season_stats::{SeasonStats, SeasonType, StatTotals, TeamStint};

    let mut repo = StatsRepository::with_lru_cap(8);
    let pid = PlayerId(8478402);

    let identity = PlayerIdentity {
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
    };
    repo.upsert_identity(identity).expect("identity upsert");

    let season = Season(20252026);
    let stats = SeasonStats {
        player_id: pid,
        season,
        season_type: SeasonType::Regular,
        position: Position::Center,
        sweater_number: Some(13),
        team_stints: vec![TeamStint {
            team: TeamAbbr("EDM".to_string()),
            started: None,
            ended: None,
            gp: 50,
            goals: 20,
            assists: 30,
            points: 50,
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
    };
    repo.upsert_stats(stats).expect("stats upsert");

    repo
}

fn fixed_today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 5, 7).unwrap()
}

/// Killer query end-to-end: `g.last10g>=5 AND age<=25`. Player
/// has 12 EDM games with goals [0,0,0,0,0,0,0,1,1,1,1,1] →
/// last 10 has 5 goals → should match. Bio age = 22 (born
/// 2003-04-15, season 20252026, HR Feb-1 convention).
#[test]
fn l1_a26_killer_query_matches_when_streak_and_age_qualify() {
    let repo = synthetic_repo_with_player();
    let lines: Vec<GameStatLine> = (1..=12)
        .map(|i| {
            line(
                &format!("2026-01-{:02}", i),
                "EDM",
                if i > 7 { 1 } else { 0 },
                0,
            )
        })
        .collect();
    let provider = CannedProvider::new(lines);
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), 20252026);

    let plan = parse_query(FilterInput::Cli("g.last10g>=5 AND age<=25".to_string())).unwrap();

    let view = repo
        .view(
            PlayerId(8478402),
            icelines_core::model::Season(20252026),
            icelines_core::season_stats::SeasonType::Regular,
        )
        .expect("view present");

    assert!(
        plan.root.matches(&view, &ctx),
        "killer query should match (last 10 = 5G; age 22 <= 25)"
    );
}

/// Same player, but query needs 6 goals — should NOT match.
#[test]
fn l1_a26_killer_query_misses_when_streak_falls_short() {
    let repo = synthetic_repo_with_player();
    let lines: Vec<GameStatLine> = (1..=12)
        .map(|i| {
            line(
                &format!("2026-01-{:02}", i),
                "EDM",
                if i > 7 { 1 } else { 0 },
                0,
            )
        })
        .collect();
    let provider = CannedProvider::new(lines);
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), 20252026);

    let plan = parse_query(FilterInput::Cli("g.last10g>=6 AND age<=25".to_string())).unwrap();

    let view = repo
        .view(
            PlayerId(8478402),
            icelines_core::model::Season(20252026),
            icelines_core::season_stats::SeasonType::Regular,
        )
        .expect("view present");

    assert!(
        !plan.root.matches(&view, &ctx),
        "should NOT match — only 5 goals in last 10, threshold is 6"
    );
}

/// Same player, query age threshold is too low — should NOT match.
#[test]
fn l1_a26_killer_query_misses_when_age_disqualifies() {
    let repo = synthetic_repo_with_player();
    let lines: Vec<GameStatLine> = (1..=12)
        .map(|i| {
            line(
                &format!("2026-01-{:02}", i),
                "EDM",
                if i > 7 { 1 } else { 0 },
                0,
            )
        })
        .collect();
    let provider = CannedProvider::new(lines);
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), 20252026);

    let plan = parse_query(FilterInput::Cli("g.last10g>=5 AND age<=20".to_string())).unwrap();

    let view = repo
        .view(
            PlayerId(8478402),
            icelines_core::model::Season(20252026),
            icelines_core::season_stats::SeasonType::Regular,
        )
        .expect("view present");

    assert!(
        !plan.root.matches(&view, &ctx),
        "should NOT match — age is 22, threshold is 20"
    );
}

/// Provider returns empty (no boxscores on disk) → SlidingWindow
/// atom returns false → killer query returns false. This
/// exercises the unwired-data path that A.2.4 will fix when the
/// real IcelinesProvider lands.
#[test]
fn l1_a26_no_boxscores_returns_false_not_panic() {
    let repo = synthetic_repo_with_player();
    let provider = CannedProvider::new(vec![]); // no game lines
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), 20252026);

    let plan = parse_query(FilterInput::Cli("g.last10g>=5".to_string())).unwrap();

    let view = repo
        .view(
            PlayerId(8478402),
            icelines_core::model::Season(20252026),
            icelines_core::season_stats::SeasonType::Regular,
        )
        .expect("view present");

    assert!(
        !plan.root.matches(&view, &ctx),
        "no boxscores → empty window → false (fail-closed default)"
    );
}

/// Bio-only filter (no SlidingWindow) doesn't touch the provider.
/// Verifies the executor doesn't accidentally fan out for atoms
/// that don't need per-game data.
#[test]
fn l1_a26_bio_only_query_doesnt_touch_provider() {
    let repo = synthetic_repo_with_player();
    let provider = CannedProvider::new(vec![]);
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), 20252026);

    let plan = parse_query(FilterInput::Cli("age<=25 AND country=CAN".to_string())).unwrap();

    let view = repo
        .view(
            PlayerId(8478402),
            icelines_core::model::Season(20252026),
            icelines_core::season_stats::SeasonType::Regular,
        )
        .expect("view present");

    assert!(plan.root.matches(&view, &ctx));
}

/// Position atom — `pos=C` should match this Center.
#[test]
fn l1_a26_position_atom_matches_center() {
    let repo = synthetic_repo_with_player();
    let provider = CannedProvider::new(vec![]);
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), 20252026);

    let plan = parse_query(FilterInput::Cli("pos=C".to_string())).unwrap();
    let view = repo
        .view(
            PlayerId(8478402),
            icelines_core::model::Season(20252026),
            icelines_core::season_stats::SeasonType::Regular,
        )
        .expect("view present");
    assert!(plan.root.matches(&view, &ctx));
}

/// Team atom — `team=EDM` matches the current stint.
#[test]
fn l1_a26_team_atom_matches_current_stint() {
    let repo = synthetic_repo_with_player();
    let provider = CannedProvider::new(vec![]);
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), 20252026);

    let plan = parse_query(FilterInput::Cli("team=EDM".to_string())).unwrap();
    let view = repo
        .view(
            PlayerId(8478402),
            icelines_core::model::Season(20252026),
            icelines_core::season_stats::SeasonType::Regular,
        )
        .expect("view present");
    assert!(plan.root.matches(&view, &ctx));

    // Cross-team check.
    let plan_neg = parse_query(FilterInput::Cli("team=DAL".to_string())).unwrap();
    assert!(!plan_neg.root.matches(&view, &ctx));
}

/// Country IN-set hits via either birth_country or nationality_code.
#[test]
fn l1_a26_country_in_set_matches() {
    let repo = synthetic_repo_with_player();
    let provider = CannedProvider::new(vec![]);
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), 20252026);

    let plan = parse_query(FilterInput::Cli("country IN (CAN, USA, SWE)".to_string())).unwrap();
    let view = repo
        .view(
            PlayerId(8478402),
            icelines_core::model::Season(20252026),
            icelines_core::season_stats::SeasonType::Regular,
        )
        .expect("view present");
    assert!(plan.root.matches(&view, &ctx));

    let plan_neg = parse_query(FilterInput::Cli(
        "country NOT IN (CAN, USA, SWE)".to_string(),
    ))
    .unwrap();
    assert!(!plan_neg.root.matches(&view, &ctx));
}

/// Strict-comparator on bio: age<22 should be FALSE for a 22-year-old.
#[test]
fn l1_a26_strict_lt_age_boundary() {
    let repo = synthetic_repo_with_player();
    let provider = CannedProvider::new(vec![]);
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), 20252026);

    // Age = 22 (born 2003-04-15, season-end 2026 → 22 since
    // born after Feb 1).
    let plan = parse_query(FilterInput::Cli("age<22".to_string())).unwrap();
    let view = repo
        .view(
            PlayerId(8478402),
            icelines_core::model::Season(20252026),
            icelines_core::season_stats::SeasonType::Regular,
        )
        .expect("view present");
    assert!(!plan.root.matches(&view, &ctx));

    let plan_le = parse_query(FilterInput::Cli("age<=22".to_string())).unwrap();
    assert!(plan_le.root.matches(&view, &ctx));
}

/// `BETWEEN` on numeric bio.
#[test]
fn l1_a26_between_age_inclusive() {
    let repo = synthetic_repo_with_player();
    let provider = CannedProvider::new(vec![]);
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), 20252026);

    let plan = parse_query(FilterInput::Cli("age BETWEEN 22 AND 28".to_string())).unwrap();
    let view = repo
        .view(
            PlayerId(8478402),
            icelines_core::model::Season(20252026),
            icelines_core::season_stats::SeasonType::Regular,
        )
        .expect("view present");
    assert!(plan.root.matches(&view, &ctx));

    // Outside range
    let plan_out = parse_query(FilterInput::Cli("age BETWEEN 18 AND 21".to_string())).unwrap();
    assert!(!plan_out.root.matches(&view, &ctx));
}

/// LIKE on a string bio field.
#[test]
fn l1_a26_country_like_pattern() {
    let repo = synthetic_repo_with_player();
    let provider = CannedProvider::new(vec![]);
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), 20252026);

    let plan = parse_query(FilterInput::Cli(r#"country LIKE "CA*""#.to_string())).unwrap();
    let view = repo
        .view(
            PlayerId(8478402),
            icelines_core::model::Season(20252026),
            icelines_core::season_stats::SeasonType::Regular,
        )
        .expect("view present");
    assert!(plan.root.matches(&view, &ctx));
}
