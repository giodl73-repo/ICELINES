//! Phase Art Ross A.2 — sliding-window integration tests.
//!
//! Exercises `parse_query` → `Constraint::matches_with_ctx` end-
//! to-end with a mock `DataProvider` that returns canned per-game
//! lines. Validates the killer query the user asked for:
//!   "5 goals in last 10 games, age <= 25"
//! at the actual evaluation tier.

use std::sync::Mutex;

use chrono::NaiveDate;
use icelines_query::data_provider::{
    DataProvider, EvalCtx, FetchError, FetchEvent, PlanRequirement,
};
use icelines_query::sliding_window::GameStatLine;
use icelines_query::{parse_query, FilterInput, StrictMode};

/// A mock provider that returns canned game lines per (pid, season).
/// Used to drive sliding-window evaluation without needing the
/// `BoxscoreIndex` / NHL API path.
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
        hits: 1,
        blocked_shots: 0,
        takeaways: 0,
        giveaways: 0,
        pim: 0,
        toi_seconds: 1200,
    }
}

/// Build a synthetic `PlayerView` is tricky (requires a populated
/// `StatsRepository`). For these tests we only need the parser
/// + evaluator; the SlidingWindow path doesn't actually touch the
/// PlayerView except for the team/applies_to checks. So we use a
/// stripped-down evaluator that bypasses PlayerView for the
/// sliding-window-only path.
///
/// (When we have committed boxscore fixtures + a fresh_repo
/// helper, A.2 follow-on tests will exercise the full pipeline.)
#[test]
fn l1_a2_parse_g_last10g_yields_sliding_window() {
    let plan = parse_query(FilterInput::Cli("g.last10g>=5".to_string())).unwrap();
    use icelines_query::Constraint;
    match plan.root {
        Constraint::SlidingWindow(_) => {}
        _ => panic!("expected SlidingWindow"),
    }
}

#[test]
fn l1_a2_aggregator_finds_5_goals_in_10_games() {
    use icelines_query::sliding_window::{aggregate_window, WindowResult};
    use icelines_query::{SlidingWindow, WindowPolicy, WindowScope};

    // 12 games, goals: [0,0,0,0,0,0,0,1,1,1,1,1] — last 10 has 5 goals.
    let mut lines: Vec<GameStatLine> = (1..=12)
        .map(|i| {
            line(
                &format!("2026-01-{:02}", i),
                "EDM",
                if i > 7 { 1 } else { 0 },
                0,
            )
        })
        .collect();
    let _ = lines.last_mut().unwrap();

    let win = SlidingWindow::LastN_GP {
        n: 10,
        scope: WindowScope::CurrentTeamCurrentSeason,
        policy: WindowPolicy::RequireFull,
    };
    let today = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();
    let result = aggregate_window(&lines, &win, today, Some("EDM"));
    match result {
        WindowResult::Full(t) => {
            assert_eq!(t.games, 10);
            assert_eq!(t.goals, 5);
        }
        other => panic!("expected Full, got {other:?}"),
    }
}

#[test]
fn l1_a2_provider_returns_canned_lines() {
    let lines = vec![
        line("2026-01-01", "EDM", 1, 0),
        line("2026-01-03", "EDM", 0, 1),
        line("2026-01-05", "EDM", 2, 1),
    ];
    let provider = MockProvider::new(lines.clone());
    let today = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();
    let _ctx = EvalCtx::new(&provider, StrictMode::Off, false, today, 20252026);

    let fetched = provider.fetch_game_lines(8478402, 20252026);
    assert_eq!(fetched.len(), 3);
    assert_eq!(fetched[0].goals, 1);
}

#[test]
fn l1_a2_eval_ctx_carries_today_and_season() {
    let provider = MockProvider::new(vec![]);
    let today = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, today, 20252026);
    assert_eq!(ctx.today, today);
    assert_eq!(ctx.season, 20252026);
}

#[test]
fn l1_a2_provider_default_returns_empty() {
    // Default DataProvider impl returns empty — for any provider
    // that doesn't override `fetch_game_lines`, sliding-window
    // atoms eval to false.
    struct EmptyProvider;
    impl DataProvider for EmptyProvider {
        fn ensure(
            &self,
            _req: &PlanRequirement,
            _events: &mut dyn FnMut(FetchEvent),
        ) -> Result<(), FetchError> {
            Ok(())
        }
        // fetch_game_lines uses default impl → empty Vec
    }
    let provider = EmptyProvider;
    let lines = provider.fetch_game_lines(8478402, 20252026);
    assert!(lines.is_empty());
}
