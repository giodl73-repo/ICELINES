//! League-wide preseason affiliate facts workboard.
//!
//! This is a composition/readiness artifact, not an assignment model. It
//! merges the rollover candidate pool with professional-game evidence and
//! names every authority still required before an AHL projection can exist.

use std::collections::{BTreeMap, BTreeSet};

use chrono::DateTime;
use icelines_core::{
    build_ahl_affiliate_projection, AhlAffiliatePlayerInput, AhlAffiliateProjectionInput,
    AhlDevelopmentRuleInput, AhlRosterPoolAuthority, AhlRosterPoolAuthorityKind, Position,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    ahl::AhlFeedError,
    ahl_professional_games::{AhlProfessionalGameLedgerView, AHL_PROFESSIONAL_GAME_LEDGER_SCHEMA},
    ahl_rollover::{
        AhlPreseasonDecisionKind, AhlPreseasonLeagueRolloverView, AhlPreseasonPositionGroup,
        AhlPreseasonRolloverOrigin, AHL_PRESEASON_LEAGUE_ROLLOVER_SCHEMA,
    },
};

pub const AHL_PRESEASON_LEAGUE_FACTS_WORKBOARD_SCHEMA: &str =
    "ahl_preseason_league_facts_workboard.v1";
pub const AHL_PRESEASON_LEAGUE_FACTS_OVERLAY_SCHEMA: &str = "ahl_preseason_league_facts_overlay.v1";
pub const AHL_PRESEASON_LEAGUE_FACTS_APPLICATION_SCHEMA: &str =
    "ahl_preseason_league_facts_application.v1";
pub const AHL_PRESEASON_LEAGUE_PROJECTION_INPUTS_SCHEMA: &str =
    "ahl_preseason_league_projection_inputs.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AhlPreseasonFactBlocker {
    IdentityReview,
    OrganizationStatus,
    WaiverClearance,
    ExactPosition,
    ProjectedScore,
    ProspectStatus,
    RecallReadiness,
    ProfessionalGames,
    DevelopmentRuleQualification,
    AssignmentAuthority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AhlPreseasonFactsCandidateStatus {
    Candidate,
    NotAssigned,
    ProjectedNhlRoster,
    Departed,
    OtherLeague,
    IdentityBlocked,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlPreseasonFactsPlayerRow {
    pub nhl_player_id: Option<u32>,
    pub display_name: String,
    pub status: AhlPreseasonFactsCandidateStatus,
    pub origins: Vec<AhlPreseasonRolloverOrigin>,
    pub position_group: AhlPreseasonPositionGroup,
    pub primary_position: Option<Position>,
    pub eligible_positions: Vec<Position>,
    pub projected_score: Option<f64>,
    #[serde(default)]
    pub prospect: Option<bool>,
    #[serde(default)]
    pub recall_readiness: Option<f64>,
    #[serde(default)]
    pub assigned_to_affiliate: Option<bool>,
    #[serde(default)]
    pub waiver_cleared: Option<bool>,
    #[serde(default)]
    pub review_source_urls: Vec<String>,
    #[serde(default)]
    pub review_note: Option<String>,
    #[serde(default)]
    pub reviewer: Option<String>,
    #[serde(default)]
    pub reviewed_at: Option<String>,
    pub professional_games_at_season_start: Option<u32>,
    pub development_rule_qualified: Option<bool>,
    pub blockers: Vec<AhlPreseasonFactBlocker>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlPreseasonFactsTeamCounts {
    pub players: usize,
    pub candidates: usize,
    pub facts_ready_candidates: usize,
    #[serde(default)]
    pub not_assigned: usize,
    pub projected_nhl_roster: usize,
    pub explicit_departures: usize,
    pub identity_blocked: usize,
    pub missing_assignment_authority: usize,
    pub missing_organization_status: usize,
    pub missing_waiver_clearance: usize,
    pub missing_exact_position: usize,
    pub missing_projected_score: usize,
    pub missing_prospect_status: usize,
    pub missing_recall_readiness: usize,
    pub missing_professional_games: usize,
    pub missing_development_rule_qualification: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlPreseasonFactsTeamView {
    pub nhl_team: String,
    pub ahl_team: String,
    #[serde(default)]
    pub source_urls: Vec<String>,
    pub counts: AhlPreseasonFactsTeamCounts,
    pub players: Vec<AhlPreseasonFactsPlayerRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlPreseasonLeagueFactsWorkboardView {
    pub schema: String,
    pub prior_season: u32,
    pub target_season: u32,
    pub professional_game_policy_id: String,
    pub professional_game_policy_authority: String,
    #[serde(default)]
    pub professional_game_threshold: u32,
    #[serde(default)]
    pub source_fingerprint: String,
    pub teams: usize,
    pub candidates: usize,
    pub facts_ready_candidates: usize,
    pub blocker_counts: BTreeMap<AhlPreseasonFactBlocker, usize>,
    pub team_workboards: Vec<AhlPreseasonFactsTeamView>,
    pub disclosures: Vec<String>,
}

/// Separately reviewed facts. Every optional field clears only its matching
/// blocker; absence is never interpreted as false or zero.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlPreseasonPlayerFactsOverlayRow {
    pub nhl_team: String,
    pub nhl_player_id: u32,
    #[serde(default)]
    pub primary_position: Option<Position>,
    #[serde(default)]
    pub eligible_positions: Option<Vec<Position>>,
    #[serde(default)]
    pub projected_score: Option<f64>,
    #[serde(default)]
    pub prospect: Option<bool>,
    #[serde(default)]
    pub recall_readiness: Option<f64>,
    #[serde(default)]
    pub assigned_to_affiliate: Option<bool>,
    #[serde(default)]
    pub waiver_cleared: Option<bool>,
    pub source_urls: Vec<String>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlPreseasonLeagueFactsOverlay {
    pub schema: String,
    pub prior_season: u32,
    pub target_season: u32,
    pub workboard_fingerprint: String,
    pub draft: bool,
    #[serde(default)]
    pub reviewer: Option<String>,
    #[serde(default)]
    pub reviewed_at: Option<String>,
    pub rows: Vec<AhlPreseasonPlayerFactsOverlayRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlPreseasonLeagueFactsApplicationView {
    pub schema: String,
    pub source_workboard_fingerprint: String,
    pub overlay_fingerprint: String,
    pub prior_season: u32,
    pub target_season: u32,
    pub rows_applied: usize,
    pub candidates: usize,
    pub facts_ready_candidates: usize,
    pub blocker_counts: BTreeMap<AhlPreseasonFactBlocker, usize>,
    pub workboard: AhlPreseasonLeagueFactsWorkboardView,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlPreseasonLeagueProjectionInputFailure {
    pub nhl_team: String,
    pub ahl_team: String,
    pub reason: String,
    pub blocker_counts: BTreeMap<AhlPreseasonFactBlocker, usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlPreseasonLeagueProjectionInputsView {
    pub schema: String,
    pub target_season: u32,
    pub facts_application_fingerprint: String,
    pub teams_requested: usize,
    pub teams_built: usize,
    pub inputs: Vec<AhlAffiliateProjectionInput>,
    pub failures: Vec<AhlPreseasonLeagueProjectionInputFailure>,
    pub disclosures: Vec<String>,
}

pub fn build_ahl_preseason_league_facts_workboard(
    rollover: &AhlPreseasonLeagueRolloverView,
    professional_games: &AhlProfessionalGameLedgerView,
) -> Result<AhlPreseasonLeagueFactsWorkboardView, AhlFeedError> {
    if rollover.schema != AHL_PRESEASON_LEAGUE_ROLLOVER_SCHEMA
        || professional_games.schema != AHL_PROFESSIONAL_GAME_LEDGER_SCHEMA
        || rollover.prior_season != professional_games.prior_season
        || rollover.target_season != professional_games.target_season
        || rollover.teams_built != rollover.rollovers.len()
        || rollover.teams_requested != rollover.teams_built + rollover.failures.len()
        || !rollover.failures.is_empty()
    {
        return Err(AhlFeedError::Validation(
            "preseason facts workboard requires complete matching league rollover and professional-game authorities"
                .to_owned(),
        ));
    }
    let ledger = professional_games
        .players
        .iter()
        .map(|player| (player.nhl_player_id, player))
        .collect::<BTreeMap<_, _>>();
    if ledger.len() != professional_games.players.len() {
        return Err(AhlFeedError::Validation(
            "professional-game ledger contains duplicate players".to_owned(),
        ));
    }
    let mut team_names = BTreeSet::new();
    let mut team_workboards = Vec::with_capacity(rollover.rollovers.len());
    let mut blocker_counts = BTreeMap::new();
    for team in &rollover.rollovers {
        if !team_names.insert(team.nhl_team.as_str())
            || team.prior_season != rollover.prior_season
            || team.target_season != rollover.target_season
        {
            return Err(AhlFeedError::Validation(
                "preseason facts rollover contains duplicate or mismatched teams".to_owned(),
            ));
        }
        let mut players = Vec::with_capacity(team.players.len());
        for player in &team.players {
            let status = if !player.identity_reviewed {
                AhlPreseasonFactsCandidateStatus::IdentityBlocked
            } else if player.modal_nhl_roster {
                AhlPreseasonFactsCandidateStatus::ProjectedNhlRoster
            } else {
                match player.organization_decision {
                    Some(AhlPreseasonDecisionKind::Departed) => {
                        AhlPreseasonFactsCandidateStatus::Departed
                    }
                    Some(AhlPreseasonDecisionKind::OtherLeague) => {
                        AhlPreseasonFactsCandidateStatus::OtherLeague
                    }
                    _ => AhlPreseasonFactsCandidateStatus::Candidate,
                }
            };
            let evidence = player.nhl_player_id.and_then(|id| ledger.get(&id).copied());
            let professional_games_at_season_start =
                evidence.and_then(|row| row.professional_games_at_season_start);
            let development_rule_qualified =
                evidence.and_then(|row| row.development_rule_qualified);
            let mut blockers = BTreeSet::new();
            if status == AhlPreseasonFactsCandidateStatus::IdentityBlocked {
                blockers.insert(AhlPreseasonFactBlocker::IdentityReview);
            }
            if status == AhlPreseasonFactsCandidateStatus::Candidate {
                blockers.insert(AhlPreseasonFactBlocker::AssignmentAuthority);
                blockers.insert(AhlPreseasonFactBlocker::ProspectStatus);
                blockers.insert(AhlPreseasonFactBlocker::RecallReadiness);
                if player
                    .blockers
                    .iter()
                    .any(|blocker| blocker == "organization_status_review")
                {
                    blockers.insert(AhlPreseasonFactBlocker::OrganizationStatus);
                }
                if player
                    .blockers
                    .iter()
                    .any(|blocker| blocker == "waiver_clearance")
                {
                    blockers.insert(AhlPreseasonFactBlocker::WaiverClearance);
                }
                if player.primary_position.is_none()
                    || !player
                        .primary_position
                        .is_some_and(|position| player.eligible_positions.contains(&position))
                {
                    blockers.insert(AhlPreseasonFactBlocker::ExactPosition);
                }
                if player.projected_score.is_none() {
                    blockers.insert(AhlPreseasonFactBlocker::ProjectedScore);
                }
                if player.position_group != AhlPreseasonPositionGroup::Goalie {
                    if professional_games_at_season_start.is_none() {
                        blockers.insert(AhlPreseasonFactBlocker::ProfessionalGames);
                    }
                    if development_rule_qualified.is_none() {
                        blockers.insert(AhlPreseasonFactBlocker::DevelopmentRuleQualification);
                    }
                }
            }
            let blockers = blockers.into_iter().collect::<Vec<_>>();
            for blocker in &blockers {
                *blocker_counts.entry(*blocker).or_default() += 1;
            }
            players.push(AhlPreseasonFactsPlayerRow {
                nhl_player_id: player.nhl_player_id,
                display_name: player.display_name.clone(),
                status,
                origins: player.origins.clone(),
                position_group: player.position_group,
                primary_position: player.primary_position,
                eligible_positions: player.eligible_positions.clone(),
                projected_score: player.projected_score,
                prospect: None,
                recall_readiness: None,
                assigned_to_affiliate: None,
                waiver_cleared: player.waiver_exempt.filter(|waiver_exempt| *waiver_exempt),
                review_source_urls: Vec::new(),
                review_note: None,
                reviewer: None,
                reviewed_at: None,
                professional_games_at_season_start,
                development_rule_qualified,
                blockers,
            });
        }
        players.sort_by(|left, right| {
            left.status
                .ordinal()
                .cmp(&right.status.ordinal())
                .then_with(|| left.position_group.cmp(&right.position_group))
                .then_with(|| left.display_name.cmp(&right.display_name))
        });
        let counts = team_counts(&players);
        team_workboards.push(AhlPreseasonFactsTeamView {
            nhl_team: team.nhl_team.clone(),
            ahl_team: team.ahl_team.clone(),
            source_urls: team.source_urls.clone(),
            counts,
            players,
        });
    }
    team_workboards.sort_by(|left, right| left.nhl_team.cmp(&right.nhl_team));
    let candidates = team_workboards
        .iter()
        .map(|team| team.counts.candidates)
        .sum();
    let facts_ready_candidates = team_workboards
        .iter()
        .map(|team| team.counts.facts_ready_candidates)
        .sum();
    let mut view = AhlPreseasonLeagueFactsWorkboardView {
        schema: AHL_PRESEASON_LEAGUE_FACTS_WORKBOARD_SCHEMA.to_owned(),
        prior_season: rollover.prior_season,
        target_season: rollover.target_season,
        professional_game_policy_id: professional_games.policy_id.clone(),
        professional_game_policy_authority: format!(
            "{:?}",
            professional_games.policy_authority_status
        )
        .to_ascii_lowercase(),
        professional_game_threshold: professional_games.threshold,
        source_fingerprint: String::new(),
        teams: team_workboards.len(),
        candidates,
        facts_ready_candidates,
        blocker_counts,
        team_workboards,
        disclosures: vec![
            "This workboard composes evidence gaps; it does not assign any player to an affiliate.".to_owned(),
            "Every viable preseason candidate requires explicit assignment, prospect-status, and recall-readiness authority. Waiver exposure is not clearance.".to_owned(),
            "Skater development-rule qualification remains blocked until the professional-game policy is final; goaltenders are outside the dressed-skater development rule.".to_owned(),
        ],
    };
    view.source_fingerprint = fingerprint_workboard(&view)?;
    Ok(view)
}

pub fn apply_ahl_preseason_league_facts_overlay(
    workboard: &AhlPreseasonLeagueFactsWorkboardView,
    overlay: &AhlPreseasonLeagueFactsOverlay,
) -> Result<AhlPreseasonLeagueFactsApplicationView, AhlFeedError> {
    validate_workboard(workboard)?;
    validate_overlay(workboard, overlay)?;
    let overlay_fingerprint = fingerprint_overlay(overlay)?;
    let source_workboard_fingerprint = workboard.source_fingerprint.clone();
    let mut applied = workboard.clone();
    let locations = applied
        .team_workboards
        .iter()
        .enumerate()
        .flat_map(|(team_index, team)| {
            team.players
                .iter()
                .enumerate()
                .filter_map(move |(player_index, player)| {
                    player.nhl_player_id.map(|player_id| {
                        (
                            (team.nhl_team.clone(), player_id),
                            (team_index, player_index),
                        )
                    })
                })
        })
        .collect::<BTreeMap<_, _>>();
    for fact in &overlay.rows {
        let (team_index, player_index) = locations
            .get(&(fact.nhl_team.clone(), fact.nhl_player_id))
            .copied()
            .ok_or_else(|| {
                AhlFeedError::Validation(format!(
                    "preseason facts overlay references unknown player {} for {}",
                    fact.nhl_player_id, fact.nhl_team
                ))
            })?;
        let player = &mut applied.team_workboards[team_index].players[player_index];
        if player.status != AhlPreseasonFactsCandidateStatus::Candidate {
            return Err(AhlFeedError::Validation(format!(
                "preseason facts overlay can only review active candidates; {} for {} is {:?}",
                fact.nhl_player_id, fact.nhl_team, player.status
            )));
        }
        apply_player_fact(
            player,
            fact,
            overlay.reviewer.as_deref().unwrap_or_default(),
            overlay.reviewed_at.as_deref().unwrap_or_default(),
        )?;
    }
    recompute_workboard(&mut applied)?;
    applied.disclosures.push(format!(
        "Reviewed facts overlay {} applied by {} at {}; fields absent from the overlay remain blocked.",
        overlay_fingerprint,
        overlay.reviewer.as_deref().unwrap_or_default(),
        overlay.reviewed_at.as_deref().unwrap_or_default()
    ));
    applied.source_fingerprint = fingerprint_workboard(&applied)?;
    Ok(AhlPreseasonLeagueFactsApplicationView {
        schema: AHL_PRESEASON_LEAGUE_FACTS_APPLICATION_SCHEMA.to_owned(),
        source_workboard_fingerprint,
        overlay_fingerprint,
        prior_season: applied.prior_season,
        target_season: applied.target_season,
        rows_applied: overlay.rows.len(),
        candidates: applied.candidates,
        facts_ready_candidates: applied.facts_ready_candidates,
        blocker_counts: applied.blocker_counts.clone(),
        workboard: applied,
        disclosures: vec![
            "Application clears only explicitly reviewed facts and never creates an AHL assignment from a camp outcome.".to_owned(),
            "A reviewed not-assigned decision removes that player from the candidate pool; it is not rewritten as another-league or departed status.".to_owned(),
        ],
    })
}

pub fn build_ahl_preseason_league_facts_overlay_draft(
    workboard: &AhlPreseasonLeagueFactsWorkboardView,
) -> Result<AhlPreseasonLeagueFactsOverlay, AhlFeedError> {
    validate_workboard(workboard)?;
    let rows = workboard
        .team_workboards
        .iter()
        .flat_map(|team| {
            team.players.iter().filter_map(|player| {
                (player.status == AhlPreseasonFactsCandidateStatus::Candidate)
                    .then_some(player.nhl_player_id)
                    .flatten()
                    .map(|nhl_player_id| AhlPreseasonPlayerFactsOverlayRow {
                        nhl_team: team.nhl_team.clone(),
                        nhl_player_id,
                        primary_position: None,
                        eligible_positions: None,
                        projected_score: None,
                        prospect: None,
                        recall_readiness: None,
                        assigned_to_affiliate: None,
                        waiver_cleared: None,
                        source_urls: Vec::new(),
                        note: String::new(),
                    })
            })
        })
        .collect();
    Ok(AhlPreseasonLeagueFactsOverlay {
        schema: AHL_PRESEASON_LEAGUE_FACTS_OVERLAY_SCHEMA.to_owned(),
        prior_season: workboard.prior_season,
        target_season: workboard.target_season,
        workboard_fingerprint: workboard.source_fingerprint.clone(),
        draft: true,
        reviewer: None,
        reviewed_at: None,
        rows,
    })
}

pub fn build_ahl_preseason_league_projection_inputs(
    application: &AhlPreseasonLeagueFactsApplicationView,
    rule: &AhlDevelopmentRuleInput,
) -> Result<AhlPreseasonLeagueProjectionInputsView, AhlFeedError> {
    validate_facts_application(application)?;
    if application.workboard.professional_game_policy_authority != "final"
        || rule.professional_game_threshold != application.workboard.professional_game_threshold
        || rule.dressed_skaters != 18
        || rule.minimum_development_skaters > rule.dressed_skaters
        || reqwest::Url::parse(&rule.source_url)
            .ok()
            .is_none_or(|url| !matches!(url.scheme(), "http" | "https"))
        || DateTime::parse_from_rfc3339(&rule.checked_at).is_err()
    {
        return Err(AhlFeedError::Validation(
            "preseason projection lowering requires matching final professional-game and valid AHL dressed-roster rule authority"
                .to_owned(),
        ));
    }
    let facts_application_fingerprint = fingerprint_application(application)?;
    let mut inputs = Vec::new();
    let mut failures = Vec::new();
    for team in &application.workboard.team_workboards {
        let mut blocker_counts = BTreeMap::new();
        for player in &team.players {
            for blocker in &player.blockers {
                *blocker_counts.entry(*blocker).or_default() += 1;
            }
        }
        if team
            .players
            .iter()
            .any(|player| player.status == AhlPreseasonFactsCandidateStatus::IdentityBlocked)
            || !blocker_counts.is_empty()
        {
            failures.push(AhlPreseasonLeagueProjectionInputFailure {
                nhl_team: team.nhl_team.clone(),
                ahl_team: team.ahl_team.clone(),
                reason: "team retains identity or player-facts blockers".to_owned(),
                blocker_counts,
            });
            continue;
        }
        let mut source_urls = team.source_urls.iter().cloned().collect::<BTreeSet<_>>();
        source_urls.insert(rule.source_url.clone());
        let mut reviewed_at = Vec::new();
        let mut players = Vec::new();
        for player in team.players.iter().filter(|player| {
            player.status == AhlPreseasonFactsCandidateStatus::Candidate
                && player.assigned_to_affiliate == Some(true)
        }) {
            source_urls.extend(player.review_source_urls.iter().cloned());
            reviewed_at.extend(player.reviewed_at.iter().cloned());
            let (Some(player_id), Some(primary_position), Some(projected_score), Some(prospect)) = (
                player.nhl_player_id,
                player.primary_position,
                player.projected_score,
                player.prospect,
            ) else {
                return Err(AhlFeedError::Validation(format!(
                    "facts-ready player {} has incomplete lowering fields",
                    player.display_name
                )));
            };
            players.push(AhlAffiliatePlayerInput {
                player_id,
                display_name: player.display_name.clone(),
                primary_position,
                eligible_positions: player.eligible_positions.clone(),
                projected_score,
                prospect,
                recall_readiness: player.recall_readiness,
                professional_games_at_season_start: player.professional_games_at_season_start,
                development_rule_qualified: player.development_rule_qualified,
                assigned_to_affiliate: true,
                waiver_required: false,
                source_league: "AHL preseason projection".to_owned(),
            });
        }
        players.sort_by(|left, right| {
            left.player_id
                .cmp(&right.player_id)
                .then_with(|| left.display_name.cmp(&right.display_name))
        });
        reviewed_at.sort();
        let input = AhlAffiliateProjectionInput {
            nhl_team: team.nhl_team.clone(),
            ahl_team: team.ahl_team.clone(),
            season: application.target_season,
            rule: rule.clone(),
            pool_authority: AhlRosterPoolAuthority {
                kind: AhlRosterPoolAuthorityKind::PreseasonProjection,
                as_of: reviewed_at.last().cloned(),
                source_urls: source_urls.into_iter().collect(),
                note: Some(format!(
                    "Lowered from facts application {} and result workboard {}",
                    facts_application_fingerprint, application.workboard.source_fingerprint
                )),
            },
            players,
        };
        match build_ahl_affiliate_projection(&input) {
            Ok(_) => inputs.push(input),
            Err(reason) => failures.push(AhlPreseasonLeagueProjectionInputFailure {
                nhl_team: team.nhl_team.clone(),
                ahl_team: team.ahl_team.clone(),
                reason,
                blocker_counts: BTreeMap::new(),
            }),
        }
    }
    inputs.sort_by(|left, right| left.nhl_team.cmp(&right.nhl_team));
    failures.sort_by(|left, right| left.nhl_team.cmp(&right.nhl_team));
    Ok(AhlPreseasonLeagueProjectionInputsView {
        schema: AHL_PRESEASON_LEAGUE_PROJECTION_INPUTS_SCHEMA.to_owned(),
        target_season: application.target_season,
        facts_application_fingerprint,
        teams_requested: application.workboard.teams,
        teams_built: inputs.len(),
        inputs,
        failures,
        disclosures: vec![
            "Only fully reviewed assigned rows lower into preseason affiliate inputs; omitted or blocked candidates remain named team failures.".to_owned(),
            "Every emitted input has already passed the canonical 12F/6D/2G and AHL development-rule projection builder.".to_owned(),
        ],
    })
}

fn validate_facts_application(
    application: &AhlPreseasonLeagueFactsApplicationView,
) -> Result<(), AhlFeedError> {
    validate_workboard(&application.workboard)?;
    if application.schema != AHL_PRESEASON_LEAGUE_FACTS_APPLICATION_SCHEMA
        || application.source_workboard_fingerprint.trim().is_empty()
        || application.overlay_fingerprint.trim().is_empty()
        || application.prior_season != application.workboard.prior_season
        || application.target_season != application.workboard.target_season
        || application.candidates != application.workboard.candidates
        || application.facts_ready_candidates != application.workboard.facts_ready_candidates
        || application.blocker_counts != application.workboard.blocker_counts
    {
        return Err(AhlFeedError::Validation(
            "preseason facts application is incomplete or inconsistent with its result workboard"
                .to_owned(),
        ));
    }
    Ok(())
}

fn fingerprint_application(
    application: &AhlPreseasonLeagueFactsApplicationView,
) -> Result<String, AhlFeedError> {
    let bytes = serde_json::to_vec(application).map_err(|error| {
        AhlFeedError::Validation(format!("serialize preseason facts application: {error}"))
    })?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn validate_workboard(
    workboard: &AhlPreseasonLeagueFactsWorkboardView,
) -> Result<(), AhlFeedError> {
    if workboard.schema != AHL_PRESEASON_LEAGUE_FACTS_WORKBOARD_SCHEMA
        || workboard.teams != workboard.team_workboards.len()
        || workboard.source_fingerprint.trim().is_empty()
        || workboard.source_fingerprint != fingerprint_workboard(workboard)?
    {
        return Err(AhlFeedError::Validation(
            "preseason facts workboard is mismatched, incomplete, or fingerprint-invalid"
                .to_owned(),
        ));
    }
    let mut teams = BTreeSet::new();
    let mut players = BTreeSet::new();
    if workboard.team_workboards.iter().any(|team| {
        team.nhl_team.trim().is_empty()
            || !teams.insert(team.nhl_team.as_str())
            || team.players.iter().any(|player| {
                player
                    .nhl_player_id
                    .is_some_and(|id| !players.insert((team.nhl_team.as_str(), id)))
            })
    }) {
        return Err(AhlFeedError::Validation(
            "preseason facts workboard contains empty or duplicate team/player identities"
                .to_owned(),
        ));
    }
    Ok(())
}

fn validate_overlay(
    workboard: &AhlPreseasonLeagueFactsWorkboardView,
    overlay: &AhlPreseasonLeagueFactsOverlay,
) -> Result<(), AhlFeedError> {
    let reviewed_at_valid = overlay
        .reviewed_at
        .as_deref()
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .is_some();
    if overlay.schema != AHL_PRESEASON_LEAGUE_FACTS_OVERLAY_SCHEMA
        || overlay.prior_season != workboard.prior_season
        || overlay.target_season != workboard.target_season
        || overlay.workboard_fingerprint != workboard.source_fingerprint
        || overlay.draft
        || overlay
            .reviewer
            .as_deref()
            .is_none_or(|reviewer| reviewer.trim().is_empty())
        || !reviewed_at_valid
        || overlay.rows.is_empty()
    {
        return Err(AhlFeedError::Validation(
            "preseason facts overlay must be finalized and bound to the exact workboard".to_owned(),
        ));
    }
    let mut identities = BTreeSet::new();
    if overlay.rows.iter().any(|row| {
        row.nhl_team.trim().is_empty()
            || row.nhl_player_id == 0
            || row.note.trim().is_empty()
            || row.source_urls.is_empty()
            || row.source_urls.iter().any(|source| {
                reqwest::Url::parse(source)
                    .ok()
                    .is_none_or(|url| !matches!(url.scheme(), "http" | "https"))
            })
            || !identities.insert((row.nhl_team.as_str(), row.nhl_player_id))
            || row.projected_score.is_some_and(|score| !score.is_finite())
            || row.recall_readiness.is_some_and(|readiness| {
                !readiness.is_finite() || !(0.0..=1.0).contains(&readiness)
            })
            || row.eligible_positions.as_ref().is_some_and(|positions| {
                positions.is_empty()
                    || positions
                        .iter()
                        .enumerate()
                        .any(|(index, position)| positions[index + 1..].contains(position))
            })
            || matches!(row.waiver_cleared, Some(false))
                && matches!(row.assigned_to_affiliate, Some(true))
    }) {
        return Err(AhlFeedError::Validation(
            "preseason facts overlay contains invalid, duplicate, contradictory, or unsourced rows"
                .to_owned(),
        ));
    }
    Ok(())
}

fn apply_player_fact(
    player: &mut AhlPreseasonFactsPlayerRow,
    fact: &AhlPreseasonPlayerFactsOverlayRow,
    reviewer: &str,
    reviewed_at: &str,
) -> Result<(), AhlFeedError> {
    player.review_source_urls = fact.source_urls.clone();
    player.review_note = Some(fact.note.clone());
    player.reviewer = Some(reviewer.to_owned());
    player.reviewed_at = Some(reviewed_at.to_owned());
    merge_exact(
        &mut player.primary_position,
        fact.primary_position,
        "primary position",
    )?;
    if let Some(positions) = &fact.eligible_positions {
        if !player.eligible_positions.is_empty() && player.eligible_positions != *positions {
            return Err(AhlFeedError::Validation(format!(
                "preseason facts overlay conflicts with eligible positions for {}",
                player.display_name
            )));
        }
        player.eligible_positions = positions.clone();
    }
    if player.primary_position.is_some_and(|position| {
        !player.eligible_positions.is_empty() && !player.eligible_positions.contains(&position)
    }) {
        return Err(AhlFeedError::Validation(format!(
            "preseason facts overlay omits primary eligibility for {}",
            player.display_name
        )));
    }
    if let Some(score) = fact.projected_score {
        if player
            .projected_score
            .is_some_and(|current| (current - score).abs() > 1e-9)
        {
            return Err(AhlFeedError::Validation(format!(
                "preseason facts overlay conflicts with projected score for {}",
                player.display_name
            )));
        }
        player.projected_score = Some(score);
    }
    merge_exact(&mut player.prospect, fact.prospect, "prospect status")?;
    merge_exact(
        &mut player.recall_readiness,
        fact.recall_readiness,
        "recall readiness",
    )?;
    merge_exact(
        &mut player.assigned_to_affiliate,
        fact.assigned_to_affiliate,
        "assignment",
    )?;
    merge_exact(
        &mut player.waiver_cleared,
        fact.waiver_cleared,
        "waiver clearance",
    )?;
    if player.assigned_to_affiliate == Some(false) {
        player.status = AhlPreseasonFactsCandidateStatus::NotAssigned;
        player.blockers.clear();
        return Ok(());
    }
    let has_exact_position = player
        .primary_position
        .is_some_and(|position| player.eligible_positions.contains(&position));
    let has_projected_score = player.projected_score.is_some();
    let has_prospect_status = player.prospect.is_some();
    let has_recall_readiness = player.recall_readiness.is_some();
    let has_assignment = player.assigned_to_affiliate.is_some();
    let has_waiver_clearance = player.waiver_cleared == Some(true);
    clear_if(
        &mut player.blockers,
        AhlPreseasonFactBlocker::ExactPosition,
        has_exact_position,
    );
    clear_if(
        &mut player.blockers,
        AhlPreseasonFactBlocker::ProjectedScore,
        has_projected_score,
    );
    clear_if(
        &mut player.blockers,
        AhlPreseasonFactBlocker::ProspectStatus,
        has_prospect_status,
    );
    clear_if(
        &mut player.blockers,
        AhlPreseasonFactBlocker::RecallReadiness,
        has_recall_readiness,
    );
    clear_if(
        &mut player.blockers,
        AhlPreseasonFactBlocker::AssignmentAuthority,
        has_assignment,
    );
    clear_if(
        &mut player.blockers,
        AhlPreseasonFactBlocker::WaiverClearance,
        has_waiver_clearance,
    );
    Ok(())
}

fn merge_exact<T: Copy + PartialEq>(
    current: &mut Option<T>,
    supplied: Option<T>,
    label: &str,
) -> Result<(), AhlFeedError> {
    if let Some(supplied) = supplied {
        if current.is_some_and(|current| current != supplied) {
            return Err(AhlFeedError::Validation(format!(
                "preseason facts overlay conflicts with existing {label}"
            )));
        }
        *current = Some(supplied);
    }
    Ok(())
}

fn clear_if(
    blockers: &mut Vec<AhlPreseasonFactBlocker>,
    blocker: AhlPreseasonFactBlocker,
    predicate: bool,
) {
    if predicate {
        blockers.retain(|candidate| *candidate != blocker);
    }
}

fn recompute_workboard(
    workboard: &mut AhlPreseasonLeagueFactsWorkboardView,
) -> Result<(), AhlFeedError> {
    let mut blocker_counts = BTreeMap::new();
    for team in &mut workboard.team_workboards {
        for player in &team.players {
            for blocker in &player.blockers {
                *blocker_counts.entry(*blocker).or_default() += 1;
            }
        }
        team.counts = team_counts(&team.players);
    }
    workboard.candidates = workboard
        .team_workboards
        .iter()
        .map(|team| team.counts.candidates)
        .sum();
    workboard.facts_ready_candidates = workboard
        .team_workboards
        .iter()
        .map(|team| team.counts.facts_ready_candidates)
        .sum();
    workboard.blocker_counts = blocker_counts;
    workboard.source_fingerprint.clear();
    Ok(())
}

fn fingerprint_workboard(
    workboard: &AhlPreseasonLeagueFactsWorkboardView,
) -> Result<String, AhlFeedError> {
    let mut canonical = workboard.clone();
    canonical.source_fingerprint.clear();
    canonical
        .team_workboards
        .sort_by(|left, right| left.nhl_team.cmp(&right.nhl_team));
    for team in &mut canonical.team_workboards {
        team.players.sort_by(|left, right| {
            left.nhl_player_id
                .cmp(&right.nhl_player_id)
                .then_with(|| left.display_name.cmp(&right.display_name))
        });
    }
    let bytes = serde_json::to_vec(&canonical).map_err(|error| {
        AhlFeedError::Validation(format!("serialize preseason facts workboard: {error}"))
    })?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn fingerprint_overlay(overlay: &AhlPreseasonLeagueFactsOverlay) -> Result<String, AhlFeedError> {
    let mut canonical = overlay.clone();
    canonical.rows.sort_by(|left, right| {
        left.nhl_team
            .cmp(&right.nhl_team)
            .then_with(|| left.nhl_player_id.cmp(&right.nhl_player_id))
    });
    for row in &mut canonical.rows {
        row.source_urls.sort();
    }
    let bytes = serde_json::to_vec(&canonical).map_err(|error| {
        AhlFeedError::Validation(format!("serialize preseason facts overlay: {error}"))
    })?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

impl AhlPreseasonFactsCandidateStatus {
    fn ordinal(self) -> u8 {
        match self {
            Self::Candidate => 0,
            Self::IdentityBlocked => 1,
            Self::NotAssigned => 2,
            Self::ProjectedNhlRoster => 3,
            Self::Departed => 4,
            Self::OtherLeague => 5,
        }
    }
}

fn team_counts(players: &[AhlPreseasonFactsPlayerRow]) -> AhlPreseasonFactsTeamCounts {
    let candidates = players
        .iter()
        .filter(|player| player.status == AhlPreseasonFactsCandidateStatus::Candidate)
        .count();
    let count_blocker = |blocker| {
        players
            .iter()
            .filter(|player| player.blockers.contains(&blocker))
            .count()
    };
    AhlPreseasonFactsTeamCounts {
        players: players.len(),
        candidates,
        facts_ready_candidates: players
            .iter()
            .filter(|player| {
                player.status == AhlPreseasonFactsCandidateStatus::Candidate
                    && player.blockers.is_empty()
            })
            .count(),
        not_assigned: players
            .iter()
            .filter(|player| player.status == AhlPreseasonFactsCandidateStatus::NotAssigned)
            .count(),
        projected_nhl_roster: players
            .iter()
            .filter(|player| player.status == AhlPreseasonFactsCandidateStatus::ProjectedNhlRoster)
            .count(),
        explicit_departures: players
            .iter()
            .filter(|player| {
                matches!(
                    player.status,
                    AhlPreseasonFactsCandidateStatus::Departed
                        | AhlPreseasonFactsCandidateStatus::OtherLeague
                )
            })
            .count(),
        identity_blocked: count_blocker(AhlPreseasonFactBlocker::IdentityReview),
        missing_assignment_authority: count_blocker(AhlPreseasonFactBlocker::AssignmentAuthority),
        missing_organization_status: count_blocker(AhlPreseasonFactBlocker::OrganizationStatus),
        missing_waiver_clearance: count_blocker(AhlPreseasonFactBlocker::WaiverClearance),
        missing_exact_position: count_blocker(AhlPreseasonFactBlocker::ExactPosition),
        missing_projected_score: count_blocker(AhlPreseasonFactBlocker::ProjectedScore),
        missing_prospect_status: count_blocker(AhlPreseasonFactBlocker::ProspectStatus),
        missing_recall_readiness: count_blocker(AhlPreseasonFactBlocker::RecallReadiness),
        missing_professional_games: count_blocker(AhlPreseasonFactBlocker::ProfessionalGames),
        missing_development_rule_qualification: count_blocker(
            AhlPreseasonFactBlocker::DevelopmentRuleQualification,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ahl_professional_games::{
            AhlProfessionalGameLedgerView, AhlProfessionalGamePlayerRow,
            AhlProfessionalGamePolicyAuthority,
        },
        ahl_rollover::{
            AhlPreseasonLeagueRolloverView, AhlPreseasonRolloverCountsView,
            AhlPreseasonRolloverPlayerView, AhlPreseasonRolloverView,
        },
    };

    fn rollover() -> AhlPreseasonLeagueRolloverView {
        AhlPreseasonLeagueRolloverView {
            schema: AHL_PRESEASON_LEAGUE_ROLLOVER_SCHEMA.to_owned(),
            prior_season: 20252026,
            target_season: 20262027,
            teams_requested: 1,
            teams_built: 1,
            teams_projection_ready: 0,
            rollovers: vec![AhlPreseasonRolloverView {
                schema: crate::ahl_rollover::AHL_PRESEASON_ROLLOVER_SCHEMA.to_owned(),
                nhl_team: "NYR".to_owned(),
                ahl_team: "Hartford Wolf Pack".to_owned(),
                prior_season: 20252026,
                target_season: 20262027,
                as_of: "2026-07-28".to_owned(),
                source_urls: vec!["https://example.com/camp".to_owned()],
                counts: AhlPreseasonRolloverCountsView {
                    prior_roster_players: 1,
                    current_camp_players: 1,
                    reconciled_players: 1,
                    unresolved_prior_identities: 0,
                    prior_players_needing_organization_review: 0,
                    waiver_gated_candidates: 0,
                    projectable_forwards: 1,
                    projectable_defensemen: 0,
                    projectable_goalies: 0,
                    forwards_needed: 11,
                    defensemen_needed: 6,
                    goalies_needed: 2,
                    projection_ready: false,
                },
                players: vec![AhlPreseasonRolloverPlayerView {
                    nhl_player_id: Some(1),
                    prior_provider_player_id: Some("p1".to_owned()),
                    display_name: "Player One".to_owned(),
                    position_group: AhlPreseasonPositionGroup::Forward,
                    primary_position: Some(Position::LeftWing),
                    eligible_positions: vec![Position::LeftWing, Position::Center],
                    origins: vec![
                        AhlPreseasonRolloverOrigin::PriorAffiliate,
                        AhlPreseasonRolloverOrigin::CurrentCamp,
                    ],
                    identity_reviewed: true,
                    organization_decision: None,
                    camp_make_probability: Some(0.1),
                    camp_cut_probability: Some(0.9),
                    modal_nhl_roster: false,
                    waiver_exempt: Some(true),
                    projected_score: Some(42.0),
                    projectable_affiliate_candidate: true,
                    blockers: Vec::new(),
                }],
                disclosures: Vec::new(),
            }],
            failures: Vec::new(),
            disclosures: Vec::new(),
        }
    }

    fn ledger(authority: AhlProfessionalGamePolicyAuthority) -> AhlProfessionalGameLedgerView {
        AhlProfessionalGameLedgerView {
            schema: AHL_PROFESSIONAL_GAME_LEDGER_SCHEMA.to_owned(),
            policy_id: "policy.v1".to_owned(),
            policy_authority_status: authority,
            prior_season: 20252026,
            target_season: 20262027,
            as_of: "2026-07-28".to_owned(),
            threshold: 260,
            career_store_fetched_at: "2026-07-28T00:00:00Z".to_owned(),
            source_fingerprint: "sha256:test".to_owned(),
            canonical_players: 1,
            complete_players: 1,
            missing_histories: 0,
            unresolved_players: 0,
            players: vec![AhlProfessionalGamePlayerRow {
                nhl_player_id: 1,
                display_name: "Player One".to_owned(),
                affiliate_appearances: 1,
                professional_games_at_season_start: Some(100),
                within_game_threshold: Some(true),
                birth_date: Some("2000-01-01".to_owned()),
                age_at_policy_cutoff: Some(26),
                automatically_age_qualified: Some(false),
                development_rule_qualified: (authority
                    == AhlProfessionalGamePolicyAuthority::Final)
                    .then_some(true),
                included_leagues: Vec::new(),
                exempted_european_elite_leagues: Vec::new(),
                excluded_leagues: Vec::new(),
                unresolved_professional_leagues: Vec::new(),
                blockers: Vec::new(),
            }],
            disclosures: Vec::new(),
        }
    }

    #[test]
    fn workboard_preserves_positions_and_names_every_missing_authority() {
        let board = build_ahl_preseason_league_facts_workboard(
            &rollover(),
            &ledger(AhlProfessionalGamePolicyAuthority::Provisional),
        )
        .unwrap();
        assert_eq!(board.teams, 1);
        assert_eq!(board.candidates, 1);
        assert_eq!(board.facts_ready_candidates, 0);
        let row = &board.team_workboards[0].players[0];
        assert_eq!(
            row.eligible_positions,
            [Position::LeftWing, Position::Center]
        );
        assert_eq!(row.professional_games_at_season_start, Some(100));
        assert!(row
            .blockers
            .contains(&AhlPreseasonFactBlocker::AssignmentAuthority));
        assert!(row
            .blockers
            .contains(&AhlPreseasonFactBlocker::ProspectStatus));
        assert!(row
            .blockers
            .contains(&AhlPreseasonFactBlocker::RecallReadiness));
        assert!(row
            .blockers
            .contains(&AhlPreseasonFactBlocker::DevelopmentRuleQualification));
        assert!(!row
            .blockers
            .contains(&AhlPreseasonFactBlocker::ExactPosition));
    }

    #[test]
    fn final_rule_authority_removes_only_its_own_blocker() {
        let board = build_ahl_preseason_league_facts_workboard(
            &rollover(),
            &ledger(AhlProfessionalGamePolicyAuthority::Final),
        )
        .unwrap();
        let row = &board.team_workboards[0].players[0];
        assert_eq!(row.development_rule_qualified, Some(true));
        assert!(!row
            .blockers
            .contains(&AhlPreseasonFactBlocker::DevelopmentRuleQualification));
        assert!(row
            .blockers
            .contains(&AhlPreseasonFactBlocker::AssignmentAuthority));
    }

    fn overlay(
        board: &AhlPreseasonLeagueFactsWorkboardView,
        assigned_to_affiliate: bool,
    ) -> AhlPreseasonLeagueFactsOverlay {
        AhlPreseasonLeagueFactsOverlay {
            schema: AHL_PRESEASON_LEAGUE_FACTS_OVERLAY_SCHEMA.to_owned(),
            prior_season: board.prior_season,
            target_season: board.target_season,
            workboard_fingerprint: board.source_fingerprint.clone(),
            draft: false,
            reviewer: Some("facts-reviewer".to_owned()),
            reviewed_at: Some("2026-07-28T12:00:00Z".to_owned()),
            rows: vec![AhlPreseasonPlayerFactsOverlayRow {
                nhl_team: "NYR".to_owned(),
                nhl_player_id: 1,
                primary_position: None,
                eligible_positions: None,
                projected_score: None,
                prospect: Some(false),
                recall_readiness: Some(0.7),
                assigned_to_affiliate: Some(assigned_to_affiliate),
                waiver_cleared: None,
                source_urls: vec!["https://example.com/assignment".to_owned()],
                note: "Reviewed assignment and organization facts".to_owned(),
            }],
        }
    }

    #[test]
    fn finalized_overlay_clears_only_explicit_matching_blockers() {
        let board = build_ahl_preseason_league_facts_workboard(
            &rollover(),
            &ledger(AhlProfessionalGamePolicyAuthority::Final),
        )
        .unwrap();
        let applied =
            apply_ahl_preseason_league_facts_overlay(&board, &overlay(&board, true)).unwrap();
        let player = &applied.workboard.team_workboards[0].players[0];
        assert_eq!(player.prospect, Some(false));
        assert_eq!(player.recall_readiness, Some(0.7));
        assert_eq!(player.assigned_to_affiliate, Some(true));
        assert!(player.blockers.is_empty());
        assert_eq!(applied.facts_ready_candidates, 1);
        assert_ne!(
            applied.source_workboard_fingerprint,
            applied.workboard.source_fingerprint
        );
    }

    #[test]
    fn reviewed_not_assigned_decision_removes_candidate_without_relabeling() {
        let board = build_ahl_preseason_league_facts_workboard(
            &rollover(),
            &ledger(AhlProfessionalGamePolicyAuthority::Provisional),
        )
        .unwrap();
        let applied =
            apply_ahl_preseason_league_facts_overlay(&board, &overlay(&board, false)).unwrap();
        let team = &applied.workboard.team_workboards[0];
        assert_eq!(
            team.players[0].status,
            AhlPreseasonFactsCandidateStatus::NotAssigned
        );
        assert!(team.players[0].blockers.is_empty());
        assert_eq!(team.players[0].prospect, Some(false));
        assert_eq!(team.players[0].recall_readiness, Some(0.7));
        assert_eq!(team.counts.candidates, 0);
        assert_eq!(team.counts.not_assigned, 1);
    }

    #[test]
    fn overlay_rejects_stale_workboard_binding_and_draft_authority() {
        let board = build_ahl_preseason_league_facts_workboard(
            &rollover(),
            &ledger(AhlProfessionalGamePolicyAuthority::Final),
        )
        .unwrap();
        let mut facts = overlay(&board, true);
        facts.workboard_fingerprint = "sha256:stale".to_owned();
        assert!(apply_ahl_preseason_league_facts_overlay(&board, &facts).is_err());
        facts.workboard_fingerprint = board.source_fingerprint.clone();
        facts.draft = true;
        assert!(apply_ahl_preseason_league_facts_overlay(&board, &facts).is_err());
    }

    #[test]
    fn overlay_draft_exactly_covers_canonical_candidates_without_claiming_facts() {
        let board = build_ahl_preseason_league_facts_workboard(
            &rollover(),
            &ledger(AhlProfessionalGamePolicyAuthority::Final),
        )
        .unwrap();
        let draft = build_ahl_preseason_league_facts_overlay_draft(&board).unwrap();
        assert!(draft.draft);
        assert_eq!(draft.workboard_fingerprint, board.source_fingerprint);
        assert_eq!(draft.rows.len(), 1);
        assert_eq!(draft.rows[0].nhl_player_id, 1);
        assert!(draft.rows[0].source_urls.is_empty());
        assert!(draft.rows[0].assigned_to_affiliate.is_none());
    }

    fn facts_ready_application() -> AhlPreseasonLeagueFactsApplicationView {
        let board = build_ahl_preseason_league_facts_workboard(
            &rollover(),
            &ledger(AhlProfessionalGamePolicyAuthority::Final),
        )
        .unwrap();
        let mut application =
            apply_ahl_preseason_league_facts_overlay(&board, &overlay(&board, true)).unwrap();
        let template = application.workboard.team_workboards[0].players[0].clone();
        let mut players = Vec::new();
        for id in 1..=20 {
            let mut player = template.clone();
            player.nhl_player_id = Some(id);
            player.display_name = format!("Player {id}");
            player.position_group = if id <= 12 {
                AhlPreseasonPositionGroup::Forward
            } else if id <= 18 {
                AhlPreseasonPositionGroup::Defense
            } else {
                AhlPreseasonPositionGroup::Goalie
            };
            player.primary_position = Some(if id <= 4 {
                Position::Center
            } else if id <= 12 {
                Position::LeftWing
            } else if id <= 18 {
                Position::Defense
            } else {
                Position::Goalie
            });
            player.eligible_positions = if id <= 12 {
                vec![player.primary_position.unwrap(), Position::Center]
            } else {
                vec![player.primary_position.unwrap()]
            };
            player.projected_score = Some(100.0 - f64::from(id));
            player.prospect = Some(true);
            player.recall_readiness = Some(0.5);
            player.assigned_to_affiliate = Some(true);
            player.waiver_cleared = Some(true);
            player.professional_games_at_season_start = (id <= 18).then_some(100);
            player.development_rule_qualified = (id <= 18).then_some(true);
            player.blockers.clear();
            players.push(player);
        }
        application.workboard.team_workboards[0].players = players;
        recompute_workboard(&mut application.workboard).unwrap();
        application.workboard.source_fingerprint =
            fingerprint_workboard(&application.workboard).unwrap();
        application.candidates = application.workboard.candidates;
        application.facts_ready_candidates = application.workboard.facts_ready_candidates;
        application.blocker_counts = application.workboard.blocker_counts.clone();
        application
    }

    #[test]
    fn facts_ready_team_lowers_through_canonical_affiliate_builder() {
        let application = facts_ready_application();
        let view = build_ahl_preseason_league_projection_inputs(
            &application,
            &AhlDevelopmentRuleInput::default(),
        )
        .unwrap();
        assert_eq!(view.teams_requested, 1);
        assert_eq!(view.teams_built, 1);
        assert!(view.failures.is_empty());
        assert_eq!(view.inputs[0].players.len(), 20);
        assert_eq!(
            view.inputs[0].pool_authority.kind,
            AhlRosterPoolAuthorityKind::PreseasonProjection
        );
    }

    #[test]
    fn lowering_retains_team_failure_when_roster_shape_is_incomplete() {
        let mut application = facts_ready_application();
        application.workboard.team_workboards[0]
            .players
            .retain(|player| player.nhl_player_id != Some(20));
        recompute_workboard(&mut application.workboard).unwrap();
        application.workboard.source_fingerprint =
            fingerprint_workboard(&application.workboard).unwrap();
        application.candidates = application.workboard.candidates;
        application.facts_ready_candidates = application.workboard.facts_ready_candidates;
        application.blocker_counts = application.workboard.blocker_counts.clone();
        let view = build_ahl_preseason_league_projection_inputs(
            &application,
            &AhlDevelopmentRuleInput::default(),
        )
        .unwrap();
        assert_eq!(view.teams_built, 0);
        assert_eq!(view.failures.len(), 1);
        assert!(view.failures[0].reason.contains("two assigned goalies"));
    }

    #[test]
    fn lowering_rejects_nonfinal_rule_authority() {
        let mut application = facts_ready_application();
        application.workboard.professional_game_policy_authority = "provisional".to_owned();
        application.workboard.source_fingerprint =
            fingerprint_workboard(&application.workboard).unwrap();
        assert!(build_ahl_preseason_league_projection_inputs(
            &application,
            &AhlDevelopmentRuleInput::default()
        )
        .is_err());
    }
}
