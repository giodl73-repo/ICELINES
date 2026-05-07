//! Phase Art Ross A.0 — planner.
//!
//! `QueryPlan::requirements()` walks the IR once and emits a
//! `PlanRequirement` describing what data must be local before
//! execution. The CLI/web/TUI surface then calls
//! `EvalCtx::strict_check(&req.eligibility)` and (if not strict-
//! rejected) `provider.ensure(&req)` before the executor runs.

use crate::data_provider::{PlanRequirement, StrictEligibility};
use crate::plan::{Constraint, QueryPlan};

impl QueryPlan {
    /// Walk the constraint tree and emit a `PlanRequirement`.
    ///
    /// In A.0 only `Bio` and `SeasonStat` variants exist in the
    /// wild (the others are reserved with `FeatureNotYet`), so the
    /// requirements walk is shallow: bios are always loaded;
    /// season stats need their respective reports for the active
    /// season.
    pub fn requirements(&self) -> PlanRequirement {
        let mut req = PlanRequirement::default();
        // For A.0 the eligibility is always "satisfiable" — no
        // sliding-window or career-aggregate atoms exist yet.
        req.eligible_for_strict = StrictEligibility {
            all_seasons_have_boxscores: true,
            all_pids_have_career_history: true,
            fallback_seasons: Vec::new(),
        };
        walk(&self.root, &mut req);
        req
    }

    /// Render the plan as a flat tree string for `--explain`. The
    /// format is intentionally simple in A.0 — A.5 ships the full
    /// rendering with cost estimates, requirements summary, and the
    /// JSON envelope.
    pub fn explain(&self) -> String {
        let mut out = String::new();
        render(&self.root, 0, &mut out);
        out
    }
}

fn walk(c: &Constraint, req: &mut PlanRequirement) {
    match c {
        Constraint::Bio(_) => {
            // Bios are always loaded — no requirement contribution.
        }
        Constraint::SeasonStat(_) => {
            // SeasonStat needs the active-season stats. The exact
            // (season, report) tuple is determined by the surface
            // (which carries the active season) so for A.0 we just
            // mark "stats" as needed without a season number. A.1
            // tightens this when --season is wired through.
            if !req.reports_needed.contains(&"stats") {
                req.reports_needed.push("stats");
            }
        }
        Constraint::SlidingWindow(_) => {
            // Reserved for A.2 — when populated, this branch will
            // contribute boxscore_seasons_needed +
            // boxscore_date_range to the requirement set.
        }
        Constraint::CareerAggregate(_) => {
            // Reserved for A.3 — will contribute the union of all
            // bundled seasons (minus lockout) to
            // boxscore_seasons_needed.
        }
        Constraint::CareerLeague(_) => {
            // Reserved for A.4 — will contribute career_pids_needed
            // for every active-roster pid.
        }
        Constraint::All(children) | Constraint::Any(children) => {
            for child in children {
                walk(child, req);
            }
        }
        Constraint::Not(inner) => walk(inner, req),
    }
}

fn render(c: &Constraint, indent: usize, out: &mut String) {
    let pad = "  ".repeat(indent);
    match c {
        Constraint::Bio(b) => {
            out.push_str(&format!("{pad}Bio({:?}, {:?})\n", b.field, b.predicate));
        }
        Constraint::SeasonStat(s) => {
            out.push_str(&format!(
                "{pad}SeasonStat({}, {:?}, axis={:?})\n",
                s.stat.cli_key(),
                s.predicate,
                s.axis
            ));
        }
        Constraint::SlidingWindow(_) => {
            out.push_str(&format!("{pad}SlidingWindow(<reserved A.2>)\n"));
        }
        Constraint::CareerAggregate(_) => {
            out.push_str(&format!("{pad}CareerAggregate(<reserved A.3>)\n"));
        }
        Constraint::CareerLeague(_) => {
            out.push_str(&format!("{pad}CareerLeague(<reserved A.4>)\n"));
        }
        Constraint::All(children) => {
            out.push_str(&format!("{pad}All\n"));
            for c in children {
                render(c, indent + 1, out);
            }
        }
        Constraint::Any(children) => {
            out.push_str(&format!("{pad}Any\n"));
            for c in children {
                render(c, indent + 1, out);
            }
        }
        Constraint::Not(inner) => {
            out.push_str(&format!("{pad}Not\n"));
            render(inner, indent + 1, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::FilterInput;
    use crate::parser::parse_query;

    #[test]
    fn l0_a0_requirements_simple_atom_needs_stats() {
        let plan = parse_query(FilterInput::Cli("g>=10".to_string())).unwrap();
        let req = plan.requirements();
        assert!(req.reports_needed.contains(&"stats"));
    }

    #[test]
    fn l0_a0_requirements_bio_only_needs_nothing() {
        // Pure bio chain (using the bio helper) — no SeasonStat
        // atom. The walk should NOT add "stats" to reports_needed.
        let plan = QueryPlan {
            root: Constraint::Bio(crate::plan::BioConstraint {
                field: crate::plan::BioField::Age,
                predicate: crate::plan::Predicate::Scalar(
                    crate::plan::ScalarOp::Le,
                    crate::plan::ScalarValue::Number(24.0),
                ),
            }),
        };
        let req = plan.requirements();
        assert!(req.reports_needed.is_empty());
    }

    #[test]
    fn l0_a0_requirements_all_strict_eligible() {
        // A.0 has no atoms that produce fallback seasons.
        let plan = parse_query(FilterInput::Cli("g>=10 AND a>=10".to_string())).unwrap();
        let req = plan.requirements();
        assert!(req.eligible_for_strict.fallback_seasons.is_empty());
    }

    #[test]
    fn l0_a0_explain_renders_n_ary_tree() {
        let plan = parse_query(FilterInput::Cli("g>=10 AND a>=10".to_string())).unwrap();
        let out = plan.explain();
        assert!(out.contains("All"));
        assert!(out.contains("SeasonStat(goals"));
        assert!(out.contains("SeasonStat(assists"));
    }

    #[test]
    fn l0_a0_explain_renders_or_tree() {
        let plan = parse_query(FilterInput::Cli("g>=10 OR a>=10".to_string())).unwrap();
        let out = plan.explain();
        assert!(out.contains("Any"));
    }

    #[test]
    fn l0_a0_explain_renders_not() {
        let plan = parse_query(FilterInput::Cli("NOT g>=100".to_string())).unwrap();
        let out = plan.explain();
        assert!(out.contains("Not"));
    }
}
