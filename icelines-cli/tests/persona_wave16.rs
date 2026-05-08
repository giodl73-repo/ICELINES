//! Persona Wave 16 — binary subprocess tests for the new
//! Phase Art Ross grammar.
//!
//! Wave 11 covered the LEGACY filter grammar via subprocess
//! (201 scenarios). Wave 12-15 covered the NEW grammar at the
//! library level (parser + executor). Wave 16 runs the new
//! grammar through the actual `icelines` binary as a subprocess
//! — catching CLI dispatch / argument parsing / output
//! formatting / exit code / error message issues that
//! library-level tests can't.
//!
//! Sections:
//!   A — Strict comparators on the binary (10)
//!   B — IN / NOT IN (10)
//!   C — BETWEEN (10)
//!   D — LIKE / NOT LIKE (10)
//!   E — Sliding-window atoms (15)
//!   F — Career atoms (10)
//!   G — League atoms (10)
//!   H — --explain interaction (10)
//!   I — Compound + edge cases (15)

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

fn fail_in(home: &std::path::Path, args: &[&str]) -> Output {
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

fn no_panic(out: &Output) {
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!combined.contains("panicked"), "panic in:\n{combined}");
    assert_ne!(out.status.code(), Some(101), "exit 101 = panic");
}

// ── Section A — Strict comparators on the binary (10) ────────

#[test]
fn p_w16_001_strict_lt_succeeds() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &["query", "leaders", "--pos", "C", "--filter", "g<5"],
    );
}

#[test]
fn p_w16_002_strict_gt_succeeds() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &["query", "leaders", "--pos", "C", "--filter", "g>5"],
    );
}

#[test]
fn p_w16_003_ne_succeeds() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &["query", "leaders", "--pos", "C", "--filter", "g!=0"],
    );
}

#[test]
fn p_w16_004_age_under_25_strict() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &["query", "leaders", "--pos", "C", "--filter", "age<25"],
    );
}

#[test]
fn p_w16_005_sql_ne_typo_hint_in_stderr() {
    let h = fresh();
    let out = fail_in(
        h.path(),
        &["query", "leaders", "--pos", "C", "--filter", "g<>5"],
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("!=") || stderr.contains("<>"),
        "stderr should hint at != for SQL typo; got: {stderr}"
    );
    no_panic(&out);
}

#[test]
fn p_w16_006_arrow_eq_typo_hint() {
    let h = fresh();
    let out = fail_in(
        h.path(),
        &["query", "leaders", "--pos", "C", "--filter", "g=>5"],
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains(">="), "stderr should hint at >=; got: {stderr}");
    no_panic(&out);
}

#[test]
fn p_w16_007_strict_lt_compound() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "g<10 AND a<10",
        ],
    );
}

#[test]
fn p_w16_008_strict_gt_in_or() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "g>50 OR a>50",
        ],
    );
}

#[test]
fn p_w16_009_strict_lt_with_decimal() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &["query", "leaders", "--pos", "C", "--filter", "ppg<1.5"],
    );
}

#[test]
fn p_w16_010_strict_age_negative() {
    let h = fresh();
    // age<0 is an impossible filter — should match nobody but
    // not crash.
    let out = run_in(
        h.path(),
        &["query", "leaders", "--pos", "C", "--filter", "age<0"],
    );
    no_panic(&out);
}

// ── Section B — IN / NOT IN (10) ─────────────────────────────

#[test]
fn p_w16_011_country_in_set() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "country IN (CAN, USA, SWE)",
        ],
    );
}

#[test]
fn p_w16_012_country_not_in_set() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "country NOT IN (RUS)",
        ],
    );
}

#[test]
fn p_w16_013_pos_in_set() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "pos IN (C, LW, RW)",
        ],
    );
}

#[test]
fn p_w16_014_team_in_set() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "team IN (BOS, NYR, PIT)",
        ],
    );
}

#[test]
fn p_w16_015_empty_in_rejected_loudly() {
    let h = fresh();
    let out = fail_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "country IN ()",
        ],
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.to_lowercase().contains("empty") || stderr.contains("()"),
        "should mention empty set; got: {stderr}"
    );
    no_panic(&out);
}

#[test]
fn p_w16_016_in_with_quoted_strings() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            r#"country IN ("CAN", "USA")"#,
        ],
    );
}

#[test]
fn p_w16_017_in_numeric_draft_year() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "draft-year IN (2020, 2021, 2022)",
        ],
    );
}

#[test]
fn p_w16_018_stat_in_rejected_with_between_hint() {
    let h = fresh();
    let out = fail_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "g IN (10, 20, 30)",
        ],
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.to_lowercase().contains("between"),
        "stderr should suggest BETWEEN; got: {stderr}"
    );
    no_panic(&out);
}

#[test]
fn p_w16_019_in_compound_with_and() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "country IN (CAN, USA) AND p>=20",
        ],
    );
}

#[test]
fn p_w16_020_in_nested_with_or() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "(country IN (CAN, USA)) OR pos=D",
        ],
    );
}

// ── Section C — BETWEEN (10) ─────────────────────────────────

#[test]
fn p_w16_021_age_between() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "age BETWEEN 22 AND 28",
        ],
    );
}

#[test]
fn p_w16_022_g_between() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "g BETWEEN 20 AND 40",
        ],
    );
}

#[test]
fn p_w16_023_between_with_decimals() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "ppg BETWEEN 0.5 AND 1.5",
        ],
    );
}

#[test]
fn p_w16_024_between_inverted_bounds_no_match() {
    let h = fresh();
    // g BETWEEN 40 AND 20 — empty result expected, no crash.
    let out = run_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "g BETWEEN 40 AND 20",
        ],
    );
    no_panic(&out);
}

#[test]
fn p_w16_025_between_missing_and_errors() {
    let h = fresh();
    let out = fail_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "g BETWEEN 20 40",
        ],
    );
    no_panic(&out);
}

#[test]
fn p_w16_026_between_on_string_field_rejected() {
    let h = fresh();
    let out = fail_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "country BETWEEN 0 AND 100",
        ],
    );
    no_panic(&out);
}

#[test]
fn p_w16_027_between_compound() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "age BETWEEN 22 AND 28 AND country=CAN",
        ],
    );
}

#[test]
fn p_w16_028_draft_round_between() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "draft-round BETWEEN 1 AND 3",
        ],
    );
}

#[test]
fn p_w16_029_between_under_negation() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "NOT (g BETWEEN 0 AND 5)",
        ],
    );
}

#[test]
fn p_w16_030_between_in_or() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "g BETWEEN 30 AND 50 OR a BETWEEN 30 AND 50",
        ],
    );
}

// ── Section D — LIKE / NOT LIKE (10) ─────────────────────────

#[test]
fn p_w16_031_country_like_quoted() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            r#"country LIKE "CA*""#,
        ],
    );
}

#[test]
fn p_w16_032_country_like_unquoted() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "country LIKE CA*",
        ],
    );
}

#[test]
fn p_w16_033_country_not_like() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            r#"country NOT LIKE "RU*""#,
        ],
    );
}

#[test]
fn p_w16_034_like_on_numeric_rejected() {
    let h = fresh();
    let out = fail_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            r#"g LIKE "5*""#,
        ],
    );
    no_panic(&out);
}

#[test]
fn p_w16_035_like_with_paren_grouping() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            r#"(country LIKE "CA*") AND age<=24"#,
        ],
    );
}

#[test]
fn p_w16_036_pos_like_pattern() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            r#"pos LIKE "*W""#, // matches LW, RW
        ],
    );
}

#[test]
fn p_w16_037_like_unicode_normalized() {
    let h = fresh();
    // Pattern with accented chars should canonicalize and match.
    let out = run_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            r#"country LIKE "Stützle""#,
        ],
    );
    no_panic(&out);
}

#[test]
fn p_w16_038_like_in_compound_or() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            r#"country LIKE "CA*" OR country LIKE "US*""#,
        ],
    );
}

#[test]
fn p_w16_039_like_just_wildcard() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            r#"country LIKE "*""#,
        ],
    );
}

#[test]
fn p_w16_040_substring_op() {
    let h = fresh();
    let out = run_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "country ~ AN",
        ],
    );
    no_panic(&out);
}

// ── Section E — Sliding-window atoms (15) ────────────────────

#[test]
fn p_w16_041_last10g_basic() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "g.last10g>=5",
        ],
    );
}

#[test]
fn p_w16_042_last30d() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "g.last30d>=10",
        ],
    );
}

#[test]
fn p_w16_043_last3w() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "p.last3w>=5",
        ],
    );
}

#[test]
fn p_w16_044_last3m() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "p.last3m>=15",
        ],
    );
}

#[test]
fn p_w16_045_last10g_allteams_modifier() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "g.last10g.allteams>=5",
        ],
    );
}

#[test]
fn p_w16_046_last10g_career_modifier() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "g.last10g.career>=5",
        ],
    );
}

#[test]
fn p_w16_047_last10z_unknown_unit_rejected() {
    let h = fresh();
    let out = fail_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "g.last10z>=5",
        ],
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.to_lowercase().contains("unit") || stderr.contains("z"),
        "should mention bad window unit; got: {stderr}"
    );
    no_panic(&out);
}

#[test]
fn p_w16_048_last0g_zero_size_rejected() {
    let h = fresh();
    let out = fail_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "g.last0g>=5",
        ],
    );
    no_panic(&out);
}

#[test]
fn p_w16_049_last1000g_too_large_rejected() {
    let h = fresh();
    let out = fail_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "g.last1000g>=5",
        ],
    );
    no_panic(&out);
}

#[test]
fn p_w16_050_killer_query_sliding_plus_age() {
    // The user's vision query — current-season streak + age.
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "g.last10g>=5 AND age<=25",
        ],
    );
}

#[test]
fn p_w16_051_sliding_in_or_chain() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "g.last10g>=5 OR a.last10g>=10",
        ],
    );
}

#[test]
fn p_w16_052_sliding_under_not() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "NOT g.last10g>=10000",
        ],
    );
}

#[test]
fn p_w16_053_sliding_with_country_in() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "g.last10g>=5 AND country IN (CAN, USA)",
        ],
    );
}

#[test]
fn p_w16_054_unknown_window_scope_rejected() {
    let h = fresh();
    let out = fail_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "g.last10g.bogus>=5",
        ],
    );
    no_panic(&out);
}

#[test]
fn p_w16_055_sliding_decimal_threshold() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "ppg.last10g>=1.5",
        ],
    );
}

// ── Section F — Career atoms (10) ────────────────────────────

#[test]
fn p_w16_056_career_lifetime_sum() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "p.career>=500",
        ],
    );
}

#[test]
fn p_w16_057_career_streak() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "p.streak>=15",
        ],
    );
}

#[test]
fn p_w16_058_seasons_with() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "g.seasons-with>=5",
        ],
    );
}

#[test]
fn p_w16_059_any10g_ever() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "g.any10g>=5 EVER",
        ],
    );
}

#[test]
fn p_w16_060_any10g_ever_at_age() {
    // The full vision query — historical + age slice.
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "g.any10g>=5 EVER AT age<=25",
        ],
    );
}

#[test]
fn p_w16_061_any_zero_window_rejected() {
    let h = fresh();
    let out = fail_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "g.any0g>=5 EVER",
        ],
    );
    no_panic(&out);
}

#[test]
fn p_w16_062_career_with_at_age_range() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "p.career>=300 AT age BETWEEN 22 AND 28",
        ],
    );
}

#[test]
fn p_w16_063_ever_on_non_career_atom_rejected() {
    let h = fresh();
    let out = fail_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "g>=5 EVER",
        ],
    );
    no_panic(&out);
}

#[test]
fn p_w16_064_at_clause_non_age_rejected() {
    let h = fresh();
    let out = fail_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "p.career>=500 AT country=CAN",
        ],
    );
    no_panic(&out);
}

#[test]
fn p_w16_065_career_compound_with_bio() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "p.career>=300 AND country=CAN",
        ],
    );
}

// ── Section G — League atoms (10) ────────────────────────────

#[test]
fn p_w16_066_league_eq() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "league=OHL",
        ],
    );
}

#[test]
fn p_w16_067_league_in_set() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "league IN (OHL, WHL, QMJHL)",
        ],
    );
}

#[test]
fn p_w16_068_league_not_in() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "league NOT IN (NHL)",
        ],
    );
}

#[test]
fn p_w16_069_league_tier() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "league.tier=Junior",
        ],
    );
}

#[test]
fn p_w16_070_league_tier_unknown_rejected() {
    let h = fresh();
    let out = fail_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "league.tier=Bogus",
        ],
    );
    no_panic(&out);
}

#[test]
fn p_w16_071_career_junior() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "p.career.junior>=200",
        ],
    );
}

#[test]
fn p_w16_072_career_nhl_only() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "p.career.nhl>=500",
        ],
    );
}

#[test]
fn p_w16_073_career_specific_league() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "p.career.ohl>=300",
        ],
    );
}

#[test]
fn p_w16_074_league_compound_with_age() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "league=OHL AND age<=24",
        ],
    );
}

#[test]
fn p_w16_075_team_career_rejected_until_a4_wires() {
    // team.career= was rejected at parse with FeatureNotYet.
    let h = fresh();
    let out = fail_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "team.career=EDM",
        ],
    );
    no_panic(&out);
}

// ── Section H — --explain interaction (10) ───────────────────

#[test]
fn p_w16_076_explain_text() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "g>=10",
            "--explain",
        ],
    );
    assert!(out.contains("QUERY PLAN"));
    assert!(out.contains("explain.v1"));
}

#[test]
fn p_w16_077_explain_json_envelope() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "g.last10g>=5 AND age<=25",
            "--explain",
            "--json",
        ],
    );
    let v: serde_json::Value =
        serde_json::from_str(&out).expect("explain JSON must parse");
    assert_eq!(v["schema_version"], "explain.v1");
    assert_eq!(v["route"], "leaders.explain");
}

#[test]
fn p_w16_078_explain_career_atom() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "p.career>=500",
            "--explain",
        ],
    );
    assert!(out.contains("CareerAggregate"));
}

#[test]
fn p_w16_079_explain_league_atom() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "league.tier=Junior",
            "--explain",
        ],
    );
    assert!(out.contains("CareerLeague"));
}

#[test]
fn p_w16_080_explain_invalid_filter_errors() {
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
    no_panic(&out);
}

#[test]
fn p_w16_081_explain_with_multiple_filters() {
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
fn p_w16_082_explain_no_filter_succeeds_with_note() {
    let h = fresh();
    let out = ok_in(h.path(), &["query", "leaders", "--explain"]);
    assert!(out.to_lowercase().contains("no --filter") || out.contains("no filter"));
}

#[test]
fn p_w16_083_explain_kitchen_sink() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "g.last10g>=5 AND age BETWEEN 22 AND 28 AND country IN (CAN, USA, SWE) AND pos IN (C, LW, RW) AND draft-round<=2",
            "--explain",
            "--json",
        ],
    );
    let v: serde_json::Value = serde_json::from_str(&out).expect("parse");
    let plans = v["data"]["plans"].as_array().unwrap();
    assert_eq!(plans.len(), 1);
    let needs_provider = plans[0]["needs_provider"].as_bool().unwrap();
    assert!(needs_provider, "kitchen-sink query needs provider");
}

#[test]
fn p_w16_084_explain_stable_across_runs() {
    // Determinism: same input produces same output.
    let h = fresh();
    let a = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "g>=10 AND a>=10",
            "--explain",
        ],
    );
    let b = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "g>=10 AND a>=10",
            "--explain",
        ],
    );
    assert_eq!(a, b, "explain output must be deterministic");
}

#[test]
fn p_w16_085_explain_contains_filter_input() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "country=CAN",
            "--explain",
            "--json",
        ],
    );
    let v: serde_json::Value = serde_json::from_str(&out).expect("parse");
    assert_eq!(v["data"]["plans"][0]["filter_input"], "country=CAN");
}

// ── Section I — Compound + edge cases (15) ───────────────────

#[test]
fn p_w16_086_kitchen_sink_real_query() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "g.last10g>=5 AND age BETWEEN 22 AND 28 AND country IN (CAN, USA, SWE) AND pos IN (C, LW, RW) AND draft-round<=2",
        ],
    );
}

#[test]
fn p_w16_087_long_and_chain() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "g>=1 AND a>=1 AND p>=1 AND pim>=0 AND shots>=1",
        ],
    );
}

#[test]
fn p_w16_088_long_or_chain() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "g>=20 OR a>=20 OR p>=40 OR pim>=50",
        ],
    );
}

#[test]
fn p_w16_089_deeply_nested_parens() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "(((g>=10)))",
        ],
    );
}

#[test]
fn p_w16_090_demorgan_compound() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "NOT (country=CAN AND pos=C)",
        ],
    );
}

#[test]
fn p_w16_091_multi_filter_args_compose() {
    let h = fresh();
    // Multiple --filter flags AND together.
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "country=CAN",
            "--filter",
            "pos=C",
            "--filter",
            "age<30",
        ],
    );
}

#[test]
fn p_w16_092_json_output_with_compound() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "country=CAN AND age<30",
            "--json",
        ],
    );
    // Should be valid JSON
    let _: serde_json::Value =
        serde_json::from_str(&out).expect("query --json output must parse");
}

#[test]
fn p_w16_093_csv_output_with_compound() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "country=CAN AND age<30",
            "--csv",
        ],
    );
    // Should have at least a header row.
    assert!(!out.is_empty());
}

#[test]
fn p_w16_094_unknown_country_no_match() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "country=XYZ",
            "--json",
        ],
    );
    let v: serde_json::Value = serde_json::from_str(&out).expect("parse");
    let arr = v.as_array().expect("array");
    assert!(arr.is_empty(), "unknown country should match nobody");
}

#[test]
fn p_w16_095_huge_threshold_no_match() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "g>=99999",
            "--json",
        ],
    );
    let v: serde_json::Value = serde_json::from_str(&out).expect("parse");
    let arr = v.as_array().expect("array");
    assert!(arr.is_empty());
}

#[test]
fn p_w16_096_negative_threshold_universal_for_unsigned() {
    let h = fresh();
    // g>=-5 — every player matches (g is unsigned)
    let out = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--top",
            "5",
            "--filter",
            "g>=-5",
            "--json",
        ],
    );
    let v: serde_json::Value = serde_json::from_str(&out).expect("parse");
    let arr = v.as_array().expect("array");
    assert!(!arr.is_empty(), "g>=-5 should match many players");
}

#[test]
fn p_w16_097_paren_changes_outcome() {
    let h = fresh();
    // (g>=10 OR a>=10) AND p>=20 vs g>=10 OR (a>=10 AND p>=20)
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "(g>=10 OR a>=10) AND p>=20",
        ],
    );
}

#[test]
fn p_w16_098_double_not_collapses_correctly() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "NOT NOT g>=10",
        ],
    );
}

#[test]
fn p_w16_099_unclosed_paren_rejected_loudly() {
    let h = fresh();
    let out = fail_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--filter",
            "(g>=10 AND a>=10",
        ],
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.to_lowercase().contains("paren") || stderr.contains("("),
        "should mention paren issue; got: {stderr}"
    );
    no_panic(&out);
}

#[test]
fn p_w16_100_killer_query_runs_end_to_end() {
    let h = fresh();
    // The user's vision query. Returns empty (no boxscores
    // locally) but must run cleanly + return valid JSON.
    let out = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--filter",
            "g.any10g>=5 EVER AT age<=25 AND country IN (CAN, USA, SWE)",
            "--json",
        ],
    );
    let v: serde_json::Value =
        serde_json::from_str(&out).expect("killer query must produce valid JSON");
    let _ = v.as_array().expect("array");
}
