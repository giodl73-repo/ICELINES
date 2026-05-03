use crate::tui::app::App;
use icelines_core::cross_team::{
    compute_all_views_with_mode, compute_team_strength_views, fantasy_score_view, ScoringMode,
    WebFitClass,
};
use icelines_core::stats_repository::PlayerView;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

// ── League view ───────────────────────────────────────────────────────────────

pub fn render_league(f: &mut Frame, app: &App, area: Rect) {
    let mode = app.depth_mode;
    let block = Block::default().borders(Borders::ALL).title(format!(
        " Depth Rankings — {} · s: toggle scoring · Enter: team chart · Esc: back ",
        mode.label()
    ));
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Hart.5c.6 Phase B-3.2: collect views, then compute team strength.
    let views = app.views();
    if views.is_empty() {
        f.render_widget(Paragraph::new("  Loading…"), inner);
        return;
    }

    let strength = compute_team_strength_views(&views, mode);
    let mut ranked: Vec<(&str, &icelines_core::cross_team::TeamStrength)> =
        strength.iter().map(|(k, v)| (k.as_str(), v)).collect();
    // Sort total desc, then team abbrev asc for tie-break — without
    // the secondary key, teams that tie (especially common in playoff
    // mode where non-qualifying teams all score 0) shuffle every
    // frame because the input came from HashMap iteration. That's the
    // visible flicker.
    ranked.sort_by(|a, b| {
        b.1.total
            .partial_cmp(&a.1.total)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(b.0))
    });

    let dim = Style::default().fg(Color::DarkGray);
    let col_label = if mode == ScoringMode::Fantasy {
        "FPts"
    } else {
        "Pts/82"
    };

    let mut items: Vec<ListItem> = vec![
        ListItem::new(Line::styled(
            format!(
                "  {:<4} {:<5} {:>8} {:>8} {:>8} {:>8} {:>9}  {}",
                "Rk", "Team", "C", "LW", "RW", "D", "Total", col_label
            ),
            dim,
        )),
        ListItem::new(Line::styled(format!("  {}", "─".repeat(62)), dim)),
    ];

    let max_total = ranked.first().map(|(_, s)| s.total).unwrap_or(1.0);

    for (i, (team, s)) in ranked.iter().enumerate() {
        let bar_len = ((s.total / max_total) * 16.0).round() as usize;
        let bar: String = "█".repeat(bar_len) + &"░".repeat(16 - bar_len);

        let (tier_color, tier_prefix) = match i {
            0..=7 => (Color::Green, ""),
            8..=23 => (Color::Yellow, ""),
            _ => (Color::Red, ""),
        };

        let style = if i == app.selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(tier_color)
        };

        items.push(ListItem::new(Line::from(vec![Span::styled(
            format!(
                "  {:<4} {:<5} {:>8.0} {:>8.0} {:>8.0} {:>8.0} {:>9.0}  {}{}",
                i + 1,
                team,
                s.c_score,
                s.lw_score,
                s.rw_score,
                s.d_score,
                s.total,
                tier_prefix,
                bar
            ),
            style,
        )])));
    }

    f.render_widget(List::new(items), inner);
}

// ── Team depth chart view ─────────────────────────────────────────────────────

pub fn render_team(f: &mut Frame, app: &App, area: Rect, abbrev: &str) {
    let mode = app.depth_mode;
    let block = Block::default().borders(Borders::ALL).title(format!(
        " {} Depth Chart — {} · s: toggle · g/f: group · Esc: back ",
        abbrev,
        mode.label()
    ));
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Hart.5c.6 Phase B-3.2: collect views, then compute metrics.
    let views = app.views();
    if views.is_empty() {
        f.render_widget(Paragraph::new("  Loading…"), inner);
        return;
    }

    // Phase G.4: split inner vertically — grid on top, goalie strip
    // below. Strip takes 5 lines (header + 1 separator + up to 3 goalies);
    // suppressed when the team has no goalies in the active window.
    let goalie_views = app.goalie_views();
    let team_goalies = super::team::collect_team_goalie_views(&goalie_views, abbrev);
    let strip_height: u16 = if team_goalies.is_empty() { 0 } else { 5 };
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(strip_height)])
        .split(inner);
    let grid_area = outer[0];
    let strip_area = outer[1];

    // Compute cross-team metrics for all views — view-based path.
    let metrics = compute_all_views_with_mode(&views, mode);
    let metrics_map: std::collections::HashMap<u32, &icelines_core::cross_team::CrossTeamMetrics> =
        metrics
            .iter()
            .filter_map(|m| m.player_nhl_id.map(|id| (id, m)))
            .collect();

    let score_of = |v: &PlayerView<'_>| -> f64 {
        match mode {
            ScoringMode::Fantasy => fantasy_score_view(v),
            ScoringMode::Pace => v.pace_82().unwrap_or(0.0),
            // Phase Lindsay L.5.3 — None → 0.0 (parity with Pace).
            ScoringMode::Custom(sid) => sid.read(v).unwrap_or(0.0),
        }
    };

    // 5 columns: LW | C | RW | LD | RD
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
        ])
        .split(grid_area);

    // Greedy forward assignment: sort all forwards by score, assign to primary pos.
    // Overflow (>4 at any pos) spills into the thinnest other forward slot.
    use icelines_core::model::Position;
    let mut fwd_buckets: std::collections::HashMap<Position, Vec<&PlayerView<'_>>> =
        std::collections::HashMap::new();
    let mut all_fwds: Vec<&PlayerView<'_>> = views
        .iter()
        .filter(|v| v.team_display() == abbrev && v.position().is_forward())
        .collect();
    // PlayerId tiebreak — input from app.views() iterates a HashMap
    // (non-deterministic order); without a tiebreak, equal-scored
    // players swap each frame and cause depth-slot flicker.
    all_fwds.sort_by(|a, b| {
        score_of(b)
            .partial_cmp(&score_of(a))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.id().0.cmp(&b.id().0))
    });

    for v in &all_fwds {
        let bucket = fwd_buckets.entry(v.position()).or_default();
        if bucket.len() < 4 {
            bucket.push(v);
        } else {
            // Primary slot full — spill to natural wing based on shooting hand,
            // fall back to least-populated if natural wing is also full.
            let natural = match v.identity.bio.shoots_catches.as_deref() {
                Some("R") => Position::RightWing,
                _ => Position::LeftWing, // lefty or unknown → LW
            };
            let spill = if fwd_buckets.get(&natural).map_or(0, |b| b.len()) < 4 {
                natural
            } else {
                [Position::LeftWing, Position::Center, Position::RightWing]
                    .iter()
                    .min_by_key(|&&pos| fwd_buckets.get(&pos).map_or(0, |b| b.len()))
                    .copied()
                    .unwrap_or(v.position())
            };
            fwd_buckets.entry(spill).or_default().push(v);
        }
    }

    // Defense: split by handedness
    let mut all_d: Vec<&PlayerView<'_>> = views
        .iter()
        .filter(|v| v.team_display() == abbrev && v.position() == Position::Defense)
        .collect();
    all_d.sort_by(|a, b| {
        score_of(b)
            .partial_cmp(&score_of(a))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.id().0.cmp(&b.id().0))
    });
    let ld_players: Vec<_> = all_d
        .iter()
        .filter(|v| v.identity.bio.shoots_catches.as_deref() != Some("R"))
        .copied()
        .collect();
    let rd_players: Vec<_> = all_d
        .iter()
        .filter(|v| v.identity.bio.shoots_catches.as_deref() == Some("R"))
        .copied()
        .collect();

    let empty = vec![];
    let fwd_cols = [
        (
            Position::LeftWing,
            "LEFT WING",
            fwd_buckets.get(&Position::LeftWing).unwrap_or(&empty),
        ),
        (
            Position::Center,
            "CENTER",
            fwd_buckets.get(&Position::Center).unwrap_or(&empty),
        ),
        (
            Position::RightWing,
            "RIGHT WING",
            fwd_buckets.get(&Position::RightWing).unwrap_or(&empty),
        ),
    ];

    for (col, (_pos, label, players)) in fwd_cols.iter().enumerate() {
        render_pos_col(
            f,
            chunks[col],
            label,
            players,
            4,
            &score_of,
            &metrics_map,
            mode,
        );
    }

    render_pos_col(
        f,
        chunks[3],
        "LD",
        &ld_players,
        3,
        &score_of,
        &metrics_map,
        mode,
    );
    render_pos_col(
        f,
        chunks[4],
        "RD",
        &rd_players,
        3,
        &score_of,
        &metrics_map,
        mode,
    );

    // Phase G.4: goalie strip rendered below the grid when present.
    if !team_goalies.is_empty() {
        render_goalie_strip(f, strip_area, &team_goalies);
    }
}

/// Render the per-team goalie strip — single header row plus one row
/// per goalie sorted by GP descending. Designed to fit in a 5-line
/// vertical band at the bottom of the depth chart.
fn render_goalie_strip(
    f: &mut Frame,
    area: Rect,
    goalies: &[&icelines_core::stats_repository::PlayerView<'_>],
) {
    let dim = Style::default().fg(Color::DarkGray);
    let gold = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let mut lines: Vec<Line> = Vec::with_capacity(goalies.len() + 2);
    lines.push(Line::styled("  GOALTENDING", gold));
    lines.push(Line::styled(
        format!(
            "  {:<22} {:<4}  {:>6}  {:>9}",
            "Goalie", "GP", "SV%", "Record"
        ),
        dim,
    ));
    for v in goalies {
        let stats = match v.stats.goalie.as_ref() {
            Some(s) => s,
            None => {
                lines.push(Line::from(format!(
                    "  {:<22} {:<4}  {:>6}  {:>9}",
                    v.full_name().chars().take(22).collect::<String>(),
                    "—",
                    "—",
                    "—",
                )));
                continue;
            }
        };
        let sv_pct = stats
            .save_pct
            .map(|x| format!("{:.3}", x))
            .unwrap_or_else(|| "—".to_owned());
        let record = match stats.ot_losses {
            Some(otl) => format!("{}-{}-{}", stats.wins, stats.losses, otl),
            None => format!("{}-{}", stats.wins, stats.losses),
        };
        lines.push(Line::from(format!(
            "  {:<22} {:<4}  {:>6}  {:>9}",
            v.full_name().chars().take(22).collect::<String>(),
            v.gp(),
            sv_pct,
            record,
        )));
    }
    f.render_widget(Paragraph::new(lines), area);
}

#[allow(clippy::too_many_arguments)] // Render fn signature: 4 layout + 2 data + 2 mode params.
fn render_pos_col(
    f: &mut Frame,
    area: Rect,
    label: &str,
    players: &[&PlayerView<'_>],
    depth: usize,
    score_of: &impl Fn(&PlayerView<'_>) -> f64,
    metrics_map: &std::collections::HashMap<u32, &icelines_core::cross_team::CrossTeamMetrics>,
    mode: ScoringMode,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", label));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let dim = Style::default().fg(Color::DarkGray);
    // Header: Line / Name (12) / Score (4). Total visible = 19 chars,
    // fits inside ~21-char column at 100-cols+ terminals. Fit class is
    // encoded as the player-name color (Green=Elite, Yellow=Solid,
    // Cyan=Buried, Red=Stretch) so the score column never truncates.
    let mut lines: Vec<Line> = vec![
        Line::styled(format!(" {:<12} {:>4}", "Player", mode.label()), dim),
        Line::styled(format!(" {}", "─".repeat(18)), dim),
    ];

    for (i, v) in players.iter().enumerate() {
        let score = score_of(v);
        // PlayerId.0 is the canonical nhl_id (post-Hart unified shape).
        let nhl_id = v.identity.id.0;
        let fit_color = metrics_map
            .get(&nhl_id)
            .map(|m| match m.web_fit_class() {
                WebFitClass::Elite => Color::Green,
                WebFitClass::Solid => Color::Yellow,
                WebFitClass::Buried => Color::Cyan,
                WebFitClass::Stretch => Color::Red,
            })
            .unwrap_or(Color::DarkGray);

        let name = v.full_name().chars().take(12).collect::<String>();
        let line_style = if i < depth {
            Style::default().fg(fit_color)
        } else {
            // Bench / overflow rows render dim so the eye anchors on the
            // top-{depth} actually-skating skaters.
            dim
        };
        let sep = if i + 1 == depth {
            dim
        } else {
            Style::default()
        };

        lines.push(Line::styled(
            format!(" L{} {:<12} {:>4.0}", i + 1, name, score),
            line_style,
        ));
        if i + 1 == depth {
            lines.push(Line::styled(format!(" {}", "┄".repeat(18)), sep));
        }
    }

    f.render_widget(Paragraph::new(lines), inner);
}
