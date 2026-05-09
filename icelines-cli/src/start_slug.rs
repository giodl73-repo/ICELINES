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
use icelines_core::identity::PlayerId;

/// Stability tier for a slug. Canonical slugs are the public contract;
/// aliases can be added freely but removal still requires WARN cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stability {
    Canonical,
    Alias,
}

/// Nav-tab slug table. The 8 canonical names + their aliases. Drives
/// `parse_start_slug` for nav surfaces. Parameterized slugs
/// (`player:NAME`, `team:ABBR`, ...) are matched separately because
/// they carry an opaque arg.
pub const SLUG_TABLE: &[(&str, NavSpec, Stability)] = &[
    ("league", NavSpec::Home, Stability::Canonical),
    ("depth", NavSpec::Depth, Stability::Canonical),
    ("stats", NavSpec::Queries, Stability::Canonical),
    ("queries", NavSpec::Queries, Stability::Alias),
    ("goalies", NavSpec::Goalies, Stability::Canonical),
    ("scores", NavSpec::Tonight, Stability::Canonical),
    ("tonight", NavSpec::Tonight, Stability::Alias),
    ("schedule", NavSpec::Schedule, Stability::Canonical),
    ("transactions", NavSpec::Transactions, Stability::Canonical),
    ("moves", NavSpec::Transactions, Stability::Alias),
    ("playoffs", NavSpec::Playoffs, Stability::Canonical),
    ("poach", NavSpec::Poach, Stability::Canonical),
];

/// Nav-tab variant — pure mapping, no parameter, no resolution needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavSpec {
    Home,
    Depth,
    Queries,
    Goalies,
    Tonight,
    Schedule,
    Transactions,
    Playoffs,
    Poach,
}

impl NavSpec {
    pub fn into_screen(self) -> Screen {
        match self {
            NavSpec::Home => Screen::Home,
            NavSpec::Depth => Screen::Depth,
            NavSpec::Queries => Screen::Queries,
            NavSpec::Goalies => Screen::Goalies,
            NavSpec::Tonight => Screen::Tonight,
            NavSpec::Schedule => Screen::Schedule,
            NavSpec::Transactions => Screen::Transactions,
            NavSpec::Playoffs => Screen::Playoffs,
            NavSpec::Poach => Screen::Poach,
        }
    }
}

/// Opaque arg — either an explicit pid (digits-only) or a name needle.
/// LB.3 — populated at parse time, resolved later via `into_screen`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Needle {
    Pid(u32),
    Name(String),
}

impl Needle {
    /// Build from an arg string. Empty / whitespace-only rejected by
    /// the caller before this is invoked.
    pub fn from_arg(arg: &str) -> Self {
        let trimmed = arg.trim();
        if !trimmed.is_empty() && trimmed.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(n) = trimmed.parse::<u32>() {
                return Needle::Pid(n);
            }
        }
        Needle::Name(trimmed.to_owned())
    }
}

/// Full ScreenSpec covering nav tabs + parameterized drill-downs. The
/// CLI dispatch resolves this to a runtime `Screen` AFTER name/abbrev
/// resolution, which can fail (no match / multi-match / bad abbrev).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScreenSpec {
    Nav(NavSpec),
    /// LB.3 — `player:NAME` or `player:PID`. Resolution lookups skater
    /// bios first, then goalie bios.
    Player(Needle),
    /// LB.3 — `team:ABBR`. Validated against the 32-team set.
    Team(String),
    /// LB.3 — `goalie:NAME` or `goalie:PID`. Same lookup as Player but
    /// the resolved Screen is `GoalieDetailById` rather than `PlayerById`.
    Goalie(Needle),
    /// LB.3 — `comps:NAME` or `comps:PID`. Resolves to `CompsById`.
    Comps(Needle),
}

impl ScreenSpec {
    /// Convenience constructor for a nav-tab spec.
    pub fn nav(nav: NavSpec) -> Self {
        ScreenSpec::Nav(nav)
    }

    /// Resolve to a runtime `Screen`. For nav-tabs this is a pure
    /// mapping. For parameterized variants, this triggers name/abbrev
    /// resolution which can fail with `ResolveError`.
    pub fn into_screen(self) -> Result<Screen, ResolveError> {
        match self {
            ScreenSpec::Nav(nav) => Ok(nav.into_screen()),
            ScreenSpec::Player(needle) => resolve_player_screen(needle, /*goalie=*/ false),
            ScreenSpec::Goalie(needle) => resolve_player_screen(needle, /*goalie=*/ true),
            ScreenSpec::Comps(needle) => {
                let pid = resolve_pid(needle)?;
                Ok(Screen::CompsById(pid))
            }
            ScreenSpec::Team(abbrev) => {
                let upper = abbrev.trim().to_ascii_uppercase();
                if upper.is_empty() {
                    return Err(ResolveError::EmptyArg { slug: "team" });
                }
                if !icelines_fetch::teams::ALL_NHL_TEAMS.contains(&upper.as_str()) {
                    return Err(ResolveError::UnknownTeam {
                        input: abbrev,
                        valid: icelines_fetch::teams::ALL_NHL_TEAMS.to_vec(),
                    });
                }
                Ok(Screen::Team(upper))
            }
        }
    }
}

/// Shared helper for `player:` / `goalie:` resolution. Both use
/// `find_player_candidates` which walks skater bios then goalie bios;
/// the only difference is which Screen variant the resolved pid lands
/// on.
fn resolve_player_screen(needle: Needle, goalie_card: bool) -> Result<Screen, ResolveError> {
    let pid = resolve_pid(needle)?;
    Ok(if goalie_card {
        Screen::GoalieDetailById(pid)
    } else {
        Screen::PlayerById(pid)
    })
}

/// Resolve a Needle to a PlayerId. Pid bypasses lookup; Name routes
/// through the candidates search and branches on count.
fn resolve_pid(needle: Needle) -> Result<PlayerId, ResolveError> {
    match needle {
        Needle::Pid(n) => Ok(PlayerId(n)),
        Needle::Name(name) => {
            if name.is_empty() {
                return Err(ResolveError::EmptyArg { slug: "player" });
            }
            let mut candidates = icelines_fetch::stats_loader::find_player_candidates(&name);
            match candidates.len() {
                0 => Err(ResolveError::NoMatch { input: name }),
                1 => Ok(PlayerId(candidates.remove(0).pid)),
                _ => Err(ResolveError::Ambiguous {
                    input: name,
                    candidates,
                }),
            }
        }
    }
}

/// Post-LP review fix #11 — maximum candidates listed in an Ambiguous
/// error. A needle like "a" can match hundreds; flood the terminal
/// and the user can't see the prompt to retry. 15 is enough rows to
/// disambiguate any plausible last-name collision while staying inside
/// a single screen on a typical 30-row terminal.
const AMBIGUOUS_LIST_CAP: usize = 15;

/// Errors that surface during ScreenSpec → Screen resolution. These
/// are distinct from parse-time errors (`StartSlugError`) because
/// they only fire after a syntactically-valid slug has been parsed.
#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("no player matching '{input}' in bundled bios — try the full name or `player:<pid>`")]
    NoMatch { input: String },

    /// Sebastian Aho problem — multiple players match. List candidates;
    /// user re-runs with a more specific name or pid. Post-LP review
    /// fix #11: cap the listing at 15 (most-recent first per the
    /// `find_player_candidates` sort) so a needle like "a" doesn't
    /// flood the terminal.
    #[error(
        "ambiguous name '{input}' — pick one (showing {} of {}):\n{}{}",
        candidates.len().min(AMBIGUOUS_LIST_CAP),
        candidates.len(),
        candidates.iter().take(AMBIGUOUS_LIST_CAP).map(|c| {
            let team = c.last_team.as_deref().unwrap_or("?");
            let season = c.last_season.map(format_season_id).unwrap_or_else(|| "?".into());
            let role = if c.is_goalie { "goalie" } else { "skater" };
            format!("  player:{:<10} {} ({} · {} · {role})", c.pid, c.full_name, team, season)
        }).collect::<Vec<_>>().join("\n"),
        if candidates.len() > AMBIGUOUS_LIST_CAP {
            format!("\n  ...and {} more (use a more specific name or `player:<pid>`)", candidates.len() - AMBIGUOUS_LIST_CAP)
        } else {
            String::new()
        }
    )]
    Ambiguous {
        input: String,
        candidates: Vec<icelines_fetch::stats_loader::PlayerCandidate>,
    },

    #[error("unknown team abbreviation '{input}'. Valid: {}", valid.join(", "))]
    UnknownTeam {
        input: String,
        valid: Vec<&'static str>,
    },

    #[error("'{slug}:' requires an argument (e.g. `{slug}:Bedard` or `{slug}:8478402`)")]
    EmptyArg { slug: &'static str },
}

/// Render a YYYYZZZZ season id as "YYYY-YY". 20242025 → "2024-25".
fn format_season_id(id: u32) -> String {
    let start = id / 10_000;
    let end = id % 10_000;
    format!("{start}-{:02}", end % 100)
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

    /// LB.3 — `player:` / `team:` etc. with empty or whitespace-only arg.
    /// Rejected at parse time to avoid `normalize_name` stripping to ""
    /// and matching the first bio in the bundle.
    #[error("'{slug}:' requires an argument (e.g. `{slug}:Bedard`)")]
    EmptyParameterizedArg { slug: String },

    /// LB.3 — `<unknown>:<arg>` — slug isn't one of player/team/goalie/comps.
    #[error(
        "unknown parameterized slug '{slug}:'. Valid: {}:NAME-or-PID",
        valid.join(":NAME-or-PID, ")
    )]
    UnknownParameterized {
        slug: String,
        valid: Vec<&'static str>,
    },
}

/// Parse a slug string into a `ScreenSpec`. Case-insensitive on the
/// slug key. Whitespace is trimmed before lookup. Empty / whitespace
/// input is rejected with `StartSlugError::Empty`.
///
/// Two grammars supported:
/// - **Bare slug** (`goalies`, `scores`, ...) → nav-tab `ScreenSpec::Nav`.
/// - **Parameterized** (`player:NAME`, `team:ABBR`, ...) →
///   `ScreenSpec::{Player,Team,Goalie,Comps}`. Exactly one `:` separates
///   slug from arg; arg is opaque to the parser. Empty/whitespace arg
///   is rejected at parse time.
///
/// Aliases resolve to the same `ScreenSpec` as their canonical form;
/// the error message's "valid" list contains canonical slugs only.
pub fn parse_start_slug(s: &str) -> Result<ScreenSpec, StartSlugError> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(StartSlugError::Empty);
    }

    // Parameterized form first: `slug:arg` (exactly one colon by splitn).
    if let Some((slug, arg)) = trimmed.split_once(':') {
        let lower = slug.to_ascii_lowercase();
        let arg_trimmed = arg.trim();
        // Empty/whitespace arg rejected BEFORE name normalization (which
        // would silently strip to "" and match the first bio).
        if arg_trimmed.is_empty() {
            return Err(StartSlugError::EmptyParameterizedArg {
                slug: lower.clone(),
            });
        }
        let needle = Needle::from_arg(arg_trimmed);
        match lower.as_str() {
            "player" => return Ok(ScreenSpec::Player(needle)),
            "goalie" => return Ok(ScreenSpec::Goalie(needle)),
            "comps" => return Ok(ScreenSpec::Comps(needle)),
            "team" => return Ok(ScreenSpec::Team(arg_trimmed.to_owned())),
            _ => {
                return Err(StartSlugError::UnknownParameterized {
                    slug: lower,
                    valid: vec!["player", "team", "goalie", "comps"],
                });
            }
        }
    }

    // Bare-slug nav lookup.
    let lower = trimmed.to_ascii_lowercase();
    for (slug, spec, _) in SLUG_TABLE {
        if *slug == lower.as_str() {
            return Ok(ScreenSpec::Nav(*spec));
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
            ("league", NavSpec::Home),
            ("depth", NavSpec::Depth),
            ("stats", NavSpec::Queries),
            ("goalies", NavSpec::Goalies),
            ("poach", NavSpec::Poach),
            ("scores", NavSpec::Tonight),
            ("schedule", NavSpec::Schedule),
            ("transactions", NavSpec::Transactions),
            ("playoffs", NavSpec::Playoffs),
        ];
        for (slug, nav) in cases {
            assert_eq!(
                parse_start_slug(slug).unwrap(),
                ScreenSpec::Nav(nav),
                "slug={slug}"
            );
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

    /// LB.1 / l0_nav_spec_into_screen_covers_every_variant
    /// — Each NavSpec variant maps to a real Screen value. If a new
    ///   variant is added without updating into_screen, this fails to
    ///   compile.
    #[test]
    fn l0_nav_spec_into_screen_covers_every_variant() {
        let _ = NavSpec::Home.into_screen();
        let _ = NavSpec::Depth.into_screen();
        let _ = NavSpec::Queries.into_screen();
        let _ = NavSpec::Goalies.into_screen();
        let _ = NavSpec::Tonight.into_screen();
        let _ = NavSpec::Schedule.into_screen();
        let _ = NavSpec::Transactions.into_screen();
        let _ = NavSpec::Playoffs.into_screen();
        let _ = NavSpec::Poach.into_screen();
    }

    // ── LB.3 — parameterized slugs ─────────────────────────────────────

    /// LB.3 / l0_player_pid_parses
    #[test]
    fn l0_player_pid_parses() {
        let spec = parse_start_slug("player:8478402").unwrap();
        assert_eq!(spec, ScreenSpec::Player(Needle::Pid(8478402)));
    }

    /// LB.3 / l0_player_name_parses_with_normalize
    #[test]
    fn l0_player_name_parses_with_normalize() {
        let spec = parse_start_slug("player:Bedard").unwrap();
        assert_eq!(spec, ScreenSpec::Player(Needle::Name("Bedard".into())));
    }

    /// LB.3 / l0_player_name_with_quotes_unwrapped
    #[test]
    fn l0_player_name_with_diacritic_preserved() {
        // Parse-time stores the raw name; resolution-time normalizes.
        let spec = parse_start_slug("player:Léhkonen").unwrap();
        assert_eq!(spec, ScreenSpec::Player(Needle::Name("Léhkonen".into())));
    }

    /// LB.3 / l0_team_abbrev_parses_uppercase_at_resolve_time
    #[test]
    fn l0_team_abbrev_parses_preserves_case_until_resolve() {
        // Parse keeps the original casing; resolve does the upper+trim.
        let spec = parse_start_slug("team:edm").unwrap();
        assert_eq!(spec, ScreenSpec::Team("edm".into()));
    }

    /// LB.3 / l0_goalie_slug_routes_to_goalie_variant
    #[test]
    fn l0_goalie_slug_routes_to_goalie_variant() {
        let spec = parse_start_slug("goalie:Brodeur").unwrap();
        assert_eq!(spec, ScreenSpec::Goalie(Needle::Name("Brodeur".into())));
    }

    /// LB.3 / l0_comps_slug_routes_to_comps_variant
    #[test]
    fn l0_comps_slug_routes_to_comps_variant() {
        let spec = parse_start_slug("comps:McDavid").unwrap();
        assert_eq!(spec, ScreenSpec::Comps(Needle::Name("McDavid".into())));
    }

    /// LB.3 / l0_unknown_parameterized_slug_lists_valid
    #[test]
    fn l0_unknown_parameterized_slug_lists_valid() {
        let err = parse_start_slug("xyz:foo").unwrap_err();
        match err {
            StartSlugError::UnknownParameterized { slug, valid } => {
                assert_eq!(slug, "xyz");
                assert!(valid.contains(&"player"));
                assert!(valid.contains(&"team"));
                assert!(valid.contains(&"goalie"));
                assert!(valid.contains(&"comps"));
            }
            _ => panic!("expected UnknownParameterized variant"),
        }
    }

    /// LB.3 / l0_empty_parameterized_arg_rejected
    /// — `player:` (no arg) must be rejected at PARSE time, before
    ///   normalization strips it to "" and matches every bio.
    #[test]
    fn l0_empty_parameterized_arg_rejected() {
        for input in ["player:", "team:", "goalie:", "comps:"] {
            let err = parse_start_slug(input).unwrap_err();
            assert!(
                matches!(err, StartSlugError::EmptyParameterizedArg { .. }),
                "input={input} expected EmptyParameterizedArg, got {err:?}"
            );
        }
    }

    /// LB.3 / l0_whitespace_parameterized_arg_rejected
    /// — `player: ` (whitespace-only) is the same hazard as empty.
    #[test]
    fn l0_whitespace_parameterized_arg_rejected() {
        for input in ["player: ", "team:   ", "goalie:\t"] {
            let err = parse_start_slug(input).unwrap_err();
            assert!(
                matches!(err, StartSlugError::EmptyParameterizedArg { .. }),
                "input={input:?} expected EmptyParameterizedArg, got {err:?}"
            );
        }
    }

    /// LB.3 / l0_param_slug_case_insensitive
    #[test]
    fn l0_param_slug_case_insensitive() {
        let spec1 = parse_start_slug("PLAYER:Bedard").unwrap();
        let spec2 = parse_start_slug("Player:Bedard").unwrap();
        let spec3 = parse_start_slug("player:Bedard").unwrap();
        assert_eq!(spec1, spec2);
        assert_eq!(spec2, spec3);
    }

    /// LB.3 / l0_param_arg_inner_whitespace_preserved
    /// — Multi-word names (Connor Bedard, Wayne Gretzky) survive parse.
    #[test]
    fn l0_param_arg_inner_whitespace_preserved() {
        let spec = parse_start_slug("player:Connor Bedard").unwrap();
        assert_eq!(
            spec,
            ScreenSpec::Player(Needle::Name("Connor Bedard".into()))
        );
    }

    /// LB.3 / l0_needle_from_arg_distinguishes_pid_vs_name
    #[test]
    fn l0_needle_from_arg_distinguishes_pid_vs_name() {
        assert_eq!(Needle::from_arg("8478402"), Needle::Pid(8478402));
        assert_eq!(Needle::from_arg("Bedard"), Needle::Name("Bedard".into()));
        // Mixed digit+letter falls back to Name (e.g. a hypothetical
        // future "K2L" or just an oddball string).
        assert_eq!(Needle::from_arg("123abc"), Needle::Name("123abc".into()));
    }

    /// LB.3 / l0_no_colon_falls_through_to_nav_lookup
    /// — `playerz` (no colon) hits the nav-lookup path and errors as
    ///   unknown surface, not as parameterized.
    #[test]
    fn l0_no_colon_falls_through_to_nav_lookup() {
        let err = parse_start_slug("playerz").unwrap_err();
        match err {
            StartSlugError::Unknown { input, .. } => {
                assert_eq!(input, "playerz");
            }
            other => panic!("expected Unknown, got {other:?}"),
        }
    }

    // ── LB.3 — Team resolution ─────────────────────────────────────────

    /// LB.3 / l0_team_resolution_uppercases_and_trims
    #[test]
    fn l0_team_resolution_uppercases_and_trims() {
        let screen = ScreenSpec::Team("  edm  ".into()).into_screen().unwrap();
        match screen {
            Screen::Team(s) => assert_eq!(s, "EDM"),
            other => panic!("expected Team(\"EDM\"), got {other:?}"),
        }
    }

    /// LB.3 / l0_team_resolution_rejects_unknown_abbrev
    #[test]
    fn l0_team_resolution_rejects_unknown_abbrev() {
        let err = ScreenSpec::Team("ZZZ".into()).into_screen().unwrap_err();
        match err {
            ResolveError::UnknownTeam { input, valid } => {
                assert_eq!(input, "ZZZ");
                assert!(valid.contains(&"EDM"));
                assert_eq!(valid.len(), 32);
            }
            other => panic!("expected UnknownTeam, got {other:?}"),
        }
    }

    /// LB.3 / l0_team_resolution_empty_arg
    /// — Empty string after trim hits the parse-time guard, but if it
    ///   slips through (e.g. constructed in code), resolution must
    ///   still error cleanly.
    #[test]
    fn l0_team_resolution_empty_arg() {
        let err = ScreenSpec::Team("   ".into()).into_screen().unwrap_err();
        assert!(matches!(err, ResolveError::EmptyArg { slug: "team" }));
    }

    /// LB.3 / l0_pid_resolution_passes_through
    #[test]
    fn l0_pid_resolution_passes_through() {
        let screen = ScreenSpec::Player(Needle::Pid(8478402))
            .into_screen()
            .unwrap();
        match screen {
            Screen::PlayerById(pid) => assert_eq!(pid.0, 8478402),
            other => panic!("expected PlayerById(8478402), got {other:?}"),
        }
    }

    /// LB.3 / l0_goalie_pid_resolves_to_goalie_screen
    /// — Same pid in a Goalie spec produces GoalieDetailById, not
    ///   PlayerById. Locks the discriminator on goalie vs skater
    ///   card routing.
    #[test]
    fn l0_goalie_pid_resolves_to_goalie_screen() {
        let screen = ScreenSpec::Goalie(Needle::Pid(8478402))
            .into_screen()
            .unwrap();
        assert!(matches!(screen, Screen::GoalieDetailById(_)));
    }

    /// LB.3 / l0_comps_pid_resolves_to_comps_screen
    #[test]
    fn l0_comps_pid_resolves_to_comps_screen() {
        let screen = ScreenSpec::Comps(Needle::Pid(8478402))
            .into_screen()
            .unwrap();
        assert!(matches!(screen, Screen::CompsById(_)));
    }

    // ── LB.3 — format_season_id helper ────────────────────────────────

    /// LB.3 / l0_format_season_id
    #[test]
    fn l0_format_season_id() {
        assert_eq!(format_season_id(20242025), "2024-25");
        assert_eq!(format_season_id(19921993), "1992-93");
        assert_eq!(format_season_id(19992000), "1999-00");
    }

    // ── LB.6 — Docs drift fence ───────────────────────────────────────

    /// LB.6 / l0_commands_md_lists_every_canonical_slug
    /// — Every canonical slug in SLUG_TABLE must appear in COMMANDS.md.
    ///   Catches: a slug renamed in the SLUG_TABLE but not in the docs;
    ///   a slug added to SLUG_TABLE but not yet documented. The drift
    ///   fence is a string-grep — robust to formatting changes in the
    ///   markdown.
    #[test]
    fn l0_commands_md_lists_every_canonical_slug() {
        const COMMANDS_MD: &str = include_str!("../../COMMANDS.md");
        for slug in canonical_slugs() {
            // Match either `tui SLUG` (sugar form) or `--start SLUG`.
            // Both are documented in the TUI surfaces section.
            let sugar_pattern = format!("tui {slug}");
            let start_pattern = format!("--start {slug}");
            let start_quoted = format!("--start \"{slug}");
            assert!(
                COMMANDS_MD.contains(&sugar_pattern)
                    || COMMANDS_MD.contains(&start_pattern)
                    || COMMANDS_MD.contains(&start_quoted),
                "canonical slug '{slug}' not found in COMMANDS.md — \
                 add it to the TUI surfaces section or remove the canonical \
                 declaration from SLUG_TABLE"
            );
        }
    }

    /// LB.6 / l0_commands_md_mentions_menu_and_drill_downs
    /// — The TUI surfaces section must call out `icelines menu` and the
    ///   drill-down sugar forms. Catches: section deleted or renamed
    ///   without updating users.
    #[test]
    fn l0_commands_md_mentions_menu_and_drill_downs() {
        const COMMANDS_MD: &str = include_str!("../../COMMANDS.md");
        assert!(
            COMMANDS_MD.contains("icelines menu"),
            "COMMANDS.md must document `icelines menu`"
        );
        assert!(
            COMMANDS_MD.contains("tui player"),
            "COMMANDS.md must document `tui player <name|pid>`"
        );
        assert!(
            COMMANDS_MD.contains("tui team"),
            "COMMANDS.md must document `tui team <abbrev>`"
        );
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
