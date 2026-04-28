use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::tui::app::App;

pub fn render(f: &mut Frame, app: &App, area: Rect, idx: usize) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Player Card  (Esc: back) ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines: Vec<Line> = if let Some(p) = app.players.get(idx) {
        let ppg  = p.pace_score.map(|s| format!("{:.3}", s.pace_82/82.0)).unwrap_or_else(|| "—".to_owned());
        let proj = p.pace_score.map(|s| format!("{:.1}", s.pace_82)).unwrap_or_else(|| "—".to_owned());
        let gp   = p.pace_score.map(|s| s.gp.to_string()).unwrap_or_else(|| "—".to_owned());
        let age  = p.birth_date.as_deref()
            .and_then(|d| d.get(..4)).and_then(|y| y.parse::<u16>().ok())
            .map(|y| 2026u16.saturating_sub(y).to_string()).unwrap_or_else(|| "—".to_owned());
        let draft = match (p.draft_year, p.draft_round, p.draft_overall) {
            (Some(y), Some(r), Some(o)) => format!("{y} · R{r} #{o}"),
            (Some(y), _, _)             => y.to_string(),
            _                           => "Undrafted".to_owned(),
        };
        let pm = if p.plus_minus >= 0 { format!("+{}", p.plus_minus) } else { p.plus_minus.to_string() };
        let toi = p.toi_mmss().unwrap_or_else(|| "—".to_owned());
        let sh_pct = p.shooting_pct.map(|v| format!("{:.1}%", v * 100.0)).unwrap_or_else(|| "—".to_owned());

        let header_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
        vec![
            Line::styled(format!("  {}", p.full_name), header_style),
            Line::from(format!("  {} · {} · Age {}", p.team.as_str(), p.position.abbreviation(), age)),
            Line::from(""),
            Line::from(format!("  {:<14} {:>6}    {:<14} {:>6}", "PPG",   ppg,  "Proj/82",   proj)),
            Line::from(format!("  {:<14} {:>6}    {:<14} {:>6}", "GP",    gp,   "TOI/gm",    toi)),
            Line::from(format!("  {:<14} {:>6}    {:<14} {:>6}", "+/-",   pm,   "SH%",       sh_pct)),
            Line::from(""),
            Line::from(format!("  {:<14} {:>6}    {:<14} {:>6}", "Goals", p.season_goals, "PP Goals", p.pp_goals)),
            Line::from(format!("  {:<14} {:>6}    {:<14} {:>6}", "Assists", p.season_assists, "PP Pts", p.pp_points)),
            Line::from(format!("  {:<14} {:>6}    {:<14} {:>6}", "Points", p.season_points, "Shots", p.shots)),
            Line::from(format!("  {:<14} {:>6}    {:<14} {:>6}", "GWG", p.gwg, "Hits", p.hits)),
            Line::from(""),
            Line::from(format!("  Draft:   {}", draft)),
            Line::from(format!("  Country: {}", p.nationality_code.as_deref().unwrap_or("—"))),
            Line::from(format!("  Shoots:  {}", p.shoots_catches.as_deref().unwrap_or("—"))),
        ]
    } else {
        vec![Line::from("  No player data — run `icelines fetch all`")]
    };

    f.render_widget(Paragraph::new(lines), inner);
}
