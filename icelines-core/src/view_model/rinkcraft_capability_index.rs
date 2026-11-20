//! Candidate attribute-battery arithmetic. Never a published player capability.
//! Inputs must already be context-adjusted, calibrated component quantiles.
use serde::{Deserialize, Serialize};
use statrs::distribution::{ContinuousCDF, Normal};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub id: String,
    pub name: String,
    pub weight: f64,
    pub measure: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Definition {
    pub id: String,
    pub name: String,
    pub branch: String,
    pub construct: String,
    pub current_coverage: String,
    pub components: Vec<Component>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rules {
    pub schema: String,
    pub status: String,
    pub source_registry_version: String,
    pub source_registry_sha256: String,
    pub reference_scope: String,
    pub quantile_knots: Vec<f64>,
    pub score_knots: Vec<f64>,
    pub release_gates: Vec<String>,
    pub attributes: Vec<Definition>,
}
pub fn rules() -> Rules {
    serde_json::from_str(include_str!("rinkcraft_capability_rules.json"))
        .expect("embedded capability draft")
}
#[derive(Debug, Serialize)]
pub struct CandidateIndex {
    pub status: String,
    pub attribute_id: String,
    pub balanced_index: f64,
    pub reference_quantile: f64,
    pub proposed_display_score: u8,
    pub reference_count: usize,
    pub note: String,
}
pub fn ordinal_score(q: f64) -> Result<u8, String> {
    if !q.is_finite() || !(0.0..=1.0).contains(&q) {
        return Err("invalid reference quantile".into());
    }
    let r = rules();
    let mut score = *r.score_knots.last().unwrap();
    for i in 0..r.quantile_knots.len() - 1 {
        let (a, b) = (r.quantile_knots[i], r.quantile_knots[i + 1]);
        if q <= b {
            score =
                r.score_knots[i] + (q - a) / (b - a) * (r.score_knots[i + 1] - r.score_knots[i]);
            break;
        }
    }
    Ok(score.round().clamp(1., 20.) as u8)
}
/// Research math only. A caller's complete quantiles/reference do not establish
/// source reliability, construct validity, uncertainty or publication authority.
pub fn candidate_index(
    id: &str,
    components: &[(String, f64)],
    reference: &[f64],
) -> Result<CandidateIndex, String> {
    let r = rules();
    let d = r
        .attributes
        .iter()
        .find(|a| a.id == id)
        .ok_or("unknown attribute")?;
    if reference.len() < 20 || reference.iter().any(|n| !n.is_finite()) {
        return Err(
            "complete calibrated reference of at least 20 required for this math helper".into(),
        );
    }
    let mut supplied = BTreeMap::new();
    for (id, q) in components {
        if !q.is_finite() || !(0.0..=1.0).contains(q) || supplied.insert(id.as_str(), *q).is_some()
        {
            return Err("duplicate/invalid component quantile".into());
        }
    }
    let required = d
        .components
        .iter()
        .map(|c| c.id.as_str())
        .collect::<BTreeSet<_>>();
    if supplied.keys().copied().collect::<BTreeSet<_>>() != required {
        return Err(
            "every required component must be supplied; no available-facet renormalization".into(),
        );
    }
    let normal = Normal::new(0., 1.).map_err(|e| e.to_string())?;
    let h = d
        .components
        .iter()
        .map(|c| c.weight * normal.inverse_cdf(supplied[c.id.as_str()].clamp(0.005, 0.995)))
        .sum::<f64>();
    let below = reference.iter().filter(|x| **x < h).count();
    let tied = reference.iter().filter(|x| **x == h).count();
    let q = (below as f64 + 0.5 * tied as f64) / reference.len() as f64;
    Ok(CandidateIndex{status:"unvalidated_candidate".into(),attribute_id:id.into(),balanced_index:h,reference_quantile:q,proposed_display_score:ordinal_score(q)?,reference_count:reference.len(),note:"Research-only arithmetic, not a released player grade. Component models, full-cohort calibration, joint uncertainty, held-out validation and independent methods review remain required. No D&D modifier or simulation bonus.".into()})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ordinal_anchors_and_invalid_inputs() {
        for (q, s) in [
            (0., 1),
            (0.05, 4),
            (0.25, 8),
            (0.5, 10),
            (0.7, 12),
            (0.9, 16),
            (0.97, 18),
            (0.995, 20),
            (1., 20),
        ] {
            assert_eq!(ordinal_score(q).unwrap(), s);
        }
        for q in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
            assert!(ordinal_score(q).is_err());
        }
    }
    #[test]
    fn all_twelve_batteries_are_complete_and_balanced() {
        let r = rules();
        assert_eq!(r.attributes.len(), 12);
        for d in r.attributes {
            assert!((d.components.iter().map(|c| c.weight).sum::<f64>() - 1.).abs() < 1e-10);
            let components = d
                .components
                .iter()
                .map(|c| (c.id.clone(), 0.5))
                .collect::<Vec<_>>();
            let x = candidate_index(&d.id, &components, &[0.; 20]).unwrap();
            assert_eq!(x.proposed_display_score, 10);
            assert_eq!(x.status, "unvalidated_candidate");
        }
    }
    #[test]
    fn missing_duplicate_and_bad_references_are_not_scores() {
        let d = rules().attributes.remove(0);
        let c = d
            .components
            .iter()
            .map(|c| (c.id.clone(), 0.5))
            .collect::<Vec<_>>();
        assert!(candidate_index(&d.id, &c[..c.len() - 1], &[0.; 20]).is_err());
        let mut twice = c.clone();
        twice.push(c[0].clone());
        assert!(candidate_index(&d.id, &twice, &[0.; 20]).is_err());
        assert!(candidate_index(&d.id, &c, &[0.; 19]).is_err());
        assert!(candidate_index(&d.id, &c, &[f64::NAN; 20]).is_err());
    }
    #[test]
    fn independent_normal_quantile_and_empirical_reference_examples() {
        // NormalDist().inv_cdf(.75) independently gives this value. The
        // reference has 27/41 observations below it, mapping to ordinal 12.
        let d = rules().attributes.remove(0);
        let reference = (0..41).map(|i| -2. + i as f64 / 10.).collect::<Vec<_>>();
        let c = d
            .components
            .iter()
            .map(|c| (c.id.clone(), 0.75))
            .collect::<Vec<_>>();
        let score = candidate_index(&d.id, &c, &reference).unwrap();
        assert!((score.balanced_index - 0.6744897501960817).abs() < 1e-12);
        assert!((score.reference_quantile - 27. / 41.).abs() < 1e-12);
        assert_eq!(score.proposed_display_score, 12);
        let mut previous = 0;
        for i in 0..=1000 {
            let s = ordinal_score(i as f64 / 1000.).unwrap();
            assert!(s >= previous);
            previous = s;
        }
    }
}
