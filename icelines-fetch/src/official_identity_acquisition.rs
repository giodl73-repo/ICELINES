//! Official NHL search/landing acquisition for unresolved draft identities.

use crate::ahl::{
    normalize_ahl_identity_name, normalized_surname, parse_official_nhl_draft_search_candidates,
};
use crate::fletch::{
    fetch_generic_http_batch_with_policy_async, fetch_player_landing_batch_bytes_async,
    fletch_cache_manifest_path, player_landing_url, read_fletch_cache_manifest,
    read_verified_fletch_cache_batch_bytes, FletchPlayerLandingArtifact,
};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use icelines_core::source_facts::{
    AdapterVersion, ContentHash, IdentityReviewAction, ProviderId, SourceEvidence, SourceId,
    SourceUrl,
};
use icelines_core::{
    build_official_identity_candidate_board, IdentityReviewWorkboardRow,
    IdentityReviewWorkboardView, OfficialIdentityCandidateBoardView, OfficialIdentityCandidateRow,
    OfficialIdentityCandidateStatus, OfficialIdentityCandidateView,
    OfficialIdentityDraftCoordinates,
};
#[cfg(test)]
use icelines_sources::adapter::{SourceAdapter, SourceInput};
use icelines_sources::identity_review::{
    IdentityReviewEvidence, IdentityReviewLedgerDocument, IdentityReviewLedgerRow,
    IDENTITY_REVIEW_LEDGER_V1,
};
use icelines_sources::nhl::official_identity_landing::{
    parse_official_identity_landing, OfficialIdentityLandingRecord as LandingRecord,
};
use reqwest::Url;
#[cfg(test)]
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const SEARCH_BASE: &str = "https://search.d3.nhle.com/api/v1/search/player";
// Keep provider retries from withholding a large completed cohort from the
// atomic FLETCH manifest. League-wide runs can resume after at most 25 rows.
const SEARCH_BATCH_SIZE: usize = 25;
const LANDING_BATCH_SIZE: usize = 100;

#[derive(Debug, Clone)]
pub struct OfficialIdentityAcquisitionOptions {
    pub cache_root: PathBuf,
    pub refresh: bool,
    pub offline: bool,
    pub search_concurrency: usize,
    pub landing_delay_ms: u64,
    /// Optional replay boundary. Live acquisition leaves this unset and emits
    /// a new evidence horizon for the next source-package seal.
    pub evidence_cutoff: Option<DateTime<Utc>>,
}

impl OfficialIdentityAcquisitionOptions {
    pub fn new(cache_root: impl Into<PathBuf>) -> Self {
        Self {
            cache_root: cache_root.into(),
            refresh: false,
            offline: false,
            search_concurrency: 6,
            landing_delay_ms: 50,
            evidence_cutoff: None,
        }
    }
}

#[derive(Debug, Clone)]
struct SearchRequest {
    dataset_id: String,
    source_url: String,
}

type ParsedSearchCandidates = Result<Vec<(u32, String)>, String>;

/// Finalize the board's exact-coordinate rows into the generic review-ledger
/// contract. Supplying reviewer metadata is the explicit authority action;
/// non-eligible rows are retained in the candidate board and never inferred.
pub fn build_official_identity_review_ledger(
    board: &OfficialIdentityCandidateBoardView,
    provider: impl Into<String>,
    registry_url: impl Into<String>,
    reviewer: impl Into<String>,
    reviewed_at: DateTime<Utc>,
) -> Result<IdentityReviewLedgerDocument> {
    if board.schema != icelines_core::OFFICIAL_IDENTITY_CANDIDATE_BOARD_SCHEMA
        || board.evaluated_count != board.rows.len()
        || board.eligible_count
            != board
                .rows
                .iter()
                .filter(|row| row.eligible_player_id.is_some())
                .count()
    {
        anyhow::bail!("official identity candidate board counts or schema are invalid");
    }
    let provider = provider.into();
    let registry_url = registry_url.into();
    let reviewer = reviewer.into();
    ProviderId::try_new(provider.clone())?;
    SourceUrl::try_new(registry_url.clone())?;
    if reviewer.trim().is_empty() {
        anyhow::bail!("official identity review requires a reviewer");
    }
    let mut decisions = Vec::with_capacity(board.eligible_count);
    for row in &board.rows {
        let Some(player_id) = row.eligible_player_id else {
            continue;
        };
        if row.status != OfficialIdentityCandidateStatus::ExactCoordinateMatch {
            anyhow::bail!(
                "proposal {} has an eligible ID without exact status",
                row.proposal_id
            );
        }
        let exact = row
            .candidates
            .iter()
            .filter(|candidate| {
                candidate.player_id == player_id
                    && candidate.draft.as_ref() == Some(&row.expected_draft)
            })
            .collect::<Vec<_>>();
        if exact.len() != 1 {
            anyhow::bail!(
                "proposal {} does not retain one exact eligible candidate",
                row.proposal_id
            );
        }
        let candidate = exact[0];
        let landing = candidate.landing_evidence.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "proposal {} exact candidate has no landing evidence",
                row.proposal_id
            )
        })?;
        let mut evidence = vec![
            review_evidence(&candidate.search_evidence, reviewed_at)?,
            review_evidence(landing, reviewed_at)?,
        ];
        evidence.extend(
            row.proposal_evidence
                .iter()
                .map(|item| review_evidence(item, reviewed_at))
                .collect::<Result<Vec<_>>>()?,
        );
        evidence.sort_by(|left, right| {
            (&left.source_id, &left.source_url, &left.content_sha256).cmp(&(
                &right.source_id,
                &right.source_url,
                &right.content_sha256,
            ))
        });
        evidence.dedup();
        decisions.push(IdentityReviewLedgerRow {
            decision_id: format!("official-draft-coordinate:{}", row.proposal_id),
            proposal_id: row.proposal_id.clone(),
            action: IdentityReviewAction::SetIdentity,
            player_id: Some(player_id),
            rationale: format!(
                "Official player search and player landing uniquely agree with {} {} round {} overall {}.",
                row.expected_draft.organization,
                row.expected_draft.year,
                row.expected_draft.round,
                row.expected_draft.overall
            ),
            evidence,
        });
    }
    if decisions.is_empty() {
        anyhow::bail!("official identity candidate board has no eligible decisions");
    }
    Ok(IdentityReviewLedgerDocument {
        schema: IDENTITY_REVIEW_LEDGER_V1.to_owned(),
        season: board.evaluation_season,
        provider,
        registry_url,
        reviewer,
        reviewed_at: reviewed_at.to_rfc3339(),
        decisions,
    })
}

fn review_evidence(
    evidence: &SourceEvidence,
    reviewed_at: DateTime<Utc>,
) -> Result<IdentityReviewEvidence> {
    if evidence.captured_at() > reviewed_at {
        anyhow::bail!("identity evidence cannot be captured after review");
    }
    Ok(IdentityReviewEvidence {
        source_id: evidence.source_id().as_str().to_owned(),
        source_url: evidence.source_url().as_str().to_owned(),
        provider: evidence.provider().as_str().to_owned(),
        captured_at: evidence.captured_at().to_rfc3339(),
        content_sha256: evidence.content_sha256().as_str().to_owned(),
        adapter_version: evidence.adapter_version().as_str().to_owned(),
    })
}

/// Acquire official evidence for every unresolved draft row. Name matching is
/// discovery-only; exact names are preferred and a surname fallback can widen
/// discovery. Eligibility is set only when all retained candidates have valid
/// landings and exactly one has identical draft coordinates.
pub async fn acquire_official_identity_candidates(
    workboard: &IdentityReviewWorkboardView,
    options: OfficialIdentityAcquisitionOptions,
) -> Result<OfficialIdentityCandidateBoardView> {
    validate_workboard(workboard)?;
    let base_knowledge_cutoff = DateTime::parse_from_rfc3339(&workboard.knowledge_cutoff)
        .context("parse identity workboard knowledge cutoff")?
        .with_timezone(&Utc);
    let draft_rows = workboard
        .rows
        .iter()
        .filter(|row| row.contexts.iter().any(|context| context.family == "draft"))
        .collect::<Vec<_>>();
    if draft_rows.is_empty() {
        return build_official_identity_candidate_board(
            workboard.evaluation_season,
            workboard.source_package_id.clone(),
            workboard.source_package_fingerprint.clone(),
            workboard.effective_cutoff.clone(),
            workboard.knowledge_cutoff.clone(),
            workboard.knowledge_cutoff.clone(),
            Vec::new(),
        )
        .map_err(anyhow::Error::msg);
    }

    let mut requests = BTreeMap::<String, SearchRequest>::new();
    let mut dataset_by_proposal = BTreeMap::new();
    for row in &draft_rows {
        let (dataset_id, source_url) = official_player_search_request(&row.search_query)?;
        let request = SearchRequest {
            dataset_id,
            source_url,
        };
        dataset_by_proposal.insert(row.proposal_id.clone(), request.dataset_id.clone());
        requests
            .entry(request.dataset_id.clone())
            .or_insert(request);
    }
    let mut search_results = BTreeMap::<String, Result<Vec<u8>, String>>::new();
    let request_list = requests
        .values()
        .map(|request| (request.dataset_id.clone(), request.source_url.clone()))
        .collect::<Vec<_>>();
    let mut relevant_dataset_ids = requests.keys().cloned().collect::<BTreeSet<_>>();
    if options.offline {
        let cached = read_verified_fletch_cache_batch_bytes(
            &options.cache_root,
            request_list
                .iter()
                .map(|(dataset_id, _)| dataset_id.clone()),
        )?;
        for (dataset_id, _) in request_list {
            let result = cached
                .get(&dataset_id)
                .cloned()
                .ok_or_else(|| "verified search cacheline is unavailable offline".to_owned());
            search_results.insert(dataset_id, result);
        }
    } else {
        for (dataset_id, result) in fetch_official_player_search_cachelines(
            request_list,
            options.cache_root.clone(),
            options.refresh,
            options.search_concurrency,
        )
        .await
        {
            search_results.insert(dataset_id, result.map_err(|error| format!("{error:#}")));
        }
    }
    let mut captured = verified_capture_times(&options.cache_root)?;

    let mut parsed_search = BTreeMap::<String, ParsedSearchCandidates>::new();
    let mut player_ids = BTreeSet::new();
    for row in &draft_rows {
        let dataset_id = &dataset_by_proposal[&row.proposal_id];
        let source_url = &requests[dataset_id].source_url;
        let parsed = match search_results.get(dataset_id) {
            Some(Ok(bytes)) => {
                parse_official_nhl_draft_search_candidates(&row.displayed_name, source_url, bytes)
                    .map(|candidates| {
                        candidates
                            .into_iter()
                            .map(|candidate| (candidate.nhl_player_id, candidate.display_name))
                            .collect::<Vec<_>>()
                    })
                    .map_err(|error| error.to_string())
            }
            Some(Err(error)) => Err(error.clone()),
            None => Err("official search produced no cacheline result".to_owned()),
        };
        if let Ok(candidates) = &parsed {
            player_ids.extend(candidates.iter().map(|(player_id, _)| *player_id));
        }
        parsed_search.insert(row.proposal_id.clone(), parsed);
    }

    // Some provider searches return no rows for the full published draft name
    // (parentheticals, alternate given names, or transliterations). Query only
    // those empty rows by surname. This widens discovery, never eligibility:
    // classify_row still requires a verified landing and one exact immutable
    // draft-coordinate match.
    let mut fallback_requests = BTreeMap::<String, SearchRequest>::new();
    let mut fallback_by_proposal = BTreeMap::<String, String>::new();
    for row in &draft_rows {
        if !matches!(parsed_search.get(&row.proposal_id), Some(Ok(rows)) if rows.is_empty()) {
            continue;
        }
        let Some(surname) = normalized_surname(&row.displayed_name) else {
            continue;
        };
        let (dataset_id, source_url) = official_player_search_request(&surname)?;
        fallback_by_proposal.insert(row.proposal_id.clone(), dataset_id.clone());
        fallback_requests
            .entry(dataset_id.clone())
            .or_insert(SearchRequest {
                dataset_id,
                source_url,
            });
    }
    let fallback_list = fallback_requests
        .values()
        .map(|request| (request.dataset_id.clone(), request.source_url.clone()))
        .collect::<Vec<_>>();
    let mut fallback_results = BTreeMap::<String, Result<Vec<u8>, String>>::new();
    if options.offline {
        let cached = read_verified_fletch_cache_batch_bytes(
            &options.cache_root,
            fallback_list
                .iter()
                .map(|(dataset_id, _)| dataset_id.clone()),
        )?;
        for (dataset_id, _) in &fallback_list {
            if let Some(bytes) = cached.get(dataset_id) {
                fallback_results.insert(dataset_id.clone(), Ok(bytes.clone()));
            }
        }
    } else {
        for (dataset_id, result) in fetch_official_player_search_cachelines(
            fallback_list,
            options.cache_root.clone(),
            options.refresh,
            options.search_concurrency,
        )
        .await
        {
            fallback_results.insert(dataset_id, result.map_err(|error| format!("{error:#}")));
        }
    }
    captured = merge_capture_times(captured, verified_capture_times(&options.cache_root)?);
    for row in &draft_rows {
        let Some(dataset_id) = fallback_by_proposal.get(&row.proposal_id) else {
            continue;
        };
        let Some(result) = fallback_results.get(dataset_id) else {
            continue;
        };
        let request = &fallback_requests[dataset_id];
        let parsed = match result {
            Ok(bytes) => parse_official_nhl_draft_search_candidates(
                &row.displayed_name,
                &request.source_url,
                bytes,
            )
            .map(|candidates| {
                candidates
                    .into_iter()
                    .map(|candidate| (candidate.nhl_player_id, candidate.display_name))
                    .collect::<Vec<_>>()
            })
            .map_err(|error| error.to_string()),
            Err(error) => Err(error.clone()),
        };
        if let Ok(candidates) = &parsed {
            player_ids.extend(candidates.iter().map(|(player_id, _)| *player_id));
        }
        relevant_dataset_ids.insert(dataset_id.clone());
        dataset_by_proposal.insert(row.proposal_id.clone(), dataset_id.clone());
        parsed_search.insert(row.proposal_id.clone(), parsed);
        search_results.insert(dataset_id.clone(), result.clone());
        requests.insert(dataset_id.clone(), request.clone());
    }

    let mut landing_bytes = BTreeMap::new();
    let player_ids = player_ids.into_iter().collect::<Vec<_>>();
    relevant_dataset_ids.extend(
        player_ids
            .iter()
            .map(|player_id| format!("icelines.player.landing.{player_id}")),
    );
    if options.offline {
        let cached = read_verified_fletch_cache_batch_bytes(
            &options.cache_root,
            player_ids
                .iter()
                .map(|player_id| format!("icelines.player.landing.{player_id}")),
        )?;
        for player_id in player_ids {
            let dataset_id = format!("icelines.player.landing.{player_id}");
            if let Some(bytes) = cached.get(&dataset_id) {
                landing_bytes.insert(player_id, bytes.clone());
            }
        }
    } else {
        landing_bytes.extend(
            fetch_official_player_landing_cachelines(
                player_ids,
                options.cache_root.clone(),
                options.refresh,
                options.landing_delay_ms,
            )
            .await?,
        );
    }
    let captured = merge_capture_times(captured, verified_capture_times(&options.cache_root)?);

    let mut rows = Vec::with_capacity(draft_rows.len());
    for row in draft_rows {
        rows.push(classify_row(
            row,
            &requests,
            &dataset_by_proposal,
            &search_results,
            &parsed_search,
            &landing_bytes,
            &captured,
            options.evidence_cutoff,
        )?);
    }
    let evidence_cutoff = options
        .evidence_cutoff
        .or_else(|| {
            relevant_dataset_ids
                .iter()
                .filter_map(|dataset_id| captured.get(dataset_id).copied())
                .max()
        })
        .unwrap_or(base_knowledge_cutoff);
    build_official_identity_candidate_board(
        workboard.evaluation_season,
        workboard.source_package_id.clone(),
        workboard.source_package_fingerprint.clone(),
        workboard.effective_cutoff.clone(),
        workboard.knowledge_cutoff.clone(),
        evidence_cutoff.to_rfc3339(),
        rows,
    )
    .map_err(anyhow::Error::msg)
}

fn validate_workboard(workboard: &IdentityReviewWorkboardView) -> Result<()> {
    if workboard.schema != icelines_core::IDENTITY_REVIEW_WORKBOARD_SCHEMA
        || workboard.unresolved_count != workboard.rows.len()
    {
        anyhow::bail!("identity workboard schema or unresolved count is invalid");
    }
    for row in workboard
        .rows
        .iter()
        .filter(|row| row.contexts.iter().any(|context| context.family == "draft"))
    {
        let drafts = row
            .contexts
            .iter()
            .filter(|context| context.family == "draft")
            .collect::<Vec<_>>();
        if drafts.len() != 1 || drafts[0].organization.is_none() || drafts[0].draft.is_none() {
            anyhow::bail!(
                "draft proposal {} requires one structured draft context; regenerate the workboard",
                row.proposal_id
            );
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn classify_row(
    row: &IdentityReviewWorkboardRow,
    requests: &BTreeMap<String, SearchRequest>,
    dataset_by_proposal: &BTreeMap<String, String>,
    search_results: &BTreeMap<String, Result<Vec<u8>, String>>,
    parsed_search: &BTreeMap<String, ParsedSearchCandidates>,
    landing_bytes: &BTreeMap<u32, Vec<u8>>,
    captured: &BTreeMap<String, DateTime<Utc>>,
    evidence_cutoff: Option<DateTime<Utc>>,
) -> Result<OfficialIdentityCandidateRow> {
    let context = row
        .contexts
        .iter()
        .find(|context| context.family == "draft")
        .expect("validated draft context");
    let coordinates = context.draft.as_ref().expect("validated draft coordinates");
    let expected = OfficialIdentityDraftCoordinates {
        organization: context
            .organization
            .clone()
            .expect("validated organization"),
        year: coordinates.year,
        round: coordinates.round,
        overall: coordinates.overall,
    };
    let dataset_id = &dataset_by_proposal[&row.proposal_id];
    let request = &requests[dataset_id];
    let mut errors = Vec::new();
    let Some(search_capture) = captured.get(dataset_id).copied() else {
        return Ok(failure_row(
            row,
            expected,
            "missing verified search capture timestamp",
        ));
    };
    if evidence_cutoff.is_some_and(|cutoff| search_capture > cutoff) {
        return Ok(failure_row(
            row,
            expected,
            "search capture is after the replay evidence cutoff",
        ));
    }
    let search_bytes = match search_results.get(dataset_id) {
        Some(Ok(bytes)) => bytes,
        Some(Err(error)) => return Ok(failure_row(row, expected, error)),
        None => return Ok(failure_row(row, expected, "missing search bytes")),
    };
    let search_evidence = evidence(
        dataset_id,
        &request.source_url,
        "official_nhl_search",
        search_capture,
        search_bytes,
        "v1",
    )?;
    let search_candidates = match &parsed_search[&row.proposal_id] {
        Ok(candidates) => candidates,
        Err(error) => return Ok(failure_row(row, expected, error)),
    };
    if search_candidates.is_empty() {
        return Ok(row_with(
            row,
            expected,
            OfficialIdentityCandidateStatus::NoExactName,
            None,
            Vec::new(),
            errors,
        ));
    }

    let mut candidates = Vec::new();
    let mut missing_landing = false;
    let mut provider_failure = false;
    for (player_id, search_name) in search_candidates {
        let landing_dataset = format!("icelines.player.landing.{player_id}");
        let Some(bytes) = landing_bytes.get(player_id) else {
            missing_landing = true;
            errors.push(format!("player {player_id}: missing landing bytes"));
            candidates.push(OfficialIdentityCandidateView {
                player_id: *player_id,
                display_name: search_name.clone(),
                birth_date: None,
                draft: None,
                search_evidence: search_evidence.clone(),
                landing_evidence: None,
            });
            continue;
        };
        let Some(landing_capture) = captured.get(&landing_dataset).copied() else {
            provider_failure = true;
            errors.push(format!(
                "player {player_id}: missing verified landing timestamp"
            ));
            continue;
        };
        if evidence_cutoff.is_some_and(|cutoff| landing_capture > cutoff) {
            provider_failure = true;
            errors.push(format!(
                "player {player_id}: landing capture is after replay evidence cutoff"
            ));
            continue;
        }
        match parse_landing(*player_id, bytes, landing_capture) {
            Ok(landing)
                if normalize_ahl_identity_name(&landing.display_name)
                    == normalize_ahl_identity_name(search_name)
                    && row
                        .birth_date
                        .as_deref()
                        .is_none_or(|expected| landing.birth_date.as_deref() == Some(expected)) =>
            {
                candidates.push(OfficialIdentityCandidateView {
                    player_id: *player_id,
                    display_name: landing.display_name,
                    birth_date: landing.birth_date,
                    draft: landing.draft,
                    search_evidence: search_evidence.clone(),
                    landing_evidence: Some(landing.evidence),
                })
            }
            Ok(landing) => {
                provider_failure = true;
                errors.push(format!(
                    "player {player_id}: landing identity {} {:?} conflicts with search/proposal identity {} {:?}",
                    landing.display_name, landing.birth_date, search_name, row.birth_date
                ));
            }
            Err(error) => {
                provider_failure = true;
                errors.push(format!("player {player_id}: {error:#}"));
            }
        }
    }
    candidates.sort_by_key(|candidate| candidate.player_id);
    if provider_failure || missing_landing {
        return Ok(row_with(
            row,
            expected,
            if provider_failure {
                OfficialIdentityCandidateStatus::ProviderFailure
            } else {
                OfficialIdentityCandidateStatus::LandingMissing
            },
            None,
            candidates,
            errors,
        ));
    }
    let exact = candidates
        .iter()
        .filter(|candidate| candidate.draft.as_ref() == Some(&expected))
        .map(|candidate| candidate.player_id)
        .collect::<Vec<_>>();
    let (status, eligible) = match exact.as_slice() {
        [player_id] => (
            OfficialIdentityCandidateStatus::ExactCoordinateMatch,
            Some(*player_id),
        ),
        [] => (OfficialIdentityCandidateStatus::CoordinateMismatch, None),
        _ => (
            OfficialIdentityCandidateStatus::AmbiguousCoordinateMatch,
            None,
        ),
    };
    Ok(row_with(
        row, expected, status, eligible, candidates, errors,
    ))
}

fn parse_landing(
    player_id: u32,
    bytes: &[u8],
    captured_at: DateTime<Utc>,
) -> Result<LandingRecord> {
    let source_url = player_landing_url(
        "https://api-web.nhle.com/v1",
        player_id,
        FletchPlayerLandingArtifact::Landing,
    );
    parse_official_identity_landing(player_id, bytes, captured_at, &source_url)
        .map_err(anyhow::Error::msg)
}

fn row_with(
    row: &IdentityReviewWorkboardRow,
    expected_draft: OfficialIdentityDraftCoordinates,
    status: OfficialIdentityCandidateStatus,
    eligible_player_id: Option<u32>,
    candidates: Vec<OfficialIdentityCandidateView>,
    errors: Vec<String>,
) -> OfficialIdentityCandidateRow {
    OfficialIdentityCandidateRow {
        rank: row.rank,
        proposal_id: row.proposal_id.clone(),
        displayed_name: row.displayed_name.clone(),
        search_query: row.search_query.clone(),
        expected_draft,
        proposal_evidence: row.evidence.clone(),
        status,
        eligible_player_id,
        candidates,
        errors,
    }
}

fn failure_row(
    row: &IdentityReviewWorkboardRow,
    expected_draft: OfficialIdentityDraftCoordinates,
    error: impl Into<String>,
) -> OfficialIdentityCandidateRow {
    row_with(
        row,
        expected_draft,
        OfficialIdentityCandidateStatus::ProviderFailure,
        None,
        Vec::new(),
        vec![error.into()],
    )
}

pub fn official_player_search_request(query: &str) -> Result<(String, String)> {
    let normalized = icelines_core::normalize_name(query);
    let mut url = Url::parse(SEARCH_BASE)?;
    url.query_pairs_mut()
        .append_pair("culture", "en-us")
        .append_pair("limit", "20")
        .append_pair("q", &normalized);
    let digest = format!("{:x}", Sha256::digest(normalized.as_bytes()));
    Ok((
        format!("icelines.nhl.player-search.{}", &digest[..20]),
        url.to_string(),
    ))
}

/// Fetch official NHL player-search requests in bounded FLETCH batches. The
/// caller supplies stable dataset IDs so exact-name and discovery-only query
/// families cannot collide.
pub async fn fetch_official_player_search_cachelines(
    requests: Vec<(String, String)>,
    cache_root: PathBuf,
    refresh: bool,
    max_concurrency: usize,
) -> Vec<(String, anyhow::Result<Vec<u8>>)> {
    let mut results = Vec::with_capacity(requests.len());
    let pending = if refresh {
        requests
    } else {
        let dataset_ids = requests
            .iter()
            .map(|(dataset_id, _)| dataset_id.clone())
            .collect::<Vec<_>>();
        match read_verified_fletch_cache_batch_bytes(&cache_root, dataset_ids) {
            Ok(cached) => partition_cached_search_requests(requests, cached, &mut results),
            Err(error) => {
                results.push(("fletch-manifest".to_owned(), Err(error)));
                return results;
            }
        }
    };
    for chunk in pending.chunks(SEARCH_BATCH_SIZE) {
        results.extend(
            fetch_generic_http_batch_with_policy_async(
                chunk.to_vec(),
                cache_root.clone(),
                refresh,
                max_concurrency,
                10_000,
                3,
            )
            .await,
        );
    }
    results
}

fn partition_cached_search_requests(
    requests: Vec<(String, String)>,
    mut cached: BTreeMap<String, Vec<u8>>,
    results: &mut Vec<(String, anyhow::Result<Vec<u8>>)>,
) -> Vec<(String, String)> {
    let mut pending = Vec::new();
    for (dataset_id, source_url) in requests {
        if let Some(bytes) = cached.remove(&dataset_id) {
            results.push((dataset_id, Ok(bytes)));
        } else {
            pending.push((dataset_id, source_url));
        }
    }
    pending
}

/// Fetch official player landings in bounded FLETCH batches and return the raw
/// verified cacheline bytes keyed by canonical NHL player ID.
pub async fn fetch_official_player_landing_cachelines(
    player_ids: Vec<u32>,
    cache_root: PathBuf,
    refresh: bool,
    delay_between_items_ms: u64,
) -> Result<BTreeMap<u32, Vec<u8>>> {
    let mut results = BTreeMap::new();
    let player_ids = player_ids.into_iter().collect::<BTreeSet<_>>();
    let mut pending = player_ids.iter().copied().collect::<Vec<_>>();
    if !refresh {
        let dataset_ids = player_ids
            .iter()
            .map(|player_id| {
                format!(
                    "icelines.player.{}.{player_id}",
                    FletchPlayerLandingArtifact::Landing.id_segment()
                )
            })
            .collect::<Vec<_>>();
        let cached = read_verified_fletch_cache_batch_bytes(&cache_root, dataset_ids)?;
        pending.retain(|player_id| {
            let dataset_id = format!(
                "icelines.player.{}.{player_id}",
                FletchPlayerLandingArtifact::Landing.id_segment()
            );
            if let Some(bytes) = cached.get(&dataset_id) {
                results.insert(*player_id, bytes.clone());
                false
            } else {
                true
            }
        });
    }
    for chunk in pending.chunks(LANDING_BATCH_SIZE) {
        results.extend(
            fetch_player_landing_batch_bytes_async(
                chunk.to_vec(),
                FletchPlayerLandingArtifact::Landing,
                cache_root.clone(),
                // The batch read above proved these IDs absent. Bypass the
                // per-chunk manifest scan; the global fetch lock prevents a
                // competing writer from filling them between phases.
                true,
                delay_between_items_ms,
            )
            .await?,
        );
    }
    Ok(results)
}

fn verified_capture_times(cache_root: &Path) -> Result<BTreeMap<String, DateTime<Utc>>> {
    let path = fletch_cache_manifest_path(cache_root);
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let manifest = read_fletch_cache_manifest(&path)?;
    Ok(manifest
        .entries
        .into_iter()
        .filter(|entry| entry.verified)
        .filter_map(|entry| {
            i64::try_from(entry.fetched_at_ms)
                .ok()
                .and_then(DateTime::from_timestamp_millis)
                .map(|captured_at| (entry.dataset_id, captured_at))
        })
        .collect())
}

fn merge_capture_times(
    mut left: BTreeMap<String, DateTime<Utc>>,
    right: BTreeMap<String, DateTime<Utc>>,
) -> BTreeMap<String, DateTime<Utc>> {
    left.extend(right);
    left
}

fn evidence(
    source_id: &str,
    source_url: &str,
    provider: &str,
    captured_at: DateTime<Utc>,
    bytes: &[u8],
    adapter_version: &str,
) -> Result<SourceEvidence> {
    Ok(SourceEvidence::new(
        SourceId::try_new(source_id)?,
        SourceUrl::try_new(source_url)?,
        ProviderId::try_new(provider)?,
        captured_at,
        hash(bytes)?,
        AdapterVersion::try_new(adapter_version)?,
    ))
}

fn hash(bytes: &[u8]) -> Result<ContentHash> {
    ContentHash::try_new(format!("{:x}", Sha256::digest(bytes))).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use icelines_core::{
        IdentityReviewContextInput, IdentityReviewDraftCoordinates, IdentityReviewFamilyCount,
    };
    use icelines_sources::identity_review::IdentityReviewLedgerV1Adapter;

    fn row() -> IdentityReviewWorkboardRow {
        IdentityReviewWorkboardRow {
            rank: 1,
            proposal_id: "draft:2026:1:5".to_owned(),
            displayed_name: "Alex Example".to_owned(),
            birth_date: None,
            proposed_player_id: None,
            search_query: "Alex Example".to_owned(),
            providers: vec!["official_nhl".to_owned()],
            evidence_urls: vec!["https://example.test/draft".to_owned()],
            evidence: vec![evidence(
                "draft",
                "https://example.test/draft",
                "official_nhl_api",
                Utc.with_ymd_and_hms(2026, 6, 28, 0, 0, 0).single().unwrap(),
                b"draft fixture",
                "v1",
            )
            .unwrap()],
            contexts: vec![IdentityReviewContextInput {
                family: "draft".to_owned(),
                organization: Some("NYR".to_owned()),
                draft: Some(IdentityReviewDraftCoordinates {
                    year: 2026,
                    round: 1,
                    overall: 5,
                }),
                detail: "2026 round 1 overall 5".to_owned(),
            }],
        }
    }

    fn landing(player_id: u32, team: &str, overall: u16) -> Vec<u8> {
        landing_named(player_id, team, overall, "Example")
    }

    fn landing_named(player_id: u32, team: &str, overall: u16, last_name: &str) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "playerId": player_id,
            "firstName": {"default":"Alex"},
            "lastName": {"default":last_name},
            "birthDate":"2008-01-02",
            "draftDetails": {
                "year": 2026,
                "teamAbbrev": team,
                "round": 1,
                "overallPick": overall
            }
        }))
        .unwrap()
    }

    fn classify(search: Value, landings: &[(u32, Vec<u8>)]) -> OfficialIdentityCandidateRow {
        classify_with_cutoff(search, landings, None)
    }

    fn classify_with_cutoff(
        search: Value,
        landings: &[(u32, Vec<u8>)],
        evidence_cutoff: Option<DateTime<Utc>>,
    ) -> OfficialIdentityCandidateRow {
        let row = row();
        let (dataset_id, source_url) = official_player_search_request(&row.search_query).unwrap();
        let request = SearchRequest {
            dataset_id,
            source_url,
        };
        let search_bytes = serde_json::to_vec(&search).unwrap();
        let parsed = parse_official_nhl_draft_search_candidates(
            &row.displayed_name,
            &request.source_url,
            &search_bytes,
        )
        .map(|candidates| {
            candidates
                .into_iter()
                .map(|candidate| (candidate.nhl_player_id, candidate.display_name))
                .collect()
        })
        .map_err(|error| error.to_string());
        let captured_at = Utc.with_ymd_and_hms(2026, 7, 31, 0, 0, 0).single().unwrap();
        let mut captures = BTreeMap::from([(request.dataset_id.clone(), captured_at)]);
        for (player_id, _) in landings {
            captures.insert(format!("icelines.player.landing.{player_id}"), captured_at);
        }
        classify_row(
            &row,
            &BTreeMap::from([(request.dataset_id.clone(), request.clone())]),
            &BTreeMap::from([(row.proposal_id.clone(), request.dataset_id.clone())]),
            &BTreeMap::from([(request.dataset_id, Ok(search_bytes))]),
            &BTreeMap::from([(row.proposal_id.clone(), parsed)]),
            &landings.iter().cloned().collect(),
            &captures,
            evidence_cutoff,
        )
        .unwrap()
    }

    fn search(ids: &[u32]) -> Value {
        Value::Array(
            ids.iter()
                .map(|id| serde_json::json!({"playerId": id, "name":"Alex Example"}))
                .collect(),
        )
    }

    #[test]
    fn only_one_exact_unique_draft_coordinate_is_eligible() {
        let exact = landing(101, "NYR", 5);
        let mismatch = landing(102, "SEA", 5);
        let result = classify(
            search(&[101, 102]),
            &[(101, exact.clone()), (102, mismatch)],
        );
        assert_eq!(
            result.status,
            OfficialIdentityCandidateStatus::ExactCoordinateMatch
        );
        assert_eq!(result.eligible_player_id, Some(101));
        assert_eq!(
            result.candidates[0]
                .landing_evidence
                .as_ref()
                .unwrap()
                .content_sha256()
                .as_str(),
            hash(&exact).unwrap().as_str()
        );
    }

    #[test]
    fn surname_discovery_requires_one_unique_official_draft_coordinate() {
        let search = serde_json::json!([{"playerId":101,"name":"Alex J. Example"}]);
        let result = classify(search, &[(101, landing_named(101, "NYR", 5, "J. Example"))]);
        assert_eq!(
            result.status,
            OfficialIdentityCandidateStatus::ExactCoordinateMatch
        );
        assert_eq!(result.eligible_player_id, Some(101));
        assert_eq!(result.candidates.len(), 1);
    }

    #[test]
    fn ambiguity_mismatch_missing_and_non_exact_names_never_become_eligible() {
        let ambiguous = classify(
            search(&[101, 102]),
            &[(101, landing(101, "NYR", 5)), (102, landing(102, "NYR", 5))],
        );
        assert_eq!(
            ambiguous.status,
            OfficialIdentityCandidateStatus::AmbiguousCoordinateMatch
        );
        let mismatch = classify(search(&[101]), &[(101, landing(101, "SEA", 5))]);
        assert_eq!(
            mismatch.status,
            OfficialIdentityCandidateStatus::CoordinateMismatch
        );
        let missing = classify(search(&[101]), &[]);
        assert_eq!(
            missing.status,
            OfficialIdentityCandidateStatus::LandingMissing
        );
        let fuzzy = classify(
            serde_json::json!([{"playerId":101,"name":"Alex Examples"}]),
            &[],
        );
        assert_eq!(fuzzy.status, OfficialIdentityCandidateStatus::NoExactName);
        assert!(ambiguous.eligible_player_id.is_none());
        assert!(mismatch.eligible_player_id.is_none());
        assert!(missing.eligible_player_id.is_none());
        assert!(fuzzy.eligible_player_id.is_none());
    }

    #[test]
    fn explicit_replay_cutoff_rejects_newer_cachelines() {
        let cutoff = Utc.with_ymd_and_hms(2026, 7, 30, 0, 0, 0).single().unwrap();
        let result = classify_with_cutoff(
            search(&[101]),
            &[(101, landing(101, "NYR", 5))],
            Some(cutoff),
        );
        assert_eq!(
            result.status,
            OfficialIdentityCandidateStatus::ProviderFailure
        );
        assert!(result.eligible_player_id.is_none());
        assert!(result.errors[0].contains("after the replay evidence cutoff"));
    }

    #[test]
    fn landing_identity_conflict_fails_even_when_draft_coordinates_match() {
        let result = classify(
            search(&[101]),
            &[(101, landing_named(101, "NYR", 5, "Different"))],
        );
        assert_eq!(
            result.status,
            OfficialIdentityCandidateStatus::ProviderFailure
        );
        assert!(result.eligible_player_id.is_none());
        assert!(result.errors[0].contains("conflicts with search"));
    }

    #[test]
    fn explicit_reviewer_can_finalize_only_the_exact_rows() {
        let captured_at = Utc.with_ymd_and_hms(2026, 7, 31, 0, 0, 0).single().unwrap();
        let exact = classify(search(&[101]), &[(101, landing(101, "NYR", 5))]);
        let board = build_official_identity_candidate_board(
            20_262_027,
            "package".to_owned(),
            "a".repeat(64),
            "2026-07-31T00:00:00Z".to_owned(),
            "2026-07-31T00:00:00Z".to_owned(),
            "2026-07-31T00:00:00Z".to_owned(),
            vec![exact],
        )
        .unwrap();
        let document = build_official_identity_review_ledger(
            &board,
            "official_identity_review",
            "https://example.test/identity-candidates",
            "fixture-reviewer",
            captured_at,
        )
        .unwrap();
        assert_eq!(document.decisions.len(), 1);
        assert_eq!(document.decisions[0].player_id, Some(101));
        assert_eq!(document.decisions[0].evidence.len(), 3);
        let bytes = serde_json::to_vec(&document).unwrap();
        let adapter = IdentityReviewLedgerV1Adapter;
        let parsed = adapter
            .parse(SourceInput::new(
                &bytes,
                adapter.descriptor().source_id,
                hash(&bytes).unwrap(),
            ))
            .unwrap();
        assert_eq!(parsed.decisions.len(), 1);
    }

    #[tokio::test]
    async fn offline_mode_classifies_an_empty_cache_without_network() {
        let temp = tempfile::tempdir().unwrap();
        let workboard = IdentityReviewWorkboardView {
            schema: icelines_core::IDENTITY_REVIEW_WORKBOARD_SCHEMA.to_owned(),
            evaluation_season: 20_262_027,
            source_package_id: "package".to_owned(),
            source_package_fingerprint: "a".repeat(64),
            effective_cutoff: "2026-07-31T00:00:00Z".to_owned(),
            knowledge_cutoff: "2026-07-31T00:00:00Z".to_owned(),
            unresolved_count: 1,
            family_counts: vec![IdentityReviewFamilyCount {
                family: "draft".to_owned(),
                proposals: 1,
            }],
            rows: vec![row()],
            disclosures: vec!["fixture".to_owned()],
        };
        let mut options = OfficialIdentityAcquisitionOptions::new(temp.path());
        options.offline = true;
        let board = acquire_official_identity_candidates(&workboard, options)
            .await
            .unwrap();
        assert_eq!(board.evaluated_count, 1);
        assert_eq!(board.eligible_count, 0);
        assert_eq!(
            board.rows[0].status,
            OfficialIdentityCandidateStatus::ProviderFailure
        );
        assert!(!temp.path().join("cache-manifest.json").exists());
    }

    #[test]
    fn cached_search_requests_are_removed_before_network_chunking() {
        let requests = (0..(SEARCH_BATCH_SIZE + 1))
            .map(|index| {
                (
                    format!("search-{index}"),
                    format!("https://example.test/search/{index}"),
                )
            })
            .collect::<Vec<_>>();
        let cached = requests
            .iter()
            .map(|(dataset_id, _)| (dataset_id.clone(), dataset_id.as_bytes().to_vec()))
            .collect::<BTreeMap<_, _>>();
        let mut results = Vec::new();

        let pending = partition_cached_search_requests(requests, cached, &mut results);

        assert!(pending.is_empty());
        assert_eq!(results.len(), SEARCH_BATCH_SIZE + 1);
        assert!(results.iter().all(|(_, result)| result.is_ok()));
    }
}
