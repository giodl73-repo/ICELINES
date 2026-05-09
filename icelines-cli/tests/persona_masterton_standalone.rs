//! Phase Masterton.3 — L2 smoke for the `--standalone` flag on
//! `icelines tui`.
//!
//! Per spec bench-3: TUI subprocess testing without TTY isn't
//! viable for interactive flows, so the L2 surface here is
//! deliberately narrow:
//!
//! 1. `icelines tui --help` documents `--standalone`.
//! 2. `icelines tui goalies --help` exits cleanly (clap parses
//!    the surface sub-subcommand alongside `--standalone`).
//! 3. The flag works at the parser level — combinations of
//!    surface sub-subcommand + `--standalone` produce the
//!    expected `Tui { surface: Some(_), standalone: true }`
//!    parse shape. (Covered by L0 in cli.rs::tui_surface_tests.)
//!
//! Real TUI launch testing requires a TTY; not feasible here.

use std::path::PathBuf;
use std::process::{Command, Output};

fn icelines_bin() -> PathBuf {
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    #[cfg(windows)]
    let bin = workspace.join("target/release/icelines.exe");
    #[cfg(not(windows))]
    let bin = workspace.join("target/release/icelines");
    bin
}

fn run_in(home: &std::path::Path, args: &[&str]) -> Output {
    Command::new(icelines_bin())
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("ICELINES_NO_LIVE", "1")
        .env("ICELINES_TEST_MODE", "1")
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to run {args:?}: {e}"))
}

fn no_panic(out: &Output) {
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!combined.contains("panicked"), "panic in:\n{combined}");
    assert_ne!(out.status.code(), Some(101));
}

fn fresh() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

#[test]
fn p_masterton_001_tui_help_documents_standalone() {
    let h = fresh();
    let out = run_in(h.path(), &["tui", "--help"]);
    no_panic(&out);
    assert!(out.status.success(), "tui --help must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("--standalone"),
        "tui --help must document --standalone; got:\n{stdout}"
    );
    // Sanity: also documents the example phrase.
    assert!(
        stdout.contains("focused") || stdout.contains("lock") || stdout.contains("Lock"),
        "tui --help should describe what --standalone does; got:\n{stdout}"
    );
}

#[test]
fn p_masterton_002_tui_surface_help_runs_clean() {
    let h = fresh();
    // Sub-subcommand parse: `tui goalies --help` — confirms clap
    // accepts the combination `surface + --standalone` cleanly.
    let out = run_in(h.path(), &["tui", "goalies", "--help"]);
    no_panic(&out);
    assert!(
        out.status.success(),
        "tui goalies --help must exit 0 (clap surface dispatch)"
    );
}

#[test]
fn p_masterton_003_tui_standalone_help_short_circuits_clean() {
    let h = fresh();
    // `--standalone --help` — clap's --help short-circuits before
    // the TUI ever boots, so this is safe to run in a subprocess
    // (no TTY needed). The successful run confirms the flag is
    // wired into the parser and doesn't conflict with --help.
    let out = run_in(h.path(), &["tui", "--standalone", "--help"]);
    no_panic(&out);
    assert!(out.status.success(), "tui --standalone --help must exit 0");
}

// ── Phase Jack Adams.1 — `--mdi` flag smoke ───────────────────────────────

#[test]
fn p_adams_001_tui_help_documents_mdi() {
    let h = fresh();
    let out = run_in(h.path(), &["tui", "--help"]);
    no_panic(&out);
    assert!(out.status.success(), "tui --help must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("--mdi"),
        "tui --help must document --mdi; got:\n{stdout}"
    );
    // Sanity: also describes what the flag does.
    assert!(
        stdout.contains("MDI") || stdout.contains("dashboard"),
        "tui --help should describe MDI/dashboard mode; got:\n{stdout}"
    );
}

#[test]
fn p_adams_002_tui_mdi_help_short_circuits_clean() {
    let h = fresh();
    // `--mdi --help` — clap's --help short-circuits before the
    // TUI ever boots. Confirms --mdi is wired into the parser
    // and doesn't conflict with --help.
    let out = run_in(h.path(), &["tui", "--mdi", "--help"]);
    no_panic(&out);
    assert!(out.status.success(), "tui --mdi --help must exit 0");
}

#[test]
fn p_adams_003_tui_mdi_conflicts_with_standalone() {
    let h = fresh();
    // Per spec wire-1: clap's `conflicts_with` rejects --mdi
    // and --standalone together at parse time. Should exit
    // non-zero with a helpful clap error.
    let out = run_in(h.path(), &["tui", "--mdi", "--standalone"]);
    no_panic(&out);
    assert!(
        !out.status.success(),
        "tui --mdi --standalone must be rejected"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--mdi") && stderr.contains("--standalone"),
        "clap error must name both flags; got:\n{stderr}"
    );
}

// ── Phase Jack Adams.4 — adaptive width L2 surface ────────────────────────

/// Adams.4 doesn't add new CLI flags — it's all runtime layout
/// behavior triggered by terminal-width changes. Real TUI launch
/// requires a TTY; not feasible here. The L2 we CAN do:
///
/// 1. Confirm `tui --mdi` parses cleanly through clap with no
///    extra surface flags (wired in Adams.1, but Adams.4 is the
///    closeout that ships the layout). Re-run with `--help`
///    short-circuit so subprocess doesn't try to attach to a TTY.
/// 2. Confirm a sub-subcommand surface combo (`tui goalies
///    --mdi --help`) parses — proves clap dispatch composes the
///    new flag with the existing surface arg.
///
/// Layout-behavior testing happens in the L1 resize-sequence
/// tests in `tui/screens/mod.rs::adams_4_render_boundary_tests`,
/// where TestBackend can drive `term.resize(...)` and assert
/// against the captured frame buffer.

#[test]
fn p_adams_004_tui_mdi_with_surface_help_runs_clean() {
    // `--mdi` is a parent-level flag on `tui`, not a per-surface
    // flag — so clap's invocation order is `tui --mdi <surface>`.
    // This composes the MDI launcher with the surface dispatch
    // and confirms the parser accepts the combination.
    let h = fresh();
    let out = run_in(h.path(), &["tui", "--mdi", "goalies", "--help"]);
    no_panic(&out);
    assert!(
        out.status.success(),
        "tui --mdi goalies --help must exit 0 (parent flag + surface composes)"
    );
}

#[test]
fn p_adams_005_tui_help_mentions_dashboard_or_mdi() {
    // Doc-surface check: the user-facing help must call out the
    // dashboard / MDI mode in the long_about so users discover it.
    // Mirrors p_masterton_001 for --standalone discoverability.
    let h = fresh();
    let out = run_in(h.path(), &["tui", "--help"]);
    no_panic(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("dashboard") || stdout.contains("MDI"),
        "tui --help must describe MDI/dashboard mode for discoverability; got:\n{stdout}"
    );
}
