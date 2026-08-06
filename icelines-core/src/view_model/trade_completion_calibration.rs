use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

pub const TRADE_COMPLETION_CALIBRATION_SCHEMA: &str = "trade_completion_calibration.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeCompletionObservationInput {
    pub proposal_id: String,
    pub proposal_at: String,
    pub evidence_as_of: String,
    pub resolved_at: String,
    pub predicted_probability: f64,
    pub completed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeCompletionCalibrationInput {
    pub as_of: String,
    pub evaluation_start: String,
    pub evaluation_end: String,
    pub model_id: String,
    pub model_method: String,
    pub model_trained_through: String,
    pub training_fingerprint: String,
    pub bin_count: usize,
    pub observations: Vec<TradeCompletionObservationInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeCompletionCalibrationBinView {
    pub probability_low: f64,
    pub probability_high: f64,
    pub observations: usize,
    pub mean_predicted_probability: f64,
    pub completion_rate: f64,
    pub absolute_gap: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeCompletionCalibrationView {
    pub schema: String,
    pub as_of: String,
    pub evaluation_start: String,
    pub evaluation_end: String,
    pub model_id: String,
    pub model_method: String,
    pub model_trained_through: String,
    pub training_fingerprint: String,
    pub observations: usize,
    pub completions: usize,
    pub completion_rate: f64,
    pub brier_score: f64,
    pub log_loss: f64,
    pub expected_calibration_error: f64,
    pub bins: Vec<TradeCompletionCalibrationBinView>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TradeCompletionCalibrationError {
    #[error("invalid trade-completion calibration input: {0}")]
    InvalidInput(String),
}

pub fn calibrate_trade_completion(
    input: TradeCompletionCalibrationInput,
) -> Result<TradeCompletionCalibrationView, TradeCompletionCalibrationError> {
    let as_of = parse_timestamp("as_of", &input.as_of)?;
    let evaluation_start = parse_timestamp("evaluation_start", &input.evaluation_start)?;
    let evaluation_end = parse_timestamp("evaluation_end", &input.evaluation_end)?;
    let model_trained_through =
        parse_timestamp("model_trained_through", &input.model_trained_through)?;
    if evaluation_start > evaluation_end || evaluation_end > as_of {
        return Err(TradeCompletionCalibrationError::InvalidInput(
            "evaluation_start must be at or before evaluation_end, and evaluation_end must not exceed as_of"
                .to_owned(),
        ));
    }
    if !(2..=20).contains(&input.bin_count) || input.observations.is_empty() {
        return Err(TradeCompletionCalibrationError::InvalidInput(
            "bin_count must be 2-20 and observations must not be empty".to_owned(),
        ));
    }
    if input.model_id.trim().is_empty()
        || input.model_method.trim().is_empty()
        || model_trained_through >= evaluation_start
        || !valid_sha256(&input.training_fingerprint)
    {
        return Err(TradeCompletionCalibrationError::InvalidInput(
            "model identity/method are required, model_trained_through must precede the evaluation window, and training_fingerprint must be a SHA-256 hex digest"
                .to_owned(),
        ));
    }
    let mut ids = BTreeSet::new();
    for observation in &input.observations {
        let proposal_at = parse_timestamp("proposal_at", &observation.proposal_at)?;
        let evidence_as_of = parse_timestamp("evidence_as_of", &observation.evidence_as_of)?;
        let resolved_at = parse_timestamp("resolved_at", &observation.resolved_at)?;
        if observation.proposal_id.trim().is_empty()
            || !ids.insert(observation.proposal_id.as_str())
            || !observation.predicted_probability.is_finite()
            || !(0.0..=1.0).contains(&observation.predicted_probability)
            || evidence_as_of > proposal_at
            || proposal_at < evaluation_start
            || proposal_at > evaluation_end
            || resolved_at < proposal_at
            || resolved_at > as_of
        {
            return Err(TradeCompletionCalibrationError::InvalidInput(format!(
                "observation {} requires a unique ID, probability in 0-1, evidence no later than proposal, proposal inside the evaluation window, and resolution between proposal and as_of",
                observation.proposal_id
            )));
        }
    }

    let count = input.observations.len();
    let completions = input
        .observations
        .iter()
        .filter(|observation| observation.completed)
        .count();
    let brier_score = input
        .observations
        .iter()
        .map(|observation| {
            let outcome = f64::from(observation.completed);
            (observation.predicted_probability - outcome).powi(2)
        })
        .sum::<f64>()
        / count as f64;
    let log_loss = input
        .observations
        .iter()
        .map(|observation| {
            let probability = observation.predicted_probability.clamp(1e-15, 1.0 - 1e-15);
            if observation.completed {
                -probability.ln()
            } else {
                -(1.0 - probability).ln()
            }
        })
        .sum::<f64>()
        / count as f64;

    let mut bins = Vec::new();
    let mut expected_calibration_error = 0.0;
    for index in 0..input.bin_count {
        let low = index as f64 / input.bin_count as f64;
        let high = (index + 1) as f64 / input.bin_count as f64;
        let members = input
            .observations
            .iter()
            .filter(|observation| {
                observation.predicted_probability >= low
                    && (index + 1 == input.bin_count || observation.predicted_probability < high)
            })
            .collect::<Vec<_>>();
        if members.is_empty() {
            continue;
        }
        let member_count = members.len();
        let mean_predicted_probability = members
            .iter()
            .map(|observation| observation.predicted_probability)
            .sum::<f64>()
            / member_count as f64;
        let completion_rate = members
            .iter()
            .filter(|observation| observation.completed)
            .count() as f64
            / member_count as f64;
        let absolute_gap = (mean_predicted_probability - completion_rate).abs();
        expected_calibration_error += member_count as f64 / count as f64 * absolute_gap;
        bins.push(TradeCompletionCalibrationBinView {
            probability_low: low,
            probability_high: high,
            observations: member_count,
            mean_predicted_probability,
            completion_rate,
            absolute_gap,
        });
    }

    Ok(TradeCompletionCalibrationView {
        schema: TRADE_COMPLETION_CALIBRATION_SCHEMA.to_owned(),
        as_of: input.as_of,
        evaluation_start: input.evaluation_start,
        evaluation_end: input.evaluation_end,
        model_id: input.model_id,
        model_method: input.model_method,
        model_trained_through: input.model_trained_through,
        training_fingerprint: input.training_fingerprint,
        observations: count,
        completions,
        completion_rate: completions as f64 / count as f64,
        brier_score,
        log_loss,
        expected_calibration_error,
        bins,
        disclosures: vec![
            "Every label must come from a reviewed proposal cohort with evidence frozen no later than proposal time; later player performance is excluded from completion scoring."
                .to_owned(),
            "Completed transaction feeds provide positive outcomes but cannot establish failed negotiations; IceLines does not synthesize negative labels from absent trades."
                .to_owned(),
            "Calibration describes the supplied cohort and does not train or authorize a completion model."
                .to_owned(),
        ],
    })
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn parse_timestamp(
    field: &str,
    value: &str,
) -> Result<DateTime<FixedOffset>, TradeCompletionCalibrationError> {
    DateTime::parse_from_rfc3339(value).map_err(|_| {
        TradeCompletionCalibrationError::InvalidInput(format!(
            "{field} must be an RFC3339 timestamp"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(id: &str, probability: f64, completed: bool) -> TradeCompletionObservationInput {
        TradeCompletionObservationInput {
            proposal_id: id.to_owned(),
            proposal_at: "2025-03-01T12:00:00-05:00".to_owned(),
            evidence_as_of: "2025-03-01T11:00:00-05:00".to_owned(),
            resolved_at: "2025-03-07T15:00:00-05:00".to_owned(),
            predicted_probability: probability,
            completed,
        }
    }

    fn input() -> TradeCompletionCalibrationInput {
        TradeCompletionCalibrationInput {
            as_of: "2025-04-01T00:00:00-04:00".to_owned(),
            evaluation_start: "2025-02-01T00:00:00-05:00".to_owned(),
            evaluation_end: "2025-03-07T15:00:00-05:00".to_owned(),
            model_id: "trade-completion-evaluation-v1".to_owned(),
            model_method: "frozen logistic completion model".to_owned(),
            model_trained_through: "2025-01-31T23:59:59-05:00".to_owned(),
            training_fingerprint:
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_owned(),
            bin_count: 5,
            observations: vec![
                observation("likely-completed", 0.8, true),
                observation("unlikely-failed", 0.2, false),
            ],
        }
    }

    #[test]
    fn calibrated_cohort_reports_proper_scores_and_bins() {
        let view = calibrate_trade_completion(input()).unwrap();
        assert_eq!(view.schema, TRADE_COMPLETION_CALIBRATION_SCHEMA);
        assert_eq!(view.observations, 2);
        assert_eq!(view.completions, 1);
        assert!((view.completion_rate - 0.5).abs() < 1e-12);
        assert!((view.brier_score - 0.04).abs() < 1e-12);
        assert!((view.log_loss - (-0.8_f64.ln())).abs() < 1e-12);
        assert!((view.expected_calibration_error - 0.2).abs() < 1e-12);
        assert_eq!(
            view.bins.iter().map(|bin| bin.observations).sum::<usize>(),
            2
        );
    }

    #[test]
    fn calibration_rejects_future_evidence_and_duplicate_ids() {
        let mut invalid = input();
        invalid.observations[0].evidence_as_of = "2025-03-02T00:00:00-05:00".to_owned();
        assert!(calibrate_trade_completion(invalid).is_err());

        let mut invalid = input();
        invalid.observations[1].proposal_id = invalid.observations[0].proposal_id.clone();
        assert!(calibrate_trade_completion(invalid).is_err());
    }

    #[test]
    fn calibration_rejects_model_trained_inside_evaluation_window() {
        let mut invalid = input();
        invalid.model_trained_through = "2025-02-15T00:00:00-05:00".to_owned();
        assert!(calibrate_trade_completion(invalid).is_err());

        let mut invalid = input();
        invalid.training_fingerprint = "not-a-fingerprint".to_owned();
        assert!(calibrate_trade_completion(invalid).is_err());
    }
}
