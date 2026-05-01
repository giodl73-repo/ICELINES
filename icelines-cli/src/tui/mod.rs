pub mod app;
pub mod dashboard_panel;
pub mod event;
pub mod headshot;
pub mod sparkline;
pub mod loader;
pub mod playoffs;
pub mod schedule;
pub mod screens;
pub mod tonight;
pub mod widgets;

pub use app::App;

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

/// Entry point for the TUI. Sets up terminal, runs event loop, restores on exit.
///
/// Hart.5c.6 Phase A: the event loop body runs inside a
/// `tokio::task::LocalSet` because the post-Hart loader yields
/// `LoadOutcome` (carrying `StatsRepository: !Send`). `tokio::spawn`
/// requires Send; `spawn_local` does not, but it panics outside a
/// LocalSet. Pinning the LocalSet here means consumers don't have to
/// know — they just call `loader::spawn_repo_load(...)` and we ensure
/// the right runtime is in scope.
pub async fn run_tui(no_color: bool) -> Result<()> {
    // Setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend  = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let local = tokio::task::LocalSet::new();
    let result = local.run_until(run_loop(&mut term, no_color)).await;

    // Restore terminal regardless of result
    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen)?;
    term.show_cursor()?;

    result
}

async fn run_loop(
    term: &mut Terminal<CrosstermBackend<io::Stdout>>,
    no_color: bool,
) -> Result<()> {
    let mut app = App::new(no_color);

    // Start background player loading immediately
    loader::spawn_loader(app.load_state.clone());

    // Hart.5c.6 Phase A — spawn_local-based repo loader running in
    // parallel with the legacy spawn_loader. Both populate App;
    // consumers migrate one at a time in Phase B/C, after which the
    // legacy path is deleted.
    {
        let cfg = crate::config::Config::load().ok();
        let snapshot_dir = cfg
            .map(|c| c.snapshot_dir())
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        app.load_rx = Some(loader::spawn_repo_load(
            app.active_season_typed,
            app.active_type,
            snapshot_dir,
        ));
    }

    loop {
        // Tick counter for spinner animation
        app.tick = app.tick.wrapping_add(1);

        // Hart.5c.6 Phase A — drain the spawn_local repo loader's
        // mpsc channel. On Loaded, swaps repo + rebuilds league
        // context. No-op when the channel is empty or absent.
        app.poll_repo_load();

        // Auto-refresh live Scores every 30s while the tab is active.
        // Pure decision in App; this loop just calls it once per frame.
        app.tick_auto_refresh();

        // Phase T.5: poll for loaded transactions. Loaded-but-empty is a
        // valid state (renders the legend card), so we mark via fetched_at
        // rather than `is_empty()` to know if we've already pulled.
        if app.transactions.is_empty() && app.transactions_fetched_at.is_empty() {
            if let Some(bundle) = app.load_state.take_transactions() {
                app.transactions             = bundle.rows;
                app.transactions_fetched_at  = bundle.fetched_at;
                app.transactions_stale       = bundle.stale;
            }
        }

        // Poll install state → update status bar
        use loader::InstallPhase;
        match app.install_state.phase() {
            InstallPhase::Downloading(season) => {
                let frames = ["⠋","⠙","⠹","⠸","⠼","⠴","⠦","⠧","⠇","⠏"];
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
