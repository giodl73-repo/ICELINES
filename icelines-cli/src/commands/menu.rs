//! Phase Lady Byng (LB.4) — `icelines menu` interactive looping launcher.
//!
//! Friendly entry point for users who don't want to memorize subcommand
//! names. Prints a numbered menu, reads a choice, dispatches to the
//! matching surface, then re-prints the menu. `Q` is the only way out.
//!
//! ## Loop semantics
//!
//! After a launched surface quits (TUI exits, web server stops, docs
//! finish printing) the menu re-renders and prompts again. This matches
//! the user's mental model — the menu is "home base" to return to, not
//! a one-shot dispatcher.
//!
//! ## Out of scope (deferred)
//!
//! - **`ctrlc::set_handler`** for clean Ctrl-C exit 0. Today Ctrl-C
//!   inside the prompt kills the process (exit 130 on Unix). Documented
//!   in `--help` so scripted callers know.
//! - **`[menu]` config section** for `--port` etc. Not needed until the
//!   menu accepts flags.
//! - **Server-detection on option W** — don't spawn a duplicate if
//!   `:8000` is bound. Future polish.

use crate::config::Config;
use crate::start_slug::{NavSpec, Needle, ResolveError, ScreenSpec};
use crate::tui::{self, RunTuiOpts};
use anyhow::Result;
use crossterm::{
    execute,
    terminal::{Clear, ClearType},
};
use std::io::{self, IsTerminal, Write};

/// Top-level entry point.
pub async fn run(cfg: &Config) -> Result<()> {
    if !io::stdin().is_terminal() {
        eprintln!(
            "icelines menu needs an interactive terminal — \
             use `icelines tui --start <slug>` instead"
        );
        return Ok(());
    }

    loop {
        clear_screen();
        print_menu();
        let line = match read_line()? {
            Some(s) => s,
            None => return Ok(()), // EOF — exit cleanly
        };
        match parse_menu_choice(&line) {
            MenuAction::LaunchNav(nav) => {
                run_tui_then_continue(nav.into_screen()).await;
            }
            MenuAction::PromptDrillDown(kind) => {
                if let Some(spec) = prompt_drill_down(kind) {
                    match spec.into_screen() {
                        Ok(screen) => run_tui_then_continue(screen).await,
                        Err(e) => {
                            print_error_and_pause(&format!("{e}"));
                        }
                    }
                }
            }
            MenuAction::LaunchWeb => {
                run_web_then_continue(cfg).await;
            }
            MenuAction::PrintDocs => {
                print_docs_then_pause();
            }
            MenuAction::Quit => return Ok(()),
            MenuAction::Invalid(s) => {
                print_invalid_choice(&s);
            }
        }
    }
}

/// What `parse_menu_choice` returns. Unit-testable; isolates the
/// dispatch table from I/O so the menu's logic can be exercised
/// without spawning a TUI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuAction {
    LaunchNav(NavSpec),
    PromptDrillDown(DrillDownKind),
    LaunchWeb,
    PrintDocs,
    Quit,
    Invalid(String),
}

/// Which drill-down surface the user picked. Each kind triggers its
/// own sub-prompt for the name/abbrev arg.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrillDownKind {
    Player,
    Team,
    Goalie,
    Comps,
}

/// Parse a menu input line into a MenuAction. Pure — no I/O, no
/// dispatch. Trims the input; case-insensitive on letter keys.
pub fn parse_menu_choice(input: &str) -> MenuAction {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return MenuAction::Invalid(String::new());
    }
    match trimmed.to_ascii_lowercase().as_str() {
        "1" => MenuAction::LaunchNav(NavSpec::Home),
        "2" => MenuAction::LaunchNav(NavSpec::Queries),
        "3" => MenuAction::LaunchNav(NavSpec::Goalies),
        "4" => MenuAction::LaunchNav(NavSpec::Tonight),
        "5" => MenuAction::LaunchNav(NavSpec::Schedule),
        "6" => MenuAction::LaunchNav(NavSpec::Playoffs),
        "7" => MenuAction::LaunchNav(NavSpec::Transactions),
        "8" => MenuAction::LaunchNav(NavSpec::Depth),
        "p" => MenuAction::PromptDrillDown(DrillDownKind::Player),
        "t" => MenuAction::PromptDrillDown(DrillDownKind::Team),
        "g" => MenuAction::PromptDrillDown(DrillDownKind::Goalie),
        "c" => MenuAction::PromptDrillDown(DrillDownKind::Comps),
        "w" => MenuAction::LaunchWeb,
        "d" => MenuAction::PrintDocs,
        "q" => MenuAction::Quit,
        _ => MenuAction::Invalid(trimmed.to_owned()),
    }
}

/// Print the menu chrome.
fn print_menu() {
    let version = env!("CARGO_PKG_VERSION");
    println!();
    println!("  ICELINES v{version} — pick a surface");
    println!();
    println!("    1. League         32-team rankings");
    println!("    2. Stats          interactive query builder");
    println!("    3. Goalies        goalie leaderboard");
    println!("    4. Scores         tonight's games + boxscores");
    println!("    5. Schedule       weekly + season schedule");
    println!("    6. Playoffs       bracket + series detail");
    println!("    7. Transactions   league-wide moves feed");
    println!("    8. Depth          cross-team depth chart");
    println!();
    println!("    P. Player card    (will prompt for name)");
    println!("    T. Team card      (will prompt for abbrev)");
    println!("    G. Goalie card    (will prompt for name)");
    println!("    C. Comps          (will prompt for name)");
    println!();
    println!("    W. Web dashboard  http://localhost:8000");
    println!("    D. Docs           command reference");
    println!("    Q. Quit");
    println!();
    println!("  Tip: skip the menu next time with `icelines tui goalies`");
    println!();
    print!("  Choose [1-8 / P / T / G / C / W / D / Q]: ");
    let _ = io::stdout().flush();
}

/// Read one line from stdin. Returns Ok(None) on EOF (Ctrl-D / piped
/// input exhausted), Ok(Some(_)) on a normal line.
fn read_line() -> Result<Option<String>> {
    let mut line = String::new();
    let bytes = io::stdin().read_line(&mut line)?;
    if bytes == 0 {
        return Ok(None); // EOF
    }
    Ok(Some(line))
}

/// Sub-prompt for a drill-down arg (player name, team abbrev, etc.).
/// Empty / whitespace input cancels back to the main menu (returns None).
fn prompt_drill_down(kind: DrillDownKind) -> Option<ScreenSpec> {
    let label = match kind {
        DrillDownKind::Player => "Player name (or pid)",
        DrillDownKind::Team => "Team abbreviation",
        DrillDownKind::Goalie => "Goalie name (or pid)",
        DrillDownKind::Comps => "Player name (or pid) for comps",
    };
    println!();
    print!("  {label}: ");
    let _ = io::stdout().flush();
    let line = read_line().ok()??;
    let arg = line.trim();
    if arg.is_empty() {
        return None;
    }
    Some(match kind {
        DrillDownKind::Player => ScreenSpec::Player(Needle::from_arg(arg)),
        DrillDownKind::Team => ScreenSpec::Team(arg.to_owned()),
        DrillDownKind::Goalie => ScreenSpec::Goalie(Needle::from_arg(arg)),
        DrillDownKind::Comps => ScreenSpec::Comps(Needle::from_arg(arg)),
    })
}

/// Launch the TUI on the given screen, swallow any error (display it
/// and return — the menu loop continues regardless).
async fn run_tui_then_continue(start_screen: crate::tui::app::Screen) {
    let opts = RunTuiOpts {
        no_color: false,
        start_screen,
    };
    if let Err(e) = tui::run_tui(opts).await {
        print_error_and_pause(&format!("TUI exited with error: {e}"));
    }
}

/// Launch the web dashboard with menu defaults. Catches `AddrInUse`
/// explicitly and prints a hint instead of letting the menu loop die.
async fn run_web_then_continue(cfg: &Config) {
    println!();
    println!("  Starting web dashboard on http://127.0.0.1:8000 — Ctrl-C to stop and return.");
    println!();
    // Default port + bind. LB.5 follow-up: read from a [menu] config
    // section so users can override.
    match crate::commands::serve::run(8000, None, /*no_open=*/ false, false, None, cfg).await {
        Ok(()) => {}
        Err(e) => {
            let msg = format!("{e}");
            // axum::Server::bind returns AddrInUse on a duplicate launch.
            // The error chain stringifies with that text on every OS we
            // care about (Windows / Linux / macOS).
            if msg.to_ascii_lowercase().contains("address")
                && msg.to_ascii_lowercase().contains("use")
            {
                print_error_and_pause(
                    "port 8000 is already in use — \
                     visit http://localhost:8000 if it's already an icelines server, \
                     or stop the conflicting process and try again.",
                );
            } else {
                print_error_and_pause(&format!("web server exited with error: {e}"));
            }
        }
    }
}

/// Print COMMANDS.md to stdout, then pause for Enter so the user has
/// time to read before the menu re-renders. v1: no pager — relies on
/// terminal scrollback. Future: spawn `$PAGER` (less / more).
fn print_docs_then_pause() {
    println!();
    print!("{}", include_str!("../../../COMMANDS.md"));
    println!();
    print!("  Press Enter to return to the menu...");
    let _ = io::stdout().flush();
    let mut buf = String::new();
    let _ = io::stdin().read_line(&mut buf);
}

/// Show a "Choose 1-8..." reminder, then loop back to the menu
/// immediately. Post-LP review fix #3: don't pause on Enter — the
/// loop would alternate between "main prompt" and "press Enter"
/// prompts on hold-down-Enter input, which is GLASS-flagged
/// surprising. Also avoids the infinite-prompt risk on a stuck stdin
/// (the launcher's non-TTY guard already handles piped input at
/// startup, but this is belt-and-suspenders).
fn print_invalid_choice(input: &str) {
    println!();
    if input.is_empty() {
        println!("  (empty choice — try again)");
    } else {
        println!("  Unknown choice '{input}'. Choose 1-8, P, T, G, C, W, D, or Q.");
    }
    println!();
    let _ = io::stdout().flush();
}

/// Show an error then pause for Enter.
fn print_error_and_pause(msg: &str) {
    println!();
    println!("  {msg}");
    println!();
    print!("  Press Enter to return to the menu...");
    let _ = io::stdout().flush();
    let mut buf = String::new();
    let _ = io::stdin().read_line(&mut buf);
}

/// Clear the screen + reset cursor to top. Prevents ConPTY artifact
/// buildup on Windows between dispatches (per GLASS roles review).
fn clear_screen() {
    let _ = execute!(io::stdout(), Clear(ClearType::All));
    // Cursor reset to (0,0):
    let _ = execute!(io::stdout(), crossterm::cursor::MoveTo(0, 0));
}

// Suppress unused-warning for ResolveError import (it's used through
// the ? operator above).
#[allow(dead_code)]
fn _unused_resolve_error_silencer(e: ResolveError) -> String {
    format!("{e}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// LB.4 / l0_parse_menu_choice_nav_digits
    /// — Each digit 1-8 maps to its NavSpec.
    #[test]
    fn l0_parse_menu_choice_nav_digits() {
        assert_eq!(parse_menu_choice("1"), MenuAction::LaunchNav(NavSpec::Home));
        assert_eq!(
            parse_menu_choice("2"),
            MenuAction::LaunchNav(NavSpec::Queries)
        );
        assert_eq!(
            parse_menu_choice("3"),
            MenuAction::LaunchNav(NavSpec::Goalies)
        );
        assert_eq!(
            parse_menu_choice("4"),
            MenuAction::LaunchNav(NavSpec::Tonight)
        );
        assert_eq!(
            parse_menu_choice("5"),
            MenuAction::LaunchNav(NavSpec::Schedule)
        );
        assert_eq!(
            parse_menu_choice("6"),
            MenuAction::LaunchNav(NavSpec::Playoffs)
        );
        assert_eq!(
            parse_menu_choice("7"),
            MenuAction::LaunchNav(NavSpec::Transactions)
        );
        assert_eq!(
            parse_menu_choice("8"),
            MenuAction::LaunchNav(NavSpec::Depth)
        );
    }

    /// LB.4 / l0_parse_menu_choice_drill_down_letters
    #[test]
    fn l0_parse_menu_choice_drill_down_letters() {
        assert_eq!(
            parse_menu_choice("p"),
            MenuAction::PromptDrillDown(DrillDownKind::Player)
        );
        assert_eq!(
            parse_menu_choice("t"),
            MenuAction::PromptDrillDown(DrillDownKind::Team)
        );
        assert_eq!(
            parse_menu_choice("g"),
            MenuAction::PromptDrillDown(DrillDownKind::Goalie)
        );
        assert_eq!(
            parse_menu_choice("c"),
            MenuAction::PromptDrillDown(DrillDownKind::Comps)
        );
    }

    /// LB.4 / l0_parse_menu_choice_case_insensitive
    /// — Uppercase + lowercase letters resolve identically.
    #[test]
    fn l0_parse_menu_choice_case_insensitive() {
        assert_eq!(parse_menu_choice("Q"), MenuAction::Quit);
        assert_eq!(parse_menu_choice("q"), MenuAction::Quit);
        assert_eq!(parse_menu_choice("W"), MenuAction::LaunchWeb);
        assert_eq!(parse_menu_choice("w"), MenuAction::LaunchWeb);
        assert_eq!(parse_menu_choice("D"), MenuAction::PrintDocs);
    }

    /// LB.4 / l0_parse_menu_choice_whitespace_trimmed
    #[test]
    fn l0_parse_menu_choice_whitespace_trimmed() {
        assert_eq!(parse_menu_choice("  q  "), MenuAction::Quit);
        assert_eq!(
            parse_menu_choice("\t1\n"),
            MenuAction::LaunchNav(NavSpec::Home)
        );
    }

    /// LB.4 / l0_parse_menu_choice_empty_input
    #[test]
    fn l0_parse_menu_choice_empty_input() {
        assert_eq!(parse_menu_choice(""), MenuAction::Invalid(String::new()));
        assert_eq!(parse_menu_choice("   "), MenuAction::Invalid(String::new()));
        assert_eq!(parse_menu_choice("\n"), MenuAction::Invalid(String::new()));
    }

    /// LB.4 / l0_parse_menu_choice_unknown_input
    #[test]
    fn l0_parse_menu_choice_unknown_input() {
        match parse_menu_choice("xyz") {
            MenuAction::Invalid(s) => assert_eq!(s, "xyz"),
            other => panic!("expected Invalid, got {other:?}"),
        }
        match parse_menu_choice("9") {
            MenuAction::Invalid(s) => assert_eq!(s, "9"),
            other => panic!("expected Invalid, got {other:?}"),
        }
    }

    /// LB.4 / l0_parse_menu_choice_non_letter_invalid
    /// — Single digits outside 1-8 are invalid; punctuation is invalid.
    #[test]
    fn l0_parse_menu_choice_non_letter_invalid() {
        assert!(matches!(parse_menu_choice("0"), MenuAction::Invalid(_)));
        assert!(matches!(parse_menu_choice("!"), MenuAction::Invalid(_)));
        assert!(matches!(parse_menu_choice("*"), MenuAction::Invalid(_)));
    }
}
