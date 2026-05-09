//! Persona Wave 20 — cohort-filter coverage for `query player`
//! and `query compare`.
//!
//! These two subcommands accept `--filter` for narrowing the
//! peers/similarity cohort. Pre-Wave-20, both used legacy-only
//! parse_filter_expr — the same dispatch bug as Waves 16-19.
//! Wave 20 covers the fix.

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

fn assert_runs_clean(home: &std::path::Path, args: &[&str]) {
    let out = run_in(home, args);
    // For these subcommands, "fail" can mean "player not found"
    // which is OK; we only care about no-panic + no parser-bug.
    no_panic(&out);
    let stderr = String::from_utf8_lossy(&out.stderr);
    // Parser bugs surface as legacy "no op" errors on new
    // grammar — assert that's NOT the failure mode.
    assert!(
        !stderr.contains("has no op"),
        "{:?}: legacy parser leaked through; stderr:\n{stderr}",
        args
    );
}

fn fresh() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

// ── query player --peers --filter (new grammar) ──────────────

#[test]
fn p_w20_001_player_peers_country_in() {
    let h = fresh();
    assert_runs_clean(
        h.path(),
        &[
            "query",
            "player",
            "Connor McDavid",
            "--peers",
            "10",
            "--filter",
            "country IN (CAN, USA)",
        ],
    );
}

#[test]
fn p_w20_002_player_peers_strict_age() {
    let h = fresh();
    assert_runs_clean(
        h.path(),
        &[
            "query",
            "player",
            "Connor McDavid",
            "--peers",
            "10",
            "--filter",
            "age<30",
        ],
    );
}

#[test]
fn p_w20_003_player_peers_age_between() {
    let h = fresh();
    assert_runs_clean(
        h.path(),
        &[
            "query",
            "player",
            "Connor McDavid",
            "--peers",
            "10",
            "--filter",
            "age BETWEEN 25 AND 32",
        ],
    );
}

#[test]
fn p_w20_004_player_peers_country_like() {
    let h = fresh();
    assert_runs_clean(
        h.path(),
        &[
            "query",
            "player",
            "Connor McDavid",
            "--peers",
            "10",
            "--filter",
            r#"country LIKE "CA*""#,
        ],
    );
}

#[test]
fn p_w20_005_player_peers_pos_in() {
    let h = fresh();
    assert_runs_clean(
        h.path(),
        &[
            "query",
            "player",
            "Connor McDavid",
            "--peers",
            "10",
            "--filter",
            "pos IN (C, LW, RW)",
        ],
    );
}

#[test]
fn p_w20_006_player_peers_country_ne() {
    let h = fresh();
    assert_runs_clean(
        h.path(),
        &[
            "query",
            "player",
            "Connor McDavid",
            "--peers",
            "10",
            "--filter",
            "country!=RUS",
        ],
    );
}

#[test]
fn p_w20_007_player_peers_compound_new_grammar() {
    let h = fresh();
    assert_runs_clean(
        h.path(),
        &[
            "query",
            "player",
            "Connor McDavid",
            "--peers",
            "10",
            "--filter",
            "country IN (CAN, USA) AND age<30",
        ],
    );
}

#[test]
fn p_w20_008_player_peers_kitchen_sink() {
    let h = fresh();
    assert_runs_clean(
        h.path(),
        &[
            "query",
            "player",
            "Connor McDavid",
            "--peers",
            "10",
            "--filter",
            "country IN (CAN, USA) AND age BETWEEN 22 AND 32 AND pos=C AND draft-round<=2",
        ],
    );
}

#[test]
fn p_w20_009_player_peers_legacy_still_works() {
    // Pre-existing legacy filter shape — must still work.
    let h = fresh();
    assert_runs_clean(
        h.path(),
        &[
            "query",
            "player",
            "Connor McDavid",
            "--peers",
            "10",
            "--filter",
            "g>=10",
        ],
    );
}

#[test]
fn p_w20_010_player_peers_strict_lt_compound() {
    let h = fresh();
    assert_runs_clean(
        h.path(),
        &[
            "query",
            "player",
            "Connor McDavid",
            "--peers",
            "10",
            "--filter",
            "age<30 AND p>=20",
        ],
    );
}

// ── query compare --similar --filter (new grammar) ───────────

#[test]
fn p_w20_011_compare_similar_country_in() {
    let h = fresh();
    assert_runs_clean(
        h.path(),
        &[
            "query",
            "compare",
            "Connor McDavid",
            "--similar",
            "5",
            "--filter",
            "country IN (CAN, USA)",
        ],
    );
}

#[test]
fn p_w20_012_compare_similar_strict_age() {
    let h = fresh();
    assert_runs_clean(
        h.path(),
        &[
            "query",
            "compare",
            "Connor McDavid",
            "--similar",
            "5",
            "--filter",
            "age<30",
        ],
    );
}

#[test]
fn p_w20_013_compare_similar_between() {
    let h = fresh();
    assert_runs_clean(
        h.path(),
        &[
            "query",
            "compare",
            "Connor McDavid",
            "--similar",
            "5",
            "--filter",
            "age BETWEEN 25 AND 32",
        ],
    );
}

#[test]
fn p_w20_014_compare_similar_like() {
    let h = fresh();
    assert_runs_clean(
        h.path(),
        &[
            "query",
            "compare",
            "Connor McDavid",
            "--similar",
            "5",
            "--filter",
            r#"country LIKE "CA*""#,
        ],
    );
}

#[test]
fn p_w20_015_compare_similar_pos_in() {
    let h = fresh();
    assert_runs_clean(
        h.path(),
        &[
            "query",
            "compare",
            "Connor McDavid",
            "--similar",
            "5",
            "--filter",
            "pos IN (C, LW, RW)",
        ],
    );
}

#[test]
fn p_w20_016_compare_similar_compound() {
    let h = fresh();
    assert_runs_clean(
        h.path(),
        &[
            "query",
            "compare",
            "Connor McDavid",
            "--similar",
            "5",
            "--filter",
            "country IN (CAN, USA) AND age<30",
        ],
    );
}

#[test]
fn p_w20_017_compare_similar_kitchen_sink() {
    let h = fresh();
    assert_runs_clean(
        h.path(),
        &[
            "query",
            "compare",
            "Connor McDavid",
            "--similar",
            "5",
            "--filter",
            "country IN (CAN, USA) AND age BETWEEN 22 AND 32 AND pos=C",
        ],
    );
}

#[test]
fn p_w20_018_compare_similar_legacy_still_works() {
    let h = fresh();
    assert_runs_clean(
        h.path(),
        &[
            "query",
            "compare",
            "Connor McDavid",
            "--similar",
            "5",
            "--filter",
            "g>=10",
        ],
    );
}

#[test]
fn p_w20_019_compare_similar_country_ne() {
    let h = fresh();
    assert_runs_clean(
        h.path(),
        &[
            "query",
            "compare",
            "Connor McDavid",
            "--similar",
            "5",
            "--filter",
            "country!=RUS",
        ],
    );
}

#[test]
fn p_w20_020_compare_similar_paren_compound() {
    let h = fresh();
    assert_runs_clean(
        h.path(),
        &[
            "query",
            "compare",
            "Connor McDavid",
            "--similar",
            "5",
            "--filter",
            "(country=CAN OR country=USA) AND age<=30",
        ],
    );
}
