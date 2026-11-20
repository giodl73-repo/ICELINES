//! Descriptive gaps within one matched source, never an overall or class score.
use super::rinkcraft_numbers::{primary_facets, ComparisonRow, EvidenceCard};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContrastSummary {
    pub schema: String,
    pub facet_count: usize,
    pub ranked_facets: usize,
    pub equal_rank_facets: usize,
    pub highlights: Vec<ContrastHighlight>,
    pub omitted: Vec<ContrastOmission>,
    pub selection_note: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContrastHighlight {
    pub label: String,
    pub measurement: ComparisonRow,
    pub limit: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContrastOmission {
    pub key: String,
    pub label: String,
    pub reason: String,
}

pub fn build(left: &EvidenceCard, right: &EvidenceCard, rows: &[ComparisonRow]) -> ContrastSummary {
    let facets = primary_facets(left.group == "Goalies");
    let mut ranked = 0;
    let mut equal = 0;
    let mut candidates = Vec::new();
    let mut omitted = Vec::new();
    for (order, (key, label)) in facets.iter().enumerate() {
        let a = left.metrics.iter().find(|m| m.key == *key);
        let b = right.metrics.iter().find(|m| m.key == *key);
        let row = rows.iter().find(|r| r.key == *key);
        let reason = match (a, b, row) {
            (Some(a), Some(b), Some(r))
                if a.state == "ranked"
                    && b.state == "ranked"
                    && a.measured_peers == b.measured_peers
                    && r.left.is_some()
                    && r.right.is_some()
                    && r.left_percentile.is_some()
                    && r.right_percentile.is_some()
                    && r.percentile_difference.is_some() =>
            {
                None
            }
            (None, _, _) | (_, None, _) => {
                Some("Primary measurement missing on at least one side.")
            }
            _ => Some("Both sides need ranked raw values and the same measured peer population."),
        };
        if let Some(reason) = reason {
            omitted.push(ContrastOmission {
                key: (*key).into(),
                label: (*label).into(),
                reason: reason.into(),
            });
            continue;
        }
        ranked += 1;
        let row = row.expect("matched ranked row");
        let gap = row.percentile_difference.expect("matched ranked gap");
        if gap == 0. {
            equal += 1;
            continue;
        }
        let limit = left
            .facets
            .iter()
            .find(|f| f.measurement.key == *key)
            .and_then(|f| f.explanation.as_ref())
            .map(|e| e.limit.clone())
            .unwrap_or_else(|| row.interpretation.clone());
        candidates.push((
            order,
            gap.abs(),
            ContrastHighlight {
                label: (*label).into(),
                measurement: row.clone(),
                limit,
            },
        ));
    }
    candidates.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ContrastSummary {
        schema: "icelines.rinkcraft-contrast-summary.v1".into(),
        facet_count: facets.len(), ranked_facets: ranked, equal_rank_facets: equal,
        highlights: candidates.into_iter().take(3).map(|(_, _, h)| h).collect(), omitted,
        selection_note: "Up to three nonzero gaps, ordered by absolute raw-value percentile difference among the fixed displayed attributes. Exact ties keep attribute order. Signs are left minus right. Higher means a larger recorded value, not necessarily better. No overall score, style/class inference or significance claim.".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::rinkcraft_numbers::{self, InputRow};
    fn catalog() -> rinkcraft_numbers::NumericalCatalog {
        let rows = (1..=25)
            .map(|id| InputRow {
                player_id: id,
                name: format!("Synthetic {id}"),
                group: "Forwards".into(),
                games: 30,
                seconds: 30000.,
                values: [
                    ("primary_assists".into(), id as f64),
                    ("shots".into(), id as f64 * 2.),
                    ("ixg".into(), id as f64 * 0.1),
                    ("unblocked_shots".into(), 100.),
                    ("hits".into(), id as f64),
                    ("goals".into(), id as f64),
                ]
                .into_iter()
                .collect(),
            })
            .collect::<Vec<_>>();
        rinkcraft_numbers::build(&rows, 20242025, &[], vec![]).unwrap()
    }
    #[test]
    fn stable_ties_signed_gaps_and_fixed_pool() {
        let mut c = catalog();
        let contrast = c.compare(25, 1).unwrap().contrast.unwrap();
        assert_eq!(contrast.facet_count, 6);
        assert_eq!(contrast.ranked_facets, 6);
        assert_eq!(
            contrast
                .highlights
                .iter()
                .map(|h| h.measurement.key.as_str())
                .collect::<Vec<_>>(),
            ["primary-assists-per-60", "shots-per-60", "ixg-per-60"]
        );
        assert_eq!(
            contrast.highlights[0].measurement.percentile_difference,
            Some(96.)
        );
        let reverse = c.compare(1, 25).unwrap().contrast.unwrap();
        assert_eq!(
            reverse.highlights[0].measurement.percentile_difference,
            Some(-96.)
        );
        // A nondisplayed measure cannot enter the headline pool.
        c.profiles[0]
            .metrics
            .iter_mut()
            .find(|m| m.key == "penalties-taken-per-60")
            .unwrap()
            .percentile = Some(100.);
        assert!(c
            .compare(1, 25)
            .unwrap()
            .contrast
            .unwrap()
            .highlights
            .iter()
            .all(|h| h.measurement.key != "penalties-taken-per-60"));
    }
    #[test]
    fn missing_unranked_and_equal_values_are_not_fake_contrasts() {
        let mut c = catalog();
        c.profiles[0].metrics.retain(|m| m.key != "shots-per-60");
        let a = c.profiles[0]
            .metrics
            .iter_mut()
            .find(|m| m.key == "primary-assists-per-60")
            .unwrap();
        a.state = "below_exposure".into();
        a.percentile = None;
        let contrast = c.compare(25, 1).unwrap().contrast.unwrap();
        assert_eq!(contrast.ranked_facets, 4);
        assert_eq!(contrast.omitted.len(), 2);
        assert!(contrast.highlights.iter().all(|h| !matches!(
            h.measurement.key.as_str(),
            "shots-per-60" | "primary-assists-per-60"
        )));
        let mut c = catalog();
        c.profiles[0].metrics = c
            .profiles
            .iter()
            .find(|p| p.player_id == 25)
            .unwrap()
            .metrics
            .clone();
        let contrast = c.compare(25, 1).unwrap().contrast.unwrap();
        assert!(contrast.highlights.is_empty());
        assert_eq!(contrast.equal_rank_facets, 6);
    }
    #[test]
    fn changed_definition_or_scale_refuses_comparison() {
        let mut c = catalog();
        c.profiles[0].metrics[0].formula = "different".into();
        assert!(c.compare(1, 25).is_err());
        let mut c = catalog();
        c.profiles[0].metrics[0].scale = 60.;
        assert!(c.compare(1, 25).is_err());
    }
}
