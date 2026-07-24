//! Content-addressed local scenario registry.
//!
//! Interactive surfaces resolve stable IDs through this store. Only explicit
//! CLI experimentation reads arbitrary scenario paths before importing them.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use icelines_core::{
    scenario_content_sha256, EvidenceLabel, ScenarioRegistryContractError,
    ScenarioRegistryEntryView, ScenarioRegistryReferenceView, ScenarioRegistryView,
    ScenarioScopeView, TeamSeasonScenario, TeamSeasonScenarioEventKind,
    SCENARIO_REGISTRY_ENTRY_SCHEMA, TEAM_SEASON_SCENARIO_SCHEMA,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::atomic_write::{write_bytes_atomic, write_json_atomic};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioImportDisposition {
    Inserted,
    Unchanged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioImportResult {
    pub disposition: ScenarioImportDisposition,
    pub entry: ScenarioRegistryEntryView,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedTeamSeasonScenario {
    pub reference: ScenarioRegistryReferenceView,
    pub scenario: TeamSeasonScenario,
}

pub struct ScenarioRegistryStore {
    root: PathBuf,
}

impl ScenarioRegistryStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn default_root() -> PathBuf {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".icelines").join("scenarios")
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn list(&self) -> Result<ScenarioRegistryView, ScenarioRegistryStoreError> {
        let manifest = self.load_manifest()?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn import_team_season_scenario(
        &self,
        id: &str,
        mut scope: ScenarioScopeView,
        evidence_label: EvidenceLabel,
        scenario: &TeamSeasonScenario,
        imported_at: DateTime<Utc>,
        source_name: impl Into<String>,
    ) -> Result<ScenarioImportResult, ScenarioRegistryStoreError> {
        validate_team_season_scenario(scenario)?;
        let event_teams = scenario
            .events
            .iter()
            .map(|event| event.team.clone())
            .collect::<BTreeSet<_>>();
        if scope.team_ids.is_empty() {
            scope.team_ids = event_teams.iter().cloned().collect();
        } else if event_teams
            .iter()
            .any(|team| !scope.team_ids.contains(team))
        {
            return Err(ScenarioRegistryStoreError::EventOutsideScope);
        }
        let content_sha256 = scenario_content_sha256(scenario)?;
        let entry = ScenarioRegistryEntryView {
            schema: SCENARIO_REGISTRY_ENTRY_SCHEMA.to_string(),
            id: id.to_string(),
            scenario_schema: TEAM_SEASON_SCENARIO_SCHEMA.to_string(),
            scope,
            evidence_label,
            content_sha256: content_sha256.clone(),
            imported_at,
            source_name: source_name.into(),
        };
        entry.validate()?;

        let mut manifest = self.load_manifest()?;
        manifest.validate()?;
        if let Some(existing) = manifest.entries.iter().find(|existing| existing.id == id) {
            if existing.content_sha256 == content_sha256
                && existing.scope == entry.scope
                && existing.scenario_schema == entry.scenario_schema
                && existing.evidence_label == entry.evidence_label
            {
                return Ok(ScenarioImportResult {
                    disposition: ScenarioImportDisposition::Unchanged,
                    entry: existing.clone(),
                });
            }
            if existing.content_sha256 == content_sha256 {
                return Err(ScenarioRegistryStoreError::MetadataConflict { id: id.to_string() });
            }
            return Err(ScenarioRegistryStoreError::IdConflict {
                id: id.to_string(),
                existing_sha256: existing.content_sha256.clone(),
                incoming_sha256: content_sha256,
            });
        }

        let content = serde_json::to_vec_pretty(scenario)?;
        write_bytes_atomic(&self.content_path(&entry.content_sha256), &content)?;
        manifest.entries.push(entry.clone());
        manifest.entries.sort_by(|a, b| a.id.cmp(&b.id));
        manifest.validate()?;
        write_json_atomic(&self.index_path(), &manifest)?;
        Ok(ScenarioImportResult {
            disposition: ScenarioImportDisposition::Inserted,
            entry,
        })
    }

    pub fn resolve_team_season_scenario(
        &self,
        id: &str,
        expected_scope: &ScenarioScopeView,
    ) -> Result<ResolvedTeamSeasonScenario, ScenarioRegistryStoreError> {
        let manifest = self.load_manifest()?;
        manifest.validate()?;
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.id == id)
            .ok_or_else(|| ScenarioRegistryStoreError::NotFound(id.to_string()))?;
        if entry.scenario_schema != TEAM_SEASON_SCENARIO_SCHEMA {
            return Err(ScenarioRegistryStoreError::WrongScenarioSchema(
                entry.scenario_schema.clone(),
            ));
        }
        entry.scope.validate_compatible_with(expected_scope)?;
        let path = self.content_path(&entry.content_sha256);
        let bytes = std::fs::read(&path)?;
        let scenario: TeamSeasonScenario = serde_json::from_slice(&bytes)?;
        validate_team_season_scenario(&scenario)?;
        let actual_sha256 = scenario_content_sha256(&scenario)?;
        if actual_sha256 != entry.content_sha256 {
            return Err(ScenarioRegistryStoreError::IntegrityMismatch {
                id: id.to_string(),
                expected_sha256: entry.content_sha256.clone(),
                actual_sha256,
            });
        }
        Ok(ResolvedTeamSeasonScenario {
            reference: entry.reference(),
            scenario,
        })
    }

    fn load_manifest(&self) -> Result<ScenarioRegistryView, ScenarioRegistryStoreError> {
        let path = self.index_path();
        if !path.exists() {
            return Ok(ScenarioRegistryView::default());
        }
        Ok(serde_json::from_slice(&std::fs::read(path)?)?)
    }

    fn index_path(&self) -> PathBuf {
        self.root.join("index.json")
    }

    fn content_path(&self, sha256: &str) -> PathBuf {
        self.root.join("content").join(format!("{sha256}.json"))
    }
}

fn validate_team_season_scenario(
    scenario: &TeamSeasonScenario,
) -> Result<(), ScenarioRegistryStoreError> {
    if scenario.name.trim().is_empty() {
        return Err(ScenarioRegistryStoreError::InvalidScenario(
            "name cannot be empty".to_string(),
        ));
    }
    let mut ids = BTreeSet::new();
    for event in &scenario.events {
        if event.id.trim().is_empty() || !ids.insert(event.id.as_str()) {
            return Err(ScenarioRegistryStoreError::InvalidScenario(
                "event IDs must be non-empty and unique".to_string(),
            ));
        }
        if event.team.len() != 3
            || !event
                .team
                .bytes()
                .all(|byte| byte.is_ascii_uppercase() && byte.is_ascii_alphabetic())
        {
            return Err(ScenarioRegistryStoreError::InvalidScenario(format!(
                "event {} has invalid team {}",
                event.id, event.team
            )));
        }
        if event.label.trim().is_empty() {
            return Err(ScenarioRegistryStoreError::InvalidScenario(format!(
                "event {} has an empty label",
                event.id
            )));
        }
        if event.end_date.is_some_and(|end| end < event.effective_date) {
            return Err(ScenarioRegistryStoreError::InvalidScenario(format!(
                "event {} ends before it starts",
                event.id
            )));
        }
        if !event.strength_delta.is_finite() || !(-50.0..=50.0).contains(&event.strength_delta) {
            return Err(ScenarioRegistryStoreError::InvalidScenario(format!(
                "event {} strength_delta must be between -50 and 50",
                event.id
            )));
        }
        if !event.occurrence_probability.is_finite()
            || !(0.0..=1.0).contains(&event.occurrence_probability)
        {
            return Err(ScenarioRegistryStoreError::InvalidScenario(format!(
                "event {} occurrence_probability must be between 0 and 1",
                event.id
            )));
        }
        if event.kind == TeamSeasonScenarioEventKind::Trade {
            let deadline = scenario.trade_deadline.ok_or_else(|| {
                ScenarioRegistryStoreError::InvalidScenario(format!(
                    "trade event {} requires trade_deadline",
                    event.id
                ))
            })?;
            if event.effective_date > deadline {
                return Err(ScenarioRegistryStoreError::InvalidScenario(format!(
                    "trade event {} occurs after trade_deadline",
                    event.id
                )));
            }
        }
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum ScenarioRegistryStoreError {
    #[error(transparent)]
    Contract(#[from] ScenarioRegistryContractError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("scenario id {id} already points to {existing_sha256}; incoming content is {incoming_sha256}")]
    IdConflict {
        id: String,
        existing_sha256: String,
        incoming_sha256: String,
    },
    #[error("scenario id {id} already uses the same content with different immutable scope or evidence metadata")]
    MetadataConflict { id: String },
    #[error("scenario not found in registry: {0}")]
    NotFound(String),
    #[error("scenario events are outside the declared team scope")]
    EventOutsideScope,
    #[error("registry entry has unsupported scenario schema: {0}")]
    WrongScenarioSchema(String),
    #[error("scenario {id} integrity mismatch: expected {expected_sha256}, got {actual_sha256}")]
    IntegrityMismatch {
        id: String,
        expected_sha256: String,
        actual_sha256: String,
    },
    #[error("invalid team-season scenario: {0}")]
    InvalidScenario(String),
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone};
    use icelines_core::{season_stats::SeasonType, TeamSeasonScenarioEvent};

    use super::*;

    fn scope() -> ScenarioScopeView {
        ScenarioScopeView {
            league_id: "nhl".to_string(),
            season: 20262027,
            season_type: SeasonType::Regular,
            team_ids: Vec::new(),
            calendar_fingerprint: None,
        }
    }

    fn scenario(delta: f64) -> TeamSeasonScenario {
        TeamSeasonScenario {
            name: "Rangers development variance".to_string(),
            trade_deadline: Some(NaiveDate::from_ymd_opt(2027, 3, 5).unwrap()),
            events: vec![TeamSeasonScenarioEvent {
                id: "kartye-breakout".to_string(),
                kind: TeamSeasonScenarioEventKind::Form,
                team: "NYR".to_string(),
                player: Some("Tye Kartye".to_string()),
                effective_date: NaiveDate::from_ymd_opt(2026, 10, 7).unwrap(),
                end_date: None,
                strength_delta: delta,
                occurrence_probability: 0.2,
                correlation_key: None,
                label: "Kartye breakout".to_string(),
            }],
            adaptive_lineup_policies: Vec::new(),
            opening_roster_policies: Vec::new(),
        }
    }

    #[test]
    fn import_is_idempotent_and_changed_content_requires_a_new_id() {
        let temp = tempfile::tempdir().unwrap();
        let store = ScenarioRegistryStore::new(temp.path());
        let imported_at = Utc.with_ymd_and_hms(2026, 7, 21, 18, 0, 0).unwrap();
        let inserted = store
            .import_team_season_scenario(
                "nyr-development-variance",
                scope(),
                EvidenceLabel::Estimated,
                &scenario(2.5),
                imported_at,
                "fixture.json",
            )
            .unwrap();
        assert_eq!(inserted.disposition, ScenarioImportDisposition::Inserted);
        assert_eq!(inserted.entry.scope.team_ids, vec!["NYR"]);

        let unchanged = store
            .import_team_season_scenario(
                "nyr-development-variance",
                scope(),
                EvidenceLabel::Estimated,
                &scenario(2.5),
                imported_at,
                "fixture.json",
            )
            .unwrap();
        assert_eq!(unchanged.disposition, ScenarioImportDisposition::Unchanged);
        assert!(matches!(
            store.import_team_season_scenario(
                "nyr-development-variance",
                scope(),
                EvidenceLabel::Confirmed,
                &scenario(2.5),
                imported_at,
                "fixture.json",
            ),
            Err(ScenarioRegistryStoreError::MetadataConflict { .. })
        ));
        assert!(matches!(
            store.import_team_season_scenario(
                "nyr-development-variance",
                scope(),
                EvidenceLabel::Estimated,
                &scenario(3.0),
                imported_at,
                "changed.json",
            ),
            Err(ScenarioRegistryStoreError::IdConflict { .. })
        ));
    }

    #[test]
    fn resolve_checks_scope_and_content_integrity() {
        let temp = tempfile::tempdir().unwrap();
        let store = ScenarioRegistryStore::new(temp.path());
        let result = store
            .import_team_season_scenario(
                "nyr-development-variance",
                scope(),
                EvidenceLabel::Estimated,
                &scenario(2.5),
                Utc.with_ymd_and_hms(2026, 7, 21, 18, 0, 0).unwrap(),
                "fixture.json",
            )
            .unwrap();
        let mut expected = scope();
        expected.team_ids = vec!["NYR".to_string()];
        let resolved = store
            .resolve_team_season_scenario("nyr-development-variance", &expected)
            .unwrap();
        assert_eq!(resolved.scenario, scenario(2.5));

        expected.season = 20252026;
        assert!(matches!(
            store.resolve_team_season_scenario("nyr-development-variance", &expected),
            Err(ScenarioRegistryStoreError::Contract(
                ScenarioRegistryContractError::ScopeMismatch("season")
            ))
        ));

        std::fs::write(
            store.content_path(&result.entry.content_sha256),
            serde_json::to_vec_pretty(&scenario(3.0)).unwrap(),
        )
        .unwrap();
        expected = scope();
        assert!(matches!(
            store.resolve_team_season_scenario("nyr-development-variance", &expected),
            Err(ScenarioRegistryStoreError::IntegrityMismatch { .. })
        ));
    }
}
