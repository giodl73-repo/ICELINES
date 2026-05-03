//! Goalies tab — league-wide goalie leaderboard. Phase G.3.
//!
//! Sort cycle (`s` key):  SV% ↓ → GAA ↑ → Wins ↓ → GP ↓ → Saves ↓ → SO ↓
//! Min-GP cycle (`m` key): 5 → 15 → 25 → 40 → 5
//! `Enter` opens a per-goalie detail card.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::tui::app::App;

/// Sort selectors. App stores the index; we map index → comparator here
/// so the cycle order is centralised.
pub const SORTS: &[GoalieSort] = &[
    GoalieSort::SvPctDesc,
    GoalieSort::GaaAsc,
    GoalieSort::WinsDesc,
    GoalieSort::GpDesc,
    GoalieSort::SavesDesc,
    GoalieSort::ShutoutsDesc,
];

#[derive(Clone, Copy)]
pub enum GoalieSort {
    SvPctDesc,
    GaaAsc,
    WinsDesc,
    GpDesc,
    SavesDesc,
    ShutoutsDesc,
}

impl GoalieSort {
    pub fn label(self) -> &'static str {
        match self {
            Self::SvPctDesc => "SV%",
            Self::GaaAsc => "GAA",
            Self::WinsDesc => "Wins",
            Self::GpDesc => "GP",
            Self::SavesDesc => "Saves",
            Self::ShutoutsDesc => "SO",
        }
    }
}

/// The min-GP cycle values exposed under the `m` key. Stops at sensible
/// NHL leaderboard thresholds rather than allowing arbitrary input.
pub const MIN_GP_CYCLE: &[u32] = &[5, 15, 25, 40];

/// View-based goalie sort. Pure function; testable without rendering.
/// Filters to views where `view.is_goalie() == true` AND
/// `view.gp() >= min_gp` so a stray non-goalie view in the input slice
/// gets excluded. GP comes from `view.gp()` (canonical post-Hart
/// source per Hart.4.1).
pub fn sort_goalie_views<'a>(
    views: &'a [icelines_core::stats_repository::PlayerView<'a>],
    sort: GoalieSort,
    min_gp: u32,
) -> Vec<&'a icelines_core::stats_repository::PlayerView<'a>> {
    use std::cmp::Ordering;
    let mut out: Vec<&icelines_core::stats_repository::PlayerView<'a>> = views
        .iter()
        .filter(|v| v.is_goalie() && v.gp() >= min_gp)
        .collect();
    out.sort_by(|a, b| {
        let sa = a.stats.goalie.as_ref();
        let sb = b.stats.goalie.as_ref();
        let ord = match sort {
            GoalieSort::SvPctDesc => {
                let av = sa.and_then(|s| s.save_pct).unwrap_or(0.0);
                let bv = sb.and_then(|s| s.save_pct).unwrap_or(0.0);
                bv.partial_cmp(&av).unwrap_or(Ordering::Equal)
            }
            GoalieSort::GaaAsc => {
                let av = sa
                    .and_then(|s| s.goals_against_average)
                    .unwrap_or(f32::INFINITY);
                let bv = sb
                    .and_then(|s| s.goals_against_average)
                    .unwrap_or(f32::INFINITY);
                av.partial_cmp(&bv).unwrap_or(Ordering::Equal)
            }
            GoalieSort::WinsDesc => sb
                .map(|s| s.wins)
                .unwrap_or(0)
                .cmp(&sa.map(|s| s.wins).unwrap_or(0)),
            GoalieSort::GpDesc => b.gp().cmp(&a.gp()),
            GoalieSort::SavesDesc => sb
                .map(|s| s.saves)
                .unwrap_or(0)
                .cmp(&sa.map(|s| s.saves).unwrap_or(0)),
            GoalieSort::ShutoutsDesc => sb
                .map(|s| s.shutouts)
                .unwrap_or(0)
                .cmp(&sa.map(|s| s.shutouts).unwrap_or(0)),
        };
        // Tiebreaker: SV% desc, same as sort_goalies.
        ord.then_with(|| {
            let av = sa.and_then(|s| s.save_pct).unwrap_or(0.0);
            let bv = sb.and_then(|s| s.save_pct).unwrap_or(0.0);
            bv.partial_cmp(&av).unwrap_or(Ordering::Equal)
        })
    });
    out
}

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let sort = SORTS
        .get(app.goalie_sort as usize)
        .copied()
        .unwrap_or(GoalieSort::SvPctDesc);
    let title = format!(
        " Goalies · sort: {} · min GP: {} · s:sort  m:min-gp  Enter:detail  Esc:back ",
        sort.label(),
        app.goalie_min_gp,
    );
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Hart.5c.6 Phase B-3: collect goalie views, then sort+filter.
    // app.goalie_views() honors the active (season, season_type)
    // window; the empty-pool branch fires when no goalies are
    // populated for that window.
    let views = app.goalie_views();
    if views.is_empty() {
        let dim = Style::default().fg(Color::DarkGray);
        f.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::styled("  No goalie data loaded yet.", dim),
                Line::from(""),
                Line::styled(
                    "  Run `icelines fetch goalies` to populate, or wait for the loader.",
                    dim,
                ),
            ]),
            inner,
        );
        return;
    }

    let qualified = sort_goalie_views(&views, sort, app.goalie_min_gp);
    if qualified.is_empty() {
        let dim = Style::default().fg(Color::DarkGray);
        f.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::styled(
                    format!(
                        "  No goalies have played at least {} games this season.",
                        app.goalie_min_gp
                    ),
                    dim,
                ),
                Line::from(""),
                Line::styled("  Press m to lower the minimum (5/15/25/40).", dim),
            ]),
            inner,
        );
        return;
    }

    let dim = Style::default().fg(Color::DarkGray);
    let gold = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let cyan = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);

    // Header row + horizontal rule.
    let header_line = format!(
        "  {:<3}  {:<22} {:<5} {:<4}  {:<10}  {:<6}  {:<6}  {:<3}  {:<6}",
        "#", "Goalie", "Team", "GP", "W-L-OT", "SV%", "GAA", "SO", "Saves",
    );
    let mut items: Vec<ListItem> = Vec::new();
    items.push(ListItem::new(Line::styled(header_line, gold)));
    items.push(ListItem::new(Line::styled(
        format!("  {}", "─".repeat(80)),
        dim,
    )));

    let selected_idx = app.goalie_selected.min(qualified.len().saturating_sub(1));
    for (rank, v) in qualified.iter().enumerate() {
        let stats = match v.stats.goalie.as_ref() {
            Some(s) => s,
            None => continue,
        };
        let record = match stats.ot_losses {
            Some(otl) => format!("{}-{}-{}", stats.wins, stats.losses, otl),
            None => format!("{}-{}", stats.wins, stats.losses),
        };
        let sv_pct = stats
            .save_pct
            .map(|v| format!("{:.3}", v))
            .unwrap_or_else(|| "—".to_owned());
        let gaa = stats
            .goals_against_average
            .map(|v| format!("{:.2}", v))
            .unwrap_or_else(|| "—".to_owned());
        let row = format!(
            "  {:<3}  {:<22} {:<5} {:<4}  {:<10}  {:<6}  {:<6}  {:<3}  {:<6}",
            rank + 1,
            short_name(v.full_name()),
            v.team_display(),
            v.gp(), // post-Hart canonical GP source
            record,
            sv_pct,
            gaa,
            stats.shutouts,
            stats.saves,
        );
        let style = if rank == selected_idx {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else if rank < 3 {
            cyan // top-3 highlighted
        } else {
            Style::default()
        };
        items.push(ListItem::new(Line::styled(row, style)));
    }

    items.push(ListItem::new(Line::from("")));
    items.push(ListItem::new(Line::from(vec![
        Span::styled(format!("  {} qualified · ", qualified.len()), dim),
        Span::styled("s", cyan),
        Span::styled(":sort  ", dim),
        Span::styled("m", cyan),
        Span::styled(":min-gp  ", dim),
        Span::styled("Enter", cyan),
        Span::styled(":detail", dim),
    ])));
    f.render_widget(List::new(items), area);
}

/// Trim "Connor Hellebuyck" → "C. Hellebuyck" so the leaderboard fits in
/// the 22-col column width without wrapping.
fn short_name(full: &str) -> String {
    let mut parts = full.split_whitespace();
    match (parts.next(), parts.next_back()) {
        (Some(first), Some(last)) if first != last => {
            let initial = first.chars().next().unwrap_or('?');
            format!("{initial}. {last}")
        }
        (Some(only), _) => only.to_owned(),
        _ => full.to_owned(),
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)] // render_detail_by_id + helpers live below; keeping tests next to sort_goalie_views.
mod tests {
    use super::*;
    use icelines_core::fixtures;
    use icelines_core::model::Season;
    use icelines_core::season_stats::{GoalieSeasonStats, SeasonType};
    use icelines_core::stats_repository::StatsRepository;

    /// `(id, name, team, gp, wins, sv_pct, gaa, shutouts)` test seed tuple.
    type GoalieSeed<'a> = (u32, &'a str, &'a str, u32, u32, f32, f32, u32);

    fn build_goalie_pool(seeds: &[GoalieSeed<'_>]) -> StatsRepository {
        let mut r = StatsRepository::new();
        for &(id, name, team, gp, wins, sv_pct, gaa, so) in seeds {
            let normalized = icelines_core::name::normalize_name(name);
            let identity = fixtures::identity(id).name(name, &normalized).build();
            let goalie_stats = GoalieSeasonStats {
                games_started: gp,
                wins,
                losses: gp.saturating_sub(wins).saturating_sub(2),
                ot_losses: Some(2),
                ties: None,
                shots_against: 30 * gp,
                goals_against: (gaa as u32) * gp,
                saves: 28 * gp,
                save_pct: Some(sv_pct),
                goals_against_average: Some(gaa),
                shutouts: so,
                time_on_ice_sec: gp * 3600,
            };
            // Build SeasonStats with goalie variant: position=Goalie + goalie:Some.
            let mut stats = fixtures::stats(id, 20242025, team)
                .position(icelines_core::model::Position::Goalie)
                .goalie(goalie_stats)
                .build();
            // Override totals.gp so view.gp() returns the goalie's gp.
            stats.totals.gp = gp;
            r.upsert_identity(identity).unwrap();
            r.upsert_stats(stats).unwrap();
        }
        r
    }

    fn collect_goalie_views(
        repo: &StatsRepository,
    ) -> Vec<icelines_core::stats_repository::PlayerView<'_>> {
        repo.goalies(Season(20242025), SeasonType::Regular)
            .collect()
    }

    #[test]
    fn l0_sort_goalie_views_sv_pct_desc_default() {
        let repo = build_goalie_pool(&[
            (1, "Backup", "WPG", 20, 8, 0.890, 3.20, 0),
            (2, "Connor Hellebuyck", "WPG", 63, 47, 0.925, 2.00, 8),
            (3, "Mid Tier", "BOS", 35, 18, 0.910, 2.50, 2),
        ]);
        let views = collect_goalie_views(&repo);
        let sorted = sort_goalie_views(&views, GoalieSort::SvPctDesc, 15);
        assert_eq!(
            sorted[0].full_name(),
            "Connor Hellebuyck",
            "highest SV% should sort first"
        );
        assert_eq!(sorted[1].full_name(), "Mid Tier");
        assert_eq!(sorted[2].full_name(), "Backup");
    }

    #[test]
    fn l0_sort_goalie_views_gaa_asc_low_is_best() {
        let repo = build_goalie_pool(&[
            (1, "High GAA", "OTT", 30, 12, 0.900, 3.20, 1),
            (2, "Low GAA", "WPG", 30, 18, 0.920, 2.00, 5),
        ]);
        let views = collect_goalie_views(&repo);
        let sorted = sort_goalie_views(&views, GoalieSort::GaaAsc, 15);
        assert_eq!(
            sorted[0].full_name(),
            "Low GAA",
            "GAA sort: smaller is better — low GAA first"
        );
    }

    #[test]
    fn l0_sort_goalie_views_filters_by_min_gp() {
        let repo = build_goalie_pool(&[
            (1, "Backup", "WPG", 5, 2, 0.999, 1.00, 0),
            (2, "Starter", "WPG", 50, 28, 0.910, 2.50, 5),
        ]);
        let views = collect_goalie_views(&repo);
        let sorted = sort_goalie_views(&views, GoalieSort::SvPctDesc, 15);
        assert_eq!(sorted.len(), 1);
        assert_eq!(sorted[0].full_name(), "Starter");
    }

    #[test]
    fn l0_sort_goalie_views_lower_min_gp_includes_more() {
        let repo = build_goalie_pool(&[
            (1, "Backup", "WPG", 7, 3, 0.940, 2.00, 1),
            (2, "Starter", "WPG", 50, 28, 0.910, 2.50, 5),
        ]);
        let views = collect_goalie_views(&repo);
        let lo = sort_goalie_views(&views, GoalieSort::SvPctDesc, 5);
        assert_eq!(lo.len(), 2, "min_gp=5 includes both");
        let hi = sort_goalie_views(&views, GoalieSort::SvPctDesc, 15);
        assert_eq!(hi.len(), 1, "min_gp=15 excludes the backup");
    }

    #[test]
    fn l0_short_name_uses_initial() {
        assert_eq!(short_name("Connor Hellebuyck"), "C. Hellebuyck");
        assert_eq!(short_name("Igor"), "Igor");
    }
}

// ── Hart.5c.6 Phase B-2.3 — view-based goalie detail ─────────────────────────
//
// `render_detail_by_id` looks up the goalie view via `app.view_for(pid)`
// and renders the same headshot | stats | dashboard layout as
// `render_detail`. Field-by-field equivalent, sourcing through
// PlayerView accessors instead of the legacy `&Goalie` struct.

pub fn render_detail_by_id(
    f: &mut Frame,
    app: &App,
    area: Rect,
    pid: icelines_core::identity::PlayerId,
) {
    let Some(view) = app.view_for(pid) else {
        let dim = Style::default().fg(Color::DarkGray);
        let name = app
            .repo
            .identity(pid)
            .map(|i| i.full_name.as_str())
            .unwrap_or("Goalie");
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Goalie · Esc back ");
        let inner = block.inner(area);
        f.render_widget(block, area);
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
        return;
    };

    // Trigger headshot fetch if not cached. Same NHL CDN URL pattern.
    let nhl_id = view.identity.id.0;
    if app.headshot_cache.get(nhl_id).is_none() {
        let url = view
            .identity
            .headshot_canonical_url
            .clone()
            .unwrap_or_else(|| {
                format!(
                    "https://assets.nhle.com/mugs/nhl/{}/{}/{}.png",
                    app.active_season,
                    view.team_display(),
                    nhl_id,
                )
            });
        crate::tui::headshot::spawn_fetch(nhl_id, url, app.headshot_cache.clone(), 22, 15);
    }

    let title = format!(" Goalie — {}  ·  Esc back ", view.full_name());
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let dashboards_on = crate::config::dashboards_enabled() && inner.width >= 100;
    let constraints: Vec<Constraint> = if dashboards_on {
        vec![
            Constraint::Length(26),
            Constraint::Min(0),
            Constraint::Length(34),
        ]
    } else {
        vec![Constraint::Length(26), Constraint::Min(0)]
    };
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(inner);

    render_headshot_view(f, app, &view, layout[0]);
    render_detail_stats_view(f, &view, layout[1]);
    if dashboards_on {
        if let Some(area_right) = layout.get(2).copied() {
            let panel_block = Block::default()
                .borders(Borders::ALL)
                .title(" Scout card ")
                .style(Style::default().fg(Color::DarkGray));
            let panel_inner = panel_block.inner(area_right);
            f.render_widget(panel_block, area_right);
            // compile() branches on view.is_goalie() and uses the
            // goalie panel builder when true.
            let result = app.dashboard_panel.compile(
                &app.repo,
                app.active_season_typed,
                app.active_type,
                view.identity.id,
                &app.league_context,
                app.league_context_window,
            );
            match result {
                Ok(out) => f.render_widget(Paragraph::new(out.lines), panel_inner),
                Err(err) => {
                    let dim = Style::default().fg(Color::DarkGray);
                    f.render_widget(
                        Paragraph::new(vec![
                            Line::styled("  Scout card unavailable", dim),
                            Line::styled(format!("  {err}"), dim),
                        ]),
                        panel_inner,
                    );
                }
            }
        }
    }
}

fn render_headshot_view(
    f: &mut Frame,
    app: &App,
    v: &icelines_core::stats_repository::PlayerView<'_>,
    area: Rect,
) {
    let nhl_id = v.identity.id.0;
    let rows = app.headshot_cache.get(nhl_id);
    let lines: Vec<Line> = match rows.as_deref() {
        None => {
            let abbr = v.team_display();
            vec![
                Line::from(""),
                Line::from(""),
                Line::from(""),
                Line::from(format!("  {:^20}", abbr)),
                Line::from(""),
                Line::from("  loading…"),
            ]
        }
        Some(r) if crate::tui::headshot::is_loading(r) => {
            vec![Line::from(""), Line::from("  downloading…")]
        }
        Some(r) if crate::tui::headshot::is_error(r) => vec![
            Line::from("  ┌──────────────────┐"),
            Line::from("  │                  │"),
            Line::from("  │   no headshot    │"),
            Line::from("  │                  │"),
            Line::from("  └──────────────────┘"),
        ],
        Some(rows) => rows
            .iter()
            .map(|row| Line::styled(row.clone(), Style::default().fg(Color::White)))
            .collect(),
    };
    f.render_widget(Paragraph::new(lines), area);
}

fn render_detail_stats_view(
    f: &mut Frame,
    v: &icelines_core::stats_repository::PlayerView<'_>,
    area: Rect,
) {
    let dim = Style::default().fg(Color::DarkGray);
    let gold = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let cyan = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(format!("  {}", v.full_name()), gold),
        Span::styled(format!("  ·  {}  ·  G", v.team_display()), dim),
    ]));
    if let Some(catches) = v.identity.bio.shoots_catches.as_deref() {
        lines.push(Line::styled(format!("  Catches: {catches}"), dim));
    }
    lines.push(Line::from(""));

    if let Some(s) = v.stats.goalie.as_ref() {
        let record = match s.ot_losses {
            Some(otl) => format!("{}-{}-{}", s.wins, s.losses, otl),
            None => format!("{}-{}", s.wins, s.losses),
        };
        let sv = s
            .save_pct
            .map(|x| format!("{:.4}", x))
            .unwrap_or_else(|| "—".to_owned());
        let gaa = s
            .goals_against_average
            .map(|x| format!("{:.2}", x))
            .unwrap_or_else(|| "—".to_owned());
        lines.push(Line::styled("  RECORD", gold));
        // GP comes from view.gp() (i.e. SeasonStats.totals.gp), not
        // GoalieSeasonStats — post-Hart the goalie struct doesn't
        // carry games_played; Hart.4.1 documents the canonical source.
        lines.push(Line::from(vec![
            Span::styled("    GP    ", dim),
            Span::styled(v.gp().to_string(), cyan),
            Span::styled("       Record   ", dim),
            Span::styled(record, cyan),
        ]));
        lines.push(Line::from(vec![
            Span::styled("    SV%   ", dim),
            Span::styled(sv, cyan),
            Span::styled("    GAA  ", dim),
            Span::styled(gaa, cyan),
        ]));
        lines.push(Line::from(vec![
            Span::styled("    SO    ", dim),
            Span::styled(s.shutouts.to_string(), cyan),
            Span::styled("        Saves   ", dim),
            Span::styled(s.saves.to_string(), cyan),
        ]));
    } else {
        lines.push(Line::styled("  No stats recorded yet.", dim));
    }

    f.render_widget(Paragraph::new(lines), area);
}
