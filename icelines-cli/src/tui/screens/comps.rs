use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use icelines_core::identity::PlayerId;
use icelines_core::stats_repository::PlayerView;
use crate::tui::app::App;

/// Return players similar to `target` — same broad position, sorted by
/// closeness in PPG pace. Excludes the target themselves.
pub fn find_comps_views<'a>(
    views: &'a [PlayerView<'a>],
    target: &PlayerView<'_>,
) -> Vec<&'a PlayerView<'a>> {
    let target_pace = target.pace_82().map(|p| p / 82.0).unwrap_or(0.0);
    let target_is_fwd = target.position().is_forward();
    let target_id = target.identity.id;

    let mut pool: Vec<(&PlayerView<'a>, f64)> = views
        .iter()
        .filter(|v| {
            v.identity.id != target_id
                && v.pace_82().is_some()
                && v.position().is_forward() == target_is_fwd
        })
        .map(|v| {
            let ppg = v.pace_82().map(|p| p / 82.0).unwrap_or(0.0);
            (v, (ppg - target_pace).abs())
        })
        .collect();

    pool.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    pool.into_iter().take(20).map(|(v, _)| v).collect()
}

/// Hart.5c.6 Phase B-2.1 — PlayerId-keyed render path. Looks up the
/// anchor view via `app.view_for(pid)`; on miss renders the
/// not-in-window placeholder (D6 auto-pop UX is event-handler side —
/// the next tick after this render moves the user back to parent).
pub fn render_by_id(f: &mut Frame, app: &App, area: Rect, target_pid: PlayerId) {
    let views = app.views();
    let Some(target) = app.view_for(target_pid) else {
        render_player_not_in_window(f, area, target_pid, app);
        return;
    };

    let comps = find_comps_views(&views, &target);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(28), Constraint::Min(0)])
        .split(area);

    render_target_view(f, &target, chunks[0]);
    render_list_view(f, app, &target, &comps, chunks[1]);
}

fn render_player_not_in_window(f: &mut Frame, area: Rect, pid: PlayerId, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Comps · Esc back ");
    let inner = block.inner(area);
    f.render_widget(block, area);
    let name = app
        .repo
        .identity(pid)
        .map(|i| i.full_name.as_str())
        .unwrap_or("Player");
    let dim = Style::default().fg(Color::DarkGray);
    f.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::styled(
                format!("  {} not in {} roster.", name, app.active_season),
                dim,
            ),
            Line::from(""),
            Line::styled("  Press Esc to return.", dim),
        ]),
        inner,
    );
}

fn render_target_view(f: &mut Frame, v: &PlayerView<'_>, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Target ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let p82  = v.pace_82();
    let ppg  = p82.map(|p| format!("{:.3}", p / 82.0)).unwrap_or_else(|| "—".to_owned());
    let proj = p82.map(|p| format!("{:.1}", p)).unwrap_or_else(|| "—".to_owned());
    let age  = v
        .identity
        .bio
        .birth_date
        .as_deref()
        .and_then(|d| d.get(..4))
        .and_then(|y| y.parse::<u16>().ok())
        .map(|y| (2026u16.saturating_sub(y)).to_string())
        .unwrap_or_else(|| "—".to_owned());

    let hi  = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);
    let totals = &v.stats.totals;

    let lines = vec![
        Line::styled(format!(" {}", v.full_name()), hi),
        Line::from(format!(
            " {} · {} · Age {}",
            v.team_display(),
            v.position().abbreviation(),
            age,
        )),
        Line::from(""),
        Line::styled(" PPG", dim),
        Line::from(format!(" {}", ppg)),
        Line::from(""),
        Line::styled(" Pts/82", dim),
        Line::from(format!(" {}", proj)),
        Line::from(""),
        Line::styled(" GP", dim),
        Line::from(format!(
            " {}",
            if v.gp() > 0 { v.gp().to_string() } else { "—".to_owned() }
        )),
        Line::from(""),
        Line::styled(" G / A / Pts", dim),
        Line::from(format!(" {} / {} / {}", totals.goals, totals.assists, totals.points)),
    ];
    f.render_widget(Paragraph::new(lines), inner);
}

fn render_list_view(
    f: &mut Frame,
    app: &App,
    target: &PlayerView<'_>,
    comps: &[&PlayerView<'_>],
    area: Rect,
) {
    let ppg_target = target.pace_82().map(|p| p / 82.0).unwrap_or(0.0);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Similar players — ↑↓ · Enter: card · Esc: back · g/f: group ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let dim = Style::default().fg(Color::DarkGray);

    let mut lines: Vec<ListItem> = vec![
        ListItem::new(Line::styled(
            format!(
                "  {:<24} {:<5} {:<4} {:>6}  {:>7}  {:>6}",
                "Player", "Team", "Pos", "PPG", "Pts/82", "Δ PPG"
            ),
            dim,
        )),
        ListItem::new(Line::styled(format!("  {}", "─".repeat(56)), dim)),
    ];

    for (i, v) in comps.iter().enumerate() {
        let ppg  = v.pace_82().map(|p| p / 82.0).unwrap_or(0.0);
        let proj = v.pace_82().unwrap_or(0.0);
        let delta = ppg - ppg_target;
        let delta_str = if delta >= 0.0 {
            format!("+{:.3}", delta)
        } else {
            format!("{:.3}", delta)
        };

        let name = v.full_name().chars().take(24).collect::<String>();
        let style = if i == app.selected {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else if delta.abs() < 0.020 {
            Style::default().fg(Color::Green)
        } else {
            Style::default()
        };

        lines.push(ListItem::new(Line::styled(
            format!(
                "  {:<24} {:<5} {:<4} {:>6.3}  {:>7.1}  {:>6}",
                name,
                v.team_display(),
                v.position().abbreviation(),
                ppg,
                proj,
                delta_str,
            ),
            style,
        )));
    }

    if comps.is_empty() {
        lines.push(ListItem::new(Line::from("  No comparable players found.")));
    }

    f.render_widget(List::new(lines), inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_core::fixtures;
    use icelines_core::model::{Position, Season};
    use icelines_core::season_stats::SeasonType;
    use icelines_core::stats_repository::StatsRepository;

    fn build_pool(seeds: &[(u32, &str, &str, Position, f64)]) -> StatsRepository {
        let mut r = StatsRepository::new();
        for &(id, name, team, pos, pace) in seeds {
            let normalized = icelines_core::name::normalize_name(name);
            let identity = fixtures::identity(id).name(name, &normalized).build();
            let mut stats = fixtures::stats(id, 20242025, team)
                .position(pos)
                .build();
            if let Some(ref mut ps) = stats.totals.pace_score {
                ps.pace_82 = pace;
            }
            r.upsert_identity(identity).unwrap();
            r.upsert_stats(stats).unwrap();
        }
        r
    }

    #[test]
    fn l0_find_comps_views_excludes_target_self() {
        let repo = build_pool(&[
            (8478402, "Connor McDavid", "EDM", Position::Center, 138.0),
            (1,       "Other Center",   "TOR", Position::Center, 100.0),
        ]);
        let views: Vec<_> = repo
            .skaters(Season(20242025), SeasonType::Regular)
            .collect();
        let target = views.iter().find(|v| v.identity.id.0 == 8478402).unwrap();
        let comps = find_comps_views(&views, target);
        assert!(
            comps.iter().all(|v| v.identity.id.0 != 8478402),
            "target must not appear in own comps list",
        );
    }

    #[test]
    fn l0_find_comps_views_filters_by_forward_vs_defense() {
        let repo = build_pool(&[
            (1, "Forward A", "EDM", Position::Center,    100.0),
            (2, "Forward B", "TOR", Position::LeftWing,   95.0),
            (3, "Forward C", "BOS", Position::RightWing,  90.0),
            (4, "Defense A", "EDM", Position::Defense,    80.0),
            (5, "Defense B", "TOR", Position::Defense,    75.0),
        ]);
        let views: Vec<_> = repo
            .skaters(Season(20242025), SeasonType::Regular)
            .collect();
        let target = views.iter().find(|v| v.identity.id.0 == 1).unwrap();
        let comps = find_comps_views(&views, target);

        for c in &comps {
            assert!(
                c.position().is_forward(),
                "comps for a forward must all be forwards, got {:?} for {}",
                c.position(),
                c.full_name(),
            );
        }
    }

    #[test]
    fn l0_find_comps_views_sorts_by_ppg_distance() {
        let repo = build_pool(&[
            (1, "Target",   "EDM", Position::Center, 100.0),
            (2, "Closest",  "TOR", Position::Center, 102.0),
            (3, "Far",      "BOS", Position::Center, 60.0),
            (4, "Closer",   "MTL", Position::Center, 95.0),
        ]);
        let views: Vec<_> = repo
            .skaters(Season(20242025), SeasonType::Regular)
            .collect();
        let target = views.iter().find(|v| v.identity.id.0 == 1).unwrap();
        let comps = find_comps_views(&views, target);
        let names: Vec<&str> = comps.iter().map(|v| v.full_name()).collect();
        assert_eq!(names, vec!["Closest", "Closer", "Far"]);
    }

    #[test]
    fn l0_find_comps_views_caps_at_twenty() {
        let mut seeds: Vec<(u32, &str, &str, Position, f64)> = vec![
            (1, "Target", "EDM", Position::Center, 100.0),
        ];
        let names: Vec<String> = (0..25).map(|i| format!("Comp {i:02}")).collect();
        for (i, name) in names.iter().enumerate() {
            seeds.push((100 + i as u32, name.as_str(), "EDM", Position::Center, 100.0 - i as f64));
        }
        let repo = build_pool(&seeds);
        let views: Vec<_> = repo
            .skaters(Season(20242025), SeasonType::Regular)
            .collect();
        let target = views.iter().find(|v| v.identity.id.0 == 1).unwrap();
        let comps = find_comps_views(&views, target);
        assert_eq!(comps.len(), 20);
    }
}
