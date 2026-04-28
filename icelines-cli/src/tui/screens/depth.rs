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

    // 5 columns: C | LW | RW | LD | RD
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 5), Constraint::Ratio(1, 5), Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5), Constraint::Ratio(1, 5),
        ])
        .split(inner);

    // Build the LD/RD split from shoots_catches
    let mut all_d: Vec<&icelines_core::model::Player> = app.players.iter()
        .filter(|p| p.team.as_str() == abbrev
            && p.position == icelines_core::model::Position::Defense)
        .collect();
    all_d.sort_by(|a, b| score_of(b).partial_cmp(&score_of(a))
        .unwrap_or(std::cmp::Ordering::Equal));

    let ld_players: Vec<&icelines_core::model::Player> = all_d.iter()
        .filter(|p| p.shoots_catches.as_deref() != Some("R"))
        .copied().collect();
    let rd_players: Vec<&icelines_core::model::Player> = all_d.iter()
        .filter(|p| p.shoots_catches.as_deref() == Some("R"))
        .copied().collect();

    let fwd_cols = [
        (icelines_core::model::Position::Center,    "CENTER",     4usize),
        (icelines_core::model::Position::LeftWing,  "LEFT WING",  4),
        (icelines_core::model::Position::RightWing, "RIGHT WING", 4),
    ];

    for (col, (pos, label, depth)) in fwd_cols.iter().enumerate() {
        let mut players: Vec<&icelines_core::model::Player> = app.players.iter()
            .filter(|p| p.team.as_str() == abbrev && p.position == *pos)
            .collect();
        players.sort_by(|a, b| score_of(b).partial_cmp(&score_of(a))
            .unwrap_or(std::cmp::Ordering::Equal));
        render_pos_col(f, chunks[col], label, &players, *depth, &score_of, &metrics_map, mode);
    }

    render_pos_col(f, chunks[3], "LD", &ld_players, 3, &score_of, &metrics_map, mode);
    render_pos_col(f, chunks[4], "RD", &rd_players, 3, &score_of, &metrics_map, mode);
}

fn render_pos_col(
    f: &mut Frame,
    area: Rect,
    label: &str,
    players: &[&icelines_core::model::Player],
    depth: usize,
    score_of: &impl Fn(&icelines_core::model::Player) -> f64,
    metrics_map: &std::collections::HashMap<u32, &icelines_core::cross_team::CrossTeamMetrics>,
    mode: ScoringMode,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", label));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let dim = Style::default().fg(Color::DarkGray);
    let mut lines: Vec<Line> = vec![
        Line::styled(format!(" {:<14} {:>5} Fit", "Player", mode.label()), dim),
        Line::styled(format!(" {}", "─".repeat(22)), dim),
    ];

    for (i, p) in players.iter().enumerate() {
        let score = score_of(p);
        let (fit_label, fit_color) = p.nhl_id
            .and_then(|id| metrics_map.get(&id))
            .map(|m| {
                let cls = m.web_fit_class();
                let color = match cls {
                    WebFitClass::Elite   => Color::Green,
                    WebFitClass::Solid   => Color::Yellow,
                    WebFitClass::Buried  => Color::Cyan,
                    WebFitClass::Stretch => Color::Red,
                };
                (cls.label(), color)
            })
            .unwrap_or(("?", Color::DarkGray));

        let name = p.full_name.chars().take(14).collect::<String>();
        let base_style = if i < depth { Style::default() } else { dim };
        let sep = if i + 1 == depth { Style::default().fg(Color::DarkGray) } else { Style::default() };

        lines.push(Line::from(vec![
            Span::styled(format!(" L{} {:<14} {:>5.0} ", i + 1, name, score), base_style),
            Span::styled(fit_label, Style::default().fg(fit_color).add_modifier(Modifier::BOLD)),
        ]));
        if i + 1 == depth {
            lines.push(Line::styled(format!(" {}", "┄".repeat(22)), sep));
        }
    }

    f.render_widget(Paragraph::new(lines), inner);
}
