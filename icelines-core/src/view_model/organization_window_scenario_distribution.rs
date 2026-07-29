//! Seeded uncertainty propagation for The Window scenarios.

use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::organization_window::{
    build_organization_window_board, load_organization_window_profile_inventory,
    validate_organization_window_board, OrganizationProfileInput, OrganizationWindowBoardInput,
    OrganizationWindowBoardView, OrganizationWindowProfileInventory,
};
use super::organization_window_comparison::{
    compare_organization_window_typed_scenario, OrganizationWindowScenarioImpactView,
    WindowScenarioAuthorityView,
};

pub const ORGANIZATION_WINDOW_SCENARIO_DISTRIBUTION_SCHEMA: &str =
    "organization_window_scenario_distribution.v1";
pub const ORGANIZATION_WINDOW_SCENARIO_DISTRIBUTION_INPUT_SCHEMA: &str =
    "organization_window_scenario_distribution_input.v1";
pub const ORGANIZATION_WINDOW_SCENARIO_DISTRIBUTION_INPUT_JSON_SCHEMA: &str = include_str!(
    "../../../design/schemas/organization_window_scenario_distribution_input.v1.schema.json"
);
pub const ORGANIZATION_WINDOW_SCENARIO_DISTRIBUTION_JSON_SCHEMA: &str = include_str!(
    "../../../design/schemas/organization_window_scenario_distribution.v1.schema.json"
);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowScenarioProfileShockInput {
    pub organization: String,
    pub profile_key: String,
    pub method_version: String,
    pub authority_id: String,
    /// Fingerprint of the artifact that supplied this shock's numeric estimate.
    pub estimate_source_fingerprint: String,
    /// Conditional mean raw-value delta when the event occurs.
    pub mean_raw_delta: f64,
    /// Raw-value delta when the event does not occur. Defaults to zero.
    #[serde(default)]
    pub inactive_raw_delta: f64,
    /// Bounded triangular uncertainty: `mean + half_range * (u1 - u2)`.
    pub half_range: f64,
    pub occurrence_probability: f64,
    /// Shocks sharing a key use one occurrence draw per trial.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationWindowScenarioDistributionInput {
    pub schema: String,
    pub scenario_id: String,
    pub trials: u32,
    pub seed: u64,
    pub authorities: Vec<WindowScenarioAuthorityView>,
    pub shocks: Vec<WindowScenarioProfileShockInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowScenarioDistributionSummaryView {
    pub mean: f64,
    pub p10: f64,
    pub p50: f64,
    pub p90: f64,
    pub positive_probability: f64,
    pub negative_probability: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowScenarioDimensionDistributionView {
    pub dimension_key: String,
    pub score_delta: WindowScenarioDistributionSummaryView,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowScenarioOrganizationDistributionView {
    pub organization: String,
    pub overall_score_delta: WindowScenarioDistributionSummaryView,
    pub dimensions: Vec<WindowScenarioDimensionDistributionView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowScenarioShockDistributionView {
    pub organization: String,
    pub profile_key: String,
    pub method_version: String,
    pub authority_id: String,
    pub estimate_source_fingerprint: String,
    pub sampled_raw_delta: WindowScenarioDistributionSummaryView,
    pub activation_probability: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationWindowScenarioDistributionView {
    pub schema: String,
    pub scenario_id: String,
    pub season: u32,
    pub as_of: NaiveDate,
    pub trials: u32,
    pub seed: u64,
    pub baseline_board_fingerprint: String,
    pub central_impact: OrganizationWindowScenarioImpactView,
    pub organizations: Vec<WindowScenarioOrganizationDistributionView>,
    pub shocks: Vec<WindowScenarioShockDistributionView>,
    pub disclosures: Vec<String>,
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OrganizationWindowScenarioDistributionError {
    #[error("Window scenario distribution id is empty")]
    EmptyScenarioId,
    #[error("unsupported Window scenario distribution input schema: {0}")]
    UnsupportedInputSchema(String),
    #[error("Window scenario distribution requires at least 100 trials")]
    InsufficientTrials,
    #[error("Window scenario distribution requires at least one shock")]
    EmptyShocks,
    #[error("Window scenario distribution baseline is invalid: {0}")]
    InvalidBaseline(String),
    #[error("Window scenario distribution shock is invalid: {0}")]
    InvalidShock(String),
    #[error("Window scenario distribution build failed: {0}")]
    Build(String),
    #[error("Window scenario distribution serialization failed: {0}")]
    Serialization(String),
}

pub fn simulate_organization_window_scenario_distribution(
    baseline: &OrganizationWindowBoardView,
    mut input: OrganizationWindowScenarioDistributionInput,
) -> Result<OrganizationWindowScenarioDistributionView, OrganizationWindowScenarioDistributionError>
{
    if input.schema != ORGANIZATION_WINDOW_SCENARIO_DISTRIBUTION_INPUT_SCHEMA {
        return Err(
            OrganizationWindowScenarioDistributionError::UnsupportedInputSchema(input.schema),
        );
    }
    if input.scenario_id.trim().is_empty() {
        return Err(OrganizationWindowScenarioDistributionError::EmptyScenarioId);
    }
    if input.trials < 100 {
        return Err(OrganizationWindowScenarioDistributionError::InsufficientTrials);
    }
    if input.shocks.is_empty() {
        return Err(OrganizationWindowScenarioDistributionError::EmptyShocks);
    }
    let inventory = load_organization_window_profile_inventory().map_err(|error| {
        OrganizationWindowScenarioDistributionError::InvalidBaseline(error.to_string())
    })?;
    validate_organization_window_board(baseline, &inventory).map_err(|error| {
        OrganizationWindowScenarioDistributionError::InvalidBaseline(error.to_string())
    })?;
    if baseline
        .organizations
        .iter()
        .any(|organization| organization.overall.score.is_none())
    {
        return Err(
            OrganizationWindowScenarioDistributionError::InvalidBaseline(
                "every organization needs a sealed score".to_owned(),
            ),
        );
    }

    canonicalize_input(&mut input);
    validate_shocks(baseline, &input, &inventory)?;
    let input_fingerprint = input_fingerprint(&input)?;
    let mut central_deltas = BTreeMap::new();
    for shock in &input.shocks {
        *central_deltas.entry(shock_key(shock)).or_default() += shock.mean_raw_delta
            * shock.occurrence_probability
            + shock.inactive_raw_delta * (1.0 - shock.occurrence_probability);
    }
    let central_board =
        build_scenario_board(baseline, &central_deltas, &input, &input_fingerprint)?;
    let central_impact = compare_organization_window_typed_scenario(
        &input.scenario_id,
        baseline,
        &central_board,
        input.authorities.clone(),
    )
    .map_err(|error| OrganizationWindowScenarioDistributionError::Build(error.to_string()))?;

    let organization_keys = baseline
        .organizations
        .iter()
        .map(|organization| organization.organization.clone())
        .collect::<Vec<_>>();
    let dimension_keys = baseline
        .manifest
        .dimensions
        .iter()
        .map(|row| row.key.clone())
        .collect::<Vec<_>>();
    let mut overall_samples = organization_keys
        .iter()
        .map(|team| (team.clone(), Vec::with_capacity(input.trials as usize)))
        .collect::<BTreeMap<_, _>>();
    let mut dimension_samples = organization_keys
        .iter()
        .flat_map(|team| {
            dimension_keys.iter().map(move |dimension| {
                (
                    (team.clone(), dimension.clone()),
                    Vec::with_capacity(input.trials as usize),
                )
            })
        })
        .collect::<BTreeMap<_, _>>();
    let mut shock_samples = input
        .shocks
        .iter()
        .map(|shock| {
            (
                shock_key_with_authority(shock),
                Vec::with_capacity(input.trials as usize),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut shock_activations = input
        .shocks
        .iter()
        .map(|shock| (shock_key_with_authority(shock), 0_u32))
        .collect::<BTreeMap<_, _>>();

    for trial in 0..input.trials {
        let mut occurrence_draws = BTreeMap::<String, f64>::new();
        let mut deltas = BTreeMap::<(String, String, String), f64>::new();
        for shock in &input.shocks {
            let occurrence_key = shock
                .correlation_key
                .clone()
                .unwrap_or_else(|| format!("shock:{}", shock_identity(shock)));
            let occurrence = *occurrence_draws
                .entry(occurrence_key.clone())
                .or_insert_with(|| {
                    deterministic_unit(input.seed, trial, &format!("occurrence:{occurrence_key}"))
                })
                < shock.occurrence_probability;
            let sampled = if occurrence {
                let first = deterministic_unit(
                    input.seed,
                    trial,
                    &format!("amplitude-a:{}", shock_identity(shock)),
                );
                let second = deterministic_unit(
                    input.seed,
                    trial,
                    &format!("amplitude-b:{}", shock_identity(shock)),
                );
                shock.mean_raw_delta + shock.half_range * (first - second)
            } else {
                shock.inactive_raw_delta
            };
            *deltas.entry(shock_key(shock)).or_default() += sampled;
            shock_samples
                .get_mut(&shock_key_with_authority(shock))
                .expect("canonical shock sample")
                .push(sampled);
            if occurrence {
                *shock_activations
                    .get_mut(&shock_key_with_authority(shock))
                    .expect("canonical shock activation") += 1;
            }
        }
        let trial_board = build_scenario_board(baseline, &deltas, &input, &input_fingerprint)?;
        for organization in &trial_board.organizations {
            let baseline_organization = baseline
                .organization(&organization.organization)
                .expect("validated common cohort");
            overall_samples
                .get_mut(&organization.organization)
                .expect("canonical organization sample")
                .push(
                    organization.overall.score.unwrap()
                        - baseline_organization.overall.score.unwrap(),
                );
            for dimension in &organization.dimensions {
                let baseline_dimension = baseline_organization
                    .dimensions
                    .iter()
                    .find(|candidate| candidate.key == dimension.key)
                    .expect("validated common dimension");
                if let (Some(score), Some(baseline_score)) =
                    (dimension.score, baseline_dimension.score)
                {
                    dimension_samples
                        .get_mut(&(organization.organization.clone(), dimension.key.clone()))
                        .expect("canonical dimension sample")
                        .push(score - baseline_score);
                }
            }
        }
    }

    let organizations = organization_keys
        .iter()
        .map(|organization| WindowScenarioOrganizationDistributionView {
            organization: organization.clone(),
            overall_score_delta: summarize(&overall_samples[organization]),
            dimensions: dimension_keys
                .iter()
                .filter(|dimension| {
                    !dimension_samples[&(organization.clone(), (*dimension).clone())].is_empty()
                })
                .map(|dimension| WindowScenarioDimensionDistributionView {
                    dimension_key: dimension.clone(),
                    score_delta: summarize(
                        &dimension_samples[&(organization.clone(), dimension.clone())],
                    ),
                })
                .collect(),
        })
        .collect();
    let shocks = input
        .shocks
        .iter()
        .map(|shock| {
            let key = shock_key_with_authority(shock);
            WindowScenarioShockDistributionView {
                organization: shock.organization.clone(),
                profile_key: shock.profile_key.clone(),
                method_version: shock.method_version.clone(),
                authority_id: shock.authority_id.clone(),
                estimate_source_fingerprint: shock.estimate_source_fingerprint.clone(),
                sampled_raw_delta: summarize(&shock_samples[&key]),
                activation_probability: f64::from(shock_activations[&key])
                    / f64::from(input.trials),
            }
        })
        .collect();
    let mut result = OrganizationWindowScenarioDistributionView {
        schema: ORGANIZATION_WINDOW_SCENARIO_DISTRIBUTION_SCHEMA.to_owned(),
        scenario_id: input.scenario_id,
        season: baseline.season,
        as_of: baseline.as_of,
        trials: input.trials,
        seed: input.seed,
        baseline_board_fingerprint: baseline.fingerprint.clone(),
        central_impact,
        organizations,
        shocks,
        disclosures: vec![
            "Every trial perturbs registered raw profile inputs and rebuilds the complete cohort through the canonical Window scorer.".to_owned(),
            "Active-shock uncertainty is a bounded symmetric triangular distribution around the conditional mean; occurrence probability is sampled separately and the inactive outcome may carry its own raw delta.".to_owned(),
            "Shared correlation keys coordinate occurrence only; amplitudes remain shock-specific.".to_owned(),
            "The central impact uses probability-weighted expected raw deltas and is not substituted for the seeded distribution.".to_owned(),
            "Pane distributions are omitted when the sealed baseline withholds that pane score; missing panes are never coerced to zero.".to_owned(),
        ],
        fingerprint: String::new(),
    };
    result.fingerprint = result_fingerprint(&result)?;
    Ok(result)
}

fn validate_shocks(
    baseline: &OrganizationWindowBoardView,
    input: &OrganizationWindowScenarioDistributionInput,
    inventory: &OrganizationWindowProfileInventory,
) -> Result<(), OrganizationWindowScenarioDistributionError> {
    let authority_by_id = input
        .authorities
        .iter()
        .map(|authority| (authority.authority_id.as_str(), authority))
        .collect::<BTreeMap<_, _>>();
    let mut identities = BTreeSet::new();
    for shock in &input.shocks {
        let identity = shock_key_with_authority(shock);
        if !identities.insert(identity.clone())
            || !shock.mean_raw_delta.is_finite()
            || !is_sha256_fingerprint(&shock.estimate_source_fingerprint)
            || !shock.inactive_raw_delta.is_finite()
            || !shock.half_range.is_finite()
            || shock.half_range < 0.0
            || !shock.occurrence_probability.is_finite()
            || !(0.0..=1.0).contains(&shock.occurrence_probability)
            || shock
                .correlation_key
                .as_deref()
                .is_some_and(|key| key.trim().is_empty())
        {
            return Err(OrganizationWindowScenarioDistributionError::InvalidShock(
                format!(
                    "{}:{}@{}",
                    shock.organization, shock.profile_key, shock.method_version
                ),
            ));
        }
        let authority = authority_by_id
            .get(shock.authority_id.as_str())
            .ok_or_else(|| {
                OrganizationWindowScenarioDistributionError::InvalidShock(format!(
                    "unknown authority {}",
                    shock.authority_id
                ))
            })?;
        if !authority.organizations.contains(&shock.organization)
            || !authority.profile_methods.iter().any(|method| {
                method.profile_key == shock.profile_key
                    && method.method_version == shock.method_version
            })
        {
            return Err(OrganizationWindowScenarioDistributionError::InvalidShock(
                format!("authority {} does not scope the shock", shock.authority_id),
            ));
        }
        let profile = baseline
            .organization(&shock.organization)
            .and_then(|organization| {
                organization
                    .dimensions
                    .iter()
                    .flat_map(|dimension| &dimension.profiles)
                    .find(|profile| {
                        profile.profile_key == shock.profile_key
                            && profile.method_version == shock.method_version
                    })
            })
            .ok_or_else(|| {
                OrganizationWindowScenarioDistributionError::InvalidShock(format!(
                    "baseline is missing {}:{}@{}",
                    shock.organization, shock.profile_key, shock.method_version
                ))
            })?;
        if profile.raw_value.is_none() {
            return Err(OrganizationWindowScenarioDistributionError::InvalidShock(
                format!("baseline raw value is missing for {}", shock.profile_key),
            ));
        }
        let descriptor = inventory
            .find(&shock.profile_key, &shock.method_version)
            .ok_or_else(|| {
                OrganizationWindowScenarioDistributionError::InvalidShock(format!(
                    "unregistered profile {}@{}",
                    shock.profile_key, shock.method_version
                ))
            })?;
        if !descriptor.scenario_support {
            return Err(OrganizationWindowScenarioDistributionError::InvalidShock(
                format!(
                    "profile {}@{} does not support scenarios",
                    shock.profile_key, shock.method_version
                ),
            ));
        }
    }
    Ok(())
}

fn build_scenario_board(
    baseline: &OrganizationWindowBoardView,
    deltas: &BTreeMap<(String, String, String), f64>,
    input: &OrganizationWindowScenarioDistributionInput,
    input_fingerprint: &str,
) -> Result<OrganizationWindowBoardView, OrganizationWindowScenarioDistributionError> {
    let authority_fingerprints = input
        .authorities
        .iter()
        .map(|authority| {
            (
                authority.authority_id.as_str(),
                authority.source_fingerprint.as_str(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut profile_inputs = Vec::new();
    for organization in &baseline.organizations {
        for dimension in &organization.dimensions {
            for profile in &dimension.profiles {
                let key = (
                    organization.organization.clone(),
                    profile.profile_key.clone(),
                    profile.method_version.clone(),
                );
                let raw_value = profile
                    .raw_value
                    .map(|value| value + deltas.get(&key).copied().unwrap_or_default());
                if raw_value.is_some_and(|value| !value.is_finite()) {
                    return Err(OrganizationWindowScenarioDistributionError::Build(format!(
                        "non-finite scenario raw value for {}:{}",
                        organization.organization, profile.profile_key
                    )));
                }
                let mut source_fingerprints = profile.source_fingerprints.clone();
                if deltas.contains_key(&key) {
                    source_fingerprints.push(format!("sha256:{input_fingerprint}"));
                    source_fingerprints.extend(
                        input
                            .shocks
                            .iter()
                            .filter(|shock| shock_key(shock) == key)
                            .filter_map(|shock| {
                                authority_fingerprints.get(shock.authority_id.as_str())
                            })
                            .map(|fingerprint| (*fingerprint).to_owned()),
                    );
                    source_fingerprints.extend(
                        input
                            .shocks
                            .iter()
                            .filter(|shock| shock_key(shock) == key)
                            .map(|shock| shock.estimate_source_fingerprint.clone()),
                    );
                }
                profile_inputs.push(OrganizationProfileInput {
                    profile_key: profile.profile_key.clone(),
                    method_version: profile.method_version.clone(),
                    organization: profile.organization.clone(),
                    organization_identity_version: profile.organization_identity_version.clone(),
                    season: profile.season,
                    season_type: profile.season_type.clone(),
                    as_of: profile.as_of,
                    horizon: profile.horizon,
                    raw_value,
                    raw_unit: profile.raw_unit.clone(),
                    sample_size: profile.sample_size,
                    confidence: profile.confidence,
                    coverage: profile.coverage,
                    status: profile.status,
                    evidence: profile.evidence.clone(),
                    limitations: profile.limitations.clone(),
                    source_fingerprints,
                });
            }
        }
    }
    let mut source_fingerprints = baseline.source_fingerprints.clone();
    source_fingerprints.push(format!("sha256:{input_fingerprint}"));
    source_fingerprints.extend(
        input
            .authorities
            .iter()
            .map(|authority| authority.source_fingerprint.clone()),
    );
    let inventory = load_organization_window_profile_inventory()
        .map_err(|error| OrganizationWindowScenarioDistributionError::Build(error.to_string()))?;
    build_organization_window_board(
        OrganizationWindowBoardInput {
            season: baseline.season,
            season_type: baseline.season_type.clone(),
            as_of: baseline.as_of,
            generated_at: baseline.generated_at.clone(),
            manifest: baseline.manifest.clone(),
            profile_inputs,
            source_fingerprints,
        },
        &inventory,
    )
    .map_err(|error| OrganizationWindowScenarioDistributionError::Build(error.to_string()))
}

fn canonicalize_input(input: &mut OrganizationWindowScenarioDistributionInput) {
    input
        .authorities
        .sort_by(|left, right| left.authority_id.cmp(&right.authority_id));
    for authority in &mut input.authorities {
        authority.organizations.sort();
        authority.organizations.dedup();
        authority.profile_methods.sort();
        authority.profile_methods.dedup();
    }
    input.shocks.sort_by_key(shock_key_with_authority);
}

fn shock_key(shock: &WindowScenarioProfileShockInput) -> (String, String, String) {
    (
        shock.organization.clone(),
        shock.profile_key.clone(),
        shock.method_version.clone(),
    )
}

fn shock_key_with_authority(
    shock: &WindowScenarioProfileShockInput,
) -> (String, String, String, String) {
    (
        shock.organization.clone(),
        shock.profile_key.clone(),
        shock.method_version.clone(),
        shock.authority_id.clone(),
    )
}

fn shock_identity(shock: &WindowScenarioProfileShockInput) -> String {
    format!(
        "{}:{}@{}:{}",
        shock.organization, shock.profile_key, shock.method_version, shock.authority_id
    )
}

fn is_sha256_fingerprint(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|digest| digest.len() == 64 && digest.chars().all(|c| c.is_ascii_hexdigit()))
}

fn deterministic_unit(seed: u64, trial: u32, stream: &str) -> f64 {
    let mut digest = Sha256::new();
    digest.update(seed.to_le_bytes());
    digest.update(trial.to_le_bytes());
    digest.update(stream.as_bytes());
    let bytes = digest.finalize();
    let mut value_bytes = [0_u8; 8];
    value_bytes.copy_from_slice(&bytes[..8]);
    let value = u64::from_le_bytes(value_bytes);
    value as f64 / u64::MAX as f64
}

fn summarize(values: &[f64]) -> WindowScenarioDistributionSummaryView {
    let mut ordered = values.to_vec();
    ordered.sort_by(f64::total_cmp);
    let count = ordered.len() as f64;
    WindowScenarioDistributionSummaryView {
        mean: ordered.iter().sum::<f64>() / count,
        p10: percentile(&ordered, 0.10),
        p50: percentile(&ordered, 0.50),
        p90: percentile(&ordered, 0.90),
        positive_probability: ordered.iter().filter(|value| **value > 0.0).count() as f64 / count,
        negative_probability: ordered.iter().filter(|value| **value < 0.0).count() as f64 / count,
    }
}

fn percentile(values: &[f64], quantile: f64) -> f64 {
    let index = ((values.len() - 1) as f64 * quantile).round() as usize;
    values[index]
}

fn input_fingerprint(
    input: &OrganizationWindowScenarioDistributionInput,
) -> Result<String, OrganizationWindowScenarioDistributionError> {
    let bytes = serde_json::to_vec(input).map_err(|error| {
        OrganizationWindowScenarioDistributionError::Serialization(error.to_string())
    })?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn result_fingerprint(
    result: &OrganizationWindowScenarioDistributionView,
) -> Result<String, OrganizationWindowScenarioDistributionError> {
    let mut canonical = result.clone();
    canonical.fingerprint.clear();
    canonical
        .organizations
        .sort_by(|left, right| left.organization.cmp(&right.organization));
    for organization in &mut canonical.organizations {
        organization
            .dimensions
            .sort_by(|left, right| left.dimension_key.cmp(&right.dimension_key));
    }
    canonical.shocks.sort_by(|left, right| {
        (
            &left.organization,
            &left.profile_key,
            &left.method_version,
            &left.authority_id,
        )
            .cmp(&(
                &right.organization,
                &right.profile_key,
                &right.method_version,
                &right.authority_id,
            ))
    });
    canonical.disclosures.sort();
    let wire = serde_json::to_vec(&canonical).map_err(|error| {
        OrganizationWindowScenarioDistributionError::Serialization(error.to_string())
    })?;
    let normalized: OrganizationWindowScenarioDistributionView = serde_json::from_slice(&wire)
        .map_err(|error| {
            OrganizationWindowScenarioDistributionError::Serialization(error.to_string())
        })?;
    let bytes = serde_json::to_vec(&normalized).map_err(|error| {
        OrganizationWindowScenarioDistributionError::Serialization(error.to_string())
    })?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_model::organization_window::{
        build_organization_window_board, load_organization_window_profile_inventory,
        OrganizationProfileInput, OrganizationWindowBoardInput, WindowProfileStatus,
    };
    use crate::view_model::organization_window_comparison::{
        WindowScenarioAuthorityKind, WindowScenarioProfileImpactKind,
        WindowScenarioProfileMethodView,
    };

    fn ranked_board() -> OrganizationWindowBoardView {
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
                        season: source.season,
                        season_type: profile.season_type.clone(),
                        as_of: source.as_of,
                        horizon: profile.horizon,
                        raw_value: Some(index as f64 + 1.0),
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
                season: source.season,
                season_type: source.season_type,
                as_of: source.as_of,
                generated_at: source.generated_at,
                manifest: source.manifest,
                profile_inputs,
                source_fingerprints: vec![format!("sha256:{}", "b".repeat(64))],
            },
            &inventory,
        )
        .unwrap()
    }

    fn authority(
        id: &str,
        kind: WindowScenarioAuthorityKind,
        organization: &str,
        profile_key: &str,
        method_version: &str,
        fingerprint_character: char,
    ) -> WindowScenarioAuthorityView {
        WindowScenarioAuthorityView {
            authority_id: id.to_owned(),
            kind,
            source_schema: "test_authority.v1".to_owned(),
            source_fingerprint: format!("sha256:{}", fingerprint_character.to_string().repeat(64)),
            organizations: vec![organization.to_owned()],
            profile_methods: vec![WindowScenarioProfileMethodView {
                profile_key: profile_key.to_owned(),
                method_version: method_version.to_owned(),
            }],
            rationale: format!("{id} test authority"),
        }
    }

    fn combined_input() -> OrganizationWindowScenarioDistributionInput {
        OrganizationWindowScenarioDistributionInput {
            schema: ORGANIZATION_WINDOW_SCENARIO_DISTRIBUTION_INPUT_SCHEMA.to_owned(),
            scenario_id: "nyr-trade-plus-sea-development".to_owned(),
            trials: 128,
            seed: 73,
            authorities: vec![
                authority(
                    "nyr-trade",
                    WindowScenarioAuthorityKind::Trade,
                    "NYR",
                    "nhl.expected_points",
                    "icecast_expected_points.v1",
                    'c',
                ),
                authority(
                    "sea-development",
                    WindowScenarioAuthorityKind::PlayerDevelopment,
                    "SEA",
                    "pipeline.prospect_readiness",
                    "prospect_readiness_score.v1",
                    'd',
                ),
            ],
            shocks: vec![
                WindowScenarioProfileShockInput {
                    organization: "NYR".to_owned(),
                    profile_key: "nhl.expected_points".to_owned(),
                    method_version: "icecast_expected_points.v1".to_owned(),
                    authority_id: "nyr-trade".to_owned(),
                    estimate_source_fingerprint: format!("sha256:{}", "f".repeat(64)),
                    mean_raw_delta: 6.0,
                    inactive_raw_delta: 0.0,
                    half_range: 3.0,
                    occurrence_probability: 0.80,
                    correlation_key: Some("trade-completes".to_owned()),
                },
                WindowScenarioProfileShockInput {
                    organization: "SEA".to_owned(),
                    profile_key: "pipeline.prospect_readiness".to_owned(),
                    method_version: "prospect_readiness_score.v1".to_owned(),
                    authority_id: "sea-development".to_owned(),
                    estimate_source_fingerprint: format!("sha256:{}", "9".repeat(64)),
                    mean_raw_delta: 10.0,
                    inactive_raw_delta: -2.0,
                    half_range: 5.0,
                    occurrence_probability: 0.65,
                    correlation_key: Some("prospect-hits".to_owned()),
                },
            ],
        }
    }

    #[test]
    fn seeded_isolated_and_combined_distributions_rebuild_the_full_cohort() {
        let baseline = ranked_board();
        let combined = combined_input();
        let result =
            simulate_organization_window_scenario_distribution(&baseline, combined.clone())
                .unwrap();
        assert_eq!(result.organizations.len(), 32);
        assert_eq!(result.shocks.len(), 2);
        assert_eq!(result.central_impact.authorities.len(), 2);
        assert_eq!(result.fingerprint.len(), 64);
        assert!(result.central_impact.profile_impacts.iter().any(|impact| {
            impact.organization == "NYR"
                && impact.profile_key == "nhl.expected_points"
                && impact.kind == WindowScenarioProfileImpactKind::RawInput
                && impact.authority_ids == ["nyr-trade"]
        }));
        assert!(result.central_impact.profile_impacts.iter().any(|impact| {
            impact.organization == "SEA"
                && impact.profile_key == "pipeline.prospect_readiness"
                && impact.kind == WindowScenarioProfileImpactKind::RawInput
                && impact.authority_ids == ["sea-development"]
        }));
        assert!(result
            .shocks
            .iter()
            .all(|shock| shock.activation_probability > 0.5));
        let sea_shock = result
            .shocks
            .iter()
            .find(|shock| shock.organization == "SEA")
            .unwrap();
        assert!(sea_shock.sampled_raw_delta.positive_probability > 0.0);
        assert!(sea_shock.sampled_raw_delta.negative_probability > 0.0);

        let mut reordered = combined.clone();
        reordered.authorities.reverse();
        reordered.shocks.reverse();
        let same =
            simulate_organization_window_scenario_distribution(&baseline, reordered).unwrap();
        assert_eq!(result.fingerprint, same.fingerprint);

        let isolated = OrganizationWindowScenarioDistributionInput {
            scenario_id: "nyr-trade-isolated".to_owned(),
            authorities: vec![combined.authorities[0].clone()],
            shocks: vec![combined.shocks[0].clone()],
            ..combined
        };
        let isolated =
            simulate_organization_window_scenario_distribution(&baseline, isolated).unwrap();
        assert_eq!(isolated.shocks.len(), 1);
        assert_eq!(
            isolated
                .central_impact
                .profile_impacts
                .iter()
                .filter(|impact| impact.kind == WindowScenarioProfileImpactKind::RawInput)
                .count(),
            1
        );
    }

    #[test]
    fn scenario_distribution_is_seed_sensitive_and_fails_closed_on_authority_scope() {
        let baseline = ranked_board();
        let input = combined_input();
        let first =
            simulate_organization_window_scenario_distribution(&baseline, input.clone()).unwrap();
        let mut other_seed = input.clone();
        other_seed.seed += 1;
        let second =
            simulate_organization_window_scenario_distribution(&baseline, other_seed).unwrap();
        assert_ne!(first.fingerprint, second.fingerprint);

        let mut invalid = input;
        invalid.shocks[0].authority_id = "missing-authority".to_owned();
        assert!(matches!(
            simulate_organization_window_scenario_distribution(&baseline, invalid),
            Err(OrganizationWindowScenarioDistributionError::InvalidShock(_))
        ));

        let mut unsupported = combined_input();
        unsupported.authorities[1].profile_methods[0].profile_key =
            "pipeline.prospect_pool".to_owned();
        unsupported.authorities[1].profile_methods[0].method_version =
            "prospect_pool_score.v1".to_owned();
        unsupported.shocks[1].profile_key = "pipeline.prospect_pool".to_owned();
        unsupported.shocks[1].method_version = "prospect_pool_score.v1".to_owned();
        assert!(matches!(
            simulate_organization_window_scenario_distribution(&baseline, unsupported),
            Err(OrganizationWindowScenarioDistributionError::InvalidShock(message))
                if message.contains("does not support scenarios")
        ));
    }

    #[test]
    fn scenario_distribution_schema_is_embedded_json() {
        let input_schema: serde_json::Value =
            serde_json::from_str(ORGANIZATION_WINDOW_SCENARIO_DISTRIBUTION_INPUT_JSON_SCHEMA)
                .unwrap();
        let schema: serde_json::Value =
            serde_json::from_str(ORGANIZATION_WINDOW_SCENARIO_DISTRIBUTION_JSON_SCHEMA).unwrap();
        assert_eq!(
            input_schema["properties"]["schema"]["const"],
            ORGANIZATION_WINDOW_SCENARIO_DISTRIBUTION_INPUT_SCHEMA
        );
        assert_eq!(
            schema["properties"]["schema"]["const"],
            ORGANIZATION_WINDOW_SCENARIO_DISTRIBUTION_SCHEMA
        );
    }

    #[test]
    fn partial_board_omits_unscored_panes_without_panicking() {
        let baseline: OrganizationWindowBoardView = serde_json::from_str(include_str!(
            "../../../examples/organization-window-board-evaluation-2026-27.json"
        ))
        .unwrap();
        let input = OrganizationWindowScenarioDistributionInput {
            schema: ORGANIZATION_WINDOW_SCENARIO_DISTRIBUTION_INPUT_SCHEMA.to_owned(),
            scenario_id: "partial-board".to_owned(),
            trials: 100,
            seed: 29,
            authorities: vec![authority(
                "nyr-readiness",
                WindowScenarioAuthorityKind::PlayerDevelopment,
                "NYR",
                "pipeline.prospect_readiness",
                "prospect_readiness_score.v1",
                'e',
            )],
            shocks: vec![WindowScenarioProfileShockInput {
                organization: "NYR".to_owned(),
                profile_key: "pipeline.prospect_readiness".to_owned(),
                method_version: "prospect_readiness_score.v1".to_owned(),
                authority_id: "nyr-readiness".to_owned(),
                estimate_source_fingerprint: format!("sha256:{}", "8".repeat(64)),
                mean_raw_delta: 1.0,
                inactive_raw_delta: 0.0,
                half_range: 0.0,
                occurrence_probability: 1.0,
                correlation_key: None,
            }],
        };
        let result = simulate_organization_window_scenario_distribution(&baseline, input).unwrap();
        let nyr = result
            .organizations
            .iter()
            .find(|organization| organization.organization == "NYR")
            .unwrap();
        assert!(!nyr.dimensions.is_empty());
        assert!(nyr.dimensions.len() < baseline.manifest.dimensions.len());
    }
}
