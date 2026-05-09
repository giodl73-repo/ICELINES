//! Persona Wave 23 — L2 smoke for the TUI filter overlay.
//!
//! The TUI is interactive and can't be driven through stdin in a
//! subprocess (no terminal is attached), so the L2 surface here is
//! deliberately narrow:
//!
//!   1. `icelines tui --help` and `icelines tui stats --help` parse
//!      cleanly through clap. Regression guard against breaking the
//!      `TuiSurface` enum or its surface-launcher dispatch when
//!      adding new modes (e.g. `QueryMode::FilterEdit`).
//!   2. The shipped `COMMANDS.md` documents the new `f` keybind.
//!      The release artifact ships this doc; if it's not in there,
//!      users have no way to discover the overlay.
//!   3. The same Phase Art Ross filter grammar reachable through the
//!      TUI overlay (via `parse_query`) is also reachable through
//!      the CLI's `--filter` flag (per the cross-surface parity
//!      contract — Wave 21). We re-exercise a representative set
//!      through `query leaders --filter` so any divergence between
//!      the two parser entry points fails the suite.

use std::path::PathBuf;
use std::process::{Command, Output};

fn icelines_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_icelines"))
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

// ── tui subcommand surface still parses ─────────────────────────

#[test]
fn p_w23_001_tui_help_runs_clean() {
    let h = fresh();
    let out = run_in(h.path(), &["tui", "--help"]);
    no_panic(&out);
    assert!(out.status.success(), "`tui --help` must succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Usage") || stdout.contains("USAGE"),
        "tui --help must print clap usage; got:\n{stdout}"
    );
}

#[test]
fn p_w23_002_tui_stats_help_runs_clean() {
    let h = fresh();
    let out = run_in(h.path(), &["tui", "stats", "--help"]);
    no_panic(&out);
    assert!(
        out.status.success(),
        "`tui stats --help` must succeed (Queries-screen launcher)"
    );
}

// ── COMMANDS.md doc regression guard ────────────────────────────

#[test]
fn p_w23_003_commands_md_documents_filter_keybind() {
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let path = workspace.join("COMMANDS.md");
    let body = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read COMMANDS.md at {path:?}: {e}"));

    // The keybind table row added in Wave 23. Use a substring
    // match that survives minor wording edits but fails if the
    // entry is dropped wholesale.
    assert!(
        body.contains("Free-form filter overlay"),
        "COMMANDS.md must document the Wave 23 filter keybind \
         (search for 'Free-form filter overlay'); not found in \
         {path:?}"
    );
    // Sanity-check the grammar examples are present somewhere too,
    // so the user has a starting point.
    assert!(
        body.contains("country IN") || body.contains("BETWEEN"),
        "COMMANDS.md must show Phase Art Ross filter examples \
         the overlay accepts"
    );
}

// ── CLI/TUI parser parity (indirect) ────────────────────────────

/// Helper: run `query leaders --filter X` and assert the binary
/// accepts the filter (no parser-bug stderr leak). Exit code may
/// indicate "no rows" — that's fine; we only care about parse
/// acceptance, the same gate the TUI overlay's `parse_query` call
/// applies.
fn assert_filter_accepted(home: &std::path::Path, filter: &str) {
    let out = run_in(home, &["query", "leaders", "--filter", filter, "--json"]);
    no_panic(&out);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("has no op"),
        "{filter:?}: legacy parser leaked through; stderr:\n{stderr}"
    );
    // The new pipeline emits a structured error we'd see on parse
    // failure. None of these filters should fail to parse.
    assert!(
        !stderr.contains("UnexpectedToken") && !stderr.contains("FeatureNotYet"),
        "{filter:?}: unexpected new-pipeline parse error; stderr:\n{stderr}"
    );
}

#[test]
fn p_w23_004_filter_country_eq_accepted() {
    let h = fresh();
    assert_filter_accepted(h.path(), "country=CAN");
}

#[test]
fn p_w23_005_filter_country_in_set_accepted() {
    let h = fresh();
    assert_filter_accepted(h.path(), "country IN (CAN, USA)");
}

#[test]
fn p_w23_006_filter_age_strict_lt_accepted() {
    let h = fresh();
    assert_filter_accepted(h.path(), "age<25");
}

#[test]
fn p_w23_007_filter_age_between_accepted() {
    let h = fresh();
    assert_filter_accepted(h.path(), "age BETWEEN 22 AND 28");
}

#[test]
fn p_w23_008_filter_country_like_accepted() {
    let h = fresh();
    assert_filter_accepted(h.path(), r#"country LIKE "CA*""#);
}

#[test]
fn p_w23_009_filter_pos_in_accepted() {
    let h = fresh();
    assert_filter_accepted(h.path(), "pos IN (C, LW, RW)");
}

#[test]
fn p_w23_010_filter_compound_accepted() {
    let h = fresh();
    assert_filter_accepted(h.path(), "country IN (CAN, USA) AND age<25");
}

#[test]
fn p_w23_011_filter_demorgan_accepted() {
    let h = fresh();
    assert_filter_accepted(h.path(), "NOT (country=CAN AND pos=C)");
}

#[test]
fn p_w23_012_filter_or_accepted() {
    let h = fresh();
    assert_filter_accepted(h.path(), "country=CAN OR country=USA");
}

#[test]
fn p_w23_013_filter_paren_grouping_accepted() {
    let h = fresh();
    assert_filter_accepted(h.path(), "(country=CAN OR country=USA) AND pos=C");
}

#[test]
fn p_w23_014_filter_kitchen_sink_accepted() {
    let h = fresh();
    assert_filter_accepted(
        h.path(),
        "country IN (CAN, USA) AND pos IN (C, LW, RW) AND age BETWEEN 22 AND 32",
    );
}
