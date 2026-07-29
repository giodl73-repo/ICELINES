//! Source-bound all-league AHL prior-performance value ledger.

use std::collections::{BTreeMap, BTreeSet};

use icelines_core::{
    estimate_ahl_goalie_value, estimate_ahl_skater_value, AhlPlayerValueEstimate,
    AhlPlayerValuePolicy, AhlPlayerValuePositionGroup, Position,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    ahl::{
        AhlFeedError, AhlIdentityLeagueCrosswalkView, AhlIdentityReviewStatus,
        AhlRosterStatsSnapshot, AHL_IDENTITY_LEAGUE_CROSSWALK_SCHEMA, AHL_ROSTER_STATS_SCHEMA,
    },
    ahl_preseason_facts::{
        fingerprint_workboard, recompute_workboard, validate_workboard, AhlPreseasonFactBlocker,
        AhlPreseasonFactsCandidateStatus, AhlPreseasonLeagueFactsWorkboardView,
    },
};

pub const AHL_PLAYER_VALUE_LEDGER_SCHEMA: &str = "ahl_player_value_ledger.v1";
pub const AHL_PLAYER_VALUE_APPLICATION_SCHEMA: &str = "ahl_player_value_application.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlPlayerValueLedgerRow {
    pub nhl_player_id: u32,
    pub display_name: String,
    pub provider_player_ids: Vec<String>,
    pub estimate: AhlPlayerValueEstimate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlPlayerValueLedgerView {
    pub schema: String,
    pub prior_season: u32,
    pub policy: AhlPlayerValuePolicy,
    pub snapshot_fetched_at: String,
    pub source_urls: Vec<String>,
    pub players_scored: usize,
    pub source_fingerprint: String,
    pub players: Vec<AhlPlayerValueLedgerRow>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlPlayerValueApplicationView {
    pub schema: String,
    pub prior_season: u32,
    pub target_season: u32,
    pub source_workboard_fingerprint: String,
    pub value_ledger_fingerprint: String,
    pub rows_applied: usize,
    pub candidates_without_value: usize,
    pub workboard: AhlPreseasonLeagueFactsWorkboardView,
    pub disclosures: Vec<String>,
}

#[derive(Default)]
struct Aggregate {
    display_name: String,
    provider_ids: BTreeSet<String>,
    group: Option<AhlPlayerValuePositionGroup>,
    games: u32,
    points: u32,
    shots_against: u32,
    saves: u32,
}

pub fn build_ahl_player_value_ledger(
    snapshot: &AhlRosterStatsSnapshot,
    crosswalk: &AhlIdentityLeagueCrosswalkView,
    policy: &AhlPlayerValuePolicy,
) -> Result<AhlPlayerValueLedgerView, AhlFeedError> {
    snapshot.validate()?;
    if snapshot.schema != AHL_ROSTER_STATS_SCHEMA
        || crosswalk.schema != AHL_IDENTITY_LEAGUE_CROSSWALK_SCHEMA
        || snapshot.season != crosswalk.season
        || snapshot.provider != crosswalk.provider
        || crosswalk.teams != crosswalk.crosswalks.len()
    {
        return Err(AhlFeedError::Validation(
            "AHL player-value ledger requires matching official snapshot and reviewed league crosswalk"
                .to_owned(),
        ));
    }

    let mut identities = BTreeMap::<String, (u32, String)>::new();
    for team in &crosswalk.crosswalks {
        for row in &team.rows {
            if row.review_status != AhlIdentityReviewStatus::Reviewed {
                continue;
            }
            let Some(nhl_player_id) = row.nhl_player_id else {
                return Err(AhlFeedError::Validation(format!(
                    "reviewed AHL identity {} has no NHL player ID",
                    row.provider_player_id
                )));
            };
            let display_name = row
                .nhl_display_name
                .clone()
                .unwrap_or_else(|| row.ahl_display_name.clone());
            if identities
                .insert(
                    row.provider_player_id.clone(),
                    (nhl_player_id, display_name.clone()),
                )
                .is_some_and(|prior| prior != (nhl_player_id, display_name))
            {
                return Err(AhlFeedError::Validation(format!(
                    "AHL provider player {} maps inconsistently across teams",
                    row.provider_player_id
                )));
            }
        }
    }

    let mut aggregates = BTreeMap::<u32, Aggregate>::new();
    for team in &snapshot.teams {
        for row in &team.skaters {
            let Some((nhl_player_id, display_name)) = identities.get(&row.provider_player_id)
            else {
                continue;
            };
            let group = if row.position.eq_ignore_ascii_case("D") {
                AhlPlayerValuePositionGroup::Defense
            } else {
                AhlPlayerValuePositionGroup::Forward
            };
            add_observation(
                aggregates.entry(*nhl_player_id).or_default(),
                display_name,
                &row.provider_player_id,
                group,
                row.games_played,
                row.points,
                0,
                0,
            )?;
        }
        for row in &team.goalies {
            let Some((nhl_player_id, display_name)) = identities.get(&row.provider_player_id)
            else {
                continue;
            };
            add_observation(
                aggregates.entry(*nhl_player_id).or_default(),
                display_name,
                &row.provider_player_id,
                AhlPlayerValuePositionGroup::Goalie,
                row.games_played,
                0,
                row.shots_against,
                row.saves,
            )?;
        }
    }

    let mut players = aggregates
        .into_iter()
        .map(|(nhl_player_id, aggregate)| {
            let group = aggregate.group.ok_or_else(|| {
                AhlFeedError::Validation("AHL player-value aggregate has no group".to_owned())
            })?;
            let estimate = if group == AhlPlayerValuePositionGroup::Goalie {
                estimate_ahl_goalie_value(
                    policy,
                    aggregate.games,
                    aggregate.shots_against,
                    aggregate.saves,
                )
            } else {
                estimate_ahl_skater_value(policy, group, aggregate.games, aggregate.points)
            }
            .map_err(AhlFeedError::Validation)?;
            Ok(AhlPlayerValueLedgerRow {
                nhl_player_id,
                display_name: aggregate.display_name,
                provider_player_ids: aggregate.provider_ids.into_iter().collect(),
                estimate,
            })
        })
        .collect::<Result<Vec<_>, AhlFeedError>>()?;
    players.sort_by_key(|row| row.nhl_player_id);
    let mut view = AhlPlayerValueLedgerView {
        schema: AHL_PLAYER_VALUE_LEDGER_SCHEMA.to_owned(),
        prior_season: snapshot.season,
        policy: policy.clone(),
        snapshot_fetched_at: snapshot.fetched_at.clone(),
        source_urls: vec![snapshot.source_url.clone(), snapshot.roster_source_url.clone()],
        players_scored: players.len(),
        source_fingerprint: String::new(),
        players,
        disclosures: vec![
            "Scores are confidence-weighted prior-season AHL performance estimates for within-position affiliate ordering; they are not NHL equivalencies or calibrated forecasts.".to_owned(),
            "Skater rates use position-group priors and AHL schedule pace; goalie rates use save percentage with shot-based evidence strength.".to_owned(),
        ],
    };
    view.source_fingerprint = fingerprint_ledger(&view)?;
    Ok(view)
}

#[allow(clippy::too_many_arguments)]
fn add_observation(
    aggregate: &mut Aggregate,
    display_name: &str,
    provider_id: &str,
    group: AhlPlayerValuePositionGroup,
    games: u32,
    points: u32,
    shots_against: u32,
    saves: u32,
) -> Result<(), AhlFeedError> {
    if aggregate.group.is_some_and(|prior| prior != group) {
        return Err(AhlFeedError::Validation(format!(
            "AHL player {display_name} has conflicting position groups"
        )));
    }
    aggregate.display_name = display_name.to_owned();
    aggregate.provider_ids.insert(provider_id.to_owned());
    aggregate.group = Some(group);
    aggregate.games = checked_add(aggregate.games, games, display_name)?;
    aggregate.points = checked_add(aggregate.points, points, display_name)?;
    aggregate.shots_against = checked_add(aggregate.shots_against, shots_against, display_name)?;
    aggregate.saves = checked_add(aggregate.saves, saves, display_name)?;
    Ok(())
}

fn checked_add(left: u32, right: u32, name: &str) -> Result<u32, AhlFeedError> {
    left.checked_add(right).ok_or_else(|| {
        AhlFeedError::Validation(format!("AHL player-value totals overflow for {name}"))
    })
}

pub fn apply_ahl_player_value_ledger(
    workboard: &AhlPreseasonLeagueFactsWorkboardView,
    ledger: &AhlPlayerValueLedgerView,
) -> Result<AhlPlayerValueApplicationView, AhlFeedError> {
    validate_workboard(workboard)?;
    if ledger.schema != AHL_PLAYER_VALUE_LEDGER_SCHEMA
        || ledger.prior_season != workboard.prior_season
        || ledger.players_scored != ledger.players.len()
    {
        return Err(AhlFeedError::Validation(
            "AHL player-value application requires an intact matching ledger".to_owned(),
        ));
    }
    let actual_fingerprint = fingerprint_ledger(ledger)?;
    if actual_fingerprint != ledger.source_fingerprint {
        return Err(AhlFeedError::Validation(format!(
            "AHL player-value ledger fingerprint mismatch: expected {}, recomputed {}",
            ledger.source_fingerprint, actual_fingerprint
        )));
    }
    let values = ledger
        .players
        .iter()
        .map(|row| (row.nhl_player_id, row))
        .collect::<BTreeMap<_, _>>();
    if values.len() != ledger.players.len() {
        return Err(AhlFeedError::Validation(
            "AHL player-value ledger contains duplicate NHL player IDs".to_owned(),
        ));
    }
    let source_workboard_fingerprint = workboard.source_fingerprint.clone();
    let mut applied = workboard.clone();
    let mut rows_applied = 0usize;
    for team in &mut applied.team_workboards {
        for player in &mut team.players {
            if player.status != AhlPreseasonFactsCandidateStatus::Candidate
                || player.projected_score.is_some()
            {
                continue;
            }
            let Some(value) = player.nhl_player_id.and_then(|id| values.get(&id).copied()) else {
                continue;
            };
            let expected_group = match player.primary_position {
                Some(Position::Goalie) => AhlPlayerValuePositionGroup::Goalie,
                Some(Position::Defense) => AhlPlayerValuePositionGroup::Defense,
                Some(_) => AhlPlayerValuePositionGroup::Forward,
                None => continue,
            };
            if expected_group != value.estimate.position_group {
                continue;
            }
            player.projected_score = Some(value.estimate.projected_score);
            player.projected_score_method = Some(value.estimate.method_version.clone());
            player.projected_score_confidence = Some(value.estimate.evidence_confidence);
            player.projected_score_sample_games = Some(value.estimate.sample_games);
            player.projected_score_source_fingerprint = Some(ledger.source_fingerprint.clone());
            player
                .blockers
                .retain(|blocker| *blocker != AhlPreseasonFactBlocker::ProjectedScore);
            rows_applied += 1;
        }
    }
    recompute_workboard(&mut applied)?;
    applied.disclosures.push(format!(
        "AHL player-value ledger {} filled {} previously missing scores; unmatched or position-conflicting candidates remain blocked.",
        ledger.source_fingerprint, rows_applied
    ));
    applied.source_fingerprint = fingerprint_workboard(&applied)?;
    let candidates_without_value = applied
        .blocker_counts
        .get(&AhlPreseasonFactBlocker::ProjectedScore)
        .copied()
        .unwrap_or_default();
    Ok(AhlPlayerValueApplicationView {
        schema: AHL_PLAYER_VALUE_APPLICATION_SCHEMA.to_owned(),
        prior_season: applied.prior_season,
        target_season: applied.target_season,
        source_workboard_fingerprint,
        value_ledger_fingerprint: ledger.source_fingerprint.clone(),
        rows_applied,
        candidates_without_value,
        workboard: applied,
        disclosures: vec![
            "Only missing projected_score facts are filled; all assignment, status, waiver, prospect, recall, and development-rule blockers retain their original authority.".to_owned(),
        ],
    })
}

fn fingerprint_ledger(view: &AhlPlayerValueLedgerView) -> Result<String, AhlFeedError> {
    let mut digest = Sha256::new();
    hash_string(&mut digest, &view.schema);
    digest.update(view.prior_season.to_le_bytes());
    hash_string(&mut digest, &view.policy.schema);
    hash_string(&mut digest, &view.policy.method_version);
    digest.update(view.policy.skater_schedule_games.to_le_bytes());
    digest.update(view.policy.skater_prior_games.to_le_bytes());
    hash_f64(&mut digest, view.policy.forward_prior_points_per_game);
    hash_f64(&mut digest, view.policy.defense_prior_points_per_game);
    digest.update(view.policy.goalie_prior_shots.to_le_bytes());
    hash_f64(&mut digest, view.policy.goalie_prior_save_percentage);
    hash_string(&mut digest, &view.snapshot_fetched_at);
    hash_strings(&mut digest, &view.source_urls);
    digest.update((view.players_scored as u64).to_le_bytes());
    digest.update((view.players.len() as u64).to_le_bytes());
    for player in &view.players {
        digest.update(player.nhl_player_id.to_le_bytes());
        hash_string(&mut digest, &player.display_name);
        hash_strings(&mut digest, &player.provider_player_ids);
        hash_string(&mut digest, &player.estimate.method_version);
        digest.update([match player.estimate.position_group {
            AhlPlayerValuePositionGroup::Forward => 0,
            AhlPlayerValuePositionGroup::Defense => 1,
            AhlPlayerValuePositionGroup::Goalie => 2,
        }]);
        hash_f64(&mut digest, player.estimate.projected_score);
        hash_f64(&mut digest, player.estimate.evidence_confidence);
        digest.update(player.estimate.sample_games.to_le_bytes());
        hash_f64(&mut digest, player.estimate.observed_rate);
        hash_f64(&mut digest, player.estimate.prior_rate);
        hash_f64(&mut digest, player.estimate.shrunk_rate);
    }
    hash_strings(&mut digest, &view.disclosures);
    Ok(format!("sha256:{:x}", digest.finalize()))
}

fn hash_strings(digest: &mut Sha256, values: &[String]) {
    digest.update((values.len() as u64).to_le_bytes());
    for value in values {
        hash_string(digest, value);
    }
}

fn hash_string(digest: &mut Sha256, value: &str) {
    digest.update((value.len() as u64).to_le_bytes());
    digest.update(value.as_bytes());
}

fn hash_f64(digest: &mut Sha256, value: f64) {
    hash_string(digest, &format!("{value:.9}"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ahl::{
        AhlIdentityCrosswalkCounts, AhlIdentityCrosswalkRow, AhlIdentityCrosswalkView,
        AhlIdentityMatchBasis, AhlSkaterSeasonRow, AhlTeamRosterStats,
    };
    use crate::ahl_preseason_facts::{
        AhlPreseasonFactsPlayerRow, AhlPreseasonFactsTeamCounts, AhlPreseasonFactsTeamView,
        AHL_PRESEASON_LEAGUE_FACTS_WORKBOARD_SCHEMA,
    };
    use crate::ahl_rollover::AhlPreseasonPositionGroup;

    fn snapshot() -> AhlRosterStatsSnapshot {
        AhlRosterStatsSnapshot {
            schema: AHL_ROSTER_STATS_SCHEMA.to_owned(),
            season: 20252026,
            provider: "official_ahl".to_owned(),
            provider_season_id: "90".to_owned(),
            provider_season_name: "2025-26".to_owned(),
            fetched_at: "2026-07-28T12:00:00Z".to_owned(),
            source_url: "https://theahl.com/stats".to_owned(),
            roster_source_url: "https://theahl.com/stats/rosters".to_owned(),
            identity_note: "fixture".to_owned(),
            teams: vec![AhlTeamRosterStats {
                provider: "official_ahl".to_owned(),
                provider_team_id: "1".to_owned(),
                team_code: "HFD".to_owned(),
                team_name: "Hartford Wolf Pack".to_owned(),
                nickname: "Wolf Pack".to_owned(),
                division_id: "A".to_owned(),
                logo_url: String::new(),
                nhl_affiliate: Some("NYR".to_owned()),
                roster: Vec::new(),
                skaters: vec![AhlSkaterSeasonRow {
                    provider: "official_ahl".to_owned(),
                    provider_player_id: "p1".to_owned(),
                    name: "Test Player".to_owned(),
                    team_code: "HFD".to_owned(),
                    position: "C".to_owned(),
                    active: true,
                    rookie: true,
                    games_played: 50,
                    goals: 20,
                    assists: 30,
                    points: 50,
                    plus_minus: 5,
                    penalty_minutes: 10,
                    power_play_goals: 5,
                    short_handed_goals: 0,
                    shots: 100,
                }],
                goalies: Vec::new(),
                source_warnings: Vec::new(),
            }],
        }
    }

    fn crosswalk() -> AhlIdentityLeagueCrosswalkView {
        AhlIdentityLeagueCrosswalkView {
            schema: AHL_IDENTITY_LEAGUE_CROSSWALK_SCHEMA.to_owned(),
            season: 20252026,
            provider: "official_ahl".to_owned(),
            roster_fetched_at: "2026-07-28T12:00:00Z".to_owned(),
            candidates_checked_at: "2026-07-28T12:05:00Z".to_owned(),
            teams: 1,
            roster_appearances: 1,
            unique_provider_players: 1,
            crosswalks: vec![AhlIdentityCrosswalkView {
                schema: crate::ahl::AHL_IDENTITY_CROSSWALK_SCHEMA.to_owned(),
                season: 20252026,
                provider: "official_ahl".to_owned(),
                ahl_team: "Hartford Wolf Pack".to_owned(),
                nhl_affiliate: Some("NYR".to_owned()),
                roster_fetched_at: "2026-07-28T12:00:00Z".to_owned(),
                candidates_checked_at: "2026-07-28T12:05:00Z".to_owned(),
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
                    provider_player_id: "p1".to_owned(),
                    ahl_display_name: "Test Player".to_owned(),
                    ahl_birth_date: "2004-01-01".to_owned(),
                    match_basis: AhlIdentityMatchBasis::ExactNameAndBirthDate,
                    review_status: AhlIdentityReviewStatus::Reviewed,
                    nhl_player_id: Some(8479999),
                    nhl_display_name: Some("Test Player".to_owned()),
                    nhl_birth_date: Some("2004-01-01".to_owned()),
                    evidence_urls: vec!["https://www.nhl.com/player/8479999".to_owned()],
                    note: "fixture review".to_owned(),
                }],
                disclosures: Vec::new(),
            }],
            disclosures: Vec::new(),
        }
    }

    fn workboard() -> AhlPreseasonLeagueFactsWorkboardView {
        let mut view = AhlPreseasonLeagueFactsWorkboardView {
            schema: AHL_PRESEASON_LEAGUE_FACTS_WORKBOARD_SCHEMA.to_owned(),
            prior_season: 20252026,
            target_season: 20262027,
            professional_game_policy_id: "test-policy".to_owned(),
            professional_game_policy_authority: "final".to_owned(),
            professional_game_threshold: 260,
            source_fingerprint: String::new(),
            teams: 1,
            candidates: 0,
            facts_ready_candidates: 0,
            blocker_counts: BTreeMap::new(),
            team_workboards: vec![AhlPreseasonFactsTeamView {
                nhl_team: "NYR".to_owned(),
                ahl_team: "Hartford Wolf Pack".to_owned(),
                source_urls: vec!["https://theahl.com/stats".to_owned()],
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
                    nhl_player_id: Some(8479999),
                    display_name: "Test Player".to_owned(),
                    status: AhlPreseasonFactsCandidateStatus::Candidate,
                    origins: Vec::new(),
                    position_group: AhlPreseasonPositionGroup::Forward,
                    primary_position: Some(Position::Center),
                    eligible_positions: vec![Position::Center],
                    projected_score: None,
                    projected_score_method: None,
                    projected_score_confidence: None,
                    projected_score_sample_games: None,
                    projected_score_source_fingerprint: None,
                    prospect: None,
                    prospect_method: None,
                    prospect_source_fingerprint: None,
                    recall_readiness: None,
                    recall_readiness_method: None,
                    recall_readiness_confidence: None,
                    recall_readiness_coverage: None,
                    recall_readiness_source_fingerprint: None,
                    assigned_to_affiliate: None,
                    assignment_authority: None,
                    waiver_cleared: None,
                    review_source_urls: Vec::new(),
                    review_note: None,
                    reviewer: None,
                    reviewed_at: None,
                    professional_games_at_season_start: Some(50),
                    development_rule_qualified: Some(true),
                    blockers: vec![AhlPreseasonFactBlocker::ProjectedScore],
                }],
            }],
            disclosures: vec!["fixture".to_owned()],
        };
        recompute_workboard(&mut view).unwrap();
        view.source_fingerprint = fingerprint_workboard(&view).unwrap();
        view
    }

    #[test]
    fn ledger_scores_reviewed_identity_and_seals_source() {
        let ledger = build_ahl_player_value_ledger(
            &snapshot(),
            &crosswalk(),
            &AhlPlayerValuePolicy::default(),
        )
        .unwrap();
        assert_eq!(ledger.players_scored, 1);
        assert_eq!(ledger.players[0].nhl_player_id, 8479999);
        assert_eq!(
            ledger.players[0].estimate.position_group,
            AhlPlayerValuePositionGroup::Forward
        );
        assert!(ledger.players[0].estimate.evidence_confidence > 0.7);
        assert!(ledger.source_fingerprint.starts_with("sha256:"));
        let round_trip: AhlPlayerValueLedgerView =
            serde_json::from_str(&serde_json::to_string_pretty(&ledger).unwrap()).unwrap();
        assert_eq!(
            fingerprint_ledger(&round_trip).unwrap(),
            round_trip.source_fingerprint
        );
    }

    #[test]
    fn rejected_identity_is_not_scored() {
        let mut crosswalk = crosswalk();
        crosswalk.crosswalks[0].rows[0].review_status = AhlIdentityReviewStatus::Rejected;
        crosswalk.crosswalks[0].rows[0].nhl_player_id = None;
        let ledger = build_ahl_player_value_ledger(
            &snapshot(),
            &crosswalk,
            &AhlPlayerValuePolicy::default(),
        )
        .unwrap();
        assert!(ledger.players.is_empty());
    }

    #[test]
    fn invalid_policy_fails_before_publishing_ledger() {
        let mut policy = AhlPlayerValuePolicy::default();
        policy.method_version = "unknown".to_owned();
        assert!(build_ahl_player_value_ledger(&snapshot(), &crosswalk(), &policy).is_err());
    }

    #[test]
    fn application_clears_only_score_and_reseals_workboard() {
        let ledger = build_ahl_player_value_ledger(
            &snapshot(),
            &crosswalk(),
            &AhlPlayerValuePolicy::default(),
        )
        .unwrap();
        let application = apply_ahl_player_value_ledger(&workboard(), &ledger).unwrap();
        assert_eq!(application.rows_applied, 1);
        assert_eq!(application.candidates_without_value, 0);
        let player = &application.workboard.team_workboards[0].players[0];
        assert!(player.projected_score.is_some());
        assert!(player.blockers.is_empty());
        validate_workboard(&application.workboard).unwrap();
    }
}
