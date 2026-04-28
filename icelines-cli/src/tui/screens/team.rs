use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::tui::app::App;

pub fn render(f: &mut Frame, app: &App, area: Rect, abbrev: &str) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} — Roster  (Enter: player card  Esc: back) ", abbrev));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.players.is_empty() {
        let msg = vec![
            Line::from(format!("  {} — Lineup Card", abbrev)),
            Line::from(""),
            Line::from("  Run `icelines fetch all` to load roster data."),
            Line::from(""),
            Line::from("  4×3 forward grid + 3×2 defense pairs will appear here"),
            Line::from("  with fit colors: ★ elite  ~ solid  ↑ buried  ↓ stretch"),
        ];
        f.render_widget(Paragraph::new(msg), inner);
        return;
    }

    let team_players: Vec<_> = app.players.iter()
        .filter(|p| p.team.as_str() == abbrev)
        .collect();

    let mut lines: Vec<Line> = vec![
        Line::from(format!("  {} players  ·  ↑↓ select  ·  Enter: open player card", team_players.len())),
        Line::from(""),
        Line::from(format!("  {:<22} {:<4}  {:>6}  {:>7}", "Player", "Pos", "PPG", "Pts/82")),
        Line::from(format!("  {}", "─".repeat(46))),
    ];

    for (i, p) in team_players.iter().enumerate() {
        let ppg  = p.pace_score.map(|s| format!("{:.3}", s.pace_82/82.0)).unwrap_or_else(|| "—".to_owned());
        let proj = p.pace_score.map(|s| format!("{:.1}", s.pace_82)).unwrap_or_else(|| "—".to_owned());
        let name = p.full_name.chars().take(22).collect::<String>();

        let text = format!("  {:<22} {:<4}  {:>6}  {:>7}", name, p.position.abbreviation(), ppg, proj);

        let style = if i == app.selected {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        lines.push(Line::styled(text, style));
    }

    f.render_widget(Paragraph::new(lines), inner);
}
