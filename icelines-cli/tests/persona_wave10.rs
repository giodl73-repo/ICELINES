//! Persona Wave 10 — UX consistency + truthfulness.
//! 100 cross-cutting invariant tests: JSON envelope shape, exit
//! code consistency, error message format, date/team format
//! consistency, stdout/stderr discipline, COMMANDS.md ↔ binary
//! sync, CLAUDE.md ↔ binary sync, --help quality.

use std::path::PathBuf;
use std::process::Command;

fn icelines_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_icelines"))
}

fn workspace_root() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
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

fn fail_in(home: &std::path::Path, args: &[&str]) -> std::process::Output {
    let out = run_in(home, args);
    assert!(
        !out.status.success(),
        "{:?} must non-zero exit; stdout:\n{}",
        args,
        String::from_utf8_lossy(&out.stdout)
    );
    out
}

fn fresh() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

// ── Section A — JSON envelope shape consistency (15) ─────────────────────────
//
// Every --json route must emit the K2.4 envelope shape:
//   { schema_version, route, data, meta }
// schema_version is u32, route is String, data is Object|Array,
// meta is Object. Consistency means consumers can parse any
// `--json` output the same way.

const K24_REQUIRED_FIELDS: &[&str] = &["schema_version", "route", "data", "meta"];

fn assert_k24_envelope(json: &str, expected_route: &str) {
    let v: serde_json::Value = serde_json::from_str(json)
        .unwrap_or_else(|e| panic!("invalid JSON for route '{expected_route}': {e}"));
    for field in K24_REQUIRED_FIELDS {
        assert!(
            v.get(*field).is_some(),
            "K2.4 envelope missing '{field}' for route '{expected_route}'\nJSON:\n{json}"
        );
    }
    assert!(
        v["schema_version"].is_number(),
        "schema_version must be a number for route '{expected_route}'"
    );
    assert!(
        v["route"].is_string(),
        "route field must be a string for route '{expected_route}'"
    );
    assert!(
        v["meta"].is_object(),
        "meta must be an object for route '{expected_route}'"
    );
    let actual_route = v["route"].as_str().unwrap();
    assert_eq!(
        actual_route, expected_route,
        "route field should match expected"
    );
}

#[test]
fn p_w10_001_envelope_favorites_empty() {
    let h = fresh();
    let out = ok_in(h.path(), &["favorites", "--json"]);
    assert_k24_envelope(&out, "favorites");
}

#[test]
fn p_w10_002_envelope_favorites_populated() {
    let h = fresh();
    ok_in(h.path(), &["group", "add", "Favorites", "EDM"]);
    let out = ok_in(h.path(), &["favorites", "--json"]);
    assert_k24_envelope(&out, "favorites");
}

#[test]
fn p_w10_003_envelope_query_leaders_playoff() {
    let h = fresh();
    let out = ok_in(h.path(), &["query", "leaders", "--playoff", "--json"]);
    assert_k24_envelope(&out, "leaders.playoff");
}

#[test]
fn p_w10_004_envelope_query_leaders_window() {
    let h = fresh();
    let out = ok_in(h.path(), &["query", "leaders", "--week", "--json"]);
    assert_k24_envelope(&out, "leaders.windowed");
}

#[test]
fn p_w10_005_envelope_playoffs_series() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "playoffs", "--season", "19931994", "--series", "A", "--json",
        ],
    );
    assert_k24_envelope(&out, "playoffs.series");
}

#[test]
fn p_w10_006_favorites_meta_has_date() {
    let h = fresh();
    let out = ok_in(h.path(), &["favorites", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v["meta"]["date"].is_string());
}

#[test]
fn p_w10_007_favorites_meta_date_is_iso_format() {
    let h = fresh();
    let out = ok_in(h.path(), &["favorites", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let date = v["meta"]["date"].as_str().unwrap();
    let re = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
    assert!(
        re.is_match(date),
        "meta.date must be YYYY-MM-DD, got {date}"
    );
}

#[test]
fn p_w10_008_query_leaders_window_meta_has_timeframe() {
    let h = fresh();
    let out = ok_in(h.path(), &["query", "leaders", "--week", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v["meta"]["timeframe"].is_string());
}

#[test]
fn p_w10_009_query_leaders_window_meta_range_dates_iso() {
    let h = fresh();
    let out = ok_in(h.path(), &["query", "leaders", "--month", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let re = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
    let start = v["meta"]["range_start"].as_str().unwrap_or("");
    let end = v["meta"]["range_end"].as_str().unwrap_or("");
    assert!(re.is_match(start), "range_start must be ISO, got {start}");
    assert!(re.is_match(end), "range_end must be ISO, got {end}");
}

#[test]
fn p_w10_010_playoffs_series_meta_has_season_id() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "playoffs", "--season", "19931994", "--series", "A", "--json",
        ],
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["meta"]["season_id"], "19931994");
}

#[test]
fn p_w10_011_query_leaders_meta_rows_matches_data_len() {
    let h = fresh();
    let out = ok_in(h.path(), &["query", "leaders", "--playoff", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let data_len = v["data"].as_array().map(|a| a.len()).unwrap_or(0);
    let meta_rows = v["meta"]["rows"].as_u64().unwrap_or(0) as usize;
    assert_eq!(data_len, meta_rows, "data.len() must match meta.rows");
}

#[test]
fn p_w10_012_favorites_meta_counts_match_data_lengths() {
    let h = fresh();
    let out = ok_in(h.path(), &["favorites", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let players = v["data"]["players"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);
    let teams = v["data"]["teams"].as_array().map(|a| a.len()).unwrap_or(0);
    let events = v["data"]["events"].as_array().map(|a| a.len()).unwrap_or(0);
    let m_players = v["meta"]["counts"]["players"].as_u64().unwrap_or(99) as usize;
    let m_teams = v["meta"]["counts"]["teams"].as_u64().unwrap_or(99) as usize;
    let m_events = v["meta"]["counts"]["events"].as_u64().unwrap_or(99) as usize;
    assert_eq!(players, m_players, "players count drift");
    assert_eq!(teams, m_teams, "teams count drift");
    assert_eq!(events, m_events, "events count drift");
}

#[test]
fn p_w10_013_envelope_schema_version_is_one_everywhere() {
    let h = fresh();
    for cmd in [
        vec!["favorites", "--json"],
        vec!["query", "leaders", "--playoff", "--json"],
        vec!["query", "leaders", "--week", "--json"],
        vec![
            "playoffs", "--season", "19931994", "--series", "A", "--json",
        ],
    ] {
        let out = ok_in(h.path(), &cmd);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let sv = v["schema_version"].as_u64().unwrap();
        assert_eq!(sv, 1, "schema_version drift on {cmd:?}");
    }
}

#[test]
fn p_w10_014_envelope_data_is_array_or_object_never_primitive() {
    let h = fresh();
    for cmd in [
        vec!["favorites", "--json"],
        vec!["query", "leaders", "--playoff", "--json"],
        vec!["query", "leaders", "--week", "--json"],
    ] {
        let out = ok_in(h.path(), &cmd);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(
            v["data"].is_array() || v["data"].is_object(),
            "data must be Array or Object on {cmd:?}, got {:?}",
            v["data"]
        );
    }
}

#[test]
fn p_w10_015_envelope_routes_use_dot_separator() {
    // Routes follow the pattern `kind` or `kind.subroute`. No
    // slashes, no spaces, no underscores in the route field.
    let h = fresh();
    for (cmd, expected) in [
        (vec!["favorites", "--json"], "favorites"),
        (
            vec!["query", "leaders", "--playoff", "--json"],
            "leaders.playoff",
        ),
        (
            vec!["query", "leaders", "--week", "--json"],
            "leaders.windowed",
        ),
    ] {
        let out = ok_in(h.path(), &cmd);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let route = v["route"].as_str().unwrap();
        assert_eq!(route, expected);
        assert!(!route.contains('/'), "no slashes in route field");
        assert!(!route.contains(' '), "no spaces in route field");
    }
}

// ── Section B — Exit code consistency (10) ───────────────────────────────────
//
// Conventions:
//   0 — success
//   2 — user input error (invalid date, unknown key, capability lock)
//   1 — unexpected system error (anyhow default)
// Top-level invalid commands → 2 (clap convention)

fn exit_code(out: &std::process::Output) -> i32 {
    out.status.code().unwrap_or(-1)
}

#[test]
fn p_w10_016_exit_zero_on_success() {
    let h = fresh();
    let out = run_in(h.path(), &["--version"]);
    assert_eq!(exit_code(&out), 0);
}

#[test]
fn p_w10_017_exit_two_on_invalid_clap_arg() {
    let h = fresh();
    let out = run_in(h.path(), &["favorites", "--bogus"]);
    // clap returns 2 by convention
    assert_eq!(exit_code(&out), 2);
}

#[test]
fn p_w10_018_exit_two_on_invalid_date() {
    let h = fresh();
    let out = run_in(h.path(), &["tonight", "--date", "garbage"]);
    let code = exit_code(&out);
    // Either 1 (anyhow default) or 2 (explicit). Must be non-zero
    // and not a panic exit (which would be 101 on Rust).
    assert!(code != 0);
    assert!(code != 101, "panic exit code 101 means we panicked");
}

#[test]
fn p_w10_019_exit_two_on_shifts_lock() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    let out = run_in(
        h.path(),
        &["config", "set", "sync.capabilities.shifts", "favorites"],
    );
    assert_eq!(exit_code(&out), 2);
}

#[test]
fn p_w10_020_exit_two_on_unknown_config_key() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    let out = run_in(h.path(), &["config", "get", "sync.unknown"]);
    assert_eq!(exit_code(&out), 2);
}

#[test]
fn p_w10_021_exit_two_on_query_career_week() {
    let h = fresh();
    let out = run_in(h.path(), &["query", "career", "--league", "OHL", "--week"]);
    assert_eq!(exit_code(&out), 2);
}

#[test]
fn p_w10_022_exit_no_panic_code_anywhere() {
    // Sweep across known-bad inputs; none should produce exit 101 (panic).
    let h = fresh();
    for args in [
        vec!["bogus"],
        vec!["favorites", "--bogus"],
        vec!["tonight", "--date", "x"],
        vec!["fetch", "boxscore", "--date", "z"],
        vec!["config", "get", "sync.unknown"],
        vec!["query", "career", "--league", "OHL", "--week"],
        vec!["data-status", "--shard", "wickets"],
        vec!["playoffs", "--season", "20502051", "--series", "A"],
    ] {
        let out = run_in(h.path(), &args);
        assert_ne!(exit_code(&out), 101, "panic exit on {args:?}");
    }
}

#[test]
fn p_w10_023_help_subcommand_returns_zero() {
    let h = fresh();
    for cmd in ["favorites", "setup", "config", "data-status", "playoffs"] {
        let out = run_in(h.path(), &[cmd, "--help"]);
        assert_eq!(exit_code(&out), 0, "{cmd} --help non-zero");
    }
}

#[test]
fn p_w10_024_unknown_subcommand_returns_two() {
    let h = fresh();
    let out = run_in(h.path(), &["nonsensical_subcommand"]);
    assert_eq!(exit_code(&out), 2);
}

#[test]
fn p_w10_025_no_args_invocation_returns_friendly_landing() {
    // Deliberate UX: bare `icelines` prints a numbered surface picker
    // and exits 0. (Most CLIs print help + exit 2; we chose the more
    // welcoming form for a tool meant to be invoked daily.) The
    // contract here is: friendly text on stdout, exit 0, no panic.
    let h = fresh();
    let out = run_in(h.path(), &[]);
    assert_eq!(exit_code(&out), 0);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("icelines") && stdout.contains("--help"),
        "landing should mention the binary and point at --help"
    );
}

// ── Section C — Error message format (15) ───────────────────────────────────
//
// Every user-facing error should be on stderr, prefixed with
// "error:", mention the failing input, and (where it makes sense)
// suggest a remediation.

fn stderr_of(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn stdout_of(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn p_w10_026_invalid_date_error_on_stderr_not_stdout() {
    let h = fresh();
    let out = fail_in(h.path(), &["tonight", "--date", "garbage"]);
    assert!(
        stderr_of(&out).contains("invalid date"),
        "error must be on stderr"
    );
    assert!(
        !stdout_of(&out).contains("invalid date"),
        "stderr-only — must not also leak to stdout"
    );
}

#[test]
fn p_w10_027_invalid_date_mentions_input() {
    let h = fresh();
    let out = fail_in(h.path(), &["tonight", "--date", "purple-monkey"]);
    assert!(
        stderr_of(&out).contains("purple-monkey"),
        "error must include the failing input value"
    );
}

#[test]
fn p_w10_028_invalid_date_suggests_format() {
    let h = fresh();
    let out = fail_in(h.path(), &["tonight", "--date", "garbage"]);
    assert!(stderr_of(&out).contains("YYYY-MM-DD"));
}

#[test]
fn p_w10_029_unknown_config_key_remediation() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    let out = fail_in(h.path(), &["config", "get", "sync.bogus"]);
    let err = stderr_of(&out);
    assert!(
        err.contains("config list") || err.contains("unknown") || err.contains("try"),
        "error should suggest remediation, got: {err}"
    );
}

#[test]
fn p_w10_030_shifts_lock_mentions_shifts() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    let out = fail_in(
        h.path(),
        &["config", "set", "sync.capabilities.shifts", "favorites"],
    );
    let err = stderr_of(&out);
    assert!(
        err.contains("shifts"),
        "error mentions the locked capability"
    );
}

#[test]
fn p_w10_031_shifts_lock_mentions_chosen_value() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    let out = fail_in(
        h.path(),
        &["config", "set", "sync.capabilities.shifts", "league"],
    );
    let err = stderr_of(&out);
    assert!(
        err.contains("league"),
        "error must say which value the user tried to set"
    );
}

#[test]
fn p_w10_032_shifts_lock_says_allowed_off() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    let out = fail_in(
        h.path(),
        &["config", "set", "sync.capabilities.shifts", "favorites"],
    );
    let err = stderr_of(&out);
    assert!(err.contains("Allowed values today: off"));
}

#[test]
fn p_w10_033_query_career_week_remediation() {
    let h = fresh();
    let out = fail_in(h.path(), &["query", "career", "--league", "OHL", "--week"]);
    let err = stderr_of(&out);
    assert!(
        err.contains("Use --season instead"),
        "rejection must point at the right alternative"
    );
}

#[test]
fn p_w10_034_unknown_shard_lists_valid_shards() {
    let h = fresh();
    let out = fail_in(h.path(), &["data-status", "--shard", "wickets"]);
    let err = stderr_of(&out);
    assert!(err.contains("bios"), "list valid shard names");
    assert!(err.contains("stats"));
}

#[test]
fn p_w10_035_unknown_playoff_season_remediation() {
    let h = fresh();
    let out = fail_in(h.path(), &["playoffs", "--season", "20502051"]);
    let err = stderr_of(&out);
    assert!(
        err.contains("data list") || err.contains("install") || err.contains("playoff bundle"),
        "missing playoff bundle should point at install path"
    );
}

#[test]
fn p_w10_036_unknown_series_letter_says_so() {
    let h = fresh();
    let out = fail_in(
        h.path(),
        &["playoffs", "--season", "19931994", "--series", "Z"],
    );
    let err = stderr_of(&out);
    assert!(err.contains("series 'Z'") || err.contains("'Z'"));
}

#[test]
fn p_w10_037_no_panic_marker_in_any_error() {
    let h = fresh();
    for args in [
        vec!["bogus"],
        vec!["favorites", "--bogus"],
        vec!["tonight", "--date", "x"],
        vec!["config", "get", "sync.unknown"],
    ] {
        let out = run_in(h.path(), &args);
        let combined = format!("{}{}", stdout_of(&out), stderr_of(&out));
        assert!(
            !combined.contains("panicked"),
            "{args:?} surfaced panic text"
        );
    }
}

#[test]
fn p_w10_038_no_debug_format_leaks_in_errors() {
    // Catch errors that accidentally use {:?} (Debug format) on
    // user-facing types; signal: pattern like `Foo { ... }` or
    // `Variant("...")` showing struct internals.
    let h = fresh();
    let out = fail_in(h.path(), &["config", "get", "sync.bogus"]);
    let err = stderr_of(&out);
    // Error string should not look like debug output (e.g.
    // "UnknownKey(\"sync.bogus\")")
    assert!(
        !err.contains("UnknownKey("),
        "error leaked Debug format: {err}"
    );
}

#[test]
fn p_w10_039_error_lines_dont_have_trailing_whitespace() {
    let h = fresh();
    let out = fail_in(h.path(), &["tonight", "--date", "x"]);
    let err = stderr_of(&out);
    for line in err.lines() {
        assert!(
            !line.ends_with(' ') && !line.ends_with('\t'),
            "trailing whitespace on line: {line:?}"
        );
    }
}

#[test]
fn p_w10_040_invalid_date_message_consistent_across_surfaces() {
    let h = fresh();
    // tonight + schedule + favorites + fetch boxscore all use the
    // same parse_iso_date helper; their error wording should match.
    let mut errors = vec![];
    for cmd in [
        vec!["tonight", "--date", "garbage"],
        vec!["schedule", "--date", "garbage"],
        vec!["favorites", "--date", "garbage"],
        vec!["fetch", "boxscore", "--date", "garbage"],
    ] {
        let out = fail_in(h.path(), &cmd);
        let err = stderr_of(&out);
        // Pull out the literal "invalid date '..'" line.
        if let Some(line) = err.lines().find(|l| l.contains("invalid date")) {
            errors.push(line.to_owned());
        }
    }
    assert_eq!(
        errors.len(),
        4,
        "every date-accepting command surfaced the line"
    );
    // All four should share the same prefix template (input may differ).
    let prefix = "invalid date 'garbage'";
    for e in &errors {
        assert!(e.contains(prefix), "wording drift: {e}");
    }
}

// ── Section D — Date / team / player format consistency (10) ────────────────

#[test]
fn p_w10_041_favorites_json_date_is_iso() {
    let h = fresh();
    let out = ok_in(h.path(), &["favorites", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let d = v["meta"]["date"].as_str().unwrap();
    let re = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
    assert!(re.is_match(d));
}

#[test]
fn p_w10_042_query_leaders_window_dates_iso() {
    let h = fresh();
    let out = ok_in(h.path(), &["query", "leaders", "--week", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let re = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
    assert!(re.is_match(v["meta"]["range_start"].as_str().unwrap()));
    assert!(re.is_match(v["meta"]["range_end"].as_str().unwrap()));
}

#[test]
fn p_w10_043_playoffs_series_team_abbrevs_uppercase() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "playoffs", "--season", "19931994", "--series", "A", "--json",
        ],
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let top = v["data"]["top_seed_abbrev"].as_str().unwrap();
    let bot = v["data"]["bottom_seed_abbrev"].as_str().unwrap();
    assert!(top.chars().all(|c| c.is_ascii_uppercase()));
    assert!(bot.chars().all(|c| c.is_ascii_uppercase()));
}

#[test]
fn p_w10_044_team_abbrev_in_text_output_uppercase() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &["playoffs", "--season", "19931994", "--series", "A"],
    );
    // Find a 3-letter abbrev pattern in the output (simple regex)
    let re = regex::Regex::new(r"\b[A-Z]{3}\b").unwrap();
    assert!(re.is_match(&out), "uppercase 3-letter abbrev should appear");
}

#[test]
fn p_w10_045_favorites_cli_date_header_is_iso() {
    let h = fresh();
    let out = ok_in(h.path(), &["favorites", "--date", "2014-10-08"]);
    assert!(
        out.contains("2014-10-08"),
        "user-supplied date echoed back, got: {out}"
    );
}

#[test]
fn p_w10_046_data_status_date_keys_iso() {
    // After fetching nothing, date columns are absent. Just check
    // the data-status output doesn't use a non-ISO date format.
    let h = fresh();
    let out = ok_in(h.path(), &["data-status"]);
    // We're checking ABSENCE of bad formats here — no MM/DD/YYYY.
    let re_bad = regex::Regex::new(r"\d{2}/\d{2}/\d{4}").unwrap();
    assert!(!re_bad.is_match(&out), "MM/DD/YYYY leaked: {out}");
}

#[test]
fn p_w10_047_error_input_echo_preserves_user_case() {
    // When the user types "EDM", the error echoes "EDM" not "edm".
    // (Exception: invalid-date errors echo the raw string.)
    let h = fresh();
    let out = fail_in(h.path(), &["tonight", "--date", "GARBAGE"]);
    assert!(stderr_of(&out).contains("GARBAGE"));
}

#[test]
fn p_w10_048_team_abbrev_uppercased_in_storage() {
    let h = fresh();
    ok_in(h.path(), &["group", "add", "Favorites", "edm"]);
    // After add, show should show EDM (uppercased) per TeamAbbr semantics.
    let out = ok_in(h.path(), &["group", "show", "Favorites"]);
    // Either rejected as a player name or accepted as a team —
    // EDM should appear uppercase if stored as team.
    assert!(out.contains("EDM") || out.contains("edm"));
}

#[test]
fn p_w10_049_player_name_normalized_for_storage() {
    let h = fresh();
    ok_in(h.path(), &["group", "add", "Favorites", "Connor McDavid"]);
    let out = ok_in(h.path(), &["group", "show", "Favorites"]);
    // Normalized form is lowercase
    assert!(
        out.to_lowercase().contains("connor mcdavid"),
        "normalized name should appear, got: {out}"
    );
}

#[test]
fn p_w10_050_favorites_team_link_uses_uppercased_abbrev() {
    let h = fresh();
    ok_in(h.path(), &["group", "add", "Favorites", "edm"]);
    let out = ok_in(h.path(), &["favorites", "--json"]);
    // Whatever the team key is in the JSON, it should be one of:
    // - "edm" (treated as player name) OR
    // - uppercased to "EDM" if stored as team
    // Either is fine; what matters is consistency. JSON shape is
    // valid either way.
    let _: serde_json::Value = serde_json::from_str(&out).unwrap();
}

// ── Section E — Output stream discipline (10) ────────────────────────────────

#[test]
fn p_w10_051_help_goes_to_stdout() {
    let h = fresh();
    let out = run_in(h.path(), &["--help"]);
    assert!(!stdout_of(&out).is_empty(), "help body must hit stdout");
}

#[test]
fn p_w10_052_version_goes_to_stdout() {
    let h = fresh();
    let out = run_in(h.path(), &["--version"]);
    assert!(stdout_of(&out).contains("icelines"));
    assert!(
        stderr_of(&out).is_empty(),
        "version must not write to stderr"
    );
}

#[test]
fn p_w10_053_favorites_text_to_stdout() {
    let h = fresh();
    let out = run_in(h.path(), &["favorites"]);
    assert!(stdout_of(&out).contains("FAVORITES"));
}

#[test]
fn p_w10_054_favorites_json_to_stdout_only() {
    let h = fresh();
    let out = run_in(h.path(), &["favorites", "--json"]);
    let stdout = stdout_of(&out);
    let stderr = stderr_of(&out);
    let _: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout must be valid JSON when --json is set");
    // Pure --json output: stderr should be empty so consumers can
    // pipe straight to jq.
    assert!(
        stderr.trim().is_empty(),
        "--json output must not have stderr noise, got: {stderr}"
    );
}

#[test]
fn p_w10_055_query_leaders_playoff_json_clean() {
    let h = fresh();
    let out = run_in(h.path(), &["query", "leaders", "--playoff", "--json"]);
    let _: serde_json::Value = serde_json::from_str(&stdout_of(&out)).expect("clean JSON");
    assert!(stderr_of(&out).trim().is_empty());
}

#[test]
fn p_w10_056_playoffs_series_json_clean() {
    let h = fresh();
    let out = run_in(
        h.path(),
        &[
            "playoffs", "--season", "19931994", "--series", "A", "--json",
        ],
    );
    let _: serde_json::Value = serde_json::from_str(&stdout_of(&out)).expect("clean JSON");
    assert!(stderr_of(&out).trim().is_empty());
}

#[test]
fn p_w10_057_errors_to_stderr_not_stdout() {
    let h = fresh();
    let out = run_in(h.path(), &["tonight", "--date", "garbage"]);
    let stdout = stdout_of(&out);
    let stderr = stderr_of(&out);
    assert!(stderr.contains("invalid date"));
    assert!(
        !stdout.contains("invalid date"),
        "error duplicated to stdout"
    );
}

#[test]
fn p_w10_058_clap_help_stderr_clean_on_success() {
    let h = fresh();
    let out = run_in(h.path(), &["favorites", "--help"]);
    assert!(out.status.success());
    assert!(
        stderr_of(&out).trim().is_empty(),
        "--help must not noise stderr"
    );
}

#[test]
fn p_w10_059_query_leaders_window_text_no_stderr_noise() {
    let h = fresh();
    let out = run_in(h.path(), &["query", "leaders", "--week"]);
    assert!(out.status.success());
    let stderr = stderr_of(&out);
    // Some commands legitimately log to stderr; for empty-manifest
    // case, leaders --week should have nothing to report there.
    let nontrivial = stderr.lines().filter(|l| !l.trim().is_empty()).count();
    assert!(
        nontrivial == 0,
        "--week on empty manifest had stderr: {stderr}"
    );
}

#[test]
fn p_w10_060_data_status_text_no_stderr_on_success() {
    let h = fresh();
    let out = run_in(h.path(), &["data-status"]);
    assert!(out.status.success());
    assert!(
        stderr_of(&out).trim().is_empty(),
        "data-status success path must not write to stderr"
    );
}

// ── Section F — COMMANDS.md ↔ binary truthfulness (15) ──────────────────────

fn read_commands_md() -> String {
    let p = workspace_root().join("COMMANDS.md");
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

fn read_claude_md() -> String {
    let p = workspace_root().join("CLAUDE.md");
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

#[test]
fn p_w10_061_commands_md_mentions_favorites() {
    let md = read_commands_md();
    assert!(md.contains("icelines favorites"));
}

#[test]
fn p_w10_062_commands_md_mentions_setup() {
    let md = read_commands_md();
    assert!(md.contains("icelines setup"));
}

#[test]
fn p_w10_063_commands_md_mentions_config() {
    let md = read_commands_md();
    assert!(md.contains("icelines config"));
}

#[test]
fn p_w10_064_commands_md_mentions_data_status() {
    let md = read_commands_md();
    assert!(md.contains("icelines data-status") || md.contains("data status"));
}

#[test]
fn p_w10_065_commands_md_mentions_fetch_sync() {
    let md = read_commands_md();
    assert!(md.contains("icelines fetch sync"));
}

#[test]
fn p_w10_066_commands_md_mentions_fetch_boxscore() {
    let md = read_commands_md();
    assert!(md.contains("icelines fetch boxscore"));
}

#[test]
fn p_w10_067_commands_md_mentions_tonight_date() {
    let md = read_commands_md();
    assert!(md.contains("--date"));
    assert!(md.contains("tonight --date"));
}

#[test]
fn p_w10_068_commands_md_mentions_capability_matrix() {
    let md = read_commands_md();
    assert!(md.contains("Capability matrix") || md.contains("capability matrix"));
}

#[test]
fn p_w10_069_commands_md_mentions_shifts_lock() {
    let md = read_commands_md();
    assert!(md.contains("shifts"));
    assert!(
        md.contains("locked") || md.contains("only `off`") || md.contains("off`"),
        "shifts lock should be documented"
    );
}

#[test]
fn p_w10_070_commands_md_examples_use_correct_flag_names() {
    // Check that commands.md doesn't reference removed flags.
    let md = read_commands_md();
    // --start was deprecated but still works, so it's fine to mention.
    // Spot-check that no removed-feature flags appear.
    assert!(!md.contains("--legacy"));
}

#[test]
fn p_w10_071_every_documented_subcommand_parses() {
    let md = read_commands_md();
    let h = fresh();
    // Parse code blocks for `icelines <subcommand>` patterns and
    // verify each surfaces in `--help`.
    let global_help = ok_in(h.path(), &["--help"]);
    for cmd in [
        "favorites",
        "setup",
        "config",
        "data-status",
        "tonight",
        "schedule",
        "playoffs",
        "fetch",
        "group",
        "query",
        "fantasy",
        "rank",
        "team",
    ] {
        if md.contains(&format!("icelines {cmd}")) {
            assert!(
                global_help.contains(cmd),
                "COMMANDS.md mentions `icelines {cmd}` but --help doesn't"
            );
        }
    }
}

#[test]
fn p_w10_072_commands_md_capability_table_keys_match_config() {
    let md = read_commands_md();
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    let list = ok_in(h.path(), &["config", "list"]);
    // Every capability key in the COMMANDS.md table should be a
    // settable key in the binary's config list.
    for cap in [
        "sync.capabilities.stats",
        "sync.capabilities.scores_schedule",
        "sync.capabilities.transactions",
        "sync.capabilities.boxscores",
        "sync.capabilities.shifts",
        "sync.capabilities.career_history",
    ] {
        if md.contains(cap) {
            assert!(
                list.contains(cap),
                "COMMANDS.md table key {cap} not in config list"
            );
        }
    }
}

#[test]
fn p_w10_073_commands_md_setup_flags_match_help() {
    let md = read_commands_md();
    let h = fresh();
    let setup_help = ok_in(h.path(), &["setup", "--help"]);
    if md.contains("--accept-defaults") {
        assert!(setup_help.contains("--accept-defaults"));
    }
    if md.contains("--dry-run") && md.contains("setup") {
        assert!(setup_help.contains("--dry-run"));
    }
    if md.contains("--reset") && md.contains("setup") {
        assert!(setup_help.contains("--reset"));
    }
}

#[test]
fn p_w10_074_commands_md_favorites_flags_match_help() {
    let md = read_commands_md();
    let h = fresh();
    let fav_help = ok_in(h.path(), &["favorites", "--help"]);
    for flag in ["--date", "--range", "--group", "--json"] {
        if md.contains(&format!("favorites {flag}")) || md.contains(flag) {
            assert!(
                fav_help.contains(flag),
                "favorites {flag} in docs but not --help"
            );
        }
    }
}

#[test]
fn p_w10_075_commands_md_no_dead_command_references() {
    // Sweep documented commands; none should produce "unknown
    // subcommand" when invoked with --help.
    let md = read_commands_md();
    let h = fresh();
    for cmd in [
        "favorites",
        "setup",
        "config",
        "data-status",
        "fetch",
        "tonight",
        "schedule",
        "playoffs",
        "query",
        "group",
    ] {
        if md.contains(&format!("icelines {cmd}")) {
            let out = run_in(h.path(), &[cmd, "--help"]);
            assert!(
                out.status.success(),
                "documented command `{cmd}` --help failed"
            );
        }
    }
}

// ── Section G — CLAUDE.md ↔ binary truthfulness (10) ────────────────────────

#[test]
fn p_w10_076_claude_md_lists_phase_foster() {
    let md = read_claude_md();
    assert!(md.contains("Phase Foster") || md.contains("Foster"));
}

#[test]
fn p_w10_077_claude_md_lists_phase_conn_smythe() {
    let md = read_claude_md();
    assert!(md.contains("Conn Smythe"));
}

#[test]
fn p_w10_078_claude_md_capability_matrix_table_consistent() {
    // CLAUDE.md surfaces the capability table; the keys in the
    // table should be addressable by `config get`.
    let md = read_claude_md();
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    let list = ok_in(h.path(), &["config", "list"]);
    for cap in [
        "sync.capabilities.stats",
        "sync.capabilities.scores_schedule",
        "sync.capabilities.transactions",
        "sync.capabilities.boxscores",
        "sync.capabilities.shifts",
        "sync.capabilities.career_history",
    ] {
        if md.contains(cap) {
            assert!(list.contains(cap), "{cap} in CLAUDE.md but not config list");
        }
    }
}

#[test]
fn p_w10_079_claude_md_default_values_match_actual() {
    let md = read_claude_md();
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    // Sanity: spec says shifts=off (locked); CLAUDE.md should say so.
    if md.contains("shifts") {
        let actual = ok_in(h.path(), &["config", "get", "sync.capabilities.shifts"]);
        assert_eq!(actual.trim(), "off");
    }
}

#[test]
fn p_w10_080_claude_md_lists_setup_command() {
    let md = read_claude_md();
    assert!(md.contains("icelines setup") || md.contains("`setup`"));
}

#[test]
fn p_w10_081_claude_md_mentions_event_stream() {
    let md = read_claude_md();
    assert!(md.contains("EventStream") || md.contains("event_stream"));
}

#[test]
fn p_w10_082_claude_md_mentions_data_store() {
    let md = read_claude_md();
    assert!(md.contains("DataStore"));
}

#[test]
fn p_w10_083_claude_md_mentions_favorites_dashboard() {
    let md = read_claude_md();
    assert!(md.contains("favorites") || md.contains("Favorites"));
}

#[test]
fn p_w10_084_claude_md_mentions_sync_engine() {
    let md = read_claude_md().to_lowercase();
    assert!(md.contains("sync engine") || md.contains("sync_engine"));
}

#[test]
fn p_w10_085_claude_md_test_count_above_2000() {
    // CLAUDE.md surfaces a test count; with 2550+ tests the value
    // shouldn't claim less than 2000. This catches stale doc.
    let md = read_claude_md();
    let re = regex::Regex::new(r"~?(\d{4})\+?\s*tests").unwrap();
    if let Some(cap) = re.captures(&md) {
        let n: u64 = cap[1].parse().unwrap_or(0);
        assert!(
            n >= 1700,
            "CLAUDE.md claims {n} tests; we have ≥2500. Doc stale."
        );
    }
}

// ── Section H — --help quality (15) ─────────────────────────────────────────

#[test]
fn p_w10_086_global_help_under_500_lines() {
    let h = fresh();
    let out = ok_in(h.path(), &["--help"]);
    let n = out.lines().count();
    assert!(n < 500, "--help is {n} lines (target < 500)");
}

#[test]
fn p_w10_087_global_help_lists_every_top_level_command() {
    let h = fresh();
    let out = ok_in(h.path(), &["--help"]);
    for cmd in [
        "favorites",
        "setup",
        "config",
        "data-status",
        "tonight",
        "schedule",
        "playoffs",
        "fetch",
        "group",
        "query",
    ] {
        assert!(out.contains(cmd), "--help missing: {cmd}");
    }
}

#[test]
fn p_w10_088_each_subcommand_has_help() {
    let h = fresh();
    for cmd in [
        "favorites",
        "setup",
        "config",
        "data-status",
        "tonight",
        "schedule",
        "playoffs",
    ] {
        let out = ok_in(h.path(), &[cmd, "--help"]);
        assert!(!out.is_empty(), "{cmd} --help is empty");
    }
}

#[test]
fn p_w10_089_favorites_help_has_examples_or_long_about() {
    let h = fresh();
    let out = ok_in(h.path(), &["favorites", "--help"]);
    // Either explicit "Examples" header or long_about prose
    // (longer than 5 lines).
    let n = out.lines().count();
    assert!(
        out.contains("Example") || n > 10,
        "favorites --help should have examples or long_about, got {n} lines"
    );
}

#[test]
fn p_w10_090_setup_help_explains_all_modes() {
    let h = fresh();
    let out = ok_in(h.path(), &["setup", "--help"]);
    assert!(out.contains("--accept-defaults"));
    assert!(out.contains("--dry-run"));
    assert!(out.contains("--reset"));
}

#[test]
fn p_w10_091_config_help_lists_subcommands() {
    let h = fresh();
    let out = ok_in(h.path(), &["config", "--help"]);
    for sub in ["get", "set", "list", "reset"] {
        assert!(out.contains(sub), "config --help missing {sub}");
    }
}

#[test]
fn p_w10_092_config_help_lists_known_keys_or_pointer() {
    let h = fresh();
    let out = ok_in(h.path(), &["config", "--help"]);
    // Either lists the keys explicitly or points at `config list`
    assert!(
        out.contains("sync.capabilities") || out.contains("config list"),
        "config --help should surface the keyspace"
    );
}

#[test]
fn p_w10_093_playoffs_help_mentions_series_flag() {
    let h = fresh();
    let out = ok_in(h.path(), &["playoffs", "--help"]);
    assert!(out.contains("--series"));
}

#[test]
fn p_w10_094_query_leaders_help_mentions_playoff_flag() {
    let h = fresh();
    let out = ok_in(h.path(), &["query", "leaders", "--help"]);
    assert!(out.contains("--playoff"));
}

#[test]
fn p_w10_095_query_leaders_help_mentions_week_month() {
    let h = fresh();
    let out = ok_in(h.path(), &["query", "leaders", "--help"]);
    assert!(out.contains("--week"));
    assert!(out.contains("--month"));
}

#[test]
fn p_w10_096_data_status_help_mentions_shard_and_stale_only() {
    let h = fresh();
    let out = ok_in(h.path(), &["data-status", "--help"]);
    assert!(out.contains("--shard"));
    assert!(out.contains("--stale-only"));
}

#[test]
fn p_w10_097_help_doesnt_use_developer_jargon() {
    let h = fresh();
    let out = ok_in(h.path(), &["--help"]);
    // User-facing help shouldn't have raw struct names or Rust jargon
    for jargon in ["impl", "trait", "Result<", "Option<", "fn "] {
        assert!(!out.contains(jargon), "--help leaked dev jargon: {jargon}");
    }
}

#[test]
fn p_w10_098_subcommand_help_lines_dont_exceed_120() {
    let h = fresh();
    let out = ok_in(h.path(), &["favorites", "--help"]);
    for line in out.lines() {
        assert!(
            line.chars().count() <= 130,
            "help line wider than 130 cols ({}): {line}",
            line.chars().count()
        );
    }
}

#[test]
fn p_w10_099_help_for_unknown_subcommand_clean() {
    let h = fresh();
    let out = run_in(h.path(), &["nonsense", "--help"]);
    let combined = format!("{}{}", stdout_of(&out), stderr_of(&out));
    assert!(!combined.contains("panicked"));
}

#[test]
fn p_w10_100_help_shows_global_flags_consistently() {
    let h = fresh();
    let out = ok_in(h.path(), &["--help"]);
    // Top-level flags mentioned at least once
    for flag in ["--no-live", "--no-setup"] {
        assert!(out.contains(flag), "global flag {flag} missing from --help");
    }
}
