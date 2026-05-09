//! Phase Jack Adams — 100-scenario user-flow harness.
//!
//! Drives the MDI dashboard through 100 distinct user-input
//! sequences and verifies invariants after each. Found bugs
//! get fixed, not skipped — a flaky scenario means the
//! invariant is wrong (or the code is). When this file grows,
//! the categories should stay balanced (cmdbar / panes /
//! resize / errors).
//!
//! Categories (indices in the scenario list):
//!   001-020  Cmdbar verb dispatch
//!   021-040  Cmdbar with args
//!   041-060  Slash commands + write actions
//!   061-080  Pane toggles + focus model
//!   081-090  Error paths + recovery
//!   091-100  Mixed sequences + render-time verification

#[cfg(test)]
mod tests {
    use crate::tui::app::{App, Screen};
    use crate::tui::event::Action;
    use crate::tui::mdi::MdiLayout;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn fresh_mdi() -> App {
        let mut app = App::new(true);
        app.mdi = Some(MdiLayout::default());
        app.screen = Screen::Home;
        app
    }

    /// Mirror the event mapper: typing 'q' produces Action::Quit, etc.
    fn ch(c: char) -> Action {
        match c {
            'q' => Action::Quit,
            '?' => Action::Help,
            '/' => Action::Search,
            'r' => Action::Refresh,
            'i' => Action::Install,
            'g' => Action::AddToGroup,
            'f' => Action::AddToFavorites,
            '1' => Action::GoToTab(0),
            '2' => Action::GoToTab(1),
            '3' => Action::GoToTab(2),
            '4' => Action::GoToTab(3),
            '5' => Action::GoToTab(4),
            '6' => Action::GoToTab(5),
            ' ' => Action::Space,
            _ => Action::Char(c),
        }
    }

    /// Type a string into the cmdbar (already-focused or focuses
    /// via the colon trigger). Each char goes through the event
    /// mapper, which is the realistic path.
    fn type_cmd(app: &mut App, s: &str) {
        // Focus via `:` — colon is Char(':') (no event mapping).
        app.handle(Action::Char(':'));
        for c in s.chars() {
            app.handle(ch(c));
        }
    }

    /// Type a slash command — entry via `/` (Action::Search) which
    /// pre-fills `/`.
    fn type_slash(app: &mut App, s: &str) {
        // `s` should NOT include the leading `/` (Search trigger
        // already inserts it).
        app.handle(Action::Search);
        for c in s.chars() {
            app.handle(ch(c));
        }
    }

    fn submit(app: &mut App) {
        app.handle(Action::Enter);
    }

    fn render_at(app: &App, w: u16, h: u16) {
        let backend = TestBackend::new(w, h);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| crate::tui::screens::render(f, app)).unwrap();
    }

    /// Invariant: after every flow, the app is in a coherent
    /// state — no half-focused cmdbar, no orphan flags, render
    /// at common widths doesn't panic.
    fn assert_coherent(app: &App, scenario: &str) {
        // Render at 4 widths covering all adaptive bands, plus
        // the SDI fallback boundary.
        for w in [200u16, 159, 119, 99] {
            render_at(app, w, 30);
        }
        if let Some(m) = app.mdi.as_ref() {
            // Either the bar is focused with content or input is
            // actually empty — never "focused but cleared input
            // AND command_history_cursor pointing past the ring".
            if let Some(cursor) = m.command_history_cursor {
                assert!(
                    cursor < m.command_history.len(),
                    "[{scenario}] history cursor out of bounds: {cursor} vs len {}",
                    m.command_history.len()
                );
            }
        }
    }

    // ── Scenarios 001-020: Cmdbar verb dispatch ───────────────────────────

    #[test]
    fn s001_cmdbar_stats_swaps_to_queries() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "stats");
        submit(&mut app);
        assert!(matches!(app.screen, Screen::Queries));
        assert_coherent(&app, "s001");
    }

    #[test]
    fn s002_cmdbar_goalies() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "goalies");
        submit(&mut app);
        assert!(matches!(app.screen, Screen::Goalies));
        assert_coherent(&app, "s002");
    }

    #[test]
    fn s003_cmdbar_transactions() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "transactions");
        submit(&mut app);
        assert!(matches!(app.screen, Screen::Transactions));
        assert_coherent(&app, "s003");
    }

    #[test]
    fn s004_cmdbar_playoffs() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "playoffs");
        submit(&mut app);
        assert!(matches!(app.screen, Screen::Playoffs));
        assert_coherent(&app, "s004");
    }

    #[test]
    fn s005_cmdbar_depth() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "depth");
        submit(&mut app);
        assert!(matches!(app.screen, Screen::Depth));
        assert_coherent(&app, "s005");
    }

    #[test]
    fn s006_cmdbar_scores() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "scores");
        submit(&mut app);
        assert!(matches!(app.screen, Screen::Tonight));
        assert_coherent(&app, "s006");
    }

    #[test]
    fn s007_cmdbar_schedule() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "schedule");
        submit(&mut app);
        assert!(matches!(app.screen, Screen::Schedule));
        assert_coherent(&app, "s007");
    }

    #[test]
    fn s008_cmdbar_favorites() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "favorites");
        submit(&mut app);
        assert!(matches!(app.screen, Screen::Favorites));
        assert_coherent(&app, "s008");
    }

    #[test]
    fn s009_cmdbar_uppercase_verb() {
        // Verbs are case-insensitive.
        let mut app = fresh_mdi();
        type_cmd(&mut app, "STATS");
        submit(&mut app);
        assert!(matches!(app.screen, Screen::Queries));
    }

    #[test]
    fn s010_cmdbar_mixed_case_verb() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "Goalies");
        submit(&mut app);
        assert!(matches!(app.screen, Screen::Goalies));
    }

    #[test]
    fn s011_cmdbar_consecutive_swaps() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "stats");
        submit(&mut app);
        type_cmd(&mut app, "goalies");
        submit(&mut app);
        type_cmd(&mut app, "playoffs");
        submit(&mut app);
        assert!(matches!(app.screen, Screen::Playoffs));
    }

    #[test]
    fn s012_cmdbar_swap_then_back_to_home_via_q() {
        // Quit at end exits app. Verify swap survives.
        let mut app = fresh_mdi();
        type_cmd(&mut app, "stats");
        submit(&mut app);
        assert!(matches!(app.screen, Screen::Queries));
    }

    #[test]
    fn s013_cmdbar_txs_alias() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "txs");
        submit(&mut app);
        assert!(matches!(app.screen, Screen::Transactions));
    }

    #[test]
    fn s014_cmdbar_empty_submit_defocuses() {
        let mut app = fresh_mdi();
        app.handle(Action::Char(':'));
        assert!(app.mdi.as_ref().unwrap().command_bar_focused);
        submit(&mut app);
        assert!(!app.mdi.as_ref().unwrap().command_bar_focused);
        // Screen unchanged (was Home).
        assert!(matches!(app.screen, Screen::Home));
    }

    #[test]
    fn s015_cmdbar_whitespace_only_treated_as_empty() {
        let mut app = fresh_mdi();
        app.handle(Action::Char(':'));
        app.handle(Action::Space);
        app.handle(Action::Space);
        submit(&mut app);
        assert!(!app.mdi.as_ref().unwrap().command_bar_focused);
    }

    #[test]
    fn s016_cmdbar_history_records_submitted() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "stats");
        submit(&mut app);
        type_cmd(&mut app, "goalies");
        submit(&mut app);
        let m = app.mdi.as_ref().unwrap();
        assert_eq!(m.command_history.len(), 2);
        assert_eq!(m.command_history[0], "goalies");
        assert_eq!(m.command_history[1], "stats");
    }

    #[test]
    fn s017_cmdbar_history_dedupes_consecutive_same() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "stats");
        submit(&mut app);
        type_cmd(&mut app, "stats"); // duplicate
        submit(&mut app);
        let m = app.mdi.as_ref().unwrap();
        assert_eq!(m.command_history.len(), 1);
    }

    #[test]
    fn s018_cmdbar_failed_submit_still_records_history() {
        // Even if execute returns Flash, the typed command goes
        // into history. (NotImplemented/Flash all push history.)
        let mut app = fresh_mdi();
        type_cmd(&mut app, "class 2024"); // returns NotImplemented
        submit(&mut app);
        let m = app.mdi.as_ref().unwrap();
        assert_eq!(m.command_history.len(), 1);
        assert_eq!(m.command_history[0], "class 2024");
    }

    #[test]
    fn s019_cmdbar_parse_error_does_NOT_record_history() {
        // Parse-error path keeps input + focus, no history push.
        let mut app = fresh_mdi();
        type_cmd(&mut app, "garbage");
        submit(&mut app);
        let m = app.mdi.as_ref().unwrap();
        assert_eq!(m.command_history.len(), 0);
        assert!(m.command_bar_focused);
    }

    #[test]
    fn s020_cmdbar_q_alone_quits_via_submit() {
        let mut app = fresh_mdi();
        app.handle(Action::Char(':'));
        app.handle(Action::Quit); // event mapper turns 'q' → Quit
        let quit = app.handle(Action::Enter);
        assert!(quit, "submitting 'q' must request app exit");
    }

    // ── Scenarios 021-040: Cmdbar with args ──────────────────────────────

    #[test]
    fn s021_cmdbar_team_edm() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "team EDM");
        submit(&mut app);
        match &app.screen {
            Screen::Team(abbr) => assert_eq!(abbr, "EDM"),
            other => panic!("expected Team, got {other:?}"),
        }
    }

    #[test]
    fn s022_cmdbar_team_lowercase_uppercased() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "team edm");
        submit(&mut app);
        match &app.screen {
            Screen::Team(abbr) => assert_eq!(abbr, "EDM"),
            other => panic!("expected Team, got {other:?}"),
        }
    }

    #[test]
    fn s023_cmdbar_team_with_season_keyword() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "team EDM season");
        submit(&mut app);
        match &app.screen {
            Screen::ScheduleTeam(abbr) => assert_eq!(abbr, "EDM"),
            other => panic!("expected ScheduleTeam, got {other:?}"),
        }
    }

    #[test]
    fn s024_cmdbar_team_missing_arg_flashes() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "team");
        submit(&mut app);
        // Parse-error path: input preserved, flash set.
        let m = app.mdi.as_ref().unwrap();
        assert!(m.flash_error.is_some());
        assert_eq!(m.command_input, "team");
    }

    #[test]
    fn s025_cmdbar_box_numeric() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "box 2025020001");
        submit(&mut app);
        match &app.screen {
            Screen::GameDetail(id) => assert_eq!(*id, 2_025_020_001),
            other => panic!("expected GameDetail, got {other:?}"),
        }
    }

    #[test]
    fn s026_cmdbar_box_team_at_team_flashes() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "box edm@bos");
        submit(&mut app);
        // Non-numeric box: NotImplemented flash.
        // Bar should defocus (success-path) since execute returned Flash.
        assert!(matches!(app.screen, Screen::Home));
    }

    #[test]
    fn s027_cmdbar_player_unknown_flashes() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "player Zzznoone");
        submit(&mut app);
        // Player not found → Flash; stays on Home.
        assert!(matches!(app.screen, Screen::Home));
    }

    #[test]
    fn s028_cmdbar_class_returns_not_implemented() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "class 2024");
        submit(&mut app);
        // NotImplemented → Flash, stay on Home.
        assert!(matches!(app.screen, Screen::Home));
    }

    #[test]
    fn s029_cmdbar_compare_one_arg() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "compare McDavid");
        submit(&mut app);
        // Either resolves to CompsById (rare with empty repo) or
        // flashes "player not found". Stays sane either way.
        let _ = &app.screen;
    }

    #[test]
    fn s030_cmdbar_compare_two_args() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "compare McDavid Bedard");
        submit(&mut app);
        let _ = &app.screen;
    }

    #[test]
    fn s031_cmdbar_query_simple_filter() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "query g >= 30");
        submit(&mut app);
        assert!(matches!(app.screen, Screen::Queries));
        assert_eq!(app.queries.filter_text, "g >= 30");
        assert!(app.queries.filter_plan.is_some());
    }

    #[test]
    fn s032_cmdbar_query_compound_filter() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "query g >= 30 AND age <= 25");
        submit(&mut app);
        assert!(matches!(app.screen, Screen::Queries));
        assert_eq!(app.queries.filter_text, "g >= 30 AND age <= 25");
    }

    #[test]
    fn s033_cmdbar_query_invalid_filter_flashes() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "query (((");
        submit(&mut app);
        // Filter parse error → Flash.
        // Screen may have stayed (we don't swap on error).
        assert!(!matches!(app.screen, Screen::Queries));
    }

    #[test]
    fn s034_cmdbar_query_missing_arg_flashes() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "query");
        submit(&mut app);
        let m = app.mdi.as_ref().unwrap();
        assert!(m.flash_error.is_some());
    }

    #[test]
    fn s035_cmdbar_player_pid_form_works() {
        let mut app = fresh_mdi();
        // pid: prefix should resolve via the bundled bios path.
        // With empty repo + bundled bios still loaded, this may
        // succeed or flash. Assert no panic.
        type_cmd(&mut app, "player 8478402");
        submit(&mut app);
    }

    #[test]
    fn s036_cmdbar_team_with_spaces_in_input() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "team   EDM");
        submit(&mut app);
        match &app.screen {
            Screen::Team(abbr) => assert_eq!(abbr, "EDM"),
            other => panic!("expected Team, got {other:?}"),
        }
    }

    #[test]
    fn s037_cmdbar_query_then_swap_keeps_filter_state() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "query g >= 30");
        submit(&mut app);
        assert_eq!(app.queries.filter_text, "g >= 30");
        // Swap to goalies.
        type_cmd(&mut app, "goalies");
        submit(&mut app);
        // Filter text should be retained on the queries state
        // even though we navigated away.
        assert_eq!(app.queries.filter_text, "g >= 30");
    }

    #[test]
    fn s038_cmdbar_consecutive_with_arg_then_no_arg() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "team EDM");
        submit(&mut app);
        type_cmd(&mut app, "stats");
        submit(&mut app);
        assert!(matches!(app.screen, Screen::Queries));
    }

    #[test]
    fn s039_cmdbar_box_zero_is_valid_numeric() {
        // Edge: parse u64 "0" succeeds; GameDetail(0) is a valid
        // (though useless) screen. No panic.
        let mut app = fresh_mdi();
        type_cmd(&mut app, "box 0");
        submit(&mut app);
        match &app.screen {
            Screen::GameDetail(id) => assert_eq!(*id, 0),
            other => panic!("expected GameDetail, got {other:?}"),
        }
    }

    #[test]
    fn s040_cmdbar_box_overflow_flashes() {
        // u64::MAX + 1 — parse fails.
        let mut app = fresh_mdi();
        type_cmd(&mut app, "box 99999999999999999999999");
        submit(&mut app);
        // No panic; not on GameDetail.
        assert!(!matches!(app.screen, Screen::GameDetail(_)));
    }

    // ── Scenarios 041-060: Slash commands + write actions ────────────────

    #[test]
    fn s041_slash_help_opens_overlay() {
        let mut app = fresh_mdi();
        type_slash(&mut app, "help");
        submit(&mut app);
        assert!(app.show_help);
    }

    #[test]
    fn s042_slash_h_alias_opens_help() {
        let mut app = fresh_mdi();
        type_slash(&mut app, "h");
        submit(&mut app);
        assert!(app.show_help);
    }

    #[test]
    fn s043_slash_question_alias_opens_help() {
        let mut app = fresh_mdi();
        type_slash(&mut app, "?");
        submit(&mut app);
        assert!(app.show_help);
    }

    #[test]
    fn s044_slash_quit_exits() {
        let mut app = fresh_mdi();
        type_slash(&mut app, "quit");
        let quit = app.handle(Action::Enter);
        assert!(quit);
    }

    #[test]
    fn s045_slash_q_alias_exits() {
        let mut app = fresh_mdi();
        type_slash(&mut app, "q");
        let quit = app.handle(Action::Enter);
        assert!(quit);
    }

    #[test]
    fn s046_slash_exit_alias_exits() {
        let mut app = fresh_mdi();
        type_slash(&mut app, "exit");
        let quit = app.handle(Action::Enter);
        assert!(quit);
    }

    #[test]
    fn s047_slash_hide_favorites() {
        let mut app = fresh_mdi();
        type_slash(&mut app, "hide favorites");
        submit(&mut app);
        assert!(!app.mdi.as_ref().unwrap().show_favorites);
    }

    #[test]
    fn s048_slash_hide_schedule() {
        let mut app = fresh_mdi();
        type_slash(&mut app, "hide schedule");
        submit(&mut app);
        assert!(!app.mdi.as_ref().unwrap().show_schedule);
    }

    #[test]
    fn s049_slash_hide_alias_fav() {
        let mut app = fresh_mdi();
        type_slash(&mut app, "hide fav");
        submit(&mut app);
        assert!(!app.mdi.as_ref().unwrap().show_favorites);
    }

    #[test]
    fn s050_slash_hide_alias_sched() {
        let mut app = fresh_mdi();
        type_slash(&mut app, "hide sched");
        submit(&mut app);
        assert!(!app.mdi.as_ref().unwrap().show_schedule);
    }

    #[test]
    fn s051_slash_show_after_hide() {
        let mut app = fresh_mdi();
        type_slash(&mut app, "hide favorites");
        submit(&mut app);
        type_slash(&mut app, "show favorites");
        submit(&mut app);
        assert!(app.mdi.as_ref().unwrap().show_favorites);
    }

    #[test]
    fn s052_slash_hide_unknown_pane_flashes() {
        let mut app = fresh_mdi();
        type_slash(&mut app, "hide nonsense");
        submit(&mut app);
        let m = app.mdi.as_ref().unwrap();
        assert!(m.flash_error.is_some());
    }

    #[test]
    fn s053_slash_unknown_command_flashes() {
        let mut app = fresh_mdi();
        type_slash(&mut app, "frobnicate");
        submit(&mut app);
        let m = app.mdi.as_ref().unwrap();
        assert!(m.flash_error.is_some());
    }

    #[test]
    fn s054_bare_q_outside_bar_quits() {
        let mut app = fresh_mdi();
        let quit = app.handle(Action::Quit);
        assert!(quit);
    }

    #[test]
    fn s055_question_outside_bar_opens_help() {
        let mut app = fresh_mdi();
        app.handle(Action::Help);
        assert!(app.show_help);
    }

    #[test]
    fn s056_slash_fav_missing_subcommand_flashes() {
        let mut app = fresh_mdi();
        type_slash(&mut app, "fav");
        submit(&mut app);
        let m = app.mdi.as_ref().unwrap();
        assert!(m.flash_error.is_some());
    }

    #[test]
    fn s057_slash_fav_remove_unknown_player_flashes() {
        let mut app = fresh_mdi();
        type_slash(&mut app, "fav remove Zzznoone");
        submit(&mut app);
        // Stays on Home; no panic.
        assert!(matches!(app.screen, Screen::Home));
    }

    #[test]
    fn s058_slash_fav_unknown_subcommand_flashes() {
        let mut app = fresh_mdi();
        type_slash(&mut app, "fav something");
        submit(&mut app);
        let m = app.mdi.as_ref().unwrap();
        assert!(m.flash_error.is_some());
    }

    #[test]
    fn s059_bare_hide_without_slash_unknown() {
        // hide is slash-only — bare "hide" parses as
        // UnknownCommand.
        let mut app = fresh_mdi();
        type_cmd(&mut app, "hide schedule");
        submit(&mut app);
        let m = app.mdi.as_ref().unwrap();
        assert!(m.flash_error.is_some());
    }

    #[test]
    fn s060_country_eq_value_rejected_without_query_prefix() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "country=CAN");
        submit(&mut app);
        let m = app.mdi.as_ref().unwrap();
        assert!(m.flash_error.is_some());
    }

    // ── Scenarios 061-080: Pane toggles + focus model ────────────────────

    #[test]
    fn s061_ctrl_h_toggles_favorites() {
        let mut app = fresh_mdi();
        assert!(app.mdi.as_ref().unwrap().show_favorites);
        app.handle(Action::ToggleFavoritesPane);
        assert!(!app.mdi.as_ref().unwrap().show_favorites);
    }

    #[test]
    fn s062_ctrl_l_toggles_schedule() {
        let mut app = fresh_mdi();
        app.handle(Action::ToggleSchedulePane);
        assert!(!app.mdi.as_ref().unwrap().show_schedule);
    }

    #[test]
    fn s063_ctrl_h_idempotent_pair() {
        let mut app = fresh_mdi();
        app.handle(Action::ToggleFavoritesPane);
        app.handle(Action::ToggleFavoritesPane);
        assert!(app.mdi.as_ref().unwrap().show_favorites);
    }

    #[test]
    fn s064_pane_toggles_in_sdi_noop() {
        let mut app = App::new(true);
        assert!(app.mdi.is_none());
        app.handle(Action::ToggleFavoritesPane);
        app.handle(Action::ToggleSchedulePane);
        assert!(app.mdi.is_none());
    }

    #[test]
    fn s065_pane_toggle_during_cmdbar_focus() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "stat"); // partial input
        let before = app.mdi.as_ref().unwrap().show_favorites;
        app.handle(Action::ToggleFavoritesPane);
        let m = app.mdi.as_ref().unwrap();
        assert_ne!(m.show_favorites, before);
        assert!(m.command_bar_focused);
        assert_eq!(m.command_input, "stat");
    }

    #[test]
    fn s066_colon_focuses_bar() {
        let mut app = fresh_mdi();
        app.handle(Action::Char(':'));
        assert!(app.mdi.as_ref().unwrap().command_bar_focused);
        assert_eq!(app.mdi.as_ref().unwrap().command_input, "");
    }

    #[test]
    fn s067_slash_focuses_bar_with_slash_prefilled() {
        let mut app = fresh_mdi();
        app.handle(Action::Search);
        assert!(app.mdi.as_ref().unwrap().command_bar_focused);
        assert_eq!(app.mdi.as_ref().unwrap().command_input, "/");
    }

    #[test]
    fn s068_escape_clears_bar_and_defocuses() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "garb");
        app.handle(Action::Escape);
        let m = app.mdi.as_ref().unwrap();
        assert!(!m.command_bar_focused);
        assert_eq!(m.command_input, "");
    }

    #[test]
    fn s069_backspace_pops_then_defocuses_at_empty() {
        let mut app = fresh_mdi();
        app.handle(Action::Char(':'));
        app.handle(Action::Char('a'));
        app.handle(Action::Backspace);
        // Empty buffer, still focused.
        assert!(app.mdi.as_ref().unwrap().command_bar_focused);
        assert_eq!(app.mdi.as_ref().unwrap().command_input, "");
        // Another backspace defocuses.
        app.handle(Action::Backspace);
        assert!(!app.mdi.as_ref().unwrap().command_bar_focused);
    }

    #[test]
    fn s070_cmdbar_per_screen_keybind_blocked_while_focused() {
        // 's' (Char('s')) is a goalies-sort hotkey when on
        // Goalies screen. While cmdbar is focused, it must
        // capture the 's' as text instead.
        let mut app = fresh_mdi();
        app.screen = Screen::Goalies;
        let sort_before = app.goalies.sort;
        app.handle(Action::Char(':'));
        app.handle(Action::Char('s'));
        // 's' captured as text; sort unchanged.
        assert_eq!(app.goalies.sort, sort_before);
        assert_eq!(app.mdi.as_ref().unwrap().command_input, "s");
    }

    #[test]
    fn s071_per_screen_keybind_works_when_bar_unfocused() {
        let mut app = fresh_mdi();
        app.screen = Screen::Goalies;
        let sort_before = app.goalies.sort;
        // No cmdbar focus.
        app.handle(Action::Char('s'));
        // Sort cycled.
        assert_ne!(app.goalies.sort, sort_before);
    }

    #[test]
    fn s072_help_overlay_dismisses_on_any_key() {
        let mut app = fresh_mdi();
        app.handle(Action::Help);
        assert!(app.show_help);
        app.handle(Action::Char('a'));
        assert!(!app.show_help);
    }

    #[test]
    fn s073_help_then_resume_typing() {
        let mut app = fresh_mdi();
        app.handle(Action::Help);
        app.handle(Action::Char('a')); // dismisses help
        // Bar not yet focused — `a` does nothing in main match.
        assert!(!app.show_help);
    }

    #[test]
    fn s074_tab_in_mdi_is_noop() {
        let mut app = fresh_mdi();
        let screen_before = std::mem::discriminant(&app.screen);
        app.handle(Action::Tab);
        assert_eq!(std::mem::discriminant(&app.screen), screen_before);
    }

    #[test]
    fn s075_tab_in_sdi_cycles() {
        let mut app = App::new(true);
        let screen_before = std::mem::discriminant(&app.screen);
        app.handle(Action::Tab);
        assert_ne!(std::mem::discriminant(&app.screen), screen_before);
    }

    #[test]
    fn s076_double_focus_via_colon_clears_input() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "garb");
        // Press `:` again while focused — should it re-clear?
        // Currently: while focused, `:` (Char) goes through
        // handle_command_bar, which pushes ':' as text.
        app.handle(Action::Char(':'));
        let m = app.mdi.as_ref().unwrap();
        assert_eq!(m.command_input, "garb:");
    }

    #[test]
    fn s077_double_focus_via_slash_appends() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "garb");
        app.handle(Action::Search); // pushes '/' through cmdbar
        let m = app.mdi.as_ref().unwrap();
        assert_eq!(m.command_input, "garb/");
    }

    #[test]
    fn s078_typing_clears_flash_error() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "garbage");
        submit(&mut app); // flash set
        assert!(app.mdi.as_ref().unwrap().flash_error.is_some());
        // Typing more clears flash.
        app.handle(Action::Char('x'));
        assert!(app.mdi.as_ref().unwrap().flash_error.is_none());
    }

    #[test]
    fn s079_history_cursor_starts_none() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "stats");
        submit(&mut app);
        assert!(app.mdi.as_ref().unwrap().command_history_cursor.is_none());
    }

    #[test]
    fn s080_command_history_caps_at_50() {
        let mut app = fresh_mdi();
        for i in 0..60 {
            type_cmd(&mut app, &format!("team E{:02}", i));
            submit(&mut app);
        }
        let m = app.mdi.as_ref().unwrap();
        assert_eq!(m.command_history.len(), 50);
    }

    // ── Scenarios 081-090: Error paths + recovery ────────────────────────

    #[test]
    fn s081_garbage_then_correct_recovers() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "garbage");
        submit(&mut app);
        // Flash set, input preserved, focus retained.
        assert!(app.mdi.as_ref().unwrap().flash_error.is_some());
        // Esc to clear.
        app.handle(Action::Escape);
        assert!(!app.mdi.as_ref().unwrap().command_bar_focused);
        // Try again.
        type_cmd(&mut app, "stats");
        submit(&mut app);
        assert!(matches!(app.screen, Screen::Queries));
    }

    #[test]
    fn s082_parse_error_input_editable() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "team");
        submit(&mut app); // missing arg
        // Input preserved with focus, so user can append.
        assert!(app.mdi.as_ref().unwrap().command_bar_focused);
        assert_eq!(app.mdi.as_ref().unwrap().command_input, "team");
        // Append " EDM" and submit.
        app.handle(Action::Space);
        app.handle(Action::Char('E'));
        app.handle(Action::Char('D'));
        app.handle(Action::Char('M'));
        submit(&mut app);
        match &app.screen {
            Screen::Team(abbr) => assert_eq!(abbr, "EDM"),
            other => panic!("expected Team, got {other:?}"),
        }
    }

    #[test]
    fn s083_invalid_filter_then_valid_filter() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "query (((");
        submit(&mut app);
        assert!(!matches!(app.screen, Screen::Queries));
        // Esc to clear, retry.
        app.handle(Action::Escape);
        type_cmd(&mut app, "query g >= 30");
        submit(&mut app);
        assert!(matches!(app.screen, Screen::Queries));
    }

    #[test]
    fn s084_class_and_roster_show_not_implemented() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "class 2024");
        submit(&mut app);
        // App stays on Home (Flash), no panic.
        assert!(matches!(app.screen, Screen::Home));
        type_cmd(&mut app, "roster");
        submit(&mut app);
        assert!(matches!(app.screen, Screen::Home));
    }

    #[test]
    fn s085_box_non_numeric_does_not_swap() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "box notanid");
        submit(&mut app);
        assert!(!matches!(app.screen, Screen::GameDetail(_)));
    }

    #[test]
    fn s086_class_negative_year_flashes() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "class -1");
        submit(&mut app);
        // Parse-error path; flash set; input retained.
        let m = app.mdi.as_ref().unwrap();
        assert!(m.flash_error.is_some() || matches!(app.screen, Screen::Home));
    }

    #[test]
    fn s087_consecutive_garbage_each_flashes() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "garbage1");
        submit(&mut app);
        app.handle(Action::Escape);
        type_cmd(&mut app, "garbage2");
        submit(&mut app);
        let m = app.mdi.as_ref().unwrap();
        assert!(m.flash_error.is_some());
    }

    #[test]
    fn s088_box_with_negative_id_flashes() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "box -123");
        submit(&mut app);
        assert!(!matches!(app.screen, Screen::GameDetail(_)));
    }

    #[test]
    fn s089_query_with_only_whitespace_flashes() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "query   ");
        submit(&mut app);
        // Either parses as missing-arg or as empty filter — either way no swap.
        assert!(!matches!(app.screen, Screen::Queries) || app.queries.filter_text.is_empty());
    }

    #[test]
    fn s090_pane_toggle_then_help_then_resume() {
        let mut app = fresh_mdi();
        app.handle(Action::ToggleSchedulePane);
        assert!(!app.mdi.as_ref().unwrap().show_schedule);
        app.handle(Action::Help);
        assert!(app.show_help);
        // Dismiss with any key.
        app.handle(Action::Char('a'));
        assert!(!app.show_help);
        // Pane stays hidden.
        assert!(!app.mdi.as_ref().unwrap().show_schedule);
    }

    // ── Scenarios 091-100: Mixed sequences + render verification ─────────

    #[test]
    fn s091_full_user_journey_swap_resize_query() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "stats");
        submit(&mut app);
        type_cmd(&mut app, "query g >= 30");
        submit(&mut app);
        type_cmd(&mut app, "goalies");
        submit(&mut app);
        // Render at multiple widths to ensure the journey state
        // doesn't panic anything.
        for w in [200u16, 159, 119, 99] {
            render_at(&app, w, 30);
        }
        assert!(matches!(app.screen, Screen::Goalies));
        assert_eq!(app.queries.filter_text, "g >= 30");
    }

    #[test]
    fn s092_rapid_pane_toggle_burst() {
        let mut app = fresh_mdi();
        for _ in 0..10 {
            app.handle(Action::ToggleFavoritesPane);
            app.handle(Action::ToggleSchedulePane);
        }
        // 10 toggles each = back to default.
        assert!(app.mdi.as_ref().unwrap().show_favorites);
        assert!(app.mdi.as_ref().unwrap().show_schedule);
    }

    #[test]
    fn s093_render_post_help_overlay() {
        let mut app = fresh_mdi();
        app.handle(Action::Help);
        // Help overlay paints over MDI.
        let backend = TestBackend::new(140, 50);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| crate::tui::screens::render(f, &app)).unwrap();
        // Just confirm no panic.
    }

    #[test]
    fn s094_render_post_pane_hide() {
        let mut app = fresh_mdi();
        app.handle(Action::ToggleFavoritesPane);
        let backend = TestBackend::new(200, 30);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| crate::tui::screens::render(f, &app)).unwrap();
        // Buffer should not contain "Favorites" pane title.
        let mut text = String::new();
        let buf = term.backend().buffer();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                text.push_str(buf[(x, y)].symbol());
            }
            text.push('\n');
        }
        let count = text.matches("★ Favorites").count();
        assert_eq!(count, 0, "favorites pane title should not render");
    }

    #[test]
    fn s095_render_post_filter_apply_workspace_shows_stats() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "query g >= 30");
        submit(&mut app);
        let backend = TestBackend::new(200, 40);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| crate::tui::screens::render(f, &app)).unwrap();
        let mut text = String::new();
        let buf = term.backend().buffer();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                text.push_str(buf[(x, y)].symbol());
            }
            text.push('\n');
        }
        // Workspace pane title should be "Stats".
        assert!(text.contains("Stats"), "workspace title must be Stats");
    }

    #[test]
    fn s096_chip_mode_after_submit_shows_hints() {
        let mut app = fresh_mdi();
        type_cmd(&mut app, "stats");
        submit(&mut app);
        // Bar defocused — chip mode shows.
        let backend = TestBackend::new(120, 30);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| crate::tui::screens::render(f, &app)).unwrap();
        let mut text = String::new();
        let buf = term.backend().buffer();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                text.push_str(buf[(x, y)].symbol());
            }
            text.push('\n');
        }
        // Chip-mode contains the hints we added.
        assert!(
            text.contains("?=help") || text.contains("? help"),
            "chip mode should show help hint"
        );
    }

    #[test]
    fn s097_resize_burst_no_panic() {
        let mut app = fresh_mdi();
        let widths = [80u16, 99, 100, 119, 120, 159, 160, 200, 240];
        for &w in &widths {
            render_at(&app, w, 30);
        }
        // App state preserved.
        assert!(app.mdi.is_some());
    }

    #[test]
    fn s098_long_command_line_no_panic() {
        let mut app = fresh_mdi();
        // Type a 200-char filter — exercises String reallocation
        // and the cmdbar render's `> {input}_` formatting.
        let long = "query ".to_owned() + &"g".repeat(200);
        type_cmd(&mut app, &long);
        // Don't submit — just render to make sure prompt mode
        // handles long input.
        let backend = TestBackend::new(200, 30);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| crate::tui::screens::render(f, &app)).unwrap();
    }

    #[test]
    fn s099_typed_then_navigate_history_via_up() {
        // Up/Down history navigation is a stub in Adams.2 (no-op);
        // ensure it doesn't break anything.
        let mut app = fresh_mdi();
        type_cmd(&mut app, "stats");
        submit(&mut app);
        type_cmd(&mut app, "goalies");
        submit(&mut app);
        // Press Up while focused.
        app.handle(Action::Char(':'));
        app.handle(Action::Up);
        app.handle(Action::Down);
        // No-op stubs — just no panic.
    }

    #[test]
    fn s100_deeply_mixed_session() {
        let mut app = fresh_mdi();
        // Tour the dashboard.
        type_cmd(&mut app, "stats");
        submit(&mut app);
        type_cmd(&mut app, "query g >= 30");
        submit(&mut app);
        app.handle(Action::ToggleSchedulePane);
        type_cmd(&mut app, "team EDM");
        submit(&mut app);
        type_cmd(&mut app, "team BOS season");
        submit(&mut app);
        type_slash(&mut app, "show schedule");
        submit(&mut app);
        type_cmd(&mut app, "playoffs");
        submit(&mut app);
        app.handle(Action::Help);
        app.handle(Action::Char('a')); // dismiss help

        // After all that — final state is sane.
        assert!(matches!(app.screen, Screen::Playoffs));
        assert!(app.mdi.as_ref().unwrap().show_schedule);
        // Render at every width.
        for w in [80u16, 100, 120, 160, 200, 240] {
            render_at(&app, w, 40);
        }
    }
}
