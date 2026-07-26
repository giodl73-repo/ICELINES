//! Preseason AHL roster-pool rollover planning.
//!
//! This module never emits an affiliate projection. It reconciles prior
//! official roster identities with a current NHL camp forecast and reports
//! whether a sourced 12F/6D/2G projected pool can be authored safely.

use std::collections::{BTreeMap, BTreeSet};

use icelines_core::{
    normalize_name, Position, TrainingCampForecastView, TrainingCampSimulationInput,
    TRAINING_CAMP_FORECAST_SCHEMA,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::ahl::{
    AhlFeedError, AhlIdentityCrosswalkView, AhlIdentityReviewStatus, AhlRosterPlayer,
    AhlRosterStatsSnapshot, AHL_IDENTITY_CROSSWALK_SCHEMA,
};

pub const AHL_PRESEASON_ROLLOVER_SCHEMA: &str = "ahl_preseason_rollover.v1";
pub const AHL_PRESEASON_ORGANIZATION_REVIEW_SCHEMA: &str = "ahl_preseason_organization_review.v1";

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
    pub ahl_team: String,
    pub as_of: String,
    pub source_urls: Vec<String>,
    #[serde(default)]
    pub prior_player_decisions: Vec<AhlPreseasonOrganizationDecision>,
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
        camp_input,
        nhl_team,
        ahl_team,
    )?;
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
    let camp_ids = camp_input
        .players
        .iter()
        .map(|row| row.player_id)
        .collect::<BTreeSet<_>>();
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
        target_season: camp_input.season,
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
        &base_config.ahl_team,
    )?;
    if review.schema != AHL_PRESEASON_ORGANIZATION_REVIEW_SCHEMA
        || review.prior_season != expected.prior_season
        || review.target_season != expected.target_season
        || review.nhl_team != expected.nhl_team
        || review.ahl_team != expected.ahl_team
        || review.provider != expected.provider
        || review.roster_fetched_at != expected.roster_fetched_at
        || review.crosswalk_fingerprint != expected.crosswalk_fingerprint
        || review.draft
        || expected.identity_blockers != 0
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
        || base_config.target_season != camp_input.season
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
    camp_input: &TrainingCampSimulationInput,
    nhl_team: &str,
    ahl_team: &str,
) -> Result<(), AhlFeedError> {
    prior_snapshot.validate()?;
    let team = prior_snapshot
        .teams
        .iter()
        .find(|team| team.team_name == ahl_team)
        .ok_or_else(|| {
            AhlFeedError::Validation(format!("prior AHL snapshot has no team named `{ahl_team}`"))
        })?;
    if nhl_team != camp_input.team
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

pub fn build_ahl_preseason_rollover(
    prior_snapshot: &AhlRosterStatsSnapshot,
    crosswalk: &AhlIdentityCrosswalkView,
    camp_input: &TrainingCampSimulationInput,
    camp_forecast: &TrainingCampForecastView,
    config: &AhlPreseasonRolloverConfig,
) -> Result<AhlPreseasonRolloverView, AhlFeedError> {
    validate_inputs(prior_snapshot, crosswalk, camp_input, camp_forecast, config)?;
    let prior_team = prior_snapshot
        .teams
        .iter()
        .find(|team| team.team_name == config.ahl_team)
        .expect("validation found prior affiliate");
    let crosswalk_by_provider = crosswalk
        .rows
        .iter()
        .map(|row| (row.provider_player_id.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let camp_input_by_id = camp_input
        .players
        .iter()
        .map(|player| (player.player_id, player))
        .collect::<BTreeMap<_, _>>();
    let camp_view_by_id = camp_forecast
        .players
        .iter()
        .map(|player| (player.player_id, player))
        .collect::<BTreeMap<_, _>>();
    let modal_ids = camp_forecast
        .modal_opening_roster_ids
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
        let camp_input_player = nhl_player_id.and_then(|id| camp_input_by_id.get(&id).copied());
        let camp_view_player = nhl_player_id.and_then(|id| camp_view_by_id.get(&id).copied());
        if let Some(id) = nhl_player_id.filter(|_| camp_input_player.is_some()) {
            reconciled_camp_ids.insert(id);
        }
        let decision = nhl_player_id.and_then(|id| decisions.get(&id).copied());
        players.push(rollover_prior_row(
            prior,
            reviewed,
            nhl_player_id,
            camp_input_player,
            camp_view_player,
            decision,
            &modal_ids,
        ));
    }
    for camp_player in &camp_input.players {
        if reconciled_camp_ids.contains(&camp_player.player_id) {
            continue;
        }
        let view = camp_view_by_id
            .get(&camp_player.player_id)
            .copied()
            .expect("validated camp forecast covers input players");
        players.push(rollover_camp_row(camp_player, view, &modal_ids));
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
    let counts = rollover_counts(prior_team.roster.len(), camp_input.players.len(), &players);
    Ok(AhlPreseasonRolloverView {
        schema: AHL_PRESEASON_ROLLOVER_SCHEMA.to_owned(),
        nhl_team: camp_input.team.clone(),
        ahl_team: config.ahl_team.clone(),
        prior_season: prior_snapshot.season,
        target_season: config.target_season,
        as_of: config.as_of.clone(),
        source_urls: config.source_urls.clone(),
        counts,
        players,
        disclosures: vec![
            "This is a preseason rollover planning document, not an official AHL roster or an affiliate lineup projection.".to_owned(),
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
    camp_input: Option<&icelines_core::TrainingCampPlayerInput>,
    camp_view: Option<&icelines_core::TrainingCampPlayerView>,
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
    let waiver_exempt = camp_input.map(|row| row.waiver_exempt);
    if camp_input.is_some() && !modal && waiver_exempt == Some(false) {
        blockers.push("waiver_clearance".to_owned());
    }
    if camp_input.is_none() && decision.is_none() && identity_reviewed {
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
        && (camp_input.is_some_and(|row| row.waiver_exempt)
            || (camp_input.is_none()
                && decision.is_some_and(|row| row.kind == AhlPreseasonDecisionKind::Retained)));
    AhlPreseasonRolloverPlayerView {
        nhl_player_id,
        prior_provider_player_id: Some(prior.provider_player_id.clone()),
        display_name: camp_input
            .map(|row| row.display_name.clone())
            .unwrap_or_else(|| prior.name.clone()),
        position_group: camp_input
            .map(|row| position_group(row.primary_position))
            .unwrap_or_else(|| prior_position_group(prior)),
        origins: if camp_input.is_some() {
            vec![
                AhlPreseasonRolloverOrigin::PriorAffiliate,
                AhlPreseasonRolloverOrigin::CurrentCamp,
            ]
        } else {
            vec![AhlPreseasonRolloverOrigin::PriorAffiliate]
        },
        identity_reviewed,
        organization_decision: decision.map(|row| row.kind),
        camp_make_probability: camp_view.map(|row| row.make_probability),
        camp_cut_probability: camp_view.map(|row| row.cut_probability),
        modal_nhl_roster: modal,
        waiver_exempt,
        projected_score: camp_input.map(|row| row.projected_score),
        projectable_affiliate_candidate: projectable,
        blockers,
    }
}

fn rollover_camp_row(
    camp_input: &icelines_core::TrainingCampPlayerInput,
    camp_view: &icelines_core::TrainingCampPlayerView,
    modal_ids: &BTreeSet<u32>,
) -> AhlPreseasonRolloverPlayerView {
    let modal = modal_ids.contains(&camp_input.player_id);
    let mut blockers = Vec::new();
    if modal {
        blockers.push("projected_nhl_roster".to_owned());
    } else if !camp_input.waiver_exempt {
        blockers.push("waiver_clearance".to_owned());
    }
    AhlPreseasonRolloverPlayerView {
        nhl_player_id: Some(camp_input.player_id),
        prior_provider_player_id: None,
        display_name: camp_input.display_name.clone(),
        position_group: position_group(camp_input.primary_position),
        origins: vec![AhlPreseasonRolloverOrigin::CurrentCamp],
        identity_reviewed: true,
        organization_decision: None,
        camp_make_probability: Some(camp_view.make_probability),
        camp_cut_probability: Some(camp_view.cut_probability),
        modal_nhl_roster: modal,
        waiver_exempt: Some(camp_input.waiver_exempt),
        projected_score: Some(camp_input.projected_score),
        projectable_affiliate_candidate: !modal && camp_input.waiver_exempt,
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
    prior_snapshot.validate()?;
    if config.target_season != camp_input.season
        || config.nhl_team != camp_input.team
        || camp_forecast.schema != TRAINING_CAMP_FORECAST_SCHEMA
        || camp_forecast.team != camp_input.team
        || camp_forecast.season != camp_input.season
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
        .find(|team| team.team_name == config.ahl_team)
        .ok_or_else(|| {
            AhlFeedError::Validation(format!(
                "prior AHL snapshot has no team named `{}`",
                config.ahl_team
            ))
        })?;
    if prior_team
        .nhl_affiliate
        .as_deref()
        .is_some_and(|team| team != config.nhl_team)
        || crosswalk.schema != AHL_IDENTITY_CROSSWALK_SCHEMA
        || crosswalk.season != prior_snapshot.season
        || crosswalk.provider != prior_snapshot.provider
        || crosswalk.ahl_team != config.ahl_team
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
    fn organization_review_draft_requires_finalized_sourced_prior_only_decision() {
        let (snapshot, crosswalk, mut camp, _forecast, config) = inputs();
        camp.players.retain(|player| player.player_id != 8_482_193);
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
