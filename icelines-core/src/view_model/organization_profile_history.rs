//! Standing, point-in-time history for Organization Window profile inputs.
//!
//! The ledger stores raw profile observations rather than normalized board
//! ranks. A later preseason board may carry an observation forward only under
//! an explicit policy, with confidence decay and stale evidence attached.

use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::organization_window::{
    OrganizationProfileInput, OrganizationWindowBoardView, WindowEvidenceView, WindowFreshness,
    WindowProfileStatus,
};
use crate::teams::CANONICAL_TEAMS;

pub const ORGANIZATION_PROFILE_HISTORY_SCHEMA: &str = "organization_profile_history.v1";
pub const ORGANIZATION_PROFILE_HISTORY_JSON_SCHEMA: &str =
    include_str!("../../../design/schemas/organization_profile_history.v1.schema.json");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationProfileHistoryView {
    pub schema: String,
    pub history_id: String,
    pub created_at: String,
    pub organization_identity_version: String,
    pub observations: Vec<OrganizationProfileInput>,
    #[serde(default)]
    pub source_fingerprints: Vec<String>,
    #[serde(default)]
    pub disclosures: Vec<String>,
    #[serde(default)]
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationProfileCarryForwardRule {
    pub profile_key: String,
    pub method_version: String,
    pub maximum_season_age: u32,
    pub annual_confidence_decay: f64,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum OrganizationProfileHistoryError {
    #[error("unsupported organization profile history schema {0}")]
    UnsupportedSchema(String),
    #[error("invalid organization profile history: {0}")]
    Invalid(String),
    #[error("organization profile history serialization failed: {0}")]
    Serialization(String),
}

pub fn build_organization_profile_history(
    history_id: impl Into<String>,
    created_at: impl Into<String>,
    boards: &[OrganizationWindowBoardView],
) -> Result<OrganizationProfileHistoryView, OrganizationProfileHistoryError> {
    if boards.is_empty() {
        return Err(OrganizationProfileHistoryError::Invalid(
            "at least one sealed board is required".to_owned(),
        ));
    }
    let organization_identity_version = boards[0]
        .manifest
        .comparison_cohort
        .team_catalog_version
        .clone();
    let mut observations = Vec::new();
    let mut source_fingerprints = Vec::new();
    for board in boards {
        if board.fingerprint.trim().is_empty()
            || board.manifest.comparison_cohort.team_catalog_version
                != organization_identity_version
        {
            return Err(OrganizationProfileHistoryError::Invalid(
                "boards must be sealed and use one organization identity version".to_owned(),
            ));
        }
        source_fingerprints.push(format!("organization-window-board:{}", board.fingerprint));
        for organization in &board.organizations {
            for profile in organization
                .dimensions
                .iter()
                .flat_map(|dimension| &dimension.profiles)
            {
                observations.push(OrganizationProfileInput {
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
                });
            }
        }
    }
    seal_organization_profile_history(OrganizationProfileHistoryView {
        schema: ORGANIZATION_PROFILE_HISTORY_SCHEMA.to_owned(),
        history_id: history_id.into(),
        created_at: created_at.into(),
        organization_identity_version,
        observations,
        source_fingerprints,
        disclosures: vec![
            "The standing history stores raw point-in-time profile observations; normalized ranks remain properties of their sealed source boards.".to_owned(),
            "Historical observations never become current silently. Carry-forward requires an explicit profile policy and emits modeled, stale evidence with confidence decay.".to_owned(),
        ],
        fingerprint: String::new(),
    })
}

pub fn seal_organization_profile_history(
    mut history: OrganizationProfileHistoryView,
) -> Result<OrganizationProfileHistoryView, OrganizationProfileHistoryError> {
    validate_history_header(&history)?;
    for observation in &mut history.observations {
        observation.source_fingerprints.sort();
        observation.source_fingerprints.dedup();
    }
    history.observations.sort_by(observation_order);
    history.source_fingerprints.sort();
    history.source_fingerprints.dedup();
    history.disclosures.sort();
    history.disclosures.dedup();
    validate_observations(&history)?;

    let supplied = std::mem::take(&mut history.fingerprint);
    // Hash the JSON-round-tripped representation. Floating-point profile
    // values can otherwise retain in-memory precision that is not reproduced
    // after the sealed document is loaded from JSON.
    let canonical = serde_json::to_vec(&history)
        .map_err(|error| OrganizationProfileHistoryError::Serialization(error.to_string()))?;
    history = serde_json::from_slice(&canonical)
        .map_err(|error| OrganizationProfileHistoryError::Serialization(error.to_string()))?;
    let bytes = serde_json::to_vec(&history)
        .map_err(|error| OrganizationProfileHistoryError::Serialization(error.to_string()))?;
    let calculated = format!("{:x}", Sha256::digest(bytes));
    if !supplied.is_empty() && supplied != calculated {
        return Err(OrganizationProfileHistoryError::Invalid(format!(
            "profile history fingerprint mismatch: supplied {supplied}, calculated {calculated}"
        )));
    }
    history.fingerprint = calculated;
    Ok(history)
}

pub fn carry_forward_organization_profiles(
    history: &OrganizationProfileHistoryView,
    target_season: u32,
    target_as_of: NaiveDate,
    rules: &[OrganizationProfileCarryForwardRule],
) -> Result<Vec<OrganizationProfileInput>, OrganizationProfileHistoryError> {
    let history = seal_organization_profile_history(history.clone())?;
    validate_season(target_season)?;
    if rules.is_empty() {
        return Ok(Vec::new());
    }
    let mut configured = BTreeSet::new();
    for rule in rules {
        if rule.profile_key.trim().is_empty()
            || rule.method_version.trim().is_empty()
            || rule.maximum_season_age == 0
            || !rule.annual_confidence_decay.is_finite()
            || !(0.0..=1.0).contains(&rule.annual_confidence_decay)
            || !configured.insert((rule.profile_key.as_str(), rule.method_version.as_str()))
        {
            return Err(OrganizationProfileHistoryError::Invalid(
                "carry-forward rules must be unique, non-empty, bounded, and permit at least one prior season"
                    .to_owned(),
            ));
        }
    }

    let rule_map = rules
        .iter()
        .map(|rule| {
            (
                (rule.profile_key.as_str(), rule.method_version.as_str()),
                rule,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut latest = BTreeMap::<(&str, &str, &str), (&OrganizationProfileInput, u32)>::new();
    for observation in &history.observations {
        let Some(rule) = rule_map.get(&(
            observation.profile_key.as_str(),
            observation.method_version.as_str(),
        )) else {
            continue;
        };
        let Some(age) = season_age(observation.season, target_season) else {
            continue;
        };
        if age == 0 || age > rule.maximum_season_age || !score_eligible(observation) {
            continue;
        }
        let key = (
            observation.organization.as_str(),
            observation.profile_key.as_str(),
            observation.method_version.as_str(),
        );
        if latest
            .get(&key)
            .is_none_or(|(_, current_age)| age < *current_age)
        {
            latest.insert(key, (observation, age));
        }
    }

    let mut output = latest
        .into_values()
        .map(|(source, age)| {
            let rule = rule_map[&(
                source.profile_key.as_str(),
                source.method_version.as_str(),
            )];
            let mut evidence = source.evidence.clone();
            evidence.push(WindowEvidenceView {
                source_schema: ORGANIZATION_PROFILE_HISTORY_SCHEMA.to_owned(),
                source_id: history.fingerprint.clone(),
                captured_at: Some(history.created_at.clone()),
                as_of: Some(source.as_of),
                freshness: WindowFreshness::Stale,
                source_url: None,
            });
            let mut limitations = source.limitations.clone();
            limitations.push(format!(
                "preseason carry-forward from {} ({} season old); replace with target-season authority when available",
                source.season, age
            ));
            let mut source_fingerprints = source.source_fingerprints.clone();
            source_fingerprints.push(format!("organization-profile-history:{}", history.fingerprint));
            source_fingerprints.sort();
            source_fingerprints.dedup();
            OrganizationProfileInput {
                profile_key: source.profile_key.clone(),
                method_version: source.method_version.clone(),
                organization: source.organization.clone(),
                organization_identity_version: history.organization_identity_version.clone(),
                season: target_season,
                season_type: source.season_type.clone(),
                as_of: target_as_of,
                horizon: source.horizon,
                raw_value: source.raw_value,
                raw_unit: source.raw_unit.clone(),
                sample_size: source.sample_size,
                confidence: source.confidence * rule.annual_confidence_decay.powi(age as i32),
                coverage: source.coverage,
                status: WindowProfileStatus::Modeled,
                evidence,
                limitations,
                source_fingerprints,
            }
        })
        .collect::<Vec<_>>();
    output.sort_by(observation_order);
    Ok(output)
}

fn validate_history_header(
    history: &OrganizationProfileHistoryView,
) -> Result<(), OrganizationProfileHistoryError> {
    if history.schema != ORGANIZATION_PROFILE_HISTORY_SCHEMA {
        return Err(OrganizationProfileHistoryError::UnsupportedSchema(
            history.schema.clone(),
        ));
    }
    if history.history_id.trim().is_empty()
        || history.created_at.trim().is_empty()
        || history.organization_identity_version.trim().is_empty()
        || history.observations.is_empty()
    {
        return Err(OrganizationProfileHistoryError::Invalid(
            "history id, creation time, identity version, and observations are required".to_owned(),
        ));
    }
    Ok(())
}

fn validate_observations(
    history: &OrganizationProfileHistoryView,
) -> Result<(), OrganizationProfileHistoryError> {
    let teams = CANONICAL_TEAMS
        .iter()
        .map(|(team, _)| *team)
        .collect::<BTreeSet<_>>();
    let mut identities = BTreeSet::new();
    for observation in &history.observations {
        validate_season(observation.season)?;
        let identity = (
            observation.season,
            observation.season_type.as_str(),
            observation.as_of,
            observation.organization.as_str(),
            observation.profile_key.as_str(),
            observation.method_version.as_str(),
            observation.horizon,
        );
        if !teams.contains(observation.organization.as_str())
            || observation.organization_identity_version != history.organization_identity_version
            || observation.profile_key.trim().is_empty()
            || observation.method_version.trim().is_empty()
            || observation.raw_unit.trim().is_empty()
            || !observation.confidence.is_finite()
            || !(0.0..=1.0).contains(&observation.confidence)
            || !observation.coverage.is_finite()
            || !(0.0..=1.0).contains(&observation.coverage)
            || observation
                .raw_value
                .is_some_and(|value| !value.is_finite())
            || !identities.insert(identity)
            || (score_eligible(observation) && observation.raw_value.is_none())
        {
            return Err(OrganizationProfileHistoryError::Invalid(format!(
                "invalid or duplicate observation {}:{}@{} for {}",
                observation.season,
                observation.profile_key,
                observation.method_version,
                observation.organization
            )));
        }
    }
    Ok(())
}

fn score_eligible(observation: &OrganizationProfileInput) -> bool {
    matches!(
        observation.status,
        WindowProfileStatus::Observed
            | WindowProfileStatus::Modeled
            | WindowProfileStatus::Provisional
    )
}

fn validate_season(season: u32) -> Result<(), OrganizationProfileHistoryError> {
    let start = season / 10_000;
    let end = season % 10_000;
    if start < 1900 || end != start + 1 {
        return Err(OrganizationProfileHistoryError::Invalid(format!(
            "invalid season {season}"
        )));
    }
    Ok(())
}

fn season_age(source: u32, target: u32) -> Option<u32> {
    let source_start = source / 10_000;
    let target_start = target / 10_000;
    (target_start >= source_start).then_some(target_start - source_start)
}

fn observation_order(
    left: &OrganizationProfileInput,
    right: &OrganizationProfileInput,
) -> std::cmp::Ordering {
    left.season
        .cmp(&right.season)
        .then_with(|| left.as_of.cmp(&right.as_of))
        .then_with(|| left.organization.cmp(&right.organization))
        .then_with(|| left.profile_key.cmp(&right.profile_key))
        .then_with(|| left.method_version.cmp(&right.method_version))
        .then_with(|| left.horizon.cmp(&right.horizon))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_model::organization_window::{WindowHorizon, WindowProfileStatus};

    fn observation(team: &str, season: u32, value: f64) -> OrganizationProfileInput {
        OrganizationProfileInput {
            profile_key: "development.organization_depth".to_owned(),
            method_version: "organization_lineup_depth.v1".to_owned(),
            organization: team.to_owned(),
            organization_identity_version: "nhl_32.v1".to_owned(),
            season,
            season_type: "regular".to_owned(),
            as_of: NaiveDate::from_ymd_opt((season / 10_000 + 1) as i32, 6, 30).unwrap(),
            horizon: WindowHorizon::Current,
            raw_value: Some(value),
            raw_unit: "average_unit_score".to_owned(),
            sample_size: 16,
            confidence: 0.8,
            coverage: 1.0,
            status: WindowProfileStatus::Observed,
            evidence: Vec::new(),
            limitations: Vec::new(),
            source_fingerprints: vec![format!("source:{team}:{season}")],
        }
    }

    fn history(observations: Vec<OrganizationProfileInput>) -> OrganizationProfileHistoryView {
        seal_organization_profile_history(OrganizationProfileHistoryView {
            schema: ORGANIZATION_PROFILE_HISTORY_SCHEMA.to_owned(),
            history_id: "standing-window".to_owned(),
            created_at: "2026-07-29T12:00:00Z".to_owned(),
            organization_identity_version: "nhl_32.v1".to_owned(),
            observations,
            source_fingerprints: vec!["boards:test".to_owned()],
            disclosures: Vec::new(),
            fingerprint: String::new(),
        })
        .unwrap()
    }

    #[test]
    fn carry_forward_uses_latest_prior_season_and_decays_confidence() {
        let history = history(vec![
            observation("NYR", 20242025, 40.0),
            observation("NYR", 20252026, 55.0),
        ]);
        let carried = carry_forward_organization_profiles(
            &history,
            20262027,
            NaiveDate::from_ymd_opt(2026, 7, 29).unwrap(),
            &[OrganizationProfileCarryForwardRule {
                profile_key: "development.organization_depth".to_owned(),
                method_version: "organization_lineup_depth.v1".to_owned(),
                maximum_season_age: 2,
                annual_confidence_decay: 0.75,
            }],
        )
        .unwrap();
        assert_eq!(carried.len(), 1);
        assert_eq!(carried[0].raw_value, Some(55.0));
        assert!((carried[0].confidence - 0.6).abs() < 1e-12);
        assert_eq!(carried[0].status, WindowProfileStatus::Modeled);
        assert_eq!(
            carried[0].evidence.last().unwrap().freshness,
            WindowFreshness::Stale
        );
    }

    #[test]
    fn sealing_rejects_duplicate_point_in_time_observations() {
        let row = observation("SEA", 20252026, 50.0);
        let error = seal_organization_profile_history(OrganizationProfileHistoryView {
            schema: ORGANIZATION_PROFILE_HISTORY_SCHEMA.to_owned(),
            history_id: "duplicates".to_owned(),
            created_at: "2026-07-29T12:00:00Z".to_owned(),
            organization_identity_version: "nhl_32.v1".to_owned(),
            observations: vec![row.clone(), row],
            source_fingerprints: Vec::new(),
            disclosures: Vec::new(),
            fingerprint: String::new(),
        })
        .unwrap_err();
        assert!(error.to_string().contains("duplicate"));
    }

    #[test]
    fn sealed_history_replays_after_json_round_trip() {
        let sealed = history(vec![observation("NYR", 20252026, 36.817603728033994)]);
        let json = serde_json::to_vec_pretty(&sealed).unwrap();
        let loaded: OrganizationProfileHistoryView = serde_json::from_slice(&json).unwrap();
        assert_eq!(seal_organization_profile_history(loaded).unwrap(), sealed);
    }

    #[test]
    fn profile_history_schema_is_valid_json() {
        let schema: serde_json::Value =
            serde_json::from_str(ORGANIZATION_PROFILE_HISTORY_JSON_SCHEMA).unwrap();
        assert_eq!(
            schema["$id"],
            "https://icelines.app/schemas/organization_profile_history.v1.schema.json"
        );
    }

    #[test]
    fn observed_2025_26_example_is_sealed_and_complete_for_both_profiles() {
        let loaded: OrganizationProfileHistoryView = serde_json::from_str(include_str!(
            "../../../examples/organization-profile-history-observed-2025-26.json"
        ))
        .unwrap();
        let sealed = seal_organization_profile_history(loaded).unwrap();
        assert_eq!(sealed.observations.len(), 64);
        assert_eq!(
            sealed
                .observations
                .iter()
                .map(|row| row.organization.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            32
        );
    }
}
