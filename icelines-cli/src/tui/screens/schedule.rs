//! Schedule tab renderer — week view, team-season view, head-to-head matchup view.

// Phase Norris.2 — `ScheduleScreenState` repeats the module name
// in the type identifier. Same canonical pattern as Norris.1's
// QueriesState — file-level allow keeps the lint quiet without
// renaming each per-screen struct to `State` (which would lose
// cross-module readability when imported into app.rs).
#![allow(clippy::module_name_repetitions)]

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::tui::app::App;
use crate::tui::schedule::{
    new_team_cache, new_week_cache, today_iso, week_label, monday_of, ScheduleState, SearchFilter,
    TeamSeasonCache, WeekCache,
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

impl Default for ScheduleScreenState {
    fn default() -> Self {
        Self {
            week_cache: new_week_cache(),
            team_cache: new_team_cache(),
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

// ── Default week view ─────────────────────────────────────────────────────────

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    // Split area: main list | bottom strip (search bar OR date picker)
    // Phase Foster.1.4 — when the shared date picker is open with a
    // Schedule target, render it in the same 3-line bottom strip the
    // search bar uses. The two are mutually exclusive in practice
    // (search and date-jump aren't both active at once).
    let picker_active = app.scores_picker_open
        && matches!(app.picker_target, crate::tui::app::PickerTarget::Schedule);
    let bottom_h: u16 = if app.schedule.search_mode
        || app.schedule.filter_err.is_some()
        || picker_active
    {
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
    let block = Block::default().borders(Borders::ALL).title(title);
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
                    Line::styled("  Loading schedule…", Style::default().fg(Color::DarkGray)),
                ]),
                inner,
            );
        }
        ScheduleState::Loading => {
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(""),
                    Line::styled("  Fetching NHL schedule…", Style::default().fg(Color::Cyan)),
                ]),
                inner,
            );
        }
        ScheduleState::Error(e) => {
            let dim = Style::default().fg(Color::DarkGray);
            let red = Style::default().fg(Color::Red);
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
        let dim = Style::default().fg(Color::DarkGray);
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

    let dim = Style::default().fg(Color::DarkGray);
    let gold = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);

    // Group by date (preserves API ordering)
    let mut items: Vec<ListItem> = Vec::new();
    let mut current_date = String::new();
    let max_idx = games.len().saturating_sub(1);
    let selected_idx = app.schedule.selected.min(max_idx);

    for (row_idx, g) in games.iter().enumerate() {
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
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else if g.is_live() {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else if g.is_final() {
            Style::default()
        } else {
            Style::default().fg(Color::White)
        };
        items.push(ListItem::new(Line::styled(line, style)));
    }

    items.push(ListItem::new(Line::from("")));
    items.push(ListItem::new(Line::styled(
        format!("  {} game(s) shown · times in ET", games.len()),
        dim,
    )));

    f.render_widget(List::new(items), area);
}

fn render_search_bar(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Search ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let cursor = if app.schedule.search_mode { "█" } else { "" };
    let prompt = if let Some(err) = &app.schedule.filter_err {
        Line::styled(format!("  ⚠ {err}"), Style::default().fg(Color::Red))
    } else if app.schedule.search_mode {
        Line::from(format!("  / {}{}", app.schedule.query, cursor))
    } else {
        Line::styled(
            "  Press / to filter by team (SEA) or matchup (NYR WSH)",
            Style::default().fg(Color::DarkGray),
        )
    };
    f.render_widget(Paragraph::new(prompt), inner);
}

// ── Team season schedule ─────────────────────────────────────────────────────

pub fn render_team_schedule(f: &mut Frame, app: &App, area: Rect, team: &str) {
    let title = format!(" {team} — Season Schedule  ·  Esc back ");
    let block = Block::default().borders(Borders::ALL).title(title);
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
            let dim = Style::default().fg(Color::Cyan);
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(""),
                    Line::styled(format!("  Fetching {team} schedule…"), dim),
                ]),
                inner,
            );
        }
        ScheduleState::Error(e) => {
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(""),
                    Line::styled(
                        format!("  Could not fetch team schedule: {e}"),
                        Style::default().fg(Color::Red),
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
    let dim = Style::default().fg(Color::DarkGray);
    let gold = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);

    // Compute record (W-L-OT) over completed games — exclude preseason (game_type=1)
    let mut wins = 0u32;
    let mut losses = 0u32;
    let mut ot_l = 0u32;
    for g in games.iter().filter(|g| g.game_type != 1 && g.is_final()) {
        let team_is_away = g.away_abbrev == team;
        let (team_score, opp_score) = if team_is_away {
            (g.away_score.unwrap_or(0), g.home_score.unwrap_or(0))
        } else {
            (g.home_score.unwrap_or(0), g.away_score.unwrap_or(0))
        };
        if team_score > opp_score {
            wins += 1;
        } else if matches!(g.last_period.as_deref(), Some("OT") | Some("SO")) {
            ot_l += 1;
        } else {
            losses += 1;
        }
    }
    let played = wins + losses + ot_l;

    let max_idx = games.len().saturating_sub(1);
    let visible = (area.height as usize).saturating_sub(4);
    let selected_idx = app.schedule.selected.min(max_idx);
    let offset = selected_idx
        .saturating_sub(visible / 2)
        .min(games.len().saturating_sub(visible));

    let mut lines: Vec<Line> = Vec::with_capacity(visible + 4);
    lines.push(Line::styled(
        format!("  Played: {played} · Record: {wins}-{losses}-{ot_l}"),
        gold,
    ));
    lines.push(Line::styled(format!("  {}", "─".repeat(60)), dim));

    for (i, g) in games.iter().enumerate().skip(offset).take(visible) {
        let team_is_away = g.away_abbrev == team;
        let opp = if team_is_away {
            &g.home_abbrev
        } else {
            &g.away_abbrev
        };
        let venue = if team_is_away { "@" } else { "vs" };

        let (marker, result_str, color) = if g.is_final() {
            let (s, o) = if team_is_away {
                (g.away_score.unwrap_or(0), g.home_score.unwrap_or(0))
            } else {
                (g.home_score.unwrap_or(0), g.away_score.unwrap_or(0))
            };
            let ot_tag = match g.last_period.as_deref() {
                Some("OT") => " (OT)",
                Some("SO") => " (SO)",
                _ => "",
            };
            if s > o {
                ("✓", format!("{s}-{o}{ot_tag}"), Color::Green)
            } else if matches!(g.last_period.as_deref(), Some("OT") | Some("SO")) {
                ("○", format!("{s}-{o}{ot_tag}"), Color::Yellow)
            } else {
                ("✗", format!("{s}-{o}{ot_tag}"), Color::Red)
            }
        } else if g.is_live() {
            ("◉", "LIVE".to_owned(), Color::Cyan)
        } else {
            let utc = g.start_time_utc.get(11..16).unwrap_or("?");
            ("○", fmt_et(utc), Color::DarkGray)
        };

        let row = format!(
            "  {marker} {date}  {team:<3} {venue:<2} {opp:<3}   {result_str}",
            date = pretty_date(&g.date),
        );

        let style = if i == selected_idx {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(color)
        };
        lines.push(Line::styled(row, style));
    }

    lines.push(Line::from(""));
    lines.push(Line::styled(
        format!("  {} games total · ↑↓ scroll · Esc back", games.len()),
        dim,
    ));
    f.render_widget(Paragraph::new(lines), area);
}

// ── Head-to-head matchup ──────────────────────────────────────────────────────

pub fn render_matchup(f: &mut Frame, app: &App, area: Rect, t1: &str, t2: &str) {
    let title = format!(" {t1} vs {t2} — Season Series  ·  Esc back ");
    let block = Block::default().borders(Borders::ALL).title(title);
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
                    Line::styled(
                        format!("  Fetching {t1} schedule…"),
                        Style::default().fg(Color::Cyan),
                    ),
                ]),
                inner,
            );
        }
        ScheduleState::Error(e) => {
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(""),
                    Line::styled(
                        format!("  Could not fetch matchup: {e}"),
                        Style::default().fg(Color::Red),
                    ),
                ]),
                inner,
            );
        }
        ScheduleState::Loaded(games) => render_matchup_loaded(f, inner, t1, t2, &games),
    }
}

fn render_matchup_loaded(f: &mut Frame, area: Rect, t1: &str, t2: &str, all: &[ScheduledGame]) {
    let dim = Style::default().fg(Color::DarkGray);
    let gold = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);

    // Filter to games involving both teams
    let relevant: Vec<&ScheduledGame> = all
        .iter()
        .filter(|g| g.involves(t1) && g.involves(t2))
        .collect();

    if relevant.is_empty() {
        f.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::styled(format!("  No {t1} vs {t2} games in this season."), dim),
            ]),
            area,
        );
        return;
    }

    // Tally regular season + playoff records (from t1 perspective)
    let mut t1_reg_w = 0u32;
    let mut t1_reg_l = 0u32;
    let mut t1_po_w = 0u32;
    let mut t1_po_l = 0u32;
    for g in &relevant {
        if !g.is_final() {
            continue;
        }
        let t1_is_away = g.away_abbrev == t1;
        let (t1_score, t2_score) = if t1_is_away {
            (g.away_score.unwrap_or(0), g.home_score.unwrap_or(0))
        } else {
            (g.home_score.unwrap_or(0), g.away_score.unwrap_or(0))
        };
        let t1_won = t1_score > t2_score;
        match (g.is_playoff(), t1_won) {
            (true, true) => t1_po_w += 1,
            (true, false) => t1_po_l += 1,
            (false, true) => t1_reg_w += 1,
            (false, false) => t1_reg_l += 1,
        }
    }

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::styled(
        format!(
            "  Regular season: {t1} {t1_reg_w}-{t1_reg_l} {t2}     ·     Playoffs: {t1} {t1_po_w}-{t1_po_l} {t2}"
        ),
        gold,
    ));
    lines.push(Line::styled(format!("  {}", "─".repeat(64)), dim));

    let regular: Vec<&ScheduledGame> = relevant
        .iter()
        .copied()
        .filter(|g| !g.is_playoff())
        .collect();
    let playoffs: Vec<&ScheduledGame> = relevant
        .iter()
        .copied()
        .filter(|g| g.is_playoff())
        .collect();

    if !regular.is_empty() {
        lines.push(Line::styled("  Regular Season", gold));
        for g in &regular {
            lines.push(matchup_row(g, t1));
        }
        lines.push(Line::from(""));
    }
    if !playoffs.is_empty() {
        lines.push(Line::styled("  Playoffs", gold));
        for g in &playoffs {
            lines.push(matchup_row(g, t1));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::styled(
        format!("  {} matchup(s) · Esc back", relevant.len()),
        dim,
    ));
    f.render_widget(Paragraph::new(lines), area);
}

fn matchup_row(g: &ScheduledGame, t1: &str) -> Line<'static> {
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
        let t1_is_away = g.away_abbrev == t1;
        let (t1_score, t2_score) = if t1_is_away {
            (g.away_score.unwrap_or(0), g.home_score.unwrap_or(0))
        } else {
            (g.home_score.unwrap_or(0), g.away_score.unwrap_or(0))
        };
        if t1_score > t2_score {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Red)
        }
    } else {
        Style::default()
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
    use icelines_fetch::nhl_api::ScheduledGame;
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
        app.schedule.week_cache
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
        app.schedule.week_cache
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
        app.schedule.week_cache
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
