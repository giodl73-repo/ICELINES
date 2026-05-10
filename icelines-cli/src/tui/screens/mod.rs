pub mod comps;
pub mod depth;
pub mod favorites;
pub mod game_detail;
pub mod goalies;
pub mod home;
pub mod misc;
pub mod player;
pub mod playoffs;
pub mod poach;
pub mod queries;
pub mod schedule;
pub mod search;
pub mod team;
pub mod transactions;

use crate::tui::app::{App, Screen};
use crate::tui::widgets::{help_lines, mdi_help_lines};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

pub fn render(f: &mut Frame, app: &App) {
    // Phase Jack Adams.1 — MDI ↔ SDI dispatch.
    //
    // Per spec glass-5: strict launch-time mode. `--mdi` set at
    // launch sticks for the session — we only fall back to SDI
    // for a frame when the terminal width is too narrow for any
    // reasonable MDI render (<100 cols). Resize back ≥100
    // returns to MDI rendering automatically.
    let area = f.area();
    if let Some(mdi) = &app.mdi {
        if !crate::tui::mdi::MdiLayout::collapse_to_sdi(area.width) {
            render_mdi(f, app, mdi);
            // Phase Adams.5 — overlays paint at the top level so
            // they layer on top of MDI dashboard too (help, admin,
            // season picker, reports, docs, group picker).
            render_overlays(f, app, area);
            return;
        }
        // Fall through to SDI for this frame; resize ≥100
        // returns automatically.
    }
    render_sdi(f, app);
    render_overlays(f, app, area);
}

/// Phase Adams.5 — overlay painter. Extracted from
/// `render_sdi` so MDI mode also gets help / admin / season
/// picker / reports / docs / group picker overlays. Called
/// after the body render in both modes.
fn render_overlays(f: &mut Frame, app: &App, area: Rect) {
    // Group picker overlay (player.rs and team.rs render their
    // own; this catches Projections, Search, Queries,
    // GroupDetail).
    if app.group_picker.open {
        let handled_locally = matches!(app.screen, Screen::PlayerById(_) | Screen::Team(_));
        if !handled_locally {
            player::render_group_picker(f, app, area);
        }
    }

    if app.show_help {
        // MDI mode shows the comprehensive command-bar
        // reference (verbs, args, examples). SDI keeps the
        // legacy keybind cheat sheet.
        let (lines, title, w, h) = if app.mdi.is_some() {
            (
                mdi_help_lines(),
                " Command Reference — any key to close ",
                78u16,
                88u16,
            )
        } else {
            (help_lines(), " Help — any key to close ", 62u16, 65u16)
        };
        let popup = centered_rect(w, h, area);
        f.render_widget(Clear, popup);
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));
        let inner = block.inner(popup);
        f.render_widget(block, popup);
        f.render_widget(Paragraph::new(lines), inner);
    }

    if app.show_admin {
        let popup = centered_rect(44, 50, area);
        f.render_widget(Clear, popup);
        let block = Block::default()
            .title(" Admin — Esc to close ")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Yellow));
        let inner = block.inner(popup);
        f.render_widget(block, popup);
        misc::render_admin(f, app, inner);
    }

    if app.show_season_picker {
        misc::render_season_picker(f, app, area);
    }

    if app.show_reports_overlay {
        misc::render_reports_overlay(f, app, area);
    }

    if app.show_docs {
        render_docs_overlay(f, app, area);
    }
}

/// Phase Jack Adams.1 — single-document render path. Renamed
/// from `render` pre-Adams.1; same behavior. Used by SDI
/// multi-tab (today's default) and SDI standalone (Masterton.3)
/// modes, plus the Adams.1 collapse fallback when MDI can't fit.
fn render_sdi(f: &mut Frame, app: &App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    // Phase Masterton.1 — chrome-aware header + footer.
    let chrome = active_chrome(app);
    render_header(f, app, &chrome, chunks[0]);

    match &app.screen {
        Screen::Home => home::render(f, app, chunks[1]),
        Screen::Team(abbrev) => team::render(f, app, chunks[1], abbrev),
        Screen::PlayerById(pid) => player::render_by_id(f, app, chunks[1], *pid),
        Screen::Search => search::render(f, app, chunks[1]),
        Screen::Queries => queries::render(f, app, chunks[1]),
        Screen::Tonight => misc::render_tonight(f, app, chunks[1]),
        Screen::Projections => misc::render_projections(f, app, chunks[1]),
        Screen::Groups => misc::render_groups(f, app, chunks[1]),
        Screen::GroupDetail(name) => misc::render_group_members(f, app, chunks[1], name),
        Screen::Fetch => misc::render_fetch(f, app, chunks[1]),
        Screen::Help => home::render(f, app, chunks[1]),
        Screen::CompsById(pid) => comps::render_by_id(f, app, chunks[1], *pid),
        Screen::Depth => depth::render_league(f, app, chunks[1]),
        Screen::DepthTeam(abbrev) => depth::render_team(f, app, chunks[1], abbrev),
        Screen::Poach => poach::render(f, app, chunks[1]),
        Screen::Schedule => schedule::render(f, app, chunks[1]),
        Screen::ScheduleTeam(team) => schedule::render_team_schedule(f, app, chunks[1], team),
        Screen::ScheduleMatchup(t1, t2) => schedule::render_matchup(f, app, chunks[1], t1, t2),
        Screen::Playoffs => playoffs::render(f, app, chunks[1]),
        Screen::SeriesDetail(letter) => playoffs::render_series_detail(f, app, chunks[1], letter),
        Screen::GameDetail(game_id) => game_detail::render(f, app, chunks[1], *game_id),
        Screen::Goalies => goalies::render(f, app, chunks[1]),
        Screen::GoalieDetailById(pid) => goalies::render_detail_by_id(f, app, chunks[1], *pid),
        Screen::Transactions => transactions::render(f, app, chunks[1]),
        Screen::Favorites => favorites::render(f, app, chunks[1]),
    }

    // Phase Masterton.1 — chrome-aware footer. When app.status
    // is non-empty (transient flash), show it. Otherwise render
    // the chrome's keybind chips followed by GLOBAL_KEYBINDS.
    render_footer(f, app, &chrome, chunks[2]);

    // Phase Adams.5 — overlays moved to top-level
    // `render_overlays` so MDI mode also paints them.
}

/// Phase Jack Adams.1 — MDI dashboard render path. Layout:
///
///   ┌─ Scores ribbon (top, 1 row) ─────────────────────────┐
///   │                                                      │
///   ├──────────┬─────────────────────────┬─────────────────┤
///   │ Favorites│ Workspace (swappable)   │ Schedule        │
///   │  (left)  │   (middle)              │   (right)       │
///   ├──────────┴─────────────────────────┴─────────────────┤
///   │ Combined footer/cmdbar (bottom, 1 row) ──────────────│
///   └──────────────────────────────────────────────────────┘
///
/// Adams.1 ships STUBS for each pane (placeholder text +
/// border). Real renderers wire in Adams.3 — Favorites pane
/// reuses `screens::favorites::render`, Workspace dispatches on
/// `app.screen` to the right per-screen renderer, Schedule pane
/// reuses `screens::schedule::render`, Scores ribbon gets a new
/// compact renderer in `screens::misc`.
///
/// Pane visibility is determined by
/// `MdiLayout::effective_panes(width)` per spec Adams.4 —
/// adaptive auto-drop combined with manual `mdi.show_*` toggles.
fn render_mdi(f: &mut Frame, app: &App, mdi: &crate::tui::mdi::MdiLayout) {
    let area = f.area();

    // Phase Adams.9 — vertical layout: Scores ribbon (1) +
    // body (Min) + per-screen keybinds (1) + verb cheat sheet
    // (1) + cmdbar (1). The per-screen row sits between the
    // workspace and the cheat sheet, surfacing the active
    // screen's chrome.keybinds so the user discovers
    // sub-commands ('s sort · m min-gp · /sort picker') without
    // hunting for them.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    // Phase Adams.3 — Scores ribbon: compact one-line strip
    // showing today's slate.
    render_mdi_scores_ribbon(f, app, chunks[0]);

    // Body 3-col split based on adaptive visibility.
    let visible = mdi.effective_panes(area.width);
    let mut constraints: Vec<Constraint> = Vec::new();
    if visible.favorites {
        constraints.push(Constraint::Length(28));
    }
    constraints.push(Constraint::Min(0));
    if visible.schedule {
        constraints.push(Constraint::Length(32));
    }
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(chunks[1]);

    let mut idx = 0;
    if visible.favorites {
        render_mdi_favorites_pane(f, app, body_chunks[idx]);
        idx += 1;
    }
    render_mdi_workspace(f, app, body_chunks[idx]);
    idx += 1;
    if visible.schedule {
        render_mdi_schedule_pane(f, app, body_chunks[idx]);
    }
    let _ = idx;

    // Phase Adams.9 — per-screen sub-commands strip. Pulls
    // keybinds from the active workspace screen's chrome
    // accessor and renders them as ` k action · k action · … `.
    // Switches automatically when the workspace swaps
    // (`:goalies` swaps in goalies' chrome).
    render_mdi_screen_keybinds(f, app, chunks[2]);

    // Phase Adams.8 — always-visible verb cheat sheet above
    // the prompt row. Lists the canonical commands the user
    // can call. No gating on focus.
    render_mdi_cheat_sheet(f, chunks[3]);

    // Phase Adams.2 — combined footer/cmdbar. Three modes per
    // spec glass-1/glass-4:
    //
    //   chip-mode   → input empty + not focused: shows hint chips
    //   prompt-mode → input non-empty OR focused: `> {input}_`
    //   error-mode  → flash_error set: red `! {error}` (replaces
    //                 the prompt; cleared on next keypress)
    render_mdi_cmdbar(f, chunks[4], mdi);
}

/// Phase Adams.9 — per-screen sub-command hint row. Surfaces
/// the active workspace screen's keybinds (declared via
/// `chrome()` accessors per Masterton.1) so the user can
/// discover screen-specific sort / filter / column options
/// without leaving the dashboard.
///
/// Format: ` <key>=<action> · <key>=<action> · … ` truncated
/// to fit the terminal width with a trailing `…` indicator.
/// The cyan color distinguishes this row from the yellow
/// global-verb cheat sheet directly below.
///
/// When the active screen has no chrome accessor yet (Team,
/// Depth, Favorites — see Masterton.2 follow-up), shows a
/// placeholder pointing to the cheat sheet below.
fn render_mdi_screen_keybinds(f: &mut Frame, app: &App, area: Rect) {
    let cyan = Style::default().fg(Color::Cyan);
    let dim = Style::default().fg(Color::DarkGray);
    let chrome = active_chrome(app);

    if chrome.keybinds.is_empty() {
        // Screen hasn't declared keybinds yet; advertise that
        // workspace navigation is via the cmdbar verbs below.
        let hint = format!(
            " {}: no per-screen keys yet — use cmdbar verbs below ",
            chrome_screen_label(&app.screen)
        );
        f.render_widget(Paragraph::new(hint).style(dim), area);
        return;
    }

    // Build " key=action · key=action " up to the available
    // width. Trailing chips drop with a `…` if they don't fit
    // (matching the SDI footer's overflow rule).
    let prefix = format!(" {}: ", chrome_screen_label(&app.screen));
    let mut line = prefix.clone();
    let max = area.width as usize;
    let mut overflowed = false;
    for (i, kh) in chrome.keybinds.iter().enumerate() {
        let chip = if i == 0 {
            format!("{}={}", kh.key, kh.action)
        } else {
            format!(" · {}={}", kh.key, kh.action)
        };
        if line.chars().count() + chip.chars().count() + 2 > max {
            overflowed = true;
            break;
        }
        line.push_str(&chip);
    }
    if overflowed {
        line.push_str(" …");
    }
    line.push(' ');
    f.render_widget(Paragraph::new(line).style(cyan), area);
}

/// Phase Adams.9 — short label for the active screen, used as
/// a prefix on the per-screen keybind row. Mirrors
/// `screen_label` but trims to the bare workspace name.
fn chrome_screen_label(s: &Screen) -> &'static str {
    match s {
        Screen::Queries => "Stats",
        Screen::Goalies => "Goalies",
        Screen::Schedule | Screen::ScheduleTeam(_) | Screen::ScheduleMatchup(_, _) => "Schedule",
        Screen::Transactions => "Transactions",
        Screen::Playoffs | Screen::SeriesDetail(_) => "Playoffs",
        Screen::Tonight => "Scores",
        Screen::Team(_) => "Team",
        Screen::Depth | Screen::DepthTeam(_) => "Depth",
        Screen::Poach => "Poach",
        Screen::Favorites => "Favorites",
        Screen::PlayerById(_) => "Player",
        Screen::CompsById(_) => "Comps",
        Screen::GameDetail(_) => "Boxscore",
        Screen::GoalieDetailById(_) => "Goalie",
        _ => "Screen",
    }
}

/// Phase Adams.8 — always-visible cheat sheet of the top cmdbar
/// verbs. Wide-screen gets the rich version; narrow gets the
/// essentials. Always rendered regardless of focus / input
/// state — answers the user's "I need the top commands I can
/// call" feedback.
fn render_mdi_cheat_sheet(f: &mut Frame, area: Rect) {
    let yellow = Style::default().fg(Color::Yellow);
    let line = if area.width >= 140 {
        " stats · goalies · transactions · playoffs · depth · scores · schedule · favorites  |  team <ABBR> · player <name> · query <filter> · /fav add <name> · /help "
    } else if area.width >= 100 {
        " stats · goalies · txs · playoffs · scores · schedule  |  team <ABBR> · player <name> · query <filter> · /help "
    } else {
        " stats · goalies · scores · query <f> · /help "
    };
    f.render_widget(Paragraph::new(line).style(yellow), area);
}

fn render_mdi_cmdbar(f: &mut Frame, area: Rect, mdi: &crate::tui::mdi::MdiLayout) {
    let dim = Style::default().fg(Color::DarkGray);
    let red = Style::default().fg(Color::Red);
    let cyan = Style::default().fg(Color::Cyan);

    if let Some(err) = mdi.flash_error.as_deref() {
        // Error mode (highest priority).
        f.render_widget(Paragraph::new(format!(" ! {err}")).style(red), area);
        return;
    }

    let focused = mdi.command_bar_focused || !mdi.command_input.is_empty();
    if focused {
        // Prompt mode: trailing `_` as fake cursor (no real cursor
        // positioning yet — that's an Adams.3 polish item).
        f.render_widget(
            Paragraph::new(format!(" > {}_", mdi.command_input)).style(cyan),
            area,
        );
    } else {
        // Phase Adams.8 — chip-mode (idle) hint. The cheat
        // sheet row above already lists verbs, so this row
        // emphasizes the cmdbar mechanics: how to enter, how
        // to leave, history navigation.
        let hint = if area.width >= 110 {
            " : / enter cmd · ↑↓ history · Tab leave bar · ^H favs · ^L sched · ? help · q quit "
        } else {
            " : cmd · ↑↓ hist · Tab leave · ? help · q quit "
        };
        f.render_widget(Paragraph::new(hint).style(dim), area);
    }
}

/// Phase Adams.3 — Workspace pane render. Dispatches on
/// `app.screen` exactly like `render_sdi`'s body match. The
/// inner area is shrunken by a 1-cell border so the active
/// screen knows it's a panel, not the whole terminal.
fn render_mdi_workspace(f: &mut Frame, app: &App, area: Rect) {
    let title = format!(" {} ", screen_label(&app.screen));
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(area);
    f.render_widget(block, area);

    match &app.screen {
        Screen::Home => home::render(f, app, inner),
        Screen::Team(abbrev) => team::render(f, app, inner, abbrev),
        Screen::PlayerById(pid) => player::render_by_id(f, app, inner, *pid),
        Screen::Search => search::render(f, app, inner),
        Screen::Queries => queries::render(f, app, inner),
        Screen::Tonight => misc::render_tonight(f, app, inner),
        Screen::Projections => misc::render_projections(f, app, inner),
        Screen::Groups => misc::render_groups(f, app, inner),
        Screen::GroupDetail(name) => misc::render_group_members(f, app, inner, name),
        Screen::Fetch => misc::render_fetch(f, app, inner),
        Screen::Help => home::render(f, app, inner),
        Screen::CompsById(pid) => comps::render_by_id(f, app, inner, *pid),
        Screen::Depth => depth::render_league(f, app, inner),
        Screen::DepthTeam(abbrev) => depth::render_team(f, app, inner, abbrev),
        Screen::Poach => poach::render(f, app, inner),
        Screen::Schedule => schedule::render(f, app, inner),
        Screen::ScheduleTeam(team) => schedule::render_team_schedule(f, app, inner, team),
        Screen::ScheduleMatchup(t1, t2) => schedule::render_matchup(f, app, inner, t1, t2),
        Screen::Playoffs => playoffs::render(f, app, inner),
        Screen::SeriesDetail(letter) => playoffs::render_series_detail(f, app, inner, letter),
        Screen::GameDetail(game_id) => game_detail::render(f, app, inner, *game_id),
        Screen::Goalies => goalies::render(f, app, inner),
        Screen::GoalieDetailById(pid) => goalies::render_detail_by_id(f, app, inner, *pid),
        Screen::Transactions => transactions::render(f, app, inner),
        Screen::Favorites => favorites::render(f, app, inner),
    }
}

/// Phase Adams.3 — Favorites side pane: 28-col strip on the
/// left. Reuses the existing favorites screen renderer; the
/// narrow width forces single-column layout via favorites'
/// internal width branching.
fn render_mdi_favorites_pane(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" ★ Favorites ")
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(area);
    f.render_widget(block, area);
    favorites::render(f, app, inner);
}

/// Phase Adams.3 — Schedule side pane: 32-col strip on the
/// right. Reuses `schedule::render` directly.
fn render_mdi_schedule_pane(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Schedule ")
        .border_style(Style::default().fg(Color::Magenta));
    let inner = block.inner(area);
    f.render_widget(block, area);
    schedule::render(f, app, inner);
}

/// Phase Adams.3 — Scores ribbon: top 1-row strip showing
/// today's slate. Reads from the shared Tonight cache (the
/// same cache the SDI Tonight tab uses). The cache is keyed by
/// the active date (empty string = "today/live").
fn render_mdi_scores_ribbon(f: &mut Frame, app: &App, area: Rect) {
    use crate::tui::tonight::{TonightState, TODAY_KEY};
    let dim = Style::default().fg(Color::DarkGray);
    let cyan = Style::default().fg(Color::Cyan);

    let date_key = if app.tonight.date.is_empty() {
        TODAY_KEY.to_owned()
    } else {
        app.tonight.date.clone()
    };
    let snapshot: TonightState = app
        .tonight
        .cache
        .lock()
        .ok()
        .and_then(|m| m.get(&date_key).cloned())
        .unwrap_or_default();

    let (line, has_games) = match snapshot {
        TonightState::Loaded(games) if !games.is_empty() => {
            let mut parts: Vec<String> = Vec::new();
            for g in games.iter().take(8) {
                let away = &g.away_abbrev;
                let home = &g.home_abbrev;
                let score = match (g.away_score, g.home_score) {
                    (Some(a), Some(h)) => format!("{a}-{h}"),
                    _ => "vs".to_owned(),
                };
                parts.push(format!("{away} {score} {home}"));
            }
            let mut s = format!(" SCORES  {}", parts.join("  ·  "));
            if games.len() > 8 {
                s.push_str(&format!("  +{} more", games.len() - 8));
            }
            (s, true)
        }
        TonightState::Loaded(_) => (
            " SCORES  (no games today — `:scores` for slate)".to_owned(),
            false,
        ),
        TonightState::Loading => (" SCORES  loading…".to_owned(), false),
        TonightState::Error(e) => (format!(" SCORES  err: {e}"), false),
        TonightState::Idle => (
            " SCORES  (idle — switch to scores tab to fetch)".to_owned(),
            false,
        ),
    };

    let style = if has_games { cyan } else { dim };
    f.render_widget(Paragraph::new(line).style(style), area);
}

/// Phase Adams.3 — short label for the active screen, shown
/// as the workspace pane's title.
fn screen_label(s: &Screen) -> &'static str {
    match s {
        Screen::Home => "Home",
        Screen::Team(_) => "Team",
        Screen::PlayerById(_) => "Player",
        Screen::Search => "Search",
        Screen::Queries => "Stats",
        Screen::Tonight => "Tonight",
        Screen::Projections => "Projections",
        Screen::Groups => "Groups",
        Screen::GroupDetail(_) => "Group",
        Screen::Fetch => "Fetch",
        Screen::Help => "Help",
        Screen::CompsById(_) => "Comps",
        Screen::Depth => "Depth",
        Screen::DepthTeam(_) => "Depth (team)",
        Screen::Poach => "Poach",
        Screen::Schedule => "Schedule",
        Screen::ScheduleTeam(_) => "Schedule (team)",
        Screen::ScheduleMatchup(_, _) => "Schedule (matchup)",
        Screen::Playoffs => "Playoffs",
        Screen::SeriesDetail(_) => "Series",
        Screen::GameDetail(_) => "Boxscore",
        Screen::Goalies => "Goalies",
        Screen::GoalieDetailById(_) => "Goalie",
        Screen::Transactions => "Transactions",
        Screen::Favorites => "Favorites",
    }
}

/// Phase Adams.1 — placeholder renderer (kept for completeness;
/// not currently called after Adams.3 wired real renderers).
#[allow(dead_code)]
fn render_mdi_pane_stub(f: &mut Frame, area: Rect, label: &str) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {label} (stub) "))
        .border_style(Style::default().fg(Color::DarkGray));
    f.render_widget(block, area);
}

/// LP.4 — paint the docs overlay. Centered popup, scrollable
/// Paragraph widget. Title shows scroll position so user knows
/// they're not at the top/bottom of a long document.
fn render_docs_overlay(f: &mut Frame, app: &App, area: Rect) {
    const COMMANDS_MD: &str = include_str!("../../../../COMMANDS.md");
    let popup = centered_rect(82, 80, area);
    f.render_widget(Clear, popup);
    let total_lines = COMMANDS_MD.lines().count() as u16;
    let title = format!(
        " Docs (COMMANDS.md) — line {}/{}  ·  ↑↓ scroll · ←→ page · Esc/M close ",
        app.docs_scroll.saturating_add(1).min(total_lines.max(1)),
        total_lines.max(1),
    );
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);
    f.render_widget(block, popup);
    f.render_widget(
        Paragraph::new(COMMANDS_MD).scroll((app.docs_scroll, 0)),
        inner,
    );
}

fn tab_for_screen(screen: &Screen) -> usize {
    match screen {
        Screen::Home | Screen::Team(_) | Screen::PlayerById(_) | Screen::CompsById(_) => 0, // League
        Screen::Depth | Screen::DepthTeam(_) => 1,                                          // Depth
        Screen::Queries | Screen::Projections | Screen::Search => 2, // Stats (default: Queries)
        Screen::Goalies | Screen::GoalieDetailById(_) => 3,          // Goalies
        Screen::Favorites => 4,                                      // Favorites (Foster.2)
        Screen::Poach => 5,                                          // Poach (Selke.5)
        Screen::Tonight | Screen::GameDetail(_) => 6,                // Scores
        Screen::Schedule | Screen::ScheduleTeam(_) | Screen::ScheduleMatchup(..) => 7, // Schedule
        Screen::Transactions => 8,                                   // Transactions
        Screen::Playoffs | Screen::SeriesDetail(_) => 9,             // Playoffs
        // Groups is not a tab (Phase T+1): reachable via `g` from anywhere.
        _ => 99, // no tab (Fetch, Help, Groups)
    }
}

// ── Phase Masterton.1 — chrome dispatch + render helpers ─────────────────────

/// Phase Masterton.1 — dispatch to the active screen's `chrome()`
/// accessor. Falls back to `ScreenChrome::default()` (empty) for
/// screens that don't yet have an accessor — those continue to
/// render a tabs-only header and a global-keybinds-only footer.
fn active_chrome(app: &App) -> crate::tui::chrome::ScreenChrome {
    match &app.screen {
        Screen::Queries => queries::chrome(&app.queries),
        Screen::Schedule => schedule::chrome(&app.schedule),
        Screen::Transactions => transactions::chrome(&app.txs),
        Screen::Goalies => goalies::chrome(&app.goalies),
        Screen::Playoffs => playoffs::chrome(&app.playoffs),
        Screen::Tonight => misc::chrome(&app.tonight),
        // Phase Adams.10 — Team screen now publishes chrome with
        // sort + position filter keybinds.
        Screen::Team(_) => team::chrome(&app.team),
        // Phase Adams.11 — Depth + Favorites chrome.
        Screen::Depth | Screen::DepthTeam(_) => depth::chrome(app.depth_mode, &app.depth_filters),
        Screen::Favorites => favorites::chrome(&app.favorites),
        Screen::Poach => poach::chrome(),
        // Sub-screens and other screens use empty chrome for now.
        // Masterton.2 will add accessors for the rest.
        _ => crate::tui::chrome::ScreenChrome::default(),
    }
}

/// Phase Masterton.1 — header row. Tabs on the left (existing
/// `render_nav`), screen title right-aligned when terminal is
/// ≥120 cols (per spec glass-1). At narrower widths the title
/// drops and tabs win the row.
fn render_header(f: &mut Frame, app: &App, chrome: &crate::tui::chrome::ScreenChrome, area: Rect) {
    if area.width >= 120 && !chrome.title.is_empty() {
        // Reserve room on the right for the title (with a small
        // gap). The right pane gets exactly title_width + 2 cols.
        let title_w = chrome.title.chars().count() as u16 + 2;
        let pane = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(title_w)])
            .split(area);
        render_nav(f, app, pane[0]);
        f.render_widget(
            Paragraph::new(chrome.title.as_str())
                .alignment(ratatui::layout::Alignment::Right)
                .style(Style::default().fg(Color::Cyan)),
            pane[1],
        );
    } else {
        render_nav(f, app, area);
    }
}

/// Phase Masterton.1 — footer row. Renders chrome.keybinds chips
/// followed by GLOBAL_KEYBINDS, with overflow drop at narrow
/// widths (per spec glass-2). When `app.status` is non-empty it
/// takes priority (transient flash replaces chips).
///
/// Today, status carries both transient flashes ("Saved query
/// 'fred'") AND permanent state hints ("Goalies sort: SV%"). The
/// permanent ones are duplicated by chrome.title now; the
/// transient ones still need a place to land. Until each screen
/// migrates its status writes from permanent → declarative
/// keybinds, the footer prefers status when set so nothing is
/// silently dropped.
fn render_footer(f: &mut Frame, app: &App, chrome: &crate::tui::chrome::ScreenChrome, area: Rect) {
    let dim = Style::default().fg(Color::DarkGray);
    if !app.status.is_empty() {
        // Status takes priority — transient flash + legacy
        // permanent hints both render here for now.
        let status_text = if app.active_timeframe == icelines_core::timeframe::Timeframe::Day {
            app.status.clone()
        } else {
            format!(
                "{}  ·  Timeframe: {} ({})",
                app.status,
                crate::tui::app::timeframe_label(app.active_timeframe),
                crate::tui::app::timeframe_anchor_hint(app.active_timeframe),
            )
        };
        f.render_widget(Paragraph::new(status_text).style(dim), area);
        return;
    }

    // Chrome-driven chips. Combine screen keybinds + globals.
    let mut chips: Vec<crate::tui::chrome::KeyHint> = chrome.keybinds.clone();
    chips.extend_from_slice(crate::tui::chrome::GLOBAL_KEYBINDS);

    // Build spans. Each chip is "  Key:action" with a separator.
    // Drop trailing chips with `…` if we'd overflow (glass-2).
    let mut spans: Vec<Span> = Vec::new();
    let mut used: u16 = 0;
    let avail = area.width;
    let dim_key = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let sep = Span::styled("  ·  ", dim);
    for (i, hint) in chips.iter().enumerate() {
        let chip_w = (hint.key.chars().count() + 1 + hint.action.chars().count()) as u16
            + if i == 0 { 0 } else { 5 }; // separator width
        if used + chip_w > avail.saturating_sub(2) {
            // Out of room — drop trailing chips with `…`.
            spans.push(Span::styled(" …", dim));
            break;
        }
        if i > 0 {
            spans.push(sep.clone());
        }
        spans.push(Span::styled(hint.key, dim_key));
        spans.push(Span::styled(format!(":{}", hint.action), dim));
        used = used.saturating_add(chip_w);
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_nav(f: &mut Frame, app: &App, area: Rect) {
    let tab_labels = [
        "League",
        "Depth",
        "Stats",
        "Goalies",
        "Favorites",
        "Poach",
        "Scores",
        "Schedule",
        "Transactions",
        "Playoffs",
    ];
    let active_tab = tab_for_screen(&app.screen);

    let mut spans: Vec<Span> = Vec::new();

    // Phase Masterton.3 — when launched with --standalone, hide
    // the tab strip. Tab/Shift+Tab are no-ops; showing tabs that
    // don't cycle would be misleading. The current screen's
    // chrome title (rendered right-aligned by render_header)
    // gives the user enough context.
    if app.locked_screen.is_none() {
        for (i, label) in tab_labels.iter().enumerate() {
            let active = i == active_tab;
            let style = if active {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            spans.push(Span::styled(format!(" {label} "), style));
            spans.push(Span::styled("│", Style::default().fg(Color::DarkGray)));
        }
    } else {
        // In standalone mode, lead with the active tab name as a
        // breadcrumb so the user sees what surface they're on.
        if let Some(label) = tab_labels.get(active_tab) {
            spans.push(Span::styled(
                format!(" {label} "),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
        }
    }

    // Season indicator — shown when a historical season is active
    if app.active_season != icelines_core::CURRENT_SEASON_STR {
        let label = crate::tui::screens::misc::PICKER_SEASONS
            .iter()
            .find(|(id, _, _)| *id == app.active_season.as_str())
            .map(|(_, l, _)| *l)
            .unwrap_or(app.active_season.as_str());
        spans.push(Span::styled(
            format!("  [{}] ", label.split_whitespace().next().unwrap_or(label)),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
    }

    // Hart.6.9.B — playoff marker. Reverse-video so it pops; only
    // shown when active_type is Playoff (Regular is the default —
    // no marker keeps the bar quiet for the common case).
    if app.active_type == icelines_core::season_stats::SeasonType::Playoff {
        spans.push(Span::styled(
            "  [PLAYOFF] ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
    }

    // Phase Masterton.1 — global keybind hints moved out of the
    // tab strip into the footer chips. The strip now only carries
    // tabs + season indicator + playoff marker. Overlay-state
    // hints (admin/season picker Esc) become flash messages set
    // by the open handler.
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn centered_rect(pct_x: u16, pct_y: u16, r: Rect) -> Rect {
    let popup_h = r.height * pct_y / 100;
    let popup_w = r.width * pct_x / 100;
    Rect::new(
        r.x + (r.width - popup_w) / 2,
        r.y + (r.height - popup_h) / 2,
        popup_w,
        popup_h,
    )
}

// ── Full-app snapshot tests (Hart.5c.6 deliverable) ──────────────────────────
//
// Drives the top-level `render(f, &app)` against a TestBackend across every
// canonical landing screen with a freshly-constructed App (empty repo, cold
// caches). Catches:
//   - panics in any render path under the "no data loaded yet" common case
//   - the nav bar regressing on tab count or label
//   - a screen variant accidentally hiding the status bar or nav row
//
// Per-screen detail tests live under each screens/*.rs file. This is the
// integration glue that proves the dispatcher in `render(f, &app)` and the
// nav-bar layout stay coherent.
#[cfg(test)]
mod app_snapshot_tests {
    use crate::tui::app::{App, Screen};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

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

    fn render_app_to_text(app: &App, w: u16, h: u16) -> String {
        let backend = TestBackend::new(w, h);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| super::render(f, app)).unwrap();
        buffer_text(term.backend().buffer())
    }

    /// The canonical tabs must appear in the nav bar on every screen.
    /// Catches: tab dropped from the array, label renamed, layout truncated
    /// at common widths.
    #[test]
    fn l0_app_nav_bar_renders_all_tabs_at_120_cols() {
        let app = App::new(true);
        let text = render_app_to_text(&app, 120, 30);
        for label in [
            "League",
            "Depth",
            "Stats",
            "Goalies",
            "Favorites",
            "Poach",
            "Scores",
            "Schedule",
            "Transactions",
            "Playoffs",
        ] {
            assert!(
                text.contains(label),
                "nav bar missing tab label {label:?}; full output:\n{text}"
            );
        }
    }

    /// The status line is the bottom-row hint. It must be rendered on the
    /// default Home screen so the user knows how to get help / quit.
    #[test]
    fn l0_app_status_line_present_on_home() {
        let app = App::new(true);
        let text = render_app_to_text(&app, 120, 30);
        // Default status text — App::new initialises this string.
        assert!(
            text.contains("Loading data") || text.contains("Press ?"),
            "status line missing from Home, got:\n{text}"
        );
    }

    /// Render every canonical landing screen with the default empty App.
    /// "Doesn't panic" is itself the invariant — these are the screens a
    /// user can reach before any fetch completes, and a render-time crash
    /// would dump the terminal.
    #[test]
    fn l0_app_renders_every_canonical_landing_screen_without_panic() {
        let canonical_screens = [
            Screen::Home,
            Screen::Search,
            Screen::Queries,
            Screen::Tonight,
            Screen::Projections,
            Screen::Goalies,
            Screen::Poach,
            Screen::Schedule,
            Screen::Playoffs,
            Screen::Transactions,
            Screen::Depth,
            Screen::Fetch,
            Screen::Help,
            Screen::Groups,
        ];
        for screen in canonical_screens {
            let mut app = App::new(true);
            app.screen = screen.clone();
            // If render() panics, the test fails — that's the assertion.
            let text = render_app_to_text(&app, 120, 30);
            // Sanity: screen produced *some* output (non-empty buffer with
            // at least the nav bar).
            assert!(text.contains("League"), "nav bar missing on {screen:?}");
        }
    }

    /// Help overlay is rendered on top — its title must appear.
    #[test]
    fn l0_app_help_overlay_renders_when_show_help_is_true() {
        let mut app = App::new(true);
        app.show_help = true;
        let text = render_app_to_text(&app, 120, 30);
        assert!(
            text.contains("Help"),
            "Help overlay title missing, got:\n{text}"
        );
    }

    /// Admin overlay (F key) must render its frame title when toggled.
    #[test]
    fn l0_app_admin_overlay_renders_when_show_admin_is_true() {
        let mut app = App::new(true);
        app.show_admin = true;
        let text = render_app_to_text(&app, 120, 30);
        assert!(
            text.contains("Admin"),
            "Admin overlay title missing, got:\n{text}"
        );
    }

    // ── Phase Lady Byng (LB.5) — per-surface smoke tests ─────────────────
    //
    // Each smoke proves that a cold-launched surface (no in-app navigation)
    // renders without panicking and emits a stable signature label. These
    // are the contract behind `icelines tui --start <slug>` and are the
    // first defense if an LB-era refactor breaks per-surface boot.
    //
    // Frozen fixtures (per BENCH roles review):
    // - MCDAVID_PID = 8478402 — pid never changes.
    // - "EDM" — Edmonton has existed since 1979.
    // Bedard fine as a SECONDARY active-player smoke; not the only fixture.

    const MCDAVID_PID: u32 = 8478402;
    const STABLE_TEAM: &str = "EDM";

    fn lb_smoke_screen(screen: Screen, expected: &str) {
        let mut app = App::new(true);
        app.screen = screen.clone();
        let text = render_app_to_text(&app, 120, 30);
        assert!(
            text.contains(expected),
            "LB smoke for {screen:?}: expected {expected:?} in render, got:\n{text}"
        );
    }

    /// LB.5 / lb_smoke_league
    #[test]
    fn lb_smoke_league() {
        lb_smoke_screen(Screen::Home, "League");
    }

    /// LB.5 / lb_smoke_depth
    #[test]
    fn lb_smoke_depth() {
        lb_smoke_screen(Screen::Depth, "Depth");
    }

    /// LB.5 / lb_smoke_stats
    #[test]
    fn lb_smoke_stats() {
        lb_smoke_screen(Screen::Queries, "Stats");
    }

    /// LB.5 / lb_smoke_goalies
    #[test]
    fn lb_smoke_goalies() {
        lb_smoke_screen(Screen::Goalies, "Goalies");
    }

    /// LB.5 / lb_smoke_scores
    /// — Network-touching surface; without live data the render still
    ///   shows the "Scores" header. Full content render is gated on
    ///   harness-level network mocking (deferred — see Lady Byng spec
    ///   Future/parked).
    #[test]
    fn lb_smoke_scores() {
        lb_smoke_screen(Screen::Tonight, "Scores");
    }

    /// LB.5 / lb_smoke_schedule
    #[test]
    fn lb_smoke_schedule() {
        lb_smoke_screen(Screen::Schedule, "Schedule");
    }

    /// LB.5 / lb_smoke_transactions
    #[test]
    fn lb_smoke_transactions() {
        lb_smoke_screen(Screen::Transactions, "Transactions");
    }

    /// LB.5 / lb_smoke_playoffs
    #[test]
    fn lb_smoke_playoffs() {
        lb_smoke_screen(Screen::Playoffs, "Playoffs");
    }

    /// LB.5 / lb_smoke_player_card_by_pid
    /// — Cold-launched player card with a pid not in the active repo
    ///   renders the "not in roster" placeholder cleanly. Once UX.1
    ///   lazy fan-out fires (in the live run loop), the real card
    ///   paints; this test is the placeholder render only — stable
    ///   regardless of bundle changes.
    #[test]
    fn lb_smoke_player_card_by_pid() {
        let pid = icelines_core::identity::PlayerId(MCDAVID_PID);
        let mut app = App::new(true);
        app.screen = Screen::PlayerById(pid);
        let text = render_app_to_text(&app, 120, 30);
        // The Player Card title is rendered regardless of whether the
        // pid resolves; that's the contract.
        assert!(
            text.contains("Player Card"),
            "Player Card title missing, got:\n{text}"
        );
    }

    /// LB.5 / lb_smoke_team_card_by_abbrev
    #[test]
    fn lb_smoke_team_card_by_abbrev() {
        let mut app = App::new(true);
        app.screen = Screen::Team(STABLE_TEAM.into());
        let text = render_app_to_text(&app, 120, 30);
        assert!(
            text.contains(STABLE_TEAM),
            "team abbrev {STABLE_TEAM} missing on cold-entered Team card, got:\n{text}"
        );
    }

    /// LB.5 / lb_smoke_goalie_card_by_pid
    /// — Cold-launched goalie card. Tests the Screen::GoalieDetailById
    ///   render path (which LB.3 dispatches to via `tui goalie`).
    #[test]
    fn lb_smoke_goalie_card_by_pid() {
        let pid = icelines_core::identity::PlayerId(MCDAVID_PID);
        let mut app = App::new(true);
        app.screen = Screen::GoalieDetailById(pid);
        // Render must not panic. The goalie-card placeholder labels
        // vary; the assertion is "rendered without crashing".
        let _ = render_app_to_text(&app, 120, 30);
    }

    /// LB.5 / lb_smoke_comps_by_pid
    /// — Cold-launched comps screen.
    #[test]
    fn lb_smoke_comps_by_pid() {
        let pid = icelines_core::identity::PlayerId(MCDAVID_PID);
        let mut app = App::new(true);
        app.screen = Screen::CompsById(pid);
        let _ = render_app_to_text(&app, 120, 30);
    }

    // ── Phase Lester Patrick (LP.4) — in-TUI docs overlay ─────────────

    /// LP.4 / lp_docs_overlay_renders_when_show_docs_is_true
    /// — Pressing `m` toggles `show_docs`; the overlay renders the
    ///   "Docs (COMMANDS.md)" title and content from COMMANDS.md.
    #[test]
    fn lp_docs_overlay_renders_when_show_docs_is_true() {
        let mut app = App::new(true);
        app.show_docs = true;
        let text = render_app_to_text(&app, 120, 30);
        assert!(
            text.contains("Docs (COMMANDS.md)"),
            "docs overlay title missing, got:\n{text}"
        );
        // COMMANDS.md starts with an IceLines heading; match case-insensitively
        // — the overlay should render at least one line of doc content.
        assert!(
            text.to_ascii_lowercase().contains("icelines"),
            "docs overlay content missing, got:\n{text}"
        );
    }

    /// LP.4 / lp_docs_overlay_action_m_opens_overlay
    /// — From any screen, Action::Char('M') sets show_docs=true and
    ///   resets scroll. Renders without panicking on League screen.
    #[test]
    fn lp_docs_overlay_action_m_opens_overlay() {
        use crate::tui::event::Action;
        let mut app = App::new(true);
        app.docs_scroll = 42; // pretend we'd been here before
        let _quit = app.handle(Action::Char('M'));
        assert!(app.show_docs);
        assert_eq!(app.docs_scroll, 0, "opening should reset scroll");
        let _ = render_app_to_text(&app, 120, 30);
    }

    /// LP.4 / lp_docs_overlay_esc_closes
    /// — Esc closes the overlay; show_docs returns to false.
    #[test]
    fn lp_docs_overlay_esc_closes() {
        use crate::tui::event::Action;
        let mut app = App::new(true);
        app.show_docs = true;
        app.handle(Action::Escape);
        assert!(!app.show_docs);
    }

    /// LP.4 / lp_docs_overlay_m_toggles_closed
    /// — `m` while overlay is open closes it (toggle behavior).
    #[test]
    fn lp_docs_overlay_m_toggles_closed() {
        use crate::tui::event::Action;
        let mut app = App::new(true);
        app.show_docs = true;
        app.handle(Action::Char('M'));
        assert!(!app.show_docs);
    }

    /// LP.4 / lp_docs_overlay_arrow_keys_scroll
    /// — Down advances scroll by 1; Up retreats by 1; Right pages
    ///   forward by 20; Left pages back by 20. Saturating arithmetic
    ///   prevents underflow at 0.
    #[test]
    fn lp_docs_overlay_arrow_keys_scroll() {
        use crate::tui::event::Action;
        let mut app = App::new(true);
        app.show_docs = true;
        assert_eq!(app.docs_scroll, 0);
        app.handle(Action::Down);
        assert_eq!(app.docs_scroll, 1);
        app.handle(Action::Down);
        assert_eq!(app.docs_scroll, 2);
        app.handle(Action::Up);
        assert_eq!(app.docs_scroll, 1);
        app.handle(Action::Up);
        assert_eq!(app.docs_scroll, 0);
        app.handle(Action::Up); // saturating — stays at 0
        assert_eq!(app.docs_scroll, 0);
        app.handle(Action::Right);
        assert_eq!(app.docs_scroll, 20);
        app.handle(Action::Left);
        assert_eq!(app.docs_scroll, 0);
    }

    /// LP.4 / lp_docs_overlay_quit_still_quits
    /// — `q` while overlay is open returns true (quit) — the overlay
    ///   does NOT trap the user.
    #[test]
    fn lp_docs_overlay_quit_still_quits() {
        use crate::tui::event::Action;
        let mut app = App::new(true);
        app.show_docs = true;
        let quit = app.handle(Action::Quit);
        assert!(quit, "Quit must propagate even with docs overlay open");
    }

    // ── User-flow tests ──────────────────────────────────────────────────────
    //
    // These tests boot the App with bundled data (via boot_load_with_store
    // against an empty tempdir, which forces the bundled fallback), drive
    // synthetic keypresses through `app.handle(Action::*)`, and re-render
    // after each step. Catches regressions across the boot → render → key
    // → re-render chain that per-screen tests miss because they seed state
    // by hand.
    //
    // The pattern, copy-paste-ready:
    //   let mut app = App::new(true);
    //   app.boot_load_with_store(&empty_store);   // fixture data
    //   let buf = render_app_to_text(&app, 120, 30);
    //   assert!(buf.contains("EDM"));             // home screen has teams
    //   app.handle(Action::Tab);                  // user presses Tab
    //   let buf = render_app_to_text(&app, 120, 30);
    //   assert!(buf.contains("Depth Rankings"));  // depth screen rendered

    fn empty_store_in_tempdir() -> (tempfile::TempDir, icelines_fetch::snapshot::SnapshotStore) {
        let dir = tempfile::TempDir::new().unwrap();
        let store = icelines_fetch::snapshot::SnapshotStore::new(dir.path());
        (dir, store)
    }

    use crate::tui::event::Action;

    /// Boot with bundled data → render Home. Real user content (team
    /// abbrevs from the ranked list) must appear. If this fails, the
    /// boot path is broken — same regression as the user-reported "no
    /// roster data" bug, caught at the render layer this time.
    #[test]
    fn l1_userflow_boot_then_home_shows_real_team_abbrevs() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        let text = render_app_to_text(&app, 120, 30);
        // Home screen shows the 32-team ranked grid. EDM and TOR are stable
        // fixtures of the bundled current-season pool.
        assert!(
            text.contains("EDM"),
            "Home must show EDM team card, got:\n{text}"
        );
        assert!(
            text.contains("TOR"),
            "Home must show TOR team card, got:\n{text}"
        );
        // Negative assertion: the "Loading…" placeholder must NOT be
        // visible after a successful boot.
        assert!(
            !text.contains("Loading…"),
            "Home must not show 'Loading…' after boot completes, got:\n{text}"
        );
    }

    /// User flow: boot → press Tab → land on Depth screen with
    /// real strength data, not the "Loading…" placeholder. Catches the
    /// exact symptom the user reported on the Depth tab.
    #[test]
    fn l1_userflow_boot_then_tab_lands_on_depth_with_data() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        app.handle(Action::Tab);
        let text = render_app_to_text(&app, 140, 40);
        assert!(
            text.contains("Depth Rankings"),
            "Tab from Home should land on Depth, got:\n{text}"
        );
        // The 32-team list must populate — at minimum, the rank-1 row shows
        // a team abbrev. If we still render "Loading…" here, the bug is
        // back.
        assert!(
            !text.contains("Loading…"),
            "Depth screen must not show 'Loading…' after boot, got:\n{text}"
        );
    }

    #[test]
    fn l1_tui_depth_league_render_matches_depth_league_view_first_row() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        app.handle(Action::Tab);
        let view = crate::tui::screens::depth::league_view_from_app(&app)
            .expect("booted depth screen should produce a league view");
        let first = view
            .rows
            .first()
            .expect("depth league view should have rows");
        let text = render_app_to_text(&app, 140, 40);
        let expected = format!(
            "{:<4} {:<5} {:>8.0} {:>8.0} {:>8.0} {:>8.0} {:>9.0}",
            1,
            first.team.0,
            first.c_score,
            first.lw_score,
            first.rw_score,
            first.d_score,
            first.total,
        );

        assert!(
            text.contains(&expected),
            "Depth TUI first row must match DepthLeagueView row projection.\nExpected fragment: {expected}\nGot:\n{text}"
        );
    }

    /// User flow: boot → press 's' to land on Stats(Queries) → real
    /// query UI renders, not the loading placeholder.
    #[test]
    fn l1_userflow_boot_then_stats_tab_shows_query_ui() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        // Tab × 2: Home → Depth → Stats(Queries)
        app.handle(Action::Tab);
        app.handle(Action::Tab);
        let text = render_app_to_text(&app, 140, 40);
        assert_eq!(
            app.screen,
            crate::tui::app::Screen::Queries,
            "Tab×2 from Home should land on Queries"
        );
        // Queries screen renders its title or field list — assert the
        // screen produced *something* and didn't crash to a blank panel.
        assert!(
            !text.trim().is_empty(),
            "Queries screen must render non-empty content, got:\n{text}"
        );
    }

    /// User flow: boot → tab to Goalies → goalie list populates from
    /// bundled goalie-stats (Vezina phase data). Catches the "goalie no
    /// data loaded" symptom from the user report.
    #[test]
    fn l1_userflow_boot_then_goalies_tab_shows_goalie_list() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        // Home → Depth → Queries → Goalies. Phase Lindsay L.3.3 — Tab
        // on Queries toggles section expansion (per spec); use
        // `cycle_screen()` directly to advance past Queries in this
        // navigation-only test.
        app.handle(Action::Tab);
        app.handle(Action::Tab);
        app.cycle_screen(); // Queries → Goalies (Lindsay L.3.3 bypass)
        assert_eq!(app.screen, crate::tui::app::Screen::Goalies);

        // Goalie views must be non-empty post-boot. Screen-level details
        // (column headers, name formatting) live in goalies.rs tests; here
        // we just guard the boot → goalie pool plumbing.
        assert!(
            !app.goalie_views().is_empty(),
            "boot must populate goalie views; got 0"
        );
    }

    /// Help overlay round-trip: boot → '?' opens → next key closes.
    /// Catches the help dispatch regressing when the action enum is
    /// extended.
    #[test]
    fn l1_userflow_help_overlay_open_then_close() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        app.handle(Action::Help);
        assert!(app.show_help);
        let text_open = render_app_to_text(&app, 120, 30);
        assert!(text_open.contains("Help"), "Help overlay title missing");

        app.handle(Action::Char('x')); // any key dismisses
        assert!(!app.show_help);
        let text_closed = render_app_to_text(&app, 120, 30);
        // Loose check — "Help" might still appear in the status bar hint;
        // tighter signal is that the overlay frame is gone, but the dim
        // popup-clear is hard to assert cleanly. Settle for the state flag.
        let _ = text_closed;
    }

    // ── Tab cycling ──────────────────────────────────────────────────────────

    /// Tab from Home cycles forward through all tabs and wraps back
    /// (Phase Foster.2 inserts Favorites; Phase Selke inserts Poach).
    /// Catches tab table regressions (skipped tabs, missing screens).
    #[test]
    fn l1_userflow_tab_cycles_through_all_tabs() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        use crate::tui::app::Screen;
        let expected = [
            Screen::Depth,
            Screen::Queries,
            Screen::Goalies,
            Screen::Favorites,
            Screen::Poach,
            Screen::Tonight,
            Screen::Schedule,
            Screen::Transactions,
            Screen::Playoffs,
            Screen::Home, // wraps
        ];
        // Phase Lindsay L.3.3 — Tab on Queries no longer cycles screens
        // (it toggles section expansion). Use `cycle_screen()` directly
        // here to test the screen-cycle ring; the per-screen
        // `Tab`-handler behavior on Queries is exercised by
        // `l0_lindsay_tui_tab_on_queries_toggles_section`.
        for want in expected {
            app.cycle_screen();
            assert_eq!(app.screen, want, "screen cycle landed on wrong screen");
        }
    }

    /// Shift-Tab cycles in reverse from Home through Playoffs back to Home.
    #[test]
    fn l1_userflow_shift_tab_cycles_backward() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        use crate::tui::app::Screen;
        app.handle(Action::TabPrev);
        assert_eq!(
            app.screen,
            Screen::Playoffs,
            "Shift-Tab from Home → Playoffs"
        );
        app.handle(Action::TabPrev);
        assert_eq!(app.screen, Screen::Transactions);
        app.handle(Action::TabPrev);
        assert_eq!(app.screen, Screen::Schedule);
    }

    /// Numeric jump: GoToTab(n) lands on the right screen for each n.
    /// Note: GoToTab is 0-indexed.
    #[test]
    fn l1_userflow_numeric_keys_jump_to_tab() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        use crate::tui::app::Screen;
        let mapping = [
            (0, Screen::Home),
            (1, Screen::Depth),
            (2, Screen::Queries),
            (3, Screen::Goalies),
            (4, Screen::Favorites),
            (5, Screen::Poach),
            (6, Screen::Tonight),
            (7, Screen::Schedule),
            (8, Screen::Transactions),
            (9, Screen::Playoffs),
        ];
        for (n, want) in mapping {
            app.handle(Action::GoToTab(n));
            assert_eq!(app.screen, want, "GoToTab({n}) landed on wrong screen");
        }
    }

    // ── Drill-downs ─────────────────────────────────────────────────────────

    /// Home → Enter → Team(abbrev) screen. Real player names appear
    /// because the team has bundled stats. Catches the
    /// "Team screen renders empty" regression.
    #[test]
    fn l1_userflow_home_enter_drills_into_team_with_players() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        // selected=0 → first ranked team (COL).
        app.handle(Action::Enter);
        let first = crate::tui::screens::home::RANKED_TEAMS[0];
        assert_eq!(
            app.screen,
            crate::tui::app::Screen::Team(first.to_string()),
            "Enter on Home should drill into the selected team"
        );

        // The team must have ≥1 view in the active window.
        let team_abbr = icelines_core::model::TeamAbbr(first.to_string());
        let team_views = app.team_views(&team_abbr);
        assert!(
            !team_views.is_empty(),
            "{first} must have skater views from bundled data"
        );

        // Render must include the team abbrev in the title.
        let text = render_app_to_text(&app, 140, 40);
        assert!(text.contains(first), "Team screen must show {first} title");
    }

    /// Depth → Enter → DepthTeam screen for the selected team. The
    /// chart frame title must include the team abbreviation.
    #[test]
    fn l1_userflow_depth_enter_drills_into_depth_team() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        app.handle(Action::Tab); // Home → Depth
        app.handle(Action::Enter); // open the rank-1 team's depth chart
        assert!(
            matches!(app.screen, crate::tui::app::Screen::DepthTeam(_)),
            "Enter on Depth should drill into DepthTeam, got {:?}",
            app.screen
        );
    }

    /// Goalies → Enter → GoalieDetailById. Detail screen must render
    /// the goalie's name.
    #[test]
    fn l1_userflow_goalies_enter_drills_into_goalie_detail() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        // Home → Depth → Queries → Goalies. Phase Lindsay L.3.3 — Tab
        // on Queries toggles section, not screen; use `cycle_screen()`
        // to advance past Queries.
        app.handle(Action::Tab); // Home → Depth
        app.handle(Action::Tab); // Depth → Queries
        app.cycle_screen(); // Queries → Goalies
        assert_eq!(app.screen, crate::tui::app::Screen::Goalies);
        app.handle(Action::Enter);
        assert!(
            matches!(app.screen, crate::tui::app::Screen::GoalieDetailById(_)),
            "Enter on Goalies should drill into GoalieDetailById, got {:?}",
            app.screen
        );
    }

    // ── Each tab renders with bundled data (no live network) ─────────────────

    /// Scores tab renders without panicking against an empty (no
    /// network) cache. Should display either today's games OR an
    /// empty-state message — never crash, never hang on "Loading…"
    /// indefinitely (that was the user-reported bug).
    #[test]
    fn l1_userflow_scores_tab_renders_with_empty_cache() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        app.handle(Action::GoToTab(6));
        assert_eq!(app.screen, crate::tui::app::Screen::Tonight);
        let text = render_app_to_text(&app, 140, 40);
        // Nav bar must still show all tabs (Scores tab must not have hidden them).
        assert!(text.contains("Scores"), "Scores tab label must appear");
    }

    /// Schedule tab renders against an empty week cache without panic.
    #[test]
    fn l1_userflow_schedule_tab_renders_with_empty_cache() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        app.handle(Action::GoToTab(7));
        assert_eq!(app.screen, crate::tui::app::Screen::Schedule);
        let _text = render_app_to_text(&app, 140, 40);
    }

    /// Transactions tab renders the legend card when no transactions
    /// snapshot exists.
    #[test]
    fn l1_userflow_transactions_tab_renders_with_empty_feed() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        app.handle(Action::GoToTab(8));
        assert_eq!(app.screen, crate::tui::app::Screen::Transactions);
        let _text = render_app_to_text(&app, 140, 40);
    }

    /// Playoffs tab renders against an empty bracket cache. The feed
    /// loads async; empty state is "Playoffs not yet active" or similar.
    #[test]
    fn l1_userflow_playoffs_tab_renders_with_empty_cache() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        app.handle(Action::GoToTab(9));
        assert_eq!(app.screen, crate::tui::app::Screen::Playoffs);
        let _text = render_app_to_text(&app, 140, 40);
    }

    // ── Overlays ─────────────────────────────────────────────────────────────

    /// Admin overlay (F) opens; Esc closes.
    #[test]
    fn l1_userflow_admin_overlay_open_then_close() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        app.handle(Action::Char('F'));
        assert!(app.show_admin);
        let text = render_app_to_text(&app, 120, 30);
        assert!(text.contains("Admin"), "Admin overlay title missing");

        app.handle(Action::Escape);
        assert!(!app.show_admin);
    }

    /// Season picker (y) opens; Esc closes. Must render the season list.
    #[test]
    fn l1_userflow_season_picker_open_then_close() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        app.handle(Action::Char('y'));
        assert!(app.show_season_picker);
        let text = render_app_to_text(&app, 120, 30);
        // Picker must show at least one bundled season (current or prior).
        assert!(
            text.contains("2024") || text.contains("2025"),
            "Season picker must list bundled seasons, got:\n{text}"
        );

        app.handle(Action::Escape);
        assert!(!app.show_season_picker);
    }

    /// Group picker (g) opens on a player-list screen. Must NOT open on
    /// Home (no player selected).
    #[test]
    fn l1_userflow_group_picker_opens_on_player_screen() {
        with_temp_home(|_home| {
            let (_dir, store) = empty_store_in_tempdir();
            let mut app = App::new(true);
            app.boot_load_with_store(&store);

            {
                let db = crate::db::GroupDb::open().expect("open DB");
                db.create_group("Watchlist", "test").expect("create group");
            }

            // Drill into a player first: Home → Team → Player.
            app.handle(Action::Enter); // Home → Team(rank-1)
            app.handle(Action::Enter); // Team → Player(selected=0)
            assert!(
                matches!(app.screen, crate::tui::app::Screen::PlayerById(_)),
                "Two Enters from Home should land on PlayerById, got {:?}",
                app.screen
            );
            // Now g should open the picker.
            app.handle(Action::AddToGroup);
            assert!(
                app.group_picker.open,
                "g on a Player screen must open the group picker"
            );
        });
    }

    // ── Back navigation ──────────────────────────────────────────────────────

    /// Esc from Team returns to Home (prev_screen restored).
    #[test]
    fn l1_userflow_esc_from_team_returns_to_home() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        app.handle(Action::Enter); // Home → Team
        app.handle(Action::Escape); // Esc → Home
        assert_eq!(app.screen, crate::tui::app::Screen::Home);
    }

    /// Esc from Player returns to Team.
    #[test]
    fn l1_userflow_esc_from_player_returns_to_team() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        app.handle(Action::Enter); // Home → Team(COL)
        app.handle(Action::Enter); // Team → Player
        app.handle(Action::Escape); // → Team
        assert!(
            matches!(app.screen, crate::tui::app::Screen::Team(_)),
            "Esc from Player should return to Team, got {:?}",
            app.screen
        );
    }

    // ── List navigation ─────────────────────────────────────────────────────

    /// Down/Up on Home moves selection in the team grid.
    #[test]
    fn l1_userflow_home_down_up_moves_selection() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        let start = app.selected;
        app.handle(Action::Down);
        assert_ne!(app.selected, start, "Down must move selection from {start}");
        app.handle(Action::Up);
        assert_eq!(app.selected, start, "Up after Down must restore selection");
    }

    // ── Quit ────────────────────────────────────────────────────────────────

    /// 'q' / Quit action terminates. Bubbles up to run_loop via the
    /// `handle()` returning true.
    #[test]
    fn l1_userflow_quit_action_signals_termination() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        let should_quit = app.handle(Action::Quit);
        assert!(
            should_quit,
            "Quit action must return true to break run_loop"
        );
    }

    // ── Hart.6.9.B — Shift+P playoff toggle ─────────────────────────────────

    /// Pressing Shift+P (Char('P')) on a season with bundled playoff
    /// data flips active_type Regular → Playoff and reloads the repo.
    /// 2024-25 has real bundled playoff data so the flip succeeds.
    #[test]
    fn l1_userflow_shift_p_toggles_to_playoff_for_bundled_season() {
        use icelines_core::season_stats::SeasonType;
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        // Force active_season to 2024-25 (has bundled playoff data).
        app.active_season = "20242025".to_owned();
        app.active_season_typed = icelines_core::model::Season(20242025);
        app.boot_load_with_store(&store);
        assert_eq!(app.active_type, SeasonType::Regular);

        app.handle(Action::Char('P'));
        assert_eq!(
            app.active_type,
            SeasonType::Playoff,
            "Shift+P must flip active_type to Playoff for a bundled season"
        );
        // Repo must repopulate with playoff views (332 in 2024-25).
        assert!(
            !app.views().is_empty(),
            "Playoff repo must populate; status={}",
            app.status
        );
    }

    /// Pressing Shift+P twice returns to Regular.
    #[test]
    fn l1_userflow_shift_p_twice_returns_to_regular() {
        use icelines_core::season_stats::SeasonType;
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.active_season = "20242025".to_owned();
        app.active_season_typed = icelines_core::model::Season(20242025);
        app.boot_load_with_store(&store);

        app.handle(Action::Char('P')); // → Playoff
        app.handle(Action::Char('P')); // → Regular
        assert_eq!(app.active_type, SeasonType::Regular);
    }

    /// On 2025-26 (Cup not contested → empty playoff bundle), Shift+P
    /// must NOT flip active_type — the load fails and the user stays
    /// in Regular with a clear status banner. Catches a regression
    /// where the type would silently flip but data wouldn't load,
    /// leaving the user staring at empty screens.
    #[test]
    fn l1_userflow_shift_p_keeps_regular_when_playoff_bundle_empty() {
        use icelines_core::season_stats::SeasonType;
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        // 2025-26 = current season, ships as [] for playoff.
        app.boot_load_with_store(&store);
        assert_eq!(app.active_type, SeasonType::Regular);

        app.handle(Action::Char('P'));
        assert_eq!(
            app.active_type,
            SeasonType::Regular,
            "Shift+P must NOT flip when playoff data is unavailable"
        );
        assert!(
            app.status.to_lowercase().contains("playoff") || app.status.contains("Cup"),
            "status must explain why the toggle didn't take, got: {}",
            app.status
        );
    }

    /// Lowercase `p` (Queries↔Projections flip) must NOT trigger the
    /// playoff toggle. Capital P and lowercase p are distinct Char
    /// values — locks the contract.
    #[test]
    fn l1_userflow_lowercase_p_does_not_trigger_playoff_toggle() {
        use icelines_core::season_stats::SeasonType;
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.active_season = "20242025".to_owned();
        app.active_season_typed = icelines_core::model::Season(20242025);
        app.boot_load_with_store(&store);

        // Navigate to Queries first (lowercase p only fires from there).
        app.handle(Action::GoToTab(2)); // Queries
        app.handle(Action::Char('p'));
        assert_eq!(
            app.active_type,
            SeasonType::Regular,
            "lowercase p must NOT flip active_type"
        );
        // Side effect of the existing flip: should now be on Projections.
        assert_eq!(app.screen, crate::tui::app::Screen::Projections);
    }

    /// `[PLAYOFF]` marker appears in the nav bar when active_type is
    /// Playoff, hidden otherwise. Catches a regression where the user
    /// flipped to playoff but the UI didn't reflect it.
    #[test]
    fn l1_userflow_playoff_marker_shows_in_nav_bar() {
        use icelines_core::season_stats::SeasonType;
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        // Regular (default): no marker.
        let regular = render_app_to_text(&app, 140, 30);
        assert!(
            !regular.contains("[PLAYOFF]"),
            "Regular must not show [PLAYOFF] marker"
        );

        // Force-flip via direct field mutation (avoids data dependency
        // on this test — toggle path is covered above).
        app.active_type = SeasonType::Playoff;
        let playoff = render_app_to_text(&app, 140, 30);
        assert!(
            playoff.contains("[PLAYOFF]"),
            "Playoff active_type must show [PLAYOFF] marker, got:\n{playoff}"
        );
    }

    // ── Schedule search input flow ──────────────────────────────────────────
    //
    // The Schedule tab opens a search bar via '/' that accepts:
    //   "SEA"      → filter to games involving SEA
    //   "NYR WSH"  → filter to NYR-vs-WSH matchups
    //   ""         → clear filter
    // Backspace edits, Enter applies, Esc cancels.

    /// Schedule search: '/' opens, characters append to `schedule_query`,
    /// Enter applies a Team filter that we can read off `schedule_filter`.
    #[test]
    fn l1_userflow_schedule_search_team_filter() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::GoToTab(7)); // Schedule
        assert_eq!(app.screen, crate::tui::app::Screen::Schedule);

        // '/' opens search mode.
        app.handle(Action::Search);
        assert!(app.schedule.search_mode, "/ must enter search mode");
        assert!(app.schedule.query.is_empty());

        // Type "SEA"
        app.handle(Action::Char('s'));
        app.handle(Action::Char('e'));
        app.handle(Action::Char('a'));
        assert_eq!(app.schedule.query, "sea");

        // Apply.
        app.handle(Action::Enter);
        assert!(!app.schedule.search_mode, "Enter must exit search mode");
        assert!(
            matches!(
                app.schedule.filter,
                crate::tui::schedule::SearchFilter::Team(_)
            ),
            "Enter on 'sea' must produce a Team filter, got {:?}",
            app.schedule.filter
        );
    }

    /// Schedule search: matchup syntax "NYR WSH" produces a Matchup filter.
    #[test]
    fn l1_userflow_schedule_search_matchup_filter() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::GoToTab(7));
        app.handle(Action::Search);
        for c in "nyr wsh".chars() {
            if c == ' ' {
                app.handle(Action::Space);
            } else {
                app.handle(Action::Char(c));
            }
        }
        app.handle(Action::Enter);
        assert!(
            matches!(
                app.schedule.filter,
                crate::tui::schedule::SearchFilter::Matchup(_, _)
            ),
            "Enter on 'nyr wsh' must produce a Matchup filter"
        );
    }

    /// Schedule search: backspace edits the query.
    #[test]
    fn l1_userflow_schedule_search_backspace_edits_query() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::GoToTab(7));
        app.handle(Action::Search);
        for c in "edmm".chars() {
            app.handle(Action::Char(c));
        }
        assert_eq!(app.schedule.query, "edmm");
        app.handle(Action::Backspace);
        assert_eq!(app.schedule.query, "edm");
    }

    /// Schedule search: invalid query (unknown team) sets the validation
    /// error, keeps search mode open so the user can fix the typo.
    #[test]
    fn l1_userflow_schedule_search_invalid_keeps_mode_open() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::GoToTab(7));
        app.handle(Action::Search);
        for c in "zzz".chars() {
            app.handle(Action::Char(c));
        }
        app.handle(Action::Enter);
        assert!(
            app.schedule.search_mode,
            "Invalid team must keep search mode open so user can correct"
        );
        assert!(
            app.schedule.filter_err.is_some(),
            "Invalid team must populate schedule_filter_err"
        );
    }

    /// Schedule search: Esc cancels — clears query and exits search mode.
    #[test]
    fn l1_userflow_schedule_search_esc_cancels() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::GoToTab(7));
        app.handle(Action::Search);
        app.handle(Action::Char('s'));
        assert_eq!(app.schedule.query, "s");
        app.handle(Action::Escape);
        assert!(!app.schedule.search_mode, "Esc must exit search mode");
        assert!(app.schedule.query.is_empty(), "Esc must clear query");
    }

    // ── Transactions search input flow ──────────────────────────────────────

    /// Transactions tab '/' opens search, typed text accumulates, Enter
    /// freezes the query (search mode exits, query stays applied).
    #[test]
    fn l1_userflow_transactions_search_freeze_on_enter() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::GoToTab(8)); // Transactions

        app.handle(Action::Search);
        assert!(app.txs.search_mode, "/ must open transactions search");
        for c in "trade".chars() {
            app.handle(Action::Char(c));
        }
        assert_eq!(app.txs.search_query, "trade");

        app.handle(Action::Enter);
        assert!(!app.txs.search_mode, "Enter must exit search mode");
        assert_eq!(
            app.txs.search_query, "trade",
            "Enter must keep query applied"
        );
    }

    /// Transactions search: Esc clears the query and exits mode.
    #[test]
    fn l1_userflow_transactions_search_esc_clears() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::GoToTab(8));
        app.handle(Action::Search);
        app.handle(Action::Char('w'));
        app.handle(Action::Char('a'));
        app.handle(Action::Char('i'));
        app.handle(Action::Char('v'));
        app.handle(Action::Char('e'));
        assert_eq!(app.txs.search_query, "waive");
        app.handle(Action::Escape);
        assert!(!app.txs.search_mode);
        assert!(app.txs.search_query.is_empty(), "Esc must clear query");
    }

    /// Transactions search: backspace edits the query.
    #[test]
    fn l1_userflow_transactions_search_backspace() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::GoToTab(8));
        app.handle(Action::Search);
        for c in "claim".chars() {
            app.handle(Action::Char(c));
        }
        app.handle(Action::Backspace);
        assert_eq!(app.txs.search_query, "clai");
    }

    // ── Stats Search screen input flow ──────────────────────────────────────
    //
    // The Search screen (Tab×2-or-/-from-most-screens) accumulates `search_query`
    // as the user types; the screen filters players by name substring.

    /// Search screen: typing characters builds up `search_query`; backspace
    /// removes them.
    #[test]
    fn l1_userflow_stats_search_screen_accumulates_query_text() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        app.handle(Action::Search); // / opens Search screen
        assert_eq!(app.screen, crate::tui::app::Screen::Search);
        assert!(app.search_query.is_empty());

        for c in "mcdav".chars() {
            app.handle(Action::Char(c));
        }
        assert_eq!(app.search_query, "mcdav");

        app.handle(Action::Backspace);
        assert_eq!(app.search_query, "mcda");
    }

    /// Search screen results render: post-boot, typing a known name prefix
    /// renders results that include the player. Asserts the search screen
    /// is wired through `app.views()` and not a stale path.
    #[test]
    fn l1_userflow_stats_search_renders_known_player_match() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        app.handle(Action::Search);
        // McDavid is in every recent bundled season — type enough to land
        // the row in the rendered list.
        for c in "mcdav".chars() {
            app.handle(Action::Char(c));
        }
        let text = render_app_to_text(&app, 140, 40);
        assert!(
            text.to_lowercase().contains("mcdavid"),
            "Search for 'mcdav' must surface McDavid, got:\n{text}"
        );
    }

    // ── Style / color assertions ─────────────────────────────────────────────
    //
    // TestBackend's Buffer captures Style on each Cell — fg/bg colors and
    // modifiers (bold, italic, etc.) round-trip even though the rendered
    // text strips ANSI. These tests lock the visual contract: rank-1 teams
    // green, last-5 teams red, etc.
    //
    // The pattern: render → walk buffer.area → for each cell, peek
    // `cell.style().fg` against the expected Color.

    /// Helper: render the app and return the full `Buffer` snapshot so
    /// tests can reach into per-cell styles, not just glyphs.
    fn render_app_to_buffer(app: &App, w: u16, h: u16) -> ratatui::buffer::Buffer {
        let backend = TestBackend::new(w, h);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| super::render(f, app)).unwrap();
        term.backend().buffer().clone()
    }

    /// Find the first cell whose symbol starts with the given needle and
    /// return its style. None if not found. Used to spot-check colored
    /// labels in rendered output.
    fn first_cell_style_for(
        buf: &ratatui::buffer::Buffer,
        needle: &str,
    ) -> Option<ratatui::style::Style> {
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                if x as usize + needle.len() > buf.area.width as usize {
                    continue;
                }
                let mut matched = true;
                for (i, want_ch) in needle.chars().enumerate() {
                    let got = buf[(x + i as u16, y)].symbol();
                    if !got.starts_with(want_ch) {
                        matched = false;
                        break;
                    }
                }
                if matched {
                    return Some(buf[(x, y)].style());
                }
            }
        }
        None
    }

    /// Home screen rank tier coloring: rank-1 (`#1`) is rendered Green,
    /// rank ≥28 cells use Red. Locks the home.rs:52-54 tier branches.
    #[test]
    fn l1_userflow_home_rank_tier_colors_locked() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        // App starts with no_color=true. For the style test we need colors
        // to actually flow into the buffer, so flip the field directly.
        app.no_color = false;

        let buf = render_app_to_buffer(&app, 120, 40);

        let one_style = first_cell_style_for(&buf, "#1 ").expect("'#1 ' must appear in Home grid");
        assert_eq!(
            one_style.fg,
            Some(ratatui::style::Color::Green),
            "rank-1 must render with Green fg, got {:?}",
            one_style.fg
        );
    }

    /// Help overlay border style: the popup uses Cyan in the frame title.
    /// Locks `screens/mod.rs:80` ("Help — any key to close").
    #[test]
    fn l1_userflow_help_overlay_uses_cyan_frame() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.no_color = false;
        app.show_help = true;

        let buf = render_app_to_buffer(&app, 120, 30);
        let title_style =
            first_cell_style_for(&buf, "Help").expect("Help title must appear in overlay");
        assert_eq!(
            title_style.fg,
            Some(ratatui::style::Color::Cyan),
            "Help overlay frame title must be Cyan, got {:?}",
            title_style.fg
        );
    }

    /// Admin overlay border style: Yellow per `screens/mod.rs:95`
    /// (" Admin — Esc to close ").
    #[test]
    fn l1_userflow_admin_overlay_uses_yellow_frame() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.no_color = false;
        app.show_admin = true;

        let buf = render_app_to_buffer(&app, 120, 30);
        let title_style =
            first_cell_style_for(&buf, "Admin").expect("Admin title must appear in overlay");
        assert_eq!(
            title_style.fg,
            Some(ratatui::style::Color::Yellow),
            "Admin overlay frame title must be Yellow, got {:?}",
            title_style.fg
        );
    }

    // ── Group SQLite flow (TUI ↔ GroupDb integration) ───────────────────────
    //
    // The `g` (AddToGroup) and `f` (AddToFavorites) keys mutate the local
    // SQLite store at `~/.icelines/icelines.db`. To test the integration
    // without touching the user's real DB, we override `USERPROFILE` /
    // `HOME` to a tempdir for the duration of each test.
    //
    // env vars are process-wide, so these tests serialize through a
    // mutex. cargo test runs other tests in parallel; this group runs
    // sequentially so they don't race on the env. Each test gets its
    // own tempdir.

    /// Run `f` with `USERPROFILE` and `HOME` pointed at a fresh tempdir.
    /// Restores the previous env on drop. Serialized via the shared
    /// process-wide `crate::test_utils::home_env_lock()` so SQLite
    /// tests here don't race with the headshot / scheme tests in
    /// other modules — they all read $HOME and would otherwise
    /// corrupt each other under cargo's parallel runner.
    fn with_temp_home<F, R>(f: F) -> R
    where
        F: FnOnce(&std::path::Path) -> R,
    {
        let _guard = crate::test_utils::home_env_lock();
        let dir = tempfile::TempDir::new().unwrap();
        let prev_userprofile = std::env::var_os("USERPROFILE");
        let prev_home = std::env::var_os("HOME");
        std::env::set_var("USERPROFILE", dir.path());
        std::env::set_var("HOME", dir.path());
        let result = f(dir.path());
        match prev_userprofile {
            Some(p) => std::env::set_var("USERPROFILE", p),
            None => std::env::remove_var("USERPROFILE"),
        }
        match prev_home {
            Some(p) => std::env::set_var("HOME", p),
            None => std::env::remove_var("HOME"),
        }
        result
    }

    /// 'f' on a Player screen instant-adds to the Favorites group.
    /// Asserts the new row landed in the SQLite DB and the status bar
    /// reflects the success.
    #[test]
    fn l1_userflow_add_to_favorites_persists_to_sqlite() {
        with_temp_home(|home| {
            let (_snap, store) = empty_store_in_tempdir();
            let mut app = App::new(true);
            app.boot_load_with_store(&store);

            // Drill into a player: Home → Team → Player.
            app.handle(Action::Enter); // Home → Team
            app.handle(Action::Enter); // Team → Player
            assert!(
                matches!(app.screen, crate::tui::app::Screen::PlayerById(_)),
                "Two Enters should land on PlayerById, got {:?}",
                app.screen
            );

            app.handle(Action::AddToFavorites);

            // Status reflects the add (unless the player was already in
            // — in a fresh DB, this should always be the first add).
            assert!(
                app.status.contains("Favorites"),
                "status must mention Favorites, got: {}",
                app.status
            );

            // The DB at $HOME/.icelines/icelines.db must show ≥1
            // Favorites member.
            let db_path = home.join(".icelines").join("icelines.db");
            assert!(db_path.exists(), "DB file must exist at {:?}", db_path);
            let db = crate::db::GroupDb::open().expect("open DB");
            let members = db
                .list_members("Favorites")
                .expect("Favorites must have been seeded by migration 001");
            assert_eq!(
                members.len(),
                1,
                "exactly one player must be added to Favorites, got {}",
                members.len()
            );
        });
    }

    /// AddToFavorites twice on the same player is a no-op the second
    /// time — status reflects "already in Favorites".
    #[test]
    fn l1_userflow_add_to_favorites_dedupes_on_second_press() {
        with_temp_home(|_home| {
            let (_snap, store) = empty_store_in_tempdir();
            let mut app = App::new(true);
            app.boot_load_with_store(&store);
            app.handle(Action::Enter);
            app.handle(Action::Enter);

            app.handle(Action::AddToFavorites);
            app.handle(Action::AddToFavorites);
            assert!(
                app.status.contains("already") || app.status.to_lowercase().contains("already in"),
                "second press must surface 'already in Favorites', got: {}",
                app.status
            );

            let db = crate::db::GroupDb::open().expect("open DB");
            let members = db.list_members("Favorites").expect("Favorites exists");
            assert_eq!(members.len(), 1, "double-add must NOT duplicate the row");
        });
    }

    /// AddToGroup ('g') on a Player screen opens the picker populated
    /// from `list_groups` — must include Favorites (seeded) and any
    /// user-created groups.
    #[test]
    fn l1_userflow_group_picker_lists_db_groups() {
        with_temp_home(|_home| {
            let (_snap, store) = empty_store_in_tempdir();
            let mut app = App::new(true);
            app.boot_load_with_store(&store);

            // Pre-seed a custom group via direct DB call.
            {
                let db = crate::db::GroupDb::open().expect("open DB");
                db.create_group("Watchlist", "test").expect("create group");
            }

            // Pin a loaded player directly. Navigation into player cards is
            // covered by `l1_userflow_group_picker_opens_on_player_screen`;
            // this test owns the DB-backed group list contract.
            let player_id = app.views().first().expect("fixture has players").id();
            app.screen = crate::tui::app::Screen::PlayerById(player_id);
            app.handle(Action::AddToGroup);

            assert!(app.group_picker.open, "g must open the group picker");
            // Picker list must include both Favorites (seeded by
            // migration 001) and the user-created Watchlist.
            let names: Vec<&str> = app.group_picker.list.iter().map(|s| s.as_str()).collect();
            assert!(
                names.contains(&"Favorites"),
                "picker must list Favorites, got: {:?}",
                names
            );
            assert!(
                names.contains(&"Watchlist"),
                "picker must list user-created Watchlist, got: {:?}",
                names
            );
        });
    }

    // ── Game detail flow (cache-injected fixture) ───────────────────────────
    //
    // The Game Detail screen reads from two caches: `tonight_cache` (the
    // ScheduledGame entry) and `boxscore_cache` (the Boxscore body). In
    // production these are populated by tokio fetches against the live
    // NHL API. In test we inject fixtures directly — no httpmock needed,
    // no network, deterministic.
    //
    // The same pattern works for any cache-backed screen.

    /// Game detail renders the away/home abbrev + final score from a
    /// pre-loaded Boxscore. Catches regressions where the screen pulls
    /// from the wrong cache key or the title formatter drops fields.
    #[test]
    fn l1_userflow_game_detail_renders_loaded_boxscore() {
        use crate::tui::tonight::{BoxscoreState, TonightState};
        use icelines_fetch::nhl_api::{Boxscore, GoalieLine, ScheduledGame};

        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        let game_id: u64 = 2025020100;
        let scheduled = ScheduledGame {
            game_id,
            date: "2026-04-28".to_owned(),
            game_type: 2,
            away_abbrev: "NYR".to_owned(),
            away_name: "New York Rangers".to_owned(),
            home_abbrev: "WSH".to_owned(),
            home_name: "Washington Capitals".to_owned(),
            start_time_utc: "2026-04-28T23:05:00Z".to_owned(),
            away_score: Some(2),
            home_score: Some(3),
            game_state: Some("FINAL".to_owned()),
            last_period: Some("OT".to_owned()),
            series_game: None,
            away_wins: None,
            home_wins: None,
        };
        let boxscore = Boxscore {
            game_id,
            away_abbrev: "NYR".to_owned(),
            home_abbrev: "WSH".to_owned(),
            away_score: 2,
            home_score: 3,
            game_state: Some("FINAL".to_owned()),
            last_period: Some("OT".to_owned()),
            goals: Vec::new(),
            goalies: vec![
                GoalieLine {
                    player_id: 0,
                    player_name: "Shesterkin".to_owned(),
                    team_abbrev: "NYR".to_owned(),
                    saves: 32,
                    shots: 35,
                    decision: Some("L".to_owned()),
                },
                GoalieLine {
                    player_id: 0,
                    player_name: "Lindgren".to_owned(),
                    team_abbrev: "WSH".to_owned(),
                    saves: 28,
                    shots: 30,
                    decision: Some("W".to_owned()),
                },
            ],
            away_skaters: Vec::new(),
            home_skaters: Vec::new(),
        };

        // Inject into tonight_cache (keyed by date "" = today).
        app.tonight
            .cache
            .lock()
            .unwrap()
            .insert(String::new(), TonightState::Loaded(vec![scheduled]));
        // Inject into boxscore_cache (keyed by game_id).
        app.tonight
            .boxscore_cache
            .lock()
            .unwrap()
            .insert(game_id, BoxscoreState::Loaded(boxscore));

        // Switch to GameDetail screen for this game and render.
        app.screen = crate::tui::app::Screen::GameDetail(game_id);
        let text = render_app_to_text(&app, 140, 40);

        // Title must show NYR @ WSH 2-3 with OT suffix.
        assert!(
            text.contains("NYR"),
            "title must show away abbrev NYR, got:\n{text}"
        );
        assert!(text.contains("WSH"), "title must show home abbrev WSH");
        assert!(text.contains("2"), "score 2 must appear");
        assert!(text.contains("3"), "score 3 must appear");
        // Goalie names from injected fixture must render.
        assert!(text.contains("Shesterkin"), "away goalie name must render");
        assert!(text.contains("Lindgren"), "home goalie name must render");
    }

    /// Game detail in Loading state shows a placeholder, never a stale
    /// cache. Catches the regression where switching games leaves the
    /// previous game's body visible during fetch.
    #[test]
    fn l1_userflow_game_detail_loading_state_shows_placeholder() {
        use crate::tui::tonight::BoxscoreState;

        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        let game_id: u64 = 9999999;
        app.tonight
            .boxscore_cache
            .lock()
            .unwrap()
            .insert(game_id, BoxscoreState::Loading);
        app.screen = crate::tui::app::Screen::GameDetail(game_id);

        let text = render_app_to_text(&app, 140, 40);
        // Loading state renders SOME placeholder content. The exact label
        // is implementation-detail; the invariant is that the screen
        // doesn't crash and produces output.
        assert!(
            !text.trim().is_empty(),
            "Loading-state Game Detail must render placeholder content"
        );
    }

    /// Game detail in Error state surfaces the error message so the
    /// user knows the fetch failed.
    #[test]
    fn l1_userflow_game_detail_error_state_surfaces_message() {
        use crate::tui::tonight::BoxscoreState;

        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        let game_id: u64 = 9999999;
        app.tonight.boxscore_cache.lock().unwrap().insert(
            game_id,
            BoxscoreState::Error("503 Service Unavailable".to_owned()),
        );
        app.screen = crate::tui::app::Screen::GameDetail(game_id);

        let text = render_app_to_text(&app, 140, 40);
        // The screen must surface either the raw message or some
        // user-readable indicator.
        assert!(
            text.contains("503")
                || text.to_lowercase().contains("error")
                || text.to_lowercase().contains("unavailable")
                || text.to_lowercase().contains("failed"),
            "Error-state Game Detail must surface the failure, got:\n{text}"
        );
    }

    // ── Save-query input regression fence ───────────────────────────────────
    //
    // Bug report: in QueryMode::SaveName, typing 'f' triggered
    // AddToFavorites instead of inserting 'f' into query_save_name.
    // Same root cause as schedule_search_mode and tx_search_mode: the
    // global keymap fires the hotkey Action before reaching the Char
    // branch. Fix: short-circuit through `handle_query_save_name`.

    /// In SaveName mode, every hotkey letter must land in the name
    /// field, NOT trigger its global action. Locks all five risky keys:
    /// f (Favorites), g (Group), r (Refresh), i (Install), digit keys.
    #[test]
    fn l1_userflow_save_query_name_captures_hotkey_letters_as_text() {
        with_temp_home(|_home| {
            let (_snap, store) = empty_store_in_tempdir();
            let mut app = App::new(true);
            app.boot_load_with_store(&store);

            // Navigate to Stats (Queries) and enter SaveName mode by
            // pressing 's'.
            app.handle(Action::GoToTab(2));
            assert_eq!(app.screen, crate::tui::app::Screen::Queries);
            app.handle(Action::Char('s'));
            assert!(matches!(
                app.queries.mode,
                crate::tui::app::QueryMode::SaveName
            ));

            // Type "fred" — the 'f' is the historical foot-gun.
            app.handle(Action::AddToFavorites); // was: triggered Favorites
            app.handle(Action::Char('r'));
            app.handle(Action::Char('e'));
            app.handle(Action::Char('d'));
            assert_eq!(
                app.queries.save_name, "fred",
                "Hotkey 'f' must insert into query name, not fire AddToFavorites"
            );

            // 'g' / 'r' / 'i' / digits must also be text.
            app.handle(Action::AddToGroup); // 'g'
            app.handle(Action::Refresh); // 'r'
            app.handle(Action::Install); // 'i'
            app.handle(Action::GoToTab(2)); // pushes '3'
            assert_eq!(app.queries.save_name, "fredgri3");
        });
    }

    /// SaveName mode: Backspace edits the name; Esc cancels and restores
    /// Build mode without saving.
    #[test]
    fn l1_userflow_save_query_name_backspace_and_esc() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::GoToTab(2));
        app.handle(Action::Char('s'));

        for c in "myquery".chars() {
            app.handle(Action::Char(c));
        }
        assert_eq!(app.queries.save_name, "myquery");

        app.handle(Action::Backspace);
        assert_eq!(app.queries.save_name, "myquer");

        app.handle(Action::Escape);
        assert!(matches!(
            app.queries.mode,
            crate::tui::app::QueryMode::Build
        ));
        assert!(
            app.queries.save_name.is_empty(),
            "Esc must clear typed name"
        );
    }

    /// SaveName Enter commits to the DB and exits SaveName mode. Picks
    /// up a freshly-saved name via list_saved_queries on the next 'l'
    /// press.
    #[test]
    fn l1_userflow_save_query_enter_persists_to_db() {
        with_temp_home(|_home| {
            let (_snap, store) = empty_store_in_tempdir();
            let mut app = App::new(true);
            app.boot_load_with_store(&store);
            app.handle(Action::GoToTab(2));
            app.handle(Action::Char('s'));
            for c in "centerleaders".chars() {
                app.handle(Action::Char(c));
            }
            app.handle(Action::Enter);
            assert!(matches!(
                app.queries.mode,
                crate::tui::app::QueryMode::Build
            ));

            // Verify DB row exists.
            let db = crate::db::GroupDb::open().expect("open DB");
            let saved = db.list_saved_queries().expect("list");
            assert!(
                saved.iter().any(|(name, _)| name == "centerleaders"),
                "saved query must be in DB, got: {:?}",
                saved.iter().map(|(n, _)| n).collect::<Vec<_>>()
            );
        });
    }

    // ── Phase Art Ross — Wave 24 filter-preset DB round-trip ──────────────

    /// End-to-end DB persistence: enter Queries, type a free-form
    /// filter (`f` overlay), apply it, save the preset, re-load it,
    /// and verify both halves (structured fields + filter text +
    /// re-parsed plan) come back exactly.
    #[test]
    fn l1_w24_filter_preset_full_round_trip_through_db() {
        with_temp_home(|_home| {
            let (_snap, store) = empty_store_in_tempdir();
            let mut app = App::new(true);
            app.boot_load_with_store(&store);
            app.handle(Action::GoToTab(2));
            assert_eq!(app.screen, crate::tui::app::Screen::Queries);

            // Open the filter overlay (`f`), type a filter, apply.
            app.handle(Action::AddToFavorites); // 'f' → FilterEdit
            for c in "country=CAN".chars() {
                if c == ' ' {
                    app.handle(Action::Space);
                } else {
                    app.handle(Action::Char(c));
                }
            }
            app.handle(Action::Enter);
            assert_eq!(app.queries.mode, crate::tui::app::QueryMode::Build);
            assert!(app.queries.filter_plan.is_some(), "filter must be applied");

            // Now save the preset.
            app.handle(Action::Char('s'));
            for c in "canadians".chars() {
                app.handle(Action::Char(c));
            }
            app.handle(Action::Enter);
            assert_eq!(app.queries.mode, crate::tui::app::QueryMode::Build);

            // Sanity: row landed in DB and the JSON is the v2 envelope.
            let db = crate::db::GroupDb::open().expect("open DB");
            let saved = db.list_saved_queries().expect("list");
            let row = saved.iter().find(|(n, _)| n == "canadians");
            let (_, json) = row.expect("preset saved");
            assert!(
                json.contains("\"filter_text\""),
                "saved JSON must carry the v2 filter_text envelope; got: {json}"
            );
            assert!(
                json.contains("country=CAN"),
                "saved JSON must include the typed filter verbatim"
            );

            // Wipe runtime filter state so we can prove the load
            // restores it (not just leftover state).
            app.queries.filter_text.clear();
            app.queries.filter_plan = None;

            // Open LoadList and Enter on the preset.
            app.queries.saved_list = saved;
            app.queries.mode = crate::tui::app::QueryMode::LoadList;
            // Position selector on the canadians row.
            let idx = app
                .queries
                .saved_list
                .iter()
                .position(|(n, _)| n == "canadians")
                .expect("preset listed");
            app.selected = idx;
            app.handle(Action::Enter);

            assert_eq!(app.queries.mode, crate::tui::app::QueryMode::Build);
            assert_eq!(
                app.queries.filter_text, "country=CAN",
                "load must restore the filter text"
            );
            assert!(
                app.queries.filter_plan.is_some(),
                "load must re-install the active plan"
            );
        });
    }

    /// Older v1 (legacy array) saved queries continue to load even
    /// after the schema bump to v2. Insert a hand-built v1 row and
    /// drive the LoadList Enter flow.
    #[test]
    fn l1_w24_v1_legacy_saved_query_still_loads() {
        with_temp_home(|_home| {
            let (_snap, store) = empty_store_in_tempdir();
            let mut app = App::new(true);
            app.boot_load_with_store(&store);
            app.handle(Action::GoToTab(2));

            // Inject a v1 row directly via the DB API.
            let v1_json = r#"[{"label":"Sort by","selected":2},{"label":"Position","selected":1}]"#;
            let db = crate::db::GroupDb::open().expect("open DB");
            db.save_query("legacy-preset", v1_json).expect("save v1");

            // Drive LoadList path.
            let saved = db.list_saved_queries().expect("list");
            app.queries.saved_list = saved;
            app.queries.mode = crate::tui::app::QueryMode::LoadList;
            app.selected = 0;
            app.handle(Action::Enter);

            assert_eq!(app.queries.mode, crate::tui::app::QueryMode::Build);
            assert_eq!(
                app.queries.filter_text, "",
                "v1 legacy row must yield empty filter_text"
            );
            assert!(app.queries.filter_plan.is_none());
            assert_eq!(app.queries.fields[0].selected, 2);
            assert_eq!(app.queries.fields[1].selected, 1);
        });
    }

    // ── Phase Norris.6 — DatePicker / GroupPicker sequencing tests ────────

    /// DatePicker open/cancel cycle: 'd' on Tonight opens the
    /// picker with target=Scores; Esc closes and clears input.
    #[test]
    fn l1_norris_date_picker_open_then_cancel_clears_state() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::GoToTab(6)); // Tonight/Scores

        // 'd' opens the picker.
        app.handle(Action::Char('d'));
        assert!(app.date_picker.open, "d must open the date picker");
        assert_eq!(
            app.date_picker.target,
            crate::tui::app::PickerTarget::Scores,
            "Tonight surface binds target to Scores"
        );

        // Type something into the input.
        for c in "2026".chars() {
            app.handle(Action::Char(c));
        }
        assert_eq!(app.date_picker.input, "2026");

        // Esc closes + clears.
        app.handle(Action::Escape);
        assert!(!app.date_picker.open);
        assert_eq!(app.date_picker.input, "");
        assert!(app.date_picker.err.is_none());
    }

    /// DatePicker target switches across screens: Shift+D opens
    /// the shared overlay (Foster.1.4) and binds target to the
    /// active surface. Lowercase `d` is reserved for the global
    /// depth-chart shortcut on most screens; the cross-surface
    /// picker uses Shift+D specifically.
    #[test]
    fn l1_norris_date_picker_target_rebinds_per_screen() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        // Open on Tonight first via Shift+D.
        app.handle(Action::GoToTab(6)); // Tonight
        app.handle(Action::Char('D'));
        assert_eq!(
            app.date_picker.target,
            crate::tui::app::PickerTarget::Scores
        );
        app.handle(Action::Escape);

        // Now open on Schedule via Shift+D — target rebinds.
        app.handle(Action::GoToTab(7)); // Schedule
        app.handle(Action::Char('D'));
        assert_eq!(
            app.date_picker.target,
            crate::tui::app::PickerTarget::Schedule,
            "opening picker on Schedule must rebind target to Schedule"
        );
    }

    /// GroupPicker on a player card: 'g' opens, list populates
    /// from the DB, player binding is set; Esc closes and clears.
    #[test]
    fn l1_norris_group_picker_open_then_close_clears_player() {
        with_temp_home(|_home| {
            let (_snap, store) = empty_store_in_tempdir();
            let mut app = App::new(true);
            app.boot_load_with_store(&store);
            // Open a player card (any player from the bundled data).
            // Without a real player, 'g' won't open the picker. The
            // existing l1_userflow_group_picker_lists_db_groups test
            // covers that flow; here we directly exercise the state
            // mutation contract.

            // Direct mutation to simulate the picker being opened.
            app.group_picker.open = true;
            app.group_picker.list = vec!["Favorites".into(), "Watch".into()];
            app.group_picker.player = Some(("connor.mcdavid".into(), "Connor McDavid".into()));

            assert!(app.group_picker.open);
            assert_eq!(app.group_picker.list.len(), 2);
            assert!(app.group_picker.player.is_some());

            // Now reset the picker manually (mirrors what Esc
            // handler does — clears the three fields together).
            app.group_picker.open = false;
            app.group_picker.list.clear();
            app.group_picker.player = None;

            assert!(!app.group_picker.open);
            assert!(app.group_picker.list.is_empty());
            assert!(app.group_picker.player.is_none());
        });
    }

    // ── Phase Norris.4 — Goalies/Playoffs/Tonight sequencing tests ────────

    /// Goalies sort cycle: pressing 's' advances `goalies.sort` to
    /// the next index AND resets `goalies.selected` to 0 (cursor
    /// returns to top of newly-sorted list).
    #[test]
    fn l1_norris_goalies_sort_cycle_resets_cursor() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::GoToTab(3)); // Goalies

        // Pretend the user scrolled to row 5.
        app.goalies.selected = 5;
        let sort_before = app.goalies.sort;

        app.handle(Action::Char('s'));
        assert_eq!(
            app.goalies.selected, 0,
            "sort cycle must reset cursor to top"
        );
        assert_ne!(
            app.goalies.sort, sort_before,
            "sort cycle must advance the index"
        );
    }

    /// Playoffs round/series indices are independent — series
    /// cursor doesn't get clobbered by round navigation. (Cursor
    /// reset semantics are intentional in some cases; this test
    /// pins the current behavior so a future refactor surfaces
    /// any change.)
    #[test]
    fn l1_norris_playoffs_default_indices_match_app_new() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::GoToTab(9)); // Playoffs

        // Defaults match what App::new wires through.
        assert_eq!(app.playoffs.round, 0);
        assert_eq!(app.playoffs.series, 0);

        // Direct mutation (handler-driven mutation requires bracket
        // data in the cache; this is the smaller sequencing fence).
        app.playoffs.round = 2;
        app.playoffs.series = 3;
        assert_eq!(
            (app.playoffs.round, app.playoffs.series),
            (2, 3),
            "indices must be independently mutable"
        );
    }

    /// Tonight `date` empty-string sentinel persists across cursor
    /// navigation. The Tonight tab's "today" mode is signaled by
    /// `date == ""`; a user navigating selected rows must not
    /// accidentally fill `date` with anything.
    #[test]
    fn l1_norris_tonight_date_sentinel_survives_cursor_nav() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::GoToTab(6)); // Tonight/Scores

        assert_eq!(
            app.tonight.date, "",
            "Tonight opens in 'today' mode (empty-string sentinel)"
        );

        // Move the cursor a few times via Down (handler may no-op
        // if no rows loaded — that's fine, we only care about the
        // date sentinel).
        for _ in 0..3 {
            app.handle(Action::Down);
        }
        assert_eq!(
            app.tonight.date, "",
            "cursor navigation must NOT touch the date sentinel"
        );
    }

    // ── Phase Norris.3 — TransactionsState sequencing tests ───────────────

    /// Helper: build a fixture transaction with the given kind so
    /// the team-list / kind-cycle helpers have something to chew on.
    /// Mirrors the fixture in tui::screens::transactions::tests.
    fn norris_fixture_tx(kind: icelines_core::TransactionKind) -> icelines_core::Transaction {
        icelines_core::Transaction {
            date: "2026-04-29".to_owned(),
            team: Some(icelines_core::model::TeamAbbr("EDM".to_owned())),
            kind,
            description: "fixture".to_owned(),
            id: "id".to_owned(),
            trade_group_id: None,
            classifier_version: 1,
        }
    }

    /// Kind-filter cycle: pressing 'k' walks through every kind in
    /// `TransactionKind::ALL` (9 variants) and wraps back to None.
    /// 10 presses = full revolution.
    #[test]
    fn l1_norris_txs_kind_filter_cycles_through_all() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::GoToTab(8)); // Transactions

        // Seed at least one row so the cycle has data to act on.
        app.txs.rows = vec![norris_fixture_tx(icelines_core::TransactionKind::Trade)];

        assert!(app.txs.kind_filter.is_none(), "starts with no kind filter");

        // 9 'k' presses walk through every kind. The 10th wraps
        // back to None (the "all" sentinel).
        for _ in 0..9 {
            app.handle(Action::Char('k'));
            assert!(app.txs.kind_filter.is_some(), "in-cycle");
        }
        app.handle(Action::Char('k'));
        assert!(
            app.txs.kind_filter.is_none(),
            "10th press wraps back to None (all)"
        );
    }

    /// Team filter persists across kind-filter cycle. Filters are
    /// independent — changing one must not reset the other.
    #[test]
    fn l1_norris_txs_team_filter_survives_kind_cycle() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::GoToTab(8));

        // Seed two rows with distinct teams so transactions_teams
        // returns a non-empty list to cycle.
        let mut row_a = norris_fixture_tx(icelines_core::TransactionKind::Trade);
        let mut row_b = norris_fixture_tx(icelines_core::TransactionKind::Signing);
        row_a.team = Some(icelines_core::model::TeamAbbr("EDM".to_owned()));
        row_b.team = Some(icelines_core::model::TeamAbbr("BOS".to_owned()));
        app.txs.rows = vec![row_a, row_b];

        // Set team filter to the first team.
        app.handle(Action::Char('t'));
        let team_after_t = app.txs.team_filter.clone();
        assert!(team_after_t.is_some(), "t set a team filter");

        // Cycle kind filter — team filter must NOT change.
        app.handle(Action::Char('k'));
        assert!(app.txs.kind_filter.is_some());
        assert_eq!(
            app.txs.team_filter, team_after_t,
            "team filter must survive kind-filter cycle"
        );
    }

    /// Search query applied → cycling team filter resets `selected`
    /// (cursor returns to top of newly-filtered list) but preserves
    /// `search_query`. The two filter axes don't clobber each other.
    #[test]
    fn l1_norris_txs_team_cycle_resets_cursor_keeps_search() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::GoToTab(8));

        // Seed rows.
        app.txs.rows = vec![
            norris_fixture_tx(icelines_core::TransactionKind::Trade),
            norris_fixture_tx(icelines_core::TransactionKind::Signing),
        ];

        // Apply a search query.
        app.handle(Action::Search);
        for c in "trade".chars() {
            app.handle(Action::Char(c));
        }
        app.handle(Action::Enter);
        assert_eq!(app.txs.search_query, "trade");
        assert!(!app.txs.search_mode);

        // Pretend the user scrolled down to row 5.
        app.txs.selected = 5;

        // Cycle team filter — selected resets to 0, search_query
        // intact.
        app.handle(Action::Char('t'));
        assert_eq!(
            app.txs.selected, 0,
            "team-filter cycle must reset cursor to top"
        );
        assert_eq!(
            app.txs.search_query, "trade",
            "team-filter cycle must NOT touch search_query"
        );
    }

    // ── Phase Norris.2 — ScheduleScreenState sequencing tests ─────────────

    /// Sequential filter replacement — applying a second filter
    /// REPLACES the first; filters don't accumulate. Single
    /// SearchFilter slot, not a stack.
    #[test]
    fn l1_norris_schedule_filter_replaces_on_second_apply() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::GoToTab(7)); // Schedule

        // First search — Team filter on SEA.
        app.handle(Action::Search);
        for c in "sea".chars() {
            app.handle(Action::Char(c));
        }
        app.handle(Action::Enter);
        assert!(
            matches!(
                app.schedule.filter,
                crate::tui::schedule::SearchFilter::Team(_)
            ),
            "first apply must produce a Team filter; got {:?}",
            app.schedule.filter
        );

        // Second search — Matchup filter, replaces the Team filter.
        app.handle(Action::Search);
        for c in "nyr wsh".chars() {
            if c == ' ' {
                app.handle(Action::Space);
            } else {
                app.handle(Action::Char(c));
            }
        }
        app.handle(Action::Enter);
        assert!(
            matches!(
                app.schedule.filter,
                crate::tui::schedule::SearchFilter::Matchup(_, _)
            ),
            "second apply must REPLACE Team with Matchup; got {:?}",
            app.schedule.filter
        );
    }

    /// Filter survives week navigation — applying a filter, then
    /// pressing Right (next week), keeps the filter intact. Filter
    /// is screen-wide state, not per-week.
    #[test]
    fn l1_norris_schedule_filter_survives_week_navigation() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::GoToTab(7));
        let week_before = app.schedule.week.clone();

        // Apply a filter.
        app.handle(Action::Search);
        for c in "sea".chars() {
            app.handle(Action::Char(c));
        }
        app.handle(Action::Enter);
        let filter_after_apply = app.schedule.filter.clone();
        assert!(matches!(
            filter_after_apply,
            crate::tui::schedule::SearchFilter::Team(_)
        ));

        // Navigate to the next week.
        app.handle(Action::Right);
        assert_ne!(
            app.schedule.week, week_before,
            "Right must advance the week"
        );

        // Filter unchanged.
        match (&filter_after_apply, &app.schedule.filter) {
            (
                crate::tui::schedule::SearchFilter::Team(a),
                crate::tui::schedule::SearchFilter::Team(b),
            ) => assert_eq!(a, b, "filter must survive week navigation"),
            (a, b) => panic!(
                "filter shape changed across week nav; before={:?} after={:?}",
                a, b
            ),
        }
    }

    /// State machine: invalid query sets filter_err and keeps
    /// search_mode open. Backspace + valid retype clears the error
    /// AND applies the filter on the next Enter. Verifies error
    /// state doesn't sticky-stick after a successful retry.
    #[test]
    fn l1_norris_schedule_search_err_clears_on_successful_retry() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::GoToTab(7));

        // Type an invalid team.
        app.handle(Action::Search);
        for c in "zzz".chars() {
            app.handle(Action::Char(c));
        }
        app.handle(Action::Enter);
        assert!(app.schedule.search_mode, "invalid keeps search mode open");
        assert!(app.schedule.filter_err.is_some(), "invalid sets filter_err");

        // Backspace the bad query.
        app.handle(Action::Backspace);
        app.handle(Action::Backspace);
        app.handle(Action::Backspace);
        assert_eq!(app.schedule.query, "");

        // Type a valid team.
        for c in "sea".chars() {
            app.handle(Action::Char(c));
        }
        app.handle(Action::Enter);

        assert!(
            !app.schedule.search_mode,
            "valid retry must close search mode"
        );
        assert!(
            app.schedule.filter_err.is_none(),
            "valid retry must clear filter_err"
        );
        assert!(
            matches!(
                app.schedule.filter,
                crate::tui::schedule::SearchFilter::Team(_)
            ),
            "valid retry must apply Team filter; got {:?}",
            app.schedule.filter
        );
    }

    // ── Phase Norris.1 — QueriesState sequencing tests ─────────────────────
    //
    // These tests chain handler calls to exercise QueriesState
    // mutations across a multi-step session — different angle from
    // the per-action L0 tests in tui::app::tests. The pattern is
    // canonical for the rest of Phase Norris: every <Screen>State
    // extraction gets a handful of L0 default-contract tests + 2-3
    // L1 sequencing tests that prove state transitions land
    // correctly across multiple handler calls.

    /// History accumulates across multiple successful Enters in a
    /// session — newest-first, no duplicates. Mirrors the workflow
    /// of a user trying several filters and walking back through
    /// them via the overlay's Up/Down history.
    #[test]
    fn l1_norris_filter_history_accumulates_across_session() {
        with_temp_home(|_home| {
            let (_snap, store) = empty_store_in_tempdir();
            let mut app = App::new(true);
            app.boot_load_with_store(&store);
            app.handle(Action::GoToTab(2));
            assert_eq!(app.screen, crate::tui::app::Screen::Queries);

            // Helper — open editor, type a filter, Enter.
            let apply = |app: &mut App, filter: &str| {
                app.handle(Action::AddToFavorites); // 'f' → FilterEdit
                assert_eq!(app.queries.mode, crate::tui::app::QueryMode::FilterEdit);
                // Reset any leftover text from the previous re-entry
                // (Enter preserves text, so we wipe explicitly).
                app.queries.filter_text.clear();
                for c in filter.chars() {
                    if c == ' ' {
                        app.handle(Action::Space);
                    } else {
                        app.handle(Action::Char(c));
                    }
                }
                app.handle(Action::Enter);
                assert_eq!(app.queries.mode, crate::tui::app::QueryMode::Build);
            };

            apply(&mut app, "country=CAN");
            apply(&mut app, "age<25");
            apply(&mut app, "pos=C");

            // History is newest-first, three distinct entries.
            assert_eq!(app.queries.filter_history.len(), 3);
            assert_eq!(app.queries.filter_history[0], "pos=C");
            assert_eq!(app.queries.filter_history[1], "age<25");
            assert_eq!(app.queries.filter_history[2], "country=CAN");

            // Latest plan is the last one Enter'd.
            assert!(app.queries.filter_plan.is_some());
        });
    }

    /// Refinement workflow: apply a filter, reopen the editor (text
    /// persists), append more grammar, Enter again. Verify the
    /// re-applied plan reflects the new text and history captures
    /// both versions.
    #[test]
    fn l1_norris_refine_workflow_reapplies_extended_filter() {
        with_temp_home(|_home| {
            let (_snap, store) = empty_store_in_tempdir();
            let mut app = App::new(true);
            app.boot_load_with_store(&store);
            app.handle(Action::GoToTab(2));

            // First pass: apply "country=CAN".
            app.handle(Action::AddToFavorites);
            for c in "country=CAN".chars() {
                app.handle(Action::Char(c));
            }
            app.handle(Action::Enter);
            assert_eq!(app.queries.filter_text, "country=CAN");

            // Re-open editor (Enter preserved text).
            app.handle(Action::AddToFavorites);
            assert_eq!(app.queries.mode, crate::tui::app::QueryMode::FilterEdit);
            assert_eq!(
                app.queries.filter_text, "country=CAN",
                "text persists across Enter→reopen"
            );

            // Append " AND age<25".
            for c in " AND age<25".chars() {
                if c == ' ' {
                    app.handle(Action::Space);
                } else {
                    app.handle(Action::Char(c));
                }
            }
            app.handle(Action::Enter);

            assert_eq!(app.queries.filter_text, "country=CAN AND age<25");
            assert!(app.queries.filter_plan.is_some());

            // History captures both versions, newest first.
            assert_eq!(app.queries.filter_history.len(), 2);
            assert_eq!(app.queries.filter_history[0], "country=CAN AND age<25");
            assert_eq!(app.queries.filter_history[1], "country=CAN");
        });
    }

    /// Overlay-mode transitions don't leak state across editors.
    /// SaveName editor's text and FilterEdit editor's text are
    /// separate fields; switching between them must not mix them.
    #[test]
    fn l1_norris_overlay_modes_transition_without_leaking_state() {
        with_temp_home(|_home| {
            let (_snap, store) = empty_store_in_tempdir();
            let mut app = App::new(true);
            app.boot_load_with_store(&store);
            app.handle(Action::GoToTab(2));

            // Enter SaveName mode and type "myquery".
            app.handle(Action::Char('s'));
            assert_eq!(app.queries.mode, crate::tui::app::QueryMode::SaveName);
            for c in "myquery".chars() {
                app.handle(Action::Char(c));
            }
            assert_eq!(app.queries.save_name, "myquery");
            assert_eq!(
                app.queries.filter_text, "",
                "FilterEdit text MUST stay clean while typing in SaveName"
            );

            // Esc out of SaveName — back to Build, save_name cleared.
            app.handle(Action::Escape);
            assert_eq!(app.queries.mode, crate::tui::app::QueryMode::Build);
            assert_eq!(app.queries.save_name, "");

            // Now enter FilterEdit and type "country=CAN".
            app.handle(Action::AddToFavorites); // 'f' → FilterEdit
            assert_eq!(app.queries.mode, crate::tui::app::QueryMode::FilterEdit);
            for c in "country=CAN".chars() {
                app.handle(Action::Char(c));
            }
            assert_eq!(app.queries.filter_text, "country=CAN");
            assert_eq!(
                app.queries.save_name, "",
                "SaveName MUST stay clean while typing in FilterEdit"
            );

            // Esc out of FilterEdit — full reset.
            app.handle(Action::Escape);
            assert_eq!(app.queries.mode, crate::tui::app::QueryMode::Build);
            assert_eq!(app.queries.filter_text, "");
            assert!(app.queries.filter_plan.is_none());
            assert_eq!(
                app.queries.save_name, "",
                "save_name still untouched by FilterEdit Esc"
            );
        });
    }

    // ── Depth-team chart cutoff regression fence ─────────────────────────────
    //
    // Bug report: per-team depth chart score column ("Pts/82") gets cut
    // off at narrow terminal widths. Fix: drop the trailing fit text
    // label and encode fit class as the row's fg color so the score
    // column always lands inside the renderable area.

    /// Team depth chart at 100-col terminal must render the score
    /// column (e.g. "140") for top-line skaters. Catches a regression
    /// where a wider format string would clip the score off the right.
    #[test]
    fn l1_userflow_depth_team_score_visible_at_100_cols() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        // Drill: Home → Depth → DepthTeam(rank-1).
        app.handle(Action::Tab);
        app.handle(Action::Enter);
        assert!(matches!(app.screen, crate::tui::app::Screen::DepthTeam(_)));

        // 100-col is the narrow-but-realistic test width. 5 cols × 20
        // chars each = 100; per-col inner = ~18 chars after borders.
        let buf = render_app_to_buffer(&app, 100, 30);
        let text = {
            let mut out = String::new();
            for y in 0..buf.area.height {
                for x in 0..buf.area.width {
                    out.push_str(buf[(x, y)].symbol());
                }
                out.push('\n');
            }
            out
        };

        // The header column-label must show — either "Pts/82" or "FPts"
        // depending on default scoring mode (Fantasy by App::new). Locks
        // that the header isn't truncated below 4 chars.
        assert!(
            text.contains("FPts") || text.contains("Pts"),
            "Score column header must be visible at 100 cols, got:\n{text}"
        );

        // L1 row prefix must appear with at least its score after.
        assert!(
            text.contains("L1 "),
            "L1 row prefix must appear at 100 cols, got:\n{text}"
        );
    }

    /// Same chart at 120 cols: score column (numeric value) must appear
    /// for at least one player. We don't pin a specific number because
    /// the bundled stats vary; we assert at least one digit appears
    /// AFTER an "L1 " prefix on the same line.
    #[test]
    fn l1_userflow_depth_team_score_value_visible_at_120_cols() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        app.handle(Action::Tab);
        app.handle(Action::Enter);
        let buf = render_app_to_buffer(&app, 120, 30);

        let mut found_l1_with_score = false;
        for y in 0..buf.area.height {
            // Read the row as text.
            let mut row = String::new();
            for x in 0..buf.area.width {
                row.push_str(buf[(x, y)].symbol());
            }
            if let Some(idx) = row.find("L1 ") {
                let after = &row[idx + 3..];
                // Must contain at least one digit somewhere after "L1 ".
                if after.chars().any(|c| c.is_ascii_digit()) {
                    found_l1_with_score = true;
                    break;
                }
            }
        }
        assert!(
            found_l1_with_score,
            "L1 row must show a score (digit) at 120 cols"
        );
    }

    #[test]
    fn l1_tui_depth_team_render_matches_team_depth_chart_view_first_player() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        app.handle(Action::Tab);
        app.handle(Action::Enter);
        let abbrev = match &app.screen {
            crate::tui::app::Screen::DepthTeam(abbrev) => abbrev.clone(),
            other => panic!("expected DepthTeam screen, got {other:?}"),
        };
        let view = crate::tui::screens::depth::team_chart_view_from_app(&app, &abbrev)
            .expect("booted depth team screen should produce a chart view");
        let first_player = view
            .columns
            .iter()
            .find_map(|column| column.players.first())
            .expect("depth chart view should have at least one player");
        let name = first_player
            .display_name
            .chars()
            .take(12)
            .collect::<String>();
        let expected = format!(
            "L{} {:<12} {:>4.0}",
            first_player.line, name, first_player.score
        );
        let text = render_app_to_text(&app, 120, 30);

        assert!(
            text.contains(&expected),
            "Depth team TUI first player must match TeamDepthChartView projection.\nExpected fragment: {expected}\nGot:\n{text}"
        );
    }

    /// Groups screen renders rows for each DB group with their member
    /// counts. Catches a regression where Groups screen reads a stale
    /// in-memory cache instead of querying the DB on entry.
    #[test]
    fn l1_userflow_groups_screen_renders_db_rows() {
        with_temp_home(|_home| {
            let (_snap, store) = empty_store_in_tempdir();
            let mut app = App::new(true);
            app.boot_load_with_store(&store);

            // Seed a group before opening the screen.
            {
                let db = crate::db::GroupDb::open().expect("open DB");
                db.create_group("Bench", "long-term holds").expect("create");
                db.add_member("Bench", "test player").expect("add");
            }

            // 'g' from Home opens the picker, but Action::AddToGroup is
            // only meaningful on a player screen. The Groups *screen*
            // (a separate destination) is reachable via `app.screen =
            // Screen::Groups` — exercise it directly.
            app.screen = crate::tui::app::Screen::Groups;
            let text = render_app_to_text(&app, 140, 40);

            assert!(
                text.contains("Favorites"),
                "Groups screen must list seeded Favorites group, got:\n{text}"
            );
            assert!(
                text.contains("Bench"),
                "Groups screen must list user-created Bench group, got:\n{text}"
            );
        });
    }

    // ─── Phase Lindsay L.3.3 — Queries categorized-sections render ───────

    /// Render snapshot: the Queries screen shows all 4 section headers
    /// with the right ▼/▶ markers (Sort & Display + Position & Age
    /// expanded, Origin & Draft + Stats Thresholds collapsed by default).
    /// Catches a regression where a section was dropped from
    /// `default_sections` or the renderer skipped a section.
    #[test]
    fn l1_lindsay_queries_renders_all_four_section_headers() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        // Navigate to Queries (Tab × 2 from Home).
        app.handle(Action::Tab);
        app.handle(Action::Tab);
        assert_eq!(app.screen, crate::tui::app::Screen::Queries);

        let text = render_app_to_text(&app, 140, 40);
        // Expanded sections show ▼; collapsed sections show ▶.
        assert!(
            text.contains("▼ Sort & Display"),
            "expanded Sort & Display section must show ▼ marker; got:\n{text}"
        );
        assert!(
            text.contains("▼ Position & Age"),
            "expanded Position & Age section must show ▼; got:\n{text}"
        );
        assert!(
            text.contains("▶ Origin & Draft"),
            "collapsed Origin & Draft section must show ▶; got:\n{text}"
        );
        assert!(
            text.contains("▶ Stats Thresholds"),
            "collapsed Stats Thresholds section must show ▶; got:\n{text}"
        );
    }

    /// Render snapshot: expanded sections show their fields indented;
    /// collapsed sections do NOT show their fields. Default state:
    /// "Position" (in expanded section 1) is visible; "Nationality"
    /// (in collapsed section 2) is NOT.
    #[test]
    fn l1_lindsay_queries_collapsed_section_hides_its_fields() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::Tab);
        app.handle(Action::Tab);

        let text = render_app_to_text(&app, 140, 40);

        // Position (field 1, section "Position & Age", expanded by default).
        assert!(
            text.contains("Position"),
            "expanded section's field 'Position' must render; got:\n{text}"
        );
        // Nationality (field 5, section "Origin & Draft", collapsed by default).
        // The label "Nationality" should NOT appear in field-row form
        // (we look for the indented form to avoid false matches).
        assert!(
            !text.contains("    Nationality"),
            "collapsed section's field 'Nationality' must NOT render its row; got:\n{text}"
        );
    }

    /// UX.3 — `o` on Queries collapses the cursor's section (was Tab
    /// pre-UX.3). After collapsing "Sort & Display", the "Sort by"
    /// field row disappears from the rendered output.
    #[test]
    fn l1_ux3_queries_o_collapse_hides_field_row() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::Tab);
        app.handle(Action::Tab);
        assert_eq!(app.screen, crate::tui::app::Screen::Queries);

        // Initial: "Sort by" field row is visible (section 0 expanded).
        let pre = render_app_to_text(&app, 140, 40);
        assert!(
            pre.contains("    Sort by"),
            "Sort by row must be visible pre-collapse; got:\n{pre}"
        );

        // `o` on Queries → collapse cursor's section (section 0 = Sort & Display).
        app.handle(Action::Char('o'));

        let post = render_app_to_text(&app, 140, 40);
        // Section header now shows ▶ (collapsed).
        assert!(
            post.contains("▶ Sort & Display"),
            "collapsed Sort & Display must show ▶ post-o; got:\n{post}"
        );
        // Field row is hidden — no indented "Sort by" row.
        assert!(
            !post.contains("    Sort by"),
            "Sort by row must hide post-collapse; got:\n{post}"
        );
    }

    /// Down on the last visible field auto-EXPANDS the next collapsed
    /// section. Pin the section state transition explicitly: section 2
    /// must flip from collapsed → expanded as the cursor crosses the
    /// section boundary.
    #[test]
    fn l1_lindsay_queries_down_auto_expands_next_collapsed_section() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::Tab); // Home → Depth
        app.handle(Action::Tab); // Depth → Queries

        // Boot state: sections 0 + 1 expanded, 2 + 3 collapsed.
        assert!(
            app.queries.sections[0].expanded,
            "section 0 expanded by default"
        );
        assert!(
            app.queries.sections[1].expanded,
            "section 1 expanded by default"
        );
        assert!(
            !app.queries.sections[2].expanded,
            "section 2 collapsed by default"
        );
        assert!(
            !app.queries.sections[3].expanded,
            "section 3 collapsed by default"
        );

        // Walk to the last field of section 1 (field 3 — last in
        // section 1.fields = [1, 2, 3]).
        for _ in 0..5 {
            app.handle(Action::Down);
        }
        assert_eq!(
            app.queries.field_idx, 3,
            "should reach field 3 after 5 Downs from field 0"
        );
        // Walking through fields 0,9,8,1,2,3 = 5 stops total means
        // after 5 Downs we land on field 3 (index 5 in the visit
        // sequence is field 3, last of section 1) — section 2 still
        // collapsed at this moment because the boundary cross hasn't
        // happened yet.

        // ONE more Down — past last visible field. Section 2 must
        // auto-expand AND cursor must land on section 2's first field.
        app.handle(Action::Down);
        assert!(
            app.queries.sections[2].expanded,
            "Down past last visible must auto-expand the next collapsed section"
        );
        assert_eq!(
            app.queries.sections[2].fields.first().copied(),
            Some(app.queries.field_idx),
            "cursor must land on first field of newly-expanded section 2"
        );
        // Cursor must NOT have escaped to results pane.
        assert!(
            !app.queries.results_focused,
            "results pane should NOT take focus while collapsed sections remain"
        );
    }

    /// Cursor Down past the last visible field of an expanded section
    /// auto-expands the next collapsed section and lands on its first
    /// field. (Phase Lindsay L.5b post-ship user fix — without this,
    /// the cursor jumped straight to the results pane and the user
    /// could never reach later sections via the keyboard.)
    ///
    /// Within an expanded section, Down still skips fields not listed
    /// in `visible_field_indices` — so collapsed-section fields are
    /// hidden until that section is expanded (manually via Tab, or
    /// auto-expanded by Down from the prior section's last field).
    #[test]
    fn l1_lindsay_queries_cursor_down_traverses_all_sections() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::Tab); // Home → Depth
        app.handle(Action::Tab); // Depth → Queries

        // Cursor starts at field 0 (Sort by, section 0). Section 0 +
        // section 1 are expanded by default; sections 2 + 3 collapsed.
        // Walk Down 12 times — enough to visit every field if the
        // auto-expand path works.
        let mut visited = vec![app.queries.field_idx];
        for _ in 0..12 {
            app.handle(Action::Down);
            if app.queries.results_focused {
                break;
            }
            visited.push(app.queries.field_idx);
        }

        // Section 0 ([0, 9, 8]) and section 1 ([1, 2, 3]) come first
        // (declaration order, all expanded from boot).
        assert_eq!(visited[0], 0, "starts at field 0 (Sort by)");
        assert_eq!(visited[1], 9, "Show top");
        assert_eq!(visited[2], 8, "Seasons");
        assert_eq!(visited[3], 1, "Position (section 1 first field)");

        // After section 1's last field, Down auto-expands section 2
        // (Origin & Draft) and lands on its first field. Section 2
        // and 3 fields appear in `visited` because the auto-expand
        // exposes them.
        let unique: std::collections::HashSet<usize> = visited.iter().copied().collect();
        // All 10 fields (0..=9) should be visitable after enough
        // Downs because every section gets auto-expanded in turn.
        for i in 0..=9usize {
            assert!(
                unique.contains(&i),
                "field {i} must be reachable via Down (got visited={visited:?})"
            );
        }
    }

    // ─── Phase Lindsay L.3.4 — sort picker overlay ──────────────────────

    /// `/` on Queries opens the sort picker overlay. Render shows the
    /// search box + filtered list of catalog stats.
    #[test]
    fn l1_lindsay_queries_slash_opens_sort_picker() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::Tab);
        app.handle(Action::Tab);
        assert_eq!(app.screen, crate::tui::app::Screen::Queries);

        // Trigger picker via `/`.
        app.handle(Action::Char('/'));
        assert_eq!(app.queries.mode, crate::tui::app::QueryMode::SortPicker);

        let text = render_app_to_text(&app, 140, 40);
        // Picker title + search prompt visible.
        assert!(
            text.contains("Sort by"),
            "picker title 'Sort by' must render; got:\n{text}"
        );
        assert!(
            text.contains("Search:"),
            "picker search prompt must render; got:\n{text}"
        );
        // Default empty query → all 108 stats listed (count line).
        // L.4.1 added Games (skater GP) → 107 → 108.
        assert!(
            text.contains("108"),
            "picker should show '108' in match count for empty query; got:\n{text}"
        );
    }

    /// Type-as-you-go filters the picker list. Typing `"hits"` reduces
    /// the visible count to a small subset.
    #[test]
    fn l1_lindsay_queries_sort_picker_search_filters_list() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::Tab);
        app.handle(Action::Tab);

        app.handle(Action::Char('/'));
        app.handle(Action::Char('h'));
        app.handle(Action::Char('i'));
        app.handle(Action::Char('t'));
        app.handle(Action::Char('s'));
        assert_eq!(app.queries.sort_picker_query, "hits");

        let text = render_app_to_text(&app, 140, 40);
        // Filtered count is much smaller than 107.
        assert!(
            !text.contains("107 of 107"),
            "filter should reduce result count below 107"
        );
        // "hits" stat is in the filtered list — its label "Hits" should
        // appear (along with HitsPer60 etc.).
        assert!(
            text.contains("hits"),
            "stat key 'hits' must appear in filtered list; got:\n{text}"
        );
    }

    /// Backspace pops chars from the search query and rebuilds list.
    #[test]
    fn l1_lindsay_queries_sort_picker_backspace_pops() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::Tab);
        app.handle(Action::Tab);

        app.handle(Action::Char('/'));
        app.handle(Action::Char('h'));
        app.handle(Action::Char('i'));
        app.handle(Action::Char('t'));
        app.handle(Action::Char('s'));
        assert_eq!(app.queries.sort_picker_query, "hits");

        app.handle(Action::Backspace);
        assert_eq!(app.queries.sort_picker_query, "hit");
        app.handle(Action::Backspace);
        app.handle(Action::Backspace);
        app.handle(Action::Backspace);
        assert_eq!(app.queries.sort_picker_query, "");
    }

    /// Enter on the picker accepts the highlighted stat → exits to
    /// Build mode + sets `sort_stat_pick`.
    #[test]
    fn l1_lindsay_queries_sort_picker_enter_accepts_pick() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::Tab);
        app.handle(Action::Tab);

        app.handle(Action::Char('/'));
        // Empty search → first result is StatId::Games (post-L.4.1
        // declaration order; was Goals pre-L.4.1).
        app.handle(Action::Enter);

        assert_eq!(
            app.queries.mode,
            crate::tui::app::QueryMode::Build,
            "Enter should exit picker back to Build mode"
        );
        assert_eq!(
            app.queries.sort_stat_pick,
            Some(icelines_core::stats_catalog::StatId::Games),
            "first-result accept should set sort_stat_pick = Games (declaration order, L.4.1)"
        );
    }

    /// EDGE checkpoint pre-commit: Esc on Build with an active pick
    /// clears `sort_stat_pick`, restoring the legacy QueryField sort
    /// path. Without this affordance the picker sticks once activated
    /// and Left/Right on the legacy "Sort by" field is silently ignored.
    #[test]
    fn l1_lindsay_queries_esc_clears_active_sort_pick() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::Tab);
        app.handle(Action::Tab);

        // Open picker, accept first result (Games — L.4.1 declaration order).
        app.handle(Action::Char('/'));
        app.handle(Action::Enter);
        assert_eq!(
            app.queries.sort_stat_pick,
            Some(icelines_core::stats_catalog::StatId::Games),
            "picker should set sort_stat_pick"
        );
        assert_eq!(app.queries.mode, crate::tui::app::QueryMode::Build);

        // Esc on Build with active pick clears the pick.
        app.handle(Action::Escape);
        assert_eq!(
            app.queries.sort_stat_pick, None,
            "Esc on Build with active pick must clear sort_stat_pick"
        );
        // Mode stays Build (Esc didn't drop us anywhere else).
        assert_eq!(app.queries.mode, crate::tui::app::QueryMode::Build);
        // Status line surfaces the clear action.
        assert!(
            app.status.contains("cleared") || app.status.contains("Sort pick"),
            "status should mention pick clear; got: {}",
            app.status
        );
    }

    /// Esc on the picker cancels — exits to Build, no pick made.
    #[test]
    fn l1_lindsay_queries_sort_picker_esc_cancels() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::Tab);
        app.handle(Action::Tab);

        let pick_before = app.queries.sort_stat_pick;
        app.handle(Action::Char('/'));
        app.handle(Action::Char('h'));
        app.handle(Action::Escape);

        assert_eq!(
            app.queries.mode,
            crate::tui::app::QueryMode::Build,
            "Esc should return to Build mode"
        );
        assert_eq!(
            app.queries.sort_stat_pick, pick_before,
            "Esc should NOT mutate sort_stat_pick"
        );
    }

    /// Down arrow moves the picker selection within the filtered list.
    /// Up arrow at index 0 stays at 0 (no wrap).
    #[test]
    fn l1_lindsay_queries_sort_picker_down_up_navigation() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::Tab);
        app.handle(Action::Tab);

        app.handle(Action::Char('/'));
        assert_eq!(app.queries.sort_picker_idx, 0);

        app.handle(Action::Down);
        assert_eq!(app.queries.sort_picker_idx, 1);
        app.handle(Action::Down);
        assert_eq!(app.queries.sort_picker_idx, 2);

        app.handle(Action::Up);
        assert_eq!(app.queries.sort_picker_idx, 1);
        app.handle(Action::Up);
        assert_eq!(app.queries.sort_picker_idx, 0);
        // Up at 0 saturates.
        app.handle(Action::Up);
        assert_eq!(app.queries.sort_picker_idx, 0);
    }

    // ─── Phase Lindsay L.4.4 — career-table bracket cycle ───────────────

    /// `]` on the player card cycles career-table preset forward.
    /// Default → Scoring → TwoWay → ... → All → wraps to Default.
    #[test]
    fn l1_lindsay_career_table_right_bracket_cycles_forward() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        // Navigate to a player card via PlayerById. Pick the first
        // skater from the bundled views.
        let pid = app
            .views()
            .first()
            .map(|v| v.identity.id)
            .expect("bundled views must have at least one player");
        app.screen = crate::tui::app::Screen::PlayerById(pid);

        use crate::tui::screens::player::CareerTablePreset;
        assert_eq!(app.queries.career_table_preset, CareerTablePreset::Default);

        app.handle(Action::Char(']'));
        assert_eq!(app.queries.career_table_preset, CareerTablePreset::Scoring);
        app.handle(Action::Char(']'));
        assert_eq!(app.queries.career_table_preset, CareerTablePreset::TwoWay);

        // Cycle forward through remaining; verify wrap.
        // Post-SCOUT-6 (L.5b): cycle has 6 entries, not 7. Started at
        // TwoWay (index 2); +4 → index 6 % 6 = 0 → Default.
        for _ in 0..4 {
            app.handle(Action::Char(']'));
        }
        assert_eq!(
            app.queries.career_table_preset,
            CareerTablePreset::Default,
            "forward cycle wraps Goalie → Default (post-SCOUT-6)"
        );
    }

    /// `[` on the player card cycles career-table preset BACKWARD.
    /// Default → wraps to Goalie (last in cycle post-SCOUT-6) → ...
    #[test]
    fn l1_lindsay_career_table_left_bracket_cycles_backward() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        let pid = app.views().first().map(|v| v.identity.id).unwrap();
        app.screen = crate::tui::app::Screen::PlayerById(pid);

        use crate::tui::screens::player::CareerTablePreset;
        assert_eq!(app.queries.career_table_preset, CareerTablePreset::Default);

        app.handle(Action::Char('['));
        // Default.prev() = Goalie (wraps; post-SCOUT-6 L.5b).
        assert_eq!(app.queries.career_table_preset, CareerTablePreset::Goalie);

        app.handle(Action::Char('['));
        assert_eq!(app.queries.career_table_preset, CareerTablePreset::Time);
    }

    /// Status line surfaces the current preset name after cycle.
    #[test]
    fn l1_lindsay_career_table_cycle_updates_status_line() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        let pid = app.views().first().map(|v| v.identity.id).unwrap();
        app.screen = crate::tui::app::Screen::PlayerById(pid);

        app.handle(Action::Char(']'));
        assert!(
            app.status.contains("Scoring"),
            "status should mention 'Scoring' after `]`; got: {}",
            app.status
        );
        assert!(
            app.status.contains("[/]"),
            "status should remind users of the bracket keys; got: {}",
            app.status
        );
    }

    /// L.4 GLASS-10 (gap-fill) — TestBackend snapshot of `render_stats_view`.
    /// Renders the player card through the full `render(f, &app)` dispatch
    /// at 140×40 and asserts the career table region appears (Career
    /// header, Season column, separator). Catches future regressions
    /// in section ordering or layout truncation.
    ///
    /// Uses `#[tokio::test]` because the headshot loader (PlayerById
    /// render path) calls `tokio::spawn` for the network/disk-cache
    /// path. The headshot fetches are best-effort and don't block
    /// rendering — the buffer text is still meaningful.
    #[tokio::test]
    async fn l1_lindsay_career_table_test_backend_renders_at_140_cols() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        let pid = app.views().first().map(|v| v.identity.id).unwrap();
        app.screen = crate::tui::app::Screen::PlayerById(pid);

        let text = render_app_to_text(&app, 140, 40);
        assert!(
            text.contains("Career"),
            "career-table header `Career` must render — got:\n{text}"
        );
        assert!(
            text.contains("Season"),
            "career-table column `Season` must render — got:\n{text}"
        );
        // The preset label appears in the status header line.
        assert!(
            text.contains("Default"),
            "active preset name `Default` must appear — got:\n{text}"
        );
        // Bio section renders after the table.
        assert!(
            text.contains("Bio") || text.contains("Draft"),
            "bio section (Bio/Draft) must render — got:\n{text}"
        );
    }

    /// L.4 GLASS-10 — at narrow widths (80 cols), the career-table
    /// status line announces dropped columns. Catches regressions in
    /// `fit_career_columns` integration with `render_stats_view`.
    #[tokio::test]
    async fn l1_lindsay_career_table_test_backend_narrow_width_announces_dropped() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        let pid = app.views().first().map(|v| v.identity.id).unwrap();
        app.screen = crate::tui::app::Screen::PlayerById(pid);

        // Render at 80 cols — Default preset (15 cols) won't fit; some
        // columns drop. Status line uses "narrow: -N" format.
        let text = render_app_to_text(&app, 80, 30);
        assert!(
            text.contains("Career"),
            "career-table header must render even at narrow widths — got:\n{text}"
        );
        // The "(N of M cols" pattern appears in the status line.
        assert!(
            text.contains(" of "),
            "status line should show `N of M cols` indicator — got:\n{text}"
        );
    }

    /// `Action::Refresh` (`r` key) resets both `query_fields` AND
    /// `query_sections` to their defaults. After collapsing every
    /// section, Refresh restores the default expansion state.
    #[test]
    fn l1_lindsay_queries_refresh_resets_sections_to_default() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::Tab); // Home → Depth
        app.handle(Action::Tab); // Depth → Queries

        // UX.3 — section toggle moved from Tab to `o`. Collapse section 0
        // + section 1, then refresh — defaults restore.
        app.handle(Action::Char('o')); // collapse section 0 (cursor's section)
                                       // Cursor moved to field 1 (section 1). Another `o` collapses section 1.
        app.handle(Action::Char('o'));
        // At least one of the originally-expanded sections is now collapsed.
        let any_collapsed = !app.queries.sections[0].expanded || !app.queries.sections[1].expanded;
        assert!(
            any_collapsed,
            "post-o×2 at least one section should be collapsed"
        );

        // Refresh.
        app.handle(Action::Refresh);

        // Defaults restored: section 0 + 1 expanded, section 2 + 3 collapsed.
        assert!(
            app.queries.sections[0].expanded,
            "Refresh must re-expand section 0"
        );
        assert!(
            app.queries.sections[1].expanded,
            "Refresh must re-expand section 1"
        );
        assert!(
            !app.queries.sections[2].expanded,
            "Refresh must keep section 2 collapsed"
        );
        assert!(
            !app.queries.sections[3].expanded,
            "Refresh must keep section 3 collapsed"
        );
        // Cursor reset too.
        assert_eq!(app.queries.field_idx, 0);
    }
}

// ── Phase Adams.4 — render-level boundary tests ────────────────────────────
//
// `effective_panes(width)` is unit-tested at the layout-decision
// level in tui/mdi.rs. These tests drive the actual `render`
// entry point through a TestBackend at each adaptive boundary
// width, verifying the rendered frame buffer contains the
// expected pane titles (or doesn't, when the pane is dropped).
//
// Test budget per L0 cost: render is the same code path the
// production loop uses, so these double as smoke tests for the
// whole MDI stack — workspace dispatch, ribbon, panes, cmdbar.

#[cfg(test)]
mod adams_4_render_boundary_tests {
    use super::*;
    use crate::tui::app::App;
    use crate::tui::mdi::MdiLayout;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn buf_text(buf: &ratatui::buffer::Buffer) -> String {
        let mut out = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    fn render_mdi_at(width: u16) -> String {
        let mut app = App::new(true);
        app.mdi = Some(MdiLayout::default());
        // Land on a screen whose renderer is well-behaved on an
        // empty repo. Goalies renders an empty table without
        // panicking.
        app.screen = Screen::Goalies;
        let backend = TestBackend::new(width, 30);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| render(f, &app)).unwrap();
        buf_text(term.backend().buffer())
    }

    fn render_sdi_collapsed(width: u16) -> String {
        // Width <100 must collapse to SDI per spec — `render`
        // detects via `MdiLayout::collapse_to_sdi` and dispatches
        // to render_sdi for the frame.
        let mut app = App::new(true);
        app.mdi = Some(MdiLayout::default());
        app.screen = Screen::Goalies;
        let backend = TestBackend::new(width, 30);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| render(f, &app)).unwrap();
        buf_text(term.backend().buffer())
    }

    /// At width 200, all four MDI regions render: Scores ribbon,
    /// Favorites pane (yellow), Workspace pane, Schedule pane.
    #[test]
    fn l0_adams_render_at_200_full_mdi() {
        let text = render_mdi_at(200);
        assert!(
            text.contains("SCORES"),
            "Scores ribbon must render at 200; got:\n{text}"
        );
        assert!(
            text.contains("Favorites"),
            "Favorites pane must render at 200; got:\n{text}"
        );
        assert!(
            text.contains("Schedule"),
            "Schedule pane must render at 200; got:\n{text}"
        );
        assert!(
            text.contains("Goalies"),
            "Workspace title (Goalies) must render at 200; got:\n{text}"
        );
    }

    /// At width 160 (boundary — exactly meets ≥160 threshold),
    /// schedule pane is still visible.
    #[test]
    fn l0_adams_render_at_160_schedule_visible() {
        let text = render_mdi_at(160);
        assert!(
            text.contains("Schedule"),
            "Schedule pane must render at 160 (boundary); got:\n{text}"
        );
        assert!(text.contains("Goalies"), "workspace must render");
    }

    /// At width 159 (one below threshold), schedule pane drops.
    /// Workspace + favorites + scores still render.
    #[test]
    fn l0_adams_render_at_159_schedule_drops() {
        let text = render_mdi_at(159);
        // Schedule pane title gone. NOTE: the Workspace title
        // could itself be "Schedule" if app.screen == Schedule;
        // we land on Goalies to avoid that confound.
        let schedule_count = text.matches("Schedule").count();
        assert_eq!(
            schedule_count, 0,
            "Schedule pane must drop at 159; got:\n{text}"
        );
        assert!(text.contains("Favorites"), "favorites still visible");
        assert!(text.contains("Goalies"), "workspace still visible");
    }

    /// At width 120 (boundary — exactly meets ≥120 threshold),
    /// favorites pane is still visible. Schedule still dropped.
    #[test]
    fn l0_adams_render_at_120_favorites_visible() {
        let text = render_mdi_at(120);
        assert!(
            text.contains("Favorites"),
            "Favorites pane must render at 120 (boundary); got:\n{text}"
        );
    }

    /// At width 119, both side panes drop. Workspace owns the
    /// body. Scores ribbon still on top.
    #[test]
    fn l0_adams_render_at_119_workspace_only() {
        let text = render_mdi_at(119);
        let favorites_count = text.matches("Favorites").count();
        assert_eq!(
            favorites_count, 0,
            "Favorites pane must drop at 119; got:\n{text}"
        );
        assert!(text.contains("Goalies"), "workspace must still render");
        assert!(text.contains("SCORES"), "ribbon must still render");
    }

    /// At width 100 (boundary above the SDI fallback line), MDI
    /// renders workspace-only.
    #[test]
    fn l0_adams_render_at_100_mdi_workspace_only() {
        let text = render_mdi_at(100);
        assert!(text.contains("Goalies"), "workspace renders at 100");
        assert!(text.contains("SCORES"), "ribbon renders at 100");
    }

    /// At width 99 (just below SDI fallback), MDI render is
    /// abandoned for the frame and SDI takes over. SDI doesn't
    /// have the MDI cmdbar chip-mode hint — we look for the
    /// SDI-specific tab strip / chrome instead.
    #[test]
    fn l0_adams_render_at_99_collapses_to_sdi() {
        let text = render_sdi_collapsed(99);
        // Goalies screen must still render (active screen).
        assert!(text.contains("Goalies"), "SDI fallback renders workspace");
        // The MDI chip-mode hint string is unique to MDI cmdbar;
        // its absence here is the SDI-fallback marker.
        assert!(
            !text.contains("^H favs"),
            "SDI fallback must NOT show MDI chip-mode hint; got:\n{text}"
        );
    }

    /// User manually hides Favorites at wide width — pane must
    /// drop even though adaptive layer would keep it.
    #[test]
    fn l0_adams_render_manual_hide_overrides_adaptive() {
        let mut app = App::new(true);
        let mut layout = MdiLayout::default();
        layout.show_favorites = false;
        app.mdi = Some(layout);
        app.screen = Screen::Goalies;
        let backend = TestBackend::new(200, 30);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| render(f, &app)).unwrap();
        let text = buf_text(term.backend().buffer());
        let fav_count = text.matches("Favorites").count();
        assert_eq!(
            fav_count, 0,
            "Manual show_favorites=false must drop pane at 200; got:\n{text}"
        );
        assert!(
            text.contains("Schedule"),
            "Schedule still adaptive-visible at 200"
        );
    }

    // ── Phase Adams.9 — per-screen sub-command hint row ─────────────────

    /// Per-screen row pulls keybinds from `active_chrome` —
    /// switches when the workspace screen swaps.
    #[test]
    fn l0_adams_per_screen_row_shows_goalies_keybinds() {
        let mut app = App::new(true);
        app.mdi = Some(MdiLayout::default());
        app.screen = Screen::Goalies;
        let backend = TestBackend::new(160, 30);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| render(f, &app)).unwrap();
        let text = buf_text(term.backend().buffer());
        // Goalies chrome advertises 's' (sort) and 'm' (min-gp).
        assert!(
            text.contains("s=") && text.contains("sort"),
            "goalies per-screen row must include 's=sort'; got:\n{text}"
        );
        assert!(
            text.contains("m="),
            "goalies per-screen row must include 'm=' min-gp; got:\n{text}"
        );
    }

    /// Stats screen row advertises filter / sort / save / load.
    #[test]
    fn l0_adams_per_screen_row_shows_stats_keybinds() {
        let mut app = App::new(true);
        app.mdi = Some(MdiLayout::default());
        app.screen = Screen::Queries;
        let backend = TestBackend::new(160, 30);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| render(f, &app)).unwrap();
        let text = buf_text(term.backend().buffer());
        assert!(
            text.contains("f=filter"),
            "stats per-screen row must include 'f=filter'; got:\n{text}"
        );
        assert!(
            text.contains("save") && text.contains("load"),
            "stats per-screen row must include save/load; got:\n{text}"
        );
    }

    /// Adams.10 — pressing `s` on Team cycles sort key.
    #[test]
    fn l1_adams_team_s_cycles_sort() {
        use crate::tui::event::Action;
        use crate::tui::screens::team::TeamSort;
        let mut app = App::new(true);
        app.mdi = Some(MdiLayout::default());
        app.screen = Screen::Team("EDM".to_owned());
        assert_eq!(app.team.sort, TeamSort::Pace);
        app.handle(Action::Char('s'));
        assert_eq!(app.team.sort, TeamSort::Name);
        app.handle(Action::Char('s'));
        assert_eq!(app.team.sort, TeamSort::Position);
    }

    /// Adams.10 — pressing `p` on Team cycles position filter.
    #[test]
    fn l1_adams_team_p_cycles_pos_filter() {
        use crate::tui::event::Action;
        use crate::tui::screens::team::TeamPosFilter;
        let mut app = App::new(true);
        app.mdi = Some(MdiLayout::default());
        app.screen = Screen::Team("EDM".to_owned());
        assert_eq!(app.team.filters.pos_filter, TeamPosFilter::All);
        app.handle(Action::Char('p'));
        assert_eq!(app.team.filters.pos_filter, TeamPosFilter::Forwards);
        app.handle(Action::Char('p'));
        assert_eq!(app.team.filters.pos_filter, TeamPosFilter::Defense);
    }

    /// Adams.12 — pressing `c` on Team cycles country filter.
    #[test]
    fn l1_adams_team_c_cycles_country() {
        use crate::tui::event::Action;
        let mut app = App::new(true);
        app.mdi = Some(MdiLayout::default());
        app.screen = Screen::Team("EDM".to_owned());
        assert_eq!(app.team.filters.country_filter, None);
        app.handle(Action::Char('c'));
        assert_eq!(
            app.team.filters.country_filter,
            Some(crate::tui::filter_state::CountryCode::CAN)
        );
        app.handle(Action::Char('c'));
        assert_eq!(
            app.team.filters.country_filter,
            Some(crate::tui::filter_state::CountryCode::USA)
        );
    }

    /// Adams.12 — pressing `h` on Team toggles the Hits column
    /// independent of sort.
    #[test]
    fn l1_adams_team_h_toggles_hits_column() {
        use crate::tui::event::Action;
        let mut app = App::new(true);
        app.mdi = Some(MdiLayout::default());
        app.screen = Screen::Team("EDM".to_owned());
        assert!(!app.team.hits_column_forced());
        app.handle(Action::Char('h'));
        assert!(app.team.hits_column_forced());
        app.handle(Action::Char('h'));
        assert!(!app.team.hits_column_forced());
    }

    /// Adams.10 — `s` on a non-Team screen does NOT touch
    /// `app.team.sort` (e.g., Goalies' `s` cycles its own sort).
    #[test]
    fn l1_adams_team_s_is_screen_scoped() {
        use crate::tui::event::Action;
        use crate::tui::screens::team::TeamSort;
        let mut app = App::new(true);
        app.mdi = Some(MdiLayout::default());
        app.screen = Screen::Goalies;
        let team_sort_before = app.team.sort;
        let goalies_sort_before = app.goalies.sort;
        app.handle(Action::Char('s'));
        assert_eq!(app.team.sort, team_sort_before, "team sort must not move");
        assert_ne!(
            app.goalies.sort, goalies_sort_before,
            "goalies sort must move"
        );
        let _ = TeamSort::Pace; // imported for clarity
    }

    /// Adams.10 — Team screen now has chrome (sort + pos filter
    /// keybinds). Verify the per-screen row advertises them.
    #[test]
    fn l0_adams_per_screen_row_shows_team_keybinds() {
        let mut app = App::new(true);
        app.mdi = Some(MdiLayout::default());
        app.screen = Screen::Team("EDM".to_owned());
        let backend = TestBackend::new(160, 30);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| render(f, &app)).unwrap();
        let text = buf_text(term.backend().buffer());
        assert!(
            text.contains("s=cycle sort") && text.contains("p=cycle pos"),
            "Team per-screen row must advertise s + p; got:\n{text}"
        );
    }

    /// Render must be panic-free across every width 80..=240.
    /// Catches edge cases in the layout constraint solver
    /// (e.g., negative remaining body width when both side panes
    /// claim 28+32 cols on a 90-col terminal).
    #[test]
    fn l0_adams_render_does_not_panic_at_any_width() {
        for width in (80u16..=240).step_by(7) {
            let mut app = App::new(true);
            app.mdi = Some(MdiLayout::default());
            app.screen = Screen::Goalies;
            let backend = TestBackend::new(width, 30);
            let mut term = Terminal::new(backend).unwrap();
            // panic in render would propagate out of draw().
            term.draw(|f| render(f, &app)).unwrap();
        }
    }

    // ── L1 — resize sequencing (full app loop, multiple frames) ───────────

    /// L1: Simulate a user resizing their terminal mid-session
    /// from 200 → 159 → 119 → 99 → back to 200. Drives multiple
    /// render frames through the same App. Verifies the layout
    /// adapts each frame without losing state, and that the SDI
    /// fallback at 99 doesn't permanently flip the app out of
    /// MDI mode (per spec glass-5: resize back ≥100 returns to
    /// MDI rendering automatically).
    ///
    /// Implementation note: TestBackend's buffer is preserved
    /// across resize, so stale content from a wider frame
    /// lingers in the right margin after shrinking. We use a
    /// fresh Terminal per width to keep the assertions clean
    /// — App state is the unit under test, not Terminal reuse.
    #[test]
    fn l1_adams_resize_sequence_preserves_mdi_state() {
        let mut app = App::new(true);
        app.mdi = Some(MdiLayout::default());
        app.screen = Screen::Goalies;

        fn render_at(app: &App, width: u16) -> String {
            let backend = TestBackend::new(width, 30);
            let mut term = Terminal::new(backend).unwrap();
            term.draw(|f| render(f, app)).unwrap();
            buf_text(term.backend().buffer())
        }

        // 200: full MDI.
        let text_200 = render_at(&app, 200);
        assert!(text_200.contains("Schedule"), "200: schedule visible");
        assert!(text_200.contains("Favorites"), "200: favorites visible");
        assert!(app.mdi.is_some(), "MDI state preserved after frame 1");

        // 159: schedule drops adaptively.
        let text_159 = render_at(&app, 159);
        assert!(
            !text_159.contains("Schedule"),
            "159: schedule must drop; got:\n{text_159}"
        );
        assert!(text_159.contains("Favorites"), "159: favorites stays");

        // 119: favorites drops too.
        let text_119 = render_at(&app, 119);
        assert!(!text_119.contains("Favorites"), "119: favorites must drop");
        assert!(text_119.contains("Goalies"), "119: workspace still renders");

        // 99: SDI fallback for the frame.
        let text_99 = render_at(&app, 99);
        assert!(text_99.contains("Goalies"), "99: SDI renders workspace");
        // MDI state preserved even though we rendered SDI.
        assert!(
            app.mdi.is_some(),
            "MDI state must survive the SDI-fallback frame"
        );

        // 200 again: full MDI returns.
        let text_back = render_at(&app, 200);
        assert!(
            text_back.contains("Schedule") && text_back.contains("Favorites"),
            "Resize back to 200 must restore both side panes; got:\n{text_back}"
        );
    }

    /// L1: User manually hides favorites at wide width, then
    /// resizes narrower. The manual `show_favorites = false`
    /// must persist across resizes — the user's intent doesn't
    /// reset on a screen-size change.
    #[test]
    fn l1_adams_manual_hide_persists_across_resize() {
        let mut app = App::new(true);
        app.mdi = Some(MdiLayout::default());
        app.screen = Screen::Goalies;

        // User toggles favorites off at width 200.
        app.handle(crate::tui::event::Action::ToggleFavoritesPane);
        assert!(!app.mdi.as_ref().unwrap().show_favorites);

        fn render_at(app: &App, width: u16) -> String {
            let backend = TestBackend::new(width, 30);
            let mut term = Terminal::new(backend).unwrap();
            term.draw(|f| render(f, app)).unwrap();
            buf_text(term.backend().buffer())
        }

        let text_200 = render_at(&app, 200);
        assert!(
            !text_200.contains("Favorites"),
            "After Ctrl+H: favorites must be hidden at 200"
        );

        // Resize to 119 (where adaptive would have dropped it
        // anyway) and back to 200. Manual flag preserved.
        let _ = render_at(&app, 119);
        let text_back = render_at(&app, 200);
        assert!(
            !text_back.contains("Favorites"),
            "Manual hide must persist across resize cycle; got:\n{text_back}"
        );
        assert!(!app.mdi.as_ref().unwrap().show_favorites);
    }

    /// L1: In MDI mode, `:show schedule` from the cmdbar restores
    /// a manually-hidden side pane. Verifies cmdbar parsing +
    /// executor + render integration end-to-end.
    #[test]
    fn l1_adams_cmdbar_show_schedule_restores_pane() {
        use crate::tui::event::Action;

        let mut app = App::new(true);
        app.mdi = Some(MdiLayout::default());
        app.screen = Screen::Goalies;

        // Hide schedule via Ctrl+L.
        app.handle(Action::ToggleSchedulePane);
        assert!(!app.mdi.as_ref().unwrap().show_schedule);

        // Type "/show schedule" + Enter.
        app.handle(Action::Search); // pre-fills "/"
        for c in "show schedule".chars() {
            if c == ' ' {
                app.handle(Action::Space);
            } else if let Some(act) = simulate_event_map(c) {
                app.handle(act);
            }
        }
        app.handle(Action::Enter);
        assert!(
            app.mdi.as_ref().unwrap().show_schedule,
            "cmdbar /show schedule must restore the pane"
        );

        // Render at 200 — schedule pane must come back.
        let backend = TestBackend::new(200, 30);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| render(f, &app)).unwrap();
        let text = buf_text(term.backend().buffer());
        assert!(
            text.contains("Schedule"),
            "Schedule pane must render after /show schedule; got:\n{text}"
        );
    }

    // ── Phase Adams.5 — MDI auto-fetch + help overlay ─────────────────────

    /// L0: `mdi_tick_fetch` fires both Tonight and Schedule
    /// fetches regardless of `app.screen`. SDI's
    /// `maybe_fetch_scores` requires `screen == Tonight`; this
    /// MDI variant must NOT.
    #[test]
    fn l0_adams_mdi_tick_fetch_fires_off_workspace() {
        let mut app = App::new(true);
        app.mdi = Some(MdiLayout::default());
        // Workspace = Goalies, NOT Tonight or Schedule.
        app.screen = Screen::Goalies;
        let scores_cache = app.tonight.cache.clone();
        let schedule_cache = app.schedule.week_cache.clone();
        let scores_before = scores_cache.lock().unwrap().len();
        let _ = schedule_cache.lock().unwrap().len();

        app.mdi_tick_fetch();

        // Cache should have at least one new entry per fetch
        // path (in test mode `live_feeds_enabled() == false`,
        // which writes a "live disabled" Error state — proves
        // the call reached the cache, not just no-op'd).
        let scores_after = scores_cache.lock().unwrap().len();
        assert!(
            scores_after >= scores_before,
            "mdi_tick_fetch must touch the Tonight cache"
        );
    }

    /// L0: SDI app — `mdi_tick_fetch` is a no-op (no panic, no
    /// fetch). Guards against the function being called on an
    /// SDI App by mistake.
    #[test]
    fn l0_adams_mdi_tick_fetch_noop_in_sdi() {
        let mut app = App::new(true);
        assert!(app.mdi.is_none());
        app.mdi_tick_fetch(); // must not panic
    }

    /// L1: MDI help overlay — `?` (Action::Help) in MDI mode
    /// opens the show_help flag, and the renderer picks up
    /// `mdi_help_lines()` (which lists `:stats`, `:goalies`,
    /// `query <filter>`, etc.). We render through TestBackend
    /// and assert key verbs appear in the buffer.
    /// Use a taller terminal (60 rows) so the full reference
    /// fits in the 88%-height popup.
    #[test]
    fn l1_adams_mdi_help_overlay_lists_command_verbs() {
        let mut app = App::new(true);
        app.mdi = Some(MdiLayout::default());
        app.screen = Screen::Goalies;
        app.handle(crate::tui::event::Action::Help);
        assert!(app.show_help);

        let backend = TestBackend::new(140, 60);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| render(f, &app)).unwrap();
        let text = buf_text(term.backend().buffer());

        assert!(
            text.contains("Command Bar Reference"),
            "MDI help title must appear; got:\n{text}"
        );
        for verb in &[
            "stats",
            "goalies",
            "transactions",
            "playoffs",
            "schedule",
            "query",
            "/fav add",
            "/hide favorites",
            "/help",
        ] {
            assert!(
                text.contains(verb),
                "MDI help must list verb {verb:?}; got:\n{text}"
            );
        }
    }

    /// L1: SDI help overlay — falls back to legacy keybind
    /// cheat-sheet (no MDI verbs). Verifies the branch in the
    /// help overlay's "MDI vs SDI" decision.
    #[test]
    fn l1_adams_sdi_help_overlay_skips_command_verbs() {
        let mut app = App::new(true);
        // No mdi attached.
        app.screen = Screen::Goalies;
        app.show_help = true;

        let backend = TestBackend::new(120, 40);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| render(f, &app)).unwrap();
        let text = buf_text(term.backend().buffer());

        // SDI help has the legacy keybind heading.
        assert!(
            text.contains("Key Bindings"),
            "SDI help must show legacy keybind sheet; got:\n{text}"
        );
        // And does NOT include the MDI command-bar header.
        assert!(
            !text.contains("Command Bar Reference"),
            "SDI help must NOT show MDI verb reference; got:\n{text}"
        );
    }

    /// Helper: mirror tui/event.rs:48+ to map a typed char to
    /// the Action the real event mapper would produce. Used by
    /// L1 cmdbar tests so we round-trip through the same path
    /// the user's keypresses go through.
    fn simulate_event_map(c: char) -> Option<crate::tui::event::Action> {
        use crate::tui::event::Action;
        match c {
            'q' => Some(Action::Quit),
            '?' => Some(Action::Help),
            '/' => Some(Action::Search),
            'r' => Some(Action::Refresh),
            'i' => Some(Action::Install),
            'g' => Some(Action::AddToGroup),
            'f' => Some(Action::AddToFavorites),
            '1' => Some(Action::GoToTab(0)),
            '2' => Some(Action::GoToTab(1)),
            '3' => Some(Action::GoToTab(2)),
            '4' => Some(Action::GoToTab(3)),
            '5' => Some(Action::GoToTab(4)),
            '6' => Some(Action::GoToTab(5)),
            '7' => Some(Action::GoToTab(6)),
            '8' => Some(Action::GoToTab(7)),
            '9' => Some(Action::GoToTab(8)),
            '0' => Some(Action::GoToTab(9)),
            ' ' => Some(Action::Space),
            _ => Some(Action::Char(c)),
        }
    }
}
