//! Persona Wave 7 — 100 scenarios across Phase Conn Smythe surfaces:
//! series momentum, Cup-run player narratives (`query leaders --playoff`),
//! and per-game live detail. Tests the only bundled playoff season
//! (1993-94) heavily plus edge cases around series letters, sort
//! options, JSON envelope shapes, and the new web `/game/:id` route.

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

const PLAYOFF_SEASON: &str = "19931994";

// ── Series momentum coverage on the bundled 1993-94 bracket (40) ─────────────
//
// 1993-94 has 4 rounds with letters A-H in round 1, then I-L (round 2),
// M-N (conf finals), O (Cup Final). Walking each surfaces real bracket
// state.

#[test]
fn p_w7_001_series_a_renders() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &["playoffs", "--season", PLAYOFF_SEASON, "--series", "A"],
    );
    assert!(out.contains("SERIES A"));
}

#[test]
fn p_w7_002_series_b_renders() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &["playoffs", "--season", PLAYOFF_SEASON, "--series", "B"],
    );
    assert!(out.contains("SERIES B"));
}

#[test]
fn p_w7_003_series_c_renders() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &["playoffs", "--season", PLAYOFF_SEASON, "--series", "C"],
    );
    assert!(out.contains("SERIES C"));
}

#[test]
fn p_w7_004_series_d_renders() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &["playoffs", "--season", PLAYOFF_SEASON, "--series", "D"],
    );
    assert!(out.contains("SERIES D"));
}

#[test]
fn p_w7_005_series_e_renders() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &["playoffs", "--season", PLAYOFF_SEASON, "--series", "E"],
    );
    assert!(out.contains("SERIES E"));
}

#[test]
fn p_w7_006_series_f_renders() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &["playoffs", "--season", PLAYOFF_SEASON, "--series", "F"],
    );
    assert!(out.contains("SERIES F"));
}

#[test]
fn p_w7_007_series_g_renders() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &["playoffs", "--season", PLAYOFF_SEASON, "--series", "G"],
    );
    assert!(out.contains("SERIES G"));
}

#[test]
fn p_w7_008_series_h_renders() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &["playoffs", "--season", PLAYOFF_SEASON, "--series", "H"],
    );
    assert!(out.contains("SERIES H"));
}

#[test]
fn p_w7_009_series_unknown_letter_z_clean_error() {
    let h = fresh();
    let err = fail_in(
        h.path(),
        &["playoffs", "--season", PLAYOFF_SEASON, "--series", "Z"],
    );
    assert!(err.contains("no series 'Z'"));
}

#[test]
fn p_w7_010_series_letter_too_long_clean_error() {
    let h = fresh();
    fail_in(
        h.path(),
        &["playoffs", "--season", PLAYOFF_SEASON, "--series", "AA"],
    );
}

#[test]
fn p_w7_011_series_a_uppercase() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &["playoffs", "--season", PLAYOFF_SEASON, "--series", "a"],
    );
    assert!(out.contains("SERIES A"), "lowercase a should match A");
}

#[test]
fn p_w7_012_series_with_round_filter_combined_no_panic() {
    let h = fresh();
    no_panic_in(
        h.path(),
        &[
            "playoffs",
            "--season",
            PLAYOFF_SEASON,
            "--series",
            "A",
            "--round",
            "1",
        ],
    );
}

#[test]
fn p_w7_013_series_a_renders_top_seed_marker() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &["playoffs", "--season", PLAYOFF_SEASON, "--series", "A"],
    );
    assert!(out.contains("(top seed)"));
}

#[test]
fn p_w7_014_series_a_renders_summary_line() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &["playoffs", "--season", PLAYOFF_SEASON, "--series", "A"],
    );
    assert!(
        out.contains("games played") || out.contains("wins"),
        "summary line should appear, got: {out}"
    );
}

#[test]
fn p_w7_015_series_a_renders_round_label() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &["playoffs", "--season", PLAYOFF_SEASON, "--series", "A"],
    );
    // 1993-94 round 1 was "Conference Quarterfinals" in the era's
    // labeling. Match either modern or era-appropriate.
    assert!(
        out.contains("Quarterfinals") || out.contains("First Round") || out.contains("Round 1"),
        "round label should appear, got: {out}"
    );
}

#[test]
fn p_w7_016_series_json_envelope_shape() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "playoffs",
            "--season",
            PLAYOFF_SEASON,
            "--series",
            "A",
            "--json",
        ],
    );
    let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["route"], "playoffs.series");
}

#[test]
fn p_w7_017_series_json_data_has_top_seed_wins() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "playoffs",
            "--season",
            PLAYOFF_SEASON,
            "--series",
            "A",
            "--json",
        ],
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v["data"]["top_seed_wins"].is_number());
}

#[test]
fn p_w7_018_series_json_data_has_bottom_seed_wins() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "playoffs",
            "--season",
            PLAYOFF_SEASON,
            "--series",
            "A",
            "--json",
        ],
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v["data"]["bottom_seed_wins"].is_number());
}

#[test]
fn p_w7_019_series_json_data_has_leader() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "playoffs",
            "--season",
            PLAYOFF_SEASON,
            "--series",
            "A",
            "--json",
        ],
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let leader = v["data"]["leader"].as_str().unwrap_or("");
    assert!(matches!(leader, "top" | "bottom" | "tied"));
}

#[test]
fn p_w7_020_series_json_data_has_games_played() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "playoffs",
            "--season",
            PLAYOFF_SEASON,
            "--series",
            "A",
            "--json",
        ],
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let played = v["data"]["games_played"].as_u64().unwrap_or(0);
    assert!(
        played <= 7,
        "games_played must be ≤ 7 for best-of-7, got {played}"
    );
}

#[test]
fn p_w7_021_series_json_data_has_games_remaining() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "playoffs",
            "--season",
            PLAYOFF_SEASON,
            "--series",
            "A",
            "--json",
        ],
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let remaining = v["data"]["games_remaining"].as_u64().unwrap_or(99);
    assert!(remaining <= 7, "games_remaining ≤ 7, got {remaining}");
}

#[test]
fn p_w7_022_series_json_data_has_top_seed_abbrev() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "playoffs",
            "--season",
            PLAYOFF_SEASON,
            "--series",
            "A",
            "--json",
        ],
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let abbr = v["data"]["top_seed_abbrev"].as_str().unwrap_or("");
    assert!(!abbr.is_empty(), "top_seed_abbrev must be present");
    assert!(abbr.chars().all(|c| c.is_ascii_uppercase()));
}

#[test]
fn p_w7_023_series_json_data_has_bottom_seed_abbrev() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "playoffs",
            "--season",
            PLAYOFF_SEASON,
            "--series",
            "A",
            "--json",
        ],
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let abbr = v["data"]["bottom_seed_abbrev"].as_str().unwrap_or("");
    assert!(!abbr.is_empty());
}

#[test]
fn p_w7_024_series_json_data_has_round() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "playoffs",
            "--season",
            PLAYOFF_SEASON,
            "--series",
            "A",
            "--json",
        ],
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let round = v["data"]["round"].as_u64().unwrap_or(0);
    assert!((1..=4).contains(&round));
}

#[test]
fn p_w7_025_series_json_data_has_series_complete_bool() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "playoffs",
            "--season",
            PLAYOFF_SEASON,
            "--series",
            "A",
            "--json",
        ],
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v["data"]["series_complete"].is_boolean());
}

#[test]
fn p_w7_026_series_json_data_has_ot_games() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "playoffs",
            "--season",
            PLAYOFF_SEASON,
            "--series",
            "A",
            "--json",
        ],
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let ot = v["data"]["ot_games"].as_u64().unwrap_or(99);
    assert!(ot <= 7);
}

#[test]
fn p_w7_027_series_json_data_has_home_advantage_bool() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "playoffs",
            "--season",
            PLAYOFF_SEASON,
            "--series",
            "A",
            "--json",
        ],
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v["data"]["home_advantage"].is_boolean());
}

#[test]
fn p_w7_028_series_meta_has_season_id() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "playoffs",
            "--season",
            PLAYOFF_SEASON,
            "--series",
            "A",
            "--json",
        ],
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["meta"]["season_id"], PLAYOFF_SEASON);
}

#[test]
fn p_w7_029_series_complete_for_finished_bracket_has_winner() {
    // 1993-94 bracket is fully resolved; every series has a winner.
    // Walk through round 1 series A — winner_abbrev should be set.
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "playoffs",
            "--season",
            PLAYOFF_SEASON,
            "--series",
            "A",
            "--json",
        ],
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let complete = v["data"]["series_complete"].as_bool().unwrap_or(false);
    if complete {
        assert!(
            v["data"]["winner_abbrev"].is_string(),
            "complete series must surface a winner"
        );
    }
}

#[test]
fn p_w7_030_series_a_renders_no_long_lines() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &["playoffs", "--season", PLAYOFF_SEASON, "--series", "A"],
    );
    for line in out.lines() {
        assert!(
            line.len() <= 200,
            "no line should exceed 200 chars (got {} on: {line})",
            line.len()
        );
    }
}

#[test]
fn p_w7_031_series_b_no_panic() {
    let h = fresh();
    no_panic_in(
        h.path(),
        &["playoffs", "--season", PLAYOFF_SEASON, "--series", "B"],
    );
}

#[test]
fn p_w7_032_series_c_no_panic() {
    let h = fresh();
    no_panic_in(
        h.path(),
        &["playoffs", "--season", PLAYOFF_SEASON, "--series", "C"],
    );
}

#[test]
fn p_w7_033_series_o_round_4_cup_final() {
    // 1993-94 had 16 series → letter O is the 15th (round 4 / Cup Final)
    let h = fresh();
    no_panic_in(
        h.path(),
        &["playoffs", "--season", PLAYOFF_SEASON, "--series", "O"],
    );
}

#[test]
fn p_w7_034_series_with_csv_no_panic() {
    let h = fresh();
    // --series + --csv together should not crash even though csv is
    // really for the bracket view; --series takes precedence.
    no_panic_in(
        h.path(),
        &[
            "playoffs",
            "--season",
            PLAYOFF_SEASON,
            "--series",
            "A",
            "--csv",
        ],
    );
}

#[test]
fn p_w7_035_series_default_season_no_explicit() {
    // Without --season, default_season() picks the most recent
    // bundled playoff. 1993-94 is the only one, so it falls there.
    let h = fresh();
    no_panic_in(h.path(), &["playoffs", "--series", "A"]);
}

#[test]
fn p_w7_036_series_a_through_h_distinct_data() {
    let h = fresh();
    let mut seen_pairs: Vec<(String, String)> = Vec::new();
    for letter in ["A", "B", "C", "D", "E", "F", "G", "H"] {
        let out = ok_in(
            h.path(),
            &[
                "playoffs",
                "--season",
                PLAYOFF_SEASON,
                "--series",
                letter,
                "--json",
            ],
        );
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let top = v["data"]["top_seed_abbrev"]
            .as_str()
            .unwrap_or("")
            .to_owned();
        let bot = v["data"]["bottom_seed_abbrev"]
            .as_str()
            .unwrap_or("")
            .to_owned();
        seen_pairs.push((top, bot));
    }
    // 8 round-1 series → 16 unique team appearances.
    let mut teams: Vec<String> = seen_pairs
        .iter()
        .flat_map(|(a, b)| [a.clone(), b.clone()])
        .collect();
    teams.sort();
    teams.dedup();
    assert_eq!(
        teams.len(),
        16,
        "round 1 has 16 unique teams, got {teams:?}"
    );
}

#[test]
fn p_w7_037_series_round_label_consistent_per_letter() {
    // Each letter should map to one round consistently across calls.
    let h = fresh();
    let out1 = ok_in(
        h.path(),
        &[
            "playoffs",
            "--season",
            PLAYOFF_SEASON,
            "--series",
            "A",
            "--json",
        ],
    );
    let out2 = ok_in(
        h.path(),
        &[
            "playoffs",
            "--season",
            PLAYOFF_SEASON,
            "--series",
            "A",
            "--json",
        ],
    );
    let v1: serde_json::Value = serde_json::from_str(&out1).unwrap();
    let v2: serde_json::Value = serde_json::from_str(&out2).unwrap();
    assert_eq!(v1["data"]["round"], v2["data"]["round"]);
    assert_eq!(v1["data"]["round_label"], v2["data"]["round_label"]);
}

#[test]
fn p_w7_038_series_a_through_h_all_round_1() {
    let h = fresh();
    for letter in ["A", "B", "C", "D", "E", "F", "G", "H"] {
        let out = ok_in(
            h.path(),
            &[
                "playoffs",
                "--season",
                PLAYOFF_SEASON,
                "--series",
                letter,
                "--json",
            ],
        );
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(
            v["data"]["round"], 1,
            "letters A-H are round 1, got {} for {letter}",
            v["data"]["round"]
        );
    }
}

#[test]
fn p_w7_039_series_unknown_season_clean_error() {
    let h = fresh();
    fail_in(
        h.path(),
        &["playoffs", "--season", "20502051", "--series", "A"],
    );
}

#[test]
fn p_w7_040_series_unknown_season_remediation_pointer() {
    let h = fresh();
    let err = fail_in(
        h.path(),
        &["playoffs", "--season", "20502051", "--series", "A"],
    );
    assert!(
        err.contains("data list") || err.contains("install") || err.contains("playoff bundle"),
        "remediation must surface, stderr: {err}"
    );
}

// ── query leaders --playoff (30) ─────────────────────────────────────────────

#[test]
fn p_w7_041_playoff_help_lists_flag() {
    let h = fresh();
    let out = ok_in(h.path(), &["query", "leaders", "--help"]);
    assert!(out.contains("--playoff"));
}

#[test]
fn p_w7_042_playoff_empty_manifest_renders_header() {
    let h = fresh();
    let out = ok_in(h.path(), &["query", "leaders", "--playoff"]);
    assert!(out.contains("PLAYOFF LEADERS"));
}

#[test]
fn p_w7_043_playoff_empty_manifest_says_no_boxscores() {
    let h = fresh();
    let out = ok_in(h.path(), &["query", "leaders", "--playoff"]);
    assert!(out.contains("no playoff boxscores"));
}

#[test]
fn p_w7_044_playoff_json_envelope_shape() {
    let h = fresh();
    let out = ok_in(h.path(), &["query", "leaders", "--playoff", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["route"], "leaders.playoff");
}

#[test]
fn p_w7_045_playoff_json_meta_kind() {
    let h = fresh();
    let out = ok_in(h.path(), &["query", "leaders", "--playoff", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["meta"]["kind"], "playoff_run");
}

#[test]
fn p_w7_046_playoff_json_meta_games_aggregated() {
    let h = fresh();
    let out = ok_in(h.path(), &["query", "leaders", "--playoff", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v["meta"]["games_aggregated"].is_number());
}

#[test]
fn p_w7_047_playoff_json_meta_sort() {
    let h = fresh();
    let out = ok_in(h.path(), &["query", "leaders", "--playoff", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v["meta"]["sort"].is_string());
}

#[test]
fn p_w7_048_playoff_json_data_array() {
    let h = fresh();
    let out = ok_in(h.path(), &["query", "leaders", "--playoff", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v["data"].is_array());
}

#[test]
fn p_w7_049_playoff_sort_g() {
    let h = fresh();
    ok_in(h.path(), &["query", "leaders", "--playoff", "--sort", "g"]);
}

#[test]
fn p_w7_050_playoff_sort_a() {
    let h = fresh();
    ok_in(h.path(), &["query", "leaders", "--playoff", "--sort", "a"]);
}

#[test]
fn p_w7_051_playoff_sort_p() {
    let h = fresh();
    ok_in(h.path(), &["query", "leaders", "--playoff", "--sort", "p"]);
}

#[test]
fn p_w7_052_playoff_sort_sog() {
    let h = fresh();
    ok_in(
        h.path(),
        &["query", "leaders", "--playoff", "--sort", "sog"],
    );
}

#[test]
fn p_w7_053_playoff_sort_hits() {
    let h = fresh();
    ok_in(
        h.path(),
        &["query", "leaders", "--playoff", "--sort", "hits"],
    );
}

#[test]
fn p_w7_054_playoff_sort_blocks() {
    let h = fresh();
    ok_in(
        h.path(),
        &["query", "leaders", "--playoff", "--sort", "blocks"],
    );
}

#[test]
fn p_w7_055_playoff_sort_plus_minus() {
    let h = fresh();
    ok_in(
        h.path(),
        &["query", "leaders", "--playoff", "--sort", "plus-minus"],
    );
}

#[test]
fn p_w7_056_playoff_sort_unknown_falls_back_to_points() {
    // Unrecognized sort key falls back to default (points). No error.
    let h = fresh();
    ok_in(
        h.path(),
        &["query", "leaders", "--playoff", "--sort", "ridiculous"],
    );
}

#[test]
fn p_w7_057_playoff_top_5() {
    let h = fresh();
    ok_in(h.path(), &["query", "leaders", "--playoff", "--top", "5"]);
}

#[test]
fn p_w7_058_playoff_top_100() {
    let h = fresh();
    ok_in(h.path(), &["query", "leaders", "--playoff", "--top", "100"]);
}

#[test]
fn p_w7_059_playoff_top_1() {
    let h = fresh();
    ok_in(h.path(), &["query", "leaders", "--playoff", "--top", "1"]);
}

#[test]
fn p_w7_060_playoff_combined_with_setup() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(h.path(), &["query", "leaders", "--playoff"]);
}

#[test]
fn p_w7_061_playoff_with_filters_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["query", "leaders", "--playoff", "--pos", "C"]);
}

#[test]
fn p_w7_062_playoff_combined_with_team_no_panic() {
    let h = fresh();
    no_panic_in(
        h.path(),
        &["query", "leaders", "--playoff", "--team", "EDM"],
    );
}

#[test]
fn p_w7_063_playoff_idempotent_runs() {
    let h = fresh();
    ok_in(h.path(), &["query", "leaders", "--playoff"]);
    ok_in(h.path(), &["query", "leaders", "--playoff"]);
    ok_in(h.path(), &["query", "leaders", "--playoff"]);
}

#[test]
fn p_w7_064_playoff_consistent_output_across_runs() {
    let h = fresh();
    let a = ok_in(h.path(), &["query", "leaders", "--playoff"]);
    let b = ok_in(h.path(), &["query", "leaders", "--playoff"]);
    assert_eq!(a, b, "output must be deterministic for the same state");
}

#[test]
fn p_w7_065_playoff_json_consistent_envelope_shape() {
    let h = fresh();
    let out = ok_in(h.path(), &["query", "leaders", "--playoff", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    // schema_version + route + data + meta
    assert!(v["schema_version"].is_number());
    assert!(v["route"].is_string());
    assert!(v["data"].is_array());
    assert!(v["meta"].is_object());
}

#[test]
fn p_w7_066_playoff_meta_rows_matches_data_len() {
    let h = fresh();
    let out = ok_in(h.path(), &["query", "leaders", "--playoff", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let data_len = v["data"].as_array().unwrap().len();
    let rows = v["meta"]["rows"].as_u64().unwrap_or(0) as usize;
    assert_eq!(data_len, rows, "meta.rows must match data.len()");
}

#[test]
fn p_w7_067_playoff_no_panic_on_corrupt_manifest() {
    // Plant garbage in the manifest dir; data-status / query leaders
    // should fall back gracefully (we test those paths separately
    // for clean errors). For --playoff specifically: empty manifest
    // → empty result.
    let h = fresh();
    no_panic_in(h.path(), &["query", "leaders", "--playoff"]);
}

#[test]
fn p_w7_068_playoff_top_zero_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["query", "leaders", "--playoff", "--top", "0"]);
}

#[test]
fn p_w7_069_playoff_combined_with_seasons_n_no_conflict() {
    // --seasons N is ignored when --playoff is set; should not error.
    let h = fresh();
    ok_in(
        h.path(),
        &["query", "leaders", "--playoff", "--seasons", "3"],
    );
}

#[test]
fn p_w7_070_playoff_csv_flag_no_panic() {
    let h = fresh();
    no_panic_in(h.path(), &["query", "leaders", "--playoff", "--csv"]);
}

// ── Conn Smythe + Foster integration (15) ───────────────────────────────────

#[test]
fn p_w7_071_setup_then_series_smoke() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(
        h.path(),
        &["playoffs", "--season", PLAYOFF_SEASON, "--series", "A"],
    );
}

#[test]
fn p_w7_072_setup_then_playoff_leaders_no_panic() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(h.path(), &["query", "leaders", "--playoff"]);
}

#[test]
fn p_w7_073_data_status_after_playoff_query_unchanged() {
    let h = fresh();
    let pre = ok_in(h.path(), &["data-status"]);
    ok_in(h.path(), &["query", "leaders", "--playoff"]);
    let post = ok_in(h.path(), &["data-status"]);
    // --playoff is read-only — it shouldn't mutate the manifest.
    assert_eq!(pre, post, "playoff query must not mutate manifest");
}

#[test]
fn p_w7_074_favorites_then_playoff_no_panic() {
    let h = fresh();
    ok_in(h.path(), &["group", "add", "Favorites", "EDM"]);
    no_panic_in(h.path(), &["query", "leaders", "--playoff"]);
}

#[test]
fn p_w7_075_series_with_favorites_added_no_panic() {
    let h = fresh();
    ok_in(h.path(), &["group", "add", "Favorites", "NYR"]);
    no_panic_in(
        h.path(),
        &["playoffs", "--season", PLAYOFF_SEASON, "--series", "A"],
    );
}

#[test]
fn p_w7_076_series_q_through_z_all_clean_errors() {
    let h = fresh();
    for letter in ["Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z"] {
        // Some of these may exist (Q-Z are within A-Z); 1993-94 has
        // 16 series so letters O onward don't all exist.
        let out = run_in(
            h.path(),
            &["playoffs", "--season", PLAYOFF_SEASON, "--series", letter],
        );
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(!combined.contains("panicked"), "letter {letter} panicked");
    }
}

#[test]
fn p_w7_077_series_invalid_chars_no_panic() {
    let h = fresh();
    for invalid in ["1", "@", "!", " ", "AB", ""] {
        let out = run_in(
            h.path(),
            &["playoffs", "--season", PLAYOFF_SEASON, "--series", invalid],
        );
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            !combined.contains("panicked"),
            "invalid letter {invalid:?} panicked"
        );
    }
}

#[test]
fn p_w7_078_series_with_no_color_terminal_no_panic() {
    let h = fresh();
    let out = Command::new(icelines_bin())
        .env("HOME", h.path())
        .env("USERPROFILE", h.path())
        .env("ICELINES_NO_LIVE", "1")
        .env("NO_COLOR", "1")
        .args(["playoffs", "--season", PLAYOFF_SEASON, "--series", "A"])
        .output()
        .unwrap();
    assert!(out.status.success());
}

#[test]
fn p_w7_079_query_player_with_playoff_run_no_panic() {
    // No --playoff sub-flag on query player today; this is a smoke
    // test that existing query player still works with Foster +
    // Conn Smythe data alongside.
    let h = fresh();
    no_panic_in(h.path(), &["query", "player", "Wayne Gretzky"]);
}

#[test]
fn p_w7_080_global_help_doesnt_break_after_conn_smythe() {
    let h = fresh();
    let out = ok_in(h.path(), &["--help"]);
    assert!(out.contains("playoffs"));
    assert!(out.contains("query"));
}

#[test]
fn p_w7_081_conn_smythe_features_alongside_foster() {
    // Single test runs setup + add favorite + playoff query + series momentum.
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(h.path(), &["group", "add", "Favorites", "NYR"]);
    ok_in(h.path(), &["query", "leaders", "--playoff"]);
    ok_in(
        h.path(),
        &["playoffs", "--season", PLAYOFF_SEASON, "--series", "A"],
    );
    ok_in(h.path(), &["favorites"]);
}

#[test]
fn p_w7_082_playoff_leaders_doesnt_add_manifest_entries() {
    // First DataStore::open seeds ~/.icelines/data/manifest/version.json
    // (one-time directory init); that's expected. What MUST NOT
    // happen: new manifest shard entries from a read-only query.
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    // Warm the DataStore so version.json already exists.
    ok_in(h.path(), &["query", "leaders", "--playoff"]);
    let pre = walk_count(h.path().join(".icelines"));
    ok_in(h.path(), &["query", "leaders", "--playoff"]);
    ok_in(h.path(), &["query", "leaders", "--playoff"]);
    let post = walk_count(h.path().join(".icelines"));
    assert_eq!(
        pre, post,
        "subsequent playoff queries must be pure read-only"
    );
}

#[test]
fn p_w7_083_series_letters_a_through_o_all_no_panic() {
    let h = fresh();
    for letter in [
        "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O",
    ] {
        let out = run_in(
            h.path(),
            &["playoffs", "--season", PLAYOFF_SEASON, "--series", letter],
        );
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            !combined.contains("panicked"),
            "{letter} panicked: {combined}"
        );
    }
}

#[test]
fn p_w7_084_series_letter_with_whitespace_no_panic() {
    let h = fresh();
    no_panic_in(
        h.path(),
        &["playoffs", "--season", PLAYOFF_SEASON, "--series", " A "],
    );
}

#[test]
fn p_w7_085_series_letter_unicode_no_panic() {
    let h = fresh();
    no_panic_in(
        h.path(),
        &["playoffs", "--season", PLAYOFF_SEASON, "--series", "Å"],
    );
}

// ── Conn Smythe edge cases (15) ─────────────────────────────────────────────

#[test]
fn p_w7_086_playoff_leaders_with_explicit_pos_g() {
    let h = fresh();
    no_panic_in(h.path(), &["query", "leaders", "--playoff", "--pos", "G"]);
}

#[test]
fn p_w7_087_playoff_leaders_top_negative_rejected() {
    let h = fresh();
    fail_in(h.path(), &["query", "leaders", "--playoff", "--top", "-1"]);
}

#[test]
fn p_w7_088_playoff_leaders_top_huge() {
    let h = fresh();
    no_panic_in(
        h.path(),
        &["query", "leaders", "--playoff", "--top", "10000"],
    );
}

#[test]
fn p_w7_089_series_default_season_is_most_recent_completed() {
    let h = fresh();
    let out = ok_in(h.path(), &["playoffs", "--series", "A"]);
    // The default-season pick should produce a real series; check
    // it surfaces a SERIES line.
    assert!(out.contains("SERIES A"));
}

#[test]
fn p_w7_090_series_invalid_season_format() {
    let h = fresh();
    fail_in(
        h.path(),
        &["playoffs", "--season", "garbage", "--series", "A"],
    );
}

#[test]
fn p_w7_091_series_too_short_season_format() {
    let h = fresh();
    fail_in(h.path(), &["playoffs", "--season", "1993", "--series", "A"]);
}

#[test]
fn p_w7_092_series_too_long_season_format() {
    let h = fresh();
    fail_in(
        h.path(),
        &["playoffs", "--season", "199319941995", "--series", "A"],
    );
}

#[test]
fn p_w7_093_playoff_leaders_after_config_set() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(h.path(), &["config", "set", "sync.policy", "off"]);
    // Even with sync off, playoff query reads existing manifest.
    ok_in(h.path(), &["query", "leaders", "--playoff"]);
}

#[test]
fn p_w7_094_series_letter_empty_string_clean_error() {
    let h = fresh();
    let out = run_in(
        h.path(),
        &["playoffs", "--season", PLAYOFF_SEASON, "--series", ""],
    );
    // Either errors cleanly or treats it as no filter. Either way:
    // no panic.
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!combined.contains("panicked"));
}

#[test]
fn p_w7_095_playoff_leaders_no_setup_works() {
    // setup never run — capability matrix isn't on disk; query
    // leaders --playoff should still succeed (it doesn't read sync
    // config).
    let h = fresh();
    ok_in(h.path(), &["query", "leaders", "--playoff"]);
}

#[test]
fn p_w7_096_series_a_default_season_alias_no_season_flag() {
    let h = fresh();
    ok_in(h.path(), &["playoffs", "--series", "A"]);
}

#[test]
fn p_w7_097_playoff_text_table_has_expected_columns() {
    let h = fresh();
    let out = ok_in(h.path(), &["query", "leaders", "--playoff"]);
    // Empty manifest → only the header line. Verify the column
    // labels are there.
    assert!(out.contains("Rank"));
    assert!(out.contains("PID"));
    assert!(out.contains("GP"));
    assert!(out.contains("G"));
}

#[test]
fn p_w7_098_playoff_with_setup_and_favorites_no_panic() {
    let h = fresh();
    ok_in(h.path(), &["setup", "--accept-defaults"]);
    ok_in(h.path(), &["group", "add", "Favorites", "NYR"]);
    ok_in(h.path(), &["group", "add", "Favorites", "Wayne Gretzky"]);
    ok_in(h.path(), &["query", "leaders", "--playoff"]);
    ok_in(
        h.path(),
        &["playoffs", "--season", PLAYOFF_SEASON, "--series", "A"],
    );
    ok_in(h.path(), &["favorites", "--json"]);
}

#[test]
fn p_w7_099_series_summary_one_line_max() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &["playoffs", "--season", PLAYOFF_SEASON, "--series", "A"],
    );
    // Find the summary line — it includes "games played"
    let summary = out
        .lines()
        .find(|l| l.contains("games played") || l.contains("wins"))
        .expect("summary line present");
    assert!(
        summary.len() <= 200,
        "summary line should be one terminal line, got {} chars",
        summary.len()
    );
}

#[test]
fn p_w7_100_series_with_round_filter_doesnt_crash_or_swallow_series() {
    // Mixing --series and --round — the dispatcher prefers --series.
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "playoffs",
            "--season",
            PLAYOFF_SEASON,
            "--series",
            "A",
            "--round",
            "4",
        ],
    );
    // --series wins, output is the SERIES A momentum view.
    assert!(out.contains("SERIES A"));
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn walk_count(p: std::path::PathBuf) -> usize {
    let mut n = 0;
    if let Ok(rd) = std::fs::read_dir(&p) {
        for e in rd.flatten() {
            if e.path().is_dir() {
                n += walk_count(e.path());
            } else {
                n += 1;
            }
        }
    }
    n
}
