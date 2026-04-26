pub mod app;
pub mod event;
pub mod screens;
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

    loop {
        // Draw
        term.draw(|f| screens::render(f, &app))?;

        // Handle event (100ms timeout → ~10fps, enough for a stats TUI)
        if let Some(action) = event::next_event(std::time::Duration::from_millis(100)).await? {
            if app.handle(action) {
                break; // quit
            }
        }
    }
    Ok(())
}
