//! Phase Art Ross A.2 — sliding-window aggregation.
//!
//! `aggregate_window` operates on a sorted-ascending-by-date Vec
//! of `GameStatLine` entries (one per game played). It supports:
//! - `LastN_GP` — the last N games (with optional team-stint
//!   filtering and `WindowPolicy` for short-window handling)
//! - `LastN_Days` / `_Weeks` / `_Months` — calendar windows ending
//!   at the anchor date
//!
//! `GameStatLine` is icelines-query's own per-game struct;
//! `icelines-fetch` builds them from `SkaterLine` + the boxscore's
//! date metadata when populating the `BoxscoreIndex`. This
//! preserves the crate layering rule (icelines-query doesn't
//! reach up to icelines-fetch).

use chrono::{Days, NaiveDate};
use icelines_core::stats_catalog::StatId;

use crate::plan::{SlidingWindow, WindowPolicy, WindowScope};

/// One per-game stat line for one player. icelines-fetch produces
/// these from `SkaterLine` + the boxscore game date when building
/// a `BoxscoreIndex`. Sorted ascending by date.
#[derive(Debug, Clone, PartialEq)]
pub struct GameStatLine {
    pub player_id: u32,
    pub date: NaiveDate,
    pub game_id: u64,
    pub team_abbrev: String,
    pub goals: u32,
    pub assists: u32,
    pub plus_minus: i32,
    pub sog: u32,
    pub hits: u32,
    pub blocked_shots: u32,
    pub takeaways: u32,
    pub giveaways: u32,
    pub pim: u32,
    pub toi_seconds: u32,
}

/// The aggregated totals for a window. Fields mirror what's
/// queryable via StatId: `g` (goals), `a` (assists), `p` (points),
/// `+/-` (plus_minus), `sog`, `hits`, `blocks`, `tk`, `gv`,
/// `pim`, plus `toi_seconds` and `games` (counted).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WindowTotals {
    pub games: u32,
    pub goals: u32,
    pub assists: u32,
    pub plus_minus: i32,
    pub sog: u32,
    pub hits: u32,
    pub blocks: u32,
    pub takeaways: u32,
    pub giveaways: u32,
    pub pim: u32,
    pub toi_seconds: u32,
}

impl WindowTotals {
    pub fn points(&self) -> u32 {
        self.goals + self.assists
    }

    /// Per-game-played rate. Returns 0.0 when games == 0 to avoid
    /// division-by-zero (caller can choose to interpret as None
    /// via a separate gate if desired).
    pub fn rate(&self, value: f64) -> f64 {
        if self.games == 0 {
            0.0
        } else {
            value / self.games as f64
        }
    }
}

/// The result of evaluating a window. `Full` means the window
/// matched the requested size; `ShortWindow` means the player has
/// fewer GP than requested AND the policy allowed partial; `Empty`
/// means GP=0 (which always returns false from the predicate, so
/// the executor short-circuits without computing).
#[derive(Debug, Clone, PartialEq)]
pub enum WindowResult {
    Full(WindowTotals),
    ShortWindow { totals: WindowTotals, gp: u8 },
    Empty,
}

impl WindowResult {
    /// True iff there's at least one game in the window. Callers
    /// that don't care about full/short distinction use this gate.
    pub fn has_games(&self) -> bool {
        !matches!(self, WindowResult::Empty)
    }

    /// Borrow the totals. Returns None for Empty.
    pub fn totals(&self) -> Option<&WindowTotals> {
        match self {
            WindowResult::Full(t) | WindowResult::ShortWindow { totals: t, .. } => Some(t),
            WindowResult::Empty => None,
        }
    }
}

/// Aggregate per-game lines into a window total per the requested
/// `SlidingWindow` shape. `lines` must be sorted ascending by date.
/// `today` is the anchor date for calendar windows. `current_team`
/// is honored when `WindowScope::CurrentTeamCurrentSeason`.
pub fn aggregate_window(
    lines: &[GameStatLine],
    window: &SlidingWindow,
    today: NaiveDate,
    current_team: Option<&str>,
) -> WindowResult {
    if lines.is_empty() {
        return WindowResult::Empty;
    }

    match window {
        SlidingWindow::LastN_GP { n, scope, policy } => {
            let filtered: Vec<&GameStatLine> = match scope {
                WindowScope::CurrentTeamCurrentSeason => match current_team {
                    Some(team) => lines
                        .iter()
                        .filter(|l| l.team_abbrev.eq_ignore_ascii_case(team))
                        .collect(),
                    None => {
                        // A.2.5 review (edge) — IR-only player with
                        // no team_stints. Don't silently fall back
                        // to "all teams" when scope demands a team
                        // — the user's `team=EDM AND g.last10g>=5`
                        // would otherwise accept a bench-warmer's
                        // whole-league total. Empty is the
                        // honest answer.
                        return WindowResult::Empty;
                    }
                },
                WindowScope::AllTeamsCurrentSeason | WindowScope::Career => {
                    lines.iter().collect()
                }
            };

            if filtered.is_empty() {
                return WindowResult::Empty;
            }

            let n = *n as usize;
            let gp = filtered.len();
            let take = if gp >= n {
                n
            } else {
                // GP < n: WindowPolicy decides.
                match policy {
                    WindowPolicy::RequireFull => return WindowResult::Empty,
                    WindowPolicy::AllowPartial => gp,
                    WindowPolicy::AllowPartialAbove(threshold) => {
                        if gp >= *threshold as usize {
                            gp
                        } else {
                            return WindowResult::Empty;
                        }
                    }
                }
            };

            // Take the trailing `take` games (most recent).
            let start = filtered.len().saturating_sub(take);
            let totals = sum_lines(filtered.iter().skip(start).copied());
            if take == n {
                WindowResult::Full(totals)
            } else {
                WindowResult::ShortWindow {
                    totals,
                    gp: take as u8,
                }
            }
        }
        SlidingWindow::LastN_Days(n) => {
            let cutoff = today.checked_sub_days(Days::new(*n as u64)).unwrap_or(today);
            aggregate_calendar(lines, cutoff, today)
        }
        SlidingWindow::LastN_Weeks(n) => {
            let cutoff = today
                .checked_sub_days(Days::new(*n as u64 * 7))
                .unwrap_or(today);
            aggregate_calendar(lines, cutoff, today)
        }
        SlidingWindow::LastN_Months(n) => {
            // Approximate months as 30 days each — for hockey
            // queries this is the sensible meaning ("last 3 months
            // of play" ≈ "last 90 days").
            let cutoff = today
                .checked_sub_days(Days::new(*n as u64 * 30))
                .unwrap_or(today);
            aggregate_calendar(lines, cutoff, today)
        }
    }
}

/// Helper for calendar windows. Filters by `cutoff <= date <= today`
/// and sums. Calendar windows always return `Full` (no short-window
/// gating — calendar windows don't have a "size" the user expects
/// to match).
fn aggregate_calendar(
    lines: &[GameStatLine],
    cutoff: NaiveDate,
    today: NaiveDate,
) -> WindowResult {
    let in_window: Vec<&GameStatLine> = lines
        .iter()
        .filter(|l| l.date >= cutoff && l.date <= today)
        .collect();
    if in_window.is_empty() {
        return WindowResult::Empty;
    }
    let totals = sum_lines(in_window.into_iter());
    WindowResult::Full(totals)
}

fn sum_lines<'a, I: Iterator<Item = &'a GameStatLine>>(lines: I) -> WindowTotals {
    let mut t = WindowTotals::default();
    for l in lines {
        t.games += 1;
        t.goals += l.goals;
        t.assists += l.assists;
        t.plus_minus += l.plus_minus;
        t.sog += l.sog;
        t.hits += l.hits;
        t.blocks += l.blocked_shots;
        t.takeaways += l.takeaways;
        t.giveaways += l.giveaways;
        t.pim += l.pim;
        t.toi_seconds += l.toi_seconds;
    }
    t
}

/// Map a `StatId` to the corresponding field on `WindowTotals`.
/// Returns None for stats that don't have a per-game representation
/// (e.g. season-derived rates, on-ice deployment metrics — these
/// require a different aggregation tier).
pub fn extract_window_stat(stat: StatId, totals: &WindowTotals) -> Option<f64> {
    let key = stat.cli_key();
    match key {
        "games" => Some(totals.games as f64),
        "goals" => Some(totals.goals as f64),
        "assists" => Some(totals.assists as f64),
        "points" => Some(totals.points() as f64),
        "plus-minus" => Some(totals.plus_minus as f64),
        "shots" => Some(totals.sog as f64),
        "hits" => Some(totals.hits as f64),
        "blocked-shots" => Some(totals.blocks as f64),
        "takeaways" => Some(totals.takeaways as f64),
        "giveaways" => Some(totals.giveaways as f64),
        "pim" => Some(totals.pim as f64),
        "total-toi" => Some(totals.toi_seconds as f64),
        // Per-game rates derived from the window itself.
        "points-per-game" => {
            if totals.games == 0 {
                None
            } else {
                Some(totals.points() as f64 / totals.games as f64)
            }
        }
        "goals-per-game" => {
            if totals.games == 0 {
                None
            } else {
                Some(totals.goals as f64 / totals.games as f64)
            }
        }
        "assists-per-game" => {
            if totals.games == 0 {
                None
            } else {
                Some(totals.assists as f64 / totals.games as f64)
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(date: &str, team: &str, g: u32, a: u32) -> GameStatLine {
        GameStatLine {
            player_id: 8478402,
            date: NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap(),
            game_id: 0,
            team_abbrev: team.to_string(),
            goals: g,
            assists: a,
            plus_minus: 0,
            sog: 0,
            hits: 0,
            blocked_shots: 0,
            takeaways: 0,
            giveaways: 0,
            pim: 0,
            toi_seconds: 1200,
        }
    }

    #[test]
    fn l0_aggregate_last10g_full_window() {
        // 12 games on EDM, scope=current-team, n=10. Should take
        // last 10 (most recent) games.
        let lines: Vec<GameStatLine> = (1..=12)
            .map(|i| {
                line(
                    &format!("2026-01-{:02}", i),
                    "EDM",
                    if i > 2 { 1 } else { 0 }, // 10 goals total in last 10
                    0,
                )
            })
            .collect();
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
                assert_eq!(t.goals, 10);
            }
            other => panic!("expected Full, got {other:?}"),
        }
    }

    #[test]
    fn l0_aggregate_last10g_require_full_short_returns_empty() {
        // 5 games — RequireFull rejects.
        let lines: Vec<GameStatLine> = (1..=5)
            .map(|i| line(&format!("2026-01-{:02}", i), "EDM", 1, 0))
            .collect();
        let win = SlidingWindow::LastN_GP {
            n: 10,
            scope: WindowScope::CurrentTeamCurrentSeason,
            policy: WindowPolicy::RequireFull,
        };
        let today = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();
        let result = aggregate_window(&lines, &win, today, Some("EDM"));
        assert_eq!(result, WindowResult::Empty);
    }

    #[test]
    fn l0_aggregate_last10g_allow_partial_returns_short() {
        // 5 games — AllowPartial yields ShortWindow.
        let lines: Vec<GameStatLine> = (1..=5)
            .map(|i| line(&format!("2026-01-{:02}", i), "EDM", 1, 0))
            .collect();
        let win = SlidingWindow::LastN_GP {
            n: 10,
            scope: WindowScope::CurrentTeamCurrentSeason,
            policy: WindowPolicy::AllowPartial,
        };
        let today = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();
        let result = aggregate_window(&lines, &win, today, Some("EDM"));
        match result {
            WindowResult::ShortWindow { totals, gp } => {
                assert_eq!(gp, 5);
                assert_eq!(totals.games, 5);
                assert_eq!(totals.goals, 5);
            }
            other => panic!("expected ShortWindow, got {other:?}"),
        }
    }

    #[test]
    fn l0_aggregate_last10g_allow_partial_above_threshold() {
        // 3 games, threshold=5 → Empty.
        let lines: Vec<GameStatLine> = (1..=3)
            .map(|i| line(&format!("2026-01-{:02}", i), "EDM", 1, 0))
            .collect();
        let win = SlidingWindow::LastN_GP {
            n: 10,
            scope: WindowScope::CurrentTeamCurrentSeason,
            policy: WindowPolicy::AllowPartialAbove(5),
        };
        let today = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();
        let result = aggregate_window(&lines, &win, today, Some("EDM"));
        assert_eq!(result, WindowResult::Empty);
    }

    #[test]
    fn l0_aggregate_last10g_current_team_filters_stints() {
        // 5 games on EDM, then 5 on DAL. Current team = DAL.
        // Last 10 games on DAL = only 5 → with RequireFull this is
        // Empty (the DAL stint is short of 10).
        let mut lines: Vec<GameStatLine> = Vec::new();
        for i in 1..=5 {
            lines.push(line(&format!("2026-01-{:02}", i), "EDM", 2, 0));
        }
        for i in 1..=5 {
            lines.push(line(&format!("2026-02-{:02}", i), "DAL", 1, 0));
        }
        let win = SlidingWindow::LastN_GP {
            n: 10,
            scope: WindowScope::CurrentTeamCurrentSeason,
            policy: WindowPolicy::RequireFull,
        };
        let today = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();
        let result = aggregate_window(&lines, &win, today, Some("DAL"));
        // DAL only has 5 games — RequireFull rejects.
        assert_eq!(result, WindowResult::Empty);
    }

    #[test]
    fn l0_aggregate_last10g_allteams_includes_all_stints() {
        let mut lines: Vec<GameStatLine> = Vec::new();
        for i in 1..=5 {
            lines.push(line(&format!("2026-01-{:02}", i), "EDM", 2, 0));
        }
        for i in 1..=5 {
            lines.push(line(&format!("2026-02-{:02}", i), "DAL", 1, 0));
        }
        let win = SlidingWindow::LastN_GP {
            n: 10,
            scope: WindowScope::AllTeamsCurrentSeason,
            policy: WindowPolicy::RequireFull,
        };
        let today = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();
        let result = aggregate_window(&lines, &win, today, Some("DAL"));
        match result {
            WindowResult::Full(t) => {
                assert_eq!(t.games, 10);
                assert_eq!(t.goals, 15); // 5×2 EDM + 5×1 DAL
            }
            other => panic!("expected Full, got {other:?}"),
        }
    }

    #[test]
    fn l0_aggregate_last30d_calendar() {
        // 5 games in Jan, 5 in Feb. today = Mar 1. last 30 days
        // catches Feb 1–28.
        let mut lines: Vec<GameStatLine> = Vec::new();
        for i in 1..=5 {
            lines.push(line(&format!("2026-01-{:02}", i), "EDM", 1, 0));
        }
        for i in 1..=5 {
            lines.push(line(&format!("2026-02-{:02}", i), "EDM", 2, 0));
        }
        let win = SlidingWindow::LastN_Days(30);
        let today = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        let result = aggregate_window(&lines, &win, today, Some("EDM"));
        match result {
            WindowResult::Full(t) => {
                assert_eq!(t.games, 5); // only Feb games
                assert_eq!(t.goals, 10);
            }
            other => panic!("expected Full, got {other:?}"),
        }
    }

    #[test]
    fn l0_aggregate_last3w_calendar() {
        // today = Mar 1. last 3 weeks (21 days) = Feb 8 onward.
        let mut lines: Vec<GameStatLine> = Vec::new();
        for i in 1..=10 {
            lines.push(line(&format!("2026-02-{:02}", i), "EDM", 1, 0));
        }
        // Feb 8, 9, 10 are in window (i=8,9,10).
        let win = SlidingWindow::LastN_Weeks(3);
        let today = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        let result = aggregate_window(&lines, &win, today, Some("EDM"));
        match result {
            WindowResult::Full(t) => assert_eq!(t.games, 3),
            other => panic!("expected Full, got {other:?}"),
        }
    }

    #[test]
    fn l0_aggregate_last3m_calendar() {
        // today = Apr 1. last 3 months (90 days) = Jan 1 onward.
        let lines: Vec<GameStatLine> = (1..=10)
            .map(|i| line(&format!("2026-01-{:02}", i), "EDM", 1, 0))
            .collect();
        let win = SlidingWindow::LastN_Months(3);
        let today = NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
        let result = aggregate_window(&lines, &win, today, Some("EDM"));
        match result {
            WindowResult::Full(t) => assert_eq!(t.games, 10),
            other => panic!("expected Full, got {other:?}"),
        }
    }

    #[test]
    fn l0_aggregate_empty_lines_returns_empty() {
        let win = SlidingWindow::LastN_GP {
            n: 10,
            scope: WindowScope::CurrentTeamCurrentSeason,
            policy: WindowPolicy::RequireFull,
        };
        let today = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();
        let result = aggregate_window(&[], &win, today, Some("EDM"));
        assert_eq!(result, WindowResult::Empty);
    }

    /// A.2.5 review (edge) — IR-only player without team_stints
    /// and scope=CurrentTeam must NOT silently fall back to
    /// all-stints (that would accept a bench-warmer's whole
    /// league total when the user asked for current-team only).
    #[test]
    fn l0_aggregate_current_team_none_returns_empty_not_silent_fallback() {
        let lines: Vec<GameStatLine> = (1..=10)
            .map(|i| line(&format!("2026-01-{:02}", i), "EDM", 1, 0))
            .collect();
        let win = SlidingWindow::LastN_GP {
            n: 10,
            scope: WindowScope::CurrentTeamCurrentSeason,
            policy: WindowPolicy::RequireFull,
        };
        let today = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();
        let result = aggregate_window(&lines, &win, today, None);
        assert_eq!(result, WindowResult::Empty);
    }

    #[test]
    fn l0_aggregate_calendar_no_games_in_window_returns_empty() {
        // All games in Dec 2025; today is May 2026; last 30 days
        // = April 2026 onward → no games in window.
        let lines: Vec<GameStatLine> = (1..=5)
            .map(|i| line(&format!("2025-12-{:02}", i), "EDM", 1, 0))
            .collect();
        let win = SlidingWindow::LastN_Days(30);
        let today = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();
        let result = aggregate_window(&lines, &win, today, Some("EDM"));
        assert_eq!(result, WindowResult::Empty);
    }

    // ── extract_window_stat (StatId → WindowTotals field) ───────

    fn fixture_totals() -> WindowTotals {
        WindowTotals {
            games: 10,
            goals: 8,
            assists: 12,
            plus_minus: 5,
            sog: 35,
            hits: 22,
            blocks: 8,
            takeaways: 12,
            giveaways: 9,
            pim: 6,
            toi_seconds: 12_000,
        }
    }

    #[test]
    fn l0_extract_goals() {
        let stat = StatId::from_cli_key("goals").unwrap();
        let t = fixture_totals();
        assert_eq!(extract_window_stat(stat, &t), Some(8.0));
    }

    #[test]
    fn l0_extract_points_derived() {
        let stat = StatId::from_cli_key("points").unwrap();
        let t = fixture_totals();
        assert_eq!(extract_window_stat(stat, &t), Some(20.0));
    }

    #[test]
    fn l0_extract_ppg_derived() {
        let stat = StatId::from_cli_key("points-per-game").unwrap();
        let t = fixture_totals();
        assert_eq!(extract_window_stat(stat, &t), Some(2.0));
    }

    #[test]
    fn l0_extract_ppg_zero_games_returns_none() {
        let stat = StatId::from_cli_key("points-per-game").unwrap();
        let t = WindowTotals::default();
        assert_eq!(extract_window_stat(stat, &t), None);
    }

    #[test]
    fn l0_extract_unknown_stat_returns_none() {
        // A stat with no per-game representation, e.g. on-ice xG.
        let stat = StatId::from_cli_key("ixg").unwrap_or_else(|| {
            // Fallback if the alias isn't in the catalog
            StatId::from_cli_key("on-ice-xg-for").unwrap()
        });
        let t = fixture_totals();
        // ixg / xG-for has no per-game representation in our
        // window — boxscores don't carry xG.
        assert_eq!(extract_window_stat(stat, &t), None);
    }
}
