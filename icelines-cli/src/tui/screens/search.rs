use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    layout::{Direction, Constraint, Layout},
    Frame,
};
use crate::tui::app::App;
use icelines_core::name::normalize_name;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // Search input box
    let input = Paragraph::new(format!("/ {}_", app.search_query))
        .block(Block::default().borders(Borders::ALL).title(" Search Players "));
    f.render_widget(input, chunks[0]);

    // Results
    let query = normalize_name(&app.search_query);
    let results: Vec<_> = if query.is_empty() {
        app.players.iter().take(20).collect()
    } else {
        app.players.iter()
            .filter(|p| p.name_normalized.contains(&query))
            .take(20)
            .collect()
    };

    let items: Vec<ListItem> = results.iter().map(|p| {
        let ppg = p.pace_score.map(|s| format!("{:.2}", s.pace_82/82.0)).unwrap_or_else(|| "—".to_owned());
        ListItem::new(Line::from(format!("  {:<22} {:<5} {:<4}  {} pts/gp",
            p.full_name.chars().take(22).collect::<String>(),
            p.team.as_str(), p.position.abbreviation(), ppg)))
    }).collect();

    let mut state = ListState::default();
    state.select(Some(app.selected.min(results.len().saturating_sub(1))));

    let label = if query.is_empty() {
        format!(" Top {} players ", results.len())
    } else {
        format!(" {} matches for '{}' ", results.len(), app.search_query)
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(label))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));

    f.render_stateful_widget(list, chunks[1], &mut state);
}
