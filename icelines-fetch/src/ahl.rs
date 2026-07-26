//! Official AHL roster and season-stat ingestion.
//!
//! The AHL statistics pages are backed by HockeyTech's Statview feed.  The
//! identifiers returned by that feed are provider-local: an AHL `player_id`
//! is never an NHL player id.  This module preserves that boundary explicitly
//! and leaves identity linking to a reviewed crosswalk.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub const AHL_ROSTER_STATS_SCHEMA: &str = "ahl_roster_stats.v1";
pub const AHL_CANONICAL_IDENTITY_CATALOG_SCHEMA: &str = "ahl_canonical_identity_catalog.v1";
pub const AHL_IDENTITY_CROSSWALK_SCHEMA: &str = "ahl_identity_crosswalk.v1";
pub const AHL_IDENTITY_LEAGUE_CROSSWALK_SCHEMA: &str = "ahl_identity_league_crosswalk.v1";
pub const AHL_IDENTITY_LEAGUE_REVIEW_DECISIONS_SCHEMA: &str =
    "ahl_identity_league_review_decisions.v1";
pub const AHL_IDENTITY_LEAGUE_REVIEW_DRAFT_SCHEMA: &str = "ahl_identity_league_review_draft.v1";
pub const AHL_IDENTITY_REVIEW_INSPECTION_SCHEMA: &str = "ahl_identity_review_inspection.v1";
pub const AHL_IDENTITY_REVIEW_DECISIONS_SCHEMA: &str = "ahl_identity_review_decisions.v1";
pub const AHL_IDENTITY_LEAGUE_REVIEW_SCHEMA: &str = "ahl_identity_league_review.v1";
pub const AHL_IDENTITY_EXCEPTION_BOARD_SCHEMA: &str = "ahl_identity_exception_board.v1";
pub const AHL_IDENTITY_COLLISION_DELTA_DAYS: u32 = 1_460;
pub const AHL_PROVIDER: &str = "ahl_hockeytech_statview";
pub const AHL_STATS_SOURCE_URL: &str = "https://theahl.com/stats/player-stats";
pub const AHL_ROSTER_SOURCE_URL: &str = "https://theahl.com/stats/roster";
pub const AHL_FEED_BASE_URL: &str = "https://lscluster.hockeytech.com/feed/index.php";
const AHL_FEED_KEY: &str = "ccb91f29d6744675";
const AHL_CLIENT_CODE: &str = "ahl";

#[derive(Debug, Error)]
pub enum AhlFeedError {
    #[error("AHL feed request failed for {url}: {detail}")]
    Request { url: String, detail: String },
    #[error("AHL feed returned HTTP {status} for {url}")]
    Http { status: u16, url: String },
    #[error("AHL feed schema changed: {0}")]
    Schema(String),
    #[error("AHL season not found: {0}")]
    SeasonNotFound(String),
    #[error("unknown AHL team filter(s): {0}")]
    UnknownTeams(String),
    #[error("invalid AHL snapshot: {0}")]
    Validation(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlRosterStatsSnapshot {
    pub schema: String,
    pub season: u32,
    pub provider: String,
    pub provider_season_id: String,
    pub provider_season_name: String,
    pub fetched_at: String,
    pub source_url: String,
    pub roster_source_url: String,
    pub identity_note: String,
    pub teams: Vec<AhlTeamRosterStats>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlTeamRosterStats {
    pub provider: String,
    pub provider_team_id: String,
    pub team_code: String,
    pub team_name: String,
    pub nickname: String,
    pub division_id: String,
    pub logo_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nhl_affiliate: Option<String>,
    /// Official season roster. It may be empty before the AHL publishes the
    /// club roster and can include players who appeared earlier in-season.
    pub roster: Vec<AhlRosterPlayer>,
    pub skaters: Vec<AhlSkaterSeasonRow>,
    pub goalies: Vec<AhlGoalieSeasonRow>,
    /// Provider rows excluded from typed team stats with an auditable reason.
    #[serde(default)]
    pub source_warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlRosterPlayer {
    pub provider: String,
    pub provider_player_id: String,
    pub name: String,
    pub position_group: String,
    pub position: String,
    pub jersey_number: String,
    pub handedness: String,
    pub height: String,
    pub weight_pounds: String,
    pub birthdate: String,
    pub birthplace: String,
}

/// Explicit bridge from one provider-scoped AHL roster identity to the
/// canonical NHL identity and scenario facts required by the core projection.
/// No AHL id is ever copied into `nhl_player_id` implicitly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlProjectionPlayerEnrichment {
    pub provider_player_id: String,
    pub nhl_player_id: u32,
    pub primary_position: icelines_core::model::Position,
    pub eligible_positions: Vec<icelines_core::model::Position>,
    pub projected_score: f64,
    #[serde(default)]
    pub prospect: bool,
    #[serde(default)]
    pub recall_readiness: Option<f64>,
    #[serde(default)]
    pub professional_games_at_season_start: Option<u32>,
    #[serde(default = "default_true")]
    pub assigned_to_affiliate: bool,
    #[serde(default)]
    pub waiver_required: bool,
}

/// Canonical NHL identity candidates from reviewed NHL roster, draft, or
/// player-profile authorities. This catalog proposes links; it never approves
/// them automatically.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlCanonicalIdentityCatalog {
    pub schema: String,
    pub checked_at: String,
    pub candidates: Vec<AhlCanonicalIdentityCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlCanonicalIdentityCandidate {
    pub nhl_player_id: u32,
    pub display_name: String,
    #[serde(default)]
    pub birth_date: Option<String>,
    pub evidence_urls: Vec<String>,
}

/// Canonicalize provider formatting differences for identity comparison.
/// Hyphens become word boundaries while apostrophes and periods are ignored,
/// so provider punctuation variants compare equally without changing global
/// name search.
pub fn normalize_ahl_identity_name(name: &str) -> String {
    let normalized = icelines_core::normalize_name(name);
    let mut identity = String::with_capacity(normalized.len());
    let mut pending_boundary = false;
    for character in normalized.chars() {
        if character.is_alphanumeric() {
            if pending_boundary && !identity.is_empty() {
                identity.push(' ');
            }
            identity.push(character);
            pending_boundary = false;
        } else if character.is_whitespace() || character == '-' {
            pending_boundary = true;
        }
    }
    identity
}

/// Return provider search spellings without discarding the supplied form.
/// Some official search indexes distinguish curly and straight apostrophes
/// even though the resulting identities compare equally.
pub fn ahl_identity_search_name_variants(name: &str) -> Vec<String> {
    let straight_apostrophes = name
        .chars()
        .map(|character| match character {
            '‘' | '’' => '\'',
            _ => character,
        })
        .collect::<String>();
    if straight_apostrophes == name {
        vec![name.to_owned()]
    } else {
        vec![name.to_owned(), straight_apostrophes]
    }
}

/// Parse exact-name candidates from the official NHL player-search response.
/// Search results establish a player ID/name proposal only; birth-date
/// corroboration comes from the matching NHL player landing document.
pub fn parse_official_nhl_search_candidates(
    expected_name: &str,
    source_url: &str,
    bytes: &[u8],
) -> Result<Vec<AhlCanonicalIdentityCandidate>, AhlFeedError> {
    let expected = normalize_ahl_identity_name(expected_name);
    parse_official_nhl_search_candidates_matching(expected_name, source_url, bytes, |name| {
        normalize_ahl_identity_name(name) == expected
    })
}

/// Parse official search candidates sharing the expected surname. This is a
/// discovery expansion only; the crosswalk still requires exact birth-date
/// corroboration and explicit alias review.
pub fn parse_official_nhl_search_candidates_by_surname(
    expected_name: &str,
    source_url: &str,
    bytes: &[u8],
) -> Result<Vec<AhlCanonicalIdentityCandidate>, AhlFeedError> {
    let expected_surname = normalized_surname(expected_name).ok_or_else(|| {
        AhlFeedError::Validation(format!("cannot derive surname from `{expected_name}`"))
    })?;
    parse_official_nhl_search_candidates_matching(expected_name, source_url, bytes, |name| {
        normalized_surname(name).as_deref() == Some(expected_surname.as_str())
    })
}

fn parse_official_nhl_search_candidates_matching(
    expected_name: &str,
    source_url: &str,
    bytes: &[u8],
    matches: impl Fn(&str) -> bool,
) -> Result<Vec<AhlCanonicalIdentityCandidate>, AhlFeedError> {
    let rows: Vec<Value> = serde_json::from_slice(bytes).map_err(|error| {
        AhlFeedError::Schema(format!("invalid NHL player-search JSON: {error}"))
    })?;
    let mut candidates = Vec::new();
    let mut ids = BTreeSet::new();
    for row in rows {
        let Some(name) = row.get("name").and_then(Value::as_str) else {
            continue;
        };
        if !matches(name) {
            continue;
        }
        let player_id = row
            .get("playerId")
            .and_then(|value| {
                value
                    .as_u64()
                    .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
            })
            .and_then(|value| u32::try_from(value).ok())
            .filter(|value| *value != 0)
            .ok_or_else(|| {
                AhlFeedError::Schema(format!(
                    "matching NHL player-search result for `{expected_name}` has no valid playerId"
                ))
            })?;
        if ids.insert(player_id) {
            candidates.push(AhlCanonicalIdentityCandidate {
                nhl_player_id: player_id,
                display_name: name.to_owned(),
                birth_date: None,
                evidence_urls: vec![source_url.to_owned()],
            });
        }
    }
    candidates.sort_by_key(|candidate| candidate.nhl_player_id);
    Ok(candidates)
}

fn normalized_surname(name: &str) -> Option<String> {
    normalize_ahl_identity_name(name)
        .split_whitespace()
        .last()
        .map(str::to_owned)
}

/// Enrich a search proposal from the official NHL player landing response.
pub fn enrich_official_nhl_landing_candidate(
    candidate: &AhlCanonicalIdentityCandidate,
    source_url: &str,
    bytes: &[u8],
) -> Result<AhlCanonicalIdentityCandidate, AhlFeedError> {
    let row: Value = serde_json::from_slice(bytes)
        .map_err(|error| AhlFeedError::Schema(format!("invalid NHL landing JSON: {error}")))?;
    let player_id = row
        .get("playerId")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok());
    let first_name = localized_default(row.get("firstName"));
    let last_name = localized_default(row.get("lastName"));
    let display_name = format!("{first_name} {last_name}").trim().to_owned();
    if player_id != Some(candidate.nhl_player_id)
        || display_name.is_empty()
        || normalize_ahl_identity_name(&display_name)
            != normalize_ahl_identity_name(&candidate.display_name)
    {
        return Err(AhlFeedError::Validation(format!(
            "NHL landing identity conflicts with search proposal {} ({})",
            candidate.nhl_player_id, candidate.display_name
        )));
    }
    let birth_date = row
        .get("birthDate")
        .and_then(Value::as_str)
        .map(str::to_owned);
    if birth_date
        .as_deref()
        .is_some_and(|date| chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err())
    {
        return Err(AhlFeedError::Schema(format!(
            "NHL landing identity {} has an invalid birthDate",
            candidate.nhl_player_id
        )));
    }
    let mut evidence_urls = candidate.evidence_urls.clone();
    evidence_urls.push(source_url.to_owned());
    evidence_urls.sort();
    evidence_urls.dedup();
    Ok(AhlCanonicalIdentityCandidate {
        nhl_player_id: candidate.nhl_player_id,
        display_name,
        birth_date,
        evidence_urls,
    })
}

/// Merge independently sourced NHL identity catalogs by canonical player ID.
/// Conflicting names or birth dates fail closed.
pub fn merge_ahl_canonical_identity_catalogs(
    checked_at: impl Into<String>,
    catalogs: &[AhlCanonicalIdentityCatalog],
) -> Result<AhlCanonicalIdentityCatalog, AhlFeedError> {
    let checked_at = checked_at.into();
    let mut merged = BTreeMap::<u32, AhlCanonicalIdentityCandidate>::new();
    for catalog in catalogs {
        validate_identity_catalog_authority(catalog)?;
        for candidate in &catalog.candidates {
            validate_identity_candidate(candidate)?;
            match merged.entry(candidate.nhl_player_id) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(candidate.clone());
                }
                std::collections::btree_map::Entry::Occupied(mut entry) => {
                    let current = entry.get_mut();
                    if normalize_ahl_identity_name(&current.display_name)
                        != normalize_ahl_identity_name(&candidate.display_name)
                        || matches!(
                            (&current.birth_date, &candidate.birth_date),
                            (Some(left), Some(right)) if left != right
                        )
                    {
                        return Err(AhlFeedError::Validation(format!(
                            "canonical NHL identity sources conflict for player {}",
                            candidate.nhl_player_id
                        )));
                    }
                    if current.birth_date.is_none() {
                        current.birth_date.clone_from(&candidate.birth_date);
                    }
                    current
                        .evidence_urls
                        .extend(candidate.evidence_urls.clone());
                    current.evidence_urls.sort();
                    current.evidence_urls.dedup();
                }
            }
        }
    }
    let catalog = AhlCanonicalIdentityCatalog {
        schema: AHL_CANONICAL_IDENTITY_CATALOG_SCHEMA.to_owned(),
        checked_at,
        candidates: merged.into_values().collect(),
    };
    validate_identity_catalog(&catalog)?;
    Ok(catalog)
}

fn localized_default(value: Option<&Value>) -> &str {
    value
        .and_then(|value| {
            value
                .as_str()
                .or_else(|| value.get("default").and_then(Value::as_str))
        })
        .unwrap_or("")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AhlIdentityMatchBasis {
    ExactNameAndBirthDate,
    SurnameAndBirthDate,
    ExactNameOnly,
    BirthDateConflict,
    Ambiguous,
    Unmatched,
    ReviewedOverride,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AhlIdentityReviewStatus {
    Pending,
    Reviewed,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AhlIdentityReviewAction {
    AcceptProposal,
    SetIdentity,
    Reject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlIdentityReviewDecision {
    pub provider_player_id: String,
    pub action: AhlIdentityReviewAction,
    #[serde(default)]
    pub nhl_player_id: Option<u32>,
    #[serde(default)]
    pub nhl_display_name: Option<String>,
    #[serde(default)]
    pub nhl_birth_date: Option<String>,
    #[serde(default)]
    pub evidence_urls: Vec<String>,
    pub note: String,
}

/// Separately authored approval authority. A generated draft has `draft=true`
/// and cannot be applied until a reviewer finalizes its identity and timestamp.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlIdentityReviewDecisions {
    pub schema: String,
    pub season: u32,
    pub provider: String,
    pub ahl_team: String,
    pub roster_fetched_at: String,
    pub draft: bool,
    #[serde(default)]
    pub reviewer: Option<String>,
    #[serde(default)]
    pub reviewed_at: Option<String>,
    pub decisions: Vec<AhlIdentityReviewDecision>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlIdentityCrosswalkRow {
    pub provider_player_id: String,
    pub ahl_display_name: String,
    pub ahl_birth_date: String,
    pub match_basis: AhlIdentityMatchBasis,
    pub review_status: AhlIdentityReviewStatus,
    #[serde(default)]
    pub nhl_player_id: Option<u32>,
    #[serde(default)]
    pub nhl_display_name: Option<String>,
    #[serde(default)]
    pub nhl_birth_date: Option<String>,
    #[serde(default)]
    pub evidence_urls: Vec<String>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlIdentityCrosswalkCounts {
    pub roster_players: usize,
    pub exact_name_and_birth_date: usize,
    #[serde(default)]
    pub surname_and_birth_date: usize,
    pub exact_name_only: usize,
    pub ambiguous: usize,
    pub conflicts: usize,
    pub unmatched: usize,
    pub reviewed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlIdentityCrosswalkView {
    pub schema: String,
    pub season: u32,
    pub provider: String,
    pub ahl_team: String,
    pub nhl_affiliate: Option<String>,
    pub roster_fetched_at: String,
    pub candidates_checked_at: String,
    pub counts: AhlIdentityCrosswalkCounts,
    pub rows: Vec<AhlIdentityCrosswalkRow>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlIdentityLeagueCrosswalkView {
    pub schema: String,
    pub season: u32,
    pub provider: String,
    pub roster_fetched_at: String,
    pub candidates_checked_at: String,
    pub teams: usize,
    pub roster_appearances: usize,
    pub unique_provider_players: usize,
    pub crosswalks: Vec<AhlIdentityCrosswalkView>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AhlIdentityLeagueRoutineReviewKind {
    Exact,
    Aliases,
    Conflicts,
    CollisionRemaps,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlIdentityLeagueReviewDecisionsView {
    pub schema: String,
    pub season: u32,
    pub provider: String,
    pub roster_fetched_at: String,
    pub kind: AhlIdentityLeagueRoutineReviewKind,
    pub reviewer: String,
    pub reviewed_at: String,
    pub eligible_teams: usize,
    pub skipped_teams: Vec<String>,
    pub applied_decisions: usize,
    pub batches: Vec<AhlIdentityReviewDecisions>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlIdentityLeagueReviewDraftView {
    pub schema: String,
    pub season: u32,
    pub provider: String,
    pub roster_fetched_at: String,
    pub include_aliases: bool,
    pub include_conflicts: bool,
    pub eligible_teams: usize,
    pub skipped_teams: Vec<String>,
    pub proposed_decisions: usize,
    pub pending_without_proposal: usize,
    pub batches: Vec<AhlIdentityReviewDecisions>,
    pub disclosures: Vec<String>,
}

/// Build every team identity queue in a season snapshot against one canonical
/// candidate catalog. Review state remains pending in each child crosswalk.
pub fn build_ahl_identity_league_crosswalk(
    snapshot: &AhlRosterStatsSnapshot,
    candidates: &AhlCanonicalIdentityCatalog,
) -> Result<AhlIdentityLeagueCrosswalkView, AhlFeedError> {
    snapshot.validate()?;
    let mut team_names = snapshot
        .teams
        .iter()
        .map(|team| team.team_name.clone())
        .collect::<Vec<_>>();
    team_names.sort();
    let mut crosswalks = Vec::with_capacity(team_names.len());
    let mut unique_provider_players = BTreeSet::new();
    let mut roster_appearances = 0usize;
    for team_name in team_names {
        let crosswalk = build_ahl_identity_crosswalk(snapshot, &team_name, candidates)?;
        roster_appearances += crosswalk.rows.len();
        unique_provider_players.extend(
            crosswalk
                .rows
                .iter()
                .map(|row| row.provider_player_id.clone()),
        );
        crosswalks.push(crosswalk);
    }
    Ok(AhlIdentityLeagueCrosswalkView {
        schema: AHL_IDENTITY_LEAGUE_CROSSWALK_SCHEMA.to_owned(),
        season: snapshot.season,
        provider: snapshot.provider.clone(),
        roster_fetched_at: snapshot.fetched_at.clone(),
        candidates_checked_at: candidates.checked_at.clone(),
        teams: crosswalks.len(),
        roster_appearances,
        unique_provider_players: unique_provider_players.len(),
        crosswalks,
        disclosures: vec![
            "League identity acquisition applies one canonical candidate catalog to every team in the sealed AHL season snapshot; every proposal remains pending explicit review.".to_owned(),
            "Roster appearances count team-season rows. Unique provider players deduplicate AHL provider IDs across clubs without claiming NHL identity.".to_owned(),
        ],
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AhlIdentityInspectionScope {
    All,
    Attention,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlIdentityReviewInspectionView {
    pub schema: String,
    pub season: u32,
    pub provider: String,
    pub ahl_team: String,
    pub roster_fetched_at: String,
    pub candidates_checked_at: String,
    pub scope: AhlIdentityInspectionScope,
    pub total_rows: usize,
    pub attention_count: usize,
    pub declared_counts: AhlIdentityCrosswalkCounts,
    pub computed_counts: AhlIdentityCrosswalkCounts,
    pub declared_counts_stale: bool,
    pub rows: Vec<AhlIdentityCrosswalkRow>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlIdentityLeagueReviewSummary {
    pub season: u32,
    pub provider: String,
    pub ahl_team: String,
    pub nhl_affiliate: Option<String>,
    pub roster_fetched_at: String,
    pub roster_players: usize,
    pub reviewed: usize,
    pub rejected: usize,
    pub pending: usize,
    pub attention: usize,
    pub declared_counts_stale: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlIdentityLeagueReviewAppearance {
    pub season: u32,
    pub provider: String,
    pub ahl_team: String,
    pub provider_player_id: String,
    pub ahl_birth_date: String,
    pub nhl_birth_date: Option<String>,
    pub review_status: AhlIdentityReviewStatus,
    pub match_basis: AhlIdentityMatchBasis,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlIdentityLeagueAttentionGroup {
    pub identity_key: String,
    pub ahl_display_name: String,
    pub nhl_player_id: Option<u32>,
    pub nhl_display_name: Option<String>,
    pub occurrences: usize,
    pub review_statuses: Vec<AhlIdentityReviewStatus>,
    pub match_bases: Vec<AhlIdentityMatchBasis>,
    pub evidence_urls: Vec<String>,
    pub appearances: Vec<AhlIdentityLeagueReviewAppearance>,
}

/// League-scale, UI-neutral coverage and exception queue composed from
/// independently snapshot-bound team-season crosswalks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlIdentityLeagueReviewView {
    pub schema: String,
    pub crosswalks: usize,
    pub roster_appearances: usize,
    pub reviewed: usize,
    pub rejected: usize,
    pub pending: usize,
    /// Reviewed or explicitly rejected rows, in basis points.
    pub resolved_basis_points: u16,
    /// Rows with a reviewed canonical NHL identity, in basis points.
    pub canonical_identity_basis_points: u16,
    pub summaries: Vec<AhlIdentityLeagueReviewSummary>,
    pub attention_groups: Vec<AhlIdentityLeagueAttentionGroup>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AhlIdentityExceptionAction {
    ApplyRoutineExact,
    ApplyRoutineAlias,
    ResolveBirthDateConflict,
    InvestigateIdentityCollision,
    InspectAmbiguous,
    AcquireCanonicalEvidence,
    AuditRejectedMapping,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlIdentityConflictDatePair {
    pub ahl_birth_date: String,
    pub nhl_birth_date: String,
    pub absolute_delta_days: u32,
    pub appearances: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlIdentityExceptionBoardRow {
    pub rank: usize,
    pub priority_score: u16,
    pub identity_key: String,
    pub ahl_display_name: String,
    pub nhl_player_id: Option<u32>,
    pub nhl_display_name: Option<String>,
    pub occurrences: usize,
    pub seasons: Vec<u32>,
    pub ahl_teams: Vec<String>,
    pub review_statuses: Vec<AhlIdentityReviewStatus>,
    pub match_bases: Vec<AhlIdentityMatchBasis>,
    pub recommended_action: AhlIdentityExceptionAction,
    pub conflict_date_pairs: Vec<AhlIdentityConflictDatePair>,
    pub evidence_urls: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlIdentityExceptionBoardView {
    pub schema: String,
    pub source_schema: String,
    pub groups: usize,
    pub appearances: usize,
    pub rows: Vec<AhlIdentityExceptionBoardRow>,
    pub disclosures: Vec<String>,
}

pub fn build_ahl_identity_league_review(
    crosswalks: &[AhlIdentityCrosswalkView],
) -> Result<AhlIdentityLeagueReviewView, AhlFeedError> {
    if crosswalks.is_empty() {
        return Err(AhlFeedError::Validation(
            "league identity review requires at least one crosswalk".to_owned(),
        ));
    }
    let mut ordered = crosswalks.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| {
        left.season
            .cmp(&right.season)
            .then_with(|| left.ahl_team.cmp(&right.ahl_team))
            .then_with(|| left.provider.cmp(&right.provider))
    });
    let mut bindings = BTreeSet::new();
    let mut summaries = Vec::with_capacity(ordered.len());
    let mut attention = BTreeMap::<String, AhlIdentityLeagueAttentionGroup>::new();
    let mut roster_appearances = 0usize;
    let mut reviewed = 0usize;
    let mut rejected = 0usize;

    for crosswalk in ordered {
        let binding = (
            crosswalk.season,
            crosswalk.provider.as_str(),
            crosswalk.ahl_team.as_str(),
        );
        if !bindings.insert(binding) {
            return Err(AhlFeedError::Validation(format!(
                "duplicate league identity crosswalk for {} {} {}",
                crosswalk.season, crosswalk.provider, crosswalk.ahl_team
            )));
        }
        let inspection =
            build_ahl_identity_review_inspection(crosswalk, AhlIdentityInspectionScope::All)?;
        let team_reviewed = crosswalk
            .rows
            .iter()
            .filter(|row| row.review_status == AhlIdentityReviewStatus::Reviewed)
            .count();
        let team_rejected = crosswalk
            .rows
            .iter()
            .filter(|row| row.review_status == AhlIdentityReviewStatus::Rejected)
            .count();
        let team_pending = crosswalk.rows.len() - team_reviewed - team_rejected;
        roster_appearances += crosswalk.rows.len();
        reviewed += team_reviewed;
        rejected += team_rejected;
        summaries.push(AhlIdentityLeagueReviewSummary {
            season: crosswalk.season,
            provider: crosswalk.provider.clone(),
            ahl_team: crosswalk.ahl_team.clone(),
            nhl_affiliate: crosswalk.nhl_affiliate.clone(),
            roster_fetched_at: crosswalk.roster_fetched_at.clone(),
            roster_players: crosswalk.rows.len(),
            reviewed: team_reviewed,
            rejected: team_rejected,
            pending: team_pending,
            attention: team_pending + team_rejected,
            declared_counts_stale: inspection.declared_counts_stale,
        });

        for row in crosswalk
            .rows
            .iter()
            .filter(|row| row.review_status != AhlIdentityReviewStatus::Reviewed)
        {
            let identity_key = row.nhl_player_id.map_or_else(
                || {
                    format!(
                        "ahl:{}:{}",
                        normalize_ahl_identity_name(&row.ahl_display_name),
                        row.ahl_birth_date
                    )
                },
                |player_id| format!("nhl:{player_id}"),
            );
            let group = attention.entry(identity_key.clone()).or_insert_with(|| {
                AhlIdentityLeagueAttentionGroup {
                    identity_key,
                    ahl_display_name: row.ahl_display_name.clone(),
                    nhl_player_id: row.nhl_player_id,
                    nhl_display_name: row.nhl_display_name.clone(),
                    occurrences: 0,
                    review_statuses: Vec::new(),
                    match_bases: Vec::new(),
                    evidence_urls: Vec::new(),
                    appearances: Vec::new(),
                }
            });
            group.occurrences += 1;
            if !group.review_statuses.contains(&row.review_status) {
                group.review_statuses.push(row.review_status);
            }
            if !group.match_bases.contains(&row.match_basis) {
                group.match_bases.push(row.match_basis);
            }
            for url in &row.evidence_urls {
                if !group.evidence_urls.contains(url) {
                    group.evidence_urls.push(url.clone());
                }
            }
            group.appearances.push(AhlIdentityLeagueReviewAppearance {
                season: crosswalk.season,
                provider: crosswalk.provider.clone(),
                ahl_team: crosswalk.ahl_team.clone(),
                provider_player_id: row.provider_player_id.clone(),
                ahl_birth_date: row.ahl_birth_date.clone(),
                nhl_birth_date: row.nhl_birth_date.clone(),
                review_status: row.review_status,
                match_basis: row.match_basis,
                note: row.note.clone(),
            });
        }
    }

    let pending = roster_appearances - reviewed - rejected;
    let mut attention_groups = attention.into_values().collect::<Vec<_>>();
    for group in &mut attention_groups {
        group.evidence_urls.sort();
        group.appearances.sort_by(|left, right| {
            left.season
                .cmp(&right.season)
                .then_with(|| left.ahl_team.cmp(&right.ahl_team))
                .then_with(|| left.provider_player_id.cmp(&right.provider_player_id))
        });
    }
    attention_groups.sort_by(|left, right| {
        left.ahl_display_name
            .cmp(&right.ahl_display_name)
            .then_with(|| left.identity_key.cmp(&right.identity_key))
    });
    Ok(AhlIdentityLeagueReviewView {
        schema: AHL_IDENTITY_LEAGUE_REVIEW_SCHEMA.to_owned(),
        crosswalks: summaries.len(),
        roster_appearances,
        reviewed,
        rejected,
        pending,
        resolved_basis_points: coverage_basis_points(reviewed + rejected, roster_appearances),
        canonical_identity_basis_points: coverage_basis_points(reviewed, roster_appearances),
        summaries,
        attention_groups,
        disclosures: vec![
            "League coverage composes independently reviewed, snapshot-bound team-season crosswalks; it does not create or approve identity decisions.".to_owned(),
            "Attention groups contain every pending or rejected appearance. Canonical NHL IDs are the strongest grouping key; AHL name plus birth date is used only when no NHL ID exists.".to_owned(),
            "Resolved coverage includes explicit mapping rejections, while canonical identity coverage counts only reviewed NHL identities.".to_owned(),
        ],
    })
}

/// Rank the read-only league exception queue by review leverage. This board
/// creates no identity authority and never changes review state.
pub fn build_ahl_identity_exception_board(
    review: &AhlIdentityLeagueReviewView,
) -> Result<AhlIdentityExceptionBoardView, AhlFeedError> {
    if review.schema != AHL_IDENTITY_LEAGUE_REVIEW_SCHEMA {
        return Err(AhlFeedError::Validation(
            "identity exception board requires an AHL league review authority".to_owned(),
        ));
    }
    let mut rows = Vec::with_capacity(review.attention_groups.len());
    let mut identity_keys = BTreeSet::new();
    for group in &review.attention_groups {
        if group.identity_key.trim().is_empty()
            || !identity_keys.insert(group.identity_key.as_str())
            || group.occurrences == 0
            || group.occurrences != group.appearances.len()
        {
            return Err(AhlFeedError::Validation(format!(
                "identity exception group `{}` has stale occurrence coverage",
                group.identity_key
            )));
        }
        let pending_bases = group
            .appearances
            .iter()
            .filter(|appearance| appearance.review_status == AhlIdentityReviewStatus::Pending)
            .map(|appearance| appearance.match_basis)
            .collect::<Vec<_>>();
        let mut collision_scale_delta = false;
        for appearance in group.appearances.iter().filter(|appearance| {
            appearance.review_status == AhlIdentityReviewStatus::Pending
                && appearance.match_basis == AhlIdentityMatchBasis::BirthDateConflict
        }) {
            let Some(nhl_birth_date) = appearance.nhl_birth_date.as_deref() else {
                continue;
            };
            let ahl_date =
                chrono::NaiveDate::parse_from_str(&appearance.ahl_birth_date, "%Y-%m-%d").map_err(
                    |_| {
                        AhlFeedError::Validation(format!(
                            "identity exception {} has invalid AHL conflict date",
                            group.identity_key
                        ))
                    },
                )?;
            let nhl_date =
                chrono::NaiveDate::parse_from_str(nhl_birth_date, "%Y-%m-%d").map_err(|_| {
                    AhlFeedError::Validation(format!(
                        "identity exception {} has invalid NHL conflict date",
                        group.identity_key
                    ))
                })?;
            if u32::try_from((ahl_date - nhl_date).num_days().unsigned_abs()).unwrap_or(u32::MAX)
                >= AHL_IDENTITY_COLLISION_DELTA_DAYS
            {
                collision_scale_delta = true;
            }
        }
        let recommended_action =
            if pending_bases.contains(&AhlIdentityMatchBasis::ExactNameAndBirthDate) {
                AhlIdentityExceptionAction::ApplyRoutineExact
            } else if pending_bases.contains(&AhlIdentityMatchBasis::SurnameAndBirthDate) {
                AhlIdentityExceptionAction::ApplyRoutineAlias
            } else if pending_bases.contains(&AhlIdentityMatchBasis::BirthDateConflict)
                && collision_scale_delta
            {
                AhlIdentityExceptionAction::InvestigateIdentityCollision
            } else if pending_bases.contains(&AhlIdentityMatchBasis::BirthDateConflict) {
                AhlIdentityExceptionAction::ResolveBirthDateConflict
            } else if pending_bases.contains(&AhlIdentityMatchBasis::Ambiguous) {
                AhlIdentityExceptionAction::InspectAmbiguous
            } else if !pending_bases.is_empty() {
                AhlIdentityExceptionAction::AcquireCanonicalEvidence
            } else {
                AhlIdentityExceptionAction::AuditRejectedMapping
            };
        let seasons = group
            .appearances
            .iter()
            .map(|appearance| appearance.season)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let ahl_teams = group
            .appearances
            .iter()
            .map(|appearance| appearance.ahl_team.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let mut date_pair_counts = BTreeMap::<(String, String), usize>::new();
        for appearance in group
            .appearances
            .iter()
            .filter(|appearance| appearance.match_basis == AhlIdentityMatchBasis::BirthDateConflict)
        {
            if let Some(nhl_birth_date) = appearance.nhl_birth_date.as_deref() {
                *date_pair_counts
                    .entry((appearance.ahl_birth_date.clone(), nhl_birth_date.to_owned()))
                    .or_default() += 1;
            }
        }
        let mut conflict_date_pairs = Vec::with_capacity(date_pair_counts.len());
        for ((ahl_birth_date, nhl_birth_date), appearances) in date_pair_counts {
            let ahl_date =
                chrono::NaiveDate::parse_from_str(&ahl_birth_date, "%Y-%m-%d").map_err(|_| {
                    AhlFeedError::Validation(format!(
                        "identity exception {} has invalid AHL conflict date",
                        group.identity_key
                    ))
                })?;
            let nhl_date =
                chrono::NaiveDate::parse_from_str(&nhl_birth_date, "%Y-%m-%d").map_err(|_| {
                    AhlFeedError::Validation(format!(
                        "identity exception {} has invalid NHL conflict date",
                        group.identity_key
                    ))
                })?;
            conflict_date_pairs.push(AhlIdentityConflictDatePair {
                ahl_birth_date,
                nhl_birth_date,
                absolute_delta_days: u32::try_from((ahl_date - nhl_date).num_days().unsigned_abs())
                    .unwrap_or(u32::MAX),
                appearances,
            });
        }
        let base_score = match recommended_action {
            AhlIdentityExceptionAction::ApplyRoutineExact => 80u16,
            AhlIdentityExceptionAction::InvestigateIdentityCollision => 75,
            AhlIdentityExceptionAction::ApplyRoutineAlias => 70,
            AhlIdentityExceptionAction::ResolveBirthDateConflict => 60,
            AhlIdentityExceptionAction::InspectAmbiguous => 50,
            AhlIdentityExceptionAction::AcquireCanonicalEvidence => 40,
            AhlIdentityExceptionAction::AuditRejectedMapping => 20,
        };
        let recurrence_score = u16::try_from(group.occurrences.min(20)).unwrap_or(20) * 5;
        let season_score = u16::try_from(seasons.len().saturating_sub(1).min(10)).unwrap_or(10) * 8;
        let team_score = u16::try_from(ahl_teams.len().saturating_sub(1).min(10)).unwrap_or(10) * 4;
        let evidence_ready_score = if recommended_action
            == AhlIdentityExceptionAction::ResolveBirthDateConflict
            && group.nhl_player_id.is_some()
            && !conflict_date_pairs.is_empty()
            && group.evidence_urls.len() >= 2
        {
            10
        } else {
            0
        };
        rows.push(AhlIdentityExceptionBoardRow {
            rank: 0,
            priority_score: base_score
                + recurrence_score
                + season_score
                + team_score
                + evidence_ready_score,
            identity_key: group.identity_key.clone(),
            ahl_display_name: group.ahl_display_name.clone(),
            nhl_player_id: group.nhl_player_id,
            nhl_display_name: group.nhl_display_name.clone(),
            occurrences: group.occurrences,
            seasons,
            ahl_teams,
            review_statuses: group.review_statuses.clone(),
            match_bases: group.match_bases.clone(),
            recommended_action,
            conflict_date_pairs,
            evidence_urls: group.evidence_urls.clone(),
        });
    }
    rows.sort_by(|left, right| {
        right
            .priority_score
            .cmp(&left.priority_score)
            .then_with(|| right.occurrences.cmp(&left.occurrences))
            .then_with(|| left.ahl_display_name.cmp(&right.ahl_display_name))
            .then_with(|| left.identity_key.cmp(&right.identity_key))
    });
    for (index, row) in rows.iter_mut().enumerate() {
        row.rank = index + 1;
    }
    let appearances = rows.iter().map(|row| row.occurrences).sum::<usize>();
    if appearances != review.pending + review.rejected {
        return Err(AhlFeedError::Validation(
            "identity exception board source coverage does not match pending plus rejected rows"
                .to_owned(),
        ));
    }
    Ok(AhlIdentityExceptionBoardView {
        schema: AHL_IDENTITY_EXCEPTION_BOARD_SCHEMA.to_owned(),
        source_schema: review.schema.clone(),
        groups: rows.len(),
        appearances,
        rows,
        disclosures: vec![
            "The exception board is read-only triage: priority never approves, rejects, or remaps an identity.".to_owned(),
            "Priority score = action base (routine exact 80, identity-collision investigation 75, routine alias 70, birth conflict 60, ambiguous 50, missing evidence 40, rejected audit 20) + 5 per appearance (max 20) + 8 per additional season (max 10) + 4 per additional team (max 10) + 10 for a conflict with canonical ID, date pair, and at least two retained sources.".to_owned(),
            format!("A pending birth conflict with an absolute date delta of at least {} days is triaged as an identity-collision investigation; this threshold creates no rejection or remap authority.", AHL_IDENTITY_COLLISION_DELTA_DAYS),
            "Ranks are deterministic; ties use occurrence count, AHL display name, then identity key.".to_owned(),
        ],
    })
}

fn coverage_basis_points(numerator: usize, denominator: usize) -> u16 {
    if denominator == 0 {
        0
    } else {
        ((numerator * 10_000 + denominator / 2) / denominator) as u16
    }
}

/// Project a crosswalk into a read-only, UI-neutral inspection view. Filtering
/// changes only the inspection rows; the source crosswalk remains intact.
pub fn build_ahl_identity_review_inspection(
    crosswalk: &AhlIdentityCrosswalkView,
    scope: AhlIdentityInspectionScope,
) -> Result<AhlIdentityReviewInspectionView, AhlFeedError> {
    if crosswalk.schema != AHL_IDENTITY_CROSSWALK_SCHEMA
        || crosswalk.provider.trim().is_empty()
        || crosswalk.ahl_team.trim().is_empty()
        || crosswalk.roster_fetched_at.trim().is_empty()
    {
        return Err(AhlFeedError::Validation(
            "invalid AHL identity crosswalk inspection authority".to_owned(),
        ));
    }
    let mut provider_ids = BTreeSet::new();
    if crosswalk
        .rows
        .iter()
        .any(|row| !provider_ids.insert(row.provider_player_id.as_str()))
    {
        return Err(AhlFeedError::Validation(
            "identity inspection contains duplicate provider players".to_owned(),
        ));
    }
    let computed_counts = identity_crosswalk_counts(&crosswalk.rows);
    let attention_count = crosswalk
        .rows
        .iter()
        .filter(|row| ahl_identity_row_needs_attention(row))
        .count();
    let rows = crosswalk
        .rows
        .iter()
        .filter(|row| {
            scope == AhlIdentityInspectionScope::All || ahl_identity_row_needs_attention(row)
        })
        .cloned()
        .collect();
    Ok(AhlIdentityReviewInspectionView {
        schema: AHL_IDENTITY_REVIEW_INSPECTION_SCHEMA.to_owned(),
        season: crosswalk.season,
        provider: crosswalk.provider.clone(),
        ahl_team: crosswalk.ahl_team.clone(),
        roster_fetched_at: crosswalk.roster_fetched_at.clone(),
        candidates_checked_at: crosswalk.candidates_checked_at.clone(),
        scope,
        total_rows: crosswalk.rows.len(),
        attention_count,
        declared_counts: crosswalk.counts.clone(),
        declared_counts_stale: crosswalk.counts != computed_counts,
        computed_counts,
        rows,
        disclosures: crosswalk.disclosures.clone(),
    })
}

pub fn ahl_identity_row_needs_attention(row: &AhlIdentityCrosswalkRow) -> bool {
    row.review_status == AhlIdentityReviewStatus::Rejected
        || (row.review_status == AhlIdentityReviewStatus::Pending
            && row.match_basis != AhlIdentityMatchBasis::ExactNameAndBirthDate)
}

/// Generate a deliberately non-applicable review draft for the strongest
/// proposals only. The reviewer must inspect it, set `draft=false`, and add
/// reviewer/timestamp authority before application.
pub fn build_ahl_identity_review_draft(
    crosswalk: &AhlIdentityCrosswalkView,
) -> Result<AhlIdentityReviewDecisions, AhlFeedError> {
    build_ahl_identity_review_draft_with_options(
        crosswalk,
        AhlIdentityReviewDraftOptions::default(),
    )
}

/// Generate the same non-applicable draft with optional, fully sourced alias
/// remap proposals. Alias rows are never converted to ordinary exact accepts.
pub fn build_ahl_identity_review_draft_with_aliases(
    crosswalk: &AhlIdentityCrosswalkView,
    include_aliases: bool,
) -> Result<AhlIdentityReviewDecisions, AhlFeedError> {
    build_ahl_identity_review_draft_with_options(
        crosswalk,
        AhlIdentityReviewDraftOptions {
            include_aliases,
            ..AhlIdentityReviewDraftOptions::default()
        },
    )
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AhlIdentityReviewDraftOptions {
    pub include_aliases: bool,
    pub include_conflicts: bool,
}

/// Build an applicable review batch only for pending exact-name-and-birth-date
/// proposals with retained absolute HTTP evidence. Every non-exact row remains
/// untouched for manual inspection.
pub fn build_ahl_exact_identity_review(
    crosswalk: &AhlIdentityCrosswalkView,
    reviewer: impl Into<String>,
    reviewed_at: impl Into<String>,
) -> Result<AhlIdentityReviewDecisions, AhlFeedError> {
    validate_crosswalk_shape(crosswalk)?;
    let reviewer = reviewer.into();
    let reviewed_at = reviewed_at.into();
    if reviewer.trim().is_empty() || chrono::DateTime::parse_from_rfc3339(&reviewed_at).is_err() {
        return Err(AhlFeedError::Validation(
            "exact identity review requires reviewer and RFC3339 timestamp authority".to_owned(),
        ));
    }
    let mut decisions = Vec::new();
    for row in crosswalk.rows.iter().filter(|row| {
        row.review_status == AhlIdentityReviewStatus::Pending
            && row.match_basis == AhlIdentityMatchBasis::ExactNameAndBirthDate
    }) {
        let nhl_name = row.nhl_display_name.as_deref().unwrap_or("");
        if row.nhl_player_id.is_none()
            || normalize_ahl_identity_name(&row.ahl_display_name)
                != normalize_ahl_identity_name(nhl_name)
            || row.ahl_birth_date.is_empty()
            || row.nhl_birth_date.as_deref() != Some(row.ahl_birth_date.as_str())
            || row.evidence_urls.is_empty()
            || row.evidence_urls.iter().any(|url| !absolute_http_url(url))
        {
            return Err(AhlFeedError::Validation(format!(
                "exact identity {} lacks matching name, birth date, or retained evidence",
                row.provider_player_id
            )));
        }
        decisions.push(AhlIdentityReviewDecision {
            provider_player_id: row.provider_player_id.clone(),
            action: AhlIdentityReviewAction::AcceptProposal,
            nhl_player_id: None,
            nhl_display_name: None,
            nhl_birth_date: None,
            evidence_urls: Vec::new(),
            note: "Confirmed exact normalized name, exact birth date, canonical NHL ID, and retained official identity evidence; non-exact rows were intentionally excluded from this batch.".to_owned(),
        });
    }
    if decisions.is_empty() {
        return Err(AhlFeedError::Validation(
            "exact identity review contains no eligible pending proposals".to_owned(),
        ));
    }
    Ok(AhlIdentityReviewDecisions {
        schema: AHL_IDENTITY_REVIEW_DECISIONS_SCHEMA.to_owned(),
        season: crosswalk.season,
        provider: crosswalk.provider.clone(),
        ahl_team: crosswalk.ahl_team.clone(),
        roster_fetched_at: crosswalk.roster_fetched_at.clone(),
        draft: false,
        reviewer: Some(reviewer),
        reviewed_at: Some(reviewed_at),
        decisions,
    })
}

/// Build an applicable review batch only for pending surname-and-birth-date
/// alias proposals. The canonical identity and retained evidence are copied
/// into explicit `set_identity` decisions so the name difference survives.
pub fn build_ahl_alias_identity_review(
    crosswalk: &AhlIdentityCrosswalkView,
    reviewer: impl Into<String>,
    reviewed_at: impl Into<String>,
) -> Result<AhlIdentityReviewDecisions, AhlFeedError> {
    validate_crosswalk_shape(crosswalk)?;
    let reviewer = reviewer.into();
    let reviewed_at = reviewed_at.into();
    if reviewer.trim().is_empty() || chrono::DateTime::parse_from_rfc3339(&reviewed_at).is_err() {
        return Err(AhlFeedError::Validation(
            "alias identity review requires reviewer and RFC3339 timestamp authority".to_owned(),
        ));
    }
    let mut decisions = Vec::new();
    for row in crosswalk.rows.iter().filter(|row| {
        row.review_status == AhlIdentityReviewStatus::Pending
            && row.match_basis == AhlIdentityMatchBasis::SurnameAndBirthDate
    }) {
        let nhl_name = row.nhl_display_name.as_deref().unwrap_or("");
        if row.nhl_player_id.is_none()
            || normalized_surname(&row.ahl_display_name) != normalized_surname(nhl_name)
            || normalize_ahl_identity_name(&row.ahl_display_name)
                == normalize_ahl_identity_name(nhl_name)
            || row.ahl_birth_date.is_empty()
            || row.nhl_birth_date.as_deref() != Some(row.ahl_birth_date.as_str())
            || row.evidence_urls.is_empty()
            || row.evidence_urls.iter().any(|url| !absolute_http_url(url))
        {
            return Err(AhlFeedError::Validation(format!(
                "alias identity {} lacks matching surname, birth date, or retained evidence",
                row.provider_player_id
            )));
        }
        decisions.push(AhlIdentityReviewDecision {
            provider_player_id: row.provider_player_id.clone(),
            action: AhlIdentityReviewAction::SetIdentity,
            nhl_player_id: row.nhl_player_id,
            nhl_display_name: row.nhl_display_name.clone(),
            nhl_birth_date: row.nhl_birth_date.clone(),
            evidence_urls: row.evidence_urls.clone(),
            note: format!(
                "Confirmed AHL alias `{}` maps to canonical NHL identity `{}` by surname, equal birth date, canonical ID, and retained official evidence.",
                row.ahl_display_name, nhl_name
            ),
        });
    }
    if decisions.is_empty() {
        return Err(AhlFeedError::Validation(
            "alias identity review contains no eligible pending proposals".to_owned(),
        ));
    }
    Ok(AhlIdentityReviewDecisions {
        schema: AHL_IDENTITY_REVIEW_DECISIONS_SCHEMA.to_owned(),
        season: crosswalk.season,
        provider: crosswalk.provider.clone(),
        ahl_team: crosswalk.ahl_team.clone(),
        roster_fetched_at: crosswalk.roster_fetched_at.clone(),
        draft: false,
        reviewer: Some(reviewer),
        reviewed_at: Some(reviewed_at),
        decisions,
    })
}

/// Build an applicable, evidence-backed override for selected pending birth
/// date conflicts. The proposed NHL identity is retained explicitly while the
/// provider and NHL dates remain visible in the decision note.
pub fn build_ahl_identity_conflict_review(
    crosswalk: &AhlIdentityCrosswalkView,
    nhl_player_ids: &[u32],
    evidence_urls: &[String],
    reviewer: impl Into<String>,
    reviewed_at: impl Into<String>,
    note: impl Into<String>,
) -> Result<AhlIdentityReviewDecisions, AhlFeedError> {
    validate_crosswalk_shape(crosswalk)?;
    let reviewer = reviewer.into();
    let reviewed_at = reviewed_at.into();
    let note = note.into();
    let requested_ids = nhl_player_ids.iter().copied().collect::<BTreeSet<_>>();
    if nhl_player_ids.is_empty()
        || requested_ids.len() != nhl_player_ids.len()
        || requested_ids.contains(&0)
        || reviewer.trim().is_empty()
        || note.trim().is_empty()
        || evidence_urls.is_empty()
        || evidence_urls.iter().any(|url| !absolute_http_url(url))
        || chrono::DateTime::parse_from_rfc3339(&reviewed_at).is_err()
    {
        return Err(AhlFeedError::Validation(
            "identity conflict review requires unique NHL IDs, new evidence, reviewer, timestamp, and rationale authority".to_owned(),
        ));
    }
    let mut matched_ids = BTreeSet::new();
    let mut decisions = Vec::new();
    for row in crosswalk.rows.iter().filter(|row| {
        row.nhl_player_id
            .is_some_and(|id| requested_ids.contains(&id))
    }) {
        let Some(nhl_player_id) = row.nhl_player_id else {
            unreachable!("filtered conflict row has an NHL player ID")
        };
        let nhl_display_name = row.nhl_display_name.as_deref().unwrap_or("");
        let nhl_birth_date = row.nhl_birth_date.as_deref().unwrap_or("");
        if !matched_ids.insert(nhl_player_id)
            || row.review_status != AhlIdentityReviewStatus::Pending
            || row.match_basis != AhlIdentityMatchBasis::BirthDateConflict
            || normalize_ahl_identity_name(nhl_display_name).is_empty()
            || chrono::NaiveDate::parse_from_str(&row.ahl_birth_date, "%Y-%m-%d").is_err()
            || chrono::NaiveDate::parse_from_str(nhl_birth_date, "%Y-%m-%d").is_err()
            || row.ahl_birth_date == nhl_birth_date
            || row.evidence_urls.is_empty()
            || row.evidence_urls.iter().any(|url| !absolute_http_url(url))
            || evidence_urls
                .iter()
                .all(|url| row.evidence_urls.contains(url))
        {
            return Err(AhlFeedError::Validation(format!(
                "identity conflict proposal {nhl_player_id} is duplicate, ineligible, or lacks retained source dates/evidence"
            )));
        }
        let mut retained_evidence = row.evidence_urls.iter().cloned().collect::<BTreeSet<_>>();
        retained_evidence.extend(evidence_urls.iter().cloned());
        decisions.push(AhlIdentityReviewDecision {
            provider_player_id: row.provider_player_id.clone(),
            action: AhlIdentityReviewAction::SetIdentity,
            nhl_player_id: Some(nhl_player_id),
            nhl_display_name: Some(nhl_display_name.to_owned()),
            nhl_birth_date: Some(nhl_birth_date.to_owned()),
            evidence_urls: retained_evidence.into_iter().collect(),
            note: format!(
                "Confirmed AHL `{}` as NHL `{}` ({}) while retaining conflicting source dates AHL {} and NHL {}. Evidence-backed conflict rationale: {}",
                row.ahl_display_name,
                nhl_display_name,
                nhl_player_id,
                row.ahl_birth_date,
                nhl_birth_date,
                note.trim()
            ),
        });
    }
    if matched_ids != requested_ids {
        let missing = requested_ids
            .difference(&matched_ids)
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(AhlFeedError::Validation(format!(
            "identity conflict review found no eligible proposal for NHL player(s) {missing}"
        )));
    }
    Ok(AhlIdentityReviewDecisions {
        schema: AHL_IDENTITY_REVIEW_DECISIONS_SCHEMA.to_owned(),
        season: crosswalk.season,
        provider: crosswalk.provider.clone(),
        ahl_team: crosswalk.ahl_team.clone(),
        roster_fetched_at: crosswalk.roster_fetched_at.clone(),
        draft: false,
        reviewer: Some(reviewer),
        reviewed_at: Some(reviewed_at),
        decisions,
    })
}

/// Atomically apply one routine review lane to every eligible child crosswalk
/// in a league envelope and retain the per-team decision batches as authority.
pub fn apply_ahl_identity_league_routine_review(
    league: &AhlIdentityLeagueCrosswalkView,
    kind: AhlIdentityLeagueRoutineReviewKind,
    reviewer: impl Into<String>,
    reviewed_at: impl Into<String>,
) -> Result<
    (
        AhlIdentityLeagueCrosswalkView,
        AhlIdentityLeagueReviewDecisionsView,
    ),
    AhlFeedError,
> {
    validate_ahl_identity_league_crosswalk(league)?;
    if matches!(
        kind,
        AhlIdentityLeagueRoutineReviewKind::Conflicts
            | AhlIdentityLeagueRoutineReviewKind::CollisionRemaps
    ) {
        return Err(AhlFeedError::Validation(
            "conflict and collision-remap reviews require targeted NHL IDs and additional evidence"
                .to_owned(),
        ));
    }
    let reviewer = reviewer.into();
    let reviewed_at = reviewed_at.into();
    if reviewer.trim().is_empty() || chrono::DateTime::parse_from_rfc3339(&reviewed_at).is_err() {
        return Err(AhlFeedError::Validation(
            "league identity review requires reviewer and RFC3339 timestamp authority".to_owned(),
        ));
    }
    let mut output = league.clone();
    let mut batches = Vec::new();
    let mut skipped_teams = Vec::new();
    let mut applied_decisions = 0usize;
    for crosswalk in &mut output.crosswalks {
        let eligible = crosswalk.rows.iter().any(|row| {
            row.review_status == AhlIdentityReviewStatus::Pending
                && match kind {
                    AhlIdentityLeagueRoutineReviewKind::Exact => {
                        row.match_basis == AhlIdentityMatchBasis::ExactNameAndBirthDate
                    }
                    AhlIdentityLeagueRoutineReviewKind::Aliases => {
                        row.match_basis == AhlIdentityMatchBasis::SurnameAndBirthDate
                    }
                    AhlIdentityLeagueRoutineReviewKind::Conflicts
                    | AhlIdentityLeagueRoutineReviewKind::CollisionRemaps => false,
                }
        });
        if !eligible {
            skipped_teams.push(crosswalk.ahl_team.clone());
            continue;
        }
        let decisions = match kind {
            AhlIdentityLeagueRoutineReviewKind::Exact => {
                build_ahl_exact_identity_review(crosswalk, reviewer.clone(), reviewed_at.clone())?
            }
            AhlIdentityLeagueRoutineReviewKind::Aliases => {
                build_ahl_alias_identity_review(crosswalk, reviewer.clone(), reviewed_at.clone())?
            }
            AhlIdentityLeagueRoutineReviewKind::Conflicts
            | AhlIdentityLeagueRoutineReviewKind::CollisionRemaps => {
                unreachable!("conflict reviews are rejected before routine league review")
            }
        };
        applied_decisions += decisions.decisions.len();
        *crosswalk = apply_ahl_identity_review_decisions(crosswalk, &decisions)?;
        batches.push(decisions);
    }
    let eligible_teams = batches.len();
    output.disclosures.push(format!(
        "Applied league {:?} review: {} decision(s) across {} eligible team(s) by {} at {}; {} team(s) had no eligible rows.",
        kind,
        applied_decisions,
        eligible_teams,
        reviewer,
        reviewed_at,
        skipped_teams.len()
    ));
    Ok((
        output,
        AhlIdentityLeagueReviewDecisionsView {
            schema: AHL_IDENTITY_LEAGUE_REVIEW_DECISIONS_SCHEMA.to_owned(),
            season: league.season,
            provider: league.provider.clone(),
            roster_fetched_at: league.roster_fetched_at.clone(),
            kind,
            reviewer,
            reviewed_at,
            eligible_teams,
            skipped_teams,
            applied_decisions,
            batches,
            disclosures: vec![
                "Each batch remains bound to its original season, provider, team, and roster fetch. Teams without eligible rows are recorded rather than treated as failures.".to_owned(),
                "League routine review is atomic: invalid evidence in any eligible child prevents an updated envelope from being returned.".to_owned(),
            ],
        },
    ))
}

/// Atomically resolve selected proposed NHL identities across a league
/// envelope. Only pending birth-date conflicts are eligible, and every
/// requested NHL ID must match at least one child row.
pub fn apply_ahl_identity_league_conflict_review(
    league: &AhlIdentityLeagueCrosswalkView,
    nhl_player_ids: &[u32],
    evidence_urls: &[String],
    reviewer: impl Into<String>,
    reviewed_at: impl Into<String>,
    note: impl Into<String>,
) -> Result<
    (
        AhlIdentityLeagueCrosswalkView,
        AhlIdentityLeagueReviewDecisionsView,
    ),
    AhlFeedError,
> {
    validate_ahl_identity_league_crosswalk(league)?;
    let reviewer = reviewer.into();
    let reviewed_at = reviewed_at.into();
    let note = note.into();
    let requested_ids = nhl_player_ids.iter().copied().collect::<BTreeSet<_>>();
    if nhl_player_ids.is_empty()
        || requested_ids.len() != nhl_player_ids.len()
        || requested_ids.contains(&0)
    {
        return Err(AhlFeedError::Validation(
            "league identity conflict review requires unique non-zero NHL player IDs".to_owned(),
        ));
    }
    let mut output = league.clone();
    let mut matched_ids = BTreeSet::new();
    let mut batches = Vec::new();
    let mut skipped_teams = Vec::new();
    let mut applied_decisions = 0usize;
    for crosswalk in &mut output.crosswalks {
        let team_ids = crosswalk
            .rows
            .iter()
            .filter(|row| {
                row.review_status == AhlIdentityReviewStatus::Pending
                    && row.match_basis == AhlIdentityMatchBasis::BirthDateConflict
            })
            .filter_map(|row| row.nhl_player_id)
            .filter(|id| requested_ids.contains(id))
            .collect::<BTreeSet<_>>();
        if team_ids.is_empty() {
            skipped_teams.push(crosswalk.ahl_team.clone());
            continue;
        }
        let team_ids = team_ids.into_iter().collect::<Vec<_>>();
        let decisions = build_ahl_identity_conflict_review(
            crosswalk,
            &team_ids,
            evidence_urls,
            reviewer.clone(),
            reviewed_at.clone(),
            note.clone(),
        )?;
        matched_ids.extend(team_ids);
        applied_decisions += decisions.decisions.len();
        *crosswalk = apply_ahl_identity_review_decisions(crosswalk, &decisions)?;
        batches.push(decisions);
    }
    if matched_ids != requested_ids {
        let missing = requested_ids
            .difference(&matched_ids)
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(AhlFeedError::Validation(format!(
            "league identity conflict review found no eligible proposal for NHL player(s) {missing}"
        )));
    }
    let eligible_teams = batches.len();
    output.disclosures.push(format!(
        "Applied targeted league conflict review: {} decision(s) across {} eligible team(s) by {} at {}; {} team(s) had no selected conflict row.",
        applied_decisions,
        eligible_teams,
        reviewer,
        reviewed_at,
        skipped_teams.len()
    ));
    Ok((
        output,
        AhlIdentityLeagueReviewDecisionsView {
            schema: AHL_IDENTITY_LEAGUE_REVIEW_DECISIONS_SCHEMA.to_owned(),
            season: league.season,
            provider: league.provider.clone(),
            roster_fetched_at: league.roster_fetched_at.clone(),
            kind: AhlIdentityLeagueRoutineReviewKind::Conflicts,
            reviewer,
            reviewed_at,
            eligible_teams,
            skipped_teams,
            applied_decisions,
            batches,
            disclosures: vec![
                "Every batch uses explicit set_identity decisions, retains both conflicting source dates in its note, and unions retained proposal evidence with the reviewer-supplied sources.".to_owned(),
                "League conflict review is atomic: every requested NHL ID must have an eligible pending birth-date conflict, or no updated envelope is returned.".to_owned(),
            ],
        },
    ))
}

/// Atomically replace one demonstrably collided NHL proposal across every
/// affected team in a league envelope. The canonical identity must share the
/// AHL birth date and surname, while the displaced proposal must differ by at
/// least the collision-review threshold.
#[allow(clippy::too_many_arguments)]
pub fn apply_ahl_identity_league_collision_remap(
    league: &AhlIdentityLeagueCrosswalkView,
    proposed_nhl_player_id: u32,
    canonical_nhl_player_id: u32,
    canonical_display_name: impl Into<String>,
    canonical_birth_date: impl Into<String>,
    evidence_urls: &[String],
    reviewer: impl Into<String>,
    reviewed_at: impl Into<String>,
    note: impl Into<String>,
) -> Result<
    (
        AhlIdentityLeagueCrosswalkView,
        AhlIdentityLeagueReviewDecisionsView,
    ),
    AhlFeedError,
> {
    validate_ahl_identity_league_crosswalk(league)?;
    let canonical_display_name = canonical_display_name.into();
    let canonical_birth_date = canonical_birth_date.into();
    let reviewer = reviewer.into();
    let reviewed_at = reviewed_at.into();
    let note = note.into();
    let canonical_date = chrono::NaiveDate::parse_from_str(&canonical_birth_date, "%Y-%m-%d")
        .map_err(|_| {
            AhlFeedError::Validation(
                "collision remap requires a canonical YYYY-MM-DD birth date".to_owned(),
            )
        })?;
    if proposed_nhl_player_id == 0
        || canonical_nhl_player_id == 0
        || proposed_nhl_player_id == canonical_nhl_player_id
        || normalize_ahl_identity_name(&canonical_display_name).is_empty()
        || normalized_surname(&canonical_display_name).is_none()
        || evidence_urls.is_empty()
        || evidence_urls.iter().any(|url| !absolute_http_url(url))
        || reviewer.trim().is_empty()
        || chrono::DateTime::parse_from_rfc3339(&reviewed_at).is_err()
        || note.trim().is_empty()
    {
        return Err(AhlFeedError::Validation(
            "collision remap requires distinct non-zero NHL IDs, canonical identity/date, new evidence, reviewer, timestamp, and rationale authority".to_owned(),
        ));
    }

    let mut output = league.clone();
    let mut batches = Vec::new();
    let mut skipped_teams = Vec::new();
    let mut applied_decisions = 0usize;
    for crosswalk in &mut output.crosswalks {
        let mut decisions = Vec::new();
        for row in crosswalk
            .rows
            .iter()
            .filter(|row| row.nhl_player_id == Some(proposed_nhl_player_id))
        {
            let proposed_date = row
                .nhl_birth_date
                .as_deref()
                .and_then(|date| chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").ok())
                .ok_or_else(|| AhlFeedError::Validation(format!(
                    "collision proposal {proposed_nhl_player_id} lacks a valid retained NHL birth date"
                )))?;
            let ahl_date = chrono::NaiveDate::parse_from_str(&row.ahl_birth_date, "%Y-%m-%d")
                .map_err(|_| {
                    AhlFeedError::Validation(format!(
                        "collision proposal {proposed_nhl_player_id} lacks a valid AHL birth date"
                    ))
                })?;
            let delta_days = (ahl_date - proposed_date).num_days().unsigned_abs();
            if row.review_status != AhlIdentityReviewStatus::Pending
                || row.match_basis != AhlIdentityMatchBasis::BirthDateConflict
                || delta_days < u64::from(AHL_IDENTITY_COLLISION_DELTA_DAYS)
                || ahl_date != canonical_date
                || normalized_surname(&row.ahl_display_name)
                    != normalized_surname(&canonical_display_name)
                || evidence_urls
                    .iter()
                    .all(|url| row.evidence_urls.contains(url))
            {
                return Err(AhlFeedError::Validation(format!(
                    "collision proposal {proposed_nhl_player_id} is ineligible or lacks canonical equal-date/surname evidence"
                )));
            }
            let mut retained_evidence = row.evidence_urls.iter().cloned().collect::<BTreeSet<_>>();
            retained_evidence.extend(evidence_urls.iter().cloned());
            decisions.push(AhlIdentityReviewDecision {
                provider_player_id: row.provider_player_id.clone(),
                action: AhlIdentityReviewAction::SetIdentity,
                nhl_player_id: Some(canonical_nhl_player_id),
                nhl_display_name: Some(canonical_display_name.clone()),
                nhl_birth_date: Some(canonical_birth_date.clone()),
                evidence_urls: retained_evidence.into_iter().collect(),
                note: format!(
                    "Replaced collided NHL proposal `{}` ({}, {}) with canonical NHL `{}` ({}, {}) for AHL `{}` ({}, {}-day displaced-date delta). Evidence-backed collision rationale: {}",
                    row.nhl_display_name.as_deref().unwrap_or("unknown"),
                    proposed_nhl_player_id,
                    row.nhl_birth_date.as_deref().unwrap_or("unknown"),
                    canonical_display_name,
                    canonical_nhl_player_id,
                    canonical_birth_date,
                    row.ahl_display_name,
                    row.provider_player_id,
                    delta_days,
                    note.trim()
                ),
            });
        }
        if decisions.is_empty() {
            skipped_teams.push(crosswalk.ahl_team.clone());
            continue;
        }
        let batch = AhlIdentityReviewDecisions {
            schema: AHL_IDENTITY_REVIEW_DECISIONS_SCHEMA.to_owned(),
            season: crosswalk.season,
            provider: crosswalk.provider.clone(),
            ahl_team: crosswalk.ahl_team.clone(),
            roster_fetched_at: crosswalk.roster_fetched_at.clone(),
            draft: false,
            reviewer: Some(reviewer.clone()),
            reviewed_at: Some(reviewed_at.clone()),
            decisions,
        };
        applied_decisions += batch.decisions.len();
        *crosswalk = apply_ahl_identity_review_decisions(crosswalk, &batch)?;
        batches.push(batch);
    }
    if applied_decisions == 0 {
        return Err(AhlFeedError::Validation(format!(
            "league collision remap found no eligible proposal for NHL player {proposed_nhl_player_id}"
        )));
    }
    let eligible_teams = batches.len();
    output.disclosures.push(format!(
        "Applied collision remap from NHL {} to NHL {}: {} decision(s) across {} eligible team(s) by {} at {}; {} team(s) had no selected collision row.",
        proposed_nhl_player_id,
        canonical_nhl_player_id,
        applied_decisions,
        eligible_teams,
        reviewer,
        reviewed_at,
        skipped_teams.len()
    ));
    Ok((output, AhlIdentityLeagueReviewDecisionsView {
        schema: AHL_IDENTITY_LEAGUE_REVIEW_DECISIONS_SCHEMA.to_owned(),
        season: league.season,
        provider: league.provider.clone(),
        roster_fetched_at: league.roster_fetched_at.clone(),
        kind: AhlIdentityLeagueRoutineReviewKind::CollisionRemaps,
        reviewer,
        reviewed_at,
        eligible_teams,
        skipped_teams,
        applied_decisions,
        batches,
        disclosures: vec![
            "A collision remap changes only the NHL identity mapping; it never rejects or removes the AHL player.".to_owned(),
            "The displaced proposal, both source dates, threshold delta, canonical identity, and unioned evidence remain in each decision audit note.".to_owned(),
        ],
    }))
}

/// Build an applicable rejection batch for explicitly selected pending rows.
/// A rejection closes only the proposed NHL identity mapping; its required
/// note must explain whether the source row is an AHL-only player, non-player,
/// or otherwise unsuitable for the NHL-linked adapter.
pub fn build_ahl_identity_rejection_review(
    crosswalk: &AhlIdentityCrosswalkView,
    provider_player_ids: &[String],
    evidence_urls: &[String],
    reviewer: impl Into<String>,
    reviewed_at: impl Into<String>,
    note: impl Into<String>,
) -> Result<AhlIdentityReviewDecisions, AhlFeedError> {
    validate_crosswalk_shape(crosswalk)?;
    let reviewer = reviewer.into();
    let reviewed_at = reviewed_at.into();
    let note = note.into();
    if provider_player_ids.is_empty()
        || reviewer.trim().is_empty()
        || note.trim().is_empty()
        || evidence_urls.iter().any(|url| !absolute_http_url(url))
        || chrono::DateTime::parse_from_rfc3339(&reviewed_at).is_err()
    {
        return Err(AhlFeedError::Validation(
            "identity rejection review requires rows, reviewer, timestamp, and note authority"
                .to_owned(),
        ));
    }
    let rows = crosswalk
        .rows
        .iter()
        .map(|row| (row.provider_player_id.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let mut seen = BTreeSet::new();
    let mut decisions = Vec::new();
    for provider_player_id in provider_player_ids {
        let row = rows.get(provider_player_id.as_str()).ok_or_else(|| {
            AhlFeedError::Validation(format!(
                "identity rejection references unknown provider player {provider_player_id}"
            ))
        })?;
        if !seen.insert(provider_player_id.as_str())
            || row.review_status != AhlIdentityReviewStatus::Pending
        {
            return Err(AhlFeedError::Validation(format!(
                "identity rejection row {provider_player_id} is duplicate or not pending"
            )));
        }
        decisions.push(AhlIdentityReviewDecision {
            provider_player_id: provider_player_id.clone(),
            action: AhlIdentityReviewAction::Reject,
            nhl_player_id: None,
            nhl_display_name: None,
            nhl_birth_date: None,
            evidence_urls: evidence_urls.to_vec(),
            note: format!(
                "Rejected NHL identity mapping for AHL row `{}`: {}",
                row.ahl_display_name,
                note.trim()
            ),
        });
    }
    Ok(AhlIdentityReviewDecisions {
        schema: AHL_IDENTITY_REVIEW_DECISIONS_SCHEMA.to_owned(),
        season: crosswalk.season,
        provider: crosswalk.provider.clone(),
        ahl_team: crosswalk.ahl_team.clone(),
        roster_fetched_at: crosswalk.roster_fetched_at.clone(),
        draft: false,
        reviewer: Some(reviewer),
        reviewed_at: Some(reviewed_at),
        decisions,
    })
}

/// Generate a deliberately non-applicable decision draft. Optional lanes are
/// proposals only and retain their distinct review semantics.
pub fn build_ahl_identity_review_draft_with_options(
    crosswalk: &AhlIdentityCrosswalkView,
    options: AhlIdentityReviewDraftOptions,
) -> Result<AhlIdentityReviewDecisions, AhlFeedError> {
    validate_crosswalk_shape(crosswalk)?;
    Ok(AhlIdentityReviewDecisions {
        schema: AHL_IDENTITY_REVIEW_DECISIONS_SCHEMA.to_owned(),
        season: crosswalk.season,
        provider: crosswalk.provider.clone(),
        ahl_team: crosswalk.ahl_team.clone(),
        roster_fetched_at: crosswalk.roster_fetched_at.clone(),
        draft: true,
        reviewer: None,
        reviewed_at: None,
        decisions: crosswalk
            .rows
            .iter()
            .filter(|row| row.review_status == AhlIdentityReviewStatus::Pending)
            .filter_map(|row| match row.match_basis {
                AhlIdentityMatchBasis::ExactNameAndBirthDate => Some(AhlIdentityReviewDecision {
                    provider_player_id: row.provider_player_id.clone(),
                    action: AhlIdentityReviewAction::AcceptProposal,
                    nhl_player_id: None,
                    nhl_display_name: None,
                    nhl_birth_date: None,
                    evidence_urls: Vec::new(),
                    note: "Verify the retained official NHL search and landing evidence before accepting this exact name-and-birth-date proposal.".to_owned(),
                }),
                AhlIdentityMatchBasis::SurnameAndBirthDate if options.include_aliases => {
                    Some(AhlIdentityReviewDecision {
                        provider_player_id: row.provider_player_id.clone(),
                        action: AhlIdentityReviewAction::SetIdentity,
                        nhl_player_id: row.nhl_player_id,
                        nhl_display_name: row.nhl_display_name.clone(),
                        nhl_birth_date: row.nhl_birth_date.clone(),
                        evidence_urls: row.evidence_urls.clone(),
                        note: "Verify both official sources and the differing display names before approving this surname-and-birth-date alias remap.".to_owned(),
                    })
                }
                AhlIdentityMatchBasis::BirthDateConflict if options.include_conflicts => {
                    Some(AhlIdentityReviewDecision {
                        provider_player_id: row.provider_player_id.clone(),
                        action: AhlIdentityReviewAction::AcceptProposal,
                        nhl_player_id: None,
                        nhl_display_name: None,
                        nhl_birth_date: None,
                        evidence_urls: Vec::new(),
                        note: format!(
                            "Compare and preserve the conflicting provider birth dates (AHL {} / NHL {}) before accepting this identity proposal.",
                            row.ahl_birth_date,
                            row.nhl_birth_date.as_deref().unwrap_or("missing")
                        ),
                    })
                }
                _ => None,
            })
            .collect(),
    })
}

/// Build one non-applicable draft envelope across every child team queue.
/// Pending rows without a draftable proposal remain explicitly counted.
pub fn build_ahl_identity_league_review_draft(
    league: &AhlIdentityLeagueCrosswalkView,
    options: AhlIdentityReviewDraftOptions,
) -> Result<AhlIdentityLeagueReviewDraftView, AhlFeedError> {
    validate_ahl_identity_league_crosswalk(league)?;
    let mut batches = Vec::new();
    let mut skipped_teams = Vec::new();
    let mut proposed_decisions = 0usize;
    let mut pending_without_proposal = 0usize;
    for crosswalk in &league.crosswalks {
        let draft = build_ahl_identity_review_draft_with_options(crosswalk, options)?;
        let pending = crosswalk
            .rows
            .iter()
            .filter(|row| row.review_status == AhlIdentityReviewStatus::Pending)
            .count();
        proposed_decisions += draft.decisions.len();
        pending_without_proposal += pending.saturating_sub(draft.decisions.len());
        if draft.decisions.is_empty() {
            skipped_teams.push(crosswalk.ahl_team.clone());
        } else {
            batches.push(draft);
        }
    }
    Ok(AhlIdentityLeagueReviewDraftView {
        schema: AHL_IDENTITY_LEAGUE_REVIEW_DRAFT_SCHEMA.to_owned(),
        season: league.season,
        provider: league.provider.clone(),
        roster_fetched_at: league.roster_fetched_at.clone(),
        include_aliases: options.include_aliases,
        include_conflicts: options.include_conflicts,
        eligible_teams: batches.len(),
        skipped_teams,
        proposed_decisions,
        pending_without_proposal,
        batches,
        disclosures: vec![
            "Every child batch is draft=true and cannot be applied until a reviewer inspects its evidence and adds explicit reviewer/timestamp authority.".to_owned(),
            "Pending rows without a draftable proposal remain counted; unmatched and ambiguous identities require new evidence or an explicit rejection workflow.".to_owned(),
            "League drafting does not alter the source envelope or any child review status.".to_owned(),
        ],
    })
}

/// Apply an explicit review batch while preserving all untouched rows as-is.
pub fn apply_ahl_identity_review_decisions(
    crosswalk: &AhlIdentityCrosswalkView,
    review: &AhlIdentityReviewDecisions,
) -> Result<AhlIdentityCrosswalkView, AhlFeedError> {
    validate_crosswalk_shape(crosswalk)?;
    validate_review_authority(crosswalk, review)?;
    let reviewer = review.reviewer.as_deref().expect("validated reviewer");
    let reviewed_at = review.reviewed_at.as_deref().expect("validated timestamp");
    let mut output = crosswalk.clone();
    let mut resulting_nhl_ids = output
        .rows
        .iter()
        .filter(|row| row.review_status == AhlIdentityReviewStatus::Reviewed)
        .filter_map(|row| row.nhl_player_id)
        .collect::<BTreeSet<_>>();
    let mut by_provider = output
        .rows
        .iter_mut()
        .map(|row| (row.provider_player_id.clone(), row))
        .collect::<BTreeMap<_, _>>();
    for decision in &review.decisions {
        let row = by_provider
            .get_mut(&decision.provider_player_id)
            .expect("validated provider decision");
        if row.review_status == AhlIdentityReviewStatus::Reviewed {
            if let Some(id) = row.nhl_player_id {
                resulting_nhl_ids.remove(&id);
            }
        }
        match decision.action {
            AhlIdentityReviewAction::AcceptProposal => {
                if row.match_basis == AhlIdentityMatchBasis::BirthDateConflict {
                    return Err(AhlFeedError::Validation(format!(
                        "identity {} birth-date conflict requires an explicit sourced set_identity review",
                        row.provider_player_id
                    )));
                }
                let id = row.nhl_player_id.ok_or_else(|| {
                    AhlFeedError::Validation(format!(
                        "identity {} has no proposal to accept",
                        row.provider_player_id
                    ))
                })?;
                if row
                    .nhl_display_name
                    .as_deref()
                    .is_none_or(|name| name.trim().is_empty())
                    || row.evidence_urls.is_empty()
                {
                    return Err(AhlFeedError::Validation(format!(
                        "identity {} proposal lacks NHL name/evidence",
                        row.provider_player_id
                    )));
                }
                if !resulting_nhl_ids.insert(id) {
                    return Err(AhlFeedError::Validation(format!(
                        "review decisions duplicate NHL player {id}"
                    )));
                }
                row.review_status = AhlIdentityReviewStatus::Reviewed;
            }
            AhlIdentityReviewAction::SetIdentity => {
                let id = decision
                    .nhl_player_id
                    .filter(|id| *id != 0)
                    .ok_or_else(|| {
                        AhlFeedError::Validation(format!(
                            "set_identity decision {} has no NHL player ID",
                            row.provider_player_id
                        ))
                    })?;
                let name = decision
                    .nhl_display_name
                    .as_deref()
                    .filter(|name| !normalize_ahl_identity_name(name).is_empty())
                    .ok_or_else(|| {
                        AhlFeedError::Validation(format!(
                            "set_identity decision {} has no NHL display name",
                            row.provider_player_id
                        ))
                    })?;
                if decision.evidence_urls.is_empty()
                    || decision
                        .evidence_urls
                        .iter()
                        .any(|url| !absolute_http_url(url))
                    || decision.nhl_birth_date.as_deref().is_some_and(|date| {
                        chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err()
                    })
                    || !resulting_nhl_ids.insert(id)
                {
                    return Err(AhlFeedError::Validation(format!(
                        "set_identity decision {} has invalid or duplicate evidence",
                        row.provider_player_id
                    )));
                }
                row.match_basis = reviewed_match_basis(
                    &row.ahl_display_name,
                    &row.ahl_birth_date,
                    name,
                    decision.nhl_birth_date.as_deref(),
                );
                row.review_status = AhlIdentityReviewStatus::Reviewed;
                row.nhl_player_id = Some(id);
                row.nhl_display_name = Some(name.to_owned());
                row.nhl_birth_date.clone_from(&decision.nhl_birth_date);
                row.evidence_urls.clone_from(&decision.evidence_urls);
            }
            AhlIdentityReviewAction::Reject => {
                row.review_status = AhlIdentityReviewStatus::Rejected;
                for url in &decision.evidence_urls {
                    if !row.evidence_urls.contains(url) {
                        row.evidence_urls.push(url.clone());
                    }
                }
            }
        }
        row.note = format!(
            "{} Review {} by {} at {}: {}",
            row.note,
            match decision.action {
                AhlIdentityReviewAction::AcceptProposal => "accepted proposal",
                AhlIdentityReviewAction::SetIdentity => "set identity",
                AhlIdentityReviewAction::Reject => "rejected proposal",
            },
            reviewer,
            reviewed_at,
            decision.note.trim()
        );
    }
    output.counts = identity_crosswalk_counts(&output.rows);
    output.disclosures.push(format!(
        "Applied {} explicit identity review decision(s) by {} at {}; untouched rows retain their prior review status.",
        review.decisions.len(), reviewer, reviewed_at
    ));
    Ok(output)
}

fn validate_crosswalk_shape(crosswalk: &AhlIdentityCrosswalkView) -> Result<(), AhlFeedError> {
    if crosswalk.schema != AHL_IDENTITY_CROSSWALK_SCHEMA
        || crosswalk.provider.trim().is_empty()
        || crosswalk.ahl_team.trim().is_empty()
        || crosswalk.roster_fetched_at.trim().is_empty()
        || crosswalk.rows.len() != crosswalk.counts.roster_players
    {
        return Err(AhlFeedError::Validation(
            "invalid AHL identity crosswalk review authority".to_owned(),
        ));
    }
    let mut ids = BTreeSet::new();
    if crosswalk
        .rows
        .iter()
        .any(|row| !ids.insert(row.provider_player_id.as_str()))
    {
        return Err(AhlFeedError::Validation(
            "identity crosswalk contains duplicate provider players".to_owned(),
        ));
    }
    Ok(())
}

fn validate_ahl_identity_league_crosswalk(
    league: &AhlIdentityLeagueCrosswalkView,
) -> Result<(), AhlFeedError> {
    if league.schema != AHL_IDENTITY_LEAGUE_CROSSWALK_SCHEMA
        || league.provider.trim().is_empty()
        || league.roster_fetched_at.trim().is_empty()
        || league.teams != league.crosswalks.len()
    {
        return Err(AhlFeedError::Validation(
            "invalid AHL identity league crosswalk authority".to_owned(),
        ));
    }
    let mut teams = BTreeSet::new();
    let mut provider_players = BTreeSet::new();
    let mut roster_appearances = 0usize;
    for crosswalk in &league.crosswalks {
        validate_crosswalk_shape(crosswalk)?;
        if crosswalk.season != league.season
            || crosswalk.provider != league.provider
            || crosswalk.roster_fetched_at != league.roster_fetched_at
            || !teams.insert(crosswalk.ahl_team.as_str())
        {
            return Err(AhlFeedError::Validation(format!(
                "league identity child binding mismatch for {}",
                crosswalk.ahl_team
            )));
        }
        roster_appearances += crosswalk.rows.len();
        provider_players.extend(
            crosswalk
                .rows
                .iter()
                .map(|row| row.provider_player_id.as_str()),
        );
    }
    if roster_appearances != league.roster_appearances
        || provider_players.len() != league.unique_provider_players
    {
        return Err(AhlFeedError::Validation(
            "league identity envelope coverage counts are stale".to_owned(),
        ));
    }
    Ok(())
}

fn validate_review_authority(
    crosswalk: &AhlIdentityCrosswalkView,
    review: &AhlIdentityReviewDecisions,
) -> Result<(), AhlFeedError> {
    if review.schema != AHL_IDENTITY_REVIEW_DECISIONS_SCHEMA
        || review.season != crosswalk.season
        || review.provider != crosswalk.provider
        || review.ahl_team != crosswalk.ahl_team
        || review.roster_fetched_at != crosswalk.roster_fetched_at
        || review.draft
        || review
            .reviewer
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        || review
            .reviewed_at
            .as_deref()
            .is_none_or(|value| chrono::DateTime::parse_from_rfc3339(value).is_err())
        || review.decisions.is_empty()
    {
        return Err(AhlFeedError::Validation(
            "identity review decisions are draft, unbound, empty, or missing reviewer authority"
                .to_owned(),
        ));
    }
    let official_ids = crosswalk
        .rows
        .iter()
        .map(|row| row.provider_player_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut decision_ids = BTreeSet::new();
    for decision in &review.decisions {
        if !official_ids.contains(decision.provider_player_id.as_str())
            || !decision_ids.insert(decision.provider_player_id.as_str())
            || decision.note.trim().is_empty()
            || (decision.action != AhlIdentityReviewAction::SetIdentity
                && (decision.nhl_player_id.is_some()
                    || decision.nhl_display_name.is_some()
                    || decision.nhl_birth_date.is_some()))
            || (decision.action == AhlIdentityReviewAction::AcceptProposal
                && !decision.evidence_urls.is_empty())
            || (decision.action == AhlIdentityReviewAction::Reject
                && decision
                    .evidence_urls
                    .iter()
                    .any(|url| !absolute_http_url(url)))
        {
            return Err(AhlFeedError::Validation(format!(
                "invalid identity review decision for provider player {}",
                decision.provider_player_id
            )));
        }
    }
    Ok(())
}

fn reviewed_match_basis(
    ahl_name: &str,
    ahl_birth_date: &str,
    nhl_name: &str,
    nhl_birth_date: Option<&str>,
) -> AhlIdentityMatchBasis {
    if normalize_ahl_identity_name(ahl_name) != normalize_ahl_identity_name(nhl_name) {
        AhlIdentityMatchBasis::ReviewedOverride
    } else if !ahl_birth_date.is_empty() && nhl_birth_date == Some(ahl_birth_date) {
        AhlIdentityMatchBasis::ExactNameAndBirthDate
    } else if !ahl_birth_date.is_empty() && nhl_birth_date.is_some() {
        AhlIdentityMatchBasis::BirthDateConflict
    } else {
        AhlIdentityMatchBasis::ExactNameOnly
    }
}

/// Scenario and player-value facts stay separate from identity review.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlProjectionPlayerFacts {
    pub provider_player_id: String,
    pub primary_position: icelines_core::model::Position,
    pub eligible_positions: Vec<icelines_core::model::Position>,
    pub projected_score: f64,
    #[serde(default)]
    pub prospect: bool,
    #[serde(default)]
    pub recall_readiness: Option<f64>,
    #[serde(default)]
    pub professional_games_at_season_start: Option<u32>,
    #[serde(default = "default_true")]
    pub assigned_to_affiliate: bool,
    #[serde(default)]
    pub waiver_required: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlSkaterSeasonRow {
    pub provider: String,
    pub provider_player_id: String,
    pub name: String,
    pub team_code: String,
    pub position: String,
    pub active: bool,
    pub rookie: bool,
    pub games_played: u32,
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
    pub plus_minus: i32,
    pub penalty_minutes: u32,
    pub power_play_goals: u32,
    pub short_handed_goals: u32,
    pub shots: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlGoalieSeasonRow {
    pub provider: String,
    pub provider_player_id: String,
    pub name: String,
    pub team_code: String,
    pub active: bool,
    pub rookie: bool,
    pub games_played: u32,
    pub minutes_played: String,
    pub wins: u32,
    pub losses: u32,
    pub overtime_losses: u32,
    pub shots_against: u32,
    pub saves: u32,
    pub goals_against: u32,
    pub shutouts: u32,
    pub save_percentage: f64,
    pub goals_against_average: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct ProviderSeason {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct SeasonsEnvelope {
    seasons: Vec<ProviderSeason>,
}

#[derive(Debug, Clone, Deserialize)]
struct ProviderTeam {
    id: String,
    name: String,
    #[serde(default)]
    nickname: String,
    team_code: String,
    #[serde(default)]
    division_id: String,
    #[serde(default)]
    logo: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TeamsEnvelope {
    teams_no_all: Vec<ProviderTeam>,
}

/// Client for the feed behind the official AHL statistics pages.
#[derive(Debug, Clone)]
pub struct AhlFeedClient {
    client: reqwest::Client,
    base_url: String,
    key: String,
    client_code: String,
    cache: Option<(std::path::PathBuf, bool)>,
}

impl AhlFeedClient {
    pub fn production() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("IceLines/ahl-roster-stats")
            .build()
            .expect("valid AHL HTTP client configuration");
        Self {
            client,
            base_url: AHL_FEED_BASE_URL.to_owned(),
            key: AHL_FEED_KEY.to_owned(),
            client_code: AHL_CLIENT_CODE.to_owned(),
            cache: None,
        }
    }

    /// Production client whose source bytes are acquired through FLETCH's
    /// verified cacheline and shared cache manifest before IceLines parses.
    pub fn production_cached(cache_root: impl Into<std::path::PathBuf>, force: bool) -> Self {
        let mut client = Self::production();
        client.cache = Some((cache_root.into(), force));
        client
    }

    #[cfg(test)]
    fn with_base_url(base_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
            key: AHL_FEED_KEY.to_owned(),
            client_code: AHL_CLIENT_CODE.to_owned(),
            cache: None,
        }
    }

    #[cfg(test)]
    fn with_base_url_and_cache(base_url: String, cache_root: std::path::PathBuf) -> Self {
        let mut client = Self::with_base_url(base_url);
        client.cache = Some((cache_root, false));
        client
    }

    /// Fetch one league snapshot. `team_filters` accepts AHL codes or exact
    /// AHL team names; an empty slice means every team in the provider catalog.
    pub async fn fetch_roster_stats(
        &self,
        season: u32,
        team_filters: &[String],
    ) -> Result<AhlRosterStatsSnapshot, AhlFeedError> {
        let provider_season = self.resolve_regular_season(season).await?;
        let mut teams = self.fetch_teams(season, &provider_season.id).await?;
        teams.sort_by(|a, b| a.name.cmp(&b.name));
        teams = filter_teams(teams, team_filters)?;

        let affiliate_by_name = current_affiliates_for(season);
        let mut output = Vec::with_capacity(teams.len());
        for team in teams {
            let (mut roster, mut source_warnings) = self
                .fetch_roster(season, &provider_season.id, &team)
                .await?;
            let (mut skaters, skater_warnings) = self
                .fetch_skaters(season, &provider_season.id, &team)
                .await?;
            source_warnings.extend(skater_warnings);
            let (mut goalies, goalie_warnings) = self
                .fetch_goalies(season, &provider_season.id, &team)
                .await?;
            source_warnings.extend(goalie_warnings);
            roster.sort_by(|a, b| {
                a.position_group
                    .cmp(&b.position_group)
                    .then(a.name.cmp(&b.name))
                    .then(a.provider_player_id.cmp(&b.provider_player_id))
            });
            skaters.sort_by(|a, b| {
                a.name
                    .cmp(&b.name)
                    .then(a.provider_player_id.cmp(&b.provider_player_id))
            });
            goalies.sort_by(|a, b| {
                a.name
                    .cmp(&b.name)
                    .then(a.provider_player_id.cmp(&b.provider_player_id))
            });
            output.push(AhlTeamRosterStats {
                provider: AHL_PROVIDER.to_owned(),
                provider_team_id: team.id,
                team_code: team.team_code,
                nhl_affiliate: affiliate_by_name.get(&team.name).cloned(),
                team_name: team.name,
                nickname: team.nickname,
                division_id: team.division_id,
                logo_url: team.logo,
                roster,
                skaters,
                goalies,
                source_warnings,
            });
        }

        let snapshot = AhlRosterStatsSnapshot {
            schema: AHL_ROSTER_STATS_SCHEMA.to_owned(),
            season,
            provider: AHL_PROVIDER.to_owned(),
            provider_season_id: provider_season.id,
            provider_season_name: provider_season.name,
            fetched_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            source_url: AHL_STATS_SOURCE_URL.to_owned(),
            roster_source_url: AHL_ROSTER_SOURCE_URL.to_owned(),
            identity_note: "provider_player_id is an AHL HockeyTech identifier, not an NHL player_id; link only through an explicit crosswalk".to_owned(),
            teams: output,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    async fn resolve_regular_season(&self, season: u32) -> Result<ProviderSeason, AhlFeedError> {
        let target = season_label(season)?;
        let dataset_id = format!("icelines.ahl.{season}.catalog.seasons");
        let value = self
            .get_feed(
                &dataset_id,
                &[("view", "seasonsForLeague"), ("league", "4")],
            )
            .await?;
        let envelope: SeasonsEnvelope = serde_json::from_value(value)
            .map_err(|e| AhlFeedError::Schema(format!("season catalog: {e}")))?;
        envelope
            .seasons
            .into_iter()
            .find(|row| row.name == target)
            .ok_or(AhlFeedError::SeasonNotFound(target))
    }

    async fn fetch_teams(
        &self,
        season: u32,
        provider_season_id: &str,
    ) -> Result<Vec<ProviderTeam>, AhlFeedError> {
        let dataset_id = format!("icelines.ahl.{season}.catalog.teams");
        let value = self
            .get_feed(
                &dataset_id,
                &[("view", "teamsForSeason"), ("season", provider_season_id)],
            )
            .await?;
        let envelope: TeamsEnvelope = serde_json::from_value(value)
            .map_err(|e| AhlFeedError::Schema(format!("team catalog: {e}")))?;
        if envelope.teams_no_all.is_empty() {
            return Err(AhlFeedError::Schema("team catalog was empty".to_owned()));
        }
        Ok(envelope.teams_no_all)
    }

    async fn fetch_skaters(
        &self,
        season: u32,
        provider_season_id: &str,
        team: &ProviderTeam,
    ) -> Result<(Vec<AhlSkaterSeasonRow>, Vec<String>), AhlFeedError> {
        let value = self
            .fetch_player_report(
                &format!("icelines.ahl.{season}.team.{}.skaters", team.team_code),
                provider_season_id,
                &team.id,
                "skaters",
                "points",
            )
            .await?;
        let (rows, warnings) = team_report_rows(&value, &team.team_code, "skater", true)?;
        let parsed = rows
            .into_iter()
            .map(|row| parse_skater(row, &team.team_code))
            .collect::<Result<Vec<_>, _>>()?;
        Ok((parsed, warnings))
    }

    async fn fetch_roster(
        &self,
        season: u32,
        provider_season_id: &str,
        team: &ProviderTeam,
    ) -> Result<(Vec<AhlRosterPlayer>, Vec<String>), AhlFeedError> {
        let dataset_id = format!("icelines.ahl.{season}.team.{}.roster", team.team_code);
        let value = self
            .get_feed(
                &dataset_id,
                &[
                    ("view", "roster"),
                    ("team_id", &team.id),
                    ("season_id", provider_season_id),
                    ("rosterstatus", "all"),
                    ("site_id", "0"),
                    ("league_id", "4"),
                    ("lang", "en"),
                ],
            )
            .await?;
        let players = roster_rows(&value)?
            .into_iter()
            .map(|(group, row)| parse_roster_player(group, row))
            .collect::<Result<Vec<_>, _>>()?;
        deduplicate_roster_players(players, &team.team_code)
    }

    async fn fetch_goalies(
        &self,
        season: u32,
        provider_season_id: &str,
        team: &ProviderTeam,
    ) -> Result<(Vec<AhlGoalieSeasonRow>, Vec<String>), AhlFeedError> {
        let value = self
            .fetch_player_report(
                &format!("icelines.ahl.{season}.team.{}.goalies", team.team_code),
                provider_season_id,
                &team.id,
                "goalies",
                "wins",
            )
            .await?;
        let (rows, warnings) = team_report_rows(&value, &team.team_code, "goalie", false)?;
        let parsed = rows
            .into_iter()
            .map(|row| parse_goalie(row, &team.team_code))
            .collect::<Result<Vec<_>, _>>()?;
        Ok((parsed, warnings))
    }

    async fn fetch_player_report(
        &self,
        dataset_id: &str,
        season: &str,
        team: &str,
        position: &str,
        sort: &str,
    ) -> Result<Value, AhlFeedError> {
        self.get_feed(
            dataset_id,
            &[
                ("view", "players"),
                ("season", season),
                ("team", team),
                ("position", position),
                ("rookies", "0"),
                ("statsType", "standard"),
                ("rosterstatus", "all"),
                ("first", "0"),
                ("limit", "500"),
                ("lang", "en"),
                ("sort", sort),
            ],
        )
        .await
    }

    async fn get_feed(
        &self,
        dataset_id: &str,
        params: &[(&str, &str)],
    ) -> Result<Value, AhlFeedError> {
        let mut query = vec![
            ("feed", "statviewfeed"),
            ("key", self.key.as_str()),
            ("client_code", self.client_code.as_str()),
        ];
        query.extend_from_slice(params);
        let request = self.client.get(&self.base_url).query(&query);
        let url = request
            .try_clone()
            .and_then(|r| r.build().ok())
            .map(|r| r.url().to_string())
            .unwrap_or_else(|| self.base_url.clone());
        if let Some((cache_root, force)) = &self.cache {
            let bytes = crate::fletch::fetch_generic_http_bytes_async(
                dataset_id.to_owned(),
                url.clone(),
                cache_root.clone(),
                *force,
            )
            .await
            .map_err(|e| AhlFeedError::Request {
                url,
                detail: format!("FLETCH cache acquisition failed: {e:#}"),
            })?;
            let body = std::str::from_utf8(&bytes).map_err(|e| {
                AhlFeedError::Schema(format!("{dataset_id} returned non-UTF-8 bytes: {e}"))
            })?;
            return parse_jsonp(body);
        }
        let response = request.send().await.map_err(|e| AhlFeedError::Request {
            url: url.clone(),
            detail: e.to_string(),
        })?;
        let status = response.status();
        if !status.is_success() {
            return Err(AhlFeedError::Http {
                status: status.as_u16(),
                url,
            });
        }
        let body = response.text().await.map_err(|e| AhlFeedError::Request {
            url,
            detail: e.to_string(),
        })?;
        parse_jsonp(&body)
    }
}

impl AhlRosterStatsSnapshot {
    pub fn validate(&self) -> Result<(), AhlFeedError> {
        if self.schema != AHL_ROSTER_STATS_SCHEMA {
            return Err(AhlFeedError::Validation(format!(
                "unexpected schema {}",
                self.schema
            )));
        }
        let mut team_ids = BTreeSet::new();
        let mut team_codes = BTreeSet::new();
        for team in &self.teams {
            if !team_ids.insert(team.provider_team_id.as_str()) {
                return Err(AhlFeedError::Validation(format!(
                    "duplicate provider team id {}",
                    team.provider_team_id
                )));
            }
            if !team_codes.insert(team.team_code.as_str()) {
                return Err(AhlFeedError::Validation(format!(
                    "duplicate AHL team code {}",
                    team.team_code
                )));
            }
            validate_player_ids(
                team,
                &team
                    .skaters
                    .iter()
                    .map(|p| (p.provider_player_id.as_str(), p.team_code.as_str()))
                    .collect::<Vec<_>>(),
            )?;
            validate_player_ids(
                team,
                &team
                    .goalies
                    .iter()
                    .map(|p| (p.provider_player_id.as_str(), p.team_code.as_str()))
                    .collect::<Vec<_>>(),
            )?;
            let mut roster_ids = BTreeSet::new();
            for player in &team.roster {
                if !roster_ids.insert(player.provider_player_id.as_str()) {
                    return Err(AhlFeedError::Validation(format!(
                        "duplicate roster provider player id {} on {}",
                        player.provider_player_id, team.team_code
                    )));
                }
            }
            for player in &team.skaters {
                if player.goals + player.assists != player.points {
                    return Err(AhlFeedError::Validation(format!(
                        "{} points do not equal goals plus assists",
                        player.name
                    )));
                }
            }
        }
        Ok(())
    }
}

/// Adapt one official AHL team roster into the core affiliate projection
/// contract. Every roster player must have an explicit provider→NHL crosswalk
/// plus the scenario facts that the official feed does not establish.
pub fn affiliate_projection_input_from_snapshot(
    snapshot: &AhlRosterStatsSnapshot,
    nhl_team: &str,
    ahl_team: &str,
    rule: icelines_core::view_model::ahl_affiliate::AhlDevelopmentRuleInput,
    enrichments: &[AhlProjectionPlayerEnrichment],
) -> Result<icelines_core::view_model::ahl_affiliate::AhlAffiliateProjectionInput, AhlFeedError> {
    use icelines_core::view_model::ahl_affiliate::{
        AhlAffiliatePlayerInput, AhlAffiliateProjectionInput,
    };

    snapshot.validate()?;
    let team = snapshot
        .teams
        .iter()
        .find(|team| team.team_name == ahl_team)
        .ok_or_else(|| {
            AhlFeedError::Validation(format!("AHL snapshot has no team named `{ahl_team}`"))
        })?;
    if team
        .nhl_affiliate
        .as_deref()
        .is_some_and(|affiliate| affiliate != nhl_team)
    {
        return Err(AhlFeedError::Validation(format!(
            "{} snapshot affiliate is {}, not {nhl_team}",
            team.team_name,
            team.nhl_affiliate.as_deref().unwrap_or_default()
        )));
    }

    let mut by_provider_id = BTreeMap::new();
    let mut nhl_ids = BTreeSet::new();
    for enrichment in enrichments {
        if enrichment.provider_player_id.trim().is_empty()
            || enrichment.nhl_player_id == 0
            || !enrichment.projected_score.is_finite()
            || enrichment.recall_readiness.is_some_and(|readiness| {
                !readiness.is_finite() || !(0.0..=1.0).contains(&readiness)
            })
            || !enrichment
                .eligible_positions
                .contains(&enrichment.primary_position)
            || (enrichment.assigned_to_affiliate
                && enrichment.primary_position != icelines_core::model::Position::Goalie
                && enrichment.professional_games_at_season_start.is_none())
            || by_provider_id
                .insert(enrichment.provider_player_id.as_str(), enrichment)
                .is_some()
            || !nhl_ids.insert(enrichment.nhl_player_id)
        {
            return Err(AhlFeedError::Validation(
                "AHL projection crosswalk contains blank or duplicate identities".to_owned(),
            ));
        }
    }

    let official_ids = team
        .roster
        .iter()
        .map(|player| player.provider_player_id.as_str())
        .collect::<BTreeSet<_>>();
    let missing = official_ids
        .iter()
        .filter(|id| !by_provider_id.contains_key(**id))
        .copied()
        .collect::<Vec<_>>();
    let extra = by_provider_id
        .keys()
        .filter(|id| !official_ids.contains(**id))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() || !extra.is_empty() {
        return Err(AhlFeedError::Validation(format!(
            "AHL projection crosswalk must exactly cover the official roster; missing=[{}], extra=[{}]",
            missing.join(","),
            extra.join(",")
        )));
    }

    let players = team
        .roster
        .iter()
        .map(|official| {
            let enrichment = by_provider_id[official.provider_player_id.as_str()];
            AhlAffiliatePlayerInput {
                player_id: enrichment.nhl_player_id,
                display_name: official.name.clone(),
                primary_position: enrichment.primary_position,
                eligible_positions: enrichment.eligible_positions.clone(),
                projected_score: enrichment.projected_score,
                prospect: enrichment.prospect,
                recall_readiness: enrichment.recall_readiness,
                professional_games_at_season_start: enrichment.professional_games_at_season_start,
                assigned_to_affiliate: enrichment.assigned_to_affiliate,
                waiver_required: enrichment.waiver_required,
                source_league: "AHL".to_owned(),
            }
        })
        .collect();

    let input = AhlAffiliateProjectionInput {
        nhl_team: nhl_team.to_owned(),
        ahl_team: team.team_name.clone(),
        season: snapshot.season,
        rule,
        pool_authority: icelines_core::AhlRosterPoolAuthority {
            kind: icelines_core::AhlRosterPoolAuthorityKind::OfficialSnapshot,
            as_of: Some(snapshot.fetched_at.clone()),
            source_urls: vec![snapshot.roster_source_url.clone()],
            note: Some(format!(
                "Official {} roster snapshot for {}",
                snapshot.provider, team.team_name
            )),
        },
        players,
    };
    Ok(input)
}

/// Build a deterministic review queue. Exact official name and birth-date
/// agreement is a high-confidence proposal, but remains pending until a human
/// changes `review_status` to `reviewed`.
pub fn build_ahl_identity_crosswalk(
    snapshot: &AhlRosterStatsSnapshot,
    ahl_team: &str,
    catalog: &AhlCanonicalIdentityCatalog,
) -> Result<AhlIdentityCrosswalkView, AhlFeedError> {
    snapshot.validate()?;
    validate_identity_catalog(catalog)?;
    let team = snapshot
        .teams
        .iter()
        .find(|team| team.team_name == ahl_team)
        .ok_or_else(|| {
            AhlFeedError::Validation(format!("AHL snapshot has no team named `{ahl_team}`"))
        })?;
    let mut by_name = BTreeMap::<String, Vec<&AhlCanonicalIdentityCandidate>>::new();
    let mut by_surname = BTreeMap::<String, Vec<&AhlCanonicalIdentityCandidate>>::new();
    for candidate in &catalog.candidates {
        by_name
            .entry(normalize_ahl_identity_name(&candidate.display_name))
            .or_default()
            .push(candidate);
        if let Some(surname) = normalized_surname(&candidate.display_name) {
            by_surname.entry(surname).or_default().push(candidate);
        }
    }

    let mut rows = team
        .roster
        .iter()
        .map(|official| {
            let candidates = by_name
                .get(&normalize_ahl_identity_name(&official.name))
                .map(Vec::as_slice)
                .unwrap_or_default();
            let aliases = if candidates.is_empty() {
                normalized_surname(&official.name)
                    .and_then(|surname| by_surname.get(&surname))
                    .map(Vec::as_slice)
                    .unwrap_or_default()
            } else {
                &[]
            };
            identity_crosswalk_row(official, candidates, aliases)
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| {
        a.ahl_display_name
            .cmp(&b.ahl_display_name)
            .then_with(|| a.provider_player_id.cmp(&b.provider_player_id))
    });
    let counts = identity_crosswalk_counts(&rows);
    Ok(AhlIdentityCrosswalkView {
        schema: AHL_IDENTITY_CROSSWALK_SCHEMA.to_owned(),
        season: snapshot.season,
        provider: snapshot.provider.clone(),
        ahl_team: team.team_name.clone(),
        nhl_affiliate: team.nhl_affiliate.clone(),
        roster_fetched_at: snapshot.fetched_at.clone(),
        candidates_checked_at: catalog.checked_at.clone(),
        counts,
        rows,
        disclosures: vec![
            "AHL provider_player_id values remain provider-local and are never copied into NHL player IDs.".to_owned(),
            "Even exact normalized-name and birth-date matches are proposals until review_status is explicitly changed to reviewed.".to_owned(),
            "Surname-and-birth-date proposals are possible aliases, never exact matches, and require an explicit sourced remap.".to_owned(),
            "Identity approval does not establish roster assignment, prospect status, professional-game totals, waivers, player value, or recall readiness.".to_owned(),
        ],
    })
}

/// Join a fully reviewed identity artifact to separately authored projection
/// facts and feed the existing exact-coverage snapshot adapter.
pub fn affiliate_projection_input_from_reviewed_crosswalk(
    snapshot: &AhlRosterStatsSnapshot,
    nhl_team: &str,
    ahl_team: &str,
    rule: icelines_core::view_model::ahl_affiliate::AhlDevelopmentRuleInput,
    crosswalk: &AhlIdentityCrosswalkView,
    facts: &[AhlProjectionPlayerFacts],
) -> Result<icelines_core::view_model::ahl_affiliate::AhlAffiliateProjectionInput, AhlFeedError> {
    validate_reviewed_ahl_identity_crosswalk(snapshot, ahl_team, crosswalk)?;
    let identities = crosswalk
        .rows
        .iter()
        .map(|row| {
            (
                row.provider_player_id.as_str(),
                row.nhl_player_id
                    .expect("review validation requires NHL id"),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let enrichments = facts
        .iter()
        .map(|fact| {
            let nhl_player_id = identities
                .get(fact.provider_player_id.as_str())
                .copied()
                .ok_or_else(|| {
                    AhlFeedError::Validation(format!(
                        "projection facts reference provider player {} absent from reviewed crosswalk",
                        fact.provider_player_id
                    ))
                })?;
            Ok(AhlProjectionPlayerEnrichment {
                provider_player_id: fact.provider_player_id.clone(),
                nhl_player_id,
                primary_position: fact.primary_position,
                eligible_positions: fact.eligible_positions.clone(),
                projected_score: fact.projected_score,
                prospect: fact.prospect,
                recall_readiness: fact.recall_readiness,
                professional_games_at_season_start: fact.professional_games_at_season_start,
                assigned_to_affiliate: fact.assigned_to_affiliate,
                waiver_required: fact.waiver_required,
            })
        })
        .collect::<Result<Vec<_>, AhlFeedError>>()?;
    affiliate_projection_input_from_snapshot(snapshot, nhl_team, ahl_team, rule, &enrichments)
}

pub fn validate_reviewed_ahl_identity_crosswalk(
    snapshot: &AhlRosterStatsSnapshot,
    ahl_team: &str,
    crosswalk: &AhlIdentityCrosswalkView,
) -> Result<(), AhlFeedError> {
    snapshot.validate()?;
    if crosswalk.schema != AHL_IDENTITY_CROSSWALK_SCHEMA
        || crosswalk.season != snapshot.season
        || crosswalk.provider != snapshot.provider
        || crosswalk.ahl_team != ahl_team
        || crosswalk.roster_fetched_at != snapshot.fetched_at
        || crosswalk.candidates_checked_at.trim().is_empty()
    {
        return Err(AhlFeedError::Validation(
            "identity crosswalk does not match the selected AHL snapshot/team authority".to_owned(),
        ));
    }
    let team = snapshot
        .teams
        .iter()
        .find(|team| team.team_name == ahl_team)
        .ok_or_else(|| {
            AhlFeedError::Validation(format!("AHL snapshot has no team named `{ahl_team}`"))
        })?;
    if team.roster.is_empty() {
        return Err(AhlFeedError::Validation(format!(
            "official AHL roster for {ahl_team} is empty and cannot establish projection identity coverage"
        )));
    }
    if crosswalk.nhl_affiliate != team.nhl_affiliate {
        return Err(AhlFeedError::Validation(
            "identity crosswalk NHL affiliate differs from the snapshot".to_owned(),
        ));
    }
    let official = team
        .roster
        .iter()
        .map(|row| (row.provider_player_id.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let mut provider_ids = BTreeSet::new();
    let mut nhl_ids = BTreeSet::new();
    for row in &crosswalk.rows {
        let source = official
            .get(row.provider_player_id.as_str())
            .ok_or_else(|| {
                AhlFeedError::Validation(format!(
                    "identity crosswalk contains extra provider player {}",
                    row.provider_player_id
                ))
            })?;
        if !provider_ids.insert(row.provider_player_id.as_str())
            || row.ahl_display_name != source.name
            || row.ahl_birth_date != source.birthdate
        {
            return Err(AhlFeedError::Validation(format!(
                "identity crosswalk altered or duplicated official AHL identity {}",
                row.provider_player_id
            )));
        }
        if row.review_status != AhlIdentityReviewStatus::Reviewed {
            return Err(AhlFeedError::Validation(format!(
                "identity {} is not reviewed",
                row.provider_player_id
            )));
        }
        let nhl_id = row.nhl_player_id.filter(|id| *id != 0).ok_or_else(|| {
            AhlFeedError::Validation(format!(
                "reviewed identity {} has no NHL player ID",
                row.provider_player_id
            ))
        })?;
        if !nhl_ids.insert(nhl_id)
            || row
                .nhl_display_name
                .as_deref()
                .is_none_or(|name| name.trim().is_empty())
            || row.evidence_urls.is_empty()
            || row.evidence_urls.iter().any(|url| !absolute_http_url(url))
            || row.nhl_birth_date.as_deref().is_some_and(|date| {
                !source.birthdate.is_empty()
                    && date != source.birthdate
                    && !matches!(
                        row.match_basis,
                        AhlIdentityMatchBasis::BirthDateConflict
                            | AhlIdentityMatchBasis::ReviewedOverride
                    )
            })
        {
            return Err(AhlFeedError::Validation(format!(
                "reviewed identity {} has invalid or conflicting NHL evidence",
                row.provider_player_id
            )));
        }
    }
    if provider_ids.len() != official.len() {
        let missing = official
            .keys()
            .filter(|id| !provider_ids.contains(**id))
            .copied()
            .collect::<Vec<_>>();
        return Err(AhlFeedError::Validation(format!(
            "identity crosswalk is missing provider players [{}]",
            missing.join(",")
        )));
    }
    Ok(())
}

fn validate_identity_catalog(catalog: &AhlCanonicalIdentityCatalog) -> Result<(), AhlFeedError> {
    validate_identity_catalog_authority(catalog)?;
    let mut ids = BTreeSet::new();
    for candidate in &catalog.candidates {
        validate_identity_candidate(candidate)?;
        if !ids.insert(candidate.nhl_player_id) {
            return Err(AhlFeedError::Validation(
                "canonical NHL identity catalog contains invalid or duplicate candidates"
                    .to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_identity_catalog_authority(
    catalog: &AhlCanonicalIdentityCatalog,
) -> Result<(), AhlFeedError> {
    if catalog.schema != AHL_CANONICAL_IDENTITY_CATALOG_SCHEMA
        || catalog.checked_at.trim().is_empty()
    {
        return Err(AhlFeedError::Validation(
            "invalid canonical NHL identity catalog authority".to_owned(),
        ));
    }
    Ok(())
}

fn validate_identity_candidate(
    candidate: &AhlCanonicalIdentityCandidate,
) -> Result<(), AhlFeedError> {
    if candidate.nhl_player_id == 0
        || normalize_ahl_identity_name(&candidate.display_name).is_empty()
        || candidate.evidence_urls.is_empty()
        || candidate
            .evidence_urls
            .iter()
            .any(|url| !absolute_http_url(url))
        || candidate
            .birth_date
            .as_deref()
            .is_some_and(|date| chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err())
    {
        return Err(AhlFeedError::Validation(
            "canonical NHL identity catalog contains invalid or duplicate candidates".to_owned(),
        ));
    }
    Ok(())
}

fn identity_crosswalk_row(
    official: &AhlRosterPlayer,
    candidates: &[&AhlCanonicalIdentityCandidate],
    alias_candidates: &[&AhlCanonicalIdentityCandidate],
) -> AhlIdentityCrosswalkRow {
    let birth_matches = candidates
        .iter()
        .copied()
        .filter(|candidate| {
            candidate
                .birth_date
                .as_deref()
                .is_some_and(|date| !official.birthdate.is_empty() && date == official.birthdate)
        })
        .collect::<Vec<_>>();
    let alias_birth_matches = alias_candidates
        .iter()
        .copied()
        .filter(|candidate| {
            candidate
                .birth_date
                .as_deref()
                .is_some_and(|date| !official.birthdate.is_empty() && date == official.birthdate)
        })
        .collect::<Vec<_>>();
    let (basis, candidate, note) = if birth_matches.len() == 1 {
        (
            AhlIdentityMatchBasis::ExactNameAndBirthDate,
            Some(birth_matches[0]),
            "Exact normalized name and birth date; human review still required.",
        )
    } else if candidates.len() == 1 {
        let candidate = candidates[0];
        if candidate.birth_date.is_some()
            && !official.birthdate.is_empty()
            && candidate.birth_date.as_deref() != Some(official.birthdate.as_str())
        {
            (
                AhlIdentityMatchBasis::BirthDateConflict,
                Some(candidate),
                "Exact normalized name but conflicting birth date; candidate evidence is retained for review.",
            )
        } else {
            (
                AhlIdentityMatchBasis::ExactNameOnly,
                Some(candidate),
                "Exact normalized name with incomplete birth-date corroboration; human review required.",
            )
        }
    } else if candidates.is_empty() && alias_birth_matches.len() == 1 {
        (
            AhlIdentityMatchBasis::SurnameAndBirthDate,
            Some(alias_birth_matches[0]),
            "Official surname and birth date agree but display names differ; explicit alias review is required.",
        )
    } else if candidates.is_empty() && alias_birth_matches.len() > 1 {
        (
            AhlIdentityMatchBasis::Ambiguous,
            None,
            "Multiple official surname-and-birth-date candidates remain unresolved.",
        )
    } else if candidates.is_empty() {
        (
            AhlIdentityMatchBasis::Unmatched,
            None,
            "No exact normalized-name candidate.",
        )
    } else {
        (
            AhlIdentityMatchBasis::Ambiguous,
            None,
            "Multiple exact normalized-name candidates remain unresolved.",
        )
    };
    AhlIdentityCrosswalkRow {
        provider_player_id: official.provider_player_id.clone(),
        ahl_display_name: official.name.clone(),
        ahl_birth_date: official.birthdate.clone(),
        match_basis: basis,
        review_status: AhlIdentityReviewStatus::Pending,
        nhl_player_id: candidate.map(|candidate| candidate.nhl_player_id),
        nhl_display_name: candidate.map(|candidate| candidate.display_name.clone()),
        nhl_birth_date: candidate.and_then(|candidate| candidate.birth_date.clone()),
        evidence_urls: candidate
            .map(|candidate| candidate.evidence_urls.clone())
            .unwrap_or_default(),
        note: note.to_owned(),
    }
}

fn identity_crosswalk_counts(rows: &[AhlIdentityCrosswalkRow]) -> AhlIdentityCrosswalkCounts {
    AhlIdentityCrosswalkCounts {
        roster_players: rows.len(),
        exact_name_and_birth_date: rows
            .iter()
            .filter(|row| row.match_basis == AhlIdentityMatchBasis::ExactNameAndBirthDate)
            .count(),
        surname_and_birth_date: rows
            .iter()
            .filter(|row| row.match_basis == AhlIdentityMatchBasis::SurnameAndBirthDate)
            .count(),
        exact_name_only: rows
            .iter()
            .filter(|row| row.match_basis == AhlIdentityMatchBasis::ExactNameOnly)
            .count(),
        ambiguous: rows
            .iter()
            .filter(|row| row.match_basis == AhlIdentityMatchBasis::Ambiguous)
            .count(),
        conflicts: rows
            .iter()
            .filter(|row| row.match_basis == AhlIdentityMatchBasis::BirthDateConflict)
            .count(),
        unmatched: rows
            .iter()
            .filter(|row| row.match_basis == AhlIdentityMatchBasis::Unmatched)
            .count(),
        reviewed: rows
            .iter()
            .filter(|row| row.review_status == AhlIdentityReviewStatus::Reviewed)
            .count(),
    }
}

fn absolute_http_url(value: &str) -> bool {
    value.starts_with("https://") || value.starts_with("http://")
}

fn validate_player_ids(
    team: &AhlTeamRosterStats,
    players: &[(&str, &str)],
) -> Result<(), AhlFeedError> {
    let mut ids = BTreeSet::new();
    for (id, code) in players {
        if !ids.insert(*id) {
            return Err(AhlFeedError::Validation(format!(
                "duplicate provider player id {id} on {}",
                team.team_code
            )));
        }
        if *code != team.team_code {
            return Err(AhlFeedError::Validation(format!(
                "player team code {code} does not match {}",
                team.team_code
            )));
        }
    }
    Ok(())
}

fn filter_teams(
    teams: Vec<ProviderTeam>,
    filters: &[String],
) -> Result<Vec<ProviderTeam>, AhlFeedError> {
    if filters.is_empty() {
        return Ok(teams);
    }
    let wanted: BTreeSet<String> = filters
        .iter()
        .map(|s| s.trim().to_ascii_uppercase())
        .collect();
    let selected: Vec<_> = teams
        .into_iter()
        .filter(|team| {
            wanted.contains(&team.team_code.to_ascii_uppercase())
                || wanted.contains(&team.name.to_ascii_uppercase())
        })
        .collect();
    let found: BTreeSet<String> = selected
        .iter()
        .flat_map(|team| {
            [
                team.team_code.to_ascii_uppercase(),
                team.name.to_ascii_uppercase(),
            ]
        })
        .collect();
    let unknown: Vec<_> = wanted.difference(&found).cloned().collect();
    if !unknown.is_empty() {
        return Err(AhlFeedError::UnknownTeams(unknown.join(", ")));
    }
    Ok(selected)
}

fn season_label(season: u32) -> Result<String, AhlFeedError> {
    let text = format!("{season:08}");
    let start: u32 = text[..4]
        .parse()
        .map_err(|_| AhlFeedError::SeasonNotFound(text.clone()))?;
    let end: u32 = text[4..]
        .parse()
        .map_err(|_| AhlFeedError::SeasonNotFound(text.clone()))?;
    if end != start + 1 {
        return Err(AhlFeedError::SeasonNotFound(text));
    }
    Ok(format!("{start}-{:02} Regular Season", end % 100))
}

fn current_affiliates_for(season: u32) -> BTreeMap<String, String> {
    if season != icelines_core::view_model::ahl_affiliate::CURRENT_AHL_AFFILIATION_SEASON {
        return BTreeMap::new();
    }
    icelines_core::view_model::ahl_affiliate::current_ahl_affiliation_catalog()
        .affiliations
        .into_iter()
        .map(|row| (row.ahl_team, row.nhl_team))
        .collect()
}

/// Parse the JSONP wrappers used by Statview (`({...})` and `([...])`).
pub fn parse_jsonp(body: &str) -> Result<Value, AhlFeedError> {
    let trimmed = body.trim();
    let json = trimmed
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .ok_or_else(|| AhlFeedError::Schema("expected parenthesized JSONP body".to_owned()))?;
    serde_json::from_str(json)
        .map_err(|e| AhlFeedError::Schema(format!("invalid JSONP payload: {e}")))
}

fn report_rows(value: &Value) -> Result<Vec<&Value>, AhlFeedError> {
    let reports = value
        .as_array()
        .ok_or_else(|| AhlFeedError::Schema("player report root was not an array".to_owned()))?;
    let mut rows = Vec::new();
    for report in reports {
        let sections = report
            .get("sections")
            .and_then(Value::as_array)
            .ok_or_else(|| AhlFeedError::Schema("player report sections missing".to_owned()))?;
        for section in sections {
            let data = section
                .get("data")
                .and_then(Value::as_array)
                .ok_or_else(|| AhlFeedError::Schema("player report data missing".to_owned()))?;
            for item in data {
                let row = item
                    .get("row")
                    .ok_or_else(|| AhlFeedError::Schema("player report row missing".to_owned()))?;
                if row.get("player_id").is_some() {
                    rows.push(row);
                    continue;
                }
                let label = row
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .trim();
                if !matches!(label, "Empty Net" | "Totals") {
                    return Err(AhlFeedError::Schema(format!(
                        "player report row `{label}` had no player_id"
                    )));
                }
            }
        }
    }
    Ok(rows)
}

fn team_report_rows<'a>(
    value: &'a Value,
    expected_team: &str,
    report_kind: &str,
    exclude_goalies: bool,
) -> Result<(Vec<&'a Value>, Vec<String>), AhlFeedError> {
    let rows = report_rows(value)?;
    let mut retained = Vec::new();
    let mut wrong_team = Vec::new();
    let mut goalie_scoring_rows = Vec::new();
    for row in rows {
        let actual_team = string_field(row, "team_code")?;
        let identity = format!(
            "{} #{}",
            string_field(row, "name")?,
            string_field(row, "player_id")?
        );
        if actual_team != expected_team {
            wrong_team.push(format!("{identity} ({actual_team})"));
        } else if exclude_goalies && string_field(row, "position")? == "G" {
            goalie_scoring_rows.push(identity);
        } else {
            retained.push(row);
        }
    }
    if retained.is_empty() && !wrong_team.is_empty() {
        return Err(AhlFeedError::Validation(format!(
            "{report_kind} report for {expected_team} contained only other-team rows: {}",
            wrong_team.join(", ")
        )));
    }
    let mut warnings = Vec::new();
    if !wrong_team.is_empty() {
        warnings.push(format!(
            "Excluded {} other-team row(s) from the {report_kind} report for {expected_team}: {}.",
            wrong_team.len(),
            wrong_team.join(", ")
        ));
    }
    if !goalie_scoring_rows.is_empty() {
        warnings.push(format!(
            "Excluded {} goalie scoring row(s) from the skater report for {expected_team}; typed goalie totals come from the separate goalie report: {}.",
            goalie_scoring_rows.len(),
            goalie_scoring_rows.join(", ")
        ));
    }
    Ok((retained, warnings))
}

fn roster_rows(value: &Value) -> Result<Vec<(&str, &Value)>, AhlFeedError> {
    let reports = value
        .get("roster")
        .and_then(Value::as_array)
        .ok_or_else(|| AhlFeedError::Schema("roster report missing".to_owned()))?;
    let mut rows = Vec::new();
    for report in reports {
        let sections = report
            .get("sections")
            .and_then(Value::as_array)
            .ok_or_else(|| AhlFeedError::Schema("roster sections missing".to_owned()))?;
        for section in sections {
            let title = section
                .get("title")
                .and_then(Value::as_str)
                .ok_or_else(|| AhlFeedError::Schema("roster section title missing".to_owned()))?;
            if title == "Team Personnel" {
                continue;
            }
            let data = section
                .get("data")
                .and_then(Value::as_array)
                .ok_or_else(|| AhlFeedError::Schema("roster section data missing".to_owned()))?;
            for item in data {
                let row = item
                    .get("row")
                    .ok_or_else(|| AhlFeedError::Schema("roster player row missing".to_owned()))?;
                rows.push((title, row));
            }
        }
    }
    Ok(rows)
}

fn deduplicate_roster_players(
    players: Vec<AhlRosterPlayer>,
    team_code: &str,
) -> Result<(Vec<AhlRosterPlayer>, Vec<String>), AhlFeedError> {
    let mut retained: Vec<AhlRosterPlayer> = Vec::new();
    let mut index_by_id = BTreeMap::new();
    let mut warnings = Vec::new();
    for player in players {
        let Some(existing_index) = index_by_id.get(&player.provider_player_id).copied() else {
            index_by_id.insert(player.provider_player_id.clone(), retained.len());
            retained.push(player);
            continue;
        };
        let existing = &mut retained[existing_index];
        let existing_jersey = existing.jersey_number.clone();
        let duplicate_jersey = player.jersey_number.clone();
        let existing_position = existing.position.clone();
        let duplicate_position = player.position.clone();
        let mut comparable_existing = existing.clone();
        let mut comparable_duplicate = player.clone();
        comparable_existing.jersey_number.clear();
        comparable_duplicate.jersey_number.clear();
        comparable_existing.position.clear();
        comparable_duplicate.position.clear();
        if comparable_existing != comparable_duplicate {
            return Err(AhlFeedError::Validation(format!(
                "conflicting duplicate roster rows for {} #{} on {team_code}",
                player.name, player.provider_player_id
            )));
        }
        let position_changed = existing_position != duplicate_position;
        if position_changed
            && !(is_forward_roster_position(&existing_position)
                && is_forward_roster_position(&duplicate_position))
        {
            return Err(AhlFeedError::Validation(format!(
                "conflicting duplicate roster positions `{existing_position}` and `{duplicate_position}` for {} #{} on {team_code}",
                player.name, player.provider_player_id
            )));
        }
        let jersey_changed = existing_jersey != duplicate_jersey;
        if position_changed || jersey_changed {
            let mut changes = Vec::new();
            if position_changed {
                existing.position = "F".to_owned();
                changes.push(format!(
                    "forward positions `{existing_position}` and `{duplicate_position}` were generalized to `F`"
                ));
            }
            if jersey_changed {
                existing.jersey_number.clear();
                changes.push(format!(
                    "jersey numbers `{existing_jersey}` and `{duplicate_jersey}` were omitted"
                ));
            }
            warnings.push(format!(
                "Collapsed compatible duplicate roster rows for {} #{} on {team_code}; {}.",
                player.name,
                player.provider_player_id,
                changes.join(" and ")
            ));
        } else {
            warnings.push(format!(
                "Collapsed an exact duplicate roster row for {} #{} on {team_code}.",
                player.name, player.provider_player_id
            ));
        }
    }
    Ok((retained, warnings))
}

fn is_forward_roster_position(position: &str) -> bool {
    matches!(position, "F" | "C" | "LW" | "RW")
}

fn parse_roster_player(group: &str, row: &Value) -> Result<AhlRosterPlayer, AhlFeedError> {
    let mut handedness = optional_string_field(row, "shoots");
    if handedness.is_empty() {
        handedness = optional_string_field(row, "catches");
    }
    Ok(AhlRosterPlayer {
        provider: AHL_PROVIDER.to_owned(),
        provider_player_id: string_field(row, "player_id")?,
        name: string_field(row, "name")?,
        position_group: group.to_owned(),
        position: string_field(row, "position")?,
        jersey_number: optional_string_field(row, "tp_jersey_number"),
        handedness,
        height: optional_string_field(row, "height_hyphenated"),
        weight_pounds: optional_string_field(row, "w"),
        birthdate: optional_string_field(row, "birthdate"),
        birthplace: optional_string_field(row, "birthplace"),
    })
}

fn parse_skater(row: &Value, expected_team: &str) -> Result<AhlSkaterSeasonRow, AhlFeedError> {
    Ok(AhlSkaterSeasonRow {
        provider: AHL_PROVIDER.to_owned(),
        provider_player_id: string_field(row, "player_id")?,
        name: string_field(row, "name")?,
        team_code: checked_team_code(row, expected_team)?,
        position: string_field(row, "position")?,
        active: bool_field(row, "active")?,
        rookie: bool_field(row, "rookie")?,
        games_played: u32_field(row, "games_played")?,
        goals: u32_field(row, "goals")?,
        assists: u32_field(row, "assists")?,
        points: u32_field(row, "points")?,
        plus_minus: i32_field(row, "plus_minus")?,
        penalty_minutes: u32_field(row, "penalty_minutes")?,
        power_play_goals: u32_field(row, "power_play_goals")?,
        short_handed_goals: u32_field(row, "short_handed_goals")?,
        shots: u32_field(row, "shots")?,
    })
}

fn parse_goalie(row: &Value, expected_team: &str) -> Result<AhlGoalieSeasonRow, AhlFeedError> {
    Ok(AhlGoalieSeasonRow {
        provider: AHL_PROVIDER.to_owned(),
        provider_player_id: string_field(row, "player_id")?,
        name: string_field(row, "name")?,
        team_code: checked_team_code(row, expected_team)?,
        active: bool_field(row, "active")?,
        rookie: bool_field(row, "rookie")?,
        games_played: u32_field(row, "games_played")?,
        minutes_played: string_field(row, "minutes_played")?,
        wins: u32_field(row, "wins")?,
        losses: u32_field(row, "losses")?,
        overtime_losses: u32_field(row, "ot_losses")?,
        shots_against: u32_field(row, "shots")?,
        saves: u32_field(row, "saves")?,
        goals_against: u32_field(row, "goals_against")?,
        shutouts: u32_field(row, "shutouts")?,
        save_percentage: f64_field(row, "save_percentage")?,
        goals_against_average: f64_field(row, "goals_against_average")?,
    })
}

fn checked_team_code(row: &Value, expected: &str) -> Result<String, AhlFeedError> {
    let actual = string_field(row, "team_code")?;
    if actual != expected {
        return Err(AhlFeedError::Validation(format!(
            "feed returned team code {actual} while fetching {expected}"
        )));
    }
    Ok(actual)
}

fn string_field(row: &Value, field: &str) -> Result<String, AhlFeedError> {
    row.get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| AhlFeedError::Schema(format!("missing string field `{field}`")))
}

fn optional_string_field(row: &Value, field: &str) -> String {
    row.get(field)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn bool_field(row: &Value, field: &str) -> Result<bool, AhlFeedError> {
    match string_field(row, field)?.as_str() {
        "1" => Ok(true),
        "0" => Ok(false),
        value => Err(AhlFeedError::Schema(format!(
            "invalid boolean `{field}` value {value}"
        ))),
    }
}

fn u32_field(row: &Value, field: &str) -> Result<u32, AhlFeedError> {
    string_field(row, field)?
        .parse()
        .map_err(|e| AhlFeedError::Schema(format!("invalid integer `{field}`: {e}")))
}

fn i32_field(row: &Value, field: &str) -> Result<i32, AhlFeedError> {
    string_field(row, field)?
        .parse()
        .map_err(|e| AhlFeedError::Schema(format!("invalid integer `{field}`: {e}")))
}

fn f64_field(row: &Value, field: &str) -> Result<f64, AhlFeedError> {
    string_field(row, field)?
        .parse()
        .map_err(|e| AhlFeedError::Schema(format!("invalid decimal `{field}`: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    const SKATER_REPORT: &str = r#"([{"sections":[{"data":[{"row":{"player_id":"7669","name":"Trey Fix-Wolansky","active":"1","position":"F","rookie":"0","team_code":"HFD","games_played":"72","goals":"31","assists":"24","points":"55","plus_minus":"-9","penalty_minutes":"121","power_play_goals":"7","short_handed_goals":"0","shots":"214"}}]}]}])"#;
    const GOALIE_REPORT: &str = r#"([{"sections":[{"data":[{"row":{"player_id":"8430","name":"Dylan Garand","active":"0","rookie":"0","team_code":"HFD","games_played":"36","minutes_played":"2013:22","saves":"821","shots":"916","save_percentage":"0.896","goals_against":"95","shutouts":"1","wins":"16","losses":"15","ot_losses":"2","goals_against_average":"2.83"}}]}]}])"#;
    const ROSTER_REPORT: &str = r#"({"roster":[{"sections":[{"title":"Forwards","data":[{"row":{"shoots":"L","birthplace":"Fort Collins, CO","height_hyphenated":"5-11","player_id":"10618","birthdate":"2002-02-18","tp_jersey_number":"6","position":"F","w":"180","name":"Aidan Thompson"}}]},{"title":"Team Personnel","data":[{"row":{"name":"Ryan Martin","role":"General Manager"}}]}]}]})"#;

    #[test]
    fn parses_both_statview_jsonp_shapes() {
        assert!(parse_jsonp("({\"teamsNoAll\":[]})").unwrap().is_object());
        assert!(parse_jsonp(SKATER_REPORT).unwrap().is_array());
        assert!(parse_jsonp("{\"no\":\"wrapper\"}").is_err());
    }

    #[test]
    fn ignores_documented_goalie_summary_rows_but_not_unknown_malformed_rows() {
        let summary = parse_jsonp(
            r#"([{"sections":[{"data":[{"row":{"name":"Empty Net "}},{"row":{"name":"Totals "}}]}]}])"#,
        )
        .unwrap();
        assert!(report_rows(&summary).unwrap().is_empty());
        let malformed =
            parse_jsonp(r#"([{"sections":[{"data":[{"row":{"name":"Mystery Player"}}]}]}])"#)
                .unwrap();
        assert!(report_rows(&malformed).is_err());
    }

    #[test]
    fn parses_provider_ids_without_claiming_nhl_identity() {
        let value = parse_jsonp(SKATER_REPORT).unwrap();
        let player = parse_skater(report_rows(&value).unwrap()[0], "HFD").unwrap();
        assert_eq!(player.provider, AHL_PROVIDER);
        assert_eq!(player.provider_player_id, "7669");
        assert_eq!(player.points, 55);
        assert_eq!(player.plus_minus, -9);
    }

    #[test]
    fn identity_name_normalization_handles_provider_punctuation_variants() {
        assert_eq!(
            normalize_ahl_identity_name("Ryan O’Rourke"),
            normalize_ahl_identity_name("Ryan O'Rourke")
        );
        assert_eq!(
            normalize_ahl_identity_name("Alex Kannok Leipert"),
            normalize_ahl_identity_name("Alex Kannok-Leipert")
        );
        assert_eq!(
            normalize_ahl_identity_name("C.J. Smith"),
            normalize_ahl_identity_name("CJ Smith")
        );
        assert_eq!(
            normalize_ahl_identity_name("Jack O'Brien"),
            normalize_ahl_identity_name("Jack OBrien")
        );

        let candidates = parse_official_nhl_search_candidates(
            "Ryan O’Rourke",
            "https://search.d3.nhle.com/player/ryan-o-rourke",
            br#"[{"playerId":"8482123","name":"Ryan O'Rourke"}]"#,
        )
        .unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].nhl_player_id, 8_482_123);
    }

    #[test]
    fn identity_search_variants_retain_source_and_add_straight_apostrophes() {
        assert_eq!(
            ahl_identity_search_name_variants("Ryan O’Rourke"),
            vec!["Ryan O’Rourke".to_owned(), "Ryan O'Rourke".to_owned()]
        );
        assert_eq!(
            ahl_identity_search_name_variants("Alex Kannok Leipert"),
            vec!["Alex Kannok Leipert".to_owned()]
        );
    }

    #[test]
    fn rejects_team_identity_mismatch() {
        let value = parse_jsonp(SKATER_REPORT).unwrap();
        let error = parse_skater(report_rows(&value).unwrap()[0], "CV").unwrap_err();
        assert!(error.to_string().contains("while fetching CV"));
    }

    #[test]
    fn team_report_filter_retains_typed_rows_and_audits_provider_contamination() {
        let value = serde_json::json!([{"sections": [{"data": [
            {"row": {"player_id": "1", "name": "Chicago Forward", "team_code": "CHI", "position": "F"}},
            {"row": {"player_id": "2", "name": "Chicago Goalie", "team_code": "CHI", "position": "G"}},
            {"row": {"player_id": "3", "name": "Loaned Goalie", "team_code": "SYR", "position": "G"}}
        ]}]}]);

        let (rows, warnings) = team_report_rows(&value, "CHI", "skater", true).unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(string_field(rows[0], "name").unwrap(), "Chicago Forward");
        assert_eq!(warnings.len(), 2);
        assert!(warnings[0].contains("Loaned Goalie #3 (SYR)"));
        assert!(warnings[1].contains("Chicago Goalie #2"));
    }

    #[test]
    fn team_report_filter_fails_when_every_player_belongs_to_another_team() {
        let value = serde_json::json!([{"sections": [{"data": [
            {"row": {"player_id": "3", "name": "Loaned Goalie", "team_code": "SYR"}}
        ]}]}]);

        let error = team_report_rows(&value, "CHI", "goalie", false).unwrap_err();

        assert!(error.to_string().contains("contained only other-team rows"));
        assert!(error.to_string().contains("Loaned Goalie #3 (SYR)"));
    }

    #[test]
    fn roster_filter_collapses_compatible_jersey_history_but_rejects_conflicts() {
        let player = AhlRosterPlayer {
            provider: AHL_PROVIDER.to_owned(),
            provider_player_id: "9657".to_owned(),
            name: "Chris Jandric".to_owned(),
            position_group: "Defenders".to_owned(),
            position: "D".to_owned(),
            jersey_number: "7".to_owned(),
            handedness: "L".to_owned(),
            height: "5-11".to_owned(),
            weight_pounds: "181".to_owned(),
            birthdate: "1998-10-03".to_owned(),
            birthplace: "Prince George, BC".to_owned(),
        };
        let mut renumbered = player.clone();
        renumbered.jersey_number = "37".to_owned();

        let (players, warnings) =
            deduplicate_roster_players(vec![player.clone(), renumbered], "LAV").unwrap();

        assert_eq!(players.len(), 1);
        assert!(players[0].jersey_number.is_empty());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("jersey numbers `7` and `37` were omitted"));

        let mut center = player.clone();
        center.provider_player_id = "6354".to_owned();
        center.name = "Danton Heinen".to_owned();
        center.position_group = "Forwards".to_owned();
        center.position = "C".to_owned();
        let mut wing = center.clone();
        wing.position = "LW".to_owned();
        let (players, warnings) = deduplicate_roster_players(vec![center, wing], "CLE").unwrap();
        assert_eq!(players[0].position, "F");
        assert!(warnings[0].contains("forward positions `C` and `LW` were generalized to `F`"));

        let mut conflicting = player.clone();
        conflicting.birthdate = "1998-10-04".to_owned();
        let error = deduplicate_roster_players(vec![player, conflicting], "LAV").unwrap_err();
        assert!(error
            .to_string()
            .contains("conflicting duplicate roster rows"));
    }

    #[test]
    fn season_labels_are_annual_and_validated() {
        assert_eq!(season_label(20262027).unwrap(), "2026-27 Regular Season");
        assert!(season_label(20262028).is_err());
    }

    #[tokio::test]
    async fn fetches_catalog_and_both_player_shapes() {
        let server = MockServer::start();
        let cache = tempfile::tempdir().unwrap();
        let seasons = server.mock(|when, then| {
            when.method(GET).query_param("view", "seasonsForLeague");
            then.status(200)
                .body("({\"seasons\":[{\"id\":\"94\",\"name\":\"2026-27 Regular Season\"}]})");
        });
        let teams = server.mock(|when, then| {
            when.method(GET).query_param("view", "teamsForSeason");
            then.status(200).body("({\"teamsNoAll\":[{\"id\":\"307\",\"name\":\"Hartford Wolf Pack\",\"nickname\":\"Wolf Pack\",\"team_code\":\"HFD\",\"division_id\":\"15\",\"logo\":\"https://example.test/hfd.png\"}]})");
        });
        let roster = server.mock(|when, then| {
            when.method(GET).query_param("view", "roster");
            then.status(200).body(ROSTER_REPORT);
        });
        let skaters = server.mock(|when, then| {
            when.method(GET).query_param("position", "skaters");
            then.status(200).body(SKATER_REPORT);
        });
        let goalies = server.mock(|when, then| {
            when.method(GET).query_param("position", "goalies");
            then.status(200).body(GOALIE_REPORT);
        });

        let snapshot =
            AhlFeedClient::with_base_url_and_cache(server.url("/feed"), cache.path().to_path_buf())
                .fetch_roster_stats(20262027, &["HFD".to_owned()])
                .await
                .unwrap();
        assert_eq!(snapshot.provider_season_id, "94");
        assert_eq!(snapshot.teams.len(), 1);
        assert_eq!(snapshot.teams[0].nhl_affiliate.as_deref(), Some("NYR"));
        assert_eq!(snapshot.teams[0].roster.len(), 1);
        assert_eq!(snapshot.teams[0].roster[0].provider_player_id, "10618");
        assert_eq!(snapshot.teams[0].skaters.len(), 1);
        assert_eq!(snapshot.teams[0].goalies.len(), 1);
        let enrichment = AhlProjectionPlayerEnrichment {
            provider_player_id: "10618".to_owned(),
            nhl_player_id: 8_480_001,
            primary_position: icelines_core::model::Position::Center,
            eligible_positions: vec![icelines_core::model::Position::Center],
            projected_score: 42.0,
            prospect: true,
            recall_readiness: Some(0.65),
            professional_games_at_season_start: Some(80),
            assigned_to_affiliate: true,
            waiver_required: false,
        };
        let input = affiliate_projection_input_from_snapshot(
            &snapshot,
            "NYR",
            "Hartford Wolf Pack",
            icelines_core::view_model::ahl_affiliate::AhlDevelopmentRuleInput::default(),
            &[enrichment],
        )
        .unwrap();
        assert_eq!(input.players[0].player_id, 8_480_001);
        assert_eq!(input.players[0].display_name, "Aidan Thompson");
        assert_eq!(
            input.pool_authority.kind,
            icelines_core::AhlRosterPoolAuthorityKind::OfficialSnapshot
        );
        assert!(affiliate_projection_input_from_snapshot(
            &snapshot,
            "NYR",
            "Hartford Wolf Pack",
            icelines_core::view_model::ahl_affiliate::AhlDevelopmentRuleInput::default(),
            &[],
        )
        .unwrap_err()
        .to_string()
        .contains("exactly cover"));
        let cache_manifest = crate::fletch::read_fletch_cache_manifest(
            &crate::fletch::fletch_cache_manifest_path(cache.path()),
        )
        .unwrap();
        assert_eq!(cache_manifest.entries.len(), 5);
        assert!(cache_manifest.entries.iter().all(|entry| entry.verified));
        seasons.assert();
        teams.assert();
        roster.assert();
        skaters.assert();
        goalies.assert();
    }

    fn identity_snapshot() -> AhlRosterStatsSnapshot {
        AhlRosterStatsSnapshot {
            schema: AHL_ROSTER_STATS_SCHEMA.to_owned(),
            season: 20262027,
            provider: AHL_PROVIDER.to_owned(),
            provider_season_id: "94".to_owned(),
            provider_season_name: "2026-27 Regular Season".to_owned(),
            fetched_at: "2026-07-24T12:00:00Z".to_owned(),
            source_url: AHL_STATS_SOURCE_URL.to_owned(),
            roster_source_url: AHL_ROSTER_SOURCE_URL.to_owned(),
            identity_note: "provider-local identity".to_owned(),
            teams: vec![AhlTeamRosterStats {
                provider: AHL_PROVIDER.to_owned(),
                provider_team_id: "307".to_owned(),
                team_code: "HFD".to_owned(),
                team_name: "Hartford Wolf Pack".to_owned(),
                nickname: "Wolf Pack".to_owned(),
                division_id: "15".to_owned(),
                logo_url: "https://example.test/hfd.png".to_owned(),
                nhl_affiliate: Some("NYR".to_owned()),
                roster: vec![AhlRosterPlayer {
                    provider: AHL_PROVIDER.to_owned(),
                    provider_player_id: "10618".to_owned(),
                    name: "Aidan Thompson".to_owned(),
                    position_group: "Forwards".to_owned(),
                    position: "F".to_owned(),
                    jersey_number: "6".to_owned(),
                    handedness: "L".to_owned(),
                    height: "5-11".to_owned(),
                    weight_pounds: "180".to_owned(),
                    birthdate: "2002-02-18".to_owned(),
                    birthplace: "Fort Collins, CO".to_owned(),
                }],
                skaters: Vec::new(),
                goalies: Vec::new(),
                source_warnings: Vec::new(),
            }],
        }
    }

    fn identity_catalog() -> AhlCanonicalIdentityCatalog {
        AhlCanonicalIdentityCatalog {
            schema: AHL_CANONICAL_IDENTITY_CATALOG_SCHEMA.to_owned(),
            checked_at: "2026-07-24".to_owned(),
            candidates: vec![AhlCanonicalIdentityCandidate {
                nhl_player_id: 8_480_001,
                display_name: "Aidan Thompson".to_owned(),
                birth_date: Some("2002-02-18".to_owned()),
                evidence_urls: vec!["https://www.nhl.com/player/8480001".to_owned()],
            }],
        }
    }

    #[test]
    fn exact_identity_match_remains_pending_review() {
        let view = build_ahl_identity_crosswalk(
            &identity_snapshot(),
            "Hartford Wolf Pack",
            &identity_catalog(),
        )
        .unwrap();
        assert_eq!(view.counts.exact_name_and_birth_date, 1);
        assert_eq!(view.counts.reviewed, 0);
        assert_eq!(view.rows[0].nhl_player_id, Some(8_480_001));
        assert_eq!(view.rows[0].review_status, AhlIdentityReviewStatus::Pending);
    }

    #[test]
    fn identity_inspection_owns_counts_and_attention_filtering() {
        let mut crosswalk = build_ahl_identity_crosswalk(
            &identity_snapshot(),
            "Hartford Wolf Pack",
            &identity_catalog(),
        )
        .unwrap();
        crosswalk.rows[0].ahl_display_name = "A. Thompson".to_owned();
        crosswalk.rows[0].match_basis = AhlIdentityMatchBasis::SurnameAndBirthDate;

        let all = build_ahl_identity_review_inspection(&crosswalk, AhlIdentityInspectionScope::All)
            .unwrap();
        assert_eq!(all.schema, AHL_IDENTITY_REVIEW_INSPECTION_SCHEMA);
        assert_eq!(all.total_rows, 1);
        assert_eq!(all.attention_count, 1);
        assert_eq!(all.rows.len(), 1);
        assert_eq!(all.computed_counts.surname_and_birth_date, 1);
        assert!(all.declared_counts_stale);

        let attention =
            build_ahl_identity_review_inspection(&crosswalk, AhlIdentityInspectionScope::Attention)
                .unwrap();
        assert_eq!(attention.scope, AhlIdentityInspectionScope::Attention);
        assert_eq!(attention.total_rows, 1);
        assert_eq!(attention.attention_count, 1);
        assert_eq!(attention.rows.len(), 1);
        assert_eq!(attention.rows[0].provider_player_id, "10618");

        crosswalk.rows[0].match_basis = AhlIdentityMatchBasis::ExactNameAndBirthDate;
        let routine =
            build_ahl_identity_review_inspection(&crosswalk, AhlIdentityInspectionScope::Attention)
                .unwrap();
        assert_eq!(routine.attention_count, 0);
        assert!(routine.rows.is_empty());
    }

    #[test]
    fn review_draft_cannot_apply_until_reviewer_finalizes_it() {
        let snapshot = identity_snapshot();
        let crosswalk =
            build_ahl_identity_crosswalk(&snapshot, "Hartford Wolf Pack", &identity_catalog())
                .unwrap();
        let mut review = build_ahl_identity_review_draft(&crosswalk).unwrap();
        assert!(review.draft);
        assert_eq!(review.decisions.len(), 1);
        assert!(apply_ahl_identity_review_decisions(&crosswalk, &review).is_err());

        review.draft = false;
        review.reviewer = Some("Test Reviewer".to_owned());
        review.reviewed_at = Some("2026-07-24T20:00:00-07:00".to_owned());
        let reviewed = apply_ahl_identity_review_decisions(&crosswalk, &review).unwrap();
        assert_eq!(
            reviewed.rows[0].review_status,
            AhlIdentityReviewStatus::Reviewed
        );
        assert_eq!(reviewed.counts.reviewed, 1);
        validate_reviewed_ahl_identity_crosswalk(&snapshot, "Hartford Wolf Pack", &reviewed)
            .unwrap();
    }

    #[test]
    fn exact_review_applies_only_verified_exact_rows() {
        let snapshot = identity_snapshot();
        let mut crosswalk =
            build_ahl_identity_crosswalk(&snapshot, "Hartford Wolf Pack", &identity_catalog())
                .unwrap();
        let mut alias = crosswalk.rows[0].clone();
        alias.provider_player_id = "alias-1".to_owned();
        alias.ahl_display_name = "A. Thompson".to_owned();
        alias.match_basis = AhlIdentityMatchBasis::SurnameAndBirthDate;
        crosswalk.rows.push(alias);
        crosswalk.counts.roster_players = 2;

        let review = build_ahl_exact_identity_review(
            &crosswalk,
            "Exact Evidence Pilot",
            "2026-07-25T12:00:00Z",
        )
        .unwrap();
        assert!(!review.draft);
        assert_eq!(review.decisions.len(), 1);
        assert_eq!(review.decisions[0].provider_player_id, "10618");
        let reviewed = apply_ahl_identity_review_decisions(&crosswalk, &review).unwrap();
        assert_eq!(
            reviewed.rows[0].review_status,
            AhlIdentityReviewStatus::Reviewed
        );
        assert_eq!(
            reviewed.rows[1].review_status,
            AhlIdentityReviewStatus::Pending
        );
    }

    #[test]
    fn alias_review_preserves_the_explicit_name_remap() {
        let mut crosswalk = build_ahl_identity_crosswalk(
            &identity_snapshot(),
            "Hartford Wolf Pack",
            &identity_catalog(),
        )
        .unwrap();
        crosswalk.rows[0].ahl_display_name = "A. Thompson".to_owned();
        crosswalk.rows[0].match_basis = AhlIdentityMatchBasis::SurnameAndBirthDate;

        let review = build_ahl_alias_identity_review(
            &crosswalk,
            "Alias Evidence Pilot",
            "2026-07-25T13:00:00Z",
        )
        .unwrap();
        assert_eq!(review.decisions.len(), 1);
        assert_eq!(
            review.decisions[0].action,
            AhlIdentityReviewAction::SetIdentity
        );
        assert_eq!(
            review.decisions[0].nhl_display_name.as_deref(),
            Some("Aidan Thompson")
        );
        let reviewed = apply_ahl_identity_review_decisions(&crosswalk, &review).unwrap();
        assert_eq!(
            reviewed.rows[0].review_status,
            AhlIdentityReviewStatus::Reviewed
        );
        assert_eq!(
            reviewed.rows[0].match_basis,
            AhlIdentityMatchBasis::ReviewedOverride
        );
        assert!(reviewed.rows[0].note.contains("A. Thompson"));
    }

    #[test]
    fn rejection_review_closes_only_selected_pending_identity_mapping() {
        let mut catalog = identity_catalog();
        catalog.candidates.clear();
        let crosswalk =
            build_ahl_identity_crosswalk(&identity_snapshot(), "Hartford Wolf Pack", &catalog)
                .unwrap();
        let review = build_ahl_identity_rejection_review(
            &crosswalk,
            &["10618".to_owned()],
            &["https://www.hartfordwolfpack.com/players/example".to_owned()],
            "Exception Reviewer",
            "2026-07-25T14:00:00Z",
            "official club evidence identifies an AHL-only player without a canonical NHL ID",
        )
        .unwrap();
        assert_eq!(review.decisions.len(), 1);
        assert_eq!(review.decisions[0].action, AhlIdentityReviewAction::Reject);
        assert!(review.decisions[0].note.contains("NHL identity mapping"));
        let reviewed = apply_ahl_identity_review_decisions(&crosswalk, &review).unwrap();
        assert_eq!(
            reviewed.rows[0].review_status,
            AhlIdentityReviewStatus::Rejected
        );
        assert_eq!(reviewed.rows[0].nhl_player_id, None);
        assert_eq!(reviewed.rows[0].evidence_urls.len(), 1);
    }

    #[test]
    fn league_review_deduplicates_recurring_attention_and_computes_coverage() {
        let mut first = build_ahl_identity_crosswalk(
            &identity_snapshot(),
            "Hartford Wolf Pack",
            &identity_catalog(),
        )
        .unwrap();
        first.season = 20252026;
        first.rows[0].match_basis = AhlIdentityMatchBasis::BirthDateConflict;
        first.rows[0].ahl_birth_date = "2002-02-17".to_owned();
        first.counts = identity_crosswalk_counts(&first.rows);

        let mut second = first.clone();
        second.season = 20262027;
        second.roster_fetched_at = "2026-07-25T15:00:00Z".to_owned();

        let mut exact = build_ahl_identity_crosswalk(
            &identity_snapshot(),
            "Hartford Wolf Pack",
            &identity_catalog(),
        )
        .unwrap();
        exact.ahl_team = "Bridgeport Islanders".to_owned();
        let exact_review =
            build_ahl_exact_identity_review(&exact, "League Reviewer", "2026-07-25T16:00:00Z")
                .unwrap();
        let reviewed = apply_ahl_identity_review_decisions(&exact, &exact_review).unwrap();

        let league = build_ahl_identity_league_review(&[second, reviewed, first]).unwrap();
        assert_eq!(league.crosswalks, 3);
        assert_eq!(league.roster_appearances, 3);
        assert_eq!(league.reviewed, 1);
        assert_eq!(league.pending, 2);
        assert_eq!(league.resolved_basis_points, 3_333);
        assert_eq!(league.canonical_identity_basis_points, 3_333);
        assert_eq!(league.attention_groups.len(), 1);
        assert_eq!(league.attention_groups[0].identity_key, "nhl:8480001");
        assert_eq!(league.attention_groups[0].occurrences, 2);
        assert_eq!(league.attention_groups[0].appearances[0].season, 20252026);
        assert_eq!(league.attention_groups[0].appearances[1].season, 20262027);
        assert_eq!(league.summaries[0].ahl_team, "Hartford Wolf Pack");
        assert_eq!(league.summaries[1].ahl_team, "Bridgeport Islanders");
    }

    #[test]
    fn exception_board_ranks_recurring_conflicts_and_retains_date_pairs() {
        let mut first = build_ahl_identity_crosswalk(
            &identity_snapshot(),
            "Hartford Wolf Pack",
            &identity_catalog(),
        )
        .unwrap();
        first.season = 20252026;
        first.rows[0].match_basis = AhlIdentityMatchBasis::BirthDateConflict;
        first.rows[0].ahl_birth_date = "2002-02-17".to_owned();
        first.rows[0]
            .evidence_urls
            .push("https://example.test/club/player-8480001".to_owned());
        first.counts = identity_crosswalk_counts(&first.rows);

        let mut second = first.clone();
        second.season = 20262027;
        second.roster_fetched_at = "2026-07-25T15:00:00Z".to_owned();

        let mut exact = build_ahl_identity_crosswalk(
            &identity_snapshot(),
            "Hartford Wolf Pack",
            &identity_catalog(),
        )
        .unwrap();
        exact.ahl_team = "Bridgeport Islanders".to_owned();
        exact.rows[0].nhl_player_id = Some(8_480_002);
        let mut collision = exact.clone();
        collision.ahl_team = "Springfield Thunderbirds".to_owned();
        collision.rows[0].nhl_player_id = Some(8_480_003);
        collision.rows[0].match_basis = AhlIdentityMatchBasis::BirthDateConflict;
        collision.rows[0].ahl_birth_date = "1990-01-01".to_owned();
        collision.counts = identity_crosswalk_counts(&collision.rows);
        let review = build_ahl_identity_league_review(&[collision, exact, second, first]).unwrap();
        let board = build_ahl_identity_exception_board(&review).unwrap();

        assert_eq!(board.schema, AHL_IDENTITY_EXCEPTION_BOARD_SCHEMA);
        assert_eq!(board.groups, 3);
        assert_eq!(board.appearances, 4);
        assert_eq!(board.rows[0].rank, 1);
        assert_eq!(board.rows[0].occurrences, 2);
        assert_eq!(
            board.rows[0].recommended_action,
            AhlIdentityExceptionAction::ResolveBirthDateConflict
        );
        assert_eq!(board.rows[0].conflict_date_pairs.len(), 1);
        assert_eq!(
            board.rows[0].conflict_date_pairs[0].ahl_birth_date,
            "2002-02-17"
        );
        assert_eq!(board.rows[0].conflict_date_pairs[0].appearances, 2);
        assert_eq!(board.rows[0].conflict_date_pairs[0].absolute_delta_days, 1);
        assert_eq!(
            board.rows[1].recommended_action,
            AhlIdentityExceptionAction::ApplyRoutineExact
        );
        let collision = board
            .rows
            .iter()
            .find(|row| row.nhl_player_id == Some(8_480_003))
            .unwrap();
        assert_eq!(
            collision.recommended_action,
            AhlIdentityExceptionAction::InvestigateIdentityCollision
        );
        assert!(collision.conflict_date_pairs[0].absolute_delta_days >= 1_460);

        let mut stale = review;
        stale.attention_groups[0].occurrences += 1;
        assert!(build_ahl_identity_exception_board(&stale)
            .unwrap_err()
            .to_string()
            .contains("stale occurrence"));
    }

    #[test]
    fn league_review_rejects_duplicate_team_season_bindings() {
        let crosswalk = build_ahl_identity_crosswalk(
            &identity_snapshot(),
            "Hartford Wolf Pack",
            &identity_catalog(),
        )
        .unwrap();
        let error = build_ahl_identity_league_review(&[crosswalk.clone(), crosswalk]).unwrap_err();
        assert!(error
            .to_string()
            .contains("duplicate league identity crosswalk"));
    }

    #[test]
    fn league_crosswalk_builds_every_snapshot_team_without_approving_rows() {
        let snapshot = identity_snapshot();
        let league = build_ahl_identity_league_crosswalk(&snapshot, &identity_catalog()).unwrap();
        assert_eq!(league.schema, AHL_IDENTITY_LEAGUE_CROSSWALK_SCHEMA);
        assert_eq!(league.teams, 1);
        assert_eq!(league.roster_appearances, 1);
        assert_eq!(league.unique_provider_players, 1);
        assert_eq!(league.crosswalks[0].ahl_team, "Hartford Wolf Pack");
        assert_eq!(
            league.crosswalks[0].rows[0].review_status,
            AhlIdentityReviewStatus::Pending
        );
    }

    #[test]
    fn league_exact_review_retains_child_batches_and_skips_empty_lanes() {
        let league =
            build_ahl_identity_league_crosswalk(&identity_snapshot(), &identity_catalog()).unwrap();
        let (reviewed, audit) = apply_ahl_identity_league_routine_review(
            &league,
            AhlIdentityLeagueRoutineReviewKind::Exact,
            "League Exact Reviewer",
            "2026-07-25T17:00:00Z",
        )
        .unwrap();
        assert_eq!(audit.schema, AHL_IDENTITY_LEAGUE_REVIEW_DECISIONS_SCHEMA);
        assert_eq!(audit.applied_decisions, 1);
        assert_eq!(audit.eligible_teams, 1);
        assert_eq!(audit.batches.len(), 1);
        assert_eq!(
            reviewed.crosswalks[0].rows[0].review_status,
            AhlIdentityReviewStatus::Reviewed
        );

        let (_, alias_audit) = apply_ahl_identity_league_routine_review(
            &reviewed,
            AhlIdentityLeagueRoutineReviewKind::Aliases,
            "League Alias Reviewer",
            "2026-07-25T18:00:00Z",
        )
        .unwrap();
        assert_eq!(alias_audit.applied_decisions, 0);
        assert_eq!(alias_audit.skipped_teams, ["Hartford Wolf Pack"]);
    }

    #[test]
    fn league_alias_review_preserves_explicit_remaps() {
        let mut league =
            build_ahl_identity_league_crosswalk(&identity_snapshot(), &identity_catalog()).unwrap();
        league.crosswalks[0].rows[0].ahl_display_name = "A. Thompson".to_owned();
        league.crosswalks[0].rows[0].match_basis = AhlIdentityMatchBasis::SurnameAndBirthDate;
        league.crosswalks[0].counts = identity_crosswalk_counts(&league.crosswalks[0].rows);
        let (reviewed, audit) = apply_ahl_identity_league_routine_review(
            &league,
            AhlIdentityLeagueRoutineReviewKind::Aliases,
            "League Alias Reviewer",
            "2026-07-25T18:00:00Z",
        )
        .unwrap();
        assert_eq!(audit.applied_decisions, 1);
        assert_eq!(
            reviewed.crosswalks[0].rows[0].match_basis,
            AhlIdentityMatchBasis::ReviewedOverride
        );
    }

    #[test]
    fn explicit_set_identity_supports_sourced_alias_override() {
        let mut catalog = identity_catalog();
        catalog.candidates.clear();
        let crosswalk =
            build_ahl_identity_crosswalk(&identity_snapshot(), "Hartford Wolf Pack", &catalog)
                .unwrap();
        let review = AhlIdentityReviewDecisions {
            schema: AHL_IDENTITY_REVIEW_DECISIONS_SCHEMA.to_owned(),
            season: crosswalk.season,
            provider: crosswalk.provider.clone(),
            ahl_team: crosswalk.ahl_team.clone(),
            roster_fetched_at: crosswalk.roster_fetched_at.clone(),
            draft: false,
            reviewer: Some("Test Reviewer".to_owned()),
            reviewed_at: Some("2026-07-24T20:00:00-07:00".to_owned()),
            decisions: vec![AhlIdentityReviewDecision {
                provider_player_id: "10618".to_owned(),
                action: AhlIdentityReviewAction::SetIdentity,
                nhl_player_id: Some(8_480_001),
                nhl_display_name: Some("A. Thompson".to_owned()),
                nhl_birth_date: Some("2002-02-18".to_owned()),
                evidence_urls: vec!["https://www.nhl.com/player/8480001".to_owned()],
                note: "Reviewed documented alias.".to_owned(),
            }],
        };
        let reviewed = apply_ahl_identity_review_decisions(&crosswalk, &review).unwrap();
        assert_eq!(
            reviewed.rows[0].match_basis,
            AhlIdentityMatchBasis::ReviewedOverride
        );
        assert_eq!(reviewed.rows[0].nhl_player_id, Some(8_480_001));
    }

    #[test]
    fn official_search_and_landing_build_birth_corroborated_proposal() {
        let search_url = "https://search.d3.nhle.com/api/v1/search/player?q=Aidan%20Thompson";
        let search = br#"[
            {"playerId":"8483451","name":"Aidan Thompson"},
            {"playerId":"8489999","name":"Aidan Smith"}
        ]"#;
        let candidates =
            parse_official_nhl_search_candidates("Aidan Thompson", search_url, search).unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].nhl_player_id, 8_483_451);
        assert_eq!(candidates[0].birth_date, None);

        let landing_url = "https://api-web.nhle.com/v1/player/8483451/landing";
        let landing = br#"{
            "playerId": 8483451,
            "firstName": {"default":"Aidan"},
            "lastName": {"default":"Thompson"},
            "birthDate": "2002-02-18"
        }"#;
        let enriched =
            enrich_official_nhl_landing_candidate(&candidates[0], landing_url, landing).unwrap();
        assert_eq!(enriched.birth_date.as_deref(), Some("2002-02-18"));
        assert_eq!(enriched.evidence_urls.len(), 2);
    }

    #[test]
    fn surname_search_builds_review_only_birth_corroborated_alias() {
        let search_url = "https://search.d3.nhle.com/api/v1/search/player?q=Thompson";
        let search = br#"[
            {"playerId":"8480001","name":"A. Thompson"},
            {"playerId":"8489999","name":"Different Player"}
        ]"#;
        let candidates =
            parse_official_nhl_search_candidates_by_surname("Aidan Thompson", search_url, search)
                .unwrap();
        assert_eq!(candidates.len(), 1);

        let catalog = AhlCanonicalIdentityCatalog {
            schema: AHL_CANONICAL_IDENTITY_CATALOG_SCHEMA.to_owned(),
            checked_at: "2026-07-25".to_owned(),
            candidates: vec![AhlCanonicalIdentityCandidate {
                birth_date: Some("2002-02-18".to_owned()),
                evidence_urls: vec![search_url.to_owned()],
                ..candidates[0].clone()
            }],
        };
        let crosswalk =
            build_ahl_identity_crosswalk(&identity_snapshot(), "Hartford Wolf Pack", &catalog)
                .unwrap();
        assert_eq!(
            crosswalk.rows[0].match_basis,
            AhlIdentityMatchBasis::SurnameAndBirthDate
        );
        assert_eq!(crosswalk.rows[0].nhl_player_id, Some(8_480_001));
        assert_eq!(crosswalk.counts.surname_and_birth_date, 1);
        assert!(build_ahl_identity_review_draft(&crosswalk)
            .unwrap()
            .decisions
            .is_empty());
        let alias_draft = build_ahl_identity_review_draft_with_aliases(&crosswalk, true).unwrap();
        assert!(alias_draft.draft);
        assert_eq!(alias_draft.decisions.len(), 1);
        assert_eq!(
            alias_draft.decisions[0].action,
            AhlIdentityReviewAction::SetIdentity
        );
        assert_eq!(alias_draft.decisions[0].nhl_player_id, Some(8_480_001));
        assert_eq!(alias_draft.decisions[0].evidence_urls.len(), 1);
        assert!(apply_ahl_identity_review_decisions(&crosswalk, &alias_draft).is_err());
    }

    #[test]
    fn catalog_merge_enriches_missing_birth_date_and_rejects_conflicts() {
        let mut search_catalog = identity_catalog();
        search_catalog.candidates[0].birth_date = None;
        let landing_catalog = identity_catalog();
        let merged = merge_ahl_canonical_identity_catalogs(
            "2026-07-24",
            &[search_catalog.clone(), landing_catalog],
        )
        .unwrap();
        assert_eq!(
            merged.candidates[0].birth_date.as_deref(),
            Some("2002-02-18")
        );

        let mut repeated_search_catalog = identity_catalog();
        let mut repeated_candidate = repeated_search_catalog.candidates[0].clone();
        repeated_candidate.evidence_urls =
            vec!["https://search.d3.nhle.com/player/a-thompson".to_owned()];
        repeated_search_catalog.candidates.push(repeated_candidate);
        let merged =
            merge_ahl_canonical_identity_catalogs("2026-07-24", &[repeated_search_catalog])
                .unwrap();
        assert_eq!(merged.candidates.len(), 1);
        assert_eq!(merged.candidates[0].evidence_urls.len(), 2);

        let mut conflicting = search_catalog;
        conflicting.candidates[0].display_name = "Different Player".to_owned();
        assert!(merge_ahl_canonical_identity_catalogs(
            "2026-07-24",
            &[identity_catalog(), conflicting]
        )
        .is_err());
    }

    #[test]
    fn reviewed_identity_and_separate_facts_build_projection_input() {
        let snapshot = identity_snapshot();
        let mut crosswalk =
            build_ahl_identity_crosswalk(&snapshot, "Hartford Wolf Pack", &identity_catalog())
                .unwrap();
        crosswalk.rows[0].review_status = AhlIdentityReviewStatus::Reviewed;
        crosswalk.counts = identity_crosswalk_counts(&crosswalk.rows);
        let facts = AhlProjectionPlayerFacts {
            provider_player_id: "10618".to_owned(),
            primary_position: icelines_core::model::Position::Center,
            eligible_positions: vec![icelines_core::model::Position::Center],
            projected_score: 42.0,
            prospect: true,
            recall_readiness: Some(0.65),
            professional_games_at_season_start: Some(80),
            assigned_to_affiliate: true,
            waiver_required: false,
        };
        let input = affiliate_projection_input_from_reviewed_crosswalk(
            &snapshot,
            "NYR",
            "Hartford Wolf Pack",
            icelines_core::view_model::ahl_affiliate::AhlDevelopmentRuleInput::default(),
            &crosswalk,
            &[facts],
        )
        .unwrap();
        assert_eq!(input.players[0].player_id, 8_480_001);
        assert_eq!(input.players[0].projected_score, 42.0);
    }

    #[test]
    fn pending_review_and_birth_date_conflicts_fail_closed() {
        let snapshot = identity_snapshot();
        let pending =
            build_ahl_identity_crosswalk(&snapshot, "Hartford Wolf Pack", &identity_catalog())
                .unwrap();
        assert!(validate_reviewed_ahl_identity_crosswalk(
            &snapshot,
            "Hartford Wolf Pack",
            &pending
        )
        .unwrap_err()
        .to_string()
        .contains("not reviewed"));

        let mut catalog = identity_catalog();
        catalog.candidates[0].birth_date = Some("2001-02-18".to_owned());
        let conflict =
            build_ahl_identity_crosswalk(&snapshot, "Hartford Wolf Pack", &catalog).unwrap();
        assert_eq!(
            conflict.rows[0].match_basis,
            AhlIdentityMatchBasis::BirthDateConflict
        );
        assert_eq!(conflict.rows[0].nhl_player_id, Some(8_480_001));
        assert_eq!(
            conflict.rows[0].nhl_birth_date.as_deref(),
            Some("2001-02-18")
        );

        let mut review = build_ahl_identity_review_draft(&conflict).unwrap();
        assert!(review.decisions.is_empty());
        let conflict_draft = build_ahl_identity_review_draft_with_options(
            &conflict,
            AhlIdentityReviewDraftOptions {
                include_conflicts: true,
                ..AhlIdentityReviewDraftOptions::default()
            },
        )
        .unwrap();
        assert!(conflict_draft.draft);
        assert_eq!(conflict_draft.decisions.len(), 1);
        assert_eq!(
            conflict_draft.decisions[0].action,
            AhlIdentityReviewAction::AcceptProposal
        );
        assert!(conflict_draft.decisions[0].note.contains("AHL 2002-02-18"));
        assert!(conflict_draft.decisions[0].note.contains("NHL 2001-02-18"));
        assert!(apply_ahl_identity_review_decisions(&conflict, &conflict_draft).is_err());
        review.draft = false;
        review.reviewer = Some("Conflict Reviewer".to_owned());
        review.reviewed_at = Some("2026-07-24T20:00:00-07:00".to_owned());
        review.decisions.push(AhlIdentityReviewDecision {
            provider_player_id: "10618".to_owned(),
            action: AhlIdentityReviewAction::AcceptProposal,
            nhl_player_id: None,
            nhl_display_name: None,
            nhl_birth_date: None,
            evidence_urls: Vec::new(),
            note: "Confirmed the NHL identity and retained both conflicting source dates."
                .to_owned(),
        });
        assert!(apply_ahl_identity_review_decisions(&conflict, &review)
            .unwrap_err()
            .to_string()
            .contains("set_identity"));
    }

    #[test]
    fn targeted_conflict_review_retains_both_dates_and_additional_evidence() {
        let snapshot = identity_snapshot();
        let mut catalog = identity_catalog();
        catalog.candidates[0].birth_date = Some("2001-02-18".to_owned());
        let conflict =
            build_ahl_identity_crosswalk(&snapshot, "Hartford Wolf Pack", &catalog).unwrap();
        let evidence = vec!["https://example.test/club/player-8480001".to_owned()];
        let decisions = build_ahl_identity_conflict_review(
            &conflict,
            &[8_480_001],
            &evidence,
            "Conflict Reviewer",
            "2026-07-26T20:00:00Z",
            "The NHL club transaction record controls the canonical NHL birth date.",
        )
        .unwrap();
        assert_eq!(decisions.decisions.len(), 1);
        let decision = &decisions.decisions[0];
        assert_eq!(decision.action, AhlIdentityReviewAction::SetIdentity);
        assert_eq!(decision.nhl_player_id, Some(8_480_001));
        assert!(decision.note.contains("AHL 2002-02-18"));
        assert!(decision.note.contains("NHL 2001-02-18"));
        assert!(decision.evidence_urls.contains(&evidence[0]));
        assert!(conflict.rows[0]
            .evidence_urls
            .iter()
            .all(|url| decision.evidence_urls.contains(url)));

        let reviewed = apply_ahl_identity_review_decisions(&conflict, &decisions).unwrap();
        assert_eq!(
            reviewed.rows[0].review_status,
            AhlIdentityReviewStatus::Reviewed
        );
        assert_eq!(
            reviewed.rows[0].match_basis,
            AhlIdentityMatchBasis::BirthDateConflict
        );
        validate_reviewed_ahl_identity_crosswalk(&snapshot, "Hartford Wolf Pack", &reviewed)
            .unwrap();
        assert!(build_ahl_identity_conflict_review(
            &conflict,
            &[8_499_999],
            &evidence,
            "Conflict Reviewer",
            "2026-07-26T20:00:00Z",
            "No matching proposal should fail.",
        )
        .is_err());
        assert!(build_ahl_identity_conflict_review(
            &conflict,
            &[8_480_001],
            &conflict.rows[0].evidence_urls,
            "Conflict Reviewer",
            "2026-07-26T20:00:00Z",
            "A conflict cannot be resolved without additional evidence.",
        )
        .unwrap_err()
        .to_string()
        .contains("evidence"));
    }

    #[test]
    fn league_conflict_review_is_targeted_and_atomic() {
        let snapshot = identity_snapshot();
        let mut catalog = identity_catalog();
        catalog.candidates[0].birth_date = Some("2001-02-18".to_owned());
        let league = build_ahl_identity_league_crosswalk(&snapshot, &catalog).unwrap();
        let evidence = vec!["https://example.test/club/player-8480001".to_owned()];
        let (reviewed, audit) = apply_ahl_identity_league_conflict_review(
            &league,
            &[8_480_001],
            &evidence,
            "League Conflict Reviewer",
            "2026-07-26T20:05:00Z",
            "The NHL club transaction record controls the canonical NHL birth date.",
        )
        .unwrap();
        assert_eq!(audit.kind, AhlIdentityLeagueRoutineReviewKind::Conflicts);
        assert_eq!(audit.eligible_teams, 1);
        assert_eq!(audit.applied_decisions, 1);
        assert_eq!(
            reviewed.crosswalks[0].rows[0].review_status,
            AhlIdentityReviewStatus::Reviewed
        );
        assert!(apply_ahl_identity_league_conflict_review(
            &league,
            &[8_499_999],
            &evidence,
            "League Conflict Reviewer",
            "2026-07-26T20:05:00Z",
            "No matching proposal should fail atomically.",
        )
        .is_err());
        assert_eq!(
            league.crosswalks[0].rows[0].review_status,
            AhlIdentityReviewStatus::Pending
        );
    }

    #[test]
    fn league_collision_remap_replaces_mapping_without_rejecting_ahl_player() {
        let snapshot = identity_snapshot();
        let mut catalog = identity_catalog();
        catalog.candidates[0].birth_date = Some("1991-02-18".to_owned());
        let league = build_ahl_identity_league_crosswalk(&snapshot, &catalog).unwrap();
        let evidence = vec!["https://api-web.nhle.com/v1/player/8489998/landing".to_owned()];
        let (reviewed, audit) = apply_ahl_identity_league_collision_remap(
            &league,
            8_480_001,
            8_489_998,
            "A. Thompson",
            "2002-02-18",
            &evidence,
            "Collision Reviewer",
            "2026-07-26T21:00:00Z",
            "Official landing evidence identifies the younger same-surname player.",
        )
        .unwrap();
        let row = &reviewed.crosswalks[0].rows[0];
        assert_eq!(
            audit.kind,
            AhlIdentityLeagueRoutineReviewKind::CollisionRemaps
        );
        assert_eq!(audit.applied_decisions, 1);
        assert_eq!(row.review_status, AhlIdentityReviewStatus::Reviewed);
        assert_eq!(row.nhl_player_id, Some(8_489_998));
        assert_eq!(row.nhl_birth_date.as_deref(), Some("2002-02-18"));
        assert!(row.note.contains("Replaced collided NHL proposal"));
        assert!(audit.disclosures[0].contains("never rejects"));

        assert!(apply_ahl_identity_league_collision_remap(
            &league,
            8_480_001,
            8_489_998,
            "Different Surname",
            "2002-02-18",
            &evidence,
            "Collision Reviewer",
            "2026-07-26T21:00:00Z",
            "A mismatched surname must fail atomically.",
        )
        .is_err());
        assert_eq!(
            league.crosswalks[0].rows[0].review_status,
            AhlIdentityReviewStatus::Pending
        );
    }

    #[test]
    fn league_review_draft_separates_conflict_proposals_from_unmatched_rows() {
        let snapshot = identity_snapshot();
        let mut catalog = identity_catalog();
        catalog.candidates[0].birth_date = Some("2001-02-18".to_owned());
        let league = build_ahl_identity_league_crosswalk(&snapshot, &catalog).unwrap();

        let draft = build_ahl_identity_league_review_draft(
            &league,
            AhlIdentityReviewDraftOptions {
                include_conflicts: true,
                ..AhlIdentityReviewDraftOptions::default()
            },
        )
        .unwrap();

        assert_eq!(draft.eligible_teams, 1);
        assert_eq!(draft.proposed_decisions, 1);
        assert_eq!(draft.pending_without_proposal, 0);
        assert!(draft.batches[0].draft);

        let mut unmatched = league;
        let row = &mut unmatched.crosswalks[0].rows[0];
        row.match_basis = AhlIdentityMatchBasis::Unmatched;
        row.nhl_player_id = None;
        row.nhl_display_name = None;
        row.nhl_birth_date = None;
        row.evidence_urls.clear();
        row.note = "No canonical NHL identity candidate.".to_owned();
        unmatched.crosswalks[0].counts = identity_crosswalk_counts(&unmatched.crosswalks[0].rows);
        let draft = build_ahl_identity_league_review_draft(
            &unmatched,
            AhlIdentityReviewDraftOptions {
                include_conflicts: true,
                ..AhlIdentityReviewDraftOptions::default()
            },
        )
        .unwrap();
        assert_eq!(draft.eligible_teams, 0);
        assert_eq!(draft.proposed_decisions, 0);
        assert_eq!(draft.pending_without_proposal, 1);
        assert_eq!(draft.skipped_teams, ["Hartford Wolf Pack"]);
    }

    #[test]
    fn empty_official_roster_can_be_audited_but_not_certified() {
        let mut snapshot = identity_snapshot();
        snapshot.teams[0].roster.clear();
        let crosswalk =
            build_ahl_identity_crosswalk(&snapshot, "Hartford Wolf Pack", &identity_catalog())
                .unwrap();
        assert_eq!(crosswalk.counts.roster_players, 0);
        let error =
            validate_reviewed_ahl_identity_crosswalk(&snapshot, "Hartford Wolf Pack", &crosswalk)
                .unwrap_err();
        assert!(error.to_string().contains("roster") && error.to_string().contains("empty"));
    }
}
