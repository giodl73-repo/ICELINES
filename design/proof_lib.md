# Consuming `proof_lib` from icelines

> **Superseded integration mode:** ICELINES no longer links `proof_lib`,
> even as a dev-dependency. Keep this note only as historical API context
> if a future site/dashboard generator shells out to PROOF or consumes
> PROOF-generated artifacts. Do not re-add `proof_lib` as an ICELINES
> runtime dependency unless ICELINES directly embeds PROOF APIs.

Hand-off note from the PROOF maintainer. Read this first; it answers the
"how do I call proof from icelines for dashboard spec rendering" question end
to end so you don't have to reverse-engineer the crate.

> **Source repo:** `C:\src\PROOF` (also at `https://github.com/giodl73-repo/PROOF`)
> **Pin reference:** commit `e125362` on `master`, post-v0.6 / pre-v1.0
> **Status:** API surface is usable but not frozen — pin by SHA, not branch.

---

## TL;DR

1. Add a path or git dep on `proof_lib` (the library half of the PROOF crate).
2. Call `proof_lib::compile::compile_file(src, out, root, &GlintConfig::default())`.
3. Dashboard specs (`*.dashboard.source.md`) flow through that same call —
   `compile_file` internally routes to the dashboard compositor based on the
   filename suffix. There is no separate dashboard entry point.
4. Inspect `CompileResult { directives_resolved, violations, written, ... }`.

That's the whole library API for compile. Everything else (slides, charts,
trees, tables) is reachable through the same `compile_file` call.

---

## Cargo dependency

The PROOF crate ships two artifacts from one package:

```toml
[package] name = "proof"             # the CLI binary
[lib]     name = "proof_lib"         # the library — what you import
```

icelines pins by SHA in the committed `Cargo.toml`:

```toml
proof_lib = { package = "proof",
              git = "https://github.com/giodl73-repo/PROOF",
              rev = "<sha>" }
```

This makes the build work in CI (which only checks out icelines) and on
fresh clones — cargo fetches proof from git, which transitively fetches
mdpath from git.

**Local-iteration override.** When you have proof and/or mdpath checked
out as siblings and want to skip the commit-push-test cycle, drop a
`.cargo/config.toml` in the icelines workspace root with cargo's `[patch]`
mechanism:

```toml
[patch."https://github.com/giodl73-repo/PROOF"]
proof = { path = "../proof" }

[patch."https://github.com/giodl73-repo/MDPATH"]
mdpath = { path = "../mdpath" }
```

The full template is at `.cargo/config.toml.example`. The real
`.cargo/config.toml` is gitignored so each developer manages their own
override and CI runners stay clean.

**Pin by SHA, not branch.** v0.5 → v0.6 just shipped breaking changes;
the API isn't 1.0 yet. Floating on `master` will surprise you.

---

## Minimum viable smoke test

A single integration test proves the library link works. Drop this into
`icelines/tests/proof_lib_smoke.rs`:

```rust
use proof_lib::compile::{compile_file, ViolationSeverity};
use proof_lib::GlintConfig;

#[test]
fn proof_compiles_a_dashboard_spec() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("hello.dashboard.source.md");
    let out = dir.path().join("hello.md");

    // Minimal dashboard fixture. Adjust to match what icelines actually feeds in.
    std::fs::write(&src, "\
# Hello dashboard

```proof:region name=summary cols=80 rows=10
- value=42 label=\"Items\"
```\n").unwrap();

    let cfg = GlintConfig::default();
    let result = compile_file(&src, &out, dir.path(), &cfg).unwrap();

    assert!(result.written, "compile must write output");
    assert!(result.directives_resolved >= 1, "expected at least one resolved directive");
    assert!(
        result.violations.iter().all(|v| !matches!(v.severity, ViolationSeverity::Error)),
        "no error-level violations expected: {:?}",
        result.violations.iter().map(|v| (v.code, &v.message)).collect::<Vec<_>>(),
    );
    assert!(std::fs::read_to_string(&out).unwrap().contains("Hello dashboard"));
}
```

This is the same pattern PROOF's own integration tests use — see
`tests/features_integration.rs:41-51` for the canonical helper.

---

## API surface you'll actually touch

```rust
use proof_lib::GlintConfig;
use proof_lib::compile::{compile_file, CompileResult, CompileViolation, ViolationSeverity};
```

### `compile_file` — the one entry point

```rust
pub fn compile_file(
    source_path: &Path,   // input, must end in .source.md (or .dashboard.source.md / .slides.source.md)
    output_path: &Path,   // where to write the compiled .md
    root: &Path,          // project root — md:// URIs resolve relative to this
    config: &GlintConfig, // default is fine for most cases; controls lint rules
) -> anyhow::Result<CompileResult>
```

Returns:

```rust
pub struct CompileResult {
    pub output_path: PathBuf,
    pub directives_resolved: usize,   // count of proof: directives successfully rendered
    pub violations: Vec<CompileViolation>,  // both warnings and errors
    pub from_cache: bool,
    pub written: bool,                // false when there were errors → output not written
    pub resolved_files: Vec<PathBuf>, // for watch-mode dependency tracking
}

pub struct CompileViolation {
    pub code: &'static str,           // e.g. "COMPILE-002", "TREE-001"
    pub severity: ViolationSeverity,  // Error | Warning
    pub uri: String,                  // md:// URI involved (may be empty)
    pub figure_id: Option<String>,
    pub invariant: String,
    pub message: String,
    pub source_line: usize,           // 1-based line in the source file
}
```

### Filename routing

`compile_file` dispatches by suffix:

| Filename suffix              | Compositor                |
|------------------------------|---------------------------|
| `*.dashboard.source.md`      | dashboard region layout   |
| `*.slides.source.md`         | slide deck                |
| `*.source.md`                | regular markdown directives |

For dashboards, write the source as `*.dashboard.source.md` and call
`compile_file` exactly as above — there is no separate
`compile_dashboard_file` to call directly from outside the crate.

---

## What's NOT public yet (and may move)

These work internally but aren't part of the supported library surface:

- `compile_str(&str) -> Result<String>` — there is no string-in/string-out
  convenience function. If icelines needs to skip the disk roundtrip, write
  to a `tempfile::NamedTempFile` (see the smoke test above). Tell us if this
  pain shows up; we'll add `compile_str` once there's a real consumer.
- Direct dashboard / slide compositor entry points — public within the crate
  but not re-exported.
- Anything in `proof_lib::checks::*`, `proof_lib::draft`, `proof_lib::fix`,
  `proof_lib::runner` — usable but undocumented; expect churn.

If you find yourself reaching past `compile::compile_file`, ping us before
building on it.

---

## Reference docs in the PROOF repo

- `design/DASHBOARD-SPEC.md` — the dashboard directive language icelines is rendering.
- `design/TREE-SPEC.md`, `design/SLIDE-SPEC.md`, `design/MAPPING-SPEC.md` — sister specs.
- `tests/features_integration.rs` — canonical integration test patterns.
- `CHANGELOG.md` — what landed in v0.5 and v0.6.

---

## Suggested next step

Once this smoke test goes green in icelines, plan the rest of Phase 8d in
earnest — at that point we know the library link works, and the question
narrows to which directives icelines actually needs and which lint diagnostics
matter.

If you hit a snag (missing API, broken compile, surprising violation), file
an issue on https://github.com/giodl73-repo/PROOF/issues with the source
fixture and the `CompileResult` you got. We've been responsive — three bugs
filed today were fixed the same afternoon.
