//! Stable scenario identity and provenance contracts shared by CLI, TUI, web,
//! simulation, and card builders.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{model::Season, season_stats::SeasonType};

use super::EvidenceLabel;

pub const SCENARIO_REGISTRY_SCHEMA: &str = "scenario_registry.v1";
pub const SCENARIO_REGISTRY_ENTRY_SCHEMA: &str = "scenario_registry_entry.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioScopeView {
    pub league_id: String,
    pub season: u32,
    pub season_type: SeasonType,
    pub team_ids: Vec<String>,
    pub calendar_fingerprint: Option<String>,
}

impl ScenarioScopeView {
    pub fn validate_compatible_with(
        &self,
        expected: &ScenarioScopeView,
    ) -> Result<(), ScenarioRegistryContractError> {
        if self.league_id != expected.league_id {
            return Err(ScenarioRegistryContractError::ScopeMismatch("league_id"));
        }
        if self.season != expected.season {
            return Err(ScenarioRegistryContractError::ScopeMismatch("season"));
        }
        if self.season_type != expected.season_type {
            return Err(ScenarioRegistryContractError::ScopeMismatch("season_type"));
        }
        if let Some(expected_calendar) = &expected.calendar_fingerprint {
            if self.calendar_fingerprint.as_ref() != Some(expected_calendar) {
                return Err(ScenarioRegistryContractError::ScopeMismatch(
                    "calendar_fingerprint",
                ));
            }
        }
        if !expected.team_ids.is_empty()
            && !self.team_ids.is_empty()
            && expected
                .team_ids
                .iter()
                .any(|team| !self.team_ids.contains(team))
        {
            return Err(ScenarioRegistryContractError::ScopeMismatch("team_ids"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioRegistryEntryView {
    pub schema: String,
    pub id: String,
    pub scenario_schema: String,
    pub scope: ScenarioScopeView,
    pub evidence_label: EvidenceLabel,
    pub content_sha256: String,
    pub imported_at: DateTime<Utc>,
    pub source_name: String,
}

impl ScenarioRegistryEntryView {
    pub fn validate(&self) -> Result<(), ScenarioRegistryContractError> {
        if self.schema != SCENARIO_REGISTRY_ENTRY_SCHEMA {
            return Err(ScenarioRegistryContractError::UnsupportedEntrySchema(
                self.schema.clone(),
            ));
        }
        validate_scenario_id(&self.id)?;
        require_text("scenario_schema", &self.scenario_schema)?;
        require_text("scope.league_id", &self.scope.league_id)?;
        require_text("source_name", &self.source_name)?;
        Season::try_new(self.scope.season)
            .map_err(|_| ScenarioRegistryContractError::InvalidSeason(self.scope.season))?;
        validate_sha256("content_sha256", &self.content_sha256)?;
        if let Some(calendar) = &self.scope.calendar_fingerprint {
            validate_sha256("calendar_fingerprint", calendar)?;
        }
        let mut previous: Option<&str> = None;
        for team in &self.scope.team_ids {
            if team.len() != 3
                || !team
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() && byte.is_ascii_alphabetic())
            {
                return Err(ScenarioRegistryContractError::InvalidTeamId(team.clone()));
            }
            if previous.is_some_and(|value| value >= team.as_str()) {
                return Err(ScenarioRegistryContractError::TeamIdsNotCanonical);
            }
            previous = Some(team);
        }
        Ok(())
    }

    pub fn reference(&self) -> ScenarioRegistryReferenceView {
        ScenarioRegistryReferenceView {
            id: self.id.clone(),
            scenario_schema: self.scenario_schema.clone(),
            scope: self.scope.clone(),
            evidence_label: self.evidence_label,
            content_sha256: self.content_sha256.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioRegistryReferenceView {
    pub id: String,
    pub scenario_schema: String,
    pub scope: ScenarioScopeView,
    pub evidence_label: EvidenceLabel,
    pub content_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioRegistryView {
    pub schema: String,
    pub entries: Vec<ScenarioRegistryEntryView>,
}

impl Default for ScenarioRegistryView {
    fn default() -> Self {
        Self {
            schema: SCENARIO_REGISTRY_SCHEMA.to_string(),
            entries: Vec::new(),
        }
    }
}

impl ScenarioRegistryView {
    pub fn validate(&self) -> Result<(), ScenarioRegistryContractError> {
        if self.schema != SCENARIO_REGISTRY_SCHEMA {
            return Err(ScenarioRegistryContractError::UnsupportedRegistrySchema(
                self.schema.clone(),
            ));
        }
        let mut previous: Option<&str> = None;
        for entry in &self.entries {
            entry.validate()?;
            if previous.is_some_and(|value| value >= entry.id.as_str()) {
                return Err(ScenarioRegistryContractError::EntriesNotCanonical);
            }
            previous = Some(&entry.id);
        }
        Ok(())
    }
}

pub fn scenario_content_sha256<T: Serialize>(
    scenario: &T,
) -> Result<String, ScenarioRegistryContractError> {
    let bytes = serde_json::to_vec(scenario)
        .map_err(|error| ScenarioRegistryContractError::Serialization(error.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub fn validate_scenario_id(id: &str) -> Result<(), ScenarioRegistryContractError> {
    if id.is_empty()
        || id.len() > 96
        || !id.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
        || !id
            .as_bytes()
            .first()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    {
        return Err(ScenarioRegistryContractError::InvalidScenarioId(
            id.to_string(),
        ));
    }
    Ok(())
}

fn validate_sha256(field: &'static str, value: &str) -> Result<(), ScenarioRegistryContractError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ScenarioRegistryContractError::InvalidSha256(field));
    }
    Ok(())
}

fn require_text(field: &'static str, value: &str) -> Result<(), ScenarioRegistryContractError> {
    if value.trim().is_empty() {
        Err(ScenarioRegistryContractError::MissingText(field))
    } else {
        Ok(())
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ScenarioRegistryContractError {
    #[error("unsupported scenario registry schema: {0}")]
    UnsupportedRegistrySchema(String),
    #[error("unsupported scenario registry entry schema: {0}")]
    UnsupportedEntrySchema(String),
    #[error("scenario id must be a lowercase stable slug of at most 96 characters: {0}")]
    InvalidScenarioId(String),
    #[error("scenario registry field is empty: {0}")]
    MissingText(&'static str),
    #[error("invalid scenario season: {0}")]
    InvalidSeason(u32),
    #[error("invalid uppercase three-letter team id: {0}")]
    InvalidTeamId(String),
    #[error("scenario team ids must be sorted and unique")]
    TeamIdsNotCanonical,
    #[error("scenario registry entries must be sorted by unique id")]
    EntriesNotCanonical,
    #[error("scenario scope does not match requested {0}")]
    ScopeMismatch(&'static str),
    #[error("scenario registry field is not a lowercase SHA-256 value: {0}")]
    InvalidSha256(&'static str),
    #[error("scenario serialization failed: {0}")]
    Serialization(String),
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn entry(id: &str) -> ScenarioRegistryEntryView {
        ScenarioRegistryEntryView {
            schema: SCENARIO_REGISTRY_ENTRY_SCHEMA.to_string(),
            id: id.to_string(),
            scenario_schema: "team_season_scenario.v1".to_string(),
            scope: ScenarioScopeView {
                league_id: "nhl".to_string(),
                season: 20262027,
                season_type: SeasonType::Regular,
                team_ids: vec!["NYR".to_string(), "SEA".to_string()],
                calendar_fingerprint: Some("a".repeat(64)),
            },
            evidence_label: EvidenceLabel::Estimated,
            content_sha256: "b".repeat(64),
            imported_at: Utc.with_ymd_and_hms(2026, 7, 21, 18, 0, 0).unwrap(),
            source_name: "fixture.json".to_string(),
        }
    }

    #[test]
    fn registry_contract_accepts_canonical_scope_and_stable_reference() {
        let entry = entry("nyr-development-variance");
        entry.validate().unwrap();
        let reference = entry.reference();
        assert_eq!(reference.id, entry.id);
        assert_eq!(reference.content_sha256, entry.content_sha256);
    }

    #[test]
    fn registry_contract_rejects_paths_unsorted_teams_and_changed_schema() {
        assert!(matches!(
            validate_scenario_id("../scenario.json"),
            Err(ScenarioRegistryContractError::InvalidScenarioId(_))
        ));
        let mut invalid = entry("valid-id");
        invalid.scope.team_ids.reverse();
        assert_eq!(
            invalid.validate(),
            Err(ScenarioRegistryContractError::TeamIdsNotCanonical)
        );
        invalid.scope.team_ids.sort();
        invalid.schema = "scenario_registry_entry.v2".to_string();
        assert!(matches!(
            invalid.validate(),
            Err(ScenarioRegistryContractError::UnsupportedEntrySchema(_))
        ));
    }

    #[test]
    fn scenario_scope_refuses_cross_season_team_and_calendar_use() {
        let scope = entry("scope").scope;
        let mut expected = scope.clone();
        expected.team_ids = vec!["NYR".to_string()];
        scope.validate_compatible_with(&expected).unwrap();

        expected.season = 20252026;
        assert_eq!(
            scope.validate_compatible_with(&expected),
            Err(ScenarioRegistryContractError::ScopeMismatch("season"))
        );
        expected = scope.clone();
        expected.team_ids = vec!["BOS".to_string()];
        assert_eq!(
            scope.validate_compatible_with(&expected),
            Err(ScenarioRegistryContractError::ScopeMismatch("team_ids"))
        );
    }

    #[test]
    fn registry_and_content_hashes_are_deterministic_and_content_sensitive() {
        let first = serde_json::json!({"name": "baseline", "events": []});
        let second = serde_json::json!({"name": "changed", "events": []});
        assert_eq!(
            scenario_content_sha256(&first).unwrap(),
            scenario_content_sha256(&first).unwrap()
        );
        assert_ne!(
            scenario_content_sha256(&first).unwrap(),
            scenario_content_sha256(&second).unwrap()
        );

        let registry = ScenarioRegistryView {
            entries: vec![entry("a"), entry("b")],
            ..ScenarioRegistryView::default()
        };
        registry.validate().unwrap();
    }
}
