//! Requested-scope source-byte acquisition.

use crate::{NhlApiClient, SourcePackageStore, SourcePackageStoreError};
use chrono::{DateTime, Utc};
use icelines_core::source_facts::{ContentHash, OrganizationId, SourceObjectState, SourceUrl};
use icelines_sources::prospect_population::PopulationObjectResult;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceAcquisitionRequest {
    /// Unique physical acquisition member.
    pub object_id: String,
    /// Logical manifest cell proven only when every member succeeds.
    pub coverage_object_id: String,
    pub source_family: String,
    pub organization: Option<OrganizationId>,
    pub source_url: SourceUrl,
    pub terminal_pagination: bool,
    /// Provider-neutral catalog variant passed to the selected adapter.
    pub adapter_variant: Option<String>,
}

impl SourceAcquisitionRequest {
    pub fn new(
        object_id: impl Into<String>,
        source_family: impl Into<String>,
        organization: Option<OrganizationId>,
        source_url: SourceUrl,
        terminal_pagination: bool,
    ) -> Result<Self, SourceAcquisitionError> {
        let object_id = object_id.into();
        let source_family = source_family.into();
        if object_id.trim().is_empty() {
            return Err(SourceAcquisitionError::InvalidRequest(
                "object_id must not be empty".to_owned(),
            ));
        }
        if source_family.trim().is_empty() {
            return Err(SourceAcquisitionError::InvalidRequest(
                "source_family must not be empty".to_owned(),
            ));
        }
        Ok(Self {
            coverage_object_id: object_id.clone(),
            object_id,
            source_family,
            organization,
            source_url,
            terminal_pagination,
            adapter_variant: None,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_member(
        object_id: impl Into<String>,
        coverage_object_id: impl Into<String>,
        source_family: impl Into<String>,
        organization: Option<OrganizationId>,
        source_url: SourceUrl,
        terminal_pagination: bool,
        adapter_variant: impl Into<String>,
    ) -> Result<Self, SourceAcquisitionError> {
        let mut request = Self::new(
            object_id,
            source_family,
            organization,
            source_url,
            terminal_pagination,
        )?;
        request.coverage_object_id = coverage_object_id.into();
        request.adapter_variant = Some(adapter_variant.into());
        if request.coverage_object_id.trim().is_empty()
            || request
                .adapter_variant
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
        {
            return Err(SourceAcquisitionError::InvalidRequest(
                "coverage_object_id and adapter_variant must not be empty".to_owned(),
            ));
        }
        Ok(request)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcquiredSourceObject {
    pub request: SourceAcquisitionRequest,
    pub captured_at: DateTime<Utc>,
    pub content_hash: ContentHash,
    pub byte_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceAcquisitionReport {
    pub acquired: Vec<AcquiredSourceObject>,
    pub results: BTreeMap<String, PopulationObjectResult>,
}

#[derive(Debug, thiserror::Error)]
pub enum SourceAcquisitionError {
    #[error("invalid source acquisition request: {0}")]
    InvalidRequest(String),
    #[error("duplicate source object request {0}")]
    DuplicateObject(String),
    #[error("source capture storage failed: {0}")]
    Store(#[from] SourcePackageStoreError),
}

pub async fn acquire_source_objects(
    client: &NhlApiClient,
    store: &SourcePackageStore,
    captured_at: DateTime<Utc>,
    requests: Vec<SourceAcquisitionRequest>,
) -> Result<SourceAcquisitionReport, SourceAcquisitionError> {
    let mut object_ids = BTreeSet::new();
    for request in &requests {
        if !object_ids.insert(request.object_id.as_str()) {
            return Err(SourceAcquisitionError::DuplicateObject(
                request.object_id.clone(),
            ));
        }
    }
    let mut acquired = Vec::new();
    let mut member_results = BTreeMap::<String, Vec<(String, PopulationObjectResult)>>::new();
    let mut fetched_urls = BTreeMap::<String, CachedFetch>::new();
    for request in requests {
        let url = request.source_url.as_str().to_owned();
        let fetched = if let Some(cached) = fetched_urls.get(&url) {
            cached.clone()
        } else {
            let fetched = match client.fetch_source_bytes(&url).await {
                Ok(bytes) => CachedFetch::Acquired {
                    content_hash: store.store_capture(&bytes)?,
                    byte_count: bytes.len(),
                },
                Err(error) => CachedFetch::Failed(error.to_string()),
            };
            fetched_urls.insert(url, fetched.clone());
            fetched
        };
        match fetched {
            CachedFetch::Acquired {
                content_hash,
                byte_count,
            } => {
                member_results
                    .entry(request.coverage_object_id.clone())
                    .or_default()
                    .push((
                        request.object_id.clone(),
                        PopulationObjectResult {
                            terminal_pagination: request.terminal_pagination,
                            state: if request.terminal_pagination {
                                SourceObjectState::Acquired { records: 1 }
                            } else {
                                SourceObjectState::IncompletePagination
                            },
                        },
                    ));
                acquired.push(AcquiredSourceObject {
                    request,
                    captured_at,
                    content_hash,
                    byte_count,
                });
            }
            CachedFetch::Failed(reason) => {
                member_results
                    .entry(request.coverage_object_id)
                    .or_default()
                    .push((
                        request.object_id,
                        PopulationObjectResult {
                            terminal_pagination: false,
                            state: SourceObjectState::Failed { reason },
                        },
                    ));
            }
        }
    }
    let results = member_results
        .into_iter()
        .map(|(coverage_object_id, members)| {
            let result = aggregate_members(&members);
            (coverage_object_id, result)
        })
        .collect();
    Ok(SourceAcquisitionReport { acquired, results })
}

fn aggregate_members(members: &[(String, PopulationObjectResult)]) -> PopulationObjectResult {
    if let Some((member, reason)) =
        members
            .iter()
            .find_map(|(member, result)| match &result.state {
                SourceObjectState::Failed { reason } => Some((member, reason)),
                _ => None,
            })
    {
        return PopulationObjectResult {
            terminal_pagination: false,
            state: SourceObjectState::Failed {
                reason: format!("acquisition member {member} failed: {reason}"),
            },
        };
    }
    if members.iter().any(|(_, result)| {
        !result.terminal_pagination
            || matches!(result.state, SourceObjectState::IncompletePagination)
    }) {
        return PopulationObjectResult {
            terminal_pagination: false,
            state: SourceObjectState::IncompletePagination,
        };
    }
    PopulationObjectResult {
        terminal_pagination: true,
        state: SourceObjectState::Acquired {
            records: members
                .iter()
                .map(|(_, result)| match result.state {
                    SourceObjectState::Acquired { records } => records,
                    _ => 0,
                })
                .sum(),
        },
    }
}

#[derive(Debug, Clone)]
enum CachedFetch {
    Acquired {
        content_hash: ContentHash,
        byte_count: usize,
    },
    Failed(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use httpmock::prelude::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn acquisition_stores_exact_bytes_and_keeps_failures_visible() {
        let server = MockServer::start();
        let success = server.mock(|when, then| {
            when.method(GET).path("/draft/2026");
            then.status(200)
                .header("content-type", "application/json")
                .body("{\"state\":\"over\"}");
        });
        let missing = server.mock(|when, then| {
            when.method(GET).path("/camp/SEA");
            then.status(404);
        });
        let directory = TempDir::new().unwrap();
        let store = SourcePackageStore::new(directory.path());
        let client =
            NhlApiClient::new(server.base_url(), server.base_url()).with_retry_params(0, 0, 0);
        let report = acquire_source_objects(
            &client,
            &store,
            Utc.with_ymd_and_hms(2026, 7, 31, 12, 0, 0)
                .single()
                .unwrap(),
            vec![
                SourceAcquisitionRequest::new(
                    "SEA:nhl_draft",
                    "nhl_draft",
                    Some(OrganizationId::try_new("SEA").unwrap()),
                    SourceUrl::try_new(server.url("/draft/2026")).unwrap(),
                    true,
                )
                .unwrap(),
                SourceAcquisitionRequest::new(
                    "SEA:nhl_club_camp_publication",
                    "nhl_club_camp_publication",
                    Some(OrganizationId::try_new("SEA").unwrap()),
                    SourceUrl::try_new(server.url("/camp/SEA")).unwrap(),
                    true,
                )
                .unwrap(),
                SourceAcquisitionRequest::new(
                    "NYR:nhl_draft",
                    "nhl_draft",
                    Some(OrganizationId::try_new("NYR").unwrap()),
                    SourceUrl::try_new(server.url("/draft/2026")).unwrap(),
                    true,
                )
                .unwrap(),
            ],
        )
        .await
        .unwrap();

        success.assert_hits(1);
        missing.assert();
        assert_eq!(report.acquired.len(), 2);
        assert_eq!(
            report.acquired[0].content_hash,
            report.acquired[1].content_hash
        );
        assert_eq!(
            store
                .read_capture(&report.acquired[0].content_hash)
                .unwrap(),
            b"{\"state\":\"over\"}"
        );
        assert!(matches!(
            report.results["SEA:nhl_draft"].state,
            SourceObjectState::Acquired { records: 1 }
        ));
        assert!(matches!(
            report.results["SEA:nhl_club_camp_publication"].state,
            SourceObjectState::Failed { .. }
        ));
    }

    #[tokio::test]
    async fn nonterminal_success_is_an_incomplete_object_not_acquired_complete() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/page/1");
            then.status(200).body("partial");
        });
        let directory = TempDir::new().unwrap();
        let report = acquire_source_objects(
            &NhlApiClient::new(server.base_url(), server.base_url()).with_retry_params(0, 0, 0),
            &SourcePackageStore::new(directory.path()),
            Utc.with_ymd_and_hms(2026, 7, 31, 12, 0, 0)
                .single()
                .unwrap(),
            vec![SourceAcquisitionRequest::new(
                "SEA:page",
                "paged_source",
                Some(OrganizationId::try_new("SEA").unwrap()),
                SourceUrl::try_new(server.url("/page/1")).unwrap(),
                false,
            )
            .unwrap()],
        )
        .await
        .unwrap();
        assert!(matches!(
            report.results["SEA:page"].state,
            SourceObjectState::IncompletePagination
        ));
    }

    #[tokio::test]
    async fn grouped_members_fail_the_logical_cell_when_any_member_fails() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/draft/2025");
            then.status(200).body("year-2025");
        });
        server.mock(|when, then| {
            when.method(GET).path("/draft/2026");
            then.status(503);
        });
        let directory = TempDir::new().unwrap();
        let report = acquire_source_objects(
            &NhlApiClient::new(server.base_url(), server.base_url()).with_retry_params(0, 0, 0),
            &SourcePackageStore::new(directory.path()),
            Utc.with_ymd_and_hms(2026, 7, 31, 12, 0, 0)
                .single()
                .unwrap(),
            ["2025", "2026"]
                .into_iter()
                .map(|year| {
                    SourceAcquisitionRequest::new_member(
                        format!("SEA:nhl_draft@{year}"),
                        "SEA:nhl_draft",
                        "nhl_draft",
                        Some(OrganizationId::try_new("SEA").unwrap()),
                        SourceUrl::try_new(server.url(format!("/draft/{year}"))).unwrap(),
                        true,
                        year,
                    )
                    .unwrap()
                })
                .collect(),
        )
        .await
        .unwrap();

        assert_eq!(report.acquired.len(), 1);
        assert_eq!(report.results.len(), 1);
        assert!(matches!(
            &report.results["SEA:nhl_draft"].state,
            SourceObjectState::Failed { reason } if reason.contains("@2026")
        ));
    }
}
