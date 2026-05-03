//! Description sanitization — strips control characters and normalizes
//! whitespace before persistence so downstream UI (ratatui rows, CSV
//! escaping, JSON consumers) doesn't have to defend against stray
//! `\n` / `\t` / NUL etc. that ESPN's free-form prose might contain.
//!
//! The contract:
//! - All control chars (Unicode category Cc) are removed.
//! - All whitespace runs collapse to a single space.
//! - Leading and trailing whitespace are trimmed.
//! - Letters, digits, punctuation, and emoji are preserved.
//! - The function is idempotent: `sanitize(sanitize(s)) == sanitize(s)`.

/// Sanitize a transaction description for safe persistence and rendering.
///
/// Strategy: drop non-whitespace control chars entirely, but treat
/// whitespace controls (`\t`, `\n`, `\r`, etc.) as word separators so
/// `"line1\nline2"` becomes `"line1 line2"` rather than `"line1line2"`.
/// `split_whitespace` then collapses runs and trims leading/trailing.
pub fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .filter(|c| !c.is_control())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn l0_sanitize_strips_newline() {
        assert_eq!(sanitize("line1\nline2"), "line1 line2");
    }

    #[test]
    fn l0_sanitize_strips_tab() {
        assert_eq!(sanitize("col1\tcol2"), "col1 col2");
    }

    #[test]
    fn l0_sanitize_strips_carriage_return() {
        assert_eq!(sanitize("a\rb"), "a b");
    }

    #[test]
    fn l0_sanitize_collapses_whitespace_runs() {
        assert_eq!(
            sanitize("Signed   F   Connor   Bedard"),
            "Signed F Connor Bedard"
        );
    }

    #[test]
    fn l0_sanitize_trims_leading_trailing() {
        assert_eq!(sanitize("  hello  "), "hello");
    }

    #[test]
    fn l0_sanitize_preserves_punctuation_and_apostrophes() {
        // Player names with apostrophes (D'Pinto, O'Reilly) and hyphens
        // (Pierre-Luc) must survive sanitization.
        let s = "Recalled F D'Pinto and Pierre-Luc Dubois.";
        assert_eq!(sanitize(s), "Recalled F D'Pinto and Pierre-Luc Dubois.");
    }

    #[test]
    fn l0_sanitize_preserves_unicode_letters() {
        // Hörnqvist, Slafkovský — accented characters must NOT be stripped.
        // (Diacritic stripping is a separate concern handled in name::normalize_name.)
        assert_eq!(
            sanitize("Signed F Patric Hörnqvist"),
            "Signed F Patric Hörnqvist"
        );
    }

    #[test]
    fn l0_sanitize_strips_null_byte() {
        assert_eq!(sanitize("evil\0prefix"), "evilprefix");
    }

    #[test]
    fn l0_sanitize_empty_string_safe() {
        assert_eq!(sanitize(""), "");
    }

    #[test]
    fn l0_sanitize_only_whitespace_returns_empty() {
        assert_eq!(sanitize("   \n\t  "), "");
    }

    proptest! {
        #[test]
        fn l0_sanitize_idempotent(s in "\\PC{0,200}") {
            // Idempotence: running sanitize on already-sanitized output
            // must produce the same string.
            let once = sanitize(&s);
            let twice = sanitize(&once);
            prop_assert_eq!(once, twice);
        }

        #[test]
        fn l0_sanitize_preserves_alphanumeric(
            s in "[a-zA-Z0-9 ]{1,80}".prop_filter(
                "must contain at least one alphanumeric",
                |s| s.chars().any(|c| c.is_alphanumeric())
            )
        ) {
            // Any alphanumeric input survives sanitization (whitespace may
            // be normalized, but no chars are dropped).
            let cleaned = sanitize(&s);
            for c in s.chars().filter(|c| c.is_alphanumeric()) {
                prop_assert!(cleaned.contains(c),
                    "alphanumeric '{}' must survive sanitization of '{}', got '{}'",
                    c, s, cleaned);
            }
        }
    }
}
