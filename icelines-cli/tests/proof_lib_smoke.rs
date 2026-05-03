//! Smoke test for proof_lib consumption from icelines.
//!
//! Lifted verbatim from `design/proof_lib.md` (the hand-off note from
//! the proof maintainer). Proves the library link works end to end —
//! once this is green, Phase 8d (markdown export → dashboard render
//! pipeline) can begin in earnest.
//!
//! `proof_lib` is a dev-dependency only at this stage so a broken
//! upstream API doesn't take down the production icelines binary.
//! Promote to a real `[dependencies]` entry when Phase 8d wires
//! dashboards into the TUI.

use proof_lib::compile::{compile_file, ViolationSeverity};
use proof_lib::GlintConfig;

#[test]
fn proof_compiles_a_dashboard_spec() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("hello.dashboard.source.md");
    let out = dir.path().join("hello.md");

    // Minimal dashboard fixture matching proof's `two-region` test shape:
    // YAML front-matter declares region geometry, body fills each region.
    std::fs::write(
        &src,
        "\
---
dashboard:
  width: 20
  height: 6
  title: \"Hello\"
  regions:
    top: { x: 0, y: 0, width: 20, height: 3 }
    bottom: { x: 0, y: 3, width: 20, height: 3 }
---

```proof:region name=top
HEADER CONTENT
```

```proof:region name=bottom
FOOTER CONTENT
```
",
    )
    .unwrap();

    let cfg = GlintConfig::default();
    let result = compile_file(&src, &out, dir.path(), &cfg).unwrap();

    // Print the full result up front so any failure below has the diagnostic
    // context the proof team asked for in the hand-off note.
    eprintln!("CompileResult.written = {}", result.written);
    eprintln!(
        "CompileResult.directives_resolved = {}",
        result.directives_resolved
    );
    eprintln!("CompileResult.from_cache = {}", result.from_cache);
    for v in &result.violations {
        let sev = match v.severity {
            ViolationSeverity::Error => "ERROR",
            ViolationSeverity::Warning => "warning",
        };
        eprintln!(
            "  violation: code={} severity={} line={} message={}",
            v.code, sev, v.source_line, v.message,
        );
    }

    assert!(
        result.written,
        "compile must write output — see violations above",
    );
    // `directives_resolved` counts md:// URI / figure / tree resolutions.
    // A pure dashboard template (regions only, no directives inside them) is
    // valid input that legitimately produces 0 resolved directives — proof's
    // own canonical `two-region.dashboard.source.md` fixture has the same
    // shape. Don't assert >= 1 here.
    assert!(
        result
            .violations
            .iter()
            .all(|v| !matches!(v.severity, ViolationSeverity::Error)),
        "no error-level violations expected",
    );
    let rendered = std::fs::read_to_string(&out).unwrap();
    assert!(
        rendered.contains("HEADER"),
        "compiled output should contain region body"
    );
    assert!(
        rendered.contains("FOOTER"),
        "compiled output should contain second region body"
    );
}
