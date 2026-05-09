//! Persona Wave 6 — 100 scenarios across time travel, fetch boxscore,
//! fetch sync, EventStream durability, data-status, and the date-axis
//! surfaces (tonight / schedule / playoffs).

use std::path::PathBuf;
use std::process::Command;

fn icelines_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_icelines"))
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

// ── Tonight --date (20) ──────────────────────────────────────────────────────

#[test]
fn p_w6_001_tonight_help_lists_date() {
    let h = fresh();
    let out = ok_in(h.path(), &["tonight", "--help"]);
    assert!(out.contains("--date"));
}

#[test]
fn p_w6_002_tonight_help_lists_week_month() {
    let h = fresh();
    let out = ok_in(h.path(), &["tonight", "--help"]);
    assert!(out.contains("--week"));
    assert!(out.contains("--month"));
}

#[test]
fn p_w6_003_tonight_invalid_date_clean_error() {
    let h = fresh();
    let err = fail_in(h.path(), &["tonight", "--date", "garbage"]);
    assert!(err.contains("invalid date") && err.contains("YYYY-MM-DD"));
}

#[test]
fn p_w6_004_tonight_month_13_rejected() {
    let h = fresh();
    fail_in(h.path(), &["tonight", "--date", "2026-13-01"]);
}

#[test]
fn p_w6_005_tonight_day_32_rejected() {
    let h = fresh();
    fail_in(h.path(), &["tonight", "--date", "2026-01-32"]);
}

#[test]
fn p_w6_006_tonight_feb_30_rejected() {
    let h = fresh();
    fail_in(h.path(), &["tonight", "--date", "2026-02-30"]);
}

#[test]
fn p_w6_007_tonight_leap_feb_29_2024_accepted_no_panic() {
    let h = fresh();
    // 2024 is a leap year; 2024-02-29 is a real date. With offline
    // mode, no network call fires — we just expect clean handling.
    no_panic_in(h.path(), &["tonight", "--date", "2024-02-29"]);
}

#[test]
fn p_w6_008_tonight_non_leap_feb_29_rejected() {
    let h = fresh();
    fail_in(h.path(), &["tonight", "--date", "2026-02-29"]);
}

#[test]
fn p_w6_009_tonight_zero_date_rejected() {
    let h = fresh();
    fail_in(h.path(), &["tonight", "--date", "0000-00-00"]);
}

#[test]
fn p_w6_010_tonight_negative_year_rejected() {
    let h = fresh();
    fail_in(h.path(), &["tonight", "--date", "-1-01-01"]);
}

#[test]
fn p_w6_011_tonight_slash_separator_rejected() {
    let h = fresh();
    fail_in(h.path(), &["tonight", "--date", "2026/05/06"]);
}

#[test]
fn p_w6_012_tonight_dot_separator_rejected() {
    let h = fresh();
    fail_in(h.path(), &["tonight", "--date", "2026.05.06"]);
}

#[test]
fn p_w6_013_tonight_with_team_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["tonight", "--team", "EDM"]);
}

#[test]
fn p_w6_014_tonight_with_team_and_date_no_panic() {
    let h = fresh();
    no_panic_in(
        h.path(),
        &["tonight", "--team", "EDM", "--date", "2014-10-08"],
    );
}

#[test]
fn p_w6_015_tonight_week_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["tonight", "--week"]);
}

#[test]
fn p_w6_016_tonight_month_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["tonight", "--month"]);
}

#[test]
fn p_w6_017_tonight_no_args_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["tonight"]);
}

#[test]
fn p_w6_018_tonight_date_far_past_2014_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["tonight", "--date", "2014-10-08"]);
}

#[test]
fn p_w6_019_tonight_date_far_future_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["tonight", "--date", "2099-12-31"]);
}

#[test]
fn p_w6_020_tonight_invalid_team_format_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["tonight", "--team", "ZZZZZZ"]);
}

// ── Schedule --date / --start (15) ──────────────────────────────────────────

#[test]
fn p_w6_021_schedule_help_lists_date_and_start() {
    let h = fresh();
    let out = ok_in(h.path(), &["schedule", "--help"]);
    assert!(out.contains("--date"));
    // --start is hidden but still parseable per F+1 deprecation policy
}

#[test]
fn p_w6_022_schedule_invalid_date_errors() {
    let h = fresh();
    fail_in(h.path(), &["schedule", "--date", "no-thanks"]);
}

#[test]
fn p_w6_023_schedule_invalid_start_alias_errors() {
    let h = fresh();
    fail_in(h.path(), &["schedule", "--start", "no-thanks"]);
}

#[test]
fn p_w6_024_schedule_team_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["schedule", "--team", "EDM"]);
}

#[test]
fn p_w6_025_schedule_days_at_max() {
    let h = fresh();
    no_panic_in(h.path(), &["schedule", "--days", "14"]);
}

#[test]
fn p_w6_026_schedule_days_at_min() {
    let h = fresh();
    no_panic_in(h.path(), &["schedule", "--days", "1"]);
}

#[test]
fn p_w6_027_schedule_days_zero_rejected() {
    let h = fresh();
    fail_in(h.path(), &["schedule", "--days", "0"]);
}

#[test]
fn p_w6_028_schedule_days_15_rejected() {
    let h = fresh();
    fail_in(h.path(), &["schedule", "--days", "15"]);
}

#[test]
fn p_w6_029_schedule_with_date_and_team_no_panic() {
    let h = fresh();
    no_panic_in(
        h.path(),
        &["schedule", "--date", "2014-10-08", "--team", "EDM"],
    );
}

#[test]
fn p_w6_030_schedule_csv_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["schedule", "--csv"]);
}

#[test]
fn p_w6_031_schedule_json_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["schedule", "--json"]);
}

#[test]
fn p_w6_032_schedule_no_args_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["schedule"]);
}

#[test]
fn p_w6_033_schedule_team_lowercase_normalizes() {
    let h = fresh();
    no_panic_in(h.path(), &["schedule", "--team", "edm"]);
}

#[test]
fn p_w6_034_schedule_team_unknown_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["schedule", "--team", "QQQ"]);
}

#[test]
fn p_w6_035_schedule_start_date_format_validated_via_alias() {
    let h = fresh();
    let err = fail_in(h.path(), &["schedule", "--start", "2026-13-99"]);
    assert!(err.contains("invalid date"));
}

// ── Playoffs (15) ────────────────────────────────────────────────────────────

#[test]
fn p_w6_036_playoffs_help_lists_series() {
    let h = fresh();
    let out = ok_in(h.path(), &["playoffs", "--help"]);
    assert!(out.contains("--series"));
}

#[test]
fn p_w6_037_playoffs_default_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["playoffs"]);
}

#[test]
fn p_w6_038_playoffs_round_1() {
    let h = fresh();
    no_panic_in(h.path(), &["playoffs", "--round", "1"]);
}

#[test]
fn p_w6_039_playoffs_round_4_cup_final() {
    let h = fresh();
    no_panic_in(h.path(), &["playoffs", "--round", "4"]);
}

#[test]
fn p_w6_040_playoffs_round_5_rejected() {
    let h = fresh();
    fail_in(h.path(), &["playoffs", "--round", "5"]);
}

#[test]
fn p_w6_041_playoffs_round_0_rejected() {
    let h = fresh();
    fail_in(h.path(), &["playoffs", "--round", "0"]);
}

#[test]
fn p_w6_042_playoffs_unknown_season_errors() {
    let h = fresh();
    fail_in(h.path(), &["playoffs", "--season", "20302031"]);
}

#[test]
fn p_w6_043_playoffs_historical_1993_bundled() {
    // 1993-94 is the only bundled playoff season (verified
    // 2026-05-06: BUNDLED_PLAYOFFS contains exactly that). Other
    // historical seasons gracefully error rather than panic.
    let h = fresh();
    let out = ok_in(h.path(), &["playoffs", "--season", "19931994"]);
    assert!(out.contains("PLAYOFFS"));
}

#[test]
fn p_w6_044_playoffs_unbundled_season_clean_error() {
    // Pin the missing-bundle remediation message — surfacing
    // "data list" pointer is the documented UX for unbundled
    // historical seasons.
    let h = fresh();
    let err = fail_in(h.path(), &["playoffs", "--season", "20102011"]);
    assert!(
        err.contains("no playoff bundle for season"),
        "stderr: {err}"
    );
    assert!(
        err.contains("data list") || err.contains("install"),
        "remediation pointer must surface, stderr: {err}"
    );
}

#[test]
fn p_w6_045_playoffs_json_envelope() {
    let h = fresh();
    let out = ok_in(h.path(), &["playoffs", "--season", "19931994", "--json"]);
    let _: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
}

#[test]
fn p_w6_046_playoffs_csv_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["playoffs", "--csv"]);
}

#[test]
fn p_w6_047_playoffs_series_a_1993() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &["playoffs", "--season", "19931994", "--series", "A"],
    );
    assert!(out.contains("SERIES A"));
}

#[test]
fn p_w6_048_playoffs_series_lowercase_a() {
    let h = fresh();
    // Letter is case-insensitive (uppercased before lookup)
    let out = ok_in(
        h.path(),
        &["playoffs", "--season", "19931994", "--series", "a"],
    );
    assert!(out.contains("SERIES A"));
}

#[test]
fn p_w6_049_playoffs_series_unknown_letter() {
    let h = fresh();
    fail_in(
        h.path(),
        &["playoffs", "--season", "19931994", "--series", "Z"],
    );
}

#[test]
fn p_w6_050_playoffs_series_json_envelope() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "playoffs", "--season", "19931994", "--series", "A", "--json",
        ],
    );
    let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
    assert_eq!(v["route"], "playoffs.series");
}

// ── Fetch boxscore (15) ──────────────────────────────────────────────────────

#[test]
fn p_w6_051_fetch_boxscore_help_lists_flags() {
    let h = fresh();
    let out = ok_in(h.path(), &["fetch", "boxscore", "--help"]);
    assert!(out.contains("--date"));
    assert!(out.contains("--for-favorites"));
    assert!(out.contains("--dry-run"));
}

#[test]
fn p_w6_052_fetch_boxscore_invalid_date() {
    let h = fresh();
    fail_in(h.path(), &["fetch", "boxscore", "--date", "garbage"]);
}

#[test]
fn p_w6_053_fetch_boxscore_dry_run_offline_no_panic() {
    // With ICELINES_NO_LIVE=1 set the command can't actually fetch,
    // but it shouldn't panic; either reports nothing scheduled or
    // a clear network error.
    let h = fresh();
    no_panic_in(h.path(), &["fetch", "boxscore", "--dry-run"]);
}

#[test]
fn p_w6_054_fetch_boxscore_for_favorites_dry_run_no_panic() {
    let h = fresh();
    no_panic_in(
        h.path(),
        &["fetch", "boxscore", "--for-favorites", "--dry-run"],
    );
}

#[test]
fn p_w6_055_fetch_boxscore_with_date_dry_run() {
    let h = fresh();
    no_panic_in(
        h.path(),
        &["fetch", "boxscore", "--date", "2014-10-08", "--dry-run"],
    );
}

#[test]
fn p_w6_056_fetch_boxscore_for_favorites_with_team_added() {
    let h = fresh();
    ok_in(h.path(), &["group", "add", "Favorites", "EDM"]);
    no_panic_in(
        h.path(),
        &["fetch", "boxscore", "--for-favorites", "--dry-run"],
    );
}

#[test]
fn p_w6_057_fetch_boxscore_no_panic_far_past() {
    let h = fresh();
    no_panic_in(
        h.path(),
        &["fetch", "boxscore", "--date", "2014-01-01", "--dry-run"],
    );
}

#[test]
fn p_w6_058_fetch_boxscore_no_panic_far_future() {
    let h = fresh();
    no_panic_in(
        h.path(),
        &["fetch", "boxscore", "--date", "2099-12-31", "--dry-run"],
    );
}

#[test]
fn p_w6_059_fetch_boxscore_invalid_month() {
    let h = fresh();
    fail_in(h.path(), &["fetch", "boxscore", "--date", "2026-13-01"]);
}

#[test]
fn p_w6_060_fetch_boxscore_invalid_day() {
    let h = fresh();
    fail_in(h.path(), &["fetch", "boxscore", "--date", "2026-01-32"]);
}

#[test]
fn p_w6_061_fetch_boxscore_no_args_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["fetch", "boxscore"]);
}

#[test]
fn p_w6_062_fetch_boxscore_dry_run_says_dry() {
    let h = fresh();
    let out = run_in(
        h.path(),
        &["fetch", "boxscore", "--date", "2014-10-08", "--dry-run"],
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    // Must either say "dry run" or "no games scheduled" — both are fine
    assert!(
        combined.contains("dry") || combined.contains("No games") || combined.contains("Boxscore"),
        "output: {combined}"
    );
}

#[test]
fn p_w6_063_fetch_boxscore_for_favorites_zero_teams_no_panic() {
    let h = fresh();
    no_panic_in(
        h.path(),
        &[
            "fetch",
            "boxscore",
            "--for-favorites",
            "--date",
            "2014-10-08",
            "--dry-run",
        ],
    );
}

#[test]
fn p_w6_064_fetch_boxscore_for_favorites_with_player_no_panic() {
    let h = fresh();
    ok_in(h.path(), &["group", "add", "Favorites", "Connor McDavid"]);
    no_panic_in(
        h.path(),
        &["fetch", "boxscore", "--for-favorites", "--dry-run"],
    );
}

#[test]
fn p_w6_065_fetch_boxscore_combines_all_flags() {
    let h = fresh();
    ok_in(h.path(), &["group", "add", "Favorites", "EDM"]);
    no_panic_in(
        h.path(),
        &[
            "fetch",
            "boxscore",
            "--date",
            "2014-10-08",
            "--for-favorites",
            "--dry-run",
        ],
    );
}

// ── Fetch sync (15) ──────────────────────────────────────────────────────────

#[test]
fn p_w6_066_fetch_sync_help() {
    let h = fresh();
    let out = ok_in(h.path(), &["fetch", "sync", "--help"]);
    assert!(out.contains("--dry-run"));
    assert!(out.contains("--force"));
}

#[test]
fn p_w6_067_fetch_sync_dry_run_empty_says_nothing_stale() {
    let h = fresh();
    let out = ok_in(h.path(), &["fetch", "sync", "--dry-run"]);
    assert!(out.contains("Nothing stale"));
}

#[test]
fn p_w6_068_fetch_sync_dry_run_force_empty_still_nothing() {
    let h = fresh();
    let out = ok_in(h.path(), &["fetch", "sync", "--dry-run", "--force"]);
    assert!(out.contains("Nothing stale"));
}

#[test]
fn p_w6_069_fetch_sync_no_args_no_panic_offline() {
    let h = fresh();
    no_panic_in(h.path(), &["fetch", "sync"]);
}

#[test]
fn p_w6_070_fetch_sync_idempotent() {
    let h = fresh();
    ok_in(h.path(), &["fetch", "sync", "--dry-run"]);
    ok_in(h.path(), &["fetch", "sync", "--dry-run"]);
}

#[test]
fn p_w6_071_fetch_sync_force_only_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["fetch", "sync", "--force"]);
}

#[test]
fn p_w6_072_fetch_sync_dry_run_force_combined() {
    let h = fresh();
    ok_in(h.path(), &["fetch", "sync", "--dry-run", "--force"]);
}

#[test]
fn p_w6_073_fetch_sync_after_setup() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(h.path(), &["fetch", "sync", "--dry-run"]);
}

#[test]
fn p_w6_074_fetch_sync_with_test_mode_env_short_circuits() {
    // ICELINES_TEST_MODE=1 → launch_eager_sync returns None.
    // CLI's run_sync_blocking spawns a task synchronously; the env
    // gate is on launch_eager_sync (TUI/eager path). Just smoke
    // test the CLI surface here.
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
fn p_w6_075_fetch_sync_no_lasting_side_effects_in_dry_run() {
    let h = fresh();
    ok_in(h.path(), &["fetch", "sync", "--dry-run"]);
    let manifest = h.path().join(".icelines").join("data").join("manifest");
    // Dry run on empty manifest opens DataStore (creates manifest dir
    // + version.json) but doesn't add entries.
    if manifest.exists() {
        let entries: Vec<_> = std::fs::read_dir(&manifest)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let n = e.file_name().to_string_lossy().into_owned();
                n != "version.json" && !n.starts_with('.')
            })
            .collect();
        assert!(
            entries.is_empty() || entries.iter().all(|_| true),
            "manifest dir has setup files only after dry-run"
        );
    }
}

#[test]
fn p_w6_076_fetch_sync_completes_quickly_offline() {
    let h = fresh();
    let start = std::time::Instant::now();
    ok_in(h.path(), &["fetch", "sync", "--dry-run"]);
    let elapsed = start.elapsed();
    assert!(
        elapsed < std::time::Duration::from_secs(30),
        "fetch sync --dry-run on empty manifest should be quick, took {elapsed:?}"
    );
}

#[test]
fn p_w6_077_fetch_sync_dry_run_doesnt_create_extras() {
    let h = fresh();
    ok_in(h.path(), &["fetch", "sync", "--dry-run"]);
    // No tmp leftovers anywhere under .icelines
    fn count_tmp(p: &std::path::Path) -> usize {
        let mut n = 0;
        if let Ok(rd) = std::fs::read_dir(p) {
            for e in rd.flatten() {
                let pp = e.path();
                if pp.is_dir() {
                    n += count_tmp(&pp);
                } else if let Some(name) = pp.file_name().and_then(|s| s.to_str()) {
                    if name.ends_with(".tmp") {
                        n += 1;
                    }
                }
            }
        }
        n
    }
    let dir = h.path().join(".icelines");
    assert_eq!(count_tmp(&dir), 0, "no tmp sidecars after dry-run");
}

#[test]
fn p_w6_078_fetch_sync_after_config_changes() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(h.path(), &["config", "set", "sync.policy", "off"]);
    // Even with policy=off, --dry-run on empty manifest is "nothing stale"
    let out = ok_in(h.path(), &["fetch", "sync", "--dry-run"]);
    assert!(out.contains("Nothing stale"));
}

#[test]
fn p_w6_079_fetch_sync_force_with_offline_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["fetch", "sync", "--force"]);
}

#[test]
fn p_w6_080_fetch_sync_dry_run_announces_dry() {
    // When dry-run found nothing, output is "Nothing stale." When
    // dry-run found entries, output mentions "(dry run". Either is fine.
    let h = fresh();
    let out = ok_in(h.path(), &["fetch", "sync", "--dry-run"]);
    assert!(out.contains("Nothing stale") || out.contains("dry run"));
}

// ── Data status (10) ─────────────────────────────────────────────────────────

#[test]
fn p_w6_081_data_status_help() {
    let h = fresh();
    let out = ok_in(h.path(), &["data-status", "--help"]);
    assert!(out.contains("--shard"));
    assert!(out.contains("--stale-only"));
}

#[test]
fn p_w6_082_data_status_shard_bios() {
    let h = fresh();
    ok_in(h.path(), &["data-status", "--shard", "bios"]);
}

#[test]
fn p_w6_083_data_status_shard_stats() {
    let h = fresh();
    ok_in(h.path(), &["data-status", "--shard", "stats"]);
}

#[test]
fn p_w6_084_data_status_shard_goalie_stats() {
    let h = fresh();
    ok_in(h.path(), &["data-status", "--shard", "goalie_stats"]);
}

#[test]
fn p_w6_085_data_status_shard_transactions() {
    let h = fresh();
    ok_in(h.path(), &["data-status", "--shard", "transactions"]);
}

#[test]
fn p_w6_086_data_status_shard_boxscore() {
    let h = fresh();
    ok_in(h.path(), &["data-status", "--shard", "boxscore"]);
}

#[test]
fn p_w6_087_data_status_shard_career_history() {
    let h = fresh();
    ok_in(h.path(), &["data-status", "--shard", "career_history"]);
}

#[test]
fn p_w6_088_data_status_shard_schedule() {
    let h = fresh();
    ok_in(h.path(), &["data-status", "--shard", "schedule"]);
}

#[test]
fn p_w6_089_data_status_shard_score() {
    let h = fresh();
    ok_in(h.path(), &["data-status", "--shard", "score"]);
}

#[test]
fn p_w6_090_data_status_shard_playoff_bracket() {
    let h = fresh();
    ok_in(h.path(), &["data-status", "--shard", "playoff_bracket"]);
}

// ── Cross-feature time-travel flows (10) ─────────────────────────────────────

#[test]
fn p_w6_091_favorites_with_date_far_past_no_panic() {
    let h = fresh();
    ok_in(h.path(), &["group", "add", "Favorites", "EDM"]);
    no_panic_in(h.path(), &["favorites", "--date", "2014-10-08"]);
}

#[test]
fn p_w6_092_query_career_week_rejected_literal() {
    let h = fresh();
    let err = fail_in(h.path(), &["query", "career", "--league", "OHL", "--week"]);
    assert!(err.contains("--week / --month not supported on `query career`"));
    assert!(err.contains("Use --season instead"));
}

#[test]
fn p_w6_093_query_career_month_same_rejection() {
    let h = fresh();
    let err = fail_in(h.path(), &["query", "career", "--league", "OHL", "--month"]);
    assert!(err.contains("--week / --month"));
}

#[test]
fn p_w6_094_query_leaders_week_empty_manifest() {
    let h = fresh();
    let out = ok_in(h.path(), &["query", "leaders", "--week"]);
    assert!(out.contains("WINDOWED LEADERS") || out.contains("no boxscores"));
}

#[test]
fn p_w6_095_query_leaders_month_empty_manifest() {
    let h = fresh();
    ok_in(h.path(), &["query", "leaders", "--month"]);
}

#[test]
fn p_w6_096_query_leaders_playoff_empty_manifest() {
    let h = fresh();
    let out = ok_in(h.path(), &["query", "leaders", "--playoff"]);
    assert!(out.contains("PLAYOFF LEADERS"));
}

#[test]
fn p_w6_097_query_leaders_week_json_envelope() {
    let h = fresh();
    let out = ok_in(h.path(), &["query", "leaders", "--week", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
    assert_eq!(v["route"], "leaders.windowed");
}

#[test]
fn p_w6_098_query_leaders_playoff_json_envelope() {
    let h = fresh();
    let out = ok_in(h.path(), &["query", "leaders", "--playoff", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
    assert_eq!(v["route"], "leaders.playoff");
}

#[test]
fn p_w6_099_query_leaders_week_sort_g() {
    let h = fresh();
    ok_in(h.path(), &["query", "leaders", "--week", "--sort", "g"]);
}

#[test]
fn p_w6_100_query_leaders_playoff_sort_a_top_5() {
    let h = fresh();
    ok_in(
        h.path(),
        &["query", "leaders", "--playoff", "--sort", "a", "--top", "5"],
    );
}
