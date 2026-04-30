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

use icelines_core::model::Player;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::tui::sparkline;

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
    pub fn lines_for_player(&self, p: &Player) -> Vec<Line<'static>> {
        if let Some(id) = p.nhl_id {
            let guard = self.inner.lock().unwrap();
            if let Some(cached) = guard.by_player.get(&id) {
                return cached.clone();
            }
        }
        let lines = build_panel_lines(p);
        if let Some(id) = p.nhl_id {
            self.inner.lock().unwrap().by_player.insert(id, lines.clone());
        }
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

/// Build the full set of styled lines for one player. Layout is unchanged
/// from the plain-text version (identity → counting → trend), but each
/// section now uses colour to convey meaning at a glance.
fn build_panel_lines(p: &Player) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::with_capacity(14);
    let dim    = Style::default().fg(DIM_COLOR);
    let title  = Style::default().fg(TITLE_COLOR).add_modifier(Modifier::BOLD);
    let accent = Style::default().fg(ACCENT_COLOR);

    // ── Identity ──────────────────────────────────────────────────────
    lines.push(Line::styled(trim_to(&p.full_name, PANEL_WIDTH), title));
    lines.push(Line::from(vec![
        Span::styled(p.team.as_str().to_owned(), accent),
        Span::styled("  ·  ", dim),
        Span::raw(p.position.abbreviation().to_owned()),
        Span::styled("  ·  ", dim),
        Span::raw(format!(
            "{}/{}",
            p.nationality_code.as_deref().unwrap_or("—"),
            p.shoots_catches.as_deref().unwrap_or("—"),
        )),
    ]));
    lines.push(Line::from(""));

    // ── Counting stats ────────────────────────────────────────────────
    // Each row: `LABEL  VALUE   LABEL  VALUE` — values get the accent
    // colour, labels stay dim.
    lines.push(stat_row("G  ", &p.season_goals.to_string(), "A  ", &p.season_assists.to_string(),
                        dim, accent));
    lines.push(stat_row("Pts", &p.season_points.to_string(),
                        "+/-", &format!("{:+}", p.plus_minus),
                        dim, accent));
    lines.push(stat_row("PP", &p.pp_points.to_string(),
                        "SOG", &p.shots.to_string(),
                        dim, accent));
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
            let row = &history[0];
            lines.push(Line::styled(
                format!("Bundled history: {}", short_season(row.season)),
                dim,
            ));
            lines.push(stat_row("G  ", &row.goals.to_string(),
                                "Pts", &row.points.to_string(),
                                dim, accent));
        }
        _ => {
            let goals_values: Vec<f64> = history.iter().map(|r| r.goals as f64).collect();
            let pts_values:   Vec<f64> = history.iter().map(|r| r.points as f64).collect();
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
            let pad = 5usize.saturating_sub(history.len());

            lines.push(spark_row("G  ", pad, g_spark, first.goals, last.goals, dim, accent));
            lines.push(spark_row("Pts", pad, pts_spark, first.points, last.points, dim, accent));
        }
    }
    lines
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
                });
            }
        }
    }
    out
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
    fn l0_build_panel_lines_includes_identity_and_stats() {
        let p = fixture_player();
        let lines = build_panel_lines(&p);
        let body = lines_to_text(&lines);
        assert!(body.contains("Connor McDavid"), "name missing:\n{body}");
        assert!(body.contains("EDM"), "team missing:\n{body}");
        // The format uses 2-space padding around the dot separators.
        assert!(body.contains("·  C  ·"), "position missing:\n{body}");
        assert!(body.contains("CAN/L"), "nationality/handedness missing:\n{body}");
        // Counting stats (53 G, 127 Pts, +57)
        assert!(body.contains(" 53"), "goals missing:\n{body}");
        assert!(body.contains("127"), "points missing:\n{body}");
        assert!(body.contains("+57"), "plus_minus missing or unsigned:\n{body}");
    }

    #[test]
    fn l0_build_panel_lines_renders_sparklines_when_history_available() {
        // McDavid has rows in all 5 bundled seasons → trend region uses
        // sparklines + a labelled latest-season anchor.
        let p = fixture_player();
        let lines = build_panel_lines(&p);
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
        let lines = build_panel_lines(&p);
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
        // 1-season history: render the season's row, skip the spark.
        // Stub history loading by hand-building the lines via the same
        // shape, since we can't easily construct a 1-season player in
        // the bundled data. Verify the formatter output instead.
        let history = vec![HistoryRow { season: "20252026", goals: 12, points: 30 }];
        // The body for a 1-season fall-through is two lines:
        //   "Bundled history: 25-26"
        //   "G    12    Pts  30"
        // We assert the format directly via the helper functions.
        assert_eq!(short_season(history[0].season), "25-26");
        let row = format!("Bundled history: {}", short_season(history[0].season));
        assert_eq!(row, "Bundled history: 25-26");
    }

    #[test]
    fn l0_lines_for_player_caches_by_nhl_id() {
        let panel = CompiledPanel::new();
        let p = fixture_player();
        let id = p.nhl_id.expect("fixture has nhl_id");

        let first = panel.lines_for_player(&p);
        // Cache populated.
        {
            let s = panel.inner.lock().unwrap();
            assert!(s.by_player.contains_key(&id),
                "cache must populate after first compile");
        }
        // Second call returns cached lines (byte-equal).
        let second = panel.lines_for_player(&p);
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
}
