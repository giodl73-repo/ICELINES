//! Phase Art Ross A.5 — `--explain` flag end-to-end tests.
//!
//! Subprocess L2 tests — verifies the rendered text and JSON
//! envelope shapes for `query leaders --filter X --explain`.

use std::path::PathBuf;
use std::process::Command;

fn icelines_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_icelines"))
}

fn fresh() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

fn run_in(home: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(icelines_bin())
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("ICELINES_NO_LIVE", "1")
        .env("ICELINES_TEST_MODE", "1")
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to run {args:?}: {e}"))
}

fn ok_in(home: &std::path::Path, args: &[&str]) -> String {
    let out = run_in(home, args);
    assert!(
        out.status.success(),
        "{:?} must succeed; stderr:\n{}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn fail_in(home: &std::path::Path, args: &[&str]) -> std::process::Output {
    let out = run_in(home, args);
    assert!(!out.status.success(), "{:?} must fail", args);
    out
}

#[test]
fn l2_a5_explain_simple_atom() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &["query", "leaders", "--filter", "g>=10", "--explain"],
    );
    assert!(out.contains("QUERY PLAN"));
    assert!(out.contains("explain.v1"));
    assert!(out.contains("SeasonStat(goals"));
    assert!(out.contains("DATA REQUIREMENTS"));
    assert!(out.contains("stats"));
}

#[test]
fn l2_a5_explain_compound_query() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "g>=10 AND a>=10 AND p>=20",
            "--explain",
        ],
    );
    assert!(out.contains("All"));
    assert!(out.contains("goals"));
    assert!(out.contains("assists"));
    assert!(out.contains("points"));
}

#[test]
fn l2_a5_explain_killer_query_sliding() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "g.last10g>=5 AND age<=25",
            "--explain",
        ],
    );
    assert!(out.contains("SlidingWindow"));
    assert!(out.contains("Bio(Age"));
    // Provider needed for sliding-window atoms.
    assert!(out.contains("provider"));
}

#[test]
fn l2_a5_explain_career_atom() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "p.career.junior>=200",
            "--explain",
        ],
    );
    assert!(out.contains("CareerLeague"));
}

#[test]
fn l2_a5_explain_ever_query() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "g.any10g>=5 EVER AT age<=25",
            "--explain",
        ],
    );
    assert!(out.contains("CareerAggregate"));
}

#[test]
fn l2_a5_explain_json_envelope_shape() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "g>=10",
            "--explain",
            "--json",
        ],
    );
    let v: serde_json::Value = serde_json::from_str(&out).expect("JSON envelope must parse");
    assert_eq!(v["schema_version"], "explain.v1");
    assert_eq!(v["route"], "leaders.explain");
    assert!(v["data"].is_object());
    assert!(v["data"]["plans"].is_array());
    assert!(v["meta"].is_object());
    assert_eq!(v["data"]["plans"].as_array().unwrap().len(), 1);
}

#[test]
fn l2_a5_explain_json_records_filter_input() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "g>=10 AND a>=5",
            "--explain",
            "--json",
        ],
    );
    let v: serde_json::Value = serde_json::from_str(&out).expect("parse");
    let plans = v["data"]["plans"].as_array().unwrap();
    assert_eq!(plans[0]["filter_input"], "g>=10 AND a>=5");
}

#[test]
fn l2_a5_explain_multiple_filters() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "g>=10",
            "--filter",
            "a>=10",
            "--explain",
            "--json",
        ],
    );
    let v: serde_json::Value = serde_json::from_str(&out).expect("parse");
    let plans = v["data"]["plans"].as_array().unwrap();
    assert_eq!(plans.len(), 2);
}

#[test]
fn l2_a5_explain_invalid_filter_errors_loudly() {
    let h = fresh();
    let out = fail_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "totally-fake-stat>=5",
            "--explain",
        ],
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.to_lowercase().contains("filter") || stderr.to_lowercase().contains("unknown"),
        "stderr should mention the filter parse problem; got: {stderr}"
    );
}

#[test]
fn l2_a5_explain_no_filter_arg_succeeds_with_note() {
    let h = fresh();
    let out = ok_in(h.path(), &["query", "leaders", "--explain"]);
    assert!(out.contains("no --filter"));
}

#[test]
fn l2_a5_explain_doesnt_load_player_data() {
    // A wall-clock threshold is not a reliable proxy here: all-target test
    // runs can saturate Windows with concurrent linkers and subprocesses.
    // Prove the actual invariant instead: explain must not initialize or
    // populate the isolated IceLines home directory.
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "g.any10g>=5 EVER AT age<=25 AND country IN (CAN, USA, SWE) AND league=OHL",
            "--explain",
        ],
    );
    let created = std::fs::read_dir(h.path())
        .expect("read isolated explain home")
        .collect::<Result<Vec<_>, _>>()
        .expect("inspect isolated explain home");
    assert!(
        created.is_empty(),
        "--explain must not initialize or load player-data state; created {created:?}"
    );
}

#[test]
fn l2_a5_explain_in_help() {
    let h = fresh();
    let out = ok_in(h.path(), &["query", "leaders", "--help"]);
    assert!(out.contains("--explain"));
}
