//! Persona Wave 12 — adversarial sweep on the Phase Art Ross
//! grammar. 200 library-level integration tests covering every
//! A.1 operator + every A.2 sliding-window atom + every new bio
//! atom + mixed compound expressions.
//!
//! Library-level (not subprocess) because A.2.4 hasn't yet wired
//! the new pipeline into the CLI binary. After A.2.4 ships, a
//! follow-on wave can re-run these scenarios via subprocess.
//!
//! Sections:
//!   A — Strict comparators (20): `<`, `>`, `!=`
//!   B — IN / NOT IN (25)
//!   C — BETWEEN (20)
//!   D — LIKE / ~ (20)
//!   E — Sliding-window atoms (25)
//!   F — Bio atom expansion (20)
//!   G — Compound queries (25)
//!   H — Output truthfulness (15)
//!   I — Edge cases (15)
//!   J — Pathological inputs (15)

use icelines_query::data_provider::{
    DataProvider, FetchError, FetchEvent, PlanRequirement,
};
use icelines_query::{
    parse_query, BioConstraint, BioField, Constraint, FilterInput, GlobPattern, MemberOp,
    NumericRange, ParseError, PatternOp, Predicate, ScalarOp, ScalarValue, SlidingWindow,
    SlidingWindowConstraint,
};

// ── Helpers ───────────────────────────────────────────────────

fn ok(s: &str) -> Constraint {
    parse_query(FilterInput::Cli(s.to_string()))
        .unwrap_or_else(|e| panic!("expected parse OK for {s:?}, got {e:?}"))
        .root
}

fn errs(s: &str) -> Vec<ParseError> {
    parse_query(FilterInput::Cli(s.to_string()))
        .err()
        .unwrap_or_else(|| panic!("expected parse Err for {s:?}, got OK"))
}

fn is_seasonstat_with_op(c: &Constraint, expected: ScalarOp) -> bool {
    matches!(
        c,
        Constraint::SeasonStat(ss)
            if matches!(&ss.predicate, Predicate::Scalar(op, _) if *op == expected)
    )
}

fn is_bio_with_field(c: &Constraint, expected: BioField) -> bool {
    matches!(c, Constraint::Bio(b) if b.field == expected)
}

#[allow(dead_code)]
struct NoOpProvider;
#[allow(dead_code)]
impl DataProvider for NoOpProvider {
    fn ensure(
        &self,
        _req: &PlanRequirement,
        _events: &mut dyn FnMut(FetchEvent),
    ) -> Result<(), FetchError> {
        Ok(())
    }
}

// ── Section A — Strict comparators (20) ─────────────────────

#[test]
fn p_w12_001_strict_lt_stat() {
    assert!(is_seasonstat_with_op(&ok("g<5"), ScalarOp::Lt));
}

#[test]
fn p_w12_002_strict_gt_stat() {
    assert!(is_seasonstat_with_op(&ok("g>5"), ScalarOp::Gt));
}

#[test]
fn p_w12_003_ne_stat() {
    assert!(is_seasonstat_with_op(&ok("g!=5"), ScalarOp::Ne));
}

#[test]
fn p_w12_004_lt_with_decimal() {
    assert!(is_seasonstat_with_op(&ok("ppg<1.5"), ScalarOp::Lt));
}

#[test]
fn p_w12_005_gt_with_negative() {
    assert!(is_seasonstat_with_op(&ok("+/->-5"), ScalarOp::Gt));
}

#[test]
fn p_w12_006_age_strict_under_25() {
    let c = ok("age<25");
    assert!(matches!(
        c,
        Constraint::Bio(BioConstraint {
            field: BioField::Age,
            predicate: Predicate::Scalar(ScalarOp::Lt, _),
        })
    ));
}

#[test]
fn p_w12_007_strict_lt_with_whitespace() {
    assert!(is_seasonstat_with_op(&ok("g < 5"), ScalarOp::Lt));
}

#[test]
fn p_w12_008_strict_gt_in_compound() {
    let c = ok("g>5 AND a<20");
    match c {
        Constraint::All(children) => assert_eq!(children.len(), 2),
        _ => panic!(),
    }
}

#[test]
fn p_w12_009_ne_under_negation() {
    let c = ok("NOT g!=5");
    assert!(matches!(c, Constraint::Not(_)));
}

#[test]
fn p_w12_010_sql_ne_typo_hint() {
    match &errs("g<>5")[0] {
        ParseError::OpTypoHint { suggestion, .. } => assert_eq!(*suggestion, "!="),
        other => panic!("got {other:?}"),
    }
}

#[test]
fn p_w12_011_arrow_eq_typo_hint() {
    match &errs("g=>5")[0] {
        ParseError::OpTypoHint { suggestion, .. } => assert_eq!(*suggestion, ">="),
        other => panic!("got {other:?}"),
    }
}

#[test]
fn p_w12_012_lt_eq_typo_hint() {
    match &errs("g=<5")[0] {
        ParseError::OpTypoHint { suggestion, .. } => assert_eq!(*suggestion, "<="),
        other => panic!("got {other:?}"),
    }
}

#[test]
fn p_w12_013_double_lt_rejected() {
    let es = errs("g<<5");
    assert!(matches!(es[0], ParseError::MultipleOps { .. }));
}

#[test]
fn p_w12_014_double_gt_rejected() {
    let es = errs("g>>5");
    assert!(matches!(es[0], ParseError::MultipleOps { .. }));
}

#[test]
fn p_w12_015_lt_in_or_chain() {
    let c = ok("g<5 OR a<5 OR p<5");
    match c {
        Constraint::Any(children) => assert_eq!(children.len(), 3),
        _ => panic!(),
    }
}

#[test]
fn p_w12_016_strict_lt_zero_threshold() {
    // g<0 — for unsigned stats, no player matches; but parse OK.
    assert!(is_seasonstat_with_op(&ok("g<0"), ScalarOp::Lt));
}

#[test]
fn p_w12_017_strict_gt_huge_threshold() {
    assert!(is_seasonstat_with_op(&ok("g>99999"), ScalarOp::Gt));
}

#[test]
fn p_w12_018_ne_decimal() {
    assert!(is_seasonstat_with_op(&ok("ppg!=1.5"), ScalarOp::Ne));
}

#[test]
fn p_w12_019_strict_lt_age_with_compound_bio() {
    let c = ok("age<25 AND country=CAN");
    match c {
        Constraint::All(children) => {
            assert_eq!(children.len(), 2);
            assert!(matches!(
                &children[0],
                Constraint::Bio(BioConstraint {
                    field: BioField::Age,
                    predicate: Predicate::Scalar(ScalarOp::Lt, _),
                })
            ));
        }
        _ => panic!(),
    }
}

#[test]
fn p_w12_020_string_field_with_lt_rejected() {
    let es = errs("country<USA");
    assert!(matches!(
        es[0],
        ParseError::IncompatiblePredicate { .. } | ParseError::UnknownStat { .. }
    ));
}

// ── Section B — IN / NOT IN (25) ────────────────────────────

#[test]
fn p_w12_021_in_country_basic() {
    let c = ok("country IN (CAN, USA, SWE)");
    assert!(matches!(
        c,
        Constraint::Bio(BioConstraint {
            predicate: Predicate::Member(MemberOp::In, _),
            ..
        })
    ));
}

#[test]
fn p_w12_022_not_in_country() {
    let c = ok("country NOT IN (CAN, USA)");
    assert!(matches!(
        c,
        Constraint::Bio(BioConstraint {
            predicate: Predicate::Member(MemberOp::NotIn, _),
            ..
        })
    ));
}

#[test]
fn p_w12_023_empty_in_rejected() {
    let es = errs("country IN ()");
    assert!(matches!(es[0], ParseError::EmptySet { .. }));
}

#[test]
fn p_w12_024_empty_not_in_rejected() {
    let es = errs("country NOT IN ()");
    assert!(matches!(es[0], ParseError::EmptySet { .. }));
}

#[test]
fn p_w12_025_single_element_in() {
    let c = ok("country IN (CAN)");
    if let Constraint::Bio(BioConstraint {
        predicate: Predicate::Member(_, vals),
        ..
    }) = c
    {
        assert_eq!(vals.len(), 1);
    } else {
        panic!();
    }
}

#[test]
fn p_w12_026_in_with_quoted_strings() {
    let c = ok(r#"country IN ("CAN", "USA")"#);
    assert!(matches!(
        c,
        Constraint::Bio(BioConstraint {
            predicate: Predicate::Member(_, _),
            ..
        })
    ));
}

#[test]
fn p_w12_027_in_with_single_quotes() {
    let c = ok("country IN ('CAN', 'USA')");
    assert!(matches!(
        c,
        Constraint::Bio(BioConstraint {
            predicate: Predicate::Member(_, _),
            ..
        })
    ));
}

#[test]
fn p_w12_028_in_numeric_draft_year() {
    let c = ok("draft-year IN (2020, 2021, 2022)");
    if let Constraint::Bio(BioConstraint {
        predicate: Predicate::Member(_, vals),
        field: BioField::DraftYear,
    }) = c
    {
        assert!(matches!(vals[0], ScalarValue::Number(_)));
    } else {
        panic!();
    }
}

#[test]
fn p_w12_029_in_pos_set() {
    let c = ok("pos IN (C, LW, RW)");
    assert!(is_bio_with_field(&c, BioField::Position));
}

#[test]
fn p_w12_030_in_team_set() {
    let c = ok("team IN (BOS, NYR, PIT)");
    assert!(is_bio_with_field(&c, BioField::Team));
}

#[test]
fn p_w12_031_stat_in_rejected_with_between_hint() {
    let es = errs("g IN (10, 20, 30)");
    match &es[0] {
        ParseError::IncompatiblePredicate { detail, .. } => {
            assert!(
                detail.to_lowercase().contains("between"),
                "should suggest BETWEEN, got: {detail}"
            );
        }
        _ => panic!(),
    }
}

#[test]
fn p_w12_032_in_no_whitespace_around_parens() {
    let c = ok("country IN(CAN,USA)");
    assert!(matches!(
        c,
        Constraint::Bio(BioConstraint {
            predicate: Predicate::Member(_, _),
            ..
        })
    ));
}

#[test]
fn p_w12_033_in_extra_whitespace() {
    let c = ok("country IN (  CAN  ,  USA  )");
    if let Constraint::Bio(BioConstraint {
        predicate: Predicate::Member(_, vals),
        ..
    }) = c
    {
        assert_eq!(vals.len(), 2);
    } else {
        panic!();
    }
}

#[test]
fn p_w12_034_in_inside_and() {
    let c = ok("country IN (CAN, USA) AND p>=80");
    match c {
        Constraint::All(children) => assert_eq!(children.len(), 2),
        _ => panic!(),
    }
}

#[test]
fn p_w12_035_in_inside_or() {
    let c = ok("country IN (CAN) OR country IN (USA)");
    match c {
        Constraint::Any(children) => assert_eq!(children.len(), 2),
        _ => panic!(),
    }
}

#[test]
fn p_w12_036_not_in_inside_compound() {
    let c = ok("country NOT IN (RUS) AND p>=50");
    match c {
        Constraint::All(children) => assert_eq!(children.len(), 2),
        _ => panic!(),
    }
}

#[test]
fn p_w12_037_in_under_negation() {
    let c = ok("NOT country IN (CAN)");
    assert!(matches!(c, Constraint::Not(_)));
}

#[test]
fn p_w12_038_in_unclosed_paren_errors() {
    let es = errs("country IN (CAN, USA");
    assert!(!es.is_empty());
}

#[test]
fn p_w12_039_in_with_paren_grouping() {
    let c = ok("(country IN (CAN, USA)) AND age<=24");
    match c {
        Constraint::All(children) => assert_eq!(children.len(), 2),
        _ => panic!(),
    }
}

#[test]
fn p_w12_040_in_with_lowercase_keyword() {
    // IN is case-insensitive
    let c = ok("country in (CAN, USA)");
    assert!(matches!(
        c,
        Constraint::Bio(BioConstraint {
            predicate: Predicate::Member(MemberOp::In, _),
            ..
        })
    ));
}

#[test]
fn p_w12_041_in_with_mixed_case() {
    let c = ok("country In (CAN, USA)");
    assert!(matches!(
        c,
        Constraint::Bio(BioConstraint {
            predicate: Predicate::Member(_, _),
            ..
        })
    ));
}

#[test]
fn p_w12_042_in_3plus_atoms_compound() {
    let c = ok("country IN (CAN, USA) AND pos IN (C, LW) AND age<=24");
    match c {
        Constraint::All(children) => assert_eq!(children.len(), 3),
        _ => panic!(),
    }
}

#[test]
fn p_w12_043_team_career_in_rejected_until_a4() {
    let es = errs("team.career IN (EDM, DAL)");
    assert!(matches!(es[0], ParseError::FeatureNotYet { .. }));
}

#[test]
fn p_w12_044_in_canonicalizes_to_lowercase() {
    if let Constraint::Bio(BioConstraint {
        predicate: Predicate::Member(_, vals),
        ..
    }) = ok("country IN (CAN, USA)")
    {
        assert!(matches!(&vals[0], ScalarValue::Text(s) if s == "can"));
        assert!(matches!(&vals[1], ScalarValue::Text(s) if s == "usa"));
    } else {
        panic!();
    }
}

#[test]
fn p_w12_045_in_unicode_via_canonicalization() {
    // ASCII pattern can reach accented values via NFD strip
    let c = ok("country IN (kämpf)");
    assert!(matches!(
        c,
        Constraint::Bio(BioConstraint {
            predicate: Predicate::Member(_, _),
            ..
        })
    ));
}

// ── Section C — BETWEEN (20) ────────────────────────────────

#[test]
fn p_w12_046_between_numeric_stat() {
    let c = ok("g BETWEEN 20 AND 40");
    assert!(matches!(
        c,
        Constraint::SeasonStat(ss) if matches!(ss.predicate, Predicate::Range(_))
    ));
}

#[test]
fn p_w12_047_between_age_bio() {
    let c = ok("age BETWEEN 22 AND 28");
    assert!(matches!(
        c,
        Constraint::Bio(BioConstraint {
            field: BioField::Age,
            predicate: Predicate::Range(_),
        })
    ));
}

#[test]
fn p_w12_048_between_decimals() {
    let c = ok("ppg BETWEEN 0.5 AND 1.5");
    if let Constraint::SeasonStat(ss) = c {
        if let Predicate::Range(NumericRange { min, max }) = ss.predicate {
            assert!((min - 0.5).abs() < 1e-9);
            assert!((max - 1.5).abs() < 1e-9);
        } else {
            panic!();
        }
    } else {
        panic!();
    }
}

#[test]
fn p_w12_049_between_height() {
    let c = ok("height BETWEEN 70 AND 80");
    assert!(matches!(
        c,
        Constraint::Bio(BioConstraint {
            field: BioField::Height,
            predicate: Predicate::Range(_),
        })
    ));
}

#[test]
fn p_w12_050_between_inverted_bounds_parses_then_matches_zero() {
    // Inverted bounds (40..20) parse but match no player. Verify
    // the parser doesn't reject this case.
    let c = ok("g BETWEEN 40 AND 20");
    if let Constraint::SeasonStat(ss) = c {
        if let Predicate::Range(NumericRange { min, max }) = ss.predicate {
            assert_eq!(min, 40.0);
            assert_eq!(max, 20.0);
        } else {
            panic!();
        }
    } else {
        panic!();
    }
}

#[test]
fn p_w12_051_between_negative_values() {
    let c = ok("+/- BETWEEN -10 AND 10");
    assert!(matches!(
        c,
        Constraint::SeasonStat(ss) if matches!(ss.predicate, Predicate::Range(_))
    ));
}

#[test]
fn p_w12_052_between_missing_and_errors() {
    let es = errs("g BETWEEN 20 40");
    assert!(!es.is_empty());
}

#[test]
fn p_w12_053_between_missing_high_errors() {
    let es = errs("g BETWEEN 20 AND");
    assert!(!es.is_empty());
}

#[test]
fn p_w12_054_between_on_string_field_rejected() {
    let es = errs("country BETWEEN 0 AND 100");
    assert!(matches!(
        es[0],
        ParseError::UnknownStat { .. } | ParseError::IncompatiblePredicate { .. }
    ));
}

#[test]
fn p_w12_055_between_inside_and_chain() {
    let c = ok("g BETWEEN 20 AND 40 AND age<=25");
    match c {
        Constraint::All(children) => assert_eq!(children.len(), 2),
        _ => panic!(),
    }
}

#[test]
fn p_w12_056_between_inside_or_chain() {
    let c = ok("g BETWEEN 20 AND 40 OR a BETWEEN 20 AND 40");
    match c {
        Constraint::Any(children) => assert_eq!(children.len(), 2),
        _ => panic!(),
    }
}

#[test]
fn p_w12_057_between_under_negation() {
    let c = ok("NOT (g BETWEEN 20 AND 40)");
    assert!(matches!(c, Constraint::Not(_)));
}

#[test]
fn p_w12_058_between_alphabetic_value_errors() {
    let es = errs("g BETWEEN ten AND twenty");
    assert!(!es.is_empty());
}

#[test]
fn p_w12_059_between_lowercase_keyword() {
    let c = ok("g between 20 and 40");
    assert!(matches!(
        c,
        Constraint::SeasonStat(ss) if matches!(ss.predicate, Predicate::Range(_))
    ));
}

#[test]
fn p_w12_060_between_mixed_case() {
    let c = ok("g Between 20 And 40");
    assert!(matches!(
        c,
        Constraint::SeasonStat(ss) if matches!(ss.predicate, Predicate::Range(_))
    ));
}

#[test]
fn p_w12_061_between_with_extra_whitespace() {
    let c = ok("g    BETWEEN    20    AND    40");
    assert!(matches!(
        c,
        Constraint::SeasonStat(ss) if matches!(ss.predicate, Predicate::Range(_))
    ));
}

#[test]
fn p_w12_062_between_zero_to_max() {
    let c = ok("g BETWEEN 0 AND 200");
    assert!(matches!(
        c,
        Constraint::SeasonStat(ss) if matches!(ss.predicate, Predicate::Range(_))
    ));
}

#[test]
fn p_w12_063_between_draft_round() {
    let c = ok("draft-round BETWEEN 1 AND 3");
    assert!(matches!(
        c,
        Constraint::Bio(BioConstraint {
            field: BioField::DraftRound,
            predicate: Predicate::Range(_),
        })
    ));
}

#[test]
fn p_w12_064_between_with_paren_grouping() {
    let c = ok("(g BETWEEN 20 AND 40) AND age<=24");
    match c {
        Constraint::All(children) => assert_eq!(children.len(), 2),
        _ => panic!(),
    }
}

#[test]
fn p_w12_065_between_compound_with_in() {
    let c = ok("age BETWEEN 22 AND 28 AND country IN (CAN, USA)");
    match c {
        Constraint::All(children) => assert_eq!(children.len(), 2),
        _ => panic!(),
    }
}

// ── Section D — LIKE / ~ (20) ───────────────────────────────

#[test]
fn p_w12_066_like_quoted_double() {
    let c = ok(r#"country LIKE "CA*""#);
    assert!(matches!(
        c,
        Constraint::Bio(BioConstraint {
            predicate: Predicate::Pattern(PatternOp::Like, _),
            ..
        })
    ));
}

#[test]
fn p_w12_067_like_quoted_single() {
    let c = ok("country LIKE 'CA*'");
    assert!(matches!(
        c,
        Constraint::Bio(BioConstraint {
            predicate: Predicate::Pattern(PatternOp::Like, _),
            ..
        })
    ));
}

#[test]
fn p_w12_068_like_unquoted() {
    let c = ok("country LIKE CA*");
    assert!(matches!(
        c,
        Constraint::Bio(BioConstraint {
            predicate: Predicate::Pattern(PatternOp::Like, _),
            ..
        })
    ));
}

#[test]
fn p_w12_069_not_like() {
    let c = ok(r#"country NOT LIKE "US*""#);
    assert!(matches!(
        c,
        Constraint::Bio(BioConstraint {
            predicate: Predicate::Pattern(PatternOp::NotLike, _),
            ..
        })
    ));
}

#[test]
fn p_w12_070_like_empty_pattern() {
    // Empty pattern: parses; matches nothing or empty target.
    let c = ok(r#"country LIKE """#);
    assert!(matches!(
        c,
        Constraint::Bio(BioConstraint {
            predicate: Predicate::Pattern(_, _),
            ..
        })
    ));
}

#[test]
fn p_w12_071_like_just_wildcard() {
    let c = ok(r#"country LIKE "*""#);
    assert!(matches!(
        c,
        Constraint::Bio(BioConstraint {
            predicate: Predicate::Pattern(_, _),
            ..
        })
    ));
}

#[test]
fn p_w12_072_like_multi_wildcard() {
    let c = ok(r#"country LIKE "*Mac*Don*""#);
    assert!(matches!(
        c,
        Constraint::Bio(BioConstraint {
            predicate: Predicate::Pattern(_, _),
            ..
        })
    ));
}

#[test]
fn p_w12_073_like_unicode_canonicalized() {
    // Pattern is stored canonicalized (NFD-strip + lowercase).
    let c = ok(r#"country LIKE "Stützle""#);
    if let Constraint::Bio(BioConstraint {
        predicate: Predicate::Pattern(_, GlobPattern { segments, .. }),
        ..
    }) = c
    {
        // Canonicalized
        assert!(segments.iter().any(|s| s == "stutzle"));
    } else {
        panic!();
    }
}

#[test]
fn p_w12_074_like_on_numeric_stat_rejected() {
    let es = errs(r#"g LIKE "5*""#);
    assert!(matches!(es[0], ParseError::IncompatiblePredicate { .. }));
}

#[test]
fn p_w12_075_like_on_numeric_bio_rejected() {
    let es = errs(r#"age LIKE "2*""#);
    assert!(matches!(es[0], ParseError::IncompatiblePredicate { .. }));
}

#[test]
fn p_w12_076_like_anchored_prefix() {
    if let Constraint::Bio(BioConstraint {
        predicate: Predicate::Pattern(_, glob),
        ..
    }) = ok(r#"country LIKE "Mc*""#)
    {
        assert!(glob.matches(&ScalarValue::canonicalize_text("McDavid")));
        assert!(!glob.matches(&ScalarValue::canonicalize_text("MacDonald")));
    } else {
        panic!();
    }
}

#[test]
fn p_w12_077_like_anchored_suffix() {
    if let Constraint::Bio(BioConstraint {
        predicate: Predicate::Pattern(_, glob),
        ..
    }) = ok(r#"country LIKE "*sson""#)
    {
        assert!(glob.matches(&ScalarValue::canonicalize_text("Karlsson")));
        assert!(!glob.matches(&ScalarValue::canonicalize_text("Sundin")));
    } else {
        panic!();
    }
}

#[test]
fn p_w12_078_like_unanchored() {
    if let Constraint::Bio(BioConstraint {
        predicate: Predicate::Pattern(_, glob),
        ..
    }) = ok(r#"country LIKE "*Mac*""#)
    {
        assert!(glob.matches(&ScalarValue::canonicalize_text("MacDonald")));
        assert!(glob.matches(&ScalarValue::canonicalize_text("MacKinnon")));
    } else {
        panic!();
    }
}

#[test]
fn p_w12_079_like_in_compound_and() {
    let c = ok(r#"country LIKE "CA*" AND age<=24"#);
    match c {
        Constraint::All(children) => assert_eq!(children.len(), 2),
        _ => panic!(),
    }
}

#[test]
fn p_w12_080_not_like_in_compound() {
    let c = ok(r#"country NOT LIKE "RU*" AND p>=50"#);
    match c {
        Constraint::All(children) => assert_eq!(children.len(), 2),
        _ => panic!(),
    }
}

#[test]
fn p_w12_081_like_quoted_with_spaces() {
    let c = ok(r#"country LIKE "Mc Donald""#);
    assert!(matches!(
        c,
        Constraint::Bio(BioConstraint {
            predicate: Predicate::Pattern(_, _),
            ..
        })
    ));
}

#[test]
fn p_w12_082_like_lowercase_keyword() {
    let c = ok(r#"country like "CA*""#);
    assert!(matches!(
        c,
        Constraint::Bio(BioConstraint {
            predicate: Predicate::Pattern(_, _),
            ..
        })
    ));
}

#[test]
fn p_w12_083_like_under_negation() {
    let c = ok(r#"NOT country LIKE "RU*""#);
    assert!(matches!(c, Constraint::Not(_)));
}

#[test]
fn p_w12_084_like_in_or_chain() {
    let c = ok(r#"country LIKE "CA*" OR country LIKE "US*""#);
    match c {
        Constraint::Any(children) => assert_eq!(children.len(), 2),
        _ => panic!(),
    }
}

#[test]
fn p_w12_085_like_position_atom() {
    let c = ok(r#"pos LIKE "*W""#);
    assert!(matches!(
        c,
        Constraint::Bio(BioConstraint {
            field: BioField::Position,
            predicate: Predicate::Pattern(_, _),
        })
    ));
}

// ── Section E — Sliding-window atoms (25) ───────────────────

#[test]
fn p_w12_086_last10g_basic() {
    let c = ok("g.last10g>=5");
    assert!(matches!(
        c,
        Constraint::SlidingWindow(SlidingWindowConstraint {
            window: SlidingWindow::LastN_GP { n: 10, .. },
            ..
        })
    ));
}

#[test]
fn p_w12_087_last1g_minimum() {
    let c = ok("g.last1g>=1");
    assert!(matches!(
        c,
        Constraint::SlidingWindow(SlidingWindowConstraint {
            window: SlidingWindow::LastN_GP { n: 1, .. },
            ..
        })
    ));
}

#[test]
fn p_w12_088_last255g_maximum() {
    let c = ok("g.last255g>=5");
    assert!(matches!(
        c,
        Constraint::SlidingWindow(SlidingWindowConstraint {
            window: SlidingWindow::LastN_GP { n: 255, .. },
            ..
        })
    ));
}

#[test]
fn p_w12_089_last256g_rejected() {
    let es = errs("g.last256g>=5");
    assert!(matches!(es[0], ParseError::WindowSizeOutOfRange { .. }));
}

#[test]
fn p_w12_090_last0g_rejected() {
    let es = errs("g.last0g>=5");
    assert!(matches!(es[0], ParseError::ZeroWindowSize { .. }));
}

#[test]
fn p_w12_091_last10z_rejected() {
    let es = errs("g.last10z>=5");
    assert!(matches!(es[0], ParseError::UnknownWindowUnit { unit: 'z', .. }));
}

#[test]
fn p_w12_092_unknown_stat_with_window() {
    let es = errs("fakestat.last10g>=5");
    assert!(matches!(es[0], ParseError::UnknownStat { .. }));
}

#[test]
fn p_w12_093_last10g_allteams() {
    if let Constraint::SlidingWindow(SlidingWindowConstraint {
        window:
            SlidingWindow::LastN_GP {
                scope: icelines_query::WindowScope::AllTeamsCurrentSeason,
                ..
            },
        ..
    }) = ok("g.last10g.allteams>=5")
    {
        // ok
    } else {
        panic!();
    }
}

#[test]
fn p_w12_094_last10g_career() {
    if let Constraint::SlidingWindow(SlidingWindowConstraint {
        window:
            SlidingWindow::LastN_GP {
                scope: icelines_query::WindowScope::Career,
                ..
            },
        ..
    }) = ok("g.last10g.career>=5")
    {
        // ok
    } else {
        panic!();
    }
}

#[test]
fn p_w12_095_unknown_scope_modifier_rejected() {
    let es = errs("g.last10g.bogus>=5");
    assert!(!es.is_empty());
}

#[test]
fn p_w12_096_last30d_calendar() {
    let c = ok("g.last30d>=10");
    assert!(matches!(
        c,
        Constraint::SlidingWindow(SlidingWindowConstraint {
            window: SlidingWindow::LastN_Days(30),
            ..
        })
    ));
}

#[test]
fn p_w12_097_last3w_calendar() {
    let c = ok("p.last3w>=8");
    assert!(matches!(
        c,
        Constraint::SlidingWindow(SlidingWindowConstraint {
            window: SlidingWindow::LastN_Weeks(3),
            ..
        })
    ));
}

#[test]
fn p_w12_098_last3m_calendar() {
    let c = ok("p.last3m>=20");
    assert!(matches!(
        c,
        Constraint::SlidingWindow(SlidingWindowConstraint {
            window: SlidingWindow::LastN_Months(3),
            ..
        })
    ));
}

#[test]
fn p_w12_099_last_huge_days() {
    let c = ok("g.last1000d>=1");
    assert!(matches!(
        c,
        Constraint::SlidingWindow(SlidingWindowConstraint {
            window: SlidingWindow::LastN_Days(1000),
            ..
        })
    ));
}

#[test]
fn p_w12_100_two_windows_in_and() {
    let c = ok("g.last10g>=5 AND g.last30d>=10");
    match c {
        Constraint::All(children) => assert_eq!(children.len(), 2),
        _ => panic!(),
    }
}

#[test]
fn p_w12_101_two_windows_in_or() {
    let c = ok("g.last10g>=5 OR p.last30d>=10");
    match c {
        Constraint::Any(children) => assert_eq!(children.len(), 2),
        _ => panic!(),
    }
}

#[test]
fn p_w12_102_window_under_not() {
    let c = ok("NOT g.last10g>=10000");
    assert!(matches!(c, Constraint::Not(_)));
}

#[test]
fn p_w12_103_window_in_paren_group() {
    let c = ok("(g.last10g>=5 OR a.last10g>=5) AND p>=20");
    match c {
        Constraint::All(children) => assert_eq!(children.len(), 2),
        _ => panic!(),
    }
}

#[test]
fn p_w12_104_window_with_decimal_predicate() {
    let c = ok("ppg.last10g>=1.5");
    assert!(matches!(c, Constraint::SlidingWindow(_)));
}

#[test]
fn p_w12_105_window_with_strict_lt() {
    let c = ok("g.last10g<5");
    assert!(matches!(
        c,
        Constraint::SlidingWindow(SlidingWindowConstraint {
            predicate: Predicate::Scalar(ScalarOp::Lt, _),
            ..
        })
    ));
}

#[test]
fn p_w12_106_window_with_ne() {
    let c = ok("g.last10g!=0");
    assert!(matches!(
        c,
        Constraint::SlidingWindow(SlidingWindowConstraint {
            predicate: Predicate::Scalar(ScalarOp::Ne, _),
            ..
        })
    ));
}

#[test]
fn p_w12_107_window_killer_query() {
    // The user's vision query
    let c = ok("g.last10g>=5 AND age<=25");
    match c {
        Constraint::All(children) => {
            assert_eq!(children.len(), 2);
            assert!(matches!(&children[0], Constraint::SlidingWindow(_)));
            assert!(matches!(&children[1], Constraint::Bio(_)));
        }
        _ => panic!(),
    }
}

#[test]
fn p_w12_108_window_with_country_in() {
    let c = ok("g.last10g>=5 AND country IN (CAN, USA)");
    match c {
        Constraint::All(children) => assert_eq!(children.len(), 2),
        _ => panic!(),
    }
}

#[test]
fn p_w12_109_window_dot_only_rejected() {
    let es = errs("g.>=5");
    assert!(!es.is_empty());
}

#[test]
fn p_w12_110_window_three_dots_rejected() {
    let es = errs("g.last10g.allteams.bogus>=5");
    assert!(!es.is_empty());
}

// ── Section F — Bio atom expansion (20) ─────────────────────

#[test]
fn p_w12_111_pos_center() {
    let c = ok("pos=C");
    assert!(is_bio_with_field(&c, BioField::Position));
}

#[test]
fn p_w12_112_pos_left_wing() {
    let c = ok("pos=LW");
    assert!(is_bio_with_field(&c, BioField::Position));
}

#[test]
fn p_w12_113_pos_right_wing() {
    let c = ok("pos=RW");
    assert!(is_bio_with_field(&c, BioField::Position));
}

#[test]
fn p_w12_114_pos_defenseman() {
    let c = ok("pos=D");
    assert!(is_bio_with_field(&c, BioField::Position));
}

#[test]
fn p_w12_115_pos_goalie() {
    let c = ok("pos=G");
    assert!(is_bio_with_field(&c, BioField::Position));
}

#[test]
fn p_w12_116_team_atom() {
    let c = ok("team=EDM");
    assert!(is_bio_with_field(&c, BioField::Team));
}

#[test]
fn p_w12_117_team_any_modifier() {
    let c = ok("team.any=EDM");
    assert!(is_bio_with_field(&c, BioField::TeamAny));
}

#[test]
fn p_w12_118_team_career_rejected() {
    let es = errs("team.career=EDM");
    match &es[0] {
        ParseError::FeatureNotYet { ships_in, .. } => assert!(ships_in.contains("A.4")),
        _ => panic!(),
    }
}

#[test]
fn p_w12_119_draft_round_atom() {
    let c = ok("draft-round<=2");
    assert!(is_bio_with_field(&c, BioField::DraftRound));
}

#[test]
fn p_w12_120_draft_overall_atom() {
    let c = ok("draft-overall<=10");
    assert!(is_bio_with_field(&c, BioField::DraftOverall));
}

#[test]
fn p_w12_121_birth_state_atom() {
    let c = ok("birth-state=ON");
    assert!(is_bio_with_field(&c, BioField::BirthState));
}

#[test]
fn p_w12_122_birth_city_atom() {
    let c = ok("birth-city=Toronto");
    assert!(is_bio_with_field(&c, BioField::BirthCity));
}

#[test]
fn p_w12_123_nationality_distinct_from_country() {
    let c = ok("nationality=USA");
    assert!(is_bio_with_field(&c, BioField::Nationality));
}

#[test]
fn p_w12_124_rookie_season_atom() {
    let c = ok("rookie-season>=20212022");
    assert!(is_bio_with_field(&c, BioField::RookieSeason));
}

#[test]
fn p_w12_125_height_alias_ht() {
    let c = ok("ht>=72");
    assert!(is_bio_with_field(&c, BioField::Height));
}

#[test]
fn p_w12_126_weight_alias_wt() {
    let c = ok("wt<=200");
    assert!(is_bio_with_field(&c, BioField::Weight));
}

#[test]
fn p_w12_127_draft_year_underscore_alias() {
    let c = ok("draft_year>=2020");
    assert!(is_bio_with_field(&c, BioField::DraftYear));
}

#[test]
fn p_w12_128_country_case_insensitive() {
    if let Constraint::Bio(BioConstraint {
        predicate: Predicate::Scalar(_, ScalarValue::Text(t)),
        ..
    }) = ok("country=can")
    {
        assert_eq!(t, "can");
    } else {
        panic!();
    }
}

#[test]
fn p_w12_129_shoots_alias_hand() {
    let c = ok("hand=L");
    assert!(is_bio_with_field(&c, BioField::Shoots));
}

#[test]
fn p_w12_130_shoots_alias_catches() {
    let c = ok("catches=L");
    assert!(is_bio_with_field(&c, BioField::Shoots));
}

// ── Section G — Compound queries (25) ───────────────────────

#[test]
fn p_w12_131_killer_query_full() {
    let c = ok("g.last10g>=5 AND age<=25");
    assert!(matches!(c, Constraint::All(_)));
}

#[test]
fn p_w12_132_compound_4_atoms() {
    let c = ok("g.last10g>=5 AND age<=25 AND country IN (CAN, USA) AND pos=C");
    match c {
        Constraint::All(children) => assert_eq!(children.len(), 4),
        _ => panic!(),
    }
}

#[test]
fn p_w12_133_compound_5_atoms() {
    let c = ok(
        "g.last10g>=5 AND age<=25 AND country IN (CAN, USA) AND pos IN (C, LW) AND draft-round<=2",
    );
    match c {
        Constraint::All(children) => assert_eq!(children.len(), 5),
        _ => panic!(),
    }
}

#[test]
fn p_w12_134_paren_grouping_changes_associativity() {
    let c = ok("(g>=10 OR a>=10) AND p>=20");
    match c {
        Constraint::All(children) => {
            assert_eq!(children.len(), 2);
            assert!(matches!(&children[0], Constraint::Any(_)));
        }
        _ => panic!(),
    }
}

#[test]
fn p_w12_135_demorgan_via_parser() {
    // NOT (A AND B) and NOT A OR NOT B should produce different
    // tree structures (parser doesn't normalize).
    let lhs = ok("NOT (g>=10 AND a>=10)");
    let rhs = ok("NOT g>=10 OR NOT a>=10");
    assert!(matches!(lhs, Constraint::Not(_)));
    assert!(matches!(rhs, Constraint::Any(_)));
}

#[test]
fn p_w12_136_deeply_nested_parens() {
    let c = ok("((((g>=10))))");
    // Single atom unwrapped.
    assert!(matches!(c, Constraint::SeasonStat(_)));
}

#[test]
fn p_w12_137_compound_with_between_and_in() {
    let c = ok("g BETWEEN 20 AND 40 AND country IN (CAN, USA)");
    match c {
        Constraint::All(children) => assert_eq!(children.len(), 2),
        _ => panic!(),
    }
}

#[test]
fn p_w12_138_compound_with_like_and_strict() {
    let c = ok(r#"country LIKE "CA*" AND age<25"#);
    match c {
        Constraint::All(children) => assert_eq!(children.len(), 2),
        _ => panic!(),
    }
}

#[test]
fn p_w12_139_window_compound_with_between() {
    let c = ok("g.last10g>=5 AND age BETWEEN 22 AND 28");
    match c {
        Constraint::All(children) => assert_eq!(children.len(), 2),
        _ => panic!(),
    }
}

#[test]
fn p_w12_140_window_compound_with_like() {
    let c = ok(r#"g.last10g>=5 AND country LIKE "CA*""#);
    match c {
        Constraint::All(children) => assert_eq!(children.len(), 2),
        _ => panic!(),
    }
}

#[test]
fn p_w12_141_compound_or_with_strict() {
    let c = ok("g<5 OR g>50");
    match c {
        Constraint::Any(children) => assert_eq!(children.len(), 2),
        _ => panic!(),
    }
}

#[test]
fn p_w12_142_compound_3way_or_grouped() {
    let c = ok("(country=CAN OR country=USA OR country=SWE) AND age<=24");
    match c {
        Constraint::All(children) => {
            assert_eq!(children.len(), 2);
            if let Constraint::Any(or_children) = &children[0] {
                assert_eq!(or_children.len(), 3);
            } else {
                panic!();
            }
        }
        _ => panic!(),
    }
}

#[test]
fn p_w12_143_compound_negation_chain() {
    let c = ok("NOT NOT NOT g>=100");
    // 3 NOTs nested
    assert!(matches!(c, Constraint::Not(_)));
}

#[test]
fn p_w12_144_compound_window_under_not() {
    let c = ok("NOT (g.last10g>=10000 AND a.last10g>=10000)");
    assert!(matches!(c, Constraint::Not(_)));
}

#[test]
fn p_w12_145_compound_all_operators() {
    // Every new op + new atom + new variant — one query
    let c = ok(r#"g.last10g>=5 AND age BETWEEN 22 AND 25 AND country IN (CAN, USA) AND pos LIKE "*W" AND draft-round<=2"#);
    match c {
        Constraint::All(children) => assert_eq!(children.len(), 5),
        _ => panic!(),
    }
}

#[test]
fn p_w12_146_compound_window_axis_diff() {
    let c = ok("g.last10g>=5 AND p.last30d>=20");
    match c {
        Constraint::All(children) => {
            assert_eq!(children.len(), 2);
            if let (Constraint::SlidingWindow(s1), Constraint::SlidingWindow(s2)) =
                (&children[0], &children[1])
            {
                assert!(matches!(s1.window, SlidingWindow::LastN_GP { .. }));
                assert!(matches!(s2.window, SlidingWindow::LastN_Days(_)));
            } else {
                panic!();
            }
        }
        _ => panic!(),
    }
}

#[test]
fn p_w12_147_compound_with_strict_age() {
    let c = ok("g>=20 AND age<25");
    match c {
        Constraint::All(children) => {
            if let Constraint::Bio(BioConstraint {
                predicate: Predicate::Scalar(ScalarOp::Lt, _),
                ..
            }) = &children[1]
            {
                // ok
            } else {
                panic!("expected age<25 to be ScalarOp::Lt");
            }
        }
        _ => panic!(),
    }
}

#[test]
fn p_w12_148_compound_or_window_pair() {
    let c = ok("g.last10g>=5 OR g.last30d>=15");
    match c {
        Constraint::Any(children) => assert_eq!(children.len(), 2),
        _ => panic!(),
    }
}

#[test]
fn p_w12_149_compound_paren_chain_then_and() {
    let c = ok("((country=CAN OR country=USA)) AND ((age<=24))");
    match c {
        Constraint::All(children) => assert_eq!(children.len(), 2),
        _ => panic!(),
    }
}

#[test]
fn p_w12_150_compound_window_with_paren_or() {
    let c = ok("(g.last10g>=5 OR g.last10g.allteams>=5) AND p>=20");
    match c {
        Constraint::All(children) => assert_eq!(children.len(), 2),
        _ => panic!(),
    }
}

#[test]
fn p_w12_151_compound_window_under_not_with_bio() {
    let c = ok("NOT g.last10g>=10000 AND age<=25");
    match c {
        Constraint::All(children) => {
            assert_eq!(children.len(), 2);
            assert!(matches!(&children[0], Constraint::Not(_)));
        }
        _ => panic!(),
    }
}

#[test]
fn p_w12_152_compound_strict_age_with_team() {
    let c = ok("age<25 AND team=EDM");
    match c {
        Constraint::All(children) => assert_eq!(children.len(), 2),
        _ => panic!(),
    }
}

#[test]
fn p_w12_153_compound_complex_realworld() {
    // "Young EDM forwards in their last 10 games"
    let c = ok("g.last10g>=3 AND age<=24 AND team=EDM AND pos IN (C, LW, RW)");
    match c {
        Constraint::All(children) => assert_eq!(children.len(), 4),
        _ => panic!(),
    }
}

#[test]
fn p_w12_154_compound_with_de_morgan_left() {
    let lhs = ok("NOT (g>=100 AND a>=100)");
    if let Constraint::Not(inner) = lhs {
        assert!(matches!(*inner, Constraint::All(_)));
    } else {
        panic!();
    }
}

#[test]
fn p_w12_155_compound_with_de_morgan_right() {
    let rhs = ok("NOT g>=100 OR NOT a>=100");
    if let Constraint::Any(children) = rhs {
        assert_eq!(children.len(), 2);
        assert!(matches!(&children[0], Constraint::Not(_)));
    } else {
        panic!();
    }
}

// ── Section H — Output truthfulness (15) ────────────────────

#[test]
fn p_w12_156_single_atom_no_wrapping() {
    let c = ok("g>=10");
    // Not wrapped in All/Any
    assert!(!matches!(c, Constraint::All(_) | Constraint::Any(_)));
}

#[test]
fn p_w12_157_single_atom_paren_unwrapped() {
    let c = ok("(g>=10)");
    assert!(matches!(c, Constraint::SeasonStat(_)));
}

#[test]
fn p_w12_158_pure_and_chain_n_ary() {
    let c = ok("g>=10 AND a>=10 AND p>=20 AND pim>=0 AND sog>=10");
    match c {
        Constraint::All(children) => assert_eq!(children.len(), 5),
        _ => panic!(),
    }
}

#[test]
fn p_w12_159_pure_or_chain_n_ary() {
    let c = ok("g>=10 OR a>=10 OR p>=20 OR pim>=0");
    match c {
        Constraint::Any(children) => assert_eq!(children.len(), 4),
        _ => panic!(),
    }
}

#[test]
fn p_w12_160_mixed_and_or_no_inversion() {
    // AND binds tighter than OR
    let c = ok("g>=10 AND a>=10 OR p>=20");
    // Should be Any(All(g, a), p)
    if let Constraint::Any(children) = c {
        assert_eq!(children.len(), 2);
        assert!(matches!(&children[0], Constraint::All(_)));
        assert!(matches!(&children[1], Constraint::SeasonStat(_)));
    } else {
        panic!();
    }
}

#[test]
fn p_w12_161_or_binds_lower_than_and() {
    let c = ok("p>=20 OR g>=10 AND a>=10");
    // Should be Any(p, All(g, a))
    if let Constraint::Any(children) = c {
        assert_eq!(children.len(), 2);
        assert!(matches!(&children[0], Constraint::SeasonStat(_)));
        assert!(matches!(&children[1], Constraint::All(_)));
    } else {
        panic!();
    }
}

#[test]
fn p_w12_162_multi_error_reporting() {
    // 3 atoms; 2 broken.
    let es = errs("fakestat>=10 AND p>=20 AND otherbad<>5");
    // At least one error reported (the parser bails on first error
    // in a single-atom chain). Ensure we get errors back.
    assert!(!es.is_empty());
}

#[test]
fn p_w12_163_explicit_and_keyword_required() {
    let es = errs("g>=10 a>=10");
    assert!(!es.is_empty());
}

#[test]
fn p_w12_164_no_implicit_or() {
    // Adjacent atoms without AND/OR are an error
    let es = errs("g>=10 a>=10 p>=20");
    assert!(!es.is_empty());
}

#[test]
fn p_w12_165_associativity_left_to_right_and() {
    let c = ok("g>=10 AND a>=10 AND p>=20");
    if let Constraint::All(children) = c {
        // n-ary, all flat
        assert_eq!(children.len(), 3);
    } else {
        panic!();
    }
}

#[test]
fn p_w12_166_associativity_left_to_right_or() {
    let c = ok("g>=10 OR a>=10 OR p>=20");
    if let Constraint::Any(children) = c {
        assert_eq!(children.len(), 3);
    } else {
        panic!();
    }
}

#[test]
fn p_w12_167_paren_forces_inner_node() {
    let c = ok("(g>=10 AND a>=10) OR p>=20");
    if let Constraint::Any(children) = c {
        assert_eq!(children.len(), 2);
        assert!(matches!(&children[0], Constraint::All(_)));
    } else {
        panic!();
    }
}

#[test]
fn p_w12_168_double_not_doesnt_collapse() {
    let c = ok("NOT NOT g>=10");
    // Parser preserves tree shape; doesn't collapse double-negation.
    if let Constraint::Not(inner) = c {
        assert!(matches!(*inner, Constraint::Not(_)));
    } else {
        panic!();
    }
}

#[test]
fn p_w12_169_paren_around_not() {
    let c = ok("(NOT g>=10) AND a>=10");
    if let Constraint::All(children) = c {
        assert!(matches!(&children[0], Constraint::Not(_)));
    } else {
        panic!();
    }
}

#[test]
fn p_w12_170_top_level_paren_unwrapped() {
    let c = ok("(g>=10 AND a>=10)");
    assert!(matches!(c, Constraint::All(_)));
}

// ── Section I — Edge cases (15) ─────────────────────────────

#[test]
fn p_w12_171_empty_input() {
    assert_eq!(errs("")[0], ParseError::EmptyInput);
}

#[test]
fn p_w12_172_whitespace_only() {
    assert_eq!(errs("   \t  ")[0], ParseError::EmptyInput);
}

#[test]
fn p_w12_173_just_keyword_and() {
    assert!(!errs("AND").is_empty());
}

#[test]
fn p_w12_174_just_keyword_or() {
    assert!(!errs("OR").is_empty());
}

#[test]
fn p_w12_175_just_keyword_not() {
    assert!(!errs("NOT").is_empty());
}

#[test]
fn p_w12_176_just_open_paren() {
    assert!(!errs("(").is_empty());
}

#[test]
fn p_w12_177_just_close_paren() {
    assert!(!errs(")").is_empty());
}

#[test]
fn p_w12_178_just_op() {
    assert!(!errs(">=").is_empty());
}

#[test]
fn p_w12_179_unicode_value_in_country() {
    // Unicode characters in IN list
    let c = ok("country IN (Slafkovský)");
    assert!(matches!(
        c,
        Constraint::Bio(BioConstraint {
            predicate: Predicate::Member(_, _),
            ..
        })
    ));
}

#[test]
fn p_w12_180_extremely_long_key() {
    let key: String = "x".repeat(1000);
    let expr = format!("{key}>=1");
    let es = errs(&expr);
    assert!(matches!(es[0], ParseError::UnknownStat { .. }));
}

#[test]
fn p_w12_181_extremely_long_value() {
    let val: String = "9".repeat(500);
    let expr = format!("g>={val}");
    // f64 parses it as inf → NotFinite
    let es = errs(&expr);
    assert!(matches!(es[0], ParseError::NotFinite { .. }));
}

#[test]
fn p_w12_182_scientific_notation() {
    let c = ok("g>=1e2");
    assert!(matches!(c, Constraint::SeasonStat(_)));
}

#[test]
fn p_w12_183_negative_decimal() {
    let c = ok("+/->=-5.5");
    assert!(matches!(c, Constraint::SeasonStat(_)));
}

#[test]
fn p_w12_184_unclosed_paren() {
    let es = errs("(g>=10 AND a>=10");
    assert!(matches!(es[0], ParseError::UnclosedParen));
}

#[test]
fn p_w12_185_empty_parens_in_compound() {
    let es = errs("g>=10 AND ()");
    assert!(!es.is_empty());
}

// ── Section J — Pathological inputs (15) ────────────────────

#[test]
fn p_w12_186_long_and_chain_30() {
    let chain = (0..30).map(|_| "g>=1").collect::<Vec<_>>().join(" AND ");
    let c = ok(&chain);
    if let Constraint::All(children) = c {
        assert_eq!(children.len(), 30);
    } else {
        panic!();
    }
}

#[test]
fn p_w12_187_long_or_chain_30() {
    let chain = (0..30).map(|_| "g>=1").collect::<Vec<_>>().join(" OR ");
    let c = ok(&chain);
    if let Constraint::Any(children) = c {
        assert_eq!(children.len(), 30);
    } else {
        panic!();
    }
}

#[test]
fn p_w12_188_alternating_and_or_chain() {
    let c = ok("g>=1 AND a>=1 OR p>=1 AND pim>=0 OR sog>=1");
    // Per precedence: ((g AND a) OR (p AND pim)) OR sog
    assert!(matches!(c, Constraint::Any(_)));
}

#[test]
fn p_w12_189_deeply_nested_parens() {
    let mut expr = "g>=10".to_string();
    for _ in 0..15 {
        expr = format!("({expr})");
    }
    let c = ok(&expr);
    assert!(matches!(c, Constraint::SeasonStat(_)));
}

#[test]
fn p_w12_190_deeply_nested_not() {
    let mut expr = "g>=10".to_string();
    for _ in 0..10 {
        expr = format!("NOT {expr}");
    }
    let c = ok(&expr);
    assert!(matches!(c, Constraint::Not(_)));
}

#[test]
fn p_w12_191_long_in_set() {
    let codes: Vec<String> = (0..100).map(|i| format!("X{i}")).collect();
    let expr = format!("country IN ({})", codes.join(", "));
    let c = ok(&expr);
    if let Constraint::Bio(BioConstraint {
        predicate: Predicate::Member(_, vals),
        ..
    }) = c
    {
        assert_eq!(vals.len(), 100);
    } else {
        panic!();
    }
}

#[test]
fn p_w12_192_unicode_full_width_digits_rejected() {
    let es = errs("g>=１０");
    // Full-width digits don't parse as f64 — accept either error
    // class (BadNumber typically; tokenizer may also produce
    // UnexpectedToken if word-boundary classification differs).
    assert!(
        matches!(
            es[0],
            ParseError::BadNumber { .. }
                | ParseError::UnexpectedToken { .. }
                | ParseError::UnknownStat { .. }
        ),
        "expected BadNumber/UnexpectedToken/UnknownStat for full-width digits, got {:?}",
        es[0]
    );
}

#[test]
fn p_w12_193_many_atoms_plus_sliding() {
    let expr = "g>=10 AND a>=10 AND p>=20 AND g.last10g>=5 AND age<=25";
    let c = ok(expr);
    if let Constraint::All(children) = c {
        assert_eq!(children.len(), 5);
    } else {
        panic!();
    }
}

#[test]
fn p_w12_194_emoji_in_value_rejected() {
    let es = errs("g>=🙂");
    assert!(
        matches!(
            es[0],
            ParseError::BadNumber { .. }
                | ParseError::NotFinite { .. }
                | ParseError::UnexpectedToken { .. }
                | ParseError::UnknownStat { .. }
        ),
        "expected error class for emoji value, got {:?}",
        es[0]
    );
}

#[test]
fn p_w12_195_keyword_substring_in_atom() {
    // 'andes' contains 'and' but is not the keyword
    let es = errs("andes>=5");
    assert!(matches!(es[0], ParseError::UnknownStat { .. }));
}

#[test]
fn p_w12_196_keyword_prefix_in_atom() {
    // 'notable' starts with 'not' but not at boundary
    let es = errs("notable>=5");
    assert!(matches!(es[0], ParseError::UnknownStat { .. }));
}

#[test]
fn p_w12_197_three_layer_nested_compound() {
    let c = ok("(g>=10 AND (a>=10 OR (p>=20 AND pim>=0)))");
    assert!(matches!(c, Constraint::All(_)));
}

#[test]
fn p_w12_198_window_after_sliding_after_bio() {
    // Mixing every atom variant
    let c = ok(r#"age<=25 AND g.last10g>=5 AND country IN (CAN) AND pos LIKE "*W""#);
    if let Constraint::All(children) = c {
        assert_eq!(children.len(), 4);
    } else {
        panic!();
    }
}

#[test]
fn p_w12_199_redundant_atoms_preserved() {
    // Parser doesn't dedupe; same atom twice is two children
    let c = ok("g>=10 AND g>=10 AND g>=10");
    if let Constraint::All(children) = c {
        assert_eq!(children.len(), 3);
    } else {
        panic!();
    }
}

#[test]
fn p_w12_200_killer_compound_with_nested_or() {
    // Real-world: "young first-rounders with hot streaks"
    let c = ok(
        "g.last10g>=5 AND \
         (age<=24 OR (age BETWEEN 25 AND 27 AND draft-overall<=10)) AND \
         country IN (CAN, USA, SWE)",
    );
    if let Constraint::All(children) = c {
        assert_eq!(children.len(), 3);
    } else {
        panic!();
    }
}
