use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use icelines_core::cross_team::{
    compute_all_with_mode, compute_team_strength, ScoringMode, WebFitClass,
};
use crate::tui::app::App;

// ── League view ───────────────────────────────────────────────────────────────

pub fn render_league(f: &mut Frame, app: &App, area: Rect) {
    let mode = app.depth_mode;
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(
            " Depth Rankings — {} · s: toggle scoring · Enter: team chart · Esc: back ",
            mode.label()
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.players.is_empty() {
        f.render_widget(Paragraph::new("  Loading…"), inner);
        return;
    }

    let strength = compute_team_strength(&app.players, mode);
    let mut ranked: Vec<(&str, &icelines_core::cross_team::TeamStrength)> =
        strength.iter().map(|(k, v)| (k.as_str(), v)).collect();
    ranked.sort_by(|a, b| b.1.total.partial_cmp(&a.1.total).unwrap_or(std::cmp::Ordering::Equal));

    let dim  = Style::default().fg(Color::DarkGray);
    let col_label = if mode == ScoringMode::Fantasy { "FPts" } else { "Pts/82" };

    let mut items: Vec<ListItem> = vec![
        ListItem::new(Line::styled(
            format!("  {:<4} {:<5} {:>8} {:>8} {:>8} {:>8} {:>9}  {}",
                "Rk", "Team", "C", "LW", "RW", "D", "Total", col_label),
            dim,
        )),
        ListItem::new(Line::styled(format!("  {}", "─".repeat(62)), dim)),
    ];

    let max_total = ranked.first().map(|(_, s)| s.total).unwrap_or(1.0);

    for (i, (team, s)) in ranked.iter().enumerate() {
        let bar_len = ((s.total / max_total) * 16.0).round() as usize;
        let bar: String = "█".repeat(bar_len) + &"░".repeat(16 - bar_len);

        let (tier_color, tier_prefix) = match i {
            0..=7  => (Color::Green,  ""),
            8..=23 => (Color::Yellow, ""),
            _      => (Color::Red,    ""),
        };

        let style = if i == app.selected {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(tier_color)
        };

        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!("  {:<4} {:<5} {:>8.0} {:>8.0} {:>8.0} {:>8.0} {:>9.0}  {}{}",
                    i + 1, team,
                    s.c_score, s.lw_score, s.rw_score, s.d_score, s.total,
                    tier_prefix, bar),
                style,
            ),
        ])));
    }

    f.render_widget(List::new(items), inner);
}

// ── Team depth chart view ─────────────────────────────────────────────────────

pub fn render_team(f: &mut Frame, app: &App, area: Rect, abbrev: &str) {
    let mode = app.depth_mode;
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(
            " {} Depth Chart — {} · s: toggle · g/f: group · Esc: back ",
            abbrev, mode.label()
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.players.is_empty() {
        f.render_widget(Paragraph::new("  Loading…"), inner);
        return;
    }

    // Compute cross-team metrics for all players
    let metrics = compute_all_with_mode(&app.players, mode);
    let metrics_map: std::collections::HashMap<u32, &icelines_core::cross_team::CrossTeamMetrics> =
        metrics.iter().filter_map(|m| m.player_nhl_id.map(|id| (id, m))).collect();

    let score_of = |p: &icelines_core::model::Player| -> f64 {
        match mode {
            ScoringMode::Fantasy  => icelines_core::cross_team::fantasy_score(p),
            ScoringMode::Pace     => p.pace_score.map(|s| s.pace_82 as f64).unwrap_or(0.0),
        }
    };

    // Split into forward positions and defense
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 4), Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4), Constraint::Ratio(1, 4),
        ])
        .split(inner);

    for (col, (pos, pos_label)) in [
        (icelines_core::model::Position::Center,    "CENTER"),
        (icelines_core::model::Position::LeftWing,  "LEFT WING"),
        (icelines_core::model::Position::RightWing, "RIGHT WING"),
        (icelines_core::model::Position::Defense,   "DEFENSE"),
    ].iter().enumerate() {
        let mut col_players: Vec<&icelines_core::model::Player> = app.players.iter()
            .filter(|p| p.team.as_str() == abbrev && p.position == *pos)
            .collect();
        col_players.sort_by(|a, b| score_of(b).partial_cmp(&score_of(a))
            .unwrap_or(std::cmp::Ordering::Equal));

        let col_block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", pos_label));
        let col_inner = col_block.inner(chunks[col]);
        f.render_widget(col_block, chunks[col]);

        let dim = Style::default().fg(Color::DarkGray);
        let depth = if *pos == icelines_core::model::Position::Defense { 6 } else { 4 };

        let mut lines: Vec<Line> = vec![
            Line::styled(
                format!(" {:<18} {:>5}  Fit", "Player", mode.label()),
                dim,
            ),
            Line::styled(format!(" {}", "─".repeat(26)), dim),
        ];

        for (i, p) in col_players.iter().enumerate() {
            let score = score_of(p);
            let (fit_label, fit_color) = if let Some(id) = p.nhl_id {
                if let Some(m) = metrics_map.get(&id) {
                    let cls = m.web_fit_class();
                    let color = match cls {
                        WebFitClass::Elite   => Color::Green,
                        WebFitClass::Solid   => Color::Yellow,
                        WebFitClass::Buried  => Color::Cyan,
                        WebFitClass::Stretch => Color::Red,
                    };
                    (cls.label(), color)
                } else {
                    ("?", Color::DarkGray)
                }
            } else {
                ("?", Color::DarkGray)
            };

            let line_num = i + 1;
            let separator = if i == depth - 1 { "┄" } else { " " };
            let name = p.full_name.chars().take(16).collect::<String>();

            let base_style = if i < depth {
                Style::default()
            } else {
                Style::default().fg(Color::DarkGray)
            };

            lines.push(Line::from(vec![
                Span::styled(
                    format!(" L{} {:<16} {:>5.0} ",
                        line_num, name, score),
                    base_style,
                ),
                Span::styled(fit_label, Style::default().fg(fit_color).add_modifier(Modifier::BOLD)),
                Span::styled(separator, dim),
            ]));
        }

        f.render_widget(Paragraph::new(lines), col_inner);
    }
}
