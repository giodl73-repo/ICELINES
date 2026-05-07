//! Phase Art Ross A.0 — `ParseError` types.
//!
//! `parse_query` returns `Result<QueryPlan, Vec<ParseError>>` so
//! a 5-atom filter with 3 errors surfaces all 3 in one round-trip.
//! Each error carries `span` info pointing at the offending atom.

use thiserror::Error;

/// A single parse error with span info. The parser collects all
/// errors into a `Vec<ParseError>` rather than bailing on the
/// first; the user sees every problem in one shot.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ParseError {
    #[error("filter is empty")]
    EmptyInput,

    #[error("filter stat-key is empty in atom {atom:?}")]
    EmptyStatKey { atom: String },

    #[error(
        "filter {atom:?} has no op — expected one of `>=`, `<=`, `==`, `=`, `<`, `>`, `!=`"
    )]
    MissingOp { atom: String },

    #[error("filter {atom:?} has multiple ops — expected exactly one")]
    MultipleOps { atom: String },

    #[error("unknown stat key {key:?}")]
    UnknownStat { key: String },

    #[error("filter {atom:?} number {token:?} couldn't be parsed")]
    BadNumber { atom: String, token: String },

    #[error("filter {atom:?} number {token:?} is not finite")]
    NotFinite { atom: String, token: String },

    #[error("unclosed paren in filter")]
    UnclosedParen,

    #[error("unexpected `)` in filter")]
    UnexpectedRParen,

    #[error("filter ends mid-parse — expected another atom")]
    UnexpectedEnd,

    #[error("unexpected token {token:?} in filter")]
    UnexpectedToken { token: String },

    /// Wave 11 / scout review item — empty `IN ()` is a user error.
    #[error("empty set in {atom:?} — `IN ()` is not valid")]
    EmptySet { atom: String },

    /// Phase Art Ross — atoms that map to a future sub-phase variant.
    /// Carries `ships_in` (e.g. "A.2") so the error message tells
    /// the user when the feature lands.
    #[error("feature {atom:?} not yet supported (ships in {ships_in})")]
    FeatureNotYet {
        atom: String,
        ships_in: &'static str,
    },

    /// `Predicate` shape doesn't match the field type — e.g. `LIKE`
    /// on a numeric field, or `BETWEEN` on a string field, or `IN`
    /// on a single-value position atom.
    #[error("predicate shape incompatible with field {field}: {detail}")]
    IncompatiblePredicate { field: String, detail: String },

    /// User typed `=>` (likely meant `>=`) or `=<` (likely meant `<=`).
    /// Hint is folded into the Display impl by the caller; this
    /// variant exists so Wave 11's typo-hint test can match on it.
    #[error("filter {atom:?} has multiple ops — did you mean `{suggestion}`?")]
    OpTypoHint {
        atom: String,
        suggestion: &'static str,
    },
}

impl ParseError {
    /// Convenience for callers building one-error result lists.
    pub fn into_vec(self) -> Vec<ParseError> {
        vec![self]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_parse_error_display_messages() {
        assert!(format!("{}", ParseError::EmptyInput).contains("empty"));
        assert!(format!(
            "{}",
            ParseError::UnknownStat { key: "fake".into() }
        )
        .contains("fake"));
        assert!(format!(
            "{}",
            ParseError::FeatureNotYet {
                atom: "g.last10g>=5".into(),
                ships_in: "A.2"
            }
        )
        .contains("A.2"));
    }

    #[test]
    fn l0_parse_error_into_vec() {
        let v = ParseError::EmptyInput.into_vec();
        assert_eq!(v.len(), 1);
    }
}
