//! Phase Art Ross A.0 — `FilterInput` enum.
//!
//! Each surface (CLI / web / TUI) owns its own decode boundary
//! before handing input to `parse_query`. The TUI builds atoms
//! incrementally via the filter overlay and skips the tokenizer
//! entirely; CLI and web pass already-decoded strings.

use crate::plan::Constraint;

/// Pre-decoded filter input. Each variant is what the surface
/// hands to `parse_query` after its own decode pass.
#[derive(Debug, Clone, PartialEq)]
pub enum FilterInput {
    /// CLI: clap has already shell-decoded the string. Multiple
    /// `--filter` flags are joined into one input via implicit AND
    /// at a higher layer (see `from_cli_filters`).
    Cli(String),
    /// Web: the surface URL-decodes form values before constructing
    /// this variant. (axum's `Query<...>` extractor handles the
    /// URL decode automatically.)
    Form(String),
    /// TUI: the user builds atoms incrementally via the filter
    /// overlay; the surface composes a `Vec<AtomFragment>` directly
    /// without round-tripping through a string.
    Tui(Vec<AtomFragment>),
}

/// A typed atom fragment built directly by the TUI overlay. Each
/// fragment encodes one user widget interaction (e.g. "selected
/// position dropdown = C", "set age max to 24").
#[derive(Debug, Clone, PartialEq)]
pub enum AtomFragment {
    /// A pre-built constraint atom — used when the TUI overlay
    /// constructs a constraint directly without going through a
    /// string form.
    Atom(Constraint),
    /// An implicit-AND boundary between adjacent atoms.
    AndJoin,
    /// An implicit-OR boundary.
    OrJoin,
    /// Group open (paren).
    GroupOpen,
    /// Group close.
    GroupClose,
}

impl FilterInput {
    /// Convenience constructor: take a slice of CLI `--filter`
    /// strings and AND-join them into one `Cli` variant. Empty
    /// slices yield `Cli("")` which `parse_query` rejects with
    /// `EmptyInput`.
    pub fn from_cli_filters(filters: &[String]) -> Self {
        if filters.is_empty() {
            return FilterInput::Cli(String::new());
        }
        if filters.len() == 1 {
            return FilterInput::Cli(filters[0].clone());
        }
        // Multiple --filter flags AND together (Wave 11 #199).
        let joined = filters
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| format!("({s})"))
            .collect::<Vec<_>>()
            .join(" AND ");
        FilterInput::Cli(joined)
    }

    /// Borrow the input as a `&str` for the string-path parser.
    /// Returns `None` for the `Tui` variant (which skips the
    /// tokenizer).
    pub fn as_str(&self) -> Option<&str> {
        match self {
            FilterInput::Cli(s) | FilterInput::Form(s) => Some(s.as_str()),
            FilterInput::Tui(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_filter_input_empty_slice_yields_empty_cli() {
        let input = FilterInput::from_cli_filters(&[]);
        assert_eq!(input, FilterInput::Cli(String::new()));
    }

    #[test]
    fn l0_filter_input_single_filter_passes_through() {
        let input = FilterInput::from_cli_filters(&["g>=10".to_string()]);
        assert_eq!(input, FilterInput::Cli("g>=10".to_string()));
    }

    #[test]
    fn l0_filter_input_multiple_filters_anded_with_parens() {
        let input = FilterInput::from_cli_filters(&[
            "g>=10".to_string(),
            "a>=10".to_string(),
        ]);
        assert_eq!(
            input,
            FilterInput::Cli("(g>=10) AND (a>=10)".to_string())
        );
    }

    #[test]
    fn l0_filter_input_strips_empty_filters_from_join() {
        let input = FilterInput::from_cli_filters(&[
            "g>=10".to_string(),
            "  ".to_string(),
            "a>=10".to_string(),
        ]);
        assert_eq!(
            input,
            FilterInput::Cli("(g>=10) AND (a>=10)".to_string())
        );
    }

    #[test]
    fn l0_filter_input_as_str_for_cli_and_form() {
        assert_eq!(FilterInput::Cli("x".into()).as_str(), Some("x"));
        assert_eq!(FilterInput::Form("y".into()).as_str(), Some("y"));
        assert_eq!(FilterInput::Tui(vec![]).as_str(), None);
    }
}
