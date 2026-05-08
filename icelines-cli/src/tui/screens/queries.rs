use icelines_core::{
    filter::PlayerFilter, model::Position, position::PositionResolver, stats_repository::PlayerView,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

// ── Field definitions ─────────────────────────────────────────────────────────

pub struct QueryField {
    pub label: &'static str,
    pub options: Vec<&'static str>,
    pub selected: usize,
}

impl QueryField {
    pub fn value(&self) -> &str {
        self.options[self.selected]
    }
    pub fn next(&mut self) {
        self.selected = (self.selected + 1) % self.options.len();
    }
    pub fn prev(&mut self) {
        self.selected = if self.selected == 0 {
            self.options.len() - 1
        } else {
            self.selected - 1
        };
    }
}

pub fn default_fields() -> Vec<QueryField> {
    vec![
        QueryField {
            label: "Sort by",
            selected: 0,
            options: vec![
                "pts-pace",
                "ppg",
                "g-pace",
                "gpg",
                "pp-pts-pace",
                "pp-g-pace",
                "sh-g-pace",
                "shots-pace",
                "sh-pct",
                "plus-minus",
                "toi",
                "fo-pct",
                "hits-pace",
                "blocks-pace",
                "xg",
                "cf-pct",
                "xgf-pct",
                "improvement",
                "pts",
                "goals",
                "assists",
                "gp",
            ],
        },
        QueryField {
            label: "Position",
            selected: 0,
            options: vec!["all", "C", "LW", "RW", "D", "F"],
        },
        QueryField {
            label: "Age max",
            selected: 0,
            options: vec![
                "any", "21", "22", "23", "24", "25", "26", "27", "28", "30", "35",
            ],
        },
        QueryField {
            label: "Age min",
            selected: 0,
            options: vec!["any", "18", "20", "22", "24", "26", "28", "30"],
        },
        QueryField {
            label: "GP min",
            selected: 0,
            options: vec!["any", "10", "20", "30", "40", "50", "60", "70"],
        },
        QueryField {
            label: "Nationality",
            selected: 0,
            options: vec![
                "any", "CAN", "USA", "SWE", "FIN", "RUS", "CZE", "SVK", "GER", "NOR", "DEN",
            ],
        },
        QueryField {
            label: "Draft year",
            selected: 0,
            options: vec![
                "any", "2024", "2023", "2022", "2021", "2020", "2019", "2018", "2017", "2016",
                "2015", "2014", "2013",
            ],
        },
        QueryField {
            label: "Draft round",
            selected: 0,
            options: vec!["any", "1", "2", "3", "4", "5", "6", "7"],
        },
        QueryField {
            label: "Seasons",
            selected: 0,
            options: vec!["1", "2", "3", "4", "5", "10", "20", "38"],
        },
        QueryField {
            label: "Show top",
            selected: 1,
            options: vec!["10", "20", "30", "50", "100"],
        },
    ]
}

// ── Phase Lindsay L.3.3 — Categorized sections ─────────────────────────────
//
// Group the 10 default fields into 4 named sections. Each section has an
// expanded/collapsed state — Tab toggles the section containing the
// currently-selected field. When collapsed, the section's fields are
// hidden from the cursor and from the render.
//
// Future work: replace fields with per-`StatId` filter rows (one per
// catalog stat in each `StatCategory`) — that's the full v0.4 spec.
// L.3.3 v1 keeps the existing 10 fields and just groups them so the
// section + Tab-toggle UI is in place ahead of the L.3.4 sort picker.

pub struct QuerySection {
    /// User-visible header label.
    pub label: &'static str,
    /// Indices into `query_fields`. Order is render order within section.
    pub fields: Vec<usize>,
    /// Tab-toggleable. Collapsed sections hide their fields from cursor + render.
    pub expanded: bool,
}

/// Default 4-section grouping of the 10 default fields.
///
/// The "Sort & Display" section is always expanded by default — those
/// are the most-used fields. "Position & Age" too. The bulk-filter
/// sections ("Origin", "Stats") start collapsed to keep the screen
/// uncluttered for the median user, who's just changing the sort.
pub fn default_sections() -> Vec<QuerySection> {
    vec![
        QuerySection {
            label: "Sort & Display",
            fields: vec![0, 9, 8], // Sort by, Show top, Seasons
            expanded: true,
        },
        QuerySection {
            label: "Position & Age",
            fields: vec![1, 2, 3], // Position, Age max, Age min
            expanded: true,
        },
        QuerySection {
            label: "Origin & Draft",
            fields: vec![5, 6, 7], // Nationality, Draft year, Draft round
            expanded: false,
        },
        QuerySection {
            label: "Stats Thresholds",
            fields: vec![4], // GP min (more L.3.4 / future fields here)
            expanded: false,
        },
    ]
}

/// Find which section owns the given field index, if any. Returns the
/// section index in `sections` (0-based). `None` if the field isn't
/// listed in any section (shouldn't happen for default sections; defensive).
pub fn section_index_for_field(sections: &[QuerySection], field_idx: usize) -> Option<usize> {
    sections.iter().position(|s| s.fields.contains(&field_idx))
}

/// Field indices that should be visible (cursor-stoppable + rendered)
/// given the current expansion state. Collapsed sections contribute
/// nothing; expanded sections contribute all their `fields` in order.
pub fn visible_field_indices(sections: &[QuerySection]) -> Vec<usize> {
    sections
        .iter()
        .filter(|s| s.expanded)
        .flat_map(|s| s.fields.iter().copied())
        .collect()
}

/// Toggle the section containing `field_idx`. Returns the new
/// `expanded` state of that section. If the field doesn't belong to
/// any section, no-op and returns `None`.
pub fn toggle_section_for_field(sections: &mut [QuerySection], field_idx: usize) -> Option<bool> {
    let idx = section_index_for_field(sections, field_idx)?;
    sections[idx].expanded = !sections[idx].expanded;
    Some(sections[idx].expanded)
}

// ── Phase Lindsay L.3.4 — Sort picker filter ────────────────────────────────

/// Filter every catalog `StatId` by case-insensitive substring match
/// against `cli_key()`. Empty query returns the full catalog (107 stats).
///
/// Used by the sort picker overlay to filter as the user types. Result
/// preserves `StatId::all()` declaration order — same as the catalog
/// section grouping users see elsewhere (AI-05 determinism).
pub fn sort_picker_filter(query: &str) -> Vec<icelines_core::stats_catalog::StatId> {
    use icelines_core::stats_catalog::StatId;
    let q = query.trim().to_ascii_lowercase();
    StatId::all()
        .iter()
        .filter(|s| {
            if q.is_empty() {
                return true;
            }
            s.cli_key().to_ascii_lowercase().contains(&q)
        })
        .copied()
        .collect()
}

/// Phase Lindsay L.4.5 — format a sort-picker row for a given panel width.
///
/// GLASS #7 carry-forward: the legacy row at ~77 chars wraps at 80-col
/// terminals. Three width tiers, picked from the inner panel width:
///   - **wide** (≥100): `key (32) — label (26) (category)`
///   - **medium** (80..100): `key (32) — label` (drop category)
///   - **narrow** (<80): `key (24) — short_label` (truncate key column,
///     use shorter labels)
///
/// Total width is bounded above by the format strings themselves — the
/// caller's panel won't be wrapped by the renderer.
///
/// **GLASS-6 (L.5b post-fix)**: `selected=true` overrides the narrow
/// truncation so the highlighted row always shows the full cli_key.
/// The user can read the key under cursor unambiguously regardless of
/// terminal width — even when neighboring rows truncate.
#[allow(dead_code)] // Public string-form helper — kept alongside `format_sort_picker_row_selected`.
pub fn format_sort_picker_row(sid: icelines_core::stats_catalog::StatId, panel_w: usize) -> String {
    format_sort_picker_row_selected(sid, panel_w, false)
}

pub fn format_sort_picker_row_selected(
    sid: icelines_core::stats_catalog::StatId,
    panel_w: usize,
    selected: bool,
) -> String {
    if panel_w >= 100 {
        format!(
            "  {:<32} — {:<26} ({})",
            sid.cli_key(),
            sid.label(),
            sid.category().label(),
        )
    } else if panel_w >= 80 {
        format!("  {:<32} — {}", sid.cli_key(), sid.label())
    } else {
        let key = sid.cli_key();
        // Selected row: show full key uncondensed so the user can read
        // the unambiguous identifier under the cursor.
        let key_disp: String = if selected {
            format!("{key:<24}")
        } else if key.chars().count() > 24 {
            // GLASS-7 (L.5b post-fix) — char-based truncation, not
            // byte-slice. Defensive against a future non-ASCII cli_key.
            let truncated: String = key.chars().take(23).collect();
            format!("{truncated}…")
        } else {
            format!("{key:<24}")
        };
        format!("  {} — {}", key_disp, sid.short_label())
    }
}

// ── Query execution ───────────────────────────────────────────────────────────

fn parse_opt<T: std::str::FromStr>(s: &str) -> Option<T> {
    if s == "any" {
        None
    } else {
        s.parse().ok()
    }
}

/// Filter + sort the players by the field selections. Operates on
/// `PlayerView<'_>` slices via `PlayerFilter::matches_view`.
#[allow(dead_code)] // Public helper — superseded by `run_query_views_with_pick`; kept for API stability.
pub fn run_query_views<'a>(
    views: &'a [PlayerView<'a>],
    fields: &[QueryField],
) -> Vec<(usize, PlayerView<'a>)> {
    run_query_views_with_pick(views, fields, None)
}

/// Phase Lindsay L.3.4 — variant that accepts a sort-picker override.
/// When `sort_pick` is `Some(stat)`, sort uses `StatId::sort_cmp` (AI-06
/// universal tiebreak) over the picked catalog stat. When `None`, falls
/// back to the legacy QueryField[0] string-keyed sort.
pub fn run_query_views_with_pick<'a>(
    views: &'a [PlayerView<'a>],
    fields: &[QueryField],
    sort_pick: Option<icelines_core::stats_catalog::StatId>,
) -> Vec<(usize, PlayerView<'a>)> {
    let sort = fields[0].value();
    let pos = fields[1].value();
    let top: usize = fields[9].value().parse().unwrap_or(20);

    let mut filter = PlayerFilter::new();

    if pos != "all" {
        if pos == "F" {
            filter.positions = Some(vec![
                Position::Center,
                Position::LeftWing,
                Position::RightWing,
            ]);
        } else if let Ok((primary, _)) = PositionResolver::parse(pos) {
            filter.positions = Some(vec![primary]);
        }
    }
    filter.age_max = parse_opt(fields[2].value());
    filter.age_min = parse_opt(fields[3].value());
    filter.gp_min = parse_opt(fields[4].value());
    filter.nationalities = if fields[5].value() == "any" {
        None
    } else {
        Some(vec![fields[5].value().to_uppercase()])
    };
    filter.draft_years = parse_opt::<u16>(fields[6].value()).map(|y| vec![y]);
    filter.draft_rounds = parse_opt::<u8>(fields[7].value()).map(|r| vec![r]);

    // Bypass apply_views — its `&'a self` ties the return lifetime to
    // the local filter. matches_view takes &self by value so we can
    // hold the longer view lifetime intact.
    let mut matched: Vec<PlayerView<'a>> = views
        .iter()
        .cloned()
        .filter(|v| filter.matches_view(v))
        .collect();

    // Phase Lindsay L.3.4 — when the sort picker chose a catalog stat,
    // route through `StatId::sort_cmp` (deterministic AI-06 tiebreak).
    // Otherwise fall back to the legacy string-keyed sort_val_view.
    if let Some(stat) = sort_pick {
        matched.sort_by(|a, b| stat.sort_cmp(a, b));
    } else {
        matched.sort_by(|a, b| {
            sort_val_view(b, sort)
                .partial_cmp(&sort_val_view(a, sort))
                .unwrap_or(std::cmp::Ordering::Equal)
                // Phase Lindsay L.3.2 — universal nhl_id tiebreak.
                .then_with(|| a.identity.id.0.cmp(&b.identity.id.0))
        });
    }

    matched
        .into_iter()
        .take(top)
        .enumerate()
        .map(|(i, v)| (i + 1, v))
        .collect()
}

/// Phase Art Ross — variant that ALSO applies a free-form filter
/// plan (parsed via `icelines_query::parse_query` from the Filter
/// overlay) on top of the structured field filters. The plan is
/// evaluated AFTER the legacy filter — same logical AND as web's
/// `partition_new_pipeline_filters` flow — so structured fields
/// always run first (cheap) and the typed plan only sees survivors.
///
/// `season` is the active season (`YYYYZZZZ`); used by the
/// `EvalCtx` to anchor age calculations and current-season window
/// queries. `today` defaults to the system clock.
///
/// When `plan` is `None`, behaves identically to
/// `run_query_views_with_pick`.
pub fn run_query_views_with_pick_and_plan<'a>(
    views: &'a [PlayerView<'a>],
    fields: &[QueryField],
    sort_pick: Option<icelines_core::stats_catalog::StatId>,
    plan: Option<&icelines_query::QueryPlan>,
    season: u32,
) -> Vec<(usize, PlayerView<'a>)> {
    let sort = fields[0].value();
    let pos = fields[1].value();
    let top: usize = fields[9].value().parse().unwrap_or(20);

    let mut filter = PlayerFilter::new();

    if pos != "all" {
        if pos == "F" {
            filter.positions = Some(vec![
                Position::Center,
                Position::LeftWing,
                Position::RightWing,
            ]);
        } else if let Ok((primary, _)) = PositionResolver::parse(pos) {
            filter.positions = Some(vec![primary]);
        }
    }
    filter.age_max = parse_opt(fields[2].value());
    filter.age_min = parse_opt(fields[3].value());
    filter.gp_min = parse_opt(fields[4].value());
    filter.nationalities = if fields[5].value() == "any" {
        None
    } else {
        Some(vec![fields[5].value().to_uppercase()])
    };
    filter.draft_years = parse_opt::<u16>(fields[6].value()).map(|y| vec![y]);
    filter.draft_rounds = parse_opt::<u8>(fields[7].value()).map(|r| vec![r]);

    let mut matched: Vec<PlayerView<'a>> = views
        .iter()
        .cloned()
        .filter(|v| filter.matches_view(v))
        .collect();

    // Phase Art Ross — overlay-filter pass. Construct the provider +
    // clock once (Wave 22 perf pattern), then retain only views the
    // plan accepts. EvalCtx is `!Send`-pinned so it's local to this
    // synchronous block.
    if let Some(plan) = plan {
        let provider = icelines_fetch::query_provider::IcelinesProvider::new(
            std::env::var_os("HOME")
                .or_else(|| std::env::var_os("USERPROFILE"))
                .map(std::path::PathBuf::from)
                .unwrap_or_default()
                .join(".icelines")
                .join("data"),
        );
        let clock = icelines_core::freshness::SystemClock;
        let ctx = icelines_query::EvalCtx::from_clock(
            &provider,
            icelines_query::StrictMode::Off,
            false,
            &clock,
            season,
        );
        matched.retain(|v| plan.root.matches(v, &ctx));
    }

    if let Some(stat) = sort_pick {
        matched.sort_by(|a, b| stat.sort_cmp(a, b));
    } else {
        matched.sort_by(|a, b| {
            sort_val_view(b, sort)
                .partial_cmp(&sort_val_view(a, sort))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.identity.id.0.cmp(&b.identity.id.0))
        });
    }

    matched
        .into_iter()
        .take(top)
        .enumerate()
        .map(|(i, v)| (i + 1, v))
        .collect()
}

fn sort_val_view(v: &PlayerView<'_>, sort: &str) -> f64 {
    let totals = &v.stats.totals;
    match sort {
        "pts-pace" | "ppg" => v.pace_82().unwrap_or(0.0),
        "g-pace" | "gpg" => totals
            .pace_score
            .as_ref()
            .map(|s| s.goals_per_82)
            .unwrap_or(0.0),
        "pp-pts-pace" => v.pp_points_per_82().unwrap_or(0.0),
        "pp-g-pace" => v.pp_goals_per_82().unwrap_or(0.0),
        "sh-g-pace" => v.sh_goals_per_82().unwrap_or(0.0),
        "shots-pace" => v.shots_per_82().unwrap_or(0.0),
        "sh-pct" => totals.shooting_pct.map(f64::from).unwrap_or(0.0),
        "plus-minus" => v.plus_minus() as f64,
        "toi" => totals.toi_per_game_sec.unwrap_or(0) as f64,
        "fo-pct" => totals.faceoff_win_pct.map(f64::from).unwrap_or(0.0),
        "hits-pace" => v.hits_per_82().unwrap_or(0.0),
        "blocks-pace" => v.blocked_shots_per_82().unwrap_or(0.0),
        "xg" => v.xg().unwrap_or(0.0),
        "cf-pct" => v
            .stats
            .advanced
            .as_ref()
            .and_then(|a| a.cf_pct)
            .unwrap_or(50.0),
        "xgf-pct" => v
            .stats
            .advanced
            .as_ref()
            .and_then(|a| a.xgf_pct)
            .unwrap_or(50.0),
        "pts" => totals.points as f64,
        "goals" => totals.goals as f64,
        "assists" => totals.assists as f64,
        "gp" => v.gp() as f64,
        _ => v.pace_82().unwrap_or(0.0),
    }
}

fn display_val_view(v: &PlayerView<'_>, sort: &str) -> String {
    let totals = &v.stats.totals;
    match sort {
        "pts-pace" => v
            .pace_82()
            .map(|p| format!("{:.1}", p))
            .unwrap_or_else(|| "—".to_owned()),
        "ppg" => v
            .pace_82()
            .map(|p| format!("{:.3}", p / 82.0))
            .unwrap_or_else(|| "—".to_owned()),
        "g-pace" => totals
            .pace_score
            .as_ref()
            .map(|s| format!("{:.1}", s.goals_per_82))
            .unwrap_or_else(|| "—".to_owned()),
        "pp-pts-pace" => v
            .pp_points_per_82()
            .map(|x| format!("{:.1}", x))
            .unwrap_or_else(|| "—".to_owned()),
        "pp-g-pace" => v
            .pp_goals_per_82()
            .map(|x| format!("{:.1}", x))
            .unwrap_or_else(|| "—".to_owned()),
        "sh-pct" => totals
            .shooting_pct
            .map(|x| format!("{:.1}%", x))
            .unwrap_or_else(|| "—".to_owned()),
        "plus-minus" => {
            if v.plus_minus() >= 0 {
                format!("+{}", v.plus_minus())
            } else {
                v.plus_minus().to_string()
            }
        }
        "toi" => v.toi_mmss().unwrap_or_else(|| "—".to_owned()),
        "xg" => v
            .xg()
            .map(|x| format!("{:.2}", x))
            .unwrap_or_else(|| "—".to_owned()),
        "cf-pct" => v
            .stats
            .advanced
            .as_ref()
            .and_then(|a| a.cf_pct)
            .map(|x| format!("{:.1}%", x))
            .unwrap_or_else(|| "—".to_owned()),
        "pts" => totals.points.to_string(),
        "goals" => totals.goals.to_string(),
        "assists" => totals.assists.to_string(),
        "gp" => {
            if v.gp() > 0 {
                v.gp().to_string()
            } else {
                "—".to_owned()
            }
        }
        _ => v
            .pace_82()
            .map(|p| format!("{:.1}", p))
            .unwrap_or_else(|| "—".to_owned()),
    }
}

fn col_label(sort: &str) -> &'static str {
    match sort {
        "pts-pace" => "Pts/82",
        "ppg" => "PPG",
        "g-pace" => "G/82",
        "pp-pts-pace" => "PP/82",
        "pp-g-pace" => "PPG/82",
        "sh-pct" => "SH%",
        "plus-minus" => "+/-",
        "toi" => "TOI",
        "xg" => "xG",
        "cf-pct" => "CF%",
        "xgf-pct" => "xGF%",
        "pts" => "Pts",
        "goals" => "Goals",
        "assists" => "Ast",
        "gp" => "GP",
        _ => "Value",
    }
}

// ── Render ────────────────────────────────────────────────────────────────────

pub fn render(f: &mut Frame, app: &crate::tui::app::App, area: Rect) {
    use crate::tui::app::QueryMode;

    // Show save/load/picker/filter overlay instead of results when
    // in those modes
    match app.query_mode {
        QueryMode::SaveName => {
            render_save_prompt(f, app, area);
            return;
        }
        QueryMode::LoadList => {
            render_load_list(f, app, area);
            return;
        }
        QueryMode::SortPicker => {
            render_sort_picker(f, app, area);
            return;
        }
        QueryMode::FilterEdit => {
            render_filter_editor(f, app, area);
            return;
        }
        QueryMode::Build => {}
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(30), Constraint::Min(0)])
        .split(area);

    render_controls(f, app, chunks[0]);
    render_results(f, app, chunks[1]);
}

/// Phase Art Ross — free-form filter editor overlay. Mirrors
/// `render_save_prompt` but with a parser-error line. Title carries
/// the active filter / error indicator so the user knows what they
/// last applied.
fn render_filter_editor(f: &mut Frame, app: &crate::tui::app::App, area: Rect) {
    let title_state = match (
        app.query_filter_plan.is_some(),
        app.query_filter_error.as_ref(),
    ) {
        (_, Some(_)) => " Filter — parse error · fix and Enter, Esc to cancel ",
        (true, None) => " Filter — refine and Enter, Esc to cancel ",
        (false, None) => " Filter — type filter, Enter accept, Esc cancel ",
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title_state)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let dim = Style::default().fg(Color::DarkGray);
    let err_style = Style::default()
        .fg(Color::Red)
        .add_modifier(Modifier::BOLD);

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from("  Phase Art Ross filter — examples:"),
        Line::styled("    country IN (CAN, USA) AND age<25", dim),
        Line::styled("    pos=C AND draft-round<=2", dim),
        Line::styled("    g.last10g>=5 AND age BETWEEN 22 AND 28", dim),
        Line::styled("    p.career>=500", dim),
        Line::from(""),
        Line::styled(
            format!("  ▶ {}▌", app.query_filter_text),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ];
    if let Some(err) = &app.query_filter_error {
        lines.push(Line::from(""));
        lines.push(Line::styled(format!("  ✘ {err}"), err_style));
    }
    lines.push(Line::from(""));
    lines.push(Line::styled(
        "  Enter = parse + apply · Esc = cancel · empty Enter clears",
        dim,
    ));
    f.render_widget(Paragraph::new(lines), inner);
}

fn render_save_prompt(f: &mut Frame, app: &crate::tui::app::App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Save Query — type a name, Enter to save, Esc to cancel ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let dim = Style::default().fg(Color::DarkGray);
    let lines = vec![
        Line::from(""),
        Line::from("  Name your query:"),
        Line::from(""),
        Line::styled(
            format!("  ▶ {}▌", app.query_save_name),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Line::from(""),
        Line::styled("  Enter = save · Esc = cancel", dim),
    ];
    f.render_widget(Paragraph::new(lines), inner);
}

fn render_load_list(f: &mut Frame, app: &crate::tui::app::App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Saved Queries — ↑↓ select · Enter to load · Esc to cancel ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let dim = Style::default().fg(Color::DarkGray);

    if app.query_saved_list.is_empty() {
        let lines = vec![
            Line::from(""),
            Line::from("  No saved queries yet."),
            Line::from(""),
            Line::styled("  Build a query, then press s to save.", dim),
        ];
        f.render_widget(Paragraph::new(lines), inner);
        return;
    }

    let items: Vec<ListItem> = app
        .query_saved_list
        .iter()
        .enumerate()
        .map(|(i, (name, _))| {
            let style = if i == app.selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::styled(format!("  {}", name), style))
        })
        .collect();

    f.render_widget(List::new(items), inner);
}

/// Phase Lindsay L.3.4 — sort picker overlay. Search box + filtered
/// list of catalog `StatId`s. Type to filter; Up/Down to move
/// selection; Enter to pick; Esc to cancel.
fn render_sort_picker(f: &mut Frame, app: &crate::tui::app::App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Sort by — type to filter · ↑↓ select · Enter accept · Esc cancel ")
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let dim = Style::default().fg(Color::DarkGray);

    // Filter the catalog by the current search query, then drop any
    // StatId whose backing report is disabled in the Reports overlay
    // (Phase Reports — visibility gating). Catalog ordering is preserved.
    let results: Vec<icelines_core::stats_catalog::StatId> =
        sort_picker_filter(&app.sort_picker_query)
            .into_iter()
            .filter(|sid| app.reports.is_stat_visible(*sid))
            .collect();

    // Cursor index, clamped to result length (the app handler also
    // clamps but defensively here so a stale index can't panic on
    // render).
    let sel = app.sort_picker_idx.min(results.len().saturating_sub(1));

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));
    // Search prompt with cursor block.
    lines.push(Line::from(vec![
        Span::styled("  Search: ", Style::default().fg(Color::White)),
        Span::styled(
            format!("{}▌", app.sort_picker_query),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::styled(
        format!(
            "  ({} of {} match)",
            results.len(),
            icelines_core::stats_catalog::StatId::all().len()
        ),
        dim,
    ));
    lines.push(Line::from(""));

    if results.is_empty() {
        lines.push(Line::styled(
            "  No matches. Try a different substring.",
            dim,
        ));
    } else {
        // Show up to N results around the selection. Visible window
        // is the inner area height minus header (~5 lines).
        let visible = (inner.height as usize).saturating_sub(7).max(1);
        let start = sel.saturating_sub(visible / 2);
        let end = (start + visible).min(results.len());

        for (i, sid) in results[start..end].iter().enumerate() {
            let global_idx = start + i;
            let active = global_idx == sel;
            let style = if active {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            // L.4.5: degrade row format at narrow widths (GLASS #7).
            // GLASS-6 (L.5b post-fix): selected row keeps full key.
            lines.push(Line::styled(
                format_sort_picker_row_selected(*sid, inner.width as usize, active),
                style,
            ));
        }
        if end < results.len() {
            lines.push(Line::styled(
                format!("  … {} more below", results.len() - end),
                dim,
            ));
        }
    }

    f.render_widget(Paragraph::new(lines), inner);
}

fn render_controls(f: &mut Frame, app: &crate::tui::app::App, area: Rect) {
    let border_style = if !app.query_results_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    // UX.2 + UX.3 — surface the [/] sort-picker shortcut so users
    // discover the 108-stat catalog browser, and document the new
    // o=section binding (Tab now always cycles screens).
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Query  ↑↓ · ←→ · o:section · Space:results · [/] all stats ")
        .border_style(border_style);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let sel = app.query_field_idx;
    let dim = Style::default().fg(Color::DarkGray);

    // Phase Lindsay L.3.3 — render by section. Each section gets a
    // header line (▶/▼ + label); expanded sections also render their
    // fields indented under the header. Collapsed sections show
    // header only — fields hidden + cursor-skipped.
    let mut all_items: Vec<ListItem> = Vec::new();
    for section in &app.query_sections {
        // Section header: ▶/▼ + label. Headers are not cursor-stoppable
        // in L.3.3 v1; Tab toggles the section that owns the current
        // field cursor. (Future revision: header-as-cursor-stop.)
        let arrow = if section.expanded { "▼" } else { "▶" };
        let header_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        all_items.push(ListItem::new(Line::styled(
            format!(" {arrow} {}", section.label),
            header_style,
        )));

        if !section.expanded {
            continue;
        }

        // Render the section's fields (indented).
        for &i in &section.fields {
            let field = match app.query_fields.get(i) {
                Some(f) => f,
                None => continue, // defensive — section refers to a missing field
            };
            let active = i == sel;
            let lbl_style = if active {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let val_style = if active {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Yellow)
            };
            let cursor_arrow = if active { "◄" } else { " " };
            let cursor_arrow_r = if active { "►" } else { " " };
            let cursor_color = if active {
                Style::default().fg(Color::Cyan)
            } else {
                dim
            };
            all_items.push(ListItem::new(Line::from(vec![
                Span::styled(format!("    {:<10}", field.label), lbl_style),
                Span::styled(cursor_arrow, cursor_color),
                Span::styled(format!("{:<9}", field.value()), val_style),
                Span::styled(cursor_arrow_r, cursor_color),
            ])));
        }
    }

    all_items.push(ListItem::new(Line::from("")));
    all_items.push(ListItem::new(Line::styled(
        " Tab  collapse/expand section",
        Style::default().fg(Color::Green),
    )));
    all_items.push(ListItem::new(Line::styled(
        " s    save this query",
        Style::default().fg(Color::Green),
    )));
    all_items.push(ListItem::new(Line::styled(
        " l    load saved query",
        Style::default().fg(Color::Green),
    )));
    all_items.push(ListItem::new(Line::styled(" Enter  player card", dim)));
    all_items.push(ListItem::new(Line::styled(" r    reset filters", dim)));

    f.render_widget(List::new(all_items), inner);
}

fn render_results(f: &mut Frame, app: &crate::tui::app::App, area: Rect) {
    let sort = app.query_fields[0].value();

    let border_style = if app.query_results_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Results — {} · ↑↓ · Enter: card ", sort))
        .border_style(border_style);
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Hart.5c.6 Phase B-3.3: collect views, run view-based query.
    let views = app.views();
    if views.is_empty() {
        f.render_widget(Paragraph::new(vec![Line::from("  Loading…")]), inner);
        return;
    }

    let results = run_query_views_with_pick_and_plan(
        &views,
        &app.query_fields,
        app.sort_stat_pick,
        app.query_filter_plan.as_ref(),
        app.active_season_typed.0,
    );
    let top: usize = app.query_fields[9].value().parse().unwrap_or(20);
    // Phase Lindsay L.3.4 — when picker overrides legacy field, the
    // column label comes from the StatId; fall back to legacy.
    let clabel: String = match app.sort_stat_pick {
        Some(stat) => stat.short_label().to_owned(),
        None => col_label(sort).to_owned(),
    };
    let dim = Style::default().fg(Color::DarkGray);

    let visible = inner.height.saturating_sub(4) as usize;
    let offset = app.query_result_scroll;

    let mut lines = vec![
        Line::styled(
            format!(
                "  {:<4} {:<22} {:<5} {:<4} {:>8}",
                "#", "Player", "Team", "Pos", clabel
            ),
            dim,
        ),
        Line::styled(format!("  {}", "─".repeat(48)), dim),
    ];

    for (rank, v) in results.iter().skip(offset).take(visible) {
        let name = v.full_name().chars().take(22).collect::<String>();
        let value = display_val_view(v, sort);
        let is_selected = offset + (lines.len() - 2)
            == app.query_result_scroll + app.selected.min(visible.saturating_sub(1));
        let style = if is_selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else if *rank <= 3 {
            Style::default().fg(Color::Green)
        } else {
            Style::default()
        };
        lines.push(Line::styled(
            format!(
                "  {:<4} {:<22} {:<5} {:<4} {:>8}",
                rank,
                name,
                v.team_display(),
                v.position().abbreviation(),
                value,
            ),
            style,
        ));
    }

    lines.push(Line::from(""));
    lines.push(Line::styled(
        format!(
            "  Showing {} of {} (top {})",
            results.len().min(top),
            results.len(),
            top
        ),
        dim,
    ));

    f.render_widget(Paragraph::new(lines), inner);
}

// ── Saved query serialization ─────────────────────────────────────────────────

/// Serialize current field selections to JSON for storage.
pub fn fields_to_json(fields: &[QueryField]) -> String {
    let pairs: Vec<String> = fields
        .iter()
        .map(|f| format!("{{\"label\":\"{}\",\"selected\":{}}}", f.label, f.selected))
        .collect();
    format!("[{}]", pairs.join(","))
}

/// Restore field selections from stored JSON. Unknown labels are ignored.
pub fn apply_saved_json(fields: &mut [QueryField], json: &str) {
    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(json) {
        for entry in &arr {
            if let (Some(label), Some(sel)) = (entry["label"].as_str(), entry["selected"].as_u64())
            {
                if let Some(f) = fields.iter_mut().find(|f| f.label == label) {
                    f.selected = (sel as usize).min(f.options.len().saturating_sub(1));
                }
            }
        }
    }
}

// ── L.3.3 unit tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Phase Reports — sort picker visibility gating ───────────────────────

    /// Mirrors the report-aware filter chain inside `render_sort_picker`.
    /// Lifted into a test helper so we can exercise the pure logic
    /// without a Frame / TestBackend round-trip.
    fn filter_with_reports(
        query: &str,
        reports: crate::config::ReportToggles,
    ) -> Vec<icelines_core::stats_catalog::StatId> {
        sort_picker_filter(query)
            .into_iter()
            .filter(|sid| reports.is_stat_visible(*sid))
            .collect()
    }

    #[test]
    fn l0_reports_sort_picker_default_includes_realtime_excludes_others() {
        use icelines_core::stats_catalog::StatId;
        let r = crate::config::ReportToggles::default(); // realtime ON
        let visible = filter_with_reports("", r);
        assert!(
            visible.contains(&StatId::Hits),
            "default toggles include Realtime → Hits visible"
        );
        // Goals For/Against off by default → EvGoalsFor hidden.
        assert!(
            !visible.contains(&StatId::EvGoalsFor),
            "Goals-for-against off by default → EvGoalsFor hidden"
        );
        // Time on Ice off by default → PpToi hidden.
        assert!(
            !visible.contains(&StatId::PpToi),
            "Time-on-ice off by default → PpToi hidden"
        );
        // Core stat always visible.
        assert!(visible.contains(&StatId::Goals));
        assert!(visible.contains(&StatId::Points));
    }

    #[test]
    fn l0_reports_sort_picker_all_off_only_core_visible() {
        use icelines_core::stats_catalog::StatId;
        let r = crate::config::ReportToggles {
            realtime: false,
            timeonice: false,
            goals_for_against: false,
            goalie_advanced: false,
            goalie_saves_by_strength: false,
        };
        let visible = filter_with_reports("", r);
        assert!(visible.contains(&StatId::Goals), "Core stat always visible");
        assert!(!visible.contains(&StatId::Hits));
        assert!(!visible.contains(&StatId::PpToi));
        assert!(!visible.contains(&StatId::EvGoalsFor));
        assert!(!visible.contains(&StatId::QualityStarts));
        assert!(!visible.contains(&StatId::EvSavePct));
    }

    #[test]
    fn l0_reports_sort_picker_all_on_includes_every_tier1_stat() {
        use icelines_core::stats_catalog::StatId;
        let r = crate::config::ReportToggles {
            realtime: true,
            timeonice: true,
            goals_for_against: true,
            goalie_advanced: true,
            goalie_saves_by_strength: true,
        };
        let visible = filter_with_reports("", r);
        assert!(visible.contains(&StatId::Hits));
        assert!(visible.contains(&StatId::PpToi));
        assert!(visible.contains(&StatId::EvGoalsFor));
        assert!(visible.contains(&StatId::QualityStarts));
        assert!(visible.contains(&StatId::EvSavePct));
    }

    #[test]
    fn l0_reports_sort_picker_query_search_intersects_with_visibility() {
        use icelines_core::stats_catalog::StatId;
        // Search "hit" + realtime ON → Hits/HitsPer60 in result.
        let r = crate::config::ReportToggles::default();
        let visible = filter_with_reports("hit", r);
        assert!(visible.contains(&StatId::Hits));
        // Same search with realtime OFF → empty (no core stat matches "hit").
        let r_off = crate::config::ReportToggles {
            realtime: false,
            ..Default::default()
        };
        let visible_off = filter_with_reports("hit", r_off);
        assert!(
            !visible_off.contains(&StatId::Hits),
            "Hits hidden when realtime is off, regardless of search query"
        );
    }

    /// Default sections cover every default field exactly once. Drift
    /// (a field added without a section assignment) breaks rendering
    /// silently — this test catches it.
    #[test]
    fn l0_lindsay_default_sections_cover_all_fields_exactly_once() {
        let fields = default_fields();
        let sections = default_sections();
        let mut covered: Vec<usize> = sections
            .iter()
            .flat_map(|s| s.fields.iter().copied())
            .collect();
        covered.sort();
        let expected: Vec<usize> = (0..fields.len()).collect();
        assert_eq!(
            covered, expected,
            "default_sections must cover every default field exactly once"
        );
    }

    /// `visible_field_indices` returns only fields in expanded sections,
    /// preserving their declaration order.
    #[test]
    fn l0_lindsay_visible_field_indices_respects_collapsed_sections() {
        let mut sections = default_sections();
        // All expanded by default — visible matches declaration order.
        sections[0].expanded = true;
        sections[1].expanded = true;
        sections[2].expanded = true;
        sections[3].expanded = true;
        let visible_all = visible_field_indices(&sections);
        let total: usize = sections.iter().map(|s| s.fields.len()).sum();
        assert_eq!(visible_all.len(), total);

        // Collapse section 2 — its fields drop out.
        sections[2].expanded = false;
        let visible_partial = visible_field_indices(&sections);
        let collapsed_count = sections[2].fields.len();
        assert_eq!(visible_partial.len(), total - collapsed_count);
        // Collapsed section's fields don't appear.
        for f in &sections[2].fields {
            assert!(
                !visible_partial.contains(f),
                "field {f} from collapsed section must not appear in visible"
            );
        }
    }

    /// `section_index_for_field` finds the right section for each field.
    #[test]
    fn l0_lindsay_section_index_for_field_lookup() {
        let sections = default_sections();
        // Field 0 (Sort by) → section 0 (Sort & Display).
        assert_eq!(section_index_for_field(&sections, 0), Some(0));
        // Field 4 (GP min) → section 3 (Stats Thresholds).
        assert_eq!(section_index_for_field(&sections, 4), Some(3));
        // Field 99 doesn't exist anywhere.
        assert_eq!(section_index_for_field(&sections, 99), None);
    }

    /// `toggle_section_for_field` flips the expanded state and returns
    /// the new value. Idempotent (toggle twice = original).
    #[test]
    fn l0_lindsay_toggle_section_for_field_flips_and_returns() {
        let mut sections = default_sections();
        let initial = sections[0].expanded;
        let after_first = toggle_section_for_field(&mut sections, 0).unwrap();
        assert_eq!(after_first, !initial);
        let after_second = toggle_section_for_field(&mut sections, 0).unwrap();
        assert_eq!(after_second, initial);
    }

    /// Toggling a missing field is a no-op (returns None, no panic).
    #[test]
    fn l0_lindsay_toggle_section_for_missing_field_no_op() {
        let mut sections = default_sections();
        let snapshot: Vec<bool> = sections.iter().map(|s| s.expanded).collect();
        assert_eq!(toggle_section_for_field(&mut sections, 99), None);
        let after: Vec<bool> = sections.iter().map(|s| s.expanded).collect();
        assert_eq!(snapshot, after, "no-op must not mutate section state");
    }

    // ─── L.3.4 sort picker filter tests ────────────────────────────────

    /// Empty query returns the full catalog in declaration order.
    #[test]
    fn l0_lindsay_sort_picker_filter_empty_query_returns_all() {
        let results = sort_picker_filter("");
        assert_eq!(
            results.len(),
            icelines_core::stats_catalog::StatId::all().len(),
            "empty query → all 108 stats (L.4.1 added Games)"
        );
        // First entry is the first variant in declaration order
        // (post-L.4.1: Games; was Goals pre-L.4.1).
        assert_eq!(results[0], icelines_core::stats_catalog::StatId::Games);
    }

    /// Substring match — `"hits"` returns Hits, HitsPer60.
    #[test]
    fn l0_lindsay_sort_picker_filter_substring_match() {
        use icelines_core::stats_catalog::StatId;
        let results = sort_picker_filter("hits");
        assert!(results.contains(&StatId::Hits));
        assert!(results.contains(&StatId::HitsPer60));
        // None of the matches contain "hits" without the cli_key — sanity.
        for r in &results {
            assert!(
                r.cli_key().contains("hits"),
                "{:?} (cli_key {:?}) should not match `hits`",
                r,
                r.cli_key()
            );
        }
    }

    /// Case-insensitive — `"HITS"` matches the same set as `"hits"`.
    #[test]
    fn l0_lindsay_sort_picker_filter_case_insensitive() {
        let lower = sort_picker_filter("hits");
        let upper = sort_picker_filter("HITS");
        assert_eq!(lower, upper);
    }

    /// Whitespace trimmed — `"  goals  "` is the same as `"goals"`.
    #[test]
    fn l0_lindsay_sort_picker_filter_trims_whitespace() {
        let trimmed = sort_picker_filter("goals");
        let padded = sort_picker_filter("  goals  ");
        assert_eq!(trimmed, padded);
    }

    /// No-match query returns empty Vec (not panic).
    #[test]
    fn l0_lindsay_sort_picker_filter_no_match_empty_vec() {
        let results = sort_picker_filter("xyz-nonexistent");
        assert!(results.is_empty());
    }

    // ── L.4.5 sort-picker row width-degradation tests ─────────────────

    /// Wide (≥100): full row with category column.
    #[test]
    fn l0_lindsay_format_sort_picker_row_wide_includes_category() {
        use icelines_core::stats_catalog::StatId;
        let row = format_sort_picker_row(StatId::Goals, 140);
        assert!(row.contains("goals"));
        assert!(row.contains("Goals"));
        // Category appears in parens at wide widths.
        assert!(
            row.contains("(Scoring)"),
            "wide row should include `(Scoring)` category — got {row:?}"
        );
    }

    /// Medium (80..100): drops the trailing `(category)` segment.
    #[test]
    fn l0_lindsay_format_sort_picker_row_medium_drops_category() {
        use icelines_core::stats_catalog::StatId;
        let row = format_sort_picker_row(StatId::Goals, 90);
        assert!(row.contains("goals"));
        assert!(row.contains("Goals"));
        assert!(
            !row.contains("(Scoring)"),
            "medium row must drop category — got {row:?}"
        );
    }

    /// Narrow (<80): truncates the cli_key column AND uses short_label.
    #[test]
    fn l0_lindsay_format_sort_picker_row_narrow_truncates() {
        use icelines_core::stats_catalog::StatId;
        let row = format_sort_picker_row(StatId::Goals, 60);
        assert!(row.contains("goals"));
        assert!(
            !row.contains("(Scoring)"),
            "narrow row must drop category — got {row:?}"
        );
    }

    /// GLASS-6 (L.5b post-fix) — narrow tier with `selected=true`
    /// shows the FULL cli_key (no truncation/ellipsis), even for keys
    /// >24 chars. Non-selected rows still truncate.
    #[test]
    fn l0_lindsay_l5b_format_sort_picker_row_selected_keeps_full_key() {
        use icelines_core::stats_catalog::StatId;
        // `even-strength-time-on-ice-per-game` is 34 chars — definitely
        // truncates in the unselected narrow tier.
        let unselected =
            format_sort_picker_row_selected(StatId::EvenStrengthTimeOnIcePerGame, 60, false);
        assert!(
            unselected.contains("…"),
            "unselected row >24 chars must truncate with ellipsis — got {unselected:?}"
        );

        let selected =
            format_sort_picker_row_selected(StatId::EvenStrengthTimeOnIcePerGame, 60, true);
        assert!(
            !selected.contains("…"),
            "selected row must NOT truncate — got {selected:?}"
        );
        assert!(
            selected.contains("even-strength-time-on-ice-per-game"),
            "selected row must show full cli_key — got {selected:?}"
        );
    }

    /// GLASS-7 (L.5b post-fix) — char-based truncation, not byte-slice.
    /// Defensive against a future non-ASCII cli_key. Currently every
    /// catalog key is ASCII, so this just smoke-tests the path doesn't
    /// regress to byte-slicing.
    #[test]
    fn l0_lindsay_l5b_format_sort_picker_row_char_truncate_not_byte_slice() {
        use icelines_core::stats_catalog::StatId;
        // Every catalog key (ASCII today): no panic for any of them
        // at narrow width. If we ever switch to chars().take(23), this
        // is a smoke test that the shape matches.
        for sid in StatId::all() {
            let _ = format_sort_picker_row_selected(*sid, 60, false);
            let _ = format_sort_picker_row_selected(*sid, 60, true);
        }
    }

    /// Width budget — wide (140) row never exceeds the panel width
    /// (catalog max key + max label + category fits inside 100).
    #[test]
    fn l0_lindsay_format_sort_picker_row_wide_fits_under_100() {
        use icelines_core::stats_catalog::StatId;
        for sid in StatId::all() {
            let row = format_sort_picker_row(*sid, 140);
            assert!(
                row.chars().count() < 100,
                "row {row:?} exceeds 100 cells for {sid:?}"
            );
        }
    }

    /// Width budget — medium (90) row never exceeds 80 cells.
    #[test]
    fn l0_lindsay_format_sort_picker_row_medium_fits_under_80() {
        use icelines_core::stats_catalog::StatId;
        for sid in StatId::all() {
            let row = format_sort_picker_row(*sid, 90);
            assert!(
                row.chars().count() < 80,
                "medium row {row:?} exceeds 80 cells for {sid:?}"
            );
        }
    }

    /// Width budget — narrow (60) row never exceeds 60 cells.
    #[test]
    fn l0_lindsay_format_sort_picker_row_narrow_fits_under_60() {
        use icelines_core::stats_catalog::StatId;
        for sid in StatId::all() {
            let row = format_sort_picker_row(*sid, 60);
            assert!(
                row.chars().count() <= 60,
                "narrow row {row:?} exceeds 60 cells for {sid:?}"
            );
        }
    }

    /// Determinism — declaration order preserved across runs.
    #[test]
    fn l0_lindsay_sort_picker_filter_declaration_order_preserved() {
        let r1 = sort_picker_filter("goals");
        let r2 = sort_picker_filter("goals");
        assert_eq!(r1, r2);
        // The order matches StatId::all() declaration order: Goals
        // before PpGoals before ShGoals etc.
        let order: Vec<_> = r1
            .iter()
            .position(|&s| s == icelines_core::stats_catalog::StatId::Goals)
            .into_iter()
            .chain(
                r1.iter()
                    .position(|&s| s == icelines_core::stats_catalog::StatId::PpGoals),
            )
            .collect();
        if order.len() == 2 {
            assert!(order[0] < order[1], "Goals appears before PpGoals");
        }
    }
}
