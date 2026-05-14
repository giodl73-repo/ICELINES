//! Schedule tab renderer — week view, team-season view, head-to-head matchup view.

// Phase Norris.2 — `ScheduleScreenState` repeats the module name
// in the type identifier. Same canonical pattern as Norris.1's
// QueriesState — file-level allow keeps the lint quiet without
// renaming each per-screen struct to `State` (which would lose
// cross-module readability when imported into app.rs).
#![allow(clippy::module_name_repetitions)]

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{List, ListItem, Paragraph},
    Frame,
};

use crate::tui::app::App;
use crate::tui::schedule::{
    monday_of, new_standings_cache, new_team_cache, new_week_cache, today_iso, week_label,
    ScheduleState, SearchFilter, StandingsCache, StandingsState, TeamSeasonCache, WeekCache,
};
use crate::visual::{
    tui_error_style, tui_header_style, tui_meta_style, tui_panel_block, tui_selected_style,
    tui_title_style,
};
use icelines_core::model::Season;
use icelines_core::{
    ScheduleGameRow, ScheduleMatchupView, ScheduleRecord, ScheduleView, ScheduledGameInput,
    TeamSeasonGameRow, TeamSeasonVenue, TeamSeasonView, ViewContext, ViewWindow,
};
use icelines_fetch::nhl_api::ScheduledGame;

// ── Phase Norris.2 — per-screen state struct ─────────────────────────────────

/// Phase Norris.2 — owns every piece of state that belongs to the
/// Schedule tab. Replaces the 8 `schedule_*` fields previously
/// scattered across `App`. App now holds this as `app.schedule`.
///
/// **Naming asymmetry with Norris.1**: Norris.1 used the simpler
/// name `QueriesState`. Here we use `ScheduleScreenState` because
/// `tui::schedule::ScheduleState` already exists as a per-week
/// load-state enum (Idle / Loading / Loaded / Error). Renaming
/// that enum would be a wider churn for a cosmetic gain, so we
/// suffix the new struct with `Screen` to disambiguate.
#[derive(Debug)]
pub struct ScheduleScreenState {
    // Week + team caches (shared between renderer threads)
    pub week_cache: WeekCache,
    pub team_cache: TeamSeasonCache,
    pub standings_cache: StandingsCache,

    // Currently-viewed week (Monday "YYYY-MM-DD")
    pub week: String,

    // Search bar
    pub query: String,
    pub search_mode: bool,
    pub filter: SearchFilter,
    pub filter_err: Option<String>,

    // Cursor on the schedule rows
    pub selected: usize,
}

// ── Phase Masterton.1 — declarative chrome ───────────────────────────────────

/// Phase Masterton.1 — chrome accessor for the Schedule tab.
/// Title carries the active week; keybinds depend on whether
/// search mode is open.
pub fn chrome(state: &ScheduleScreenState) -> crate::tui::chrome::ScreenChrome {
    use crate::tui::chrome::{KeyHint, ScreenChrome};

    let title = if state.week.is_empty() {
        "Schedule".to_owned()
    } else {
        format!("Schedule — week of {}", state.week)
    };

    let keybinds = if state.search_mode {
        vec![
            KeyHint::new("Enter", "apply"),
            KeyHint::new("Esc", "cancel"),
            KeyHint::new("type", "team or matchup"),
        ]
    } else {
        vec![
            KeyHint::new("/", "search"),
            KeyHint::new("←/→", "prev/next week"),
            KeyHint::new("t", "today"),
            KeyHint::new("D", "pick date"),
            KeyHint::new("Enter", "open team / matchup"),
        ]
    };

    ScreenChrome { title, keybinds }
}

impl Default for ScheduleScreenState {
    fn default() -> Self {
        Self {
            week_cache: new_week_cache(),
            team_cache: new_team_cache(),
            standings_cache: new_standings_cache(),
            // Monday of today; falls back to today's ISO if Monday
            // resolution fails (mirrors the legacy App::new init).
            week: monday_of(&today_iso()).unwrap_or_else(today_iso),
            query: String::new(),
            search_mode: false,
            filter: SearchFilter::None,
            filter_err: None,
            selected: 0,
        }
    }
}

#[cfg(test)]
mod norris_state_tests {
    use super::*;

    // ── Phase Norris.2 — ScheduleScreenState contract ──────────────────────

    /// Default search mode is OFF — the bottom strip stays hidden
    /// on a fresh Schedule open. Catches a regression where a
    /// future refactor flips the default to true.
    #[test]
    fn l0_norris_schedule_default_search_mode_off() {
        let s = ScheduleScreenState::default();
        assert!(!s.search_mode);
    }

    /// Default query string is empty.
    #[test]
    fn l0_norris_schedule_default_query_empty() {
        let s = ScheduleScreenState::default();
        assert_eq!(s.query, "");
    }

    /// Default filter is `SearchFilter::None` — no team / matchup
    /// narrowing on first open.
    #[test]
    fn l0_norris_schedule_default_filter_unset() {
        let s = ScheduleScreenState::default();
        assert!(matches!(s.filter, SearchFilter::None));
    }

    /// No filter validation error on first open.
    #[test]
    fn l0_norris_schedule_default_no_filter_err() {
        let s = ScheduleScreenState::default();
        assert!(s.filter_err.is_none());
    }

    /// Cursor starts at row 0.
    #[test]
    fn l0_norris_schedule_default_selected_at_zero() {
        let s = ScheduleScreenState::default();
        assert_eq!(s.selected, 0);
    }

    /// Week defaults to a non-empty ISO date string. The
    /// `monday_of(today)` fallback can yield today_iso() when
    /// monday resolution fails, but the field should never be
    /// empty.
    #[test]
    fn l0_norris_schedule_default_week_is_non_empty_iso() {
        let s = ScheduleScreenState::default();
        assert!(!s.week.is_empty(), "week must be a populated ISO date");
        // Sanity: parse it as YYYY-MM-DD.
        assert!(
            chrono::NaiveDate::parse_from_str(&s.week, "%Y-%m-%d").is_ok(),
            "week {:?} must parse as ISO date",
            s.week
        );
    }

    /// Default caches are empty (no week or team rows loaded
    /// on init). The renderer fills them on demand.
    #[test]
    fn l0_norris_schedule_default_caches_empty() {
        let s = ScheduleScreenState::default();
        assert!(
            s.week_cache.lock().unwrap().is_empty(),
            "week cache must start empty"
        );
        assert!(
            s.team_cache.lock().unwrap().is_empty(),
            "team cache must start empty"
        );
    }

    /// Week defaults to a Monday (or to today_iso() if monday_of
    /// resolution failed, which would be a bug in monday_of itself).
    /// This test fences against a future refactor that swaps the
    /// fallback to a non-Monday default.
    #[test]
    fn l0_norris_schedule_default_week_is_a_monday() {
        use chrono::Datelike;
        let s = ScheduleScreenState::default();
        let parsed =
            chrono::NaiveDate::parse_from_str(&s.week, "%Y-%m-%d").expect("week must be ISO");
        // Either the resolved Monday OR today's date if monday_of
        // failed (unlikely; monday_of works for any valid date).
        let today =
            chrono::NaiveDate::parse_from_str(&today_iso(), "%Y-%m-%d").expect("today_iso parses");
        let is_monday = parsed.weekday() == chrono::Weekday::Mon;
        let is_today_fallback = parsed == today;
        assert!(
            is_monday || is_today_fallback,
            "default week must be a Monday OR today (fallback); got {:?}",
            parsed
        );
    }

    /// `App::new` wires `app.schedule` through
    /// ScheduleScreenState::default(). Spot-check the load-bearing
    /// fields (full equality requires PartialEq on Arc<Mutex<...>>
    /// which we don't derive).
    #[test]
    fn l0_norris_schedule_app_new_uses_default() {
        let app = crate::tui::app::App::new(false);
        assert!(!app.schedule.search_mode);
        assert_eq!(app.schedule.query, "");
        assert!(matches!(app.schedule.filter, SearchFilter::None));
        assert!(app.schedule.filter_err.is_none());
        assert_eq!(app.schedule.selected, 0);
        assert!(!app.schedule.week.is_empty());
    }

    /// Debug derive renders without panicking on a default state.
    /// Sanity check for forge-1.
    #[test]
    fn l0_norris_schedule_default_debug_renders() {
        let s = ScheduleScreenState::default();
        let dbg = format!("{:?}", s);
        assert!(
            dbg.contains("ScheduleScreenState"),
            "Debug output must include the struct name; got: {dbg}"
        );
    }

    // ── Phase Masterton.1 — chrome accessor contract ───────────────────────

    /// Default state yields chrome — title carries the active
    /// week (non-empty), keybinds are the navigation set.
    #[test]
    fn l0_masterton_schedule_chrome_default_includes_week_in_title() {
        let s = ScheduleScreenState::default();
        let c = chrome(&s);
        assert!(
            c.title.starts_with("Schedule"),
            "Schedule chrome title must start with 'Schedule'; got: {}",
            c.title
        );
        let keys: Vec<&str> = c.keybinds.iter().map(|k| k.key).collect();
        assert!(keys.contains(&"/"));
        assert!(keys.contains(&"←/→"));
    }

    /// Search-mode state yields a different keybind set —
    /// Enter/Esc/type instead of the navigation keys.
    #[test]
    fn l0_masterton_schedule_chrome_search_mode_swaps_keybinds() {
        let s = ScheduleScreenState {
            search_mode: true,
            ..Default::default()
        };
        let c = chrome(&s);
        let keys: Vec<&str> = c.keybinds.iter().map(|k| k.key).collect();
        assert!(keys.contains(&"Enter"));
        assert!(keys.contains(&"Esc"));
        assert!(
            !keys.contains(&"/"),
            "search-mode chrome must NOT advertise / (already in search)"
        );
    }
}

// ── Default week view ─────────────────────────────────────────────────────────

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    // Split area: main list | bottom strip (search bar OR date picker)
    // Phase Foster.1.4 — when the shared date picker is open with a
    // Schedule target, render it in the same 3-line bottom strip the
    // search bar uses. The two are mutually exclusive in practice
    // (search and date-jump aren't both active at once).
    let picker_active = app.date_picker.open
        && matches!(
            app.date_picker.target,
            crate::tui::app::PickerTarget::Schedule
        );
    let bottom_h: u16 =
        if app.schedule.search_mode || app.schedule.filter_err.is_some() || picker_active {
            3
        } else {
            0
        };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(bottom_h)])
        .split(area);

    render_week_block(f, app, chunks[0]);

    if bottom_h > 0 {
        if picker_active {
            crate::tui::screens::misc::render_scores_date_picker(f, app, chunks[1]);
        } else {
            render_search_bar(f, app, chunks[1]);
        }
    }
}

fn render_week_block(f: &mut Frame, app: &App, area: Rect) {
    let label = week_label(&app.schedule.week);
    let title = match &app.schedule.filter {
        SearchFilter::None => {
            format!(" Schedule · Week of {label}  ·  /:search  ←→:week  t:today ")
        }
        SearchFilter::Team(t) => {
            format!(" Schedule · {label} · filter: {t}  ·  Enter: full season ")
        }
        SearchFilter::Matchup(a, b) => {
            format!(" Schedule · {label} · filter: {a} vs {b}  ·  Enter: head-to-head ")
        }
    };
    let block = tui_panel_block(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Look up cache state for this week
    let state = {
        let map = app.schedule.week_cache.lock().unwrap();
        map.get(&app.schedule.week)
            .cloned()
            .unwrap_or(ScheduleState::Idle)
    };

    match state {
        ScheduleState::Idle => {
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(""),
                    Line::styled("  Loading schedule…", tui_meta_style()),
                ]),
                inner,
            );
        }
        ScheduleState::Loading => {
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(""),
                    Line::styled("  Fetching NHL schedule…", tui_title_style()),
                ]),
                inner,
            );
        }
        ScheduleState::Error(e) => {
            let dim = tui_meta_style();
            let red = tui_error_style();
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(""),
                    Line::styled(format!("  Schedule unavailable for week of {label}"), red),
                    Line::styled(format!("  ({e})"), dim),
                    Line::from(""),
                    Line::styled("  Press r to retry  ·  ←→ to navigate to other weeks", dim),
                ]),
                inner,
            );
        }
        ScheduleState::Loaded(games) => {
            let filtered: Vec<&ScheduledGame> = games
                .iter()
                .filter(|g| app.schedule.filter.matches(g))
                .collect();
            render_games_grouped(f, app, inner, &filtered);
        }
    }
}

fn render_games_grouped(f: &mut Frame, app: &App, area: Rect, games: &[&ScheduledGame]) {
    if games.is_empty() {
        let dim = tui_meta_style();
        let msg = match &app.schedule.filter {
            SearchFilter::None => "  No games scheduled this week.",
            SearchFilter::Team(_) => "  No games match this team filter for this week.",
            SearchFilter::Matchup(..) => "  No games match this matchup for this week.",
        };
        f.render_widget(
            Paragraph::new(vec![Line::from(""), Line::styled(msg, dim)]),
            area,
        );
        return;
    }

    let schedule_view = schedule_view_from_games(app, games.iter().map(|game| (**game).clone()));
    let rows: Vec<&ScheduleGameRow> = schedule_view.rows.iter().collect();

    let dim = tui_meta_style();
    let gold = tui_header_style();

    // Group by date (preserves API ordering)
    let mut items: Vec<ListItem> = Vec::new();
    let mut current_date = String::new();
    let max_idx = rows.len().saturating_sub(1);
    let selected_idx = app.schedule.selected.min(max_idx);

    for (row_idx, g) in rows.iter().enumerate() {
        if g.date != current_date {
            current_date = g.date.clone();
            if !items.is_empty() {
                items.push(ListItem::new(Line::from("")));
            }
            items.push(ListItem::new(Line::styled(
                format!("  {}", pretty_date(&g.date)),
                gold,
            )));
        }

        let utc = g.start_time_utc.get(11..16).unwrap_or("?");
        let et = fmt_et(utc);

        let result_or_time = if g.is_final() {
            let aw = g.away_score.unwrap_or(0);
            let hw = g.home_score.unwrap_or(0);
            let ot_tag = match g.last_period.as_deref() {
                Some("OT") => " OT",
                Some("SO") => " SO",
                _ => "",
            };
            format!("Final{ot_tag}  {aw}-{hw}")
        } else if g.is_live() {
            "LIVE".to_owned()
        } else {
            et.clone()
        };

        let series = if g.is_playoff() {
            g.series_label().unwrap_or_else(|| {
                format!(
                    "Playoffs · Game {}",
                    g.series_game.as_deref().unwrap_or("?")
                )
            })
        } else {
            String::new()
        };

        let line = format!(
            "  {:<10}  {:>3} @ {:<3}  {:<22}",
            result_or_time,
            g.away_abbrev,
            g.home_abbrev,
            if series.is_empty() {
                String::new()
            } else {
                series
            },
        );

        let style = if row_idx == selected_idx {
            tui_selected_style()
        } else if g.is_live() {
            tui_header_style()
        } else if g.is_final() {
            tui_meta_style()
        } else {
            tui_title_style()
        };
        items.push(ListItem::new(Line::styled(line, style)));
    }

    items.push(ListItem::new(Line::from("")));
    items.push(ListItem::new(Line::styled(
        format!("  {} game(s) shown · times in ET", rows.len()),
        dim,
    )));

    f.render_widget(List::new(items), area);
}

fn schedule_view_from_games(
    app: &App,
    games: impl IntoIterator<Item = ScheduledGame>,
) -> ScheduleView {
    let active_team = match &app.schedule.filter {
        SearchFilter::Team(team) => team.clone(),
        SearchFilter::None | SearchFilter::Matchup(..) => String::new(),
    };
    ScheduleView::from_games(
        ViewContext::new(ViewWindow::new(
            Season(app.active_season_typed.0),
            app.active_type,
        )),
        app.active_season.clone(),
        active_team,
        Some(app.schedule.week.clone()),
        &[],
        games.into_iter().map(scheduled_game_input).collect(),
    )
}

fn scheduled_game_input(game: ScheduledGame) -> ScheduledGameInput {
    ScheduledGameInput {
        game_id: game.game_id,
        date: game.date,
        game_type: game.game_type,
        away_abbrev: game.away_abbrev,
        away_name: game.away_name,
        home_abbrev: game.home_abbrev,
        home_name: game.home_name,
        start_time_utc: game.start_time_utc,
        away_score: game.away_score,
        home_score: game.home_score,
        game_state: game.game_state,
        last_period: game.last_period,
        series_game: game.series_game,
        away_wins: game.away_wins,
        home_wins: game.home_wins,
    }
}

fn render_search_bar(f: &mut Frame, app: &App, area: Rect) {
    let block = tui_panel_block(" Search ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let cursor = if app.schedule.search_mode { "█" } else { "" };
    let prompt = if let Some(err) = &app.schedule.filter_err {
        Line::styled(format!("  ⚠ {err}"), tui_error_style())
    } else if app.schedule.search_mode {
        Line::from(format!("  / {}{}", app.schedule.query, cursor))
    } else {
        Line::styled(
            "  Press / to filter by team (SEA) or matchup (NYR WSH)",
            tui_meta_style(),
        )
    };
    f.render_widget(Paragraph::new(prompt), inner);
}

// ── Team season performance ──────────────────────────────────────────────────

pub fn render_team_schedule(f: &mut Frame, app: &App, area: Rect, team: &str) {
    let title = format!(" {team} — Season Performance  ·  Esc back ");
    let block = tui_panel_block(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let state = {
        let map = app.schedule.team_cache.lock().unwrap();
        // Hart.5c.6 Phase C — D5 widened key.
        map.get(&(team.to_owned(), app.active_season.clone()))
            .cloned()
            .unwrap_or(ScheduleState::Idle)
    };

    match state {
        ScheduleState::Idle | ScheduleState::Loading => {
            let dim = tui_title_style();
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(""),
                    Line::styled(format!("  Fetching {team} season performance..."), dim),
                ]),
                inner,
            );
        }
        ScheduleState::Error(e) => {
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(""),
                    Line::styled(
                        format!("  Could not fetch team season performance: {e}"),
                        tui_error_style(),
                    ),
                ]),
                inner,
            );
        }
        ScheduleState::Loaded(games) => render_team_schedule_loaded(f, app, inner, team, &games),
    }
}

fn render_team_schedule_loaded(
    f: &mut Frame,
    app: &App,
    area: Rect,
    team: &str,
    games: &[ScheduledGame],
) {
    let dim = tui_meta_style();
    let gold = tui_header_style();

    let (standings, standings_error) = {
        let map = app.schedule.standings_cache.lock().unwrap();
        match map.get(&app.active_season) {
            Some(StandingsState::Loaded(rows)) => (
                rows.iter()
                    .map(|row| row.to_team_standing_input())
                    .collect(),
                None,
            ),
            Some(StandingsState::Error(e)) => (Vec::new(), Some(e.clone())),
            _ => (Vec::new(), None),
        }
    };
    // Team-season performance lives in the shared Presidents Trophy viewmodel.
    let view = TeamSeasonView::from_games_and_standings(
        ViewContext::new(ViewWindow::new(
            Season(app.active_season_typed.0),
            app.active_type,
        )),
        app.active_season.clone(),
        team.to_owned(),
        games.iter().cloned().map(scheduled_game_input).collect(),
        standings,
    );
    let rows = &view.rows;
    let record = view.headline.record;

    let max_idx = rows.len().saturating_sub(1);
    let visible = (area.height as usize).saturating_sub(8);
    let selected_idx = app.schedule.selected.min(max_idx);
    let offset = selected_idx
        .saturating_sub(visible / 2)
        .min(rows.len().saturating_sub(visible));

    let mut lines: Vec<Line> = Vec::with_capacity(visible + 4);
    lines.push(Line::styled(
        format!(
            "  Played: {} · Record: {} · Pts {} · Pts% {:.3} · GD {}",
            record.played,
            schedule_record_label(record),
            view.headline.points,
            view.headline.points_percentage,
            signed_i32(view.headline.goal_differential)
        ),
        gold,
    ));
    lines.push(Line::styled(
        format!(
            "  Home {} · Away {} · One-goal {} · Last 10 {} ({})",
            schedule_record_label(view.splits.home.record),
            schedule_record_label(view.splits.away.record),
            schedule_record_label(view.splits.one_goal.record),
            schedule_record_label(view.form.last_10),
            signed_i32(view.form.last_10_goal_differential)
        ),
        dim,
    ));
    let next = if view.remaining.next_opponents.is_empty() {
        "-".to_owned()
    } else {
        view.remaining.next_opponents.join(", ")
    };
    lines.push(Line::styled(
        format!(
            "  Remaining: {} games ({} home, {} away) · Next: {}",
            view.remaining.games, view.remaining.home, view.remaining.away, next
        ),
        dim,
    ));
    lines.push(Line::styled(
        format!(
            "  SOS: faced {} · rem {} · Ledger: QW {} · EW {} · bad L {} · missed {}",
            pct_or_dash(view.schedule_strength.faced_average_points_percentage),
            pct_or_dash(view.schedule_strength.remaining_average_points_percentage),
            view.quality_ledger.quality_wins,
            view.quality_ledger.expected_wins,
            view.quality_ledger.bad_losses,
            view.quality_ledger.missed_points
        ),
        dim,
    ));
    lines.push(Line::styled(
        format!("  Records: :records team {team} · /records/team/{team}"),
        dim,
    ));
    if let Some(warning) = view.warnings.first() {
        lines.push(Line::styled(format!("  Warning: {}", warning.message), dim));
    }
    if let Some(error) = standings_error {
        lines.push(Line::styled(
            format!("  Standings unavailable: {error}"),
            tui_error_style(),
        ));
    }
    lines.push(Line::styled(format!("  {}", "─".repeat(60)), dim));

    for (i, g) in rows.iter().enumerate().skip(offset).take(visible) {
        let (row, color) = team_season_line(team, g);

        let style = if i == selected_idx {
            tui_selected_style()
        } else {
            Style::default().fg(color)
        };
        lines.push(Line::styled(row, style));
    }

    lines.push(Line::from(""));
    lines.push(Line::styled(
        format!("  {} games total · ↑↓ scroll · Esc back", view.rows.len()),
        dim,
    ));
    f.render_widget(Paragraph::new(lines), area);
}

fn team_season_line(team: &str, row: &TeamSeasonGameRow) -> (String, Color) {
    let venue = match row.venue {
        TeamSeasonVenue::Home => "H",
        TeamSeasonVenue::Away => "A",
    };
    let score = match (row.team_score, row.opponent_score) {
        (Some(team_score), Some(opponent_score)) => format!("{team_score}-{opponent_score}"),
        _ => "-".to_owned(),
    };
    let gd = row
        .goal_differential
        .map(signed_i16)
        .unwrap_or_else(|| "-".to_owned());
    let color = match row.result.as_str() {
        "W" => Color::Green,
        "OTL" => Color::Yellow,
        "L" => Color::Red,
        "LIVE" => Color::Cyan,
        _ => Color::DarkGray,
    };
    (
        format!(
            "  {result:<3} {date:<10} {team:<3} {venue:<1} {opp:<3}  {score:<5} {gd:>3}  {state}",
            result = row.result,
            date = pretty_date(&row.date),
            opp = row.opponent_abbrev,
            state = row.state_label,
        ),
        color,
    )
}

fn schedule_record_label(record: ScheduleRecord) -> String {
    format!(
        "{}-{}-{}",
        record.wins, record.losses, record.overtime_losses
    )
}

fn signed_i32(value: i32) -> String {
    format!("{value:+}")
}

fn signed_i16(value: i16) -> String {
    format!("{value:+}")
}

fn pct_or_dash(value: Option<f32>) -> String {
    value
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "-".to_owned())
}

// ── Head-to-head matchup ──────────────────────────────────────────────────────

pub fn render_matchup(f: &mut Frame, app: &App, area: Rect, t1: &str, t2: &str) {
    let title = format!(" {t1} vs {t2} — Season Series  ·  Esc back ");
    let block = tui_panel_block(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let state = {
        let map = app.schedule.team_cache.lock().unwrap();
        // Hart.5c.6 Phase C — D5 widened key.
        map.get(&(t1.to_owned(), app.active_season.clone()))
            .cloned()
            .unwrap_or(ScheduleState::Idle)
    };

    match state {
        ScheduleState::Idle | ScheduleState::Loading => {
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(""),
                    Line::styled(format!("  Fetching {t1} schedule…"), tui_title_style()),
                ]),
                inner,
            );
        }
        ScheduleState::Error(e) => {
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(""),
                    Line::styled(format!("  Could not fetch matchup: {e}"), tui_error_style()),
                ]),
                inner,
            );
        }
        ScheduleState::Loaded(games) => render_matchup_loaded(f, app, inner, t1, t2, &games),
    }
}

fn render_matchup_loaded(
    f: &mut Frame,
    app: &App,
    area: Rect,
    t1: &str,
    t2: &str,
    all: &[ScheduledGame],
) {
    let dim = tui_meta_style();
    let gold = tui_header_style();

    let view = ScheduleMatchupView::from_games(
        ViewContext::new(ViewWindow::new(
            Season(app.active_season_typed.0),
            app.active_type,
        )),
        app.active_season.clone(),
        t1.to_owned(),
        t2.to_owned(),
        all.iter().cloned().map(scheduled_game_input).collect(),
    );

    if view.rows.is_empty() {
        f.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::styled(format!("  No {t1} vs {t2} games in this season."), dim),
            ]),
            area,
        );
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::styled(
        format!(
            "  Regular season: {t1} {}-{} {t2}     ·     Playoffs: {t1} {}-{} {t2}",
            view.regular_record.wins,
            view.regular_record.losses,
            view.playoff_record.wins,
            view.playoff_record.losses
        ),
        gold,
    ));
    lines.push(Line::styled(format!("  {}", "─".repeat(64)), dim));

    if !view.regular_rows.is_empty() {
        lines.push(Line::styled("  Regular Season", gold));
        for g in &view.regular_rows {
            lines.push(matchup_row(g, t1));
        }
        lines.push(Line::from(""));
    }
    if !view.playoff_rows.is_empty() {
        lines.push(Line::styled("  Playoffs", gold));
        for g in &view.playoff_rows {
            lines.push(matchup_row(g, t1));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::styled(
        format!("  {} matchup(s) · Esc back", view.total),
        dim,
    ));
    f.render_widget(Paragraph::new(lines), area);
}

fn matchup_row(g: &ScheduleGameRow, t1: &str) -> Line<'static> {
    let date = pretty_date(&g.date);
    let body = if g.is_final() {
        let aw = g.away_score.unwrap_or(0);
        let hw = g.home_score.unwrap_or(0);
        let ot = match g.last_period.as_deref() {
            Some("OT") => " (OT)",
            Some("SO") => " (SO)",
            _ => "",
        };
        let series = if g.is_playoff() {
            g.series_game
                .as_deref()
                .map(|gl| format!("  {gl}"))
                .unwrap_or_default()
        } else {
            String::new()
        };
        format!(
            "  {date}  {} {aw}-{hw} {}{ot} Final{series}",
            g.away_abbrev, g.home_abbrev
        )
    } else {
        let utc = g.start_time_utc.get(11..16).unwrap_or("?");
        let series = if g.is_playoff() {
            g.series_game
                .as_deref()
                .map(|gl| format!("  ({gl})"))
                .unwrap_or_default()
        } else {
            String::new()
        };
        format!(
            "  {date}  {} @ {}  {}{series}",
            g.away_abbrev,
            g.home_abbrev,
            fmt_et(utc)
        )
    };

    let style = if g.is_final() {
        // Bold the winning team's name in the matchup, color from t1's perspective
        let t1_score = g.team_score(t1).unwrap_or(0);
        let t2_score = g.opponent_score(t1).unwrap_or(0);
        if t1_score > t2_score {
            tui_header_style()
        } else {
            tui_error_style()
        }
    } else {
        tui_meta_style()
    };

    Line::styled(body, style)
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn pretty_date(iso: &str) -> String {
    // "YYYY-MM-DD" → "Mon Apr 28"
    use chrono::NaiveDate;
    if let Ok(d) = NaiveDate::parse_from_str(iso, "%Y-%m-%d") {
        d.format("%a %b %-d").to_string()
    } else {
        iso.to_owned()
    }
}

fn fmt_et(utc_hhmm: &str) -> String {
    let parts: Vec<&str> = utc_hhmm.splitn(2, ':').collect();
    if let [h, m] = parts.as_slice() {
        if let (Ok(h), Ok(m)) = (h.parse::<u32>(), m.parse::<u32>()) {
            let et_h = (h + 24 - 4) % 24;
            let period = if et_h < 12 { "AM" } else { "PM" };
            let display = match et_h % 12 {
                0 => 12,
                n => n,
            };
            return format!("{display}:{m:02} {period}");
        }
    }
    format!("{utc_hhmm} UTC")
}

#[cfg(test)]
mod tests {
    //! L0 render tests — drive the renderers against a `TestBackend` buffer
    //! and assert that key text appears at the expected screen positions.

    use super::*;
    use crate::tui::schedule::{ScheduleState, SearchFilter};
    use icelines_fetch::nhl_api::{NhlStandingsRow, ScheduledGame};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[allow(clippy::too_many_arguments)] // synthetic test fixture; struct would be noisier.
    fn fixture_game(
        away: &str,
        home: &str,
        date: &str,
        state: Option<&str>,
        scores: Option<(u8, u8)>,
        last_period: Option<&str>,
        playoff: bool,
        series: Option<(&str, u8, u8)>,
    ) -> ScheduledGame {
        ScheduledGame {
            game_id: 1,
            date: date.to_owned(),
            game_type: if playoff { 3 } else { 2 },
            away_abbrev: away.to_owned(),
            away_name: away.to_owned(),
            home_abbrev: home.to_owned(),
            home_name: home.to_owned(),
            start_time_utc: format!("{date}T23:00:00Z"),
            away_score: scores.map(|s| s.0),
            home_score: scores.map(|s| s.1),
            game_state: state.map(str::to_owned),
            last_period: last_period.map(str::to_owned),
            series_game: series.map(|s| s.0.to_owned()),
            away_wins: series.map(|s| s.1),
            home_wins: series.map(|s| s.2),
        }
    }

    fn standings_row(team: &str, points_percentage: f32) -> NhlStandingsRow {
        NhlStandingsRow {
            team: team.to_owned(),
            conference: Some("Western".to_owned()),
            division: Some("Pacific".to_owned()),
            games_played: 40,
            wins: 20,
            losses: 15,
            overtime_losses: 5,
            points: (points_percentage * 80.0).round() as u32,
            points_percentage,
            regulation_wins: Some(18),
            goal_differential: 0,
            league_rank: None,
            conference_rank: None,
            division_rank: None,
            wild_card_rank: None,
        }
    }

    fn buffer_text(buf: &ratatui::buffer::Buffer) -> String {
        let mut out = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    /// Render the default Schedule screen and return the flattened buffer text.
    fn render_to_text(app: &App) -> String {
        let backend = TestBackend::new(120, 30);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| {
            let area = f.area();
            super::render(f, app, area);
        })
        .unwrap();
        buffer_text(term.backend().buffer())
    }

    #[test]
    fn l0_render_schedule_idle_shows_loading_message() {
        let mut app = App::new(false);
        app.screen = crate::tui::app::Screen::Schedule;
        // Cache empty → state is Idle → "Loading schedule…"
        let text = render_to_text(&app);
        assert!(
            text.contains("Loading schedule"),
            "expected loading message, got:\n{text}"
        );
    }

    #[test]
    fn l0_render_schedule_loaded_shows_dates_and_teams() {
        let mut app = App::new(false);
        app.screen = crate::tui::app::Screen::Schedule;
        let games = vec![
            fixture_game(
                "SEA",
                "VGK",
                "2026-04-27",
                Some("FINAL"),
                Some((3, 2)),
                Some("OT"),
                false,
                None,
            ),
            fixture_game(
                "NYR",
                "WSH",
                "2026-04-28",
                Some("FUT"),
                None,
                None,
                true,
                Some(("Game 5", 2, 2)),
            ),
        ];
        app.schedule
            .week_cache
            .lock()
            .unwrap()
            .insert(app.schedule.week.clone(), ScheduleState::Loaded(games));

        let text = render_to_text(&app);
        // Team abbrevs from both games must appear
        assert!(text.contains("SEA"), "SEA must appear, got:\n{text}");
        assert!(text.contains("VGK"), "VGK must appear");
        assert!(text.contains("NYR"), "NYR must appear");
        assert!(text.contains("WSH"), "WSH must appear");
        // The OT final marker
        assert!(text.contains("Final"), "Final marker must appear");
        // The playoff series label
        assert!(text.contains("Game 5"), "Series label must appear");
    }

    #[test]
    fn l0_render_schedule_with_team_filter_shows_filter_label() {
        let mut app = App::new(false);
        app.screen = crate::tui::app::Screen::Schedule;
        app.schedule.filter = SearchFilter::Team("SEA".to_owned());
        let games = vec![
            fixture_game(
                "SEA",
                "VGK",
                "2026-04-27",
                Some("FINAL"),
                Some((3, 2)),
                Some("OT"),
                false,
                None,
            ),
            fixture_game(
                "NYR",
                "WSH",
                "2026-04-27",
                Some("FINAL"),
                Some((1, 4)),
                Some("REG"),
                false,
                None,
            ),
        ];
        app.schedule
            .week_cache
            .lock()
            .unwrap()
            .insert(app.schedule.week.clone(), ScheduleState::Loaded(games));

        let text = render_to_text(&app);
        // Title bar advertises the filter
        assert!(
            text.contains("filter: SEA"),
            "title must show filter, got:\n{text}"
        );
        // SEA game must appear; NYR/WSH must NOT
        assert!(text.contains("SEA"), "SEA game must render");
        assert!(!text.contains("NYR"), "NYR must be filtered out");
        assert!(!text.contains("WSH"), "WSH must be filtered out");
    }

    #[test]
    fn l0_render_schedule_search_bar_visible_when_active() {
        let mut app = App::new(false);
        app.screen = crate::tui::app::Screen::Schedule;
        app.schedule.search_mode = true;
        app.schedule.query = "NY".to_owned();
        // Don't seed the cache — we only care about the search bar
        let text = render_to_text(&app);
        // The search prompt prefix
        assert!(
            text.contains("/ NY"),
            "search bar must render the typed query, got:\n{text}"
        );
    }

    #[test]
    fn l0_render_schedule_search_bar_shows_validation_error() {
        let mut app = App::new(false);
        app.screen = crate::tui::app::Screen::Schedule;
        app.schedule.filter_err = Some("Unknown team: 'XYZ'".to_owned());
        let text = render_to_text(&app);
        assert!(
            text.contains("Unknown team"),
            "validation error must surface, got:\n{text}"
        );
    }

    #[test]
    fn l0_render_schedule_loaded_empty_shows_no_games_message() {
        let mut app = App::new(false);
        app.screen = crate::tui::app::Screen::Schedule;
        app.schedule
            .week_cache
            .lock()
            .unwrap()
            .insert(app.schedule.week.clone(), ScheduleState::Loaded(Vec::new()));
        let text = render_to_text(&app);
        assert!(
            text.contains("No games"),
            "empty week must show 'No games' message, got:\n{text}"
        );
    }

    #[test]
    fn l0_render_schedule_error_state_shows_retry_hint() {
        let mut app = App::new(false);
        app.screen = crate::tui::app::Screen::Schedule;
        app.schedule.week_cache.lock().unwrap().insert(
            app.schedule.week.clone(),
            ScheduleState::Error("network down".to_owned()),
        );
        let text = render_to_text(&app);
        assert!(
            text.contains("unavailable"),
            "error must show 'unavailable', got:\n{text}"
        );
        assert!(
            text.contains("retry") || text.contains("r to retry"),
            "error must hint at retry, got:\n{text}"
        );
    }

    /// Render team-season detail screen and inspect the buffer.
    fn render_team_to_text(app: &App, team: &str) -> String {
        let backend = TestBackend::new(120, 30);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| {
            let area = f.area();
            super::render_team_schedule(f, app, area, team);
        })
        .unwrap();
        buffer_text(term.backend().buffer())
    }

    #[test]
    fn l0_render_team_schedule_shows_record_header() {
        let mut app = App::new(false);
        app.screen = crate::tui::app::Screen::ScheduleTeam("SEA".to_owned());

        // 2 wins, 1 OT loss for SEA
        let games = vec![
            // SEA home win 4-2 over CGY (REG)
            fixture_game(
                "CGY",
                "SEA",
                "2026-01-15",
                Some("FINAL"),
                Some((2, 4)),
                Some("REG"),
                false,
                None,
            ),
            // SEA away loss 3-4 to EDM (SO → OT loss column)
            fixture_game(
                "SEA",
                "EDM",
                "2026-02-03",
                Some("FINAL"),
                Some((3, 4)),
                Some("SO"),
                false,
                None,
            ),
            // SEA home win 5-1 over VAN
            fixture_game(
                "VAN",
                "SEA",
                "2026-02-10",
                Some("FINAL"),
                Some((1, 5)),
                Some("REG"),
                false,
                None,
            ),
        ];
        app.schedule.team_cache.lock().unwrap().insert(
            ("SEA".to_owned(), app.active_season.clone()),
            ScheduleState::Loaded(games),
        );
        app.schedule.standings_cache.lock().unwrap().insert(
            app.active_season.clone(),
            StandingsState::Loaded(vec![
                standings_row("EDM", 0.720),
                standings_row("VAN", 0.650),
                standings_row("SEA", 0.600),
                standings_row("CGY", 0.540),
            ]),
        );

        let text = render_team_to_text(&app, "SEA");
        // Title shows the team
        assert!(text.contains("SEA"), "team name must appear");
        // Record line: 2-0-1
        assert!(
            text.contains("2-0-1") || text.contains("Record: 2-0-1"),
            "record must show 2-0-1, got:\n{text}"
        );
        // Played count
        assert!(
            text.contains("Played: 3"),
            "played count must show 3, got:\n{text}"
        );
        assert!(
            text.contains("Remaining:"),
            "team season performance context must show remaining schedule, got:\n{text}"
        );
        assert!(
            text.contains("One-goal"),
            "team season performance context must show split labels, got:\n{text}"
        );
        assert!(
            text.contains("SOS:"),
            "team season performance context must show schedule strength, got:\n{text}"
        );
        assert!(
            text.contains("0."),
            "standings-backed schedule strength should show numeric opponent Pts%, got:\n{text}"
        );
        assert!(
            text.contains("Records:") && text.contains("/records/team/SEA"),
            "team season screen must expose team records entry point, got:\n{text}"
        );
    }

    #[test]
    fn l0_render_team_schedule_idle_shows_fetching_message() {
        let app = App::new(false);
        let text = render_team_to_text(&app, "SEA");
        assert!(
            text.contains("Fetching"),
            "idle team view must show fetching, got:\n{text}"
        );
    }

    /// Render matchup screen and inspect buffer.
    fn render_matchup_to_text(app: &App, t1: &str, t2: &str) -> String {
        let backend = TestBackend::new(120, 30);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| {
            let area = f.area();
            super::render_matchup(f, app, area, t1, t2);
        })
        .unwrap();
        buffer_text(term.backend().buffer())
    }

    #[test]
    fn l0_render_matchup_shows_record_header() {
        let app = App::new(false);
        // T1 = NYR. Cache holds NYR's full season; matchup filters to NYR vs WSH games.
        let games = vec![
            // NYR 4-1 over WSH (regular) → t1 win
            fixture_game(
                "WSH",
                "NYR",
                "2025-11-15",
                Some("FINAL"),
                Some((1, 4)),
                Some("REG"),
                false,
                None,
            ),
            // WSH 3-2 over NYR OT (regular) → t1 loss
            fixture_game(
                "NYR",
                "WSH",
                "2026-01-05",
                Some("FINAL"),
                Some((2, 3)),
                Some("OT"),
                false,
                None,
            ),
            // Playoff: NYR 5-2 over WSH → t1 playoff win
            fixture_game(
                "NYR",
                "WSH",
                "2026-04-24",
                Some("FINAL"),
                Some((5, 2)),
                Some("REG"),
                true,
                Some(("Game 3", 1, 2)),
            ),
            // Distractor: SEA @ VGK shouldn't appear
            fixture_game(
                "SEA",
                "VGK",
                "2026-02-01",
                Some("FINAL"),
                Some((3, 2)),
                None,
                false,
                None,
            ),
        ];
        app.schedule.team_cache.lock().unwrap().insert(
            ("NYR".to_owned(), app.active_season.clone()),
            ScheduleState::Loaded(games),
        );

        let text = render_matchup_to_text(&app, "NYR", "WSH");

        // Header has both team abbrevs
        assert!(
            text.contains("NYR") && text.contains("WSH"),
            "both team abbrevs must appear"
        );
        // Section labels
        assert!(
            text.contains("Regular Season"),
            "regular section header missing"
        );
        assert!(text.contains("Playoffs"), "playoffs section header missing");
        // Record from t1 (NYR) perspective: 1-1 regular, 1-0 playoffs
        assert!(
            text.contains("NYR 1-1 WSH"),
            "regular-season record line missing, got:\n{text}"
        );
        assert!(
            text.contains("NYR 1-0 WSH"),
            "playoffs record line missing, got:\n{text}"
        );
        // Distractor must NOT appear (filtered to NYR vs WSH)
        assert!(
            !text.contains("VGK"),
            "distractor game must be filtered out"
        );
    }

    #[test]
    fn l0_render_matchup_no_games_shows_message() {
        let app = App::new(false);
        // Cache for NYR has only games against teams other than WSH
        let games = vec![fixture_game(
            "EDM",
            "NYR",
            "2025-11-01",
            Some("FINAL"),
            Some((1, 3)),
            Some("REG"),
            false,
            None,
        )];
        app.schedule.team_cache.lock().unwrap().insert(
            ("NYR".to_owned(), app.active_season.clone()),
            ScheduleState::Loaded(games),
        );

        let text = render_matchup_to_text(&app, "NYR", "WSH");
        assert!(
            text.contains("No NYR vs WSH"),
            "must show no-games message, got:\n{text}"
        );
    }
}
