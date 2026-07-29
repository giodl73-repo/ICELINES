//! Lifecycle amendments for newly authored Organization Window Frames.
//!
//! The base v1 inventory remains immutable replay authority. This module adds
//! a separately sealed policy layer so deprecation, retirement, supersession,
//! and readiness demotion can constrain new Frames without changing old
//! observations, manifests, or boards.

use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::organization_window::{
    seal_organization_window_manifest, OrganizationWindowError, OrganizationWindowManifestView,
    OrganizationWindowProfileInventory, WindowProfileReadiness,
    ORGANIZATION_WINDOW_REGISTRY_VERSION,
};

pub const ORGANIZATION_WINDOW_REGISTRY_LIFECYCLE_SCHEMA: &str =
    "organization_window_registry_lifecycle.v1";
pub const ORGANIZATION_WINDOW_REGISTRY_LIFECYCLE_JSON_SCHEMA: &str =
    include_str!("../../../design/schemas/organization_window_registry_lifecycle.v1.schema.json");
pub const ORGANIZATION_WINDOW_REGISTRY_LIFECYCLE_JSON: &str =
    include_str!("../../../design/data/organization-window-registry-lifecycle.v1.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowProfileLifecycle {
    Active,
    Deprecated,
    Retired,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct WindowProfileMethodRef {
    pub profile_key: String,
    pub method_version: String,
}

impl WindowProfileMethodRef {
    pub fn id(&self) -> String {
        format!("{}@{}", self.profile_key, self.method_version)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowProfileLifecycleEntry {
    pub profile_key: String,
    pub method_version: String,
    pub lifecycle: WindowProfileLifecycle,
    pub effective_date: NaiveDate,
    pub rationale: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readiness_override: Option<WindowProfileReadiness>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replacement: Option<WindowProfileMethodRef>,
    #[serde(default)]
    pub affected_official_frames: Vec<String>,
    #[serde(default)]
    pub official_frame_holds: Vec<WindowDeprecatedProfileHold>,
    pub review_evidence: Vec<String>,
}

impl WindowProfileLifecycleEntry {
    pub fn id(&self) -> String {
        format!("{}@{}", self.profile_key, self.method_version)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationWindowRegistryLifecycleView {
    pub schema: String,
    pub base_registry_version: String,
    pub revision: String,
    pub generated_at: String,
    pub default_lifecycle: WindowProfileLifecycle,
    #[serde(default)]
    pub entries: Vec<WindowProfileLifecycleEntry>,
    #[serde(default)]
    pub fingerprint: String,
}

impl OrganizationWindowRegistryLifecycleView {
    pub fn entry(
        &self,
        profile_key: &str,
        method_version: &str,
    ) -> Option<&WindowProfileLifecycleEntry> {
        self.entries.iter().find(|entry| {
            entry.profile_key == profile_key && entry.method_version == method_version
        })
    }

    pub fn lifecycle(&self, profile_key: &str, method_version: &str) -> WindowProfileLifecycle {
        self.entry(profile_key, method_version)
            .map(|entry| entry.lifecycle)
            .unwrap_or(self.default_lifecycle)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowManifestAuthoringKind {
    Official,
    Evaluation,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowDeprecatedProfileHold {
    pub manifest_id: String,
    pub manifest_fingerprint: String,
    pub rationale: String,
    pub approved_by: String,
    pub reviewed_at: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowManifestLifecyclePolicy {
    pub kind: WindowManifestAuthoringKind,
}

impl WindowManifestLifecyclePolicy {
    pub fn official() -> Self {
        Self {
            kind: WindowManifestAuthoringKind::Official,
        }
    }

    pub fn custom() -> Self {
        Self {
            kind: WindowManifestAuthoringKind::Custom,
        }
    }

    pub fn evaluation() -> Self {
        Self {
            kind: WindowManifestAuthoringKind::Evaluation,
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum OrganizationWindowRegistryLifecycleError {
    #[error("unsupported Window registry lifecycle schema: {0}")]
    UnsupportedSchema(String),
    #[error("invalid Window registry lifecycle: {0}")]
    InvalidLifecycle(String),
    #[error("Window registry lifecycle fingerprint mismatch")]
    FingerprintMismatch,
    #[error("Window manifest is invalid: {0}")]
    InvalidManifest(String),
}

pub fn load_organization_window_registry_lifecycle(
    inventory: &OrganizationWindowProfileInventory,
) -> Result<OrganizationWindowRegistryLifecycleView, OrganizationWindowRegistryLifecycleError> {
    let lifecycle: OrganizationWindowRegistryLifecycleView =
        serde_json::from_str(ORGANIZATION_WINDOW_REGISTRY_LIFECYCLE_JSON).map_err(|error| {
            OrganizationWindowRegistryLifecycleError::InvalidLifecycle(error.to_string())
        })?;
    seal_organization_window_registry_lifecycle(lifecycle, inventory)
}

pub fn seal_organization_window_registry_lifecycle(
    mut lifecycle: OrganizationWindowRegistryLifecycleView,
    inventory: &OrganizationWindowProfileInventory,
) -> Result<OrganizationWindowRegistryLifecycleView, OrganizationWindowRegistryLifecycleError> {
    validate_lifecycle_fields(&lifecycle, inventory)?;
    let supplied_fingerprint = lifecycle.fingerprint.clone();
    canonicalize_lifecycle(&mut lifecycle);
    lifecycle.fingerprint.clear();
    let calculated = hash_json(&lifecycle)?;
    if !supplied_fingerprint.is_empty() && supplied_fingerprint != calculated {
        return Err(OrganizationWindowRegistryLifecycleError::FingerprintMismatch);
    }
    lifecycle.fingerprint = calculated;
    Ok(lifecycle)
}

pub fn seal_new_organization_window_manifest(
    manifest: OrganizationWindowManifestView,
    inventory: &OrganizationWindowProfileInventory,
    lifecycle: &OrganizationWindowRegistryLifecycleView,
    policy: &WindowManifestLifecyclePolicy,
) -> Result<OrganizationWindowManifestView, OrganizationWindowRegistryLifecycleError> {
    let lifecycle = seal_organization_window_registry_lifecycle(lifecycle.clone(), inventory)?;
    let manifest =
        seal_organization_window_manifest(manifest, inventory).map_err(map_manifest_error)?;
    validate_manifest_lifecycle(&manifest, inventory, &lifecycle, policy)?;
    Ok(manifest)
}

fn validate_lifecycle_fields(
    lifecycle: &OrganizationWindowRegistryLifecycleView,
    inventory: &OrganizationWindowProfileInventory,
) -> Result<(), OrganizationWindowRegistryLifecycleError> {
    if lifecycle.schema != ORGANIZATION_WINDOW_REGISTRY_LIFECYCLE_SCHEMA {
        return Err(OrganizationWindowRegistryLifecycleError::UnsupportedSchema(
            lifecycle.schema.clone(),
        ));
    }
    if lifecycle.base_registry_version != ORGANIZATION_WINDOW_REGISTRY_VERSION {
        return Err(OrganizationWindowRegistryLifecycleError::InvalidLifecycle(
            format!(
                "base registry {} does not match {}",
                lifecycle.base_registry_version, ORGANIZATION_WINDOW_REGISTRY_VERSION
            ),
        ));
    }
    if lifecycle.default_lifecycle != WindowProfileLifecycle::Active {
        return Err(OrganizationWindowRegistryLifecycleError::InvalidLifecycle(
            "default lifecycle must be active".to_owned(),
        ));
    }
    require_text("revision", &lifecycle.revision)?;
    require_text("generated_at", &lifecycle.generated_at)?;
    let generated_date = chrono::DateTime::parse_from_rfc3339(&lifecycle.generated_at)
        .map_err(|error| {
            OrganizationWindowRegistryLifecycleError::InvalidLifecycle(format!(
                "generated_at is not RFC 3339: {error}"
            ))
        })?
        .date_naive();

    let mut ids = BTreeSet::new();
    for entry in &lifecycle.entries {
        let id = entry.id();
        if !ids.insert(id.clone()) {
            return Err(OrganizationWindowRegistryLifecycleError::InvalidLifecycle(
                format!("duplicate lifecycle entry {id}"),
            ));
        }
        let descriptor = inventory
            .find(&entry.profile_key, &entry.method_version)
            .ok_or_else(|| {
                OrganizationWindowRegistryLifecycleError::InvalidLifecycle(format!(
                    "unknown lifecycle profile {id}"
                ))
            })?;
        require_text("rationale", &entry.rationale)?;
        if entry.effective_date > generated_date {
            return Err(OrganizationWindowRegistryLifecycleError::InvalidLifecycle(
                format!("{id} is not effective when the lifecycle was generated"),
            ));
        }
        validate_text_list("affected official frame", &entry.affected_official_frames)?;
        validate_text_list("review evidence", &entry.review_evidence)?;
        if entry.review_evidence.is_empty() {
            return Err(OrganizationWindowRegistryLifecycleError::InvalidLifecycle(
                format!("{id} has no review evidence"),
            ));
        }
        if let Some(readiness) = entry.readiness_override {
            if readiness_severity(readiness) <= readiness_severity(descriptor.readiness) {
                return Err(OrganizationWindowRegistryLifecycleError::InvalidLifecycle(
                    format!("{id} readiness override is not a demotion"),
                ));
            }
        }
        if entry.lifecycle == WindowProfileLifecycle::Active && entry.replacement.is_some() {
            return Err(OrganizationWindowRegistryLifecycleError::InvalidLifecycle(
                format!("active profile {id} cannot declare a replacement"),
            ));
        }
        if entry.lifecycle != WindowProfileLifecycle::Deprecated
            && !entry.official_frame_holds.is_empty()
        {
            return Err(OrganizationWindowRegistryLifecycleError::InvalidLifecycle(
                format!("only deprecated profile {id} may declare official Frame holds"),
            ));
        }
        let mut held_manifests = BTreeSet::new();
        for hold in &entry.official_frame_holds {
            require_text("hold manifest ID", &hold.manifest_id)?;
            require_text("hold rationale", &hold.rationale)?;
            require_text("hold approver", &hold.approved_by)?;
            if !is_sha256(&hold.manifest_fingerprint) {
                return Err(OrganizationWindowRegistryLifecycleError::InvalidLifecycle(
                    format!(
                        "{id} hold for {} has an invalid manifest fingerprint",
                        hold.manifest_id
                    ),
                ));
            }
            if hold.reviewed_at > generated_date {
                return Err(OrganizationWindowRegistryLifecycleError::InvalidLifecycle(
                    format!(
                        "{id} hold for {} was reviewed after the lifecycle was generated",
                        hold.manifest_id
                    ),
                ));
            }
            if !entry.affected_official_frames.contains(&hold.manifest_id) {
                return Err(OrganizationWindowRegistryLifecycleError::InvalidLifecycle(
                    format!(
                        "{id} hold for {} is absent from affected official Frames",
                        hold.manifest_id
                    ),
                ));
            }
            if !held_manifests.insert((
                hold.manifest_id.as_str(),
                hold.manifest_fingerprint.as_str(),
            )) {
                return Err(OrganizationWindowRegistryLifecycleError::InvalidLifecycle(
                    format!(
                        "{id} has a duplicate hold for {}@{}",
                        hold.manifest_id, hold.manifest_fingerprint
                    ),
                ));
            }
        }
        if let Some(replacement) = &entry.replacement {
            if replacement.id() == id {
                return Err(OrganizationWindowRegistryLifecycleError::InvalidLifecycle(
                    format!("{id} cannot replace itself"),
                ));
            }
            if inventory
                .find(&replacement.profile_key, &replacement.method_version)
                .is_none()
            {
                return Err(OrganizationWindowRegistryLifecycleError::InvalidLifecycle(
                    format!("{id} has unknown replacement {}", replacement.id()),
                ));
            }
        }
    }

    let entries = lifecycle
        .entries
        .iter()
        .map(|entry| (entry.id(), entry))
        .collect::<BTreeMap<_, _>>();
    for entry in &lifecycle.entries {
        if let Some(replacement) = &entry.replacement {
            if lifecycle.lifecycle(&replacement.profile_key, &replacement.method_version)
                == WindowProfileLifecycle::Retired
            {
                return Err(OrganizationWindowRegistryLifecycleError::InvalidLifecycle(
                    format!("{} replacement {} is retired", entry.id(), replacement.id()),
                ));
            }
        }
        let mut seen = BTreeSet::new();
        let mut current = entry;
        while let Some(replacement) = &current.replacement {
            if !seen.insert(current.id()) {
                return Err(OrganizationWindowRegistryLifecycleError::InvalidLifecycle(
                    format!("supersession cycle includes {}", current.id()),
                ));
            }
            let Some(next) = entries.get(&replacement.id()) else {
                break;
            };
            current = next;
        }
    }
    Ok(())
}

fn validate_manifest_lifecycle(
    manifest: &OrganizationWindowManifestView,
    inventory: &OrganizationWindowProfileInventory,
    lifecycle: &OrganizationWindowRegistryLifecycleView,
    policy: &WindowManifestLifecyclePolicy,
) -> Result<(), OrganizationWindowRegistryLifecycleError> {
    let selected = manifest
        .dimensions
        .iter()
        .flat_map(|dimension| &dimension.profiles)
        .map(|profile| WindowProfileMethodRef {
            profile_key: profile.profile_key.clone(),
            method_version: profile.method_version.clone(),
        })
        .collect::<BTreeSet<_>>();
    for profile in &selected {
        let descriptor = inventory
            .find(&profile.profile_key, &profile.method_version)
            .expect("sealed manifest references a validated descriptor");
        let entry = lifecycle.entry(&profile.profile_key, &profile.method_version);
        let state = entry
            .map(|entry| entry.lifecycle)
            .unwrap_or(lifecycle.default_lifecycle);
        let readiness = entry
            .and_then(|entry| entry.readiness_override)
            .unwrap_or(descriptor.readiness);
        if state == WindowProfileLifecycle::Retired {
            return Err(OrganizationWindowRegistryLifecycleError::InvalidManifest(
                format!("new Frame cannot select retired profile {}", profile.id()),
            ));
        }
        if readiness == WindowProfileReadiness::Blocked
            || (policy.kind == WindowManifestAuthoringKind::Official
                && readiness != WindowProfileReadiness::ReadyForAdapter)
            || (policy.kind == WindowManifestAuthoringKind::Evaluation
                && readiness == WindowProfileReadiness::ContextOnly)
        {
            return Err(OrganizationWindowRegistryLifecycleError::InvalidManifest(
                format!(
                    "new {:?} Frame cannot select {:?} profile {}",
                    policy.kind,
                    readiness,
                    profile.id()
                ),
            ));
        }
        if state == WindowProfileLifecycle::Deprecated
            && policy.kind != WindowManifestAuthoringKind::Custom
            && !entry.is_some_and(|entry| {
                entry.official_frame_holds.iter().any(|hold| {
                    hold.manifest_id == manifest.manifest_id
                        && hold.manifest_fingerprint == manifest.fingerprint
                })
            })
        {
            return Err(OrganizationWindowRegistryLifecycleError::InvalidManifest(
                format!(
                    "official Frame requires an explicit hold for deprecated profile {}",
                    profile.id()
                ),
            ));
        }
    }
    Ok(())
}

fn canonicalize_lifecycle(lifecycle: &mut OrganizationWindowRegistryLifecycleView) {
    lifecycle
        .entries
        .sort_by_key(WindowProfileLifecycleEntry::id);
    for entry in &mut lifecycle.entries {
        entry.affected_official_frames.sort();
        entry.affected_official_frames.dedup();
        entry.official_frame_holds.sort_by(|left, right| {
            (&left.manifest_id, &left.manifest_fingerprint)
                .cmp(&(&right.manifest_id, &right.manifest_fingerprint))
        });
        entry.review_evidence.sort();
        entry.review_evidence.dedup();
    }
}

fn require_text(field: &str, value: &str) -> Result<(), OrganizationWindowRegistryLifecycleError> {
    if value.trim().is_empty() {
        return Err(OrganizationWindowRegistryLifecycleError::InvalidLifecycle(
            format!("{field} is empty"),
        ));
    }
    Ok(())
}

fn validate_text_list(
    field: &str,
    values: &[String],
) -> Result<(), OrganizationWindowRegistryLifecycleError> {
    if values.iter().any(|value| value.trim().is_empty()) {
        return Err(OrganizationWindowRegistryLifecycleError::InvalidLifecycle(
            format!("{field} contains an empty value"),
        ));
    }
    let unique = values.iter().collect::<BTreeSet<_>>();
    if unique.len() != values.len() {
        return Err(OrganizationWindowRegistryLifecycleError::InvalidLifecycle(
            format!("{field} contains duplicates"),
        ));
    }
    Ok(())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn readiness_severity(readiness: WindowProfileReadiness) -> u8 {
    match readiness {
        WindowProfileReadiness::ReadyForAdapter => 0,
        WindowProfileReadiness::Evaluation => 1,
        WindowProfileReadiness::ContextOnly => 2,
        WindowProfileReadiness::Blocked => 3,
    }
}

fn map_manifest_error(error: OrganizationWindowError) -> OrganizationWindowRegistryLifecycleError {
    OrganizationWindowRegistryLifecycleError::InvalidManifest(error.to_string())
}

fn hash_json<T: Serialize>(value: &T) -> Result<String, OrganizationWindowRegistryLifecycleError> {
    let bytes = serde_json::to_vec(value).map_err(|error| {
        OrganizationWindowRegistryLifecycleError::InvalidLifecycle(error.to_string())
    })?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_model::{
        balanced_organization_window_manifest, load_organization_window_profile_inventory,
        validate_organization_window_board, OrganizationWindowBoardView,
    };

    fn inventory() -> OrganizationWindowProfileInventory {
        load_organization_window_profile_inventory().unwrap()
    }

    fn lifecycle() -> OrganizationWindowRegistryLifecycleView {
        load_organization_window_registry_lifecycle(&inventory()).unwrap()
    }

    fn selected_profile() -> WindowProfileMethodRef {
        let manifest = balanced_organization_window_manifest("2026-07-27T00:00:00Z");
        let profile = &manifest.dimensions[0].profiles[0];
        WindowProfileMethodRef {
            profile_key: profile.profile_key.clone(),
            method_version: profile.method_version.clone(),
        }
    }

    fn amendment(
        state: WindowProfileLifecycle,
        replacement: Option<WindowProfileMethodRef>,
    ) -> OrganizationWindowRegistryLifecycleView {
        let mut lifecycle = lifecycle();
        let selected = selected_profile();
        lifecycle.entries.push(WindowProfileLifecycleEntry {
            profile_key: selected.profile_key,
            method_version: selected.method_version,
            lifecycle: state,
            effective_date: NaiveDate::from_ymd_opt(2026, 7, 29).unwrap(),
            rationale: "Reviewed lifecycle change".to_owned(),
            readiness_override: None,
            replacement,
            affected_official_frames: vec!["balanced.v1".to_owned()],
            official_frame_holds: Vec::new(),
            review_evidence: vec!["review://window/lifecycle/1".to_owned()],
        });
        lifecycle.fingerprint.clear();
        lifecycle
    }

    #[test]
    fn default_lifecycle_seals_current_official_frame() {
        let inventory = inventory();
        let schema: serde_json::Value =
            serde_json::from_str(ORGANIZATION_WINDOW_REGISTRY_LIFECYCLE_JSON_SCHEMA).unwrap();
        assert_eq!(
            schema["properties"]["schema"]["const"],
            ORGANIZATION_WINDOW_REGISTRY_LIFECYCLE_SCHEMA
        );
        let sealed = seal_new_organization_window_manifest(
            balanced_organization_window_manifest("2026-07-27T00:00:00Z"),
            &inventory,
            &lifecycle(),
            &WindowManifestLifecyclePolicy::official(),
        )
        .unwrap();
        assert_eq!(sealed.manifest_id, "balanced.v1");
    }

    #[test]
    fn deprecated_official_profile_requires_explicit_hold() {
        let inventory = inventory();
        let mut lifecycle = amendment(WindowProfileLifecycle::Deprecated, None);
        let manifest = seal_organization_window_manifest(
            balanced_organization_window_manifest("2026-07-27T00:00:00Z"),
            &inventory,
        )
        .unwrap();
        assert!(seal_new_organization_window_manifest(
            manifest.clone(),
            &inventory,
            &lifecycle,
            &WindowManifestLifecyclePolicy::official(),
        )
        .is_err());
        assert!(seal_new_organization_window_manifest(
            manifest.clone(),
            &inventory,
            &lifecycle,
            &WindowManifestLifecyclePolicy::evaluation(),
        )
        .is_err());
        assert!(seal_new_organization_window_manifest(
            manifest.clone(),
            &inventory,
            &lifecycle,
            &WindowManifestLifecyclePolicy::custom(),
        )
        .is_ok());

        lifecycle.entries[0].official_frame_holds = vec![WindowDeprecatedProfileHold {
            manifest_id: manifest.manifest_id.clone(),
            manifest_fingerprint: "0".repeat(64),
            rationale: "Replacement is not cohort-ready".to_owned(),
            approved_by: "Window review board".to_owned(),
            reviewed_at: NaiveDate::from_ymd_opt(2026, 7, 29).unwrap(),
        }];
        lifecycle.fingerprint.clear();
        assert!(seal_new_organization_window_manifest(
            manifest.clone(),
            &inventory,
            &lifecycle,
            &WindowManifestLifecyclePolicy::official(),
        )
        .is_err());

        lifecycle.entries[0].official_frame_holds[0].manifest_fingerprint =
            manifest.fingerprint.clone();
        lifecycle.fingerprint.clear();
        assert!(seal_new_organization_window_manifest(
            manifest.clone(),
            &inventory,
            &lifecycle,
            &WindowManifestLifecyclePolicy::official(),
        )
        .is_ok());
        assert!(seal_new_organization_window_manifest(
            manifest,
            &inventory,
            &lifecycle,
            &WindowManifestLifecyclePolicy::evaluation(),
        )
        .is_ok());
    }

    #[test]
    fn retired_profile_is_replayable_but_not_newly_selectable() {
        let inventory = inventory();
        let lifecycle = amendment(WindowProfileLifecycle::Retired, None);
        let manifest = balanced_organization_window_manifest("2026-07-27T00:00:00Z");
        let replay = seal_organization_window_manifest(manifest.clone(), &inventory).unwrap();
        assert!(!replay.fingerprint.is_empty());
        let selected = selected_profile();
        assert!(replay.dimensions.iter().any(|dimension| {
            dimension.profiles.iter().any(|profile| {
                profile.profile_key == selected.profile_key
                    && profile.method_version == selected.method_version
            })
        }));
        let board: OrganizationWindowBoardView = serde_json::from_str(include_str!(
            "../../../examples/organization-window-board-partial-2026-07-28.json"
        ))
        .unwrap();
        assert!(validate_organization_window_board(&board, &inventory).is_ok());
        for policy in [
            WindowManifestLifecyclePolicy::official(),
            WindowManifestLifecyclePolicy::evaluation(),
            WindowManifestLifecyclePolicy::custom(),
        ] {
            assert!(seal_new_organization_window_manifest(
                manifest.clone(),
                &inventory,
                &lifecycle,
                &policy,
            )
            .is_err());
        }
    }

    #[test]
    fn readiness_demotion_does_not_mutate_base_descriptor_or_replay() {
        let inventory = inventory();
        let selected = selected_profile();
        let base_readiness = inventory
            .find(&selected.profile_key, &selected.method_version)
            .unwrap()
            .readiness;
        let mut lifecycle = amendment(WindowProfileLifecycle::Active, None);
        lifecycle.entries[0].readiness_override = Some(WindowProfileReadiness::Evaluation);
        let manifest = balanced_organization_window_manifest("2026-07-27T00:00:00Z");
        assert!(seal_organization_window_manifest(manifest.clone(), &inventory).is_ok());
        assert!(seal_new_organization_window_manifest(
            manifest.clone(),
            &inventory,
            &lifecycle,
            &WindowManifestLifecyclePolicy::official(),
        )
        .is_err());
        assert!(seal_new_organization_window_manifest(
            manifest.clone(),
            &inventory,
            &lifecycle,
            &WindowManifestLifecyclePolicy::evaluation(),
        )
        .is_ok());
        assert!(seal_new_organization_window_manifest(
            manifest,
            &inventory,
            &lifecycle,
            &WindowManifestLifecyclePolicy::custom(),
        )
        .is_ok());
        assert_eq!(
            inventory
                .find(&selected.profile_key, &selected.method_version)
                .unwrap()
                .readiness,
            base_readiness
        );
    }

    #[test]
    fn supersession_rejects_unknown_retired_and_cyclic_replacements() {
        let inventory = inventory();
        let selected = selected_profile();
        let mut unknown = amendment(
            WindowProfileLifecycle::Deprecated,
            Some(WindowProfileMethodRef {
                profile_key: "unknown".to_owned(),
                method_version: "v1".to_owned(),
            }),
        );
        assert!(seal_organization_window_registry_lifecycle(unknown.clone(), &inventory).is_err());

        let other = inventory
            .profiles
            .iter()
            .find(|profile| profile.id() != selected.id())
            .unwrap();
        unknown.entries[0].replacement = Some(WindowProfileMethodRef {
            profile_key: other.key.clone(),
            method_version: other.method_version.clone(),
        });
        unknown.entries.push(WindowProfileLifecycleEntry {
            profile_key: other.key.clone(),
            method_version: other.method_version.clone(),
            lifecycle: WindowProfileLifecycle::Retired,
            effective_date: NaiveDate::from_ymd_opt(2026, 7, 29).unwrap(),
            rationale: "Retired replacement".to_owned(),
            readiness_override: None,
            replacement: Some(selected),
            affected_official_frames: Vec::new(),
            official_frame_holds: Vec::new(),
            review_evidence: vec!["review://window/lifecycle/2".to_owned()],
        });
        assert!(seal_organization_window_registry_lifecycle(unknown.clone(), &inventory).is_err());

        unknown.entries[1].lifecycle = WindowProfileLifecycle::Deprecated;
        assert!(seal_organization_window_registry_lifecycle(unknown, &inventory).is_err());
    }

    #[test]
    fn lifecycle_fingerprint_is_order_invariant_and_tamper_evident() {
        let inventory = inventory();
        let first = seal_organization_window_registry_lifecycle(
            amendment(WindowProfileLifecycle::Deprecated, None),
            &inventory,
        )
        .unwrap();
        let mut reordered = first.clone();
        reordered.entries.reverse();
        assert_eq!(
            seal_organization_window_registry_lifecycle(reordered, &inventory)
                .unwrap()
                .fingerprint,
            first.fingerprint
        );
        let mut changed = first;
        changed.entries[0].rationale.push_str(" changed");
        assert_eq!(
            seal_organization_window_registry_lifecycle(changed, &inventory),
            Err(OrganizationWindowRegistryLifecycleError::FingerprintMismatch)
        );
    }
}
