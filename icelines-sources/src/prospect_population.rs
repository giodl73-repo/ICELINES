//! Requested-scope construction for the provider-neutral prospect population.
//!
//! Scope expansion is data-driven. Teams with no acquired records remain in
//! the manifest, and a missing acquisition is never converted to an empty
//! candidate population.

use icelines_core::source_facts::{
    OrganizationId, SourceContractError, SourceObjectOutcome, SourceObjectState, SourceRunManifest,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProspectPopulationSourceFamily {
    Draft,
    CampPublication,
    ContractPublication,
    TransactionPublication,
    CurrentNhlAssignment,
    CurrentAhlAssignment,
    NhlPlayerLanding,
}

impl ProspectPopulationSourceFamily {
    pub fn key(self) -> &'static str {
        match self {
            Self::Draft => "nhl_draft",
            Self::CampPublication => "nhl_club_camp_publication",
            Self::ContractPublication => "nhl_contract_publication",
            Self::TransactionPublication => "nhl_transaction_publication",
            Self::CurrentNhlAssignment => "nhl_current_assignment",
            Self::CurrentAhlAssignment => "ahl_current_assignment",
            Self::NhlPlayerLanding => "nhl_player_landing",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopulationObjectResult {
    pub terminal_pagination: bool,
    pub state: SourceObjectState,
}

#[derive(Debug, Clone)]
pub struct ProspectPopulationScope {
    organizations: Vec<OrganizationId>,
    families: Vec<ProspectPopulationSourceFamily>,
    source_catalog_version: String,
}

impl ProspectPopulationScope {
    pub fn new(
        organizations: Vec<OrganizationId>,
        families: Vec<ProspectPopulationSourceFamily>,
        source_catalog_version: impl Into<String>,
    ) -> Result<Self, SourceContractError> {
        if organizations.is_empty() {
            return Err(SourceContractError::Empty("population_organizations"));
        }
        if families.is_empty() {
            return Err(SourceContractError::Empty("population_source_families"));
        }
        let source_catalog_version = source_catalog_version.into();
        if source_catalog_version.trim().is_empty() {
            return Err(SourceContractError::Empty("source_catalog_version"));
        }
        let mut organization_keys = BTreeSet::new();
        for organization in &organizations {
            if !organization_keys.insert(organization.as_str()) {
                return Err(SourceContractError::DuplicateId {
                    kind: "population_organization",
                    id: organization.to_string(),
                });
            }
        }
        let mut family_keys = BTreeSet::new();
        for family in &families {
            if !family_keys.insert(family.key()) {
                return Err(SourceContractError::DuplicateId {
                    kind: "population_source_family",
                    id: family.key().to_owned(),
                });
            }
        }
        Ok(Self {
            organizations,
            families,
            source_catalog_version,
        })
    }

    pub fn object_id(organization: &OrganizationId, family: &str) -> String {
        format!("{}:{family}", organization.as_str())
    }

    pub fn build_manifest(
        &self,
        results: &BTreeMap<String, PopulationObjectResult>,
    ) -> Result<SourceRunManifest, SourceContractError> {
        let expected_ids = self
            .organizations
            .iter()
            .flat_map(|organization| {
                self.families
                    .iter()
                    .map(move |family| Self::object_id(organization, family.key()))
            })
            .collect::<BTreeSet<_>>();
        if let Some(extra) = results.keys().find(|key| !expected_ids.contains(*key)) {
            return Err(SourceContractError::InvalidCoverage(format!(
                "population result {extra} is outside requested scope"
            )));
        }
        let mut objects = Vec::with_capacity(expected_ids.len());
        for organization in &self.organizations {
            for family in &self.families {
                let object_id = Self::object_id(organization, family.key());
                let result =
                    results
                        .get(&object_id)
                        .cloned()
                        .unwrap_or_else(|| PopulationObjectResult {
                            terminal_pagination: false,
                            state: SourceObjectState::Failed {
                                reason: "requested source object was not acquired".to_owned(),
                            },
                        });
                objects.push(SourceObjectOutcome {
                    object_id,
                    source_family: family.key().to_owned(),
                    organization: Some(organization.clone()),
                    terminal_pagination: result.terminal_pagination,
                    state: result.state,
                });
            }
        }
        let complete = objects.iter().all(|object| {
            matches!(
                object.state,
                SourceObjectState::Acquired { .. } | SourceObjectState::NotApplicable { .. }
            ) && (object.terminal_pagination
                || matches!(object.state, SourceObjectState::NotApplicable { .. }))
        });
        let manifest = SourceRunManifest {
            requested_scope: "all_season_canonical_organizations".to_owned(),
            source_catalog_version: self.source_catalog_version.clone(),
            objects,
            complete,
        };
        manifest.validate()?;
        Ok(manifest)
    }
}
