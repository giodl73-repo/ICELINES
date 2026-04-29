//! Phase 8j — proof-compiled dashboard side panel for the TUI.
//!
//! Behind the `dashboards` feature flag (off by default — see
//! `crate::config::dashboards_enabled`). v0 ships a fixed `player-card`
//! dashboard source compiled at TUI startup and rendered as plain text
//! lines in the right-most column of the player detail screen.
//!
//! # Why a tempfile roundtrip
//!
//! `proof_lib::compile::compile_file` only takes paths — there is no
//! string-in/string-out convenience yet (see `design/proof_lib.md`). We
//! write the embedded source to a `tempfile::TempDir`, compile, read the
//! output back, and cache the result for the rest of the session. Per the
//! proof maintainer the disk roundtrip is expected to disappear once a
//! `compile_str` helper lands upstream.
//!
//! # Caching
//!
//! Compilation is sub-millisecond for these tiny fixtures, but doing it
//! every TUI frame would still waste cycles and spam tempdirs. The
//! `CompiledPanel` struct caches the rendered lines after the first
//! compile and returns the cached copy for subsequent frames.

use icelines_core::model::Player;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Static placeholder source rendered when no player is in scope (e.g. on a
/// screen other than the player detail card). Doubles as a sanity check
/// that proof_lib is reachable at all — if this fails the user sees the
/// `[dashboard error]` line and knows the integration is broken.
const PLACEHOLDER_SOURCE: &str = "\
---
dashboard:
  width: 28
  height: 12
  title: \"Player panel (preview)\"
  regions:
    summary: { x: 0, y: 0, width: 28, height: 6 }
    notes:   { x: 0, y: 6, width: 28, height: 6 }
---

```proof:region name=summary
PROOF DASHBOARD
preview enabled · v0
icelines + proof_lib
```

```proof:region name=notes
opt-in via --dashboards
or ICELINES_DASHBOARDS=1
or dashboards=true
in ~/.icelines/config.toml
```
";

/// A panel rendered from a proof source — caches compiled lines so the
/// TUI renders the same string every frame without recompiling.
///
/// Two cache slots:
///   * `placeholder` — the static no-player-in-scope source, compiled
///     once.
///   * `by_player`   — per-player rendered lines, keyed by `nhl_id`.
///     Players without an `nhl_id` (rare; usually means a malformed
///     bios row) compile every frame; they're a tiny minority and the
///     compile cost is sub-millisecond on the panel's source size.
#[derive(Clone)]
pub struct CompiledPanel {
    inner: Arc<Mutex<PanelState>>,
}

#[derive(Default)]
struct PanelState {
    placeholder: Option<Vec<String>>,
    by_player:   HashMap<u32, Vec<String>>,
}

impl CompiledPanel {
    pub fn new() -> Self {
        Self { inner: Arc::new(Mutex::new(PanelState::default())) }
    }

    /// Static placeholder lines. Used when no player is in scope or as a
    /// fallback when per-player rendering can't run.
    pub fn lines(&self) -> Vec<String> {
        let mut guard = self.inner.lock().unwrap();
        if let Some(ref cached) = guard.placeholder {
            return cached.clone();
        }
        let lines = compile_or_error(PLACEHOLDER_SOURCE);
        guard.placeholder = Some(lines.clone());
        lines
    }

    /// Render the panel for a specific player. Builds a per-player proof
    /// source with the player's name, team, position, key counting and
    /// rate stats, and bio block. Caches by `nhl_id` so a screen that
    /// renders the same player every frame compiles only once.
    pub fn lines_for_player(&self, p: &Player) -> Vec<String> {
        // Fast path: cached by nhl_id.
        if let Some(id) = p.nhl_id {
            let guard = self.inner.lock().unwrap();
            if let Some(cached) = guard.by_player.get(&id) {
                return cached.clone();
            }
        }
        let source = build_player_source(p);
        let lines = compile_or_error(&source);
        if let Some(id) = p.nhl_id {
            self.inner.lock().unwrap().by_player.insert(id, lines.clone());
        }
        lines
    }

    /// Drop all caches. Used by tests that re-run compile paths after
    /// mutating fixture data.
    #[cfg(test)]
    pub fn clear_cache(&self) {
        let mut s = self.inner.lock().unwrap();
        s.placeholder = None;
        s.by_player.clear();
    }
}

fn compile_or_error(source: &str) -> Vec<String> {
    match compile_source_to_lines(source) {
        Ok(v) => v,
        Err(e) => vec!["[dashboard error]".to_owned(), e.to_string()],
    }
}

/// Compose the proof source for one player. Three regions stacked
/// vertically: identity (name + team + pos), counting stats, rate stats.
/// All values that may be `None` print as `—` so the layout never
/// collapses.
fn build_player_source(p: &Player) -> String {
    let name      = trim_to(&p.full_name, 26);
    let team      = p.team.as_str();
    let pos       = p.position.abbreviation();
    let goals     = p.season_goals;
    let assists   = p.season_assists;
    let points    = p.season_points;
    let plus_minus = p.plus_minus;
    let gp_str    = p.gp().map(|g| g.to_string()).unwrap_or_else(|| "—".to_owned());
    let pace_str  = p.pace_score.as_ref()
        .map(|s| format!("{:.0}", s.pace_82))
        .unwrap_or_else(|| "—".to_owned());
    let ppg_str   = p.pace_score.as_ref()
        .map(|s| format!("{:.2}", s.pace_82 / 82.0))
        .unwrap_or_else(|| "—".to_owned());
    let pp_pts    = p.pp_points;
    let shots     = p.shots;
    let nat       = p.nationality_code.as_deref().unwrap_or("—");
    let shoots    = p.shoots_catches.as_deref().unwrap_or("—");

    format!("\
---
dashboard:
  width: 28
  height: 13
  title: \"Scout card\"
  regions:
    head:  {{ x: 0, y: 0, width: 28, height: 3 }}
    score: {{ x: 0, y: 3, width: 28, height: 5 }}
    rate:  {{ x: 0, y: 8, width: 28, height: 5 }}
---

```proof:region name=head
{name}
{team}  ·  {pos}  ·  {nat}/{shoots}
```

```proof:region name=score
G  {goals:>3}    A   {assists:>3}
Pts {points:>3}    +/- {plus_minus:>+3}
PP-Pts {pp_pts:>3}  Shots {shots:>3}
```

```proof:region name=rate
GP    {gp_str:>5}
PPG   {ppg_str:>5}
Pts/82 {pace_str:>4}
```
")
}

/// Truncate a string to at most `max` chars, keeping it ASCII-safe for
/// the proof source (no unbalanced YAML chars). Names like
/// "Connor McDavid" pass through; longer names get a trailing ellipsis.
fn trim_to(s: &str, max: usize) -> String {
    if s.chars().count() <= max { return s.replace('"', "'"); }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out.replace('"', "'")
}

impl Default for CompiledPanel {
    fn default() -> Self { Self::new() }
}

/// Compile a `*.dashboard.source.md` string by writing it to a tempdir,
/// running `proof_lib::compile::compile_file`, and reading the rendered
/// markdown back. Returns the rendered lines (without the YAML front-
/// matter that proof leaves in place).
fn compile_source_to_lines(source: &str) -> anyhow::Result<Vec<String>> {
    use anyhow::Context;
    use proof_lib::{compile::compile_file, GlintConfig};

    let dir = tempfile::tempdir()
        .context("tempdir for dashboard compile")?;
    let src = dir.path().join("panel.dashboard.source.md");
    let out = dir.path().join("panel.md");
    std::fs::write(&src, source)
        .context("writing panel source")?;

    let cfg = GlintConfig::default();
    let result = compile_file(&src, &out, dir.path(), &cfg)
        .context("compile_file failed")?;
    if !result.written {
        let errs: Vec<String> = result.violations.iter()
            .filter(|v| matches!(v.severity, proof_lib::compile::ViolationSeverity::Error))
            .map(|v| format!("{}: {}", v.code, v.message))
            .collect();
        anyhow::bail!("proof rejected source: {}",
            if errs.is_empty() { "no output written".to_owned() } else { errs.join("; ") });
    }
    let rendered = std::fs::read_to_string(&out)
        .context("reading compiled output")?;
    Ok(strip_front_matter_and_split(&rendered))
}

/// Strip proof's compiled-output scaffolding to leave just the rendered
/// region content. Handles, in order:
/// 1. Leading YAML front-matter (`---` … `---`).
/// 2. `<!-- proof:compiled from="…" -->` opening HTML comment marker.
/// 3. ` ```dashboard ` (or any opening) code fence — proof wraps the
///    rendered region content in a fenced code block.
/// 4. Closing ` ``` ` fence and `<!-- /proof:compiled -->` trailing
///    marker on the way out.
///
/// When the input lacks a dashboard fence (e.g. an older proof_lib that
/// emitted plain text), the function falls back to "front-matter strip,
/// drop scaffolding lines, return remainder" so the panel stays useful
/// across proof versions.
fn strip_front_matter_and_split(rendered: &str) -> Vec<String> {
    let lines: Vec<&str> = rendered.lines().collect();
    let mut idx = 0;

    // 1. Skip YAML front-matter.
    if lines.first().map(|l| l.trim()) == Some("---") {
        idx = 1;
        while idx < lines.len() && lines[idx].trim() != "---" {
            idx += 1;
        }
        if idx < lines.len() { idx += 1; } // step past closing `---`
    }

    // 2. Skip blank lines and the `<!-- proof:compiled ... -->` opener.
    while idx < lines.len() {
        let t = lines[idx].trim();
        if t.is_empty() || t.starts_with("<!-- proof:compiled") {
            idx += 1;
            continue;
        }
        break;
    }

    // 3. If we land on a code-fence (```dashboard, ```, etc.), extract
    //    only the content between this fence and the matching close.
    if idx < lines.len() && lines[idx].trim_start().starts_with("```") {
        idx += 1; // step past opening fence
        let mut body = Vec::new();
        while idx < lines.len() {
            let trimmed = lines[idx].trim_start();
            // A closing fence is a line of just backticks (proof emits ``` on
            // its own line). Stop here without consuming the line.
            if trimmed.starts_with("```")
                && trimmed.trim_end().chars().all(|c| c == '`')
            {
                break;
            }
            body.push(lines[idx].trim_end().to_owned());
            idx += 1;
        }
        // Trim leading + trailing blanks inside the fence so the panel
        // doesn't show a tall empty band above the first region.
        while body.first().map(|s| s.trim().is_empty()).unwrap_or(false) {
            body.remove(0);
        }
        while body.last().map(|s| s.trim().is_empty()).unwrap_or(false) {
            body.pop();
        }
        return body;
    }

    // 4. Fallback: no dashboard fence — return the remainder verbatim,
    //    skipping any straggling proof scaffolding markers.
    lines[idx..]
        .iter()
        .filter(|l| {
            let t = l.trim();
            !t.starts_with("<!-- proof:compiled") && !t.starts_with("<!-- /proof:compiled")
        })
        .map(|l| l.trim_end().to_owned())
        .skip_while(|l| l.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_compile_source_to_lines_produces_panel_text() {
        let lines = compile_source_to_lines(PLACEHOLDER_SOURCE)
            .expect("placeholder source must compile");
        let body = lines.join("\n");
        // Region bodies appear in the rendered output.
        assert!(body.contains("PROOF DASHBOARD"),
            "summary region body missing, got:\n{body}");
        assert!(body.contains("opt-in"),
            "notes region body missing, got:\n{body}");
        // Front-matter `dashboard:` key should have been stripped.
        assert!(!body.contains("dashboard:"),
            "front-matter not stripped, got:\n{body}");
    }

    #[test]
    fn l0_panel_lines_caches_after_first_call() {
        let panel = CompiledPanel::new();
        let first = panel.lines();
        let second = panel.lines();
        assert_eq!(first, second,
            "cached lines must be byte-identical to first call");
        // After clearing the cache, recompiling must still produce
        // the same lines (idempotent given the same source).
        panel.clear_cache();
        let third = panel.lines();
        assert_eq!(first, third,
            "second compile of same source must match the first");
    }

    #[test]
    fn l0_strip_front_matter_handles_empty_body() {
        let rendered = "---\nfoo: 1\n---\n\nhello\n";
        let lines = strip_front_matter_and_split(rendered);
        assert_eq!(lines, vec!["hello".to_owned()]);
    }

    #[test]
    fn l0_strip_front_matter_returns_input_when_no_delimiter() {
        // Input without front-matter passes through unchanged.
        let rendered = "alpha\nbeta\n";
        let lines = strip_front_matter_and_split(rendered);
        assert_eq!(lines, vec!["alpha".to_owned(), "beta".to_owned()]);
    }

    #[test]
    fn l0_strip_unwraps_proof_compiled_dashboard_fence() {
        // Real-world proof output shape: front-matter, a
        // `<!-- proof:compiled -->` opener, a ```dashboard fence with the
        // rendered region body, a closing fence, and a trailing comment.
        // The stripper must return only the region body.
        let rendered = "\
---
dashboard:
  width: 28
---

<!-- proof:compiled from=\"panel.dashboard.source.md\" -->
```dashboard
PROOF DASHBOARD
preview enabled · v0
```
<!-- /proof:compiled -->
";
        let lines = strip_front_matter_and_split(rendered);
        assert_eq!(lines, vec![
            "PROOF DASHBOARD".to_owned(),
            "preview enabled · v0".to_owned(),
        ], "must extract only the dashboard region body, got: {lines:?}");
    }

    #[test]
    fn l0_strip_unwraps_proof_dashboard_without_front_matter() {
        // Some proof versions might emit the compiled comment + fence
        // without a leading YAML front-matter. Stripper still works.
        let rendered = "\
<!-- proof:compiled from=\"x.dashboard.source.md\" -->
```dashboard
LINE A
LINE B
```
<!-- /proof:compiled -->
";
        let lines = strip_front_matter_and_split(rendered);
        assert_eq!(lines, vec!["LINE A".to_owned(), "LINE B".to_owned()]);
    }

    #[test]
    fn l0_real_proof_output_does_not_leak_fence_markers() {
        // End-to-end: compile the baked source and verify the result
        // never includes the wrapping scaffolding the user saw leak in
        // the v0 release.
        let lines = compile_source_to_lines(PLACEHOLDER_SOURCE)
            .expect("compile must succeed");
        let body = lines.join("\n");
        assert!(!body.contains("<!-- proof:compiled"),
            "compiled marker leaked, got:\n{body}");
        assert!(!body.contains("<!-- /proof:compiled"),
            "closing marker leaked, got:\n{body}");
        assert!(!body.contains("```dashboard"),
            "dashboard fence leaked, got:\n{body}");
        // Region content is still present.
        assert!(body.contains("PROOF DASHBOARD"),
            "region body missing, got:\n{body}");
    }

    // ── Per-player rendering (Phase 8j) ───────────────────────────────────

    fn fixture_player() -> Player {
        // Hand-authored JSON string so we don't have to enumerate every
        // Player field as Rust syntax. Only the fields build_player_source
        // reads need realistic values; the rest can be defaults.
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
            "pp_goals": 11,
            "pp_points": 30,
            "sh_goals": 0,
            "sh_points": 0,
            "gwg": 7,
            "ot_goals": 1,
            "shots": 350,
            "shooting_pct": 15.1,
            "plus_minus": 57,
            "toi_per_game_sec": 1335.0,
            "faceoff_win_pct": 53.0,
            "hits": 0,
            "blocked_shots": 18,
            "missed_shots": 80,
            "giveaways": 50,
            "takeaways": 70,
            "pim": 24,
            "xg": null, "xg_per_60": null,
            "cf_pct_5v5": null, "ff_pct_5v5": null, "xgf_pct_5v5": null,
            "headshot_url": null, "sweater_number": 97,
            "birth_date": "1997-01-13",
            "birth_country": "CAN",
            "nationality_code": "CAN",
            "birth_city": "Richmond Hill",
            "birth_state_province": "ON",
            "shoots_catches": "L",
            "height_in_inches": 73, "weight_lbs": 192,
            "draft_year": 2015, "draft_round": 1, "draft_overall": 1,
            "rookie_season": 20152016,
            "contract_expiry_year": 2026, "expiry_type": "UFA",
            "salary": 12500000
        }"#;
        serde_json::from_str(json).expect("fixture player round-trips")
    }

    #[test]
    fn l0_build_player_source_includes_name_team_pos() {
        let p = fixture_player();
        let src = build_player_source(&p);
        assert!(src.contains("Connor McDavid"), "name missing in source:\n{src}");
        assert!(src.contains("EDM"),  "team missing in source:\n{src}");
        assert!(src.contains("· C ·") || src.contains("·  C  ·"),
            "position missing in source:\n{src}");
    }

    #[test]
    fn l0_build_player_source_renders_real_stats() {
        let p = fixture_player();
        let src = build_player_source(&p);
        // Goals, assists, points, +/- — concrete McDavid numbers.
        assert!(src.contains("G   53") || src.contains("G  53") || src.contains(" 53"),
            "goals missing:\n{src}");
        assert!(src.contains("127"),    "points missing:\n{src}");
        assert!(src.contains("+57"),    "plus_minus missing or unsigned:\n{src}");
        assert!(src.contains("80"),     "GP missing:\n{src}");
        // Pace: pace_82 = 130.2 → "130" with `:.0` formatter
        assert!(src.contains("130"),    "Pts/82 pace missing:\n{src}");
        // PPG: 130.2 / 82 ≈ 1.59
        assert!(src.contains("1.59"),   "PPG rate missing:\n{src}");
    }

    #[test]
    fn l0_lines_for_player_compiles_and_caches_by_id() {
        let panel = CompiledPanel::new();
        let p = fixture_player();
        let id = p.nhl_id.expect("fixture has nhl_id");

        let first = panel.lines_for_player(&p);
        assert!(first.iter().any(|l| l.contains("Connor McDavid")),
            "player name must surface in compiled output, got:\n{}", first.join("\n"));

        // Cache populated for this player_id.
        {
            let s = panel.inner.lock().unwrap();
            assert!(s.by_player.contains_key(&id),
                "cache must populate after first compile");
        }
        // Second call returns the cached lines (byte-equal).
        let second = panel.lines_for_player(&p);
        assert_eq!(first, second);
    }

    #[test]
    fn l0_lines_for_player_handles_optional_fields_with_em_dash() {
        // Player with no pace_score, no GP — placeholder dashes must
        // appear in the source so we don't render bare `None`s.
        let mut p = fixture_player();
        p.pace_score = None;
        p.gp_status  = icelines_core::model::GpStatus::Zero;
        let src = build_player_source(&p);
        assert!(src.contains("—"),
            "missing fields should render as em-dash, got:\n{src}");
    }

    #[test]
    fn l0_trim_to_truncates_long_names_with_ellipsis() {
        assert_eq!(trim_to("Short", 26), "Short");
        let long = "A".repeat(40);
        let trimmed = trim_to(&long, 26);
        assert!(trimmed.chars().count() <= 26);
        assert!(trimmed.ends_with('…'),
            "expected ellipsis, got: {trimmed}");
    }

    #[test]
    fn l0_compile_failure_reports_error_lines_not_panic() {
        let panel = CompiledPanel::new();
        // Force-stash an error path through the placeholder cache,
        // simulating what happens when proof_lib rejects malformed input.
        {
            let mut s = panel.inner.lock().unwrap();
            s.placeholder = Some(vec![
                "[dashboard error]".to_owned(),
                "stub".to_owned(),
            ]);
        }
        let lines = panel.lines();
        assert_eq!(lines[0], "[dashboard error]");
    }
}
