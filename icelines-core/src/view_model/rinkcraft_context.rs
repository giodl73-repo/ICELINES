//! Source records and environment, not individual defensive or role grades.
use super::rinkcraft_attribute_evidence::{explanation, AttributeExplanation};
use super::rinkcraft_numbers::{Measurement, NumericalProfile};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextEvidence {
    pub schema: String,
    pub title: String,
    pub note: String,
    pub facet_count: usize,
    pub measured_facets: usize,
    pub ranked_facets: usize,
    pub groups: Vec<ContextGroup>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextGroup {
    pub label: String,
    pub note: String,
    pub facets: Vec<ContextFacet>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextFacet {
    pub key: String,
    pub label: String,
    pub measurement: Option<Measurement>,
    pub explanation: AttributeExplanation,
}

pub(crate) fn build(p: &NumericalProfile) -> ContextEvidence {
    type GroupDefinition = (
        &'static str,
        &'static str,
        &'static [(&'static str, &'static str)],
    );
    let definitions: &[GroupDefinition] = if p.group == "Goalies" {
        &[("Cumulative saving", "Total outcome depends on workload; compare alongside the saving rate and actual ice time.", &[("gsax", "GSAx total")])]
    } else {
        &[
            (
                "Possession records",
                "Recorded events per ice time, not per touch or possession opportunity.",
                &[
                    ("takeaways-per-60", "Takeaways /60"),
                    ("giveaways-per-60", "Giveaways /60"),
                ],
            ),
            (
                "Penalty records",
                "Event frequency is not a complete discipline or defensive grade.",
                &[
                    ("penalties-drawn-per-60", "Drawn /60"),
                    ("penalties-taken-per-60", "Taken /60"),
                ],
            ),
            (
                "On-ice threat",
                "Team environment includes teammates, opponents and deployment.",
                &[
                    ("on-ice-xgf-per-60", "xG for /60"),
                    ("on-ice-xga-per-60", "xG against /60"),
                ],
            ),
        ]
    };
    let groups = definitions
        .iter()
        .map(|(label, note, keys)| ContextGroup {
            label: (*label).into(),
            note: (*note).into(),
            facets: keys
                .iter()
                .map(|(key, label)| ContextFacet {
                    key: (*key).into(),
                    label: (*label).into(),
                    measurement: p.metrics.iter().find(|m| m.key == *key).cloned(),
                    explanation: explanation(key).expect("declared context explanation"),
                })
                .collect(),
        })
        .collect::<Vec<_>>();
    let facets = groups.iter().flat_map(|g| &g.facets).collect::<Vec<_>>();
    ContextEvidence {
        schema: "icelines.rinkcraft-context-evidence.v1".into(),
        title: if p.group == "Goalies" { "Season saving context" } else { "Possession, penalties & on-ice threat" }.into(),
        note: match p.group.as_str() {
            "Goalies" => "Cumulative saving is a model-relative outcome, not goalie technique or a stable talent estimate.",
            "Defense" => "These records do not measure gap control, entry denials, slot coverage, pinch decisions or which defender owned an assignment.",
            _ => "These records do not measure pursuit pressure, retrieval success, backcheck assignments or individual defensive responsibility.",
        }.into(),
        facet_count: facets.len(),
        measured_facets: facets.iter().filter(|f| f.measurement.as_ref().is_some_and(|m| m.value.is_some())).count(),
        ranked_facets: facets.iter().filter(|f| f.measurement.as_ref().is_some_and(|m| m.state == "ranked" && m.value.is_some() && m.percentile.is_some())).count(),
        groups,
    }
}

#[cfg(test)]
mod tests {
    use super::super::rinkcraft_numbers::{self, InputRow};
    use super::*;
    fn profile(group: &str) -> NumericalProfile {
        let rows = (1..=25)
            .map(|id| InputRow {
                player_id: id,
                name: format!("Synthetic {id}"),
                group: group.into(),
                games: 30,
                seconds: 60000.,
                values: [
                    ("takeaways".into(), id as f64),
                    ("giveaways".into(), 0.),
                    ("penalties_drawn".into(), 3.),
                    ("penalties_taken".into(), 4.),
                    ("on_ice_xgf".into(), 20.),
                    ("on_ice_xga".into(), 30.),
                    ("xga".into(), 30.),
                    ("goals_against".into(), 20.),
                    ("shots_against".into(), 300.),
                ]
                .into_iter()
                .collect(),
            })
            .collect::<Vec<_>>();
        rinkcraft_numbers::build(&rows, 20242025, &[], vec![])
            .unwrap()
            .profiles
            .remove(0)
    }
    #[test]
    fn source_records_are_copied_and_defensive_limits_are_explicit() {
        for group in ["Forwards", "Defense"] {
            let p = profile(group);
            let c = build(&p);
            assert_eq!(c.facet_count, 6);
            assert_eq!(c.measured_facets, 6);
            assert_eq!(c.ranked_facets, 6);
            assert_eq!(
                c.groups[0].facets[1].measurement.as_ref().unwrap().value,
                Some(0.)
            );
            for f in c.groups.iter().flat_map(|g| &g.facets) {
                assert_eq!(
                    f.measurement.as_ref(),
                    p.metrics.iter().find(|m| m.key == f.key)
                );
            }
            assert!(if group == "Defense" {
                c.note.contains("gap control")
            } else {
                c.note.contains("pursuit pressure")
            });
        }
    }
    #[test]
    fn missing_and_unranked_records_are_not_zero_or_inverted() {
        let mut p = profile("Defense");
        p.metrics.retain(|m| m.key != "giveaways-per-60");
        let m = p
            .metrics
            .iter_mut()
            .find(|m| m.key == "takeaways-per-60")
            .unwrap();
        m.state = "insufficient_exposure".into();
        m.percentile = None;
        let c = build(&p);
        assert_eq!(c.facet_count, 6);
        assert_eq!(c.measured_facets, 5);
        assert_eq!(c.ranked_facets, 4);
        assert!(c.groups[0].facets[1].measurement.is_none());
        let f = &c.groups[1].facets[1];
        assert_eq!(
            f.measurement.as_ref().unwrap().percentile,
            p.metrics
                .iter()
                .find(|m| m.key == "penalties-taken-per-60")
                .unwrap()
                .percentile
        );
    }
    #[test]
    fn goalies_use_total_saving_instead_of_skater_records() {
        let p = profile("Goalies");
        let c = build(&p);
        assert_eq!(c.facet_count, 1);
        assert_eq!(c.ranked_facets, 1);
        assert_eq!(c.groups[0].facets[0].key, "gsax");
        assert_eq!(
            c.groups[0].facets[0].measurement.as_ref().unwrap().value,
            Some(10.)
        );
    }
}
