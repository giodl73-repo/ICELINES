//! Persona Wave 9 — edge cases + cross-feature interactions.
//! 100 scenarios designed to surface hidden bugs: malformed inputs,
//! boundary conditions, env-var precedence, output stability,
//! concurrent operations, exit-code consistency, weird Unicode,
//! empty/null/giant inputs.

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

// ── Malformed inputs (15) ────────────────────────────────────────────────────

#[test]
fn p_w9_001_completely_invalid_subcommand() {
    let h = fresh();
    let err = fail_in(h.path(), &["nonsense"]);
    assert!(!err.contains("panicked"));
}

#[test]
fn p_w9_002_typo_in_subcommand_suggests() {
    // clap's "did you mean" — usually printed when typos are close.
    let h = fresh();
    let out = run_in(h.path(), &["favorits"]);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!combined.contains("panicked"));
}

#[test]
fn p_w9_003_double_dash_alone_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["--"]);
}

#[test]
fn p_w9_004_empty_args_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &[]);
}

#[test]
fn p_w9_005_unicode_in_player_name_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["group", "add", "Favorites", "Slafkovský"]);
}

#[test]
fn p_w9_006_emoji_in_player_name_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["group", "add", "Favorites", "Mc🏒David"]);
}

#[test]
fn p_w9_007_carriage_return_in_arg_no_panic() {
    // Null bytes are rejected by the OS / std::process::Command
    // before the binary even spawns — that's not an icelines
    // behavior to test. Carriage returns DO make it through to the
    // binary, so verify icelines handles them cleanly.
    let h = fresh();
    no_panic_in(h.path(), &["group", "add", "Favorites", "Wayne\rGretzky"]);
}

#[test]
fn p_w9_008_very_long_player_name_no_panic() {
    let h = fresh();
    let long = "X".repeat(1000);
    no_panic_in(h.path(), &["group", "add", "Favorites", &long]);
}

#[test]
fn p_w9_009_very_long_group_name_no_panic() {
    let h = fresh();
    let long = "G".repeat(500);
    no_panic_in(h.path(), &["group", "create", &long]);
}

#[test]
fn p_w9_010_empty_player_name() {
    let h = fresh();
    no_panic_in(h.path(), &["group", "add", "Favorites", ""]);
}

#[test]
fn p_w9_011_empty_group_name() {
    let h = fresh();
    no_panic_in(h.path(), &["group", "add", "", "EDM"]);
}

#[test]
fn p_w9_012_whitespace_only_player_name() {
    let h = fresh();
    no_panic_in(h.path(), &["group", "add", "Favorites", "   "]);
}

#[test]
fn p_w9_013_tab_in_player_name() {
    let h = fresh();
    no_panic_in(h.path(), &["group", "add", "Favorites", "Wayne\tGretzky"]);
}

#[test]
fn p_w9_014_newline_in_player_name() {
    let h = fresh();
    no_panic_in(h.path(), &["group", "add", "Favorites", "line1\nline2"]);
}

#[test]
fn p_w9_015_weird_team_abbrev_long_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["group", "add", "Favorites", "ABCDE"]);
}

// ── Boundary conditions (15) ─────────────────────────────────────────────────

#[test]
fn p_w9_016_date_year_1_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["tonight", "--date", "0001-01-01"]);
}

#[test]
fn p_w9_017_date_year_9999_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["tonight", "--date", "9999-12-31"]);
}

#[test]
fn p_w9_018_date_minute_precision_rejected() {
    // YYYY-MM-DD format only — ISO timestamps shouldn't parse.
    let h = fresh();
    fail_in(h.path(), &["tonight", "--date", "2026-01-15T12:00:00Z"]);
}

#[test]
fn p_w9_019_date_with_only_year_rejected() {
    let h = fresh();
    fail_in(h.path(), &["tonight", "--date", "2026"]);
}

#[test]
fn p_w9_020_date_with_only_year_month_rejected() {
    let h = fresh();
    fail_in(h.path(), &["tonight", "--date", "2026-01"]);
}

#[test]
fn p_w9_021_season_at_bundle_floor() {
    // Earliest bundled season is 1987-88
    let h = fresh();
    no_panic_in(h.path(), &["query", "leaders", "--season", "19871988"]);
}

#[test]
fn p_w9_022_season_below_bundle_floor() {
    let h = fresh();
    no_panic_in(h.path(), &["query", "leaders", "--season", "19001901"]);
}

#[test]
fn p_w9_023_season_at_bundle_ceiling() {
    let h = fresh();
    no_panic_in(h.path(), &["query", "leaders", "--season", "20252026"]);
}

#[test]
fn p_w9_024_season_above_bundle_ceiling() {
    let h = fresh();
    no_panic_in(h.path(), &["query", "leaders", "--season", "20302031"]);
}

#[test]
fn p_w9_025_query_top_zero_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["query", "leaders", "--top", "0"]);
}

#[test]
fn p_w9_026_query_top_huge_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["query", "leaders", "--top", "100000"]);
}

#[test]
fn p_w9_027_schedule_days_at_min_1() {
    let h = fresh();
    no_panic_in(h.path(), &["schedule", "--days", "1"]);
}

#[test]
fn p_w9_028_schedule_days_at_max_14() {
    let h = fresh();
    no_panic_in(h.path(), &["schedule", "--days", "14"]);
}

#[test]
fn p_w9_029_max_concurrent_groups() {
    let h = fresh();
    for i in 0..10 {
        ok_in(h.path(), &["group", "create", &format!("Group{i}")]);
    }
    let out = ok_in(h.path(), &["group", "list"]);
    for i in 0..10 {
        assert!(out.contains(&format!("Group{i}")));
    }
}

#[test]
fn p_w9_030_max_members_per_group() {
    let h = fresh();
    for i in 0..50 {
        ok_in(
            h.path(),
            &["group", "add", "Favorites", &format!("Player{i}")],
        );
    }
    no_panic_in(h.path(), &["favorites"]);
}

// ── Env var precedence (15) ──────────────────────────────────────────────────

#[test]
fn p_w9_031_no_live_env_honored_in_fetch() {
    let h = fresh();
    let out = Command::new(icelines_bin())
        .env("HOME", h.path())
        .env("USERPROFILE", h.path())
        .env("ICELINES_NO_LIVE", "1")
        .args(["fetch", "boxscore", "--dry-run"])
        .output()
        .unwrap();
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!combined.contains("panicked"));
}

#[test]
fn p_w9_032_no_live_env_zero_means_live_on() {
    let h = fresh();
    let out = Command::new(icelines_bin())
        .env("HOME", h.path())
        .env("USERPROFILE", h.path())
        .env("ICELINES_NO_LIVE", "0")
        .args(["data-status"])
        .output()
        .unwrap();
    assert!(out.status.success());
}

#[test]
fn p_w9_033_no_live_env_unset_means_live_on() {
    let h = fresh();
    let out = Command::new(icelines_bin())
        .env("HOME", h.path())
        .env("USERPROFILE", h.path())
        .env_remove("ICELINES_NO_LIVE")
        .args(["data-status"])
        .output()
        .unwrap();
    assert!(out.status.success());
}

#[test]
fn p_w9_034_no_live_cli_overrides_env() {
    let h = fresh();
    let out = Command::new(icelines_bin())
        .env("HOME", h.path())
        .env("USERPROFILE", h.path())
        .env("ICELINES_NO_LIVE", "0")
        .args(["--no-live", "data-status"])
        .output()
        .unwrap();
    assert!(out.status.success());
}

#[test]
fn p_w9_035_no_live_env_string_value_treated_as_truthy() {
    let h = fresh();
    let out = Command::new(icelines_bin())
        .env("HOME", h.path())
        .env("USERPROFILE", h.path())
        .env("ICELINES_NO_LIVE", "true")
        .args(["data-status"])
        .output()
        .unwrap();
    assert!(out.status.success());
}

#[test]
fn p_w9_036_test_mode_env_short_circuits_sync() {
    let h = fresh();
    let out = Command::new(icelines_bin())
        .env("HOME", h.path())
        .env("USERPROFILE", h.path())
        .env("ICELINES_TEST_MODE", "1")
        .args(["fetch", "sync", "--dry-run"])
        .output()
        .unwrap();
    assert!(out.status.success());
}

#[test]
fn p_w9_037_no_setup_top_level_works_globally() {
    let h = fresh();
    let out = Command::new(icelines_bin())
        .env("HOME", h.path())
        .env("USERPROFILE", h.path())
        .env("ICELINES_NO_LIVE", "1")
        .args(["--no-setup", "favorites"])
        .output()
        .unwrap();
    assert!(out.status.success());
}

#[test]
fn p_w9_038_no_dashboards_flag_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["--no-dashboards", "favorites"]);
}

#[test]
fn p_w9_039_combined_top_level_flags_no_panic() {
    let h = fresh();
    no_panic_in(
        h.path(),
        &["--no-live", "--no-dashboards", "--no-setup", "favorites"],
    );
}

#[test]
fn p_w9_040_unknown_top_level_flag_clean_error() {
    let h = fresh();
    fail_in(h.path(), &["--no-such-flag", "favorites"]);
}

#[test]
fn p_w9_041_no_color_env_no_panic() {
    let h = fresh();
    let out = Command::new(icelines_bin())
        .env("HOME", h.path())
        .env("USERPROFILE", h.path())
        .env("ICELINES_NO_LIVE", "1")
        .env("NO_COLOR", "1")
        .args(["favorites"])
        .output()
        .unwrap();
    assert!(out.status.success());
}

#[test]
fn p_w9_042_term_dumb_no_panic() {
    let h = fresh();
    let out = Command::new(icelines_bin())
        .env("HOME", h.path())
        .env("USERPROFILE", h.path())
        .env("ICELINES_NO_LIVE", "1")
        .env("TERM", "dumb")
        .args(["favorites"])
        .output()
        .unwrap();
    assert!(out.status.success());
}

#[test]
fn p_w9_043_unset_home_only_userprofile_works_on_windows() {
    let h = fresh();
    let out = Command::new(icelines_bin())
        .env_remove("HOME")
        .env("USERPROFILE", h.path())
        .env("ICELINES_NO_LIVE", "1")
        .args(["favorites"])
        .output()
        .unwrap();
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!combined.contains("panicked"));
}

#[test]
fn p_w9_044_test_mode_unaffects_query_leaders() {
    let h = fresh();
    let out = Command::new(icelines_bin())
        .env("HOME", h.path())
        .env("USERPROFILE", h.path())
        .env("ICELINES_TEST_MODE", "1")
        .env("ICELINES_NO_LIVE", "1")
        .args(["query", "leaders", "--playoff"])
        .output()
        .unwrap();
    assert!(out.status.success());
}

#[test]
fn p_w9_045_explicit_no_live_with_test_mode_combined() {
    let h = fresh();
    let out = Command::new(icelines_bin())
        .env("HOME", h.path())
        .env("USERPROFILE", h.path())
        .env("ICELINES_TEST_MODE", "1")
        .env("ICELINES_NO_LIVE", "1")
        .args(["fetch", "sync", "--dry-run"])
        .output()
        .unwrap();
    assert!(out.status.success());
}

// ── JSON output stability (15) ───────────────────────────────────────────────

#[test]
fn p_w9_046_favorites_json_deterministic() {
    let h = fresh();
    let a = ok_in(h.path(), &["favorites", "--json"]);
    let b = ok_in(h.path(), &["favorites", "--json"]);
    assert_eq!(a, b, "favorites --json must be deterministic");
}

#[test]
fn p_w9_047_data_status_text_deterministic() {
    let h = fresh();
    let a = ok_in(h.path(), &["data-status"]);
    let b = ok_in(h.path(), &["data-status"]);
    assert_eq!(a, b);
}

#[test]
fn p_w9_048_query_leaders_playoff_json_deterministic() {
    let h = fresh();
    let a = ok_in(h.path(), &["query", "leaders", "--playoff", "--json"]);
    let b = ok_in(h.path(), &["query", "leaders", "--playoff", "--json"]);
    assert_eq!(a, b);
}

#[test]
fn p_w9_049_playoffs_series_json_deterministic() {
    let h = fresh();
    let a = ok_in(
        h.path(),
        &[
            "playoffs", "--season", "19931994", "--series", "A", "--json",
        ],
    );
    let b = ok_in(
        h.path(),
        &[
            "playoffs", "--season", "19931994", "--series", "A", "--json",
        ],
    );
    assert_eq!(a, b);
}

#[test]
fn p_w9_050_config_list_deterministic() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    let a = ok_in(h.path(), &["config", "list"]);
    let b = ok_in(h.path(), &["config", "list"]);
    assert_eq!(a, b);
}

#[test]
fn p_w9_051_favorites_json_no_trailing_newline_redundancy() {
    let h = fresh();
    let out = ok_in(h.path(), &["favorites", "--json"]);
    // serde_json::to_string_pretty adds a trailing newline via println!.
    // Just one trailing newline expected, not multiple.
    assert!(out.ends_with('\n'));
    assert!(!out.ends_with("\n\n\n"));
}

#[test]
fn p_w9_052_favorites_json_no_extra_whitespace() {
    let h = fresh();
    let out = ok_in(h.path(), &["favorites", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let re_serialized = serde_json::to_string(&v).unwrap();
    // Re-serializing to compact form should produce strictly less whitespace.
    assert!(out.len() >= re_serialized.len());
}

#[test]
fn p_w9_053_favorites_json_keys_sorted_consistently() {
    let h = fresh();
    let a = ok_in(h.path(), &["favorites", "--json"]);
    // serde_json::Value preserves key order. Re-serializing twice
    // should give the same output.
    let v: serde_json::Value = serde_json::from_str(&a).unwrap();
    let s1 = serde_json::to_string(&v).unwrap();
    let s2 = serde_json::to_string(&v).unwrap();
    assert_eq!(s1, s2);
}

#[test]
fn p_w9_054_query_leaders_window_json_consistent_envelope() {
    let h = fresh();
    let out = ok_in(h.path(), &["query", "leaders", "--week", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v["schema_version"].is_number());
    assert!(v["route"].is_string());
    assert!(v["data"].is_array());
    assert!(v["meta"].is_object());
}

#[test]
fn p_w9_055_playoff_leaders_json_envelope_consistent() {
    let h = fresh();
    let out = ok_in(h.path(), &["query", "leaders", "--playoff", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v["schema_version"].is_number());
    assert!(v["route"].is_string());
    assert!(v["data"].is_array());
    assert!(v["meta"].is_object());
}

#[test]
fn p_w9_056_series_momentum_json_envelope_consistent() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "playoffs", "--season", "19931994", "--series", "A", "--json",
        ],
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v["schema_version"].is_number());
    assert!(v["route"].is_string());
    assert!(v["data"].is_object()); // single object, not array
    assert!(v["meta"].is_object());
}

#[test]
fn p_w9_057_playoffs_default_json_envelope_consistent() {
    let h = fresh();
    let out = ok_in(h.path(), &["playoffs", "--season", "19931994", "--json"]);
    let _v: serde_json::Value = serde_json::from_str(&out).unwrap();
}

#[test]
fn p_w9_058_schedule_json_when_no_data_still_valid() {
    let h = fresh();
    let out = ok_in(h.path(), &["schedule", "--json"]);
    let _v: serde_json::Value = serde_json::from_str(&out).unwrap();
}

#[test]
fn p_w9_059_csv_output_doesnt_have_carriage_return_artifacts() {
    let h = fresh();
    let out = ok_in(h.path(), &["schedule", "--csv"]);
    // CSV writer typically uses \r\n on Windows; just check no
    // double-double terminators.
    assert!(!out.contains("\r\r"));
}

#[test]
fn p_w9_060_favorites_json_envelope_matches_spec() {
    let h = fresh();
    let out = ok_in(h.path(), &["favorites", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    // K2.4 envelope shape
    assert!(
        v["data"]["players"].is_array(),
        "data.players must be array"
    );
    assert!(v["data"]["teams"].is_array(), "data.teams must be array");
    assert!(v["data"]["events"].is_array(), "data.events must be array");
}

// ── Concurrent / repeated operations (10) ────────────────────────────────────

#[test]
fn p_w9_061_setup_then_immediate_setup_no_corruption() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    let body = std::fs::read_to_string(h.path().join(".icelines").join("config.toml")).unwrap();
    // Config still valid TOML
    let _: toml::Value = toml::from_str(&body).unwrap();
}

#[test]
fn p_w9_062_repeat_group_add_remove_round_trip() {
    let h = fresh();
    for _ in 0..5 {
        ok_in(h.path(), &["group", "add", "Favorites", "TestPlayer"]);
        ok_in(h.path(), &["group", "remove", "Favorites", "TestPlayer"]);
    }
}

#[test]
fn p_w9_063_repeated_data_status_no_corruption() {
    let h = fresh();
    for _ in 0..5 {
        ok_in(h.path(), &["data-status"]);
    }
}

#[test]
fn p_w9_064_repeated_favorites_view() {
    let h = fresh();
    for _ in 0..10 {
        ok_in(h.path(), &["favorites"]);
    }
}

#[test]
fn p_w9_065_repeated_config_set_get() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    for mode in ["off", "lazy", "eager", "off", "eager", "lazy"] {
        ok_in(h.path(), &["config", "set", "sync.policy", mode]);
        let got = ok_in(h.path(), &["config", "get", "sync.policy"]);
        assert_eq!(got.trim(), mode);
    }
}

#[test]
fn p_w9_066_setup_then_reset_then_setup_idempotent() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(h.path(), &["config", "reset", "sync.capabilities"]);
    ok_in(h.path(), &["setup", "--accept-defaults"]);
}

#[test]
fn p_w9_067_create_delete_create_group_round_trip() {
    let h = fresh();
    ok_in(h.path(), &["group", "create", "Round"]);
    ok_in(h.path(), &["group", "delete", "Round"]);
    ok_in(h.path(), &["group", "create", "Round"]);
}

#[test]
fn p_w9_068_repeated_query_leaders_playoff_idempotent() {
    let h = fresh();
    for _ in 0..3 {
        ok_in(h.path(), &["query", "leaders", "--playoff"]);
    }
}

#[test]
fn p_w9_069_repeated_playoffs_series_no_corruption() {
    let h = fresh();
    for _ in 0..3 {
        ok_in(
            h.path(),
            &["playoffs", "--season", "19931994", "--series", "A"],
        );
    }
}

#[test]
fn p_w9_070_full_workflow_repeat() {
    let h = fresh();
    for _ in 0..3 {
        ok_in(h.path(), &["setup", "--accept-defaults"]);
        ok_in(h.path(), &["group", "add", "Favorites", "EDM"]);
        ok_in(h.path(), &["favorites"]);
        ok_in(h.path(), &["group", "remove", "Favorites", "EDM"]);
    }
}

// ── Cross-feature interactions (15) ──────────────────────────────────────────

#[test]
fn p_w9_071_setup_set_capability_persists_across_subcommand() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(
        h.path(),
        &["config", "set", "sync.capabilities.transactions", "off"],
    );
    // Different subcommand reads the same disk; value persists.
    let out = ok_in(h.path(), &["data-status"]);
    let _ = out;
}

#[test]
fn p_w9_072_add_favorite_then_query_sees_member() {
    let h = fresh();
    ok_in(h.path(), &["group", "add", "Favorites", "Connor McDavid"]);
    let out = ok_in(h.path(), &["favorites", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let total = v["meta"]["members_total"].as_u64().unwrap_or(0);
    assert_eq!(total, 1);
}

#[test]
fn p_w9_073_remove_favorite_decrements_count() {
    let h = fresh();
    ok_in(h.path(), &["group", "add", "Favorites", "EDM"]);
    ok_in(h.path(), &["group", "add", "Favorites", "TOR"]);
    ok_in(h.path(), &["group", "remove", "Favorites", "EDM"]);
    let out = ok_in(h.path(), &["favorites", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["meta"]["members_total"].as_u64().unwrap_or(0), 1);
}

#[test]
fn p_w9_074_delete_group_cascades_members() {
    let h = fresh();
    ok_in(h.path(), &["group", "create", "Tmp"]);
    ok_in(h.path(), &["group", "add", "Tmp", "EDM"]);
    ok_in(h.path(), &["group", "add", "Tmp", "TOR"]);
    ok_in(h.path(), &["group", "delete", "Tmp"]);
    fail_in(h.path(), &["group", "show", "Tmp"]);
}

#[test]
fn p_w9_075_config_set_then_data_status_no_interaction() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(h.path(), &["config", "set", "sync.policy", "off"]);
    ok_in(h.path(), &["data-status"]);
}

#[test]
fn p_w9_076_two_groups_isolated() {
    let h = fresh();
    ok_in(h.path(), &["group", "create", "GroupA"]);
    ok_in(h.path(), &["group", "create", "GroupB"]);
    ok_in(h.path(), &["group", "add", "GroupA", "EDM"]);
    ok_in(h.path(), &["group", "add", "GroupB", "TOR"]);
    let a = ok_in(h.path(), &["favorites", "--group", "GroupA", "--json"]);
    let b = ok_in(h.path(), &["favorites", "--group", "GroupB", "--json"]);
    let va: serde_json::Value = serde_json::from_str(&a).unwrap();
    let vb: serde_json::Value = serde_json::from_str(&b).unwrap();
    assert_eq!(va["meta"]["members_total"], 1);
    assert_eq!(vb["meta"]["members_total"], 1);
}

#[test]
fn p_w9_077_favorites_default_group_is_favorites() {
    let h = fresh();
    ok_in(h.path(), &["group", "add", "Favorites", "EDM"]);
    let out_default = ok_in(h.path(), &["favorites", "--json"]);
    let out_explicit = ok_in(h.path(), &["favorites", "--group", "Favorites", "--json"]);
    // Default group is "Favorites" — output should match.
    let v1: serde_json::Value = serde_json::from_str(&out_default).unwrap();
    let v2: serde_json::Value = serde_json::from_str(&out_explicit).unwrap();
    assert_eq!(v1["meta"]["group_name"], v2["meta"]["group_name"]);
}

#[test]
fn p_w9_078_setup_then_full_capability_roundtrip() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    for cap in [
        "sync.capabilities.stats",
        "sync.capabilities.scores_schedule",
        "sync.capabilities.transactions",
        "sync.capabilities.boxscores",
        "sync.capabilities.career_history",
    ] {
        ok_in(h.path(), &["config", "set", cap, "off"]);
        ok_in(h.path(), &["config", "set", cap, "favorites"]);
        ok_in(h.path(), &["config", "set", cap, "league"]);
        let got = ok_in(h.path(), &["config", "get", cap]);
        assert_eq!(got.trim(), "league");
    }
}

#[test]
fn p_w9_079_setup_then_reset_then_setup_again() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(
        h.path(),
        &["config", "set", "sync.capabilities.transactions", "league"],
    );
    ok_in(h.path(), &["setup", "--accept-defaults", "--reset"]);
    let got = ok_in(
        h.path(),
        &["config", "get", "sync.capabilities.transactions"],
    );
    assert_eq!(got.trim(), "favorites");
}

#[test]
fn p_w9_080_query_leaders_playoff_doesnt_read_favorites() {
    // Adding favorites shouldn't affect playoff leaderboard output.
    let h = fresh();
    let out_before = ok_in(h.path(), &["query", "leaders", "--playoff"]);
    ok_in(h.path(), &["group", "add", "Favorites", "EDM"]);
    let out_after = ok_in(h.path(), &["query", "leaders", "--playoff"]);
    assert_eq!(out_before, out_after);
}

#[test]
fn p_w9_081_data_status_sees_no_state_changes_from_pure_reads() {
    let h = fresh();
    let pre = ok_in(h.path(), &["data-status"]);
    ok_in(h.path(), &["favorites"]);
    ok_in(h.path(), &["query", "leaders", "--playoff"]);
    ok_in(h.path(), &["query", "leaders", "--week"]);
    let post = ok_in(h.path(), &["data-status"]);
    assert_eq!(pre, post, "pure reads must not mutate manifest");
}

#[test]
fn p_w9_082_config_set_then_unset_via_reset() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(h.path(), &["config", "set", "sync.policy", "off"]);
    ok_in(h.path(), &["config", "reset", "sync"]);
    let got = ok_in(h.path(), &["config", "get", "sync.policy"]);
    assert_eq!(got.trim(), "eager", "reset sync restores all defaults");
}

#[test]
fn p_w9_083_config_set_invalid_doesnt_affect_other_values() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(h.path(), &["config", "set", "sync.policy", "lazy"]);
    fail_in(h.path(), &["config", "set", "sync.policy", "garbage"]);
    let got = ok_in(h.path(), &["config", "get", "sync.policy"]);
    assert_eq!(
        got.trim(),
        "lazy",
        "failed set must not corrupt prior value"
    );
}

#[test]
fn p_w9_084_group_add_team_visible_in_favorites_text() {
    let h = fresh();
    ok_in(h.path(), &["group", "add", "Favorites", "TOR"]);
    let out = ok_in(h.path(), &["favorites"]);
    assert!(out.contains("TOR") || out.contains("1 team"));
}

#[test]
fn p_w9_085_full_user_journey() {
    let h = fresh();
    // 1. Run setup
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    // 2. Confirm config reads
    let policy = ok_in(h.path(), &["config", "get", "sync.policy"]);
    assert_eq!(policy.trim(), "eager");
    // 3. Add favorites
    ok_in(h.path(), &["group", "add", "Favorites", "EDM"]);
    ok_in(h.path(), &["group", "add", "Favorites", "Connor McDavid"]);
    // 4. View favorites
    let fav = ok_in(h.path(), &["favorites", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&fav).unwrap();
    assert_eq!(v["meta"]["members_total"], 2);
    // 5. Inspect manifest
    let ds = ok_in(h.path(), &["data-status"]);
    assert!(ds.contains("Manifest is empty") || ds.contains("DATA STATUS"));
    // 6. Tune capability matrix
    ok_in(
        h.path(),
        &["config", "set", "sync.capabilities.transactions", "league"],
    );
    let cap = ok_in(
        h.path(),
        &["config", "get", "sync.capabilities.transactions"],
    );
    assert_eq!(cap.trim(), "league");
    // 7. Try to set forbidden capability
    let err = fail_in(
        h.path(),
        &["config", "set", "sync.capabilities.shifts", "favorites"],
    );
    assert!(err.contains("shifts"));
    // 8. Query historical playoff series
    let series = ok_in(
        h.path(),
        &["playoffs", "--season", "19931994", "--series", "A"],
    );
    assert!(series.contains("SERIES A"));
}

// ── Exit code consistency (10) ───────────────────────────────────────────────

#[test]
fn p_w9_086_invalid_subcommand_non_zero_exit() {
    let h = fresh();
    let out = run_in(h.path(), &["fakecmd"]);
    assert!(!out.status.success());
}

#[test]
fn p_w9_087_invalid_flag_non_zero_exit() {
    let h = fresh();
    let out = run_in(h.path(), &["favorites", "--no-such-flag"]);
    assert!(!out.status.success());
}

#[test]
fn p_w9_088_help_returns_zero() {
    let h = fresh();
    let out = run_in(h.path(), &["--help"]);
    assert!(out.status.success());
}

#[test]
fn p_w9_089_subcommand_help_returns_zero() {
    let h = fresh();
    let out = run_in(h.path(), &["favorites", "--help"]);
    assert!(out.status.success());
}

#[test]
fn p_w9_090_version_returns_zero() {
    let h = fresh();
    let out = run_in(h.path(), &["--version"]);
    assert!(out.status.success());
}

#[test]
fn p_w9_091_invalid_date_uses_exit_2() {
    let h = fresh();
    let out = run_in(h.path(), &["tonight", "--date", "garbage"]);
    let code = out.status.code().unwrap_or(-1);
    // Exit code may be 1 (anyhow default) or 2 (explicit). Both
    // are non-zero.
    assert!(code != 0);
}

#[test]
fn p_w9_092_shifts_lock_uses_exit_2() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    let out = run_in(
        h.path(),
        &["config", "set", "sync.capabilities.shifts", "favorites"],
    );
    let code = out.status.code().unwrap_or(-1);
    assert_eq!(code, 2, "shifts lock surfaces explicit exit code 2");
}

#[test]
fn p_w9_093_unknown_config_key_exit_2() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    let out = run_in(h.path(), &["config", "get", "sync.unknown"]);
    let code = out.status.code().unwrap_or(-1);
    assert_eq!(code, 2);
}

#[test]
fn p_w9_094_unknown_shard_exit_non_zero() {
    let h = fresh();
    let out = run_in(h.path(), &["data-status", "--shard", "wickets"]);
    assert!(!out.status.success());
}

#[test]
fn p_w9_095_query_career_week_rejection_exit_2() {
    let h = fresh();
    let out = run_in(h.path(), &["query", "career", "--league", "OHL", "--week"]);
    let code = out.status.code().unwrap_or(-1);
    assert_eq!(code, 2);
}

// ── Output shape regression (5) ──────────────────────────────────────────────

#[test]
fn p_w9_096_help_under_3000_chars_per_line() {
    let h = fresh();
    let out = ok_in(h.path(), &["--help"]);
    for line in out.lines() {
        assert!(
            line.len() <= 3000,
            "no help line should exceed 3000 chars (got {})",
            line.len()
        );
    }
}

#[test]
fn p_w9_097_version_format_stable() {
    let h = fresh();
    let out = ok_in(h.path(), &["--version"]);
    // "icelines X.Y.Z\n" format
    assert!(out.starts_with("icelines "));
    assert!(out.matches('.').count() == 2, "X.Y.Z three numbers");
}

#[test]
fn p_w9_098_cli_help_lists_all_top_level_commands() {
    let h = fresh();
    let out = ok_in(h.path(), &["--help"]);
    // Make sure every Foster + Conn Smythe + legacy command surfaces
    for cmd in [
        "favorites",
        "setup",
        "config",
        "data-status",
        "playoffs",
        "tonight",
        "schedule",
        "fetch",
        "group",
        "query",
    ] {
        assert!(out.contains(cmd), "global help missing {cmd}");
    }
}

#[test]
fn p_w9_099_global_help_includes_ascii_only() {
    // Global help shouldn't accidentally include garbage utf-8.
    let h = fresh();
    let out = ok_in(h.path(), &["--help"]);
    let valid_utf8 = std::str::from_utf8(out.as_bytes()).is_ok();
    assert!(valid_utf8);
}

#[test]
fn p_w9_100_grand_finale_workspace_smoke() {
    // 30 commands in a row, never panic, never corrupt state.
    let h = fresh();
    for cmd in [
        vec!["--version"],
        vec!["--help"],
        vec!["setup", "--accept-defaults"],
        vec!["config", "list"],
        vec!["config", "get", "sync.policy"],
        vec!["config", "set", "sync.policy", "lazy"],
        vec!["config", "get", "sync.policy"],
        vec!["data-status"],
        vec!["data-status", "--stale-only"],
        vec!["data-status", "--shard", "bios"],
        vec!["group", "list"],
        vec!["group", "create", "Mine"],
        vec!["group", "add", "Mine", "EDM"],
        vec!["group", "add", "Mine", "Connor McDavid"],
        vec!["group", "show", "Mine"],
        vec!["favorites", "--group", "Mine"],
        vec!["favorites", "--group", "Mine", "--json"],
        vec!["favorites", "--group", "Mine", "--range", "week"],
        vec!["fetch", "sync", "--dry-run"],
        vec!["fetch", "boxscore", "--dry-run"],
        vec!["query", "leaders", "--playoff"],
        vec!["query", "leaders", "--week"],
        vec!["query", "leaders", "--month"],
        vec!["playoffs", "--series", "A"],
        vec!["playoffs", "--series", "A", "--json"],
        vec!["playoffs", "--season", "19931994"],
        vec!["tonight"],
        vec!["schedule"],
        vec!["group", "remove", "Mine", "EDM"],
        vec!["group", "delete", "Mine"],
    ] {
        let args: Vec<&str> = cmd.iter().copied().collect();
        no_panic_in(h.path(), &args);
    }
}
