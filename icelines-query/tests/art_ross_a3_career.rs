//! Phase Art Ross A.3 — career-aggregator end-to-end tests.
//!
//! Exercises `parse_query` → `Constraint::matches` for
//! `p.career`, `g.any10g EVER`, `p.streak`, and `g.seasons-with`
//! against a `MockProvider` that returns canned career-spanning
//! game lines.

use std::sync::Mutex;

use chrono::NaiveDate;
use icelines_query::data_provider::{
    DataProvider, EvalCtx, FetchError, FetchEvent, PlanRequirement,
};
use icelines_query::sliding_window::GameStatLine;
use icelines_query::{parse_query, FilterInput, StrictMode};

struct MockProvider {
    lines: Mutex<Vec<GameStatLine>>,
}
impl MockProvider {
    fn new(lines: Vec<GameStatLine>) -> Self {
        Self {
            lines: Mutex::new(lines),
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
        hits: 0,
        blocked_shots: 0,
        takeaways: 0,
        giveaways: 0,
        pim: 0,
        toi_seconds: 1200,
    }
}

fn fixed_today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 5, 7).unwrap()
}

/// Build a synthetic StatsRepository with one Center for testing
/// (mirrors the helper in art_ross_a2_executor.rs).
fn synthetic_repo() -> icelines_core::stats_repository::StatsRepository {
    use icelines_core::identity::{PlayerBio, PlayerId, PlayerIdentity};
    use icelines_core::model::{Position, Season, TeamAbbr};
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

    let stats = SeasonStats {
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
    };
    repo.upsert_stats(stats).unwrap();

    repo
}

/// `g.any10g>=5 EVER` — player has 5 goals in 10 consecutive
/// games during 2024-25 season → matches.
#[test]
fn l1_a3_any10g_ever_matches_when_streak_exists() {
    use icelines_core::identity::PlayerId;
    use icelines_core::model::Season;
    use icelines_core::season_stats::SeasonType;

    let repo = synthetic_repo();
    // 12 games in 2024-25 (Jan 2025 dates → 2024-25 season).
    // Last 5 games each have 1 goal — last 10 has 5 goals.
    let lines: Vec<GameStatLine> = (1..=12)
        .map(|i| {
            line(
                &format!("2025-01-{:02}", i),
                "EDM",
                if i > 7 { 1 } else { 0 },
                0,
            )
        })
        .collect();
    let provider = MockProvider::new(lines);
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), 20252026);

    let plan = parse_query(FilterInput::Cli("g.any10g>=5 EVER".to_string())).unwrap();
    let view = repo
        .view(PlayerId(8478402), Season(20252026), SeasonType::Regular)
        .unwrap();
    assert!(plan.root.matches(&view, &ctx));
}

/// `g.any10g>=5 EVER` — player has only 4 goals across 10 games
/// → doesn't match.
#[test]
fn l1_a3_any10g_ever_misses_when_no_streak() {
    use icelines_core::identity::PlayerId;
    use icelines_core::model::Season;
    use icelines_core::season_stats::SeasonType;

    let repo = synthetic_repo();
    let lines: Vec<GameStatLine> = (1..=12)
        .map(|i| {
            line(
                &format!("2025-01-{:02}", i),
                "EDM",
                if i > 8 { 1 } else { 0 },
                0,
            )
        })
        .collect();
    // Only 4 goals total across last 5 games; no 10-game window
    // hits 5+.
    let provider = MockProvider::new(lines);
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), 20252026);

    let plan = parse_query(FilterInput::Cli("g.any10g>=5 EVER".to_string())).unwrap();
    let view = repo
        .view(PlayerId(8478402), Season(20252026), SeasonType::Regular)
        .unwrap();
    assert!(!plan.root.matches(&view, &ctx));
}

/// `p.career>=100` — sum across all eligible seasons.
#[test]
fn l1_a3_career_lifetime_sum_matches() {
    use icelines_core::identity::PlayerId;
    use icelines_core::model::Season;
    use icelines_core::season_stats::SeasonType;

    let repo = synthetic_repo();
    // Spread 50 games × 2 goals each across two seasons = 100 goals
    // career.
    let mut lines: Vec<GameStatLine> = Vec::new();
    for i in 1..=25 {
        lines.push(line(&format!("2024-01-{:02}", i), "EDM", 2, 0));
    }
    for i in 1..=25 {
        lines.push(line(&format!("2025-01-{:02}", i), "EDM", 2, 0));
    }
    let provider = MockProvider::new(lines);
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), 20252026);

    let plan = parse_query(FilterInput::Cli("g.career>=100".to_string())).unwrap();
    let view = repo
        .view(PlayerId(8478402), Season(20252026), SeasonType::Regular)
        .unwrap();
    assert!(plan.root.matches(&view, &ctx));
}

/// `p.career>=200` — same player, threshold above career total →
/// doesn't match.
#[test]
fn l1_a3_career_lifetime_sum_misses_above_threshold() {
    use icelines_core::identity::PlayerId;
    use icelines_core::model::Season;
    use icelines_core::season_stats::SeasonType;

    let repo = synthetic_repo();
    let mut lines: Vec<GameStatLine> = Vec::new();
    for i in 1..=10 {
        lines.push(line(&format!("2025-01-{:02}", i), "EDM", 1, 0));
    }
    let provider = MockProvider::new(lines);
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), 20252026);

    let plan = parse_query(FilterInput::Cli("g.career>=100".to_string())).unwrap();
    let view = repo
        .view(PlayerId(8478402), Season(20252026), SeasonType::Regular)
        .unwrap();
    assert!(!plan.root.matches(&view, &ctx));
}

/// `p.streak>=5` — longest run of consecutive games with at least
/// one point.
#[test]
fn l1_a3_career_streak_matches() {
    use icelines_core::identity::PlayerId;
    use icelines_core::model::Season;
    use icelines_core::season_stats::SeasonType;

    let repo = synthetic_repo();
    let lines: Vec<GameStatLine> = (1..=10)
        .map(|i| line(&format!("2025-01-{:02}", i), "EDM", 0, 1))
        .collect();
    // 10 consecutive games each with 1 assist → streak = 10
    let provider = MockProvider::new(lines);
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), 20252026);

    let plan = parse_query(FilterInput::Cli("p.streak>=5".to_string())).unwrap();
    let view = repo
        .view(PlayerId(8478402), Season(20252026), SeasonType::Regular)
        .unwrap();
    assert!(plan.root.matches(&view, &ctx));
}

/// `p.streak>=5` — broken streak (gap with 0 points) →
/// the longest sub-run is shorter than 5.
#[test]
fn l1_a3_career_streak_misses_broken() {
    use icelines_core::identity::PlayerId;
    use icelines_core::model::Season;
    use icelines_core::season_stats::SeasonType;

    let repo = synthetic_repo();
    // 4 with point, 1 without, 4 with point — longest run = 4
    let lines: Vec<GameStatLine> = (1..=9)
        .map(|i| {
            line(
                &format!("2025-01-{:02}", i),
                "EDM",
                0,
                if i == 5 { 0 } else { 1 },
            )
        })
        .collect();
    let provider = MockProvider::new(lines);
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), 20252026);

    let plan = parse_query(FilterInput::Cli("p.streak>=5".to_string())).unwrap();
    let view = repo
        .view(PlayerId(8478402), Season(20252026), SeasonType::Regular)
        .unwrap();
    assert!(!plan.root.matches(&view, &ctx));
}

/// AT-age slicing — `g.any10g>=5 EVER AT age<=21` filters
/// seasons to those where the player was ≤21. With our synthetic
/// player born 2003-04-15, the 2025-26 season is age 22 (HR
/// Feb-1 convention), 2024-25 is age 21, 2023-24 is age 20.
/// Putting goals only in 2025-26 means age<=21 slices it out
/// → no matching window.
#[test]
fn l1_a3_at_age_slicing_filters_seasons() {
    use icelines_core::identity::PlayerId;
    use icelines_core::model::Season;
    use icelines_core::season_stats::SeasonType;

    let repo = synthetic_repo();
    // 12 games in early-2026 dates → 2025-26 season → age 22.
    let lines: Vec<GameStatLine> = (1..=12)
        .map(|i| {
            line(
                &format!("2026-02-{:02}", i),
                "EDM",
                if i > 7 { 1 } else { 0 },
                0,
            )
        })
        .collect();
    let provider = MockProvider::new(lines);
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), 20252026);

    // age <= 21 — slices out 2025-26. No seasons pass; returns false.
    let plan_filter =
        parse_query(FilterInput::Cli("g.any10g>=5 EVER AT age<=21".to_string())).unwrap();
    let view = repo
        .view(PlayerId(8478402), Season(20252026), SeasonType::Regular)
        .unwrap();
    assert!(!plan_filter.root.matches(&view, &ctx));

    // age <= 22 — keeps 2025-26 in the slice. 5 goals in last 10
    // → matches.
    let plan_keep =
        parse_query(FilterInput::Cli("g.any10g>=5 EVER AT age<=22".to_string())).unwrap();
    assert!(plan_keep.root.matches(&view, &ctx));
}

/// Lockout 2004-05 is skipped (no data, no partial-mark).
#[test]
fn l1_a3_lockout_season_skipped() {
    use icelines_core::identity::PlayerId;
    use icelines_core::model::Season;
    use icelines_core::season_stats::SeasonType;

    let repo = synthetic_repo();
    // 10 games dated in Jan 2005 — 2004-05 season → skipped.
    // The player otherwise has zero career data.
    let lines: Vec<GameStatLine> = (1..=10)
        .map(|i| line(&format!("2005-01-{:02}", i), "EDM", 1, 0))
        .collect();
    let provider = MockProvider::new(lines);
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), 20252026);

    let plan = parse_query(FilterInput::Cli("g.career>=5".to_string())).unwrap();
    let view = repo
        .view(PlayerId(8478402), Season(20252026), SeasonType::Regular)
        .unwrap();
    // Lockout is skipped → career sum = 0 → predicate false.
    assert!(!plan.root.matches(&view, &ctx));
}

/// Killer query: `g.any10g>=5 EVER AT age<=25` against a player
/// with the right shape.
#[test]
fn l1_a3_killer_query_with_at_age() {
    use icelines_core::identity::PlayerId;
    use icelines_core::model::Season;
    use icelines_core::season_stats::SeasonType;

    let repo = synthetic_repo();
    let lines: Vec<GameStatLine> = (1..=12)
        .map(|i| {
            line(
                &format!("2025-01-{:02}", i),
                "EDM",
                if i > 7 { 1 } else { 0 },
                0,
            )
        })
        .collect();
    let provider = MockProvider::new(lines);
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), 20252026);

    let plan = parse_query(FilterInput::Cli("g.any10g>=5 EVER AT age<=25".to_string())).unwrap();
    let view = repo
        .view(PlayerId(8478402), Season(20252026), SeasonType::Regular)
        .unwrap();
    assert!(plan.root.matches(&view, &ctx));
}

/// `needs_provider()` returns true for CareerAggregate.
#[test]
fn l0_a3_needs_provider_for_career_aggregate() {
    let plan = parse_query(FilterInput::Cli("p.career>=500".to_string())).unwrap();
    assert!(plan.root.needs_provider());
}
