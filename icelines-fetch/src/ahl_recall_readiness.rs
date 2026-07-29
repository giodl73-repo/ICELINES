//! Confidence-aware recall-readiness authority for affiliate candidates.

use std::collections::{BTreeMap, BTreeSet};

use icelines_core::{
    empirical_midrank_percentiles, estimate_ahl_recall_readiness, AhlRecallReadinessEstimate,
    AhlRecallReadinessInput, AhlRecallReadinessPolicy, CareerGameType,
    TrainingCampLeagueForecastView, AHL_RECALL_READINESS_POLICY_SCHEMA,
    TRAINING_CAMP_FORECAST_SCHEMA, TRAINING_CAMP_LEAGUE_FORECAST_SCHEMA,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    ahl::AhlFeedError,
    ahl_preseason_facts::{
        fingerprint_workboard, recompute_workboard, validate_workboard, AhlPreseasonFactBlocker,
        AhlPreseasonFactsCandidateStatus, AhlPreseasonLeagueFactsWorkboardView,
    },
    ahl_rollover::AhlPreseasonPositionGroup,
    career_landing::CareerHistoryStore,
};

pub const AHL_RECALL_READINESS_LEDGER_SCHEMA: &str = "ahl_recall_readiness_ledger.v1";
pub const AHL_RECALL_READINESS_APPLICATION_SCHEMA: &str = "ahl_recall_readiness_application.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AhlRecallReadinessUnavailableReason {
    MissingCanonicalIdentity,
    ConflictingCandidateEvidence,
    InsufficientEvidence,
    InvalidEvidence,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlRecallReadinessLedgerRow {
    pub nhl_player_id: u32,
    pub display_name: String,
    pub nhl_teams: Vec<String>,
    pub position_group: AhlPreseasonPositionGroup,
    pub estimate: AhlRecallReadinessEstimate,
    pub source_urls: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlRecallReadinessUnavailableRow {
    pub nhl_player_id: Option<u32>,
    pub display_name: String,
    pub nhl_teams: Vec<String>,
    pub reason: AhlRecallReadinessUnavailableReason,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlRecallReadinessLedgerView {
    pub schema: String,
    pub prior_season: u32,
    pub target_season: u32,
    pub workboard_fingerprint: String,
    pub career_store_fetched_at: String,
    pub camp_forecast_fingerprint: String,
    pub policy: AhlRecallReadinessPolicy,
    pub candidate_appearances: usize,
    pub candidates_requested: usize,
    pub candidates_estimated: usize,
    pub candidates_unavailable: usize,
    pub source_fingerprint: String,
    pub players: Vec<AhlRecallReadinessLedgerRow>,
    pub unavailable: Vec<AhlRecallReadinessUnavailableRow>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlRecallReadinessApplicationView {
    pub schema: String,
    pub prior_season: u32,
    pub target_season: u32,
    pub source_workboard_fingerprint: String,
    pub readiness_ledger_fingerprint: String,
    pub rows_applied: usize,
    pub candidates_without_recall_readiness: usize,
    pub workboard: AhlPreseasonLeagueFactsWorkboardView,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone)]
struct CandidateEvidence {
    display_name: String,
    teams: BTreeSet<String>,
    source_urls: BTreeSet<String>,
    group: AhlPreseasonPositionGroup,
    value: Option<f64>,
    value_confidence: Option<f64>,
    value_from_camp: bool,
    invalid_detail: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct CampEvidence {
    projected_score: f64,
    gp_confidence: f64,
    make_probability: f64,
}

pub fn build_ahl_recall_readiness_ledger(
    workboard: &AhlPreseasonLeagueFactsWorkboardView,
    career_store: &CareerHistoryStore,
    camp_forecast: &TrainingCampLeagueForecastView,
    policy: &AhlRecallReadinessPolicy,
) -> Result<AhlRecallReadinessLedgerView, AhlFeedError> {
    validate_workboard(workboard)?;
    if policy.schema != AHL_RECALL_READINESS_POLICY_SCHEMA {
        return Err(AhlFeedError::Validation(
            "recall-readiness ledger requires a supported policy".to_owned(),
        ));
    }
    if career_store.schema_version == 0
        || career_store.fetched_at.as_deref().is_none_or(str::is_empty)
    {
        return Err(AhlFeedError::Validation(
            "recall-readiness ledger requires a dated official career store".to_owned(),
        ));
    }
    let (camp_by_id, camp_forecast_fingerprint) =
        validate_and_index_camp(camp_forecast, workboard.target_season)?;

    let mut candidates = BTreeMap::<u32, CandidateEvidence>::new();
    let mut unavailable = Vec::new();
    let mut candidate_appearances = 0usize;
    for team in &workboard.team_workboards {
        for player in &team.players {
            if player.status != AhlPreseasonFactsCandidateStatus::Candidate {
                continue;
            }
            candidate_appearances += 1;
            let Some(player_id) = player.nhl_player_id else {
                unavailable.push(AhlRecallReadinessUnavailableRow {
                    nhl_player_id: None,
                    display_name: player.display_name.clone(),
                    nhl_teams: vec![team.nhl_team.clone()],
                    reason: AhlRecallReadinessUnavailableReason::MissingCanonicalIdentity,
                    detail: "Candidate has no canonical NHL player ID".to_owned(),
                });
                continue;
            };
            let camp = camp_by_id.get(&player_id).copied();
            let (value, value_confidence, value_from_camp) =
                if let (Some(value), Some(confidence)) =
                    (player.projected_score, player.projected_score_confidence)
                {
                    (Some(value), Some(confidence), false)
                } else if let Some(camp) = camp.filter(|camp| {
                    player
                        .projected_score
                        .is_some_and(|score| close(score, camp.projected_score))
                }) {
                    (Some(camp.projected_score), Some(camp.gp_confidence), true)
                } else {
                    (None, None, false)
                };
            let entry = candidates
                .entry(player_id)
                .or_insert_with(|| CandidateEvidence {
                    display_name: player.display_name.clone(),
                    teams: BTreeSet::new(),
                    source_urls: BTreeSet::new(),
                    group: player.position_group,
                    value,
                    value_confidence,
                    value_from_camp,
                    invalid_detail: None,
                });
            entry.teams.insert(team.nhl_team.clone());
            entry.source_urls.extend(team.source_urls.iter().cloned());
            if entry.group != player.position_group {
                entry.invalid_detail = Some(
                    "Canonical player has conflicting position evidence across organization appearances"
                        .to_owned(),
                );
            } else if value.is_none() {
                // Absence on one rollover appearance does not conflict with
                // usable canonical evidence from another appearance.
            } else if entry.value.is_none() {
                entry.value = value;
                entry.value_confidence = value_confidence;
                entry.value_from_camp = value_from_camp;
            } else if entry.value_from_camp && !value_from_camp {
                entry.value = value;
                entry.value_confidence = value_confidence;
                entry.value_from_camp = false;
            } else if !entry.value_from_camp && value_from_camp {
                // The separately modeled prior-AHL value remains authoritative;
                // camp proximity can then contribute without duplicating it.
            } else if !same_optional_number(entry.value, value)
                || !same_optional_number(entry.value_confidence, value_confidence)
            {
                entry.invalid_detail = Some(
                    "Canonical player has conflicting same-authority value evidence across organization appearances"
                        .to_owned(),
                );
            }
        }
    }

    let mut cohorts = BTreeMap::<AhlPreseasonPositionGroup, Vec<(u32, f64)>>::new();
    for (player_id, evidence) in &candidates {
        if evidence.invalid_detail.is_none() && evidence.group != AhlPreseasonPositionGroup::Unknown
        {
            if let Some(value) = evidence.value {
                cohorts
                    .entry(evidence.group)
                    .or_default()
                    .push((*player_id, value));
            }
        }
    }
    let percentiles = cohorts
        .into_iter()
        .map(|(group, values)| {
            empirical_midrank_percentiles(&values)
                .map(|percentiles| (group, percentiles))
                .map_err(AhlFeedError::Validation)
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;

    let mut players = Vec::new();
    for (player_id, evidence) in candidates {
        let teams = evidence.teams.into_iter().collect::<Vec<_>>();
        if let Some(detail) = evidence.invalid_detail {
            unavailable.push(unavailable_row(
                player_id,
                evidence.display_name,
                teams,
                AhlRecallReadinessUnavailableReason::ConflictingCandidateEvidence,
                detail,
            ));
            continue;
        }
        if evidence.group == AhlPreseasonPositionGroup::Unknown {
            unavailable.push(unavailable_row(
                player_id,
                evidence.display_name,
                teams,
                AhlRecallReadinessUnavailableReason::InsufficientEvidence,
                "Candidate has no exact position group for value normalization".to_owned(),
            ));
            continue;
        }
        let value_percentile = percentiles
            .get(&evidence.group)
            .and_then(|rows| rows.get(&player_id))
            .copied();
        let nhl_games = career_store
            .get(player_id)
            .map(regular_season_nhl_games)
            .transpose()?;
        let camp_make_probability = (!evidence.value_from_camp)
            .then(|| camp_by_id.get(&player_id).map(|row| row.make_probability))
            .flatten();
        let estimate = estimate_ahl_recall_readiness(
            policy,
            &AhlRecallReadinessInput {
                value_percentile,
                value_evidence_confidence: evidence.value_confidence,
                nhl_regular_season_games: nhl_games,
                camp_make_probability,
            },
        )
        .map_err(AhlFeedError::Validation)?;
        if estimate.readiness_index.is_none() {
            unavailable.push(unavailable_row(
                player_id,
                evidence.display_name,
                teams,
                AhlRecallReadinessUnavailableReason::InsufficientEvidence,
                format!(
                    "Readiness coverage {:.3} is below policy minimum {:.3}",
                    estimate.coverage, policy.minimum_coverage
                ),
            ));
            continue;
        }
        let mut source_urls = evidence.source_urls.into_iter().collect::<Vec<_>>();
        source_urls.push(format!(
            "https://api-web.nhle.com/v1/player/{player_id}/landing"
        ));
        source_urls.sort();
        source_urls.dedup();
        players.push(AhlRecallReadinessLedgerRow {
            nhl_player_id: player_id,
            display_name: evidence.display_name,
            nhl_teams: teams,
            position_group: evidence.group,
            estimate,
            source_urls,
        });
    }
    players.sort_by_key(|row| row.nhl_player_id);
    unavailable.sort_by(|left, right| {
        left.nhl_player_id
            .cmp(&right.nhl_player_id)
            .then_with(|| left.display_name.cmp(&right.display_name))
    });
    let view = AhlRecallReadinessLedgerView {
        schema: AHL_RECALL_READINESS_LEDGER_SCHEMA.to_owned(),
        prior_season: workboard.prior_season,
        target_season: workboard.target_season,
        workboard_fingerprint: workboard.source_fingerprint.clone(),
        career_store_fetched_at: career_store.fetched_at.clone().unwrap_or_default(),
        camp_forecast_fingerprint,
        policy: policy.clone(),
        candidate_appearances,
        candidates_requested: players.len() + unavailable.len(),
        candidates_estimated: players.len(),
        candidates_unavailable: unavailable.len(),
        source_fingerprint: String::new(),
        players,
        unavailable,
        disclosures: vec![
            "Recall readiness is an evaluation index, not a calibrated recall or NHL-success probability.".to_owned(),
            "The index combines within-position player value, observed NHL workload, and camp proximity. Coverage and evidence confidence remain separate from the index.".to_owned(),
            "Canonical players may appear in multiple unresolved organizations; readiness is reused without resolving assignment or waiver status.".to_owned(),
        ],
    };
    let canonical_bytes =
        serde_json::to_vec(&view).map_err(|error| AhlFeedError::Validation(error.to_string()))?;
    let mut canonical: AhlRecallReadinessLedgerView = serde_json::from_slice(&canonical_bytes)
        .map_err(|error| AhlFeedError::Validation(error.to_string()))?;
    canonical.source_fingerprint = fingerprint_ledger(&canonical)?;
    Ok(canonical)
}

pub fn apply_ahl_recall_readiness_ledger(
    workboard: &AhlPreseasonLeagueFactsWorkboardView,
    ledger: &AhlRecallReadinessLedgerView,
) -> Result<AhlRecallReadinessApplicationView, AhlFeedError> {
    validate_workboard(workboard)?;
    let expected_fingerprint = fingerprint_ledger(ledger)?;
    let invalid_reason = if ledger.schema != AHL_RECALL_READINESS_LEDGER_SCHEMA {
        Some("unsupported ledger schema")
    } else if ledger.prior_season != workboard.prior_season
        || ledger.target_season != workboard.target_season
    {
        Some("season mismatch")
    } else if ledger.workboard_fingerprint != workboard.source_fingerprint {
        Some("workboard fingerprint mismatch")
    } else if ledger.candidate_appearances < ledger.candidates_requested {
        Some("candidate appearance count is smaller than canonical request count")
    } else if ledger.candidates_requested
        != ledger.candidates_estimated + ledger.candidates_unavailable
    {
        Some("candidate result counts do not reconcile")
    } else if ledger.candidates_estimated != ledger.players.len()
        || ledger.candidates_unavailable != ledger.unavailable.len()
    {
        Some("ledger row counts do not reconcile")
    } else if ledger.source_fingerprint != expected_fingerprint {
        return Err(AhlFeedError::Validation(format!(
            "recall-readiness application requires an intact ledger bound to the exact workboard: ledger fingerprint mismatch (stored {}, recomputed {})",
            ledger.source_fingerprint, expected_fingerprint
        )));
    } else {
        None
    };
    if let Some(reason) = invalid_reason {
        return Err(AhlFeedError::Validation(format!(
            "recall-readiness application requires an intact ledger bound to the exact workboard: {reason}"
        )));
    }
    let rows = ledger
        .players
        .iter()
        .map(|row| (row.nhl_player_id, row))
        .collect::<BTreeMap<_, _>>();
    if rows.len() != ledger.players.len() {
        return Err(AhlFeedError::Validation(
            "recall-readiness ledger contains duplicate canonical player rows".to_owned(),
        ));
    }
    let source_workboard_fingerprint = workboard.source_fingerprint.clone();
    let mut applied = workboard.clone();
    let mut rows_applied = 0usize;
    for team in &mut applied.team_workboards {
        for player in &mut team.players {
            if player.status != AhlPreseasonFactsCandidateStatus::Candidate
                || player.recall_readiness.is_some()
            {
                continue;
            }
            let Some(row) = player.nhl_player_id.and_then(|id| rows.get(&id).copied()) else {
                continue;
            };
            let readiness = row.estimate.readiness_index.ok_or_else(|| {
                AhlFeedError::Validation(
                    "estimated recall-readiness row has no readiness index".to_owned(),
                )
            })?;
            player.recall_readiness = Some(readiness);
            player.recall_readiness_method = Some(row.estimate.method_version.clone());
            player.recall_readiness_confidence = Some(row.estimate.evidence_confidence);
            player.recall_readiness_coverage = Some(row.estimate.coverage);
            player.recall_readiness_source_fingerprint = Some(ledger.source_fingerprint.clone());
            player
                .blockers
                .retain(|blocker| *blocker != AhlPreseasonFactBlocker::RecallReadiness);
            rows_applied += 1;
        }
    }
    recompute_workboard(&mut applied)?;
    applied.disclosures.push(format!(
        "Recall-readiness ledger {} applied to {} candidate appearances; insufficient rows remain blocked.",
        ledger.source_fingerprint, rows_applied
    ));
    applied.source_fingerprint = fingerprint_workboard(&applied)?;
    let candidates_without_recall_readiness = applied
        .blocker_counts
        .get(&AhlPreseasonFactBlocker::RecallReadiness)
        .copied()
        .unwrap_or_default();
    Ok(AhlRecallReadinessApplicationView {
        schema: AHL_RECALL_READINESS_APPLICATION_SCHEMA.to_owned(),
        prior_season: applied.prior_season,
        target_season: applied.target_season,
        source_workboard_fingerprint,
        readiness_ledger_fingerprint: ledger.source_fingerprint.clone(),
        rows_applied,
        candidates_without_recall_readiness,
        workboard: applied,
        disclosures: vec![
            "Only recall readiness and its method evidence are applied; assignment, waiver, organization status, score, prospect, game, and final-rule authorities are unchanged.".to_owned(),
        ],
    })
}

fn validate_and_index_camp(
    league: &TrainingCampLeagueForecastView,
    target_season: u32,
) -> Result<(BTreeMap<u32, CampEvidence>, String), AhlFeedError> {
    let forecast_count = league
        .teams
        .iter()
        .filter(|team| team.forecast.is_some())
        .count();
    if league.schema != TRAINING_CAMP_LEAGUE_FORECAST_SCHEMA
        || league.season != target_season
        || league.teams_requested != league.teams.len()
        || league.teams_simulated != forecast_count
        || league.teams_failed != league.teams.len().saturating_sub(forecast_count)
    {
        return Err(AhlFeedError::Validation(
            "recall readiness requires a matching training-camp league forecast".to_owned(),
        ));
    }
    let mut teams = BTreeSet::new();
    let mut players = BTreeMap::new();
    for team in &league.teams {
        if !teams.insert(team.team.clone()) {
            return Err(AhlFeedError::Validation(
                "training-camp league forecast contains duplicate teams".to_owned(),
            ));
        }
        let Some(forecast) = &team.forecast else {
            continue;
        };
        if forecast.schema != TRAINING_CAMP_FORECAST_SCHEMA
            || forecast.season != target_season
            || forecast.team != team.team
        {
            return Err(AhlFeedError::Validation(format!(
                "training-camp forecast axes do not match {}",
                team.team
            )));
        }
        for player in &forecast.players {
            if player.player_id == 0
                || !player.projected_score.is_finite()
                || !player.gp_confidence.is_finite()
                || !(0.0..=1.0).contains(&player.gp_confidence)
                || !player.make_probability.is_finite()
                || !(0.0..=1.0).contains(&player.make_probability)
                || !player.cut_probability.is_finite()
                || !close(player.make_probability + player.cut_probability, 1.0)
                || !close(
                    player.make_probability,
                    player.dressed_probability + player.healthy_scratch_probability,
                )
            {
                return Err(AhlFeedError::Validation(format!(
                    "training-camp forecast has invalid readiness evidence for {}",
                    player.player_id
                )));
            }
            if players
                .insert(
                    player.player_id,
                    CampEvidence {
                        projected_score: player.projected_score,
                        gp_confidence: player.gp_confidence,
                        make_probability: player.make_probability,
                    },
                )
                .is_some()
            {
                return Err(AhlFeedError::Validation(format!(
                    "training-camp league forecast repeats player {}",
                    player.player_id
                )));
            }
        }
    }
    let bytes =
        serde_json::to_vec(league).map_err(|error| AhlFeedError::Validation(error.to_string()))?;
    Ok((players, format!("sha256:{:x}", Sha256::digest(bytes))))
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

fn unavailable_row(
    player_id: u32,
    display_name: String,
    nhl_teams: Vec<String>,
    reason: AhlRecallReadinessUnavailableReason,
    detail: String,
) -> AhlRecallReadinessUnavailableRow {
    AhlRecallReadinessUnavailableRow {
        nhl_player_id: Some(player_id),
        display_name,
        nhl_teams,
        reason,
        detail,
    }
}

fn same_optional_number(left: Option<f64>, right: Option<f64>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => close(left, right),
        (None, None) => true,
        _ => false,
    }
}

fn close(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1e-9 * left.abs().max(right.abs()).max(1.0)
}

fn fingerprint_ledger(view: &AhlRecallReadinessLedgerView) -> Result<String, AhlFeedError> {
    let mut players = view.players.clone();
    players.sort_by_key(|row| row.nhl_player_id);
    let mut unavailable = view.unavailable.clone();
    unavailable.sort_by(|left, right| {
        left.nhl_player_id
            .cmp(&right.nhl_player_id)
            .then_with(|| left.display_name.cmp(&right.display_name))
    });
    let mut digest = Sha256::new();
    hash_string(&mut digest, &view.schema);
    digest.update(view.prior_season.to_le_bytes());
    digest.update(view.target_season.to_le_bytes());
    hash_string(&mut digest, &view.workboard_fingerprint);
    hash_string(&mut digest, &view.career_store_fetched_at);
    hash_string(&mut digest, &view.camp_forecast_fingerprint);
    hash_string(&mut digest, &view.policy.schema);
    hash_string(&mut digest, &view.policy.method_version);
    hash_f64(&mut digest, view.policy.value_weight);
    hash_f64(&mut digest, view.policy.nhl_experience_weight);
    hash_f64(&mut digest, view.policy.camp_proximity_weight);
    digest.update(view.policy.nhl_experience_games.to_le_bytes());
    hash_f64(&mut digest, view.policy.camp_evidence_confidence);
    hash_f64(&mut digest, view.policy.minimum_coverage);
    for count in [
        view.candidate_appearances,
        view.candidates_requested,
        view.candidates_estimated,
        view.candidates_unavailable,
        players.len(),
        unavailable.len(),
    ] {
        digest.update((count as u64).to_le_bytes());
    }
    for player in &players {
        digest.update(player.nhl_player_id.to_le_bytes());
        hash_string(&mut digest, &player.display_name);
        hash_strings(&mut digest, &player.nhl_teams);
        digest.update([group_code(player.position_group)]);
        hash_string(&mut digest, &player.estimate.method_version);
        hash_optional_f64(&mut digest, player.estimate.readiness_index);
        hash_f64(&mut digest, player.estimate.evidence_confidence);
        hash_f64(&mut digest, player.estimate.coverage);
        hash_optional_f64(&mut digest, player.estimate.value_signal);
        hash_optional_f64(&mut digest, player.estimate.nhl_experience_signal);
        hash_optional_f64(&mut digest, player.estimate.camp_proximity_signal);
        hash_strings(&mut digest, &player.source_urls);
    }
    for row in &unavailable {
        hash_optional_u32(&mut digest, row.nhl_player_id);
        hash_string(&mut digest, &row.display_name);
        hash_strings(&mut digest, &row.nhl_teams);
        digest.update([match row.reason {
            AhlRecallReadinessUnavailableReason::MissingCanonicalIdentity => 0,
            AhlRecallReadinessUnavailableReason::ConflictingCandidateEvidence => 1,
            AhlRecallReadinessUnavailableReason::InsufficientEvidence => 2,
            AhlRecallReadinessUnavailableReason::InvalidEvidence => 3,
        }]);
        hash_string(&mut digest, &row.detail);
    }
    hash_strings(&mut digest, &view.disclosures);
    Ok(format!("sha256:{:x}", digest.finalize()))
}

fn group_code(group: AhlPreseasonPositionGroup) -> u8 {
    match group {
        AhlPreseasonPositionGroup::Forward => 0,
        AhlPreseasonPositionGroup::Defense => 1,
        AhlPreseasonPositionGroup::Goalie => 2,
        AhlPreseasonPositionGroup::Unknown => 3,
    }
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

fn hash_optional_f64(digest: &mut Sha256, value: Option<f64>) {
    digest.update([u8::from(value.is_some())]);
    if let Some(value) = value {
        hash_f64(digest, value);
    }
}

fn hash_optional_u32(digest: &mut Sha256, value: Option<u32>) {
    digest.update([u8::from(value.is_some())]);
    if let Some(value) = value {
        digest.update(value.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_core::{CareerHistory, Position};

    use crate::ahl_preseason_facts::{
        AhlPreseasonFactsPlayerRow, AhlPreseasonFactsTeamCounts, AhlPreseasonFactsTeamView,
        AHL_PRESEASON_LEAGUE_FACTS_WORKBOARD_SCHEMA,
    };

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
                nhl_team: "ANA".to_owned(),
                ahl_team: "San Diego Gulls".to_owned(),
                source_urls: vec!["https://theahl.com/nhl-affiliations".to_owned()],
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
                    nhl_player_id: Some(8484153),
                    display_name: "Leo Carlsson".to_owned(),
                    status: AhlPreseasonFactsCandidateStatus::Candidate,
                    origins: Vec::new(),
                    position_group: AhlPreseasonPositionGroup::Forward,
                    primary_position: Some(Position::Center),
                    eligible_positions: vec![Position::Center],
                    projected_score: Some(50.0),
                    projected_score_method: Some("fixture".to_owned()),
                    projected_score_confidence: Some(0.6),
                    projected_score_sample_games: Some(20),
                    projected_score_source_fingerprint: Some("sha256:fixture".to_owned()),
                    prospect: Some(true),
                    prospect_method: Some("fixture".to_owned()),
                    prospect_source_fingerprint: Some("sha256:fixture".to_owned()),
                    recall_readiness: None,
                    recall_readiness_method: None,
                    recall_readiness_confidence: None,
                    recall_readiness_coverage: None,
                    recall_readiness_source_fingerprint: None,
                    assigned_to_affiliate: None,
                    assignment_authority: None,
                    waiver_cleared: Some(true),
                    review_source_urls: Vec::new(),
                    review_note: None,
                    reviewer: None,
                    reviewed_at: None,
                    professional_games_at_season_start: Some(0),
                    development_rule_qualified: None,
                    blockers: vec![
                        AhlPreseasonFactBlocker::RecallReadiness,
                        AhlPreseasonFactBlocker::AssignmentAuthority,
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
        store.upsert(CareerHistory {
            player_id: 8484153,
            stints: Vec::new(),
        });
        store
    }

    fn camp() -> TrainingCampLeagueForecastView {
        serde_json::from_str(include_str!(
            "../../examples/icecast-league-training-camp-2026-27.json"
        ))
        .unwrap()
    }

    #[test]
    fn ledger_and_application_preserve_distinct_confidence_and_coverage() {
        let board = workboard();
        let ledger = build_ahl_recall_readiness_ledger(
            &board,
            &store(),
            &camp(),
            &AhlRecallReadinessPolicy::default(),
        )
        .unwrap();
        assert_eq!(ledger.candidates_estimated, 1);
        assert_eq!(ledger.candidates_unavailable, 0);
        let estimate = &ledger.players[0].estimate;
        assert_eq!(estimate.coverage, 1.0);
        assert!(estimate.evidence_confidence < estimate.coverage);
        let round_trip: AhlRecallReadinessLedgerView =
            serde_json::from_str(&serde_json::to_string_pretty(&ledger).unwrap()).unwrap();
        assert_eq!(
            fingerprint_ledger(&round_trip).unwrap(),
            round_trip.source_fingerprint
        );

        let application = apply_ahl_recall_readiness_ledger(&board, &ledger).unwrap();
        let player = &application.workboard.team_workboards[0].players[0];
        assert!(player.recall_readiness.is_some());
        assert_eq!(
            player.recall_readiness_confidence,
            Some(estimate.evidence_confidence)
        );
        assert!(!player
            .blockers
            .contains(&AhlPreseasonFactBlocker::RecallReadiness));
        assert!(player
            .blockers
            .contains(&AhlPreseasonFactBlocker::AssignmentAuthority));
    }

    #[test]
    fn stale_ledger_is_rejected() {
        let board = workboard();
        let ledger = build_ahl_recall_readiness_ledger(
            &board,
            &store(),
            &camp(),
            &AhlRecallReadinessPolicy::default(),
        )
        .unwrap();
        let mut changed = board;
        changed.disclosures.push("changed".to_owned());
        changed.source_fingerprint = fingerprint_workboard(&changed).unwrap();
        assert!(apply_ahl_recall_readiness_ledger(&changed, &ledger).is_err());
    }

    #[test]
    fn prior_ahl_value_precedes_camp_value_across_unresolved_organizations() {
        let mut board = workboard();
        let camp = camp();
        let camp_player = camp
            .teams
            .iter()
            .filter_map(|team| team.forecast.as_ref())
            .flat_map(|forecast| &forecast.players)
            .find(|player| player.player_id == 8484153)
            .unwrap();
        let mut second = board.team_workboards[0].clone();
        second.nhl_team = "SEA".to_owned();
        second.ahl_team = "Coachella Valley Firebirds".to_owned();
        second.players[0].projected_score = Some(camp_player.projected_score);
        second.players[0].projected_score_method = None;
        second.players[0].projected_score_confidence = None;
        second.players[0].projected_score_sample_games = None;
        second.players[0].projected_score_source_fingerprint = None;
        board.team_workboards.push(second);
        board.teams = 2;
        recompute_workboard(&mut board).unwrap();
        board.source_fingerprint = fingerprint_workboard(&board).unwrap();

        let ledger = build_ahl_recall_readiness_ledger(
            &board,
            &store(),
            &camp,
            &AhlRecallReadinessPolicy::default(),
        )
        .unwrap();
        assert_eq!(ledger.candidate_appearances, 2);
        assert_eq!(ledger.candidates_requested, 1);
        assert_eq!(ledger.candidates_estimated, 1);
        assert_eq!(ledger.candidates_unavailable, 0);
        assert_eq!(ledger.players[0].estimate.coverage, 1.0);

        let application = apply_ahl_recall_readiness_ledger(&board, &ledger).unwrap();
        assert_eq!(application.rows_applied, 2);
        assert!(application
            .workboard
            .team_workboards
            .iter()
            .all(|team| team.players[0].assigned_to_affiliate.is_none()));
    }
}
