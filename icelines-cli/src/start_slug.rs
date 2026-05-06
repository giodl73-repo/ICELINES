//! Phase Lady Byng (LB.1) — `--start <slug>` parsing.
//!
//! Single source of truth for the slug → Screen mapping. The same
//! `SLUG_TABLE` drives:
//! - `parse_start_slug` (used by `Commands::Tui` dispatch in `main.rs`)
//! - The error formatter's "valid slugs" list
//! - `--help` long_about for `icelines tui` (rendered from `canonical_slugs`)
//! - A drift fence in the docs tests that asserts every canonical slug
//!   appears in COMMANDS.md
//!
//! Stability tier:
//! - **Canonical** slugs are part of the public CLI contract. Removing one
//!   is a breaking change requiring a one-release deprecation cycle.
//! - **Alias** slugs are convenience renames. Listed in `--help` but
//!   hidden from the error-message suggestions (fewer choices for the
//!   user to digest).
//!
//! LB.3 will extend this module with the parameterized `<slug>:<arg>`
//! forms (player:NAME, team:ABBR, goalie:NAME, comps:NAME). For LB.1
//! only the 8 nav-tab surfaces are wired.

use crate::tui::app::Screen;

/// Stability tier for a slug. Canonical slugs are the public contract;
/// aliases can be added freely but removal still requires WARN cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stability {
    Canonical,
    Alias,
}

/// Single source of truth for the slug → Screen mapping. Append-only;
/// removing a Canonical entry requires a one-release deprecation cycle.
pub const SLUG_TABLE: &[(&str, ScreenSpec, Stability)] = &[
    ("league", ScreenSpec::Home, Stability::Canonical),
    ("depth", ScreenSpec::Depth, Stability::Canonical),
    ("stats", ScreenSpec::Queries, Stability::Canonical),
    ("queries", ScreenSpec::Queries, Stability::Alias),
    ("goalies", ScreenSpec::Goalies, Stability::Canonical),
    ("scores", ScreenSpec::Tonight, Stability::Canonical),
    ("tonight", ScreenSpec::Tonight, Stability::Alias),
    ("schedule", ScreenSpec::Schedule, Stability::Canonical),
    (
        "transactions",
        ScreenSpec::Transactions,
        Stability::Canonical,
    ),
    ("moves", ScreenSpec::Transactions, Stability::Alias),
    ("playoffs", ScreenSpec::Playoffs, Stability::Canonical),
];

/// LB.1 — nav-tab placeholder. LB.3 will extend with parameterized
/// variants (PlayerById/Team/GoalieById/CompsById) carrying a deferred
/// name-or-pid needle for resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenSpec {
    Home,
    Depth,
    Queries,
    Goalies,
    Tonight,
    Schedule,
    Transactions,
    Playoffs,
}

impl ScreenSpec {
    /// Resolve to a runtime `Screen`. For nav-tab variants this is a
    /// pure mapping; LB.3 parameterized variants will call into
    /// `resolve_player_id_by_name` here.
    pub fn into_screen(self) -> Screen {
        match self {
            ScreenSpec::Home => Screen::Home,
            ScreenSpec::Depth => Screen::Depth,
            ScreenSpec::Queries => Screen::Queries,
            ScreenSpec::Goalies => Screen::Goalies,
            ScreenSpec::Tonight => Screen::Tonight,
            ScreenSpec::Schedule => Screen::Schedule,
            ScreenSpec::Transactions => Screen::Transactions,
            ScreenSpec::Playoffs => Screen::Playoffs,
        }
    }
}

/// Error returned by `parse_start_slug`. Implements `Display` so it
/// can flow through `anyhow::Error` to stderr without further wrapping.
#[derive(Debug, thiserror::Error)]
pub enum StartSlugError {
    #[error(
        "unknown surface '{input}'. Valid: {}{}",
        valid.join(", "),
        suggestion.map(|s| format!(" — did you mean '{s}'?")).unwrap_or_default()
    )]
    Unknown {
        input: String,
        valid: Vec<&'static str>,
        suggestion: Option<&'static str>,
    },
    #[error("surface slug cannot be empty or whitespace")]
    Empty,
}

/// Parse a slug string into a `ScreenSpec`. Case-insensitive on the
/// slug key. Whitespace is trimmed before lookup. Empty / whitespace
/// input is rejected with `StartSlugError::Empty`.
///
/// Aliases resolve to the same `ScreenSpec` as their canonical form;
/// the error message's "valid" list contains canonical slugs only.
pub fn parse_start_slug(s: &str) -> Result<ScreenSpec, StartSlugError> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(StartSlugError::Empty);
    }
    let lower = trimmed.to_ascii_lowercase();
    for (slug, spec, _) in SLUG_TABLE {
        if *slug == lower.as_str() {
            return Ok(*spec);
        }
    }
    Err(StartSlugError::Unknown {
        input: trimmed.to_owned(),
        valid: canonical_slugs(),
        suggestion: suggestion_for(&lower),
    })
}

/// Canonical slugs in declaration order. Stable for `--help` rendering
/// and the COMMANDS.md drift fence.
pub fn canonical_slugs() -> Vec<&'static str> {
    SLUG_TABLE
        .iter()
        .filter(|(_, _, s)| *s == Stability::Canonical)
        .map(|(slug, _, _)| *slug)
        .collect()
}

/// Levenshtein-1 (edit distance ≤ 1) suggestion against canonical slugs.
/// Catches `goalie` → `goalies`, `score` → `scores`, etc. without
/// pulling in a fuzzy-match dependency.
fn suggestion_for(input: &str) -> Option<&'static str> {
    for (slug, _, stab) in SLUG_TABLE {
        if *stab != Stability::Canonical {
            continue;
        }
        if edit_distance_at_most_one(input, slug) {
            return Some(slug);
        }
    }
    None
}

/// Return true iff `a` and `b` differ by at most 1 character (insert,
/// delete, or substitute). Single-pass, no allocation. Adequate for
/// short slug strings.
fn edit_distance_at_most_one(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let (sa, sb) = (a.as_bytes(), b.as_bytes());
    if sa.len().abs_diff(sb.len()) > 1 {
        return false;
    }
    let (short, long) = if sa.len() < sb.len() {
        (sa, sb)
    } else {
        (sb, sa)
    };
    let mut i = 0usize;
    let mut j = 0usize;
    let mut diffs = 0u8;
    while i < short.len() && j < long.len() {
        if short[i] == long[j] {
            i += 1;
            j += 1;
        } else {
            diffs += 1;
            if diffs > 1 {
                return false;
            }
            if short.len() == long.len() {
                i += 1;
                j += 1;
            } else {
                j += 1;
            }
        }
    }
    if j < long.len() {
        diffs += 1;
    }
    diffs <= 1
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Canonical slug → ScreenSpec mapping ──────────────────────────

    /// LB.1 / l0_canonical_slugs_round_trip
    /// — Each canonical slug must resolve to its declared ScreenSpec.
    ///   Locks the public CLI contract. Renaming a slug fails this
    ///   test until COMMANDS.md + tests are updated in lockstep.
    #[test]
    fn l0_canonical_slugs_round_trip() {
        let cases = [
            ("league", ScreenSpec::Home),
            ("depth", ScreenSpec::Depth),
            ("stats", ScreenSpec::Queries),
            ("goalies", ScreenSpec::Goalies),
            ("scores", ScreenSpec::Tonight),
            ("schedule", ScreenSpec::Schedule),
            ("transactions", ScreenSpec::Transactions),
            ("playoffs", ScreenSpec::Playoffs),
        ];
        for (slug, expected) in cases {
            assert_eq!(parse_start_slug(slug).unwrap(), expected, "slug={slug}");
        }
    }

    /// LB.1 / l0_aliases_resolve_to_canonical
    /// — Aliases produce the same ScreenSpec as their canonical form.
    #[test]
    fn l0_aliases_resolve_to_canonical() {
        assert_eq!(
            parse_start_slug("queries").unwrap(),
            parse_start_slug("stats").unwrap()
        );
        assert_eq!(
            parse_start_slug("tonight").unwrap(),
            parse_start_slug("scores").unwrap()
        );
        assert_eq!(
            parse_start_slug("moves").unwrap(),
            parse_start_slug("transactions").unwrap()
        );
    }

    /// LB.1 / l0_case_insensitive
    /// — `GOALIES`, `Goalies`, `goalies` all resolve identically.
    #[test]
    fn l0_case_insensitive() {
        let g = parse_start_slug("goalies").unwrap();
        assert_eq!(parse_start_slug("GOALIES").unwrap(), g);
        assert_eq!(parse_start_slug("Goalies").unwrap(), g);
        assert_eq!(parse_start_slug("gOaLiEs").unwrap(), g);
    }

    /// LB.1 / l0_whitespace_trimmed
    /// — Leading/trailing whitespace stripped before lookup.
    #[test]
    fn l0_whitespace_trimmed() {
        let g = parse_start_slug("goalies").unwrap();
        assert_eq!(parse_start_slug("  goalies").unwrap(), g);
        assert_eq!(parse_start_slug("goalies  ").unwrap(), g);
        assert_eq!(parse_start_slug("\tgoalies\n").unwrap(), g);
    }

    /// LB.1 / l0_empty_input_rejected
    /// — Empty or whitespace-only input errors with `Empty`.
    #[test]
    fn l0_empty_input_rejected() {
        assert!(matches!(parse_start_slug(""), Err(StartSlugError::Empty)));
        assert!(matches!(
            parse_start_slug("   "),
            Err(StartSlugError::Empty)
        ));
        assert!(matches!(
            parse_start_slug("\t\n"),
            Err(StartSlugError::Empty)
        ));
    }

    /// LB.1 / l0_unknown_slug_lists_canonical_only
    /// — Error message lists canonical slugs only (no alias clutter).
    #[test]
    fn l0_unknown_slug_lists_canonical_only() {
        let err = parse_start_slug("zzz").unwrap_err();
        match err {
            StartSlugError::Unknown { valid, .. } => {
                assert!(valid.contains(&"goalies"));
                assert!(valid.contains(&"scores"));
                // Aliases hidden:
                assert!(!valid.contains(&"queries"));
                assert!(!valid.contains(&"tonight"));
                assert!(!valid.contains(&"moves"));
            }
            _ => panic!("expected Unknown variant"),
        }
    }

    /// LB.1 / l0_singular_typo_suggests_plural
    /// — `goalie` (typo for `goalies`) returns suggestion in the error.
    #[test]
    fn l0_singular_typo_suggests_plural() {
        let err = parse_start_slug("goalie").unwrap_err();
        match err {
            StartSlugError::Unknown { suggestion, .. } => {
                assert_eq!(suggestion, Some("goalies"));
            }
            _ => panic!("expected Unknown variant"),
        }
    }

    /// LB.1 / l0_score_typo_suggests_scores
    #[test]
    fn l0_score_typo_suggests_scores() {
        let err = parse_start_slug("score").unwrap_err();
        match err {
            StartSlugError::Unknown { suggestion, .. } => {
                assert_eq!(suggestion, Some("scores"));
            }
            _ => panic!("expected Unknown variant"),
        }
    }

    /// LB.1 / l0_far_typo_no_suggestion
    /// — Garbage input shouldn't produce a misleading suggestion.
    #[test]
    fn l0_far_typo_no_suggestion() {
        let err = parse_start_slug("xyzzy").unwrap_err();
        match err {
            StartSlugError::Unknown { suggestion, .. } => {
                assert_eq!(suggestion, None);
            }
            _ => panic!("expected Unknown variant"),
        }
    }

    /// LB.1 / l0_unknown_error_renders_human_readable
    /// — The Display impl produces a clean message for users.
    #[test]
    fn l0_unknown_error_renders_human_readable() {
        let err = parse_start_slug("zzz").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("unknown surface 'zzz'"));
        assert!(msg.contains("Valid:"));
        assert!(msg.contains("goalies"));
    }

    /// LB.1 / l0_typo_error_includes_suggestion
    #[test]
    fn l0_typo_error_includes_suggestion() {
        let err = parse_start_slug("goalie").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("did you mean 'goalies'?"), "msg={msg}");
    }

    // ── canonical_slugs helper ──────────────────────────────────────

    /// LB.1 / l0_canonical_slugs_count
    /// — 8 nav-tab surfaces; if this number changes, COMMANDS.md needs
    ///   to update too. Drift fence.
    #[test]
    fn l0_canonical_slugs_count() {
        assert_eq!(canonical_slugs().len(), 8);
    }

    /// LB.1 / l0_canonical_slugs_in_declaration_order
    /// — Preserves declaration order for stable `--help` rendering.
    #[test]
    fn l0_canonical_slugs_in_declaration_order() {
        let expected = vec![
            "league",
            "depth",
            "stats",
            "goalies",
            "scores",
            "schedule",
            "transactions",
            "playoffs",
        ];
        assert_eq!(canonical_slugs(), expected);
    }

    // ── ScreenSpec → Screen ─────────────────────────────────────────

    /// LB.1 / l0_screen_spec_into_screen_covers_every_variant
    /// — Each variant maps to a real Screen value. If a new variant is
    ///   added without updating into_screen, this fails to compile.
    #[test]
    fn l0_screen_spec_into_screen_covers_every_variant() {
        let _ = ScreenSpec::Home.into_screen();
        let _ = ScreenSpec::Depth.into_screen();
        let _ = ScreenSpec::Queries.into_screen();
        let _ = ScreenSpec::Goalies.into_screen();
        let _ = ScreenSpec::Tonight.into_screen();
        let _ = ScreenSpec::Schedule.into_screen();
        let _ = ScreenSpec::Transactions.into_screen();
        let _ = ScreenSpec::Playoffs.into_screen();
    }

    // ── edit_distance_at_most_one helper ───────────────────────────

    /// LB.1 / l0_edit_distance_basics
    #[test]
    fn l0_edit_distance_basics() {
        assert!(edit_distance_at_most_one("goalies", "goalies")); // identical
        assert!(edit_distance_at_most_one("goalie", "goalies")); // 1 insert
        assert!(edit_distance_at_most_one("goalies", "goalie")); // 1 delete
        assert!(edit_distance_at_most_one("goolies", "goalies")); // 1 sub
        assert!(!edit_distance_at_most_one("xyzzy", "goalies")); // far apart
        assert!(!edit_distance_at_most_one("ab", "abcd")); // 2 inserts
    }

    /// LB.1 / l0_edit_distance_empty_inputs
    #[test]
    fn l0_edit_distance_empty_inputs() {
        assert!(edit_distance_at_most_one("", ""));
        assert!(edit_distance_at_most_one("a", ""));
        assert!(edit_distance_at_most_one("", "a"));
        assert!(!edit_distance_at_most_one("ab", ""));
    }
}
