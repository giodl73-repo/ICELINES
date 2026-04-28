use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};
use crate::tui::app::App;
use crate::tui::headshot;

pub fn render(f: &mut Frame, app: &App, area: Rect, idx: usize) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Player Card  ·  c: comps  ·  g: group  ·  f: favorites  ·  Esc: back ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let Some(p) = app.players.get(idx) else {
        f.render_widget(Paragraph::new(vec![Line::from("  No player data — run icelines fetch all")]), inner);
        return;
    };

    // Trigger headshot fetch if not cached.
    // URL is derivable from nhl_id + team + season even without fetching rosters:
    //   https://assets.nhle.com/mugs/nhl/{SEASON}/{TEAM}/{ID}.png
    if let Some(id) = p.nhl_id {
        if app.headshot_cache.get(id).is_none() {
            let url = p.headshot_url.clone().unwrap_or_else(|| {
                format!(
                    "https://assets.nhle.com/mugs/nhl/{}/{}/{}.png",
                    icelines_core::CURRENT_SEASON_STR,
                    p.team.as_str(),
                    id
                )
            });
            headshot::spawn_fetch(id, url, app.headshot_cache.clone(), 22, 15);
        }
    }

    // Layout: headshot (22 cols) | stats (rest)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(26), Constraint::Min(0)])
        .split(inner);

    render_headshot(f, app, p, chunks[0]);
    render_stats(f, p, chunks[1]);

    // Group picker overlay — floats over the card when `g` is pressed
    if app.group_picker_open {
        render_group_picker(f, app, area);
    }
}

pub fn render_group_picker(f: &mut Frame, app: &App, area: Rect) {
    // Center a small popup
    let popup_h = (app.group_picker_list.len() as u16 + 4).min(area.height - 4);
    let popup_w = 36u16.min(area.width - 4);
    let popup = Rect::new(
        area.x + (area.width - popup_w) / 2,
        area.y + (area.height - popup_h) / 2,
        popup_w,
        popup_h,
    );
    f.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Add to group — ↑↓ · Enter · Esc ")
        .style(Style::default().fg(Color::Yellow));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let items: Vec<ListItem> = app.group_picker_list.iter().enumerate().map(|(i, name)| {
        let style = if i == app.selected {
            Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        ListItem::new(Line::styled(format!("  ★  {}", name), style))
    }).collect();
    f.render_widget(List::new(items), inner);
}

fn render_headshot(f: &mut Frame, app: &App, p: &icelines_core::model::Player, area: Rect) {
    let rows = p.nhl_id.and_then(|id| app.headshot_cache.get(id));
    let lines: Vec<Line> = match rows.as_deref() {
        None => {
            // headshot_url is None (bundled data) — no fetch triggered, show placeholder
            let abbr = p.team.as_str();
            vec![
                Line::from(""),
                Line::from(""),
                Line::from(""),
                Line::from(format!("  {:^20}", abbr)),
                Line::from(""),
                Line::from("  loading…"),
            ]
        }
        Some(r) if headshot::is_loading(r) => vec![
            Line::from(""),
            Line::from("  downloading…"),
        ],
        Some(r) if headshot::is_error(r)          => vec![
            Line::from("  ┌──────────────────┐"),
            Line::from("  │                  │"),
            Line::from("  │   no headshot    │"),
            Line::from("  │                  │"),
            Line::from("  └──────────────────┘"),
        ],
        Some(rows) => rows.iter().map(|row| Line::styled(row.clone(), Style::default().fg(Color::White))).collect(),
    };
    f.render_widget(Paragraph::new(lines), area);
}

fn render_stats(f: &mut Frame, p: &icelines_core::model::Player, area: Rect) {
    let ppg  = p.pace_score.map(|s| format!("{:.3}", s.pace_82/82.0)).unwrap_or_else(|| "—".to_owned());
    let proj = p.pace_score.map(|s| format!("{:.1}", s.pace_82)).unwrap_or_else(|| "—".to_owned());
    let gp   = p.pace_score.map(|s| s.gp.to_string()).unwrap_or_else(|| "—".to_owned());
    let age  = p.birth_date.as_deref().and_then(|d| d.get(..4)).and_then(|y| y.parse::<u16>().ok())
        .map(|y| 2026u16.saturating_sub(y).to_string()).unwrap_or_else(|| "—".to_owned());
    let draft = match (p.draft_year, p.draft_round, p.draft_overall) {
        (Some(y), Some(r), Some(o)) => format!("{y} R{r} #{o}"),
        (Some(y), _, _) => y.to_string(),
        _ => "Undrafted".to_owned(),
    };
    let pm  = if p.plus_minus >= 0 { format!("+{}", p.plus_minus) } else { p.plus_minus.to_string() };
    let toi = p.toi_mmss().unwrap_or_else(|| "—".to_owned());
    let sh  = p.shooting_pct.map(|v| format!("{:.1}%", v*100.0)).unwrap_or_else(|| "—".to_owned());

    let hi  = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);

    let lines = vec![
        Line::styled(format!(" {}", p.full_name), hi),
        Line::from(format!(" {} · {} · Age {}", p.team.as_str(), p.position.abbreviation(), age)),
        Line::from(""),
        Line::styled(" Scoring", dim),
        Line::from(format!(" {:<10} {:>6}    {:<10} {:>6}", "PPG", ppg, "Proj/82", proj)),
        Line::from(format!(" {:<10} {:>6}    {:<10} {:>6}", "GP", gp, "TOI/g", toi)),
        Line::from(format!(" {:<10} {:>6}    {:<10} {:>6}", "+/-", pm, "SH%", sh)),
        Line::from(""),
        Line::styled(" Stats", dim),
        Line::from(format!(" {:<10} {:>6}    {:<10} {:>6}", "G", p.season_goals, "PP-G", p.pp_goals)),
        Line::from(format!(" {:<10} {:>6}    {:<10} {:>6}", "A", p.season_assists, "PP-Pts", p.pp_points)),
        Line::from(format!(" {:<10} {:>6}    {:<10} {:>6}", "Pts", p.season_points, "Shots", p.shots)),
        Line::from(format!(" {:<10} {:>6}    {:<10} {:>6}", "GWG", p.gwg, "Hits", p.hits)),
        Line::from(""),
        Line::styled(" Bio", dim),
        Line::from(format!(" Draft: {}", draft)),
        Line::from(format!(" {}  Shoots: {}",
            p.nationality_code.as_deref().unwrap_or("—"),
            p.shoots_catches.as_deref().unwrap_or("—"))),
    ];
    f.render_widget(Paragraph::new(lines), area);
}
