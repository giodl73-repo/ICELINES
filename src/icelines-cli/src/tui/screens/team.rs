use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::tui::app::App;

pub fn render(f: &mut Frame, app: &App, area: Rect, abbrev: &str) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} — Lineup Card ", abbrev));
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Show placeholder — real depth chart loaded when snapshot is warm
    let msg = if app.players.is_empty() {
        vec![
            Line::from(format!("  {} — Lineup Card", abbrev)),
            Line::from(""),
            Line::from("  Run `icelines fetch all` to load roster data,"),
            Line::from("  then relaunch the TUI."),
            Line::from(""),
            Line::from("  4×3 forward grid + 3×2 defense pairs"),
            Line::from("  will appear here with fit colors:"),
            Line::from("  ★ green (elite) · ~ yellow (solid)"),
            Line::from("  ↑ blue (buried) · ↓ red (stretch)"),
        ]
    } else {
        // Filter players for this team
        let team_players: Vec<_> = app.players.iter()
            .filter(|p| p.team.as_str() == abbrev)
            .collect();
        let mut lines = vec![Line::from(format!("  {} ({} players on roster)", abbrev, team_players.len())), Line::from("")];
        for p in team_players.iter().take(20) {
            let ppg = p.pace_score.map(|s| format!("{:.2}", s.pace_82/82.0)).unwrap_or_else(|| "—".to_owned());
            lines.push(Line::from(format!("  {:<22} {:<4}  {} pts/gp",
                p.full_name.chars().take(22).collect::<String>(),
                p.position.abbreviation(), ppg)));
        }
        lines
    };

    let para = Paragraph::new(msg);
    f.render_widget(para, inner);
}
