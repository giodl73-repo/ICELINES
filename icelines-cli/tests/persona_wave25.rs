//! Persona Wave 25 — L2 smoke for `query career --filter`.
//!
//! Subprocess-driven coverage of the new `--filter` flag on
//! `icelines query career`. The cohort store at
//! `~/.icelines/career_history.json` is populated only when the
//! user has run `icelines fetch career`; the binary handles the
//! empty-store case with a helpful error. These tests assert
//! that:
//!
//!   1. `--help` mentions the new flag (regression guard against
//!      silently dropping it from clap).
//!   2. The binary accepts every supported filter shape via
//!      `--filter` and exits without panicking — same indirect
//!      parser-parity proof Wave 23 used for the TUI surface.
//!   3. Bad filter syntax fails fast with a parse-error message,
//!      not a panic.

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

// ── --help mentions the new flag ────────────────────────────────

#[test]
fn p_w25_001_career_help_documents_filter_flag() {
    let h = fresh();
    let out = run_in(h.path(), &["query", "career", "--help"]);
    no_panic(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("--filter"),
        "query career --help must document --filter; got:\n{stdout}"
    );
    // Bio examples should be cited so the user knows what works.
    assert!(
        stdout.contains("country") || stdout.contains("pos="),
        "query career --help should cite at least one bio-atom \
         example for --filter; got:\n{stdout}"
    );
}

// ── Filter acceptance (indirect parser parity) ──────────────────
//
// These tests assert the binary doesn't panic and doesn't emit a
// parser-bug error. The `~/.icelines/career_history.json` store is
// empty in a fresh tempdir, so the command exits with a "store is
// empty" message — that's expected and not a parser bug. The fence
// is: parse the filter cleanly, reach the empty-store branch.

fn assert_filter_accepted(home: &std::path::Path, filter: &str) {
    let out = run_in(
        home,
        &[
            "query", "career", "--league", "OHL", "--filter", filter, "--json",
        ],
    );
    no_panic(&out);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("filter parse error"),
        "{filter:?}: parser rejected a valid filter; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("UnexpectedToken") && !stderr.contains("FeatureNotYet"),
        "{filter:?}: unexpected new-pipeline parse error; stderr:\n{stderr}"
    );
}

#[test]
fn p_w25_002_filter_country_eq_accepted() {
    let h = fresh();
    assert_filter_accepted(h.path(), "country=CAN");
}

#[test]
fn p_w25_003_filter_country_in_accepted() {
    let h = fresh();
    assert_filter_accepted(h.path(), "country IN (CAN, USA)");
}

#[test]
fn p_w25_004_filter_pos_in_accepted() {
    let h = fresh();
    assert_filter_accepted(h.path(), "pos IN (C, LW, RW)");
}

#[test]
fn p_w25_005_filter_age_at_cohort_year_accepted() {
    let h = fresh();
    assert_filter_accepted(h.path(), "age<=18");
}

#[test]
fn p_w25_006_filter_draft_round_accepted() {
    let h = fresh();
    assert_filter_accepted(h.path(), "draft-round<=2");
}

#[test]
fn p_w25_007_filter_compound_accepted() {
    let h = fresh();
    assert_filter_accepted(h.path(), "country=CAN AND pos=C AND age<=18");
}

#[test]
fn p_w25_008_filter_or_accepted() {
    let h = fresh();
    assert_filter_accepted(h.path(), "country=CAN OR country=USA");
}

// ── Bad filter syntax fails fast with parser error ──────────────

#[test]
fn p_w25_009_unparsed_filter_fails_with_parse_error() {
    let h = fresh();
    let out = run_in(
        h.path(),
        &["query", "career", "--league", "OHL", "--filter", "((("],
    );
    no_panic(&out);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("filter parse error") || stderr.contains("error:"),
        "unparsed filter must surface a parse-error message; got:\n{stderr}"
    );
    // Non-zero exit (anyhow's default).
    assert_ne!(out.status.code(), Some(0), "must exit non-zero");
}

// ── Multiple --filter flags AND-join ────────────────────────────

#[test]
fn p_w25_010_multiple_filters_and_join_accepted() {
    let h = fresh();
    let out = run_in(
        h.path(),
        &[
            "query",
            "career",
            "--league",
            "OHL",
            "--filter",
            "country=CAN",
            "--filter",
            "pos=C",
            "--filter",
            "age<=18",
        ],
    );
    no_panic(&out);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("filter parse error"),
        "three AND-joined filters must parse cleanly; stderr:\n{stderr}"
    );
}
