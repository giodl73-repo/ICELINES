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
        // For A.0 the eligibility is always "satisfiable" — no
        // sliding-window or career-aggregate atoms exist yet.
        let mut req = PlanRequirement {
            eligible_for_strict: StrictEligibility {
                all_seasons_have_boxscores: true,
                all_pids_have_career_history: true,
                fallback_seasons: Vec::new(),
            },
            ..Default::default()
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
        Constraint::SlidingWindow(s) => {
            out.push_str(&format!(
                "{pad}SlidingWindow({}, {:?}, {:?}, axis={:?})\n",
                s.stat.cli_key(),
                s.window,
                s.predicate,
                s.axis
            ));
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

    // ── A.2.6 review (bench) — golden snapshot tests ──────────
    //
    // The earlier explain tests only checked `contains("All")`.
    // A renderer change that broke indentation, reordered
    // children, or dropped variants would not be caught. These
    // tests pin the exact rendered string for representative
    // plan shapes. When the format changes intentionally, update
    // the expected strings here in one commit.

    /// Single SeasonStat — flat 1-line render.
    #[test]
    fn l0_a26_explain_golden_single_seasonstat() {
        let plan = parse_query(FilterInput::Cli("g>=10".to_string())).unwrap();
        let out = plan.explain();
        let expected = "SeasonStat(goals, Scalar(Ge, Number(10.0)), axis=Regular)\n";
        assert_eq!(out, expected, "explain golden mismatch");
    }

    /// 3-child All — n-ary IR shape preserved in render.
    #[test]
    fn l0_a26_explain_golden_three_child_all() {
        let plan = parse_query(FilterInput::Cli("g>=10 AND a>=10 AND p>=20".to_string())).unwrap();
        let out = plan.explain();
        let expected = "All\n  \
            SeasonStat(goals, Scalar(Ge, Number(10.0)), axis=Regular)\n  \
            SeasonStat(assists, Scalar(Ge, Number(10.0)), axis=Regular)\n  \
            SeasonStat(points, Scalar(Ge, Number(20.0)), axis=Regular)\n";
        assert_eq!(out, expected, "explain golden mismatch");
    }

    /// Not-wrapping-Any — exercises mixed boolean nesting.
    #[test]
    fn l0_a26_explain_golden_not_wrapping_any() {
        let plan = parse_query(FilterInput::Cli("NOT (g>=100 OR a>=100)".to_string())).unwrap();
        let out = plan.explain();
        let expected = "Not\n  \
            Any\n    \
            SeasonStat(goals, Scalar(Ge, Number(100.0)), axis=Regular)\n    \
            SeasonStat(assists, Scalar(Ge, Number(100.0)), axis=Regular)\n";
        assert_eq!(out, expected, "explain golden mismatch");
    }

    /// Bio + SeasonStat compound — covers both atom variants in one tree.
    #[test]
    fn l0_a26_explain_golden_bio_plus_seasonstat() {
        let plan = parse_query(FilterInput::Cli("age<=24 AND g>=20".to_string())).unwrap();
        let out = plan.explain();
        let expected = "All\n  \
            Bio(Age, Scalar(Le, Number(24.0)))\n  \
            SeasonStat(goals, Scalar(Ge, Number(20.0)), axis=Regular)\n";
        assert_eq!(out, expected, "explain golden mismatch");
    }

    /// Sliding-window atom — exercises the SlidingWindow render
    /// path. A.5 will polish the format; this test will need
    /// updating then.
    #[test]
    fn l0_a26_explain_golden_sliding_window() {
        let plan = parse_query(FilterInput::Cli("g.last10g>=5".to_string())).unwrap();
        let out = plan.explain();
        assert_eq!(
            out,
            "SlidingWindow(goals, LastN_GP { n: 10, scope: CurrentTeamCurrentSeason, policy: RequireFull }, Scalar(Ge, Number(5.0)), axis=Regular)\n"
        );
    }

    /// Sliding-window with calendar window + scope modifier.
    #[test]
    fn l0_a26_explain_golden_sliding_window_calendar_allteams() {
        let plan = parse_query(FilterInput::Cli("g.last10g.allteams>=5".to_string())).unwrap();
        let out = plan.explain();
        assert_eq!(
            out,
            "SlidingWindow(goals, LastN_GP { n: 10, scope: AllTeamsCurrentSeason, policy: RequireFull }, Scalar(Ge, Number(5.0)), axis=Regular)\n"
        );
    }
}
