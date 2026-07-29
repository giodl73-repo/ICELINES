//! Source-bound career fallback for missing AHL preseason player values.

use std::collections::{BTreeMap, BTreeSet};

use icelines_core::{
    calibrate_ahl_cross_league_value, estimate_ahl_cross_league_value,
    validate_ahl_cross_league_value_policy, AhlCrossLeagueCalibration,
    AhlCrossLeagueCalibrationPair, AhlCrossLeagueValueEstimate, AhlCrossLeagueValuePolicy,
    AhlPlayerValuePositionGroup, CareerGameType, Position,
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

pub const AHL_CROSS_LEAGUE_VALUE_LEDGER_SCHEMA: &str = "ahl_cross_league_value_ledger.v1";
pub const AHL_CROSS_LEAGUE_VALUE_APPLICATION_SCHEMA: &str = "ahl_cross_league_value_application.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AhlCrossLeagueValueUnavailableReason {
    MissingCanonicalIdentity,
    ConflictingPosition,
    MissingCareerHistory,
    NoSupportedRecentLeague,
    InsufficientSourceWorkload,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlCrossLeagueValueLedgerRow {
    pub nhl_player_id: u32,
    pub display_name: String,
    pub nhl_teams: Vec<String>,
    pub estimate: AhlCrossLeagueValueEstimate,
    pub source_urls: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlCrossLeagueValueUnavailableRow {
    pub nhl_player_id: Option<u32>,
    pub display_name: String,
    pub nhl_teams: Vec<String>,
    pub reason: AhlCrossLeagueValueUnavailableReason,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlCrossLeagueValueLedgerView {
    pub schema: String,
    pub prior_season: u32,
    pub target_season: u32,
    pub workboard_fingerprint: String,
    pub career_store_fetched_at: String,
    pub career_source_fingerprint: String,
    pub policy: AhlCrossLeagueValuePolicy,
    pub candidate_appearances: usize,
    pub candidates_requested: usize,
    pub candidates_estimated: usize,
    pub candidates_unavailable: usize,
    pub calibrations_supported: usize,
    pub source_fingerprint: String,
    pub calibrations: Vec<AhlCrossLeagueCalibration>,
    pub players: Vec<AhlCrossLeagueValueLedgerRow>,
    pub unavailable: Vec<AhlCrossLeagueValueUnavailableRow>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlCrossLeagueValueApplicationView {
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

#[derive(Debug, Clone)]
struct Candidate {
    display_name: String,
    teams: BTreeSet<String>,
    group: Option<AhlPlayerValuePositionGroup>,
    position_conflict: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct CareerAggregateKey {
    player_id: u32,
    group: AhlPlayerValuePositionGroup,
    league: String,
    season: u32,
}

#[derive(Debug, Clone, Default, Serialize)]
struct CareerAggregate {
    games: u32,
    points: u32,
    shots_against: u32,
    saves: u32,
}

impl CareerAggregate {
    fn workload(&self, group: AhlPlayerValuePositionGroup) -> u32 {
        if group == AhlPlayerValuePositionGroup::Goalie {
            self.shots_against
        } else {
            self.games
        }
    }

    fn rate(&self, group: AhlPlayerValuePositionGroup) -> Option<f64> {
        if group == AhlPlayerValuePositionGroup::Goalie {
            (self.shots_against > 0).then(|| f64::from(self.saves) / f64::from(self.shots_against))
        } else {
            (self.games > 0).then(|| f64::from(self.points) / f64::from(self.games))
        }
    }
}

#[derive(Serialize)]
struct CareerSourceSeal<'a> {
    schema_version: u32,
    fetched_at: &'a str,
    rows: &'a [(CareerAggregateKey, CareerAggregate)],
}

pub fn build_ahl_cross_league_value_ledger(
    workboard: &AhlPreseasonLeagueFactsWorkboardView,
    career_store: &CareerHistoryStore,
    policy: &AhlCrossLeagueValuePolicy,
) -> Result<AhlCrossLeagueValueLedgerView, AhlFeedError> {
    validate_workboard(workboard)?;
    validate_ahl_cross_league_value_policy(policy).map_err(AhlFeedError::Validation)?;
    let fetched_at = career_store
        .fetched_at
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AhlFeedError::Validation(
                "cross-league value ledger requires a dated official career store".to_owned(),
            )
        })?;
    if career_store.schema_version == 0 {
        return Err(AhlFeedError::Validation(
            "cross-league value ledger requires a supported career store".to_owned(),
        ));
    }

    let (positions, candidates, candidate_appearances, identity_unavailable) =
        index_candidates(workboard)?;
    let aggregate_map = aggregate_career_rows(career_store, &positions, workboard.prior_season)?;
    let aggregate_rows = aggregate_map
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<Vec<_>>();
    let career_source_fingerprint = fingerprint_json(&CareerSourceSeal {
        schema_version: career_store.schema_version,
        fetched_at,
        rows: &aggregate_rows,
    })?;

    let pair_map = build_calibration_pairs(&aggregate_map, policy)?;
    let mut calibrations = Vec::new();
    for ((group, league), pairs) in pair_map {
        if let Ok(calibration) = calibrate_ahl_cross_league_value(policy, &league, group, &pairs) {
            calibrations.push(calibration);
        }
    }
    calibrations.sort_by(|left, right| {
        left.position_group
            .cmp(&right.position_group)
            .then_with(|| left.source_league.cmp(&right.source_league))
    });
    let calibration_index = calibrations
        .iter()
        .map(|row| ((row.position_group, row.source_league.as_str()), row))
        .collect::<BTreeMap<_, _>>();

    let mut players = Vec::new();
    let mut unavailable = identity_unavailable;
    for (player_id, candidate) in candidates {
        let teams = candidate.teams.into_iter().collect::<Vec<_>>();
        if candidate.position_conflict {
            unavailable.push(unavailable_row(
                Some(player_id),
                candidate.display_name,
                teams,
                AhlCrossLeagueValueUnavailableReason::ConflictingPosition,
                "Canonical player has conflicting position groups across appearances",
            ));
            continue;
        }
        let Some(group) = candidate.group else {
            unavailable.push(unavailable_row(
                Some(player_id),
                candidate.display_name,
                teams,
                AhlCrossLeagueValueUnavailableReason::ConflictingPosition,
                "Candidate has no exact position group",
            ));
            continue;
        };
        if career_store.get(player_id).is_none() {
            unavailable.push(unavailable_row(
                Some(player_id),
                candidate.display_name,
                teams,
                AhlCrossLeagueValueUnavailableReason::MissingCareerHistory,
                "Official career store has no player history",
            ));
            continue;
        }
        let mut attempts = aggregate_map
            .iter()
            .filter(|(key, _)| {
                key.player_id == player_id
                    && key.group == group
                    && !matches!(key.league.as_str(), "AHL" | "NHL")
                    && season_gap(key.season, workboard.prior_season)
                        .is_some_and(|gap| gap < policy.maximum_source_lookback_seasons)
                    && calibration_index.contains_key(&(group, key.league.as_str()))
            })
            .filter_map(|(key, aggregate)| {
                let calibration = calibration_index.get(&(group, key.league.as_str()))?;
                let rate = aggregate.rate(group)?;
                estimate_ahl_cross_league_value(
                    policy,
                    calibration,
                    key.season,
                    aggregate.games,
                    aggregate.workload(group),
                    rate,
                )
                .ok()
            })
            .collect::<Vec<_>>();
        attempts.sort_by(|left, right| {
            right
                .source_season
                .cmp(&left.source_season)
                .then_with(|| right.source_workload.cmp(&left.source_workload))
                .then_with(|| left.source_league.cmp(&right.source_league))
        });
        let Some(estimate) = attempts.into_iter().next() else {
            let has_supported_recent = aggregate_map.keys().any(|key| {
                key.player_id == player_id
                    && key.group == group
                    && season_gap(key.season, workboard.prior_season)
                        .is_some_and(|gap| gap < policy.maximum_source_lookback_seasons)
                    && calibration_index.contains_key(&(group, key.league.as_str()))
            });
            unavailable.push(unavailable_row(
                Some(player_id),
                candidate.display_name,
                teams,
                if has_supported_recent {
                    AhlCrossLeagueValueUnavailableReason::InsufficientSourceWorkload
                } else {
                    AhlCrossLeagueValueUnavailableReason::NoSupportedRecentLeague
                },
                if has_supported_recent {
                    "Recent supported career evidence is below the source-workload gate"
                } else {
                    "No recent non-NHL/AHL league clears the paired calibration gate"
                },
            ));
            continue;
        };
        players.push(AhlCrossLeagueValueLedgerRow {
            nhl_player_id: player_id,
            display_name: candidate.display_name,
            nhl_teams: teams,
            source_urls: vec![format!(
                "https://api-web.nhle.com/v1/player/{player_id}/landing"
            )],
            estimate,
        });
    }
    players.sort_by_key(|row| row.nhl_player_id);
    unavailable.sort_by(|left, right| {
        left.nhl_player_id
            .cmp(&right.nhl_player_id)
            .then_with(|| left.display_name.cmp(&right.display_name))
    });
    let mut view = AhlCrossLeagueValueLedgerView {
        schema: AHL_CROSS_LEAGUE_VALUE_LEDGER_SCHEMA.to_owned(),
        prior_season: workboard.prior_season,
        target_season: workboard.target_season,
        workboard_fingerprint: workboard.source_fingerprint.clone(),
        career_store_fetched_at: fetched_at.to_owned(),
        career_source_fingerprint,
        policy: policy.clone(),
        candidate_appearances,
        candidates_requested: players.len() + unavailable.len(),
        candidates_estimated: players.len(),
        candidates_unavailable: unavailable.len(),
        calibrations_supported: calibrations.len(),
        source_fingerprint: String::new(),
        calibrations,
        players,
        unavailable,
        disclosures: vec![
            "This evaluation-only fallback is used only for missing direct AHL values; it is not a universal NHLe table or a calibrated NHL projection.".to_owned(),
            "Each league/position translation is fitted from frozen same-season or next-season AHL player pairs. Unsupported leagues and short source samples remain blocked.".to_owned(),
            "Skater scoring rates use workload-weighted multiplicative translation; goalie save percentages use a workload-weighted additive adjustment. Calibration confidence reduces effective workload before AHL-prior shrinkage.".to_owned(),
        ],
    };
    view = canonical_wire_round_trip(&view)?;
    view.source_fingerprint = fingerprint_ledger(&view)?;
    Ok(view)
}

pub fn apply_ahl_cross_league_value_ledger(
    workboard: &AhlPreseasonLeagueFactsWorkboardView,
    ledger: &AhlCrossLeagueValueLedgerView,
) -> Result<AhlCrossLeagueValueApplicationView, AhlFeedError> {
    validate_workboard(workboard)?;
    if ledger.schema != AHL_CROSS_LEAGUE_VALUE_LEDGER_SCHEMA
        || ledger.prior_season != workboard.prior_season
        || ledger.target_season != workboard.target_season
        || ledger.workboard_fingerprint != workboard.source_fingerprint
        || ledger.candidates_requested
            != ledger.candidates_estimated + ledger.candidates_unavailable
        || ledger.candidates_estimated != ledger.players.len()
        || ledger.candidates_unavailable != ledger.unavailable.len()
        || ledger.calibrations_supported != ledger.calibrations.len()
    {
        return Err(AhlFeedError::Validation(
            "cross-league value application requires an intact ledger bound to the exact workboard"
                .to_owned(),
        ));
    }
    let expected_fingerprint = fingerprint_ledger(ledger)?;
    if ledger.source_fingerprint != expected_fingerprint {
        return Err(AhlFeedError::Validation(format!(
            "cross-league value ledger fingerprint mismatch: stored {}, recomputed {}",
            ledger.source_fingerprint, expected_fingerprint
        )));
    }
    let rows = ledger
        .players
        .iter()
        .map(|row| (row.nhl_player_id, row))
        .collect::<BTreeMap<_, _>>();
    if rows.len() != ledger.players.len() {
        return Err(AhlFeedError::Validation(
            "cross-league value ledger contains duplicate canonical player rows".to_owned(),
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
            let Some(row) = player.nhl_player_id.and_then(|id| rows.get(&id).copied()) else {
                continue;
            };
            if group_from_position(player.primary_position) != Some(row.estimate.position_group) {
                continue;
            }
            player.projected_score = Some(row.estimate.projected_score);
            player.projected_score_method = Some(row.estimate.method_version.clone());
            player.projected_score_confidence = Some(row.estimate.evidence_confidence);
            player.projected_score_sample_games = Some(row.estimate.source_games);
            player.projected_score_source_fingerprint = Some(ledger.source_fingerprint.clone());
            player
                .blockers
                .retain(|blocker| *blocker != AhlPreseasonFactBlocker::ProjectedScore);
            rows_applied += 1;
        }
    }
    recompute_workboard(&mut applied)?;
    applied.disclosures.push(format!(
        "Cross-league career ledger {} filled {} missing scores; unsupported or short-sample candidates remain blocked.",
        ledger.source_fingerprint, rows_applied
    ));
    applied.source_fingerprint = fingerprint_workboard(&applied)?;
    let candidates_without_value = applied
        .blocker_counts
        .get(&AhlPreseasonFactBlocker::ProjectedScore)
        .copied()
        .unwrap_or_default();
    Ok(AhlCrossLeagueValueApplicationView {
        schema: AHL_CROSS_LEAGUE_VALUE_APPLICATION_SCHEMA.to_owned(),
        prior_season: applied.prior_season,
        target_season: applied.target_season,
        source_workboard_fingerprint,
        value_ledger_fingerprint: ledger.source_fingerprint.clone(),
        rows_applied,
        candidates_without_value,
        workboard: applied,
        disclosures: vec![
            "Only missing projected_score facts are filled; assignment, organization, waiver, prospect, recall, professional-game, and development-rule authority is unchanged.".to_owned(),
        ],
    })
}

type PositionIndex = BTreeMap<u32, AhlPlayerValuePositionGroup>;

#[allow(clippy::type_complexity)]
fn index_candidates(
    workboard: &AhlPreseasonLeagueFactsWorkboardView,
) -> Result<
    (
        PositionIndex,
        BTreeMap<u32, Candidate>,
        usize,
        Vec<AhlCrossLeagueValueUnavailableRow>,
    ),
    AhlFeedError,
> {
    let mut positions = BTreeMap::new();
    let mut candidates = BTreeMap::<u32, Candidate>::new();
    let mut candidate_appearances = 0usize;
    let mut unavailable = Vec::new();
    for team in &workboard.team_workboards {
        for player in &team.players {
            if player.status != AhlPreseasonFactsCandidateStatus::Candidate {
                continue;
            }
            if let Some(player_id) = player.nhl_player_id {
                let group = group_from_position(player.primary_position)
                    .or_else(|| group_from_rollover(player.position_group));
                if let Some(group) = group {
                    if positions
                        .insert(player_id, group)
                        .is_some_and(|prior| prior != group)
                    {
                        return Err(AhlFeedError::Validation(format!(
                            "canonical player {player_id} has conflicting position groups"
                        )));
                    }
                }
            }
            if player.projected_score.is_some() {
                continue;
            }
            candidate_appearances += 1;
            let Some(player_id) = player.nhl_player_id else {
                unavailable.push(AhlCrossLeagueValueUnavailableRow {
                    nhl_player_id: None,
                    display_name: player.display_name.clone(),
                    nhl_teams: vec![team.nhl_team.clone()],
                    reason: AhlCrossLeagueValueUnavailableReason::MissingCanonicalIdentity,
                    detail: "Candidate has no canonical NHL player ID".to_owned(),
                });
                continue;
            };
            let group = group_from_position(player.primary_position)
                .or_else(|| group_from_rollover(player.position_group));
            let entry = candidates.entry(player_id).or_insert_with(|| Candidate {
                display_name: player.display_name.clone(),
                teams: BTreeSet::new(),
                group,
                position_conflict: false,
            });
            entry.teams.insert(team.nhl_team.clone());
            if entry.group != group {
                entry.position_conflict = true;
            }
        }
    }
    Ok((positions, candidates, candidate_appearances, unavailable))
}

fn aggregate_career_rows(
    store: &CareerHistoryStore,
    positions: &PositionIndex,
    prior_season: u32,
) -> Result<BTreeMap<CareerAggregateKey, CareerAggregate>, AhlFeedError> {
    let mut aggregates = BTreeMap::<CareerAggregateKey, CareerAggregate>::new();
    for (player_id, group) in positions {
        let Some(history) = store.get(*player_id) else {
            continue;
        };
        if history.player_id != *player_id {
            return Err(AhlFeedError::Validation(format!(
                "career history identity mismatch for player {player_id}"
            )));
        }
        for stint in &history.stints {
            if stint.game_type != CareerGameType::Regular
                || stint.season.0 > prior_season
                || stint.gp == 0
            {
                continue;
            }
            let league = stint.league.as_str().trim().to_ascii_uppercase();
            if league.is_empty() {
                continue;
            }
            let key = CareerAggregateKey {
                player_id: *player_id,
                group: *group,
                league,
                season: stint.season.0,
            };
            let aggregate = aggregates.entry(key).or_default();
            match group {
                AhlPlayerValuePositionGroup::Goalie => {
                    let Some(shots) = stint.shots_against.filter(|shots| *shots > 0) else {
                        continue;
                    };
                    let saves = if let Some(goals_against) = stint.goals_against {
                        shots.checked_sub(goals_against).ok_or_else(|| {
                            AhlFeedError::Validation(format!(
                                "goalie career goals against exceed shots for player {player_id}"
                            ))
                        })?
                    } else if let Some(save_percentage) = stint.save_pct {
                        (f64::from(shots) * f64::from(save_percentage)).round() as u32
                    } else {
                        continue;
                    };
                    aggregate.games = checked_add(aggregate.games, stint.gp, "career games")?;
                    aggregate.shots_against =
                        checked_add(aggregate.shots_against, shots, "career shots against")?;
                    aggregate.saves = checked_add(aggregate.saves, saves, "career saves")?;
                }
                _ => {
                    let Some(points) = stint.points else {
                        continue;
                    };
                    aggregate.games = checked_add(aggregate.games, stint.gp, "career games")?;
                    aggregate.points = checked_add(aggregate.points, points, "career points")?;
                }
            }
        }
    }
    aggregates.retain(|key, row| row.rate(key.group).is_some());
    Ok(aggregates)
}

fn build_calibration_pairs(
    aggregates: &BTreeMap<CareerAggregateKey, CareerAggregate>,
    policy: &AhlCrossLeagueValuePolicy,
) -> Result<
    BTreeMap<(AhlPlayerValuePositionGroup, String), Vec<AhlCrossLeagueCalibrationPair>>,
    AhlFeedError,
> {
    let mut pairs =
        BTreeMap::<(AhlPlayerValuePositionGroup, String), Vec<AhlCrossLeagueCalibrationPair>>::new(
        );
    for (source_key, source) in aggregates {
        if matches!(source_key.league.as_str(), "AHL" | "NHL") {
            continue;
        }
        let source_rate = source.rate(source_key.group).ok_or_else(|| {
            AhlFeedError::Validation("career source aggregate has no rate".to_owned())
        })?;
        let mut ahl_match = None;
        for gap in 0..=policy.maximum_pair_season_gap {
            let season = advance_season(source_key.season, gap)?;
            let key = CareerAggregateKey {
                player_id: source_key.player_id,
                group: source_key.group,
                league: "AHL".to_owned(),
                season,
            };
            if let Some(row) = aggregates.get(&key) {
                ahl_match = Some((season, row));
                break;
            }
        }
        let Some((ahl_season, ahl)) = ahl_match else {
            continue;
        };
        let ahl_rate = ahl.rate(source_key.group).ok_or_else(|| {
            AhlFeedError::Validation("career AHL aggregate has no rate".to_owned())
        })?;
        let paired_workload = source
            .workload(source_key.group)
            .min(ahl.workload(source_key.group));
        let minimum_pair_workload = if source_key.group == AhlPlayerValuePositionGroup::Goalie {
            policy.minimum_goalie_pair_shots
        } else {
            policy.minimum_skater_pair_games
        };
        if paired_workload < minimum_pair_workload {
            continue;
        }
        pairs
            .entry((source_key.group, source_key.league.clone()))
            .or_default()
            .push(AhlCrossLeagueCalibrationPair {
                player_id: source_key.player_id,
                source_season: source_key.season,
                ahl_season,
                source_rate,
                ahl_rate,
                paired_workload,
            });
    }
    Ok(pairs)
}

fn group_from_position(position: Option<Position>) -> Option<AhlPlayerValuePositionGroup> {
    match position {
        Some(Position::Goalie) => Some(AhlPlayerValuePositionGroup::Goalie),
        Some(Position::Defense) => Some(AhlPlayerValuePositionGroup::Defense),
        Some(_) => Some(AhlPlayerValuePositionGroup::Forward),
        None => None,
    }
}

fn group_from_rollover(group: AhlPreseasonPositionGroup) -> Option<AhlPlayerValuePositionGroup> {
    match group {
        AhlPreseasonPositionGroup::Forward => Some(AhlPlayerValuePositionGroup::Forward),
        AhlPreseasonPositionGroup::Defense => Some(AhlPlayerValuePositionGroup::Defense),
        AhlPreseasonPositionGroup::Goalie => Some(AhlPlayerValuePositionGroup::Goalie),
        AhlPreseasonPositionGroup::Unknown => None,
    }
}

fn unavailable_row(
    player_id: Option<u32>,
    display_name: String,
    teams: Vec<String>,
    reason: AhlCrossLeagueValueUnavailableReason,
    detail: &str,
) -> AhlCrossLeagueValueUnavailableRow {
    AhlCrossLeagueValueUnavailableRow {
        nhl_player_id: player_id,
        display_name,
        nhl_teams: teams,
        reason,
        detail: detail.to_owned(),
    }
}

fn season_gap(earlier: u32, later: u32) -> Option<u32> {
    (later / 10_000).checked_sub(earlier / 10_000)
}

fn advance_season(season: u32, gap: u32) -> Result<u32, AhlFeedError> {
    season
        .checked_add(gap.saturating_mul(10_001))
        .ok_or_else(|| AhlFeedError::Validation("career season advancement overflow".to_owned()))
}

fn checked_add(left: u32, right: u32, label: &str) -> Result<u32, AhlFeedError> {
    left.checked_add(right)
        .ok_or_else(|| AhlFeedError::Validation(format!("{label} overflow")))
}

fn fingerprint_ledger(view: &AhlCrossLeagueValueLedgerView) -> Result<String, AhlFeedError> {
    let mut canonical = view.clone();
    canonical.source_fingerprint.clear();
    fingerprint_json(&canonical)
}

fn fingerprint_json<T: Serialize>(value: &T) -> Result<String, AhlFeedError> {
    let bytes =
        serde_json::to_vec(value).map_err(|error| AhlFeedError::Validation(error.to_string()))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn canonical_wire_round_trip(
    value: &AhlCrossLeagueValueLedgerView,
) -> Result<AhlCrossLeagueValueLedgerView, AhlFeedError> {
    let bytes =
        serde_json::to_vec(value).map_err(|error| AhlFeedError::Validation(error.to_string()))?;
    serde_json::from_slice(&bytes).map_err(|error| AhlFeedError::Validation(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ahl_preseason_facts::{
        AhlPreseasonFactsPlayerRow, AhlPreseasonFactsTeamCounts, AhlPreseasonFactsTeamView,
        AHL_PRESEASON_LEAGUE_FACTS_WORKBOARD_SCHEMA,
    };
    use icelines_core::{CareerHistory, CareerStint, LeagueAbbrev, Season};

    fn workboard() -> AhlPreseasonLeagueFactsWorkboardView {
        let mut players = Vec::new();
        for id in 1..=31 {
            players.push(AhlPreseasonFactsPlayerRow {
                nhl_player_id: Some(id),
                display_name: format!("Player {id}"),
                status: AhlPreseasonFactsCandidateStatus::Candidate,
                origins: Vec::new(),
                position_group: AhlPreseasonPositionGroup::Forward,
                primary_position: Some(Position::Center),
                eligible_positions: vec![Position::Center],
                projected_score: (id != 1).then_some(20.0),
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
                waiver_authority: None,
                review_source_urls: Vec::new(),
                review_note: None,
                reviewer: None,
                reviewed_at: None,
                professional_games_at_season_start: Some(50),
                development_rule_qualified: Some(true),
                blockers: if id == 1 {
                    vec![AhlPreseasonFactBlocker::ProjectedScore]
                } else {
                    Vec::new()
                },
            });
        }
        let mut view = AhlPreseasonLeagueFactsWorkboardView {
            schema: AHL_PRESEASON_LEAGUE_FACTS_WORKBOARD_SCHEMA.to_owned(),
            prior_season: 20252026,
            target_season: 20262027,
            professional_game_policy_id: "final-policy".to_owned(),
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
                source_urls: vec!["https://theahl.com".to_owned()],
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
                players,
            }],
            disclosures: vec!["fixture".to_owned()],
        };
        recompute_workboard(&mut view).expect("workboard");
        view.source_fingerprint = fingerprint_workboard(&view).expect("fingerprint");
        view
    }

    fn career_store() -> CareerHistoryStore {
        let mut store = CareerHistoryStore::new();
        store.fetched_at = Some("2026-07-28T12:00:00Z".to_owned());
        for id in 1..=31 {
            store.upsert(CareerHistory {
                player_id: id,
                stints: vec![
                    stint(20242025, "ECHL", 40, 40),
                    stint(20252026, "AHL", 40, 20),
                    stint(20252026, "ECHL", 40, 40),
                ],
            });
        }
        store
    }

    fn stint(season: u32, league: &str, games: u32, points: u32) -> CareerStint {
        CareerStint {
            season: Season(season),
            league: LeagueAbbrev::new(league),
            team: "Fixture".to_owned(),
            game_type: CareerGameType::Regular,
            sequence: 1,
            gp: games,
            goals: Some(points),
            assists: Some(0),
            points: Some(points),
            pim: None,
            plus_minus: None,
            power_play_goals: None,
            power_play_points: None,
            shorthanded_goals: None,
            shorthanded_points: None,
            game_winning_goals: None,
            ot_goals: None,
            shots: None,
            shooting_pct: None,
            avg_toi_sec: None,
            faceoff_win_pct: None,
            games_started: None,
            wins: None,
            losses: None,
            ot_losses: None,
            goals_against: None,
            goals_against_avg: None,
            save_pct: None,
            shots_against: None,
            shutouts: None,
            time_on_ice_sec: None,
        }
    }

    #[test]
    fn paired_career_fallback_applies_only_to_a_missing_score() {
        let board = workboard();
        let ledger = build_ahl_cross_league_value_ledger(
            &board,
            &career_store(),
            &AhlCrossLeagueValuePolicy::default(),
        )
        .expect("ledger");
        assert_eq!(ledger.candidates_requested, 1);
        assert_eq!(ledger.candidates_estimated, 1);
        assert_eq!(ledger.players[0].estimate.source_league, "ECHL");
        let application =
            apply_ahl_cross_league_value_ledger(&board, &ledger).expect("application");
        assert_eq!(application.rows_applied, 1);
        assert_eq!(application.candidates_without_value, 0);
        assert_eq!(
            application.workboard.team_workboards[0].players[1].projected_score,
            Some(20.0)
        );
    }

    #[test]
    fn tampered_ledger_and_underpowered_calibration_fail_closed() {
        let board = workboard();
        let mut ledger = build_ahl_cross_league_value_ledger(
            &board,
            &career_store(),
            &AhlCrossLeagueValuePolicy::default(),
        )
        .expect("ledger");
        ledger.players[0].estimate.projected_score += 1.0;
        assert!(apply_ahl_cross_league_value_ledger(&board, &ledger).is_err());

        let policy = AhlCrossLeagueValuePolicy {
            method_version: "unsupported".to_owned(),
            ..AhlCrossLeagueValuePolicy::default()
        };
        assert!(build_ahl_cross_league_value_ledger(&board, &career_store(), &policy).is_err());
        let policy = AhlCrossLeagueValuePolicy {
            minimum_calibration_players: 40,
            ..AhlCrossLeagueValuePolicy::default()
        };
        let weak = build_ahl_cross_league_value_ledger(&board, &career_store(), &policy)
            .expect("partial ledger");
        assert_eq!(weak.candidates_estimated, 0);
        assert_eq!(weak.candidates_unavailable, 1);
    }
}
