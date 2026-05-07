//! Phase Art Ross A.0 — `Constraint` evaluator.
//!
//! Walks a `Constraint` tree against a `PlayerView` and decides
//! whether the player matches. For A.0 only `Bio` and `SeasonStat`
//! variants execute (the others are reserved with `FeatureNotYet`
//! at parse time, so they won't appear in produced trees).
//!
//! Missing-data semantics match the legacy `FilterExpr::matches`:
//! when a stat is unavailable for the view, the atom evaluates to
//! `false` (so `NOT (hits>=200)` accepts pre-2010 rows where hits
//! wasn't tracked).

use icelines_core::stats_catalog::StatUnit;
use icelines_core::stats_repository::PlayerView;

use crate::compute_age;
use crate::plan::{
    BioConstraint, BioField, Constraint, Predicate, ScalarOp, ScalarValue,
    SeasonStatConstraint,
};

impl Constraint {
    /// Evaluate this constraint tree against the given player view.
    /// `season` is the active season-id (as a u32) used for age
    /// computation. `is_goalie` is read from the view; `applies_to`
    /// is honored for stat atoms.
    ///
    /// **Missing data semantics**: a stat atom whose `stat.read(v)`
    /// returns `None` evaluates to `false`. This matches the legacy
    /// pipeline so `NOT (stat>=N)` flips pre-tracked rows to true.
    pub fn matches(&self, v: &PlayerView<'_>, season: u32) -> bool {
        match self {
            Constraint::Bio(b) => bio_matches(b, v, season),
            Constraint::SeasonStat(s) => season_stat_matches(s, v),
            Constraint::SlidingWindow(_) => {
                // Reserved A.2 — parser rejects these atoms today,
                // so this branch is unreachable from user input.
                // Treat as no-op (true) to avoid crashing if a
                // future caller constructs one programmatically
                // before A.2 ships.
                true
            }
            Constraint::CareerAggregate(_) => true,    // Reserved A.3
            Constraint::CareerLeague(_) => true,       // Reserved A.4
            Constraint::All(children) => children.iter().all(|c| c.matches(v, season)),
            Constraint::Any(children) => children.iter().any(|c| c.matches(v, season)),
            Constraint::Not(inner) => !inner.matches(v, season),
        }
    }
}

fn bio_matches(b: &BioConstraint, v: &PlayerView<'_>, season: u32) -> bool {
    let bio = &v.identity.bio;
    match b.field {
        BioField::Age => match &b.predicate {
            Predicate::Scalar(op, ScalarValue::Number(target)) => {
                let age = bio
                    .birth_date
                    .as_deref()
                    .and_then(|d| compute_age(d, season));
                match age {
                    Some(a) => apply_scalar_op_num(*op, a as f64, *target),
                    None => false,
                }
            }
            _ => false,
        },
        BioField::DraftYear => match &b.predicate {
            Predicate::Scalar(op, ScalarValue::Number(target)) => match bio.draft_year {
                Some(y) => apply_scalar_op_num(*op, y as f64, *target),
                None => false,
            },
            _ => false,
        },
        BioField::Height => match &b.predicate {
            Predicate::Scalar(op, ScalarValue::Number(target)) => match bio.height_in_inches {
                Some(h) => apply_scalar_op_num(*op, h as f64, *target),
                None => false,
            },
            _ => false,
        },
        BioField::Weight => match &b.predicate {
            Predicate::Scalar(op, ScalarValue::Number(target)) => match bio.weight_lbs {
                Some(w) => apply_scalar_op_num(*op, w as f64, *target),
                None => false,
            },
            _ => false,
        },
        BioField::Country => match &b.predicate {
            Predicate::Scalar(ScalarOp::Eq, ScalarValue::Text(target)) => {
                let bc = bio
                    .birth_country
                    .as_deref()
                    .map(ScalarValue::canonicalize_text);
                let nc = bio
                    .nationality_code
                    .as_deref()
                    .map(ScalarValue::canonicalize_text);
                bc.as_deref() == Some(target.as_str()) || nc.as_deref() == Some(target.as_str())
            }
            Predicate::Scalar(ScalarOp::Ne, ScalarValue::Text(target)) => {
                let bc = bio
                    .birth_country
                    .as_deref()
                    .map(ScalarValue::canonicalize_text);
                let nc = bio
                    .nationality_code
                    .as_deref()
                    .map(ScalarValue::canonicalize_text);
                let matches = bc.as_deref() == Some(target.as_str())
                    || nc.as_deref() == Some(target.as_str());
                !matches
            }
            _ => false,
        },
        BioField::Shoots => match &b.predicate {
            Predicate::Scalar(ScalarOp::Eq, ScalarValue::Text(target)) => bio
                .shoots_catches
                .as_deref()
                .map(ScalarValue::canonicalize_text)
                .as_deref()
                == Some(target.as_str()),
            Predicate::Scalar(ScalarOp::Ne, ScalarValue::Text(target)) => bio
                .shoots_catches
                .as_deref()
                .map(ScalarValue::canonicalize_text)
                .as_deref()
                != Some(target.as_str()),
            _ => false,
        },
        // Bio fields reserved for A.1 grammar expansion: parser
        // never builds Constraints with these fields today, so the
        // executor returns false defensively.
        BioField::DraftRound
        | BioField::DraftOverall
        | BioField::Nationality
        | BioField::Position
        | BioField::Team
        | BioField::TeamAny
        | BioField::TeamCareer
        | BioField::BirthCity
        | BioField::BirthState
        | BioField::RookieSeason => false,
    }
}

fn season_stat_matches(s: &SeasonStatConstraint, v: &PlayerView<'_>) -> bool {
    if !s.stat.applies_to(v.position(), v.is_goalie()) {
        // DI-08 — non-applicable filters silently pass.
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
        // Other predicate shapes reserved for A.1 — parser rejects.
        _ => false,
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Op application correctness — covers both directions on each
    /// op family. These tests don't need a PlayerView.
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
    fn l0_a0_apply_scalar_op_gt_strict() {
        assert!(apply_scalar_op_num(ScalarOp::Gt, 6.0, 5.0));
        assert!(!apply_scalar_op_num(ScalarOp::Gt, 5.0, 5.0));
        assert!(!apply_scalar_op_num(ScalarOp::Gt, 4.0, 5.0));
    }

    #[test]
    fn l0_a0_apply_scalar_op_lt_strict() {
        assert!(apply_scalar_op_num(ScalarOp::Lt, 4.0, 5.0));
        assert!(!apply_scalar_op_num(ScalarOp::Lt, 5.0, 5.0));
        assert!(!apply_scalar_op_num(ScalarOp::Lt, 6.0, 5.0));
    }

    #[test]
    fn l0_a0_apply_scalar_op_eq_with_count_tolerance() {
        // Count tolerance is 0.5 — so 10.0 == 10.4 but 10.0 != 11.0.
        assert!(apply_scalar_op_unit_aware(
            ScalarOp::Eq,
            10.4,
            10.0,
            StatUnit::Count
        ));
        assert!(!apply_scalar_op_unit_aware(
            ScalarOp::Eq,
            11.0,
            10.0,
            StatUnit::Count
        ));
    }

    #[test]
    fn l0_a0_apply_scalar_op_ne_count_tolerance() {
        assert!(!apply_scalar_op_unit_aware(
            ScalarOp::Ne,
            10.4,
            10.0,
            StatUnit::Count
        ));
        assert!(apply_scalar_op_unit_aware(
            ScalarOp::Ne,
            11.0,
            10.0,
            StatUnit::Count
        ));
    }
}
