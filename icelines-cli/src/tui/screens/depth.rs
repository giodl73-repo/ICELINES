use crate::tui::app::App;
use crate::visual::tui_web_fit_color;
use icelines_core::cross_team::ScoringMode;
use icelines_core::stats_repository::PlayerView;
use icelines_core::{
    DepthGoalieSlot, DepthLeagueView, MetricValue, TeamAbbr, TeamDepthChartPlayer,
    TeamDepthChartView,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

// ── Phase Adams.11 — chrome accessor ─────────────────────────────────────────

pub fn chrome(
    mode: ScoringMode,
    filters: &crate::tui::filter_state::RosterFilterState,
) -> crate::tui::chrome::ScreenChrome {
    use crate::tui::chrome::{KeyHint, ScreenChrome};
    let title = format!(
        "Depth - scoring={} - pos={} - country={}",
        mode.label(),
        filters.pos_filter.label(),
        filters.country_label()
    );
    let keybinds = vec![
        KeyHint::new("s", "toggle scoring"),
        KeyHint::new("p", "cycle pos"),
        KeyHint::new("n", "cycle nation"),
        KeyHint::new("f", "free filter"),
        KeyHint::new("↑↓", "select"),
        KeyHint::new("Enter", "team chart"),
    ];
    ScreenChrome { title, keybinds }
}

// ── League view ───────────────────────────────────────────────────────────────

pub(crate) fn league_view_from_app(app: &App) -> Option<DepthLeagueView> {
    let views = app.views();
    if views.is_empty() {
        return None;
    }

    let filtered_views: Vec<PlayerView<'_>> = views
        .iter()
        .filter(|v| app.depth_filters.matches_view(v))
        .copied()
        .collect();
    Some(DepthLeagueView::from_player_views(
        app.active_season_typed,
        app.active_type,
        app.repo
            .has_window(app.active_season_typed, app.active_type),
        &filtered_views,
        app.depth_mode,
    ))
}

pub(crate) fn team_chart_view_from_app(app: &App, abbrev: &str) -> Option<TeamDepthChartView> {
    let views = app.views();
    if views.is_empty() {
        return None;
    }

    let filtered_views: Vec<PlayerView<'_>> = views
        .iter()
        .filter(|v| app.depth_filters.matches_view(v))
        .copied()
        .collect();
    let goalie_views = app.goalie_views();
    Some(TeamDepthChartView::from_player_views(
        TeamAbbr(abbrev.to_string()),
        app.active_season_typed,
        app.active_type,
        app.repo
            .has_window(app.active_season_typed, app.active_type),
        &filtered_views,
        &goalie_views,
        app.depth_mode,
    ))
}

pub fn render_league(f: &mut Frame, app: &App, area: Rect) {
    let mode = app.depth_mode;
    let block = Block::default().borders(Borders::ALL).title(format!(
        " Depth Rankings — {} · s: toggle scoring · Enter: team chart · Esc: back ",
        mode.label()
    ));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let Some(view) = league_view_from_app(app) else {
        f.render_widget(Paragraph::new("  Loading…"), inner);
        return;
    };

    let ranked = &view.rows;
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

    let max_total = ranked
        .first()
        .map(|row| row.total)
        .filter(|total| *total > 0.0)
        .unwrap_or(1.0);

    for (i, row) in ranked.iter().enumerate() {
        let bar_len = ((row.total / max_total) * 16.0).round() as usize;
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
                row.team.0,
                row.c_score,
                row.lw_score,
                row.rw_score,
                row.d_score,
                row.total,
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

    let Some(view) = team_chart_view_from_app(app, abbrev) else {
        f.render_widget(Paragraph::new("  Loading…"), inner);
        return;
    };

    // Phase G.4: split inner vertically — grid on top, goalie strip
    // below. Strip takes 5 lines (header + 1 separator + up to 3 goalies);
    // suppressed when the team has no goalies in the active window.
    let strip_height: u16 = if view.goalies.is_empty() { 0 } else { 5 };
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(strip_height)])
        .split(inner);
    let grid_area = outer[0];
    let strip_area = outer[1];

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

    for (col, column) in view.columns.iter().enumerate() {
        render_pos_col(
            f,
            chunks[col],
            &column.label,
            &column.players,
            column.depth,
            &view.scoring_mode,
        );
    }

    // Phase G.4: goalie strip rendered below the grid when present.
    if !view.goalies.is_empty() {
        render_goalie_strip(f, strip_area, &view.goalies);
    }
}

/// Render the per-team goalie strip — single header row plus one row
/// per goalie sorted by GP descending. Designed to fit in a 5-line
/// vertical band at the bottom of the depth chart.
fn render_goalie_strip(f: &mut Frame, area: Rect, goalies: &[DepthGoalieSlot]) {
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
    for goalie in goalies {
        let sv_pct = metric_decimal_value(goalie, "save_pct")
            .map(|x| format!("{x:.3}"))
            .unwrap_or_else(|| "—".to_owned());
        let record = match (
            metric_int_value(goalie, "wins"),
            metric_int_value(goalie, "losses"),
            metric_int_value(goalie, "ot_losses"),
        ) {
            (Some(wins), Some(losses), Some(otl)) => format!("{wins}-{losses}-{otl}"),
            (Some(wins), Some(losses), None) => format!("{wins}-{losses}"),
            _ => "—".to_owned(),
        };
        lines.push(Line::from(format!(
            "  {:<22} {:<4}  {:>6}  {:>9}",
            goalie.display_name.chars().take(22).collect::<String>(),
            metric_int_value(goalie, "gp")
                .map(|gp| gp.to_string())
                .unwrap_or_else(|| "—".to_owned()),
            sv_pct,
            record,
        )));
    }
    f.render_widget(Paragraph::new(lines), area);
}

fn metric_int_value(goalie: &DepthGoalieSlot, key: &str) -> Option<i64> {
    goalie.metrics.iter().find_map(|metric| {
        if metric.key.0 == key {
            match metric.value {
                MetricValue::Integer(value) => Some(value),
                _ => None,
            }
        } else {
            None
        }
    })
}

fn metric_decimal_value(goalie: &DepthGoalieSlot, key: &str) -> Option<f64> {
    goalie.metrics.iter().find_map(|metric| {
        if metric.key.0 == key {
            match metric.value {
                MetricValue::Decimal(value) => Some(value),
                _ => None,
            }
        } else {
            None
        }
    })
}

fn render_pos_col(
    f: &mut Frame,
    area: Rect,
    label: &str,
    players: &[TeamDepthChartPlayer],
    depth: usize,
    mode_label: &str,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", label));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let dim = Style::default().fg(Color::DarkGray);
    // Header: Line / Name (12) / Score (4). Total visible = 19 chars,
    // fits inside ~21-char column at 100-cols+ terminals. Fit class is
    // encoded as the player-name color through the Prince visual token
    // mapping so the score column never truncates.
    let mut lines: Vec<Line> = vec![
        Line::styled(format!(" {:<12} {:>4}", "Player", mode_label), dim),
        Line::styled(format!(" {}", "─".repeat(18)), dim),
    ];

    for (i, player) in players.iter().enumerate() {
        let fit_color = player.fit.map(tui_web_fit_color).unwrap_or(Color::DarkGray);

        let name = player.display_name.chars().take(12).collect::<String>();
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
            format!(" L{} {:<12} {:>4.0}", player.line, name, player.score),
            line_style,
        ));
        if i + 1 == depth {
            lines.push(Line::styled(format!(" {}", "┄".repeat(18)), sep));
        }
    }

    f.render_widget(Paragraph::new(lines), inner);
}
