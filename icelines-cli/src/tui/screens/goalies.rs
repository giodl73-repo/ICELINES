// Phase Norris.4 — `GoaliesState` repeats the module name in the
// type identifier. Same canonical pattern as Norris.1/2/3.
#![allow(clippy::module_name_repetitions)]

//! Goalies tab — league-wide goalie leaderboard. Phase G.3.
//!
//! Sort cycle (`s` key):  SV% ↓ → GAA ↑ → Wins ↓ → GP ↓ → Saves ↓ → SO ↓
//! Min-GP cycle (`m` key): 5 → 15 → 25 → 40 → 5
//! `Enter` opens a per-goalie detail card.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
    Frame,
};

use icelines_core::{
    AppliedFilter, FilterKey, GoaliesView, MetricValue, SortDirection, SortKey, SortState,
    ViewContext, ViewWindow,
};

use crate::tui::app::App;
use crate::tui::filter_state::{ForcedColumns, GoalieRoleFilter, RosterFilterState};
use crate::visual::{
    tui_header_style, tui_meta_style, tui_panel_block, tui_selected_style, tui_title_style,
};

// ── Phase Norris.4 — per-screen state struct ─────────────────────────────────

/// Phase Norris.4 — Goalies tab state. Replaces the 3 `goalie_*`
/// fields previously on App. Held as `app.goalies`.
#[derive(Debug)]
pub struct GoaliesState {
    pub selected: usize,
    /// Sort cycle index — SV% ↓ (0) → GAA ↑ (1) → Wins ↓ (2) →
    /// GP ↓ (3) → Saves ↓ (4) → SO ↓ (5). Cycled by `s`.
    pub sort: u8,
    /// Min-GP threshold for the leaderboard. Default 15 matches the
    /// NHL leaderboard convention; cycled by `m`.
    pub min_gp: u32,
    pub filters: RosterFilterState,
    pub role_filter: GoalieRoleFilter,
}

// ── Phase Masterton.1 — declarative chrome ───────────────────────────────────

/// Phase Masterton.1 — chrome accessor for the Goalies tab.
/// Title carries the active sort + min-GP filter; keybinds
/// reflect the cycle keys.
pub fn chrome(state: &GoaliesState) -> crate::tui::chrome::ScreenChrome {
    use crate::tui::chrome::{KeyHint, ScreenChrome};

    let sort_label = SORTS
        .get(state.sort as usize)
        .map(|s| s.label())
        .unwrap_or("?");
    let title = format!(
        "Goalies - {sort_label} - GP >= {} - role={} - country={} - saves={}",
        state.min_gp,
        state.role_filter.label(),
        state.filters.country_label(),
        if state.filters.forced_columns.contains(ForcedColumns::SAVES) {
            "on"
        } else {
            "off"
        }
    );

    let keybinds = vec![
        KeyHint::new("s", "sort"),
        KeyHint::new("m", "min GP"),
        KeyHint::new("p", "role"),
        KeyHint::new("n", "nation"),
        KeyHint::new("h", "saves col"),
        KeyHint::new("↑↓", "select"),
        KeyHint::new("Enter", "open card"),
    ];

    ScreenChrome { title, keybinds }
}

impl Default for GoaliesState {
    fn default() -> Self {
        Self {
            selected: 0,
            // SV% ↓ — Vezina-eligibility default (matches the legacy
            // App::new init).
            sort: 0,
            // 15 GP — NHL leaderboard convention.
            min_gp: 15,
            filters: RosterFilterState::default(),
            role_filter: GoalieRoleFilter::All,
        }
    }
}

#[cfg(test)]
mod norris_state_tests {
    use super::*;

    // ── Phase Norris.4 — GoaliesState contract ─────────────────────────────

    /// Default sort cycle index is 0 — SV% descending (the
    /// Vezina-eligibility default that opens the Goalies tab).
    #[test]
    fn l0_norris_goalies_default_sort_is_sv_pct() {
        let s = GoaliesState::default();
        assert_eq!(s.sort, 0);
    }

    /// Default min-GP threshold is 15 (NHL leaderboard convention).
    /// Catches a regression where someone bumps the default and
    /// hides marginal performers from the first-open view.
    #[test]
    fn l0_norris_goalies_default_min_gp_is_15() {
        let s = GoaliesState::default();
        assert_eq!(s.min_gp, 15);
    }

    /// Cursor starts at row 0.
    #[test]
    fn l0_norris_goalies_default_selected_at_zero() {
        let s = GoaliesState::default();
        assert_eq!(s.selected, 0);
    }

    /// `App::new` wires `app.goalies` through GoaliesState::default().
    #[test]
    fn l0_norris_goalies_app_new_uses_default() {
        let app = crate::tui::app::App::new(false);
        assert_eq!(app.goalies.selected, 0);
        assert_eq!(app.goalies.sort, 0);
        assert_eq!(app.goalies.min_gp, 15);
    }

    /// Debug derive renders without panic (forge-1 sanity).
    #[test]
    fn l0_norris_goalies_default_debug_renders() {
        let s = GoaliesState::default();
        let dbg = format!("{:?}", s);
        assert!(
            dbg.contains("GoaliesState"),
            "Debug output must include the struct name; got: {dbg}"
        );
    }

    // ── Phase Masterton.1 — chrome accessor contract ───────────────────────

    /// Default chrome includes the SV% sort label and the GP=15
    /// filter in the title; keybinds advertise s/m cycle keys.
    #[test]
    fn l0_masterton_goalies_chrome_default() {
        let s = GoaliesState::default();
        let c = chrome(&s);
        assert!(
            c.title.contains("GP >= 15"),
            "default chrome must show GP >= 15; got: {}",
            c.title
        );
        let keys: Vec<&str> = c.keybinds.iter().map(|k| k.key).collect();
        assert!(keys.contains(&"s"));
        assert!(keys.contains(&"m"));
    }

    /// Min-GP changes surface in the title.
    #[test]
    fn l0_masterton_goalies_chrome_min_gp_in_title() {
        let s = GoaliesState {
            min_gp: 25,
            ..Default::default()
        };
        let c = chrome(&s);
        assert!(c.title.contains("GP >= 25"));
    }
}

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

    fn key(self) -> &'static str {
        self.leaderboard_sort().key()
    }

    fn direction(self) -> SortDirection {
        self.leaderboard_sort().direction()
    }

    fn leaderboard_sort(self) -> icelines_core::GoalieLeaderboardSort {
        match self {
            Self::SvPctDesc => icelines_core::GoalieLeaderboardSort::SavePct,
            Self::GaaAsc => icelines_core::GoalieLeaderboardSort::Gaa,
            Self::WinsDesc => icelines_core::GoalieLeaderboardSort::Wins,
            Self::GpDesc => icelines_core::GoalieLeaderboardSort::Games,
            Self::SavesDesc => icelines_core::GoalieLeaderboardSort::Saves,
            Self::ShutoutsDesc => icelines_core::GoalieLeaderboardSort::Shutouts,
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
    filters: &RosterFilterState,
    role_filter: GoalieRoleFilter,
    starter_threshold: Option<u32>,
) -> Vec<&'a icelines_core::stats_repository::PlayerView<'a>> {
    let mut out: Vec<&icelines_core::stats_repository::PlayerView<'a>> = views
        .iter()
        .filter(|v| v.is_goalie() && v.gp() >= min_gp)
        .filter(|v| filters.matches_view(v))
        .filter(|v| role_filter.matches_gp(v.gp(), starter_threshold))
        .collect();
    out.sort_by(|a, b| sort.leaderboard_sort().compare_player_views(a, b));
    out
}

struct GoaliesViewInput<'a, 'view> {
    views: &'view [icelines_core::stats_repository::PlayerView<'a>],
    sort: GoalieSort,
    min_gp: u32,
    filters: &'view RosterFilterState,
    role_filter: GoalieRoleFilter,
    starter_threshold: Option<u32>,
    season: icelines_core::model::Season,
    season_type: icelines_core::season_stats::SeasonType,
}

fn goalies_view_from_tui_state(input: GoaliesViewInput<'_, '_>) -> GoaliesView {
    let GoaliesViewInput {
        views,
        sort,
        min_gp,
        filters,
        role_filter,
        starter_threshold,
        season,
        season_type,
    } = input;
    let qualified = sort_goalie_views(views, sort, min_gp, filters, role_filter, starter_threshold);
    let mut view = GoaliesView::from_player_views(
        ViewContext::new(ViewWindow::new(season, season_type)),
        qualified.into_iter().copied(),
    );
    view.applied_filters.push(AppliedFilter {
        key: FilterKey::from("min_gp"),
        op: Some(icelines_core::FilterOp::Gte),
        value: min_gp.to_string(),
        label: format!("GP >= {min_gp}"),
    });
    view.sort = Some(SortState {
        key: SortKey::from(sort.key()),
        label: sort.label().to_string(),
        direction: sort.direction(),
    });
    if role_filter != GoalieRoleFilter::All {
        view.applied_filters.push(AppliedFilter {
            key: FilterKey::from("role"),
            op: Some(icelines_core::FilterOp::Eq),
            value: role_filter.label().to_ascii_lowercase(),
            label: format!("Role = {}", role_filter.label()),
        });
    }
    if let Some(country) = filters.country_filter {
        view.applied_filters.push(AppliedFilter {
            key: FilterKey::from("nationality"),
            op: Some(icelines_core::FilterOp::Eq),
            value: country.as_str().to_string(),
            label: format!("Nationality = {}", country.as_str()),
        });
    }
    view
}

fn starter_threshold(views: &[icelines_core::stats_repository::PlayerView<'_>]) -> Option<u32> {
    let total_gp: u32 = views.iter().filter(|v| v.is_goalie()).map(|v| v.gp()).sum();
    if total_gp == 0 {
        None
    } else {
        Some((total_gp * 60).div_ceil(100))
    }
}

fn goalie_metric_u32(row: &icelines_core::GoalieRow, key: &str) -> u32 {
    goalie_metric_optional_u32(row, key).unwrap_or(0)
}

fn goalie_metric_optional_u32(row: &icelines_core::GoalieRow, key: &str) -> Option<u32> {
    row.metrics.iter().find_map(|metric| {
        if metric.key.0 == key {
            match metric.value {
                MetricValue::Integer(value) => u32::try_from(value).ok(),
                _ => None,
            }
        } else {
            None
        }
    })
}

fn goalie_metric_f32(row: &icelines_core::GoalieRow, key: &str) -> Option<f32> {
    row.metrics.iter().find_map(|metric| {
        if metric.key.0 == key {
            match metric.value {
                MetricValue::Decimal(value) => Some(value as f32),
                _ => None,
            }
        } else {
            None
        }
    })
}

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let sort = SORTS
        .get(app.goalies.sort as usize)
        .copied()
        .unwrap_or(GoalieSort::SvPctDesc);
    let title = format!(
        " Goalies - sort: {} - min GP: {} - role: {} - nation: {} - s/m/p/n/h  Enter:detail  Esc:back ",
        sort.label(),
        app.goalies.min_gp,
        app.goalies.role_filter.label(),
        app.goalies.filters.country_label(),
    );
    let block = tui_panel_block(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Hart.5c.6 Phase B-3: collect goalie views, then sort+filter.
    // app.goalie_views() honors the active (season, season_type)
    // window; the empty-pool branch fires when no goalies are
    // populated for that window.
    let views = app.goalie_views();
    if views.is_empty() {
        let dim = tui_meta_style();
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

    let goalies_view = goalies_view_from_tui_state(GoaliesViewInput {
        views: &views,
        sort,
        min_gp: app.goalies.min_gp,
        filters: &app.goalies.filters,
        role_filter: app.goalies.role_filter,
        starter_threshold: starter_threshold(&views),
        season: app.active_season_typed,
        season_type: app.active_type,
    });
    if goalies_view.rows.is_empty() {
        let dim = tui_meta_style();
        f.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::styled(
                    format!(
                        "  No goalies match role={}, nation={}, GP >= {}.",
                        app.goalies.role_filter.label(),
                        app.goalies.filters.country_label(),
                        app.goalies.min_gp
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

    let dim = tui_meta_style();
    let gold = tui_header_style();
    let cyan = tui_title_style();

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

    let selected_idx = app
        .goalies
        .selected
        .min(goalies_view.rows.len().saturating_sub(1));
    for (rank, goalie) in goalies_view.rows.iter().enumerate() {
        let wins = goalie_metric_u32(goalie, "wins");
        let losses = goalie_metric_u32(goalie, "losses");
        let record = match goalie_metric_optional_u32(goalie, "ot_losses") {
            Some(ot_losses) => format!("{wins}-{losses}-{ot_losses}"),
            None => format!("{wins}-{losses}"),
        };
        let sv_pct = goalie_metric_f32(goalie, "save_pct")
            .map(|v| format!("{:.3}", v))
            .unwrap_or_else(|| "—".to_owned());
        let gaa = goalie_metric_f32(goalie, "gaa")
            .map(|v| format!("{:.2}", v))
            .unwrap_or_else(|| "—".to_owned());
        let row = format!(
            "  {:<3}  {:<22} {:<5} {:<4}  {:<10}  {:<6}  {:<6}  {:<3}  {:<6}",
            rank + 1,
            short_name(&goalie.display_name),
            goalie.team.0.as_str(),
            goalie_metric_u32(goalie, "gp"),
            record,
            sv_pct,
            gaa,
            goalie_metric_u32(goalie, "shutouts"),
            goalie_metric_u32(goalie, "saves"),
        );
        let style = if rank == selected_idx {
            tui_selected_style()
        } else if rank < 3 {
            cyan // top-3 highlighted
        } else {
            tui_meta_style()
        };
        items.push(ListItem::new(Line::styled(row, style)));
    }

    items.push(ListItem::new(Line::from("")));
    items.push(ListItem::new(Line::from(vec![
        Span::styled(format!("  {} qualified · ", goalies_view.rows.len()), dim),
        Span::styled("s", cyan),
        Span::styled(":sort  ", dim),
        Span::styled("m", cyan),
        Span::styled(":min-gp  ", dim),
        Span::styled("p", cyan),
        Span::styled(":role  ", dim),
        Span::styled("n", cyan),
        Span::styled(":nation  ", dim),
        Span::styled("h", cyan),
        Span::styled(":saves  ", dim),
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
        let sorted = sort_goalie_views(
            &views,
            GoalieSort::SvPctDesc,
            15,
            &RosterFilterState::default(),
            GoalieRoleFilter::All,
            None,
        );
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
        let sorted = sort_goalie_views(
            &views,
            GoalieSort::GaaAsc,
            15,
            &RosterFilterState::default(),
            GoalieRoleFilter::All,
            None,
        );
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
        let sorted = sort_goalie_views(
            &views,
            GoalieSort::SvPctDesc,
            15,
            &RosterFilterState::default(),
            GoalieRoleFilter::All,
            None,
        );
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
        let lo = sort_goalie_views(
            &views,
            GoalieSort::SvPctDesc,
            5,
            &RosterFilterState::default(),
            GoalieRoleFilter::All,
            None,
        );
        assert_eq!(lo.len(), 2, "min_gp=5 includes both");
        let hi = sort_goalie_views(
            &views,
            GoalieSort::SvPctDesc,
            15,
            &RosterFilterState::default(),
            GoalieRoleFilter::All,
            None,
        );
        assert_eq!(hi.len(), 1, "min_gp=15 excludes the backup");
    }

    #[test]
    fn l0_goalies_view_from_tui_state_carries_filter_and_sort() {
        let repo = build_goalie_pool(&[
            (1, "Backup", "WPG", 7, 3, 0.940, 2.00, 1),
            (2, "Starter", "WPG", 50, 28, 0.910, 2.50, 5),
        ]);
        let views = collect_goalie_views(&repo);
        let filters = RosterFilterState::default();
        let view = goalies_view_from_tui_state(GoaliesViewInput {
            views: &views,
            sort: GoalieSort::SvPctDesc,
            min_gp: 15,
            filters: &filters,
            role_filter: GoalieRoleFilter::All,
            starter_threshold: None,
            season: Season(20242025),
            season_type: SeasonType::Regular,
        });

        assert_eq!(view.rows.len(), 1, "min_gp should flow into the ViewModel");
        assert_eq!(view.rows[0].display_name, "Starter");
        assert_eq!(view.applied_filters[0].key.0, "min_gp");
        assert_eq!(view.sort.as_ref().unwrap().key.0, "save_pct");
    }

    #[test]
    fn l0_goalies_view_from_tui_state_carries_saves_sort_and_metric() {
        let repo = build_goalie_pool(&[
            (1, "Spot Starter", "WPG", 20, 9, 0.920, 2.20, 1),
            (2, "Workhorse", "WPG", 60, 35, 0.910, 2.40, 5),
        ]);
        let views = collect_goalie_views(&repo);
        let filters = RosterFilterState::default();
        let view = goalies_view_from_tui_state(GoaliesViewInput {
            views: &views,
            sort: GoalieSort::SavesDesc,
            min_gp: 5,
            filters: &filters,
            role_filter: GoalieRoleFilter::All,
            starter_threshold: None,
            season: Season(20242025),
            season_type: SeasonType::Regular,
        });

        assert_eq!(view.sort.as_ref().unwrap().key.0, "saves");
        assert_eq!(view.rows[0].display_name, "Workhorse");
        assert_eq!(goalie_metric_u32(&view.rows[0], "saves"), 1_680);
        assert_eq!(goalie_metric_u32(&view.rows[1], "saves"), 560);
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
        let dim = tui_meta_style();
        let name = app
            .repo
            .identity(pid)
            .map(|i| i.full_name.as_str())
            .unwrap_or("Goalie");
        let block = tui_panel_block(" Goalie · Esc back ");
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
    let block = tui_panel_block(title);
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
            let panel_block = tui_panel_block(" Scout card ");
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
                    let dim = tui_meta_style();
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
    let dim = tui_meta_style();
    let gold = tui_header_style();
    let cyan = tui_title_style();
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
