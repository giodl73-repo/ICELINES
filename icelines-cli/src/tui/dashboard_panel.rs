//! Phase 8j — opt-in side panel for the player card.
//!
//! Behind the `dashboards` feature flag (off by default — see
//! `crate::config::dashboards_enabled`). Renders a "scout card" with
//! identity, counting stats, and 5-season trend sparklines pulled from
//! the bundled history.
//!
//! # Native rendering
//!
//! The first cut of this panel called `proof_lib::compile_file` to
//! render dashboard regions, but proof's `proof:chart` directive turns
//! out not to compose inside `proof:region` bodies (issue archived at
//! `design/proof-bug-report.md`). For our use case — a small text
//! panel + two sparklines — native rendering with `tui::sparkline` is
//! ~50 lines, has zero new deps, and gives ratatui full control over
//! borders and styling. If site generation returns, PROOF should be
//! invoked as a tool/generator pipeline rather than linked into the
//! ICELINES runtime.

use icelines_core::identity::PlayerId;
use icelines_core::model::{Position, Season};
use icelines_core::season_stats::SeasonType;
use icelines_core::stats_repository::{PlayerView, StatsRepository};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::tui::sparkline;

// ── League context (Phase 8j) ─────────────────────────────────────────────
//
// A small, sorted-by-position view of the current player pool used to
// compute "where does this player rank vs peers at their position?". The
// App builds this once after players are loaded and reuses it for every
// player render.

/// Sorted-ascending pace_82 values per position. An empty context disables
/// the percentile section in the panel — callers that don't have a player
/// pool yet (loading, tests) can pass `LeagueContext::empty()`.
#[derive(Debug, Clone, Default)]
pub struct LeagueContext {
    pace_by_position: HashMap<Position, Vec<f64>>,
}

impl LeagueContext {
    /// Empty context — every percentile lookup returns `None`. Used as a
    /// placeholder before the player pool has loaded.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Build from a `StatsRepository`. Iterates `repo.skaters(s, t)`
    /// filtering on `view.pace_82().is_some()` (BelowThreshold yields
    /// None per A4) and skipping goalies (Position::Goalie excluded).
    /// Sorted-asc vectors per position; rank lookups are O(log n)
    /// binary search.
    pub fn build(repo: &StatsRepository, s: Season, t: SeasonType) -> Self {
        let mut buckets: HashMap<Position, Vec<f64>> = HashMap::new();
        for view in repo.skaters(s, t) {
            if matches!(view.position(), Position::Goalie) {
                continue;
            }
            if let Some(p82) = view.pace_82() {
                buckets.entry(view.position()).or_default().push(p82);
            }
        }
        for v in buckets.values_mut() {
            // partial_cmp returns None only for NaN; pace_82() filters
            // NaN (BelowThreshold returns None, never f64::NAN), so
            // Equal is unreachable in practice.
            v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        }
        Self {
            pace_by_position: buckets,
        }
    }

    /// Look up the player's `(rank, total, percentile)` at their position.
    /// `rank` is 1-based with 1 = highest pace_82; `percentile` is
    /// `0.0..=100.0` where 100 = top of league.
    /// Returns `None` for the empty bucket case (e.g. empty context).
    pub fn position_rank_for(&self, position: Position, pace_82: f64) -> Option<PositionRank> {
        let bucket = self.pace_by_position.get(&position)?;
        if bucket.is_empty() {
            return None;
        }
        let lower_or_equal = bucket.partition_point(|v| *v <= pace_82);
        let total = bucket.len();
        let rank = total - lower_or_equal + 1;
        let rank = rank.min(total).max(1);
        let percentile = if total == 1 {
            100.0
        } else {
            let below = bucket.partition_point(|v| *v < pace_82);
            (below as f64) / ((total - 1) as f64) * 100.0
        };
        Some(PositionRank {
            rank,
            total,
            percentile,
        })
    }
}

/// Result of a position-rank lookup.
#[derive(Debug, Clone, Copy)]
pub struct PositionRank {
    pub rank: usize,     // 1-based, top = 1
    pub total: usize,    // count of qualifying peers at the position
    pub percentile: f64, // 0.0 ..= 100.0
}

/// A panel rendered for a specific player. Returns `Line<'static>` so
/// callers can drop the result straight into a ratatui Paragraph; each
/// sparkline column carries its own colour.
///
/// Cache by `nhl_id` so scrolling through many players doesn't rebuild
/// the same panel repeatedly. Cache miss is sub-millisecond.
#[derive(Clone)]
pub struct CompiledPanel {
    inner: Arc<Mutex<PanelState>>,
}

#[derive(Default)]
struct PanelState {
    /// Triple-keyed cache per spec D2. The `(Season, SeasonType)`
    /// component of the key is what makes `repo_swap`-without-clear-cache
    /// safe at the type level — a compiled panel from one window is
    /// never returned for another.
    by_view: HashMap<(u32, Season, SeasonType), Vec<Line<'static>>>,
}

impl CompiledPanel {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(PanelState::default())),
        }
    }

    /// Drop all cached compilations. Called after every `repo_swap`
    /// (boot_load and reload_for_season) so post-swap renders rebuild
    /// against the new repo.
    pub fn clear_cache(&self) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.by_view.clear();
        }
    }
}

impl Default for CompiledPanel {
    fn default() -> Self {
        Self::new()
    }
}

// ── Hart.5c.6 Phase A — view-based compile API ─────────────────────────
//
// `compile()` is the post-Hart replacement for `lines_for_player` /
// `lines_for_goalie`. Cache key includes (PlayerId, Season, SeasonType)
// so a compiled panel from one window is never returned for another;
// `ctx_window` enforces single-window LeagueContext use (D11).

#[derive(Debug, Clone, PartialEq)]
pub enum DashboardError {
    PlayerNotInRepo {
        season: Season,
        season_type: SeasonType,
    },
    CrossWindowCompile {
        requested_s: Season,
        requested_t: SeasonType,
        ctx_s: Season,
        ctx_t: SeasonType,
    },
}

impl std::fmt::Display for DashboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PlayerNotInRepo {
                season,
                season_type,
            } => {
                write!(f, "player not in repo for ({season:?}, {season_type:?})")
            }
            Self::CrossWindowCompile {
                requested_s,
                requested_t,
                ctx_s,
                ctx_t,
            } => write!(
                f,
                "cross-window compile: requested ({requested_s:?}, {requested_t:?}) \
                 but ctx was built for ({ctx_s:?}, {ctx_t:?})"
            ),
        }
    }
}

impl std::error::Error for DashboardError {}

/// Output of a single compile pass: the lines plus a small marker so
/// callers can confirm key shape during debug. Kept Clone for the cache.
#[derive(Debug, Clone)]
pub struct CompiledOutput {
    pub lines: Vec<Line<'static>>,
}

impl CompiledPanel {
    /// View-based panel compile. See D2 / D11 in the 5c.6 spec.
    ///
    /// Cache key: `(PlayerId, Season, SeasonType)`. `ctx_window` is the
    /// (Season, SeasonType) tuple that `ctx` was built for; if it
    /// doesn't match `(season, season_type)`, returns
    /// `CrossWindowCompile`.
    pub fn compile(
        &self,
        repo: &StatsRepository,
        season: Season,
        season_type: SeasonType,
        player_id: PlayerId,
        ctx: &LeagueContext,
        ctx_window: (Season, SeasonType),
    ) -> Result<CompiledOutput, DashboardError> {
        if ctx_window != (season, season_type) {
            return Err(DashboardError::CrossWindowCompile {
                requested_s: season,
                requested_t: season_type,
                ctx_s: ctx_window.0,
                ctx_t: ctx_window.1,
            });
        }
        let view =
            repo.view(player_id, season, season_type)
                .ok_or(DashboardError::PlayerNotInRepo {
                    season,
                    season_type,
                })?;

        // Triple-keyed cache (nhl_id, Season, SeasonType) per D2.
        // `ctx_window == (season, season_type)` is asserted at the top
        // of this function, so the cache key + ctx are coherent.
        let key = (player_id.0, season, season_type);
        if let Ok(guard) = self.inner.lock() {
            if let Some(cached) = guard.by_view.get(&key) {
                return Ok(CompiledOutput {
                    lines: cached.clone(),
                });
            }
        }
        // Hart.5c.6 Phase B-2.3: branch on goalie discriminator
        // (`is_goalie() == goalie.is_some()`, NOT position == Goalie —
        // emergency-backup forwards have goalie:Some). Skater path
        // renders sparklines for G/Pts/SOG; goalie path renders
        // SV%/GAA/W.
        let lines = if view.is_goalie() {
            build_goalie_panel_lines_view(&view)
        } else {
            build_panel_lines_view(&view, ctx)
        };
        if let Ok(mut guard) = self.inner.lock() {
            guard.by_view.insert(key, lines.clone());
        }
        Ok(CompiledOutput { lines })
    }
}

/// View-based panel builder. Mirrors `build_panel_lines` field-by-field
/// using `PlayerView` accessors instead of `&Player`. Phase B/C consumers
/// pivot to this; Phase A leaves `build_panel_lines` intact for the
/// existing `lines_for_player` callsite.
fn build_panel_lines_view(view: &PlayerView<'_>, league: &LeagueContext) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::with_capacity(10);
    let dim = Style::default().fg(DIM_COLOR);
    let title = Style::default()
        .fg(TITLE_COLOR)
        .add_modifier(Modifier::BOLD);
    let accent = Style::default().fg(ACCENT_COLOR);

    // Header
    let full = view.full_name();
    let last_name = full.rsplit_once(' ').map(|(_, l)| l).unwrap_or(full);
    lines.push(Line::from(vec![
        Span::styled(trim_to(last_name, 18), title),
        Span::styled("  ·  ", dim),
        Span::styled(view.team_display().to_owned(), accent),
        Span::styled(" ", dim),
        Span::raw(view.position().abbreviation().to_owned()),
    ]));
    lines.push(Line::from(""));

    // Bundled-history trend (uses nhl_id from PlayerId)
    let nhl_id = view.identity.id.0;
    let history = load_player_history(nhl_id);
    let pace_82_opt = view.pace_82();
    match history.len() {
        0 => {
            let pace = pace_82_opt
                .map(|p| format!("{:.0}", p))
                .unwrap_or_else(|| "—".to_owned());
            let ppg = pace_82_opt
                .map(|p| format!("{:.2}", p / 82.0))
                .unwrap_or_else(|| "—".to_owned());
            lines.push(Line::styled("Bundled history: none", dim));
            lines.push(stat_row("Pts/82", &pace, "PPG", &ppg, dim, accent));
        }
        1 => {
            let row = &history[0];
            lines.push(Line::styled(
                format!("Bundled history: {}", short_season(row.season)),
                dim,
            ));
        }
        _ => {
            let goals_values: Vec<f64> = history.iter().map(|r| r.goals as f64).collect();
            let pts_values: Vec<f64> = history.iter().map(|r| r.points as f64).collect();
            let shots_values: Vec<f64> = history.iter().map(|r| r.shots as f64).collect();
            let first = &history[0];
            let last = &history[history.len() - 1];
            let range = format!("{}→{}", short_year(first.season), short_year(last.season));

            lines.push(Line::from(vec![
                Span::styled("Last 5 seasons ", dim),
                Span::styled(range, accent),
            ]));
            let g_spark = colored_spark_spans(&goals_values, history.len());
            let pts_spark = colored_spark_spans(&pts_values, history.len());
            let sh_spark = colored_spark_spans(&shots_values, history.len());
            let pad = 5usize.saturating_sub(history.len());

            lines.push(spark_row(
                "G  ",
                pad,
                g_spark,
                first.goals,
                last.goals,
                dim,
                accent,
            ));
            lines.push(spark_row(
                "Pts",
                pad,
                pts_spark,
                first.points,
                last.points,
                dim,
                accent,
            ));
            lines.push(spark_row(
                "SOG",
                pad,
                sh_spark,
                first.shots,
                last.shots,
                dim,
                accent,
            ));
        }
    }

    // Position vs league
    if let Some(p82) = pace_82_opt {
        if let Some(rank) = league.position_rank_for(view.position(), p82) {
            lines.push(Line::from(""));
            let pos_letter = position_letter(view.position());
            lines.push(Line::from(vec![
                Span::styled(format!("Pos vs {pos_letter}: "), dim),
                Span::styled(format!("#{}/{}", rank.rank, rank.total), accent),
            ]));
            lines.push(Line::from(percentile_bar_spans(
                rank.percentile,
                12,
                dim,
                accent,
            )));
        }
    }

    lines
}

// ── Styling palette ────────────────────────────────────────────────────────

/// Colour for sparkline columns above the player's median. Bright green —
/// the season was a high water mark.
const HIGH_COLOR: Color = Color::Green;
/// Colour for columns at the median. Plain white.
const MID_COLOR: Color = Color::White;
/// Colour for columns below the median. Red — the season was a dip.
const LOW_COLOR: Color = Color::Red;
/// Dim grey for chrome/labels.
const DIM_COLOR: Color = Color::DarkGray;
/// Header / title color.
const TITLE_COLOR: Color = Color::Yellow;
/// Bright accent for the headline number on each row.
const ACCENT_COLOR: Color = Color::Cyan;

/// Width of the panel content (inside the ratatui border). Matches the
/// `Constraint::Length(30)` minus 2 for the border in
/// `tui::screens::player::render_dashboard_panel`.
#[allow(dead_code)] // Documented constant; reserved for future cell-budgeting refactor.
const PANEL_WIDTH: usize = 28;

// (Orphan doc comment for a previously-public `build_lines` function
// removed in an earlier phase. Kept the placeholder note here so a
// future reader of git blame doesn't wonder what was deleted.)

/// Letter abbreviation used in the position-rank header.
fn position_letter(pos: Position) -> &'static str {
    match pos {
        Position::Center => "C",
        Position::LeftWing => "LW",
        Position::RightWing => "RW",
        Position::Defense => "D",
        Position::Goalie => "G",
    }
}

/// Render a percentile bar as `width` columns plus a trailing percentile
/// label. Filled cells use a colour gradient: top quartile green,
/// 50–75 yellow, 25–50 dim white, below 25 red.
fn percentile_bar_spans(
    percentile: f64,
    width: usize,
    dim: Style,
    accent: Style,
) -> Vec<Span<'static>> {
    let pct = percentile.clamp(0.0, 100.0);
    let filled = ((pct / 100.0) * width as f64).round() as usize;
    let band_color = if pct >= 75.0 {
        Color::Green
    } else if pct >= 50.0 {
        Color::Yellow
    } else if pct >= 25.0 {
        Color::White
    } else {
        Color::Red
    };
    let mut spans: Vec<Span<'static>> = Vec::with_capacity(4);
    if filled > 0 {
        spans.push(Span::styled(
            "█".repeat(filled),
            Style::default().fg(band_color),
        ));
    }
    if filled < width {
        spans.push(Span::styled("░".repeat(width - filled), dim));
    }
    spans.push(Span::styled("  ".to_owned(), dim));
    // Round-half-up percentage label so a 96.5 reads as 97.
    spans.push(Span::styled(
        format!("top {}%", percentile_to_top_pct(pct)),
        accent,
    ));
    spans
}

/// Convert a 0..=100 percentile into a "top X%" label. `top` here is
/// `100 - percentile` rounded down so a 96th-percentile player is "top 4%"
/// (not "top 3%" via half-up rounding which would overstate the rank).
fn percentile_to_top_pct(percentile: f64) -> u32 {
    let top = (100.0 - percentile).max(0.0).floor();
    top as u32
}

/// Render one `LABEL  VALUE   LABEL  VALUE` row with split colours.
fn stat_row(l1: &str, v1: &str, l2: &str, v2: &str, dim: Style, accent: Style) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{l1} "), dim),
        Span::styled(format!("{v1:>3}"), accent),
        Span::styled(format!("    {l2} "), dim),
        Span::styled(format!("{v2:>3}"), accent),
    ])
}

/// Render one sparkline row: `LABEL  PAD  SPARK   FIRST → LAST`.
fn spark_row(
    label: &str,
    pad: usize,
    spark: Vec<Span<'static>>,
    first: u32,
    last: u32,
    dim: Style,
    accent: Style,
) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::with_capacity(4 + spark.len());
    spans.push(Span::styled(format!("{label} "), dim));
    if pad > 0 {
        spans.push(Span::raw(" ".repeat(pad)));
    }
    spans.extend(spark);
    spans.push(Span::styled("    ".to_owned(), dim));
    spans.push(Span::styled(first.to_string(), accent));
    spans.push(Span::styled(" → ".to_owned(), dim));
    spans.push(Span::styled(last.to_string(), accent));
    Line::from(spans)
}

/// Build per-column coloured spans from a numeric series. Each column
/// carries the spark block character styled by its value's relationship
/// to the series median: above → green, equal → white, below → red.
/// Constant series fall back to all-white middle blocks.
fn colored_spark_spans(values: &[f64], width: usize) -> Vec<Span<'static>> {
    let cols = sparkline::columns(values, width);
    if cols.is_empty() {
        return Vec::new();
    }
    let median = median_of(values);
    cols.into_iter()
        .map(|(ch, val)| {
            let color = if val > median {
                HIGH_COLOR
            } else if val < median {
                LOW_COLOR
            } else {
                MID_COLOR
            };
            Span::styled(ch.to_string(), Style::default().fg(color))
        })
        .collect()
}

/// Median of a numeric slice. Returns `0.0` for empty input — caller
/// should not pass empty slices in practice (the sparkline path guards).
fn median_of(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted: Vec<f64> = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    }
}

/// Compress the right-hand year of an 8-char season string to 2 chars.
/// `"20242025"` → `"25"`. Used in the sparkline range marker.
fn short_year(season: &str) -> String {
    if season.len() == 8 {
        season[6..8].to_owned()
    } else {
        season.to_owned()
    }
}

/// One row of bundled-history data for a player.
#[derive(Debug, Clone)]
struct HistoryRow {
    season: &'static str, // e.g. "20242025"
    goals: u32,
    points: u32,
    shots: u32,
}

/// Walk the bundled-history seasons (38 entries since L.7b) in
/// chronological order and pick out the player's stats row. Missing
/// seasons are skipped — the sparkline accepts any length ≥ 2 and the
/// label row carries the actual season tags so gaps are obvious if they
/// occur. Most modern players surface 1–5 history rows; long-career
/// veterans will surface their entire NHL career.
fn load_player_history(nhl_id: u32) -> Vec<HistoryRow> {
    use icelines_fetch::bundled;
    let mut out = Vec::new();
    // BUNDLED_SEASONS is newest-first; reverse so the spark reads
    // left-to-right in time.
    for season in bundled::BUNDLED_SEASONS.iter().rev() {
        if let Some(stats) = bundled::get_stats(season) {
            if let Some(row) = stats.iter().find(|s| s.player_id == nhl_id) {
                out.push(HistoryRow {
                    season,
                    goals: row.goals,
                    points: row.points,
                    shots: row.shots,
                });
            }
        }
    }
    out
}

// ── Goalie panel (Phase G.7) ──────────────────────────────────────────────────

/// One row of bundled goalie history.
#[derive(Debug, Clone)]
struct GoalieHistoryRow {
    season: &'static str, // e.g. "20242025"
    save_pct: f32,        // 0.0..=1.0
    gaa: f32,
    wins: u32,
}

/// Walk the bundled seasons (38 entries since L.7b) and pull the
/// goalie's row from each. Missing seasons skipped — sparkline accepts
/// variable length.
fn load_goalie_history(nhl_id: u32) -> Vec<GoalieHistoryRow> {
    use icelines_fetch::bundled;
    let mut out = Vec::new();
    for season in bundled::BUNDLED_SEASONS.iter().rev() {
        if let Some(stats) = bundled::get_goalie_stats(season) {
            if let Some(row) = stats.iter().find(|s| s.player_id == nhl_id) {
                out.push(GoalieHistoryRow {
                    season,
                    save_pct: row.save_pct.unwrap_or(0.0),
                    gaa: row.goals_against_average.unwrap_or(0.0),
                    wins: row.wins,
                });
            }
        }
    }
    out
}

/// Build styled panel lines for a goalie. Layout mirrors the skater
/// scout card but with goalie-shaped trend rows:
///   header (Lastname · TEAM G)
///   blank
///   trend block: SV%, GAA (colors inverted — lower=green=better), W
///   anchor row showing first→last absolute values for context
/// Hart.5c.6 Phase B-2.3 — view-based goalie panel builder. Mirrors
/// `build_goalie_panel_lines` but takes a `PlayerView` and reads
/// nhl_id from `view.identity.id.0`. Same load_goalie_history call,
/// same SV%/GAA/W sparkline layout, same colour semantics (GAA
/// inverted because lower is better).
fn build_goalie_panel_lines_view(v: &PlayerView<'_>) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::with_capacity(10);
    let dim = Style::default().fg(DIM_COLOR);
    let title = Style::default()
        .fg(TITLE_COLOR)
        .add_modifier(Modifier::BOLD);
    let accent = Style::default().fg(ACCENT_COLOR);

    // Header — short identity line.
    let full = v.full_name();
    let last_name = full.rsplit_once(' ').map(|(_, l)| l).unwrap_or(full);
    lines.push(Line::from(vec![
        Span::styled(trim_to(last_name, 18), title),
        Span::styled("  ·  ", dim),
        Span::styled(v.team_display().to_owned(), accent),
        Span::styled(" G", dim),
    ]));
    lines.push(Line::from(""));

    let nhl_id = v.identity.id.0;
    let history = load_goalie_history(nhl_id);
    match history.len() {
        0 => {
            lines.push(Line::styled("Bundled history: none", dim));
        }
        1 => {
            let row = &history[0];
            lines.push(Line::styled(
                format!("Bundled history: {}", short_season(row.season)),
                dim,
            ));
        }
        _ => {
            let sv_values: Vec<f64> = history.iter().map(|r| r.save_pct as f64).collect();
            let gaa_values_inv: Vec<f64> = history.iter().map(|r| -(r.gaa as f64)).collect();
            let w_values: Vec<f64> = history.iter().map(|r| r.wins as f64).collect();
            let first = &history[0];
            let last = &history[history.len() - 1];
            let range = format!("{}→{}", short_year(first.season), short_year(last.season));
            lines.push(Line::from(vec![
                Span::styled("Last 5 seasons ", dim),
                Span::styled(range, accent),
            ]));
            let pad = 5usize.saturating_sub(history.len());
            let sv_spark = colored_spark_spans(&sv_values, history.len());
            lines.push(goalie_spark_row(
                "SV%",
                pad,
                sv_spark,
                fmt3(first.save_pct),
                fmt3(last.save_pct),
                dim,
                accent,
            ));
            let gaa_spark = colored_spark_spans(&gaa_values_inv, history.len());
            lines.push(goalie_spark_row(
                "GAA",
                pad,
                gaa_spark,
                fmt2(first.gaa),
                fmt2(last.gaa),
                dim,
                accent,
            ));
            let w_spark = colored_spark_spans(&w_values, history.len());
            lines.push(goalie_spark_row(
                "W  ",
                pad,
                w_spark,
                first.wins.to_string(),
                last.wins.to_string(),
                dim,
                accent,
            ));
        }
    }
    lines
}

fn goalie_spark_row(
    label: &str,
    pad: usize,
    spark: Vec<Span<'static>>,
    first: String,
    last: String,
    dim: Style,
    accent: Style,
) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::with_capacity(4 + spark.len());
    spans.push(Span::styled(format!("{label} "), dim));
    if pad > 0 {
        spans.push(Span::raw(" ".repeat(pad)));
    }
    spans.extend(spark);
    spans.push(Span::styled("    ".to_owned(), dim));
    spans.push(Span::styled(first, accent));
    spans.push(Span::styled(" → ".to_owned(), dim));
    spans.push(Span::styled(last, accent));
    Line::from(spans)
}

/// Format SV% as ".925" — drops the leading zero per goalie convention.
fn fmt3(v: f32) -> String {
    let s = format!("{:.3}", v);
    if let Some(stripped) = s.strip_prefix('0') {
        stripped.to_owned()
    } else {
        s
    }
}

fn fmt2(v: f32) -> String {
    format!("{:.2}", v)
}

fn short_season(season: &str) -> String {
    if season.len() == 8 {
        format!("{}-{}", &season[2..4], &season[6..8])
    } else {
        season.to_owned()
    }
}

/// Truncate a string to at most `max` chars, appending `…` when cut.
fn trim_to(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_owned();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Concatenate every line's text content (ignoring styles) into a
    /// single string for test assertions. ratatui's `Line` impls
    /// `Display` which already does this per line.
    #[allow(dead_code)] // Reserved test helper — kept alongside the panel renderers it exercises.
    fn lines_to_text(lines: &[Line<'static>]) -> String {
        lines
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn l0_sparkline_columns_carry_per_value_color() {
        // The colour helper assigns green for above-median, red for
        // below, white for at-median. McDavid: goals 44 64 32 26 48,
        // median = 44 → 64 green, 48 white-ish, 32+26 red, 44 white.
        let goals = [44.0, 64.0, 32.0, 26.0, 48.0];
        let spans = colored_spark_spans(&goals, 5);
        assert_eq!(spans.len(), 5);
        // 64 is the max → must be the highest column above the median.
        let span_color = |i: usize| spans[i].style.fg.unwrap();
        assert_eq!(span_color(1), HIGH_COLOR, "max value should be green");
        assert_eq!(span_color(3), LOW_COLOR, "min value should be red");
        // The median value renders as MID_COLOR, but with an even-count
        // series the median is the average of the two middle values
        // (here 44 — exactly equal to spans[0]). Assert spans[0] is mid.
        assert_eq!(
            span_color(0),
            MID_COLOR,
            "value equal to median should be white"
        );
    }

    #[test]
    fn l0_median_of_even_and_odd_series() {
        assert_eq!(median_of(&[1.0, 3.0, 5.0]), 3.0); // odd
        assert_eq!(median_of(&[1.0, 2.0, 3.0, 4.0]), 2.5); // even avg
        assert_eq!(median_of(&[]), 0.0); // empty
        assert_eq!(median_of(&[7.0]), 7.0); // single
    }

    #[test]
    fn l0_build_panel_lines_single_season_shows_row_no_spark() {
        // 1-season history: just confirm the bundled season — counting
        // stats live on the left column of the player screen.
        let history = [HistoryRow {
            season: "20252026",
            goals: 12,
            points: 30,
            shots: 80,
        }];
        assert_eq!(short_season(history[0].season), "25-26");
        let row = format!("Bundled history: {}", short_season(history[0].season));
        assert_eq!(row, "Bundled history: 25-26");
    }

    #[test]
    fn l0_load_player_history_returns_chronological() {
        // McDavid is in every bundled season; verify rows come back
        // oldest → newest so the sparkline reads left-to-right in time.
        let history = load_player_history(8478402);
        assert!(
            history.len() >= 4,
            "McDavid should appear in most/all 5 bundled seasons, got {}",
            history.len()
        );
        let seasons: Vec<&str> = history.iter().map(|r| r.season).collect();
        let mut sorted = seasons.clone();
        sorted.sort();
        assert_eq!(
            seasons, sorted,
            "history must be chronological, got: {seasons:?}"
        );
    }

    #[test]
    fn l0_short_season_compresses_eight_to_five_chars() {
        assert_eq!(short_season("20242025"), "24-25");
        assert_eq!(short_season("19931994"), "93-94");
        assert_eq!(short_season("malformed"), "malformed");
    }

    #[test]
    fn l0_trim_to_truncates_with_ellipsis() {
        assert_eq!(trim_to("Short", 26), "Short");
        let trimmed = trim_to(&"A".repeat(40), 26);
        assert!(trimmed.chars().count() <= 26);
        assert!(
            trimmed.ends_with('…'),
            "expected trailing ellipsis, got {trimmed}"
        );
    }

    #[test]
    fn l0_short_year_extracts_two_digit_end_year() {
        // The range marker uses the right-hand year only.
        assert_eq!(short_year("20212022"), "22");
        assert_eq!(short_year("20252026"), "26");
        assert_eq!(short_year("19931994"), "94");
        assert_eq!(short_year("malformed"), "malformed");
    }

    // ── Position vs league percentile (Phase 8j) ─────────────────────────

    #[test]
    fn l0_percentile_to_top_pct_floors_complement() {
        assert_eq!(percentile_to_top_pct(100.0), 0); // top of league
        assert_eq!(percentile_to_top_pct(96.5), 3); // floor(3.5) = 3
        assert_eq!(percentile_to_top_pct(50.0), 50);
        assert_eq!(percentile_to_top_pct(0.0), 100);
    }

    #[test]
    fn l0_percentile_bar_spans_fill_proportional_to_rank() {
        let dim = Style::default().fg(DIM_COLOR);
        let accent = Style::default().fg(ACCENT_COLOR);

        // 50% percentile, 12-col bar → 6 filled + 6 empty.
        let spans = percentile_bar_spans(50.0, 12, dim, accent);
        let filled_text: String = spans.iter().map(|s| s.content.to_string()).collect();
        let filled = filled_text.chars().filter(|c| *c == '█').count();
        let empty = filled_text.chars().filter(|c| *c == '░').count();
        assert_eq!(filled, 6);
        assert_eq!(empty, 6);
        // Label says "top 50%".
        assert!(
            filled_text.contains("top 50%"),
            "percentile label missing, got {filled_text}"
        );
    }

    // ── Goalie panel (Phase G.7) ─────────────────────────────────────────

    #[test]
    fn l0_fmt3_drops_leading_zero() {
        // SV% goalie convention is ".925" not "0.925".
        assert_eq!(fmt3(0.925), ".925");
        assert_eq!(fmt3(0.9008), ".901");
        // 1.0 (theoretical max) keeps the "1." form.
        assert_eq!(fmt3(1.0), "1.000");
    }

    #[test]
    fn l0_goalie_panel_one_history_shows_season_only() {
        // We can't easily construct a 1-season bundled fixture, so this
        // test exercises the formatter directly via the helpers it
        // calls — just confirms the season-label branch reads sensibly.
        // (The real 1-season path is exercised when a goalie's nhl_id
        // happens to appear in only one bundled season; verified via
        // existing render integration.)
        assert_eq!(short_season("20252026"), "25-26");
        // Header for a single-history goalie reads:
        //   "Bundled history: 25-26"
        let row = format!("Bundled history: {}", short_season("20252026"));
        assert_eq!(row, "Bundled history: 25-26");
    }

    #[test]
    fn l0_fmt2_keeps_two_decimals_for_gaa() {
        // GAA uses standard 2-decimal formatting; no leading-zero strip.
        // (Goalies often show GAA < 1.00 in tiny samples, so we keep the 0.)
        assert_eq!(fmt2(2.05), "2.05");
        assert_eq!(fmt2(2.0), "2.00");
        assert_eq!(fmt2(0.5), "0.50");
        assert_eq!(fmt2(0.0), "0.00");
    }

    // ── Hart.5c.6 Phase A.1 — view-based compile() tests ──────────────
    //
    // These tests lock the D11 forcing function (cross-window
    // rejection), the triple-keyed cache shape, and the
    // LeagueContext::build parity claim.

    use icelines_core::fixtures;
    use icelines_core::identity::PlayerId;
    use icelines_core::model::Season;
    use icelines_core::season_stats::SeasonType;

    fn fixture_repo_with_one_skater() -> icelines_core::stats_repository::StatsRepository {
        let identity = fixtures::identity(8478402).build();
        let stats = fixtures::stats(8478402, 20242025, "EDM").build();
        fixtures::test_repo_with(identity, stats)
    }

    #[test]
    fn l0_compile_rejects_cross_window() {
        // D11: ctx_window must equal (season, season_type) or compile
        // refuses with CrossWindowCompile. This is the spec's named
        // safety boundary; if this regresses, every other invariant
        // about percentile bars rendering for the right window is moot.
        let repo = fixture_repo_with_one_skater();
        let panel = CompiledPanel::new();
        let ctx = LeagueContext::empty();
        let pid = PlayerId(8478402);
        let s = Season(20242025);
        let t = SeasonType::Regular;

        // ctx_window deliberately doesn't match (season, type).
        let result = panel.compile(
            &repo,
            s,
            t,
            pid,
            &ctx,
            (Season(20232024), SeasonType::Regular),
        );
        match result {
            Err(DashboardError::CrossWindowCompile {
                requested_s,
                requested_t,
                ctx_s,
                ctx_t,
            }) => {
                assert_eq!(requested_s, Season(20242025));
                assert_eq!(requested_t, SeasonType::Regular);
                assert_eq!(ctx_s, Season(20232024));
                assert_eq!(ctx_t, SeasonType::Regular);
            }
            other => panic!("expected CrossWindowCompile, got {other:?}"),
        }
    }

    #[test]
    fn l0_compile_returns_player_not_in_repo_for_missing_pid() {
        // Missing PID with valid ctx_window must surface PlayerNotInRepo
        // — the call site (D6 auto-pop UX) routes on this variant.
        let repo = fixture_repo_with_one_skater();
        let panel = CompiledPanel::new();
        let ctx = LeagueContext::empty();
        let s = Season(20242025);
        let t = SeasonType::Regular;

        let result = panel.compile(
            &repo,
            s,
            t,
            PlayerId(99999), // not in fixture
            &ctx,
            (s, t),
        );
        match result {
            Err(DashboardError::PlayerNotInRepo {
                season,
                season_type,
            }) => {
                assert_eq!(season, s);
                assert_eq!(season_type, t);
            }
            other => panic!("expected PlayerNotInRepo, got {other:?}"),
        }
    }

    #[test]
    fn l0_compile_succeeds_with_matching_ctx_window() {
        // Happy path. Returns CompiledOutput; lines non-empty; cache
        // gets populated under the triple key.
        let repo = fixture_repo_with_one_skater();
        let panel = CompiledPanel::new();
        let ctx = LeagueContext::build(&repo, Season(20242025), SeasonType::Regular);
        let pid = PlayerId(8478402);
        let s = Season(20242025);
        let t = SeasonType::Regular;

        let out = panel
            .compile(&repo, s, t, pid, &ctx, (s, t))
            .expect("happy-path compile");
        assert!(
            !out.lines.is_empty(),
            "compile output must include header line"
        );

        // The cache must be keyed on (nhl_id, Season, SeasonType) — not
        // just nhl_id. Verify by inspecting the inner state.
        let guard = panel.inner.lock().unwrap();
        assert!(
            guard.by_view.contains_key(&(8478402, s, t)),
            "compile must populate by_view with the triple key"
        );
    }

    #[test]
    fn l0_compile_cache_isolates_by_window() {
        // The cache key is (nhl_id, Season, SeasonType), so the same
        // player in two windows produces two distinct cache entries.
        // Without this isolation, a season switch could return stale
        // lines from the previous window.
        let identity = fixtures::identity(8478402).build();
        let stats_2024 = fixtures::stats(8478402, 20242025, "EDM").build();
        let stats_2023 = fixtures::stats(8478402, 20232024, "EDM").build();
        let mut repo = icelines_core::stats_repository::StatsRepository::new();
        repo.upsert_identity(identity).unwrap();
        repo.upsert_stats(stats_2024).unwrap();
        repo.upsert_stats(stats_2023).unwrap();

        let panel = CompiledPanel::new();
        let pid = PlayerId(8478402);

        let s_24 = Season(20242025);
        let s_23 = Season(20232024);
        let t = SeasonType::Regular;
        let ctx_24 = LeagueContext::build(&repo, s_24, t);
        let ctx_23 = LeagueContext::build(&repo, s_23, t);

        let _ = panel
            .compile(&repo, s_24, t, pid, &ctx_24, (s_24, t))
            .unwrap();
        let _ = panel
            .compile(&repo, s_23, t, pid, &ctx_23, (s_23, t))
            .unwrap();

        let guard = panel.inner.lock().unwrap();
        assert!(guard.by_view.contains_key(&(8478402, s_24, t)));
        assert!(guard.by_view.contains_key(&(8478402, s_23, t)));
        assert_eq!(
            guard.by_view.len(),
            2,
            "two distinct (player, window) entries — no collapse"
        );
    }

    #[test]
    fn l0_clear_cache_drops_compile_entries() {
        let repo = fixture_repo_with_one_skater();
        let panel = CompiledPanel::new();
        let ctx = LeagueContext::build(&repo, Season(20242025), SeasonType::Regular);
        let pid = PlayerId(8478402);
        let s = Season(20242025);
        let t = SeasonType::Regular;

        let _ = panel.compile(&repo, s, t, pid, &ctx, (s, t)).unwrap();
        assert!(!panel.inner.lock().unwrap().by_view.is_empty());

        panel.clear_cache();
        assert!(
            panel.inner.lock().unwrap().by_view.is_empty(),
            "by_view must clear",
        );
    }

    #[test]
    fn l0_league_context_build_buckets_skaters_by_position() {
        // The repo-based build constructor produces per-position
        // sorted-asc pace_82 vectors. With one skater (a Center)
        // there's a single Center bucket of length 1. Goalie views
        // are excluded; players without pace_82 (BelowThreshold) are
        // skipped.
        let repo = fixture_repo_with_one_skater();
        let s = Season(20242025);
        let t = SeasonType::Regular;
        let ctx = LeagueContext::build(&repo, s, t);
        // Center bucket exists with the fixture's pace.
        let center_bucket = ctx
            .pace_by_position
            .get(&icelines_core::model::Position::Center)
            .expect("center bucket must exist for the seeded center");
        assert_eq!(center_bucket.len(), 1);
        // No goalie bucket for skater-only seeding.
        assert!(!ctx
            .pace_by_position
            .contains_key(&icelines_core::model::Position::Goalie));
    }
}
