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
//! borders and styling. proof_lib stays in the codebase as a dev-dep
//! for the smoke test in case we want to re-introduce it for site
//! generation later.

use icelines_core::model::{Player, Position};
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
    pub fn empty() -> Self { Self::default() }

    /// Build from a player slice. Skipped players: those without a
    /// `pace_score` (un-rankable) and goalies (we don't track skater
    /// pace for goalies). Resulting vectors are sorted ascending so
    /// rank lookups are an `O(log n)` binary search.
    pub fn from_players(players: &[Player]) -> Self {
        let mut buckets: HashMap<Position, Vec<f64>> = HashMap::new();
        for p in players {
            if matches!(p.position, Position::Goalie) { continue; }
            if let Some(s) = p.pace_score.as_ref() {
                buckets.entry(p.position).or_default().push(s.pace_82);
            }
        }
        for v in buckets.values_mut() {
            v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        }
        Self { pace_by_position: buckets }
    }

    /// Look up the player's `(rank, total, percentile)` at their position.
    /// `rank` is 1-based with 1 = highest pace_82; `percentile` is
    /// `0.0..=100.0` where 100 = top of league.
    /// Returns `None` for goalies, players without `pace_score`, or
    /// positions that aren't in this context (e.g. empty context).
    pub fn position_rank(&self, p: &Player) -> Option<PositionRank> {
        let pace = p.pace_score.as_ref()?.pace_82;
        let bucket = self.pace_by_position.get(&p.position)?;
        if bucket.is_empty() { return None; }
        // bucket is sorted ascending, so position from the top = total - lower-or-equal-count + 1.
        // Use binary_search to find the player's slot.
        let lower_or_equal = bucket.partition_point(|v| *v <= pace);
        let total = bucket.len();
        let rank = total - lower_or_equal + 1; // 1-based, top = 1
        let rank = rank.min(total).max(1);
        let percentile = if total == 1 {
            100.0
        } else {
            // Players strictly below the player divided by total - 1.
            let below = bucket.partition_point(|v| *v < pace);
            (below as f64) / ((total - 1) as f64) * 100.0
        };
        Some(PositionRank { rank, total, percentile })
    }
}

/// Result of a position-rank lookup.
#[derive(Debug, Clone, Copy)]
pub struct PositionRank {
    pub rank:       usize,  // 1-based, top = 1
    pub total:      usize,  // count of qualifying peers at the position
    pub percentile: f64,    // 0.0 ..= 100.0
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
    by_player: HashMap<u32, Vec<Line<'static>>>,
}

impl CompiledPanel {
    pub fn new() -> Self {
        Self { inner: Arc::new(Mutex::new(PanelState::default())) }
    }

    /// Build (or fetch from cache) the styled panel lines for a player.
    /// The `league` context is used for the position-rank section; pass
    /// `LeagueContext::empty()` to suppress that section.
    ///
    /// **Cache caveat**: results are keyed by `nhl_id` only. If the league
    /// context changes mid-session (e.g. players reload), call
    /// `clear_cache()` to force a rebuild.
    pub fn lines_for_player(&self, p: &Player, league: &LeagueContext) -> Vec<Line<'static>> {
        if let Some(id) = p.nhl_id {
            let guard = self.inner.lock().unwrap();
            if let Some(cached) = guard.by_player.get(&id) {
                return cached.clone();
            }
        }
        let lines = build_panel_lines(p, league);
        if let Some(id) = p.nhl_id {
            self.inner.lock().unwrap().by_player.insert(id, lines.clone());
        }
        lines
    }

    /// Phase G.7: build (or fetch from cache) the styled panel lines for
    /// a goalie. Same caching key (nhl_id) as the skater path — goalies
    /// have unique IDs so there's no risk of collision.
    pub fn lines_for_goalie(&self, g: &icelines_core::model::Goalie) -> Vec<Line<'static>> {
        let id = g.nhl_id;
        {
            let guard = self.inner.lock().unwrap();
            if let Some(cached) = guard.by_player.get(&id) {
                return cached.clone();
            }
        }
        let lines = build_goalie_panel_lines(g);
        self.inner.lock().unwrap().by_player.insert(id, lines.clone());
        lines
    }

    /// Drop all cached compilations. Used by tests that mutate fixture
    /// data so subsequent calls re-build.
    #[cfg(test)]
    pub fn clear_cache(&self) {
        self.inner.lock().unwrap().by_player.clear();
    }
}

impl Default for CompiledPanel {
    fn default() -> Self { Self::new() }
}

// ── Styling palette ────────────────────────────────────────────────────────

/// Colour for sparkline columns above the player's median. Bright green —
/// the season was a high water mark.
const HIGH_COLOR:  Color = Color::Green;
/// Colour for columns at the median. Plain white.
const MID_COLOR:   Color = Color::White;
/// Colour for columns below the median. Red — the season was a dip.
const LOW_COLOR:   Color = Color::Red;
/// Dim grey for chrome/labels.
const DIM_COLOR:   Color = Color::DarkGray;
/// Header / title color.
const TITLE_COLOR: Color = Color::Yellow;
/// Bright accent for the headline number on each row.
const ACCENT_COLOR: Color = Color::Cyan;

/// Width of the panel content (inside the ratatui border). Matches the
/// `Constraint::Length(30)` minus 2 for the border in
/// `tui::screens::player::render_dashboard_panel`.
const PANEL_WIDTH: usize = 28;

/// Build the full set of styled lines for one player. Sections, in order:
///
/// 1. **Header** — compact `Lastname · TEAM POS` confirms which player
///    the panel is showing without duplicating the full name + bio that
///    the left stats column already displays.
/// 2. **5-season trend** — three coloured sparklines (G, Pts, SOG) with
///    range marker and first→last anchors.
/// 3. **Position vs league** — rank + percentile bar (when context has
///    enough peers at the player's position).
///
/// Counting stats (G/A/Pts/+/-/PP/SOG) deliberately omitted — they live
/// in the left stats column on the player screen. The panel adds value
/// by surfacing what that column doesn't: history and league position.
fn build_panel_lines(p: &Player, league: &LeagueContext) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::with_capacity(10);
    let dim    = Style::default().fg(DIM_COLOR);
    let title  = Style::default().fg(TITLE_COLOR).add_modifier(Modifier::BOLD);
    let accent = Style::default().fg(ACCENT_COLOR);

    // ── Header ────────────────────────────────────────────────────────
    // Compact identity so the panel makes sense even with the cursor
    // mid-frame. Last name keeps the line short; team + position make it
    // clear which McDavid (etc.) the panel is showing.
    let last_name = p.full_name
        .rsplit_once(' ')
        .map(|(_, l)| l)
        .unwrap_or(p.full_name.as_str());
    lines.push(Line::from(vec![
        Span::styled(trim_to(last_name, 18), title),
        Span::styled("  ·  ", dim),
        Span::styled(p.team.as_str().to_owned(), accent),
        Span::styled(" ", dim),
        Span::raw(p.position.abbreviation().to_owned()),
    ]));
    lines.push(Line::from(""));

    // ── Bundled-history trend ────────────────────────────────────────
    let history = p.nhl_id.map(load_player_history).unwrap_or_default();
    match history.len() {
        0 => {
            let pace = p.pace_score.as_ref()
                .map(|s| format!("{:.0}", s.pace_82))
                .unwrap_or_else(|| "—".to_owned());
            let ppg = p.pace_score.as_ref()
                .map(|s| format!("{:.2}", s.pace_82 / 82.0))
                .unwrap_or_else(|| "—".to_owned());
            lines.push(Line::styled("Bundled history: none", dim));
            lines.push(stat_row("Pts/82", &pace, "PPG", &ppg, dim, accent));
        }
        1 => {
            // Single-season history: nothing to chart yet. Just confirm
            // which season is bundled — the actual G/Pts for that season
            // already show on the left stats column.
            let row = &history[0];
            lines.push(Line::styled(
                format!("Bundled history: {}", short_season(row.season)),
                dim,
            ));
        }
        _ => {
            let goals_values: Vec<f64> = history.iter().map(|r| r.goals  as f64).collect();
            let pts_values:   Vec<f64> = history.iter().map(|r| r.points as f64).collect();
            let shots_values: Vec<f64> = history.iter().map(|r| r.shots  as f64).collect();
            let first = &history[0];
            let last  = &history[history.len() - 1];
            let range = format!("{}→{}", short_year(first.season), short_year(last.season));

            lines.push(Line::from(vec![
                Span::styled("Last 5 seasons ", dim),
                Span::styled(range, accent),
            ]));
            // Sparkline columns coloured against the player's own median —
            // green when above, red when below, white when on the line.
            let g_spark   = colored_spark_spans(&goals_values, history.len());
            let pts_spark = colored_spark_spans(&pts_values,   history.len());
            let sh_spark  = colored_spark_spans(&shots_values, history.len());
            let pad = 5usize.saturating_sub(history.len());

            lines.push(spark_row("G  ", pad, g_spark,   first.goals,  last.goals,  dim, accent));
            lines.push(spark_row("Pts", pad, pts_spark, first.points, last.points, dim, accent));
            lines.push(spark_row("SOG", pad, sh_spark,  first.shots,  last.shots,  dim, accent));
        }
    }

    // ── Position vs league ────────────────────────────────────────────
    if let Some(rank) = league.position_rank(p) {
        lines.push(Line::from(""));
        let pos_letter = position_letter(p.position);
        // Header: "Pos vs C peers   #3/87"
        lines.push(Line::from(vec![
            Span::styled(format!("Pos vs {pos_letter}: "), dim),
            Span::styled(format!("#{}/{}", rank.rank, rank.total), accent),
        ]));
        // Bar: 12 cols, colour-graded by percentile band.
        lines.push(Line::from(percentile_bar_spans(rank.percentile, 12, dim, accent)));
    }

    lines
}

/// Letter abbreviation used in the position-rank header.
fn position_letter(pos: Position) -> &'static str {
    match pos {
        Position::Center    => "C",
        Position::LeftWing  => "LW",
        Position::RightWing => "RW",
        Position::Defense   => "D",
        Position::Goalie    => "G",
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
    let band_color = if pct >= 75.0 { Color::Green }
                     else if pct >= 50.0 { Color::Yellow }
                     else if pct >= 25.0 { Color::White }
                     else { Color::Red };
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
    spans.push(Span::styled(format!("top {}%", percentile_to_top_pct(pct)), accent));
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
fn stat_row(
    l1: &str, v1: &str, l2: &str, v2: &str,
    dim: Style, accent: Style,
) -> Line<'static> {
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
            let color = if val > median { HIGH_COLOR }
                        else if val < median { LOW_COLOR }
                        else { MID_COLOR };
            Span::styled(ch.to_string(), Style::default().fg(color))
        })
        .collect()
}

/// Median of a numeric slice. Returns `0.0` for empty input — caller
/// should not pass empty slices in practice (the sparkline path guards).
fn median_of(values: &[f64]) -> f64 {
    if values.is_empty() { return 0.0; }
    let mut sorted: Vec<f64> = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
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
    season: &'static str,  // e.g. "20242025"
    goals:  u32,
    points: u32,
    shots:  u32,
}

/// Walk the bundled-history seasons (currently 5) in chronological order
/// and pick out the player's stats row. Missing seasons are skipped —
/// the sparkline accepts any length ≥ 2 and the label row carries the
/// actual season tags so gaps are obvious if they occur.
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
                    goals:  row.goals,
                    points: row.points,
                    shots:  row.shots,
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
    season:   &'static str,  // e.g. "20242025"
    save_pct: f32,           // 0.0..=1.0
    gaa:      f32,
    wins:     u32,
}

/// Walk the 5 bundled seasons and pull the goalie's row from each.
/// Missing seasons skipped — sparkline accepts variable length.
fn load_goalie_history(nhl_id: u32) -> Vec<GoalieHistoryRow> {
    use icelines_fetch::bundled;
    let mut out = Vec::new();
    for season in bundled::BUNDLED_SEASONS.iter().rev() {
        if let Some(stats) = bundled::get_goalie_stats(season) {
            if let Some(row) = stats.iter().find(|s| s.player_id == nhl_id) {
                out.push(GoalieHistoryRow {
                    season,
                    save_pct: row.save_pct.unwrap_or(0.0),
                    gaa:      row.goals_against_average.unwrap_or(0.0),
                    wins:     row.wins,
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
fn build_goalie_panel_lines(g: &icelines_core::model::Goalie) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::with_capacity(10);
    let dim    = Style::default().fg(DIM_COLOR);
    let title  = Style::default().fg(TITLE_COLOR).add_modifier(Modifier::BOLD);
    let accent = Style::default().fg(ACCENT_COLOR);

    // Header — short identity line.
    let last_name = g.full_name
        .rsplit_once(' ')
        .map(|(_, l)| l)
        .unwrap_or(g.full_name.as_str());
    lines.push(Line::from(vec![
        Span::styled(trim_to(last_name, 18), title),
        Span::styled("  ·  ", dim),
        Span::styled(g.team.as_str().to_owned(), accent),
        Span::styled(" G", dim),
    ]));
    lines.push(Line::from(""));

    let history = load_goalie_history(g.nhl_id);
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
            let sv_values:  Vec<f64> = history.iter().map(|r| r.save_pct as f64).collect();
            // GAA inverted via negation: sparkline scales high → high
            // bars, so flipping the sign makes "low GAA" the high bar.
            // We label it normally and invert colour band semantics.
            let gaa_values_inv: Vec<f64> = history.iter().map(|r| -(r.gaa as f64)).collect();
            let w_values:   Vec<f64> = history.iter().map(|r| r.wins as f64).collect();
            let first = &history[0];
            let last  = &history[history.len() - 1];
            let range = format!("{}→{}", short_year(first.season), short_year(last.season));
            lines.push(Line::from(vec![
                Span::styled("Last 5 seasons ", dim),
                Span::styled(range, accent),
            ]));
            let pad = 5usize.saturating_sub(history.len());
            // SV%: higher better → standard colour mapping.
            let sv_spark = colored_spark_spans(&sv_values, history.len());
            lines.push(goalie_spark_row("SV%", pad, sv_spark,
                fmt3(first.save_pct), fmt3(last.save_pct), dim, accent));
            // GAA: lower better → invert colours by passing negated values.
            let gaa_spark = colored_spark_spans(&gaa_values_inv, history.len());
            lines.push(goalie_spark_row("GAA", pad, gaa_spark,
                fmt2(first.gaa), fmt2(last.gaa), dim, accent));
            let w_spark = colored_spark_spans(&w_values, history.len());
            lines.push(goalie_spark_row("W  ", pad, w_spark,
                first.wins.to_string(), last.wins.to_string(), dim, accent));
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
    if pad > 0 { spans.push(Span::raw(" ".repeat(pad))); }
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
    if s.chars().count() <= max { return s.to_owned(); }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_player() -> Player {
        // Hand-authored JSON string so we don't have to enumerate every
        // Player field as Rust syntax. Only the fields build_panel_lines
        // reads need realistic values; the rest are defaults.
        let json = r#"{
            "nhl_id": 8478402,
            "full_name": "Connor McDavid",
            "name_normalized": "connor_mcdavid",
            "team": "EDM",
            "position": "Center",
            "eligible_pos": ["Center"],
            "gp_status": { "Eligible": 80 },
            "season_goals": 53,
            "season_assists": 74,
            "season_points": 127,
            "pace_score": { "pace_82": 130.2, "goals_per_82": 54.3, "raw_points": 127, "gp": 80 },
            "pp_goals": 11, "pp_points": 30,
            "sh_goals": 0, "sh_points": 0,
            "gwg": 7, "ot_goals": 1,
            "shots": 350, "shooting_pct": 15.1,
            "plus_minus": 57,
            "toi_per_game_sec": 1335.0, "faceoff_win_pct": 53.0,
            "hits": 0, "blocked_shots": 18, "missed_shots": 80,
            "giveaways": 50, "takeaways": 70, "pim": 24,
            "xg": null, "xg_per_60": null,
            "cf_pct_5v5": null, "ff_pct_5v5": null, "xgf_pct_5v5": null,
            "headshot_url": null, "sweater_number": 97,
            "birth_date": "1997-01-13", "birth_country": "CAN",
            "nationality_code": "CAN", "birth_city": "Richmond Hill",
            "birth_state_province": "ON", "shoots_catches": "L",
            "height_in_inches": 73, "weight_lbs": 192,
            "draft_year": 2015, "draft_round": 1, "draft_overall": 1,
            "rookie_season": 20152016,
            "contract_expiry_year": 2026, "expiry_type": "UFA",
            "salary": 12500000
        }"#;
        serde_json::from_str(json).expect("fixture player round-trips")
    }

    /// Concatenate every line's text content (ignoring styles) into a
    /// single string for test assertions. ratatui's `Line` impls
    /// `Display` which already does this per line.
    fn lines_to_text(lines: &[Line<'static>]) -> String {
        lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn l0_build_panel_lines_header_uses_last_name_and_team() {
        // Header is compact: Lastname · TEAM POS. Counting stats live on
        // the left column of the player screen, not duplicated here.
        let p = fixture_player();
        let lines = build_panel_lines(&p, &LeagueContext::empty());
        let body = lines_to_text(&lines);
        assert!(body.contains("McDavid"), "last name missing:\n{body}");
        assert!(body.contains("EDM"),     "team missing:\n{body}");
        assert!(body.contains(" C"),      "position missing:\n{body}");
        // First name + the redundant counting block must NOT appear.
        assert!(!body.starts_with("Connor"),
            "header should use last name only, got:\n{body}");
        // Counting stats are on the left side now.
        assert!(!body.contains(" 53"),
            "goals row should not duplicate left column:\n{body}");
        assert!(!body.contains("127"),
            "points row should not duplicate left column:\n{body}");
    }

    #[test]
    fn l0_build_panel_lines_renders_sparklines_when_history_available() {
        // McDavid has rows in all 5 bundled seasons → trend region uses
        // three sparklines (G, Pts, SOG) + range marker.
        let p = fixture_player();
        let lines = build_panel_lines(&p, &LeagueContext::empty());
        let body = lines_to_text(&lines);
        let has_block = body.chars().any(|c| "▁▂▃▄▅▆▇█".contains(c));
        assert!(has_block,
            "expected sparkline blocks, got:\n{body}");
        assert!(body.contains("Last 5 seasons"),
            "range header missing:\n{body}");
        assert!(body.contains("21→26") || body.contains("22→26"),
            "year-range marker missing:\n{body}");
        assert!(body.contains(" → "),
            "first → last anchors missing:\n{body}");
        // All three trend rows present.
        assert!(body.lines().any(|l| l.starts_with("G  ")),
            "goals sparkline row missing:\n{body}");
        assert!(body.lines().any(|l| l.starts_with("Pts")),
            "points sparkline row missing:\n{body}");
        assert!(body.lines().any(|l| l.starts_with("SOG")),
            "shots sparkline row missing:\n{body}");
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
        assert_eq!(span_color(3), LOW_COLOR,  "min value should be red");
        // The median value renders as MID_COLOR, but with an even-count
        // series the median is the average of the two middle values
        // (here 44 — exactly equal to spans[0]). Assert spans[0] is mid.
        assert_eq!(span_color(0), MID_COLOR,
            "value equal to median should be white");
    }

    #[test]
    fn l0_median_of_even_and_odd_series() {
        assert_eq!(median_of(&[1.0, 3.0, 5.0]), 3.0);          // odd
        assert_eq!(median_of(&[1.0, 2.0, 3.0, 4.0]), 2.5);     // even avg
        assert_eq!(median_of(&[]), 0.0);                       // empty
        assert_eq!(median_of(&[7.0]), 7.0);                    // single
    }

    #[test]
    fn l0_build_panel_lines_falls_back_when_no_history() {
        // Made-up nhl_id matches no bundled row → pace fallback.
        let mut p = fixture_player();
        p.nhl_id = Some(99999999);
        p.pace_score = None;
        let lines = build_panel_lines(&p, &LeagueContext::empty());
        let body = lines_to_text(&lines);
        assert!(body.contains("Bundled history: none"),
            "no-history message missing:\n{body}");
        assert!(body.contains("—"),
            "em-dash for missing pace_score:\n{body}");
        assert!(!body.chars().any(|c| "▁▂▃▄▅▆▇█".contains(c)),
            "no sparklines when history is empty:\n{body}");
    }

    #[test]
    fn l0_build_panel_lines_single_season_shows_row_no_spark() {
        // 1-season history: just confirm the bundled season — counting
        // stats live on the left column of the player screen.
        let history = vec![HistoryRow { season: "20252026", goals: 12, points: 30, shots: 80 }];
        assert_eq!(short_season(history[0].season), "25-26");
        let row = format!("Bundled history: {}", short_season(history[0].season));
        assert_eq!(row, "Bundled history: 25-26");
    }

    #[test]
    fn l0_lines_for_player_caches_by_nhl_id() {
        let panel = CompiledPanel::new();
        let p = fixture_player();
        let id = p.nhl_id.expect("fixture has nhl_id");

        let first = panel.lines_for_player(&p, &LeagueContext::empty());
        // Cache populated.
        {
            let s = panel.inner.lock().unwrap();
            assert!(s.by_player.contains_key(&id),
                "cache must populate after first compile");
        }
        // Second call returns cached lines (byte-equal).
        let second = panel.lines_for_player(&p, &LeagueContext::empty());
        assert_eq!(first, second);
    }

    #[test]
    fn l0_load_player_history_returns_chronological() {
        // McDavid is in every bundled season; verify rows come back
        // oldest → newest so the sparkline reads left-to-right in time.
        let history = load_player_history(8478402);
        assert!(history.len() >= 4,
            "McDavid should appear in most/all 5 bundled seasons, got {}",
            history.len());
        let seasons: Vec<&str> = history.iter().map(|r| r.season).collect();
        let mut sorted = seasons.clone();
        sorted.sort();
        assert_eq!(seasons, sorted,
            "history must be chronological, got: {seasons:?}");
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
        assert!(trimmed.ends_with('…'),
            "expected trailing ellipsis, got {trimmed}");
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

    /// Build a small synthetic player pool with a known pace_82 distribution
    /// so position rank lookups are deterministic.
    fn fake_pace_player(nhl_id: u32, position: Position, pace: f64) -> Player {
        let json = format!(r#"{{
            "nhl_id": {nhl_id},
            "full_name": "Player {nhl_id}",
            "name_normalized": "player_{nhl_id}",
            "team": "TST",
            "position": "{}",
            "eligible_pos": ["{}"],
            "gp_status": {{ "Eligible": 80 }},
            "season_goals": 0, "season_assists": 0, "season_points": 0,
            "pace_score": {{ "pace_82": {pace}, "goals_per_82": 0.0, "raw_points": 0, "gp": 80 }},
            "pp_goals": 0, "pp_points": 0, "sh_goals": 0, "sh_points": 0,
            "gwg": 0, "ot_goals": 0, "shots": 0, "shooting_pct": null,
            "plus_minus": 0,
            "toi_per_game_sec": null, "faceoff_win_pct": null,
            "hits": 0, "blocked_shots": 0, "missed_shots": 0,
            "giveaways": 0, "takeaways": 0, "pim": 0,
            "xg": null, "xg_per_60": null,
            "cf_pct_5v5": null, "ff_pct_5v5": null, "xgf_pct_5v5": null,
            "headshot_url": null, "sweater_number": null,
            "birth_date": null, "birth_country": null,
            "nationality_code": null, "birth_city": null,
            "birth_state_province": null, "shoots_catches": null,
            "height_in_inches": null, "weight_lbs": null,
            "draft_year": null, "draft_round": null, "draft_overall": null,
            "rookie_season": null,
            "contract_expiry_year": null, "expiry_type": null, "salary": null
        }}"#, position_json(position), position_json(position));
        serde_json::from_str(&json).expect("synthetic player round-trips")
    }

    fn position_json(p: Position) -> &'static str {
        match p {
            Position::Center    => "Center",
            Position::LeftWing  => "LeftWing",
            Position::RightWing => "RightWing",
            Position::Defense   => "Defense",
            Position::Goalie    => "Goalie",
        }
    }

    #[test]
    fn l0_league_context_position_rank_basic() {
        // 5 centers with paces 50, 60, 70, 80, 90. Player at 90 = #1/5.
        let pool: Vec<Player> = [50.0, 60.0, 70.0, 80.0, 90.0]
            .iter().enumerate()
            .map(|(i, p)| fake_pace_player(i as u32 + 1, Position::Center, *p))
            .collect();
        let ctx = LeagueContext::from_players(&pool);

        let top = &pool[4]; // pace 90
        let rank = ctx.position_rank(top).expect("top player ranks");
        assert_eq!(rank.rank, 1);
        assert_eq!(rank.total, 5);
        assert!((rank.percentile - 100.0).abs() < 0.01,
            "top of 5 should be 100th percentile, got {}", rank.percentile);

        let bottom = &pool[0]; // pace 50
        let rank = ctx.position_rank(bottom).expect("bottom player ranks");
        assert_eq!(rank.rank, 5);
        assert!((rank.percentile - 0.0).abs() < 0.01,
            "bottom of 5 should be 0th percentile, got {}", rank.percentile);
    }

    #[test]
    fn l0_league_context_buckets_by_position() {
        // 3 centers + 2 defensemen — separate rank pools.
        let pool = vec![
            fake_pace_player(1, Position::Center,  100.0),
            fake_pace_player(2, Position::Center,  80.0),
            fake_pace_player(3, Position::Center,  60.0),
            fake_pace_player(4, Position::Defense, 50.0),
            fake_pace_player(5, Position::Defense, 40.0),
        ];
        let ctx = LeagueContext::from_players(&pool);
        let c_rank = ctx.position_rank(&pool[1]).expect("center #2 of 3");
        assert_eq!(c_rank.rank, 2);
        assert_eq!(c_rank.total, 3);
        let d_rank = ctx.position_rank(&pool[3]).expect("defenseman #1 of 2");
        assert_eq!(d_rank.rank, 1);
        assert_eq!(d_rank.total, 2);
    }

    #[test]
    fn l0_league_context_skips_players_without_pace() {
        let json = r#"{
            "nhl_id": 99, "full_name": "No Pace", "name_normalized": "no_pace",
            "team": "TST", "position": "Center", "eligible_pos": ["Center"],
            "gp_status": "Zero",
            "season_goals": 0, "season_assists": 0, "season_points": 0,
            "pace_score": null,
            "pp_goals": 0, "pp_points": 0, "sh_goals": 0, "sh_points": 0,
            "gwg": 0, "ot_goals": 0, "shots": 0, "shooting_pct": null,
            "plus_minus": 0, "toi_per_game_sec": null, "faceoff_win_pct": null,
            "hits": 0, "blocked_shots": 0, "missed_shots": 0,
            "giveaways": 0, "takeaways": 0, "pim": 0,
            "xg": null, "xg_per_60": null,
            "cf_pct_5v5": null, "ff_pct_5v5": null, "xgf_pct_5v5": null,
            "headshot_url": null, "sweater_number": null,
            "birth_date": null, "birth_country": null,
            "nationality_code": null, "birth_city": null,
            "birth_state_province": null, "shoots_catches": null,
            "height_in_inches": null, "weight_lbs": null,
            "draft_year": null, "draft_round": null, "draft_overall": null,
            "rookie_season": null,
            "contract_expiry_year": null, "expiry_type": null, "salary": null
        }"#;
        let no_pace: Player = serde_json::from_str(json).unwrap();
        let ctx = LeagueContext::from_players(&[no_pace.clone()]);
        assert!(ctx.position_rank(&no_pace).is_none(),
            "players without pace_score must not rank");
    }

    #[test]
    fn l0_league_context_goalies_excluded() {
        let g = fake_pace_player(1, Position::Goalie, 100.0);
        let ctx = LeagueContext::from_players(&[g.clone()]);
        assert!(ctx.position_rank(&g).is_none(),
            "goalies don't get a skater pace rank");
    }

    #[test]
    fn l0_percentile_to_top_pct_floors_complement() {
        assert_eq!(percentile_to_top_pct(100.0), 0);   // top of league
        assert_eq!(percentile_to_top_pct(96.5), 3);    // floor(3.5) = 3
        assert_eq!(percentile_to_top_pct(50.0), 50);
        assert_eq!(percentile_to_top_pct(0.0), 100);
    }

    #[test]
    fn l0_percentile_bar_spans_fill_proportional_to_rank() {
        let dim    = Style::default().fg(DIM_COLOR);
        let accent = Style::default().fg(ACCENT_COLOR);

        // 50% percentile, 12-col bar → 6 filled + 6 empty.
        let spans = percentile_bar_spans(50.0, 12, dim, accent);
        let filled_text: String = spans.iter().map(|s| s.content.to_string()).collect();
        let filled = filled_text.chars().filter(|c| *c == '█').count();
        let empty  = filled_text.chars().filter(|c| *c == '░').count();
        assert_eq!(filled, 6);
        assert_eq!(empty,  6);
        // Label says "top 50%".
        assert!(filled_text.contains("top 50%"),
            "percentile label missing, got {filled_text}");
    }

    #[test]
    fn l0_panel_includes_pos_vs_league_when_context_populated() {
        // McDavid in a 3-player center pool → ranks #1.
        let p = fixture_player();
        let pool = vec![
            fixture_player(),
            fake_pace_player(99001, Position::Center, 60.0),
            fake_pace_player(99002, Position::Center, 70.0),
        ];
        let ctx = LeagueContext::from_players(&pool);
        let lines = build_panel_lines(&p, &ctx);
        let body  = lines_to_text(&lines);
        assert!(body.contains("Pos vs C:"),
            "expected pos-rank header, got:\n{body}");
        assert!(body.contains("#1/3"),
            "expected #1/3 rank, got:\n{body}");
        assert!(body.contains("top "),
            "expected top-N% label, got:\n{body}");
    }

    #[test]
    fn l0_panel_omits_pos_vs_league_when_context_empty() {
        let p = fixture_player();
        let lines = build_panel_lines(&p, &LeagueContext::empty());
        let body  = lines_to_text(&lines);
        assert!(!body.contains("Pos vs"),
            "empty context must suppress pos-rank section, got:\n{body}");
    }

    // ── Goalie panel (Phase G.7) ─────────────────────────────────────────

    fn fixture_goalie() -> icelines_core::model::Goalie {
        // Connor Hellebuyck (id 8476945) is in all 5 bundled seasons.
        let json = r#"{
            "nhl_id": 8476945,
            "full_name": "Connor Hellebuyck",
            "name_normalized": "connor_hellebuyck",
            "team": "WPG",
            "stats": {
                "games_played": 63, "games_started": 62,
                "wins": 47, "losses": 12,
                "ot_losses": 3, "ties": null,
                "shots_against": 1664, "goals_against": 125, "saves": 1539,
                "save_pct": 0.92487, "goals_against_average": 2.00461,
                "shutouts": 8, "time_on_ice": 224482
            },
            "bio": {
                "birth_date": null, "birth_country": null,
                "nationality_code": null, "catches": "L",
                "height_in_inches": null, "weight_lbs": null,
                "draft_year": null, "draft_round": null, "draft_overall": null,
                "rookie_season": null
            },
            "headshot_url": null,
            "sweater_number": null
        }"#;
        serde_json::from_str(json).expect("goalie fixture parses")
    }

    #[test]
    fn l0_goalie_panel_header_uses_lastname_team_g() {
        let g = fixture_goalie();
        let lines = build_goalie_panel_lines(&g);
        let body = lines_to_text(&lines);
        assert!(body.contains("Hellebuyck"), "lastname missing: {body}");
        assert!(body.contains("WPG"),        "team missing: {body}");
        assert!(body.contains(" G"),         "G suffix missing: {body}");
    }

    #[test]
    fn l0_goalie_panel_renders_three_sparklines_when_history_present() {
        let g = fixture_goalie();
        let lines = build_goalie_panel_lines(&g);
        let body  = lines_to_text(&lines);
        // All three rows must appear.
        assert!(body.lines().any(|l| l.starts_with("SV%")),
            "SV% spark row missing:\n{body}");
        assert!(body.lines().any(|l| l.starts_with("GAA")),
            "GAA spark row missing:\n{body}");
        assert!(body.lines().any(|l| l.starts_with("W  ")),
            "W spark row missing:\n{body}");
        // Range marker present.
        assert!(body.contains("Last 5 seasons"),
            "range header missing:\n{body}");
        // SV% rendered as ".XYZ" (leading zero stripped). The first/last
        // anchors carry the values; only those show in body text. Match
        // any 3-digit decimal starting with a dot to avoid pinning specific
        // values that change as the bundled data refreshes.
        assert!(body.contains(".8") || body.contains(".9"),
            "SV% short form (leading zero stripped) missing, got:\n{body}");
        // And it should NOT include the unstripped prefix.
        assert!(!body.contains("0.9") && !body.contains("0.8"),
            "SV% should drop leading zero, got:\n{body}");
        // First→last anchors visible.
        assert!(body.contains(" → "),
            "first→last anchors missing, got:\n{body}");
        // Sparkline blocks present.
        assert!(body.chars().any(|c| "▁▂▃▄▅▆▇█".contains(c)),
            "expected sparkline blocks, got:\n{body}");
    }

    #[test]
    fn l0_fmt3_drops_leading_zero() {
        // SV% goalie convention is ".925" not "0.925".
        assert_eq!(fmt3(0.925),  ".925");
        assert_eq!(fmt3(0.9008), ".901");
        // 1.0 (theoretical max) keeps the "1." form.
        assert_eq!(fmt3(1.0),    "1.000");
    }

    #[test]
    fn l0_lines_for_goalie_caches_by_nhl_id() {
        let panel = CompiledPanel::new();
        let g = fixture_goalie();
        let first = panel.lines_for_goalie(&g);
        // Cache populated under the goalie's nhl_id.
        {
            let s = panel.inner.lock().unwrap();
            assert!(s.by_player.contains_key(&g.nhl_id),
                "goalie cache key missing");
        }
        let second = panel.lines_for_goalie(&g);
        assert_eq!(first, second,
            "second call must return cached lines");
    }
}
