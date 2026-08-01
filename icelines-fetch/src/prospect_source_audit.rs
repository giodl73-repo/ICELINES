//! Live/cached all-organization prospect source audit and package sealing.

use crate::ahl::{AhlIdentityReviewDecisions, AhlRosterStatsSnapshot};
use crate::{
    acquire_source_objects, build_source_package, nhl_teams_for_season, NhlApiClient,
    ProspectSourceCatalog, ProspectSourceCatalogError, SourceAcquisitionError,
    SourcePackageBuildInput, SourcePackageStore, SourcePackageStoreError,
};
use chrono::{DateTime, Utc};
use icelines_core::identity::PlayerId;
use icelines_core::model::Season;
use icelines_core::source_facts::{
    AdapterVersion, ContentHash, DecisionId, FactSubject, FreshnessClass, FreshnessStatus,
    IdentityReviewAction, IdentityReviewDecision, OrganizationId, PackageId,
    PlayerOrganizationEvent, PolicyVersion, SourceContractError, SourceCoverageBucket,
    SourceDisclosure, SourceDisclosureCode, SourceEvidence, SourceFact, SourceFreshness,
    SourceInputRecord, SourceObjectState, SourcePackage, SourceRunManifest,
};
use icelines_core::CANONICAL_TEAMS;
use icelines_sources::ahl::roster_stats::{
    AhlRosterStatsOutput, AhlRosterStatsV1Adapter, AHL_PROVIDER as SOURCE_AHL_PROVIDER,
};
use icelines_sources::compat::ahl_identity_review_v1::AhlIdentityReviewV1Adapter;
use icelines_sources::fragment::SourcePackageFragment;
use icelines_sources::identity_review::IdentityReviewLedgerV1Adapter;
use icelines_sources::nhl::camp_participation::CampParticipationLedgerV1Adapter;
use icelines_sources::nhl::contract_control::ContractControlLedgerV1Adapter;
use icelines_sources::nhl::draft_picks::OfficialNhlDraftPicksAdapter;
use icelines_sources::nhl::player_landing::OfficialNhlDraftAdapter;
use icelines_sources::nhl::roster::OfficialNhlRosterAdapter;
use icelines_sources::nhl::trade_tracker::NhlTradeTrackerAdapter;
use icelines_sources::prospect_population::{
    PopulationObjectResult, ProspectPopulationScope, ProspectPopulationSourceFamily,
};
use icelines_sources::{SourceAdapter, SourceInput};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
pub struct ProspectSourceAuditInput {
    pub captured_at: DateTime<Utc>,
    pub effective_cutoff: DateTime<Utc>,
    pub knowledge_cutoff: DateTime<Utc>,
    /// Live runs close both cutoffs after every requested acquisition. Explicit
    /// historical/replay runs leave this false and retain caller cutoffs.
    pub finalize_cutoffs_after_acquisition: bool,
}

/// Optional reviewed-source artifacts acquired by an existing IceLines source
/// client rather than through the URL catalog. Raw bytes are retained in the
/// same content-addressed store and still pass through an `icelines-sources`
/// adapter before entering the package.
#[derive(Debug, Clone, Default)]
pub struct ProspectSourceAuditArtifacts {
    pub ahl_roster_snapshot: Option<Vec<u8>>,
    pub ahl_identity_reviews: Vec<Vec<u8>>,
    pub ahl_review_registry_url: Option<String>,
    /// FLETCH cache root used to acquire current-roster player landing bytes.
    pub roster_player_landing_cache_root: Option<std::path::PathBuf>,
    pub contract_control_ledger: Option<Vec<u8>>,
    pub camp_participation_ledger: Option<Vec<u8>>,
    pub identity_review_ledgers: Vec<Vec<u8>>,
}

#[derive(Debug, thiserror::Error)]
pub enum ProspectSourceAuditError {
    #[error("source catalog failed: {0}")]
    Catalog(#[from] ProspectSourceCatalogError),
    #[error("source acquisition failed: {0}")]
    Acquisition(#[from] SourceAcquisitionError),
    #[error("source package contract failed: {0}")]
    Contract(#[from] SourceContractError),
    #[error("source package storage failed: {0}")]
    Store(#[from] SourcePackageStoreError),
    #[error("source adapter failed: {0}")]
    Adapter(String),
    #[error("invalid audit configuration: {0}")]
    Invalid(String),
}

pub async fn run_prospect_source_audit(
    client: &NhlApiClient,
    store: &SourcePackageStore,
    catalog: &ProspectSourceCatalog,
    input: ProspectSourceAuditInput,
) -> Result<SourcePackage, ProspectSourceAuditError> {
    run_prospect_source_audit_with_artifacts(
        client,
        store,
        catalog,
        input,
        ProspectSourceAuditArtifacts::default(),
    )
    .await
}

pub async fn run_prospect_source_audit_with_artifacts(
    client: &NhlApiClient,
    store: &SourcePackageStore,
    catalog: &ProspectSourceCatalog,
    input: ProspectSourceAuditInput,
    artifacts: ProspectSourceAuditArtifacts,
) -> Result<SourcePackage, ProspectSourceAuditError> {
    let season_text = catalog.season.to_string();
    let organizations = nhl_teams_for_season(&season_text)
        .into_iter()
        .map(OrganizationId::try_new)
        .collect::<Result<Vec<_>, _>>()?;
    run_for_organizations(client, store, catalog, input, organizations, artifacts).await
}

async fn run_for_organizations(
    client: &NhlApiClient,
    store: &SourcePackageStore,
    catalog: &ProspectSourceCatalog,
    input: ProspectSourceAuditInput,
    organizations: Vec<OrganizationId>,
    artifacts: ProspectSourceAuditArtifacts,
) -> Result<SourcePackage, ProspectSourceAuditError> {
    if input.effective_cutoff > input.knowledge_cutoff || input.captured_at > input.knowledge_cutoff
    {
        return Err(ProspectSourceAuditError::Invalid(
            "capture/effective cutoffs must not exceed knowledge cutoff".to_owned(),
        ));
    }
    let families = all_families();
    let scope = ProspectPopulationScope::new(
        organizations.clone(),
        families.clone(),
        catalog.catalog_version.clone(),
    )?;
    let requests = catalog.expand(&organizations, &families)?;
    let mut acquisition =
        acquire_source_objects(client, store, input.captured_at, requests).await?;
    // Acquisition counts physical objects. Adapter parsing replaces them with
    // logical record counts and may add several catalog members into one cell.
    for result in acquisition.results.values_mut() {
        if matches!(result.state, SourceObjectState::Acquired { .. }) {
            result.state = SourceObjectState::Acquired { records: 0 };
        }
    }
    let mut fragments = Vec::new();
    let mut package_inputs = Vec::new();
    let mut identity_review_decisions = Vec::new();
    let mut parsed_shared = BTreeSet::new();
    let mut roster_players = BTreeMap::<PlayerId, OrganizationId>::new();

    for acquired in &acquisition.acquired {
        let family = acquired.request.source_family.as_str();
        if matches!(family, "nhl_draft" | "nhl_transaction_publication")
            && !parsed_shared.insert((family.to_owned(), acquired.content_hash.clone()))
        {
            continue;
        }
        let bytes = store.read_capture(&acquired.content_hash)?;
        let parsed = match family {
            "nhl_draft" => parse_draft(
                catalog,
                acquired,
                &bytes,
                &organizations,
                &mut acquisition.results,
            ),
            "nhl_transaction_publication" => parse_trades(
                catalog,
                acquired,
                &bytes,
                &organizations,
                &mut acquisition.results,
            ),
            "nhl_current_assignment" => parse_roster(acquired, &bytes, &mut acquisition.results),
            _ => continue,
        };
        match parsed {
            Ok((fragment, evidence, class)) => {
                if family == ProspectPopulationSourceFamily::CurrentNhlAssignment.key() {
                    collect_roster_players(&fragment, &mut roster_players)?;
                }
                fragments.push(fragment);
                package_inputs.push(input_record(evidence, class, input.knowledge_cutoff));
            }
            Err(message) => quarantine_family_or_object(
                family,
                &acquired.request.object_id,
                message,
                &mut acquisition.results,
            ),
        }
    }

    if !artifacts.ahl_identity_reviews.is_empty() && artifacts.ahl_roster_snapshot.is_none() {
        return Err(ProspectSourceAuditError::Invalid(
            "AHL identity reviews require the exact staged --ahl-roster-snapshot".to_owned(),
        ));
    }
    if let Some(bytes) = artifacts.ahl_roster_snapshot.as_deref() {
        let (output, evidence) = parse_ahl_snapshot(
            catalog,
            store,
            bytes,
            &organizations,
            &mut acquisition.results,
        )?;
        fragments.push(SourcePackageFragment::from(&output));
        package_inputs.push(input_record(
            evidence,
            FreshnessClass::Roster,
            input.knowledge_cutoff,
        ));
        let registry_url = artifacts.ahl_review_registry_url.as_deref();
        for review in &artifacts.ahl_identity_reviews {
            let (mut decisions, evidence) = parse_ahl_identity_review(
                catalog,
                store,
                review,
                registry_url.ok_or_else(|| {
                    ProspectSourceAuditError::Invalid(
                        "AHL identity reviews require ahl_review_registry_url".to_owned(),
                    )
                })?,
                &output,
            )?;
            identity_review_decisions.append(&mut decisions);
            package_inputs.push(input_record(
                evidence,
                FreshnessClass::Static,
                input.knowledge_cutoff,
            ));
        }
    }

    if let Some(bytes) = artifacts.contract_control_ledger.as_deref() {
        let (fragment, evidence) = parse_contract_control_ledger(
            catalog,
            store,
            bytes,
            &organizations,
            &mut acquisition.results,
        )?;
        fragments.push(fragment);
        package_inputs.push(input_record(
            evidence,
            FreshnessClass::Transactional,
            input.knowledge_cutoff,
        ));
    }

    if let Some(bytes) = artifacts.camp_participation_ledger.as_deref() {
        let (fragment, evidence) = parse_camp_participation_ledger(
            catalog,
            store,
            bytes,
            &organizations,
            &mut acquisition.results,
        )?;
        fragments.push(fragment);
        package_inputs.push(input_record(
            evidence,
            FreshnessClass::Roster,
            input.knowledge_cutoff,
        ));
    }

    if let Some(cache_root) = artifacts.roster_player_landing_cache_root.as_deref() {
        let landing_knowledge_cutoff = if input.finalize_cutoffs_after_acquisition {
            DateTime::<Utc>::MAX_UTC
        } else {
            input.knowledge_cutoff
        };
        let (fragment, mut inputs) = acquire_roster_player_landings(
            store,
            cache_root,
            &roster_players,
            &organizations,
            landing_knowledge_cutoff,
            &mut acquisition.results,
        )
        .await?;
        fragments.push(fragment);
        package_inputs.append(&mut inputs);
    }

    for bytes in &artifacts.identity_review_ledgers {
        let (mut decisions, evidence) = parse_identity_review_ledger(catalog, store, bytes)?;
        identity_review_decisions.append(&mut decisions);
        package_inputs.push(input_record(
            evidence,
            FreshnessClass::Static,
            input.knowledge_cutoff,
        ));
    }

    let mut exact_draft_decisions = exact_draft_identity_decisions(&fragments)?;
    identity_review_decisions.append(&mut exact_draft_decisions);

    let (effective_cutoff, knowledge_cutoff) = if input.finalize_cutoffs_after_acquisition {
        let completed_at = Utc::now();
        (completed_at, completed_at)
    } else {
        (input.effective_cutoff, input.knowledge_cutoff)
    };
    if input.finalize_cutoffs_after_acquisition {
        for package_input in &mut package_inputs {
            package_input.freshness.evaluated_at = knowledge_cutoff;
        }
    }

    let manifest = scope.build_manifest(&acquisition.results)?;
    let (coverage, disclosures) = coverage_and_disclosures(&manifest);
    let package_id = PackageId::try_new(format!(
        "prospect-sources:{}:{}:{}",
        catalog.season,
        catalog.catalog_version,
        input.captured_at.format("%Y%m%dT%H%M%SZ")
    ))?;
    let package = build_source_package(
        SourcePackageBuildInput {
            package_id,
            evaluation_season: Season(catalog.season),
            effective_cutoff,
            knowledge_cutoff,
            adapter_registry_version: AdapterVersion::try_new("prospect-sources.v2")?,
            reconciliation_policy_version: PolicyVersion::try_new("identity-review.v1")?,
            review_registry_fingerprint: review_registry_fingerprint(&identity_review_decisions)?,
            run_manifest: manifest,
            inputs: package_inputs,
            identity_review_decisions,
            conflicts: Vec::new(),
            coverage,
            disclosures,
        },
        fragments,
    )?;
    store.store_package(&package)?;
    Ok(package)
}

fn parse_ahl_snapshot(
    catalog: &ProspectSourceCatalog,
    store: &SourcePackageStore,
    bytes: &[u8],
    organizations: &[OrganizationId],
    results: &mut BTreeMap<String, PopulationObjectResult>,
) -> Result<(AhlRosterStatsOutput, SourceEvidence), ProspectSourceAuditError> {
    let snapshot: AhlRosterStatsSnapshot = serde_json::from_slice(bytes).map_err(|error| {
        ProspectSourceAuditError::Invalid(format!("invalid AHL snapshot: {error}"))
    })?;
    snapshot
        .validate()
        .map_err(|error| ProspectSourceAuditError::Invalid(error.to_string()))?;
    if snapshot.season != catalog.season || snapshot.provider != SOURCE_AHL_PROVIDER {
        return Err(ProspectSourceAuditError::Invalid(format!(
            "AHL snapshot must match season {} and provider {}",
            catalog.season, SOURCE_AHL_PROVIDER
        )));
    }
    let content_hash = store.store_capture(bytes)?;
    let adapter = AhlRosterStatsV1Adapter;
    let descriptor = adapter.descriptor();
    let output = adapter
        .parse(SourceInput::new(
            bytes,
            descriptor.source_id.clone(),
            content_hash.clone(),
        ))
        .map_err(|error| ProspectSourceAuditError::Adapter(error.to_string()))?;
    let allowed = organizations
        .iter()
        .map(|organization| (organization.as_str(), organization))
        .collect::<BTreeMap<_, _>>();
    let mut affiliated = BTreeSet::new();
    let mut records = BTreeMap::<&str, usize>::new();
    for team in &snapshot.teams {
        let Some(affiliate) = team.nhl_affiliate.as_deref() else {
            continue;
        };
        let normalized = affiliate.trim().to_ascii_uppercase();
        let Some(organization) = allowed.get(normalized.as_str()) else {
            return Err(ProspectSourceAuditError::Invalid(format!(
                "AHL team {} references affiliate {} outside the requested NHL scope",
                team.team_code, affiliate
            )));
        };
        affiliated.insert(organization.as_str());
        *records.entry(organization.as_str()).or_default() += team.roster.len();
    }
    for organization in organizations {
        if affiliated.contains(organization.as_str()) {
            set_records(
                results,
                organization,
                ProspectPopulationSourceFamily::CurrentAhlAssignment.key(),
                records
                    .get(organization.as_str())
                    .copied()
                    .unwrap_or_default(),
            );
        }
    }
    let captured_at = DateTime::parse_from_rfc3339(&snapshot.fetched_at)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| {
            ProspectSourceAuditError::Invalid("AHL snapshot fetched_at must be RFC 3339".to_owned())
        })?;
    let evidence = SourceEvidence::new(
        descriptor.source_id,
        icelines_core::source_facts::SourceUrl::try_new(snapshot.roster_source_url)?,
        descriptor.provider,
        captured_at,
        content_hash,
        descriptor.adapter_version,
    );
    Ok((output, evidence))
}

fn parse_contract_control_ledger(
    catalog: &ProspectSourceCatalog,
    store: &SourcePackageStore,
    bytes: &[u8],
    organizations: &[OrganizationId],
    results: &mut BTreeMap<String, PopulationObjectResult>,
) -> Result<(SourcePackageFragment, SourceEvidence), ProspectSourceAuditError> {
    let content_hash = store.store_capture(bytes)?;
    let adapter = ContractControlLedgerV1Adapter;
    let descriptor = adapter.descriptor();
    let output = adapter
        .parse(SourceInput::new(
            bytes,
            descriptor.source_id.clone(),
            content_hash.clone(),
        ))
        .map_err(|error| ProspectSourceAuditError::Adapter(error.to_string()))?;
    if output.season.0 != catalog.season {
        return Err(ProspectSourceAuditError::Invalid(format!(
            "contract-control ledger season {} does not match catalog {}",
            output.season.0, catalog.season
        )));
    }
    require_exact_organization_coverage(
        "contract-control",
        organizations,
        output.records_by_organization.keys(),
    )?;
    for (organization, records) in &output.records_by_organization {
        set_records(
            results,
            organization,
            ProspectPopulationSourceFamily::ContractPublication.key(),
            *records,
        );
    }
    let evidence = SourceEvidence::new(
        descriptor.source_id,
        output.coverage_source_url.clone(),
        output.provider,
        output.captured_at,
        content_hash,
        descriptor.adapter_version,
    );
    Ok((SourcePackageFragment::from_facts(output.facts), evidence))
}

fn parse_camp_participation_ledger(
    catalog: &ProspectSourceCatalog,
    store: &SourcePackageStore,
    bytes: &[u8],
    organizations: &[OrganizationId],
    results: &mut BTreeMap<String, PopulationObjectResult>,
) -> Result<(SourcePackageFragment, SourceEvidence), ProspectSourceAuditError> {
    let content_hash = store.store_capture(bytes)?;
    let adapter = CampParticipationLedgerV1Adapter;
    let descriptor = adapter.descriptor();
    let output = adapter
        .parse(SourceInput::new(
            bytes,
            descriptor.source_id.clone(),
            content_hash.clone(),
        ))
        .map_err(|error| ProspectSourceAuditError::Adapter(error.to_string()))?;
    if output.season.0 != catalog.season {
        return Err(ProspectSourceAuditError::Invalid(format!(
            "camp-participation ledger season {} does not match catalog {}",
            output.season.0, catalog.season
        )));
    }
    require_exact_organization_coverage(
        "camp-participation",
        organizations,
        output.records_by_organization.keys(),
    )?;
    for (organization, records) in &output.records_by_organization {
        set_records(
            results,
            organization,
            ProspectPopulationSourceFamily::CampPublication.key(),
            *records,
        );
    }
    let evidence = SourceEvidence::new(
        descriptor.source_id,
        output.coverage_source_url.clone(),
        output.provider,
        output.captured_at,
        content_hash,
        descriptor.adapter_version,
    );
    Ok((SourcePackageFragment::from_facts(output.facts), evidence))
}

fn require_exact_organization_coverage<'a>(
    source: &str,
    organizations: &[OrganizationId],
    supplied: impl Iterator<Item = &'a OrganizationId>,
) -> Result<(), ProspectSourceAuditError> {
    let expected = organizations.iter().collect::<BTreeSet<_>>();
    let supplied = supplied.collect::<BTreeSet<_>>();
    if expected == supplied {
        return Ok(());
    }
    let missing = expected
        .difference(&supplied)
        .map(|organization| organization.as_str())
        .collect::<Vec<_>>();
    let extra = supplied
        .difference(&expected)
        .map(|organization| organization.as_str())
        .collect::<Vec<_>>();
    Err(ProspectSourceAuditError::Invalid(format!(
        "{source} coverage must exactly match requested organizations; missing=[{}], extra=[{}]",
        missing.join(","),
        extra.join(",")
    )))
}

fn collect_roster_players(
    fragment: &SourcePackageFragment,
    players: &mut BTreeMap<PlayerId, OrganizationId>,
) -> Result<(), ProspectSourceAuditError> {
    for assertion in &fragment.fact_assertions {
        let FactSubject::Player(player_id) = assertion.subject() else {
            continue;
        };
        let SourceFact::PlayerOrganization(PlayerOrganizationEvent::Assigned { by, .. }) =
            assertion.fact()
        else {
            continue;
        };
        if players
            .insert(*player_id, by.clone())
            .is_some_and(|prior| prior != *by)
        {
            return Err(ProspectSourceAuditError::Invalid(format!(
                "current NHL roster player {} appears for multiple organizations",
                player_id.0
            )));
        }
    }
    Ok(())
}

async fn acquire_roster_player_landings(
    store: &SourcePackageStore,
    cache_root: &std::path::Path,
    roster_players: &BTreeMap<PlayerId, OrganizationId>,
    organizations: &[OrganizationId],
    knowledge_cutoff: DateTime<Utc>,
    results: &mut BTreeMap<String, PopulationObjectResult>,
) -> Result<(SourcePackageFragment, Vec<SourceInputRecord>), ProspectSourceAuditError> {
    let player_ids = roster_players.keys().map(|player_id| player_id.0).collect();
    let bytes_by_player = crate::fletch::fetch_player_landing_batch_bytes_async(
        player_ids,
        crate::fletch::FletchPlayerLandingArtifact::Landing,
        cache_root.to_path_buf(),
        false,
        50,
    )
    .await
    .map_err(|error| {
        ProspectSourceAuditError::Invalid(format!(
            "FLETCH roster player landing acquisition failed: {error:#}"
        ))
    })?;
    let manifest = crate::fletch::read_fletch_cache_manifest(
        &crate::fletch::fletch_cache_manifest_path(cache_root),
    )
    .map_err(|error| {
        ProspectSourceAuditError::Invalid(format!("read FLETCH landing manifest: {error:#}"))
    })?;
    let captured_by_dataset = manifest
        .entries
        .into_iter()
        .filter(|entry| entry.verified)
        .filter_map(|entry| {
            i64::try_from(entry.fetched_at_ms)
                .ok()
                .and_then(DateTime::from_timestamp_millis)
                .map(|captured_at| (entry.dataset_id, captured_at))
        })
        .collect::<BTreeMap<_, _>>();
    let mut facts = Vec::new();
    let mut inputs = Vec::new();
    let mut records = BTreeMap::<String, usize>::new();
    let mut failures = BTreeMap::<String, Vec<String>>::new();
    for (player_id, organization) in roster_players {
        let dataset_id = format!("icelines.player.landing.{}", player_id.0);
        let Some(bytes) = bytes_by_player.get(&player_id.0) else {
            failures
                .entry(organization.as_str().to_owned())
                .or_default()
                .push(format!("{}: missing landing bytes", player_id.0));
            continue;
        };
        let Some(captured_at) = captured_by_dataset.get(&dataset_id).copied() else {
            failures
                .entry(organization.as_str().to_owned())
                .or_default()
                .push(format!(
                    "{}: missing verified capture timestamp",
                    player_id.0
                ));
            continue;
        };
        if captured_at > knowledge_cutoff {
            failures
                .entry(organization.as_str().to_owned())
                .or_default()
                .push(format!(
                    "{}: capture is after knowledge cutoff",
                    player_id.0
                ));
            continue;
        }
        let content_hash = store.store_capture(bytes)?;
        let adapter = OfficialNhlDraftAdapter::new(player_id.0, captured_at)
            .map_err(|error| ProspectSourceAuditError::Invalid(error.to_string()))?;
        let descriptor = adapter.descriptor();
        match adapter.parse(SourceInput::new(
            bytes,
            descriptor.source_id.clone(),
            content_hash.clone(),
        )) {
            Ok(Some(fact)) => facts.push(fact),
            Ok(None) => {}
            Err(error) => {
                failures
                    .entry(organization.as_str().to_owned())
                    .or_default()
                    .push(format!("{}: {error}", player_id.0));
                continue;
            }
        }
        let evidence = SourceEvidence::new(
            descriptor.source_id,
            icelines_core::source_facts::SourceUrl::try_new(format!(
                "https://api-web.nhle.com/v1/player/{}/landing",
                player_id.0
            ))?,
            descriptor.provider,
            captured_at,
            content_hash,
            descriptor.adapter_version,
        );
        inputs.push(input_record(
            evidence,
            FreshnessClass::Static,
            knowledge_cutoff,
        ));
        *records.entry(organization.as_str().to_owned()).or_default() += 1;
    }
    for organization in organizations {
        let object_id = ProspectPopulationScope::object_id(
            organization,
            ProspectPopulationSourceFamily::NhlPlayerLanding.key(),
        );
        if let Some(reasons) = failures.get(organization.as_str()) {
            results.insert(
                object_id,
                PopulationObjectResult {
                    terminal_pagination: false,
                    state: SourceObjectState::Quarantined {
                        reason: format!(
                            "{} roster player landing(s) failed: {}",
                            reasons.len(),
                            reasons
                                .iter()
                                .take(3)
                                .cloned()
                                .collect::<Vec<_>>()
                                .join("; ")
                        ),
                    },
                },
            );
        } else if roster_players
            .values()
            .any(|candidate| candidate == organization)
        {
            set_records(
                results,
                organization,
                ProspectPopulationSourceFamily::NhlPlayerLanding.key(),
                records
                    .get(organization.as_str())
                    .copied()
                    .unwrap_or_default(),
            );
        } else {
            results.insert(
                object_id,
                PopulationObjectResult {
                    terminal_pagination: true,
                    state: SourceObjectState::NotApplicable {
                        reason: "official current NHL roster contains no player identities"
                            .to_owned(),
                    },
                },
            );
        }
    }
    Ok((SourcePackageFragment::from_facts(facts), inputs))
}

type DraftIdentityKey = (String, u16, u8, u16);

fn exact_draft_identity_decisions(
    fragments: &[SourcePackageFragment],
) -> Result<Vec<IdentityReviewDecision>, ProspectSourceAuditError> {
    let mut canonical =
        BTreeMap::<DraftIdentityKey, BTreeMap<PlayerId, Vec<SourceEvidence>>>::new();
    let mut staged = BTreeMap::new();
    for fragment in fragments {
        for assertion in &fragment.fact_assertions {
            let FactSubject::Player(player_id) = assertion.subject() else {
                continue;
            };
            let SourceFact::PlayerOrganization(PlayerOrganizationEvent::Drafted {
                by,
                year,
                round,
                overall,
            }) = assertion.fact()
            else {
                continue;
            };
            canonical
                .entry((by.as_str().to_owned(), *year, *round, *overall))
                .or_default()
                .entry(*player_id)
                .or_default()
                .extend(assertion.evidence().iter().cloned());
        }
        for assertion in &fragment.staged_player_assertions {
            let SourceFact::PlayerOrganization(PlayerOrganizationEvent::Drafted {
                by,
                year,
                round,
                overall,
            }) = assertion.fact()
            else {
                continue;
            };
            let key = (by.as_str().to_owned(), *year, *round, *overall);
            if staged.insert(key.clone(), assertion).is_some() {
                return Err(ProspectSourceAuditError::Invalid(format!(
                    "duplicate staged draft coordinate {key:?}"
                )));
            }
        }
    }
    let mut decisions = Vec::new();
    for (key, players) in canonical {
        let Some(assertion) = staged.get(&key) else {
            continue;
        };
        if players.len() != 1 {
            return Err(ProspectSourceAuditError::Invalid(format!(
                "official landing facts conflict at draft coordinate {key:?}"
            )));
        }
        let (player_id, mut evidence) = players.into_iter().next().expect("one player");
        evidence.extend(assertion.evidence().iter().cloned());
        let reviewed_at = evidence
            .iter()
            .map(SourceEvidence::captured_at)
            .max()
            .expect("draft identity decision has evidence");
        decisions.push(IdentityReviewDecision::new(
            DecisionId::try_new(format!(
                "official-draft-coordinate:{}:{}:{}",
                key.1, key.2, key.3
            ))?,
            assertion.proposal_id().clone(),
            IdentityReviewAction::SetIdentity,
            Some(player_id),
            "icelines:official-draft-coordinate-v1",
            reviewed_at,
            "Canonical NHL player landing draftDetails exactly and uniquely match the official draft ledger year, round, overall pick, and drafting organization.",
            evidence,
        )?);
    }
    Ok(decisions)
}

fn parse_ahl_identity_review(
    catalog: &ProspectSourceCatalog,
    store: &SourcePackageStore,
    bytes: &[u8],
    registry_url: &str,
    roster: &AhlRosterStatsOutput,
) -> Result<
    (
        Vec<icelines_core::source_facts::IdentityReviewDecision>,
        SourceEvidence,
    ),
    ProspectSourceAuditError,
> {
    let review: AhlIdentityReviewDecisions = serde_json::from_slice(bytes).map_err(|error| {
        ProspectSourceAuditError::Invalid(format!("invalid AHL identity review: {error}"))
    })?;
    if review.season != catalog.season {
        return Err(ProspectSourceAuditError::Invalid(format!(
            "AHL identity review season {} does not match source catalog {}",
            review.season, catalog.season
        )));
    }
    let captured_at = review
        .reviewed_at
        .as_deref()
        .ok_or_else(|| {
            ProspectSourceAuditError::Invalid(
                "finalized AHL identity review requires reviewed_at".to_owned(),
            )
        })
        .and_then(|value| {
            DateTime::parse_from_rfc3339(value)
                .map(|value| value.with_timezone(&Utc))
                .map_err(|_| {
                    ProspectSourceAuditError::Invalid(
                        "AHL identity review reviewed_at must be RFC 3339".to_owned(),
                    )
                })
        })?;
    let content_hash = store.store_capture(bytes)?;
    let adapter = AhlIdentityReviewV1Adapter::new(
        catalog.season,
        &review.ahl_team,
        captured_at,
        registry_url,
        roster,
    )
    .map_err(ProspectSourceAuditError::Invalid)?;
    let descriptor = adapter.descriptor();
    let decisions = adapter
        .parse(SourceInput::new(
            bytes,
            descriptor.source_id.clone(),
            content_hash.clone(),
        ))
        .map_err(|error| ProspectSourceAuditError::Adapter(error.to_string()))?;
    let evidence = SourceEvidence::new(
        descriptor.source_id,
        icelines_core::source_facts::SourceUrl::try_new(registry_url.to_owned())?,
        descriptor.provider,
        captured_at,
        content_hash,
        descriptor.adapter_version,
    );
    Ok((decisions, evidence))
}

fn parse_identity_review_ledger(
    catalog: &ProspectSourceCatalog,
    store: &SourcePackageStore,
    bytes: &[u8],
) -> Result<(Vec<IdentityReviewDecision>, SourceEvidence), ProspectSourceAuditError> {
    let content_hash = store.store_capture(bytes)?;
    let adapter = IdentityReviewLedgerV1Adapter;
    let descriptor = adapter.descriptor();
    let output = adapter
        .parse(SourceInput::new(
            bytes,
            descriptor.source_id.clone(),
            content_hash.clone(),
        ))
        .map_err(|error| ProspectSourceAuditError::Adapter(error.to_string()))?;
    if output.season.0 != catalog.season {
        return Err(ProspectSourceAuditError::Invalid(format!(
            "identity-review ledger season {} does not match catalog {}",
            output.season.0, catalog.season
        )));
    }
    let evidence = SourceEvidence::new(
        descriptor.source_id,
        output.registry_url,
        output.provider,
        output.reviewed_at,
        content_hash,
        descriptor.adapter_version,
    );
    Ok((output.decisions, evidence))
}

fn review_registry_fingerprint(
    decisions: &[icelines_core::source_facts::IdentityReviewDecision],
) -> Result<ContentHash, SourceContractError> {
    let mut decisions = decisions.to_vec();
    decisions.sort_by(|left, right| left.decision_id().cmp(right.decision_id()));
    let bytes = serde_json::to_vec(&decisions).map_err(|_| {
        SourceContractError::InvalidCoverage(
            "identity review registry serialization failed".to_owned(),
        )
    })?;
    ContentHash::try_new(format!("{:x}", Sha256::digest(bytes)))
}

fn parse_draft(
    catalog: &ProspectSourceCatalog,
    acquired: &crate::AcquiredSourceObject,
    bytes: &[u8],
    organizations: &[OrganizationId],
    results: &mut BTreeMap<String, PopulationObjectResult>,
) -> Result<(SourcePackageFragment, SourceEvidence, FreshnessClass), String> {
    let year = acquired
        .request
        .adapter_variant
        .as_deref()
        .map(str::parse::<u16>)
        .transpose()
        .map_err(|_| "NHL draft catalog variant must be a four-digit year".to_owned())?
        .unwrap_or((catalog.season / 10_000) as u16);
    let adapter = OfficialNhlDraftPicksAdapter::new(year, acquired.captured_at)?;
    let output = adapter
        .parse(SourceInput::new(
            bytes,
            adapter.descriptor().source_id,
            acquired.content_hash.clone(),
        ))
        .map_err(|error| error.to_string())?;
    for organization in organizations {
        let records = output
            .selections
            .iter()
            .filter(|selection| &selection.organization == organization)
            .count();
        set_records(results, organization, "nhl_draft", records);
    }
    let evidence = output
        .identity_proposals
        .first()
        .and_then(|proposal| proposal.evidence().first())
        .cloned()
        .ok_or_else(|| "terminal draft ledger contains no evidence-bearing rows".to_owned())?;
    Ok((
        SourcePackageFragment::from(&output),
        evidence,
        FreshnessClass::Static,
    ))
}

fn parse_trades(
    catalog: &ProspectSourceCatalog,
    acquired: &crate::AcquiredSourceObject,
    bytes: &[u8],
    organizations: &[OrganizationId],
    results: &mut BTreeMap<String, PopulationObjectResult>,
) -> Result<(SourcePackageFragment, SourceEvidence, FreshnessClass), String> {
    let allowed = organizations
        .iter()
        .map(|organization| organization.as_str())
        .collect::<BTreeSet<_>>();
    let registry = CANONICAL_TEAMS
        .iter()
        .filter(|(abbr, _)| allowed.contains(*abbr))
        .map(|(abbr, name)| {
            (
                (*name).to_owned(),
                OrganizationId::try_new(*abbr).expect("canonical organization id is valid"),
            )
        })
        .chain(
            catalog
                .organization_aliases
                .iter()
                .filter_map(|(name, abbr)| {
                    if allowed.contains(abbr.as_str()) {
                        Some((
                            name.clone(),
                            OrganizationId::try_new(abbr.clone())
                                .expect("catalog organization alias was scope-validated"),
                        ))
                    } else {
                        None
                    }
                }),
        )
        .collect::<BTreeMap<_, _>>();
    let adapter = NhlTradeTrackerAdapter::new(
        (catalog.season / 10_000) as i32,
        acquired.captured_at,
        acquired.request.source_url.as_str(),
        registry,
    )?;
    let output = adapter
        .parse(SourceInput::new(
            bytes,
            adapter.descriptor().source_id,
            acquired.content_hash.clone(),
        ))
        .map_err(|error| error.to_string())?;
    for organization in organizations {
        let records = output
            .transfers
            .iter()
            .filter(|transfer| &transfer.from == organization || &transfer.to == organization)
            .count();
        set_records(
            results,
            organization,
            "nhl_transaction_publication",
            records,
        );
    }
    Ok((
        SourcePackageFragment::from(&output),
        output.evidence.clone(),
        FreshnessClass::Transactional,
    ))
}

fn parse_roster(
    acquired: &crate::AcquiredSourceObject,
    bytes: &[u8],
    results: &mut BTreeMap<String, PopulationObjectResult>,
) -> Result<(SourcePackageFragment, SourceEvidence, FreshnessClass), String> {
    let organization = acquired
        .request
        .organization
        .as_ref()
        .ok_or_else(|| "roster source object lacks organization".to_owned())?;
    let adapter = OfficialNhlRosterAdapter::new(organization.as_str(), acquired.captured_at)?;
    let assertions = adapter
        .parse(SourceInput::new(
            bytes,
            adapter.descriptor().source_id,
            acquired.content_hash.clone(),
        ))
        .map_err(|error| error.to_string())?;
    set_records(
        results,
        organization,
        "nhl_current_assignment",
        assertions.len(),
    );
    let evidence = assertions
        .first()
        .and_then(|assertion| assertion.evidence().first())
        .cloned()
        .ok_or_else(|| "official roster contains no evidence-bearing rows".to_owned())?;
    Ok((
        SourcePackageFragment::from_facts(assertions),
        evidence,
        FreshnessClass::Roster,
    ))
}

fn set_records(
    results: &mut BTreeMap<String, PopulationObjectResult>,
    organization: &OrganizationId,
    family: &str,
    records: usize,
) {
    let object_id = ProspectPopulationScope::object_id(organization, family);
    let accumulated = match results.get(&object_id) {
        Some(PopulationObjectResult {
            state: SourceObjectState::Acquired { records: existing },
            ..
        }) => *existing + records,
        Some(_) => return,
        None => records,
    };
    results.insert(
        object_id,
        PopulationObjectResult {
            terminal_pagination: true,
            state: SourceObjectState::Acquired {
                records: accumulated,
            },
        },
    );
}

fn quarantine_family_or_object(
    family: &str,
    object_id: &str,
    reason: String,
    results: &mut BTreeMap<String, PopulationObjectResult>,
) {
    let shared = matches!(family, "nhl_draft" | "nhl_transaction_publication");
    for (candidate_id, result) in results.iter_mut() {
        if (shared && candidate_id.ends_with(&format!(":{family}"))) || candidate_id == object_id {
            *result = PopulationObjectResult {
                terminal_pagination: false,
                state: SourceObjectState::Quarantined {
                    reason: reason.clone(),
                },
            };
        }
    }
}

fn input_record(
    evidence: SourceEvidence,
    class: FreshnessClass,
    knowledge_cutoff: DateTime<Utc>,
) -> SourceInputRecord {
    let captured_at = evidence.captured_at();
    SourceInputRecord {
        evidence,
        freshness: SourceFreshness {
            class,
            captured_at,
            evaluated_at: knowledge_cutoff,
            status: if class == FreshnessClass::Static {
                FreshnessStatus::Static
            } else {
                FreshnessStatus::Fresh
            },
            policy_version: PolicyVersion::try_new("source-freshness.v1")
                .expect("static policy version is valid"),
        },
    }
}

fn coverage_and_disclosures(
    manifest: &SourceRunManifest,
) -> (Vec<SourceCoverageBucket>, Vec<SourceDisclosure>) {
    let mut coverage = Vec::with_capacity(manifest.objects.len());
    let mut disclosures = Vec::new();
    for object in &manifest.objects {
        let (acquired, parsed, quarantined) = match object.state {
            SourceObjectState::Acquired { .. } => (1, 1, 0),
            SourceObjectState::Quarantined { .. } => (1, 0, 1),
            _ => (0, 0, 0),
        };
        coverage.push(SourceCoverageBucket {
            source_family: object.source_family.clone(),
            organization: object.organization.clone(),
            expected: 1,
            acquired,
            parsed,
            quarantined,
            ..SourceCoverageBucket::default()
        });
        if let SourceObjectState::Failed { reason } | SourceObjectState::Quarantined { reason } =
            &object.state
        {
            disclosures.push(SourceDisclosure {
                code: SourceDisclosureCode::MissingSourceFamily,
                scope: object.object_id.clone(),
                message: reason.clone(),
            });
        }
    }
    (coverage, disclosures)
}

fn all_families() -> Vec<ProspectPopulationSourceFamily> {
    vec![
        ProspectPopulationSourceFamily::Draft,
        ProspectPopulationSourceFamily::CampPublication,
        ProspectPopulationSourceFamily::ContractPublication,
        ProspectPopulationSourceFamily::TransactionPublication,
        ProspectPopulationSourceFamily::CurrentNhlAssignment,
        ProspectPopulationSourceFamily::CurrentAhlAssignment,
        ProspectPopulationSourceFamily::NhlPlayerLanding,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ProspectSourceCatalogTarget, ProspectSourceCatalogTemplate};
    use chrono::TimeZone;
    use httpmock::prelude::*;
    use tempfile::TempDir;

    #[test]
    fn contract_control_ledger_must_cover_the_exact_audit_scope() {
        let catalog = ProspectSourceCatalog {
            schema: crate::PROSPECT_SOURCE_CATALOG_SCHEMA.to_owned(),
            season: 20_262_027,
            catalog_version: "fixture.v1".to_owned(),
            organization_aliases: BTreeMap::new(),
            template_variants: BTreeMap::new(),
            templates: Vec::new(),
        };
        let directory = TempDir::new().unwrap();
        let store = SourcePackageStore::new(directory.path());
        let mut results = BTreeMap::new();
        let error = parse_contract_control_ledger(
            &catalog,
            &store,
            br#"{
                "schema":"contract_control_ledger.v1",
                "season":20262027,
                "provider":"fixture_contract_registry",
                "captured_at":"2026-07-31T11:45:00Z",
                "source_url":"https://example.test/contracts",
                "coverage":[{"organization":"NYR","terminal":true,"records":0}],
                "contracts":[]
            }"#,
            &[
                OrganizationId::try_new("NYR").unwrap(),
                OrganizationId::try_new("SEA").unwrap(),
            ],
            &mut results,
        )
        .unwrap_err();

        assert!(error.to_string().contains("missing=[SEA]"));
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn audit_seals_an_honest_incomplete_matrix_with_parsed_facts() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/draft");
            then.status(200).body(
                r#"{"broadcastStartTimeUTC":"2026-06-26T23:00:00Z","draftYear":2026,"state":"over","picks":[{"round":1,"pickInRound":5,"overallPick":5,"teamAbbrev":"NYR","firstName":{"default":"Alberts"},"lastName":{"default":"Smits"},"positionCode":"D"},{"round":1,"pickInRound":7,"overallPick":7,"teamAbbrev":"SEA","firstName":{"default":"Chase"},"lastName":{"default":"Reid"},"positionCode":"D"}]}"#,
            );
        });
        server.mock(|when, then| {
            when.method(GET).path("/trades");
            then.status(200).body(
                r#"<script type="application/ld+json">{"articleBody":"JULY 1: New York Rangers acquire forward Example Player from the Seattle Kraken for a draft pick."}</script>"#,
            );
        });
        server.mock(|when, then| {
            when.method(GET).path("/roster/NYR");
            then.status(200).body(
                r#"{"forwards":[{"id":8480001,"positionCode":"C"}],"defensemen":[],"goalies":[]}"#,
            );
        });
        server.mock(|when, then| {
            when.method(GET).path("/roster/SEA");
            then.status(200).body(
                r#"{"forwards":[],"defensemen":[],"goalies":[{"id":8480002,"positionCode":"G"}]}"#,
            );
        });
        let catalog = ProspectSourceCatalog {
            schema: crate::PROSPECT_SOURCE_CATALOG_SCHEMA.to_owned(),
            season: 20_262_027,
            catalog_version: "fixture.v1".to_owned(),
            organization_aliases: BTreeMap::new(),
            template_variants: BTreeMap::new(),
            templates: vec![
                ProspectSourceCatalogTemplate {
                    source_family: "nhl_draft".to_owned(),
                    target: ProspectSourceCatalogTarget::AllOrganizations,
                    url: server.url("/draft"),
                    terminal_pagination: true,
                },
                ProspectSourceCatalogTemplate {
                    source_family: "nhl_transaction_publication".to_owned(),
                    target: ProspectSourceCatalogTarget::AllOrganizations,
                    url: server.url("/trades"),
                    terminal_pagination: true,
                },
                ProspectSourceCatalogTemplate {
                    source_family: "nhl_current_assignment".to_owned(),
                    target: ProspectSourceCatalogTarget::AllOrganizations,
                    url: format!("{}/roster/{{organization}}", server.base_url()),
                    terminal_pagination: true,
                },
            ],
        };
        let directory = TempDir::new().unwrap();
        let store = SourcePackageStore::new(directory.path());
        let package = run_for_organizations(
            &NhlApiClient::new(server.base_url(), server.base_url()).with_retry_params(0, 0, 0),
            &store,
            &catalog,
            ProspectSourceAuditInput {
                captured_at: Utc
                    .with_ymd_and_hms(2026, 7, 31, 12, 0, 0)
                    .single()
                    .unwrap(),
                effective_cutoff: Utc
                    .with_ymd_and_hms(2026, 7, 31, 12, 0, 0)
                    .single()
                    .unwrap(),
                knowledge_cutoff: Utc
                    .with_ymd_and_hms(2026, 7, 31, 12, 0, 0)
                    .single()
                    .unwrap(),
                finalize_cutoffs_after_acquisition: false,
            },
            vec![
                OrganizationId::try_new("NYR").unwrap(),
                OrganizationId::try_new("SEA").unwrap(),
            ],
            ProspectSourceAuditArtifacts {
                ahl_roster_snapshot: Some(
                    br#"{
                        "schema":"ahl_roster_stats.v1",
                        "season":20262027,
                        "provider":"ahl_hockeytech_statview",
                        "provider_season_id":"90",
                        "provider_season_name":"2026-27",
                        "fetched_at":"2026-07-31T11:00:00Z",
                        "source_url":"https://theahl.com/stats/player-stats",
                        "roster_source_url":"https://theahl.com/stats/roster",
                        "identity_note":"provider scoped",
                        "teams":[
                            {
                                "provider":"ahl_hockeytech_statview",
                                "provider_team_id":"1",
                                "team_code":"HFD",
                                "team_name":"Hartford Wolf Pack",
                                "nickname":"Wolf Pack",
                                "division_id":"1",
                                "logo_url":"https://example.test/hfd.png",
                                "nhl_affiliate":"NYR",
                                "roster":[{"provider":"ahl_hockeytech_statview","provider_player_id":"1001","name":"Hartford Prospect","position_group":"Forwards","position":"C","jersey_number":"10","handedness":"L","height":"6-0","weight_pounds":"190","birthdate":"2004-01-01","birthplace":"Test"}],
                                "skaters":[],"goalies":[],"source_warnings":[]
                            },
                            {
                                "provider":"ahl_hockeytech_statview",
                                "provider_team_id":"2",
                                "team_code":"CV",
                                "team_name":"Coachella Valley Firebirds",
                                "nickname":"Firebirds",
                                "division_id":"2",
                                "logo_url":"https://example.test/cv.png",
                                "nhl_affiliate":"SEA",
                                "roster":[{"provider":"ahl_hockeytech_statview","provider_player_id":"1002","name":"Seattle Prospect","position_group":"Defensemen","position":"D","jersey_number":"20","handedness":"R","height":"6-2","weight_pounds":"200","birthdate":"2003-01-01","birthplace":"Test"}],
                                "skaters":[],"goalies":[],"source_warnings":[]
                            }
                        ]
                    }"#
                    .to_vec(),
                ),
                ahl_identity_reviews: vec![
                    br#"{
                        "schema":"ahl_identity_review_decisions.v1",
                        "season":20262027,
                        "provider":"ahl_hockeytech_statview",
                        "ahl_team":"HFD",
                        "roster_fetched_at":"2026-07-31T11:00:00Z",
                        "draft":false,
                        "reviewer":"fixture-reviewer",
                        "reviewed_at":"2026-07-31T11:30:00Z",
                        "decisions":[{
                            "provider_player_id":"1001",
                            "action":"set_identity",
                            "nhl_player_id":8480101,
                            "nhl_display_name":"Hartford Prospect",
                            "nhl_birth_date":"2004-01-01",
                            "evidence_urls":["https://api-web.nhle.com/v1/player/8480101/landing"],
                            "note":"Fixture identity review."
                        }]
                    }"#
                    .to_vec(),
                ],
                ahl_review_registry_url: Some("https://example.test/reviews".to_owned()),
                roster_player_landing_cache_root: None,
                contract_control_ledger: Some(
                    br#"{
                        "schema":"contract_control_ledger.v1",
                        "season":20262027,
                        "provider":"fixture_contract_registry",
                        "captured_at":"2026-07-31T11:45:00Z",
                        "source_url":"https://example.test/contracts",
                        "coverage":[
                            {"organization":"NYR","terminal":true,"records":1},
                            {"organization":"SEA","terminal":true,"records":0}
                        ],
                        "contracts":[{
                            "player_id":8480003,
                            "organization":"NYR",
                            "contract_kind":"entry_level",
                            "effective_at":"2026-07-01T00:00:00Z",
                            "source_url":"https://example.test/contracts/8480003"
                        }]
                    }"#
                    .to_vec(),
                ),
                camp_participation_ledger: Some(
                    br#"{
                        "schema":"camp_participation_ledger.v1",
                        "season":20262027,
                        "provider":"fixture_camp_registry",
                        "captured_at":"2026-07-31T11:50:00Z",
                        "source_url":"https://example.test/camps",
                        "coverage":[
                            {"organization":"NYR","terminal":true,"records":1},
                            {"organization":"SEA","terminal":true,"records":0}
                        ],
                        "participants":[{
                            "player_id":8480005,
                            "organization":"NYR",
                            "kind":"development_camp",
                            "authority":"controlled_player",
                            "occurred_at":"2026-07-03T00:00:00Z",
                            "source_url":"https://example.test/camps/nyr"
                        }]
                    }"#
                    .to_vec(),
                ),
                identity_review_ledgers: vec![
                    br#"{
                        "schema":"identity_review_ledger.v1",
                        "season":20262027,
                        "provider":"fixture_review_registry",
                        "registry_url":"https://example.test/identity-reviews",
                        "reviewer":"fixture-reviewer",
                        "reviewed_at":"2026-07-31T11:55:00Z",
                        "decisions":[{
                            "decision_id":"draft-2026-5-review",
                            "proposal_id":"nhl-draft:2026:5",
                            "action":"set_identity",
                            "player_id":8480006,
                            "rationale":"Official identity evidence confirms the selected player.",
                            "evidence":[{
                                "source_id":"landing-8480006",
                                "source_url":"https://api-web.nhle.com/v1/player/8480006/landing",
                                "provider":"official_nhl",
                                "captured_at":"2026-07-31T11:54:00Z",
                                "content_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                                "adapter_version":"v1"
                            }]
                        }]
                    }"#
                    .to_vec(),
                ],
            },
        )
        .await
        .unwrap();

        assert_eq!(package.run_manifest.objects.len(), 14);
        assert_eq!(
            package
                .run_manifest
                .objects
                .iter()
                .filter(|object| matches!(object.state, SourceObjectState::Acquired { .. }))
                .count(),
            12
        );
        assert_eq!(package.disclosures.len(), 2);
        assert_eq!(package.coverage.len(), 14);
        assert_eq!(package.inputs.len(), 9);
        assert_eq!(package.fact_assertions.len(), 4);
        assert_eq!(package.identity_proposals.len(), 5);
        assert_eq!(package.staged_player_assertions.len(), 5);
        assert_eq!(package.identity_review_decisions.len(), 2);
        assert_ne!(
            package.review_registry_fingerprint,
            ContentHash::try_new(format!("{:x}", Sha256::digest(b"[]"))).unwrap()
        );
        let census = crate::build_prospect_census_from_source_package(
            &package,
            &[],
            10,
            "prospect-eligibility.fixture.v1",
        )
        .unwrap();
        let ahl_candidate = census
            .losses
            .iter()
            .find(|loss| loss.player_id == Some(8_480_101) && loss.organization == "NYR")
            .expect("reviewed affiliated AHL observation must enter the census");
        assert_eq!(
            ahl_candidate.reason,
            icelines_core::ProspectCensusLossReason::UnsupportedControl
        );
        let controlled_candidate = census
            .losses
            .iter()
            .find(|loss| loss.player_id == Some(8_480_003) && loss.organization == "NYR")
            .expect("contract-controlled player must enter the census");
        assert_eq!(
            controlled_candidate.reached_stage,
            icelines_core::ProspectCensusStage::ControlledRelationship
        );
        assert_eq!(
            controlled_candidate.reason,
            icelines_core::ProspectCensusLossReason::MissingEligibilityEvidence
        );
        let camp_candidate = census
            .losses
            .iter()
            .find(|loss| loss.player_id == Some(8_480_005) && loss.organization == "NYR")
            .expect("canonical camp participant must enter the census");
        assert_eq!(
            camp_candidate.reason,
            icelines_core::ProspectCensusLossReason::UnsupportedControl
        );
        let reviewed_draft_candidate = census
            .losses
            .iter()
            .find(|loss| loss.player_id == Some(8_480_006) && loss.organization == "NYR")
            .expect("generically reviewed draft identity must enter the canonical census");
        assert_eq!(
            reviewed_draft_candidate.reason,
            icelines_core::ProspectCensusLossReason::UnsupportedControl
        );
        assert!(!package.run_manifest.complete);
        assert_eq!(
            store.load_package(&package.package_id).unwrap().fingerprint,
            package.fingerprint
        );
        assert!(matches!(
            store.activate(&package.package_id).unwrap_err(),
            SourcePackageStoreError::IncompletePackage(_)
        ));
    }

    #[test]
    fn exact_official_draft_coordinates_create_a_deterministic_identity_decision() {
        let captured_at = Utc
            .with_ymd_and_hms(2026, 7, 31, 12, 0, 0)
            .single()
            .unwrap();
        let draft = OfficialNhlDraftPicksAdapter::new(2022, captured_at).unwrap();
        let draft_output = draft
            .parse(SourceInput::new(
                br#"{"draftYear":2022,"state":"over","picks":[{"round":2,"pickInRound":30,"overallPick":62,"teamAbbrev":"MTL","firstName":{"default":"Lane"},"lastName":{"default":"Hutson"},"positionCode":"D"}]}"#,
                draft.descriptor().source_id,
                ContentHash::try_new("a".repeat(64)).unwrap(),
            ))
            .unwrap();
        let landing = OfficialNhlDraftAdapter::new(8_483_457, captured_at).unwrap();
        let landing_fact = landing
            .parse(SourceInput::new(
                br#"{"playerId":8483457,"draftDetails":{"year":2022,"teamAbbrev":"MTL","round":2,"pickInRound":30,"overallPick":62}}"#,
                landing.descriptor().source_id,
                ContentHash::try_new("b".repeat(64)).unwrap(),
            ))
            .unwrap()
            .unwrap();

        let decisions = exact_draft_identity_decisions(&[
            SourcePackageFragment::from(&draft_output),
            SourcePackageFragment::from_facts(vec![landing_fact]),
        ])
        .unwrap();

        assert_eq!(decisions.len(), 1);
        assert_eq!(
            decisions[0].canonical_player_id(),
            Some(PlayerId(8_483_457))
        );
        assert_eq!(decisions[0].proposal_id().as_str(), "nhl-draft:2022:62");
    }
}
