//! Persona Wave 5 — 100 scenarios across Foster favorites, setup wizard,
//! config, group commands. Bug-hunting at scale: catch surface
//! regressions, output stability, exit-code consistency, JSON envelope
//! shapes.
//!
//! Build: `cargo build --release -p icelines-cli`
//! Run: `cargo test -p icelines-cli --test persona_wave5`

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

fn run_in(home: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(icelines_bin())
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("ICELINES_NO_LIVE", "1")
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

fn fail_in(home: &std::path::Path, args: &[&str]) -> String {
    let out = run_in(home, args);
    assert!(
        !out.status.success(),
        "{:?} must non-zero exit; stdout:\n{}",
        args,
        String::from_utf8_lossy(&out.stdout)
    );
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn no_panic_in(home: &std::path::Path, args: &[&str]) {
    let out = run_in(home, args);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !combined.contains("panicked"),
        "{:?} panicked, output:\n{combined}",
        args
    );
}

fn fresh() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

// ── Setup wizard (15) ────────────────────────────────────────────────────────

#[test]
fn p_w5_001_setup_help_lists_flags() {
    let h = fresh();
    let out = ok_in(h.path(), &["setup", "--help"]);
    assert!(out.contains("--accept-defaults"));
    assert!(out.contains("--dry-run"));
    assert!(out.contains("--reset"));
}

#[test]
fn p_w5_002_setup_accept_defaults_writes_config_file() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    let cfg = h.path().join(".icelines").join("config.toml");
    assert!(cfg.exists());
}

#[test]
fn p_w5_003_setup_dry_run_alone_doesnt_write() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults", "--dry-run"]);
    let cfg = h.path().join(".icelines").join("config.toml");
    assert!(!cfg.exists());
}

#[test]
fn p_w5_004_setup_dry_run_announces_dry_run() {
    let h = fresh();
    let out = ok_in(h.path(), &["setup", "--accept-defaults", "--dry-run"]);
    assert!(out.contains("(dry run") || out.contains("dry-run"));
}

#[test]
fn p_w5_005_setup_creates_icelines_dir() {
    let h = fresh();
    let dir = h.path().join(".icelines");
    assert!(!dir.exists(), "fresh tempdir has no .icelines");
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    assert!(dir.exists());
}

#[test]
fn p_w5_006_setup_idempotent() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    // Second run shouldn't error — config exists, accept-defaults overwrites.
    let cfg = h.path().join(".icelines").join("config.toml");
    assert!(cfg.exists());
}

#[test]
fn p_w5_007_setup_writes_sync_section() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    let body = std::fs::read_to_string(h.path().join(".icelines").join("config.toml"))
        .expect("read config");
    assert!(
        body.contains("[sync"),
        "config must have [sync] block, got: {body}"
    );
}

#[test]
fn p_w5_008_setup_writes_capabilities_subsection() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    let body =
        std::fs::read_to_string(h.path().join(".icelines").join("config.toml")).expect("read");
    assert!(body.contains("[sync.capabilities]") || body.contains("capabilities"));
}

#[test]
fn p_w5_009_setup_default_transactions_is_favorites() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    let body = std::fs::read_to_string(h.path().join(".icelines").join("config.toml")).unwrap();
    assert!(body.contains("transactions = \"favorites\""), "got: {body}");
}

#[test]
fn p_w5_010_setup_default_shifts_is_off() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    let body = std::fs::read_to_string(h.path().join(".icelines").join("config.toml")).unwrap();
    assert!(body.contains("shifts = \"off\""));
}

#[test]
fn p_w5_011_setup_resolved_config_summary_in_stdout() {
    let h = fresh();
    let out = ok_in(h.path(), &["setup", "--accept-defaults"]);
    assert!(out.contains("transactions"));
    assert!(out.contains("favorites"));
}

#[test]
fn p_w5_012_setup_no_panic_on_existing_dir() {
    let h = fresh();
    std::fs::create_dir_all(h.path().join(".icelines")).unwrap();
    no_panic_in(h.path(), &["setup", "--accept-defaults"]);
}

#[test]
fn p_w5_013_setup_reset_with_existing_config_overwrites() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(
        h.path(),
        &["config", "set", "sync.capabilities.transactions", "off"],
    );
    ok_in(h.path(), &["setup", "--accept-defaults", "--reset"]);
    let got = ok_in(
        h.path(),
        &["config", "get", "sync.capabilities.transactions"],
    );
    assert_eq!(got.trim(), "favorites", "reset restores spec default");
}

#[test]
fn p_w5_014_setup_dry_run_and_reset_combine() {
    let h = fresh();
    no_panic_in(
        h.path(),
        &["setup", "--accept-defaults", "--dry-run", "--reset"],
    );
}

#[test]
fn p_w5_015_setup_default_invocation_no_args_smoke() {
    // Setup with no flags is the interactive prompt — pass empty
    // stdin via a redirected null. It should either succeed (with
    // empty defaults) or exit cleanly. The point is no panic.
    let h = fresh();
    let out = Command::new(icelines_bin())
        .env("HOME", h.path())
        .env("USERPROFILE", h.path())
        .env("ICELINES_NO_LIVE", "1")
        .args(["setup"])
        .stdin(std::process::Stdio::null())
        .output()
        .expect("run");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!combined.contains("panicked"), "no panic, got: {combined}");
}

// ── Config get / set (35) ────────────────────────────────────────────────────

#[test]
fn p_w5_016_config_help_lists_subcommands() {
    let h = fresh();
    let out = ok_in(h.path(), &["config", "--help"]);
    assert!(out.contains("get"));
    assert!(out.contains("set"));
    assert!(out.contains("list"));
    assert!(out.contains("reset"));
}

#[test]
fn p_w5_017_config_get_unset_key_errors() {
    let h = fresh();
    let err = fail_in(h.path(), &["config", "get", "sync.nonsense"]);
    assert!(err.contains("unknown") || err.contains("not"));
}

#[test]
fn p_w5_018_config_get_sync_policy_default_is_eager() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    let got = ok_in(h.path(), &["config", "get", "sync.policy"]);
    assert_eq!(got.trim(), "eager");
}

#[test]
fn p_w5_019_config_get_sync_banner_default_is_summary() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    let got = ok_in(h.path(), &["config", "get", "sync.banner"]);
    assert_eq!(got.trim(), "summary");
}

#[test]
fn p_w5_020_config_get_sync_season_transition_default_prompt() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    let got = ok_in(h.path(), &["config", "get", "sync.season_transition"]);
    assert_eq!(got.trim(), "prompt");
}

#[test]
fn p_w5_021_config_set_policy_lazy_round_trip() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(h.path(), &["config", "set", "sync.policy", "lazy"]);
    let got = ok_in(h.path(), &["config", "get", "sync.policy"]);
    assert_eq!(got.trim(), "lazy");
}

#[test]
fn p_w5_022_config_set_policy_off_round_trip() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(h.path(), &["config", "set", "sync.policy", "off"]);
    let got = ok_in(h.path(), &["config", "get", "sync.policy"]);
    assert_eq!(got.trim(), "off");
}

#[test]
fn p_w5_023_config_set_policy_invalid_errors() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    let err = fail_in(h.path(), &["config", "set", "sync.policy", "garbage"]);
    assert!(err.contains("unknown") || err.contains("expected"));
}

#[test]
fn p_w5_024_config_set_banner_silent() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(h.path(), &["config", "set", "sync.banner", "silent"]);
    assert_eq!(
        ok_in(h.path(), &["config", "get", "sync.banner"]).trim(),
        "silent"
    );
}

#[test]
fn p_w5_025_config_set_banner_verbose() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(h.path(), &["config", "set", "sync.banner", "verbose"]);
    assert_eq!(
        ok_in(h.path(), &["config", "get", "sync.banner"]).trim(),
        "verbose"
    );
}

#[test]
fn p_w5_026_config_set_banner_invalid_errors() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    fail_in(h.path(), &["config", "set", "sync.banner", "shouty"]);
}

#[test]
fn p_w5_027_config_set_season_transition_auto() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(
        h.path(),
        &["config", "set", "sync.season_transition", "auto"],
    );
    assert_eq!(
        ok_in(h.path(), &["config", "get", "sync.season_transition"]).trim(),
        "auto"
    );
}

#[test]
fn p_w5_028_config_set_season_transition_ignore() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(
        h.path(),
        &["config", "set", "sync.season_transition", "ignore"],
    );
    assert_eq!(
        ok_in(h.path(), &["config", "get", "sync.season_transition"]).trim(),
        "ignore"
    );
}

#[test]
fn p_w5_029_config_set_capabilities_stats_off() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(
        h.path(),
        &["config", "set", "sync.capabilities.stats", "off"],
    );
    assert_eq!(
        ok_in(h.path(), &["config", "get", "sync.capabilities.stats"]).trim(),
        "off"
    );
}

#[test]
fn p_w5_030_config_set_capabilities_stats_favorites() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(
        h.path(),
        &["config", "set", "sync.capabilities.stats", "favorites"],
    );
    assert_eq!(
        ok_in(h.path(), &["config", "get", "sync.capabilities.stats"]).trim(),
        "favorites"
    );
}

#[test]
fn p_w5_031_config_set_capabilities_scores_schedule_off() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(
        h.path(),
        &["config", "set", "sync.capabilities.scores_schedule", "off"],
    );
    assert_eq!(
        ok_in(
            h.path(),
            &["config", "get", "sync.capabilities.scores_schedule"]
        )
        .trim(),
        "off"
    );
}

#[test]
fn p_w5_032_config_set_capabilities_transactions_league() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(
        h.path(),
        &["config", "set", "sync.capabilities.transactions", "league"],
    );
    assert_eq!(
        ok_in(
            h.path(),
            &["config", "get", "sync.capabilities.transactions"]
        )
        .trim(),
        "league"
    );
}

#[test]
fn p_w5_033_config_set_capabilities_boxscores_off() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(
        h.path(),
        &["config", "set", "sync.capabilities.boxscores", "off"],
    );
    assert_eq!(
        ok_in(h.path(), &["config", "get", "sync.capabilities.boxscores"]).trim(),
        "off"
    );
}

#[test]
fn p_w5_034_config_set_capabilities_boxscores_league() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(
        h.path(),
        &["config", "set", "sync.capabilities.boxscores", "league"],
    );
    assert_eq!(
        ok_in(h.path(), &["config", "get", "sync.capabilities.boxscores"]).trim(),
        "league"
    );
}

#[test]
fn p_w5_035_config_set_capabilities_career_history_off() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(
        h.path(),
        &["config", "set", "sync.capabilities.career_history", "off"],
    );
    assert_eq!(
        ok_in(
            h.path(),
            &["config", "get", "sync.capabilities.career_history"]
        )
        .trim(),
        "off"
    );
}

#[test]
fn p_w5_036_config_set_capabilities_career_history_league() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(
        h.path(),
        &[
            "config",
            "set",
            "sync.capabilities.career_history",
            "league",
        ],
    );
    assert_eq!(
        ok_in(
            h.path(),
            &["config", "get", "sync.capabilities.career_history"]
        )
        .trim(),
        "league"
    );
}

#[test]
fn p_w5_037_config_shifts_off_succeeds() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(
        h.path(),
        &["config", "set", "sync.capabilities.shifts", "off"],
    );
}

#[test]
fn p_w5_038_config_shifts_favorites_rejected_literal() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    let err = fail_in(
        h.path(),
        &["config", "set", "sync.capabilities.shifts", "favorites"],
    );
    assert!(err.contains("capability `shifts`"));
    assert!(err.contains("Allowed values today: off"));
}

#[test]
fn p_w5_039_config_shifts_league_rejected_literal() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    let err = fail_in(
        h.path(),
        &["config", "set", "sync.capabilities.shifts", "league"],
    );
    assert!(err.contains("capability `shifts`"));
}

#[test]
fn p_w5_040_config_invalid_mode_errors() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    fail_in(
        h.path(),
        &[
            "config",
            "set",
            "sync.capabilities.transactions",
            "everywhere",
        ],
    );
}

#[test]
fn p_w5_041_config_unknown_capability_errors() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    fail_in(
        h.path(),
        &["config", "set", "sync.capabilities.aurora", "off"],
    );
}

#[test]
fn p_w5_042_config_list_includes_every_capability() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    let out = ok_in(h.path(), &["config", "list"]);
    for cap in [
        "sync.capabilities.stats",
        "sync.capabilities.scores_schedule",
        "sync.capabilities.transactions",
        "sync.capabilities.boxscores",
        "sync.capabilities.shifts",
        "sync.capabilities.career_history",
    ] {
        assert!(out.contains(cap), "missing {cap} in:\n{out}");
    }
}

#[test]
fn p_w5_043_config_list_includes_sync_policy() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    let out = ok_in(h.path(), &["config", "list"]);
    assert!(out.contains("sync.policy"));
}

#[test]
fn p_w5_044_config_reset_sync_capabilities_restores_defaults() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(
        h.path(),
        &["config", "set", "sync.capabilities.transactions", "off"],
    );
    ok_in(
        h.path(),
        &["config", "set", "sync.capabilities.boxscores", "league"],
    );
    ok_in(h.path(), &["config", "reset", "sync.capabilities"]);
    assert_eq!(
        ok_in(
            h.path(),
            &["config", "get", "sync.capabilities.transactions"]
        )
        .trim(),
        "favorites"
    );
    assert_eq!(
        ok_in(h.path(), &["config", "get", "sync.capabilities.boxscores"]).trim(),
        "favorites"
    );
}

#[test]
fn p_w5_045_config_reset_unknown_section_errors() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    fail_in(h.path(), &["config", "reset", "garbage"]);
}

#[test]
fn p_w5_046_config_get_no_args_errors() {
    let h = fresh();
    fail_in(h.path(), &["config", "get"]);
}

#[test]
fn p_w5_047_config_set_no_value_errors() {
    let h = fresh();
    fail_in(h.path(), &["config", "set", "sync.policy"]);
}

#[test]
fn p_w5_048_config_set_persists_across_invocations() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(h.path(), &["config", "set", "sync.policy", "off"]);
    // Second process invocation reads the file the first wrote.
    let got = ok_in(h.path(), &["config", "get", "sync.policy"]);
    assert_eq!(got.trim(), "off");
}

#[test]
fn p_w5_049_config_writes_atomic_no_tmp_left() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(h.path(), &["config", "set", "sync.policy", "lazy"]);
    let tmp = h.path().join(".icelines").join("config.toml.tmp");
    assert!(!tmp.exists(), "atomic write must not leave tmp sidecar");
}

#[test]
fn p_w5_050_config_no_panic_on_unset_yet_get() {
    // Setup never run; config get should error cleanly.
    let h = fresh();
    no_panic_in(h.path(), &["config", "get", "sync.policy"]);
}

// ── Group commands (15) ──────────────────────────────────────────────────────

#[test]
fn p_w5_051_group_help_smoke() {
    let h = fresh();
    let out = ok_in(h.path(), &["group", "--help"]);
    assert!(out.contains("create") && out.contains("add") && out.contains("remove"));
}

#[test]
fn p_w5_052_group_list_default_includes_favorites() {
    let h = fresh();
    let out = ok_in(h.path(), &["group", "list"]);
    assert!(
        out.contains("Favorites"),
        "default group exists, got: {out}"
    );
}

#[test]
fn p_w5_053_group_create_then_list_includes_new() {
    let h = fresh();
    ok_in(h.path(), &["group", "create", "Watchlist"]);
    let out = ok_in(h.path(), &["group", "list"]);
    assert!(out.contains("Watchlist"));
}

#[test]
fn p_w5_054_group_create_duplicate_errors() {
    let h = fresh();
    ok_in(h.path(), &["group", "create", "X"]);
    fail_in(h.path(), &["group", "create", "X"]);
}

#[test]
fn p_w5_055_group_add_team_3_letter_routes_to_team() {
    let h = fresh();
    ok_in(h.path(), &["group", "add", "Favorites", "EDM"]);
    let out = ok_in(h.path(), &["group", "show", "Favorites"]);
    assert!(out.contains("EDM"));
}

#[test]
fn p_w5_056_group_add_player_name_routes_to_player() {
    let h = fresh();
    ok_in(h.path(), &["group", "add", "Favorites", "Connor McDavid"]);
    let out = ok_in(h.path(), &["group", "show", "Favorites"]);
    assert!(out.contains("connor mcdavid") || out.contains("Connor McDavid"));
}

#[test]
fn p_w5_057_group_add_idempotent() {
    let h = fresh();
    ok_in(h.path(), &["group", "add", "Favorites", "EDM"]);
    let out = ok_in(h.path(), &["group", "add", "Favorites", "EDM"]);
    assert!(out.contains("already") || out.contains("no change"));
}

#[test]
fn p_w5_058_group_add_to_unknown_group_errors() {
    let h = fresh();
    fail_in(h.path(), &["group", "add", "NoSuch", "EDM"]);
}

#[test]
fn p_w5_059_group_remove_player_works() {
    let h = fresh();
    ok_in(h.path(), &["group", "add", "Favorites", "Connor McDavid"]);
    ok_in(
        h.path(),
        &["group", "remove", "Favorites", "Connor McDavid"],
    );
}

#[test]
fn p_w5_060_group_show_unknown_errors() {
    let h = fresh();
    fail_in(h.path(), &["group", "show", "NoSuch"]);
}

#[test]
fn p_w5_061_group_delete_then_show_errors() {
    let h = fresh();
    ok_in(h.path(), &["group", "create", "Tmp"]);
    ok_in(h.path(), &["group", "delete", "Tmp"]);
    fail_in(h.path(), &["group", "show", "Tmp"]);
}

#[test]
fn p_w5_062_group_delete_unknown_errors() {
    let h = fresh();
    fail_in(h.path(), &["group", "delete", "NoSuch"]);
}

#[test]
fn p_w5_063_group_show_empty_group() {
    let h = fresh();
    ok_in(h.path(), &["group", "create", "Empty"]);
    let out = ok_in(h.path(), &["group", "show", "Empty"]);
    assert!(out.contains("Empty"));
}

#[test]
fn p_w5_064_group_add_handles_special_chars_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["group", "add", "Favorites", "O'Reilly"]);
}

#[test]
fn p_w5_065_group_add_lowercase_team_uppercased() {
    let h = fresh();
    ok_in(h.path(), &["group", "add", "Favorites", "edm"]);
    let out = ok_in(h.path(), &["group", "show", "Favorites"]);
    // Either the team got uppercased or it routed to player kind —
    // both are acceptable; what matters is no panic + the entry appears.
    assert!(out.contains("EDM") || out.contains("edm"));
}

// ── Favorites command (20) ───────────────────────────────────────────────────

#[test]
fn p_w5_066_favorites_empty_state_text() {
    let h = fresh();
    let out = ok_in(h.path(), &["favorites"]);
    assert!(out.contains("FAVORITES"));
}

#[test]
fn p_w5_067_favorites_empty_state_teaches() {
    let h = fresh();
    let out = ok_in(h.path(), &["favorites"]);
    assert!(out.contains("group add Favorites") || out.contains("No favorites"));
}

#[test]
fn p_w5_068_favorites_json_envelope_valid() {
    let h = fresh();
    let out = ok_in(h.path(), &["favorites", "--json"]);
    let _: serde_json::Value = serde_json::from_str(&out).expect("--json must emit valid JSON");
}

#[test]
fn p_w5_069_favorites_json_has_schema_version() {
    let h = fresh();
    let out = ok_in(h.path(), &["favorites", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["schema_version"], 1);
}

#[test]
fn p_w5_070_favorites_json_has_route() {
    let h = fresh();
    let out = ok_in(h.path(), &["favorites", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["route"], "favorites");
}

#[test]
fn p_w5_071_favorites_json_data_has_players_array() {
    let h = fresh();
    let out = ok_in(h.path(), &["favorites", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v["data"]["players"].is_array());
}

#[test]
fn p_w5_072_favorites_json_data_has_teams_array() {
    let h = fresh();
    let out = ok_in(h.path(), &["favorites", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v["data"]["teams"].is_array());
}

#[test]
fn p_w5_073_favorites_json_data_has_events_array() {
    let h = fresh();
    let out = ok_in(h.path(), &["favorites", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v["data"]["events"].is_array());
}

#[test]
fn p_w5_074_favorites_invalid_date_errors() {
    let h = fresh();
    fail_in(h.path(), &["favorites", "--date", "not-a-date"]);
}

#[test]
fn p_w5_075_favorites_invalid_range_errors() {
    let h = fresh();
    fail_in(h.path(), &["favorites", "--range", "millennium"]);
}

#[test]
fn p_w5_076_favorites_range_day_works() {
    let h = fresh();
    ok_in(h.path(), &["favorites", "--range", "day"]);
}

#[test]
fn p_w5_077_favorites_range_week_works() {
    let h = fresh();
    ok_in(h.path(), &["favorites", "--range", "week"]);
}

#[test]
fn p_w5_078_favorites_range_month_works() {
    let h = fresh();
    ok_in(h.path(), &["favorites", "--range", "month"]);
}

#[test]
fn p_w5_079_favorites_range_season_works() {
    let h = fresh();
    ok_in(h.path(), &["favorites", "--range", "season"]);
}

#[test]
fn p_w5_080_favorites_unknown_group_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["favorites", "--group", "NoSuchGroup"]);
}

#[test]
fn p_w5_081_favorites_with_team_added_renders_count() {
    let h = fresh();
    ok_in(h.path(), &["group", "add", "Favorites", "EDM"]);
    let out = ok_in(h.path(), &["favorites"]);
    assert!(out.contains("1 team") || out.contains("EDM"));
}

#[test]
fn p_w5_082_favorites_with_player_added_renders() {
    let h = fresh();
    ok_in(h.path(), &["group", "add", "Favorites", "Connor McDavid"]);
    let out = ok_in(h.path(), &["favorites"]);
    assert!(out.contains("1 player") || out.contains("McDavid") || out.contains("mcdavid"));
}

#[test]
fn p_w5_083_favorites_help_lists_flags() {
    let h = fresh();
    let out = ok_in(h.path(), &["favorites", "--help"]);
    assert!(out.contains("--date"));
    assert!(out.contains("--range"));
    assert!(out.contains("--group"));
    assert!(out.contains("--json"));
}

#[test]
fn p_w5_084_favorites_with_group_flag_smoke() {
    let h = fresh();
    ok_in(h.path(), &["group", "create", "Other"]);
    ok_in(h.path(), &["group", "add", "Other", "TOR"]);
    let out = ok_in(h.path(), &["favorites", "--group", "Other"]);
    assert!(out.contains("FAVORITES") || out.contains("TOR"));
}

#[test]
fn p_w5_085_favorites_past_date_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["favorites", "--date", "2014-10-08"]);
}

// ── Cross-feature flows (15) ─────────────────────────────────────────────────

#[test]
fn p_w5_086_setup_then_data_status_empty() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    let out = ok_in(h.path(), &["data-status"]);
    assert!(out.contains("Manifest is empty"));
}

#[test]
fn p_w5_087_data_status_empty_without_setup() {
    let h = fresh();
    let out = ok_in(h.path(), &["data-status"]);
    assert!(out.contains("Manifest is empty"));
}

#[test]
fn p_w5_088_data_status_json_smoke() {
    // data-status doesn't have a --json flag; this catches if one
    // were added without thought. For now expect clean error.
    let h = fresh();
    let out = run_in(h.path(), &["data-status", "--json"]);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    // Either accepted (fine) or rejected (also fine — no panic)
    assert!(!combined.contains("panicked"));
}

#[test]
fn p_w5_089_setup_then_config_get_lists_consistent() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    let list = ok_in(h.path(), &["config", "list"]);
    let p = ok_in(h.path(), &["config", "get", "sync.policy"]);
    assert!(
        list.contains(&format!("sync.policy = {}", p.trim())),
        "list and get must agree, list:\n{list}\nget:{p}"
    );
}

#[test]
fn p_w5_090_setup_add_favorite_then_show() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(h.path(), &["group", "add", "Favorites", "EDM"]);
    let out = ok_in(h.path(), &["favorites"]);
    assert!(out.contains("EDM") || out.contains("1 team"));
}

#[test]
fn p_w5_091_full_round_trip_flow() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(h.path(), &["group", "add", "Favorites", "EDM"]);
    ok_in(h.path(), &["group", "add", "Favorites", "Connor McDavid"]);
    let out = ok_in(h.path(), &["favorites", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let total: u64 = v["meta"]["members_total"].as_u64().unwrap_or(0);
    assert_eq!(total, 2, "two members added — {v:?}");
}

#[test]
fn p_w5_092_data_status_after_setup_still_empty() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    let out = ok_in(h.path(), &["data-status"]);
    assert!(
        out.contains("Manifest is empty"),
        "setup alone seeds config, not manifest"
    );
}

#[test]
fn p_w5_093_data_status_filter_smoke() {
    let h = fresh();
    let out = ok_in(h.path(), &["data-status", "--shard", "bios"]);
    assert!(out.contains("Bios") || out.contains("No manifest entries"));
}

#[test]
fn p_w5_094_data_status_stale_only_smoke() {
    let h = fresh();
    ok_in(h.path(), &["data-status", "--stale-only"]);
}

#[test]
fn p_w5_095_data_status_stale_only_combined_with_shard() {
    let h = fresh();
    ok_in(
        h.path(),
        &["data-status", "--shard", "stats", "--stale-only"],
    );
}

#[test]
fn p_w5_096_data_status_unknown_shard_helpful_error() {
    let h = fresh();
    let err = fail_in(h.path(), &["data-status", "--shard", "wickets"]);
    assert!(err.contains("unknown shard"));
}

#[test]
fn p_w5_097_global_help_lists_foster_commands() {
    let h = fresh();
    let out = ok_in(h.path(), &["--help"]);
    for cmd in ["favorites", "setup", "config", "data-status"] {
        assert!(out.contains(cmd), "global help missing {cmd}");
    }
}

#[test]
fn p_w5_098_setup_then_config_set_stays_clean_no_extra_files() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(h.path(), &["config", "set", "sync.policy", "off"]);
    let dir = h.path().join(".icelines");
    let entries: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    // Only config.toml + icelines.db (created by group seed) expected.
    // Tmp sidecars or rename leftovers would surface as extra entries.
    for name in &entries {
        assert!(
            !name.ends_with(".tmp"),
            ".tmp file leaked: {name} in {entries:?}"
        );
    }
}

#[test]
fn p_w5_099_setup_then_no_setup_flag_skips() {
    // --no-setup is a top-level flag; passing it bypasses any
    // auto-prompt logic. With config already present, all paths
    // should still work.
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(h.path(), &["--no-setup", "favorites"]);
}

#[test]
fn p_w5_100_no_setup_top_level_flag_alone_doesnt_break_anything() {
    // Bare --no-setup with a known command shouldn't cause anything.
    let h = fresh();
    no_panic_in(h.path(), &["--no-setup", "favorites"]);
    no_panic_in(h.path(), &["--no-setup", "data-status"]);
    no_panic_in(h.path(), &["--no-setup", "group", "list"]);
}
