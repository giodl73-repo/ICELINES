pub mod home;
pub mod team;
pub mod player;
pub mod search;
pub mod misc;
pub mod queries;

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
        Screen::Player(idx)     => player::render(f, app, chunks[1], *idx),
        Screen::Search          => search::render(f, app, chunks[1]),
        Screen::Queries         => queries::render(f, app, chunks[1]),
        Screen::Tonight         => misc::render_tonight(f, chunks[1]),
        Screen::Projections     => misc::render_projections(f, app, chunks[1]),
        Screen::Groups          => misc::render_groups(f, app, chunks[1]),
        Screen::Fetch           => misc::render_fetch(f, app, chunks[1]),
        Screen::Help            => home::render(f, app, chunks[1]),
    }

    f.render_widget(
        Paragraph::new(app.status.as_str()).style(Style::default().fg(Color::DarkGray)),
        chunks[2],
    );

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
}

fn render_nav(f: &mut Frame, app: &App, area: Rect) {
    let tabs: &[(&str, Screen)] = &[
        ("League",      Screen::Home),
        ("/Search",     Screen::Search),
        ("Queries",     Screen::Queries),
        ("Tonight",     Screen::Tonight),
        ("Projections", Screen::Projections),
        ("Groups",      Screen::Groups),
        ("Fetch+Install", Screen::Fetch),
    ];

    let mut spans: Vec<Span> = Vec::new();
    for (label, tab_screen) in tabs {
        let active = std::mem::discriminant(&app.screen) == std::mem::discriminant(tab_screen);
        let style = if active {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        spans.push(Span::styled(format!(" {label} "), style));
        spans.push(Span::styled("│", Style::default().fg(Color::DarkGray)));
    }
    spans.push(Span::styled(
        "  Tab:cycle  ←→:query values  Esc:back  ?:help  q:quit",
        Style::default().fg(Color::DarkGray),
    ));
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
