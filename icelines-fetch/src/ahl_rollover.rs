//! Preseason AHL roster-pool rollover planning.
//!
//! This module never emits an affiliate projection. It reconciles prior
//! official roster identities with a current NHL camp forecast and reports
//! whether a sourced 12F/6D/2G projected pool can be authored safely.

use std::collections::{BTreeMap, BTreeSet};

use icelines_core::{
    normalize_name, AhlAffiliationCatalogView, Position, TrainingCampForecastView,
    TrainingCampLeagueForecastView, TrainingCampSimulationInput, AHL_AFFILIATION_CATALOG_SCHEMA,
    TRAINING_CAMP_FORECAST_SCHEMA, TRAINING_CAMP_LEAGUE_FORECAST_SCHEMA,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::ahl::{
    AhlFeedError, AhlIdentityCrosswalkView, AhlIdentityLeagueCrosswalkView,
    AhlIdentityReviewStatus, AhlRosterPlayer, AhlRosterStatsSnapshot,
    AHL_IDENTITY_CROSSWALK_SCHEMA, AHL_IDENTITY_LEAGUE_CROSSWALK_SCHEMA,
};

pub const AHL_PRESEASON_ROLLOVER_SCHEMA: &str = "ahl_preseason_rollover.v1";
pub const AHL_PRESEASON_ORGANIZATION_REVIEW_SCHEMA: &str = "ahl_preseason_organization_review.v1";
pub const AHL_PRESEASON_LEAGUE_ROLLOVER_CONFIG_SCHEMA: &str =
    "ahl_preseason_league_rollover_config.v1";
pub const AHL_PRESEASON_LEAGUE_ROLLOVER_SCHEMA: &str = "ahl_preseason_league_rollover.v1";
pub const AHL_PRESEASON_LEAGUE_ORGANIZATION_REVIEW_SCHEMA: &str =
    "ahl_preseason_league_organization_review.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AhlPreseasonDecisionKind {
    Retained,
    Departed,
    OtherLeague,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlPreseasonOrganizationDecision {
    pub nhl_player_id: u32,
    pub kind: AhlPreseasonDecisionKind,
    pub evidence_urls: Vec<String>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlPreseasonRolloverConfig {
    pub target_season: u32,
    pub nhl_team: String,
    /// Target-season affiliate. This may differ from the prior snapshot club.
    pub ahl_team: String,
    /// Prior-snapshot affiliate when the organization changed or relocated.
    /// Absent preserves the original same-affiliate behavior.
    #[serde(default)]
    pub prior_ahl_team: Option<String>,
    pub as_of: String,
    pub source_urls: Vec<String>,
    #[serde(default)]
    pub prior_player_decisions: Vec<AhlPreseasonOrganizationDecision>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlPreseasonLeagueRolloverConfig {
    pub schema: String,
    pub prior_season: u32,
    pub target_season: u32,
    pub teams: Vec<AhlPreseasonRolloverConfig>,
}

/// Compose the team bindings that are fully established by dated affiliation,
/// reviewed identity, and sealed camp authorities. Prior-only player decisions
/// remain empty and therefore visible as review work in the resulting rollover.
pub fn build_ahl_preseason_league_rollover_config_draft(
    league_crosswalk: &AhlIdentityLeagueCrosswalkView,
    camp_league: &TrainingCampLeagueForecastView,
    prior_affiliations: &AhlAffiliationCatalogView,
    affiliations: &AhlAffiliationCatalogView,
    as_of: impl Into<String>,
    source_urls: Vec<String>,
) -> Result<AhlPreseasonLeagueRolloverConfig, AhlFeedError> {
    let as_of = as_of.into();
    if league_crosswalk.schema != AHL_IDENTITY_LEAGUE_CROSSWALK_SCHEMA
        || camp_league.schema != TRAINING_CAMP_LEAGUE_FORECAST_SCHEMA
        || prior_affiliations.schema != AHL_AFFILIATION_CATALOG_SCHEMA
        || affiliations.schema != AHL_AFFILIATION_CATALOG_SCHEMA
        || prior_affiliations.season != league_crosswalk.season
        || affiliations.season != camp_league.season
        || prior_affiliations.checked_at.trim().is_empty()
        || affiliations.checked_at.trim().is_empty()
        || !absolute_url(&prior_affiliations.source_url)
        || !absolute_url(&affiliations.source_url)
        || as_of.trim().is_empty()
        || source_urls.is_empty()
        || source_urls.iter().any(|url| !absolute_url(url))
    {
        return Err(AhlFeedError::Validation(
            "league rollover config draft requires dated identity, camp, and affiliation authorities"
                .to_owned(),
        ));
    }
    let mut prior_by_nhl = BTreeMap::new();
    for affiliation in &prior_affiliations.affiliations {
        let nhl_team = affiliation.nhl_team.as_str();
        if nhl_team.trim().is_empty()
            || affiliation.ahl_team.trim().is_empty()
            || prior_by_nhl
                .insert(nhl_team, affiliation.ahl_team.as_str())
                .is_some()
        {
            return Err(AhlFeedError::Validation(
                "prior affiliation catalog contains empty or duplicate NHL team bindings"
                    .to_owned(),
            ));
        }
    }
    let crosswalk_by_ahl = league_crosswalk
        .crosswalks
        .iter()
        .map(|crosswalk| (crosswalk.ahl_team.as_str(), crosswalk))
        .collect::<BTreeMap<_, _>>();
    if crosswalk_by_ahl.len() != league_crosswalk.crosswalks.len() {
        return Err(AhlFeedError::Validation(
            "reviewed identity envelope contains duplicate prior affiliates".to_owned(),
        ));
    }
    for (nhl_team, ahl_team) in &prior_by_nhl {
        let crosswalk = crosswalk_by_ahl.get(ahl_team).ok_or_else(|| {
            AhlFeedError::Validation(format!(
                "prior affiliation {nhl_team}/{ahl_team} has no reviewed identity crosswalk"
            ))
        })?;
        if crosswalk
            .nhl_affiliate
            .as_deref()
            .is_some_and(|declared| declared != *nhl_team)
        {
            return Err(AhlFeedError::Validation(format!(
                "prior affiliation catalog conflicts with the reviewed {} binding",
                crosswalk.ahl_team
            )));
        }
    }
    let mut target_by_nhl = BTreeMap::new();
    for affiliation in &affiliations.affiliations {
        if affiliation.nhl_team.trim().is_empty()
            || affiliation.ahl_team.trim().is_empty()
            || target_by_nhl
                .insert(affiliation.nhl_team.as_str(), affiliation.ahl_team.as_str())
                .is_some()
        {
            return Err(AhlFeedError::Validation(
                "affiliation catalog contains empty or duplicate NHL team bindings".to_owned(),
            ));
        }
    }
    let mut teams = Vec::with_capacity(camp_league.teams.len());
    let mut camp_teams = BTreeSet::new();
    for team in &camp_league.teams {
        if team.team.trim().is_empty() || !camp_teams.insert(team.team.as_str()) {
            return Err(AhlFeedError::Validation(
                "league camp contains empty or duplicate team bindings".to_owned(),
            ));
        }
        let prior = prior_by_nhl.get(team.team.as_str()).ok_or_else(|| {
            AhlFeedError::Validation(format!(
                "reviewed prior identities have no affiliate for {}",
                team.team
            ))
        })?;
        let target = target_by_nhl.get(team.team.as_str()).ok_or_else(|| {
            AhlFeedError::Validation(format!(
                "target affiliation catalog has no affiliate for {}",
                team.team
            ))
        })?;
        teams.push(AhlPreseasonRolloverConfig {
            target_season: camp_league.season,
            nhl_team: team.team.clone(),
            ahl_team: (*target).to_owned(),
            prior_ahl_team: Some((*prior).to_owned()),
            as_of: as_of.clone(),
            source_urls: source_urls.clone(),
            prior_player_decisions: Vec::new(),
        });
    }
    if camp_teams.len() != target_by_nhl.len() || camp_teams.len() != prior_by_nhl.len() {
        return Err(AhlFeedError::Validation(
            "camp, prior identity, and target affiliation team cohorts differ".to_owned(),
        ));
    }
    teams.sort_by(|left, right| left.nhl_team.cmp(&right.nhl_team));
    Ok(AhlPreseasonLeagueRolloverConfig {
        schema: AHL_PRESEASON_LEAGUE_ROLLOVER_CONFIG_SCHEMA.to_owned(),
        prior_season: league_crosswalk.season,
        target_season: camp_league.season,
        teams,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlPreseasonOrganizationReviewRow {
    pub provider_player_id: String,
    pub display_name: String,
    #[serde(default)]
    pub nhl_player_id: Option<u32>,
    pub identity_reviewed: bool,
    #[serde(default)]
    pub in_current_camp: Option<bool>,
    #[serde(default)]
    pub decision_kind: Option<AhlPreseasonDecisionKind>,
    #[serde(default)]
    pub evidence_urls: Vec<String>,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlPreseasonOrganizationReview {
    pub schema: String,
    pub prior_season: u32,
    pub target_season: u32,
    pub nhl_team: String,
    pub ahl_team: String,
    pub provider: String,
    pub roster_fetched_at: String,
    pub crosswalk_fingerprint: String,
    pub draft: bool,
    #[serde(default)]
    pub reviewer: Option<String>,
    #[serde(default)]
    pub reviewed_at: Option<String>,
    pub identity_blockers: usize,
    pub decisions_required: usize,
    pub rows: Vec<AhlPreseasonOrganizationReviewRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlPreseasonLeagueOrganizationReviewFailure {
    pub nhl_team: String,
    pub prior_ahl_team: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlPreseasonLeagueOrganizationReview {
    pub schema: String,
    pub prior_season: u32,
    pub target_season: u32,
    pub draft: bool,
    pub teams_requested: usize,
    pub teams_built: usize,
    pub identity_blockers: usize,
    pub decisions_required: usize,
    pub reviews: Vec<AhlPreseasonOrganizationReview>,
    pub failures: Vec<AhlPreseasonLeagueOrganizationReviewFailure>,
    pub disclosures: Vec<String>,
}

pub fn build_ahl_preseason_league_organization_review_draft(
    prior_snapshot: &AhlRosterStatsSnapshot,
    league_crosswalk: &AhlIdentityLeagueCrosswalkView,
    camp_league: &TrainingCampLeagueForecastView,
    config: &AhlPreseasonLeagueRolloverConfig,
) -> Result<AhlPreseasonLeagueOrganizationReview, AhlFeedError> {
    validate_league_rollover_inputs(prior_snapshot, league_crosswalk, camp_league, config)?;
    let crosswalks = league_crosswalk
        .crosswalks
        .iter()
        .map(|crosswalk| (crosswalk.ahl_team.as_str(), crosswalk))
        .collect::<BTreeMap<_, _>>();
    let forecasts = camp_league
        .teams
        .iter()
        .map(|team| (team.team.as_str(), team))
        .collect::<BTreeMap<_, _>>();
    let mut reviews = Vec::with_capacity(config.teams.len());
    let mut failures = Vec::new();
    for team_config in &config.teams {
        let prior_team = prior_ahl_team(team_config);
        let team_forecast = forecasts
            .get(team_config.nhl_team.as_str())
            .expect("validated league forecast team");
        let result = team_forecast
            .forecast
            .as_ref()
            .ok_or_else(|| {
                AhlFeedError::Validation(
                    team_forecast
                        .error
                        .clone()
                        .unwrap_or_else(|| "league camp forecast is unavailable".to_owned()),
                )
            })
            .and_then(|forecast| {
                let crosswalk = crosswalks.get(prior_team).copied().ok_or_else(|| {
                    AhlFeedError::Validation(format!(
                        "reviewed league crosswalk has no prior affiliate `{prior_team}`"
                    ))
                })?;
                build_ahl_preseason_organization_review_draft_from_forecast(
                    prior_snapshot,
                    crosswalk,
                    forecast,
                    &team_config.nhl_team,
                    prior_team,
                )
            });
        match result {
            Ok(review) => reviews.push(review),
            Err(error) => failures.push(AhlPreseasonLeagueOrganizationReviewFailure {
                nhl_team: team_config.nhl_team.clone(),
                prior_ahl_team: prior_team.to_owned(),
                reason: error.to_string(),
            }),
        }
    }
    reviews.sort_by(|left, right| left.nhl_team.cmp(&right.nhl_team));
    failures.sort_by(|left, right| left.nhl_team.cmp(&right.nhl_team));
    let identity_blockers = reviews.iter().map(|review| review.identity_blockers).sum();
    let decisions_required = reviews.iter().map(|review| review.decisions_required).sum();
    Ok(AhlPreseasonLeagueOrganizationReview {
        schema: AHL_PRESEASON_LEAGUE_ORGANIZATION_REVIEW_SCHEMA.to_owned(),
        prior_season: config.prior_season,
        target_season: config.target_season,
        draft: true,
        teams_requested: config.teams.len(),
        teams_built: reviews.len(),
        identity_blockers,
        decisions_required,
        reviews,
        failures,
        disclosures: vec![
            "This is a non-applicable league review draft; it creates no retained, departed, or other-league decisions.".to_owned(),
            "Each child remains bound to the prior official roster, reviewed identity fingerprint, sealed camp forecast, and explicit prior affiliate.".to_owned(),
            "Mapping-rejected identities remain blockers. Prior-only reviewed players require sourced organization-status decisions before rollover readiness can change.".to_owned(),
        ],
    })
}

pub fn build_ahl_preseason_organization_review_draft(
    prior_snapshot: &AhlRosterStatsSnapshot,
    crosswalk: &AhlIdentityCrosswalkView,
    camp_input: &TrainingCampSimulationInput,
    nhl_team: &str,
    ahl_team: &str,
) -> Result<AhlPreseasonOrganizationReview, AhlFeedError> {
    validate_organization_review_sources(
        prior_snapshot,
        crosswalk,
        nhl_team,
        ahl_team,
        &camp_input.team,
    )?;
    let camp_ids = camp_input
        .players
        .iter()
        .map(|row| row.player_id)
        .collect::<BTreeSet<_>>();
    build_ahl_preseason_organization_review_draft_from_ids(
        prior_snapshot,
        crosswalk,
        nhl_team,
        ahl_team,
        camp_input.season,
        &camp_ids,
    )
}

pub fn build_ahl_preseason_organization_review_draft_from_forecast(
    prior_snapshot: &AhlRosterStatsSnapshot,
    crosswalk: &AhlIdentityCrosswalkView,
    camp_forecast: &TrainingCampForecastView,
    nhl_team: &str,
    ahl_team: &str,
) -> Result<AhlPreseasonOrganizationReview, AhlFeedError> {
    if camp_forecast.schema != TRAINING_CAMP_FORECAST_SCHEMA {
        return Err(AhlFeedError::Validation(
            "organization review requires a sealed training camp forecast".to_owned(),
        ));
    }
    validate_organization_review_sources(
        prior_snapshot,
        crosswalk,
        nhl_team,
        ahl_team,
        &camp_forecast.team,
    )?;
    let mut camp_ids = BTreeSet::new();
    if camp_forecast
        .players
        .iter()
        .any(|row| row.player_id == 0 || !camp_ids.insert(row.player_id))
    {
        return Err(AhlFeedError::Validation(
            "organization review forecast contains zero or duplicate player IDs".to_owned(),
        ));
    }
    build_ahl_preseason_organization_review_draft_from_ids(
        prior_snapshot,
        crosswalk,
        nhl_team,
        ahl_team,
        camp_forecast.season,
        &camp_ids,
    )
}

fn build_ahl_preseason_organization_review_draft_from_ids(
    prior_snapshot: &AhlRosterStatsSnapshot,
    crosswalk: &AhlIdentityCrosswalkView,
    nhl_team: &str,
    ahl_team: &str,
    target_season: u32,
    camp_ids: &BTreeSet<u32>,
) -> Result<AhlPreseasonOrganizationReview, AhlFeedError> {
    let prior_team = prior_snapshot
        .teams
        .iter()
        .find(|team| team.team_name == ahl_team)
        .expect("validated prior team");
    let identities = crosswalk
        .rows
        .iter()
        .map(|row| (row.provider_player_id.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let rows = prior_team
        .roster
        .iter()
        .map(|player| {
            let identity = identities[player.provider_player_id.as_str()];
            let identity_reviewed = identity.review_status == AhlIdentityReviewStatus::Reviewed;
            let nhl_player_id = identity_reviewed
                .then_some(identity.nhl_player_id)
                .flatten();
            AhlPreseasonOrganizationReviewRow {
                provider_player_id: player.provider_player_id.clone(),
                display_name: player.name.clone(),
                nhl_player_id,
                identity_reviewed,
                in_current_camp: nhl_player_id.map(|id| camp_ids.contains(&id)),
                decision_kind: None,
                evidence_urls: Vec::new(),
                note: String::new(),
            }
        })
        .collect::<Vec<_>>();
    let identity_blockers = rows.iter().filter(|row| !row.identity_reviewed).count();
    let decisions_required = rows
        .iter()
        .filter(|row| row.identity_reviewed && row.in_current_camp == Some(false))
        .count();
    Ok(AhlPreseasonOrganizationReview {
        schema: AHL_PRESEASON_ORGANIZATION_REVIEW_SCHEMA.to_owned(),
        prior_season: prior_snapshot.season,
        target_season,
        nhl_team: nhl_team.to_owned(),
        ahl_team: ahl_team.to_owned(),
        provider: prior_snapshot.provider.clone(),
        roster_fetched_at: prior_snapshot.fetched_at.clone(),
        crosswalk_fingerprint: crosswalk_fingerprint(crosswalk)?,
        draft: true,
        reviewer: None,
        reviewed_at: None,
        identity_blockers,
        decisions_required,
        rows,
    })
}

pub fn apply_ahl_preseason_organization_review(
    prior_snapshot: &AhlRosterStatsSnapshot,
    crosswalk: &AhlIdentityCrosswalkView,
    camp_input: &TrainingCampSimulationInput,
    base_config: &AhlPreseasonRolloverConfig,
    review: &AhlPreseasonOrganizationReview,
) -> Result<AhlPreseasonRolloverConfig, AhlFeedError> {
    let expected = build_ahl_preseason_organization_review_draft(
        prior_snapshot,
        crosswalk,
        camp_input,
        &base_config.nhl_team,
        prior_ahl_team(base_config),
    )?;
    apply_ahl_preseason_organization_review_against_expected(
        base_config,
        review,
        &expected,
        camp_input.season,
    )
}

pub fn apply_ahl_preseason_organization_review_from_forecast(
    prior_snapshot: &AhlRosterStatsSnapshot,
    crosswalk: &AhlIdentityCrosswalkView,
    camp_forecast: &TrainingCampForecastView,
    base_config: &AhlPreseasonRolloverConfig,
    review: &AhlPreseasonOrganizationReview,
) -> Result<AhlPreseasonRolloverConfig, AhlFeedError> {
    let expected = build_ahl_preseason_organization_review_draft_from_forecast(
        prior_snapshot,
        crosswalk,
        camp_forecast,
        &base_config.nhl_team,
        prior_ahl_team(base_config),
    )?;
    apply_ahl_preseason_organization_review_against_expected(
        base_config,
        review,
        &expected,
        camp_forecast.season,
    )
}

pub fn apply_ahl_preseason_league_organization_review(
    prior_snapshot: &AhlRosterStatsSnapshot,
    league_crosswalk: &AhlIdentityLeagueCrosswalkView,
    camp_league: &TrainingCampLeagueForecastView,
    base_config: &AhlPreseasonLeagueRolloverConfig,
    review: &AhlPreseasonLeagueOrganizationReview,
) -> Result<AhlPreseasonLeagueRolloverConfig, AhlFeedError> {
    validate_league_rollover_inputs(prior_snapshot, league_crosswalk, camp_league, base_config)?;
    let recomputed_identity_blockers = review
        .reviews
        .iter()
        .map(|child| child.identity_blockers)
        .sum::<usize>();
    let recomputed_decisions_required = review
        .reviews
        .iter()
        .map(|child| child.decisions_required)
        .sum::<usize>();
    if review.schema != AHL_PRESEASON_LEAGUE_ORGANIZATION_REVIEW_SCHEMA
        || review.prior_season != base_config.prior_season
        || review.target_season != base_config.target_season
        || review.draft
        || review.teams_requested != base_config.teams.len()
        || review.teams_built != base_config.teams.len()
        || review.reviews.len() != base_config.teams.len()
        || !review.failures.is_empty()
        || review.identity_blockers != recomputed_identity_blockers
        || review.decisions_required != recomputed_decisions_required
    {
        return Err(AhlFeedError::Validation(
            "league organization review is draft, stale, incomplete, or has failed teams"
                .to_owned(),
        ));
    }
    let crosswalks = league_crosswalk
        .crosswalks
        .iter()
        .map(|crosswalk| (crosswalk.ahl_team.as_str(), crosswalk))
        .collect::<BTreeMap<_, _>>();
    let forecasts = camp_league
        .teams
        .iter()
        .map(|team| (team.team.as_str(), team.forecast.as_ref()))
        .collect::<BTreeMap<_, _>>();
    let mut reviews = BTreeMap::new();
    for child in &review.reviews {
        if reviews.insert(child.nhl_team.as_str(), child).is_some() {
            return Err(AhlFeedError::Validation(format!(
                "league organization review duplicates team {}",
                child.nhl_team
            )));
        }
    }
    let mut teams = Vec::with_capacity(base_config.teams.len());
    for team_config in &base_config.teams {
        let prior_team = prior_ahl_team(team_config);
        let crosswalk = crosswalks.get(prior_team).copied().ok_or_else(|| {
            AhlFeedError::Validation(format!(
                "league organization review has no crosswalk for {prior_team}"
            ))
        })?;
        let forecast = forecasts
            .get(team_config.nhl_team.as_str())
            .copied()
            .flatten()
            .ok_or_else(|| {
                AhlFeedError::Validation(format!(
                    "league organization review has no forecast for {}",
                    team_config.nhl_team
                ))
            })?;
        let child = reviews
            .get(team_config.nhl_team.as_str())
            .copied()
            .ok_or_else(|| {
                AhlFeedError::Validation(format!(
                    "league organization review has no child for {}",
                    team_config.nhl_team
                ))
            })?;
        teams.push(apply_ahl_preseason_organization_review_from_forecast(
            prior_snapshot,
            crosswalk,
            forecast,
            team_config,
            child,
        )?);
    }
    teams.sort_by(|left, right| left.nhl_team.cmp(&right.nhl_team));
    Ok(AhlPreseasonLeagueRolloverConfig {
        schema: AHL_PRESEASON_LEAGUE_ROLLOVER_CONFIG_SCHEMA.to_owned(),
        prior_season: base_config.prior_season,
        target_season: base_config.target_season,
        teams,
    })
}

fn apply_ahl_preseason_organization_review_against_expected(
    base_config: &AhlPreseasonRolloverConfig,
    review: &AhlPreseasonOrganizationReview,
    expected: &AhlPreseasonOrganizationReview,
    target_season: u32,
) -> Result<AhlPreseasonRolloverConfig, AhlFeedError> {
    if review.schema != AHL_PRESEASON_ORGANIZATION_REVIEW_SCHEMA
        || review.prior_season != expected.prior_season
        || review.target_season != expected.target_season
        || review.nhl_team != expected.nhl_team
        || review.ahl_team != expected.ahl_team
        || review.provider != expected.provider
        || review.roster_fetched_at != expected.roster_fetched_at
        || review.crosswalk_fingerprint != expected.crosswalk_fingerprint
        || review.draft
        || review.identity_blockers != expected.identity_blockers
        || review.decisions_required != expected.decisions_required
        || review
            .reviewer
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        || review
            .reviewed_at
            .as_deref()
            .is_none_or(|value| chrono::DateTime::parse_from_rfc3339(value).is_err())
        || review.rows.len() != expected.rows.len()
        || base_config.target_season != target_season
        || base_config.as_of.trim().is_empty()
        || base_config.source_urls.is_empty()
        || base_config.source_urls.iter().any(|url| !absolute_url(url))
    {
        return Err(AhlFeedError::Validation(
            "organization review is draft, stale, incomplete, or missing reviewer/config authority"
                .to_owned(),
        ));
    }
    let expected_rows = expected
        .rows
        .iter()
        .map(|row| (row.provider_player_id.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let mut provider_ids = BTreeSet::new();
    let mut decisions = Vec::new();
    let reviewer = review
        .reviewer
        .as_deref()
        .expect("validated reviewer")
        .trim();
    let reviewed_at = review.reviewed_at.as_deref().expect("validated timestamp");
    for row in &review.rows {
        let expected = expected_rows
            .get(row.provider_player_id.as_str())
            .ok_or_else(|| {
                AhlFeedError::Validation(format!(
                    "organization review contains unknown provider player {}",
                    row.provider_player_id
                ))
            })?;
        if !provider_ids.insert(row.provider_player_id.as_str())
            || row.display_name != expected.display_name
            || row.nhl_player_id != expected.nhl_player_id
            || row.identity_reviewed != expected.identity_reviewed
            || row.in_current_camp != expected.in_current_camp
        {
            return Err(AhlFeedError::Validation(format!(
                "organization review altered or duplicated player {}",
                row.provider_player_id
            )));
        }
        let requires_decision = row.identity_reviewed && row.in_current_camp == Some(false);
        if requires_decision {
            let kind = row.decision_kind.ok_or_else(|| {
                AhlFeedError::Validation(format!(
                    "organization review has no decision for prior-only player {}",
                    row.provider_player_id
                ))
            })?;
            if row.note.trim().is_empty()
                || row.evidence_urls.is_empty()
                || row.evidence_urls.iter().any(|url| !absolute_url(url))
            {
                return Err(AhlFeedError::Validation(format!(
                    "organization review decision {} lacks sourced evidence",
                    row.provider_player_id
                )));
            }
            decisions.push(AhlPreseasonOrganizationDecision {
                nhl_player_id: row.nhl_player_id.expect("reviewed identity has NHL id"),
                kind,
                evidence_urls: row.evidence_urls.clone(),
                note: format!(
                    "Reviewed by {reviewer} at {reviewed_at}: {}",
                    row.note.trim()
                ),
            });
        } else if row.decision_kind.is_some()
            || !row.evidence_urls.is_empty()
            || !row.note.trim().is_empty()
        {
            return Err(AhlFeedError::Validation(format!(
                "organization review supplies an unnecessary decision for {}",
                row.provider_player_id
            )));
        }
    }
    if provider_ids.len() != expected_rows.len() || decisions.len() != expected.decisions_required {
        return Err(AhlFeedError::Validation(
            "organization review does not exactly cover the required prior-player decisions"
                .to_owned(),
        ));
    }
    decisions.sort_by_key(|row| row.nhl_player_id);
    let mut config = base_config.clone();
    config.prior_player_decisions = decisions;
    Ok(config)
}

fn crosswalk_fingerprint(crosswalk: &AhlIdentityCrosswalkView) -> Result<String, AhlFeedError> {
    let bytes = serde_json::to_vec(crosswalk).map_err(|error| {
        AhlFeedError::Validation(format!("cannot fingerprint identity crosswalk: {error}"))
    })?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn validate_organization_review_sources(
    prior_snapshot: &AhlRosterStatsSnapshot,
    crosswalk: &AhlIdentityCrosswalkView,
    nhl_team: &str,
    ahl_team: &str,
    camp_team: &str,
) -> Result<(), AhlFeedError> {
    prior_snapshot.validate()?;
    let team = prior_snapshot
        .teams
        .iter()
        .find(|team| team.team_name == ahl_team)
        .ok_or_else(|| {
            AhlFeedError::Validation(format!("prior AHL snapshot has no team named `{ahl_team}`"))
        })?;
    if nhl_team != camp_team
        || crosswalk.schema != AHL_IDENTITY_CROSSWALK_SCHEMA
        || crosswalk.season != prior_snapshot.season
        || crosswalk.provider != prior_snapshot.provider
        || crosswalk.ahl_team != ahl_team
        || crosswalk.roster_fetched_at != prior_snapshot.fetched_at
        || team
            .nhl_affiliate
            .as_deref()
            .is_some_and(|affiliate| affiliate != nhl_team)
    {
        return Err(AhlFeedError::Validation(
            "organization review sources do not bind the same affiliate/camp authority".to_owned(),
        ));
    }
    let official = team
        .roster
        .iter()
        .map(|row| (row.provider_player_id.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let mut provider_ids = BTreeSet::new();
    let mut reviewed_ids = BTreeSet::new();
    for row in &crosswalk.rows {
        let player = official
            .get(row.provider_player_id.as_str())
            .ok_or_else(|| {
                AhlFeedError::Validation(format!(
                    "organization review crosswalk contains extra provider player {}",
                    row.provider_player_id
                ))
            })?;
        if !provider_ids.insert(row.provider_player_id.as_str())
            || row.ahl_display_name != player.name
            || row.ahl_birth_date != player.birthdate
            || (row.review_status == AhlIdentityReviewStatus::Reviewed
                && row
                    .nhl_player_id
                    .filter(|id| *id != 0)
                    .is_none_or(|id| !reviewed_ids.insert(id)))
        {
            return Err(AhlFeedError::Validation(format!(
                "organization review crosswalk altered or duplicated identity {}",
                row.provider_player_id
            )));
        }
    }
    if provider_ids.len() != official.len() {
        return Err(AhlFeedError::Validation(
            "organization review crosswalk must exactly cover the prior roster".to_owned(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AhlPreseasonPositionGroup {
    Forward,
    Defense,
    Goalie,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AhlPreseasonRolloverOrigin {
    PriorAffiliate,
    CurrentCamp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlPreseasonRolloverPlayerView {
    pub nhl_player_id: Option<u32>,
    pub prior_provider_player_id: Option<String>,
    pub display_name: String,
    pub position_group: AhlPreseasonPositionGroup,
    pub origins: Vec<AhlPreseasonRolloverOrigin>,
    pub identity_reviewed: bool,
    pub organization_decision: Option<AhlPreseasonDecisionKind>,
    pub camp_make_probability: Option<f64>,
    pub camp_cut_probability: Option<f64>,
    pub modal_nhl_roster: bool,
    pub waiver_exempt: Option<bool>,
    pub projected_score: Option<f64>,
    pub projectable_affiliate_candidate: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlPreseasonRolloverCountsView {
    pub prior_roster_players: usize,
    pub current_camp_players: usize,
    pub reconciled_players: usize,
    pub unresolved_prior_identities: usize,
    pub prior_players_needing_organization_review: usize,
    pub waiver_gated_candidates: usize,
    pub projectable_forwards: usize,
    pub projectable_defensemen: usize,
    pub projectable_goalies: usize,
    pub forwards_needed: usize,
    pub defensemen_needed: usize,
    pub goalies_needed: usize,
    pub projection_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlPreseasonRolloverView {
    pub schema: String,
    pub nhl_team: String,
    pub ahl_team: String,
    pub prior_season: u32,
    pub target_season: u32,
    pub as_of: String,
    pub source_urls: Vec<String>,
    pub counts: AhlPreseasonRolloverCountsView,
    pub players: Vec<AhlPreseasonRolloverPlayerView>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlPreseasonLeagueRolloverFailureView {
    pub nhl_team: String,
    pub prior_ahl_team: String,
    pub ahl_team: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlPreseasonLeagueRolloverView {
    pub schema: String,
    pub prior_season: u32,
    pub target_season: u32,
    pub teams_requested: usize,
    pub teams_built: usize,
    pub teams_projection_ready: usize,
    pub rollovers: Vec<AhlPreseasonRolloverView>,
    pub failures: Vec<AhlPreseasonLeagueRolloverFailureView>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone)]
struct CampRolloverCandidate {
    player_id: u32,
    display_name: String,
    primary_position: Position,
    waiver_exempt: bool,
    projected_score: f64,
    make_probability: f64,
    cut_probability: f64,
}

pub fn build_ahl_preseason_rollover(
    prior_snapshot: &AhlRosterStatsSnapshot,
    crosswalk: &AhlIdentityCrosswalkView,
    camp_input: &TrainingCampSimulationInput,
    camp_forecast: &TrainingCampForecastView,
    config: &AhlPreseasonRolloverConfig,
) -> Result<AhlPreseasonRolloverView, AhlFeedError> {
    validate_inputs(prior_snapshot, crosswalk, camp_input, camp_forecast, config)?;
    let forecast_by_id = camp_forecast
        .players
        .iter()
        .map(|player| (player.player_id, player))
        .collect::<BTreeMap<_, _>>();
    let candidates = camp_input
        .players
        .iter()
        .map(|player| {
            let forecast = forecast_by_id
                .get(&player.player_id)
                .expect("validated camp forecast covers input player");
            CampRolloverCandidate {
                player_id: player.player_id,
                display_name: player.display_name.clone(),
                primary_position: player.primary_position,
                waiver_exempt: player.waiver_exempt,
                projected_score: player.projected_score,
                make_probability: forecast.make_probability,
                cut_probability: forecast.cut_probability,
            }
        })
        .collect::<Vec<_>>();
    build_ahl_preseason_rollover_from_candidates(
        prior_snapshot,
        crosswalk,
        &candidates,
        &camp_forecast.modal_opening_roster_ids,
        config,
    )
}

/// Build the same rollover plan directly from a sealed team forecast. League
/// camp forecasts intentionally retain every field used by rollover, so callers
/// do not need to preserve or reconstruct the simulation input artifact.
pub fn build_ahl_preseason_rollover_from_forecast(
    prior_snapshot: &AhlRosterStatsSnapshot,
    crosswalk: &AhlIdentityCrosswalkView,
    camp_forecast: &TrainingCampForecastView,
    config: &AhlPreseasonRolloverConfig,
) -> Result<AhlPreseasonRolloverView, AhlFeedError> {
    validate_forecast_inputs(prior_snapshot, crosswalk, camp_forecast, config)?;
    let candidates = camp_forecast
        .players
        .iter()
        .map(|player| CampRolloverCandidate {
            player_id: player.player_id,
            display_name: player.display_name.clone(),
            primary_position: player.primary_position,
            waiver_exempt: player.waiver_exempt,
            projected_score: player.projected_score,
            make_probability: player.make_probability,
            cut_probability: player.cut_probability,
        })
        .collect::<Vec<_>>();
    build_ahl_preseason_rollover_from_candidates(
        prior_snapshot,
        crosswalk,
        &candidates,
        &camp_forecast.modal_opening_roster_ids,
        config,
    )
}

/// Apply forecast-native rollover to every team in a sealed league camp
/// artifact. Team-specific source defects remain typed failures in the output;
/// malformed or incomplete league/config bindings fail the whole operation.
pub fn build_ahl_preseason_league_rollover(
    prior_snapshot: &AhlRosterStatsSnapshot,
    league_crosswalk: &AhlIdentityLeagueCrosswalkView,
    camp_league: &TrainingCampLeagueForecastView,
    config: &AhlPreseasonLeagueRolloverConfig,
) -> Result<AhlPreseasonLeagueRolloverView, AhlFeedError> {
    validate_league_rollover_inputs(prior_snapshot, league_crosswalk, camp_league, config)?;
    let crosswalks = league_crosswalk
        .crosswalks
        .iter()
        .map(|crosswalk| (crosswalk.ahl_team.as_str(), crosswalk))
        .collect::<BTreeMap<_, _>>();
    let forecasts = camp_league
        .teams
        .iter()
        .map(|team| (team.team.as_str(), team))
        .collect::<BTreeMap<_, _>>();
    let mut rollovers = Vec::with_capacity(config.teams.len());
    let mut failures = Vec::new();
    for team_config in &config.teams {
        let prior_team = prior_ahl_team(team_config);
        let team_forecast = forecasts
            .get(team_config.nhl_team.as_str())
            .expect("validated league forecast team");
        let result = team_forecast
            .forecast
            .as_ref()
            .ok_or_else(|| {
                AhlFeedError::Validation(
                    team_forecast
                        .error
                        .clone()
                        .unwrap_or_else(|| "league camp forecast is unavailable".to_owned()),
                )
            })
            .and_then(|forecast| {
                let crosswalk = crosswalks.get(prior_team).copied().ok_or_else(|| {
                    AhlFeedError::Validation(format!(
                        "reviewed league crosswalk has no prior affiliate `{prior_team}`"
                    ))
                })?;
                build_ahl_preseason_rollover_from_forecast(
                    prior_snapshot,
                    crosswalk,
                    forecast,
                    team_config,
                )
            });
        match result {
            Ok(view) => rollovers.push(view),
            Err(error) => failures.push(AhlPreseasonLeagueRolloverFailureView {
                nhl_team: team_config.nhl_team.clone(),
                prior_ahl_team: prior_team.to_owned(),
                ahl_team: team_config.ahl_team.clone(),
                reason: error.to_string(),
            }),
        }
    }
    rollovers.sort_by(|left, right| left.nhl_team.cmp(&right.nhl_team));
    failures.sort_by(|left, right| left.nhl_team.cmp(&right.nhl_team));
    let teams_projection_ready = rollovers
        .iter()
        .filter(|view| view.counts.projection_ready)
        .count();
    Ok(AhlPreseasonLeagueRolloverView {
        schema: AHL_PRESEASON_LEAGUE_ROLLOVER_SCHEMA.to_owned(),
        prior_season: config.prior_season,
        target_season: config.target_season,
        teams_requested: config.teams.len(),
        teams_built: rollovers.len(),
        teams_projection_ready,
        rollovers,
        failures,
        disclosures: vec![
            "League rollover composes sealed team camp forecasts with reviewed prior-affiliate identities; it does not infer final AHL assignment.".to_owned(),
            "Every camp team requires one explicit target/prior affiliation config. Missing forecasts, crosswalks, or source bindings remain typed team failures.".to_owned(),
            "Projection-ready means candidate-pool shape only; assignment authority, professional-game totals, and development-rule compliance remain downstream gates.".to_owned(),
        ],
    })
}

fn validate_league_rollover_inputs(
    prior_snapshot: &AhlRosterStatsSnapshot,
    league_crosswalk: &AhlIdentityLeagueCrosswalkView,
    camp_league: &TrainingCampLeagueForecastView,
    config: &AhlPreseasonLeagueRolloverConfig,
) -> Result<(), AhlFeedError> {
    prior_snapshot.validate()?;
    if config.schema != AHL_PRESEASON_LEAGUE_ROLLOVER_CONFIG_SCHEMA
        || config.prior_season != prior_snapshot.season
        || config.target_season != camp_league.season
        || camp_league.schema != TRAINING_CAMP_LEAGUE_FORECAST_SCHEMA
        || league_crosswalk.schema != AHL_IDENTITY_LEAGUE_CROSSWALK_SCHEMA
        || league_crosswalk.season != prior_snapshot.season
        || league_crosswalk.provider != prior_snapshot.provider
        || league_crosswalk.roster_fetched_at != prior_snapshot.fetched_at
        || config.teams.is_empty()
    {
        return Err(AhlFeedError::Validation(
            "league rollover has mismatched snapshot, identity, camp, or config authority"
                .to_owned(),
        ));
    }
    let mut config_teams = BTreeSet::new();
    let mut forecast_teams = BTreeSet::new();
    let mut prior_teams = BTreeSet::new();
    if config.teams.iter().any(|team| {
        team.target_season != config.target_season
            || team.nhl_team.trim().is_empty()
            || team.ahl_team.trim().is_empty()
            || prior_ahl_team(team).trim().is_empty()
            || !config_teams.insert(team.nhl_team.as_str())
    }) || camp_league
        .teams
        .iter()
        .any(|team| team.team.trim().is_empty() || !forecast_teams.insert(team.team.as_str()))
        || league_crosswalk.crosswalks.iter().any(|crosswalk| {
            crosswalk.ahl_team.trim().is_empty() || !prior_teams.insert(crosswalk.ahl_team.as_str())
        })
    {
        return Err(AhlFeedError::Validation(
            "league rollover contains empty or duplicate team bindings".to_owned(),
        ));
    }
    if config_teams != forecast_teams
        || camp_league.teams_requested != camp_league.teams.len()
        || league_crosswalk.teams != league_crosswalk.crosswalks.len()
    {
        return Err(AhlFeedError::Validation(
            "league rollover config must exactly cover the sealed camp team cohort".to_owned(),
        ));
    }
    Ok(())
}

fn build_ahl_preseason_rollover_from_candidates(
    prior_snapshot: &AhlRosterStatsSnapshot,
    crosswalk: &AhlIdentityCrosswalkView,
    camp_candidates: &[CampRolloverCandidate],
    modal_opening_roster_ids: &[u32],
    config: &AhlPreseasonRolloverConfig,
) -> Result<AhlPreseasonRolloverView, AhlFeedError> {
    let prior_team = prior_snapshot
        .teams
        .iter()
        .find(|team| team.team_name == prior_ahl_team(config))
        .expect("validation found prior affiliate");
    let crosswalk_by_provider = crosswalk
        .rows
        .iter()
        .map(|row| (row.provider_player_id.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let camp_by_id = camp_candidates
        .iter()
        .map(|player| (player.player_id, player))
        .collect::<BTreeMap<_, _>>();
    let modal_ids = modal_opening_roster_ids
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let decisions = config
        .prior_player_decisions
        .iter()
        .map(|row| (row.nhl_player_id, row))
        .collect::<BTreeMap<_, _>>();

    let mut reconciled_camp_ids = BTreeSet::new();
    let mut players = Vec::new();
    for prior in &prior_team.roster {
        let identity = crosswalk_by_provider
            .get(prior.provider_player_id.as_str())
            .copied();
        let reviewed =
            identity.is_some_and(|row| row.review_status == AhlIdentityReviewStatus::Reviewed);
        let nhl_player_id = reviewed
            .then(|| identity.and_then(|row| row.nhl_player_id))
            .flatten();
        let camp_player = nhl_player_id.and_then(|id| camp_by_id.get(&id).copied());
        if let Some(id) = nhl_player_id.filter(|_| camp_player.is_some()) {
            reconciled_camp_ids.insert(id);
        }
        let decision = nhl_player_id.and_then(|id| decisions.get(&id).copied());
        players.push(rollover_prior_row(
            prior,
            reviewed,
            nhl_player_id,
            camp_player,
            decision,
            &modal_ids,
        ));
    }
    for camp_player in camp_candidates {
        if reconciled_camp_ids.contains(&camp_player.player_id) {
            continue;
        }
        players.push(rollover_camp_row(camp_player, &modal_ids));
    }
    players.sort_by(|a, b| {
        b.projectable_affiliate_candidate
            .cmp(&a.projectable_affiliate_candidate)
            .then_with(|| a.position_group.cmp(&b.position_group))
            .then_with(|| {
                b.projected_score
                    .unwrap_or(f64::NEG_INFINITY)
                    .total_cmp(&a.projected_score.unwrap_or(f64::NEG_INFINITY))
            })
            .then_with(|| a.display_name.cmp(&b.display_name))
    });
    let counts = rollover_counts(prior_team.roster.len(), camp_candidates.len(), &players);
    Ok(AhlPreseasonRolloverView {
        schema: AHL_PRESEASON_ROLLOVER_SCHEMA.to_owned(),
        nhl_team: config.nhl_team.clone(),
        ahl_team: config.ahl_team.clone(),
        prior_season: prior_snapshot.season,
        target_season: config.target_season,
        as_of: config.as_of.clone(),
        source_urls: config.source_urls.clone(),
        counts,
        players,
        disclosures: vec![
            "This is a preseason rollover planning document, not an official AHL roster or an affiliate lineup projection.".to_owned(),
            format!(
                "Prior roster evidence comes from {}; the target-season affiliate is {}.",
                prior_ahl_team(config),
                config.ahl_team
            ),
            "Prior-affiliate identities must be reviewed and prior-only players need sourced organization-status decisions before they count toward projection coverage.".to_owned(),
            "Camp players outside the modal NHL roster count only when waiver-exempt; non-exempt players remain waiver-gated rather than assumed assigned.".to_owned(),
            "Projection readiness measures candidate-pool shape only. Professional-game totals, development-rule compliance, contracts, injuries, and final assignment rights remain required downstream.".to_owned(),
        ],
    })
}

fn rollover_prior_row(
    prior: &AhlRosterPlayer,
    identity_reviewed: bool,
    nhl_player_id: Option<u32>,
    camp_player: Option<&CampRolloverCandidate>,
    decision: Option<&AhlPreseasonOrganizationDecision>,
    modal_ids: &BTreeSet<u32>,
) -> AhlPreseasonRolloverPlayerView {
    let modal = nhl_player_id.is_some_and(|id| modal_ids.contains(&id));
    let mut blockers = Vec::new();
    if !identity_reviewed {
        blockers.push("identity_review".to_owned());
    }
    if modal {
        blockers.push("projected_nhl_roster".to_owned());
    }
    let waiver_exempt = camp_player.map(|row| row.waiver_exempt);
    if camp_player.is_some() && !modal && waiver_exempt == Some(false) {
        blockers.push("waiver_clearance".to_owned());
    }
    if camp_player.is_none() && decision.is_none() && identity_reviewed {
        blockers.push("organization_status_review".to_owned());
    }
    let excluded = decision.is_some_and(|row| {
        matches!(
            row.kind,
            AhlPreseasonDecisionKind::Departed | AhlPreseasonDecisionKind::OtherLeague
        )
    });
    let projectable = !excluded
        && identity_reviewed
        && !modal
        && (camp_player.is_some_and(|row| row.waiver_exempt)
            || (camp_player.is_none()
                && decision.is_some_and(|row| row.kind == AhlPreseasonDecisionKind::Retained)));
    AhlPreseasonRolloverPlayerView {
        nhl_player_id,
        prior_provider_player_id: Some(prior.provider_player_id.clone()),
        display_name: camp_player
            .map(|row| row.display_name.clone())
            .unwrap_or_else(|| prior.name.clone()),
        position_group: camp_player
            .map(|row| position_group(row.primary_position))
            .unwrap_or_else(|| prior_position_group(prior)),
        origins: if camp_player.is_some() {
            vec![
                AhlPreseasonRolloverOrigin::PriorAffiliate,
                AhlPreseasonRolloverOrigin::CurrentCamp,
            ]
        } else {
            vec![AhlPreseasonRolloverOrigin::PriorAffiliate]
        },
        identity_reviewed,
        organization_decision: decision.map(|row| row.kind),
        camp_make_probability: camp_player.map(|row| row.make_probability),
        camp_cut_probability: camp_player.map(|row| row.cut_probability),
        modal_nhl_roster: modal,
        waiver_exempt,
        projected_score: camp_player.map(|row| row.projected_score),
        projectable_affiliate_candidate: projectable,
        blockers,
    }
}

fn rollover_camp_row(
    camp_player: &CampRolloverCandidate,
    modal_ids: &BTreeSet<u32>,
) -> AhlPreseasonRolloverPlayerView {
    let modal = modal_ids.contains(&camp_player.player_id);
    let mut blockers = Vec::new();
    if modal {
        blockers.push("projected_nhl_roster".to_owned());
    } else if !camp_player.waiver_exempt {
        blockers.push("waiver_clearance".to_owned());
    }
    AhlPreseasonRolloverPlayerView {
        nhl_player_id: Some(camp_player.player_id),
        prior_provider_player_id: None,
        display_name: camp_player.display_name.clone(),
        position_group: position_group(camp_player.primary_position),
        origins: vec![AhlPreseasonRolloverOrigin::CurrentCamp],
        identity_reviewed: true,
        organization_decision: None,
        camp_make_probability: Some(camp_player.make_probability),
        camp_cut_probability: Some(camp_player.cut_probability),
        modal_nhl_roster: modal,
        waiver_exempt: Some(camp_player.waiver_exempt),
        projected_score: Some(camp_player.projected_score),
        projectable_affiliate_candidate: !modal && camp_player.waiver_exempt,
        blockers,
    }
}

fn rollover_counts(
    prior_roster_players: usize,
    current_camp_players: usize,
    players: &[AhlPreseasonRolloverPlayerView],
) -> AhlPreseasonRolloverCountsView {
    let count = |group| {
        players
            .iter()
            .filter(|row| row.projectable_affiliate_candidate && row.position_group == group)
            .count()
    };
    let forwards = count(AhlPreseasonPositionGroup::Forward);
    let defensemen = count(AhlPreseasonPositionGroup::Defense);
    let goalies = count(AhlPreseasonPositionGroup::Goalie);
    let unresolved = players
        .iter()
        .filter(|row| {
            row.origins
                .contains(&AhlPreseasonRolloverOrigin::PriorAffiliate)
        })
        .filter(|row| !row.identity_reviewed)
        .count();
    let organization_review = players
        .iter()
        .filter(|row| {
            row.blockers
                .iter()
                .any(|value| value == "organization_status_review")
        })
        .count();
    AhlPreseasonRolloverCountsView {
        prior_roster_players,
        current_camp_players,
        reconciled_players: players.iter().filter(|row| row.origins.len() == 2).count(),
        unresolved_prior_identities: unresolved,
        prior_players_needing_organization_review: organization_review,
        waiver_gated_candidates: players
            .iter()
            .filter(|row| row.blockers.iter().any(|value| value == "waiver_clearance"))
            .count(),
        projectable_forwards: forwards,
        projectable_defensemen: defensemen,
        projectable_goalies: goalies,
        forwards_needed: 12usize.saturating_sub(forwards),
        defensemen_needed: 6usize.saturating_sub(defensemen),
        goalies_needed: 2usize.saturating_sub(goalies),
        projection_ready: forwards >= 12
            && defensemen >= 6
            && goalies >= 2
            && unresolved == 0
            && organization_review == 0,
    }
}

fn validate_inputs(
    prior_snapshot: &AhlRosterStatsSnapshot,
    crosswalk: &AhlIdentityCrosswalkView,
    camp_input: &TrainingCampSimulationInput,
    camp_forecast: &TrainingCampForecastView,
    config: &AhlPreseasonRolloverConfig,
) -> Result<(), AhlFeedError> {
    validate_forecast_inputs(prior_snapshot, crosswalk, camp_forecast, config)?;
    if camp_input.team != camp_forecast.team || camp_input.season != camp_forecast.season {
        return Err(AhlFeedError::Validation(
            "preseason rollover camp input does not bind the sealed forecast".to_owned(),
        ));
    }
    let camp_ids = camp_input
        .players
        .iter()
        .map(|row| row.player_id)
        .collect::<BTreeSet<_>>();
    let forecast_ids = camp_forecast
        .players
        .iter()
        .map(|row| row.player_id)
        .collect::<BTreeSet<_>>();
    if camp_ids != forecast_ids {
        return Err(AhlFeedError::Validation(
            "camp forecast must exactly cover the rollover camp input".to_owned(),
        ));
    }
    Ok(())
}

fn validate_forecast_inputs(
    prior_snapshot: &AhlRosterStatsSnapshot,
    crosswalk: &AhlIdentityCrosswalkView,
    camp_forecast: &TrainingCampForecastView,
    config: &AhlPreseasonRolloverConfig,
) -> Result<(), AhlFeedError> {
    prior_snapshot.validate()?;
    let prior_ahl_team = prior_ahl_team(config);
    if config.target_season != camp_forecast.season
        || config.nhl_team != camp_forecast.team
        || config.ahl_team.trim().is_empty()
        || prior_ahl_team.trim().is_empty()
        || camp_forecast.schema != TRAINING_CAMP_FORECAST_SCHEMA
        || config.as_of.trim().is_empty()
        || config.source_urls.is_empty()
        || config.source_urls.iter().any(|url| !absolute_url(url))
    {
        return Err(AhlFeedError::Validation(
            "preseason rollover has mismatched camp authority or missing dated sources".to_owned(),
        ));
    }
    let prior_team = prior_snapshot
        .teams
        .iter()
        .find(|team| team.team_name == prior_ahl_team)
        .ok_or_else(|| {
            AhlFeedError::Validation(format!(
                "prior AHL snapshot has no team named `{}`",
                prior_ahl_team
            ))
        })?;
    if prior_team
        .nhl_affiliate
        .as_deref()
        .is_some_and(|team| team != config.nhl_team)
        || crosswalk.schema != AHL_IDENTITY_CROSSWALK_SCHEMA
        || crosswalk.season != prior_snapshot.season
        || crosswalk.provider != prior_snapshot.provider
        || crosswalk.ahl_team != prior_ahl_team
        || crosswalk.roster_fetched_at != prior_snapshot.fetched_at
    {
        return Err(AhlFeedError::Validation(
            "preseason rollover crosswalk does not bind the prior affiliate snapshot".to_owned(),
        ));
    }
    let official_by_provider = prior_team
        .roster
        .iter()
        .map(|row| (row.provider_player_id.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let mut crosswalk_provider_ids = BTreeSet::new();
    let mut reviewed_nhl_ids = BTreeSet::new();
    for row in &crosswalk.rows {
        let official = official_by_provider
            .get(row.provider_player_id.as_str())
            .ok_or_else(|| {
                AhlFeedError::Validation(format!(
                    "rollover crosswalk contains extra provider player {}",
                    row.provider_player_id
                ))
            })?;
        if !crosswalk_provider_ids.insert(row.provider_player_id.as_str())
            || row.ahl_display_name != official.name
            || row.ahl_birth_date != official.birthdate
        {
            return Err(AhlFeedError::Validation(format!(
                "rollover crosswalk altered or duplicated prior identity {}",
                row.provider_player_id
            )));
        }
        if row.review_status == AhlIdentityReviewStatus::Reviewed {
            let id = row.nhl_player_id.filter(|id| *id != 0).ok_or_else(|| {
                AhlFeedError::Validation(format!(
                    "reviewed rollover identity {} has no NHL player ID",
                    row.provider_player_id
                ))
            })?;
            if !reviewed_nhl_ids.insert(id) {
                return Err(AhlFeedError::Validation(format!(
                    "rollover crosswalk duplicates reviewed NHL player {id}"
                )));
            }
        }
    }
    if crosswalk_provider_ids.len() != official_by_provider.len() {
        return Err(AhlFeedError::Validation(
            "rollover crosswalk must exactly cover the prior official roster".to_owned(),
        ));
    }
    let mut forecast_ids = BTreeSet::new();
    if camp_forecast.players.is_empty()
        || camp_forecast
            .players
            .iter()
            .any(|row| row.player_id == 0 || !forecast_ids.insert(row.player_id))
        || camp_forecast
            .modal_opening_roster_ids
            .iter()
            .any(|id| !forecast_ids.contains(id))
    {
        return Err(AhlFeedError::Validation(
            "camp forecast has empty, duplicate, or unbound rollover players".to_owned(),
        ));
    }
    let reviewed_prior_ids = crosswalk
        .rows
        .iter()
        .filter(|row| row.review_status == AhlIdentityReviewStatus::Reviewed)
        .filter_map(|row| row.nhl_player_id)
        .collect::<BTreeSet<_>>();
    let mut decision_ids = BTreeSet::new();
    for decision in &config.prior_player_decisions {
        if decision.nhl_player_id == 0
            || !decision_ids.insert(decision.nhl_player_id)
            || !reviewed_prior_ids.contains(&decision.nhl_player_id)
            || decision.note.trim().is_empty()
            || decision.evidence_urls.is_empty()
            || decision.evidence_urls.iter().any(|url| !absolute_url(url))
        {
            return Err(AhlFeedError::Validation(
                "preseason rollover contains invalid or unbound prior-player decisions".to_owned(),
            ));
        }
    }
    Ok(())
}

fn prior_ahl_team(config: &AhlPreseasonRolloverConfig) -> &str {
    config
        .prior_ahl_team
        .as_deref()
        .unwrap_or(config.ahl_team.as_str())
}

fn position_group(position: Position) -> AhlPreseasonPositionGroup {
    match position {
        Position::Defense => AhlPreseasonPositionGroup::Defense,
        Position::Goalie => AhlPreseasonPositionGroup::Goalie,
        _ => AhlPreseasonPositionGroup::Forward,
    }
}

fn prior_position_group(player: &AhlRosterPlayer) -> AhlPreseasonPositionGroup {
    let position = normalize_name(&player.position);
    let group = normalize_name(&player.position_group);
    if position == "g" || group.contains("goal") {
        AhlPreseasonPositionGroup::Goalie
    } else if position == "d" || group.contains("defense") {
        AhlPreseasonPositionGroup::Defense
    } else if position == "f"
        || matches!(position.as_str(), "c" | "lw" | "rw" | "l" | "r")
        || group.contains("forward")
    {
        AhlPreseasonPositionGroup::Forward
    } else {
        AhlPreseasonPositionGroup::Unknown
    }
}

fn absolute_url(value: &str) -> bool {
    value.starts_with("https://") || value.starts_with("http://")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ahl::{
        AhlIdentityCrosswalkCounts, AhlIdentityCrosswalkRow, AhlIdentityMatchBasis,
        AhlTeamRosterStats, AHL_PROVIDER, AHL_ROSTER_SOURCE_URL, AHL_ROSTER_STATS_SCHEMA,
        AHL_STATS_SOURCE_URL,
    };

    fn inputs() -> (
        AhlRosterStatsSnapshot,
        AhlIdentityCrosswalkView,
        TrainingCampSimulationInput,
        TrainingCampForecastView,
        AhlPreseasonRolloverConfig,
    ) {
        let mut camp: TrainingCampSimulationInput = serde_json::from_str(include_str!(
            "../../examples/icecast-nyr-training-camp.json"
        ))
        .unwrap();
        camp.config.trials = 100;
        let forecast = icelines_core::simulate_training_camp(&camp).unwrap();
        let snapshot = AhlRosterStatsSnapshot {
            schema: AHL_ROSTER_STATS_SCHEMA.to_owned(),
            season: 20252026,
            provider: AHL_PROVIDER.to_owned(),
            provider_season_id: "90".to_owned(),
            provider_season_name: "2025-26 Regular Season".to_owned(),
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
                    provider_player_id: "8430".to_owned(),
                    name: "Dylan Garand".to_owned(),
                    position_group: "Goalies".to_owned(),
                    position: "G".to_owned(),
                    jersey_number: "31".to_owned(),
                    handedness: "L".to_owned(),
                    height: "6-1".to_owned(),
                    weight_pounds: "176".to_owned(),
                    birthdate: "2002-06-07".to_owned(),
                    birthplace: "Victoria, BC".to_owned(),
                }],
                skaters: Vec::new(),
                goalies: Vec::new(),
                source_warnings: Vec::new(),
            }],
        };
        let crosswalk = AhlIdentityCrosswalkView {
            schema: AHL_IDENTITY_CROSSWALK_SCHEMA.to_owned(),
            season: snapshot.season,
            provider: snapshot.provider.clone(),
            ahl_team: "Hartford Wolf Pack".to_owned(),
            nhl_affiliate: Some("NYR".to_owned()),
            roster_fetched_at: snapshot.fetched_at.clone(),
            candidates_checked_at: "2026-07-24".to_owned(),
            counts: AhlIdentityCrosswalkCounts {
                roster_players: 1,
                exact_name_and_birth_date: 1,
                surname_and_birth_date: 0,
                exact_name_only: 0,
                ambiguous: 0,
                conflicts: 0,
                unmatched: 0,
                reviewed: 1,
            },
            rows: vec![AhlIdentityCrosswalkRow {
                provider_player_id: "8430".to_owned(),
                ahl_display_name: "Dylan Garand".to_owned(),
                ahl_birth_date: "2002-06-07".to_owned(),
                match_basis: AhlIdentityMatchBasis::ExactNameAndBirthDate,
                review_status: AhlIdentityReviewStatus::Reviewed,
                nhl_player_id: Some(8_482_193),
                nhl_display_name: Some("Dylan Garand".to_owned()),
                nhl_birth_date: Some("2002-06-07".to_owned()),
                evidence_urls: vec!["https://www.nhl.com/player/8482193".to_owned()],
                note: "reviewed fixture".to_owned(),
            }],
            disclosures: Vec::new(),
        };
        let config = AhlPreseasonRolloverConfig {
            target_season: 20262027,
            nhl_team: "NYR".to_owned(),
            ahl_team: "Hartford Wolf Pack".to_owned(),
            prior_ahl_team: None,
            as_of: "2026-07-24".to_owned(),
            source_urls: vec!["https://example.test/nyr-camp".to_owned()],
            prior_player_decisions: Vec::new(),
        };
        (snapshot, crosswalk, camp, forecast, config)
    }

    #[test]
    fn reconciles_reviewed_prior_identity_with_current_camp_player() {
        let (snapshot, crosswalk, camp, forecast, config) = inputs();
        let view =
            build_ahl_preseason_rollover(&snapshot, &crosswalk, &camp, &forecast, &config).unwrap();
        let forecast_only =
            build_ahl_preseason_rollover_from_forecast(&snapshot, &crosswalk, &forecast, &config)
                .unwrap();
        assert_eq!(forecast_only, view);
        assert_eq!(view.schema, AHL_PRESEASON_ROLLOVER_SCHEMA);
        assert_eq!(view.counts.reconciled_players, 1);
        let garand = view
            .players
            .iter()
            .find(|row| row.display_name == "Dylan Garand")
            .unwrap();
        assert_eq!(garand.origins.len(), 2);
        assert_eq!(garand.nhl_player_id, Some(8_482_193));
        assert!(garand.identity_reviewed);
    }

    #[test]
    fn pending_prior_identity_keeps_rollover_not_ready() {
        let (snapshot, mut crosswalk, camp, forecast, config) = inputs();
        crosswalk.rows[0].review_status = AhlIdentityReviewStatus::Pending;
        crosswalk.rows[0].nhl_player_id = Some(8_482_193);
        let view =
            build_ahl_preseason_rollover(&snapshot, &crosswalk, &camp, &forecast, &config).unwrap();
        assert_eq!(view.counts.unresolved_prior_identities, 1);
        assert!(!view.counts.projection_ready);
        assert!(view.players.iter().any(|row| {
            row.prior_provider_player_id.as_deref() == Some("8430")
                && row.blockers.iter().any(|value| value == "identity_review")
        }));
    }

    #[test]
    fn explicit_team_binding_accepts_missing_historical_affiliate_label() {
        let (mut snapshot, mut crosswalk, camp, forecast, config) = inputs();
        snapshot.teams[0].nhl_affiliate = None;
        crosswalk.nhl_affiliate = None;

        let view =
            build_ahl_preseason_rollover(&snapshot, &crosswalk, &camp, &forecast, &config).unwrap();

        assert_eq!(view.nhl_team, "NYR");
        assert_eq!(view.counts.reconciled_players, 1);
    }

    #[test]
    fn changed_affiliate_keeps_prior_snapshot_and_target_club_distinct() {
        let (snapshot, crosswalk, camp, forecast, mut config) = inputs();
        config.nhl_team = "NYI".to_owned();
        config.ahl_team = "Hamilton Hammers".to_owned();
        config.prior_ahl_team = Some("Hartford Wolf Pack".to_owned());
        let mut camp = camp;
        camp.team = "NYI".to_owned();
        let mut forecast = forecast;
        forecast.team = "NYI".to_owned();
        let mut snapshot = snapshot;
        snapshot.teams[0].nhl_affiliate = Some("NYI".to_owned());
        let mut crosswalk = crosswalk;
        crosswalk.nhl_affiliate = Some("NYI".to_owned());

        let view =
            build_ahl_preseason_rollover(&snapshot, &crosswalk, &camp, &forecast, &config).unwrap();

        assert_eq!(view.ahl_team, "Hamilton Hammers");
        assert!(view.disclosures.iter().any(|line| {
            line.contains("Prior roster evidence comes from Hartford Wolf Pack")
                && line.contains("target-season affiliate is Hamilton Hammers")
        }));
    }

    #[test]
    fn forecast_native_rollover_rejects_unbound_modal_player() {
        let (snapshot, crosswalk, _camp, mut forecast, config) = inputs();
        forecast.modal_opening_roster_ids.push(999_999);

        let error =
            build_ahl_preseason_rollover_from_forecast(&snapshot, &crosswalk, &forecast, &config)
                .unwrap_err();

        assert!(error.to_string().contains("unbound rollover players"));
    }

    #[test]
    fn league_rollover_composes_forecasts_and_requires_exact_team_config() {
        let (snapshot, crosswalk, _camp, forecast, team_config) = inputs();
        let league_crosswalk = AhlIdentityLeagueCrosswalkView {
            schema: AHL_IDENTITY_LEAGUE_CROSSWALK_SCHEMA.to_owned(),
            season: snapshot.season,
            provider: snapshot.provider.clone(),
            roster_fetched_at: snapshot.fetched_at.clone(),
            candidates_checked_at: crosswalk.candidates_checked_at.clone(),
            teams: 1,
            roster_appearances: crosswalk.rows.len(),
            unique_provider_players: crosswalk.rows.len(),
            crosswalks: vec![crosswalk],
            disclosures: Vec::new(),
        };
        let camp_league = TrainingCampLeagueForecastView {
            schema: TRAINING_CAMP_LEAGUE_FORECAST_SCHEMA.to_owned(),
            season: forecast.season,
            teams_requested: 1,
            teams_simulated: 1,
            teams_degraded: 0,
            teams_augmented: 0,
            teams_failed: 0,
            teams: vec![icelines_core::TrainingCampLeagueTeamView {
                team: forecast.team.clone(),
                authority_status: icelines_core::TrainingCampAuthorityStatus::ConfirmedPool,
                competition_pool_status: icelines_core::TrainingCampCompetitionPoolStatus::Authored,
                current_roster_candidates: forecast.players.len(),
                sourced_overlay_candidates: 0,
                fallback_candidates: 0,
                forecast: Some(forecast),
                error: None,
                authority_warnings: Vec::new(),
            }],
            disclosures: Vec::new(),
        };
        let config = AhlPreseasonLeagueRolloverConfig {
            schema: AHL_PRESEASON_LEAGUE_ROLLOVER_CONFIG_SCHEMA.to_owned(),
            prior_season: snapshot.season,
            target_season: team_config.target_season,
            teams: vec![team_config],
        };

        let affiliations = AhlAffiliationCatalogView {
            schema: AHL_AFFILIATION_CATALOG_SCHEMA.to_owned(),
            season: camp_league.season,
            checked_at: "2026-07-24".to_owned(),
            source_url: "https://theahl.com/nhl-affiliations".to_owned(),
            affiliations: vec![icelines_core::AhlAffiliationView {
                nhl_team: "NYR".to_owned(),
                ahl_team: "Hartford Wolf Pack".to_owned(),
            }],
        };
        let mut prior_affiliations = affiliations.clone();
        prior_affiliations.season = snapshot.season;
        prior_affiliations.checked_at = "2025-10-10".to_owned();
        let drafted = build_ahl_preseason_league_rollover_config_draft(
            &league_crosswalk,
            &camp_league,
            &prior_affiliations,
            &affiliations,
            "2026-07-28",
            vec![affiliations.source_url.clone()],
        )
        .unwrap();
        assert_eq!(drafted.teams.len(), 1);
        assert_eq!(drafted.teams[0].nhl_team, config.teams[0].nhl_team);
        assert_eq!(drafted.teams[0].ahl_team, config.teams[0].ahl_team);
        assert_eq!(
            drafted.teams[0].prior_ahl_team,
            Some("Hartford Wolf Pack".to_owned())
        );
        assert!(drafted.teams[0].prior_player_decisions.is_empty());

        let mut review = build_ahl_preseason_league_organization_review_draft(
            &snapshot,
            &league_crosswalk,
            &camp_league,
            &config,
        )
        .unwrap();
        assert_eq!(
            review.schema,
            AHL_PRESEASON_LEAGUE_ORGANIZATION_REVIEW_SCHEMA
        );
        assert_eq!(review.teams_built, 1);
        assert_eq!(review.identity_blockers, 0);
        assert_eq!(review.decisions_required, 0);
        assert!(review.failures.is_empty());
        review.draft = false;
        review.reviews[0].draft = false;
        review.reviews[0].reviewer = Some("League Reviewer".to_owned());
        review.reviews[0].reviewed_at = Some("2026-07-28T20:00:00Z".to_owned());
        let applied = apply_ahl_preseason_league_organization_review(
            &snapshot,
            &league_crosswalk,
            &camp_league,
            &config,
            &review,
        )
        .unwrap();
        assert_eq!(applied, config);

        let view = build_ahl_preseason_league_rollover(
            &snapshot,
            &league_crosswalk,
            &camp_league,
            &config,
        )
        .unwrap();
        assert_eq!(view.schema, AHL_PRESEASON_LEAGUE_ROLLOVER_SCHEMA);
        assert_eq!(view.teams_requested, 1);
        assert_eq!(view.teams_built, 1);
        assert!(view.failures.is_empty());

        let mut incomplete = config;
        incomplete.teams.clear();
        assert!(build_ahl_preseason_league_rollover(
            &snapshot,
            &league_crosswalk,
            &camp_league,
            &incomplete,
        )
        .unwrap_err()
        .to_string()
        .contains("mismatched snapshot"));
    }

    #[test]
    fn organization_review_draft_requires_finalized_sourced_prior_only_decision() {
        let (snapshot, crosswalk, mut camp, mut forecast, config) = inputs();
        camp.players.retain(|player| player.player_id != 8_482_193);
        forecast
            .players
            .retain(|player| player.player_id != 8_482_193);
        forecast
            .modal_opening_roster_ids
            .retain(|id| *id != 8_482_193);
        let mut review = build_ahl_preseason_organization_review_draft(
            &snapshot,
            &crosswalk,
            &camp,
            "NYR",
            "Hartford Wolf Pack",
        )
        .unwrap();
        assert!(review.draft);
        assert_eq!(review.identity_blockers, 0);
        assert_eq!(review.decisions_required, 1);
        assert!(apply_ahl_preseason_organization_review(
            &snapshot, &crosswalk, &camp, &config, &review
        )
        .is_err());

        review.draft = false;
        review.reviewer = Some("Test Reviewer".to_owned());
        review.reviewed_at = Some("2026-07-24T20:00:00-07:00".to_owned());
        review.rows[0].decision_kind = Some(AhlPreseasonDecisionKind::Retained);
        review.rows[0].evidence_urls = vec!["https://example.test/garand-retained".to_owned()];
        review.rows[0].note =
            "Confirmed under contract and retained in the organization.".to_owned();
        let applied =
            apply_ahl_preseason_organization_review(&snapshot, &crosswalk, &camp, &config, &review)
                .unwrap();
        let forecast_applied = apply_ahl_preseason_organization_review_from_forecast(
            &snapshot, &crosswalk, &forecast, &config, &review,
        )
        .unwrap();
        assert_eq!(forecast_applied, applied);
        assert_eq!(applied.prior_player_decisions.len(), 1);
        assert_eq!(
            applied.prior_player_decisions[0].kind,
            AhlPreseasonDecisionKind::Retained
        );
        assert!(applied.prior_player_decisions[0]
            .note
            .contains("Test Reviewer"));
    }

    #[test]
    fn forecast_native_organization_review_matches_explicit_camp_input() {
        let (snapshot, crosswalk, camp, forecast, config) = inputs();
        let explicit = build_ahl_preseason_organization_review_draft(
            &snapshot,
            &crosswalk,
            &camp,
            &config.nhl_team,
            prior_ahl_team(&config),
        )
        .unwrap();
        let forecast_native = build_ahl_preseason_organization_review_draft_from_forecast(
            &snapshot,
            &crosswalk,
            &forecast,
            &config.nhl_team,
            prior_ahl_team(&config),
        )
        .unwrap();

        assert_eq!(forecast_native, explicit);
    }

    #[test]
    fn mapping_rejection_does_not_block_other_status_review_application() {
        let (snapshot, mut crosswalk, camp, _forecast, config) = inputs();
        crosswalk.rows[0].review_status = AhlIdentityReviewStatus::Rejected;
        crosswalk.rows[0].nhl_player_id = None;
        crosswalk.rows[0].nhl_display_name = None;
        crosswalk.rows[0].nhl_birth_date = None;
        let mut review = build_ahl_preseason_organization_review_draft(
            &snapshot,
            &crosswalk,
            &camp,
            &config.nhl_team,
            prior_ahl_team(&config),
        )
        .unwrap();
        assert_eq!(review.identity_blockers, 1);
        assert_eq!(review.decisions_required, 0);
        review.draft = false;
        review.reviewer = Some("Status Reviewer".to_owned());
        review.reviewed_at = Some("2026-07-28T20:00:00Z".to_owned());

        let applied =
            apply_ahl_preseason_organization_review(&snapshot, &crosswalk, &camp, &config, &review)
                .unwrap();

        assert!(applied.prior_player_decisions.is_empty());
    }

    #[test]
    fn organization_review_fingerprint_changes_with_identity_approval() {
        let (snapshot, mut crosswalk, camp, _forecast, _config) = inputs();
        let reviewed = build_ahl_preseason_organization_review_draft(
            &snapshot,
            &crosswalk,
            &camp,
            "NYR",
            "Hartford Wolf Pack",
        )
        .unwrap();
        crosswalk.rows[0].review_status = AhlIdentityReviewStatus::Pending;
        let pending = build_ahl_preseason_organization_review_draft(
            &snapshot,
            &crosswalk,
            &camp,
            "NYR",
            "Hartford Wolf Pack",
        )
        .unwrap();
        assert_ne!(
            reviewed.crosswalk_fingerprint,
            pending.crosswalk_fingerprint
        );
        assert_eq!(pending.identity_blockers, 1);
    }
}
