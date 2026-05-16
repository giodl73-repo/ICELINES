//! `icelines-query` — shared query/filter utilities.
//!
//! Phase Art Ross A.0+ — the unified query architecture lives here.
//! The legacy v0.19.x bridging API (`BioAtom`, `extract_bio`,
//! `split_top_level_and`, `compute_age`) remains exported for
//! backward compatibility while the new pipeline (`plan` /
//! `parse_query` / `DataProvider` / `EvalCtx`) ships in parallel.
//!
//! Both surfaces apply the bio atoms after the catalog parser has
//! handled the stat residue today; over A.1-A.5 the new pipeline
//! supersedes this routing while keeping the legacy API as a
//! shim until v0.22.0.
//!
//! ## Phase Art Ross modules (new)
//!
//! - [`plan`] — Constraint IR (n-ary `All`/`Any`/`Not`), Predicate
//!   shape-by-construction, `StrictMode`.
//! - [`input`] — `FilterInput` enum, decode boundary per surface.
//! - [`errors`] — `ParseError` enum, multi-error reporting.
//! - [`data_provider`] — `DataProvider` trait + `EvalCtx` (the
//!   dependency-inversion seam).
//! - [`parser`] — `parse_query(FilterInput) -> Result<QueryPlan, Vec<ParseError>>`.
//! - [`planner`] — `QueryPlan::requirements()` walks the IR.
//!
//! ## Legacy bridging API (v0.19.x — kept for backward compat)
//!
//! - [`BioAtom`] — sum type covering all bio constraints.
//! - [`try_parse_bio_atom`] — single-atom parser; returns `None` for
//!   unrecognized keys so the caller routes them to the catalog.
//! - [`split_top_level_and`] — paren-aware split on case-insensitive
//!   AND. Bails on OR/NOT (caller falls back to the catalog parser
//!   for the whole expression in those cases).
//! - [`compute_age`] — Hockey-Reference's "age as of end of Jan 31 of
//!   the season's second year" convention.
//! - [`BioConstraints`] — a folded set of all bio atoms ready to
//!   apply against a `PlayerView`.

#![deny(unsafe_code)]

pub mod data_provider;
pub mod errors;
pub mod executor;
pub mod input;
pub mod parser;
pub mod plan;
pub mod planner;
pub mod slice_selectors;
pub mod sliding_window;
pub mod tokenizer;
pub mod url;

pub use data_provider::{
    DataProvider, DateRange, EvalCtx, FetchError, FetchEvent, PlanRequirement, StrictEligibility,
};
pub use errors::ParseError;
pub use input::{AtomFragment, FilterInput};
pub use parser::parse_query;
pub use plan::{
    AgeBound, BioConstraint, BioField, CareerAggrConstraint, CareerAggregator,
    CareerLeagueConstraint, Constraint, GlobPattern, LeagueAtom, LeagueTier, MemberOp,
    NumericRange, PatternOp, Predicate, QueryPlan, ScalarOp, ScalarValue, SeasonAxis,
    SeasonStatConstraint, SlidingWindow, SlidingWindowConstraint, StrictMode, WindowPolicy,
    WindowScope,
};
pub use slice_selectors::{
    compile_prepared_player_selector, prepared_player_selector_catalog, select_prepared_player_rows,
};
pub use url::{combine_filter_exprs, parse_filters_from_query};

use icelines_core::stats_repository::PlayerView;

/// Bio atom — a single typed bio constraint extracted from a filter
/// expression. Returned by [`try_parse_bio_atom`].
#[derive(Debug, Clone, PartialEq)]
pub enum BioAtom {
    AgeMin(u32),
    AgeMax(u32),
    DraftMin(u16),
    DraftMax(u16),
    HeightMin(u32),
    HeightMax(u32),
    WeightMin(u32),
    WeightMax(u32),
    Country(String),
    Shoots(String),
}

/// Try to parse a single filter atom (no AND/OR/NOT/parens) as a bio
/// term. Returns `Some(Vec<BioAtom>)` when the key matches a bio
/// field — equality on numeric keys emits both bounds. Returns
/// `None` for unrecognized keys; callers route those to the StatId
/// catalog parser.
///
/// Recognized syntaxes:
/// ```text
///   age>=22        AgeMin(22)
///   age<=28        AgeMax(28)
///   age=24         AgeMin(24) + AgeMax(24)
///   draft>=2020    DraftMin(2020)
///   height>=72     HeightMin(72)
///   weight<=200    WeightMax(200)
///   country=CAN    Country("CAN")
///   shoots=L       Shoots("L")
/// ```
pub fn try_parse_bio_atom(s: &str) -> Option<Vec<BioAtom>> {
    let t = s.trim();
    let (key, op, val) = if let Some((k, v)) = t.split_once(">=") {
        (k.trim(), ">=", v.trim())
    } else if let Some((k, v)) = t.split_once("<=") {
        (k.trim(), "<=", v.trim())
    } else if let Some((k, v)) = t.split_once('=') {
        (k.trim(), "=", v.trim())
    } else {
        return None;
    };
    let key_norm = key.to_ascii_lowercase().replace('_', "-");

    let numeric: Option<&str> = match key_norm.as_str() {
        "age" => Some("age"),
        "draft" | "draft-year" | "draft-yr" => Some("draft"),
        "height" | "ht" => Some("height"),
        "weight" | "wt" => Some("weight"),
        _ => None,
    };
    if let Some(kind) = numeric {
        let n: u32 = val.parse().ok()?;
        let n_u16: u16 = val.parse().ok().unwrap_or(0);
        let mut out = Vec::new();
        match (kind, op) {
            ("age", ">=") => out.push(BioAtom::AgeMin(n)),
            ("age", "<=") => out.push(BioAtom::AgeMax(n)),
            ("age", "=") => {
                out.push(BioAtom::AgeMin(n));
                out.push(BioAtom::AgeMax(n));
            }
            ("draft", ">=") => out.push(BioAtom::DraftMin(n_u16)),
            ("draft", "<=") => out.push(BioAtom::DraftMax(n_u16)),
            ("draft", "=") => {
                out.push(BioAtom::DraftMin(n_u16));
                out.push(BioAtom::DraftMax(n_u16));
            }
            ("height", ">=") => out.push(BioAtom::HeightMin(n)),
            ("height", "<=") => out.push(BioAtom::HeightMax(n)),
            ("height", "=") => {
                out.push(BioAtom::HeightMin(n));
                out.push(BioAtom::HeightMax(n));
            }
            ("weight", ">=") => out.push(BioAtom::WeightMin(n)),
            ("weight", "<=") => out.push(BioAtom::WeightMax(n)),
            ("weight", "=") => {
                out.push(BioAtom::WeightMin(n));
                out.push(BioAtom::WeightMax(n));
            }
            _ => return None,
        }
        return Some(out);
    }
    if op != "=" {
        return None;
    }
    match key_norm.as_str() {
        "country" | "nation" | "nationality" => {
            Some(vec![BioAtom::Country(val.to_ascii_uppercase())])
        }
        "shoots" | "hand" | "catches" => Some(vec![BioAtom::Shoots(val.to_ascii_uppercase())]),
        _ => None,
    }
}

/// Split a filter string on top-level " AND " (case-insensitive),
/// honoring paren depth. Returns `None` when the string contains
/// `OR` / `NOT` or unbalanced parens — the caller should fall back
/// to passing the whole expression to the catalog parser.
pub fn split_top_level_and(s: &str) -> Option<Vec<String>> {
    let upper = s.to_ascii_uppercase();
    let has_or = upper.split_whitespace().any(|w| w == "OR");
    let has_not = upper.split_whitespace().any(|w| w == "NOT");
    if has_or || has_not {
        return None;
    }
    let mut depth = 0i32;
    let mut pieces: Vec<String> = Vec::new();
    let mut cur = String::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c == '(' {
            depth += 1;
            cur.push(c);
            i += 1;
            continue;
        }
        if c == ')' {
            depth -= 1;
            if depth < 0 {
                return None;
            }
            cur.push(c);
            i += 1;
            continue;
        }
        if depth == 0 && (c == ' ' || c == '\t') {
            let rest = &s[i..];
            let rest_upper = rest.to_ascii_uppercase();
            let trimmed = rest_upper.trim_start();
            if trimmed.starts_with("AND ") || trimmed.starts_with("AND\t") {
                let lead_ws = rest.len() - rest.trim_start().len();
                i += lead_ws + 3; // "AND" = 3 chars
                while i < bytes.len() && (bytes[i] as char).is_whitespace() {
                    i += 1;
                }
                if !cur.trim().is_empty() {
                    pieces.push(cur.trim().to_owned());
                }
                cur.clear();
                continue;
            }
        }
        cur.push(c);
        i += 1;
    }
    if depth != 0 {
        return None;
    }
    if !cur.trim().is_empty() {
        pieces.push(cur.trim().to_owned());
    }
    Some(pieces)
}

/// Compute a player's stats-convention age for the given season.
/// Reference date: end of January 31 of the season's second year
/// (Hockey-Reference's convention; different from NHL's Sept 15
/// contract cutoff). Returns None when the bio's birth date is
/// missing or unparsable.
pub fn compute_age(birth_date: &str, season: u32) -> Option<u32> {
    let parts: Vec<&str> = birth_date.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let by: i32 = parts[0].parse().ok()?;
    let bm: i32 = parts[1].parse().ok()?;
    let bd: i32 = parts[2].parse().ok()?;
    let season_end_year: i32 = (season % 10_000) as i32;
    let mut age = season_end_year - by;
    if (bm, bd) > (1, 31) {
        age -= 1;
    }
    if !(0..=60).contains(&age) {
        return None;
    }
    Some(age as u32)
}

/// Folded bio constraints. Build one of these from a list of
/// `BioAtom` plus discrete query params (when both apply, the
/// constraint tightens — min takes the larger value, max the
/// smaller). Apply via [`BioConstraints::matches`].
#[derive(Debug, Clone, Default)]
pub struct BioConstraints {
    pub age_min: Option<u32>,
    pub age_max: Option<u32>,
    pub draft_min: Option<u16>,
    pub draft_max: Option<u16>,
    pub height_min: Option<u32>,
    pub height_max: Option<u32>,
    pub weight_min: Option<u32>,
    pub weight_max: Option<u32>,
    pub country: Option<String>,
    pub shoots: Option<String>,
}

impl BioConstraints {
    /// True iff any field is set.
    pub fn is_active(&self) -> bool {
        self.age_min.is_some()
            || self.age_max.is_some()
            || self.draft_min.is_some()
            || self.draft_max.is_some()
            || self.height_min.is_some()
            || self.height_max.is_some()
            || self.weight_min.is_some()
            || self.weight_max.is_some()
            || self.country.is_some()
            || self.shoots.is_some()
    }

    /// Merge a single atom into self. Numeric bounds tighten; string
    /// bounds (country/shoots) overwrite (last wins).
    pub fn merge(&mut self, atom: &BioAtom) {
        use std::cmp::{max, min};
        match atom {
            BioAtom::AgeMin(v) => self.age_min = Some(self.age_min.map_or(*v, |c| max(c, *v))),
            BioAtom::AgeMax(v) => self.age_max = Some(self.age_max.map_or(*v, |c| min(c, *v))),
            BioAtom::DraftMin(v) => {
                self.draft_min = Some(self.draft_min.map_or(*v, |c| max(c, *v)))
            }
            BioAtom::DraftMax(v) => {
                self.draft_max = Some(self.draft_max.map_or(*v, |c| min(c, *v)))
            }
            BioAtom::HeightMin(v) => {
                self.height_min = Some(self.height_min.map_or(*v, |c| max(c, *v)))
            }
            BioAtom::HeightMax(v) => {
                self.height_max = Some(self.height_max.map_or(*v, |c| min(c, *v)))
            }
            BioAtom::WeightMin(v) => {
                self.weight_min = Some(self.weight_min.map_or(*v, |c| max(c, *v)))
            }
            BioAtom::WeightMax(v) => {
                self.weight_max = Some(self.weight_max.map_or(*v, |c| min(c, *v)))
            }
            BioAtom::Country(s) => self.country = Some(s.clone()),
            BioAtom::Shoots(s) => self.shoots = Some(s.clone()),
        }
    }

    /// Apply the bio constraints to a player view. Returns false
    /// when the player fails any constraint OR when a constraint
    /// is set but the matching bio field is missing on the player
    /// (e.g. age filter on a player without a birth date).
    pub fn matches(&self, v: &PlayerView<'_>, season: u32) -> bool {
        let bio = &v.identity.bio;
        if self.age_min.is_some() || self.age_max.is_some() {
            let age = bio
                .birth_date
                .as_deref()
                .and_then(|d| compute_age(d, season));
            match age {
                Some(a) => {
                    if let Some(mn) = self.age_min {
                        if a < mn {
                            return false;
                        }
                    }
                    if let Some(mx) = self.age_max {
                        if a > mx {
                            return false;
                        }
                    }
                }
                None => return false,
            }
        }
        if let Some(mn) = self.draft_min {
            match bio.draft_year {
                Some(y) if y >= mn => {}
                _ => return false,
            }
        }
        if let Some(mx) = self.draft_max {
            match bio.draft_year {
                Some(y) if y <= mx => {}
                _ => return false,
            }
        }
        if let Some(mn) = self.height_min {
            match bio.height_in_inches {
                Some(h) if h >= mn => {}
                _ => return false,
            }
        }
        if let Some(mx) = self.height_max {
            match bio.height_in_inches {
                Some(h) if h <= mx => {}
                _ => return false,
            }
        }
        if let Some(mn) = self.weight_min {
            match bio.weight_lbs {
                Some(w) if w >= mn => {}
                _ => return false,
            }
        }
        if let Some(mx) = self.weight_max {
            match bio.weight_lbs {
                Some(w) if w <= mx => {}
                _ => return false,
            }
        }
        if let Some(c) = &self.country {
            let bc = bio.birth_country.as_deref().map(str::to_ascii_uppercase);
            let nc = bio.nationality_code.as_deref().map(str::to_ascii_uppercase);
            let matches = bc.as_deref() == Some(c.as_str()) || nc.as_deref() == Some(c.as_str());
            if !matches {
                return false;
            }
        }
        if let Some(s) = &self.shoots {
            match bio.shoots_catches.as_deref().map(str::to_ascii_uppercase) {
                Some(sc) if &sc == s => {}
                _ => return false,
            }
        }
        true
    }
}

/// Pre-extract bio atoms from each raw filter string's top-level AND
/// chain. Returns `(extracted_bio, stat_residue)`. Filter strings
/// that contain OR/NOT or that fail to split go through entirely as
/// stat residue (the catalog parser will handle or reject them).
///
/// Wave 11 #070 — when a piece is fully wrapped in a single pair of
/// outer parens (e.g. user typed `(age<=24 AND p>=10)`), strip the
/// parens and recurse into the inner expression. Without this, the
/// catalog parser sees `age` as an unknown stat-key and the filter
/// fails outright. World-class flexibility means the user shouldn't
/// have to know that `(...)` wrapping inhibits bio extraction.
pub fn extract_bio(raw_filters: &[String]) -> (Vec<BioAtom>, Vec<String>) {
    let mut bio: Vec<BioAtom> = Vec::new();
    let mut stat: Vec<String> = Vec::with_capacity(raw_filters.len());
    for raw in raw_filters {
        // An empty/whitespace-only `--filter ""` must round-trip into
        // the residue so the downstream parser surfaces EmptyInput
        // instead of silently being treated as no filter. Without
        // this, `split_top_level_and("")` produces zero pieces and
        // the input vanishes — likely a user shell-quoting bug.
        if raw.trim().is_empty() {
            stat.push(raw.clone());
            continue;
        }
        extract_bio_into(raw, &mut bio, &mut stat);
    }
    (bio, stat)
}

/// Recursive worker for `extract_bio`. Pulls bio atoms out of `raw`
/// into `bio`; pushes any stat residue into `stat` (joined back with
/// AND if multiple pieces survive).
fn extract_bio_into(raw: &str, bio: &mut Vec<BioAtom>, stat: &mut Vec<String>) {
    match split_top_level_and(raw) {
        Some(pieces) => {
            let mut stat_pieces: Vec<String> = Vec::new();
            for p in pieces {
                if let Some(atoms) = try_parse_bio_atom(&p) {
                    bio.extend(atoms);
                    continue;
                }
                // Wave 11 #070 — peel a single outer pair of parens
                // and try again. Only one level so we don't strip
                // semantic groups like `((A OR B) AND C)`.
                if let Some(inner) = peel_outer_parens(&p) {
                    let mut inner_bio: Vec<BioAtom> = Vec::new();
                    let mut inner_stat: Vec<String> = Vec::new();
                    extract_bio_into(inner, &mut inner_bio, &mut inner_stat);
                    if !inner_bio.is_empty() {
                        bio.extend(inner_bio);
                        // If everything inside was bio, drop the
                        // (now-empty) wrapper. If exactly one stat
                        // piece survived, drop the parens (they're
                        // semantically a no-op around a single atom).
                        // For multiple residue pieces, preserve the
                        // grouping in case they later participate in
                        // an OR — though OR fallback already passes
                        // the whole expr through verbatim, so this
                        // only matters for AND-chain reconstruction.
                        match inner_stat.len() {
                            0 => {}
                            1 => stat_pieces.push(inner_stat.into_iter().next().unwrap()),
                            _ => stat_pieces.push(format!("({})", inner_stat.join(" AND "))),
                        }
                        continue;
                    }
                }
                stat_pieces.push(p);
            }
            if !stat_pieces.is_empty() {
                stat.push(stat_pieces.join(" AND "));
            }
        }
        None => stat.push(raw.to_owned()),
    }
}

/// If `s` is `(X)` where the parens are paren-balanced and form a
/// single outer group, return `Some(X)`. Else `None`. Used by
/// `extract_bio` to peel a single user-typed paren wrapper.
fn peel_outer_parens(s: &str) -> Option<&str> {
    let t = s.trim();
    if !t.starts_with('(') || !t.ends_with(')') {
        return None;
    }
    // Verify the opening paren matches the closing one (i.e. the
    // outer parens form ONE group, not two). For `(A) AND (B)` the
    // opening at index 0 closes before the end — depth hits 0 mid-
    // string, so we must NOT peel.
    let mut depth = 0i32;
    let chars: Vec<char> = t.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c == '(' {
            depth += 1;
        } else if c == ')' {
            depth -= 1;
            if depth == 0 && i != chars.len() - 1 {
                return None;
            }
            if depth < 0 {
                return None;
            }
        }
    }
    if depth != 0 {
        return None;
    }
    Some(&t[1..t.len() - 1])
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── try_parse_bio_atom ──────────────────────────────────────

    #[test]
    fn l0_bio_atom_age_min_max() {
        let got = try_parse_bio_atom("age>=22").unwrap();
        assert!(matches!(got.as_slice(), [BioAtom::AgeMin(22)]));
        let got = try_parse_bio_atom("age<=28").unwrap();
        assert!(matches!(got.as_slice(), [BioAtom::AgeMax(28)]));
    }

    #[test]
    fn l0_bio_atom_age_eq_emits_both_bounds() {
        let got = try_parse_bio_atom("age=24").unwrap();
        assert_eq!(got.len(), 2);
        assert!(matches!(got[0], BioAtom::AgeMin(24)));
        assert!(matches!(got[1], BioAtom::AgeMax(24)));
    }

    #[test]
    fn l0_bio_atom_draft_aliases() {
        for k in ["draft>=2020", "draft-year>=2020", "draft_year>=2020"] {
            let got = try_parse_bio_atom(k).unwrap();
            assert!(matches!(got.as_slice(), [BioAtom::DraftMin(2020)]));
        }
    }

    #[test]
    fn l0_bio_atom_height_weight() {
        assert!(matches!(
            try_parse_bio_atom("height>=72").unwrap().as_slice(),
            [BioAtom::HeightMin(72)]
        ));
        assert!(matches!(
            try_parse_bio_atom("weight<=200").unwrap().as_slice(),
            [BioAtom::WeightMax(200)]
        ));
    }

    #[test]
    fn l0_bio_atom_country_uppercase_normalized() {
        let got = try_parse_bio_atom("country=can").unwrap();
        assert!(matches!(&got[0], BioAtom::Country(s) if s == "CAN"));
        let got = try_parse_bio_atom("Country=Swe").unwrap();
        assert!(matches!(&got[0], BioAtom::Country(s) if s == "SWE"));
    }

    #[test]
    fn l0_bio_atom_shoots() {
        let got = try_parse_bio_atom("shoots=L").unwrap();
        assert!(matches!(&got[0], BioAtom::Shoots(s) if s == "L"));
    }

    #[test]
    fn l0_bio_atom_unknown_key_returns_none() {
        for s in ["g>=50", "p>=80", "hits>=200", "ppg>=1.5", "blocks>=100"] {
            assert!(try_parse_bio_atom(s).is_none(), "key {s} must NOT match");
        }
    }

    #[test]
    fn l0_bio_atom_country_only_uses_eq() {
        assert!(try_parse_bio_atom("country>=CAN").is_none());
        assert!(try_parse_bio_atom("country<=CAN").is_none());
    }

    #[test]
    fn l0_bio_atom_garbage_value_returns_none() {
        assert!(try_parse_bio_atom("age>=lots").is_none());
        assert!(try_parse_bio_atom("draft>=").is_none());
    }

    // ── split_top_level_and ────────────────────────────────────

    #[test]
    fn l0_split_simple_and_chain() {
        let got = split_top_level_and("g>=50 AND a>=50 AND age>=22").unwrap();
        assert_eq!(got, vec!["g>=50", "a>=50", "age>=22"]);
    }

    #[test]
    fn l0_split_case_insensitive_and() {
        let got = split_top_level_and("g>=50 and a>=50 And age>=22").unwrap();
        assert_eq!(got.len(), 3);
    }

    #[test]
    fn l0_split_no_and_returns_single_piece() {
        let got = split_top_level_and("age>=22").unwrap();
        assert_eq!(got, vec!["age>=22"]);
    }

    #[test]
    fn l0_split_or_returns_none() {
        assert!(split_top_level_and("g>=50 OR a>=50").is_none());
        assert!(split_top_level_and("g>=50 or a>=50").is_none());
    }

    #[test]
    fn l0_split_not_returns_none() {
        assert!(split_top_level_and("NOT pim>=100").is_none());
        assert!(split_top_level_and("g>=50 AND NOT pim>=100").is_none());
    }

    #[test]
    fn l0_split_keeps_paren_group_intact() {
        let got = split_top_level_and("(g>=30 AND a>=30) AND age>=22").unwrap();
        assert_eq!(got, vec!["(g>=30 AND a>=30)", "age>=22"]);
    }

    #[test]
    fn l0_split_unbalanced_paren_returns_none() {
        assert!(split_top_level_and("(g>=50 AND a>=50").is_none());
        assert!(split_top_level_and("g>=50)").is_none());
    }

    // ── compute_age (Hockey-Reference Jan 31 convention) ───────

    #[test]
    fn l0_compute_age_january_birthday_aged_up() {
        assert_eq!(compute_age("2003-01-15", 20252026).unwrap(), 23);
    }

    #[test]
    fn l0_compute_age_april_birthday_subtracts_one() {
        assert_eq!(compute_age("2003-04-30", 20252026).unwrap(), 22);
    }

    #[test]
    fn l0_compute_age_dec_birthday_subtracts_one() {
        assert_eq!(compute_age("2003-12-01", 20252026).unwrap(), 22);
    }

    #[test]
    fn l0_compute_age_jan_31_boundary_aged_up() {
        assert_eq!(compute_age("2003-01-31", 20252026).unwrap(), 23);
    }

    #[test]
    fn l0_compute_age_feb_1_boundary_subtracts_one() {
        assert_eq!(compute_age("2003-02-01", 20252026).unwrap(), 22);
    }

    #[test]
    fn l0_compute_age_garbage_returns_none() {
        assert!(compute_age("not-a-date", 20252026).is_none());
        assert!(compute_age("2003", 20252026).is_none());
        assert!(compute_age("2003-04", 20252026).is_none());
    }

    // ── extract_bio (the integrating helper) ────────────────────

    /// l0_extract_bio_round_trip
    /// — A typical web filter chain mixes stat and bio terms; the
    ///   extractor splits them cleanly so the catalog parser only
    ///   sees stat residue.
    #[test]
    fn l0_extract_bio_round_trip() {
        let raw = vec!["g>=30 AND age<=24 AND height>=72".to_owned()];
        let (bio, stat) = extract_bio(&raw);
        assert_eq!(bio.len(), 2);
        assert_eq!(stat, vec!["g>=30".to_owned()]);
    }

    /// l0_extract_bio_pure_bio_chain_leaves_no_stat
    /// — When every piece is a bio atom the stat residue is empty.
    #[test]
    fn l0_extract_bio_pure_bio_chain_leaves_no_stat() {
        let raw = vec!["age>=22 AND age<=28 AND country=CAN".to_owned()];
        let (bio, stat) = extract_bio(&raw);
        assert_eq!(bio.len(), 3);
        assert!(stat.is_empty());
    }

    /// l0_extract_bio_or_passes_through
    /// — OR forces fallback: nothing is extracted, the whole
    ///   string lands in stat residue (catalog parser handles it).
    #[test]
    fn l0_extract_bio_or_passes_through() {
        let raw = vec!["g>=50 OR a>=50".to_owned()];
        let (bio, stat) = extract_bio(&raw);
        assert!(bio.is_empty());
        assert_eq!(stat, vec!["g>=50 OR a>=50".to_owned()]);
    }

    /// Wave 11 #070 — bio atoms wrapped in `()` should still be
    /// extracted. World-class flexibility: user shouldn't have to
    /// know that wrapping in parens disables bio extraction.
    #[test]
    fn l0_extract_bio_peels_outer_parens() {
        let raw = vec!["(age<=24 AND p>=10)".to_owned()];
        let (bio, stat) = extract_bio(&raw);
        assert_eq!(bio.len(), 1);
        assert!(matches!(&bio[0], BioAtom::AgeMax(24)));
        // Stat residue is the inner stat piece (parens dropped
        // because nothing else needs to live with it).
        assert_eq!(stat, vec!["p>=10".to_owned()]);
    }

    /// Wave 11 #070b — pure-bio chain inside parens should yield
    /// empty stat residue.
    #[test]
    fn l0_extract_bio_peels_outer_parens_pure_bio_chain() {
        let raw = vec!["(age>=22 AND age<=28)".to_owned()];
        let (bio, stat) = extract_bio(&raw);
        assert_eq!(bio.len(), 2);
        assert!(stat.is_empty());
    }

    /// Wave 11 #070c — peel only one level. `((age<=24))` should
    /// recurse twice. (We get this for free because the recursive
    /// worker re-enters extract_bio_into.)
    #[test]
    fn l0_extract_bio_peels_nested_parens() {
        let raw = vec!["((age<=24))".to_owned()];
        let (bio, _stat) = extract_bio(&raw);
        assert_eq!(bio.len(), 1);
    }

    // ── peel_outer_parens (the helper) ──────────────────────────

    /// Wave 11 #070 — basic peel.
    #[test]
    fn l0_peel_outer_parens_basic() {
        assert_eq!(peel_outer_parens("(g>=10)"), Some("g>=10"));
    }

    /// peel only when the outer parens form a single group.
    /// `(A) AND (B)` must not peel — that would change semantics
    /// (would fuse into `A) AND (B`, which is grammar garbage).
    #[test]
    fn l0_peel_outer_parens_two_groups_not_peeled() {
        assert_eq!(peel_outer_parens("(A) AND (B)"), None);
    }

    /// `(((g>=10)))` peels one level → `((g>=10))`. The recursive
    /// caller will keep peeling.
    #[test]
    fn l0_peel_outer_parens_nested_peel_one_level() {
        assert_eq!(peel_outer_parens("(((g>=10)))"), Some("((g>=10))"));
    }

    /// No outer parens — return None.
    #[test]
    fn l0_peel_outer_parens_no_parens() {
        assert_eq!(peel_outer_parens("g>=10"), None);
        assert_eq!(peel_outer_parens("(g>=10"), None);
        assert_eq!(peel_outer_parens("g>=10)"), None);
    }

    // ── BioConstraints ──────────────────────────────────────────

    /// l0_bio_constraints_merge_tightens_bounds
    /// — Two AgeMin atoms tighten to the larger; two AgeMax to the
    ///   smaller. Matches the discrete-vs-grammar compose rule.
    #[test]
    fn l0_bio_constraints_merge_tightens_bounds() {
        let mut c = BioConstraints::default();
        c.merge(&BioAtom::AgeMin(20));
        c.merge(&BioAtom::AgeMin(24)); // tightens up
        assert_eq!(c.age_min, Some(24));
        c.merge(&BioAtom::AgeMax(30));
        c.merge(&BioAtom::AgeMax(26)); // tightens down
        assert_eq!(c.age_max, Some(26));
    }

    /// l0_bio_constraints_country_overwrites
    /// — Country is overwrite (last wins); doesn't make sense to
    ///   tighten a country to two values.
    #[test]
    fn l0_bio_constraints_country_overwrites() {
        let mut c = BioConstraints::default();
        c.merge(&BioAtom::Country("CAN".into()));
        c.merge(&BioAtom::Country("SWE".into()));
        assert_eq!(c.country.as_deref(), Some("SWE"));
    }

    /// l0_bio_constraints_is_active_default_is_false
    #[test]
    fn l0_bio_constraints_is_active_default_is_false() {
        let c = BioConstraints::default();
        assert!(!c.is_active());
    }
}
