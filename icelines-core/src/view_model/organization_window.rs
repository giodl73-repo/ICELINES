//! The Window: extensible, explainable organization-health profiles and boards.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::teams::CANONICAL_TEAMS;

pub const ORGANIZATION_WINDOW_PROFILE_INVENTORY_SCHEMA: &str =
    "organization_window_profile_inventory.v1";
pub const ORGANIZATION_PROFILE_OBSERVATION_SCHEMA: &str = "organization_profile_observation.v1";
pub const ORGANIZATION_WINDOW_MANIFEST_SCHEMA: &str = "organization_window_manifest.v1";
pub const ORGANIZATION_WINDOW_BOARD_SCHEMA: &str = "organization_window_board.v1";
pub const ORGANIZATION_WINDOW_CLASSIFICATION_METHOD: &str = "current_strength_sustainability.v1";
pub const ORGANIZATION_WINDOW_REGISTRY_VERSION: &str = "organization_window_registry.v1";

pub const ORGANIZATION_WINDOW_PROFILE_INVENTORY_JSON: &str =
    include_str!("../../../design/data/organization-window-profile-inventory.v1.json");
pub const ORGANIZATION_PROFILE_OBSERVATION_JSON_SCHEMA: &str =
    include_str!("../../../design/schemas/organization_profile_observation.v1.schema.json");
pub const ORGANIZATION_WINDOW_MANIFEST_JSON_SCHEMA: &str =
    include_str!("../../../design/schemas/organization_window_manifest.v1.schema.json");
pub const ORGANIZATION_WINDOW_BOARD_JSON_SCHEMA: &str =
    include_str!("../../../design/schemas/organization_window_board.v1.schema.json");

const WEIGHT_TOLERANCE: f64 = 1e-9;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowProfileReadiness {
    ReadyForAdapter,
    Evaluation,
    ContextOnly,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowProfileDirection {
    HigherIsBetter,
    LowerIsBetter,
    TargetRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowHorizon {
    Current,
    OneYear,
    ThreeYear,
    FiveYear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowProfileStatus {
    Observed,
    Modeled,
    Provisional,
    Blocked,
    NotApplicable,
}

impl WindowProfileStatus {
    fn is_score_eligible(self) -> bool {
        matches!(self, Self::Observed | Self::Modeled | Self::Provisional)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowFreshness {
    Current,
    Stale,
    Unknown,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowCohortKind {
    CurrentNhl,
    SeasonCanonical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowNormalizationMethod {
    EmpiricalPercentile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowMissingPolicy {
    WithholdRank,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowAggregateStatus {
    Complete,
    Provisional,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowRankState {
    Ranked,
    Withheld,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowClassification {
    Contender,
    RisingContender,
    FragileContender,
    Plateau,
    Retooling,
    Rebuilding,
    EvaluationIncomplete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowProfileInventoryCounts {
    pub total: usize,
    pub ready_for_adapter: usize,
    pub evaluation: usize,
    pub context_only: usize,
    pub blocked: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowProfileDescriptor {
    pub key: String,
    pub method_version: String,
    pub label: String,
    pub dimension: String,
    pub source_schema: String,
    pub readiness: WindowProfileReadiness,
    pub direction: WindowProfileDirection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_max: Option<f64>,
    pub raw_unit: String,
    pub signal_family: String,
    pub minimum_cohort: usize,
    #[serde(default)]
    pub dependencies: Vec<String>,
    pub scenario_support: bool,
    pub historical_support: bool,
    pub calibration_target: String,
    pub promotion_gaps: Vec<String>,
}

impl WindowProfileDescriptor {
    pub fn id(&self) -> String {
        format!("{}@{}", self.key, self.method_version)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationWindowProfileInventory {
    pub schema: String,
    pub generated_at: String,
    pub counts: WindowProfileInventoryCounts,
    pub profiles: Vec<WindowProfileDescriptor>,
}

impl OrganizationWindowProfileInventory {
    pub fn find(&self, key: &str, method_version: &str) -> Option<&WindowProfileDescriptor> {
        self.profiles
            .iter()
            .find(|row| row.key == key && row.method_version == method_version)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowEvidenceView {
    pub source_schema: String,
    pub source_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub captured_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub as_of: Option<NaiveDate>,
    pub freshness: WindowFreshness,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationProfileInput {
    pub profile_key: String,
    pub method_version: String,
    pub organization: String,
    pub organization_identity_version: String,
    pub season: u32,
    pub season_type: String,
    pub as_of: NaiveDate,
    pub horizon: WindowHorizon,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_value: Option<f64>,
    pub raw_unit: String,
    pub sample_size: u64,
    pub confidence: f64,
    pub coverage: f64,
    pub status: WindowProfileStatus,
    #[serde(default)]
    pub evidence: Vec<WindowEvidenceView>,
    #[serde(default)]
    pub limitations: Vec<String>,
    #[serde(default)]
    pub source_fingerprints: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationProfileObservationView {
    pub schema: String,
    pub profile_key: String,
    pub method_version: String,
    pub organization: String,
    pub organization_identity_version: String,
    pub season: u32,
    pub season_type: String,
    pub as_of: NaiveDate,
    pub horizon: WindowHorizon,
    pub signal_family: String,
    pub direction: WindowProfileDirection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_value: Option<f64>,
    pub raw_unit: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normalized_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub league_percentile: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub league_rank: Option<usize>,
    pub sample_size: u64,
    pub confidence: f64,
    pub coverage: f64,
    pub status: WindowProfileStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_comparable: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trend: Option<String>,
    pub evidence: Vec<WindowEvidenceView>,
    pub limitations: Vec<String>,
    pub source_fingerprints: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowCohortManifest {
    pub kind: WindowCohortKind,
    pub team_catalog_version: String,
    pub expected_organizations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowProfileWeight {
    pub profile_key: String,
    pub method_version: String,
    pub weight: f64,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowSignalFamilyCap {
    pub signal_family: String,
    pub maximum_weight: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowDimensionManifest {
    pub key: String,
    pub label: String,
    pub weight: f64,
    pub minimum_coverage: f64,
    pub rank_required: bool,
    pub profiles: Vec<WindowProfileWeight>,
    #[serde(default)]
    pub signal_family_caps: Vec<WindowSignalFamilyCap>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationWindowManifestView {
    pub schema: String,
    pub manifest_id: String,
    pub label: String,
    pub description: String,
    pub manifest_version: String,
    pub comparison_cohort: WindowCohortManifest,
    pub normalization_method: WindowNormalizationMethod,
    pub primary_horizon: WindowHorizon,
    pub dimensions: Vec<WindowDimensionManifest>,
    pub missing_policy: WindowMissingPolicy,
    pub classification_method: String,
    pub created_at: String,
    #[serde(default)]
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowRankStatusView {
    pub state: WindowRankState,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowDimensionView {
    pub key: String,
    pub label: String,
    pub weight: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    pub confidence: f64,
    pub coverage: f64,
    pub status: WindowAggregateStatus,
    pub profiles: Vec<OrganizationProfileObservationView>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowDriverView {
    pub dimension_key: String,
    pub label: String,
    pub score: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowOverallView {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    pub confidence: f64,
    pub coverage: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub percentile: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank: Option<usize>,
    pub rank_status: WindowRankStatusView,
    pub classification: WindowClassification,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowOrganizationView {
    pub organization: String,
    pub overall: WindowOverallView,
    pub dimensions: Vec<WindowDimensionView>,
    pub strengths: Vec<WindowDriverView>,
    pub vulnerabilities: Vec<WindowDriverView>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationWindowBoardView {
    pub schema: String,
    pub registry_version: String,
    pub season: u32,
    pub season_type: String,
    pub as_of: NaiveDate,
    pub generated_at: String,
    pub manifest: OrganizationWindowManifestView,
    pub source_fingerprints: Vec<String>,
    pub league_coverage: f64,
    pub expected_organizations: Vec<String>,
    pub organizations: Vec<WindowOrganizationView>,
    pub disclosures: Vec<String>,
    pub fingerprint: String,
}

impl OrganizationWindowBoardView {
    pub fn organization(&self, team: &str) -> Option<&WindowOrganizationView> {
        self.organizations
            .iter()
            .find(|row| row.organization == team)
    }

    pub fn calculate_fingerprint(&self) -> Result<String, OrganizationWindowError> {
        let mut canonical = self.clone();
        canonical.fingerprint.clear();
        canonical.source_fingerprints.sort();
        canonical.source_fingerprints.dedup();
        canonical.expected_organizations.sort();
        canonical
            .organizations
            .sort_by(|a, b| a.organization.cmp(&b.organization));
        for organization in &mut canonical.organizations {
            canonicalize_organization(organization);
        }
        // Seal the wire-normalized form. Derived floating-point values can be
        // shortened by JSON serialization; hashing only the pre-wire memory
        // representation would make a saved, unchanged board fail validation.
        let wire = serde_json::to_vec(&canonical)
            .map_err(|error| OrganizationWindowError::InvalidJson(error.to_string()))?;
        let normalized: Self = serde_json::from_slice(&wire)
            .map_err(|error| OrganizationWindowError::InvalidJson(error.to_string()))?;
        hash_json(&normalized)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationWindowBoardInput {
    pub season: u32,
    pub season_type: String,
    pub as_of: NaiveDate,
    pub generated_at: String,
    pub manifest: OrganizationWindowManifestView,
    pub profile_inputs: Vec<OrganizationProfileInput>,
    #[serde(default)]
    pub source_fingerprints: Vec<String>,
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum OrganizationWindowError {
    #[error("unsupported Window schema for {contract}: {found}")]
    UnsupportedSchema {
        contract: &'static str,
        found: String,
    },
    #[error("invalid Window inventory: {0}")]
    InvalidInventory(String),
    #[error("duplicate Window profile method: {0}")]
    DuplicateProfileMethod(String),
    #[error("unknown Window profile method: {0}")]
    UnknownProfileMethod(String),
    #[error("Window profile dependency is unknown: {0}")]
    UnknownDependency(String),
    #[error("Window profile dependency cycle includes: {0}")]
    DependencyCycle(String),
    #[error("invalid Window manifest: {0}")]
    InvalidManifest(String),
    #[error("Window manifest fingerprint mismatch")]
    ManifestFingerprintMismatch,
    #[error("Window board fingerprint mismatch")]
    BoardFingerprintMismatch,
    #[error("invalid Window board: {0}")]
    InvalidBoard(String),
    #[error("duplicate Window profile input: {0}")]
    DuplicateProfileInput(String),
    #[error("invalid Window profile input: {0}")]
    InvalidProfileInput(String),
    #[error("Window profile input context does not match the board: {0}")]
    ContextMismatch(String),
    #[error("Window cohort is invalid: {0}")]
    InvalidCohort(String),
    #[error("Window JSON is invalid: {0}")]
    InvalidJson(String),
}

pub fn load_organization_window_profile_inventory(
) -> Result<OrganizationWindowProfileInventory, OrganizationWindowError> {
    let inventory: OrganizationWindowProfileInventory =
        serde_json::from_str(ORGANIZATION_WINDOW_PROFILE_INVENTORY_JSON)
            .map_err(|error| OrganizationWindowError::InvalidJson(error.to_string()))?;
    validate_profile_inventory(&inventory)?;
    Ok(inventory)
}

pub fn validate_profile_inventory(
    inventory: &OrganizationWindowProfileInventory,
) -> Result<(), OrganizationWindowError> {
    if inventory.schema != ORGANIZATION_WINDOW_PROFILE_INVENTORY_SCHEMA {
        return Err(OrganizationWindowError::UnsupportedSchema {
            contract: "profile inventory",
            found: inventory.schema.clone(),
        });
    }
    if inventory.profiles.len() != inventory.counts.total {
        return Err(OrganizationWindowError::InvalidInventory(format!(
            "declared {} profiles but found {}",
            inventory.counts.total,
            inventory.profiles.len()
        )));
    }

    let mut ids = BTreeSet::new();
    let mut actual = BTreeMap::<WindowProfileReadiness, usize>::new();
    for profile in &inventory.profiles {
        validate_descriptor(profile)?;
        if !ids.insert(profile.id()) {
            return Err(OrganizationWindowError::DuplicateProfileMethod(
                profile.id(),
            ));
        }
        *actual.entry(profile.readiness).or_default() += 1;
    }
    let expected_counts = [
        (
            WindowProfileReadiness::ReadyForAdapter,
            inventory.counts.ready_for_adapter,
        ),
        (
            WindowProfileReadiness::Evaluation,
            inventory.counts.evaluation,
        ),
        (
            WindowProfileReadiness::ContextOnly,
            inventory.counts.context_only,
        ),
        (WindowProfileReadiness::Blocked, inventory.counts.blocked),
    ];
    for (readiness, expected) in expected_counts {
        let found = actual.get(&readiness).copied().unwrap_or_default();
        if found != expected {
            return Err(OrganizationWindowError::InvalidInventory(format!(
                "{readiness:?} count is {found}, expected {expected}"
            )));
        }
    }
    validate_dependency_graph(inventory, &ids)
}

fn validate_descriptor(profile: &WindowProfileDescriptor) -> Result<(), OrganizationWindowError> {
    for (field, value) in [
        ("key", profile.key.as_str()),
        ("method_version", profile.method_version.as_str()),
        ("label", profile.label.as_str()),
        ("dimension", profile.dimension.as_str()),
        ("source_schema", profile.source_schema.as_str()),
        ("raw_unit", profile.raw_unit.as_str()),
        ("signal_family", profile.signal_family.as_str()),
        ("calibration_target", profile.calibration_target.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(OrganizationWindowError::InvalidInventory(format!(
                "{} has empty {field}",
                profile.id()
            )));
        }
    }
    if profile.minimum_cohort == 0 && profile.readiness != WindowProfileReadiness::Blocked {
        return Err(OrganizationWindowError::InvalidInventory(format!(
            "{} has zero minimum cohort",
            profile.id()
        )));
    }
    if profile.direction == WindowProfileDirection::TargetRange {
        let (Some(minimum), Some(maximum)) = (profile.target_min, profile.target_max) else {
            return Err(OrganizationWindowError::InvalidInventory(format!(
                "{} target range is incomplete",
                profile.id()
            )));
        };
        if !minimum.is_finite() || !maximum.is_finite() || minimum > maximum {
            return Err(OrganizationWindowError::InvalidInventory(format!(
                "{} target range is invalid",
                profile.id()
            )));
        }
    }
    Ok(())
}

fn validate_dependency_graph(
    inventory: &OrganizationWindowProfileInventory,
    ids: &BTreeSet<String>,
) -> Result<(), OrganizationWindowError> {
    for profile in &inventory.profiles {
        for dependency in &profile.dependencies {
            if !ids.contains(dependency) {
                return Err(OrganizationWindowError::UnknownDependency(
                    dependency.clone(),
                ));
            }
        }
    }

    fn visit(
        id: &str,
        by_id: &BTreeMap<String, &WindowProfileDescriptor>,
        temporary: &mut BTreeSet<String>,
        permanent: &mut BTreeSet<String>,
    ) -> Result<(), OrganizationWindowError> {
        if permanent.contains(id) {
            return Ok(());
        }
        if !temporary.insert(id.to_owned()) {
            return Err(OrganizationWindowError::DependencyCycle(id.to_owned()));
        }
        if let Some(profile) = by_id.get(id) {
            for dependency in &profile.dependencies {
                visit(dependency, by_id, temporary, permanent)?;
            }
        }
        temporary.remove(id);
        permanent.insert(id.to_owned());
        Ok(())
    }

    let by_id = inventory
        .profiles
        .iter()
        .map(|profile| (profile.id(), profile))
        .collect::<BTreeMap<_, _>>();
    let mut temporary = BTreeSet::new();
    let mut permanent = BTreeSet::new();
    for id in by_id.keys() {
        visit(id, &by_id, &mut temporary, &mut permanent)?;
    }
    Ok(())
}

pub fn parse_organization_window_manifest(
    json: &str,
    inventory: &OrganizationWindowProfileInventory,
) -> Result<OrganizationWindowManifestView, OrganizationWindowError> {
    let manifest = serde_json::from_str(json)
        .map_err(|error| OrganizationWindowError::InvalidJson(error.to_string()))?;
    seal_organization_window_manifest(manifest, inventory)
}

pub fn seal_organization_window_manifest(
    mut manifest: OrganizationWindowManifestView,
    inventory: &OrganizationWindowProfileInventory,
) -> Result<OrganizationWindowManifestView, OrganizationWindowError> {
    validate_profile_inventory(inventory)?;
    if manifest.schema != ORGANIZATION_WINDOW_MANIFEST_SCHEMA {
        return Err(OrganizationWindowError::UnsupportedSchema {
            contract: "manifest",
            found: manifest.schema,
        });
    }
    validate_manifest_fields(&manifest, inventory)?;
    let supplied_fingerprint = manifest.fingerprint.clone();
    canonicalize_manifest(&mut manifest);
    manifest.fingerprint.clear();
    let calculated = hash_json(&manifest)?;
    if !supplied_fingerprint.is_empty() && supplied_fingerprint != calculated {
        return Err(OrganizationWindowError::ManifestFingerprintMismatch);
    }
    manifest.fingerprint = calculated;
    Ok(manifest)
}

fn validate_manifest_fields(
    manifest: &OrganizationWindowManifestView,
    inventory: &OrganizationWindowProfileInventory,
) -> Result<(), OrganizationWindowError> {
    for (field, value) in [
        ("manifest_id", manifest.manifest_id.as_str()),
        ("label", manifest.label.as_str()),
        ("manifest_version", manifest.manifest_version.as_str()),
        (
            "team_catalog_version",
            manifest.comparison_cohort.team_catalog_version.as_str(),
        ),
        (
            "classification_method",
            manifest.classification_method.as_str(),
        ),
        ("created_at", manifest.created_at.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(OrganizationWindowError::InvalidManifest(format!(
                "{field} is empty"
            )));
        }
    }
    validate_expected_organizations(&manifest.comparison_cohort)?;
    if manifest.dimensions.is_empty() {
        return Err(OrganizationWindowError::InvalidManifest(
            "manifest has no dimensions".to_owned(),
        ));
    }
    validate_weight_sum(
        "dimension",
        manifest.dimensions.iter().map(|dimension| dimension.weight),
    )?;
    let mut dimension_keys = BTreeSet::new();
    let mut profile_ids = BTreeSet::new();
    for dimension in &manifest.dimensions {
        if dimension.key.trim().is_empty() || dimension.label.trim().is_empty() {
            return Err(OrganizationWindowError::InvalidManifest(
                "dimension key/label is empty".to_owned(),
            ));
        }
        if !dimension_keys.insert(dimension.key.clone()) {
            return Err(OrganizationWindowError::InvalidManifest(format!(
                "duplicate dimension {}",
                dimension.key
            )));
        }
        validate_unit_interval("dimension minimum coverage", dimension.minimum_coverage)?;
        if dimension.profiles.is_empty() {
            return Err(OrganizationWindowError::InvalidManifest(format!(
                "dimension {} has no profiles",
                dimension.key
            )));
        }
        validate_weight_sum(
            "profile",
            dimension.profiles.iter().map(|profile| profile.weight),
        )?;
        let mut family_weights = BTreeMap::<String, f64>::new();
        for profile in &dimension.profiles {
            let id = format!("{}@{}", profile.profile_key, profile.method_version);
            if !profile_ids.insert(id.clone()) {
                return Err(OrganizationWindowError::InvalidManifest(format!(
                    "duplicate profile {id}"
                )));
            }
            validate_positive_weight("profile", profile.weight)?;
            let descriptor = inventory
                .find(&profile.profile_key, &profile.method_version)
                .ok_or_else(|| OrganizationWindowError::UnknownProfileMethod(id.clone()))?;
            if descriptor.dimension != dimension.key {
                return Err(OrganizationWindowError::InvalidManifest(format!(
                    "profile {id} belongs to {}, not {}",
                    descriptor.dimension, dimension.key
                )));
            }
            *family_weights
                .entry(descriptor.signal_family.clone())
                .or_default() += profile.weight;
        }
        let mut cap_families = BTreeSet::new();
        for cap in &dimension.signal_family_caps {
            if !cap_families.insert(cap.signal_family.clone()) {
                return Err(OrganizationWindowError::InvalidManifest(format!(
                    "duplicate signal-family cap {}",
                    cap.signal_family
                )));
            }
            validate_unit_interval("signal-family cap", cap.maximum_weight)?;
            let assigned = family_weights
                .get(&cap.signal_family)
                .copied()
                .unwrap_or_default();
            if assigned > cap.maximum_weight + WEIGHT_TOLERANCE {
                return Err(OrganizationWindowError::InvalidManifest(format!(
                    "signal family {} has weight {assigned}, above cap {}",
                    cap.signal_family, cap.maximum_weight
                )));
            }
        }
    }
    Ok(())
}

fn validate_expected_organizations(
    cohort: &WindowCohortManifest,
) -> Result<(), OrganizationWindowError> {
    if cohort.expected_organizations.is_empty() {
        return Err(OrganizationWindowError::InvalidCohort(
            "expected organization set is empty".to_owned(),
        ));
    }
    let expected = cohort
        .expected_organizations
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if expected.len() != cohort.expected_organizations.len() {
        return Err(OrganizationWindowError::InvalidCohort(
            "expected organization set has duplicates".to_owned(),
        ));
    }
    if cohort.kind == WindowCohortKind::CurrentNhl {
        let canonical = CANONICAL_TEAMS
            .iter()
            .map(|(abbr, _)| (*abbr).to_owned())
            .collect::<BTreeSet<_>>();
        if expected != canonical {
            return Err(OrganizationWindowError::InvalidCohort(
                "current NHL cohort must match all 32 canonical teams".to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_weight_sum(
    label: &str,
    values: impl Iterator<Item = f64>,
) -> Result<(), OrganizationWindowError> {
    let values = values.collect::<Vec<_>>();
    for value in &values {
        validate_positive_weight(label, *value)?;
    }
    let sum = values.iter().sum::<f64>();
    if (sum - 1.0).abs() > WEIGHT_TOLERANCE {
        return Err(OrganizationWindowError::InvalidManifest(format!(
            "{label} weights sum to {sum}, expected 1"
        )));
    }
    Ok(())
}

fn validate_positive_weight(label: &str, value: f64) -> Result<(), OrganizationWindowError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(OrganizationWindowError::InvalidManifest(format!(
            "{label} weight must be finite and positive; got {value}"
        )));
    }
    Ok(())
}

fn validate_unit_interval(label: &str, value: f64) -> Result<(), OrganizationWindowError> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(OrganizationWindowError::InvalidManifest(format!(
            "{label} must be in 0..=1; got {value}"
        )));
    }
    Ok(())
}

fn canonicalize_manifest(manifest: &mut OrganizationWindowManifestView) {
    manifest.comparison_cohort.expected_organizations.sort();
    manifest.comparison_cohort.expected_organizations.dedup();
    manifest.dimensions.sort_by(|a, b| a.key.cmp(&b.key));
    for dimension in &mut manifest.dimensions {
        dimension.weight = canonical_zero(dimension.weight);
        dimension.minimum_coverage = canonical_zero(dimension.minimum_coverage);
        dimension.profiles.sort_by(|a, b| {
            (&a.profile_key, &a.method_version).cmp(&(&b.profile_key, &b.method_version))
        });
        for profile in &mut dimension.profiles {
            profile.weight = canonical_zero(profile.weight);
        }
        dimension
            .signal_family_caps
            .sort_by(|a, b| a.signal_family.cmp(&b.signal_family));
        for cap in &mut dimension.signal_family_caps {
            cap.maximum_weight = canonical_zero(cap.maximum_weight);
        }
    }
}

pub fn build_organization_window_board(
    input: OrganizationWindowBoardInput,
    inventory: &OrganizationWindowProfileInventory,
) -> Result<OrganizationWindowBoardView, OrganizationWindowError> {
    let manifest = seal_organization_window_manifest(input.manifest.clone(), inventory)?;
    let expected = manifest
        .comparison_cohort
        .expected_organizations
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let normalized = normalize_profile_inputs(&input, &manifest, inventory, &expected)?;

    let mut organizations = Vec::with_capacity(expected.len());
    for organization in &expected {
        organizations.push(build_organization_row(
            organization,
            &manifest,
            &normalized,
        )?);
    }

    let mut cohort_rank_reasons = Vec::new();
    for dimension in &manifest.dimensions {
        for profile in &dimension.profiles {
            let available = expected
                .iter()
                .filter(|organization| {
                    normalized
                        .get(&(
                            (*organization).clone(),
                            profile.profile_key.clone(),
                            profile.method_version.clone(),
                        ))
                        .is_some_and(|observation| observation.normalized_score.is_some())
                })
                .count();
            if available > 0 && available < expected.len() {
                cohort_rank_reasons.push(format!(
                    "profile {}@{} is available for {available} of {} organizations",
                    profile.profile_key,
                    profile.method_version,
                    expected.len()
                ));
            }
        }
    }
    if !cohort_rank_reasons.is_empty() {
        for organization in &mut organizations {
            organization
                .overall
                .rank_status
                .reasons
                .extend(cohort_rank_reasons.iter().cloned());
            organization.overall.rank_status.reasons.sort();
            organization.overall.rank_status.reasons.dedup();
            organization.overall.rank_status.state = WindowRankState::Withheld;
        }
    }

    let board_rankable = organizations.iter().all(|organization| {
        organization.overall.score.is_some() && organization.overall.rank_status.reasons.is_empty()
    });
    if board_rankable {
        assign_overall_ranks(&mut organizations);
    } else {
        for organization in &mut organizations {
            organization.overall.rank_status.state = WindowRankState::Withheld;
            if organization.overall.rank_status.reasons.is_empty() {
                organization
                    .overall
                    .rank_status
                    .reasons
                    .push("another organization failed a comparable rank gate".to_owned());
            }
        }
    }

    let mut source_fingerprints = input.source_fingerprints;
    source_fingerprints.sort();
    source_fingerprints.dedup();
    let mut board = OrganizationWindowBoardView {
        schema: ORGANIZATION_WINDOW_BOARD_SCHEMA.to_owned(),
        registry_version: ORGANIZATION_WINDOW_REGISTRY_VERSION.to_owned(),
        season: input.season,
        season_type: input.season_type,
        as_of: input.as_of,
        generated_at: input.generated_at,
        manifest,
        source_fingerprints,
        league_coverage: organizations.len() as f64 / expected.len() as f64,
        expected_organizations: expected.into_iter().collect(),
        organizations,
        disclosures: vec![
            "The Window separates score, confidence, and coverage; missing evidence is never zero-filled.".to_owned(),
            "League rank is withheld for the complete board when any organization fails a required comparability gate.".to_owned(),
            "Classification is a versioned descriptive heuristic, not a Cup probability.".to_owned(),
        ],
        fingerprint: String::new(),
    };
    board.fingerprint = board.calculate_fingerprint()?;
    Ok(board)
}

/// Validate a loaded/saved board before any renderer or comparison trusts it.
/// Builders already enforce these invariants; this is the wire-boundary gate
/// for external or embedded artifacts.
pub fn validate_organization_window_board(
    board: &OrganizationWindowBoardView,
    inventory: &OrganizationWindowProfileInventory,
) -> Result<(), OrganizationWindowError> {
    if board.schema != ORGANIZATION_WINDOW_BOARD_SCHEMA {
        return Err(OrganizationWindowError::UnsupportedSchema {
            contract: "board",
            found: board.schema.clone(),
        });
    }
    let sealed_manifest = seal_organization_window_manifest(board.manifest.clone(), inventory)?;
    if sealed_manifest.fingerprint != board.manifest.fingerprint {
        return Err(OrganizationWindowError::ManifestFingerprintMismatch);
    }
    if board.calculate_fingerprint()? != board.fingerprint {
        return Err(OrganizationWindowError::BoardFingerprintMismatch);
    }
    let expected = board
        .expected_organizations
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let manifest_expected = board
        .manifest
        .comparison_cohort
        .expected_organizations
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if expected.is_empty()
        || expected.len() != board.expected_organizations.len()
        || expected != manifest_expected
    {
        return Err(OrganizationWindowError::InvalidBoard(
            "expected organization cohort differs from the manifest or contains duplicates"
                .to_owned(),
        ));
    }
    let organizations = board
        .organizations
        .iter()
        .map(|organization| organization.organization.clone())
        .collect::<BTreeSet<_>>();
    if organizations != expected || organizations.len() != board.organizations.len() {
        return Err(OrganizationWindowError::InvalidBoard(
            "organization rows do not exactly match the expected cohort".to_owned(),
        ));
    }
    let expected_coverage = board.organizations.len() as f64 / expected.len() as f64;
    if !board.league_coverage.is_finite()
        || (board.league_coverage - expected_coverage).abs() > WEIGHT_TOLERANCE
    {
        return Err(OrganizationWindowError::InvalidBoard(
            "league coverage does not reconcile to the cohort".to_owned(),
        ));
    }
    for organization in &board.organizations {
        validate_loaded_organization(board, organization, inventory)?;
    }
    let ranked = board
        .organizations
        .iter()
        .filter(|organization| organization.overall.rank.is_some())
        .count();
    if ranked != 0 && ranked != board.organizations.len() {
        return Err(OrganizationWindowError::InvalidBoard(
            "league rank is only partially populated".to_owned(),
        ));
    }
    if ranked == board.organizations.len() {
        let mut expected_ranks = board.organizations.clone();
        for organization in &mut expected_ranks {
            organization.overall.rank = None;
            organization.overall.percentile = None;
        }
        assign_overall_ranks(&mut expected_ranks);
        let expected_by_team = expected_ranks
            .iter()
            .map(|organization| (organization.organization.as_str(), &organization.overall))
            .collect::<BTreeMap<_, _>>();
        if board.organizations.iter().any(|organization| {
            let expected = expected_by_team[organization.organization.as_str()];
            organization.overall.rank != expected.rank
                || organization.overall.percentile != expected.percentile
                || organization.overall.rank_status.state != WindowRankState::Ranked
        }) {
            return Err(OrganizationWindowError::InvalidBoard(
                "league ranks do not reconcile to sealed scores".to_owned(),
            ));
        }
    } else if board
        .organizations
        .iter()
        .any(|organization| organization.overall.rank_status.state != WindowRankState::Withheld)
    {
        return Err(OrganizationWindowError::InvalidBoard(
            "unranked board contains a ranked status".to_owned(),
        ));
    }
    let rebuilt = rebuild_loaded_board(board, inventory)?;
    for organization in &board.organizations {
        let Some(expected) = rebuilt.organization(&organization.organization) else {
            return Err(OrganizationWindowError::InvalidBoard(format!(
                "rebuilt board omitted {}",
                organization.organization
            )));
        };
        if !same_derived_organization(organization, expected) {
            return Err(OrganizationWindowError::InvalidBoard(format!(
                "{} stored normalized, aggregate, classification, driver, or rank values do not reconcile to raw profile observations",
                organization.organization
            )));
        }
    }
    Ok(())
}

fn rebuild_loaded_board(
    board: &OrganizationWindowBoardView,
    inventory: &OrganizationWindowProfileInventory,
) -> Result<OrganizationWindowBoardView, OrganizationWindowError> {
    let profile_inputs = board
        .organizations
        .iter()
        .flat_map(|organization| {
            organization
                .dimensions
                .iter()
                .flat_map(|dimension| dimension.profiles.iter())
                .map(|profile| OrganizationProfileInput {
                    profile_key: profile.profile_key.clone(),
                    method_version: profile.method_version.clone(),
                    organization: profile.organization.clone(),
                    organization_identity_version: profile.organization_identity_version.clone(),
                    season: profile.season,
                    season_type: profile.season_type.clone(),
                    as_of: profile.as_of,
                    horizon: profile.horizon,
                    raw_value: profile.raw_value,
                    raw_unit: profile.raw_unit.clone(),
                    sample_size: profile.sample_size,
                    confidence: profile.confidence,
                    coverage: profile.coverage,
                    status: profile.status,
                    evidence: profile.evidence.clone(),
                    limitations: profile.limitations.clone(),
                    source_fingerprints: profile.source_fingerprints.clone(),
                })
        })
        .collect();
    build_organization_window_board(
        OrganizationWindowBoardInput {
            season: board.season,
            season_type: board.season_type.clone(),
            as_of: board.as_of,
            generated_at: board.generated_at.clone(),
            manifest: board.manifest.clone(),
            profile_inputs,
            source_fingerprints: board.source_fingerprints.clone(),
        },
        inventory,
    )
}

fn same_derived_organization(
    stored: &WindowOrganizationView,
    expected: &WindowOrganizationView,
) -> bool {
    if !same_derived_overall(&stored.overall, &expected.overall)
        || !same_drivers(&stored.strengths, &expected.strengths)
        || !same_drivers(&stored.vulnerabilities, &expected.vulnerabilities)
        || stored.blockers != expected.blockers
        || stored.dimensions.len() != expected.dimensions.len()
    {
        return false;
    }
    let expected_dimensions = expected
        .dimensions
        .iter()
        .map(|dimension| (dimension.key.as_str(), dimension))
        .collect::<BTreeMap<_, _>>();
    stored.dimensions.iter().all(|dimension| {
        expected_dimensions
            .get(dimension.key.as_str())
            .is_some_and(|expected| {
                dimension.label == expected.label
                    && close_number(dimension.weight, expected.weight)
                    && close_optional(dimension.score, expected.score)
                    && close_number(dimension.confidence, expected.confidence)
                    && close_number(dimension.coverage, expected.coverage)
                    && dimension.status == expected.status
                    && dimension.blockers == expected.blockers
                    && dimension.profiles.len() == expected.profiles.len()
                    && dimension.profiles.iter().all(|profile| {
                        expected.profiles.iter().any(|candidate| {
                            candidate.profile_key == profile.profile_key
                                && candidate.method_version == profile.method_version
                                && close_optional(
                                    candidate.normalized_score,
                                    profile.normalized_score,
                                )
                                && close_optional(
                                    candidate.league_percentile,
                                    profile.league_percentile,
                                )
                                && candidate.league_rank == profile.league_rank
                        })
                    })
            })
    })
}

fn same_derived_overall(stored: &WindowOverallView, expected: &WindowOverallView) -> bool {
    close_optional(stored.score, expected.score)
        && close_number(stored.confidence, expected.confidence)
        && close_number(stored.coverage, expected.coverage)
        && close_optional(stored.percentile, expected.percentile)
        && stored.rank == expected.rank
        && stored.rank_status == expected.rank_status
        && stored.classification == expected.classification
}

fn same_drivers(stored: &[WindowDriverView], expected: &[WindowDriverView]) -> bool {
    stored.len() == expected.len()
        && stored.iter().zip(expected).all(|(stored, expected)| {
            stored.dimension_key == expected.dimension_key
                && stored.label == expected.label
                && close_number(stored.score, expected.score)
        })
}

fn close_optional(left: Option<f64>, right: Option<f64>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => close_number(left, right),
        (None, None) => true,
        _ => false,
    }
}

fn close_number(left: f64, right: f64) -> bool {
    left.is_finite() && right.is_finite() && (left - right).abs() <= WEIGHT_TOLERANCE
}

fn validate_loaded_organization(
    board: &OrganizationWindowBoardView,
    organization: &WindowOrganizationView,
    inventory: &OrganizationWindowProfileInventory,
) -> Result<(), OrganizationWindowError> {
    validate_optional_score("overall score", organization.overall.score)?;
    validate_optional_score("overall percentile", organization.overall.percentile)?;
    validate_unit_interval("overall confidence", organization.overall.confidence)?;
    validate_unit_interval("overall coverage", organization.overall.coverage)?;
    if organization
        .overall
        .rank
        .is_some_and(|rank| rank == 0 || rank > board.organizations.len())
    {
        return Err(OrganizationWindowError::InvalidBoard(format!(
            "{} has an invalid rank",
            organization.organization
        )));
    }
    let configured_dimensions = board
        .manifest
        .dimensions
        .iter()
        .map(|dimension| (dimension.key.as_str(), dimension))
        .collect::<BTreeMap<_, _>>();
    if organization.dimensions.len() != configured_dimensions.len() {
        return Err(OrganizationWindowError::InvalidBoard(format!(
            "{} has the wrong dimension count",
            organization.organization
        )));
    }
    let mut dimensions = BTreeSet::new();
    for dimension in &organization.dimensions {
        let configured = configured_dimensions
            .get(dimension.key.as_str())
            .ok_or_else(|| {
                OrganizationWindowError::InvalidBoard(format!(
                    "{} has unknown dimension {}",
                    organization.organization, dimension.key
                ))
            })?;
        if !dimensions.insert(dimension.key.as_str())
            || dimension.label != configured.label
            || (dimension.weight - configured.weight).abs() > WEIGHT_TOLERANCE
        {
            return Err(OrganizationWindowError::InvalidBoard(format!(
                "{} dimension {} differs from the manifest",
                organization.organization, dimension.key
            )));
        }
        validate_optional_score("dimension score", dimension.score)?;
        validate_unit_interval("dimension confidence", dimension.confidence)?;
        validate_unit_interval("dimension coverage", dimension.coverage)?;
        let configured_profiles = configured
            .profiles
            .iter()
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
        if dimension.profiles.len() != configured_profiles.len() {
            return Err(OrganizationWindowError::InvalidBoard(format!(
                "{} dimension {} has the wrong profile count",
                organization.organization, dimension.key
            )));
        }
        let mut profiles = BTreeSet::new();
        for profile in &dimension.profiles {
            let id = (
                profile.profile_key.as_str(),
                profile.method_version.as_str(),
            );
            if !profiles.insert(id) || !configured_profiles.contains_key(&id) {
                return Err(OrganizationWindowError::InvalidBoard(format!(
                    "{} has duplicate or unknown profile {}@{}",
                    organization.organization, id.0, id.1
                )));
            }
            let descriptor = inventory.find(id.0, id.1).ok_or_else(|| {
                OrganizationWindowError::UnknownProfileMethod(format!("{}@{}", id.0, id.1))
            })?;
            if profile.schema != ORGANIZATION_PROFILE_OBSERVATION_SCHEMA
                || profile.organization != organization.organization
                || profile.organization_identity_version
                    != board.manifest.comparison_cohort.team_catalog_version
                || profile.season != board.season
                || profile.season_type != board.season_type
                || profile.as_of != board.as_of
                || profile.horizon != board.manifest.primary_horizon
                || profile.signal_family != descriptor.signal_family
                || profile.direction != descriptor.direction
                || profile.raw_unit != descriptor.raw_unit
            {
                return Err(OrganizationWindowError::InvalidBoard(format!(
                    "{} profile {}@{} has mismatched context or method metadata",
                    organization.organization, id.0, id.1
                )));
            }
            if profile.raw_value.is_some_and(|value| !value.is_finite()) {
                return Err(OrganizationWindowError::InvalidBoard(format!(
                    "{} profile {}@{} has a non-finite raw value",
                    organization.organization, id.0, id.1
                )));
            }
            validate_optional_score("profile normalized score", profile.normalized_score)?;
            validate_optional_score("profile percentile", profile.league_percentile)?;
            validate_unit_interval("profile confidence", profile.confidence)?;
            validate_unit_interval("profile coverage", profile.coverage)?;
            if profile.normalized_score.is_some() && profile.source_fingerprints.is_empty() {
                return Err(OrganizationWindowError::InvalidBoard(format!(
                    "{} profile {}@{} is scored without a source fingerprint",
                    organization.organization, id.0, id.1
                )));
            }
        }
    }
    Ok(())
}

fn validate_optional_score(label: &str, value: Option<f64>) -> Result<(), OrganizationWindowError> {
    if value.is_some_and(|value| !value.is_finite() || !(0.0..=100.0).contains(&value)) {
        return Err(OrganizationWindowError::InvalidBoard(format!(
            "{label} is outside 0..=100"
        )));
    }
    Ok(())
}

fn normalize_profile_inputs(
    input: &OrganizationWindowBoardInput,
    manifest: &OrganizationWindowManifestView,
    inventory: &OrganizationWindowProfileInventory,
    expected: &BTreeSet<String>,
) -> Result<
    BTreeMap<(String, String, String), OrganizationProfileObservationView>,
    OrganizationWindowError,
> {
    let mut supplied = BTreeMap::<(String, String, String), OrganizationProfileInput>::new();
    for mut profile in input.profile_inputs.clone() {
        validate_profile_input(&profile, input, manifest, inventory, expected)?;
        profile.raw_value = profile.raw_value.map(canonical_zero);
        profile.source_fingerprints.sort();
        profile.source_fingerprints.dedup();
        let key = (
            profile.organization.clone(),
            profile.profile_key.clone(),
            profile.method_version.clone(),
        );
        if supplied.insert(key.clone(), profile).is_some() {
            return Err(OrganizationWindowError::DuplicateProfileInput(format!(
                "{}:{}@{}",
                key.0, key.1, key.2
            )));
        }
    }

    let mut result = BTreeMap::new();
    for dimension in &manifest.dimensions {
        for configured in &dimension.profiles {
            let descriptor = inventory
                .find(&configured.profile_key, &configured.method_version)
                .ok_or_else(|| {
                    OrganizationWindowError::UnknownProfileMethod(format!(
                        "{}@{}",
                        configured.profile_key, configured.method_version
                    ))
                })?;
            let mut rows = expected
                .iter()
                .map(|organization| {
                    let key = (
                        organization.clone(),
                        configured.profile_key.clone(),
                        configured.method_version.clone(),
                    );
                    supplied
                        .get(&key)
                        .cloned()
                        .unwrap_or_else(|| missing_profile_input(input, descriptor, organization))
                })
                .collect::<Vec<_>>();
            normalize_profile_cohort(descriptor, &mut rows, &mut result)?;
        }
    }
    Ok(result)
}

fn validate_profile_input(
    profile: &OrganizationProfileInput,
    board: &OrganizationWindowBoardInput,
    manifest: &OrganizationWindowManifestView,
    inventory: &OrganizationWindowProfileInventory,
    expected: &BTreeSet<String>,
) -> Result<(), OrganizationWindowError> {
    let id = format!("{}@{}", profile.profile_key, profile.method_version);
    let descriptor = inventory
        .find(&profile.profile_key, &profile.method_version)
        .ok_or_else(|| OrganizationWindowError::UnknownProfileMethod(id.clone()))?;
    if !expected.contains(&profile.organization) {
        return Err(OrganizationWindowError::InvalidProfileInput(format!(
            "{} is outside the expected cohort",
            profile.organization
        )));
    }
    if profile.season != board.season
        || profile.season_type != board.season_type
        || profile.as_of != board.as_of
        || profile.horizon != manifest.primary_horizon
    {
        return Err(OrganizationWindowError::ContextMismatch(format!(
            "{} for {}",
            id, profile.organization
        )));
    }
    if profile.organization_identity_version != manifest.comparison_cohort.team_catalog_version {
        return Err(OrganizationWindowError::ContextMismatch(format!(
            "organization catalog for {}",
            profile.organization
        )));
    }
    if profile.raw_unit != descriptor.raw_unit {
        return Err(OrganizationWindowError::InvalidProfileInput(format!(
            "{id} raw unit is {}, expected {}",
            profile.raw_unit, descriptor.raw_unit
        )));
    }
    validate_unit_interval("profile confidence", profile.confidence)
        .map_err(|error| OrganizationWindowError::InvalidProfileInput(error.to_string()))?;
    validate_unit_interval("profile coverage", profile.coverage)
        .map_err(|error| OrganizationWindowError::InvalidProfileInput(error.to_string()))?;
    if let Some(value) = profile.raw_value {
        if !value.is_finite() {
            return Err(OrganizationWindowError::InvalidProfileInput(format!(
                "{id} has non-finite raw value"
            )));
        }
    } else if profile.status.is_score_eligible() {
        return Err(OrganizationWindowError::InvalidProfileInput(format!(
            "{id} is score-eligible without a raw value"
        )));
    }
    if descriptor.readiness == WindowProfileReadiness::Blocked
        && profile.status != WindowProfileStatus::Blocked
    {
        return Err(OrganizationWindowError::InvalidProfileInput(format!(
            "blocked descriptor {id} cannot receive a scored observation"
        )));
    }
    Ok(())
}

fn missing_profile_input(
    board: &OrganizationWindowBoardInput,
    descriptor: &WindowProfileDescriptor,
    organization: &str,
) -> OrganizationProfileInput {
    OrganizationProfileInput {
        profile_key: descriptor.key.clone(),
        method_version: descriptor.method_version.clone(),
        organization: organization.to_owned(),
        organization_identity_version: board
            .manifest
            .comparison_cohort
            .team_catalog_version
            .clone(),
        season: board.season,
        season_type: board.season_type.clone(),
        as_of: board.as_of,
        horizon: board.manifest.primary_horizon,
        raw_value: None,
        raw_unit: descriptor.raw_unit.clone(),
        sample_size: 0,
        confidence: 0.0,
        coverage: 0.0,
        status: WindowProfileStatus::Blocked,
        evidence: Vec::new(),
        limitations: vec!["profile input is missing for this organization".to_owned()],
        source_fingerprints: Vec::new(),
    }
}

fn normalize_profile_cohort(
    descriptor: &WindowProfileDescriptor,
    rows: &mut [OrganizationProfileInput],
    output: &mut BTreeMap<(String, String, String), OrganizationProfileObservationView>,
) -> Result<(), OrganizationWindowError> {
    let mut eligible = rows
        .iter()
        .enumerate()
        .filter_map(|(index, row)| {
            if row.status.is_score_eligible() {
                row.raw_value
                    .map(|value| (index, profile_utility(descriptor, value)))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    if eligible.iter().any(|(_, value)| !value.is_finite()) {
        return Err(OrganizationWindowError::InvalidProfileInput(format!(
            "{} produced a non-finite normalization value",
            descriptor.id()
        )));
    }

    let minimum_met = eligible.len() >= descriptor.minimum_cohort;
    eligible.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
    let zero_variance = minimum_met
        && eligible
            .first()
            .zip(eligible.last())
            .is_some_and(|(first, last)| first.1 == last.1);
    let mut normalized = BTreeMap::<usize, (f64, usize)>::new();
    if minimum_met && zero_variance {
        for (index, _) in &eligible {
            normalized.insert(*index, (50.0, 1));
        }
    } else if minimum_met {
        let count = eligible.len();
        let mut start = 0;
        while start < count {
            let mut end = start + 1;
            while end < count && eligible[end].1 == eligible[start].1 {
                end += 1;
            }
            let average_rank = ((start + 1 + end) as f64) / 2.0;
            let percentile = if count == 1 {
                50.0
            } else {
                100.0 * (count as f64 - average_rank) / (count as f64 - 1.0)
            };
            for (index, _) in &eligible[start..end] {
                normalized.insert(*index, (canonical_zero(percentile), start + 1));
            }
            start = end;
        }
    }

    for (index, row) in rows.iter().enumerate() {
        let mut limitations = row.limitations.clone();
        if !minimum_met {
            limitations.push(format!(
                "profile cohort has {} eligible organizations; {} required",
                eligible.len(),
                descriptor.minimum_cohort
            ));
        } else if zero_variance {
            limitations.push("no_between_team_variance".to_owned());
        }
        let normalized_row = normalized.get(&index).copied();
        let observation = OrganizationProfileObservationView {
            schema: ORGANIZATION_PROFILE_OBSERVATION_SCHEMA.to_owned(),
            profile_key: row.profile_key.clone(),
            method_version: row.method_version.clone(),
            organization: row.organization.clone(),
            organization_identity_version: row.organization_identity_version.clone(),
            season: row.season,
            season_type: row.season_type.clone(),
            as_of: row.as_of,
            horizon: row.horizon,
            signal_family: descriptor.signal_family.clone(),
            direction: descriptor.direction,
            raw_value: row.raw_value,
            raw_unit: row.raw_unit.clone(),
            normalized_score: normalized_row.map(|(score, _)| score),
            league_percentile: normalized_row.map(|(score, _)| score),
            league_rank: normalized_row.map(|(_, rank)| rank),
            sample_size: row.sample_size,
            confidence: canonical_zero(row.confidence),
            coverage: canonical_zero(row.coverage),
            status: row.status,
            previous_comparable: None,
            delta: None,
            trend: None,
            evidence: row.evidence.clone(),
            limitations,
            source_fingerprints: row.source_fingerprints.clone(),
        };
        output.insert(
            (
                row.organization.clone(),
                row.profile_key.clone(),
                row.method_version.clone(),
            ),
            observation,
        );
    }
    Ok(())
}

fn profile_utility(descriptor: &WindowProfileDescriptor, value: f64) -> f64 {
    match descriptor.direction {
        WindowProfileDirection::HigherIsBetter => value,
        WindowProfileDirection::LowerIsBetter => -value,
        WindowProfileDirection::TargetRange => {
            let minimum = descriptor.target_min.unwrap_or(value);
            let maximum = descriptor.target_max.unwrap_or(value);
            if value < minimum {
                value - minimum
            } else if value > maximum {
                maximum - value
            } else {
                0.0
            }
        }
    }
}

fn build_organization_row(
    organization: &str,
    manifest: &OrganizationWindowManifestView,
    observations: &BTreeMap<(String, String, String), OrganizationProfileObservationView>,
) -> Result<WindowOrganizationView, OrganizationWindowError> {
    let mut dimensions = Vec::with_capacity(manifest.dimensions.len());
    let mut rank_reasons = Vec::new();
    for configured in &manifest.dimensions {
        let mut profiles = Vec::with_capacity(configured.profiles.len());
        let mut score_numerator = 0.0;
        let mut confidence_numerator = 0.0;
        let mut eligible_weight = 0.0;
        let mut evidence_coverage = 0.0;
        let mut blockers = Vec::new();
        for profile in &configured.profiles {
            let key = (
                organization.to_owned(),
                profile.profile_key.clone(),
                profile.method_version.clone(),
            );
            let observation = observations.get(&key).ok_or_else(|| {
                OrganizationWindowError::InvalidProfileInput(format!(
                    "normalized observation is missing for {}:{}@{}",
                    organization, profile.profile_key, profile.method_version
                ))
            })?;
            if let Some(score) = observation.normalized_score {
                score_numerator += score * profile.weight;
                confidence_numerator += observation.confidence * profile.weight;
                eligible_weight += profile.weight;
                evidence_coverage += observation.coverage * profile.weight;
            } else if profile.required {
                blockers.push(format!(
                    "required profile {}@{} is unavailable",
                    profile.profile_key, profile.method_version
                ));
            }
            profiles.push(observation.clone());
        }
        let score =
            (eligible_weight > 0.0).then(|| canonical_zero(score_numerator / eligible_weight));
        let coverage = canonical_zero(evidence_coverage.clamp(0.0, 1.0));
        let confidence = if eligible_weight > 0.0 {
            canonical_zero((confidence_numerator / eligible_weight) * coverage)
        } else {
            0.0
        };
        let status = if score.is_none() {
            WindowAggregateStatus::Blocked
        } else if blockers.is_empty() && coverage + WEIGHT_TOLERANCE >= configured.minimum_coverage
        {
            WindowAggregateStatus::Complete
        } else {
            WindowAggregateStatus::Provisional
        };
        if configured.rank_required && status != WindowAggregateStatus::Complete {
            rank_reasons.push(format!(
                "rank-required dimension {} is {:?}",
                configured.key, status
            ));
        }
        dimensions.push(WindowDimensionView {
            key: configured.key.clone(),
            label: configured.label.clone(),
            weight: configured.weight,
            score,
            confidence,
            coverage,
            status,
            profiles,
            blockers,
        });
    }

    let mut score_numerator = 0.0;
    let mut eligible_weight = 0.0;
    let mut confidence = 0.0;
    let mut coverage = 0.0;
    for dimension in &dimensions {
        if let Some(score) = dimension.score {
            score_numerator += score * dimension.weight;
            eligible_weight += dimension.weight;
        }
        confidence += dimension.confidence * dimension.weight;
        coverage += dimension.coverage * dimension.weight;
    }
    let score = (eligible_weight > 0.0).then(|| canonical_zero(score_numerator / eligible_weight));
    if score.is_none() {
        rank_reasons.push("organization has no score-eligible dimensions".to_owned());
    }
    let classification = classify_window(score, &dimensions);
    let mut ranked_dimensions = dimensions
        .iter()
        .filter_map(|dimension| {
            dimension.score.map(|score| WindowDriverView {
                dimension_key: dimension.key.clone(),
                label: dimension.label.clone(),
                score,
            })
        })
        .collect::<Vec<_>>();
    ranked_dimensions.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
    let strengths = ranked_dimensions.iter().take(3).cloned().collect();
    let vulnerabilities = ranked_dimensions.iter().rev().take(3).cloned().collect();
    let blockers = dimensions
        .iter()
        .flat_map(|dimension| dimension.blockers.iter().cloned())
        .collect();

    Ok(WindowOrganizationView {
        organization: organization.to_owned(),
        overall: WindowOverallView {
            score,
            confidence: canonical_zero(confidence.clamp(0.0, 1.0)),
            coverage: canonical_zero(coverage.clamp(0.0, 1.0)),
            percentile: None,
            rank: None,
            rank_status: WindowRankStatusView {
                state: if rank_reasons.is_empty() {
                    WindowRankState::Ranked
                } else {
                    WindowRankState::Withheld
                },
                reasons: rank_reasons,
            },
            classification,
        },
        dimensions,
        strengths,
        vulnerabilities,
        blockers,
    })
}

fn classify_window(
    overall_score: Option<f64>,
    dimensions: &[WindowDimensionView],
) -> WindowClassification {
    let Some(overall) = overall_score else {
        return WindowClassification::EvaluationIncomplete;
    };
    let current = dimensions
        .iter()
        .find(|dimension| dimension.key == "nhl_strength")
        .and_then(|dimension| dimension.score)
        .unwrap_or(overall);
    let sustainable = dimensions
        .iter()
        .find(|dimension| dimension.key == "sustainability")
        .and_then(|dimension| dimension.score)
        .or_else(|| {
            dimensions
                .iter()
                .find(|dimension| dimension.key == "pipeline")
                .and_then(|dimension| dimension.score)
        })
        .unwrap_or(overall);
    if current >= 75.0 && sustainable >= 60.0 {
        WindowClassification::Contender
    } else if current >= 75.0 {
        WindowClassification::FragileContender
    } else if current >= 60.0 && sustainable >= 65.0 {
        WindowClassification::RisingContender
    } else if current >= 45.0 {
        WindowClassification::Plateau
    } else if sustainable >= 55.0 {
        WindowClassification::Retooling
    } else {
        WindowClassification::Rebuilding
    }
}

fn assign_overall_ranks(organizations: &mut [WindowOrganizationView]) {
    let mut order = organizations
        .iter()
        .enumerate()
        .filter_map(|(index, row)| row.overall.score.map(|score| (index, score)))
        .collect::<Vec<_>>();
    order.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
    let count = order.len();
    let mut start = 0;
    while start < count {
        let mut end = start + 1;
        while end < count && order[end].1 == order[start].1 {
            end += 1;
        }
        let average_rank = ((start + 1 + end) as f64) / 2.0;
        let percentile = if count == 1 {
            50.0
        } else {
            100.0 * (count as f64 - average_rank) / (count as f64 - 1.0)
        };
        for (index, _) in &order[start..end] {
            let overall = &mut organizations[*index].overall;
            overall.rank = Some(start + 1);
            overall.percentile = Some(canonical_zero(percentile));
            overall.rank_status.state = WindowRankState::Ranked;
            overall.rank_status.reasons.clear();
        }
        start = end;
    }
}

fn canonicalize_organization(organization: &mut WindowOrganizationView) {
    organization.dimensions.sort_by(|a, b| a.key.cmp(&b.key));
    for dimension in &mut organization.dimensions {
        dimension.profiles.sort_by(|a, b| {
            (&a.profile_key, &a.method_version).cmp(&(&b.profile_key, &b.method_version))
        });
        dimension.blockers.sort();
    }
    organization
        .strengths
        .sort_by(|a, b| a.dimension_key.cmp(&b.dimension_key));
    organization
        .vulnerabilities
        .sort_by(|a, b| a.dimension_key.cmp(&b.dimension_key));
    organization.blockers.sort();
}

fn canonical_zero(value: f64) -> f64 {
    if value == 0.0 {
        0.0
    } else {
        value
    }
}

fn hash_json(value: &impl Serialize) -> Result<String, OrganizationWindowError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| OrganizationWindowError::InvalidJson(error.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn current_teams() -> Vec<String> {
        CANONICAL_TEAMS
            .iter()
            .map(|(abbr, _)| (*abbr).to_owned())
            .collect()
    }

    fn manifest() -> OrganizationWindowManifestView {
        OrganizationWindowManifestView {
            schema: ORGANIZATION_WINDOW_MANIFEST_SCHEMA.to_owned(),
            manifest_id: "balanced.v1".to_owned(),
            label: "Balanced".to_owned(),
            description: "Synthetic foundation fixture".to_owned(),
            manifest_version: "1".to_owned(),
            comparison_cohort: WindowCohortManifest {
                kind: WindowCohortKind::CurrentNhl,
                team_catalog_version: "nhl-current-32.v1".to_owned(),
                expected_organizations: current_teams(),
            },
            normalization_method: WindowNormalizationMethod::EmpiricalPercentile,
            primary_horizon: WindowHorizon::ThreeYear,
            dimensions: vec![
                WindowDimensionManifest {
                    key: "nhl_strength".to_owned(),
                    label: "NHL strength".to_owned(),
                    weight: 0.6,
                    minimum_coverage: 1.0,
                    rank_required: true,
                    profiles: vec![WindowProfileWeight {
                        profile_key: "nhl.expected_points".to_owned(),
                        method_version: "icecast_expected_points.v1".to_owned(),
                        weight: 1.0,
                        required: true,
                    }],
                    signal_family_caps: vec![WindowSignalFamilyCap {
                        signal_family: "season_outlook".to_owned(),
                        maximum_weight: 1.0,
                    }],
                },
                WindowDimensionManifest {
                    key: "pipeline".to_owned(),
                    label: "Pipeline".to_owned(),
                    weight: 0.4,
                    minimum_coverage: 1.0,
                    rank_required: true,
                    profiles: vec![WindowProfileWeight {
                        profile_key: "pipeline.prospect_pool".to_owned(),
                        method_version: "prospect_pool_score.v1".to_owned(),
                        weight: 1.0,
                        required: true,
                    }],
                    signal_family_caps: vec![WindowSignalFamilyCap {
                        signal_family: "prospect_program".to_owned(),
                        maximum_weight: 1.0,
                    }],
                },
            ],
            missing_policy: WindowMissingPolicy::WithholdRank,
            classification_method: ORGANIZATION_WINDOW_CLASSIFICATION_METHOD.to_owned(),
            created_at: "2026-07-27T20:00:00-07:00".to_owned(),
            fingerprint: String::new(),
        }
    }

    fn inputs() -> Vec<OrganizationProfileInput> {
        let date = NaiveDate::from_ymd_opt(2026, 7, 27).expect("valid fixture date");
        current_teams()
            .into_iter()
            .enumerate()
            .flat_map(|(index, team)| {
                [
                    OrganizationProfileInput {
                        profile_key: "nhl.expected_points".to_owned(),
                        method_version: "icecast_expected_points.v1".to_owned(),
                        organization: team.clone(),
                        organization_identity_version: "nhl-current-32.v1".to_owned(),
                        season: 20_262_027,
                        season_type: "regular".to_owned(),
                        as_of: date,
                        horizon: WindowHorizon::ThreeYear,
                        raw_value: Some(70.0 + index as f64),
                        raw_unit: "standings_points".to_owned(),
                        sample_size: 10_000,
                        confidence: 0.9,
                        coverage: 1.0,
                        status: WindowProfileStatus::Modeled,
                        evidence: Vec::new(),
                        limitations: Vec::new(),
                        source_fingerprints: vec!["a".repeat(64)],
                    },
                    OrganizationProfileInput {
                        profile_key: "pipeline.prospect_pool".to_owned(),
                        method_version: "prospect_pool_score.v1".to_owned(),
                        organization: team,
                        organization_identity_version: "nhl-current-32.v1".to_owned(),
                        season: 20_262_027,
                        season_type: "regular".to_owned(),
                        as_of: date,
                        horizon: WindowHorizon::ThreeYear,
                        raw_value: Some(100.0 - index as f64),
                        raw_unit: "pipeline_score".to_owned(),
                        sample_size: 10,
                        confidence: 0.8,
                        coverage: 1.0,
                        status: WindowProfileStatus::Observed,
                        evidence: Vec::new(),
                        limitations: Vec::new(),
                        source_fingerprints: vec!["b".repeat(64)],
                    },
                ]
            })
            .collect()
    }

    #[test]
    fn inventory_has_exact_reviewed_readiness_counts() {
        let inventory = load_organization_window_profile_inventory().expect("valid inventory");
        assert_eq!(inventory.profiles.len(), 32);
        assert_eq!(inventory.counts.ready_for_adapter, 17);
        assert_eq!(inventory.counts.evaluation, 8);
        assert_eq!(inventory.counts.context_only, 4);
        assert_eq!(inventory.counts.blocked, 3);
        for key in [
            "resilience.injury_concentration",
            "deployment.shift_chemistry",
            "flexibility.confirmed_cap_space",
        ] {
            let row = inventory
                .profiles
                .iter()
                .find(|row| row.key == key)
                .expect("blocked profile exists");
            assert_eq!(row.readiness, WindowProfileReadiness::Blocked);
            assert!(!row.promotion_gaps.is_empty());
        }
    }

    #[test]
    fn manifest_fingerprint_is_order_independent_and_weight_sensitive() {
        let inventory = load_organization_window_profile_inventory().expect("valid inventory");
        let first =
            seal_organization_window_manifest(manifest(), &inventory).expect("valid manifest");
        let mut reordered = manifest();
        reordered.dimensions.reverse();
        reordered.comparison_cohort.expected_organizations.reverse();
        let second = seal_organization_window_manifest(reordered, &inventory)
            .expect("valid reordered manifest");
        assert_eq!(first.fingerprint, second.fingerprint);

        let mut changed = manifest();
        changed.dimensions[0].weight = 0.5;
        changed.dimensions[1].weight = 0.5;
        let changed =
            seal_organization_window_manifest(changed, &inventory).expect("valid changed manifest");
        assert_ne!(first.fingerprint, changed.fingerprint);
    }

    #[test]
    fn manifest_rejects_family_cap_and_non_finite_weight() {
        let inventory = load_organization_window_profile_inventory().expect("valid inventory");
        let mut capped = manifest();
        capped.dimensions[0].signal_family_caps[0].maximum_weight = 0.5;
        let error = seal_organization_window_manifest(capped, &inventory)
            .expect_err("cap must reject assigned weight");
        assert!(error.to_string().contains("above cap"));

        let mut non_finite = manifest();
        non_finite.dimensions[0].weight = f64::NAN;
        assert!(seal_organization_window_manifest(non_finite, &inventory).is_err());
    }

    #[test]
    fn synthetic_board_is_deterministic_ranked_and_explainable() {
        let inventory = load_organization_window_profile_inventory().expect("valid inventory");
        let date = NaiveDate::from_ymd_opt(2026, 7, 27).expect("valid fixture date");
        let build = |profile_inputs| OrganizationWindowBoardInput {
            season: 20_262_027,
            season_type: "regular".to_owned(),
            as_of: date,
            generated_at: "2026-07-27T20:00:00-07:00".to_owned(),
            manifest: manifest(),
            profile_inputs,
            source_fingerprints: vec!["c".repeat(64)],
        };
        let first =
            build_organization_window_board(build(inputs()), &inventory).expect("valid board");
        let mut reversed = inputs();
        reversed.reverse();
        let second = build_organization_window_board(build(reversed), &inventory)
            .expect("order-independent board");
        assert_eq!(first.fingerprint, second.fingerprint);
        assert_eq!(first.organizations.len(), 32);
        assert!(first
            .organizations
            .iter()
            .all(|row| row.overall.rank.is_some()));
        let nyr = first.organization("NYR").expect("Rangers row");
        assert_eq!(nyr.dimensions.len(), 2);
        assert!(nyr.overall.score.is_some());
        assert!(nyr.overall.confidence > 0.0);
        assert_eq!(nyr.overall.coverage, 1.0);
    }

    #[test]
    fn missing_required_profile_withholds_every_league_rank_without_zero_fill() {
        let inventory = load_organization_window_profile_inventory().expect("valid inventory");
        let date = NaiveDate::from_ymd_opt(2026, 7, 27).expect("valid fixture date");
        let mut profile_inputs = inputs();
        profile_inputs.retain(|row| {
            !(row.organization == "NYR" && row.profile_key == "pipeline.prospect_pool")
        });
        let board = build_organization_window_board(
            OrganizationWindowBoardInput {
                season: 20_262_027,
                season_type: "regular".to_owned(),
                as_of: date,
                generated_at: "2026-07-27T20:00:00-07:00".to_owned(),
                manifest: manifest(),
                profile_inputs,
                source_fingerprints: Vec::new(),
            },
            &inventory,
        )
        .expect("provisional board is still a valid artifact");
        assert!(board
            .organizations
            .iter()
            .all(|row| row.overall.rank.is_none()));
        let nyr = board.organization("NYR").expect("Rangers row");
        assert!(nyr.overall.score.is_some(), "missing is not zero-filled");
        assert!(nyr.overall.coverage < 1.0);
        assert!(!nyr.blockers.is_empty());
    }

    #[test]
    fn equal_profile_values_normalize_to_neutral_tied_rank() {
        let inventory = load_organization_window_profile_inventory().expect("valid inventory");
        let date = NaiveDate::from_ymd_opt(2026, 7, 27).expect("valid fixture date");
        let profile_inputs = inputs()
            .into_iter()
            .map(|mut row| {
                row.raw_value = Some(10.0);
                row
            })
            .collect();
        let board = build_organization_window_board(
            OrganizationWindowBoardInput {
                season: 20_262_027,
                season_type: "regular".to_owned(),
                as_of: date,
                generated_at: "2026-07-27T20:00:00-07:00".to_owned(),
                manifest: manifest(),
                profile_inputs,
                source_fingerprints: Vec::new(),
            },
            &inventory,
        )
        .expect("zero-variance board");
        for organization in board.organizations {
            assert_eq!(organization.overall.score, Some(50.0));
            assert_eq!(organization.overall.rank, Some(1));
            assert_eq!(organization.overall.percentile, Some(50.0));
            assert!(organization.dimensions.iter().all(|dimension| {
                dimension.profiles.iter().all(|profile| {
                    profile
                        .limitations
                        .contains(&"no_between_team_variance".to_owned())
                })
            }));
        }
    }

    #[test]
    fn partial_optional_profile_withholds_rank_for_the_whole_cohort() {
        let mut inventory = load_organization_window_profile_inventory().unwrap();
        inventory
            .profiles
            .iter_mut()
            .find(|profile| profile.key == "nhl.expected_points")
            .unwrap()
            .minimum_cohort = 31;
        let mut manifest = manifest();
        manifest.dimensions[0].profiles[0].required = false;
        let missing_team = current_teams()[0].clone();
        let mut profile_inputs = inputs();
        profile_inputs.retain(|row| row.organization != missing_team);
        let board = build_organization_window_board(
            OrganizationWindowBoardInput {
                season: 20_262_027,
                season_type: "regular".to_owned(),
                as_of: NaiveDate::from_ymd_opt(2026, 7, 27).unwrap(),
                generated_at: "2026-07-27T20:00:00-07:00".to_owned(),
                manifest,
                profile_inputs,
                source_fingerprints: Vec::new(),
            },
            &inventory,
        )
        .unwrap();
        assert!(board.organizations.iter().all(|row| {
            row.overall.rank.is_none()
                && row.overall.rank_status.state == WindowRankState::Withheld
                && row
                    .overall
                    .rank_status
                    .reasons
                    .iter()
                    .any(|reason| reason.contains("available for 31 of 32"))
        }));
    }

    #[test]
    fn inverse_and_target_range_directions_normalize_as_declared() {
        let inventory = load_organization_window_profile_inventory().unwrap();
        let base = inventory
            .find("nhl.team_strength", "icecast_team_strength.v1")
            .unwrap()
            .clone();
        let rows = |values: Vec<f64>| {
            current_teams()
                .into_iter()
                .zip(values)
                .map(|(organization, raw_value)| OrganizationProfileInput {
                    profile_key: base.key.clone(),
                    method_version: base.method_version.clone(),
                    organization,
                    organization_identity_version: "nhl_32.v1".to_owned(),
                    season: 20_262_027,
                    season_type: "regular".to_owned(),
                    as_of: NaiveDate::from_ymd_opt(2026, 7, 27).unwrap(),
                    horizon: WindowHorizon::Current,
                    raw_value: Some(raw_value),
                    raw_unit: base.raw_unit.clone(),
                    sample_size: 1,
                    confidence: 1.0,
                    coverage: 1.0,
                    status: WindowProfileStatus::Observed,
                    evidence: Vec::new(),
                    limitations: Vec::new(),
                    source_fingerprints: Vec::new(),
                })
                .collect::<Vec<_>>()
        };

        let mut inverse = base.clone();
        inverse.direction = WindowProfileDirection::LowerIsBetter;
        let mut inverse_rows = rows((0..32).map(|value| value as f64).collect());
        let mut inverse_output = BTreeMap::new();
        normalize_profile_cohort(&inverse, &mut inverse_rows, &mut inverse_output).unwrap();
        assert_eq!(
            inverse_output
                .values()
                .find(|row| row.raw_value == Some(0.0))
                .unwrap()
                .league_rank,
            Some(1)
        );

        let mut target = base.clone();
        target.direction = WindowProfileDirection::TargetRange;
        target.target_min = Some(10.0);
        target.target_max = Some(20.0);
        let mut target_rows = rows(
            [15.0, 0.0, 30.0]
                .into_iter()
                .chain((0..29).map(|value| 40.0 + value as f64))
                .collect(),
        );
        let mut target_output = BTreeMap::new();
        normalize_profile_cohort(&target, &mut target_rows, &mut target_output).unwrap();
        assert_eq!(
            target_output
                .values()
                .find(|row| row.raw_value == Some(15.0))
                .unwrap()
                .league_rank,
            Some(1)
        );
        assert_eq!(
            target_output
                .values()
                .find(|row| row.raw_value == Some(0.0))
                .unwrap()
                .league_rank,
            target_output
                .values()
                .find(|row| row.raw_value == Some(30.0))
                .unwrap()
                .league_rank
        );
    }

    #[test]
    fn inventory_rejects_unknown_and_cyclic_dependencies() {
        let inventory = load_organization_window_profile_inventory().unwrap();
        let mut unknown = inventory.clone();
        unknown.profiles[0].dependencies = vec!["unknown@v1".to_owned()];
        assert!(matches!(
            validate_profile_inventory(&unknown),
            Err(OrganizationWindowError::UnknownDependency(_))
        ));

        let mut cyclic = inventory;
        let first = cyclic.profiles[0].id();
        let second = cyclic.profiles[1].id();
        cyclic.profiles[0].dependencies = vec![second];
        cyclic.profiles[1].dependencies = vec![first];
        assert!(matches!(
            validate_profile_inventory(&cyclic),
            Err(OrganizationWindowError::DependencyCycle(_))
        ));
    }

    #[test]
    fn registered_profile_extension_flows_through_the_generic_scorer() {
        let mut inventory = load_organization_window_profile_inventory().unwrap();
        let mut extension = inventory
            .profiles
            .iter()
            .find(|profile| profile.key == "nhl.expected_points")
            .unwrap()
            .clone();
        extension.key = "nhl.extension_fixture".to_owned();
        extension.method_version = "extension_fixture.v1".to_owned();
        extension.label = "Extension fixture".to_owned();
        extension.source_schema = "extension_fixture.v1".to_owned();
        extension.dependencies.clear();
        inventory.profiles.push(extension);
        inventory.counts.total += 1;
        inventory.counts.ready_for_adapter += 1;
        validate_profile_inventory(&inventory).unwrap();

        let mut manifest = manifest();
        let nhl = manifest
            .dimensions
            .iter_mut()
            .find(|dimension| dimension.key == "nhl_strength")
            .unwrap();
        nhl.profiles[0].weight = 0.9;
        nhl.profiles.push(WindowProfileWeight {
            profile_key: "nhl.extension_fixture".to_owned(),
            method_version: "extension_fixture.v1".to_owned(),
            weight: 0.1,
            required: true,
        });

        let date = NaiveDate::from_ymd_opt(2026, 7, 27).unwrap();
        let mut profile_inputs = inputs();
        profile_inputs.extend(current_teams().into_iter().enumerate().map(
            |(index, organization)| OrganizationProfileInput {
                profile_key: "nhl.extension_fixture".to_owned(),
                method_version: "extension_fixture.v1".to_owned(),
                organization,
                organization_identity_version: "nhl-current-32.v1".to_owned(),
                season: 20_262_027,
                season_type: "regular".to_owned(),
                as_of: date,
                horizon: WindowHorizon::ThreeYear,
                raw_value: Some(65.0 + index as f64),
                raw_unit: "standings_points".to_owned(),
                sample_size: 32,
                confidence: 0.75,
                coverage: 1.0,
                status: WindowProfileStatus::Modeled,
                evidence: Vec::new(),
                limitations: vec!["W9 extension-kit fixture".to_owned()],
                source_fingerprints: vec!["e".repeat(64)],
            },
        ));
        let board = build_organization_window_board(
            OrganizationWindowBoardInput {
                season: 20_262_027,
                season_type: "regular".to_owned(),
                as_of: date,
                generated_at: "2026-07-27T20:00:00-07:00".to_owned(),
                manifest,
                profile_inputs,
                source_fingerprints: vec!["f".repeat(64)],
            },
            &inventory,
        )
        .unwrap();

        assert_eq!(board.organizations.len(), 32);
        assert!(board.organizations.iter().all(|organization| organization
            .dimensions
            .iter()
            .find(|dimension| dimension.key == "nhl_strength")
            .unwrap()
            .profiles
            .iter()
            .any(|profile| profile.profile_key == "nhl.extension_fixture")));
    }

    #[test]
    fn canonical_window_fingerprint_is_cross_platform_and_replay_valid() {
        let inventory = load_organization_window_profile_inventory().unwrap();
        let mut board: OrganizationWindowBoardView = serde_json::from_str(include_str!(
            "../../../examples/organization-window-board-evaluation-2026-27.json"
        ))
        .unwrap();

        assert_eq!(
            board.manifest.fingerprint,
            "d736832af289254240834ebfe9c1a19a92bf927879489e8ffb465ffea54e3365"
        );
        assert_eq!(
            board.fingerprint,
            "3f0c171287fdfb4aeb4efaf9f610698b0480301fb9e7686764c66fbede8203eb"
        );
        validate_organization_window_board(&board, &inventory).unwrap();

        let profile = &mut board.organizations[0].dimensions[0].profiles[0];
        profile.normalized_score = Some(profile.normalized_score.unwrap_or_default() + 1.0);
        board.fingerprint = board.calculate_fingerprint().unwrap();
        assert!(matches!(
            validate_organization_window_board(&board, &inventory),
            Err(OrganizationWindowError::InvalidBoard(_))
        ));

        board.schema = "organization_window_board.v2".to_owned();
        board.fingerprint = board.calculate_fingerprint().unwrap();
        assert!(matches!(
            validate_organization_window_board(&board, &inventory),
            Err(OrganizationWindowError::UnsupportedSchema {
                contract: "board",
                ..
            })
        ));
    }

    #[test]
    fn classification_boundaries_cover_every_declared_state() {
        let dimensions = |current: f64, pipeline: f64| {
            [("nhl_strength", current), ("pipeline", pipeline)]
                .into_iter()
                .map(|(key, score)| WindowDimensionView {
                    key: key.to_owned(),
                    label: key.to_owned(),
                    weight: 0.5,
                    score: Some(score),
                    confidence: 1.0,
                    coverage: 1.0,
                    status: WindowAggregateStatus::Complete,
                    profiles: Vec::new(),
                    blockers: Vec::new(),
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            classify_window(None, &[]),
            WindowClassification::EvaluationIncomplete
        );
        assert_eq!(
            classify_window(Some(80.0), &dimensions(80.0, 70.0)),
            WindowClassification::Contender
        );
        assert_eq!(
            classify_window(Some(80.0), &dimensions(80.0, 50.0)),
            WindowClassification::FragileContender
        );
        assert_eq!(
            classify_window(Some(70.0), &dimensions(70.0, 70.0)),
            WindowClassification::RisingContender
        );
        assert_eq!(
            classify_window(Some(50.0), &dimensions(50.0, 50.0)),
            WindowClassification::Plateau
        );
        assert_eq!(
            classify_window(Some(40.0), &dimensions(40.0, 60.0)),
            WindowClassification::Retooling
        );
        assert_eq!(
            classify_window(Some(30.0), &dimensions(30.0, 30.0)),
            WindowClassification::Rebuilding
        );
    }
}
