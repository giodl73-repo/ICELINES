//! Operational prospect-status authority for preseason affiliate candidates.

use std::collections::{BTreeMap, BTreeSet};

use icelines_core::{
    evaluate_organizational_prospect, CareerGameType, OrganizationalProspectPolicy,
    OrganizationalProspectStatusView,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    ahl::AhlFeedError,
    ahl_preseason_facts::{
        fingerprint_workboard, recompute_workboard, validate_workboard, AhlPreseasonFactBlocker,
        AhlPreseasonFactsCandidateStatus, AhlPreseasonLeagueFactsWorkboardView,
    },
    career_landing::CareerHistoryStore,
};

pub const AHL_PROSPECT_STATUS_LEDGER_SCHEMA: &str = "ahl_prospect_status_ledger.v1";
pub const AHL_PROSPECT_STATUS_APPLICATION_SCHEMA: &str = "ahl_prospect_status_application.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AhlProspectStatusUnavailableReason {
    MissingCanonicalIdentity,
    InsufficientOfficialEvidence,
    InvalidOfficialEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlProspectStatusLedgerRow {
    pub nhl_player_id: u32,
    pub display_name: String,
    pub status: OrganizationalProspectStatusView,
    pub source_urls: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlProspectStatusUnavailableRow {
    pub nhl_player_id: Option<u32>,
    pub display_name: String,
    pub nhl_teams: Vec<String>,
    pub reason: AhlProspectStatusUnavailableReason,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlProspectStatusLedgerView {
    pub schema: String,
    pub prior_season: u32,
    pub target_season: u32,
    pub workboard_fingerprint: String,
    pub career_store_fetched_at: String,
    pub policy: OrganizationalProspectPolicy,
    pub candidate_appearances: usize,
    pub candidates_requested: usize,
    pub candidates_classified: usize,
    pub candidates_unavailable: usize,
    pub source_fingerprint: String,
    pub players: Vec<AhlProspectStatusLedgerRow>,
    pub unavailable: Vec<AhlProspectStatusUnavailableRow>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlProspectStatusApplicationView {
    pub schema: String,
    pub prior_season: u32,
    pub target_season: u32,
    pub source_workboard_fingerprint: String,
    pub prospect_ledger_fingerprint: String,
    pub rows_applied: usize,
    pub candidates_without_prospect_status: usize,
    pub workboard: AhlPreseasonLeagueFactsWorkboardView,
    pub disclosures: Vec<String>,
}

pub fn build_ahl_prospect_status_ledger(
    workboard: &AhlPreseasonLeagueFactsWorkboardView,
    career_store: &CareerHistoryStore,
    policy: &OrganizationalProspectPolicy,
) -> Result<AhlProspectStatusLedgerView, AhlFeedError> {
    validate_workboard(workboard)?;
    if career_store.schema_version == 0
        || career_store.fetched_at.as_deref().is_none_or(str::is_empty)
    {
        return Err(AhlFeedError::Validation(
            "prospect-status ledger requires a dated official career store".to_owned(),
        ));
    }

    let mut appearances = BTreeMap::<u32, Vec<(String, String)>>::new();
    let mut unavailable = Vec::new();
    let mut candidate_appearances = 0usize;
    for team in &workboard.team_workboards {
        for player in &team.players {
            if player.status != AhlPreseasonFactsCandidateStatus::Candidate {
                continue;
            }
            candidate_appearances += 1;
            if let Some(player_id) = player.nhl_player_id {
                appearances
                    .entry(player_id)
                    .or_default()
                    .push((team.nhl_team.clone(), player.display_name.clone()));
            } else {
                unavailable.push(AhlProspectStatusUnavailableRow {
                    nhl_player_id: None,
                    display_name: player.display_name.clone(),
                    nhl_teams: vec![team.nhl_team.clone()],
                    reason: AhlProspectStatusUnavailableReason::MissingCanonicalIdentity,
                    detail: "Candidate has no canonical NHL player ID".to_owned(),
                });
            }
        }
    }

    let mut players = Vec::new();
    for (player_id, mut rows) in appearances {
        rows.sort();
        rows.dedup();
        let display_name = rows
            .first()
            .map(|row| row.1.clone())
            .unwrap_or_else(|| player_id.to_string());
        let teams = rows
            .iter()
            .map(|row| row.0.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let birth_date = career_store.birth_date(player_id);
        let nhl_games = career_store
            .get(player_id)
            .map(regular_season_nhl_games)
            .transpose()?;
        let status = match evaluate_organizational_prospect(policy, birth_date, nhl_games) {
            Ok(status) => status,
            Err(detail) => {
                unavailable.push(AhlProspectStatusUnavailableRow {
                    nhl_player_id: Some(player_id),
                    display_name,
                    nhl_teams: teams,
                    reason: AhlProspectStatusUnavailableReason::InvalidOfficialEvidence,
                    detail,
                });
                continue;
            }
        };
        if status.prospect.is_none() {
            unavailable.push(AhlProspectStatusUnavailableRow {
                nhl_player_id: Some(player_id),
                display_name,
                nhl_teams: teams,
                reason: AhlProspectStatusUnavailableReason::InsufficientOfficialEvidence,
                detail: format!(
                    "Eligibility needs age and NHL workload; birth_date={}, career_history={}",
                    birth_date.is_some(),
                    nhl_games.is_some()
                ),
            });
            continue;
        }
        players.push(AhlProspectStatusLedgerRow {
            nhl_player_id: player_id,
            display_name,
            status,
            source_urls: vec![format!(
                "https://api-web.nhle.com/v1/player/{player_id}/landing"
            )],
        });
    }
    players.sort_by_key(|row| row.nhl_player_id);
    unavailable.sort_by(|left, right| {
        left.nhl_player_id
            .cmp(&right.nhl_player_id)
            .then_with(|| left.display_name.cmp(&right.display_name))
    });
    let mut view = AhlProspectStatusLedgerView {
        schema: AHL_PROSPECT_STATUS_LEDGER_SCHEMA.to_owned(),
        prior_season: workboard.prior_season,
        target_season: workboard.target_season,
        workboard_fingerprint: workboard.source_fingerprint.clone(),
        career_store_fetched_at: career_store.fetched_at.clone().unwrap_or_default(),
        policy: policy.clone(),
        candidate_appearances,
        candidates_requested: players.len() + unavailable.len(),
        candidates_classified: players.len(),
        candidates_unavailable: unavailable.len(),
        source_fingerprint: String::new(),
        players,
        unavailable,
        disclosures: vec![
            "Prospect is an IceLines reserve-system population status based only on exact age and observed NHL regular-season workload; it is not NHL rookie, contract, waiver, assignment, or scouting status.".to_owned(),
            "Either observed graduation axis can establish non-prospect status. Prospect=true requires both age and NHL workload evidence. Status is canonical-player evidence and may apply to multiple unresolved organization appearances without resolving assignment.".to_owned(),
        ],
    };
    view.source_fingerprint = fingerprint_ledger(&view)?;
    Ok(view)
}

pub fn apply_ahl_prospect_status_ledger(
    workboard: &AhlPreseasonLeagueFactsWorkboardView,
    ledger: &AhlProspectStatusLedgerView,
) -> Result<AhlProspectStatusApplicationView, AhlFeedError> {
    validate_workboard(workboard)?;
    if ledger.schema != AHL_PROSPECT_STATUS_LEDGER_SCHEMA
        || ledger.prior_season != workboard.prior_season
        || ledger.target_season != workboard.target_season
        || ledger.workboard_fingerprint != workboard.source_fingerprint
        || ledger.candidate_appearances < ledger.candidates_requested
        || ledger.candidates_requested
            != ledger.candidates_classified + ledger.candidates_unavailable
        || ledger.candidates_classified != ledger.players.len()
        || ledger.candidates_unavailable != ledger.unavailable.len()
        || ledger.source_fingerprint != fingerprint_ledger(ledger)?
    {
        return Err(AhlFeedError::Validation(
            "prospect-status application requires an intact ledger bound to the exact workboard"
                .to_owned(),
        ));
    }
    let rows = ledger
        .players
        .iter()
        .map(|row| (row.nhl_player_id, row))
        .collect::<BTreeMap<_, _>>();
    if rows.len() != ledger.players.len() {
        return Err(AhlFeedError::Validation(
            "prospect-status ledger contains duplicate canonical player rows".to_owned(),
        ));
    }
    let source_workboard_fingerprint = workboard.source_fingerprint.clone();
    let mut applied = workboard.clone();
    let mut rows_applied = 0usize;
    for team in &mut applied.team_workboards {
        for player in &mut team.players {
            if player.status != AhlPreseasonFactsCandidateStatus::Candidate
                || player.prospect.is_some()
            {
                continue;
            }
            let Some(row) = player.nhl_player_id.and_then(|id| rows.get(&id).copied()) else {
                continue;
            };
            let prospect = row.status.prospect.ok_or_else(|| {
                AhlFeedError::Validation(
                    "classified prospect ledger row has no classification".to_owned(),
                )
            })?;
            player.prospect = Some(prospect);
            player.prospect_method = Some(row.status.method_version.clone());
            player.prospect_source_fingerprint = Some(ledger.source_fingerprint.clone());
            player
                .blockers
                .retain(|blocker| *blocker != AhlPreseasonFactBlocker::ProspectStatus);
            rows_applied += 1;
        }
    }
    recompute_workboard(&mut applied)?;
    applied.disclosures.push(format!(
        "Prospect-status ledger {} classified {} candidates; insufficient rows remain blocked.",
        ledger.source_fingerprint, rows_applied
    ));
    applied.source_fingerprint = fingerprint_workboard(&applied)?;
    let candidates_without_prospect_status = applied
        .blocker_counts
        .get(&AhlPreseasonFactBlocker::ProspectStatus)
        .copied()
        .unwrap_or_default();
    Ok(AhlProspectStatusApplicationView {
        schema: AHL_PROSPECT_STATUS_APPLICATION_SCHEMA.to_owned(),
        prior_season: applied.prior_season,
        target_season: applied.target_season,
        source_workboard_fingerprint,
        prospect_ledger_fingerprint: ledger.source_fingerprint.clone(),
        rows_applied,
        candidates_without_prospect_status,
        workboard: applied,
        disclosures: vec![
            "Only prospect_status is applied; recall, assignment, waiver, organization status, score, game, and development-rule authorities are unchanged.".to_owned(),
        ],
    })
}

fn regular_season_nhl_games(history: &icelines_core::CareerHistory) -> Result<u32, AhlFeedError> {
    history
        .stints
        .iter()
        .filter(|stint| {
            stint.game_type == CareerGameType::Regular
                && stint.league.as_str().eq_ignore_ascii_case("NHL")
        })
        .try_fold(0u32, |total, stint| {
            total.checked_add(stint.gp).ok_or_else(|| {
                AhlFeedError::Validation(format!(
                    "NHL workload overflow for player {}",
                    history.player_id
                ))
            })
        })
}

fn fingerprint_ledger(view: &AhlProspectStatusLedgerView) -> Result<String, AhlFeedError> {
    let mut canonical = view.clone();
    canonical.source_fingerprint.clear();
    canonical.players.sort_by_key(|row| row.nhl_player_id);
    canonical.unavailable.sort_by(|left, right| {
        left.nhl_player_id
            .cmp(&right.nhl_player_id)
            .then_with(|| left.display_name.cmp(&right.display_name))
    });
    let bytes = serde_json::to_vec(&canonical)
        .map_err(|error| AhlFeedError::Validation(error.to_string()))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_core::{CareerHistory, OrganizationalProspectPolicy, Position};

    use crate::{
        ahl_preseason_facts::{
            AhlPreseasonFactsPlayerRow, AhlPreseasonFactsTeamCounts, AhlPreseasonFactsTeamView,
            AHL_PRESEASON_LEAGUE_FACTS_WORKBOARD_SCHEMA,
        },
        ahl_rollover::AhlPreseasonPositionGroup,
    };

    fn policy() -> OrganizationalProspectPolicy {
        OrganizationalProspectPolicy {
            schema: icelines_core::ORGANIZATIONAL_PROSPECT_POLICY_SCHEMA.to_owned(),
            method_version: icelines_core::ORGANIZATIONAL_PROSPECT_METHOD.to_owned(),
            as_of_date: "2026-09-15".to_owned(),
            maximum_age: 24,
            maximum_nhl_regular_season_games: 50,
        }
    }

    fn workboard() -> AhlPreseasonLeagueFactsWorkboardView {
        let mut view = AhlPreseasonLeagueFactsWorkboardView {
            schema: AHL_PRESEASON_LEAGUE_FACTS_WORKBOARD_SCHEMA.to_owned(),
            prior_season: 20252026,
            target_season: 20262027,
            professional_game_policy_id: "test".to_owned(),
            professional_game_policy_authority: "provisional".to_owned(),
            professional_game_threshold: 260,
            source_fingerprint: String::new(),
            teams: 1,
            candidates: 0,
            facts_ready_candidates: 0,
            blocker_counts: BTreeMap::new(),
            team_workboards: vec![AhlPreseasonFactsTeamView {
                nhl_team: "NYR".to_owned(),
                ahl_team: "Hartford Wolf Pack".to_owned(),
                source_urls: Vec::new(),
                counts: AhlPreseasonFactsTeamCounts {
                    players: 0,
                    candidates: 0,
                    facts_ready_candidates: 0,
                    not_assigned: 0,
                    projected_nhl_roster: 0,
                    explicit_departures: 0,
                    identity_blocked: 0,
                    missing_assignment_authority: 0,
                    missing_organization_status: 0,
                    missing_waiver_clearance: 0,
                    missing_exact_position: 0,
                    missing_projected_score: 0,
                    missing_prospect_status: 0,
                    missing_recall_readiness: 0,
                    missing_professional_games: 0,
                    missing_development_rule_qualification: 0,
                },
                players: vec![AhlPreseasonFactsPlayerRow {
                    nhl_player_id: Some(8480001),
                    display_name: "Test Prospect".to_owned(),
                    status: AhlPreseasonFactsCandidateStatus::Candidate,
                    origins: Vec::new(),
                    position_group: AhlPreseasonPositionGroup::Forward,
                    primary_position: Some(Position::Center),
                    eligible_positions: vec![Position::Center],
                    projected_score: Some(40.0),
                    projected_score_method: None,
                    projected_score_confidence: None,
                    projected_score_sample_games: None,
                    projected_score_source_fingerprint: None,
                    prospect: None,
                    prospect_method: None,
                    prospect_source_fingerprint: None,
                    recall_readiness: None,
                    assigned_to_affiliate: None,
                    waiver_cleared: None,
                    review_source_urls: Vec::new(),
                    review_note: None,
                    reviewer: None,
                    reviewed_at: None,
                    professional_games_at_season_start: Some(20),
                    development_rule_qualified: None,
                    blockers: vec![
                        AhlPreseasonFactBlocker::ProspectStatus,
                        AhlPreseasonFactBlocker::RecallReadiness,
                    ],
                }],
            }],
            disclosures: vec!["fixture".to_owned()],
        };
        recompute_workboard(&mut view).unwrap();
        view.source_fingerprint = fingerprint_workboard(&view).unwrap();
        view
    }

    fn store() -> CareerHistoryStore {
        let mut store = CareerHistoryStore::new();
        store.fetched_at = Some("2026-07-28T12:00:00Z".to_owned());
        store.upsert_birth_date(8480001, "2004-01-01");
        store.upsert(CareerHistory {
            player_id: 8480001,
            stints: Vec::new(),
        });
        store
    }

    #[test]
    fn ledger_classifies_from_official_identity_and_workload() {
        let ledger = build_ahl_prospect_status_ledger(&workboard(), &store(), &policy()).unwrap();
        assert_eq!(ledger.candidates_requested, 1);
        assert_eq!(ledger.candidates_classified, 1);
        assert_eq!(ledger.players[0].status.prospect, Some(true));
        let round_trip: AhlProspectStatusLedgerView =
            serde_json::from_str(&serde_json::to_string_pretty(&ledger).unwrap()).unwrap();
        assert_eq!(
            fingerprint_ledger(&round_trip).unwrap(),
            round_trip.source_fingerprint
        );
    }

    #[test]
    fn application_clears_only_prospect_blocker_and_reseals() {
        let board = workboard();
        let ledger = build_ahl_prospect_status_ledger(&board, &store(), &policy()).unwrap();
        let application = apply_ahl_prospect_status_ledger(&board, &ledger).unwrap();
        let player = &application.workboard.team_workboards[0].players[0];
        assert_eq!(player.prospect, Some(true));
        assert!(!player
            .blockers
            .contains(&AhlPreseasonFactBlocker::ProspectStatus));
        assert!(player
            .blockers
            .contains(&AhlPreseasonFactBlocker::RecallReadiness));
        validate_workboard(&application.workboard).unwrap();
    }

    #[test]
    fn stale_ledger_is_rejected_after_workboard_changes() {
        let board = workboard();
        let ledger = build_ahl_prospect_status_ledger(&board, &store(), &policy()).unwrap();
        let mut changed = board;
        changed.disclosures.push("changed".to_owned());
        changed.source_fingerprint = fingerprint_workboard(&changed).unwrap();
        assert!(apply_ahl_prospect_status_ledger(&changed, &ledger).is_err());
    }

    #[test]
    fn canonical_status_applies_to_multiple_unresolved_organization_appearances() {
        let mut board = workboard();
        let mut second_team = board.team_workboards[0].clone();
        second_team.nhl_team = "SEA".to_owned();
        second_team.ahl_team = "Coachella Valley Firebirds".to_owned();
        board.team_workboards.push(second_team);
        board.teams = 2;
        recompute_workboard(&mut board).unwrap();
        board.source_fingerprint = fingerprint_workboard(&board).unwrap();

        let ledger = build_ahl_prospect_status_ledger(&board, &store(), &policy()).unwrap();
        assert_eq!(ledger.candidate_appearances, 2);
        assert_eq!(ledger.candidates_requested, 1);
        assert_eq!(ledger.candidates_classified, 1);

        let application = apply_ahl_prospect_status_ledger(&board, &ledger).unwrap();
        assert_eq!(application.rows_applied, 2);
        assert_eq!(application.candidates_without_prospect_status, 0);
        assert!(application
            .workboard
            .team_workboards
            .iter()
            .all(|team| team.players[0].prospect == Some(true)));
        assert_eq!(
            application.workboard.team_workboards[0].players[0].assigned_to_affiliate,
            None
        );
        assert_eq!(
            application.workboard.team_workboards[1].players[0].assigned_to_affiliate,
            None
        );
    }
}
