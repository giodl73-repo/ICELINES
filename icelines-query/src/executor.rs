//! Phase Art Ross A.0/A.1 — `Constraint` evaluator.
//!
//! Walks a `Constraint` tree against a `PlayerView` and decides
//! whether the player matches. A.0 wired Bio + SeasonStat with
//! Scalar predicates only. A.1 adds Member, Pattern, Range
//! predicate shapes plus new bio fields (Position, Team, etc.).
//!
//! Missing-data semantics match the legacy `FilterExpr::matches`:
//! when a stat is unavailable for the view, the atom evaluates to
//! `false` (so `NOT (hits>=200)` accepts pre-2010 rows where hits
//! wasn't tracked).

use icelines_core::stats_catalog::StatUnit;
use icelines_core::stats_repository::PlayerView;

use crate::compute_age;
use crate::data_provider::EvalCtx;
use crate::plan::{
    BioConstraint, BioField, Constraint, GlobPattern, MemberOp, NumericRange, PatternOp,
    Predicate, ScalarOp, ScalarValue, SeasonStatConstraint, SlidingWindowConstraint,
};
use crate::sliding_window::{aggregate_window, extract_window_stat, WindowResult};

impl Constraint {
    /// Evaluate this constraint tree against the given player view.
    ///
    /// Phase Art Ross A.2.5 review (forge) — the legacy two-method
    /// shape was a footgun (the placeholder `matches` returned
    /// `true` for unwired variants, silently over-matching). Now
    /// there's one entry point: `matches`. It takes an `EvalCtx`
    /// because every variant needs season + today + provider for
    /// correct evaluation. Bio/SeasonStat ignore most of the ctx
    /// (only `season` is read for age computation); SlidingWindow
    /// pulls per-game lines via `ctx.provider`; CareerAggregate /
    /// CareerLeague will use the full ctx in A.3/A.4.
    ///
    /// For unwired variants (CareerAggregate, CareerLeague) the
    /// parser today rejects the atom shapes that would construct
    /// them — so these branches are unreachable from user input.
    /// The match arms return false (silent over-match was the bug).
    pub fn matches(&self, v: &PlayerView<'_>, ctx: &EvalCtx<'_>) -> bool {
        match self {
            Constraint::Bio(b) => bio_matches(b, v, ctx.season),
            Constraint::SeasonStat(s) => season_stat_matches(s, v),
            Constraint::SlidingWindow(s) => sliding_window_matches(s, v, ctx),
            // Phase Art Ross A.2.5 — these variants don't yet have
            // an evaluator. Returning `false` (instead of `true`
            // per the previous shape) means a misconstructed plan
            // matches NOBODY rather than EVERYONE — a fail-closed
            // default. The parser rejects atoms that would build
            // these variants, so this code is unreachable from
            // user input today.
            Constraint::CareerAggregate(_) => false,
            Constraint::CareerLeague(_) => false,
            Constraint::All(children) => children.iter().all(|c| c.matches(v, ctx)),
            Constraint::Any(children) => children.iter().any(|c| c.matches(v, ctx)),
            Constraint::Not(inner) => !inner.matches(v, ctx),
        }
    }
}

fn sliding_window_matches(
    s: &SlidingWindowConstraint,
    v: &PlayerView<'_>,
    ctx: &EvalCtx<'_>,
) -> bool {
    if !s.stat.applies_to(v.position(), v.is_goalie()) {
        return true;
    }
    let pid = v.identity.id.0;
    let lines = ctx.provider.fetch_game_lines(pid, ctx.season);
    let current_team = v.team().map(|t| t.0.as_str());
    let result = aggregate_window(&lines, &s.window, ctx.today, current_team);
    let totals = match result {
        WindowResult::Empty => return false,
        WindowResult::Full(t) => t,
        WindowResult::ShortWindow { totals, .. } => totals,
    };
    let actual = match extract_window_stat(s.stat, &totals) {
        Some(x) => x,
        None => return false,
    };
    match &s.predicate {
        Predicate::Scalar(op, ScalarValue::Number(target)) => {
            apply_scalar_op_unit_aware(*op, actual, *target, s.stat.unit())
        }
        Predicate::Range(NumericRange { min, max }) => actual >= *min && actual <= *max,
        _ => false, // Member / Pattern on numeric stat: parser-rejected
    }
}

fn bio_matches(b: &BioConstraint, v: &PlayerView<'_>, season: u32) -> bool {
    match b.field {
        BioField::Age => match age_for(v, season) {
            Some(a) => predicate_matches_number(&b.predicate, a as f64),
            None => false,
        },
        BioField::DraftYear => match v.identity.bio.draft_year {
            Some(y) => predicate_matches_number(&b.predicate, y as f64),
            None => false,
        },
        BioField::DraftRound => match v.identity.bio.draft_round {
            Some(r) => predicate_matches_number(&b.predicate, r as f64),
            None => false,
        },
        BioField::DraftOverall => match v.identity.bio.draft_overall {
            Some(o) => predicate_matches_number(&b.predicate, o as f64),
            None => false,
        },
        BioField::Height => match v.identity.bio.height_in_inches {
            Some(h) => predicate_matches_number(&b.predicate, h as f64),
            None => false,
        },
        BioField::Weight => match v.identity.bio.weight_lbs {
            Some(w) => predicate_matches_number(&b.predicate, w as f64),
            None => false,
        },
        BioField::Country => {
            // country can match either birth_country or
            // nationality_code (legacy semantics from BioAtom).
            let bc = v
                .identity
                .bio
                .birth_country
                .as_deref()
                .map(ScalarValue::canonicalize_text);
            let nc = v
                .identity
                .bio
                .nationality_code
                .as_deref()
                .map(ScalarValue::canonicalize_text);
            // Two candidate strings — pass to the text predicate
            // applied via OR.
            text_predicate_matches_any(&b.predicate, &[bc.as_deref(), nc.as_deref()])
        }
        BioField::Nationality => {
            let nc = v
                .identity
                .bio
                .nationality_code
                .as_deref()
                .map(ScalarValue::canonicalize_text);
            text_predicate_matches(&b.predicate, nc.as_deref())
        }
        BioField::Shoots => {
            let s = v
                .identity
                .bio
                .shoots_catches
                .as_deref()
                .map(ScalarValue::canonicalize_text);
            text_predicate_matches(&b.predicate, s.as_deref())
        }
        BioField::Position => {
            let p = format!("{:?}", v.position()).to_ascii_lowercase();
            // Position::Center → "center"; map to canonical short
            // tokens "c" / "lw" / "rw" / "d" / "g" so the user's
            // `pos=C` query matches.
            let canonical = position_short_code(&p);
            text_predicate_matches(&b.predicate, Some(canonical.as_str()))
        }
        BioField::Team => {
            // Current stint only.
            let t = v
                .team()
                .map(|abbr| ScalarValue::canonicalize_text(&abbr.0));
            text_predicate_matches(&b.predicate, t.as_deref())
        }
        BioField::TeamAny => {
            // Any stint this season.
            let abbrevs: Vec<String> = v
                .stats
                .team_stints
                .iter()
                .map(|s| ScalarValue::canonicalize_text(&s.team.0))
                .collect();
            let refs: Vec<Option<&str>> = abbrevs.iter().map(|s| Some(s.as_str())).collect();
            text_predicate_matches_any(&b.predicate, &refs)
        }
        BioField::TeamCareer => {
            // A.2.5 review (scout + edge) — parser now rejects
            // `team.career=` atoms with FeatureNotYet, so this
            // branch is unreachable from user input. If a future
            // caller constructs one programmatically, fail closed
            // (return false) — silent over-match was the bug.
            false
        }
        BioField::BirthCity => {
            let s = v
                .identity
                .bio
                .birth_city
                .as_deref()
                .map(ScalarValue::canonicalize_text);
            text_predicate_matches(&b.predicate, s.as_deref())
        }
        BioField::BirthState => {
            let s = v
                .identity
                .bio
                .birth_state_province
                .as_deref()
                .map(ScalarValue::canonicalize_text);
            text_predicate_matches(&b.predicate, s.as_deref())
        }
        BioField::RookieSeason => match v.identity.bio.rookie_season.as_deref() {
            Some(s) => match s.parse::<u32>() {
                Ok(n) => predicate_matches_number(&b.predicate, n as f64),
                Err(_) => false,
            },
            None => false,
        },
    }
}

fn season_stat_matches(s: &SeasonStatConstraint, v: &PlayerView<'_>) -> bool {
    if !s.stat.applies_to(v.position(), v.is_goalie()) {
        return true;
    }
    let actual = match s.stat.read(v) {
        Some(x) => x,
        None => return false,
    };
    match &s.predicate {
        Predicate::Scalar(op, ScalarValue::Number(target)) => {
            apply_scalar_op_unit_aware(*op, actual, *target, s.stat.unit())
        }
        Predicate::Range(NumericRange { min, max }) => actual >= *min && actual <= *max,
        // Member / Pattern on numeric stat atoms is rejected at
        // parse, so this branch is unreachable from user input.
        _ => false,
    }
}

fn age_for(v: &PlayerView<'_>, season: u32) -> Option<u32> {
    v.identity
        .bio
        .birth_date
        .as_deref()
        .and_then(|d| compute_age(d, season))
}

/// Apply a numeric predicate. Used by both bio numeric fields and
/// (via the SeasonStat path) catalog stats. Range and Member with
/// numeric values are honored.
fn predicate_matches_number(p: &Predicate, actual: f64) -> bool {
    match p {
        Predicate::Scalar(op, ScalarValue::Number(target)) => {
            apply_scalar_op_num(*op, actual, *target)
        }
        Predicate::Range(NumericRange { min, max }) => actual >= *min && actual <= *max,
        Predicate::Member(op, vals) => {
            let any = vals.iter().any(|v| match v {
                ScalarValue::Number(n) => (actual - *n).abs() < 1e-9,
                _ => false,
            });
            match op {
                MemberOp::In => any,
                MemberOp::NotIn => !any,
            }
        }
        // Pattern on numeric is parser-rejected; defensively false.
        _ => false,
    }
}

/// Apply a string predicate. `actual` is the canonicalized field
/// value (already NFD-stripped + lowercased). None means the field
/// is missing on the player; predicate returns false.
fn text_predicate_matches(p: &Predicate, actual: Option<&str>) -> bool {
    match p {
        Predicate::Scalar(op, ScalarValue::Text(target)) => match actual {
            Some(s) => match op {
                ScalarOp::Eq => s == target,
                ScalarOp::Ne => s != target,
                _ => false, // <, >, <=, >= on strings: parser rejects
            },
            None => false,
        },
        Predicate::Member(op, vals) => match actual {
            Some(s) => {
                let any = vals.iter().any(|v| match v {
                    ScalarValue::Text(t) => s == t.as_str(),
                    _ => false,
                });
                match op {
                    MemberOp::In => any,
                    MemberOp::NotIn => !any,
                }
            }
            None => false,
        },
        Predicate::Pattern(op, glob) => match actual {
            Some(s) => match op {
                PatternOp::Like => glob.matches(s),
                PatternOp::NotLike => !glob.matches(s),
                PatternOp::Contains => contains_match(glob, s),
                PatternOp::NotContains => !contains_match(glob, s),
            },
            None => false,
        },
        // Scalar Number on string field: parser-rejected.
        _ => false,
    }
}

/// Same as `text_predicate_matches` but tries multiple candidate
/// values (useful for `country` which matches birth_country OR
/// nationality_code, or `team.any` which checks every stint).
fn text_predicate_matches_any(p: &Predicate, candidates: &[Option<&str>]) -> bool {
    candidates
        .iter()
        .any(|c| text_predicate_matches(p, *c))
}

/// `~ pattern` is "contains" (substring match, no anchoring).
/// We treat `glob` as a literal substring (segments joined).
fn contains_match(glob: &GlobPattern, target: &str) -> bool {
    if glob.segments.is_empty() {
        return target.is_empty();
    }
    glob.segments.iter().all(|seg| target.contains(seg.as_str()))
}

fn apply_scalar_op_num(op: ScalarOp, actual: f64, target: f64) -> bool {
    match op {
        ScalarOp::Ge => actual >= target,
        ScalarOp::Le => actual <= target,
        ScalarOp::Gt => actual > target,
        ScalarOp::Lt => actual < target,
        ScalarOp::Eq => (actual - target).abs() < 1e-9,
        ScalarOp::Ne => (actual - target).abs() >= 1e-9,
    }
}

fn apply_scalar_op_unit_aware(op: ScalarOp, actual: f64, target: f64, unit: StatUnit) -> bool {
    match op {
        ScalarOp::Ge => actual >= target,
        ScalarOp::Le => actual <= target,
        ScalarOp::Gt => actual > target,
        ScalarOp::Lt => actual < target,
        ScalarOp::Eq => match unit {
            StatUnit::Count | StatUnit::Seconds => (actual - target).abs() < 0.5,
            StatUnit::Per60 => (actual - target).abs() < 1e-3,
            StatUnit::Pct | StatUnit::Rate | StatUnit::Inverted => (actual - target).abs() < 1e-6,
        },
        ScalarOp::Ne => !match unit {
            StatUnit::Count | StatUnit::Seconds => (actual - target).abs() < 0.5,
            StatUnit::Per60 => (actual - target).abs() < 1e-3,
            StatUnit::Pct | StatUnit::Rate | StatUnit::Inverted => (actual - target).abs() < 1e-6,
        },
    }
}

/// Map a Position debug-printed string to its canonical short code
/// for `pos=C` queries.
fn position_short_code(debug_form: &str) -> String {
    match debug_form {
        "center" => "c".to_string(),
        "leftwing" | "left_wing" => "lw".to_string(),
        "rightwing" | "right_wing" => "rw".to_string(),
        "defenseman" | "defense" => "d".to_string(),
        "goalie" => "g".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_a0_apply_scalar_op_ge() {
        assert!(apply_scalar_op_num(ScalarOp::Ge, 10.0, 5.0));
        assert!(apply_scalar_op_num(ScalarOp::Ge, 5.0, 5.0));
        assert!(!apply_scalar_op_num(ScalarOp::Ge, 4.0, 5.0));
    }

    #[test]
    fn l0_a0_apply_scalar_op_le() {
        assert!(apply_scalar_op_num(ScalarOp::Le, 4.0, 5.0));
        assert!(apply_scalar_op_num(ScalarOp::Le, 5.0, 5.0));
        assert!(!apply_scalar_op_num(ScalarOp::Le, 6.0, 5.0));
    }

    #[test]
    fn l0_a1_apply_scalar_op_lt_strict() {
        assert!(apply_scalar_op_num(ScalarOp::Lt, 4.0, 5.0));
        assert!(!apply_scalar_op_num(ScalarOp::Lt, 5.0, 5.0));
        assert!(!apply_scalar_op_num(ScalarOp::Lt, 6.0, 5.0));
    }

    #[test]
    fn l0_a1_apply_scalar_op_gt_strict() {
        assert!(apply_scalar_op_num(ScalarOp::Gt, 6.0, 5.0));
        assert!(!apply_scalar_op_num(ScalarOp::Gt, 5.0, 5.0));
        assert!(!apply_scalar_op_num(ScalarOp::Gt, 4.0, 5.0));
    }

    #[test]
    fn l0_a1_apply_scalar_op_ne() {
        assert!(apply_scalar_op_num(ScalarOp::Ne, 4.0, 5.0));
        assert!(!apply_scalar_op_num(ScalarOp::Ne, 5.0, 5.0));
    }

    #[test]
    fn l0_a1_predicate_range_inclusive() {
        let p = Predicate::Range(NumericRange {
            min: 20.0,
            max: 40.0,
        });
        assert!(predicate_matches_number(&p, 20.0));
        assert!(predicate_matches_number(&p, 40.0));
        assert!(predicate_matches_number(&p, 30.0));
        assert!(!predicate_matches_number(&p, 19.0));
        assert!(!predicate_matches_number(&p, 41.0));
    }

    #[test]
    fn l0_a1_predicate_in_numeric() {
        let p = Predicate::Member(
            MemberOp::In,
            vec![
                ScalarValue::Number(2020.0),
                ScalarValue::Number(2021.0),
                ScalarValue::Number(2022.0),
            ],
        );
        assert!(predicate_matches_number(&p, 2020.0));
        assert!(predicate_matches_number(&p, 2021.0));
        assert!(!predicate_matches_number(&p, 2019.0));
    }

    #[test]
    fn l0_a1_predicate_not_in_numeric() {
        let p = Predicate::Member(
            MemberOp::NotIn,
            vec![ScalarValue::Number(2020.0), ScalarValue::Number(2021.0)],
        );
        assert!(predicate_matches_number(&p, 2019.0));
        assert!(predicate_matches_number(&p, 2022.0));
        assert!(!predicate_matches_number(&p, 2020.0));
    }

    #[test]
    fn l0_a1_predicate_in_text() {
        let p = Predicate::Member(
            MemberOp::In,
            vec![
                ScalarValue::Text("can".into()),
                ScalarValue::Text("usa".into()),
                ScalarValue::Text("swe".into()),
            ],
        );
        assert!(text_predicate_matches(&p, Some("can")));
        assert!(text_predicate_matches(&p, Some("usa")));
        assert!(!text_predicate_matches(&p, Some("rus")));
        assert!(!text_predicate_matches(&p, None));
    }

    #[test]
    fn l0_a1_predicate_not_in_text() {
        let p = Predicate::Member(
            MemberOp::NotIn,
            vec![ScalarValue::Text("can".into())],
        );
        assert!(text_predicate_matches(&p, Some("usa")));
        assert!(!text_predicate_matches(&p, Some("can")));
    }

    #[test]
    fn l0_a1_predicate_like_pattern() {
        let glob = GlobPattern::parse("Mc*");
        let p = Predicate::Pattern(PatternOp::Like, glob);
        assert!(text_predicate_matches(&p, Some("mcdavid")));
        assert!(!text_predicate_matches(&p, Some("crosby")));
    }

    #[test]
    fn l0_a1_predicate_not_like() {
        let glob = GlobPattern::parse("Mc*");
        let p = Predicate::Pattern(PatternOp::NotLike, glob);
        assert!(!text_predicate_matches(&p, Some("mcdavid")));
        assert!(text_predicate_matches(&p, Some("crosby")));
    }

    #[test]
    fn l0_a1_predicate_contains() {
        let glob = GlobPattern::parse("Da");
        let p = Predicate::Pattern(PatternOp::Contains, glob);
        assert!(text_predicate_matches(&p, Some("mcdavid")));
        assert!(!text_predicate_matches(&p, Some("crosby")));
    }

    #[test]
    fn l0_a1_position_short_codes() {
        assert_eq!(position_short_code("center"), "c");
        assert_eq!(position_short_code("leftwing"), "lw");
        assert_eq!(position_short_code("rightwing"), "rw");
        assert_eq!(position_short_code("defenseman"), "d");
        assert_eq!(position_short_code("goalie"), "g");
    }
}
