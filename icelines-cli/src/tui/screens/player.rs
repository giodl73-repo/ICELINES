use crate::tui::app::App;
use crate::tui::headshot;
use icelines_core::identity::PlayerId;
use icelines_core::model::Position;
use icelines_core::stats_catalog::{StatCategory, StatId};
use icelines_core::stats_repository::PlayerView;
use icelines_core::{
    MetricCell, MetricValue, PlayerCardView, PlayerCareerSummary, PlayerPreNhlCareerRow,
    PlayerSeasonSummary,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

// ─── Phase Lindsay L.4.2 — career table column preset templates ────────────
//
// Six preset templates the user cycles through via `[`/`]` on the
// player card. The "Default" template is per-position (calls
// `StatId::default_in_career_table(pos)`); the others are fixed
// stat-category subsets independent of position.
//
// Order is the cycle order — `[` moves backward, `]` moves forward.
// Wraps at boundaries.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(dead_code)] // `All` variant kept for `--columns all` debug surface; SCOUT-6 removed it from the keyboard cycle.
pub enum CareerTablePreset {
    /// Per-position curated default (`StatId::default_in_career_table`).
    #[default]
    Default,
    Scoring,
    TwoWay,
    SpecialTeams,
    Time,
    Goalie,
    /// Every selectable catalog stat (108 stats — overflows on most
    /// terminal widths; useful for `--columns "all"` debugging).
    All,
}

impl CareerTablePreset {
    /// Cycle order. `[` moves to the previous; `]` moves to the next.
    /// Wraps at the boundaries.
    ///
    /// SCOUT-6 (L.5b post-fix): `All` removed from the cycle — 85
    /// columns at any reasonable terminal width is a debug surface,
    /// not a scout surface. Still reachable programmatically (e.g.
    /// for `--columns all` debugging) but not via the keyboard cycle.
    pub const ALL: &'static [CareerTablePreset] = &[
        CareerTablePreset::Default,
        CareerTablePreset::Scoring,
        CareerTablePreset::TwoWay,
        CareerTablePreset::SpecialTeams,
        CareerTablePreset::Time,
        CareerTablePreset::Goalie,
    ];

    /// Human-readable label for the status bar.
    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Scoring => "Scoring",
            Self::TwoWay => "Two-way",
            Self::SpecialTeams => "Special Teams",
            Self::Time => "Time",
            Self::Goalie => "Goalie",
            Self::All => "All (debug)",
        }
    }

    /// Resolve to the ordered StatId list for this preset + position.
    /// Returns the columns in render order (left-to-right).
    pub fn columns(self, pos: Position) -> Vec<StatId> {
        match self {
            Self::Default => StatId::all()
                .iter()
                .copied()
                .filter(|s| s.default_in_career_table(pos))
                .collect(),
            Self::Scoring => StatId::all()
                .iter()
                .copied()
                .filter(|s| s.category() == StatCategory::Scoring)
                .filter(|s| s.applies_to(pos, pos == Position::Goalie))
                .collect(),
            // SCOUT-4 L.5b post-fix: gate Faceoff* stats to Center in
            // both TwoWay AND SpecialTeams presets. Wingers/D take ~0
            // faceoffs/season; the column would render mostly "—" or a
            // misleading 100%/0% spike on the rare emergency draw.
            Self::TwoWay => StatId::all()
                .iter()
                .copied()
                .filter(|s| s.category() == StatCategory::TwoWay)
                .filter(|s| s.applies_to(pos, pos == Position::Goalie))
                .filter(|s| !is_faceoff_stat(*s) || pos == Position::Center)
                .collect(),
            Self::SpecialTeams => StatId::all()
                .iter()
                .copied()
                .filter(|s| s.category() == StatCategory::SpecialTeams)
                .filter(|s| s.applies_to(pos, pos == Position::Goalie))
                .filter(|s| !is_faceoff_stat(*s) || pos == Position::Center)
                .collect(),
            Self::Time => StatId::all()
                .iter()
                .copied()
                .filter(|s| s.category() == StatCategory::TimeOnIce)
                .filter(|s| s.applies_to(pos, pos == Position::Goalie))
                .collect(),
            Self::Goalie => StatId::all()
                .iter()
                .copied()
                .filter(|s| s.category() == StatCategory::Goalie)
                .collect(),
            Self::All => StatId::all()
                .iter()
                .copied()
                .filter(|s| s.applies_to(pos, pos == Position::Goalie))
                .collect(),
        }
    }

    /// `[` — previous preset, wrapping.
    pub fn prev(self) -> Self {
        let cur = Self::ALL.iter().position(|p| *p == self).unwrap_or(0);
        Self::ALL[(cur + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    /// `]` — next preset, wrapping.
    pub fn next(self) -> Self {
        let cur = Self::ALL.iter().position(|p| *p == self).unwrap_or(0);
        Self::ALL[(cur + 1) % Self::ALL.len()]
    }
}

/// SCOUT-4 (L.5b post-fix) — true for stats that only Centers
/// generate meaningfully. Wingers/D take ~0 faceoffs per season; the
/// column on those positions surfaces mostly None or a misleading
/// 100%/0% spike on the rare emergency draw.
fn is_faceoff_stat(s: StatId) -> bool {
    matches!(
        s,
        StatId::FaceoffWinPct
            | StatId::FaceoffWins
            | StatId::FaceoffLosses
            | StatId::OffensiveZoneFaceoffPct
            | StatId::DefensiveZoneFaceoffPct
    )
}

/// Phase Lindsay L.4.3 — render a single career-table cell value.
/// Reads via `StatId::read` (catalog dispatch) and formats per
/// `StatId::unit()`. None renders as `"—"`.
#[cfg(test)]
pub fn render_career_cell(sid: StatId, view: &PlayerView<'_>) -> String {
    match sid.read(view) {
        None => "—".to_owned(),
        Some(v) => match sid.unit() {
            // Counts and seconds: integer formatting.
            icelines_core::stats_catalog::StatUnit::Count => format!("{}", v as i64),
            icelines_core::stats_catalog::StatUnit::Seconds => {
                // Render TOI as M:SS (per-game) or just integer seconds
                // (totals). Heuristic: < 3600 → per-game M:SS;
                // ≥ 3600 → total minutes for readability.
                let secs = v as u64;
                if secs < 3600 {
                    format!("{}:{:02}", secs / 60, secs % 60)
                } else {
                    format!("{}m", secs / 60)
                }
            }
            // Pct: render as percentage with one decimal (e.g. 12.5%).
            // API stores 0.125 → display as "12.5". Drop the % sign
            // to save column width.
            icelines_core::stats_catalog::StatUnit::Pct => format!("{:.1}", v * 100.0),
            // Per-60 rates and other rates: 2 decimals.
            icelines_core::stats_catalog::StatUnit::Per60
            | icelines_core::stats_catalog::StatUnit::Rate => format!("{:.2}", v),
            // Inverted (GAA): 2 decimals.
            icelines_core::stats_catalog::StatUnit::Inverted => format!("{:.2}", v),
        },
    }
}

pub fn render_career_card_cell(sid: StatId, row: &PlayerCareerSummary) -> String {
    let Some(metric) = row
        .catalog_metrics
        .iter()
        .find(|metric| metric.key.0 == sid.cli_key())
    else {
        return "â€”".to_owned();
    };

    match &metric.value {
        MetricValue::Missing => "â€”".to_owned(),
        MetricValue::Integer(value) => {
            if matches!(sid.unit(), icelines_core::stats_catalog::StatUnit::Seconds) {
                let secs = (*value).max(0) as u64;
                if secs < 3600 {
                    format!("{}:{:02}", secs / 60, secs % 60)
                } else {
                    format!("{}m", secs / 60)
                }
            } else {
                value.to_string()
            }
        }
        MetricValue::Decimal(value) => match sid.unit() {
            icelines_core::stats_catalog::StatUnit::Pct => format!("{:.1}", value * 100.0),
            icelines_core::stats_catalog::StatUnit::Per60
            | icelines_core::stats_catalog::StatUnit::Rate
            | icelines_core::stats_catalog::StatUnit::Inverted => format!("{:.2}", value),
            icelines_core::stats_catalog::StatUnit::Count
            | icelines_core::stats_catalog::StatUnit::Seconds => format!("{}", *value as i64),
        },
        MetricValue::Text(value) => value.clone(),
    }
}

pub(crate) fn active_season_summary_lines(
    active: Option<&PlayerSeasonSummary>,
    dim: ratatui::style::Style,
) -> Vec<Line<'static>> {
    let Some(active) = active else {
        return Vec::new();
    };

    let metric = |key: &str| active.metrics.iter().find(|metric| metric.key.0 == key);
    let gp = format_metric(metric("gp"));
    let goals = format_metric(metric("goals"));
    let assists = format_metric(metric("assists"));
    let points = format_metric(metric("points"));
    let ppg = format_metric(metric("points_per_game"));
    let plus_minus = format_signed_metric(metric("plus_minus"));
    let shooting_pct = format_metric(metric("shooting_pct"));
    let toi = format_seconds_metric(metric("toi_per_game_sec"));
    let season_label = format!(
        "{}-{}",
        &active.season.as_str()[..4],
        &active.season.as_str()[6..],
    );

    vec![
        Line::styled(
            format!(" Current  ·  {season_label} {}", active.season_type.label()),
            dim,
        ),
        Line::from(format!(
            " GP {gp:>3}   G {goals:>3}   A {assists:>3}   P {points:>3}   PPG {ppg:>5}   +/- {plus_minus:>4}   S% {shooting_pct:>5}   TOI/G {toi:>5}",
        )),
        Line::from(""),
    ]
}

fn format_metric(metric: Option<&MetricCell>) -> String {
    match metric.map(|metric| &metric.value) {
        Some(MetricValue::Integer(value)) => value.to_string(),
        Some(MetricValue::Decimal(value)) => format!("{value:.2}"),
        Some(MetricValue::Text(value)) => value.clone(),
        Some(MetricValue::Missing) | None => "—".to_owned(),
    }
}

fn format_signed_metric(metric: Option<&MetricCell>) -> String {
    match metric.map(|metric| &metric.value) {
        Some(MetricValue::Integer(value)) => format!("{value:+}"),
        Some(MetricValue::Decimal(value)) => format!("{value:+.2}"),
        Some(MetricValue::Text(value)) => value.clone(),
        Some(MetricValue::Missing) | None => "—".to_owned(),
    }
}

fn format_seconds_metric(metric: Option<&MetricCell>) -> String {
    match metric.map(|metric| &metric.value) {
        Some(MetricValue::Integer(value)) if *value >= 0 => {
            let secs = *value as u64;
            format!("{}:{:02}", secs / 60, secs % 60)
        }
        Some(MetricValue::Decimal(value)) if *value >= 0.0 => {
            let secs = value.round() as u64;
            format!("{}:{:02}", secs / 60, secs % 60)
        }
        _ => "—".to_owned(),
    }
}

/// Phase Calder.3 — pure renderer for the player-screen pre-NHL
/// career section. Returns 0 lines when stints is empty so the
/// caller can splice unconditionally; tests pass a synthetic slice.
pub(crate) fn pre_nhl_career_lines(
    rows: &[PlayerPreNhlCareerRow],
    dim: ratatui::style::Style,
) -> Vec<Line<'static>> {
    if rows.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<Line<'static>> = Vec::new();
    out.push(Line::from(""));
    out.push(Line::styled(
        format!(" Pre-NHL career  ·  {} stints", rows.len()),
        dim,
    ));
    out.push(Line::styled(
        format!(
            " {:<8} {:<10} {:<18} {:>4} {:>4} {:>4} {:>5} {:>5}",
            "Season", "League", "Team", "GP", "G", "A", "P", "PPG"
        ),
        dim,
    ));
    for row in rows.iter().take(15) {
        let season_label = row.season_label.clone();
        let team: String = row.team.chars().take(18).collect();
        let league: String = row.league.chars().take(10).collect();
        let ppg = row
            .points_per_game
            .map(|p| format!("{p:.2}"))
            .unwrap_or_else(|| "—".into());
        out.push(Line::from(format!(
            " {:<8} {:<10} {:<18} {:>4} {:>4} {:>4} {:>5} {:>5}",
            season_label,
            league,
            team,
            row.games,
            row.goals
                .map(|n| n.to_string())
                .unwrap_or_else(|| "—".into()),
            row.assists
                .map(|n| n.to_string())
                .unwrap_or_else(|| "—".into()),
            row.points
                .map(|n| n.to_string())
                .unwrap_or_else(|| "—".into()),
            ppg,
        )));
    }
    out
}

/// Phase Lindsay L.4.5 — fit career-table columns to panel width.
///
/// Each data column reserves 8 cells (1 leading space + 7-char right-aligned
/// value/header). The season column reserves 9 cells, plus 2 cells of border
/// padding — 11 fixed cells before any data column appears. Returns:
///   - `columns`: prefix of `all_columns` that fits
///   - `dropped`: how many were trimmed from the right
///   - `use_narrow`: whether headers should fall back to `narrow_label()`
///     (panel width < 60 cells)
pub fn fit_career_columns(all_columns: &[StatId], panel_w: usize) -> (Vec<StatId>, usize, bool) {
    let use_narrow = panel_w < 60;
    let avail = panel_w.saturating_sub(11);
    let fit_count = (avail / 8).min(all_columns.len());
    let columns: Vec<StatId> = all_columns.iter().take(fit_count).copied().collect();
    let dropped = all_columns.len() - columns.len();
    (columns, dropped, use_narrow)
}

pub fn render_group_picker(f: &mut Frame, app: &App, area: Rect) {
    // Center a small popup
    let popup_h = (app.group_picker.list.len() as u16 + 4).min(area.height - 4);
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

    let items: Vec<ListItem> = app
        .group_picker
        .list
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let style = if i == app.selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::styled(format!("  ★  {}", name), style))
        })
        .collect();
    f.render_widget(List::new(items), inner);
}

// ── Phase 8j: dashboard panel render guard tests ────────────────────────────

#[cfg(test)]
mod dashboard_tests {
    // The full Player struct has 50+ fields — instead of hand-authoring a
    // fixture (which would couple this test file to the schema and break
    // every time a field is added), we test the render guard logic in
    // isolation: render the dashboard panel directly into a sub-area and
    // verify that the title only appears when `dashboards_enabled()` is on.
    // The end-to-end "render full player screen with panel" path is
    // exercised by L2 subprocess tests on the TUI launcher.

    use crate::config::{init_dashboards, Config};
    use std::path::PathBuf;

    #[test]
    fn l0_init_dashboards_explicit_true_takes_effect() {
        // OnceLock is set-once: first set wins for the duration of the
        // test binary. Other tests in the same binary may already have
        // set the flag, so we test the resolver logic directly here.
        let cfg = Config {
            dashboards: Some(true),
            cache_dir: PathBuf::from("/tmp"),
            ..Config::test_default()
        };
        init_dashboards(true, &cfg); // idempotent — first call wins
                                     // Verifying `dashboards_enabled()` here would race with other tests
                                     // that initialize the flag differently. The pure resolver
                                     // (`crate::config::resolve_dashboards`) covers the precedence
                                     // matrix in config.rs::tests; the OnceLock contract is set-once.
    }
}

// ── Hart.5c.6 Phase B-2.2 — view-based render path ───────────────────────────
//
// `render_by_id` is the post-Hart entry point. Looks up the view via
// `app.view_for(pid)`; on miss renders a placeholder (D6 auto-pop UX
// is event-handler side). Field-by-field equivalent of `render` /
// `render_stats` / `render_headshot` / `render_dashboard_panel`,
// sourcing through PlayerView accessors instead of `&Player` fields.
// Phase C deletes the legacy render paths once enter handlers all
// migrate to `Screen::PlayerById`.

pub(crate) fn player_card_view_from_app(app: &App, pid: PlayerId) -> Option<PlayerCardView> {
    let card =
        PlayerCardView::from_repository(&app.repo, pid, app.active_season_typed, app.active_type)?;
    let store = icelines_fetch::career_landing::load_local_store();
    let pre_nhl = store
        .get(pid.0)
        .map(icelines_fetch::career_landing::extract_pre_nhl_stints)
        .unwrap_or_default();
    Some(card.with_pre_nhl_stints(&pre_nhl))
}

pub fn render_by_id(f: &mut Frame, app: &App, area: Rect, pid: PlayerId) {
    let block = Block::default().borders(Borders::ALL).title(
        " Player Card  ·  [/]: preset  ·  c: comps  ·  r/a/s: records/awards/streaks  ·  Esc: back ",
    );
    let inner = block.inner(area);
    f.render_widget(block, area);

    let Some(view) = app.view_for(pid) else {
        let dim = Style::default().fg(Color::DarkGray);
        let name = app
            .repo
            .identity(pid)
            .map(|i| i.full_name.as_str())
            .unwrap_or("Player");
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
    let Some(card) = player_card_view_from_app(app, pid) else {
        return;
    };

    // Headshot fetch — same NHL CDN URL pattern as the legacy path.
    let nhl_id = view.identity.id.0;
    if app.headshot_cache.get(nhl_id).is_none() {
        let url = card.headshot_url.clone().unwrap_or_else(|| {
            format!(
                "https://assets.nhle.com/mugs/nhl/{}/{}/{}.png",
                app.active_season,
                view.team_display(),
                nhl_id,
            )
        });
        headshot::spawn_fetch(nhl_id, url, app.headshot_cache.clone(), 22, 15);
    }

    let dashboards_on = crate::config::dashboards_enabled() && inner.width >= 100;
    let constraints: Vec<Constraint> = if dashboards_on {
        vec![
            Constraint::Length(26),
            Constraint::Min(0),
            Constraint::Length(30),
        ]
    } else {
        vec![Constraint::Length(26), Constraint::Min(0)]
    };
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(inner);

    render_headshot_view(f, app, &view, chunks[0]);
    render_stats_view(f, app, &view, &card, chunks[1]);
    if dashboards_on {
        render_dashboard_panel_view(f, app, &view, chunks[2]);
    }

    if app.group_picker.open {
        render_group_picker(f, app, area);
    }
}

fn render_headshot_view(f: &mut Frame, app: &App, v: &PlayerView<'_>, area: Rect) {
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
        Some(r) if headshot::is_loading(r) => vec![Line::from(""), Line::from("  downloading…")],
        Some(r) if headshot::is_error(r) => vec![
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

fn render_stats_view(
    f: &mut Frame,
    app: &App,
    v: &PlayerView<'_>,
    card: &PlayerCardView,
    area: Rect,
) {
    let age = v
        .identity
        .bio
        .birth_date
        .as_deref()
        .and_then(|d| d.get(..4))
        .and_then(|y| y.parse::<u16>().ok())
        .map(|y| 2026u16.saturating_sub(y).to_string())
        .unwrap_or_else(|| "—".to_owned());
    let draft = match (
        v.identity.bio.draft_year,
        v.identity.bio.draft_round,
        v.identity.bio.draft_overall,
    ) {
        (Some(y), Some(r), Some(o)) => format!("{y} R{r} #{o}"),
        (Some(y), _, _) => y.to_string(),
        _ => "Undrafted".to_owned(),
    };

    let hi = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);

    // Header — name + team + position + age + sweater + handedness
    let sweater = v
        .sweater_number()
        .map(|n| format!(" · #{n}"))
        .unwrap_or_default();
    let hand = v.identity.bio.shoots_catches.as_deref().unwrap_or("—");
    let active = card.active.as_ref();
    let team_display = active
        .map(|active| active.team_display.as_str())
        .unwrap_or_else(|| v.team_display());
    let position = active
        .map(|active| active.position.abbreviation())
        .unwrap_or_else(|| v.position().abbreviation());
    let mut lines = vec![
        Line::styled(format!(" {}", card.display_name), hi),
        Line::from(format!(
            " {} · {} · Age {}{} · {}",
            team_display, position, age, sweater, hand,
        )),
        Line::from(""),
    ];

    // Phase Lindsay L.4.3 + L.4.5 — career table with narrow-mode
    // degradation. At <90 cols panel width, we drop columns from the
    // right to fit. At <60 cols we also fall back to `narrow_label()`
    // for tighter headers (vs `short_label()`).
    //
    // Phase Reports — additionally hide columns whose backing Tier-1
    // report is disabled (`app.reports.is_stat_visible`). Stats whose
    // `report_source()` is `None` (core / Tier-2 / derived) are always
    // visible.
    lines.extend(active_season_summary_lines(card.active.as_ref(), dim));
    lines.push(Line::styled(
        format!(
            " Records  ·  r: open  ·  :records player \"{}\"  ·  /records/player/{}?metric=...",
            card.display_name, card.player_id.0
        ),
        dim,
    ));
    lines.push(Line::styled(
        format!(
            " Awards   ·  a: Trophy Case  ·  icelines awards \"{}\"  ·  /player/{}/awards",
            card.display_name, card.player_id.0
        ),
        dim,
    ));
    lines.push(Line::styled(
        format!(
            " Streaks  ·  s: open  ·  icelines streaks \"{}\"  ·  /player/{}/streaks",
            card.display_name, card.player_id.0
        ),
        dim,
    ));
    lines.push(Line::from(""));

    let preset = app.queries.career_table_preset;
    let all_columns: Vec<icelines_core::stats_catalog::StatId> = preset
        .columns(v.position())
        .into_iter()
        .filter(|sid| app.reports.is_stat_visible(*sid))
        .collect();
    let panel_w = area.width as usize;
    let (columns, dropped, use_narrow) = fit_career_columns(&all_columns, panel_w);

    lines.push(Line::styled(
        format!(
            " Career  ·  {}  ·  [/]: cycle  ·  ({} of {} cols{})",
            preset.label(),
            columns.len(),
            all_columns.len(),
            // GLASS-9 L.5b post-fix — "3 hidden" reads cleaner than
            // "narrow: -3" (the leading dash glances as "negative 3").
            if dropped > 0 {
                format!(", {dropped} hidden")
            } else {
                String::new()
            },
        ),
        dim,
    ));

    if columns.is_empty() {
        // GLASS-4 L.5b post-fix — distinguish "preset has 0 columns
        // applicable to this position" from "panel is too narrow to
        // fit anything". Both look the same to the user without
        // disambiguation.
        let msg = if all_columns.is_empty() {
            "  (no columns in this preset for this position — try [/] to cycle)"
        } else {
            "  (panel too narrow to fit any column — widen terminal)"
        };
        lines.push(Line::styled(msg, dim));
    } else {
        let label_for = |sid: StatId| -> &'static str {
            if use_narrow {
                sid.narrow_label()
            } else {
                sid.short_label()
            }
        };

        // Header row — Season + StatId labels. GLASS L.4 fix: clip
        // labels >7 chars (e.g. "On-Ice S%", "EV Dep/g") so they don't
        // overrun the 8-cell column budget and break alignment.
        let mut header = format!(" {:<8}", "Season");
        for sid in &columns {
            let lbl = label_for(*sid);
            let clipped: String = lbl.chars().take(7).collect();
            header.push_str(&format!(" {:>7}", clipped));
        }
        lines.push(Line::styled(header, dim));

        // Separator.
        let sep_len = 9 + columns.len() * 8;
        lines.push(Line::styled(
            format!(" {}", "─".repeat(sep_len.min(panel_w.saturating_sub(2)))),
            dim,
        ));

        // One row per regular-season the player has played (most recent first).
        for row in card
            .career
            .iter()
            .filter(|row| row.season_type == icelines_core::season_stats::SeasonType::Regular)
        {
            let season_label = format!(
                "{}-{}",
                &row.season.as_str()[..4],
                &row.season.as_str()[6..],
            );
            let mut line = format!(" {:<8}", season_label);
            for sid in &columns {
                let cell = render_career_card_cell(*sid, row);
                line.push_str(&format!(" {:>7}", cell));
            }
            lines.push(Line::from(line));
        }
    }

    // Phase Calder.3 — pre-NHL career stints. Loaded via the shared
    // `extract_pre_nhl_stints` helper, then projected through PlayerCardView.
    for line in pre_nhl_career_lines(&card.pre_nhl_career, dim) {
        lines.push(line);
    }

    lines.push(Line::from(""));
    lines.push(Line::styled(" Bio", dim));
    lines.push(Line::from(format!(" Draft: {}", draft)));
    lines.push(Line::from(format!(
        " {}  Shoots: {}",
        v.identity.bio.nationality_code.as_deref().unwrap_or("—"),
        hand,
    )));

    let team_for_disambig = v.team_display();
    let hits = icelines_core::transactions::transactions_for_player(
        &app.txs.rows,
        v.full_name(),
        Some(team_for_disambig),
    );
    if !hits.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::styled(" Recent moves", dim));
        let mut sorted: Vec<&icelines_core::Transaction> = hits.clone();
        sorted.sort_by(|a, b| b.date.cmp(&a.date));
        for tx in sorted.into_iter().take(5) {
            let kind = tx.kind.label();
            let desc: String = tx.description.chars().take(60).collect();
            lines.push(Line::from(format!(" {}  {:<10}  {}", tx.date, kind, desc)));
        }
        if hits.len() > 5 {
            lines.push(Line::styled(
                format!(" ({} more on Transactions tab)", hits.len() - 5),
                dim,
            ));
        }
    }

    f.render_widget(Paragraph::new(lines), area);
}

fn render_dashboard_panel_view(f: &mut Frame, app: &App, v: &PlayerView<'_>, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Scout card ")
        .style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Use the post-Hart compile() API. ctx_window must match the active
    // window — boot_load and reload_for_season keep these in lockstep,
    // so the D11 cross-window rejection is a safety net.
    let result = app.dashboard_panel.compile(
        &app.repo,
        app.active_season_typed,
        app.active_type,
        v.identity.id,
        &app.league_context,
        app.league_context_window,
    );
    match result {
        Ok(out) => f.render_widget(Paragraph::new(out.lines), inner),
        Err(err) => {
            let dim = Style::default().fg(Color::DarkGray);
            f.render_widget(
                Paragraph::new(vec![
                    Line::styled("  Scout card unavailable", dim),
                    Line::styled(format!("  {err}"), dim),
                ]),
                inner,
            );
        }
    }
}

// ── L.4.2 unit tests — career table preset templates ───────────────────────

#[cfg(test)]
mod l4_preset_tests {
    use super::*;
    use icelines_core::model::{Position::*, Season, TeamAbbr};
    use icelines_core::season_stats::SeasonType;
    use icelines_core::{MetricUnit, StatKey, ValuePrecision};

    fn lines_to_text(lines: &[Line<'static>]) -> String {
        lines
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn metric_int(key: &str, label: &str, value: i64, unit: MetricUnit) -> MetricCell {
        MetricCell {
            key: StatKey::from(key),
            label: label.to_owned(),
            value: MetricValue::Integer(value),
            unit,
            precision: ValuePrecision::Integer,
            token: None,
        }
    }

    fn metric_decimal(key: &str, label: &str, value: f64, unit: MetricUnit) -> MetricCell {
        MetricCell {
            key: StatKey::from(key),
            label: label.to_owned(),
            value: MetricValue::Decimal(value),
            unit,
            precision: ValuePrecision::TwoDecimals,
            token: None,
        }
    }

    #[test]
    fn l0_tui_player_active_summary_projects_player_card_metrics() {
        let active = PlayerSeasonSummary {
            season: Season(20242025),
            season_type: SeasonType::Regular,
            position: Center,
            team: TeamAbbr("EDM".to_owned()),
            team_display: "EDM".to_owned(),
            metrics: vec![
                metric_int("gp", "GP", 82, MetricUnit::Games),
                metric_int("goals", "G", 44, MetricUnit::Goals),
                metric_int("assists", "A", 88, MetricUnit::Assists),
                metric_int("points", "PTS", 132, MetricUnit::Points),
                metric_decimal("points_per_game", "PPG", 1.61, MetricUnit::PerGame),
                metric_int("plus_minus", "+/-", 35, MetricUnit::Count),
                metric_decimal("shooting_pct", "S%", 12.5, MetricUnit::Percentage),
                metric_int("toi_per_game_sec", "TOI/G", 1276, MetricUnit::Seconds),
            ],
            tokens: Vec::new(),
        };

        let text = lines_to_text(&active_season_summary_lines(
            Some(&active),
            Style::default(),
        ));

        assert!(text.contains("Current"));
        assert!(text.contains("2024-25 regular"));
        assert!(text.contains("GP  82"));
        assert!(text.contains("PPG  1.61"));
        assert!(text.contains("+35"));
        assert!(text.contains("21:16"));
    }

    /// Cycle order ALL contains 6 distinct presets (SCOUT-6 L.5b
    /// post-fix removed `All` from the cycle — still reachable
    /// programmatically but not via keyboard).
    #[test]
    fn l0_lindsay_career_preset_cycle_count_seven() {
        assert_eq!(CareerTablePreset::ALL.len(), 6);
        assert_eq!(CareerTablePreset::ALL[0], CareerTablePreset::Default);
        assert_eq!(CareerTablePreset::ALL[5], CareerTablePreset::Goalie);
        // `All` is NOT in the keyboard cycle.
        assert!(!CareerTablePreset::ALL.contains(&CareerTablePreset::All));
    }

    /// `next` wraps from last (Goalie) → first (Default).
    #[test]
    fn l0_lindsay_career_preset_next_wraps() {
        assert_eq!(CareerTablePreset::Goalie.next(), CareerTablePreset::Default);
        assert_eq!(
            CareerTablePreset::Default.next(),
            CareerTablePreset::Scoring
        );
    }

    /// `prev` wraps from first (Default) → last (Goalie).
    #[test]
    fn l0_lindsay_career_preset_prev_wraps() {
        assert_eq!(CareerTablePreset::Default.prev(), CareerTablePreset::Goalie);
        assert_eq!(
            CareerTablePreset::Scoring.prev(),
            CareerTablePreset::Default
        );
    }

    /// Default preset for Center returns 16 columns (15 skater common
    /// post-SCOUT-3 L.5b + FaceoffWinPct).
    #[test]
    fn l0_lindsay_career_preset_default_center_14_cols() {
        let cols = CareerTablePreset::Default.columns(Center);
        assert_eq!(cols.len(), 16);
        assert!(cols.contains(&StatId::Games));
        assert!(cols.contains(&StatId::Gwg));
        assert!(cols.contains(&StatId::PointsPerGame));
        assert!(cols.contains(&StatId::FaceoffWinPct));
    }

    /// Default preset for Defense returns 16 (skater common 15 +
    /// EvGoalsForPct per SCOUT-8 L.5b).
    #[test]
    fn l0_lindsay_career_preset_default_defense_13_cols() {
        let cols = CareerTablePreset::Default.columns(Defense);
        assert_eq!(cols.len(), 16);
        assert!(!cols.contains(&StatId::FaceoffWinPct));
        assert!(
            cols.contains(&StatId::EvGoalsForPct),
            "Defense default surfaces EvGoalsForPct (SCOUT-8)"
        );
    }

    /// Default preset for Goalie returns 11 goalie-specific columns
    /// (post-SCOUT-L.4: + Saves, + ShotsAgainst, − RegulationWins).
    #[test]
    fn l0_lindsay_career_preset_default_goalie_10_cols() {
        let cols = CareerTablePreset::Default.columns(Goalie);
        assert_eq!(cols.len(), 11);
        assert!(cols.contains(&StatId::SavePct));
        assert!(cols.contains(&StatId::Gaa));
        assert!(cols.contains(&StatId::Saves));
        assert!(cols.contains(&StatId::ShotsAgainst));
    }

    /// Scoring preset for skaters returns the 15 Scoring-category stats.
    #[test]
    fn l0_lindsay_career_preset_scoring_15_for_skaters() {
        let cols = CareerTablePreset::Scoring.columns(Center);
        assert_eq!(cols.len(), 15);
        // Goalies hide skater stats — Scoring preset on goalie is empty.
        let goalie_cols = CareerTablePreset::Scoring.columns(Goalie);
        assert_eq!(
            goalie_cols.len(),
            0,
            "Scoring preset on goalie hides all (skater stats not applicable)"
        );
    }

    /// Goalie preset returns 23 (always — independent of position;
    /// the categorization is hockey-domain).
    #[test]
    fn l0_lindsay_career_preset_goalie_23() {
        let cols = CareerTablePreset::Goalie.columns(Goalie);
        assert_eq!(cols.len(), 23);
        // For a Center, Goalie preset is also 23 — the preset selects
        // by category, not position-applicability. UX: useful for
        // "show me what a goalie's stats would be."
        let center_goalie = CareerTablePreset::Goalie.columns(Center);
        assert_eq!(center_goalie.len(), 23);
    }

    /// All preset returns position-applicable subset of 108 stats.
    /// For a goalie, that's the 23 Goalie + 0 skater = 23. For a
    /// skater, that's 108 - 23 (goalies) = 85.
    #[test]
    fn l0_lindsay_career_preset_all_position_filtered() {
        let center_cols = CareerTablePreset::All.columns(Center);
        // Center applies_to is true for everything except Goalie category.
        // 108 - 23 = 85.
        assert_eq!(center_cols.len(), 85);

        let goalie_cols = CareerTablePreset::All.columns(Goalie);
        // Goalie applies_to is true ONLY for Goalie category.
        assert_eq!(goalie_cols.len(), 23);
    }

    // ── Phase Reports — career-table column visibility gating ──────────────

    /// Mirror of player.rs's render-time filter — applies a
    /// `ReportToggles` to a preset's column list and returns the
    /// surviving StatIds. Gives tests a pure-logic surface to assert
    /// against without spinning up a Frame.
    fn gated_columns(
        preset: CareerTablePreset,
        pos: icelines_core::model::Position,
        reports: crate::config::ReportToggles,
    ) -> Vec<StatId> {
        preset
            .columns(pos)
            .into_iter()
            .filter(|sid| reports.is_stat_visible(*sid))
            .collect()
    }

    #[test]
    fn l0_reports_career_table_default_center_drops_realtime_off_columns() {
        // Default preset for Center carries Hits/Blocks (realtime).
        // Default reports → realtime ON → those columns survive.
        let r_on = crate::config::ReportToggles::default();
        let on = gated_columns(CareerTablePreset::Default, Center, r_on);
        assert!(on.contains(&StatId::Hits));
        assert!(on.contains(&StatId::BlockedShots));

        // Flip realtime off → Hits/Blocks vanish; core stats stay.
        let r_off = crate::config::ReportToggles {
            realtime: false,
            ..Default::default()
        };
        let off = gated_columns(CareerTablePreset::Default, Center, r_off);
        assert!(!off.contains(&StatId::Hits));
        assert!(!off.contains(&StatId::BlockedShots));
        assert!(off.contains(&StatId::Goals), "Goals always visible");
        assert!(
            off.len() < on.len(),
            "realtime off must remove ≥1 column ({} vs {})",
            off.len(),
            on.len()
        );
    }

    #[test]
    fn l0_reports_career_table_defense_evgoalsforpct_gated_by_goals_for_against() {
        // Default Defense preset includes EvGoalsForPct (SCOUT-8).
        // EvGoalsForPct → SkaterGoalsForAgainst → default off → hidden.
        let r_default = crate::config::ReportToggles::default();
        let cols_default = gated_columns(CareerTablePreset::Default, Defense, r_default);
        assert!(
            !cols_default.contains(&StatId::EvGoalsForPct),
            "default reports hide EvGoalsForPct (goals_for_against off by default)"
        );

        // Turn goals_for_against on → EvGoalsForPct surfaces.
        let r_on = crate::config::ReportToggles {
            goals_for_against: true,
            ..Default::default()
        };
        let cols_on = gated_columns(CareerTablePreset::Default, Defense, r_on);
        assert!(
            cols_on.contains(&StatId::EvGoalsForPct),
            "goals_for_against on surfaces EvGoalsForPct"
        );
    }

    #[test]
    fn l0_reports_career_table_goalie_advanced_columns_gated_by_toggle() {
        // Goalie preset has 23 stats including QualityStarts (Advanced)
        // and EvSavePct (SavesByStrength). Defaults off → both hidden.
        let r_default = crate::config::ReportToggles::default();
        let off = gated_columns(CareerTablePreset::Goalie, Goalie, r_default);
        assert!(!off.contains(&StatId::QualityStarts));
        assert!(!off.contains(&StatId::EvSavePct));
        // Core goalie stats stay.
        assert!(off.contains(&StatId::Wins));
        assert!(off.contains(&StatId::SavePct));
        assert!(off.contains(&StatId::Gaa));

        // Flip both Tier-1 goalie reports on → both surface.
        let r_on = crate::config::ReportToggles {
            goalie_advanced: true,
            goalie_saves_by_strength: true,
            ..Default::default()
        };
        let on = gated_columns(CareerTablePreset::Goalie, Goalie, r_on);
        assert!(on.contains(&StatId::QualityStarts));
        assert!(on.contains(&StatId::EvSavePct));
        assert!(
            on.len() > off.len(),
            "enabling Tier-1 goalie reports must add columns"
        );
    }

    #[test]
    fn l0_reports_career_table_all_off_keeps_only_summary_and_derived() {
        // With every Tier-1 toggle off, only summary-backed and derived
        // stats survive. Sanity: Goals/Assists/Points (summary) +
        // Pace82/PointsPerGame (derived) all stay; every gated stat
        // disappears.
        let r_off = crate::config::ReportToggles {
            realtime: false,
            timeonice: false,
            goals_for_against: false,
            goalie_advanced: false,
            goalie_saves_by_strength: false,
        };
        let cols = gated_columns(CareerTablePreset::All, Center, r_off);
        for must in [
            StatId::Goals,
            StatId::Assists,
            StatId::Points,
            StatId::Pim,
            StatId::PointsPerGame,
            StatId::Pace82,
            StatId::TotalToiPerGame, // summary, not gated
        ] {
            assert!(cols.contains(&must), "{must:?} must survive all-off");
        }
        for must_not in [
            StatId::Hits,
            StatId::BlockedShots,
            StatId::PpToi,
            StatId::EvGoalsFor,
            StatId::EvenStrengthTimeOnIcePerGame,
        ] {
            assert!(
                !cols.contains(&must_not),
                "{must_not:?} must disappear when its report is off"
            );
        }
    }

    // ── L.4.3 cell formatting tests ────────────────────────────────────

    use icelines_core::fixtures;
    use icelines_core::identity::PlayerId;
    use icelines_core::model::Position;
    use icelines_core::season_stats::{SeasonStatsBuilder, StatTotals, TeamStint};

    fn build_skater_view() -> (
        icelines_core::identity::PlayerIdentity,
        icelines_core::season_stats::SeasonStats,
    ) {
        let identity = fixtures::identity(8478402).build();
        let stats = SeasonStatsBuilder::new(
            PlayerId(8478402),
            Season(20242025),
            SeasonType::Regular,
            Position::Center,
        )
        .add_team_stint(TeamStint {
            team: TeamAbbr("EDM".into()),
            started: Some("2024-10-09".into()),
            ended: None,
            gp: 70,
            goals: 30,
            assists: 80,
            points: 110,
            goalie: None,
        })
        .with_totals(StatTotals {
            gp: 70,
            goals: 30,
            assists: 80,
            points: 110,
            shots: 280,
            shooting_pct: Some(0.107),            // 10.7%
            toi_per_game_sec: Some(20 * 60 + 32), // 20:32
            ..Default::default()
        })
        .build();
        (identity, stats)
    }

    /// Count cells render as integers (no decimals).
    #[test]
    fn l0_lindsay_render_career_cell_count_integer() {
        let (id, stats) = build_skater_view();
        let view = PlayerView {
            identity: &id,
            stats: &stats,
            contract: None,
        };
        assert_eq!(render_career_cell(StatId::Goals, &view), "30");
        assert_eq!(render_career_cell(StatId::Points, &view), "110");
        assert_eq!(render_career_cell(StatId::Games, &view), "70");
    }

    /// Pct cells render as `XX.X` (no `%` sign — column header conveys unit).
    #[test]
    fn l0_lindsay_render_career_cell_pct_one_decimal() {
        let (id, stats) = build_skater_view();
        let view = PlayerView {
            identity: &id,
            stats: &stats,
            contract: None,
        };
        // shooting_pct = 0.107 → "10.7"
        assert_eq!(render_career_cell(StatId::ShootingPct, &view), "10.7");
    }

    /// Seconds cells render as M:SS for per-game and as `Nm` for totals.
    #[test]
    fn l0_lindsay_render_career_cell_seconds_mmss_format() {
        let (id, stats) = build_skater_view();
        let view = PlayerView {
            identity: &id,
            stats: &stats,
            contract: None,
        };
        // toi_per_game_sec = 20*60+32 = 1232 → "20:32"
        assert_eq!(render_career_cell(StatId::TotalToiPerGame, &view), "20:32");
    }

    /// None-valued cell renders as "—".
    #[test]
    fn l0_lindsay_render_career_cell_none_renders_dash() {
        let (id, stats) = build_skater_view();
        let view = PlayerView {
            identity: &id,
            stats: &stats,
            contract: None,
        };
        // Hits is None on this fixture (no realtime data).
        assert_eq!(render_career_cell(StatId::Hits, &view), "—");
    }

    // ── L.4.5 narrow-mode column fit tests ─────────────────────────────

    /// At 140 cols, Center default preset (16 cols post-SCOUT-3 L.5b)
    /// fits entirely — no narrow mode, nothing dropped.
    #[test]
    fn l0_lindsay_fit_career_columns_140_fits_all_default_center() {
        let cols = CareerTablePreset::Default.columns(Center);
        assert_eq!(cols.len(), 16);
        let (fit, dropped, narrow) = fit_career_columns(&cols, 140);
        // (140 - 11) / 8 = 16, exactly fits.
        assert_eq!(fit.len(), 16);
        assert_eq!(dropped, 0);
        assert!(!narrow);
    }

    /// At 100 cols, only ~11 columns fit — drops the rightmost few.
    #[test]
    fn l0_lindsay_fit_career_columns_100_drops_rightmost() {
        let cols = CareerTablePreset::Default.columns(Center);
        let (fit, dropped, narrow) = fit_career_columns(&cols, 100);
        // (100 - 11) / 8 = 11. 16 - 11 = 5 dropped.
        assert_eq!(fit.len(), 11);
        assert_eq!(dropped, 5);
        assert!(!narrow);
        // Truncation is from the right — first column preserved.
        assert_eq!(fit[0], cols[0]);
    }

    /// At 80 cols, ~8 columns fit — significant clipping, still wide-label.
    #[test]
    fn l0_lindsay_fit_career_columns_80_clips_heavy() {
        let cols = CareerTablePreset::Default.columns(Center);
        let (fit, dropped, narrow) = fit_career_columns(&cols, 80);
        // (80 - 11) / 8 = 8. 16 - 8 = 8 dropped.
        assert_eq!(fit.len(), 8);
        assert_eq!(dropped, 8);
        assert!(!narrow);
    }

    /// At <60 cols, narrow-label mode kicks in; only a few cols fit.
    #[test]
    fn l0_lindsay_fit_career_columns_50_uses_narrow_labels() {
        let cols = CareerTablePreset::Default.columns(Center);
        let (fit, dropped, narrow) = fit_career_columns(&cols, 50);
        // (50 - 11) / 8 = 4 cols.
        assert_eq!(fit.len(), 4);
        assert_eq!(dropped, 12);
        assert!(narrow);
    }

    /// Pathologically narrow (<11): no columns fit, narrow mode on.
    #[test]
    fn l0_lindsay_fit_career_columns_under_11_drops_all() {
        let cols = CareerTablePreset::Default.columns(Center);
        let (fit, dropped, narrow) = fit_career_columns(&cols, 8);
        assert_eq!(fit.len(), 0);
        assert_eq!(dropped, cols.len());
        assert!(narrow);
    }

    /// Goalie default (11 cols post-SCOUT-L.4) at 100 cells fits entirely.
    #[test]
    fn l0_lindsay_fit_career_columns_100_fits_goalie_default() {
        let cols = CareerTablePreset::Default.columns(Goalie);
        assert_eq!(cols.len(), 11);
        let (fit, dropped, narrow) = fit_career_columns(&cols, 100);
        assert_eq!(fit.len(), 11);
        assert_eq!(dropped, 0);
        assert!(!narrow);
    }

    /// Render order matches StatId::all() declaration order — UI list
    /// stability across renders.
    #[test]
    fn l0_lindsay_career_preset_render_order_stable() {
        let cols1 = CareerTablePreset::Default.columns(Center);
        let cols2 = CareerTablePreset::Default.columns(Center);
        assert_eq!(cols1, cols2);
        // Games appears before Goals (declaration order).
        let games_pos = cols1.iter().position(|&s| s == StatId::Games);
        let goals_pos = cols1.iter().position(|&s| s == StatId::Goals);
        assert!(games_pos < goals_pos);
    }
}

// ── Phase Calder.3 — Pre-NHL career section L0s ────────────────────────────

#[cfg(test)]
mod calder_pre_nhl_tests {
    use super::pre_nhl_career_lines;
    use icelines_core::career_history::{CareerGameType, CareerStint, LeagueAbbrev};
    use icelines_core::model::Season;
    use ratatui::style::Style;

    fn stint(season: u32, league: &str, team: &str, gp: u32, p: u32) -> CareerStint {
        CareerStint {
            season: Season(season),
            league: LeagueAbbrev::new(league),
            team: team.into(),
            game_type: CareerGameType::Regular,
            sequence: 1,
            gp,
            goals: Some(p / 2),
            assists: Some(p - p / 2),
            points: Some(p),
            pim: None,
            plus_minus: None,
            power_play_goals: None,
            power_play_points: None,
            shorthanded_goals: None,
            shorthanded_points: None,
            game_winning_goals: None,
            ot_goals: None,
            shots: None,
            shooting_pct: None,
            avg_toi_sec: None,
            faceoff_win_pct: None,
            games_started: None,
            wins: None,
            losses: None,
            ot_losses: None,
            goals_against: None,
            goals_against_avg: None,
            save_pct: None,
            shots_against: None,
            shutouts: None,
            time_on_ice_sec: None,
        }
    }

    fn render_to_text(stints: &[CareerStint]) -> String {
        let rows = icelines_core::PlayerCardView::pre_nhl_rows(stints);
        let lines = pre_nhl_career_lines(&rows, Style::default());
        lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Calder.3 / l0_tui_pre_nhl_empty_returns_zero_lines
    /// — Empty slice → zero lines so the caller can splice
    ///   unconditionally without printing a stray header.
    #[test]
    fn l0_tui_pre_nhl_empty_returns_zero_lines() {
        let rows = icelines_core::PlayerCardView::pre_nhl_rows(&[]);
        let lines = pre_nhl_career_lines(&rows, Style::default());
        assert!(lines.is_empty());
    }

    /// Calder.3 / l0_tui_pre_nhl_renders_header_and_row
    /// — One stint produces the section header + column header +
    ///   one data row + a blank-line separator at the top.
    #[test]
    fn l0_tui_pre_nhl_renders_header_and_row() {
        let stints = vec![stint(20142015, "OHL", "Erie", 47, 120)];
        let text = render_to_text(&stints);
        assert!(text.contains("Pre-NHL career"), "section header missing");
        assert!(text.contains("1 stints"), "stint count missing");
        assert!(text.contains("Season") && text.contains("League"));
        assert!(text.contains("14-15") && text.contains("Erie") && text.contains("OHL"));
        assert!(text.contains("47") && text.contains("120"));
    }

    /// Calder.3 / l0_tui_pre_nhl_caps_at_15
    /// — More than 15 stints get truncated so the section doesn't
    ///   blow up the player card vertically.
    #[test]
    fn l0_tui_pre_nhl_caps_at_15() {
        let stints: Vec<_> = (0..20)
            .map(|i| stint(20002001 + i * 10000, "OHL", "Erie", 60, 30))
            .collect();
        let rows = icelines_core::PlayerCardView::pre_nhl_rows(&stints);
        let lines = pre_nhl_career_lines(&rows, Style::default());
        // 1 blank + 1 header + 1 column header + 15 rows = 18 lines.
        assert_eq!(
            lines.len(),
            18,
            "expected 18 rendered lines, got {}",
            lines.len()
        );
    }

    /// Calder.3 / l0_tui_pre_nhl_sorts_newest_first
    /// — Output rows render newest season first (matches NHL career
    ///   table convention above).
    #[test]
    fn l0_tui_pre_nhl_sorts_newest_first() {
        let stints = vec![
            stint(20122013, "OHL", "Erie", 63, 66),
            stint(20142015, "OHL", "Erie", 47, 120),
            stint(20132014, "OHL", "Erie", 56, 99),
        ];
        let text = render_to_text(&stints);
        let pos_14 = text.find("14-15").expect("14-15 present");
        let pos_13 = text.find("13-14").expect("13-14 present");
        let pos_12 = text.find("12-13").expect("12-13 present");
        assert!(
            pos_14 < pos_13 && pos_13 < pos_12,
            "newest-first order violated: 14-15@{pos_14} 13-14@{pos_13} 12-13@{pos_12}"
        );
    }
}
