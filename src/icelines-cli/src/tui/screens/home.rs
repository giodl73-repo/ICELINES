use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use crate::tui::app::App;

/// 32 NHL teams in default ranking order (updated by icelines build).
pub const RANKED_TEAMS: &[&str] = &[
    "COL","TB","VGK","DAL","EDM","PIT","MTL","MIN","OTT","FLA",
    "SJ","BUF","ANA","CAR","UTA","BOS","WSH","DET","CBJ","TOR",
    "NYI","NYR","PHI","NJD","STL","LAK","SEA","NSH","CHI","WPG",
    "CGY","VAN",
];

const TEAM_NAMES: &[(&str, &str)] = &[
    ("COL","Colorado Avalanche"),("TB","Tampa Bay Lightning"),("VGK","Vegas Golden Knights"),
    ("DAL","Dallas Stars"),("EDM","Edmonton Oilers"),("PIT","Pittsburgh Penguins"),
    ("MTL","Montréal Canadiens"),("MIN","Minnesota Wild"),("OTT","Ottawa Senators"),
    ("FLA","Florida Panthers"),("SJ","San Jose Sharks"),("BUF","Buffalo Sabres"),
    ("ANA","Anaheim Ducks"),("CAR","Carolina Hurricanes"),("UTA","Utah Hockey Club"),
    ("BOS","Boston Bruins"),("WSH","Washington Capitals"),("DET","Detroit Red Wings"),
    ("CBJ","Columbus Blue Jackets"),("TOR","Toronto Maple Leafs"),
    ("NYI","New York Islanders"),("NYR","New York Rangers"),("PHI","Philadelphia Flyers"),
    ("NJD","New Jersey Devils"),("STL","St. Louis Blues"),("LAK","Los Angeles Kings"),
    ("SEA","Seattle Kraken"),("NSH","Nashville Predators"),("CHI","Chicago Blackhawks"),
    ("WPG","Winnipeg Jets"),("CGY","Calgary Flames"),("VAN","Vancouver Canucks"),
];

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_col(f, app, chunks[0], 0, 16, 1);
    render_col(f, app, chunks[1], 16, 32, 17);
}

fn render_col(f: &mut Frame, app: &App, area: Rect, from: usize, to: usize, rank_start: usize) {
    let items: Vec<ListItem> = RANKED_TEAMS[from..to.min(RANKED_TEAMS.len())]
        .iter()
        .enumerate()
        .map(|(i, abbrev)| {
            let rank = rank_start + i;
            let name = TEAM_NAMES.iter()
                .find(|(a, _)| a == abbrev)
                .map(|(_, n)| *n)
                .unwrap_or(abbrev);
            let rank_color = if rank <= 5 { Color::Green }
                else if rank <= 10 { Color::Cyan }
                else if rank >= 28 { Color::Red }
                else { Color::White };

            ListItem::new(Line::from(vec![
                Span::styled(format!("#{rank:<3}"), Style::default().fg(rank_color)),
                Span::styled(format!(" {abbrev:<5}"), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::raw(format!(" {name}")),
            ]))
        })
        .collect();

    let mut state = ListState::default();
    if app.selected >= from && app.selected < to {
        state.select(Some(app.selected - from));
    }

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" IceLines — League Tracker "))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, area, &mut state);
}
