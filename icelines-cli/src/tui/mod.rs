pub mod app;
pub mod event;
pub mod headshot;
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
pub async fn run_tui(no_color: bool) -> Result<()> {
    // Setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend  = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let result = run_loop(&mut term, no_color).await;

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

    loop {
        // Tick counter for spinner animation
        app.tick = app.tick.wrapping_add(1);

        // Auto-refresh live Scores every 30s while the tab is active.
        // Pure decision in App; this loop just calls it once per frame.
        app.tick_auto_refresh();

        // Poll for loaded players
        if app.players.is_empty() {
            if let Some(players) = app.load_state.take_players() {
                app.players = players;
                app.status  = format!("{} players loaded — press ? for help", app.players.len());
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
