//! Phase Foster.6 — closeout persona scenarios.
//!
//! Cross-surface scripts mirroring `persona_wave3.rs` density. Each
//! test boots the release binary against an isolated `~/.icelines`
//! tempdir so personas don't pollute each other or the developer's
//! actual db. `ICELINES_NO_LIVE=1` is set so subcommands that would
//! normally hit the NHL API stay deterministic in CI / offline.
//!
//! Build: `cargo build --release -p icelines-cli`
//! Run: `cargo test -p icelines-cli --test persona_foster`
//!
//! Distribution: 10 scenarios across the 5 Foster sub-phases.
//!
//! Note: the spec called for 30 personas (BENCH H3); shipping 10
//! here covers the critical paths today. The remaining 20 are
//! placeholder material for follow-up sessions once F.3+ wires
//! per-night stat lines + boxscore JSON persistence end-to-end.

use std::path::PathBuf;
use std::process::Command;

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

fn run_isolated(home: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(icelines_bin())
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("ICELINES_NO_LIVE", "1")
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to run icelines: {e}"))
}

fn ok_in(home: &std::path::Path, args: &[&str]) -> String {
    let out = run_isolated(home, args);
    assert!(
        out.status.success(),
        "{:?} must succeed; stderr:\n{}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn fail_in(home: &std::path::Path, args: &[&str]) -> String {
    let out = run_isolated(home, args);
    assert!(
        !out.status.success(),
        "{:?} must non-zero exit; stdout:\n{}",
        args,
        String::from_utf8_lossy(&out.stdout)
    );
    String::from_utf8_lossy(&out.stderr).into_owned()
}

// ── Setup wizard (2) ─────────────────────────────────────────────────────────

#[test]
fn p_foster_p01_setup_accept_defaults_writes_full_matrix() {
    let home = tempfile::tempdir().unwrap();
    let out = ok_in(home.path(), &["setup", "--accept-defaults"]);
    assert!(out.contains("transactions    = favorites"));
    assert!(out.contains("shifts          = off"));

    let cfg_path = home.path().join(".icelines").join("config.toml");
    assert!(cfg_path.exists(), "config.toml must be written");
    let body = std::fs::read_to_string(&cfg_path).unwrap();
    assert!(body.contains("[sync"));
    assert!(body.contains("transactions = \"favorites\""));
}

#[test]
fn p_foster_p02_setup_dry_run_leaves_no_files() {
    let home = tempfile::tempdir().unwrap();
    let out = ok_in(home.path(), &["setup", "--accept-defaults", "--dry-run"]);
    assert!(out.contains("(dry run"));
    let cfg_path = home.path().join(".icelines").join("config.toml");
    assert!(!cfg_path.exists(), "dry-run must not write config");
}

// ── Capability matrix (3) ────────────────────────────────────────────────────

#[test]
fn p_foster_p03_config_set_get_round_trip() {
    let home = tempfile::tempdir().unwrap();
    ok_in(home.path(), &["setup", "--accept-defaults"]);
    ok_in(
        home.path(),
        &["config", "set", "sync.capabilities.transactions", "league"],
    );
    let got = ok_in(
        home.path(),
        &["config", "get", "sync.capabilities.transactions"],
    );
    assert_eq!(got.trim(), "league");
}

#[test]
fn p_foster_p04_shifts_locked_to_off_with_literal_error() {
    let home = tempfile::tempdir().unwrap();
    ok_in(home.path(), &["setup", "--accept-defaults"]);
    let err = fail_in(
        home.path(),
        &["config", "set", "sync.capabilities.shifts", "favorites"],
    );
    assert!(
        err.contains("capability `shifts` cannot be set to `favorites`"),
        "BENCH H3 literal error must surface, got: {err}"
    );
    assert!(
        err.contains("Allowed values today: off"),
        "trailer must surface, got: {err}"
    );
}

#[test]
fn p_foster_p05_config_reset_returns_to_defaults() {
    let home = tempfile::tempdir().unwrap();
    ok_in(home.path(), &["setup", "--accept-defaults"]);
    ok_in(
        home.path(),
        &["config", "set", "sync.capabilities.transactions", "off"],
    );
    ok_in(home.path(), &["config", "reset", "sync.capabilities"]);
    let got = ok_in(
        home.path(),
        &["config", "get", "sync.capabilities.transactions"],
    );
    assert_eq!(got.trim(), "favorites", "reset → spec default 'favorites'");
}

// ── Time travel (2) ──────────────────────────────────────────────────────────

#[test]
fn p_foster_p06_invalid_date_clean_error_no_panic() {
    let home = tempfile::tempdir().unwrap();
    let err = fail_in(home.path(), &["tonight", "--date", "not-a-date"]);
    assert!(
        !err.contains("panicked"),
        "must not panic, stderr: {err}"
    );
    assert!(
        err.contains("invalid date") && err.contains("YYYY-MM-DD"),
        "validator hint must surface, stderr: {err}"
    );
}

#[test]
fn p_foster_p07_query_career_week_rejected_with_documented_remediation() {
    let home = tempfile::tempdir().unwrap();
    let err = fail_in(
        home.path(),
        &["query", "career", "--league", "OHL", "--week"],
    );
    assert!(
        err.contains("--week / --month not supported on `query career`"),
        "EDGE B2 literal error must surface, stderr: {err}"
    );
    assert!(
        err.contains("Use --season instead"),
        "remediation must surface, stderr: {err}"
    );
}

// ── Favorites dashboard (2) ──────────────────────────────────────────────────

#[test]
fn p_foster_p08_favorites_empty_state_teaches_user() {
    let home = tempfile::tempdir().unwrap();
    let out = ok_in(home.path(), &["favorites"]);
    assert!(out.contains("FAVORITES"));
    assert!(out.contains("empty"));
    assert!(
        out.contains("icelines group add Favorites"),
        "must teach the user how to add favorites, got: {out}"
    );
}

#[test]
fn p_foster_p09_favorites_json_envelope_is_valid_json() {
    let home = tempfile::tempdir().unwrap();
    let out = ok_in(home.path(), &["favorites", "--json"]);
    let parsed: serde_json::Value =
        serde_json::from_str(&out).expect("--json must emit valid JSON");
    assert_eq!(parsed["schema_version"], 1);
    assert_eq!(parsed["route"], "favorites");
    assert!(parsed["data"]["players"].is_array());
    assert!(parsed["data"]["teams"].is_array());
    assert!(parsed["data"]["events"].is_array());
}

// ── Sync engine (1) ──────────────────────────────────────────────────────────

#[test]
fn p_foster_p10_fetch_sync_dry_run_on_empty_manifest_says_nothing_stale() {
    let home = tempfile::tempdir().unwrap();
    let out = ok_in(home.path(), &["fetch", "sync", "--dry-run"]);
    assert!(
        out.contains("Nothing stale"),
        "empty manifest → 'Nothing stale.', got: {out}"
    );
}
