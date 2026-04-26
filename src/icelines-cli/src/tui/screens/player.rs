use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::tui::app::App;

pub fn render(f: &mut Frame, app: &App, area: Rect, idx: usize) {
    let block = Block::default().borders(Borders::ALL).title(" Player Profile ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines: Vec<Line> = if let Some(p) = app.players.get(idx) {
        let (ppg, proj) = match p.pace_score {
            Some(s) => (format!("{:.3}", s.pace_82/82.0), format!("{:.1}", s.pace_82)),
            None    => ("—".to_owned(), "—".to_owned()),
        };
        let age = p.birth_date.as_deref()
            .and_then(|d| d.get(..4)).and_then(|y| y.parse::<u16>().ok())
            .map(|y| 2026u16.saturating_sub(y).to_string()).unwrap_or_else(|| "—".to_owned());
        let draft = match (p.draft_year, p.draft_round, p.draft_overall) {
            (Some(y), Some(r), Some(o)) => format!("{y} R{r} #{o}"),
            (Some(y), _, _)             => y.to_string(),
            _                           => "Undrafted".to_owned(),
        };
        vec![
            Line::from(format!("  {}", p.full_name)),
            Line::from(format!("  {} · {:?} · Age {}", p.team.as_str(), p.position, age)),
            Line::from(""),
            Line::from(format!("  PPG:      {}", ppg)),
            Line::from(format!("  Proj/82:  {}", proj)),
            Line::from(format!("  GP:       {}", p.pace_score.map(|s| s.gp.to_string()).unwrap_or_else(|| "—".to_owned()))),
            Line::from(""),
            Line::from(format!("  Draft:    {}", draft)),
            Line::from(format!("  Nat:      {}", p.nationality_code.as_deref().unwrap_or("—"))),
            Line::from(format!("  Shoots:   {}", p.shoots_catches.as_deref().unwrap_or("—"))),
        ]
    } else {
        vec![Line::from("  No player data — run `icelines fetch all`")]
    };

    f.render_widget(Paragraph::new(lines), inner);
}
