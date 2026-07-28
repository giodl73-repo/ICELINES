//! Leakage-aware descriptive calibration for The Window.

use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::organization_window::{
    load_organization_window_profile_inventory, validate_organization_window_board,
    OrganizationWindowBoardView,
};

pub const ORGANIZATION_WINDOW_CALIBRATION_SCHEMA: &str = "organization_window_calibration.v1";
pub const ORGANIZATION_WINDOW_ROLLING_CALIBRATION_SCHEMA: &str =
    "organization_window_rolling_calibration.v1";
pub const ORGANIZATION_WINDOW_ROLLING_CALIBRATION_JSON_SCHEMA: &str =
    include_str!("../../../design/schemas/organization_window_rolling_calibration.v1.schema.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowCalibrationClaimStatus {
    Calibrated,
    Inconclusive,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowOutcomeRow {
    pub organization: String,
    /// Continuous target normalized to the same 0..=100 interpretation scale.
    pub target_value: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowLeakageAuditRow {
    pub profile_key: String,
    pub method_version: String,
    pub point_in_time_safe: bool,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowCalibrationMetricView {
    pub key: String,
    pub sample_size: usize,
    pub mean_absolute_error: f64,
    pub baseline_mean_absolute_error: f64,
    pub rank_correlation: Option<f64>,
    pub claim_status: WindowCalibrationClaimStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationWindowCalibrationView {
    pub schema: String,
    pub target: String,
    pub board_fingerprint: String,
    pub manifest_fingerprint: String,
    pub leakage_audit: Vec<WindowLeakageAuditRow>,
    pub overall: WindowCalibrationMetricView,
    pub dimensions: Vec<WindowCalibrationMetricView>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowCalibrationOriginInput {
    pub origin_id: String,
    pub board: OrganizationWindowBoardView,
    pub outcomes: Vec<WindowOutcomeRow>,
    pub leakage_audit: Vec<WindowLeakageAuditRow>,
    /// Frozen point-in-time constant or simple-model prediction for this
    /// origin, repeated across its cohort.
    pub baseline_value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowCalibrationOriginView {
    pub origin_id: String,
    pub season: u32,
    pub as_of: NaiveDate,
    pub board_fingerprint: String,
    pub leakage_blocked: bool,
    pub overall: WindowCalibrationMetricView,
    pub dimensions: Vec<WindowCalibrationMetricView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowCalibrationAblationView {
    pub excluded_dimension: String,
    pub sample_size: usize,
    pub mean_absolute_error: f64,
    pub delta_from_full_mae: f64,
    pub rank_correlation: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowOrganizationStabilityView {
    pub organization: String,
    pub origins: usize,
    pub mean_absolute_error: f64,
    pub rank_correlation: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowTrialNoiseStatus {
    NotProvided,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowCalibrationUncertaintyView {
    pub origin_count: usize,
    pub between_origin_mae_standard_deviation: f64,
    pub mean_mae_confidence_interval_low: f64,
    pub mean_mae_confidence_interval_high: f64,
    pub trial_noise_status: WindowTrialNoiseStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationWindowRollingCalibrationView {
    pub schema: String,
    pub target: String,
    pub manifest_fingerprint: String,
    pub origins: Vec<WindowCalibrationOriginView>,
    pub overall: WindowCalibrationMetricView,
    pub dimensions: Vec<WindowCalibrationMetricView>,
    pub ablations: Vec<WindowCalibrationAblationView>,
    pub organization_stability: Vec<WindowOrganizationStabilityView>,
    pub uncertainty: WindowCalibrationUncertaintyView,
    pub claim_status: WindowCalibrationClaimStatus,
    pub disclosures: Vec<String>,
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OrganizationWindowCalibrationError {
    #[error("Window calibration target is empty")]
    EmptyTarget,
    #[error("Window calibration requires a complete ranked board")]
    UnrankedBoard,
    #[error("Window calibration outcomes do not exactly match the board cohort")]
    OutcomeCohortMismatch,
    #[error("Window calibration contains duplicate organization {0}")]
    DuplicateOutcome(String),
    #[error("Window calibration target is outside 0..=100 for {0}")]
    InvalidOutcome(String),
    #[error("Window leakage audit is incomplete for {0}")]
    IncompleteLeakageAudit(String),
    #[error("rolling Window calibration requires at least {required} origins; found {found}")]
    InsufficientOrigins { required: usize, found: usize },
    #[error("rolling Window calibration origin id is empty or duplicated: {0}")]
    InvalidOrigin(String),
    #[error("rolling Window calibration mixes manifest fingerprints")]
    MixedManifest,
    #[error("rolling Window calibration baseline is invalid for {0}")]
    InvalidBaseline(String),
    #[error("rolling Window calibration board fingerprint is invalid for {0}")]
    InvalidBoardFingerprint(String),
    #[error("rolling Window calibration is missing dimension {dimension} for {organization}")]
    IncompleteDimension {
        organization: String,
        dimension: String,
    },
    #[error("rolling Window calibration serialization failed: {0}")]
    Serialization(String),
}

pub fn calibrate_organization_window(
    target: &str,
    board: &OrganizationWindowBoardView,
    outcomes: &[WindowOutcomeRow],
    leakage_audit: &[WindowLeakageAuditRow],
) -> Result<OrganizationWindowCalibrationView, OrganizationWindowCalibrationError> {
    if target.trim().is_empty() {
        return Err(OrganizationWindowCalibrationError::EmptyTarget);
    }
    let inventory = load_organization_window_profile_inventory().map_err(|_| {
        OrganizationWindowCalibrationError::InvalidBoardFingerprint(board.fingerprint.clone())
    })?;
    validate_organization_window_board(board, &inventory).map_err(|_| {
        OrganizationWindowCalibrationError::InvalidBoardFingerprint(board.fingerprint.clone())
    })?;
    if board
        .organizations
        .iter()
        .any(|row| row.overall.score.is_none() || row.overall.rank.is_none())
    {
        return Err(OrganizationWindowCalibrationError::UnrankedBoard);
    }
    let outcome_map = outcome_map(outcomes)?;
    let expected = board
        .expected_organizations
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if outcome_map.keys().copied().collect::<BTreeSet<_>>() != expected {
        return Err(OrganizationWindowCalibrationError::OutcomeCohortMismatch);
    }
    validate_leakage_audit(board, leakage_audit)?;
    let leakage_blocked = leakage_audit.iter().any(|row| !row.point_in_time_safe);

    let actual = board
        .organizations
        .iter()
        .map(|row| outcome_map[row.organization.as_str()])
        .collect::<Vec<_>>();
    let predicted = board
        .organizations
        .iter()
        .map(|row| row.overall.score.unwrap())
        .collect::<Vec<_>>();
    let overall = metric("overall", &predicted, &actual, leakage_blocked);

    let dimension_keys = board.organizations[0]
        .dimensions
        .iter()
        .map(|row| row.key.clone())
        .collect::<Vec<_>>();
    let dimensions = dimension_keys
        .iter()
        .filter_map(|key| {
            let values = board
                .organizations
                .iter()
                .map(|organization| {
                    organization
                        .dimensions
                        .iter()
                        .find(|row| row.key == *key)
                        .and_then(|row| row.score)
                })
                .collect::<Option<Vec<_>>>()?;
            Some(metric(key, &values, &actual, leakage_blocked))
        })
        .collect();

    Ok(OrganizationWindowCalibrationView {
        schema: ORGANIZATION_WINDOW_CALIBRATION_SCHEMA.to_owned(),
        target: target.to_owned(),
        board_fingerprint: board.fingerprint.clone(),
        manifest_fingerprint: board.manifest.fingerprint.clone(),
        leakage_audit: leakage_audit.to_vec(),
        overall,
        dimensions,
        disclosures: vec![
            "This artifact evaluates a continuous 0..100 target; probability claims require separate Brier/log-loss calibration.".to_owned(),
            "Calibrated status requires both a safe point-in-time audit and improvement over the constant-mean baseline.".to_owned(),
            "One checkpoint is diagnostic evidence, not a multi-season predictive claim.".to_owned(),
        ],
    })
}

pub fn calibrate_organization_window_rolling_origins(
    target: &str,
    origins: &[WindowCalibrationOriginInput],
    minimum_origins: usize,
) -> Result<OrganizationWindowRollingCalibrationView, OrganizationWindowCalibrationError> {
    if target.trim().is_empty() {
        return Err(OrganizationWindowCalibrationError::EmptyTarget);
    }
    let required = minimum_origins.max(2);
    if origins.len() < required {
        return Err(OrganizationWindowCalibrationError::InsufficientOrigins {
            required,
            found: origins.len(),
        });
    }
    let mut ordered = origins.iter().collect::<Vec<_>>();
    ordered.sort_by(|a, b| {
        (a.board.season, a.board.as_of, a.origin_id.as_str()).cmp(&(
            b.board.season,
            b.board.as_of,
            b.origin_id.as_str(),
        ))
    });
    let mut origin_ids = BTreeSet::new();
    let manifest_fingerprint = ordered[0].board.manifest.fingerprint.clone();
    let dimension_keys = ordered[0]
        .board
        .manifest
        .dimensions
        .iter()
        .map(|dimension| dimension.key.clone())
        .collect::<Vec<_>>();
    let mut origin_views = Vec::with_capacity(ordered.len());
    let mut all_predictions = Vec::new();
    let mut all_actual = Vec::new();
    let mut all_baselines = Vec::new();
    let mut dimension_predictions = dimension_keys
        .iter()
        .map(|key| (key.clone(), Vec::new()))
        .collect::<BTreeMap<_, _>>();
    let mut organization_samples = BTreeMap::<String, (Vec<f64>, Vec<f64>)>::new();
    let mut origin_maes = Vec::with_capacity(ordered.len());
    let mut leakage_blocked = false;

    for origin in ordered {
        if origin.origin_id.trim().is_empty() || !origin_ids.insert(origin.origin_id.clone()) {
            return Err(OrganizationWindowCalibrationError::InvalidOrigin(
                origin.origin_id.clone(),
            ));
        }
        if !origin.baseline_value.is_finite() || !(0.0..=100.0).contains(&origin.baseline_value) {
            return Err(OrganizationWindowCalibrationError::InvalidBaseline(
                origin.origin_id.clone(),
            ));
        }
        if origin.board.manifest.fingerprint != manifest_fingerprint {
            return Err(OrganizationWindowCalibrationError::MixedManifest);
        }
        calibrate_organization_window(
            target,
            &origin.board,
            &origin.outcomes,
            &origin.leakage_audit,
        )?;
        let outcomes = outcome_map(&origin.outcomes)?;
        let actual = origin
            .board
            .organizations
            .iter()
            .map(|organization| outcomes[organization.organization.as_str()])
            .collect::<Vec<_>>();
        let predicted = origin
            .board
            .organizations
            .iter()
            .map(|organization| organization.overall.score.unwrap())
            .collect::<Vec<_>>();
        let baselines = vec![origin.baseline_value; actual.len()];
        let origin_blocked = origin
            .leakage_audit
            .iter()
            .any(|audit| !audit.point_in_time_safe);
        leakage_blocked |= origin_blocked;
        let overall =
            metric_with_baseline("overall", &predicted, &actual, &baselines, origin_blocked);
        origin_maes.push(overall.mean_absolute_error);
        let mut dimensions = Vec::with_capacity(dimension_keys.len());
        for key in &dimension_keys {
            let values = origin
                .board
                .organizations
                .iter()
                .map(|organization| {
                    organization
                        .dimensions
                        .iter()
                        .find(|dimension| dimension.key == *key)
                        .and_then(|dimension| dimension.score)
                        .ok_or_else(|| OrganizationWindowCalibrationError::IncompleteDimension {
                            organization: organization.organization.clone(),
                            dimension: key.clone(),
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;
            dimension_predictions
                .get_mut(key)
                .ok_or_else(|| OrganizationWindowCalibrationError::IncompleteDimension {
                    organization: "rolling calibration".to_owned(),
                    dimension: key.clone(),
                })?
                .extend(values.iter().copied());
            dimensions.push(metric_with_baseline(
                key,
                &values,
                &actual,
                &baselines,
                origin_blocked,
            ));
        }
        for ((organization, prediction), outcome) in origin
            .board
            .organizations
            .iter()
            .zip(&predicted)
            .zip(&actual)
        {
            let samples = organization_samples
                .entry(organization.organization.clone())
                .or_default();
            samples.0.push(*prediction);
            samples.1.push(*outcome);
        }
        all_predictions.extend(predicted);
        all_actual.extend(actual);
        all_baselines.extend(baselines);
        origin_views.push(WindowCalibrationOriginView {
            origin_id: origin.origin_id.clone(),
            season: origin.board.season,
            as_of: origin.board.as_of,
            board_fingerprint: origin.board.fingerprint.clone(),
            leakage_blocked: origin_blocked,
            overall,
            dimensions,
        });
    }

    let overall = metric_with_baseline(
        "overall",
        &all_predictions,
        &all_actual,
        &all_baselines,
        leakage_blocked,
    );
    let dimensions = dimension_keys
        .iter()
        .map(|key| {
            metric_with_baseline(
                key,
                &dimension_predictions[key],
                &all_actual,
                &all_baselines,
                leakage_blocked,
            )
        })
        .collect::<Vec<_>>();
    let ablations = build_dimension_ablations(
        origins,
        &all_actual,
        overall.mean_absolute_error,
        &dimension_keys,
    )?;
    let organization_stability = organization_samples
        .into_iter()
        .map(
            |(organization, (predicted, actual))| WindowOrganizationStabilityView {
                organization,
                origins: predicted.len(),
                mean_absolute_error: mean_absolute_error(&predicted, &actual),
                rank_correlation: pearson(&ranks(&predicted), &ranks(&actual)),
            },
        )
        .collect::<Vec<_>>();
    let uncertainty = between_origin_uncertainty(&origin_maes);
    let claim_status = if leakage_blocked {
        WindowCalibrationClaimStatus::Blocked
    } else if origins.len() >= required
        && overall.mean_absolute_error < overall.baseline_mean_absolute_error
        && overall.rank_correlation.is_some_and(|value| value >= 0.3)
    {
        WindowCalibrationClaimStatus::Calibrated
    } else {
        WindowCalibrationClaimStatus::Inconclusive
    };
    let mut result = OrganizationWindowRollingCalibrationView {
        schema: ORGANIZATION_WINDOW_ROLLING_CALIBRATION_SCHEMA.to_owned(),
        target: target.to_owned(),
        manifest_fingerprint,
        origins: origin_views,
        overall,
        dimensions,
        ablations,
        organization_stability,
        uncertainty,
        claim_status,
        disclosures: vec![
            "Each origin is scored only against its frozen point-in-time board and leakage audit."
                .to_owned(),
            "The baseline value is supplied and frozen per origin; it is not fitted from that origin's outcomes."
                .to_owned(),
            "Ablations remove one dimension and renormalize the remaining manifest weights."
                .to_owned(),
            "The confidence interval measures between-origin variation; trial noise is not available in the Window board contract."
                .to_owned(),
        ],
        fingerprint: String::new(),
    };
    result.fingerprint = rolling_calibration_fingerprint(&result)?;
    Ok(result)
}

fn build_dimension_ablations(
    origins: &[WindowCalibrationOriginInput],
    actual: &[f64],
    full_mae: f64,
    dimension_keys: &[String],
) -> Result<Vec<WindowCalibrationAblationView>, OrganizationWindowCalibrationError> {
    let mut ordered = origins.iter().collect::<Vec<_>>();
    ordered.sort_by(|a, b| {
        (a.board.season, a.board.as_of, a.origin_id.as_str()).cmp(&(
            b.board.season,
            b.board.as_of,
            b.origin_id.as_str(),
        ))
    });
    dimension_keys
        .iter()
        .map(|excluded| {
            let mut predicted = Vec::new();
            for origin in &ordered {
                for organization in &origin.board.organizations {
                    let mut numerator = 0.0;
                    let mut weight = 0.0;
                    for configured in &origin.board.manifest.dimensions {
                        if configured.key == *excluded {
                            continue;
                        }
                        let score = organization
                            .dimensions
                            .iter()
                            .find(|dimension| dimension.key == configured.key)
                            .and_then(|dimension| dimension.score)
                            .ok_or_else(|| {
                                OrganizationWindowCalibrationError::IncompleteDimension {
                                    organization: organization.organization.clone(),
                                    dimension: configured.key.clone(),
                                }
                            })?;
                        numerator += score * configured.weight;
                        weight += configured.weight;
                    }
                    if weight <= 0.0 {
                        return Err(OrganizationWindowCalibrationError::IncompleteDimension {
                            organization: organization.organization.clone(),
                            dimension: excluded.clone(),
                        });
                    }
                    predicted.push(numerator / weight);
                }
            }
            let mae = mean_absolute_error(&predicted, actual);
            Ok(WindowCalibrationAblationView {
                excluded_dimension: excluded.clone(),
                sample_size: actual.len(),
                mean_absolute_error: mae,
                delta_from_full_mae: mae - full_mae,
                rank_correlation: pearson(&ranks(&predicted), &ranks(actual)),
            })
        })
        .collect()
}

fn between_origin_uncertainty(origin_maes: &[f64]) -> WindowCalibrationUncertaintyView {
    let count = origin_maes.len();
    let mean = origin_maes.iter().sum::<f64>() / count as f64;
    let variance = if count > 1 {
        origin_maes
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>()
            / (count - 1) as f64
    } else {
        0.0
    };
    let standard_deviation = variance.sqrt();
    let margin = 1.96 * standard_deviation / (count as f64).sqrt();
    WindowCalibrationUncertaintyView {
        origin_count: count,
        between_origin_mae_standard_deviation: standard_deviation,
        mean_mae_confidence_interval_low: (mean - margin).max(0.0),
        mean_mae_confidence_interval_high: (mean + margin).min(100.0),
        trial_noise_status: WindowTrialNoiseStatus::NotProvided,
    }
}

fn rolling_calibration_fingerprint(
    calibration: &OrganizationWindowRollingCalibrationView,
) -> Result<String, OrganizationWindowCalibrationError> {
    let mut canonical = calibration.clone();
    canonical.fingerprint.clear();
    canonical
        .origins
        .sort_by(|a, b| a.origin_id.cmp(&b.origin_id));
    canonical.dimensions.sort_by(|a, b| a.key.cmp(&b.key));
    canonical
        .ablations
        .sort_by(|a, b| a.excluded_dimension.cmp(&b.excluded_dimension));
    canonical
        .organization_stability
        .sort_by(|a, b| a.organization.cmp(&b.organization));
    canonical.disclosures.sort();
    let bytes = serde_json::to_vec(&canonical)
        .map_err(|error| OrganizationWindowCalibrationError::Serialization(error.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn outcome_map(
    outcomes: &[WindowOutcomeRow],
) -> Result<BTreeMap<&str, f64>, OrganizationWindowCalibrationError> {
    let mut map = BTreeMap::new();
    for row in outcomes {
        if !row.target_value.is_finite() || !(0.0..=100.0).contains(&row.target_value) {
            return Err(OrganizationWindowCalibrationError::InvalidOutcome(
                row.organization.clone(),
            ));
        }
        if map
            .insert(row.organization.as_str(), row.target_value)
            .is_some()
        {
            return Err(OrganizationWindowCalibrationError::DuplicateOutcome(
                row.organization.clone(),
            ));
        }
    }
    Ok(map)
}

fn validate_leakage_audit(
    board: &OrganizationWindowBoardView,
    audit: &[WindowLeakageAuditRow],
) -> Result<(), OrganizationWindowCalibrationError> {
    let supplied = audit
        .iter()
        .map(|row| (row.profile_key.as_str(), row.method_version.as_str()))
        .collect::<BTreeSet<_>>();
    for profile in board
        .manifest
        .dimensions
        .iter()
        .flat_map(|dimension| &dimension.profiles)
    {
        if !supplied.contains(&(
            profile.profile_key.as_str(),
            profile.method_version.as_str(),
        )) {
            return Err(OrganizationWindowCalibrationError::IncompleteLeakageAudit(
                format!("{}@{}", profile.profile_key, profile.method_version),
            ));
        }
    }
    Ok(())
}

fn metric(
    key: &str,
    predicted: &[f64],
    actual: &[f64],
    leakage_blocked: bool,
) -> WindowCalibrationMetricView {
    let mean = actual.iter().sum::<f64>() / actual.len() as f64;
    let baseline = vec![mean; actual.len()];
    metric_with_baseline(key, predicted, actual, &baseline, leakage_blocked)
}

fn metric_with_baseline(
    key: &str,
    predicted: &[f64],
    actual: &[f64],
    baseline: &[f64],
    leakage_blocked: bool,
) -> WindowCalibrationMetricView {
    let mae = mean_absolute_error(predicted, actual);
    let baseline_mae = mean_absolute_error(baseline, actual);
    let correlation = pearson(&ranks(predicted), &ranks(actual));
    let claim_status = if leakage_blocked {
        WindowCalibrationClaimStatus::Blocked
    } else if mae < baseline_mae && correlation.is_some_and(|value| value >= 0.3) {
        WindowCalibrationClaimStatus::Calibrated
    } else {
        WindowCalibrationClaimStatus::Inconclusive
    };
    WindowCalibrationMetricView {
        key: key.to_owned(),
        sample_size: actual.len(),
        mean_absolute_error: mae,
        baseline_mean_absolute_error: baseline_mae,
        rank_correlation: correlation,
        claim_status,
    }
}

fn mean_absolute_error(predicted: &[f64], actual: &[f64]) -> f64 {
    predicted
        .iter()
        .zip(actual)
        .map(|(prediction, outcome)| (prediction - outcome).abs())
        .sum::<f64>()
        / actual.len() as f64
}

fn ranks(values: &[f64]) -> Vec<f64> {
    let mut order = values.iter().copied().enumerate().collect::<Vec<_>>();
    order.sort_by(|a, b| a.1.total_cmp(&b.1));
    let mut result = vec![0.0; values.len()];
    let mut start = 0;
    while start < order.len() {
        let mut end = start + 1;
        while end < order.len() && order[end].1 == order[start].1 {
            end += 1;
        }
        let average = (start + 1 + end) as f64 / 2.0;
        for (index, _) in &order[start..end] {
            result[*index] = average;
        }
        start = end;
    }
    result
}

fn pearson(left: &[f64], right: &[f64]) -> Option<f64> {
    if left.len() != right.len() || left.len() < 2 {
        return None;
    }
    let left_mean = left.iter().sum::<f64>() / left.len() as f64;
    let right_mean = right.iter().sum::<f64>() / right.len() as f64;
    let numerator = left
        .iter()
        .zip(right)
        .map(|(left, right)| (left - left_mean) * (right - right_mean))
        .sum::<f64>();
    let left_sum = left
        .iter()
        .map(|value| (value - left_mean).powi(2))
        .sum::<f64>();
    let right_sum = right
        .iter()
        .map(|value| (value - right_mean).powi(2))
        .sum::<f64>();
    let denominator = (left_sum * right_sum).sqrt();
    (denominator > 0.0).then(|| numerator / denominator)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_model::organization_window::{
        build_organization_window_board, load_organization_window_profile_inventory,
        OrganizationProfileInput, OrganizationWindowBoardInput, WindowProfileStatus,
    };

    fn ranked_board(season: u32, as_of: NaiveDate) -> OrganizationWindowBoardView {
        let source: OrganizationWindowBoardView = serde_json::from_str(include_str!(
            "../../../examples/organization-window-board-evaluation-2026-27.json"
        ))
        .unwrap();
        let profile_inputs = source
            .organizations
            .iter()
            .enumerate()
            .flat_map(|(index, organization)| {
                organization
                    .dimensions
                    .iter()
                    .flat_map(|dimension| &dimension.profiles)
                    .map(move |profile| OrganizationProfileInput {
                        profile_key: profile.profile_key.clone(),
                        method_version: profile.method_version.clone(),
                        organization: profile.organization.clone(),
                        organization_identity_version: profile
                            .organization_identity_version
                            .clone(),
                        season,
                        season_type: profile.season_type.clone(),
                        as_of,
                        horizon: profile.horizon,
                        raw_value: Some(index as f64),
                        raw_unit: profile.raw_unit.clone(),
                        sample_size: 100,
                        confidence: 1.0,
                        coverage: 1.0,
                        status: WindowProfileStatus::Observed,
                        evidence: Vec::new(),
                        limitations: Vec::new(),
                        source_fingerprints: vec![format!("sha256:{}", "a".repeat(64))],
                    })
            })
            .collect();
        let inventory = load_organization_window_profile_inventory().unwrap();
        build_organization_window_board(
            OrganizationWindowBoardInput {
                season,
                season_type: source.season_type,
                as_of,
                generated_at: format!("{as_of}T00:00:00Z"),
                manifest: source.manifest,
                profile_inputs,
                source_fingerprints: vec![format!("sha256:{}", "b".repeat(64))],
            },
            &inventory,
        )
        .unwrap()
    }

    fn safe_audit(board: &OrganizationWindowBoardView) -> Vec<WindowLeakageAuditRow> {
        board
            .manifest
            .dimensions
            .iter()
            .flat_map(|dimension| &dimension.profiles)
            .map(|profile| WindowLeakageAuditRow {
                profile_key: profile.profile_key.clone(),
                method_version: profile.method_version.clone(),
                point_in_time_safe: true,
                evidence: "frozen before target window".to_owned(),
            })
            .collect()
    }

    #[test]
    fn rank_correlation_handles_ties_and_direction() {
        assert_eq!(
            pearson(&ranks(&[1.0, 2.0, 3.0]), &ranks(&[1.0, 2.0, 3.0])),
            Some(1.0)
        );
        assert_eq!(
            pearson(&ranks(&[1.0, 2.0, 3.0]), &ranks(&[3.0, 2.0, 1.0])),
            Some(-1.0)
        );
        assert_eq!(ranks(&[1.0, 1.0, 3.0]), vec![1.5, 1.5, 3.0]);
    }

    #[test]
    fn metric_never_claims_calibration_when_leakage_is_blocked() {
        let result = metric("overall", &[10.0, 50.0, 90.0], &[10.0, 50.0, 90.0], true);
        assert_eq!(result.claim_status, WindowCalibrationClaimStatus::Blocked);
    }

    #[test]
    fn sealed_ranked_board_can_beat_the_constant_baseline() {
        let board = ranked_board(20262027, NaiveDate::from_ymd_opt(2026, 7, 27).unwrap());
        let outcomes = board
            .organizations
            .iter()
            .map(|row| WindowOutcomeRow {
                organization: row.organization.clone(),
                target_value: row.overall.score.unwrap(),
            })
            .collect::<Vec<_>>();
        let audit = safe_audit(&board);
        let result =
            calibrate_organization_window("next-season value", &board, &outcomes, &audit).unwrap();
        assert_eq!(
            result.overall.claim_status,
            WindowCalibrationClaimStatus::Calibrated
        );
        assert_eq!(result.overall.rank_correlation, Some(1.0));
    }

    fn rolling_origin(
        id: &str,
        season: u32,
        as_of: NaiveDate,
        outcome_shift: f64,
    ) -> WindowCalibrationOriginInput {
        let board = ranked_board(season, as_of);
        let outcomes = board
            .organizations
            .iter()
            .map(|organization| WindowOutcomeRow {
                organization: organization.organization.clone(),
                target_value: (organization.overall.score.unwrap() + outcome_shift)
                    .clamp(0.0, 100.0),
            })
            .collect();
        WindowCalibrationOriginInput {
            origin_id: id.to_owned(),
            leakage_audit: safe_audit(&board),
            board,
            outcomes,
            baseline_value: 50.0,
        }
    }

    #[test]
    fn rolling_origins_produce_ablations_stability_uncertainty_and_sealed_identity() {
        let origins = vec![
            rolling_origin(
                "2025-origin",
                20252026,
                NaiveDate::from_ymd_opt(2025, 7, 27).unwrap(),
                2.0,
            ),
            rolling_origin(
                "2023-origin",
                20232024,
                NaiveDate::from_ymd_opt(2023, 7, 27).unwrap(),
                1.0,
            ),
            rolling_origin(
                "2024-origin",
                20242025,
                NaiveDate::from_ymd_opt(2024, 7, 27).unwrap(),
                3.0,
            ),
        ];
        let result = calibrate_organization_window_rolling_origins(
            "next-season organization value",
            &origins,
            3,
        )
        .unwrap();
        assert_eq!(result.origins.len(), 3);
        assert_eq!(result.origins[0].origin_id, "2023-origin");
        assert_eq!(result.ablations.len(), 5);
        assert_eq!(result.organization_stability.len(), 32);
        assert!(result.uncertainty.between_origin_mae_standard_deviation > 0.0);
        assert_eq!(
            result.uncertainty.trial_noise_status,
            WindowTrialNoiseStatus::NotProvided
        );
        assert_eq!(
            result.claim_status,
            WindowCalibrationClaimStatus::Calibrated
        );
        assert_eq!(result.fingerprint.len(), 64);

        let mut reversed = origins.clone();
        reversed.reverse();
        let same = calibrate_organization_window_rolling_origins(
            "next-season organization value",
            &reversed,
            3,
        )
        .unwrap();
        assert_eq!(result.fingerprint, same.fingerprint);

        let mut blocked = origins;
        blocked[0].leakage_audit[0].point_in_time_safe = false;
        let blocked = calibrate_organization_window_rolling_origins(
            "next-season organization value",
            &blocked,
            3,
        )
        .unwrap();
        assert_eq!(blocked.claim_status, WindowCalibrationClaimStatus::Blocked);
    }
}
