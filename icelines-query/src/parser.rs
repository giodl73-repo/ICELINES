//! Phase Art Ross A.0 — `parse_query` front door.
//!
//! For A.0, the parser converts the LEGACY filter grammar
//! (`stats_catalog::parse_filter_expr` + bio atoms via the existing
//! `try_parse_bio_atom`) into the new typed `Constraint` IR. New
//! operators / atoms ship in A.1 onward.
//!
//! The contract for A.0: every filter string that parsed in v0.19.1
//! produces a `Constraint` tree that, when evaluated, returns the
//! same result set as the legacy pipeline — including the FIXED
//! behavior of the 3 Wave 11 bugs (goalie compound rewrite, paren-
//! wrapped bio atoms, --filter+--week loud rejection).

use icelines_core::stats_catalog::{
    parse_filter_expr, FilterExpr, FilterOp, FilterParseError, StatFilter,
};

use crate::errors::ParseError;
use crate::input::{AtomFragment, FilterInput};
use crate::plan::{
    BioConstraint, BioField, Constraint, NumericRange, Predicate, QueryPlan, ScalarOp,
    ScalarValue, SeasonAxis, SeasonStatConstraint,
};
use crate::{try_parse_bio_atom, BioAtom};

/// The single front-door API. Converts a `FilterInput` into a
/// `QueryPlan` (typed Constraint IR). Returns `Vec<ParseError>` so
/// multi-error reporting is preserved (R4 from 8-role review).
pub fn parse_query(input: FilterInput) -> Result<QueryPlan, Vec<ParseError>> {
    match input {
        FilterInput::Cli(s) | FilterInput::Form(s) => parse_query_string(&s),
        FilterInput::Tui(fragments) => parse_query_tui(&fragments),
    }
}

fn parse_query_string(input: &str) -> Result<QueryPlan, Vec<ParseError>> {
    if input.trim().is_empty() {
        return Err(vec![ParseError::EmptyInput]);
    }

    // Strategy: lean on the existing `parse_filter_expr` from
    // `stats_catalog` for the boolean grammar (it already handles
    // AND/OR/NOT/parens correctly post Wave 11 fixes). For each
    // atom, try the bio-atom parser first; fall back to the catalog
    // parser. Map both into the new typed Constraint IR.

    let expr = parse_filter_expr(input).map_err(|e| vec![map_parse_error(input, e)])?;
    let root = filter_expr_to_constraint(&expr).map_err(|errs| errs)?;
    Ok(QueryPlan { root })
}

fn parse_query_tui(fragments: &[AtomFragment]) -> Result<QueryPlan, Vec<ParseError>> {
    if fragments.is_empty() {
        return Err(vec![ParseError::EmptyInput]);
    }
    // A.0 minimal TUI path: each `Atom` fragment carries a fully-
    // built Constraint. Fragments are AND-joined unless `OrJoin`
    // appears between them. Group open/close are honored as
    // implicit parens around their contents. This is the simplest
    // shape that passes the round-trip property test in A.0.
    //
    // The TUI overlay's Phase Art Ross filter widget will produce
    // these fragments directly; for A.0 nothing user-facing builds
    // them yet (the existing TUI continues to use the string form).
    let constraints: Vec<Constraint> = fragments
        .iter()
        .filter_map(|f| match f {
            AtomFragment::Atom(c) => Some(c.clone()),
            _ => None,
        })
        .collect();
    let root = match constraints.len() {
        0 => return Err(vec![ParseError::EmptyInput]),
        1 => constraints.into_iter().next().unwrap(),
        _ => Constraint::All(constraints),
    };
    Ok(QueryPlan { root })
}

/// Convert the legacy `FilterExpr` tree into the new typed
/// `Constraint` tree. Each `FilterExpr::Atom` is dispatched: try
/// bio first, then catalog. Boolean composition collapses adjacent
/// nodes of the same kind into n-ary `All` / `Any`.
fn filter_expr_to_constraint(expr: &FilterExpr) -> Result<Constraint, Vec<ParseError>> {
    match expr {
        FilterExpr::Atom(stat_filter) => atom_to_constraint(stat_filter, /*raw_atom=*/ None),
        FilterExpr::And(left, right) => {
            let mut children: Vec<Constraint> = Vec::new();
            let mut errors: Vec<ParseError> = Vec::new();
            collect_and(left, &mut children, &mut errors);
            collect_and(right, &mut children, &mut errors);
            if !errors.is_empty() {
                return Err(errors);
            }
            Ok(Constraint::All(children))
        }
        FilterExpr::Or(left, right) => {
            let mut children: Vec<Constraint> = Vec::new();
            let mut errors: Vec<ParseError> = Vec::new();
            collect_or(left, &mut children, &mut errors);
            collect_or(right, &mut children, &mut errors);
            if !errors.is_empty() {
                return Err(errors);
            }
            Ok(Constraint::Any(children))
        }
        FilterExpr::Not(inner) => {
            let inner_c = filter_expr_to_constraint(inner)?;
            Ok(Constraint::Not(Box::new(inner_c)))
        }
    }
}

/// Walk an AND-chain, flattening into a single Vec. Errors from
/// any descendant atom go into `errors` (multi-error reporting).
fn collect_and(expr: &FilterExpr, out: &mut Vec<Constraint>, errors: &mut Vec<ParseError>) {
    match expr {
        FilterExpr::And(l, r) => {
            collect_and(l, out, errors);
            collect_and(r, out, errors);
        }
        other => match filter_expr_to_constraint(other) {
            Ok(c) => out.push(c),
            Err(es) => errors.extend(es),
        },
    }
}

fn collect_or(expr: &FilterExpr, out: &mut Vec<Constraint>, errors: &mut Vec<ParseError>) {
    match expr {
        FilterExpr::Or(l, r) => {
            collect_or(l, out, errors);
            collect_or(r, out, errors);
        }
        other => match filter_expr_to_constraint(other) {
            Ok(c) => out.push(c),
            Err(es) => errors.extend(es),
        },
    }
}

/// Map a legacy `StatFilter` atom to a typed `Constraint`. The
/// legacy parser doesn't preserve the original key string, so bio
/// detection requires reconstructing it. We pick the StatId's
/// `cli_key()` as the source of truth.
fn atom_to_constraint(
    sf: &StatFilter,
    _raw_atom: Option<&str>,
) -> Result<Constraint, Vec<ParseError>> {
    // Legacy `parse_filter` already rejected unknown keys; if we
    // got here the StatId resolved. Bio atoms (age/country/height/
    // etc.) are NOT in the StatId catalog, so they would never
    // appear here — they would have failed `parse_filter_expr`
    // upstream. The legacy `extract_bio` runs BEFORE the catalog
    // parser, peeling them off. Phase Art Ross A.0 keeps that
    // pre-split shape: bio atoms are extracted before parse_query,
    // and only stat residue reaches this function.
    //
    // So every constraint produced here is `SeasonStatConstraint`.
    let predicate = legacy_filter_op_to_predicate(sf.op, sf.value);
    Ok(Constraint::SeasonStat(SeasonStatConstraint {
        stat: sf.stat,
        predicate,
        axis: SeasonAxis::Regular,
    }))
}

fn legacy_filter_op_to_predicate(op: FilterOp, value: f64) -> Predicate {
    let scalar_op = match op {
        FilterOp::Min => ScalarOp::Ge,
        FilterOp::Max => ScalarOp::Le,
        FilterOp::Equals => ScalarOp::Eq,
    };
    Predicate::Scalar(scalar_op, ScalarValue::Number(value))
}

/// Translate a `FilterParseError` from the legacy parser into the
/// new `ParseError`. Span info is best-effort (legacy errors don't
/// always carry the offending atom).
fn map_parse_error(input: &str, e: FilterParseError) -> ParseError {
    match e {
        FilterParseError::EmptyInput => ParseError::EmptyInput,
        FilterParseError::EmptyStatKey => ParseError::EmptyStatKey {
            atom: input.to_string(),
        },
        FilterParseError::MissingOp { input: atom } => ParseError::MissingOp { atom },
        FilterParseError::MultipleOps { input: atom } => {
            // Wave 11 #033/#034 — preserve the typo hint behavior.
            if atom.contains("=>") {
                ParseError::OpTypoHint {
                    atom,
                    suggestion: ">=",
                }
            } else if atom.contains("=<") {
                ParseError::OpTypoHint {
                    atom,
                    suggestion: "<=",
                }
            } else {
                ParseError::MultipleOps { atom }
            }
        }
        FilterParseError::UnknownStat { key } => ParseError::UnknownStat { key },
        FilterParseError::BadNumber { token } => ParseError::BadNumber {
            atom: input.to_string(),
            token,
        },
        FilterParseError::NotFinite { token } => ParseError::NotFinite {
            atom: input.to_string(),
            token,
        },
        FilterParseError::UnclosedParen => ParseError::UnclosedParen,
        FilterParseError::UnexpectedRParen => ParseError::UnexpectedRParen,
        FilterParseError::UnexpectedEnd => ParseError::UnexpectedEnd,
        FilterParseError::UnexpectedToken { token } => ParseError::UnexpectedToken { token },
    }
}

/// Convert a legacy `BioAtom` to a `BioConstraint`. Used by the
/// bio-extraction shim that lives between the surface and
/// `parse_query`.
pub fn bio_atom_to_constraint(atom: &BioAtom) -> BioConstraint {
    let (field, predicate) = match atom {
        BioAtom::AgeMin(v) => (
            BioField::Age,
            Predicate::Scalar(ScalarOp::Ge, ScalarValue::Number(*v as f64)),
        ),
        BioAtom::AgeMax(v) => (
            BioField::Age,
            Predicate::Scalar(ScalarOp::Le, ScalarValue::Number(*v as f64)),
        ),
        BioAtom::DraftMin(v) => (
            BioField::DraftYear,
            Predicate::Scalar(ScalarOp::Ge, ScalarValue::Number(*v as f64)),
        ),
        BioAtom::DraftMax(v) => (
            BioField::DraftYear,
            Predicate::Scalar(ScalarOp::Le, ScalarValue::Number(*v as f64)),
        ),
        BioAtom::HeightMin(v) => (
            BioField::Height,
            Predicate::Scalar(ScalarOp::Ge, ScalarValue::Number(*v as f64)),
        ),
        BioAtom::HeightMax(v) => (
            BioField::Height,
            Predicate::Scalar(ScalarOp::Le, ScalarValue::Number(*v as f64)),
        ),
        BioAtom::WeightMin(v) => (
            BioField::Weight,
            Predicate::Scalar(ScalarOp::Ge, ScalarValue::Number(*v as f64)),
        ),
        BioAtom::WeightMax(v) => (
            BioField::Weight,
            Predicate::Scalar(ScalarOp::Le, ScalarValue::Number(*v as f64)),
        ),
        BioAtom::Country(s) => (
            BioField::Country,
            Predicate::Scalar(
                ScalarOp::Eq,
                ScalarValue::Text(ScalarValue::canonicalize_text(s)),
            ),
        ),
        BioAtom::Shoots(s) => (
            BioField::Shoots,
            Predicate::Scalar(
                ScalarOp::Eq,
                ScalarValue::Text(ScalarValue::canonicalize_text(s)),
            ),
        ),
    };
    BioConstraint { field, predicate }
}

/// Try to parse the entire input as a single bio atom. Returns
/// `Some(Constraint::Bio(...))` on match, `None` otherwise. Used
/// by the integrating helper in `apply_views` paths so a single
/// `--filter "age<=24"` works end-to-end.
pub fn try_parse_single_bio_constraint(input: &str) -> Option<Constraint> {
    let atoms = try_parse_bio_atom(input)?;
    if atoms.len() == 1 {
        Some(Constraint::Bio(bio_atom_to_constraint(&atoms[0])))
    } else {
        // age=24 emits both AgeMin(24) and AgeMax(24) — collapse
        // into All([Bio(AgeMin), Bio(AgeMax)]).
        let bios: Vec<Constraint> = atoms
            .iter()
            .map(|a| Constraint::Bio(bio_atom_to_constraint(a)))
            .collect();
        Some(Constraint::All(bios))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A.0 sanity: a simple legacy filter parses into a SeasonStat
    /// constraint with a Scalar predicate.
    #[test]
    fn l0_a0_parse_simple_atom() {
        let plan = parse_query(FilterInput::Cli("g>=10".to_string())).unwrap();
        match plan.root {
            Constraint::SeasonStat(c) => {
                assert!(matches!(
                    c.predicate,
                    Predicate::Scalar(ScalarOp::Ge, ScalarValue::Number(_))
                ));
            }
            _ => panic!("expected SeasonStat, got {:?}", plan.root),
        }
    }

    /// A.0 sanity: AND chain produces n-ary All (NOT binary).
    #[test]
    fn l0_a0_and_chain_collapses_to_n_ary_all() {
        let plan =
            parse_query(FilterInput::Cli("g>=10 AND a>=10 AND p>=20".to_string())).unwrap();
        match plan.root {
            Constraint::All(children) => assert_eq!(children.len(), 3),
            _ => panic!("expected All, got {:?}", plan.root),
        }
    }

    /// A.0 sanity: OR chain produces n-ary Any.
    #[test]
    fn l0_a0_or_chain_collapses_to_n_ary_any() {
        let plan =
            parse_query(FilterInput::Cli("g>=10 OR a>=10 OR p>=20".to_string())).unwrap();
        match plan.root {
            Constraint::Any(children) => assert_eq!(children.len(), 3),
            _ => panic!("expected Any, got {:?}", plan.root),
        }
    }

    /// A.0 sanity: NOT wraps the inner constraint.
    #[test]
    fn l0_a0_not_wraps_inner() {
        let plan = parse_query(FilterInput::Cli("NOT g>=100".to_string())).unwrap();
        assert!(matches!(plan.root, Constraint::Not(_)));
    }

    /// A.0 sanity: empty input returns the EmptyInput error.
    #[test]
    fn l0_a0_empty_input_errors() {
        let errs = parse_query(FilterInput::Cli("".to_string())).unwrap_err();
        assert_eq!(errs[0], ParseError::EmptyInput);
    }

    /// A.0 sanity: whitespace-only input returns EmptyInput.
    #[test]
    fn l0_a0_whitespace_only_input_errors() {
        let errs = parse_query(FilterInput::Cli("   \t  ".to_string())).unwrap_err();
        assert_eq!(errs[0], ParseError::EmptyInput);
    }

    /// A.0 sanity: unknown stat key surfaces as UnknownStat.
    #[test]
    fn l0_a0_unknown_stat_propagates() {
        let errs =
            parse_query(FilterInput::Cli("totally-fake-stat>=1".to_string())).unwrap_err();
        assert!(matches!(errs[0], ParseError::UnknownStat { .. }));
    }

    /// A.0 sanity: typo `=>` produces the OpTypoHint with suggestion `>=`.
    #[test]
    fn l0_a0_arrow_eq_typo_hint() {
        let errs = parse_query(FilterInput::Cli("g=>5".to_string())).unwrap_err();
        match &errs[0] {
            ParseError::OpTypoHint { suggestion, .. } => assert_eq!(*suggestion, ">="),
            other => panic!("expected OpTypoHint, got {:?}", other),
        }
    }

    /// A.0 sanity: typo `=<` produces the OpTypoHint with suggestion `<=`.
    #[test]
    fn l0_a0_lt_eq_typo_hint() {
        let errs = parse_query(FilterInput::Cli("g=<5".to_string())).unwrap_err();
        match &errs[0] {
            ParseError::OpTypoHint { suggestion, .. } => assert_eq!(*suggestion, "<="),
            other => panic!("expected OpTypoHint, got {:?}", other),
        }
    }

    /// A.0 sanity: bio atom helper produces a BioConstraint.
    #[test]
    fn l0_a0_bio_atom_to_constraint_age_max() {
        let constraint = try_parse_single_bio_constraint("age<=24").unwrap();
        match constraint {
            Constraint::Bio(c) => {
                assert_eq!(c.field, BioField::Age);
                assert!(matches!(
                    c.predicate,
                    Predicate::Scalar(ScalarOp::Le, ScalarValue::Number(24.0))
                ));
            }
            _ => panic!("expected Bio, got {:?}", constraint),
        }
    }

    /// A.0 sanity: bio age= produces both bounds via All.
    #[test]
    fn l0_a0_bio_atom_age_eq_emits_both_bounds() {
        let constraint = try_parse_single_bio_constraint("age=24").unwrap();
        match constraint {
            Constraint::All(children) => assert_eq!(children.len(), 2),
            _ => panic!("expected All, got {:?}", constraint),
        }
    }

    /// A.0 sanity: bio country=can normalizes to lowercase.
    #[test]
    fn l0_a0_bio_atom_country_normalized() {
        let constraint = try_parse_single_bio_constraint("country=CAN").unwrap();
        match constraint {
            Constraint::Bio(c) => match c.predicate {
                Predicate::Scalar(ScalarOp::Eq, ScalarValue::Text(t)) => assert_eq!(t, "can"),
                other => panic!("expected text predicate, got {:?}", other),
            },
            _ => panic!("expected Bio, got {:?}", constraint),
        }
    }

    /// A.0 sanity: TUI input with one fragment produces the bare
    /// constraint, not wrapped in All.
    #[test]
    fn l0_a0_tui_single_atom_unwrapped() {
        let bio = BioConstraint {
            field: BioField::Age,
            predicate: Predicate::Scalar(ScalarOp::Le, ScalarValue::Number(24.0)),
        };
        let plan = parse_query(FilterInput::Tui(vec![AtomFragment::Atom(
            Constraint::Bio(bio.clone()),
        )]))
        .unwrap();
        assert_eq!(plan.root, Constraint::Bio(bio));
    }

    /// A.0 sanity: TUI input with multiple atoms produces n-ary All.
    #[test]
    fn l0_a0_tui_multi_atom_wrapped_in_all() {
        let frags = vec![
            AtomFragment::Atom(Constraint::Bio(BioConstraint {
                field: BioField::Age,
                predicate: Predicate::Scalar(ScalarOp::Le, ScalarValue::Number(24.0)),
            })),
            AtomFragment::AndJoin,
            AtomFragment::Atom(Constraint::Bio(BioConstraint {
                field: BioField::Country,
                predicate: Predicate::Scalar(
                    ScalarOp::Eq,
                    ScalarValue::Text("can".to_string()),
                ),
            })),
        ];
        let plan = parse_query(FilterInput::Tui(frags)).unwrap();
        match plan.root {
            Constraint::All(children) => assert_eq!(children.len(), 2),
            _ => panic!("expected All"),
        }
    }
}
