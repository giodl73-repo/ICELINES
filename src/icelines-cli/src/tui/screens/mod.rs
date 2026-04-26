pub mod home;
pub mod team;
pub mod player;
pub mod search;
pub mod misc;

use ratatui::Frame;
use crate::tui::app::{App, Screen};
use crate::tui::widgets::help_lines;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};

/// Main render dispatcher.
pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    // Status bar at bottom
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let content_area = chunks[0];
    let status_area  = chunks[1];

    // Render current screen
    match &app.screen {
        Screen::Home            => home::render(f, app, content_area),
        Screen::Team(abbrev)    => team::render(f, app, content_area, abbrev),
        Screen::Player(idx)     => player::render(f, app, content_area, *idx),
        Screen::Search          => search::render(f, app, content_area),
        Screen::Tonight         => misc::render_tonight(f, content_area),
        Screen::Projections     => misc::render_projections(f, content_area),
        Screen::Groups          => misc::render_groups(f, content_area),
        Screen::Fetch           => misc::render_fetch(f, content_area),
        Screen::Help            => home::render(f, app, content_area), // home behind help
    }

    // Status bar
    let status = Paragraph::new(app.status.as_str())
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(status, status_area);

    // Help overlay (floating)
    if app.show_help {
        let popup = centered_rect(60, 60, area);
        f.render_widget(Clear, popup);
        let block = Block::default()
            .title(" Help ")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));
        let inner = block.inner(popup);
        f.render_widget(block, popup);
        let content = Paragraph::new(help_lines());
        f.render_widget(content, inner);
    }
}

/// Create a centered rect of given percentage dimensions.
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
