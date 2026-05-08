pub mod app;
pub mod dashboard_panel;
pub mod event;
pub mod headshot;
pub mod loader;
pub mod pickers;
pub mod playoffs;
pub mod schedule;
pub mod screens;
pub mod sparkline;
pub mod sync_banner;
pub mod tonight;
pub mod widgets;

pub use app::App;

use anyhow::Result;
use crossterm::{
    cursor::Show as ShowCursor,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Write};

use self::app::Screen;

/// LB.1 — options struct for `run_tui`. Forward-compatible: future
/// fields (locked surface, custom start season) land here without
/// changing every call site.
#[derive(Debug, Clone)]
pub struct RunTuiOpts {
    pub no_color: bool,
    pub start_screen: Screen,
}

impl RunTuiOpts {
    /// Default: full color, boot on League. Matches the pre-LB.1
    /// behavior so existing call sites switch over with one literal.
    pub fn home() -> Self {
        Self {
            no_color: false,
            start_screen: Screen::Home,
        }
    }
}

/// LB.0.5 — RAII terminal teardown.
///
/// Constructed immediately after `enable_raw_mode()` succeeds. Drop
/// runs on every return path, including unwinding panics. Without
/// this, a panic inside `run_loop` (or any nested screen handler)
/// would skip the manual cleanup and leave the terminal wedged in
/// alt-screen + raw-mode — fatal for the looping `icelines menu`
/// (LB.4) which would re-render onto a corrupted screen.
///
/// All operations in `drop` swallow errors via `let _ = ...`.
/// A guard whose Drop can panic causes a double-panic abort during
/// unwinding, which is worse than the original panic.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        // LeaveAlternateScreen + ShowCursor are idempotent; safe to
        // run even if alt-screen was never entered (e.g., setup
        // failed between guard construction and EnterAlternateScreen).
        let _ = execute!(io::stdout(), LeaveAlternateScreen, ShowCursor);
        let _ = io::stdout().flush();
    }
}

/// Entry point for the TUI. Sets up terminal, runs event loop, restores on exit.
///
/// Repo data is loaded synchronously in `App::boot_load` before the
/// event loop starts. The transactions loader is `tokio::spawn`'d
/// (Send-clean) — no `LocalSet` is required.
///
/// Terminal teardown is RAII via `TerminalGuard` (LB.0.5) — panic-safe.
///
/// `opts` carries the initial `Screen` (LB.1 — `--start <slug>`) plus
/// `no_color`. `RunTuiOpts::home()` matches pre-LB.1 behavior.
pub async fn run_tui(opts: RunTuiOpts) -> Result<()> {
    // Setup. Construct the guard immediately after `enable_raw_mode`
    // so any subsequent failure (EnterAlternateScreen, Terminal::new,
    // run_loop panic) triggers the guard's Drop and restores the
    // terminal cleanly.
    enable_raw_mode()?;
    let _guard = TerminalGuard;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    run_loop(&mut term, opts).await
    // _guard dropped here on happy path; restoration happens in Drop.
}

#[cfg(test)]
mod terminal_guard_tests {
    use super::TerminalGuard;
    use std::panic;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// LB.0.5 / l0_terminal_guard_drops_without_panic
    /// — TerminalGuard's Drop must use `let _ = ...` for every fallible
    ///   call. A guard whose Drop panics would cause double-panic
    ///   abort during unwinding. Construct + drop the guard inside
    ///   catch_unwind; assert no panic propagates.
    #[test]
    fn l0_terminal_guard_drops_without_panic() {
        let result = panic::catch_unwind(|| {
            let _guard = TerminalGuard;
            // Drop happens at end of scope.
        });
        assert!(
            result.is_ok(),
            "TerminalGuard's Drop must not panic — got {:?}",
            result.err()
        );
    }

    /// Post-LP review fix #7 — a tracking guard that flips a flag in
    /// its Drop impl. Stacking it inside the same scope as
    /// TerminalGuard lets us prove BOTH ran, since Rust drops in
    /// reverse construction order. If TerminalGuard's Drop had
    /// panicked, this guard's Drop would still run (double-panic
    /// abort happens AFTER all destructors in the unwinding frame
    /// complete on stable Rust unless the second panic is in a
    /// destructor — which is exactly what we need to detect).
    /// Concretely: if either guard's Drop fails, we don't reach the
    /// post-catch_unwind assertions cleanly.
    struct DropFlag<'a>(&'a AtomicBool);
    impl Drop for DropFlag<'_> {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    /// LB.0.5 / l0_terminal_guard_runs_drop_on_panic_unwind
    /// — A panic inside the scope must trigger Drop on the guard.
    ///   We co-locate a DropFlag tracker inside the same scope; both
    ///   guards drop on unwind. After catch_unwind returns Err(_),
    ///   the flag is true iff the destructors ran. If TerminalGuard's
    ///   Drop had aborted the process, we'd never reach the assert.
    #[test]
    fn l0_terminal_guard_runs_drop_on_panic_unwind() {
        let flag = AtomicBool::new(false);
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            let _guard = TerminalGuard;
            let _flag = DropFlag(&flag);
            panic!("simulated TUI panic — guard must restore terminal");
        }));
        assert!(
            result.is_err(),
            "expected the simulated panic to propagate; got Ok"
        );
        assert!(
            flag.load(Ordering::SeqCst),
            "destructor flag never flipped — Drop chain didn't run on unwind"
        );
    }

    /// Post-LP review fix #7 — happy-path drop. Sanity check that
    /// non-panic exit also runs the destructor chain, in case a
    /// future refactor introduces a noreturn path.
    #[test]
    fn l0_terminal_guard_runs_drop_on_normal_return() {
        let flag = AtomicBool::new(false);
        {
            let _guard = TerminalGuard;
            let _flag = DropFlag(&flag);
        }
        assert!(
            flag.load(Ordering::SeqCst),
            "destructor flag never flipped on normal scope exit"
        );
    }
}

async fn run_loop(
    term: &mut Terminal<CrosstermBackend<io::Stdout>>,
    opts: RunTuiOpts,
) -> Result<()> {
    let mut app = App::new(opts.no_color);
    // LB.1 — seed initial screen. Default is Home (today's behavior).
    app.screen = opts.start_screen;

    // Synchronous boot load. ~50ms against bundled data — well below
    // the user's perceptible threshold. `crossterm::event::poll` (in
    // the loop below) is a blocking sync syscall that pins the OS
    // thread, so an async load via spawn_local would never run.
    app.boot_load();

    // Background transactions loader stays async — Transaction is Send,
    // it goes through `tokio::spawn`, and the poll path is keyed on
    // `transactions_fetched_at` so a delayed populate doesn't get
    // re-fired.
    loader::spawn_loader(app.load_state.clone());

    loop {
        // Tick counter for spinner animation
        app.tick = app.tick.wrapping_add(1);

        // Auto-refresh live Scores every 30s while the tab is active.
        // Pure decision in App; this loop just calls it once per frame.
        app.tick_auto_refresh();

        // Phase T.5: poll for loaded transactions. Loaded-but-empty is a
        // valid state (renders the legend card), so we mark via fetched_at
        // rather than `is_empty()` to know if we've already pulled.
        if app.txs.rows.is_empty() && app.txs.fetched_at.is_empty() {
            if let Some(bundle) = app.load_state.take_transactions() {
                app.txs.rows = bundle.rows;
                app.txs.fetched_at = bundle.fetched_at;
                app.txs.stale = bundle.stale;
            }
        }

        // Poll install state → update status bar
        use loader::InstallPhase;
        match app.install_state.phase() {
            InstallPhase::Downloading(season) => {
                let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
                let spinner = frames[(app.tick / 2 % 10) as usize];
                app.status = format!("{spinner} Installing {season}…");
            }
            InstallPhase::Done(ref season, kb) => {
                // Only show once — reset to idle after displaying
                app.status = format!("✓ {season} installed ({kb} KB) — press i to install another");
                // Keep Done state so screen shows ✓ immediately
            }
            InstallPhase::Error(ref season, ref msg) => {
                app.status = format!("✗ {season}: {msg}");
            }
            InstallPhase::Idle => {}
        }

        // UX.1 — pull a player's full career from bundled seasons the
        // first time their card is shown. Idempotent (HashSet guard) so
        // safe to call every tick.
        app.ensure_career_loaded_for_current_screen();

        // Draw
        term.draw(|f| screens::render(f, &app))?;

        // Handle event (100ms timeout → ~10fps)
        if let Some(action) = event::next_event(std::time::Duration::from_millis(100)).await? {
            if app.handle(action) {
                break; // quit
            }
        }
    }
    Ok(())
}
