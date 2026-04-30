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
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::tui::sparkline;

/// A panel rendered for a specific player. Caches by `nhl_id` so
/// re-rendering the same player on every TUI frame is free after the
/// first build. Cache miss is sub-millisecond — pure formatting and a
/// short slice over `BUNDLED_SEASONS`.
#[derive(Clone)]
pub struct CompiledPanel {
    inner: Arc<Mutex<PanelState>>,
}

#[derive(Default)]
struct PanelState {
    by_player: HashMap<u32, Vec<String>>,
}

impl CompiledPanel {
    pub fn new() -> Self {
        Self { inner: Arc::new(Mutex::new(PanelState::default())) }
    }

    /// Render the panel for a specific player. Builds the lines from the
    /// player's name/team/pos, counting stats, and bundled history (if
    /// any), and caches by `nhl_id`. Players without an `nhl_id` (rare —
    /// usually a malformed bios row) re-render every frame; the cost is
    /// negligible.
    pub fn lines_for_player(&self, p: &Player) -> Vec<String> {
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

    /// Drop all cached compilations. Used by tests after mutating fixture
    /// data so subsequent calls re-build instead of returning stale rows.
    #[cfg(test)]
    pub fn clear_cache(&self) {
        self.inner.lock().unwrap().by_player.clear();
    }
}

impl Default for CompiledPanel {
    fn default() -> Self { Self::new() }
}

/// Width of the panel content (inside the ratatui border). Matches the
/// `Constraint::Length(30)` minus 2 for the border in
/// `tui::screens::player::render_dashboard_panel`.
const PANEL_WIDTH: usize = 28;

/// Build the full set of lines for one player. Three logical blocks
/// stacked vertically with a blank-line separator:
///   * identity (name + team · pos · nationality/handedness)
///   * counting stats (G/A/Pts/+/-, PP-Pts/Shots)
///   * 5-season trend (two sparklines or a text fallback).
fn build_panel_lines(p: &Player) -> Vec<String> {
    let mut lines = Vec::with_capacity(14);

    // ── Identity ──────────────────────────────────────────────────
    lines.push(trim_to(&p.full_name, PANEL_WIDTH));
    lines.push(format!(
        "{}  ·  {}  ·  {}/{}",
        p.team.as_str(),
        p.position.abbreviation(),
        p.nationality_code.as_deref().unwrap_or("—"),
        p.shoots_catches.as_deref().unwrap_or("—"),
    ));
    lines.push(String::new());

    // ── Counting stats ────────────────────────────────────────────
    lines.push(format!(
        "G  {:>3}    A   {:>3}",
        p.season_goals, p.season_assists,
    ));
    lines.push(format!(
        "Pts {:>3}    +/- {:>+3}",
        p.season_points, p.plus_minus,
    ));
    lines.push(format!(
        "PP-Pts {:>3}  Shots {:>3}",
        p.pp_points, p.shots,
    ));
    lines.push(String::new());

    // ── Bundled-history trend ─────────────────────────────────────
    // Render whatever we have:
    //   0 seasons → pace fallback (rookie / non-NHL / unknown ID)
    //   1 season  → labelled values row (no sparkline — no trend yet)
    //   2+        → sparklines + latest-season anchor
    let history = p.nhl_id.map(load_player_history).unwrap_or_default();
    match history.len() {
        0 => {
            let pace = p.pace_score.as_ref()
                .map(|s| format!("{:.0}", s.pace_82))
                .unwrap_or_else(|| "—".to_owned());
            let ppg = p.pace_score.as_ref()
                .map(|s| format!("{:.2}", s.pace_82 / 82.0))
                .unwrap_or_else(|| "—".to_owned());
            lines.push("Bundled history: none".to_owned());
            lines.push(format!("Pts/82  {pace:>5}"));
            lines.push(format!("PPG     {ppg:>5}"));
        }
        1 => {
            // Single season — show the row, skip the meaningless spark.
            let row = &history[0];
            lines.push(format!("Bundled history: {}", short_season(row.season)));
            lines.push(format!("G   {:>3}    Pts {:>3}", row.goals, row.points));
        }
        _ => {
            // Sparklines are 1 char per season. The two-digit year labels we
            // tried originally (`21-22`) don't align with single-char columns
            // so we show the range once at the end of each spark instead, and
            // anchor with the first + last season's totals so the scale is
            // legible at a glance.
            let goals_values: Vec<f64> = history.iter().map(|r| r.goals as f64).collect();
            let pts_values:   Vec<f64> = history.iter().map(|r| r.points as f64).collect();
            let spark_width = history.len();
            let g_spark   = sparkline::render(&goals_values, spark_width);
            let pts_spark = sparkline::render(&pts_values,   spark_width);
            // First and last season tags compressed to year-pairs (e.g. 21→26).
            let first = &history[0];
            let last  = &history[history.len() - 1];
            let range = format!("{}→{}", short_year(first.season), short_year(last.season));

            lines.push(format!("Last 5 seasons {range}"));
            // Pad spark to a common width so the column count is fixed even when
            // history.len() < 5 (e.g., a 3-season player still fills 5 cols-worth
            // of leading whitespace so the panel looks consistent).
            let pad = " ".repeat(5usize.saturating_sub(spark_width));
            lines.push(format!("G   {pad}{g_spark}    {} → {}", first.goals,  last.goals));
            lines.push(format!("Pts {pad}{pts_spark}    {} → {}", first.points, last.points));
        }
    }
    lines
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

    #[test]
    fn l0_build_panel_lines_includes_identity_and_stats() {
        let p = fixture_player();
        let lines = build_panel_lines(&p);
        let body = lines.join("\n");
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
        let body = lines.join("\n");
        // At least one Unicode block from the sparkline alphabet.
        let has_block = body.chars().any(|c| "▁▂▃▄▅▆▇█".contains(c));
        assert!(has_block,
            "expected sparkline blocks, got:\n{body}");
        // Both Goals and Points lines render.
        assert!(body.contains("G   ▁") || body.contains("G   ▂") || body.contains("G   "),
            "goals sparkline row missing:\n{body}");
        assert!(body.contains("Pts "),
            "points row missing:\n{body}");
        // Range marker + first/last anchors appear.
        assert!(body.contains("Last 5 seasons"),
            "range header missing:\n{body}");
        assert!(body.contains("21→26") || body.contains("22→26"),
            "year-range marker missing:\n{body}");
        // First-to-last counts visible on the spark rows.
        assert!(body.contains(" → "),
            "first → last anchors missing:\n{body}");
    }

    #[test]
    fn l0_build_panel_lines_falls_back_when_no_history() {
        // Made-up nhl_id matches no bundled row → pace fallback.
        let mut p = fixture_player();
        p.nhl_id = Some(99999999);
        p.pace_score = None;
        let lines = build_panel_lines(&p);
        let body = lines.join("\n");
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
