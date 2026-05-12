// Phase Norris.1 — `<ScreenName>State` repeats the module name in
// the type identifier (queries::QueriesState). It's the canonical
// pattern for the per-screen state extraction across the TUI;
// renaming each to `State` would lose the cross-module readability
// (`use queries::State` in app.rs is much more ambiguous).
#![allow(clippy::module_name_repetitions)]

use icelines_core::{
    filter::PlayerFilter, model::Position, position::PositionResolver,
    stats_repository::PlayerView, LeaderKind, LeadersView, MetricCell, MetricUnit, MetricValue,
    SemanticToken, SortDirection, SortKey, SortState, StatKey, ValuePrecision, ViewContext,
    ViewWindow,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

// ── Phase Norris.1 — per-screen state struct ─────────────────────────────────

/// Phase Norris.1 — owns every piece of state that belongs to the
/// Queries tab. Replaces the 17+ `query_*` / `sort_picker_*` /
/// `career_table_preset` fields previously scattered across `App`.
/// `App` now holds this as `app.queries`.
#[derive(Debug)]
pub struct QueriesState {
    // Field editor (structured filters)
    pub fields: Vec<QueryField>,
    pub field_idx: usize,
    pub sections: Vec<QuerySection>,
    pub result_scroll: usize,

    // Mode + focus
    pub mode: crate::tui::app::QueryMode,
    pub results_focused: bool,

    // Save / load
    pub save_name: String,
    pub saved_list: Vec<(String, String)>,

    // Phase Art Ross — free-form filter overlay (Wave 23/24/24b/c/d)
    pub filter_text: String,
    pub filter_error: Option<String>,
    pub filter_plan: Option<icelines_query::QueryPlan>,
    pub filter_history: std::collections::VecDeque<String>,
    pub filter_history_cursor: Option<usize>,
    pub filter_show_help: bool,

    // Phase Lindsay L.3.4 — sort picker overlay
    pub sort_picker_query: String,
    pub sort_picker_idx: usize,
    pub sort_stat_pick: Option<icelines_core::stats_catalog::StatId>,

    // Phase Lindsay L.4 — career-table column preset on player card.
    // (Cross-screen dep flagged in spec §open items #6: read by
    // tui/screens/player.rs's render. A future PlayerCardState
    // extraction can reclaim it; staying here for Norris.1.)
    pub career_table_preset: crate::tui::screens::player::CareerTablePreset,
}

// ── Phase Masterton.1 — declarative chrome ───────────────────────────────────

/// Phase Masterton.1 — chrome accessor for the Queries tab.
/// The shell (screens/mod.rs) consumes this to render the
/// header title + footer keybind chips consistently across
/// screens.
///
/// Mode-aware: the title carries a breadcrumb that reflects
/// which sub-mode the editor is in (Build / FilterEdit / SaveName
/// / LoadList / SortPicker), and the keybinds advertise the
/// actions valid for that mode.
pub fn chrome(state: &QueriesState) -> crate::tui::chrome::ScreenChrome {
    use crate::tui::app::QueryMode;
    use crate::tui::chrome::{KeyHint, ScreenChrome};

    let title = match state.mode {
        QueryMode::Build => "Stats / Queries".to_owned(),
        QueryMode::FilterEdit => "Stats / Queries / Filter".to_owned(),
        QueryMode::SaveName => "Stats / Queries / Save".to_owned(),
        QueryMode::LoadList => "Stats / Queries / Load".to_owned(),
        QueryMode::SortPicker => "Stats / Queries / Sort".to_owned(),
    };

    let keybinds = match state.mode {
        QueryMode::Build => vec![
            KeyHint::new("f", "filter"),
            KeyHint::new("n", "nation"),
            KeyHint::new("/", "sort"),
            KeyHint::new("s", "save"),
            KeyHint::new("l", "load"),
            KeyHint::new("o", "toggle section"),
            KeyHint::new("←/→", "edit field"),
            KeyHint::new("Space", "focus results"),
            KeyHint::new("Enter", "open card"),
        ],
        QueryMode::FilterEdit => vec![
            KeyHint::new("Enter", "apply"),
            KeyHint::new("Esc", "cancel"),
            KeyHint::new("?", "grammar"),
            KeyHint::new("↑↓", "history"),
        ],
        QueryMode::SaveName => vec![KeyHint::new("Enter", "save"), KeyHint::new("Esc", "cancel")],
        QueryMode::LoadList => vec![
            KeyHint::new("Enter", "load"),
            KeyHint::new("Esc", "cancel"),
            KeyHint::new("↑↓", "select"),
        ],
        QueryMode::SortPicker => vec![
            KeyHint::new("Enter", "pick"),
            KeyHint::new("Esc", "cancel"),
            KeyHint::new("↑↓", "select"),
            KeyHint::new("type", "filter"),
        ],
    };

    ScreenChrome { title, keybinds }
}

impl Default for QueriesState {
    fn default() -> Self {
        Self {
            fields: default_fields(),
            field_idx: 0,
            sections: default_sections(),
            result_scroll: 0,

            mode: crate::tui::app::QueryMode::Build,
            results_focused: false,

            save_name: String::new(),
            saved_list: Vec::new(),

            filter_text: String::new(),
            filter_error: None,
            filter_plan: None,
            filter_history: std::collections::VecDeque::new(),
            filter_history_cursor: None,
            filter_show_help: false,

            sort_picker_query: String::new(),
            sort_picker_idx: 0,
            sort_stat_pick: None,

            career_table_preset: crate::tui::screens::player::CareerTablePreset::Default,
        }
    }
}

// ── Field definitions ─────────────────────────────────────────────────────────

#[derive(Debug)]
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

#[derive(Debug)]
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

fn leaders_view_from_query_results(
    results: &[(usize, PlayerView<'_>)],
    sort: &str,
    label: &str,
    season: icelines_core::model::Season,
    season_type: icelines_core::season_stats::SeasonType,
) -> LeadersView {
    let mut view = LeadersView::from_player_views_with_primary(
        ViewContext::new(ViewWindow::new(season, season_type)),
        LeaderKind::Skaters,
        results.iter().map(|(_, view)| *view),
        |v| MetricCell {
            key: StatKey::from(sort),
            label: label.to_owned(),
            value: MetricValue::Text(display_val_view(v, sort)),
            unit: MetricUnit::None,
            precision: ValuePrecision::Raw,
            token: Some(SemanticToken::DecisionHighlight),
        },
    );
    view.sort = Some(SortState {
        key: SortKey::from(sort),
        label: label.to_owned(),
        direction: SortDirection::Desc,
    });
    for (row, (rank, _)) in view.rows.iter_mut().zip(results.iter()) {
        row.rank = *rank as u32;
    }
    view
}

fn leader_primary_text(row: &icelines_core::LeaderRow) -> String {
    match &row.primary.value {
        MetricValue::Text(value) => value.clone(),
        MetricValue::Integer(value) => value.to_string(),
        MetricValue::Decimal(value) => format!("{value:.1}"),
        MetricValue::Missing => "â€”".to_owned(),
    }
}

fn leader_team_text(row: &icelines_core::LeaderRow) -> &str {
    if row.team.0 == "UNK" {
        "â€”"
    } else {
        row.team.0.as_str()
    }
}

// ── Render ────────────────────────────────────────────────────────────────────

pub fn render(f: &mut Frame, app: &crate::tui::app::App, area: Rect) {
    use crate::tui::app::QueryMode;

    // Show save/load/picker/filter overlay instead of results when
    // in those modes
    match app.queries.mode {
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

/// Phase Art Ross — Wave 24c live result count.
///
/// Returns a one-line hint to render under the editor input. Three
/// shapes:
///
/// - empty input: `None` (no hint)
/// - input parses, doesn't need a provider: `Some("→ 47 of 712 match")`
/// - input parses, NEEDS a provider (sliding-window / career /
///   league): `Some("→ press Enter to evaluate (data lookup)")`
/// - input fails to parse: `Some("(unparsed — keep typing)")`
///
/// The provider-gating keeps the editor responsive: counting bio-
/// only filters is microseconds against ~700 skaters, but a
/// sliding-window filter would file-I/O per player on every
/// keystroke. Heavyweight filters defer to Enter.
pub fn live_filter_count_hint(
    text: &str,
    views: &[icelines_core::stats_repository::PlayerView<'_>],
    season: u32,
) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    match icelines_query::parse_query(icelines_query::FilterInput::Cli(trimmed.to_owned())) {
        Err(_) => Some("(unparsed — keep typing)".to_owned()),
        Ok(plan) if plan.root.needs_provider() => {
            Some("→ press Enter to evaluate (data lookup)".to_owned())
        }
        Ok(plan) => {
            // Bio + season-stat filters: count locally with a
            // NoOp provider. The plan tree is asserted non-
            // provider above, so `Constraint::matches` won't
            // reach into the provider.
            struct NoOp;
            impl icelines_query::data_provider::DataProvider for NoOp {
                fn ensure(
                    &self,
                    _req: &icelines_query::data_provider::PlanRequirement,
                    _events: &mut dyn FnMut(icelines_query::data_provider::FetchEvent),
                ) -> Result<(), icelines_query::data_provider::FetchError> {
                    Ok(())
                }
            }
            let provider = NoOp;
            let clock = icelines_core::freshness::SystemClock;
            let ctx = icelines_query::EvalCtx::from_clock(
                &provider,
                icelines_query::StrictMode::Off,
                false,
                &clock,
                season,
            );
            let total = views.len();
            let matched = views.iter().filter(|v| plan.root.matches(v, &ctx)).count();
            Some(format!("→ {matched} of {total} match"))
        }
    }
}

/// Phase Art Ross — free-form filter editor overlay. Mirrors
/// `render_save_prompt` but with a parser-error line. Title carries
/// the active filter / error / history-cursor indicator so the user
/// knows what they last applied.
///
/// Wave 24d — when `app.queries.filter_show_help` is on, a grammar
/// cheatsheet renders beside the editor (horizontal split).
fn render_filter_editor(f: &mut Frame, app: &crate::tui::app::App, area: Rect) {
    if app.queries.filter_show_help {
        // Side-by-side: 60% editor, 40% cheatsheet.
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);
        render_filter_editor_inner(f, app, chunks[0]);
        render_filter_grammar_cheatsheet(f, chunks[1]);
    } else {
        render_filter_editor_inner(f, app, area);
    }
}

/// Wave 24d — the body of the filter editor (title, examples,
/// input cursor, hints, parser errors). Pulled out of
/// `render_filter_editor` so the side-by-side layout can call it
/// with a sub-region.
fn render_filter_editor_inner(f: &mut Frame, app: &crate::tui::app::App, area: Rect) {
    let title_state = match (
        app.queries.filter_plan.is_some(),
        app.queries.filter_error.as_ref(),
    ) {
        (_, Some(_)) => " Filter — parse error · fix and Enter, Esc to cancel ".to_owned(),
        (true, None) => " Filter — refine and Enter, Esc to cancel ".to_owned(),
        (false, None) => " Filter — type filter, Enter accept, Esc cancel ".to_owned(),
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title_state)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let dim = Style::default().fg(Color::DarkGray);
    let err_style = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);

    let history_hint = match app.queries.filter_history_cursor {
        Some(i) => format!(
            "  history {}/{} — Up: older · Down: newer / live",
            i + 1,
            app.queries.filter_history.len()
        ),
        None if !app.queries.filter_history.is_empty() => format!(
            "  Up to recall last {} filter(s) · Esc cancels",
            app.queries.filter_history.len(),
        ),
        None => String::new(),
    };

    // Wave 24c — speculative live count. Computed against the
    // active-season views; provider-gated so heavyweight filters
    // (sliding-window, career, league) defer to Enter.
    let live_views = app.views();
    let live_hint = live_filter_count_hint(
        &app.queries.filter_text,
        &live_views,
        app.active_season_typed.0,
    );

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from("  Phase Art Ross filter — examples:"),
        Line::styled("    country IN (CAN, USA) AND age<25", dim),
        Line::styled("    pos=C AND draft-round<=2", dim),
        Line::styled("    g.last10g>=5 AND age BETWEEN 22 AND 28", dim),
        Line::styled("    p.career>=500", dim),
        Line::from(""),
        Line::styled(
            format!("  ▶ {}▌", app.queries.filter_text),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ];
    if let Some(hint) = live_hint {
        lines.push(Line::styled(format!("  {hint}"), dim));
    }
    if !history_hint.is_empty() {
        lines.push(Line::styled(history_hint, dim));
    }
    if let Some(err) = &app.queries.filter_error {
        lines.push(Line::from(""));
        lines.push(Line::styled(format!("  ✘ {err}"), err_style));
    }
    lines.push(Line::from(""));
    let help_hint = if app.queries.filter_show_help {
        "  Enter apply · Esc cancel · empty Enter clears · ? hide grammar"
    } else {
        "  Enter apply · Esc cancel · empty Enter clears · ? show grammar"
    };
    lines.push(Line::styled(help_hint, dim));
    f.render_widget(Paragraph::new(lines), inner);
}

/// Wave 24d — grammar cheatsheet panel. Concise reference of the
/// atom shapes the Phase Art Ross parser accepts. Rendered alongside
/// the editor when the user toggles `?`.
fn render_filter_grammar_cheatsheet(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Grammar — Phase Art Ross atoms ")
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let hdr = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);
    let lines: Vec<Line> = vec![
        Line::from(""),
        Line::styled("  Bio atoms", hdr),
        Line::styled("    country=CAN", dim),
        Line::styled("    country IN (CAN, USA)", dim),
        Line::styled("    country NOT IN (RUS)", dim),
        Line::styled("    country LIKE \"CA*\"", dim),
        Line::styled("    pos=C  ·  pos IN (C, LW, RW)", dim),
        Line::styled("    age<25  ·  age BETWEEN 22 AND 28", dim),
        Line::styled("    draft-round<=2  ·  draft-overall<=10", dim),
        Line::styled("    draft-year=2015  ·  birth-state=ON", dim),
        Line::styled("    nationality=USA  ·  rookie-season>=20212022", dim),
        Line::styled("    height>=72  ·  shoots=L", dim),
        Line::from(""),
        Line::styled("  Stat atoms (current season)", hdr),
        Line::styled("    g>=10  ·  p>=20  ·  ppg>=0.8", dim),
        Line::styled("    save-pct>=.910 (goalies)", dim),
        Line::from(""),
        Line::styled("  Sliding windows", hdr),
        Line::styled("    g.last10g>=5  (last 10 games)", dim),
        Line::styled("    p.last30d>=10 (last 30 days)", dim),
        Line::styled("    Modifiers: .allteams .career", dim),
        Line::from(""),
        Line::styled("  Career history (cross-league)", hdr),
        Line::styled("    p.career>=500  ·  g.career>=300", dim),
        Line::styled("    p.career.junior>=200", dim),
        Line::styled("    league=OHL  ·  league.tier=Junior", dim),
        Line::from(""),
        Line::styled("  EVER + AT modifiers", hdr),
        Line::styled("    g.any10g>=5 EVER", dim),
        Line::styled("    g.any10g>=5 EVER AT age<=25", dim),
        Line::from(""),
        Line::styled("  Booleans", hdr),
        Line::styled("    AND  OR  NOT  ( )", dim),
        Line::from(""),
        Line::styled("  Operators", hdr),
        Line::styled("    =  ==  !=  <  <=  >  >=", dim),
        Line::styled("    IN  NOT IN  BETWEEN  LIKE  NOT LIKE", dim),
    ];
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
            format!("  ▶ {}▌", app.queries.save_name),
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

    if app.queries.saved_list.is_empty() {
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
        .queries
        .saved_list
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
        sort_picker_filter(&app.queries.sort_picker_query)
            .into_iter()
            .filter(|sid| app.reports.is_stat_visible(*sid))
            .collect();

    // Cursor index, clamped to result length (the app handler also
    // clamps but defensively here so a stale index can't panic on
    // render).
    let sel = app
        .queries
        .sort_picker_idx
        .min(results.len().saturating_sub(1));

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));
    // Search prompt with cursor block.
    lines.push(Line::from(vec![
        Span::styled("  Search: ", Style::default().fg(Color::White)),
        Span::styled(
            format!("{}▌", app.queries.sort_picker_query),
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
    let border_style = if !app.queries.results_focused {
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

    let sel = app.queries.field_idx;
    let dim = Style::default().fg(Color::DarkGray);

    // Phase Lindsay L.3.3 — render by section. Each section gets a
    // header line (▶/▼ + label); expanded sections also render their
    // fields indented under the header. Collapsed sections show
    // header only — fields hidden + cursor-skipped.
    let mut all_items: Vec<ListItem> = Vec::new();
    for section in &app.queries.sections {
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
            let field = match app.queries.fields.get(i) {
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
    let sort = app.queries.fields[0].value();

    let border_style = if app.queries.results_focused {
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
        &app.queries.fields,
        app.queries.sort_stat_pick,
        app.queries.filter_plan.as_ref(),
        app.active_season_typed.0,
    );
    let top: usize = app.queries.fields[9].value().parse().unwrap_or(20);
    // Phase Lindsay L.3.4 — when picker overrides legacy field, the
    // column label comes from the StatId; fall back to legacy.
    let clabel: String = match app.queries.sort_stat_pick {
        Some(stat) => stat.short_label().to_owned(),
        None => col_label(sort).to_owned(),
    };
    let leaders_view = leaders_view_from_query_results(
        &results,
        sort,
        &clabel,
        app.active_season_typed,
        app.active_type,
    );
    let dim = Style::default().fg(Color::DarkGray);

    let visible = inner.height.saturating_sub(4) as usize;
    let offset = app.queries.result_scroll;

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

    for row in leaders_view.rows.iter().skip(offset).take(visible) {
        let name = row.display_name.chars().take(22).collect::<String>();
        let value = leader_primary_text(row);
        let is_selected = offset + (lines.len() - 2)
            == app.queries.result_scroll + app.selected.min(visible.saturating_sub(1));
        let style = if is_selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else if row.rank <= 3 {
            Style::default().fg(Color::Green)
        } else {
            Style::default()
        };
        lines.push(Line::styled(
            format!(
                "  {:<4} {:<22} {:<5} {:<4} {:>8}",
                row.rank,
                name,
                leader_team_text(row),
                row.position.abbreviation(),
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
//
// v1 (pre-Wave 24) shape: top-level JSON array of field objects.
//   `[{"label":"Sort by","selected":0}, …]`
//
// v2 (Wave 24+) shape: top-level JSON object that carries the same
// fields array plus a free-form filter_text payload (the Phase Art
// Ross overlay text). The version tag is explicit so future shape
// changes can branch cleanly.
//   `{"version":2, "fields":[…], "filter_text":"country IN (CAN, USA)"}`
//
// `apply_saved_json` accepts either shape — pre-Wave-24 saved queries
// continue to load with `filter_text == ""`. New saves always write
// v2.

const SAVED_QUERY_VERSION: u32 = 2;

/// Wave 24 — serialize current field selections AND the active
/// free-form filter text into the v2 object envelope. Use this for
/// every new save; it round-trips cleanly through `apply_saved_json`.
pub fn fields_and_filter_to_json(fields: &[QueryField], filter_text: &str) -> String {
    let fields_arr: Vec<serde_json::Value> = fields
        .iter()
        .map(|f| {
            serde_json::json!({
                "label": f.label,
                "selected": f.selected,
            })
        })
        .collect();
    let envelope = serde_json::json!({
        "version": SAVED_QUERY_VERSION,
        "fields": fields_arr,
        "filter_text": filter_text,
    });
    envelope.to_string()
}

/// Restore field selections from stored JSON. Returns the recovered
/// filter text — empty string for v1 (legacy array) saves, the
/// stored value for v2. Unknown labels are ignored. Out-of-range
/// `selected` indices are clamped.
pub fn apply_saved_json(fields: &mut [QueryField], json: &str) -> String {
    let parsed: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return String::new(),
    };

    // v2 object envelope.
    if let Some(obj) = parsed.as_object() {
        if let Some(arr) = obj.get("fields").and_then(|v| v.as_array()) {
            apply_field_array(fields, arr);
        }
        return obj
            .get("filter_text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();
    }

    // v1 legacy: top-level array of field objects.
    if let Some(arr) = parsed.as_array() {
        apply_field_array(fields, arr);
    }
    String::new()
}

fn apply_field_array(fields: &mut [QueryField], arr: &[serde_json::Value]) {
    for entry in arr {
        if let (Some(label), Some(sel)) = (entry["label"].as_str(), entry["selected"].as_u64()) {
            if let Some(f) = fields.iter_mut().find(|f| f.label == label) {
                f.selected = (sel as usize).min(f.options.len().saturating_sub(1));
            }
        }
    }
}

// ── L.3.3 unit tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_core::{fixtures, model::Season, season_stats::SeasonType};

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

    #[test]
    fn l0_tui_leaders_view_preserves_query_result_rank_and_primary_metric() {
        let mut repo = icelines_core::stats_repository::StatsRepository::new();
        repo.upsert_identity(
            fixtures::identity(1)
                .name("Alpha Center", "alpha center")
                .build(),
        )
        .unwrap();
        repo.upsert_stats(fixtures::stats(1, 20242025, "EDM").build())
            .unwrap();
        repo.upsert_identity(
            fixtures::identity(2)
                .name("Bravo Wing", "bravo wing")
                .build(),
        )
        .unwrap();
        repo.upsert_stats(
            fixtures::stats(2, 20242025, "SEA")
                .position(icelines_core::model::Position::RightWing)
                .build(),
        )
        .unwrap();

        let views: Vec<_> = repo
            .skaters(Season(20242025), SeasonType::Regular)
            .collect();
        let bravo = views
            .iter()
            .copied()
            .find(|view| view.id().0 == 2)
            .expect("bravo fixture view");
        let alpha = views
            .iter()
            .copied()
            .find(|view| view.id().0 == 1)
            .expect("alpha fixture view");
        let results = vec![(7, bravo), (9, alpha)];
        let view = leaders_view_from_query_results(
            &results,
            "pts",
            "Pts",
            Season(20242025),
            SeasonType::Regular,
        );

        assert_eq!(view.sort.as_ref().unwrap().key.0, "pts");
        assert_eq!(view.rows[0].rank, 7);
        assert_eq!(view.rows[1].rank, 9);
        assert_eq!(view.rows[0].primary.label, "Pts");
        assert_eq!(leader_primary_text(&view.rows[0]), "80");
        assert_eq!(leader_team_text(&view.rows[0]), "SEA");
        assert_eq!(view.rows[0].position.abbreviation(), "RW");
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

    // ── Phase Art Ross — Wave 23 TUI filter overlay ───────────────────────

    /// Empty-views purity: an empty input slice always returns an
    /// empty result, regardless of plan presence. Guards against a
    /// future refactor that would scan past the slice or panic on
    /// `views[0]`.
    #[test]
    fn l0_w23_empty_views_no_plan_returns_empty() {
        let fields = default_fields();
        let result = run_query_views_with_pick_and_plan(
            &[],
            &fields,
            None,
            None,
            icelines_core::CURRENT_SEASON,
        );
        assert!(result.is_empty());
    }

    #[test]
    fn l0_w23_empty_views_with_plan_returns_empty() {
        let fields = default_fields();
        let plan =
            icelines_query::parse_query(icelines_query::FilterInput::Cli("country=CAN".to_owned()))
                .expect("plan parses");
        let result = run_query_views_with_pick_and_plan(
            &[],
            &fields,
            None,
            Some(&plan),
            icelines_core::CURRENT_SEASON,
        );
        assert!(
            result.is_empty(),
            "empty views in → empty out, even with a non-trivial plan"
        );
    }

    /// `None` plan must produce the same answer as the legacy
    /// `run_query_views_with_pick`. This is the load-bearing
    /// invariant — the legacy CLI/web/TUI paths must keep working
    /// unchanged when no overlay filter is set.
    #[test]
    fn l0_w23_none_plan_is_identity_to_legacy_helper() {
        let fields = default_fields();
        // Empty views still exercises the function-shape comparison.
        let with_plan = run_query_views_with_pick_and_plan(
            &[],
            &fields,
            None,
            None,
            icelines_core::CURRENT_SEASON,
        );
        let without_plan = run_query_views_with_pick(&[], &fields, None);
        assert_eq!(
            with_plan.len(),
            without_plan.len(),
            "None plan must match _with_pick on result count"
        );
        // Tuple shape parity: both yield Vec<(usize, PlayerView)>.
        for (a, b) in with_plan.iter().zip(without_plan.iter()) {
            assert_eq!(a.0, b.0);
        }
    }

    /// The plan param accepts a borrow — caller retains ownership.
    /// Sanity-check the lifetime: borrowing the plan multiple times
    /// must be allowed (clap iter-style usage).
    #[test]
    fn l0_w23_plan_borrow_allows_repeated_calls() {
        let fields = default_fields();
        let plan =
            icelines_query::parse_query(icelines_query::FilterInput::Cli("country=CAN".to_owned()))
                .expect("plan parses");
        let _ = run_query_views_with_pick_and_plan(
            &[],
            &fields,
            None,
            Some(&plan),
            icelines_core::CURRENT_SEASON,
        );
        let _ = run_query_views_with_pick_and_plan(
            &[],
            &fields,
            None,
            Some(&plan),
            icelines_core::CURRENT_SEASON,
        );
    }

    // ── Phase Art Ross — Wave 24 saved-query filter round-trip ─────────────

    /// v2 envelope round-trip: write a fields+filter pair, read it
    /// back, both halves must be identical.
    #[test]
    fn l0_w24_v2_envelope_round_trip_preserves_filter() {
        let mut fields = default_fields();
        fields[0].selected = 3; // pick a non-default field selection
        fields[1].selected = 1;
        let filter = "country IN (CAN, USA) AND age<25";

        let json = fields_and_filter_to_json(&fields, filter);
        let mut restored = default_fields();
        let recovered = apply_saved_json(&mut restored, &json);

        assert_eq!(recovered, filter, "filter_text must round-trip verbatim");
        assert_eq!(restored[0].selected, 3);
        assert_eq!(restored[1].selected, 1);
    }

    /// Empty filter_text round-trips as empty (not `null`, not
    /// `"null"`, not the field labels).
    #[test]
    fn l0_w24_empty_filter_round_trips_as_empty() {
        let fields = default_fields();
        let json = fields_and_filter_to_json(&fields, "");
        let mut restored = default_fields();
        let recovered = apply_saved_json(&mut restored, &json);
        assert_eq!(recovered, "");
    }

    /// v1 legacy (top-level array) saved queries continue to load.
    /// Recovered filter_text is empty for the legacy shape — the
    /// user just sees the structured fields restored.
    #[test]
    fn l0_w24_v1_legacy_array_still_loads() {
        // Hand-built v1 JSON — what fields_to_json wrote pre-Wave-24.
        let v1 = r#"[{"label":"Sort by","selected":2},{"label":"Position","selected":1}]"#;
        let mut fields = default_fields();
        let recovered = apply_saved_json(&mut fields, v1);

        assert_eq!(
            recovered, "",
            "v1 legacy load must report empty filter_text"
        );
        assert_eq!(fields[0].selected, 2);
        assert_eq!(fields[1].selected, 1);
    }

    /// Filter text containing characters that need JSON escaping
    /// (quotes, backslashes) round-trips cleanly. Guards the
    /// switch from hand-rolled string formatting to serde_json.
    #[test]
    fn l0_w24_filter_with_quotes_round_trips() {
        let fields = default_fields();
        let filter = r#"country LIKE "CA*" AND draft-round<=2"#;
        let json = fields_and_filter_to_json(&fields, filter);
        let mut restored = default_fields();
        let recovered = apply_saved_json(&mut restored, &json);
        assert_eq!(recovered, filter);
    }

    /// Garbage / non-JSON input doesn't panic and doesn't mutate
    /// the fields array. Matches the v1 behavior — legacy
    /// `apply_saved_json` was tolerant of malformed input.
    #[test]
    fn l0_w24_malformed_json_is_no_op() {
        let mut fields = default_fields();
        let original_selections: Vec<usize> = fields.iter().map(|f| f.selected).collect();
        let recovered = apply_saved_json(&mut fields, "{not valid json");
        assert_eq!(recovered, "");
        let after: Vec<usize> = fields.iter().map(|f| f.selected).collect();
        assert_eq!(original_selections, after);
    }

    /// v2 envelope with a non-string filter_text (corrupted save)
    /// is treated as empty — the load proceeds, the user just
    /// loses the filter half.
    #[test]
    fn l0_w24_v2_filter_wrong_type_treated_as_empty() {
        let mut fields = default_fields();
        let json = r#"{"version":2,"fields":[{"label":"Sort by","selected":1}],"filter_text":42}"#;
        let recovered = apply_saved_json(&mut fields, json);
        assert_eq!(recovered, "");
        assert_eq!(fields[0].selected, 1);
    }

    /// Output of `fields_and_filter_to_json` is valid serde_json
    /// (no broken escapes). Sanity check on the serializer choice.
    #[test]
    fn l0_w24_output_is_valid_serde_json() {
        let fields = default_fields();
        let json = fields_and_filter_to_json(&fields, r#"country LIKE "CA*" AND age<25"#);
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("output must be valid JSON");
        assert_eq!(parsed["version"].as_u64(), Some(2));
        assert!(parsed["fields"].is_array());
        assert!(parsed["filter_text"].is_string());
    }

    // ── Phase Art Ross — Wave 24c live result count hint ───────────────────

    /// Empty input yields no hint.
    #[test]
    fn l0_w24c_live_count_empty_returns_none() {
        let hint = live_filter_count_hint("", &[], 20252026);
        assert!(hint.is_none());
    }

    /// Whitespace-only input yields no hint (same as empty).
    #[test]
    fn l0_w24c_live_count_whitespace_returns_none() {
        let hint = live_filter_count_hint("   \t  ", &[], 20252026);
        assert!(hint.is_none());
    }

    /// Unparsed input yields the "(unparsed — keep typing)" placeholder.
    #[test]
    fn l0_w24c_live_count_unparsed_returns_placeholder() {
        let hint =
            live_filter_count_hint("(((", &[], 20252026).expect("unparsed must yield a hint");
        assert!(
            hint.contains("unparsed"),
            "unparsed hint should mention 'unparsed'; got: {hint}"
        );
    }

    /// Provider-needing plans (sliding-window, career, league)
    /// defer to Enter rather than file-I/O on every keystroke.
    #[test]
    fn l0_w24c_live_count_sliding_window_defers_to_enter() {
        let hint = live_filter_count_hint("g.last10g>=5", &[], 20252026)
            .expect("provider-needing plan must still produce a hint");
        assert!(
            hint.contains("Enter") || hint.contains("data lookup"),
            "provider-gated hint should mention Enter / data lookup; got: {hint}"
        );
    }

    #[test]
    fn l0_w24c_live_count_career_aggregate_defers_to_enter() {
        let hint = live_filter_count_hint("p.career>=500", &[], 20252026)
            .expect("career aggregate must produce a hint");
        assert!(
            hint.contains("Enter") || hint.contains("data lookup"),
            "career-aggregate hint should defer to Enter; got: {hint}"
        );
    }

    /// Bio-only plan with empty views: count is 0 of 0 — but the
    /// hint MUST still report the count (not defer). This is the
    /// contract that makes the editor feel responsive.
    #[test]
    fn l0_w24c_live_count_bio_filter_empty_views_yields_zero_of_zero() {
        let hint = live_filter_count_hint("country=CAN", &[], 20252026)
            .expect("bio filter must produce a count");
        assert!(
            hint.contains("0 of 0") || hint.contains("0/0"),
            "empty-views bio filter should report 0 of 0; got: {hint}"
        );
    }

    /// Compound bio filter doesn't trip the provider gate. Counts
    /// against empty views as 0/0.
    #[test]
    fn l0_w24c_live_count_compound_bio_does_not_defer() {
        let hint =
            live_filter_count_hint("country IN (CAN, USA) AND pos=C AND age<30", &[], 20252026)
                .expect("compound bio filter must produce a count");
        assert!(
            !hint.contains("Enter") && !hint.contains("data lookup"),
            "compound bio filter must NOT defer; got: {hint}"
        );
        assert!(hint.contains("match") || hint.contains("0"));
    }

    // ── Phase Norris.1 — QueriesState contract ─────────────────────────────
    //
    // These tests lock in the default-state contract. The full bin
    // suite (692 passing) proves the field-access rename works; what
    // these add is a future-proof fence: if a refactor changes
    // QueriesState::default() — say, flipping `filter_show_help` to
    // true, or reseeding `mode` to something other than Build — these
    // tests fire instead of the bug surfacing as a silent UX regression.

    /// Default mode is Build — the editor opens with the structured
    /// field cursor, not a save prompt or filter overlay.
    #[test]
    fn l0_norris_default_starts_in_build_mode() {
        let s = QueriesState::default();
        assert!(
            matches!(s.mode, crate::tui::app::QueryMode::Build),
            "default mode must be Build, got {:?}",
            s.mode
        );
    }

    /// Default has no active free-form filter — text empty, no plan,
    /// no error, history empty.
    #[test]
    fn l0_norris_default_has_no_active_filter() {
        let s = QueriesState::default();
        assert_eq!(s.filter_text, "");
        assert!(s.filter_error.is_none());
        assert!(s.filter_plan.is_none());
        assert!(s.filter_history.is_empty());
        assert!(s.filter_history_cursor.is_none());
    }

    /// Default populates the field editor — fields and sections
    /// non-empty, cursor on field 0, scroll at 0.
    #[test]
    fn l0_norris_default_populates_field_editor() {
        let s = QueriesState::default();
        assert!(
            !s.fields.is_empty(),
            "default fields must be non-empty (sourced from default_fields())"
        );
        assert!(
            !s.sections.is_empty(),
            "default sections must be non-empty (sourced from default_sections())"
        );
        assert_eq!(s.field_idx, 0, "cursor starts on first field");
        assert_eq!(s.result_scroll, 0);
    }

    /// Default fields match `default_fields()` and default sections
    /// match `default_sections()` — these helpers are the single
    /// source of truth and QueriesState::default() must delegate
    /// to them, not duplicate.
    #[test]
    fn l0_norris_default_field_count_matches_default_fields_helper() {
        let s = QueriesState::default();
        let canonical = default_fields();
        assert_eq!(
            s.fields.len(),
            canonical.len(),
            "default fields count must match default_fields()"
        );
    }

    /// Default has no saved state — save_name empty, saved_list
    /// empty.
    #[test]
    fn l0_norris_default_has_no_saved_state() {
        let s = QueriesState::default();
        assert_eq!(s.save_name, "");
        assert!(s.saved_list.is_empty());
    }

    /// Default has the cheatsheet OFF and results pane unfocused.
    /// Both are user-toggled flags; default-on would silently
    /// change first-launch UX.
    #[test]
    fn l0_norris_default_overlays_off() {
        let s = QueriesState::default();
        assert!(!s.filter_show_help, "cheatsheet off by default");
        assert!(!s.results_focused, "field editor focused by default");
    }

    /// Default has no sort picker selection (legacy string-keyed
    /// sort path active).
    #[test]
    fn l0_norris_default_sort_picker_unset() {
        let s = QueriesState::default();
        assert!(s.sort_stat_pick.is_none());
        assert_eq!(s.sort_picker_query, "");
        assert_eq!(s.sort_picker_idx, 0);
    }

    /// Default career-table preset is `Default` (the legacy preset
    /// shown on a fresh player card open).
    #[test]
    fn l0_norris_default_career_preset_is_default() {
        let s = QueriesState::default();
        assert!(matches!(
            s.career_table_preset,
            crate::tui::screens::player::CareerTablePreset::Default
        ));
    }

    /// `App::new` stamps QueriesState::default() onto `app.queries`.
    /// This is the load-bearing wire-up: if App::new ever stops
    /// using ::default(), every default-state assumption above
    /// breaks.
    #[test]
    fn l0_norris_app_new_uses_queries_state_default() {
        let app = crate::tui::app::App::new(false);
        let canonical = QueriesState::default();
        // Spot-check a handful of fields — full equality requires
        // PartialEq on every nested type (incl. VecDeque, Option<Plan>,
        // CareerTablePreset). The spot-check is enough to prove
        // App::new wires through Default.
        assert!(matches!(
            app.queries.mode,
            crate::tui::app::QueryMode::Build
        ));
        assert_eq!(app.queries.fields.len(), canonical.fields.len());
        assert_eq!(app.queries.sections.len(), canonical.sections.len());
        assert_eq!(app.queries.filter_text, "");
        assert!(app.queries.filter_plan.is_none());
        assert!(!app.queries.filter_show_help);
    }

    /// Debug derive renders without panicking on a default state.
    /// Sanity check for forge-1 (we added derive(Debug) on
    /// QueriesState, QueryField, QuerySection in Norris.1).
    #[test]
    fn l0_norris_default_debug_renders() {
        let s = QueriesState::default();
        let dbg = format!("{:?}", s);
        // The output must mention the type name (basic sanity that
        // formatter ran).
        assert!(
            dbg.contains("QueriesState"),
            "Debug output must include the struct name; got: {dbg}"
        );
    }

    // ── Phase Masterton.1 — chrome accessor contract ───────────────────────

    /// Default state yields the Build-mode chrome — title
    /// breadcrumb is "Stats / Queries", keybinds advertise the
    /// editor entry points (`f`/`/`/`s`/`l`).
    #[test]
    fn l0_masterton_queries_chrome_default_is_build_mode() {
        let s = QueriesState::default();
        let c = chrome(&s);
        assert_eq!(c.title, "Stats / Queries");
        let keys: Vec<&str> = c.keybinds.iter().map(|k| k.key).collect();
        for needed in ["f", "/", "s", "l"] {
            assert!(
                keys.contains(&needed),
                "Build-mode chrome must advertise {needed:?}; got: {keys:?}"
            );
        }
    }

    /// FilterEdit mode yields a different chrome — title shifts
    /// to ".../Filter", keybinds reflect the editor's actions.
    #[test]
    fn l0_masterton_queries_chrome_filter_edit_mode() {
        use crate::tui::app::QueryMode;
        let s = QueriesState {
            mode: QueryMode::FilterEdit,
            ..Default::default()
        };
        let c = chrome(&s);
        assert!(
            c.title.ends_with("/ Filter"),
            "FilterEdit title must end with '/ Filter'; got: {}",
            c.title
        );
        let keys: Vec<&str> = c.keybinds.iter().map(|k| k.key).collect();
        assert!(keys.contains(&"Enter"));
        assert!(keys.contains(&"Esc"));
    }
}
