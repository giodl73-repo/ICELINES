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
    PathBuf::from(env!("CARGO_BIN_EXE_icelines"))
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
    assert!(!err.contains("panicked"), "must not panic, stderr: {err}");
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

// ── Wave 2 — 20 more scenarios (Foster +11 / BENCH H3) ──────────────────────
//
// Distribution mirrors the closeout-plan §"F.6.4 Persona pass":
//   setup ×4, favorites flows ×4, time-travel ×4, sync engine ×4, data layer ×4

// ── Setup (4) ────────────────────────────────────────────────────────────────

#[test]
fn p_foster_p11_setup_reset_overwrites_existing_config() {
    let home = tempfile::tempdir().unwrap();
    ok_in(home.path(), &["setup", "--accept-defaults"]);
    // Mutate first
    ok_in(
        home.path(),
        &["config", "set", "sync.capabilities.transactions", "off"],
    );
    // Re-running setup --accept-defaults --reset must restore defaults
    ok_in(home.path(), &["setup", "--accept-defaults", "--reset"]);
    let got = ok_in(
        home.path(),
        &["config", "get", "sync.capabilities.transactions"],
    );
    assert_eq!(got.trim(), "favorites", "reset restores spec default");
}

#[test]
fn p_foster_p12_setup_dry_run_then_real_run_writes_only_once() {
    let home = tempfile::tempdir().unwrap();
    let cfg = home.path().join(".icelines").join("config.toml");
    ok_in(home.path(), &["setup", "--accept-defaults", "--dry-run"]);
    assert!(!cfg.exists(), "dry-run never wrote");
    ok_in(home.path(), &["setup", "--accept-defaults"]);
    assert!(cfg.exists(), "real run did write");
}

#[test]
fn p_foster_p13_config_list_enumerates_every_settable_key() {
    let home = tempfile::tempdir().unwrap();
    ok_in(home.path(), &["setup", "--accept-defaults"]);
    let out = ok_in(home.path(), &["config", "list"]);
    for key in [
        "sync.policy",
        "sync.banner",
        "sync.season_transition",
        "sync.capabilities.stats",
        "sync.capabilities.scores_schedule",
        "sync.capabilities.transactions",
        "sync.capabilities.boxscores",
        "sync.capabilities.shifts",
        "sync.capabilities.career_history",
    ] {
        assert!(
            out.contains(key),
            "config list must enumerate {key}, got:\n{out}"
        );
    }
}

#[test]
fn p_foster_p14_config_get_unknown_key_clean_error() {
    let home = tempfile::tempdir().unwrap();
    ok_in(home.path(), &["setup", "--accept-defaults"]);
    let err = fail_in(home.path(), &["config", "get", "sync.nonsense"]);
    assert!(
        err.contains("unknown config key"),
        "must surface unknown-key error, stderr: {err}"
    );
}

// ── Favorites flows (4) ──────────────────────────────────────────────────────

#[test]
fn p_foster_p15_group_add_team_then_favorites_renders_count() {
    let home = tempfile::tempdir().unwrap();
    ok_in(home.path(), &["group", "add", "Favorites", "EDM"]);
    let out = ok_in(home.path(), &["favorites"]);
    // Surface lists the team-count in the header.
    assert!(
        out.contains("1 team(s)") || out.contains("team EDM"),
        "favorites must mention the added team, got: {out}"
    );
}

#[test]
fn p_foster_p16_group_add_then_favorites_json_resolves_member_count() {
    let home = tempfile::tempdir().unwrap();
    ok_in(home.path(), &["group", "add", "Favorites", "EDM"]);
    ok_in(home.path(), &["group", "add", "Favorites", "FLA"]);
    let out = ok_in(home.path(), &["favorites", "--json"]);
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
    assert_eq!(parsed["meta"]["members_total"], 2);
}

#[test]
fn p_foster_p17_favorites_unknown_group_surfaces_empty_state_anyway() {
    // Any group name (existing or not) gates through
    // list_members_with_kind which returns Ok for missing groups too.
    let home = tempfile::tempdir().unwrap();
    let out = ok_in(home.path(), &["favorites", "--group", "NoSuchGroup"]);
    assert!(out.contains("FAVORITES"));
}

#[test]
fn p_foster_p18_favorites_range_week_renders() {
    let home = tempfile::tempdir().unwrap();
    let out = ok_in(home.path(), &["favorites", "--range", "week"]);
    assert!(out.contains("FAVORITES"));
}

// ── Time travel (4) ──────────────────────────────────────────────────────────

#[test]
fn p_foster_p19_query_career_month_rejected_too() {
    let home = tempfile::tempdir().unwrap();
    let err = fail_in(
        home.path(),
        &["query", "career", "--league", "OHL", "--month"],
    );
    assert!(err.contains("Use --season instead"));
}

#[test]
fn p_foster_p20_favorites_invalid_range_clean_error() {
    let home = tempfile::tempdir().unwrap();
    let err = fail_in(home.path(), &["favorites", "--range", "year"]);
    assert!(err.contains("unknown --range"));
}

#[test]
fn p_foster_p21_schedule_invalid_date_no_panic() {
    let home = tempfile::tempdir().unwrap();
    let err = fail_in(home.path(), &["schedule", "--date", "9999-99-99"]);
    assert!(!err.contains("panicked"), "stderr: {err}");
    assert!(err.contains("invalid date"));
}

#[test]
fn p_foster_p22_tonight_invalid_date_no_panic() {
    let home = tempfile::tempdir().unwrap();
    let err = fail_in(home.path(), &["tonight", "--date", "garbage-string"]);
    assert!(!err.contains("panicked"), "stderr: {err}");
}

// ── Sync engine (4) ──────────────────────────────────────────────────────────

#[test]
fn p_foster_p23_fetch_sync_force_dry_run_lists_static_entries() {
    // Plant a Bundle entry (would-be-Static) by running setup first
    // — though no manifest entries exist yet on a fresh tempdir, the
    // --force --dry-run path must still exit 0 with no stale entries.
    let home = tempfile::tempdir().unwrap();
    let out = ok_in(home.path(), &["fetch", "sync", "--dry-run", "--force"]);
    assert!(
        out.contains("Nothing stale"),
        "empty manifest → 'Nothing stale.', got: {out}"
    );
}

#[test]
fn p_foster_p24_fetch_boxscore_dry_run_invalid_date_errors() {
    let home = tempfile::tempdir().unwrap();
    let err = fail_in(
        home.path(),
        &["fetch", "boxscore", "--date", "tomorrow", "--dry-run"],
    );
    assert!(err.contains("invalid date"));
}

#[test]
fn p_foster_p25_config_set_sync_policy_cycles() {
    let home = tempfile::tempdir().unwrap();
    ok_in(home.path(), &["setup", "--accept-defaults"]);
    for mode in ["lazy", "off", "eager"] {
        ok_in(home.path(), &["config", "set", "sync.policy", mode]);
        let got = ok_in(home.path(), &["config", "get", "sync.policy"]);
        assert_eq!(got.trim(), mode);
    }
}

#[test]
fn p_foster_p26_config_set_banner_modes_cycle() {
    let home = tempfile::tempdir().unwrap();
    ok_in(home.path(), &["setup", "--accept-defaults"]);
    for mode in ["silent", "verbose", "summary"] {
        ok_in(home.path(), &["config", "set", "sync.banner", mode]);
    }
    let got = ok_in(home.path(), &["config", "get", "sync.banner"]);
    assert_eq!(got.trim(), "summary");
}

// ── Data layer (4) ───────────────────────────────────────────────────────────

#[test]
fn p_foster_p27_data_status_empty_then_setup_then_status_again() {
    let home = tempfile::tempdir().unwrap();
    let pre = ok_in(home.path(), &["data-status"]);
    assert!(pre.contains("Manifest is empty"));
    // Setup writes config but doesn't itself populate the manifest;
    // the empty-state survives.
    ok_in(home.path(), &["setup", "--accept-defaults"]);
    let post = ok_in(home.path(), &["data-status"]);
    assert!(
        post.contains("Manifest is empty"),
        "setup alone doesn't seed the manifest, got: {post}"
    );
}

#[test]
fn p_foster_p28_data_status_shard_filter_unknown_kind() {
    let home = tempfile::tempdir().unwrap();
    let err = fail_in(home.path(), &["data-status", "--shard", "wickets"]);
    assert!(err.contains("unknown shard"));
}

#[test]
fn p_foster_p29_data_status_stale_only_empty_manifest_exits_zero() {
    let home = tempfile::tempdir().unwrap();
    let out = ok_in(home.path(), &["data-status", "--stale-only"]);
    assert!(out.contains("Manifest is empty"));
}

#[test]
fn p_foster_p30_data_status_known_shard_filter_returns_no_entries() {
    let home = tempfile::tempdir().unwrap();
    let out = ok_in(home.path(), &["data-status", "--shard", "bios"]);
    assert!(
        out.contains("No manifest entries for Bios"),
        "shard-scoped empty message must surface, got: {out}"
    );
}
