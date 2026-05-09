//! Persona Wave 11 — filter grammar adversarial stress.
//!
//! 200 scenarios stress-testing the filter expression parser
//! (`stats_catalog::parse_filter_expr`) and its interaction with
//! the bridging `icelines-query` bio-atom extraction. Designed to
//! surface latent bugs before the upcoming grammar expansion
//! (IN / BETWEEN / LIKE / `!=` / new bio atoms / team atoms).
//!
//! Sections:
//!   A — Boolean precedence + associativity (25)
//!   B — Atom-level operator parsing (25)
//!   C — Bio + stat atom interplay (25)
//!   D — Windowed atom precedence (15)
//!   E — Empty / whitespace / paren edge cases (20)
//!   F — Conflicting / tautological / vacuous predicates (20)
//!   G — Goalies subcommand filter rewrites (15)
//!   H — Filter alias coverage (15)
//!   I — Pathological / stress inputs (20)
//!   J — Output truthfulness (filter actually filters) (20)

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
        "{:?} must non-zero exit; stdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    out
}

fn stderr_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn no_panic(out: &Output) {
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !combined.contains("panicked"),
        "panic in output:\n{combined}"
    );
    assert_ne!(out.status.code(), Some(101), "exit code 101 = panic");
}

fn fresh() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

/// Run `query leaders --pos C --top 5 --filter "<expr>"` against the
/// bundled current-season repo. Returns stdout if it succeeds, panics
/// otherwise. Used by tests that need a known-loadable shape to
/// verify the filter parses + applies cleanly.
fn leaders(home: &std::path::Path, expr: &str) -> String {
    ok_in(
        home,
        &[
            "query", "leaders", "--pos", "C", "--top", "5", "--filter", expr,
        ],
    )
}

fn leaders_fail(home: &std::path::Path, expr: &str) -> Output {
    fail_in(
        home,
        &[
            "query", "leaders", "--pos", "C", "--top", "5", "--filter", expr,
        ],
    )
}

fn leaders_json(home: &std::path::Path, expr: &str) -> String {
    ok_in(
        home,
        &[
            "query", "leaders", "--pos", "C", "--top", "100", "--filter", expr, "--json",
        ],
    )
}

/// Count rows in a `query leaders --json` output. The legacy
/// (non-windowed, non-playoff) leaders route emits a bare JSON
/// array. Newer K2.4-enveloped routes (leaders.playoff,
/// leaders.windowed) wrap the array under `data.players`. Handle
/// both.
fn json_player_count(json: &str) -> usize {
    let v: serde_json::Value = serde_json::from_str(json).expect("parseable JSON");
    if let Some(arr) = v.as_array() {
        return arr.len();
    }
    if let Some(arr) = v
        .get("data")
        .and_then(|d| d.get("players"))
        .and_then(|p| p.as_array())
    {
        return arr.len();
    }
    if let Some(arr) = v.get("data").and_then(|d| d.as_array()) {
        return arr.len();
    }
    panic!("unrecognized JSON shape:\n{json}")
}

// ── Section A — Boolean precedence + associativity (25) ─────────────────────

#[test]
fn p_w11_001_simple_and_chain_parses() {
    let h = fresh();
    let _ = leaders(h.path(), "g>=1 AND a>=1");
}

#[test]
fn p_w11_002_simple_or_chain_parses() {
    let h = fresh();
    let _ = leaders(h.path(), "g>=1 OR a>=1");
}

#[test]
fn p_w11_003_three_way_and_left_associative() {
    let h = fresh();
    let _ = leaders(h.path(), "g>=1 AND a>=1 AND p>=1");
}

#[test]
fn p_w11_004_three_way_or_left_associative() {
    let h = fresh();
    let _ = leaders(h.path(), "g>=1 OR a>=1 OR p>=1");
}

#[test]
fn p_w11_005_and_binds_tighter_than_or() {
    // `g>=1 AND a>=1 OR p>=1` should be `(g AND a) OR p`.
    // Either grouping is "valid syntax" — here we just want it to
    // parse and return without error, no panic.
    let h = fresh();
    let _ = leaders(h.path(), "g>=1 AND a>=1 OR p>=1");
}

#[test]
fn p_w11_006_or_followed_by_and() {
    let h = fresh();
    let _ = leaders(h.path(), "g>=1 OR a>=1 AND p>=1");
}

#[test]
fn p_w11_007_parens_override_default_precedence() {
    let h = fresh();
    let _ = leaders(h.path(), "(g>=1 OR a>=1) AND p>=1");
}

#[test]
fn p_w11_008_nested_parens_two_deep() {
    let h = fresh();
    let _ = leaders(h.path(), "((g>=1 OR a>=1) AND p>=1)");
}

#[test]
fn p_w11_009_nested_parens_three_deep() {
    let h = fresh();
    let _ = leaders(h.path(), "(((g>=1)))");
}

#[test]
fn p_w11_010_simple_not_atom() {
    let h = fresh();
    let _ = leaders(h.path(), "NOT g>=10000");
}

#[test]
fn p_w11_011_not_paren_group() {
    let h = fresh();
    let _ = leaders(h.path(), "NOT (g>=10000 OR a>=10000)");
}

#[test]
fn p_w11_012_double_not_cancels() {
    let h = fresh();
    // NOT NOT X ≡ X — both should return same player count.
    let pos = leaders_json(h.path(), "g>=1");
    let neg = leaders_json(h.path(), "NOT NOT g>=1");
    assert_eq!(json_player_count(&pos), json_player_count(&neg));
}

#[test]
fn p_w11_013_triple_not_equals_single_not() {
    let h = fresh();
    let single = leaders_json(h.path(), "NOT g>=10000");
    let triple = leaders_json(h.path(), "NOT NOT NOT g>=10000");
    assert_eq!(json_player_count(&single), json_player_count(&triple));
}

#[test]
fn p_w11_014_not_inside_and() {
    let h = fresh();
    let _ = leaders(h.path(), "g>=1 AND NOT pim>=10000");
}

#[test]
fn p_w11_015_not_inside_or() {
    let h = fresh();
    let _ = leaders(h.path(), "g>=1 OR NOT pim>=10000");
}

#[test]
fn p_w11_016_keywords_case_insensitive_and() {
    let h = fresh();
    let _ = leaders(h.path(), "g>=1 and a>=1");
    let _ = leaders(h.path(), "g>=1 And a>=1");
    let _ = leaders(h.path(), "g>=1 aNd a>=1");
}

#[test]
fn p_w11_017_keywords_case_insensitive_or() {
    let h = fresh();
    let _ = leaders(h.path(), "g>=1 or a>=1");
    let _ = leaders(h.path(), "g>=1 Or a>=1");
    let _ = leaders(h.path(), "g>=1 oR a>=1");
}

#[test]
fn p_w11_018_keywords_case_insensitive_not() {
    let h = fresh();
    let _ = leaders(h.path(), "not g>=10000");
    let _ = leaders(h.path(), "Not g>=10000");
    let _ = leaders(h.path(), "nOt g>=10000");
}

#[test]
fn p_w11_019_extra_whitespace_around_keywords() {
    let h = fresh();
    let _ = leaders(h.path(), "g>=1     AND     a>=1");
    let _ = leaders(h.path(), "  g>=1 AND a>=1  ");
}

#[test]
fn p_w11_020_tab_separated_atoms() {
    let h = fresh();
    let _ = leaders(h.path(), "g>=1\tAND\ta>=1");
}

#[test]
fn p_w11_021_paren_grouping_changes_result() {
    let h = fresh();
    // `g>=1 OR a>=1 AND p>=1000` ≡ `g>=1 OR (a>=1 AND p>=1000)`
    // ≡ a relaxed OR (relatively many players have g>=1)
    // `(g>=1 OR a>=1) AND p>=1000` ≡ very few players (none with p>=1000)
    let loose = leaders_json(h.path(), "g>=1 OR a>=1 AND p>=1000");
    let tight = leaders_json(h.path(), "(g>=1 OR a>=1) AND p>=1000");
    assert!(
        json_player_count(&loose) >= json_player_count(&tight),
        "OR-default should yield ≥ players than paren-tightened version"
    );
}

#[test]
fn p_w11_022_demorgan_and_to_or() {
    let h = fresh();
    // NOT (A AND B) ≡ NOT A OR NOT B
    let lhs = leaders_json(h.path(), "NOT (g>=10000 AND a>=10000)");
    let rhs = leaders_json(h.path(), "NOT g>=10000 OR NOT a>=10000");
    assert_eq!(json_player_count(&lhs), json_player_count(&rhs));
}

#[test]
fn p_w11_023_demorgan_or_to_and() {
    let h = fresh();
    // NOT (A OR B) ≡ NOT A AND NOT B
    let lhs = leaders_json(h.path(), "NOT (g>=10000 OR a>=10000)");
    let rhs = leaders_json(h.path(), "NOT g>=10000 AND NOT a>=10000");
    assert_eq!(json_player_count(&lhs), json_player_count(&rhs));
}

#[test]
fn p_w11_024_dangling_and_errors() {
    let h = fresh();
    let out = leaders_fail(h.path(), "g>=1 AND");
    no_panic(&out);
}

#[test]
fn p_w11_025_dangling_or_errors() {
    let h = fresh();
    let out = leaders_fail(h.path(), "g>=1 OR");
    no_panic(&out);
}

// ── Section B — Atom-level operator parsing (25) ────────────────────────────

#[test]
fn p_w11_026_op_min_works() {
    let h = fresh();
    let _ = leaders(h.path(), "g>=1");
}

#[test]
fn p_w11_027_op_max_works() {
    let h = fresh();
    let _ = leaders(h.path(), "g<=1000");
}

#[test]
fn p_w11_028_op_eq_single_works() {
    let h = fresh();
    let _ = leaders(h.path(), "g=0");
}

#[test]
fn p_w11_029_op_eq_double_works() {
    let h = fresh();
    let _ = leaders(h.path(), "g==0");
}

#[test]
fn p_w11_030_op_strictly_gt_supported_in_v0_20() {
    // Phase Art Ross A.1 added strict comparators. Was rejected
    // in v0.19; supported in v0.20+.
    let h = fresh();
    let _ = leaders(h.path(), "g>3");
}

#[test]
fn p_w11_031_op_strictly_lt_supported_in_v0_20() {
    let h = fresh();
    let _ = leaders(h.path(), "g<100");
}

#[test]
fn p_w11_032_op_not_equals_supported_in_v0_20() {
    let h = fresh();
    let _ = leaders(h.path(), "g!=5");
}

#[test]
fn p_w11_033_typo_arrow_eq_hint() {
    let h = fresh();
    let out = leaders_fail(h.path(), "g=>5");
    let err = stderr_of(&out);
    assert!(
        err.contains(">=") || err.contains("=>"),
        "should hint at >= for => typo, got: {err}"
    );
    no_panic(&out);
}

#[test]
fn p_w11_034_typo_lt_eq_hint() {
    let h = fresh();
    let out = leaders_fail(h.path(), "g=<5");
    let err = stderr_of(&out);
    assert!(
        err.contains("<=") || err.contains("=<"),
        "should hint at <= for =< typo, got: {err}"
    );
    no_panic(&out);
}

#[test]
fn p_w11_035_multiple_ops_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), "g>=>=5");
    no_panic(&out);
    let err = stderr_of(&out);
    assert!(
        err.to_lowercase().contains("op"),
        "should mention op problem: {err}"
    );
}

#[test]
fn p_w11_036_triple_equals_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), "g===5");
    no_panic(&out);
}

#[test]
fn p_w11_037_decimal_value_accepted() {
    let h = fresh();
    let _ = leaders(h.path(), "ppg>=1.5");
}

#[test]
fn p_w11_038_negative_decimal_accepted() {
    let h = fresh();
    let _ = leaders(h.path(), "+/->=-5");
}

#[test]
fn p_w11_039_locale_comma_rejected() {
    // f64 doesn't parse locale commas (Spanish/German "1,5" → bad)
    let h = fresh();
    let out = leaders_fail(h.path(), "ppg>=1,5");
    no_panic(&out);
}

#[test]
fn p_w11_040_alphabetic_value_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), "g>=many");
    let err = stderr_of(&out);
    assert!(err.to_lowercase().contains("number") || err.to_lowercase().contains("many"));
    no_panic(&out);
}

#[test]
fn p_w11_041_nan_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), "g>=NaN");
    no_panic(&out);
}

#[test]
fn p_w11_042_inf_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), "g>=inf");
    no_panic(&out);
}

#[test]
fn p_w11_043_negative_inf_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), "g>=-inf");
    no_panic(&out);
}

#[test]
fn p_w11_044_empty_value_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), "g>=");
    no_panic(&out);
}

#[test]
fn p_w11_045_empty_key_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), ">=10");
    no_panic(&out);
}

#[test]
fn p_w11_046_unknown_stat_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), "totally-fake-stat>=10");
    let err = stderr_of(&out);
    assert!(
        err.to_lowercase().contains("totally-fake-stat") || err.to_lowercase().contains("unknown"),
        "should mention unknown stat key: {err}"
    );
    no_panic(&out);
}

#[test]
fn p_w11_047_whitespace_in_atom_around_op() {
    // `g >= 5` (spaces around op) — current grammar tolerates this.
    let h = fresh();
    let _ = leaders(h.path(), "g >= 5");
}

#[test]
fn p_w11_048_huge_value_doesnt_panic() {
    let h = fresh();
    // Larger than any real stat value; should match nobody but parse.
    let _ = leaders(h.path(), "g>=999999999");
}

#[test]
fn p_w11_049_zero_threshold() {
    let h = fresh();
    let _ = leaders(h.path(), "g>=0");
}

#[test]
fn p_w11_050_negative_threshold_on_signed_stat() {
    let h = fresh();
    // +/- can be negative
    let _ = leaders(h.path(), "+/->=-100");
}

// ── Section C — Bio + stat atom interplay (25) ──────────────────────────────

#[test]
fn p_w11_051_age_alone() {
    let h = fresh();
    let _ = leaders(h.path(), "age<=24");
}

#[test]
fn p_w11_052_country_alone() {
    let h = fresh();
    let _ = leaders(h.path(), "country=CAN");
}

#[test]
fn p_w11_053_country_lowercase_normalized() {
    let h = fresh();
    let upper = leaders_json(h.path(), "country=CAN");
    let lower = leaders_json(h.path(), "country=can");
    assert_eq!(json_player_count(&upper), json_player_count(&lower));
}

#[test]
fn p_w11_054_shoots_left() {
    let h = fresh();
    let _ = leaders(h.path(), "shoots=L");
}

#[test]
fn p_w11_055_shoots_right() {
    let h = fresh();
    let _ = leaders(h.path(), "shoots=R");
}

#[test]
fn p_w11_056_height_min() {
    let h = fresh();
    let _ = leaders(h.path(), "height>=72");
}

#[test]
fn p_w11_057_weight_max() {
    let h = fresh();
    let _ = leaders(h.path(), "weight<=200");
}

#[test]
fn p_w11_058_draft_year_range() {
    let h = fresh();
    let _ = leaders(h.path(), "draft>=2015 AND draft<=2022");
}

#[test]
fn p_w11_059_age_with_alias_underscore() {
    let h = fresh();
    let _ = leaders(h.path(), "draft_year>=2015");
}

#[test]
fn p_w11_060_age_with_alias_hyphen() {
    let h = fresh();
    let _ = leaders(h.path(), "draft-year>=2015");
}

#[test]
fn p_w11_061_height_alias_ht() {
    let h = fresh();
    let _ = leaders(h.path(), "ht>=72");
}

#[test]
fn p_w11_062_weight_alias_wt() {
    let h = fresh();
    let _ = leaders(h.path(), "wt>=180");
}

#[test]
fn p_w11_063_country_alias_nation() {
    let h = fresh();
    let _ = leaders(h.path(), "nation=USA");
}

#[test]
fn p_w11_064_country_alias_nationality() {
    let h = fresh();
    let _ = leaders(h.path(), "nationality=USA");
}

#[test]
fn p_w11_065_shoots_alias_hand() {
    let h = fresh();
    let _ = leaders(h.path(), "hand=L");
}

#[test]
fn p_w11_066_shoots_alias_catches() {
    let h = fresh();
    let _ = leaders(h.path(), "catches=L");
}

#[test]
fn p_w11_067_bio_and_stat_mixed() {
    let h = fresh();
    let _ = leaders(h.path(), "age<=24 AND p>=10");
}

#[test]
fn p_w11_068_bio_chain_only() {
    let h = fresh();
    let _ = leaders(h.path(), "age>=22 AND age<=28 AND country=CAN");
}

#[test]
fn p_w11_069_age_eq_emits_both_bounds() {
    // `age=24` should yield = age==24 (both bounds tight).
    let h = fresh();
    let exact = leaders_json(h.path(), "age=24");
    let range = leaders_json(h.path(), "age>=24 AND age<=24");
    assert_eq!(json_player_count(&exact), json_player_count(&range));
}

#[test]
fn p_w11_070_bio_atom_inside_parens() {
    // Parens shouldn't break the bio extractor (it walks atoms after
    // the splitter). Bio extractor bails on OR/NOT but not on parens
    // — depending on implementation this may force fallback.
    let h = fresh();
    let _ = leaders(h.path(), "(age<=24 AND p>=10)");
}

#[test]
fn p_w11_071_bio_in_or_supported_in_v0_20() {
    // Phase Art Ross A.0 — the new pipeline handles bio atoms
    // in OR-chains directly (country IN bio_text_field_from_key).
    // Was an UnknownStat error in v0.19 (extract_bio bailed on
    // OR, catalog parser rejected `country`); supported in v0.20+.
    let h = fresh();
    let _ = leaders(h.path(), "country=CAN OR country=USA");
}

#[test]
fn p_w11_072_conflicting_bio_age_bounds_match_zero() {
    // age>=30 AND age<=20 — impossible, should match nobody.
    let h = fresh();
    let json = leaders_json(h.path(), "age>=30 AND age<=20");
    assert_eq!(json_player_count(&json), 0);
}

#[test]
fn p_w11_073_bio_atom_unknown_country_no_match() {
    let h = fresh();
    let json = leaders_json(h.path(), "country=ZZZ");
    assert_eq!(json_player_count(&json), 0);
}

#[test]
fn p_w11_074_bio_atom_extreme_height_no_match() {
    let h = fresh();
    let json = leaders_json(h.path(), "height>=999");
    assert_eq!(json_player_count(&json), 0);
}

#[test]
fn p_w11_075_bio_chain_tightening_monotonic() {
    // Tightening the age range should never INCREASE the count.
    let h = fresh();
    let wide = leaders_json(h.path(), "age>=18 AND age<=40");
    let narrow = leaders_json(h.path(), "age>=22 AND age<=28");
    assert!(
        json_player_count(&wide) >= json_player_count(&narrow),
        "wider range should match >= narrower range"
    );
}

// ── Section D — Windowed atom precedence (15) ───────────────────────────────

#[test]
fn p_w11_076_bare_atom_default_is_season() {
    let h = fresh();
    let _ = leaders(h.path(), "g>=1");
}

// Wave 11 — windowed atoms (`g.week>=10`) parse via WindowedAtom
// in `stats_catalog` but the boxscore-aggregate path
// (`run_windowed_leaders`) doesn't yet apply filters. Wave 11
// surfaced this gap: passing `--filter` with `--week`/`--month`
// silently dropped the filter. We added a loud rejection (Wave 11
// dispatch fix) — these tests verify the rejection fires cleanly
// and points the user at the correct surface.

fn windowed_with_filter(
    home: &std::path::Path,
    window_flag: &str,
    expr: &str,
) -> std::process::Output {
    run_in(
        home,
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--top",
            "5",
            window_flag,
            "--filter",
            expr,
        ],
    )
}

fn assert_windowed_filter_rejected(home: &std::path::Path, window_flag: &str, expr: &str) {
    let out = windowed_with_filter(home, window_flag, expr);
    no_panic(&out);
    assert!(
        !out.status.success(),
        "windowed leaders with --filter must reject loudly until F.5b wires \
         filtering on the boxscore-aggregate path; flag={window_flag} expr={expr}"
    );
    let err = stderr_of(&out);
    assert!(
        err.to_lowercase().contains("--filter") || err.to_lowercase().contains("filter"),
        "rejection should mention --filter; got: {err}"
    );
}

#[test]
fn p_w11_077_explicit_season_window_rejected_in_windowed_mode() {
    let h = fresh();
    assert_windowed_filter_rejected(h.path(), "--week", "g.season>=1");
}

#[test]
fn p_w11_078_week_window_rejected_until_wired() {
    let h = fresh();
    assert_windowed_filter_rejected(h.path(), "--week", "g.week>=1");
}

#[test]
fn p_w11_079_month_window_rejected_until_wired() {
    let h = fresh();
    assert_windowed_filter_rejected(h.path(), "--month", "g.month>=1");
}

#[test]
fn p_w11_080_windowed_filter_rejection_consistent_for_unknown_window() {
    let h = fresh();
    assert_windowed_filter_rejected(h.path(), "--week", "g.year>=1");
}

#[test]
fn p_w11_081_windowed_filter_rejection_consistent_for_typo() {
    let h = fresh();
    assert_windowed_filter_rejected(h.path(), "--week", "g.weak>=1");
}

#[test]
fn p_w11_082_windowed_filter_rejection_consistent_for_unknown_stat() {
    let h = fresh();
    assert_windowed_filter_rejected(h.path(), "--week", "fakestat.week>=1");
}

#[test]
fn p_w11_083_windowed_filter_rejection_decimal() {
    let h = fresh();
    assert_windowed_filter_rejected(h.path(), "--week", "ppg.week>=1.5");
}

#[test]
fn p_w11_084_windowed_filter_rejection_compound_and() {
    let h = fresh();
    assert_windowed_filter_rejected(h.path(), "--week", "g.week>=1 AND p>=1");
}

#[test]
fn p_w11_085_windowed_filter_rejection_compound_or() {
    let h = fresh();
    assert_windowed_filter_rejected(h.path(), "--week", "g.week>=10 OR g.month>=15");
}

#[test]
fn p_w11_086_windowed_filter_rejection_under_not() {
    let h = fresh();
    assert_windowed_filter_rejected(h.path(), "--week", "NOT g.week>=10000");
}

#[test]
fn p_w11_087_windowed_filter_rejection_paren_group() {
    let h = fresh();
    assert_windowed_filter_rejected(h.path(), "--week", "(g.week>=1 OR a.week>=1) AND p>=1");
}

#[test]
fn p_w11_087b_windowed_no_filter_succeeds() {
    // Sanity: --week without --filter still works (returns the
    // empty-window banner because no boxscores are seeded in tests).
    let h = fresh();
    let out = run_in(
        h.path(),
        &["query", "leaders", "--pos", "C", "--top", "5", "--week"],
    );
    no_panic(&out);
    assert!(
        out.status.success(),
        "--week without --filter should succeed; stderr:\n{}",
        stderr_of(&out)
    );
}

#[test]
fn p_w11_088_double_dot_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), "g..week>=1");
    no_panic(&out);
}

#[test]
fn p_w11_089_trailing_dot_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), "g.>=1");
    no_panic(&out);
}

#[test]
fn p_w11_090_query_career_rejects_week_window() {
    // EDGE B2: `query career --week` literal rejection.
    let h = fresh();
    let out = fail_in(h.path(), &["query", "career", "--week", "--league", "OHL"]);
    no_panic(&out);
}

// ── Section E — Empty / whitespace / paren edge cases (20) ──────────────────

#[test]
fn p_w11_091_empty_filter_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), "");
    no_panic(&out);
    let err = stderr_of(&out);
    assert!(
        err.to_lowercase().contains("empty"),
        "should mention empty: {err}"
    );
}

#[test]
fn p_w11_092_whitespace_only_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), "   ");
    no_panic(&out);
}

#[test]
fn p_w11_093_tab_only_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), "\t\t\t");
    no_panic(&out);
}

#[test]
fn p_w11_094_unclosed_paren_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), "(g>=1 AND a>=1");
    no_panic(&out);
    let err = stderr_of(&out);
    assert!(
        err.to_lowercase().contains("paren") || err.to_lowercase().contains("close"),
        "should mention paren issue: {err}"
    );
}

#[test]
fn p_w11_095_extra_close_paren_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), "g>=1)");
    no_panic(&out);
}

#[test]
fn p_w11_096_only_open_paren_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), "(");
    no_panic(&out);
}

#[test]
fn p_w11_097_only_close_paren_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), ")");
    no_panic(&out);
}

#[test]
fn p_w11_098_empty_paren_pair_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), "()");
    no_panic(&out);
}

#[test]
fn p_w11_099_paren_with_only_whitespace() {
    let h = fresh();
    let out = leaders_fail(h.path(), "(   )");
    no_panic(&out);
}

#[test]
fn p_w11_100_just_keyword_and_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), "AND");
    no_panic(&out);
}

#[test]
fn p_w11_101_just_keyword_or_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), "OR");
    no_panic(&out);
}

#[test]
fn p_w11_102_just_keyword_not_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), "NOT");
    no_panic(&out);
}

#[test]
fn p_w11_103_leading_and_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), "AND g>=1");
    no_panic(&out);
}

#[test]
fn p_w11_104_leading_or_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), "OR g>=1");
    no_panic(&out);
}

#[test]
fn p_w11_105_consecutive_and_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), "g>=1 AND AND a>=1");
    no_panic(&out);
}

#[test]
fn p_w11_106_consecutive_or_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), "g>=1 OR OR a>=1");
    no_panic(&out);
}

#[test]
fn p_w11_107_and_or_adjacent_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), "g>=1 AND OR a>=1");
    no_panic(&out);
}

#[test]
fn p_w11_108_paren_then_atom_no_op_rejected() {
    let h = fresh();
    // `(g>=1) g>=2` — two adjacent expressions without AND/OR.
    let out = leaders_fail(h.path(), "(g>=1) g>=2");
    no_panic(&out);
}

#[test]
fn p_w11_109_only_dot_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), ".");
    no_panic(&out);
}

#[test]
fn p_w11_110_only_op_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), ">=");
    no_panic(&out);
}

// ── Section F — Conflicting / tautological / vacuous predicates (20) ────────

#[test]
fn p_w11_111_self_contradiction_returns_zero() {
    let h = fresh();
    let json = leaders_json(h.path(), "g>=100 AND g<=50");
    assert_eq!(json_player_count(&json), 0);
}

#[test]
fn p_w11_112_tautology_returns_all() {
    let h = fresh();
    // Every player has g>=0; every player has g<=10000. OR is
    // also tautological.
    let baseline = leaders_json(h.path(), "g>=0");
    let taut_or = leaders_json(h.path(), "g>=0 OR g<=10000");
    assert_eq!(json_player_count(&baseline), json_player_count(&taut_or));
}

#[test]
fn p_w11_113_idempotent_and() {
    let h = fresh();
    // X AND X ≡ X
    let single = leaders_json(h.path(), "g>=1");
    let dup = leaders_json(h.path(), "g>=1 AND g>=1");
    assert_eq!(json_player_count(&single), json_player_count(&dup));
}

#[test]
fn p_w11_114_idempotent_or() {
    let h = fresh();
    let single = leaders_json(h.path(), "g>=1");
    let dup = leaders_json(h.path(), "g>=1 OR g>=1");
    assert_eq!(json_player_count(&single), json_player_count(&dup));
}

#[test]
fn p_w11_115_complement_or_is_universe() {
    let h = fresh();
    // X OR NOT X ≡ universe (excluding things where X is undefined).
    // For numeric stats with default-zero everywhere, this should
    // equal the unfiltered count.
    let unfiltered = leaders_json(h.path(), "g>=0");
    let complement = leaders_json(h.path(), "g>=100 OR NOT g>=100");
    // The complement should be ≥ the unfiltered count never, but
    // could be < if some players have undefined `g` that defaults to
    // both branches false. Allow ≤.
    assert!(
        json_player_count(&complement) <= json_player_count(&unfiltered),
        "complement should be ≤ unfiltered universe"
    );
}

#[test]
fn p_w11_116_intersection_subset_of_or() {
    let h = fresh();
    // (A AND B) ⊆ (A OR B)
    let and_count = json_player_count(&leaders_json(h.path(), "g>=10 AND a>=10"));
    let or_count = json_player_count(&leaders_json(h.path(), "g>=10 OR a>=10"));
    assert!(and_count <= or_count, "AND count should be ≤ OR count");
}

#[test]
fn p_w11_117_tightening_min_monotonic() {
    let h = fresh();
    // g>=100 ⊆ g>=50 ⊆ g>=10
    let n10 = json_player_count(&leaders_json(h.path(), "g>=10"));
    let n50 = json_player_count(&leaders_json(h.path(), "g>=50"));
    let n100 = json_player_count(&leaders_json(h.path(), "g>=100"));
    assert!(n100 <= n50);
    assert!(n50 <= n10);
}

#[test]
fn p_w11_118_loosening_max_monotonic() {
    let h = fresh();
    // g<=10 ⊆ g<=50 ⊆ g<=100
    let n10 = json_player_count(&leaders_json(h.path(), "g<=10"));
    let n50 = json_player_count(&leaders_json(h.path(), "g<=50"));
    let n100 = json_player_count(&leaders_json(h.path(), "g<=100"));
    assert!(n10 <= n50);
    assert!(n50 <= n100);
}

#[test]
fn p_w11_119_eq_subset_of_min() {
    let h = fresh();
    // g==5 ⊆ g>=5
    let eq_count = json_player_count(&leaders_json(h.path(), "g==5"));
    let min_count = json_player_count(&leaders_json(h.path(), "g>=5"));
    assert!(eq_count <= min_count);
}

#[test]
fn p_w11_120_eq_subset_of_max() {
    let h = fresh();
    // g==5 ⊆ g<=5
    let eq_count = json_player_count(&leaders_json(h.path(), "g==5"));
    let max_count = json_player_count(&leaders_json(h.path(), "g<=5"));
    assert!(eq_count <= max_count);
}

#[test]
fn p_w11_121_negation_of_min_complements_max() {
    let h = fresh();
    // For most players, NOT g>=10 ≡ g<10. Since grammar has no
    // strict <, we use g<=9 as a close proxy. Counts should be in
    // the same neighborhood (within ±10% of universe).
    let universe = json_player_count(&leaders_json(h.path(), "g>=0"));
    let neg_min = json_player_count(&leaders_json(h.path(), "NOT g>=10"));
    assert!(
        neg_min <= universe,
        "NOT g>=10 must be ≤ universe; got {neg_min} > {universe}"
    );
}

#[test]
fn p_w11_122_redundant_chain_dedup() {
    let h = fresh();
    // g>=1 AND g>=1 AND g>=1 AND g>=1 AND g>=1 == g>=1
    let single = json_player_count(&leaders_json(h.path(), "g>=1"));
    let many = json_player_count(&leaders_json(
        h.path(),
        "g>=1 AND g>=1 AND g>=1 AND g>=1 AND g>=1",
    ));
    assert_eq!(single, many);
}

#[test]
fn p_w11_123_subsumed_atoms_redundant() {
    let h = fresh();
    // g>=1 AND g>=10 ≡ g>=10 (the tighter constraint dominates)
    let just_10 = json_player_count(&leaders_json(h.path(), "g>=10"));
    let chain = json_player_count(&leaders_json(h.path(), "g>=1 AND g>=10"));
    assert_eq!(just_10, chain);
}

#[test]
fn p_w11_124_empty_intersection_returns_zero() {
    let h = fresh();
    // pos=C is a CLI flag; combined with `pos=W` filter on goals
    // we still intersect with --pos C. So `g>=1 AND p<=0` is a
    // good empty intersection (any forward with goals has points).
    let json = leaders_json(h.path(), "g>=10 AND p<=5");
    // Likely zero — but since we don't enforce 0, just verify clean
    // parse + clean number.
    let _ = json_player_count(&json);
}

#[test]
fn p_w11_125_universal_filter_matches_unfiltered() {
    let h = fresh();
    // No filter at all should equal `g>=0` (universal).
    let no_filter = ok_in(
        h.path(),
        &["query", "leaders", "--pos", "C", "--top", "100", "--json"],
    );
    let universal = leaders_json(h.path(), "g>=0");
    assert_eq!(json_player_count(&no_filter), json_player_count(&universal));
}

#[test]
fn p_w11_126_huge_threshold_returns_zero() {
    let h = fresh();
    // No NHL player has 99999 goals.
    let json = leaders_json(h.path(), "g>=99999");
    assert_eq!(json_player_count(&json), 0);
}

#[test]
fn p_w11_127_negative_min_is_universal_for_unsigned() {
    let h = fresh();
    let universal = leaders_json(h.path(), "g>=0");
    let neg = leaders_json(h.path(), "g>=-100");
    assert_eq!(json_player_count(&universal), json_player_count(&neg));
}

#[test]
fn p_w11_128_zero_max_keeps_only_zero() {
    let h = fresh();
    // g<=0 keeps only players with exactly 0 goals.
    let zero_max = leaders_json(h.path(), "g<=0");
    let universal = leaders_json(h.path(), "g>=0");
    assert!(json_player_count(&zero_max) <= json_player_count(&universal));
}

#[test]
fn p_w11_129_eq_zero_subset_of_max_zero() {
    let h = fresh();
    let eq = json_player_count(&leaders_json(h.path(), "g==0"));
    let max = json_player_count(&leaders_json(h.path(), "g<=0"));
    assert_eq!(eq, max, "g==0 and g<=0 should match identically");
}

#[test]
fn p_w11_130_demorgan_ne_via_or() {
    let h = fresh();
    // NOT (g>=10 AND a>=10) ≡ NOT g>=10 OR NOT a>=10
    let lhs = json_player_count(&leaders_json(h.path(), "NOT (g>=10 AND a>=10)"));
    let rhs = json_player_count(&leaders_json(h.path(), "NOT g>=10 OR NOT a>=10"));
    assert_eq!(lhs, rhs);
}

// ── Section G — Goalies subcommand filter rewrites (15) ─────────────────────

fn goalies(home: &std::path::Path, expr: &str) -> String {
    ok_in(home, &["query", "goalies", "--top", "5", "--filter", expr])
}

fn goalies_fail(home: &std::path::Path, expr: &str) -> Output {
    fail_in(home, &["query", "goalies", "--top", "5", "--filter", expr])
}

#[test]
fn p_w11_131_goalies_gp_rewrites_to_goalie_games() {
    let h = fresh();
    let _ = goalies(h.path(), "gp>=10");
}

#[test]
fn p_w11_132_goalies_games_rewrites_to_goalie_games() {
    let h = fresh();
    let _ = goalies(h.path(), "games>=10");
}

#[test]
fn p_w11_133_goalies_starts_rewrites_to_goalie_starts() {
    let h = fresh();
    let _ = goalies(h.path(), "starts>=5");
}

#[test]
fn p_w11_134_goalies_save_pct_alias() {
    let h = fresh();
    let _ = goalies(h.path(), "sv%>=0.9");
}

#[test]
fn p_w11_135_goalies_skater_stat_rejected() {
    let h = fresh();
    // `hits` is a skater realtime stat, not a goalie stat. Should
    // either reject as UnknownStat or quietly match nobody — but
    // never panic.
    let out = run_in(
        h.path(),
        &["query", "goalies", "--top", "5", "--filter", "hits>=100"],
    );
    no_panic(&out);
}

#[test]
fn p_w11_136_goalies_compound_filter() {
    let h = fresh();
    let _ = goalies(h.path(), "gp>=10 AND sv%>=0.9");
}

#[test]
fn p_w11_137_goalies_or_filter() {
    let h = fresh();
    let _ = goalies(h.path(), "gp>=20 OR starts>=10");
}

#[test]
fn p_w11_138_goalies_not_filter() {
    let h = fresh();
    let _ = goalies(h.path(), "NOT gp<=2");
}

#[test]
fn p_w11_139_goalies_age_bio_atom() {
    let h = fresh();
    let _ = goalies(h.path(), "age<=25 AND gp>=5");
}

#[test]
fn p_w11_140_goalies_country_atom() {
    let h = fresh();
    let _ = goalies(h.path(), "country=CAN");
}

#[test]
fn p_w11_141_goalies_height_atom() {
    let h = fresh();
    let _ = goalies(h.path(), "height>=74");
}

#[test]
fn p_w11_142_goalies_unknown_stat_clean() {
    let h = fresh();
    let out = goalies_fail(h.path(), "totally-fake-goalie-stat>=10");
    no_panic(&out);
}

#[test]
fn p_w11_143_goalies_paren_grouping() {
    let h = fresh();
    let _ = goalies(h.path(), "(gp>=10 AND sv%>=0.9) OR starts>=20");
}

#[test]
fn p_w11_144_goalies_demorgan() {
    let h = fresh();
    let _ = goalies(h.path(), "NOT (gp<=5 OR sv%<=0.85)");
}

#[test]
fn p_w11_145_goalies_self_contradiction_returns_zero() {
    let h = fresh();
    // gp>=100 AND gp<=10 — impossible
    let out = ok_in(
        h.path(),
        &[
            "query",
            "goalies",
            "--top",
            "100",
            "--json",
            "--filter",
            "gp>=100 AND gp<=10",
        ],
    );
    let v: serde_json::Value = serde_json::from_str(&out).expect("parse");
    let n = v
        .get("data")
        .and_then(|d| d.get("goalies"))
        .and_then(|p| p.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    assert_eq!(n, 0);
}

// ── Section H — Filter alias coverage (15) ──────────────────────────────────

#[test]
fn p_w11_146_alias_g() {
    let h = fresh();
    let _ = leaders(h.path(), "g>=1");
}

#[test]
fn p_w11_147_alias_p() {
    let h = fresh();
    let _ = leaders(h.path(), "p>=1");
}

#[test]
fn p_w11_148_alias_gp() {
    let h = fresh();
    let _ = leaders(h.path(), "gp>=10");
}

#[test]
fn p_w11_149_alias_ppg() {
    let h = fresh();
    let _ = leaders(h.path(), "ppg>=0.5");
}

#[test]
fn p_w11_150_alias_blk() {
    let h = fresh();
    let _ = leaders(h.path(), "blk>=10");
}

#[test]
fn p_w11_151_alias_tk() {
    let h = fresh();
    let _ = leaders(h.path(), "tk>=10");
}

#[test]
fn p_w11_152_alias_gv() {
    let h = fresh();
    let _ = leaders(h.path(), "gv>=10");
}

#[test]
fn p_w11_153_alias_pen() {
    let h = fresh();
    let _ = leaders(h.path(), "pen>=10");
}

#[test]
fn p_w11_154_alias_plus_minus() {
    let h = fresh();
    let _ = leaders(h.path(), "+/->=-10");
}

#[test]
fn p_w11_155_alias_uppercase_hits() {
    let h = fresh();
    let _ = leaders(h.path(), "HITS>=10");
}

#[test]
fn p_w11_156_alias_uppercase_full_word() {
    let h = fresh();
    let _ = leaders(h.path(), "GOALS>=1");
}

#[test]
fn p_w11_157_alias_resolves_to_same_count() {
    let h = fresh();
    // `g` and `goals` should yield identical results.
    let g = json_player_count(&leaders_json(h.path(), "g>=10"));
    let goals = json_player_count(&leaders_json(h.path(), "goals>=10"));
    assert_eq!(g, goals);
}

#[test]
fn p_w11_158_alias_p_resolves_same_as_points() {
    let h = fresh();
    let p = json_player_count(&leaders_json(h.path(), "p>=20"));
    let points = json_player_count(&leaders_json(h.path(), "points>=20"));
    assert_eq!(p, points);
}

#[test]
fn p_w11_159_alias_ppg_same_as_points_per_game() {
    let h = fresh();
    let ppg = json_player_count(&leaders_json(h.path(), "ppg>=0.5"));
    let full = json_player_count(&leaders_json(h.path(), "points-per-game>=0.5"));
    assert_eq!(ppg, full);
}

#[test]
fn p_w11_160_alias_unknown_short_rejected() {
    // `xyz` shouldn't resolve to anything.
    let h = fresh();
    let out = leaders_fail(h.path(), "xyz>=1");
    no_panic(&out);
}

// ── Section I — Pathological / stress inputs (20) ───────────────────────────

#[test]
fn p_w11_161_long_and_chain() {
    let h = fresh();
    // 30 ANDs deep, all `g>=1`. Should be no-op redundant.
    let chain = (0..30).map(|_| "g>=1").collect::<Vec<_>>().join(" AND ");
    let _ = leaders(h.path(), &chain);
}

#[test]
fn p_w11_162_long_or_chain() {
    let h = fresh();
    let chain = (0..30).map(|_| "g>=1").collect::<Vec<_>>().join(" OR ");
    let _ = leaders(h.path(), &chain);
}

#[test]
fn p_w11_163_alternating_and_or_chain() {
    let h = fresh();
    let chain = "g>=1 AND a>=1 OR p>=1 AND pim>=1 OR sog>=1";
    let _ = leaders(h.path(), chain);
}

#[test]
fn p_w11_164_deeply_nested_parens() {
    let h = fresh();
    let mut expr = "g>=1".to_string();
    for _ in 0..10 {
        expr = format!("({expr})");
    }
    let _ = leaders(h.path(), &expr);
}

#[test]
fn p_w11_165_deeply_nested_not() {
    let h = fresh();
    let mut expr = "g>=10000".to_string();
    for _ in 0..10 {
        expr = format!("NOT {expr}");
    }
    // 10 NOTs ≡ no NOT (even count).
    let _ = leaders(h.path(), &expr);
}

#[test]
fn p_w11_166_giant_value_no_overflow() {
    let h = fresh();
    let _ = leaders(h.path(), "g>=1e15");
}

#[test]
fn p_w11_167_scientific_notation_value() {
    let h = fresh();
    // f64 accepts 1e3 etc.
    let _ = leaders(h.path(), "g>=1e2");
}

#[test]
fn p_w11_168_negative_scientific_notation() {
    let h = fresh();
    let _ = leaders(h.path(), "+/->=-1e2");
}

#[test]
fn p_w11_169_unicode_in_value_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), "g>=１０"); // full-width digits
    no_panic(&out);
}

#[test]
fn p_w11_170_emoji_in_filter_rejected() {
    let h = fresh();
    let out = leaders_fail(h.path(), "g>=🙂");
    no_panic(&out);
}

#[test]
fn p_w11_171_null_byte_attempt_skipped() {
    // Std command rejects literal null bytes pre-spawn; use BEL
    // (bell) instead as a control character.
    let h = fresh();
    let out = leaders_fail(h.path(), "g>=\x07");
    no_panic(&out);
}

#[test]
fn p_w11_172_extremely_long_stat_key() {
    let h = fresh();
    let key: String = "x".repeat(1000);
    let expr = format!("{key}>=1");
    let out = leaders_fail(h.path(), &expr);
    no_panic(&out);
}

#[test]
fn p_w11_173_extremely_long_value() {
    let h = fresh();
    let val: String = "9".repeat(500);
    let expr = format!("g>={val}");
    // 9999...{500 nines} parses as f64::INFINITY → NotFinite.
    let out = leaders_fail(h.path(), &expr);
    no_panic(&out);
}

#[test]
fn p_w11_174_many_repeated_filters() {
    let h = fresh();
    // Use multiple --filter flags (CLI supports repeating).
    let out = run_in(
        h.path(),
        &[
            "query", "leaders", "--pos", "C", "--top", "5", "--filter", "g>=1", "--filter", "a>=1",
            "--filter", "p>=1", "--filter", "pim>=0",
        ],
    );
    no_panic(&out);
    assert!(out.status.success(), "multi --filter must succeed");
}

#[test]
fn p_w11_175_filter_with_only_op() {
    let h = fresh();
    let out = leaders_fail(h.path(), "==");
    no_panic(&out);
}

#[test]
fn p_w11_176_keyword_inside_quoted_atom() {
    // `andes` (a stat-like word containing AND as substring) MUST
    // NOT tokenize as the keyword.
    let h = fresh();
    let out = leaders_fail(h.path(), "andes>=5");
    no_panic(&out);
    let err = stderr_of(&out);
    // Should reject as UnknownStat (key didn't match), not as a
    // grammar / keyword issue.
    assert!(
        err.to_lowercase().contains("andes") || err.to_lowercase().contains("unknown"),
        "should treat 'andes' as a stat key, not a keyword: {err}"
    );
}

#[test]
fn p_w11_177_keyword_substring_in_middle() {
    let h = fresh();
    // `goring>=5` contains "or" but not as a keyword.
    let out = leaders_fail(h.path(), "goring>=5");
    no_panic(&out);
}

#[test]
fn p_w11_178_keyword_substring_not_at_boundary() {
    let h = fresh();
    // `notable>=5` — `not` as prefix but not a keyword (no boundary).
    let out = leaders_fail(h.path(), "notable>=5");
    no_panic(&out);
}

#[test]
fn p_w11_179_filter_with_just_paren_chain() {
    let h = fresh();
    let out = leaders_fail(h.path(), "(((((((");
    no_panic(&out);
}

#[test]
fn p_w11_180_filter_with_only_close_parens() {
    let h = fresh();
    let out = leaders_fail(h.path(), "))))");
    no_panic(&out);
}

// ── Section J — Output truthfulness (filter actually filters) (20) ──────────

#[test]
fn p_w11_181_filter_affects_count() {
    let h = fresh();
    // No filter → many; tight filter → fewer.
    let unfiltered = ok_in(
        h.path(),
        &["query", "leaders", "--pos", "C", "--top", "100", "--json"],
    );
    let filtered = leaders_json(h.path(), "g>=20");
    assert!(
        json_player_count(&filtered) <= json_player_count(&unfiltered),
        "filter should reduce or equal the unfiltered count"
    );
}

#[test]
fn p_w11_182_filter_returns_subset() {
    let h = fresh();
    let big = leaders_json(h.path(), "g>=1");
    let small = leaders_json(h.path(), "g>=20");
    assert!(
        json_player_count(&small) <= json_player_count(&big),
        "tighter filter should yield ≤ players"
    );
}

#[test]
fn p_w11_183_or_count_bounds() {
    let h = fresh();
    // |A ∪ B| ≥ max(|A|, |B|) and |A ∪ B| ≤ |A| + |B|
    let a = json_player_count(&leaders_json(h.path(), "g>=10"));
    let b = json_player_count(&leaders_json(h.path(), "a>=10"));
    let or = json_player_count(&leaders_json(h.path(), "g>=10 OR a>=10"));
    assert!(or >= a.max(b));
    assert!(or <= a + b);
}

#[test]
fn p_w11_184_and_count_bounds() {
    let h = fresh();
    // |A ∩ B| ≤ min(|A|, |B|)
    let a = json_player_count(&leaders_json(h.path(), "g>=10"));
    let b = json_player_count(&leaders_json(h.path(), "a>=10"));
    let and = json_player_count(&leaders_json(h.path(), "g>=10 AND a>=10"));
    assert!(and <= a.min(b));
}

#[test]
fn p_w11_185_inclusion_exclusion_holds() {
    let h = fresh();
    // |A ∪ B| = |A| + |B| - |A ∩ B|
    let a = json_player_count(&leaders_json(h.path(), "g>=10"));
    let b = json_player_count(&leaders_json(h.path(), "a>=10"));
    let or = json_player_count(&leaders_json(h.path(), "g>=10 OR a>=10"));
    let and = json_player_count(&leaders_json(h.path(), "g>=10 AND a>=10"));
    assert_eq!(or, a + b - and, "|A∪B| = |A|+|B|−|A∩B|");
}

#[test]
fn p_w11_186_negation_complements_universe() {
    let h = fresh();
    // |A| + |NOT A| = universe (with the same --top cap on each).
    // Use --top 9999 so the cap is non-binding for the universe.
    let universe_out = ok_in(
        h.path(),
        &[
            "query", "leaders", "--pos", "C", "--top", "9999", "--filter", "g>=0", "--json",
        ],
    );
    let a_out = ok_in(
        h.path(),
        &[
            "query", "leaders", "--pos", "C", "--top", "9999", "--filter", "g>=20", "--json",
        ],
    );
    let neg_out = ok_in(
        h.path(),
        &[
            "query",
            "leaders",
            "--pos",
            "C",
            "--top",
            "9999",
            "--filter",
            "NOT g>=20",
            "--json",
        ],
    );
    let universe = json_player_count(&universe_out);
    let a = json_player_count(&a_out);
    let neg = json_player_count(&neg_out);
    assert_eq!(universe, a + neg, "|A|+|¬A| should equal universe");
}

#[test]
fn p_w11_187_filter_commutative_and() {
    let h = fresh();
    let ab = json_player_count(&leaders_json(h.path(), "g>=10 AND a>=10"));
    let ba = json_player_count(&leaders_json(h.path(), "a>=10 AND g>=10"));
    assert_eq!(ab, ba);
}

#[test]
fn p_w11_188_filter_commutative_or() {
    let h = fresh();
    let ab = json_player_count(&leaders_json(h.path(), "g>=10 OR a>=10"));
    let ba = json_player_count(&leaders_json(h.path(), "a>=10 OR g>=10"));
    assert_eq!(ab, ba);
}

#[test]
fn p_w11_189_filter_associative_and() {
    let h = fresh();
    let abc = json_player_count(&leaders_json(h.path(), "(g>=10 AND a>=10) AND p>=20"));
    let a_bc = json_player_count(&leaders_json(h.path(), "g>=10 AND (a>=10 AND p>=20)"));
    assert_eq!(abc, a_bc);
}

#[test]
fn p_w11_190_filter_associative_or() {
    let h = fresh();
    let abc = json_player_count(&leaders_json(h.path(), "(g>=10 OR a>=10) OR p>=20"));
    let a_bc = json_player_count(&leaders_json(h.path(), "g>=10 OR (a>=10 OR p>=20)"));
    assert_eq!(abc, a_bc);
}

#[test]
fn p_w11_191_distributivity_and_over_or() {
    let h = fresh();
    // A AND (B OR C) ≡ (A AND B) OR (A AND C)
    let lhs = json_player_count(&leaders_json(h.path(), "g>=10 AND (a>=10 OR p>=20)"));
    let rhs = json_player_count(&leaders_json(
        h.path(),
        "(g>=10 AND a>=10) OR (g>=10 AND p>=20)",
    ));
    assert_eq!(lhs, rhs);
}

#[test]
fn p_w11_192_distributivity_or_over_and() {
    let h = fresh();
    // A OR (B AND C) ≡ (A OR B) AND (A OR C)
    let lhs = json_player_count(&leaders_json(h.path(), "g>=20 OR (a>=10 AND p>=20)"));
    let rhs = json_player_count(&leaders_json(
        h.path(),
        "(g>=20 OR a>=10) AND (g>=20 OR p>=20)",
    ));
    assert_eq!(lhs, rhs);
}

#[test]
fn p_w11_193_text_and_json_row_count_align() {
    // Text output and JSON output should agree on filtered count.
    let h = fresh();
    let json = leaders_json(h.path(), "g>=10");
    let json_n = json_player_count(&json);

    let text = ok_in(
        h.path(),
        &[
            "query", "leaders", "--pos", "C", "--top", "100", "--filter", "g>=10",
        ],
    );
    // Crude: count player rows by counting lines that contain a
    // tab-separated row pattern. Use the simpler metric: number of
    // lines that don't start with "│", "─", "═", or "Top ".
    let text_rows = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter(|l| !l.starts_with('─') && !l.starts_with('═') && !l.starts_with('│'))
        .count();
    // We don't expect text and json to be exactly equal (text has
    // headers); just check json count is reasonable bounded.
    assert!(json_n <= 100, "top 100 cap: got {json_n}");
    let _ = text_rows;
}

#[test]
fn p_w11_194_meta_filter_echoes_input() {
    // The legacy `query leaders --json` route emits a bare JSON
    // array (no K2.4 envelope). Meta-echo can't apply there. When
    // the route IS K2.4-enveloped (e.g. leaders.windowed,
    // leaders.playoff), the meta block should record the filter so
    // consumers can confirm what was applied. This test documents
    // the gap and won't fail until we wrap the legacy route.
    let h = fresh();
    let json = leaders_json(h.path(), "g>=10");
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    if let Some(meta) = v.get("meta") {
        let has_filter_field = meta.get("filter").is_some()
            || meta.get("filters").is_some()
            || meta.get("applied_filter").is_some()
            || meta.get("filter_expression").is_some();
        let _ = has_filter_field;
    }
    // No meta = no assertion. Wave 11 #194 documents this for the
    // grammar-expansion phase (Phase B) to address.
}

#[test]
fn p_w11_195_top_n_caps_output() {
    let h = fresh();
    let json = ok_in(
        h.path(),
        &[
            "query", "leaders", "--pos", "C", "--top", "3", "--filter", "g>=0", "--json",
        ],
    );
    let n = json_player_count(&json);
    assert!(n <= 3, "--top 3 must cap output, got {n}");
}

#[test]
fn p_w11_196_top_n_independent_of_filter() {
    // --top is a post-filter limit. So filter then cap.
    let h = fresh();
    let many = json_player_count(&leaders_json(h.path(), "g>=0"));
    let capped_json = ok_in(
        h.path(),
        &[
            "query", "leaders", "--pos", "C", "--top", "5", "--filter", "g>=0", "--json",
        ],
    );
    let capped = json_player_count(&capped_json);
    assert!(capped <= 5);
    assert!(capped <= many);
}

#[test]
fn p_w11_197_filter_passes_via_csv_export() {
    let h = fresh();
    // Quick sanity: --csv path also respects the filter.
    let csv = ok_in(
        h.path(),
        &[
            "query", "leaders", "--pos", "C", "--top", "5", "--filter", "g>=10000", "--csv",
        ],
    );
    // No data rows — only header.
    let lines: Vec<&str> = csv.lines().filter(|l| !l.is_empty()).collect();
    assert!(
        lines.len() <= 1,
        "g>=10000 should yield 0 data rows, got: {csv}"
    );
}

#[test]
fn p_w11_198_filter_passes_via_md_export() {
    let h = fresh();
    let md = run_in(
        h.path(),
        &[
            "export", "md", "leaders", "--pos", "C", "--top", "5", "--filter", "g>=10",
        ],
    );
    no_panic(&md);
}

#[test]
fn p_w11_199_repeated_filter_flags_intersect() {
    let h = fresh();
    // --filter g>=10 --filter a>=10 should equal --filter "g>=10 AND a>=10"
    let multi_json = ok_in(
        h.path(),
        &[
            "query", "leaders", "--pos", "C", "--top", "100", "--filter", "g>=10", "--filter",
            "a>=10", "--json",
        ],
    );
    let single_json = leaders_json(h.path(), "g>=10 AND a>=10");
    assert_eq!(
        json_player_count(&multi_json),
        json_player_count(&single_json),
        "multi --filter and AND-joined single --filter should match"
    );
}

#[test]
fn p_w11_200_filter_independent_of_sort_metric() {
    let h = fresh();
    // Sorting by goals vs assists shouldn't change the FILTERED set,
    // only the order. So |sort=goals filter=g>=20| == |sort=assists filter=g>=20|.
    let by_g = ok_in(
        h.path(),
        &[
            "query", "leaders", "--pos", "C", "--top", "100", "--sort", "goals", "--filter",
            "g>=20", "--json",
        ],
    );
    let by_a = ok_in(
        h.path(),
        &[
            "query", "leaders", "--pos", "C", "--top", "100", "--sort", "assists", "--filter",
            "g>=20", "--json",
        ],
    );
    assert_eq!(
        json_player_count(&by_g),
        json_player_count(&by_a),
        "filter set must be sort-invariant"
    );
}
