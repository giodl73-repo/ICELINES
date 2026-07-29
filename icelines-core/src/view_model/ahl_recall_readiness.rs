//! Confidence-aware AHL-to-NHL recall readiness.
//!
//! This evaluation index combines distinct evidence signals. It is not a
//! calibrated probability that a player will be recalled or succeed in the NHL.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const AHL_RECALL_READINESS_POLICY_SCHEMA: &str = "ahl_recall_readiness_policy.v1";
pub const AHL_RECALL_READINESS_METHOD: &str = "weighted_value_experience_camp.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlRecallReadinessPolicy {
    pub schema: String,
    pub method_version: String,
    pub value_weight: f64,
    pub nhl_experience_weight: f64,
    pub camp_proximity_weight: f64,
    pub nhl_experience_games: u32,
    pub camp_evidence_confidence: f64,
    pub minimum_coverage: f64,
}

impl Default for AhlRecallReadinessPolicy {
    fn default() -> Self {
        Self {
            schema: AHL_RECALL_READINESS_POLICY_SCHEMA.to_owned(),
            method_version: AHL_RECALL_READINESS_METHOD.to_owned(),
            value_weight: 0.5,
            nhl_experience_weight: 0.3,
            camp_proximity_weight: 0.2,
            nhl_experience_games: 50,
            camp_evidence_confidence: 0.5,
            minimum_coverage: 0.7,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlRecallReadinessInput {
    pub value_percentile: Option<f64>,
    pub value_evidence_confidence: Option<f64>,
    pub nhl_regular_season_games: Option<u32>,
    pub camp_make_probability: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlRecallReadinessEstimate {
    pub method_version: String,
    pub readiness_index: Option<f64>,
    pub evidence_confidence: f64,
    pub coverage: f64,
    pub value_signal: Option<f64>,
    pub nhl_experience_signal: Option<f64>,
    pub camp_proximity_signal: Option<f64>,
}

pub fn estimate_ahl_recall_readiness(
    policy: &AhlRecallReadinessPolicy,
    input: &AhlRecallReadinessInput,
) -> Result<AhlRecallReadinessEstimate, String> {
    validate_policy(policy)?;
    validate_probability(input.value_percentile, "value percentile")?;
    validate_probability(input.value_evidence_confidence, "value evidence confidence")?;
    validate_probability(input.camp_make_probability, "camp make probability")?;
    if input.value_percentile.is_some() != input.value_evidence_confidence.is_some() {
        return Err("recall readiness value signal and confidence must appear together".to_owned());
    }

    let total_weight =
        policy.value_weight + policy.nhl_experience_weight + policy.camp_proximity_weight;
    let nhl_experience_signal = input
        .nhl_regular_season_games
        .map(|games| (f64::from(games) / f64::from(policy.nhl_experience_games)).clamp(0.0, 1.0));
    let mut weighted_score = 0.0;
    let mut available_weight = 0.0;
    let mut confidence_weight = 0.0;
    if let (Some(signal), Some(confidence)) =
        (input.value_percentile, input.value_evidence_confidence)
    {
        weighted_score += signal * policy.value_weight;
        available_weight += policy.value_weight;
        confidence_weight += confidence * policy.value_weight;
    }
    if let Some(signal) = nhl_experience_signal {
        weighted_score += signal * policy.nhl_experience_weight;
        available_weight += policy.nhl_experience_weight;
        confidence_weight += policy.nhl_experience_weight;
    }
    if let Some(signal) = input.camp_make_probability {
        weighted_score += signal * policy.camp_proximity_weight;
        available_weight += policy.camp_proximity_weight;
        confidence_weight += policy.camp_evidence_confidence * policy.camp_proximity_weight;
    }
    let coverage = available_weight / total_weight;
    let readiness_index = (available_weight > 0.0 && coverage >= policy.minimum_coverage)
        .then_some(weighted_score / available_weight);
    Ok(AhlRecallReadinessEstimate {
        method_version: policy.method_version.clone(),
        readiness_index,
        evidence_confidence: confidence_weight / total_weight,
        coverage,
        value_signal: input.value_percentile,
        nhl_experience_signal,
        camp_proximity_signal: input.camp_make_probability,
    })
}

/// Return deterministic ascending empirical midrank percentiles keyed by
/// canonical player ID. A one-player cohort receives the neutral 0.5 value.
pub fn empirical_midrank_percentiles(values: &[(u32, f64)]) -> Result<BTreeMap<u32, f64>, String> {
    if values.is_empty() {
        return Ok(BTreeMap::new());
    }
    if values.iter().any(|(_, value)| !value.is_finite()) {
        return Err("recall-readiness percentile cohort contains a non-finite value".to_owned());
    }
    if values
        .iter()
        .map(|(player_id, _)| *player_id)
        .collect::<BTreeSet<_>>()
        .len()
        != values.len()
    {
        return Err("recall-readiness percentile cohort contains duplicate player IDs".to_owned());
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|left, right| {
        left.1
            .total_cmp(&right.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    let denominator = sorted.len().saturating_sub(1) as f64;
    let mut output = BTreeMap::new();
    let mut start = 0usize;
    while start < sorted.len() {
        let mut end = start + 1;
        while end < sorted.len() && sorted[end].1.total_cmp(&sorted[start].1).is_eq() {
            end += 1;
        }
        let percentile = if denominator == 0.0 {
            0.5
        } else {
            let midrank_zero_based = (start as f64 + (end - 1) as f64) / 2.0;
            midrank_zero_based / denominator
        };
        for (player_id, _) in &sorted[start..end] {
            output.insert(*player_id, percentile);
        }
        start = end;
    }
    Ok(output)
}

fn validate_policy(policy: &AhlRecallReadinessPolicy) -> Result<(), String> {
    let weights = [
        policy.value_weight,
        policy.nhl_experience_weight,
        policy.camp_proximity_weight,
    ];
    if policy.schema != AHL_RECALL_READINESS_POLICY_SCHEMA
        || policy.method_version != AHL_RECALL_READINESS_METHOD
        || weights
            .iter()
            .any(|weight| !weight.is_finite() || *weight < 0.0)
        || weights.iter().sum::<f64>() <= 0.0
        || policy.nhl_experience_games == 0
        || !policy.camp_evidence_confidence.is_finite()
        || !(0.0..=1.0).contains(&policy.camp_evidence_confidence)
        || !policy.minimum_coverage.is_finite()
        || !(0.0..=1.0).contains(&policy.minimum_coverage)
    {
        return Err("invalid AHL recall-readiness policy".to_owned());
    }
    Ok(())
}

fn validate_probability(value: Option<f64>, label: &str) -> Result<(), String> {
    if value.is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value)) {
        return Err(format!("invalid recall readiness {label}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_value_keeps_score_confidence_and_coverage_separate() {
        let estimate = estimate_ahl_recall_readiness(
            &AhlRecallReadinessPolicy::default(),
            &AhlRecallReadinessInput {
                value_percentile: Some(0.8),
                value_evidence_confidence: Some(0.6),
                nhl_regular_season_games: Some(25),
                camp_make_probability: Some(0.4),
            },
        )
        .unwrap();
        // (0.8*0.5 + 0.5*0.3 + 0.4*0.2) / 1.0 = 0.63.
        assert!((estimate.readiness_index.unwrap() - 0.63).abs() < 1e-12);
        // Confidence = 0.6*0.5 + 1.0*0.3 + 0.5*0.2 = 0.70.
        assert!((estimate.evidence_confidence - 0.70).abs() < 1e-12);
        assert!((estimate.coverage - 1.0).abs() < 1e-12);
    }

    #[test]
    fn missing_camp_is_renormalized_but_reduces_coverage_and_confidence() {
        let estimate = estimate_ahl_recall_readiness(
            &AhlRecallReadinessPolicy::default(),
            &AhlRecallReadinessInput {
                value_percentile: Some(0.8),
                value_evidence_confidence: Some(0.6),
                nhl_regular_season_games: Some(25),
                camp_make_probability: None,
            },
        )
        .unwrap();
        // (0.8*0.5 + 0.5*0.3) / 0.8 = 0.6875.
        assert!((estimate.readiness_index.unwrap() - 0.6875).abs() < 1e-12);
        assert!((estimate.coverage - 0.8).abs() < 1e-12);
        assert!((estimate.evidence_confidence - 0.6).abs() < 1e-12);
    }

    #[test]
    fn low_coverage_evidence_does_not_publish_an_index() {
        let estimate = estimate_ahl_recall_readiness(
            &AhlRecallReadinessPolicy::default(),
            &AhlRecallReadinessInput {
                value_percentile: None,
                value_evidence_confidence: None,
                nhl_regular_season_games: Some(0),
                camp_make_probability: Some(0.9),
            },
        )
        .unwrap();
        assert_eq!(estimate.readiness_index, None);
        assert!((estimate.coverage - 0.5).abs() < 1e-12);
    }

    #[test]
    fn value_signal_requires_its_confidence() {
        let input = AhlRecallReadinessInput {
            value_percentile: Some(0.8),
            value_evidence_confidence: None,
            nhl_regular_season_games: Some(0),
            camp_make_probability: None,
        };
        assert!(
            estimate_ahl_recall_readiness(&AhlRecallReadinessPolicy::default(), &input).is_err()
        );
    }

    #[test]
    fn empirical_percentiles_are_tied_and_order_invariant() {
        let values = [(3, 30.0), (1, 10.0), (4, 30.0), (2, 20.0)];
        let percentiles = empirical_midrank_percentiles(&values).unwrap();
        assert_eq!(percentiles[&1], 0.0);
        assert!((percentiles[&2] - (1.0 / 3.0)).abs() < 1e-12);
        assert!((percentiles[&3] - (5.0 / 6.0)).abs() < 1e-12);
        assert_eq!(percentiles[&3], percentiles[&4]);
    }
}
