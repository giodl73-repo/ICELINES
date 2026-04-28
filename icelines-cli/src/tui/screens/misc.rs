//! Non-home TUI screens: Tonight, Projections, Groups, Fetch.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::tui::app::App;

// ── Tonight ───────────────────────────────────────────────────────────────────

pub fn render_tonight(f: &mut Frame, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Tonight's Games ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines = vec![
        Line::from("  Live schedule requires a network call — run in your terminal:"),
        Line::from(""),
        Line::styled("  icelines tonight", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Line::from("  icelines tonight --team EDM"),
        Line::from(""),
        Line::from("  icelines schedule --days 7"),
        Line::from("  icelines schedule --team SEA --days 3"),
        Line::from(""),
        Line::from("  The NHL API is free — no key required."),
    ];
    f.render_widget(Paragraph::new(lines), inner);
}

// ── Projections ───────────────────────────────────────────────────────────────

pub fn render_projections(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Top Projections (pts/82) — ↑↓ scroll ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.players.is_empty() {
        let lines = vec![
            Line::from("  Loading player data…"),
            Line::from(""),
            Line::from("  Run `icelines fetch all` if data never loads."),
        ];
        f.render_widget(Paragraph::new(lines), inner);
        return;
    }

    // Sort by pts-pace descending, take top 50
    let mut sorted: Vec<_> = app.players.iter()
        .filter(|p| p.pace_score.is_some())
        .collect();
    sorted.sort_by(|a, b| {
        let sa = a.pace_score.map(|s| s.pace_82).unwrap_or(0.0);
        let sb = b.pace_score.map(|s| s.pace_82).unwrap_or(0.0);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });

    let offset = app.selected.saturating_sub(5).min(sorted.len().saturating_sub(20));
    let header_style = Style::default().fg(Color::DarkGray);

    let mut lines = vec![
        Line::styled(
            format!("  {:<4} {:<22} {:<5} {:<4} {:>6}  {:>7}", "Rank", "Player", "Team", "Pos", "PPG", "Pts/82"),
            header_style,
        ),
        Line::styled(format!("  {}", "─".repeat(52)), header_style),
    ];

    for (i, p) in sorted.iter().skip(offset).take(inner.height.saturating_sub(3) as usize).enumerate() {
        let rank   = offset + i + 1;
        let ppg    = p.pace_score.map(|s| format!("{:.3}", s.pace_82 / 82.0)).unwrap_or_else(|| "—".to_owned());
        let proj   = p.pace_score.map(|s| format!("{:.1}", s.pace_82)).unwrap_or_else(|| "—".to_owned());
        let name   = p.full_name.chars().take(22).collect::<String>();

        let style = if offset + i == app.selected {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else if rank <= 5 {
            Style::default().fg(Color::Green)
        } else {
            Style::default()
        };

        lines.push(Line::styled(
            format!("  {:<4} {:<22} {:<5} {:<4} {:>6}  {:>7}", rank, name, p.team.as_str(), p.position.abbreviation(), ppg, proj),
            style,
        ));
    }

    f.render_widget(Paragraph::new(lines), inner);
}

// ── Groups ────────────────────────────────────────────────────────────────────

pub fn render_groups(f: &mut Frame, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Player Groups & Watchlists ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines = vec![
        Line::from("  Manage player watchlists from your terminal:"),
        Line::from(""),
        Line::styled("  icelines group list", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Line::from("  icelines group create \"My Watchlist\""),
        Line::from("  icelines group add \"My Watchlist\" \"McDavid\""),
        Line::from("  icelines group show \"My Watchlist\""),
        Line::from("  icelines group delete \"My Watchlist\""),
        Line::from(""),
        Line::from("  Groups are persisted in ~/.icelines/icelines.db"),
    ];
    f.render_widget(Paragraph::new(lines), inner);
}

// ── Fetch ─────────────────────────────────────────────────────────────────────

pub fn render_fetch(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Data & Fetch Status ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let player_status = if app.players.is_empty() {
        "Loading…".to_owned()
    } else {
        format!("{} players loaded from bundled/snapshot data", app.players.len())
    };

    let lines = vec![
        Line::from(format!("  Status: {}", player_status)),
        Line::from(""),
        Line::styled("  Fetch commands:", Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
        Line::from(""),
        Line::styled("  icelines fetch all", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Line::from("    → rosters + stats from NHL API (~5 min)"),
        Line::from(""),
        Line::styled("  icelines fetch realtime", Style::default().fg(Color::Cyan)),
        Line::from("    → hits, blocks, giveaways, takeaways, PIM"),
        Line::from(""),
        Line::styled("  icelines fetch money-puck", Style::default().fg(Color::Cyan)),
        Line::from("    → xG, CF%, FF%, xGF% from MoneyPuck (free)"),
        Line::from(""),
        Line::styled("  icelines data install --seasons 38", Style::default().fg(Color::Cyan)),
        Line::from("    → full history 1987–2025 from GitHub Releases"),
        Line::from(""),
        Line::styled("  icelines snapshot list", Style::default().fg(Color::Cyan)),
        Line::from("    → all cached snapshots with integrity hashes"),
    ];
    f.render_widget(Paragraph::new(lines), inner);
}
