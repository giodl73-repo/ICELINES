use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use icelines_core::model::Player;
use crate::tui::app::App;

/// Return players similar to `target` — same broad position, sorted by
/// closeness in PPG pace. Excludes the target themselves.
pub fn find_comps<'a>(players: &'a [Player], target: &Player) -> Vec<&'a Player> {
    let target_pace = target.pace_score.map(|s| s.pace_82 / 82.0).unwrap_or(0.0);
    let target_is_fwd = target.position.is_forward();

    let mut pool: Vec<(&Player, f64)> = players.iter()
        .filter(|p| {
            p.nhl_id != target.nhl_id
                && p.pace_score.is_some()
                && p.position.is_forward() == target_is_fwd
        })
        .map(|p| {
            let ppg = p.pace_score.map(|s| s.pace_82 / 82.0).unwrap_or(0.0);
            (p, (ppg - target_pace).abs())
        })
        .collect();

    pool.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    pool.into_iter().take(20).map(|(p, _)| p).collect()
}

pub fn render(f: &mut Frame, app: &App, area: Rect, target_idx: usize) {
    let Some(target) = app.players.get(target_idx) else {
        f.render_widget(Paragraph::new("No player data."), area);
        return;
    };

    let comps = find_comps(&app.players, target);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(28), Constraint::Min(0)])
        .split(area);

    render_target(f, target, chunks[0]);
    render_list(f, app, target, &comps, chunks[1]);
}

fn render_target(f: &mut Frame, p: &Player, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Target ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let ppg  = p.pace_score.map(|s| format!("{:.3}", s.pace_82 / 82.0)).unwrap_or_else(|| "—".to_owned());
    let proj = p.pace_score.map(|s| format!("{:.1}", s.pace_82)).unwrap_or_else(|| "—".to_owned());
    let age  = p.birth_date.as_deref().and_then(|d| d.get(..4))
        .and_then(|y| y.parse::<u16>().ok())
        .map(|y| (2026u16.saturating_sub(y)).to_string())
        .unwrap_or_else(|| "—".to_owned());

    let hi  = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);

    let lines = vec![
        Line::styled(format!(" {}", p.full_name), hi),
        Line::from(format!(" {} · {} · Age {}", p.team.as_str(), p.position.abbreviation(), age)),
        Line::from(""),
        Line::styled(" PPG", dim),
        Line::from(format!(" {}", ppg)),
        Line::from(""),
        Line::styled(" Pts/82", dim),
        Line::from(format!(" {}", proj)),
        Line::from(""),
        Line::styled(" GP", dim),
        Line::from(format!(" {}", p.pace_score.map(|s| s.gp.to_string()).unwrap_or_else(|| "—".to_owned()))),
        Line::from(""),
        Line::styled(" G / A / Pts", dim),
        Line::from(format!(" {} / {} / {}", p.season_goals, p.season_assists, p.season_points)),
    ];
    f.render_widget(Paragraph::new(lines), inner);
}

fn render_list(f: &mut Frame, app: &App, target: &Player, comps: &[&Player], area: Rect) {
    let ppg_target = target.pace_score.map(|s| s.pace_82 / 82.0).unwrap_or(0.0);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Similar players — ↑↓ · Enter: card · Esc: back · g/f: group ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let dim = Style::default().fg(Color::DarkGray);

    let mut lines: Vec<ListItem> = vec![
        ListItem::new(Line::styled(
            format!("  {:<24} {:<5} {:<4} {:>6}  {:>7}  {:>6}", "Player", "Team", "Pos", "PPG", "Pts/82", "Δ PPG"),
            dim,
        )),
        ListItem::new(Line::styled(format!("  {}", "─".repeat(56)), dim)),
    ];

    for (i, p) in comps.iter().enumerate() {
        let ppg  = p.pace_score.map(|s| s.pace_82 / 82.0).unwrap_or(0.0);
        let proj = p.pace_score.map(|s| s.pace_82).unwrap_or(0.0);
        let delta = ppg - ppg_target;
        let delta_str = if delta >= 0.0 {
            format!("+{:.3}", delta)
        } else {
            format!("{:.3}", delta)
        };

        let name = p.full_name.chars().take(24).collect::<String>();
        let style = if i == app.selected {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else if delta.abs() < 0.020 {
            Style::default().fg(Color::Green)
        } else {
            Style::default()
        };

        lines.push(ListItem::new(Line::styled(
            format!("  {:<24} {:<5} {:<4} {:>6.3}  {:>7.1}  {:>6}",
                name, p.team.as_str(), p.position.abbreviation(), ppg, proj, delta_str),
            style,
        )));
    }

    if comps.is_empty() {
        lines.push(ListItem::new(Line::from("  No comparable players found.")));
    }

    f.render_widget(List::new(lines), inner);
}
