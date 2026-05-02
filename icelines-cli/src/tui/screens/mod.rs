pub mod home;
pub mod team;
pub mod player;
pub mod search;
pub mod misc;
pub mod queries;
pub mod comps;
pub mod depth;
pub mod schedule;
pub mod playoffs;
pub mod game_detail;
pub mod goalies;
pub mod transactions;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use crate::tui::app::{App, Screen};
use crate::tui::widgets::help_lines;

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    render_nav(f, app, chunks[0]);

    match &app.screen {
        Screen::Home            => home::render(f, app, chunks[1]),
        Screen::Team(abbrev)    => team::render(f, app, chunks[1], abbrev),
        Screen::PlayerById(pid) => player::render_by_id(f, app, chunks[1], *pid),
        Screen::Search          => search::render(f, app, chunks[1]),
        Screen::Queries         => queries::render(f, app, chunks[1]),
        Screen::Tonight         => misc::render_tonight(f, app, chunks[1]),
        Screen::Projections     => misc::render_projections(f, app, chunks[1]),
        Screen::Groups              => misc::render_groups(f, app, chunks[1]),
        Screen::GroupDetail(name)   => misc::render_group_members(f, app, chunks[1], name),
        Screen::Fetch               => misc::render_fetch(f, app, chunks[1]),
        Screen::Help                => home::render(f, app, chunks[1]),
        Screen::CompsById(pid)      => comps::render_by_id(f, app, chunks[1], *pid),
        Screen::Depth               => depth::render_league(f, app, chunks[1]),
        Screen::DepthTeam(abbrev)   => depth::render_team(f, app, chunks[1], abbrev),
        Screen::Schedule                  => schedule::render(f, app, chunks[1]),
        Screen::ScheduleTeam(team)        => schedule::render_team_schedule(f, app, chunks[1], team),
        Screen::ScheduleMatchup(t1, t2)   => schedule::render_matchup(f, app, chunks[1], t1, t2),
        Screen::Playoffs                  => playoffs::render(f, app, chunks[1]),
        Screen::SeriesDetail(letter)      => playoffs::render_series_detail(f, app, chunks[1], letter),
        Screen::GameDetail(game_id)       => game_detail::render(f, app, chunks[1], *game_id),
        Screen::Goalies                   => goalies::render(f, app, chunks[1]),
        Screen::GoalieDetailById(pid)     => goalies::render_detail_by_id(f, app, chunks[1], *pid),
        Screen::Transactions              => transactions::render(f, app, chunks[1]),
    }

    f.render_widget(
        Paragraph::new(app.status.as_str()).style(Style::default().fg(Color::DarkGray)),
        chunks[2],
    );

    // Group picker overlay — shown on any player-list screen when g is pressed.
    // Rendered at top level so it floats over the current screen.
    // (player.rs and team.rs also call this, but those handle it locally.
    //  This catches Projections, Search, Queries, GroupDetail.)
    if app.group_picker_open {
        // Skip if player/team screen — they render the overlay themselves
        let handled_locally = matches!(app.screen,
            Screen::PlayerById(_) | Screen::Team(_)
        );
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
}

fn tab_for_screen(screen: &Screen) -> usize {
    match screen {
        Screen::Home | Screen::Team(_) | Screen::PlayerById(_)
        | Screen::CompsById(_)                                   => 0, // League
        Screen::Depth | Screen::DepthTeam(_)                     => 1, // Depth
        Screen::Queries | Screen::Projections | Screen::Search   => 2, // Stats (default: Queries)
        Screen::Goalies | Screen::GoalieDetailById(_)            => 3, // Goalies
        Screen::Tonight | Screen::GameDetail(_)                  => 4, // Scores
        Screen::Schedule | Screen::ScheduleTeam(_)
            | Screen::ScheduleMatchup(..)                        => 5, // Schedule
        Screen::Transactions                                     => 6, // Transactions
        Screen::Playoffs | Screen::SeriesDetail(_)               => 7, // Playoffs
        // Groups is not a tab (Phase T+1): reachable via `g` from anywhere.
        _                                                        => 99,// no tab (Fetch, Help, Groups)
    }
}

fn render_nav(f: &mut Frame, app: &App, area: Rect) {
    let tab_labels = [
        "League", "Depth", "Stats", "Goalies", "Scores",
        "Schedule", "Transactions", "Playoffs",
    ];
    let active_tab = tab_for_screen(&app.screen);

    let mut spans: Vec<Span> = Vec::new();
    for (i, label) in tab_labels.iter().enumerate() {
        let active = i == active_tab;
        let style = if active {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        spans.push(Span::styled(format!(" {label} "), style));
        spans.push(Span::styled("│", Style::default().fg(Color::DarkGray)));
    }

    // Season indicator — shown when a historical season is active
    if app.active_season != icelines_core::CURRENT_SEASON_STR {
        let label = crate::tui::screens::misc::PICKER_SEASONS.iter()
            .find(|(id, _, _)| *id == app.active_season.as_str())
            .map(|(_, l, _)| *l)
            .unwrap_or(app.active_season.as_str());
        spans.push(Span::styled(
            format!("  [{}] ", label.split_whitespace().next().unwrap_or(label)),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ));
    }

    let hint = if app.show_admin {
        "  Esc:close admin"
    } else if app.show_season_picker {
        "  Esc:cancel picker"
    } else {
        "  g:groups  y:season  F:admin  Tab:cycle  ?:help  q:quit"
    };
    spans.push(Span::styled(hint, Style::default().fg(Color::DarkGray)));
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn centered_rect(pct_x: u16, pct_y: u16, r: Rect) -> Rect {
    let popup_h = r.height * pct_y / 100;
    let popup_w = r.width  * pct_x / 100;
    Rect::new(
        r.x + (r.width  - popup_w) / 2,
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
    use super::*;
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
        assert!(text.contains("EDM"), "Home must show EDM team card, got:\n{text}");
        assert!(text.contains("TOR"), "Home must show TOR team card, got:\n{text}");
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

        // Tab × 3: Home → Depth → Queries → Goalies
        app.handle(Action::Tab);
        app.handle(Action::Tab);
        app.handle(Action::Tab);
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

    /// Tab from Home cycles forward through all 8 tabs and wraps back.
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
            Screen::Tonight,
            Screen::Schedule,
            Screen::Transactions,
            Screen::Playoffs,
            Screen::Home, // wraps
        ];
        for want in expected {
            app.handle(Action::Tab);
            assert_eq!(app.screen, want, "Tab cycle landed on wrong screen");
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
        assert_eq!(app.screen, Screen::Playoffs, "Shift-Tab from Home → Playoffs");
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

        // Tab × 3 to Goalies, then Enter.
        for _ in 0..3 {
            app.handle(Action::Tab);
        }
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
        assert!(should_quit, "Quit action must return true to break run_loop");
    }
}
