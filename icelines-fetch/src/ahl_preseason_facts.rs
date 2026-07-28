//! League-wide preseason affiliate facts workboard.
//!
//! This is a composition/readiness artifact, not an assignment model. It
//! merges the rollover candidate pool with professional-game evidence and
//! names every authority still required before an AHL projection can exist.

use std::collections::{BTreeMap, BTreeSet};

use icelines_core::Position;
use serde::{Deserialize, Serialize};

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
    pub professional_games_at_season_start: Option<u32>,
    pub development_rule_qualified: Option<bool>,
    pub blockers: Vec<AhlPreseasonFactBlocker>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlPreseasonFactsTeamCounts {
    pub players: usize,
    pub candidates: usize,
    pub facts_ready_candidates: usize,
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
    pub teams: usize,
    pub candidates: usize,
    pub facts_ready_candidates: usize,
    pub blocker_counts: BTreeMap<AhlPreseasonFactBlocker, usize>,
    pub team_workboards: Vec<AhlPreseasonFactsTeamView>,
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
    Ok(AhlPreseasonLeagueFactsWorkboardView {
        schema: AHL_PRESEASON_LEAGUE_FACTS_WORKBOARD_SCHEMA.to_owned(),
        prior_season: rollover.prior_season,
        target_season: rollover.target_season,
        professional_game_policy_id: professional_games.policy_id.clone(),
        professional_game_policy_authority: format!(
            "{:?}",
            professional_games.policy_authority_status
        )
        .to_ascii_lowercase(),
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
    })
}

impl AhlPreseasonFactsCandidateStatus {
    fn ordinal(self) -> u8 {
        match self {
            Self::Candidate => 0,
            Self::IdentityBlocked => 1,
            Self::ProjectedNhlRoster => 2,
            Self::Departed => 3,
            Self::OtherLeague => 4,
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
}
