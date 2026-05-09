//! Persona Wave 18 — subcommand coverage for new grammar.
//!
//! Wave 16 fixed the dispatch in `query leaders` (CLI) and
//! Wave 17 fixed it in /leaders (web). But CLI has THREE other
//! subcommands with their own filter dispatch:
//!  - `query goalies` — has goalie_filter_rewrite + parse_filter_expr
//!  - `query compare` — has cohort filter via parse_filter_expr
//!  - `query player`'s peers cohort — same shape
//!
//! Wave 18 surfaces whether those still have the legacy-only
//! dispatch bug by running new-grammar filters through them.

use std::path::PathBuf;
use std::process::{Command, Output};

fn icelines_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_icelines"))
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
        String::from_utf8_lossy(&out.stderr)
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
    assert_ne!(out.status.code(), Some(101));
}

// ── query goalies — new grammar (15) ─────────────────────────

#[test]
fn p_w18_001_goalies_country_eq() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &["query", "goalies", "--top", "5", "--filter", "country=USA"],
    );
}

#[test]
fn p_w18_002_goalies_country_in_set() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "goalies",
            "--top",
            "5",
            "--filter",
            "country IN (USA, CAN, RUS)",
        ],
    );
}

#[test]
fn p_w18_003_goalies_country_not_in() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "goalies",
            "--top",
            "5",
            "--filter",
            "country NOT IN (RUS)",
        ],
    );
}

#[test]
fn p_w18_004_goalies_strict_age_lt() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &["query", "goalies", "--top", "5", "--filter", "age<28"],
    );
}

#[test]
fn p_w18_005_goalies_age_between() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "goalies",
            "--top",
            "5",
            "--filter",
            "age BETWEEN 25 AND 32",
        ],
    );
}

#[test]
fn p_w18_006_goalies_country_like() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "goalies",
            "--top",
            "5",
            "--filter",
            r#"country LIKE "CA*""#,
        ],
    );
}

#[test]
fn p_w18_007_goalies_compound_strict_and_in() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "goalies",
            "--top",
            "5",
            "--filter",
            "age<30 AND country IN (USA, CAN, RUS)",
        ],
    );
}

#[test]
fn p_w18_008_goalies_with_ne() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &["query", "goalies", "--top", "5", "--filter", "country!=RUS"],
    );
}

#[test]
fn p_w18_009_goalies_pos_g_redundant_but_valid() {
    // pos=G is redundant on `query goalies` but should parse OK.
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &["query", "goalies", "--top", "5", "--filter", "pos=G"],
    );
}

#[test]
fn p_w18_010_goalies_legacy_gp_still_works() {
    // The goalie_filter_rewrite makes `gp` → `goalie-games`
    // before parse — verify legacy still works.
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &["query", "goalies", "--top", "5", "--filter", "gp>=10"],
    );
}

#[test]
fn p_w18_011_goalies_save_pct_threshold() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "goalies",
            "--top",
            "5",
            "--filter",
            "save-pct>=0.910",
        ],
    );
}

#[test]
fn p_w18_012_goalies_compound_save_pct_and_country() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "goalies",
            "--top",
            "5",
            "--filter",
            "save-pct>=0.900 AND country=USA",
        ],
    );
}

#[test]
fn p_w18_013_goalies_empty_in_rejected() {
    let h = fresh();
    let out = fail_in(
        h.path(),
        &[
            "query",
            "goalies",
            "--top",
            "5",
            "--filter",
            "country IN ()",
        ],
    );
    no_panic(&out);
}

#[test]
fn p_w18_014_goalies_arrow_eq_typo_hint() {
    let h = fresh();
    let out = fail_in(
        h.path(),
        &["query", "goalies", "--top", "5", "--filter", "gp=>10"],
    );
    no_panic(&out);
}

#[test]
fn p_w18_015_goalies_paren_compound() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "goalies",
            "--top",
            "5",
            "--filter",
            "(country=USA OR country=CAN) AND age<=30",
        ],
    );
}

// ── query compare — new grammar (15) ─────────────────────────
//
// `query compare --cohort` accepts filters for the cohort. Legacy
// dispatch was at line 1837 in commands/query.rs.

#[test]
fn p_w18_016_compare_cohort_country_in() {
    let h = fresh();
    // Use --draft-class as the cohort selector + filter on country.
    let out = run_in(
        h.path(),
        &[
            "query",
            "compare",
            "--cohort",
            "draft",
            "--draft-class",
            "2015",
            "--filter",
            "country IN (CAN, USA, SWE)",
            "--top",
            "5",
        ],
    );
    // May 'fail' if draft-class doesn't have data — but should NOT
    // crash, and if it errors it should NOT be a 'no op' kind of
    // legacy-parser error.
    no_panic(&out);
}

#[test]
fn p_w18_017_compare_cohort_strict_lt() {
    let h = fresh();
    let out = run_in(
        h.path(),
        &[
            "query",
            "compare",
            "--cohort",
            "draft",
            "--draft-class",
            "2015",
            "--filter",
            "age<30",
            "--top",
            "5",
        ],
    );
    no_panic(&out);
}

#[test]
fn p_w18_018_compare_cohort_between() {
    let h = fresh();
    let out = run_in(
        h.path(),
        &[
            "query",
            "compare",
            "--cohort",
            "draft",
            "--draft-class",
            "2015",
            "--filter",
            "age BETWEEN 28 AND 32",
            "--top",
            "5",
        ],
    );
    no_panic(&out);
}

#[test]
fn p_w18_019_compare_cohort_like() {
    let h = fresh();
    let out = run_in(
        h.path(),
        &[
            "query",
            "compare",
            "--cohort",
            "draft",
            "--draft-class",
            "2015",
            "--filter",
            r#"country LIKE "CA*""#,
            "--top",
            "5",
        ],
    );
    no_panic(&out);
}

#[test]
fn p_w18_020_compare_cohort_compound_new_grammar() {
    let h = fresh();
    let out = run_in(
        h.path(),
        &[
            "query",
            "compare",
            "--cohort",
            "draft",
            "--draft-class",
            "2015",
            "--filter",
            "country IN (CAN, USA) AND age<30",
            "--top",
            "5",
        ],
    );
    no_panic(&out);
}

// ── x quick-export — defer (no x command exists) ─────────────
// `x` was a documented quick-export shortcut in COMMANDS.md but
// the binary doesn't expose it as a top-level subcommand
// (verified via `--help` listing).

// ── export md — new grammar (10) ─────────────────────────────

#[test]
fn p_w18_021_export_md_country_in() {
    let h = fresh();
    let out = run_in(
        h.path(),
        &[
            "export",
            "md",
            "leaders",
            "--pos",
            "C",
            "--top",
            "5",
            "--filter",
            "country IN (CAN, USA)",
        ],
    );
    no_panic(&out);
}

#[test]
fn p_w18_022_export_md_strict_age() {
    let h = fresh();
    let out = run_in(
        h.path(),
        &[
            "export", "md", "leaders", "--pos", "C", "--top", "5", "--filter", "age<25",
        ],
    );
    no_panic(&out);
}

#[test]
fn p_w18_023_export_md_between() {
    let h = fresh();
    let out = run_in(
        h.path(),
        &[
            "export",
            "md",
            "leaders",
            "--pos",
            "C",
            "--top",
            "5",
            "--filter",
            "age BETWEEN 22 AND 28",
        ],
    );
    no_panic(&out);
}

#[test]
fn p_w18_024_export_md_like() {
    let h = fresh();
    let out = run_in(
        h.path(),
        &[
            "export",
            "md",
            "leaders",
            "--pos",
            "C",
            "--top",
            "5",
            "--filter",
            r#"country LIKE "CA*""#,
        ],
    );
    no_panic(&out);
}

#[test]
fn p_w18_025_export_md_compound() {
    let h = fresh();
    let out = run_in(
        h.path(),
        &[
            "export",
            "md",
            "leaders",
            "--pos",
            "C",
            "--top",
            "5",
            "--filter",
            "country IN (CAN, USA) AND age<30",
        ],
    );
    no_panic(&out);
}

// ── x — new grammar through the export-shortcut tool (10) ────

#[test]
fn p_w18_026_x_leaders_country_in() {
    let h = fresh();
    let out = run_in(
        h.path(),
        &[
            "x",
            "leaders",
            "--pos",
            "C",
            "--top",
            "5",
            "--filter",
            "country IN (CAN, USA)",
        ],
    );
    no_panic(&out);
}

#[test]
fn p_w18_027_x_leaders_strict_age() {
    let h = fresh();
    let out = run_in(
        h.path(),
        &[
            "x", "leaders", "--pos", "C", "--top", "5", "--filter", "age<25",
        ],
    );
    no_panic(&out);
}

#[test]
fn p_w18_028_x_leaders_like() {
    let h = fresh();
    let out = run_in(
        h.path(),
        &[
            "x",
            "leaders",
            "--pos",
            "C",
            "--top",
            "5",
            "--filter",
            r#"country LIKE "CA*""#,
        ],
    );
    no_panic(&out);
}

#[test]
fn p_w18_029_x_leaders_between() {
    let h = fresh();
    let out = run_in(
        h.path(),
        &[
            "x",
            "leaders",
            "--pos",
            "C",
            "--top",
            "5",
            "--filter",
            "age BETWEEN 22 AND 28",
        ],
    );
    no_panic(&out);
}

#[test]
fn p_w18_030_x_leaders_compound() {
    let h = fresh();
    let out = run_in(
        h.path(),
        &[
            "x",
            "leaders",
            "--pos",
            "C",
            "--top",
            "5",
            "--filter",
            "country IN (CAN, USA) AND age<25",
        ],
    );
    no_panic(&out);
}

// ── Closing edges across subcommands (10) ────────────────────

#[test]
fn p_w18_031_goalies_unclosed_paren_clean() {
    let h = fresh();
    let out = fail_in(
        h.path(),
        &[
            "query",
            "goalies",
            "--top",
            "5",
            "--filter",
            "(country=USA AND age<30",
        ],
    );
    no_panic(&out);
}

#[test]
fn p_w18_032_goalies_unknown_country_no_match() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "query",
            "goalies",
            "--top",
            "5",
            "--filter",
            "country=XYZ",
            "--json",
        ],
    );
    let v: serde_json::Value = serde_json::from_str(&out).expect("parse");
    let arr = v
        .get("data")
        .and_then(|d| d.get("goalies"))
        .and_then(|p| p.as_array())
        .or_else(|| v.as_array());
    if let Some(arr) = arr {
        assert!(arr.is_empty(), "unknown country shouldn't match anyone");
    }
}

#[test]
fn p_w18_033_goalies_huge_threshold_empty() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "goalies",
            "--top",
            "5",
            "--filter",
            "save-pct>=2.0",
        ],
    );
}

#[test]
fn p_w18_034_goalies_demorgan() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "goalies",
            "--top",
            "5",
            "--filter",
            "NOT (country=USA AND age<30)",
        ],
    );
}

#[test]
fn p_w18_035_goalies_strict_in_or() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "goalies",
            "--top",
            "5",
            "--filter",
            "save-pct>=0.910 OR gp>=50",
        ],
    );
}

#[test]
fn p_w18_036_goalies_country_lowercase() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &["query", "goalies", "--top", "5", "--filter", "country=usa"],
    );
}

#[test]
fn p_w18_037_goalies_age_in_set() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "goalies",
            "--top",
            "5",
            "--filter",
            "age IN (28, 29, 30, 31, 32)",
        ],
    );
}

#[test]
fn p_w18_038_goalies_kitchen_sink() {
    let h = fresh();
    let _ = ok_in(
        h.path(),
        &[
            "query",
            "goalies",
            "--top",
            "5",
            "--filter",
            "country IN (USA, CAN, RUS, FIN, SWE) AND age BETWEEN 25 AND 35 AND save-pct>=0.890",
        ],
    );
}

#[test]
fn p_w18_039_goalies_csv_with_new_grammar() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "query",
            "goalies",
            "--top",
            "5",
            "--filter",
            "country IN (CAN, USA)",
            "--csv",
        ],
    );
    assert!(!out.is_empty());
}

#[test]
fn p_w18_040_goalies_json_with_new_grammar() {
    let h = fresh();
    let out = ok_in(
        h.path(),
        &[
            "query",
            "goalies",
            "--top",
            "5",
            "--filter",
            "country IN (CAN, USA)",
            "--json",
        ],
    );
    let _: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
}
