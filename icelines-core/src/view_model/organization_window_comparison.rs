//! Comparable Window checkpoints and scenario sensitivity.

use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::line_combination::{LineCombinationForecastView, LINE_COMBINATION_FORECAST_SCHEMA};
use super::organization_window::{
    build_organization_window_board, load_organization_window_profile_inventory,
    seal_organization_window_manifest, validate_organization_window_board,
    OrganizationProfileInput, OrganizationProfileObservationView, OrganizationWindowBoardInput,
    OrganizationWindowBoardView, OrganizationWindowManifestView,
    OrganizationWindowProfileInventory, WindowClassification, WindowEvidenceView, WindowFreshness,
    WindowOrganizationView,
};
use super::team_season_forecast::{
    TeamSeasonForecastView, TeamSeasonScenarioEventKind, TEAM_SEASON_FORECAST_SCHEMA,
};
use super::training_camp::{TrainingCampLeagueForecastView, TRAINING_CAMP_LEAGUE_FORECAST_SCHEMA};

pub const ORGANIZATION_WINDOW_MOVEMENT_SCHEMA: &str = "organization_window_movement.v1";
pub const ORGANIZATION_WINDOW_HISTORY_SCHEMA: &str = "organization_window_history.v1";
pub const ORGANIZATION_WINDOW_SCENARIO_IMPACT_SCHEMA: &str =
    "organization_window_scenario_impact.v1";
pub const ORGANIZATION_WINDOW_BRIDGE_SCHEMA: &str = "organization_window_bridge.v1";
pub const ORGANIZATION_WINDOW_BRIDGE_JSON_SCHEMA: &str =
    include_str!("../../../design/schemas/organization_window_bridge.v1.schema.json");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowProfileBridgeView {
    pub from_profile_key: String,
    pub from_method_version: String,
    pub to_profile_key: String,
    pub to_method_version: String,
    /// Applied to the source raw value before target-method normalization.
    pub raw_scale: f64,
    pub raw_offset: f64,
    pub rationale: String,
    pub evidence_fingerprints: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationWindowBridgeView {
    pub schema: String,
    pub bridge_id: String,
    pub created_at: String,
    pub from_manifest_fingerprint: String,
    pub to_manifest_fingerprint: String,
    pub profile_mappings: Vec<WindowProfileBridgeView>,
    pub disclosures: Vec<String>,
    #[serde(default)]
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowProfileDeltaView {
    pub profile_key: String,
    pub method_version: String,
    pub earlier_raw_value: Option<f64>,
    pub later_raw_value: Option<f64>,
    pub raw_delta: Option<f64>,
    pub earlier_score: Option<f64>,
    pub later_score: Option<f64>,
    pub score_delta: Option<f64>,
    pub confidence_delta: f64,
    pub coverage_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowDimensionDeltaView {
    pub dimension_key: String,
    pub earlier_score: Option<f64>,
    pub later_score: Option<f64>,
    pub score_delta: Option<f64>,
    pub confidence_delta: f64,
    pub coverage_delta: f64,
    pub profiles: Vec<WindowProfileDeltaView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowOrganizationDeltaView {
    pub organization: String,
    pub earlier_score: Option<f64>,
    pub later_score: Option<f64>,
    pub score_delta: Option<f64>,
    /// Positive means an improvement in league rank.
    pub rank_delta: Option<i32>,
    pub confidence_delta: f64,
    pub coverage_delta: f64,
    pub earlier_classification: WindowClassification,
    pub later_classification: WindowClassification,
    pub dimensions: Vec<WindowDimensionDeltaView>,
    /// Overall movement attributable to the sealed profile/dimension result.
    pub observed_input_delta: Option<f64>,
    /// Personnel attribution requires a typed scenario/personnel source.
    pub personnel_delta: Option<f64>,
    pub method_manifest_delta: Option<f64>,
    pub residual_revaluation: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationWindowMovementView {
    pub schema: String,
    pub season: u32,
    pub earlier_as_of: NaiveDate,
    pub later_as_of: NaiveDate,
    pub manifest_fingerprint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_manifest_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridge_fingerprint: Option<String>,
    pub earlier_board_fingerprint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rebased_earlier_board_fingerprint: Option<String>,
    pub later_board_fingerprint: String,
    pub organizations: Vec<WindowOrganizationDeltaView>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationWindowHistoryView {
    pub schema: String,
    pub season: u32,
    pub manifest_fingerprint: String,
    pub checkpoint_fingerprints: Vec<String>,
    pub movements: Vec<OrganizationWindowMovementView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationWindowScenarioImpactView {
    pub schema: String,
    pub scenario_id: String,
    pub season: u32,
    pub as_of: NaiveDate,
    pub manifest_fingerprint: String,
    pub baseline_board_fingerprint: String,
    pub scenario_board_fingerprint: String,
    #[serde(default)]
    pub authorities: Vec<WindowScenarioAuthorityView>,
    #[serde(default)]
    pub profile_impacts: Vec<WindowScenarioProfileImpactView>,
    pub organizations: Vec<WindowOrganizationDeltaView>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowScenarioAuthorityKind {
    Trade,
    Injury,
    Goalie,
    PlayerDevelopment,
    TrainingCamp,
    LineCombination,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct WindowScenarioProfileMethodView {
    pub profile_key: String,
    pub method_version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowScenarioAuthorityView {
    pub authority_id: String,
    pub kind: WindowScenarioAuthorityKind,
    pub source_schema: String,
    pub source_fingerprint: String,
    pub organizations: Vec<String>,
    pub profile_methods: Vec<WindowScenarioProfileMethodView>,
    pub rationale: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowScenarioProfileImpactKind {
    RawInput,
    Evidence,
    CohortRevaluation,
    Unchanged,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowScenarioProfileImpactView {
    pub organization: String,
    pub dimension_key: String,
    pub profile_key: String,
    pub method_version: String,
    pub kind: WindowScenarioProfileImpactKind,
    pub raw_delta: Option<f64>,
    pub score_delta: Option<f64>,
    pub confidence_delta: f64,
    pub coverage_delta: f64,
    pub authority_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OrganizationWindowComparisonError {
    #[error("Window comparison requires distinct board fingerprints")]
    IdenticalBoard,
    #[error("Window comparison board fingerprint is invalid: {0}")]
    InvalidBoardFingerprint(String),
    #[error("Window checkpoints are not ordered")]
    InvalidCheckpointOrder,
    #[error("Window comparison manifest or cohort differs: {0}")]
    Incomparable(String),
    #[error("Window comparison board is missing source fingerprints: {0}")]
    MissingSourceFingerprints(String),
    #[error("Window comparison is missing organization {0}")]
    MissingOrganization(String),
    #[error("Window comparison is missing dimension {dimension} for {organization}")]
    MissingDimension {
        organization: String,
        dimension: String,
    },
    #[error("Window comparison is missing profile {profile} for {organization}")]
    MissingProfile {
        organization: String,
        profile: String,
    },
    #[error("Window history requires at least two checkpoints")]
    InsufficientHistory,
    #[error("Window scenario id is empty")]
    EmptyScenarioId,
    #[error("unsupported Window bridge schema: {0}")]
    UnsupportedBridgeSchema(String),
    #[error("invalid Window bridge: {0}")]
    InvalidBridge(String),
    #[error("Window bridge fingerprint mismatch")]
    BridgeFingerprintMismatch,
    #[error("Window bridge cannot map profile {0}")]
    MissingBridgeMapping(String),
    #[error("Window rebase failed: {0}")]
    Rebase(String),
    #[error("invalid Window scenario authority: {0}")]
    InvalidScenarioAuthority(String),
    #[error("Window scenario contains an unattributed change: {0}")]
    UnattributedScenarioChange(String),
}

impl OrganizationWindowBridgeView {
    pub fn calculate_fingerprint(&self) -> Result<String, OrganizationWindowComparisonError> {
        let mut canonical = self.clone();
        canonical.fingerprint.clear();
        canonical.profile_mappings.sort_by(|a, b| {
            (
                &a.to_profile_key,
                &a.to_method_version,
                &a.from_profile_key,
                &a.from_method_version,
            )
                .cmp(&(
                    &b.to_profile_key,
                    &b.to_method_version,
                    &b.from_profile_key,
                    &b.from_method_version,
                ))
        });
        for mapping in &mut canonical.profile_mappings {
            mapping.evidence_fingerprints.sort();
            mapping.evidence_fingerprints.dedup();
            if mapping.raw_scale == 0.0 {
                mapping.raw_scale = 0.0;
            }
            if mapping.raw_offset == 0.0 {
                mapping.raw_offset = 0.0;
            }
        }
        canonical.disclosures.sort();
        canonical.disclosures.dedup();
        let wire = serde_json::to_vec(&canonical)
            .map_err(|error| OrganizationWindowComparisonError::InvalidBridge(error.to_string()))?;
        let normalized: Self = serde_json::from_slice(&wire)
            .map_err(|error| OrganizationWindowComparisonError::InvalidBridge(error.to_string()))?;
        let bytes = serde_json::to_vec(&normalized)
            .map_err(|error| OrganizationWindowComparisonError::InvalidBridge(error.to_string()))?;
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }
}

pub fn seal_organization_window_bridge(
    mut bridge: OrganizationWindowBridgeView,
) -> Result<OrganizationWindowBridgeView, OrganizationWindowComparisonError> {
    if bridge.schema != ORGANIZATION_WINDOW_BRIDGE_SCHEMA {
        return Err(OrganizationWindowComparisonError::UnsupportedBridgeSchema(
            bridge.schema,
        ));
    }
    for (field, value) in [
        ("bridge_id", bridge.bridge_id.as_str()),
        ("created_at", bridge.created_at.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(OrganizationWindowComparisonError::InvalidBridge(format!(
                "{field} is empty"
            )));
        }
    }
    for (field, fingerprint) in [
        (
            "from_manifest_fingerprint",
            bridge.from_manifest_fingerprint.as_str(),
        ),
        (
            "to_manifest_fingerprint",
            bridge.to_manifest_fingerprint.as_str(),
        ),
    ] {
        if !is_fingerprint(fingerprint) {
            return Err(OrganizationWindowComparisonError::InvalidBridge(format!(
                "{field} is not a SHA-256 fingerprint"
            )));
        }
    }
    if bridge.from_manifest_fingerprint == bridge.to_manifest_fingerprint {
        return Err(OrganizationWindowComparisonError::InvalidBridge(
            "source and target manifests are identical".to_owned(),
        ));
    }
    if bridge.profile_mappings.is_empty() {
        return Err(OrganizationWindowComparisonError::InvalidBridge(
            "profile mappings are empty".to_owned(),
        ));
    }
    let mut sources = BTreeSet::new();
    let mut targets = BTreeSet::new();
    for mapping in &mut bridge.profile_mappings {
        let source = format!(
            "{}@{}",
            mapping.from_profile_key, mapping.from_method_version
        );
        let target = format!("{}@{}", mapping.to_profile_key, mapping.to_method_version);
        if mapping.from_profile_key.trim().is_empty()
            || mapping.from_method_version.trim().is_empty()
            || mapping.to_profile_key.trim().is_empty()
            || mapping.to_method_version.trim().is_empty()
            || mapping.rationale.trim().is_empty()
        {
            return Err(OrganizationWindowComparisonError::InvalidBridge(format!(
                "mapping {source} -> {target} has an empty field"
            )));
        }
        if !mapping.raw_scale.is_finite()
            || mapping.raw_scale == 0.0
            || !mapping.raw_offset.is_finite()
        {
            return Err(OrganizationWindowComparisonError::InvalidBridge(format!(
                "mapping {source} -> {target} has an invalid affine transform"
            )));
        }
        if !sources.insert(source.clone()) || !targets.insert(target.clone()) {
            return Err(OrganizationWindowComparisonError::InvalidBridge(format!(
                "duplicate source or target mapping {source} -> {target}"
            )));
        }
        if mapping.evidence_fingerprints.is_empty()
            || mapping
                .evidence_fingerprints
                .iter()
                .any(|fingerprint| !is_fingerprint(fingerprint))
        {
            return Err(OrganizationWindowComparisonError::InvalidBridge(format!(
                "mapping {source} -> {target} lacks valid evidence fingerprints"
            )));
        }
        mapping.evidence_fingerprints.sort();
        mapping.evidence_fingerprints.dedup();
    }
    bridge.profile_mappings.sort_by(|a, b| {
        (&a.to_profile_key, &a.to_method_version).cmp(&(&b.to_profile_key, &b.to_method_version))
    });
    bridge.disclosures.sort();
    bridge.disclosures.dedup();
    let supplied = bridge.fingerprint.clone();
    bridge.fingerprint.clear();
    let calculated = bridge.calculate_fingerprint()?;
    if !supplied.is_empty() && supplied != calculated {
        return Err(OrganizationWindowComparisonError::BridgeFingerprintMismatch);
    }
    bridge.fingerprint = calculated;
    Ok(bridge)
}

pub fn compare_organization_window_snapshots(
    earlier: &OrganizationWindowBoardView,
    later: &OrganizationWindowBoardView,
) -> Result<OrganizationWindowMovementView, OrganizationWindowComparisonError> {
    validate_common(earlier, later, false)?;
    if later.as_of <= earlier.as_of {
        return Err(OrganizationWindowComparisonError::InvalidCheckpointOrder);
    }
    Ok(OrganizationWindowMovementView {
        schema: ORGANIZATION_WINDOW_MOVEMENT_SCHEMA.to_owned(),
        season: earlier.season,
        earlier_as_of: earlier.as_of,
        later_as_of: later.as_of,
        manifest_fingerprint: earlier.manifest.fingerprint.clone(),
        source_manifest_fingerprint: None,
        bridge_fingerprint: None,
        earlier_board_fingerprint: earlier.fingerprint.clone(),
        rebased_earlier_board_fingerprint: None,
        later_board_fingerprint: later.fingerprint.clone(),
        organizations: compare_organizations(earlier, later)?,
        disclosures: vec![
            "Movement is computed only between boards with identical manifests, cohorts, and method versions.".to_owned(),
            "Personnel attribution remains unset unless supplied by a typed scenario artifact.".to_owned(),
        ],
    })
}

/// Rebuild a historical board under a reviewed target manifest.
///
/// The bridge transforms raw profile observations and then delegates all
/// normalization and aggregation to the canonical Window scorer. It never
/// edits the source board or directly adjusts an aggregate score.
pub fn rebase_organization_window_board(
    source: &OrganizationWindowBoardView,
    target_manifest: &OrganizationWindowManifestView,
    bridge: &OrganizationWindowBridgeView,
    inventory: &OrganizationWindowProfileInventory,
) -> Result<OrganizationWindowBoardView, OrganizationWindowComparisonError> {
    validate_organization_window_board(source, inventory).map_err(|_| {
        OrganizationWindowComparisonError::InvalidBoardFingerprint(source.fingerprint.clone())
    })?;
    validate_sources(source)?;
    let bridge = seal_organization_window_bridge(bridge.clone())?;
    let target_manifest = seal_organization_window_manifest(target_manifest.clone(), inventory)
        .map_err(|error| OrganizationWindowComparisonError::Rebase(error.to_string()))?;
    if source.manifest.fingerprint != bridge.from_manifest_fingerprint {
        return Err(OrganizationWindowComparisonError::Incomparable(
            "bridge source manifest fingerprint".to_owned(),
        ));
    }
    if target_manifest.fingerprint != bridge.to_manifest_fingerprint {
        return Err(OrganizationWindowComparisonError::Incomparable(
            "bridge target manifest fingerprint".to_owned(),
        ));
    }
    if source.manifest.comparison_cohort != target_manifest.comparison_cohort
        || source.manifest.primary_horizon != target_manifest.primary_horizon
    {
        return Err(OrganizationWindowComparisonError::Incomparable(
            "bridge cohort or horizon".to_owned(),
        ));
    }

    let source_profiles = source
        .manifest
        .dimensions
        .iter()
        .flat_map(|dimension| dimension.profiles.iter())
        .map(|profile| {
            (
                (
                    profile.profile_key.as_str(),
                    profile.method_version.as_str(),
                ),
                profile,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mappings = bridge
        .profile_mappings
        .iter()
        .map(|mapping| {
            (
                (
                    mapping.to_profile_key.as_str(),
                    mapping.to_method_version.as_str(),
                ),
                mapping,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let target_profiles = target_manifest
        .dimensions
        .iter()
        .flat_map(|dimension| dimension.profiles.iter())
        .collect::<Vec<_>>();
    if mappings.len() != target_profiles.len() {
        return Err(OrganizationWindowComparisonError::InvalidBridge(format!(
            "bridge maps {} target profiles; target manifest requires {}",
            mappings.len(),
            target_profiles.len()
        )));
    }

    let source_rows = source
        .organizations
        .iter()
        .map(|organization| {
            (
                organization.organization.as_str(),
                organization
                    .dimensions
                    .iter()
                    .flat_map(|dimension| dimension.profiles.iter())
                    .map(|profile| {
                        (
                            (
                                profile.profile_key.as_str(),
                                profile.method_version.as_str(),
                            ),
                            profile,
                        )
                    })
                    .collect::<BTreeMap<_, _>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut profile_inputs = Vec::with_capacity(
        source
            .expected_organizations
            .len()
            .saturating_mul(target_profiles.len()),
    );
    for organization in &source.expected_organizations {
        let rows = source_rows.get(organization.as_str()).ok_or_else(|| {
            OrganizationWindowComparisonError::MissingOrganization(organization.clone())
        })?;
        for target in &target_profiles {
            let target_id = (target.profile_key.as_str(), target.method_version.as_str());
            let mapping = mappings.get(&target_id).ok_or_else(|| {
                OrganizationWindowComparisonError::MissingBridgeMapping(format!(
                    "{}@{}",
                    target_id.0, target_id.1
                ))
            })?;
            let source_id = (
                mapping.from_profile_key.as_str(),
                mapping.from_method_version.as_str(),
            );
            if !source_profiles.contains_key(&source_id) {
                return Err(OrganizationWindowComparisonError::MissingBridgeMapping(
                    format!("{}@{}", source_id.0, source_id.1),
                ));
            }
            let row = rows.get(&source_id).ok_or_else(|| {
                OrganizationWindowComparisonError::MissingProfile {
                    organization: organization.clone(),
                    profile: format!("{}@{}", source_id.0, source_id.1),
                }
            })?;
            let descriptor = inventory.find(target_id.0, target_id.1).ok_or_else(|| {
                OrganizationWindowComparisonError::MissingBridgeMapping(format!(
                    "{}@{} is absent from the target registry",
                    target_id.0, target_id.1
                ))
            })?;
            let raw_value = row
                .raw_value
                .map(|value| value.mul_add(mapping.raw_scale, mapping.raw_offset));
            if raw_value.is_some_and(|value| !value.is_finite()) {
                return Err(OrganizationWindowComparisonError::InvalidBridge(format!(
                    "{}@{} produces a non-finite value for {organization}",
                    target_id.0, target_id.1
                )));
            }
            let mut source_fingerprints = row.source_fingerprints.clone();
            source_fingerprints.extend(mapping.evidence_fingerprints.clone());
            source_fingerprints.push(bridge.fingerprint.clone());
            source_fingerprints.sort();
            source_fingerprints.dedup();
            let mut evidence = row.evidence.clone();
            evidence.push(WindowEvidenceView {
                source_schema: ORGANIZATION_WINDOW_BRIDGE_SCHEMA.to_owned(),
                source_id: bridge.bridge_id.clone(),
                captured_at: Some(bridge.created_at.clone()),
                as_of: Some(source.as_of),
                freshness: WindowFreshness::Current,
                source_url: None,
            });
            let mut limitations = row.limitations.clone();
            limitations.push(format!(
                "rebased through {}: {}",
                bridge.bridge_id, mapping.rationale
            ));
            profile_inputs.push(OrganizationProfileInput {
                profile_key: target.profile_key.clone(),
                method_version: target.method_version.clone(),
                organization: organization.clone(),
                organization_identity_version: target_manifest
                    .comparison_cohort
                    .team_catalog_version
                    .clone(),
                season: source.season,
                season_type: source.season_type.clone(),
                as_of: source.as_of,
                horizon: target_manifest.primary_horizon,
                raw_value,
                raw_unit: descriptor.raw_unit.clone(),
                sample_size: row.sample_size,
                confidence: row.confidence,
                coverage: row.coverage,
                status: row.status,
                evidence,
                limitations,
                source_fingerprints,
            });
        }
    }
    let mut source_fingerprints = source.source_fingerprints.clone();
    source_fingerprints.push(bridge.fingerprint.clone());
    source_fingerprints.sort();
    source_fingerprints.dedup();
    build_organization_window_board(
        OrganizationWindowBoardInput {
            season: source.season,
            season_type: source.season_type.clone(),
            as_of: source.as_of,
            generated_at: bridge.created_at.clone(),
            manifest: target_manifest,
            profile_inputs,
            source_fingerprints,
        },
        inventory,
    )
    .map_err(|error| OrganizationWindowComparisonError::Rebase(error.to_string()))
}

pub fn compare_organization_window_snapshots_with_bridge(
    earlier: &OrganizationWindowBoardView,
    later: &OrganizationWindowBoardView,
    bridge: &OrganizationWindowBridgeView,
    inventory: &OrganizationWindowProfileInventory,
) -> Result<OrganizationWindowMovementView, OrganizationWindowComparisonError> {
    let bridge = seal_organization_window_bridge(bridge.clone())?;
    let rebased = rebase_organization_window_board(earlier, &later.manifest, &bridge, inventory)?;
    let mut movement = compare_organization_window_snapshots(&rebased, later)?;
    movement.source_manifest_fingerprint = Some(earlier.manifest.fingerprint.clone());
    movement.bridge_fingerprint = Some(bridge.fingerprint.clone());
    movement.earlier_board_fingerprint = earlier.fingerprint.clone();
    movement.rebased_earlier_board_fingerprint = Some(rebased.fingerprint.clone());
    movement.disclosures.push(format!(
        "The earlier checkpoint was rebuilt through bridge {} before comparison; the source board remains immutable.",
        bridge.bridge_id
    ));
    for row in &mut movement.organizations {
        let original = earlier.organization(&row.organization).ok_or_else(|| {
            OrganizationWindowComparisonError::MissingOrganization(row.organization.clone())
        })?;
        let rebased_row = rebased.organization(&row.organization).ok_or_else(|| {
            OrganizationWindowComparisonError::MissingOrganization(row.organization.clone())
        })?;
        let method_delta = delta(original.overall.score, rebased_row.overall.score);
        let observed_delta = delta(rebased_row.overall.score, row.later_score);
        let actual_delta = delta(original.overall.score, row.later_score);
        row.method_manifest_delta = method_delta;
        row.observed_input_delta = observed_delta;
        row.residual_revaluation = actual_delta.zip(method_delta).zip(observed_delta).map(
            |((actual, method), observed)| {
                let residual = actual - method - observed;
                if residual.abs() < 1e-10 {
                    0.0
                } else {
                    residual
                }
            },
        );
    }
    Ok(movement)
}

pub fn build_organization_window_history(
    checkpoints: &[OrganizationWindowBoardView],
) -> Result<OrganizationWindowHistoryView, OrganizationWindowComparisonError> {
    if checkpoints.len() < 2 {
        return Err(OrganizationWindowComparisonError::InsufficientHistory);
    }
    let mut ordered = checkpoints.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|board| board.as_of);
    for pair in ordered.windows(2) {
        if pair[0].as_of == pair[1].as_of {
            return Err(OrganizationWindowComparisonError::InvalidCheckpointOrder);
        }
    }
    let movements = ordered
        .windows(2)
        .map(|pair| compare_organization_window_snapshots(pair[0], pair[1]))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(OrganizationWindowHistoryView {
        schema: ORGANIZATION_WINDOW_HISTORY_SCHEMA.to_owned(),
        season: ordered[0].season,
        manifest_fingerprint: ordered[0].manifest.fingerprint.clone(),
        checkpoint_fingerprints: ordered
            .iter()
            .map(|board| board.fingerprint.clone())
            .collect(),
        movements,
    })
}

pub fn compare_organization_window_scenario(
    scenario_id: &str,
    baseline: &OrganizationWindowBoardView,
    scenario: &OrganizationWindowBoardView,
) -> Result<OrganizationWindowScenarioImpactView, OrganizationWindowComparisonError> {
    if scenario_id.trim().is_empty() {
        return Err(OrganizationWindowComparisonError::EmptyScenarioId);
    }
    validate_common(baseline, scenario, true)?;
    Ok(OrganizationWindowScenarioImpactView {
        schema: ORGANIZATION_WINDOW_SCENARIO_IMPACT_SCHEMA.to_owned(),
        scenario_id: scenario_id.to_owned(),
        season: baseline.season,
        as_of: baseline.as_of,
        manifest_fingerprint: baseline.manifest.fingerprint.clone(),
        baseline_board_fingerprint: baseline.fingerprint.clone(),
        scenario_board_fingerprint: scenario.fingerprint.clone(),
        authorities: Vec::new(),
        profile_impacts: Vec::new(),
        organizations: compare_organizations(baseline, scenario)?,
        disclosures: vec![
            "Scenario impacts compare sealed same-context boards and do not rewrite the baseline artifact.".to_owned(),
            "Interaction effects belong to combined scenarios and need not equal isolated impacts.".to_owned(),
        ],
    })
}

/// Compare a scenario with explicit upstream authorities and profile-level
/// attribution. A direct raw/evidence change must be covered by an authority
/// scoped to that organization and profile. A normalized-only change may be
/// attributed to the cohort effect of an authority changing the same profile
/// elsewhere in the league.
pub fn compare_organization_window_typed_scenario(
    scenario_id: &str,
    baseline: &OrganizationWindowBoardView,
    scenario: &OrganizationWindowBoardView,
    mut authorities: Vec<WindowScenarioAuthorityView>,
) -> Result<OrganizationWindowScenarioImpactView, OrganizationWindowComparisonError> {
    let mut impact = compare_organization_window_scenario(scenario_id, baseline, scenario)?;
    validate_scenario_authorities(scenario, &mut authorities)?;
    let mut profile_impacts = Vec::new();
    for organization in &impact.organizations {
        for dimension in &organization.dimensions {
            for profile in &dimension.profiles {
                let raw_changed =
                    option_values_changed(profile.earlier_raw_value, profile.later_raw_value);
                let evidence_changed =
                    profile.confidence_delta.abs() > 1e-10 || profile.coverage_delta.abs() > 1e-10;
                let score_changed =
                    option_values_changed(profile.earlier_score, profile.later_score);
                let kind = if raw_changed {
                    WindowScenarioProfileImpactKind::RawInput
                } else if evidence_changed {
                    WindowScenarioProfileImpactKind::Evidence
                } else if score_changed {
                    WindowScenarioProfileImpactKind::CohortRevaluation
                } else {
                    WindowScenarioProfileImpactKind::Unchanged
                };
                let mut authority_ids = authorities
                    .iter()
                    .filter(|authority| {
                        authority.profile_methods.iter().any(|method| {
                            method.profile_key == profile.profile_key
                                && method.method_version == profile.method_version
                        }) && (kind == WindowScenarioProfileImpactKind::CohortRevaluation
                            || authority.organizations.contains(&organization.organization))
                    })
                    .map(|authority| authority.authority_id.clone())
                    .collect::<Vec<_>>();
                authority_ids.sort();
                authority_ids.dedup();
                if kind != WindowScenarioProfileImpactKind::Unchanged && authority_ids.is_empty() {
                    return Err(
                        OrganizationWindowComparisonError::UnattributedScenarioChange(format!(
                            "{}:{}@{} ({kind:?})",
                            organization.organization, profile.profile_key, profile.method_version
                        )),
                    );
                }
                profile_impacts.push(WindowScenarioProfileImpactView {
                    organization: organization.organization.clone(),
                    dimension_key: dimension.dimension_key.clone(),
                    profile_key: profile.profile_key.clone(),
                    method_version: profile.method_version.clone(),
                    kind,
                    raw_delta: profile.raw_delta,
                    score_delta: profile.score_delta,
                    confidence_delta: profile.confidence_delta,
                    coverage_delta: profile.coverage_delta,
                    authority_ids,
                });
            }
        }
        if option_values_changed(organization.earlier_score, organization.later_score)
            && !profile_impacts.iter().any(|profile| {
                profile.organization == organization.organization
                    && profile.kind != WindowScenarioProfileImpactKind::Unchanged
            })
        {
            return Err(
                OrganizationWindowComparisonError::UnattributedScenarioChange(format!(
                    "{} overall score changed without a profile change",
                    organization.organization
                )),
            );
        }
    }
    impact.authorities = authorities;
    impact.profile_impacts = profile_impacts;
    impact.disclosures.push(
        "Typed scenario attribution distinguishes direct raw/evidence changes, cohort normalization revaluation, and unchanged profiles."
            .to_owned(),
    );
    Ok(impact)
}

pub fn adapt_team_season_window_scenario_authorities(
    forecast: &TeamSeasonForecastView,
) -> Result<Vec<WindowScenarioAuthorityView>, OrganizationWindowComparisonError> {
    if forecast.schema != TEAM_SEASON_FORECAST_SCHEMA {
        return Err(OrganizationWindowComparisonError::InvalidScenarioAuthority(
            format!("unsupported team-season schema {}", forecast.schema),
        ));
    }
    let scenario = forecast.scenario.as_ref().ok_or_else(|| {
        OrganizationWindowComparisonError::InvalidScenarioAuthority(
            "team-season forecast has no scenario".to_owned(),
        )
    })?;
    let fingerprint = source_document_fingerprint(forecast)?;
    let mut organizations = forecast
        .teams
        .iter()
        .map(|team| team.team.clone())
        .collect::<Vec<_>>();
    organizations.sort();
    organizations.dedup();
    if organizations.is_empty() {
        return Err(OrganizationWindowComparisonError::InvalidScenarioAuthority(
            "team-season forecast has no organization outcomes".to_owned(),
        ));
    }
    scenario
        .events
        .iter()
        .map(|event| {
            let kind = match event.kind {
                TeamSeasonScenarioEventKind::Trade => WindowScenarioAuthorityKind::Trade,
                TeamSeasonScenarioEventKind::Injury | TeamSeasonScenarioEventKind::Return => {
                    WindowScenarioAuthorityKind::Injury
                }
                TeamSeasonScenarioEventKind::Goalie => WindowScenarioAuthorityKind::Goalie,
                TeamSeasonScenarioEventKind::Form => WindowScenarioAuthorityKind::PlayerDevelopment,
                TeamSeasonScenarioEventKind::Custom => WindowScenarioAuthorityKind::Custom,
            };
            Ok(WindowScenarioAuthorityView {
                authority_id: event.id.clone(),
                kind,
                source_schema: TEAM_SEASON_FORECAST_SCHEMA.to_owned(),
                source_fingerprint: fingerprint.clone(),
                organizations: organizations.clone(),
                profile_methods: vec![scenario_profile(
                    "nhl.expected_points",
                    "icecast_expected_points.v1",
                )],
                rationale: format!(
                    "{}; originating team {} with league-simulation consequences",
                    event.label, event.team
                ),
            })
        })
        .collect()
}

pub fn adapt_training_camp_window_scenario_authorities(
    forecast: &TrainingCampLeagueForecastView,
) -> Result<Vec<WindowScenarioAuthorityView>, OrganizationWindowComparisonError> {
    if forecast.schema != TRAINING_CAMP_LEAGUE_FORECAST_SCHEMA {
        return Err(OrganizationWindowComparisonError::InvalidScenarioAuthority(
            format!("unsupported training-camp schema {}", forecast.schema),
        ));
    }
    let fingerprint = source_document_fingerprint(forecast)?;
    Ok(forecast
        .teams
        .iter()
        .filter(|team| team.forecast.is_some())
        .map(|team| WindowScenarioAuthorityView {
            authority_id: format!("training-camp:{}", team.team),
            kind: WindowScenarioAuthorityKind::TrainingCamp,
            source_schema: TRAINING_CAMP_LEAGUE_FORECAST_SCHEMA.to_owned(),
            source_fingerprint: fingerprint.clone(),
            organizations: vec![team.team.clone()],
            profile_methods: vec![scenario_profile(
                "pipeline.training_camp_arrival",
                "training_camp_arrival.v1",
            )],
            rationale: "Opening-roster trial outcomes change expected rookie arrivals.".to_owned(),
        })
        .collect())
}

pub fn adapt_line_combination_window_scenario_authority(
    forecast: &LineCombinationForecastView,
    candidate_id: &str,
) -> Result<WindowScenarioAuthorityView, OrganizationWindowComparisonError> {
    if forecast.schema != LINE_COMBINATION_FORECAST_SCHEMA {
        return Err(OrganizationWindowComparisonError::InvalidScenarioAuthority(
            format!("unsupported line-combination schema {}", forecast.schema),
        ));
    }
    let candidate = forecast
        .candidates
        .iter()
        .find(|candidate| candidate.id == candidate_id)
        .ok_or_else(|| {
            OrganizationWindowComparisonError::InvalidScenarioAuthority(format!(
                "line-combination candidate {candidate_id} is missing"
            ))
        })?;
    if candidate.is_baseline {
        return Err(OrganizationWindowComparisonError::InvalidScenarioAuthority(
            "line-combination scenario selects the baseline".to_owned(),
        ));
    }
    Ok(WindowScenarioAuthorityView {
        authority_id: format!("line-combination:{}:{candidate_id}", forecast.team),
        kind: WindowScenarioAuthorityKind::LineCombination,
        source_schema: LINE_COMBINATION_FORECAST_SCHEMA.to_owned(),
        source_fingerprint: source_document_fingerprint(forecast)?,
        organizations: vec![forecast.team.clone()],
        profile_methods: vec![scenario_profile(
            "deployment.lineup_optionality",
            "line_combination_optionality.v1",
        )],
        rationale: format!(
            "Selected line candidate {} ({})",
            candidate.id, candidate.label
        ),
    })
}

fn validate_scenario_authorities(
    scenario: &OrganizationWindowBoardView,
    authorities: &mut [WindowScenarioAuthorityView],
) -> Result<(), OrganizationWindowComparisonError> {
    if authorities.is_empty() {
        return Err(OrganizationWindowComparisonError::InvalidScenarioAuthority(
            "no typed authorities supplied".to_owned(),
        ));
    }
    let manifest_profiles = scenario
        .manifest
        .dimensions
        .iter()
        .flat_map(|dimension| dimension.profiles.iter())
        .map(|profile| (profile.profile_key.clone(), profile.method_version.clone()))
        .collect::<BTreeSet<_>>();
    let expected = scenario
        .expected_organizations
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut ids = BTreeSet::new();
    for authority in authorities.iter_mut() {
        authority.organizations.sort();
        authority.organizations.dedup();
        authority.profile_methods.sort();
        authority.profile_methods.dedup();
        if authority.authority_id.trim().is_empty()
            || authority.source_schema.trim().is_empty()
            || authority.rationale.trim().is_empty()
            || authority.organizations.is_empty()
            || authority.profile_methods.is_empty()
            || !ids.insert(authority.authority_id.clone())
        {
            return Err(OrganizationWindowComparisonError::InvalidScenarioAuthority(
                format!(
                    "{} has empty or duplicate identity/scope",
                    authority.authority_id
                ),
            ));
        }
        if !is_source_fingerprint(&authority.source_fingerprint)
            || !scenario
                .source_fingerprints
                .contains(&authority.source_fingerprint)
        {
            return Err(OrganizationWindowComparisonError::InvalidScenarioAuthority(
                format!(
                    "{} source fingerprint is invalid or absent from the scenario board",
                    authority.authority_id
                ),
            ));
        }
        if authority
            .organizations
            .iter()
            .any(|organization| !expected.contains(organization))
        {
            return Err(OrganizationWindowComparisonError::InvalidScenarioAuthority(
                format!(
                    "{} names an organization outside the cohort",
                    authority.authority_id
                ),
            ));
        }
        if authority.profile_methods.iter().any(|method| {
            !manifest_profiles
                .contains(&(method.profile_key.clone(), method.method_version.clone()))
        }) {
            return Err(OrganizationWindowComparisonError::InvalidScenarioAuthority(
                format!(
                    "{} names a profile outside the Frame",
                    authority.authority_id
                ),
            ));
        }
    }
    authorities.sort_by(|a, b| a.authority_id.cmp(&b.authority_id));
    Ok(())
}

fn scenario_profile(profile_key: &str, method_version: &str) -> WindowScenarioProfileMethodView {
    WindowScenarioProfileMethodView {
        profile_key: profile_key.to_owned(),
        method_version: method_version.to_owned(),
    }
}

fn source_document_fingerprint<T: Serialize>(
    document: &T,
) -> Result<String, OrganizationWindowComparisonError> {
    let bytes = serde_json::to_vec(document).map_err(|error| {
        OrganizationWindowComparisonError::InvalidScenarioAuthority(error.to_string())
    })?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn option_values_changed(earlier: Option<f64>, later: Option<f64>) -> bool {
    match (earlier, later) {
        (Some(earlier), Some(later)) => (later - earlier).abs() > 1e-10,
        (None, None) => false,
        _ => true,
    }
}

fn validate_common(
    earlier: &OrganizationWindowBoardView,
    later: &OrganizationWindowBoardView,
    same_checkpoint: bool,
) -> Result<(), OrganizationWindowComparisonError> {
    for board in [earlier, later] {
        validate_board_fingerprint(board)?;
    }
    if earlier.fingerprint == later.fingerprint {
        return Err(OrganizationWindowComparisonError::IdenticalBoard);
    }
    if earlier.manifest.fingerprint != later.manifest.fingerprint {
        return Err(OrganizationWindowComparisonError::Incomparable(
            "manifest fingerprint".to_owned(),
        ));
    }
    if earlier.season != later.season
        || earlier.season_type != later.season_type
        || earlier.expected_organizations != later.expected_organizations
    {
        return Err(OrganizationWindowComparisonError::Incomparable(
            "season, season type, or cohort".to_owned(),
        ));
    }
    if same_checkpoint && earlier.as_of != later.as_of {
        return Err(OrganizationWindowComparisonError::Incomparable(
            "scenario as-of".to_owned(),
        ));
    }
    validate_sources(earlier)?;
    validate_sources(later)
}

fn validate_board_fingerprint(
    board: &OrganizationWindowBoardView,
) -> Result<(), OrganizationWindowComparisonError> {
    let inventory = load_organization_window_profile_inventory().map_err(|_| {
        OrganizationWindowComparisonError::InvalidBoardFingerprint(board.fingerprint.clone())
    })?;
    validate_organization_window_board(board, &inventory).map_err(|_| {
        OrganizationWindowComparisonError::InvalidBoardFingerprint(board.fingerprint.clone())
    })
}

fn is_fingerprint(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_source_fingerprint(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(is_fingerprint) || is_fingerprint(value)
}

fn validate_sources(
    board: &OrganizationWindowBoardView,
) -> Result<(), OrganizationWindowComparisonError> {
    if board.source_fingerprints.is_empty() {
        return Err(
            OrganizationWindowComparisonError::MissingSourceFingerprints(board.fingerprint.clone()),
        );
    }
    let missing = board.organizations.iter().any(|organization| {
        organization.dimensions.iter().any(|dimension| {
            dimension.profiles.iter().any(|profile| {
                profile.normalized_score.is_some() && profile.source_fingerprints.is_empty()
            })
        })
    });
    if missing {
        return Err(
            OrganizationWindowComparisonError::MissingSourceFingerprints(board.fingerprint.clone()),
        );
    }
    Ok(())
}

fn compare_organizations(
    earlier: &OrganizationWindowBoardView,
    later: &OrganizationWindowBoardView,
) -> Result<Vec<WindowOrganizationDeltaView>, OrganizationWindowComparisonError> {
    let later_by_team = later
        .organizations
        .iter()
        .map(|row| (row.organization.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let mut rows = Vec::with_capacity(earlier.organizations.len());
    for prior in &earlier.organizations {
        let next = later_by_team
            .get(prior.organization.as_str())
            .ok_or_else(|| {
                OrganizationWindowComparisonError::MissingOrganization(prior.organization.clone())
            })?;
        rows.push(compare_organization(prior, next)?);
    }
    Ok(rows)
}

fn compare_organization(
    earlier: &WindowOrganizationView,
    later: &WindowOrganizationView,
) -> Result<WindowOrganizationDeltaView, OrganizationWindowComparisonError> {
    let later_dimensions = later
        .dimensions
        .iter()
        .map(|row| (row.key.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let mut dimensions = Vec::with_capacity(earlier.dimensions.len());
    for prior in &earlier.dimensions {
        let next = later_dimensions.get(prior.key.as_str()).ok_or_else(|| {
            OrganizationWindowComparisonError::MissingDimension {
                organization: earlier.organization.clone(),
                dimension: prior.key.clone(),
            }
        })?;
        let next_profiles = next
            .profiles
            .iter()
            .map(|row| ((row.profile_key.as_str(), row.method_version.as_str()), row))
            .collect::<BTreeMap<_, _>>();
        let mut profiles = Vec::with_capacity(prior.profiles.len());
        for prior_profile in &prior.profiles {
            let key = (
                prior_profile.profile_key.as_str(),
                prior_profile.method_version.as_str(),
            );
            let next_profile = next_profiles.get(&key).ok_or_else(|| {
                OrganizationWindowComparisonError::MissingProfile {
                    organization: earlier.organization.clone(),
                    profile: format!("{}@{}", key.0, key.1),
                }
            })?;
            profiles.push(profile_delta(prior_profile, next_profile));
        }
        dimensions.push(WindowDimensionDeltaView {
            dimension_key: prior.key.clone(),
            earlier_score: prior.score,
            later_score: next.score,
            score_delta: delta(prior.score, next.score),
            confidence_delta: next.confidence - prior.confidence,
            coverage_delta: next.coverage - prior.coverage,
            profiles,
        });
    }
    let score_delta = delta(earlier.overall.score, later.overall.score);
    Ok(WindowOrganizationDeltaView {
        organization: earlier.organization.clone(),
        earlier_score: earlier.overall.score,
        later_score: later.overall.score,
        score_delta,
        rank_delta: earlier
            .overall
            .rank
            .zip(later.overall.rank)
            .map(|(prior, next)| prior as i32 - next as i32),
        confidence_delta: later.overall.confidence - earlier.overall.confidence,
        coverage_delta: later.overall.coverage - earlier.overall.coverage,
        earlier_classification: earlier.overall.classification,
        later_classification: later.overall.classification,
        dimensions,
        observed_input_delta: score_delta,
        personnel_delta: None,
        method_manifest_delta: Some(0.0),
        residual_revaluation: score_delta.map(|_| 0.0),
    })
}

fn profile_delta(
    earlier: &OrganizationProfileObservationView,
    later: &OrganizationProfileObservationView,
) -> WindowProfileDeltaView {
    WindowProfileDeltaView {
        profile_key: earlier.profile_key.clone(),
        method_version: earlier.method_version.clone(),
        earlier_raw_value: earlier.raw_value,
        later_raw_value: later.raw_value,
        raw_delta: delta(earlier.raw_value, later.raw_value),
        earlier_score: earlier.normalized_score,
        later_score: later.normalized_score,
        score_delta: delta(earlier.normalized_score, later.normalized_score),
        confidence_delta: later.confidence - earlier.confidence,
        coverage_delta: later.coverage - earlier.coverage,
    }
}

fn delta(earlier: Option<f64>, later: Option<f64>) -> Option<f64> {
    earlier.zip(later).map(|(earlier, later)| later - earlier)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_model::organization_window::{
        load_organization_window_profile_inventory, seal_organization_window_manifest,
        WindowHorizon,
    };
    use crate::view_model::organization_window_adapters::{
        build_balanced_organization_window_board, OrganizationWindowAdapterContext,
        OrganizationWindowSourceSet,
    };

    fn empty_board(date: NaiveDate, generated_at: &str) -> OrganizationWindowBoardView {
        build_balanced_organization_window_board(
            OrganizationWindowAdapterContext {
                season: 20_262_027,
                season_type: "regular".to_owned(),
                as_of: date,
                horizon: WindowHorizon::Current,
                organization_identity_version: "nhl_32.v1".to_owned(),
            },
            generated_at,
            OrganizationWindowSourceSet::default(),
        )
        .unwrap()
    }

    #[test]
    fn comparison_requires_source_complete_boards() {
        let earlier = empty_board(
            NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
            "2026-10-01T00:00:00Z",
        );
        let later = empty_board(
            NaiveDate::from_ymd_opt(2026, 11, 1).unwrap(),
            "2026-11-01T00:00:00Z",
        );
        assert!(matches!(
            compare_organization_window_snapshots(&earlier, &later),
            Err(OrganizationWindowComparisonError::MissingSourceFingerprints(_))
        ));
    }

    #[test]
    fn scenario_requires_same_checkpoint_and_named_scenario() {
        let baseline = empty_board(
            NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
            "2026-10-01T00:00:00Z",
        );
        let scenario = empty_board(
            NaiveDate::from_ymd_opt(2026, 10, 2).unwrap(),
            "2026-10-02T00:00:00Z",
        );
        assert_eq!(
            compare_organization_window_scenario("", &baseline, &scenario),
            Err(OrganizationWindowComparisonError::EmptyScenarioId)
        );
        assert!(matches!(
            compare_organization_window_scenario("trade", &baseline, &scenario),
            Err(OrganizationWindowComparisonError::Incomparable(_))
        ));
    }

    fn evaluation_board() -> OrganizationWindowBoardView {
        serde_json::from_str(include_str!(
            "../../../examples/organization-window-board-evaluation-2026-27.json"
        ))
        .unwrap()
    }

    fn at_checkpoint(
        board: OrganizationWindowBoardView,
        date: NaiveDate,
        generated_at: &str,
        score_delta: f64,
    ) -> OrganizationWindowBoardView {
        let mut changed_nyr = false;
        let profile_inputs = board
            .organizations
            .iter()
            .flat_map(|organization| {
                organization.dimensions.iter().flat_map(|dimension| {
                    dimension
                        .profiles
                        .iter()
                        .map(move |profile| (dimension.score.is_some(), profile))
                })
            })
            .map(|(contributes_to_overall, profile)| {
                let mut raw_value = profile.raw_value;
                if !changed_nyr
                    && profile.organization == "NYR"
                    && contributes_to_overall
                    && raw_value.is_some()
                    && profile.normalized_score.is_some()
                    && score_delta != 0.0
                {
                    raw_value = raw_value.map(|value| value + score_delta * 10_000.0);
                    changed_nyr = true;
                }
                OrganizationProfileInput {
                    profile_key: profile.profile_key.clone(),
                    method_version: profile.method_version.clone(),
                    organization: profile.organization.clone(),
                    organization_identity_version: profile.organization_identity_version.clone(),
                    season: board.season,
                    season_type: profile.season_type.clone(),
                    as_of: date,
                    horizon: profile.horizon,
                    raw_value,
                    raw_unit: profile.raw_unit.clone(),
                    sample_size: profile.sample_size,
                    confidence: profile.confidence,
                    coverage: profile.coverage,
                    status: profile.status,
                    evidence: profile.evidence.clone(),
                    limitations: profile.limitations.clone(),
                    source_fingerprints: profile.source_fingerprints.clone(),
                }
            })
            .collect();
        let inventory = load_organization_window_profile_inventory().unwrap();
        build_organization_window_board(
            OrganizationWindowBoardInput {
                season: board.season,
                season_type: board.season_type,
                as_of: date,
                generated_at: generated_at.to_owned(),
                manifest: board.manifest,
                profile_inputs,
                source_fingerprints: board.source_fingerprints,
            },
            &inventory,
        )
        .unwrap()
    }

    #[test]
    fn three_sealed_checkpoints_and_same_date_scenario_compare() {
        let base = evaluation_board();
        let october = at_checkpoint(
            base.clone(),
            NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
            "2026-10-01T00:00:00Z",
            0.0,
        );
        let january = at_checkpoint(
            base.clone(),
            NaiveDate::from_ymd_opt(2027, 1, 1).unwrap(),
            "2027-01-01T00:00:00Z",
            2.0,
        );
        let march = at_checkpoint(
            base,
            NaiveDate::from_ymd_opt(2027, 3, 1).unwrap(),
            "2027-03-01T00:00:00Z",
            3.0,
        );
        let history =
            build_organization_window_history(&[march.clone(), october.clone(), january.clone()])
                .unwrap();
        assert_eq!(history.movements.len(), 2);
        assert_eq!(history.checkpoint_fingerprints[0], october.fingerprint);

        let scenario = at_checkpoint(january.clone(), january.as_of, "2027-01-01T00:01:00Z", -4.0);
        let impact =
            compare_organization_window_scenario("deadline-addition", &january, &scenario).unwrap();
        assert_eq!(impact.organizations.len(), 32);
        assert!(impact
            .organizations
            .iter()
            .find(|row| row.organization == "NYR")
            .unwrap()
            .score_delta
            .is_some_and(|delta| delta.abs() > 0.0));

        let mut legacy_scenario = serde_json::to_value(&impact).unwrap();
        let legacy_scenario = legacy_scenario.as_object_mut().unwrap();
        legacy_scenario.remove("authorities");
        legacy_scenario.remove("profile_impacts");
        let decoded: OrganizationWindowScenarioImpactView =
            serde_json::from_value(serde_json::Value::Object(legacy_scenario.clone())).unwrap();
        assert!(decoded.authorities.is_empty());
        assert!(decoded.profile_impacts.is_empty());

        let mut legacy_movement = serde_json::to_value(&history.movements[0]).unwrap();
        let legacy_movement = legacy_movement.as_object_mut().unwrap();
        legacy_movement.remove("source_manifest_fingerprint");
        legacy_movement.remove("bridge_fingerprint");
        legacy_movement.remove("rebased_earlier_board_fingerprint");
        let decoded: OrganizationWindowMovementView =
            serde_json::from_value(serde_json::Value::Object(legacy_movement.clone())).unwrap();
        assert!(decoded.source_manifest_fingerprint.is_none());
        assert!(decoded.bridge_fingerprint.is_none());
        assert!(decoded.rebased_earlier_board_fingerprint.is_none());
    }

    fn reweighted_manifest(
        source: &OrganizationWindowBoardView,
        inventory: &OrganizationWindowProfileInventory,
    ) -> OrganizationWindowManifestView {
        let mut manifest = source.manifest.clone();
        manifest.manifest_id = "balanced-reweighted.v1".to_owned();
        manifest.manifest_version = "1.0.0".to_owned();
        manifest.created_at = "2026-07-28T00:00:00Z".to_owned();
        manifest.fingerprint.clear();
        let pipeline = manifest
            .dimensions
            .iter_mut()
            .find(|dimension| dimension.key == "pipeline")
            .unwrap();
        for profile in &mut pipeline.profiles {
            profile.weight = match profile.profile_key.as_str() {
                "pipeline.prospect_development" => 0.15,
                "pipeline.prospect_pool" => 0.40,
                _ => profile.weight,
            };
        }
        seal_organization_window_manifest(manifest, inventory).unwrap()
    }

    fn identity_bridge(
        source: &OrganizationWindowBoardView,
        target: &OrganizationWindowManifestView,
    ) -> OrganizationWindowBridgeView {
        let mappings = target
            .dimensions
            .iter()
            .flat_map(|dimension| dimension.profiles.iter())
            .map(|profile| WindowProfileBridgeView {
                from_profile_key: profile.profile_key.clone(),
                from_method_version: profile.method_version.clone(),
                to_profile_key: profile.profile_key.clone(),
                to_method_version: profile.method_version.clone(),
                raw_scale: 1.0,
                raw_offset: 0.0,
                rationale: "The profile method is unchanged; only Frame weights changed."
                    .to_owned(),
                evidence_fingerprints: vec!["d".repeat(64)],
            })
            .collect();
        seal_organization_window_bridge(OrganizationWindowBridgeView {
            schema: ORGANIZATION_WINDOW_BRIDGE_SCHEMA.to_owned(),
            bridge_id: "balanced-v1-to-reweighted-v1".to_owned(),
            created_at: "2026-07-28T00:00:00Z".to_owned(),
            from_manifest_fingerprint: source.manifest.fingerprint.clone(),
            to_manifest_fingerprint: target.fingerprint.clone(),
            profile_mappings: mappings,
            disclosures: vec![
                "This bridge changes Frame weights without changing profile methods.".to_owned(),
            ],
            fingerprint: String::new(),
        })
        .unwrap()
    }

    #[test]
    fn explicit_bridge_rebases_through_canonical_scorer_and_attributes_change() {
        let inventory = load_organization_window_profile_inventory().unwrap();
        let source = evaluation_board();
        let target = reweighted_manifest(&source, &inventory);
        let bridge = identity_bridge(&source, &target);

        let rebased =
            rebase_organization_window_board(&source, &target, &bridge, &inventory).unwrap();
        assert_eq!(rebased.manifest.fingerprint, target.fingerprint);
        assert_eq!(
            source.fingerprint,
            evaluation_board().fingerprint,
            "rebase must not mutate the sealed source board"
        );
        assert_ne!(rebased.fingerprint, source.fingerprint);

        let mut later = rebased.clone();
        later.as_of = NaiveDate::from_ymd_opt(2026, 8, 1).unwrap();
        later.generated_at = "2026-08-01T00:00:00Z".to_owned();
        for organization in &mut later.organizations {
            for dimension in &mut organization.dimensions {
                for profile in &mut dimension.profiles {
                    profile.as_of = later.as_of;
                }
            }
        }
        later.fingerprint.clear();
        later.fingerprint = later.calculate_fingerprint().unwrap();

        let movement =
            compare_organization_window_snapshots_with_bridge(&source, &later, &bridge, &inventory)
                .unwrap();
        assert_eq!(
            movement.source_manifest_fingerprint.as_deref(),
            Some(source.manifest.fingerprint.as_str())
        );
        assert_eq!(
            movement.bridge_fingerprint.as_deref(),
            Some(bridge.fingerprint.as_str())
        );
        assert_eq!(
            movement.rebased_earlier_board_fingerprint.as_deref(),
            Some(rebased.fingerprint.as_str())
        );
        let nyr = movement
            .organizations
            .iter()
            .find(|row| row.organization == "NYR")
            .unwrap();
        assert!(nyr.method_manifest_delta.is_some_and(|delta| delta != 0.0));
        assert_eq!(nyr.observed_input_delta, Some(0.0));
        assert_eq!(nyr.residual_revaluation, Some(0.0));
    }

    #[test]
    fn bridge_refuses_missing_mappings_and_tampered_identity() {
        let inventory = load_organization_window_profile_inventory().unwrap();
        let source = evaluation_board();
        let target = reweighted_manifest(&source, &inventory);
        let mut bridge = identity_bridge(&source, &target);
        bridge.profile_mappings.pop();
        bridge.fingerprint.clear();
        bridge = seal_organization_window_bridge(bridge).unwrap();
        assert!(matches!(
            rebase_organization_window_board(&source, &target, &bridge, &inventory),
            Err(OrganizationWindowComparisonError::InvalidBridge(_))
        ));

        let mut tampered = identity_bridge(&source, &target);
        tampered.profile_mappings[0].raw_offset = 1.0;
        assert_eq!(
            seal_organization_window_bridge(tampered),
            Err(OrganizationWindowComparisonError::BridgeFingerprintMismatch)
        );
    }

    fn scenario_board_with_nyr_pipeline_change(
        baseline: &OrganizationWindowBoardView,
        source_fingerprint: &str,
    ) -> OrganizationWindowBoardView {
        let inventory = load_organization_window_profile_inventory().unwrap();
        let mut profile_inputs = Vec::new();
        for organization in &baseline.organizations {
            for dimension in &organization.dimensions {
                for profile in &dimension.profiles {
                    let changed = organization.organization == "NYR"
                        && profile.profile_key == "pipeline.prospect_pool";
                    let mut fingerprints = profile.source_fingerprints.clone();
                    if changed {
                        fingerprints.push(source_fingerprint.to_owned());
                    }
                    profile_inputs.push(OrganizationProfileInput {
                        profile_key: profile.profile_key.clone(),
                        method_version: profile.method_version.clone(),
                        organization: organization.organization.clone(),
                        organization_identity_version: profile
                            .organization_identity_version
                            .clone(),
                        season: profile.season,
                        season_type: profile.season_type.clone(),
                        as_of: profile.as_of,
                        horizon: profile.horizon,
                        raw_value: profile.raw_value.map(|value| {
                            if changed {
                                value + 10.0
                            } else {
                                value
                            }
                        }),
                        raw_unit: profile.raw_unit.clone(),
                        sample_size: profile.sample_size,
                        confidence: profile.confidence,
                        coverage: profile.coverage,
                        status: profile.status,
                        evidence: profile.evidence.clone(),
                        limitations: profile.limitations.clone(),
                        source_fingerprints: fingerprints,
                    });
                }
            }
        }
        let mut source_fingerprints = baseline.source_fingerprints.clone();
        source_fingerprints.push(source_fingerprint.to_owned());
        build_organization_window_board(
            OrganizationWindowBoardInput {
                season: baseline.season,
                season_type: baseline.season_type.clone(),
                as_of: baseline.as_of,
                generated_at: "2026-07-27T00:01:00Z".to_owned(),
                manifest: baseline.manifest.clone(),
                profile_inputs,
                source_fingerprints,
            },
            &inventory,
        )
        .unwrap()
    }

    #[test]
    fn typed_scenario_names_direct_cohort_and_unchanged_profile_impacts() {
        let baseline = evaluation_board();
        let source_fingerprint = format!("sha256:{}", "e".repeat(64));
        let scenario = scenario_board_with_nyr_pipeline_change(&baseline, &source_fingerprint);
        let authority = WindowScenarioAuthorityView {
            authority_id: "nyr-prospect-breakout".to_owned(),
            kind: WindowScenarioAuthorityKind::PlayerDevelopment,
            source_schema: "player_development_scenario.v1".to_owned(),
            source_fingerprint: source_fingerprint.clone(),
            organizations: vec!["NYR".to_owned()],
            profile_methods: vec![scenario_profile(
                "pipeline.prospect_pool",
                "prospect_pool_score.v1",
            )],
            rationale: "A prospect breakout raises the evaluated pool value.".to_owned(),
        };
        let impact = compare_organization_window_typed_scenario(
            "nyr-prospect-breakout",
            &baseline,
            &scenario,
            vec![authority.clone()],
        )
        .unwrap();
        assert_eq!(impact.authorities, vec![authority]);
        assert!(impact.profile_impacts.iter().any(|row| {
            row.organization == "NYR"
                && row.profile_key == "pipeline.prospect_pool"
                && row.kind == WindowScenarioProfileImpactKind::RawInput
                && row.authority_ids == vec!["nyr-prospect-breakout"]
        }));
        assert!(impact.profile_impacts.iter().any(|row| {
            row.organization != "NYR"
                && row.profile_key == "pipeline.prospect_pool"
                && row.kind == WindowScenarioProfileImpactKind::CohortRevaluation
        }));
        assert!(impact
            .profile_impacts
            .iter()
            .any(|row| row.kind == WindowScenarioProfileImpactKind::Unchanged));

        let wrong_authority = WindowScenarioAuthorityView {
            profile_methods: vec![scenario_profile(
                "pipeline.prospect_development",
                "prospect_development_score.v1",
            )],
            ..impact.authorities[0].clone()
        };
        assert!(matches!(
            compare_organization_window_typed_scenario(
                "unattributed",
                &baseline,
                &scenario,
                vec![wrong_authority],
            ),
            Err(OrganizationWindowComparisonError::UnattributedScenarioChange(_))
        ));
    }

    #[test]
    fn team_season_authorities_cover_league_simulation_consequences() {
        let forecast: TeamSeasonForecastView = serde_json::from_str(include_str!(
            "../../../examples/icecast-nyr-development-variance-10000-result.json"
        ))
        .unwrap();
        let authorities = adapt_team_season_window_scenario_authorities(&forecast).unwrap();
        assert_eq!(authorities.len(), forecast.scenario.unwrap().events.len());
        assert!(authorities
            .iter()
            .all(|authority| authority.organizations.len() == 32));
        assert!(authorities
            .iter()
            .all(|authority| authority.rationale.contains("originating team NYR")));
    }
}
