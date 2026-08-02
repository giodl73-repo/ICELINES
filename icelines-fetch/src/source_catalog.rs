//! Versioned, data-driven expansion of prospect-population source objects.

use crate::source_acquisition::SourceAcquisitionRequest;
use icelines_core::source_facts::{OrganizationId, SourceUrl};
use icelines_sources::prospect_population::{
    ProspectPopulationScope, ProspectPopulationSourceFamily,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const PROSPECT_SOURCE_CATALOG_SCHEMA: &str = "icelines_prospect_source_catalog.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProspectSourceCatalog {
    pub schema: String,
    pub season: u32,
    pub catalog_version: String,
    #[serde(default)]
    pub organization_aliases: BTreeMap<String, String>,
    /// Reusable physical members for a logical source-family coverage cell.
    /// Templates opt in with a `{variant}` URL placeholder.
    #[serde(default)]
    pub template_variants: BTreeMap<String, Vec<String>>,
    pub templates: Vec<ProspectSourceCatalogTemplate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProspectSourceCatalogTemplate {
    pub source_family: String,
    pub target: ProspectSourceCatalogTarget,
    pub url: String,
    pub terminal_pagination: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "scope", rename_all = "snake_case")]
pub enum ProspectSourceCatalogTarget {
    AllOrganizations,
    Organizations { organizations: Vec<String> },
}

#[derive(Debug, thiserror::Error)]
pub enum ProspectSourceCatalogError {
    #[error("unsupported prospect source catalog schema {0}")]
    UnsupportedSchema(String),
    #[error("invalid prospect source catalog: {0}")]
    Invalid(String),
    #[error("duplicate expanded source object {0}")]
    DuplicateObject(String),
}

impl ProspectSourceCatalog {
    pub fn expand(
        &self,
        organizations: &[OrganizationId],
        families: &[ProspectPopulationSourceFamily],
    ) -> Result<Vec<SourceAcquisitionRequest>, ProspectSourceCatalogError> {
        if self.schema != PROSPECT_SOURCE_CATALOG_SCHEMA {
            return Err(ProspectSourceCatalogError::UnsupportedSchema(
                self.schema.clone(),
            ));
        }
        if self.catalog_version.trim().is_empty() || self.templates.is_empty() {
            return Err(ProspectSourceCatalogError::Invalid(
                "catalog_version and templates must not be empty".to_owned(),
            ));
        }
        let organization_map = organizations
            .iter()
            .map(|organization| (organization.as_str(), organization.clone()))
            .collect::<BTreeMap<_, _>>();
        let family_map = families
            .iter()
            .map(|family| (family.key(), *family))
            .collect::<BTreeMap<_, _>>();
        for (name, abbreviation) in &self.organization_aliases {
            if name.trim().is_empty()
                || !organization_map.contains_key(abbreviation.trim().to_ascii_uppercase().as_str())
            {
                return Err(ProspectSourceCatalogError::Invalid(format!(
                    "organization alias {name:?} points outside requested scope: {abbreviation}"
                )));
            }
        }
        let mut object_ids = BTreeSet::new();
        let mut requests = Vec::new();
        for template in &self.templates {
            let family = family_map
                .get(template.source_family.as_str())
                .ok_or_else(|| {
                    ProspectSourceCatalogError::Invalid(format!(
                        "unknown or unrequested source family {}",
                        template.source_family
                    ))
                })?;
            let targets = match &template.target {
                ProspectSourceCatalogTarget::AllOrganizations => organizations.to_vec(),
                ProspectSourceCatalogTarget::Organizations {
                    organizations: selected,
                } => {
                    let mut seen = BTreeSet::new();
                    selected
                        .iter()
                        .map(|value| {
                            let normalized = value.trim().to_ascii_uppercase();
                            if !seen.insert(normalized.clone()) {
                                return Err(ProspectSourceCatalogError::Invalid(format!(
                                    "duplicate template organization {normalized}"
                                )));
                            }
                            organization_map
                                .get(normalized.as_str())
                                .cloned()
                                .ok_or_else(|| {
                                    ProspectSourceCatalogError::Invalid(format!(
                                        "organization {normalized} is outside requested scope"
                                    ))
                                })
                        })
                        .collect::<Result<Vec<_>, _>>()?
                }
            };
            for organization in targets {
                let object_id = ProspectPopulationScope::object_id(&organization, family.key());
                if !object_ids.insert(object_id.clone()) {
                    return Err(ProspectSourceCatalogError::DuplicateObject(object_id));
                }
                let variants = self.template_variants.get(family.key());
                match (template.url.contains("{variant}"), variants) {
                    (true, Some(variants)) if !variants.is_empty() => {
                        let mut seen = BTreeSet::new();
                        for variant in variants {
                            let variant = variant.trim();
                            if variant.is_empty()
                                || variant.contains(['{', '}'])
                                || !seen.insert(variant)
                            {
                                return Err(ProspectSourceCatalogError::Invalid(format!(
                                    "invalid or duplicate {} template variant {variant:?}",
                                    family.key()
                                )));
                            }
                            let expanded_url = expand_url(template, &organization, Some(variant))?;
                            requests.push(
                                SourceAcquisitionRequest::new_member(
                                    format!("{object_id}@{variant}"),
                                    object_id.clone(),
                                    family.key(),
                                    Some(organization.clone()),
                                    expanded_url,
                                    template.terminal_pagination,
                                    variant,
                                )
                                .map_err(|error| {
                                    ProspectSourceCatalogError::Invalid(error.to_string())
                                })?,
                            );
                        }
                    }
                    (true, _) => {
                        return Err(ProspectSourceCatalogError::Invalid(format!(
                            "{} template requires non-empty template_variants",
                            family.key()
                        )));
                    }
                    (false, Some(variants)) if !variants.is_empty() => {
                        return Err(ProspectSourceCatalogError::Invalid(format!(
                            "{} has variants but its URL lacks {{variant}}",
                            family.key()
                        )));
                    }
                    (false, _) => requests.push(
                        SourceAcquisitionRequest::new(
                            object_id,
                            family.key(),
                            Some(organization.clone()),
                            expand_url(template, &organization, None)?,
                            template.terminal_pagination,
                        )
                        .map_err(|error| ProspectSourceCatalogError::Invalid(error.to_string()))?,
                    ),
                }
            }
        }
        requests.sort_by(|left, right| left.object_id.cmp(&right.object_id));
        Ok(requests)
    }
}

fn expand_url(
    template: &ProspectSourceCatalogTemplate,
    organization: &OrganizationId,
    variant: Option<&str>,
) -> Result<SourceUrl, ProspectSourceCatalogError> {
    let mut expanded = template
        .url
        .replace("{organization}", organization.as_str());
    if let Some(variant) = variant {
        expanded = expanded.replace("{variant}", variant);
    }
    if expanded.contains('{') || expanded.contains('}') {
        return Err(ProspectSourceCatalogError::Invalid(format!(
            "unresolved URL template in {expanded}"
        )));
    }
    SourceUrl::try_new(expanded)
        .map_err(|error| ProspectSourceCatalogError::Invalid(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn organizations() -> Vec<OrganizationId> {
        ["NYR", "SEA"]
            .into_iter()
            .map(|value| OrganizationId::try_new(value).unwrap())
            .collect()
    }

    #[test]
    fn shared_and_templated_sources_expand_without_team_branches() {
        let catalog = ProspectSourceCatalog {
            schema: PROSPECT_SOURCE_CATALOG_SCHEMA.to_owned(),
            season: 20_262_027,
            catalog_version: "fixture.v1".to_owned(),
            organization_aliases: BTreeMap::new(),
            template_variants: BTreeMap::new(),
            templates: vec![
                ProspectSourceCatalogTemplate {
                    source_family: "nhl_draft".to_owned(),
                    target: ProspectSourceCatalogTarget::AllOrganizations,
                    url: "https://api.example/draft/2026".to_owned(),
                    terminal_pagination: true,
                },
                ProspectSourceCatalogTemplate {
                    source_family: "nhl_current_assignment".to_owned(),
                    target: ProspectSourceCatalogTarget::AllOrganizations,
                    url: "https://api.example/roster/{organization}/current".to_owned(),
                    terminal_pagination: true,
                },
            ],
        };
        let requests = catalog
            .expand(
                &organizations(),
                &[
                    ProspectPopulationSourceFamily::Draft,
                    ProspectPopulationSourceFamily::CurrentNhlAssignment,
                ],
            )
            .unwrap();
        assert_eq!(requests.len(), 4);
        assert_eq!(requests[0].object_id, "NYR:nhl_current_assignment");
        assert_eq!(
            requests[0].source_url.as_str(),
            "https://api.example/roster/NYR/current"
        );
        assert_eq!(requests[1].source_url, requests[3].source_url);
    }

    #[test]
    fn duplicate_expansion_fails_closed() {
        let template = ProspectSourceCatalogTemplate {
            source_family: "nhl_draft".to_owned(),
            target: ProspectSourceCatalogTarget::AllOrganizations,
            url: "https://api.example/draft/2026".to_owned(),
            terminal_pagination: true,
        };
        let catalog = ProspectSourceCatalog {
            schema: PROSPECT_SOURCE_CATALOG_SCHEMA.to_owned(),
            season: 20_262_027,
            catalog_version: "fixture.v1".to_owned(),
            organization_aliases: BTreeMap::new(),
            template_variants: BTreeMap::new(),
            templates: vec![template.clone(), template],
        };
        assert!(matches!(
            catalog.expand(&organizations(), &[ProspectPopulationSourceFamily::Draft]),
            Err(ProspectSourceCatalogError::DuplicateObject(_))
        ));
    }

    #[test]
    fn variants_fan_in_to_one_logical_coverage_object() {
        let catalog = ProspectSourceCatalog {
            schema: PROSPECT_SOURCE_CATALOG_SCHEMA.to_owned(),
            season: 20_262_027,
            catalog_version: "fixture.v1".to_owned(),
            organization_aliases: BTreeMap::new(),
            template_variants: BTreeMap::from([(
                "nhl_draft".to_owned(),
                vec!["2025".to_owned(), "2026".to_owned()],
            )]),
            templates: vec![ProspectSourceCatalogTemplate {
                source_family: "nhl_draft".to_owned(),
                target: ProspectSourceCatalogTarget::Organizations {
                    organizations: vec!["SEA".to_owned()],
                },
                url: "https://api.example/draft/{variant}".to_owned(),
                terminal_pagination: true,
            }],
        };
        let requests = catalog
            .expand(&organizations(), &[ProspectPopulationSourceFamily::Draft])
            .unwrap();

        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].object_id, "SEA:nhl_draft@2025");
        assert_eq!(requests[1].adapter_variant.as_deref(), Some("2026"));
        assert!(requests
            .iter()
            .all(|request| request.coverage_object_id == "SEA:nhl_draft"));
    }

    #[test]
    fn checked_2026_27_catalog_expands_to_three_real_families_for_all_32() {
        let catalog: ProspectSourceCatalog = serde_json::from_str(include_str!(
            "../../design/data/prospect-source-catalog-2026-27.v1.json"
        ))
        .unwrap();
        let organizations = crate::teams::nhl_teams_for_season("20262027")
            .into_iter()
            .map(|value| OrganizationId::try_new(value).unwrap())
            .collect::<Vec<_>>();
        let requests = catalog
            .expand(
                &organizations,
                &[
                    ProspectPopulationSourceFamily::Draft,
                    ProspectPopulationSourceFamily::TransactionPublication,
                    ProspectPopulationSourceFamily::CurrentNhlAssignment,
                ],
            )
            .unwrap();
        assert_eq!(organizations.len(), 32);
        assert_eq!(requests.len(), 352);
        assert_eq!(
            requests
                .iter()
                .map(|request| request.source_url.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            42,
            "nine draft ledgers and one trade ledger are shared; rosters are organization-specific"
        );
    }
}
