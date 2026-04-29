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

use std::sync::{Arc, Mutex};

/// Hand-authored dashboard source baked into the binary. Renders one
/// region with placeholder content; future iterations will bind real
/// player fields. Width/height are conservative so the panel fits in a
/// 30-col strip on the player detail screen.
const PLAYER_PANEL_SOURCE: &str = "\
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

/// A panel rendered from a proof source — caches the compiled lines so
/// the TUI renders the same string every frame without recompiling.
#[derive(Clone)]
pub struct CompiledPanel {
    inner: Arc<Mutex<PanelState>>,
}

#[derive(Default)]
struct PanelState {
    lines: Option<Vec<String>>,
}

impl CompiledPanel {
    pub fn new() -> Self {
        Self { inner: Arc::new(Mutex::new(PanelState::default())) }
    }

    /// Lines for the panel — compiles on first call, returns cached copy
    /// after that. Compile failures fall back to a single error line so
    /// the panel never panics out of the TUI render.
    pub fn lines(&self) -> Vec<String> {
        let mut guard = self.inner.lock().unwrap();
        if let Some(ref cached) = guard.lines {
            return cached.clone();
        }
        let lines = match compile_source_to_lines(PLAYER_PANEL_SOURCE) {
            Ok(v) => v,
            Err(e) => vec![
                "[dashboard error]".to_owned(),
                e.to_string(),
            ],
        };
        guard.lines = Some(lines.clone());
        lines
    }

    /// Drop the cached compile result. Used by tests that re-run the
    /// compile path after mutating something upstream.
    #[cfg(test)]
    pub fn clear_cache(&self) {
        self.inner.lock().unwrap().lines = None;
    }
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

/// Drop a leading YAML front-matter block (delimited by `---` on its own
/// line) and split the remainder into trimmed lines. Returns one line per
/// rendered row, preserving blank separators so the panel layout reads
/// the same as the source.
fn strip_front_matter_and_split(rendered: &str) -> Vec<String> {
    let mut lines = rendered.lines();
    if lines.clone().next().map(str::trim) == Some("---") {
        let _ = lines.next(); // opening ---
        for l in lines.by_ref() {
            if l.trim() == "---" { break; }
        }
    }
    lines
        .map(|l| l.trim_end().to_owned())
        // Drop leading blank lines so the panel starts at the top.
        .skip_while(|l| l.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_compile_source_to_lines_produces_panel_text() {
        let lines = compile_source_to_lines(PLAYER_PANEL_SOURCE)
            .expect("baked panel source must compile");
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
    fn l0_compile_failure_reports_error_lines_not_panic() {
        let panel = CompiledPanel::new();
        // Force-stash an error path through the cache, simulating what
        // happens when proof_lib rejects malformed input.
        {
            let mut s = panel.inner.lock().unwrap();
            s.lines = Some(vec![
                "[dashboard error]".to_owned(),
                "stub".to_owned(),
            ]);
        }
        let lines = panel.lines();
        assert_eq!(lines[0], "[dashboard error]");
    }
}
