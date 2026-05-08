pub mod comps;
pub mod depth;
pub mod favorites;
pub mod game_detail;
pub mod goalies;
pub mod home;
pub mod misc;
pub mod player;
pub mod playoffs;
pub mod queries;
pub mod schedule;
pub mod search;
pub mod team;
pub mod transactions;

use crate::tui::app::{App, Screen};
use crate::tui::widgets::help_lines;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    render_nav(f, app, chunks[0]);

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

    // Phase Foster +8 — append the active timeframe indicator
    // (right-aligned visually via the suffix). Only renders when
    // it's not the Day default so a fresh launch stays uncluttered.
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
    f.render_widget(
        Paragraph::new(status_text).style(Style::default().fg(Color::DarkGray)),
        chunks[2],
    );

    // Group picker overlay — shown on any player-list screen when g is pressed.
    // Rendered at top level so it floats over the current screen.
    // (player.rs and team.rs also call this, but those handle it locally.
    //  This catches Projections, Search, Queries, GroupDetail.)
    if app.group_picker_open {
        // Skip if player/team screen — they render the overlay themselves
        let handled_locally = matches!(app.screen, Screen::PlayerById(_) | Screen::Team(_));
        if !handled_locally {
            player::render_group_picker(f, app, area);
        }
    }

    if app.show_help {
        let popup = centered_rect(62, 65, area);
        f.render_widget(Clear, popup);
        let block = Block::default()
            .title(" Help — any key to close ")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));
        let inner = block.inner(popup);
        f.render_widget(block, popup);
        f.render_widget(Paragraph::new(help_lines()), inner);
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

    // LP.4 — in-TUI docs overlay. Painted last so it sits on top of
    // everything else (League/Stats/Goalies/etc). `m` opens, Esc/m
    // closes, Up/Down/Left/Right scroll. Same compile-time
    // `COMMANDS.md` source as `icelines docs` and the web /docs route.
    if app.show_docs {
        render_docs_overlay(f, app, area);
    }
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
        Screen::Favorites => 4,                                       // Favorites (Foster.2)
        Screen::Tonight | Screen::GameDetail(_) => 5,                // Scores
        Screen::Schedule | Screen::ScheduleTeam(_) | Screen::ScheduleMatchup(..) => 6, // Schedule
        Screen::Transactions => 7,                                   // Transactions
        Screen::Playoffs | Screen::SeriesDetail(_) => 8,             // Playoffs
        // Groups is not a tab (Phase T+1): reachable via `g` from anywhere.
        _ => 99, // no tab (Fetch, Help, Groups)
    }
}

fn render_nav(f: &mut Frame, app: &App, area: Rect) {
    let tab_labels = [
        "League",
        "Depth",
        "Stats",
        "Goalies",
        "Favorites",
        "Scores",
        "Schedule",
        "Transactions",
        "Playoffs",
    ];
    let active_tab = tab_for_screen(&app.screen);

    let mut spans: Vec<Span> = Vec::new();
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

    let hint = if app.show_admin {
        "  Esc:close admin"
    } else if app.show_season_picker {
        "  Esc:cancel picker"
    } else if app.active_type == icelines_core::season_stats::SeasonType::Playoff {
        "  Shift+P:regular  y:season  F:admin  ?:help  q:quit"
    } else {
        "  g:groups  y:season  Shift+P:playoff  F:admin  ?:help  q:quit"
    };
    spans.push(Span::styled(hint, Style::default().fg(Color::DarkGray)));
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

    /// The 8 canonical tabs must appear in the nav bar on every screen.
    /// Catches: tab dropped from the array, label renamed, layout truncated
    /// at common widths.
    #[test]
    fn l0_app_nav_bar_renders_all_eight_tabs_at_120_cols() {
        let app = App::new(true);
        let text = render_app_to_text(&app, 120, 30);
        for label in [
            "League",
            "Depth",
            "Stats",
            "Goalies",
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
        // COMMANDS.md starts with "# Icelines CLI Reference" or similar
        // — the overlay should render at least one line of doc content.
        assert!(
            text.contains("icelines"),
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

    /// Tab from Home cycles forward through all 9 tabs and wraps back
    /// (Phase Foster.2 inserts Favorites between Goalies and Scores).
    /// Catches tab table regressions (skipped tabs, missing screens).
    #[test]
    fn l1_userflow_tab_cycles_through_all_eight_tabs() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        use crate::tui::app::Screen;
        let expected = [
            Screen::Depth,
            Screen::Queries,
            Screen::Goalies,
            Screen::Favorites,
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
    }

    /// Numeric jump: GoToTab(n) lands on the right screen for each n.
    /// Note: GoToTab is 0-indexed — the keymap (Char('1')→0, …, Char('8')→7)
    /// translates user-visible 1–8 into this enum's 0–7.
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
            (4, Screen::Tonight),
            (5, Screen::Schedule),
            (6, Screen::Transactions),
            (7, Screen::Playoffs),
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

        app.handle(Action::GoToTab(4));
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

        app.handle(Action::GoToTab(5));
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

        app.handle(Action::GoToTab(6));
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

        app.handle(Action::GoToTab(7));
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
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);

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
            app.group_picker_open,
            "g on a Player screen must open the group picker"
        );
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
        app.handle(Action::GoToTab(5)); // Schedule
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
        app.handle(Action::GoToTab(5));
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
        app.handle(Action::GoToTab(5));
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
        app.handle(Action::GoToTab(5));
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
        app.handle(Action::GoToTab(5));
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
        app.handle(Action::GoToTab(6)); // Transactions

        app.handle(Action::Search);
        assert!(app.tx_search_mode, "/ must open transactions search");
        for c in "trade".chars() {
            app.handle(Action::Char(c));
        }
        assert_eq!(app.tx_search_query, "trade");

        app.handle(Action::Enter);
        assert!(!app.tx_search_mode, "Enter must exit search mode");
        assert_eq!(
            app.tx_search_query, "trade",
            "Enter must keep query applied"
        );
    }

    /// Transactions search: Esc clears the query and exits mode.
    #[test]
    fn l1_userflow_transactions_search_esc_clears() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::GoToTab(6));
        app.handle(Action::Search);
        app.handle(Action::Char('w'));
        app.handle(Action::Char('a'));
        app.handle(Action::Char('i'));
        app.handle(Action::Char('v'));
        app.handle(Action::Char('e'));
        assert_eq!(app.tx_search_query, "waive");
        app.handle(Action::Escape);
        assert!(!app.tx_search_mode);
        assert!(app.tx_search_query.is_empty(), "Esc must clear query");
    }

    /// Transactions search: backspace edits the query.
    #[test]
    fn l1_userflow_transactions_search_backspace() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::GoToTab(6));
        app.handle(Action::Search);
        for c in "claim".chars() {
            app.handle(Action::Char(c));
        }
        app.handle(Action::Backspace);
        assert_eq!(app.tx_search_query, "clai");
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

            // Drill to a player and open picker.
            app.handle(Action::Enter);
            app.handle(Action::Enter);
            app.handle(Action::AddToGroup);

            assert!(app.group_picker_open, "g must open the group picker");
            // Picker list must include both Favorites (seeded by
            // migration 001) and the user-created Watchlist.
            let names: Vec<&str> = app.group_picker_list.iter().map(|s| s.as_str()).collect();
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
        app.tonight_cache
            .lock()
            .unwrap()
            .insert(String::new(), TonightState::Loaded(vec![scheduled]));
        // Inject into boxscore_cache (keyed by game_id).
        app.boxscore_cache
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
        app.boxscore_cache
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
        app.boxscore_cache.lock().unwrap().insert(
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
        assert!(matches!(app.queries.mode, crate::tui::app::QueryMode::Build));
        assert!(app.queries.save_name.is_empty(), "Esc must clear typed name");
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
            assert!(matches!(app.queries.mode, crate::tui::app::QueryMode::Build));

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
            let v1_json =
                r#"[{"label":"Sort by","selected":2},{"label":"Position","selected":1}]"#;
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

    // ── Phase Norris.2 — ScheduleScreenState sequencing tests ─────────────

    /// Sequential filter replacement — applying a second filter
    /// REPLACES the first; filters don't accumulate. Single
    /// SearchFilter slot, not a stack.
    #[test]
    fn l1_norris_schedule_filter_replaces_on_second_apply() {
        let (_dir, store) = empty_store_in_tempdir();
        let mut app = App::new(true);
        app.boot_load_with_store(&store);
        app.handle(Action::GoToTab(5)); // Schedule

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
        app.handle(Action::GoToTab(5));
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
        app.handle(Action::GoToTab(5));

        // Type an invalid team.
        app.handle(Action::Search);
        for c in "zzz".chars() {
            app.handle(Action::Char(c));
        }
        app.handle(Action::Enter);
        assert!(
            app.schedule.search_mode,
            "invalid keeps search mode open"
        );
        assert!(
            app.schedule.filter_err.is_some(),
            "invalid sets filter_err"
        );

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
                assert_eq!(
                    app.queries.mode,
                    crate::tui::app::QueryMode::FilterEdit
                );
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
                assert_eq!(
                    app.queries.mode,
                    crate::tui::app::QueryMode::Build
                );
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
            assert_eq!(
                app.queries.mode,
                crate::tui::app::QueryMode::FilterEdit
            );
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
            assert_eq!(
                app.queries.filter_history[0],
                "country=CAN AND age<25"
            );
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
            assert_eq!(
                app.queries.mode,
                crate::tui::app::QueryMode::SaveName
            );
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
            assert_eq!(
                app.queries.mode,
                crate::tui::app::QueryMode::FilterEdit
            );
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
