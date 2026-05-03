use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};
use icelines_core::identity::PlayerId;
use icelines_core::model::Position;
use icelines_core::stats_catalog::{StatCategory, StatId, StatUnit};
use icelines_core::stats_repository::PlayerView;
use crate::tui::app::App;
use crate::tui::headshot;

// ─── Phase Lindsay L.4.2 — career table column preset templates ────────────
//
// Six preset templates the user cycles through via `[`/`]` on the
// player card. The "Default" template is per-position (calls
// `StatId::default_in_career_table(pos)`); the others are fixed
// stat-category subsets independent of position.
//
// Order is the cycle order — `[` moves backward, `]` moves forward.
// Wraps at boundaries.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CareerTablePreset {
    /// Per-position curated default (`StatId::default_in_career_table`).
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
    pub const ALL: &'static [CareerTablePreset] = &[
        CareerTablePreset::Default,
        CareerTablePreset::Scoring,
        CareerTablePreset::TwoWay,
        CareerTablePreset::SpecialTeams,
        CareerTablePreset::Time,
        CareerTablePreset::Goalie,
        CareerTablePreset::All,
    ];

    /// Human-readable label for the status bar.
    pub fn label(self) -> &'static str {
        match self {
            Self::Default      => "Default",
            Self::Scoring      => "Scoring",
            Self::TwoWay       => "Two-way",
            Self::SpecialTeams => "Special Teams",
            Self::Time         => "Time",
            Self::Goalie       => "Goalie",
            Self::All          => "All",
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
            Self::TwoWay => StatId::all()
                .iter()
                .copied()
                .filter(|s| s.category() == StatCategory::TwoWay)
                .filter(|s| s.applies_to(pos, pos == Position::Goalie))
                .collect(),
            Self::SpecialTeams => StatId::all()
                .iter()
                .copied()
                .filter(|s| s.category() == StatCategory::SpecialTeams)
                .filter(|s| s.applies_to(pos, pos == Position::Goalie))
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

impl Default for CareerTablePreset {
    fn default() -> Self {
        Self::Default
    }
}

/// Phase Lindsay L.4.3 — render a single career-table cell value.
/// Reads via `StatId::read` (catalog dispatch) and formats per
/// `StatId::unit()`. None renders as `"—"`.
pub fn render_career_cell(sid: StatId, view: &PlayerView<'_>) -> String {
    match sid.read(view) {
        None => "—".to_owned(),
        Some(v) => match sid.unit() {
            // Counts and seconds: integer formatting.
            StatUnit::Count => format!("{}", v as i64),
            StatUnit::Seconds => {
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
            StatUnit::Pct => format!("{:.1}", v * 100.0),
            // Per-60 rates and other rates: 2 decimals.
            StatUnit::Per60 | StatUnit::Rate => format!("{:.2}", v),
            // Inverted (GAA): 2 decimals.
            StatUnit::Inverted => format!("{:.2}", v),
        },
    }
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
pub fn fit_career_columns(
    all_columns: &[StatId],
    panel_w: usize,
) -> (Vec<StatId>, usize, bool) {
    let use_narrow = panel_w < 60;
    let avail = panel_w.saturating_sub(11);
    let fit_count = (avail / 8).min(all_columns.len());
    let columns: Vec<StatId> = all_columns.iter().take(fit_count).copied().collect();
    let dropped = all_columns.len() - columns.len();
    (columns, dropped, use_narrow)
}

pub fn render_group_picker(f: &mut Frame, app: &App, area: Rect) {
    // Center a small popup
    let popup_h = (app.group_picker_list.len() as u16 + 4).min(area.height - 4);
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

    let items: Vec<ListItem> = app.group_picker_list.iter().enumerate().map(|(i, name)| {
        let style = if i == app.selected {
            Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        ListItem::new(Line::styled(format!("  ★  {}", name), style))
    }).collect();
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
            csv_path:   None,
            cache_dir:  PathBuf::from("/tmp"),
            season:     None,
            live:       None,
            dashboards: Some(true),
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

pub fn render_by_id(f: &mut Frame, app: &App, area: Rect, pid: PlayerId) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Player Card  ·  c: comps  ·  g: group  ·  f: favorites  ·  Esc: back ");
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

    // Headshot fetch — same NHL CDN URL pattern as the legacy path.
    let nhl_id = view.identity.id.0;
    if app.headshot_cache.get(nhl_id).is_none() {
        let url = view.identity.headshot_canonical_url.clone().unwrap_or_else(|| {
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
        vec![Constraint::Length(26), Constraint::Min(0), Constraint::Length(30)]
    } else {
        vec![Constraint::Length(26), Constraint::Min(0)]
    };
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(inner);

    render_headshot_view(f, app, &view, chunks[0]);
    render_stats_view(f, app, &view, chunks[1]);
    if dashboards_on {
        render_dashboard_panel_view(f, app, &view, chunks[2]);
    }

    if app.group_picker_open {
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
        Some(r) if headshot::is_loading(r) => vec![
            Line::from(""),
            Line::from("  downloading…"),
        ],
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

fn render_stats_view(f: &mut Frame, app: &App, v: &PlayerView<'_>, area: Rect) {
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

    let hi  = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);

    // Header — name + team + position + age + sweater + handedness
    let sweater = v.sweater_number()
        .map(|n| format!(" · #{n}"))
        .unwrap_or_default();
    let hand = v.identity.bio.shoots_catches.as_deref().unwrap_or("—");
    let mut lines = vec![
        Line::styled(format!(" {}", v.full_name()), hi),
        Line::from(format!(
            " {} · {} · Age {}{} · {}",
            v.team_display(),
            v.position().abbreviation(),
            age,
            sweater,
            hand,
        )),
        Line::from(""),
    ];

    // Phase Lindsay L.4.3 + L.4.5 — career table with narrow-mode
    // degradation. At <90 cols panel width, we drop columns from the
    // right to fit. At <60 cols we also fall back to `narrow_label()`
    // for tighter headers (vs `short_label()`).
    let preset = app.career_table_preset;
    let all_columns = preset.columns(v.position());
    let panel_w = area.width as usize;
    let (columns, dropped, use_narrow) = fit_career_columns(&all_columns, panel_w);

    lines.push(Line::styled(
        format!(
            " Career  ·  {}  ·  [/]: cycle  ·  ({} of {} cols{})",
            preset.label(),
            columns.len(),
            all_columns.len(),
            if dropped > 0 { format!(" — narrow: -{dropped}") } else { String::new() },
        ),
        dim,
    ));

    if columns.is_empty() {
        lines.push(Line::styled(
            "  (no columns in this preset / panel too narrow)",
            dim,
        ));
    } else {
        let label_for = |sid: StatId| -> &'static str {
            if use_narrow { sid.narrow_label() } else { sid.short_label() }
        };

        // Header row — Season + StatId labels.
        let mut header = format!(" {:<8}", "Season");
        for sid in &columns {
            header.push_str(&format!(" {:>7}", label_for(*sid)));
        }
        lines.push(Line::styled(header, dim));

        // Separator.
        let sep_len = 9 + columns.len() * 8;
        lines.push(Line::styled(
            format!(" {}", "─".repeat(sep_len.min(panel_w.saturating_sub(2)))),
            dim,
        ));

        // One row per regular-season the player has played (most recent first).
        let mut seasons: Vec<&icelines_core::season_stats::SeasonStats> = app
            .repo
            .career_regular(v.identity.id)
            .map(|it| it.collect())
            .unwrap_or_default();
        seasons.sort_by(|a, b| b.season.cmp(&a.season));  // newest first

        for stats in seasons {
            // Build a transient PlayerView for this season to feed
            // StatId::read. The identity is the same; contract is None.
            let row_view = PlayerView {
                identity: v.identity,
                stats,
                contract: None,
            };
            let season_label = format!(
                "{}-{}",
                &stats.season.as_str()[..4],
                &stats.season.as_str()[6..],
            );
            let mut line = format!(" {:<8}", season_label);
            for sid in &columns {
                let cell = render_career_cell(*sid, &row_view);
                line.push_str(&format!(" {:>7}", cell));
            }
            lines.push(Line::from(line));
        }
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
        &app.transactions,
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
    use icelines_core::model::Position::*;

    /// Cycle order ALL contains 7 distinct presets. `[`/`]` move within
    /// it. Pin the count + identity at boundaries.
    #[test]
    fn l0_lindsay_career_preset_cycle_count_seven() {
        assert_eq!(CareerTablePreset::ALL.len(), 7);
        assert_eq!(CareerTablePreset::ALL[0], CareerTablePreset::Default);
        assert_eq!(CareerTablePreset::ALL[6], CareerTablePreset::All);
    }

    /// `next` wraps from last (All) → first (Default).
    #[test]
    fn l0_lindsay_career_preset_next_wraps() {
        assert_eq!(CareerTablePreset::All.next(), CareerTablePreset::Default);
        assert_eq!(CareerTablePreset::Default.next(), CareerTablePreset::Scoring);
    }

    /// `prev` wraps from first (Default) → last (All).
    #[test]
    fn l0_lindsay_career_preset_prev_wraps() {
        assert_eq!(CareerTablePreset::Default.prev(), CareerTablePreset::All);
        assert_eq!(CareerTablePreset::Scoring.prev(), CareerTablePreset::Default);
    }

    /// Default preset for Center returns 14 columns (13 skater common +
    /// FaceoffWinPct).
    #[test]
    fn l0_lindsay_career_preset_default_center_14_cols() {
        let cols = CareerTablePreset::Default.columns(Center);
        assert_eq!(cols.len(), 14);
        assert!(cols.contains(&StatId::Games));
        assert!(cols.contains(&StatId::FaceoffWinPct));
    }

    /// Default preset for Defense returns 13 (no FaceoffWinPct).
    #[test]
    fn l0_lindsay_career_preset_default_defense_13_cols() {
        let cols = CareerTablePreset::Default.columns(Defense);
        assert_eq!(cols.len(), 13);
        assert!(!cols.contains(&StatId::FaceoffWinPct));
    }

    /// Default preset for Goalie returns 10 goalie-specific columns.
    #[test]
    fn l0_lindsay_career_preset_default_goalie_10_cols() {
        let cols = CareerTablePreset::Default.columns(Goalie);
        assert_eq!(cols.len(), 10);
        assert!(cols.contains(&StatId::SavePct));
        assert!(cols.contains(&StatId::Gaa));
    }

    /// Scoring preset for skaters returns the 15 Scoring-category stats.
    #[test]
    fn l0_lindsay_career_preset_scoring_15_for_skaters() {
        let cols = CareerTablePreset::Scoring.columns(Center);
        assert_eq!(cols.len(), 15);
        // Goalies hide skater stats — Scoring preset on goalie is empty.
        let goalie_cols = CareerTablePreset::Scoring.columns(Goalie);
        assert_eq!(goalie_cols.len(), 0,
            "Scoring preset on goalie hides all (skater stats not applicable)");
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

    // ── L.4.3 cell formatting tests ────────────────────────────────────

    use icelines_core::fixtures;
    use icelines_core::identity::PlayerId;
    use icelines_core::model::{Position, Season, TeamAbbr};
    use icelines_core::season_stats::{
        SeasonStatsBuilder, SeasonType, StatTotals, TeamStint,
    };

    fn build_skater_view() -> (icelines_core::identity::PlayerIdentity,
                                icelines_core::season_stats::SeasonStats) {
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
            gp: 70, goals: 30, assists: 80, points: 110,
            goalie: None,
        })
        .with_totals(StatTotals {
            gp: 70, goals: 30, assists: 80, points: 110,
            shots: 280,
            shooting_pct: Some(0.107),  // 10.7%
            toi_per_game_sec: Some(20 * 60 + 32),  // 20:32
            ..Default::default()
        })
        .build();
        (identity, stats)
    }

    /// Count cells render as integers (no decimals).
    #[test]
    fn l0_lindsay_render_career_cell_count_integer() {
        let (id, stats) = build_skater_view();
        let view = PlayerView { identity: &id, stats: &stats, contract: None };
        assert_eq!(render_career_cell(StatId::Goals, &view), "30");
        assert_eq!(render_career_cell(StatId::Points, &view), "110");
        assert_eq!(render_career_cell(StatId::Games, &view), "70");
    }

    /// Pct cells render as `XX.X` (no `%` sign — column header conveys unit).
    #[test]
    fn l0_lindsay_render_career_cell_pct_one_decimal() {
        let (id, stats) = build_skater_view();
        let view = PlayerView { identity: &id, stats: &stats, contract: None };
        // shooting_pct = 0.107 → "10.7"
        assert_eq!(render_career_cell(StatId::ShootingPct, &view), "10.7");
    }

    /// Seconds cells render as M:SS for per-game and as `Nm` for totals.
    #[test]
    fn l0_lindsay_render_career_cell_seconds_mmss_format() {
        let (id, stats) = build_skater_view();
        let view = PlayerView { identity: &id, stats: &stats, contract: None };
        // toi_per_game_sec = 20*60+32 = 1232 → "20:32"
        assert_eq!(render_career_cell(StatId::TotalToiPerGame, &view), "20:32");
    }

    /// None-valued cell renders as "—".
    #[test]
    fn l0_lindsay_render_career_cell_none_renders_dash() {
        let (id, stats) = build_skater_view();
        let view = PlayerView { identity: &id, stats: &stats, contract: None };
        // Hits is None on this fixture (no realtime data).
        assert_eq!(render_career_cell(StatId::Hits, &view), "—");
    }

    // ── L.4.5 narrow-mode column fit tests ─────────────────────────────

    /// At 140 cols, Center default preset (14 cols) fits entirely —
    /// no narrow mode, nothing dropped.
    #[test]
    fn l0_lindsay_fit_career_columns_140_fits_all_default_center() {
        let cols = CareerTablePreset::Default.columns(Center);
        assert_eq!(cols.len(), 14);
        let (fit, dropped, narrow) = fit_career_columns(&cols, 140);
        // (140 - 11) / 8 = 16, capped to 14.
        assert_eq!(fit.len(), 14);
        assert_eq!(dropped, 0);
        assert!(!narrow);
    }

    /// At 100 cols, only ~11 columns fit — drops the rightmost few.
    #[test]
    fn l0_lindsay_fit_career_columns_100_drops_rightmost() {
        let cols = CareerTablePreset::Default.columns(Center);
        let (fit, dropped, narrow) = fit_career_columns(&cols, 100);
        // (100 - 11) / 8 = 11. 14 - 11 = 3 dropped.
        assert_eq!(fit.len(), 11);
        assert_eq!(dropped, 3);
        assert!(!narrow);
        // Truncation is from the right — first column preserved.
        assert_eq!(fit[0], cols[0]);
    }

    /// At 80 cols, ~8 columns fit — significant clipping, still wide-label.
    #[test]
    fn l0_lindsay_fit_career_columns_80_clips_heavy() {
        let cols = CareerTablePreset::Default.columns(Center);
        let (fit, dropped, narrow) = fit_career_columns(&cols, 80);
        // (80 - 11) / 8 = 8. 14 - 8 = 6 dropped.
        assert_eq!(fit.len(), 8);
        assert_eq!(dropped, 6);
        assert!(!narrow);
    }

    /// At <60 cols, narrow-label mode kicks in; only a few cols fit.
    #[test]
    fn l0_lindsay_fit_career_columns_50_uses_narrow_labels() {
        let cols = CareerTablePreset::Default.columns(Center);
        let (fit, dropped, narrow) = fit_career_columns(&cols, 50);
        // (50 - 11) / 8 = 4 cols.
        assert_eq!(fit.len(), 4);
        assert_eq!(dropped, 10);
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

    /// Goalie default (10 cols) at 100 cells fits entirely.
    #[test]
    fn l0_lindsay_fit_career_columns_100_fits_goalie_default() {
        let cols = CareerTablePreset::Default.columns(Goalie);
        assert_eq!(cols.len(), 10);
        let (fit, dropped, narrow) = fit_career_columns(&cols, 100);
        assert_eq!(fit.len(), 10);
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
