//! Historical reconstruction of the identities knowable immediately before puck drop.
//!
//! Final boxscores are used only as a later-retrieved authority for dressed player IDs,
//! explicit starter flags, teams, and scheduled start time. No result or performance
//! field enters the projection or its fingerprint. Player quality comes exclusively
//! from the already-frozen opening-strength rows on the source forecast.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Duration, NaiveDate, Utc};
use icelines_core::{
    TeamGameEvidenceState, TeamGameForecastView, TeamGameForecastVintage, TeamGameOpeningPlayerRow,
    TeamGamePredictionTeamEvidence,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::game_prediction_edge_package::{
    GamePredictionEdgeEvidencePackage, GamePredictionEdgePackageError,
    GamePredictionEvidenceSource, GamePredictionEvidenceSourceAuthority,
};

#[derive(Debug, Clone, PartialEq)]
pub struct HistoricalOfficialBoxscoreInput {
    pub game_id: u64,
    pub raw: serde_json::Value,
    pub source_uri: String,
    pub retrieved_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoricalConfirmedEdgeBuildResult {
    pub package: GamePredictionEdgeEvidencePackage,
    pub games: usize,
    pub availability_sides: usize,
    pub goalie_sides: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Error)]
pub enum HistoricalOfficialEdgeError {
    #[error("invalid historical official edge input: {0}")]
    Invalid(String),
    #[error(transparent)]
    Package(#[from] GamePredictionEdgePackageError),
    #[error("historical official edge serialization failed: {0}")]
    Serialization(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PregameIdentitySnapshot {
    game_id: u64,
    game_date: NaiveDate,
    start_time_utc: DateTime<Utc>,
    away: TeamPregameIdentity,
    home: TeamPregameIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct TeamPregameIdentity {
    team: String,
    forward_ids: Vec<u32>,
    defense_ids: Vec<u32>,
    goalie_ids: Vec<u32>,
    starter_id: u32,
}

/// Enrich a sealed game-morning xG package into an independently sealed
/// pregame-confirmed package. The source package is never mutated.
pub fn build_historical_confirmed_edge_package(
    forecast: &TeamGameForecastView,
    morning: &GamePredictionEdgeEvidencePackage,
    inputs: Vec<HistoricalOfficialBoxscoreInput>,
    created_at: DateTime<Utc>,
) -> Result<HistoricalConfirmedEdgeBuildResult, HistoricalOfficialEdgeError> {
    morning.validate()?;
    let forecast_fingerprint = fingerprint(forecast)?;
    if morning.season != forecast.season
        || morning.vintage != TeamGameForecastVintage::GameMorning
        || morning.source_forecast_fingerprint != forecast_fingerprint
        || morning.games.len() != forecast.games.len()
    {
        return Err(HistoricalOfficialEdgeError::Invalid(
            "morning package must be complete and bind the exact source forecast".to_owned(),
        ));
    }

    let scheduled_ids = forecast
        .games
        .iter()
        .map(|game| game.game_id)
        .collect::<BTreeSet<_>>();
    let mut snapshots = BTreeMap::new();
    for input in inputs {
        if input.game_id == 0
            || input.source_uri.trim().is_empty()
            || input.retrieved_at > created_at
            || !scheduled_ids.contains(&input.game_id)
            || snapshots.contains_key(&input.game_id)
        {
            return Err(HistoricalOfficialEdgeError::Invalid(format!(
                "boxscore {} has invalid identity, URI, retrieval time, or duplication",
                input.game_id
            )));
        }
        let snapshot = parse_identity_snapshot(input.game_id, &input.raw)?;
        snapshots.insert(
            input.game_id,
            (snapshot, input.source_uri, input.retrieved_at),
        );
    }
    if snapshots.len() != scheduled_ids.len() {
        let missing = scheduled_ids
            .iter()
            .filter(|game_id| !snapshots.contains_key(game_id))
            .take(10)
            .map(u64::to_string)
            .collect::<Vec<_>>();
        return Err(HistoricalOfficialEdgeError::Invalid(format!(
            "official boxscores missing for {} games (first: {})",
            scheduled_ids.len() - snapshots.len(),
            missing.join(", ")
        )));
    }

    let player_values = opening_player_values(forecast)?;
    let opening_players = forecast
        .opening_strengths
        .iter()
        .map(|row| (row.team.clone(), row.players.clone()))
        .collect::<BTreeMap<_, _>>();
    let morning_by_game = morning
        .games
        .iter()
        .map(|game| (game.game_id, game))
        .collect::<BTreeMap<_, _>>();
    let forecast_by_game = forecast
        .games
        .iter()
        .map(|game| (game.game_id, game))
        .collect::<BTreeMap<_, _>>();
    let mut sources = morning.sources.clone();
    let mut games = Vec::with_capacity(forecast.games.len());
    let mut warnings = Vec::new();
    let mut availability_sides = 0;
    let mut goalie_sides = 0;

    for game_id in scheduled_ids {
        let source_game = forecast_by_game[&game_id];
        let morning_game = morning_by_game.get(&game_id).ok_or_else(|| {
            HistoricalOfficialEdgeError::Invalid(format!("morning evidence missing game {game_id}"))
        })?;
        let (snapshot, source_uri, retrieved_at) = &snapshots[&game_id];
        if snapshot.away.team != source_game.away_team
            || snapshot.home.team != source_game.home_team
            || snapshot.game_date != source_game.date
        {
            return Err(HistoricalOfficialEdgeError::Invalid(format!(
                "boxscore identity/date does not match forecast game {game_id}"
            )));
        }
        let forecast_at = snapshot.start_time_utc - Duration::minutes(1);
        let identity_fingerprint = fingerprint(snapshot)?;
        sources.push(GamePredictionEvidenceSource {
            source_key: format!("official.pregame_identity.{game_id}"),
            evidence_cutoff_at: forecast_at,
            retrieved_at: *retrieved_at,
            authority: GamePredictionEvidenceSourceAuthority::HistoricalReconstruction,
            source_uri: source_uri.clone(),
            fingerprint: identity_fingerprint.clone(),
        });

        let mut away = morning_game.away.clone();
        let mut home = morning_game.home.clone();
        let away_enrichment = enrich_team(
            &mut away,
            &snapshot.away,
            &player_values,
            opening_players.get(&snapshot.away.team).ok_or_else(|| {
                HistoricalOfficialEdgeError::Invalid(format!(
                    "forecast has no opening players for {}",
                    snapshot.away.team
                ))
            })?,
            &identity_fingerprint,
            game_id,
        );
        let home_enrichment = enrich_team(
            &mut home,
            &snapshot.home,
            &player_values,
            opening_players.get(&snapshot.home.team).ok_or_else(|| {
                HistoricalOfficialEdgeError::Invalid(format!(
                    "forecast has no opening players for {}",
                    snapshot.home.team
                ))
            })?,
            &identity_fingerprint,
            game_id,
        );
        for enrichment in [away_enrichment, home_enrichment] {
            availability_sides += usize::from(enrichment.availability);
            goalie_sides += usize::from(enrichment.goalie);
            warnings.extend(enrichment.warnings);
        }
        games.push(icelines_core::TeamGamePredictionEvidenceInput {
            game_id,
            forecast_at,
            captured_at: forecast_at,
            away,
            home,
        });
    }
    warnings.sort();
    warnings.dedup();
    let package = GamePredictionEdgeEvidencePackage::build(
        forecast.season,
        TeamGameForecastVintage::PregameConfirmed,
        created_at,
        forecast_fingerprint,
        sources,
        games,
    )?;
    Ok(HistoricalConfirmedEdgeBuildResult {
        package,
        games: forecast.games.len(),
        availability_sides,
        goalie_sides,
        warnings,
    })
}

struct TeamEnrichment {
    availability: bool,
    goalie: bool,
    warnings: Vec<String>,
}

fn enrich_team(
    evidence: &mut TeamGamePredictionTeamEvidence,
    identity: &TeamPregameIdentity,
    player_values: &BTreeMap<u32, TeamGameOpeningPlayerRow>,
    opening_players: &[TeamGameOpeningPlayerRow],
    source_fingerprint: &str,
    game_id: u64,
) -> TeamEnrichment {
    let mut availability = false;
    let goalie = true;
    let mut warnings = Vec::new();
    evidence.goalie_player_id = Some(identity.starter_id);
    let skaters = identity
        .forward_ids
        .iter()
        .chain(&identity.defense_ids)
        .copied()
        .collect::<Vec<_>>();
    let known_values = skaters
        .iter()
        .filter_map(|player_id| player_values.get(player_id).map(|row| row.modeled_value))
        .filter(|value| value.is_finite() && (0.0..=100.0).contains(value))
        .collect::<Vec<_>>();
    if !skaters.is_empty() && known_values.len() * 2 >= skaters.len() {
        let missing = skaters.len() - known_values.len();
        evidence.availability_strength =
            Some((known_values.iter().sum::<f64>() + 50.0 * missing as f64) / skaters.len() as f64);
        evidence.availability_state = if missing == 0 {
            TeamGameEvidenceState::Confirmed
        } else {
            TeamGameEvidenceState::Modeled
        };
        availability = true;
        if missing > 0 {
            warnings.push(format!(
                "game {game_id} {} uses a neutral prior for {missing}/{} dressed skaters",
                identity.team,
                skaters.len()
            ));
        }
        let actual_forwards = identity
            .forward_ids
            .iter()
            .map(|player_id| {
                player_values
                    .get(player_id)
                    .map_or(50.0, |row| row.modeled_value)
            })
            .collect::<Vec<_>>();
        let actual_defense = identity
            .defense_ids
            .iter()
            .map(|player_id| {
                player_values
                    .get(player_id)
                    .map_or(50.0, |row| row.modeled_value)
            })
            .collect::<Vec<_>>();
        let expected_forwards = opening_players
            .iter()
            .filter(|player| player.position_group == "forward")
            .map(|player| player.modeled_value)
            .collect::<Vec<_>>();
        let expected_defense = opening_players
            .iter()
            .filter(|player| player.position_group == "defense")
            .map(|player| player.modeled_value)
            .collect::<Vec<_>>();
        evidence.lineup_impact = Some(
            (replacement_adjusted_lineup_value(actual_forwards, actual_defense)
                - replacement_adjusted_lineup_value(expected_forwards, expected_defense))
            .clamp(-55.0, 55.0),
        );
        evidence.lineup_impact_state = evidence.availability_state;
    } else {
        evidence.availability_strength = None;
        evidence.availability_state = TeamGameEvidenceState::Unavailable;
        evidence.lineup_impact = None;
        evidence.lineup_impact_state = TeamGameEvidenceState::Unavailable;
        warnings.push(format!(
            "game {game_id} {} dressed-lineup value coverage {}/{} is below 50%",
            identity.team,
            known_values.len(),
            skaters.len()
        ));
    }
    if let Some(value) = player_values
        .get(&identity.starter_id)
        .filter(|row| row.position_group == "goalie")
        .map(|row| row.modeled_value)
        .filter(|value| value.is_finite() && (0.0..=100.0).contains(value))
    {
        evidence.goalie_quality = Some(value);
        evidence.goalie_state = TeamGameEvidenceState::Confirmed;
    } else {
        evidence.goalie_quality = Some(50.0);
        evidence.goalie_state = TeamGameEvidenceState::Modeled;
        warnings.push(format!(
            "game {game_id} {} starter {} uses a neutral frozen prior",
            identity.team, identity.starter_id
        ));
    }
    evidence
        .source_fingerprints
        .push(source_fingerprint.to_owned());
    evidence.source_fingerprints.sort();
    evidence.source_fingerprints.dedup();
    TeamEnrichment {
        availability,
        goalie,
        warnings,
    }
}

fn replacement_adjusted_lineup_value(mut forwards: Vec<f64>, mut defense: Vec<f64>) -> f64 {
    const REPLACEMENT_VALUE: f64 = 45.0;
    forwards.sort_by(|left, right| right.total_cmp(left));
    defense.sort_by(|left, right| right.total_cmp(left));
    forwards.truncate(12);
    defense.truncate(6);
    forwards.resize(12, 50.0);
    defense.resize(6, 50.0);
    forwards
        .into_iter()
        .chain(defense)
        .map(|value| (value - REPLACEMENT_VALUE).max(0.0))
        .sum::<f64>()
        / 18.0
}

fn opening_player_values(
    forecast: &TeamGameForecastView,
) -> Result<BTreeMap<u32, TeamGameOpeningPlayerRow>, HistoricalOfficialEdgeError> {
    let mut values = BTreeMap::new();
    for player in forecast
        .opening_strengths
        .iter()
        .flat_map(|team| team.players.iter())
    {
        if player.player_id == 0
            || !player.modeled_value.is_finite()
            || !(0.0..=100.0).contains(&player.modeled_value)
        {
            return Err(HistoricalOfficialEdgeError::Invalid(
                "opening player-value authority is invalid".to_owned(),
            ));
        }
        if let Some(existing) = values.insert(player.player_id, player.clone()) {
            if (existing.modeled_value - player.modeled_value).abs() > 1e-9
                || existing.position_group != player.position_group
            {
                return Err(HistoricalOfficialEdgeError::Invalid(format!(
                    "opening player {} has conflicting frozen values",
                    player.player_id
                )));
            }
        }
    }
    if values.is_empty() {
        return Err(HistoricalOfficialEdgeError::Invalid(
            "forecast has no frozen opening player values".to_owned(),
        ));
    }
    Ok(values)
}

fn parse_identity_snapshot(
    game_id: u64,
    raw: &serde_json::Value,
) -> Result<PregameIdentitySnapshot, HistoricalOfficialEdgeError> {
    let start_time_utc = raw["startTimeUTC"]
        .as_str()
        .ok_or_else(|| {
            HistoricalOfficialEdgeError::Invalid(format!("game {game_id} has no startTimeUTC"))
        })?
        .parse::<DateTime<Utc>>()
        .map_err(|error| {
            HistoricalOfficialEdgeError::Invalid(format!("game {game_id} startTimeUTC: {error}"))
        })?;
    let game_date = raw["gameDate"]
        .as_str()
        .ok_or_else(|| {
            HistoricalOfficialEdgeError::Invalid(format!("game {game_id} has no gameDate"))
        })?
        .parse::<NaiveDate>()
        .map_err(|error| {
            HistoricalOfficialEdgeError::Invalid(format!("game {game_id} gameDate: {error}"))
        })?;
    let parse_side = |side: &str| -> Result<TeamPregameIdentity, HistoricalOfficialEdgeError> {
        let team = raw[format!("{side}Team")]["abbrev"]
            .as_str()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                HistoricalOfficialEdgeError::Invalid(format!(
                    "game {game_id} {side} team is missing"
                ))
            })?
            .to_ascii_uppercase();
        let stats = &raw["playerByGameStats"][format!("{side}Team")];
        let ids = |key: &str| -> Result<Vec<u32>, HistoricalOfficialEdgeError> {
            let mut rows = stats[key]
                .as_array()
                .ok_or_else(|| {
                    HistoricalOfficialEdgeError::Invalid(format!(
                        "game {game_id} {team} has no {key}"
                    ))
                })?
                .iter()
                .map(|row| {
                    row["playerId"]
                        .as_u64()
                        .filter(|id| *id <= u64::from(u32::MAX))
                        .map(|id| id as u32)
                })
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| {
                    HistoricalOfficialEdgeError::Invalid(format!(
                        "game {game_id} {team} {key} has invalid player IDs"
                    ))
                })?;
            rows.sort_unstable();
            rows.dedup();
            Ok(rows)
        };
        let forward_ids = ids("forwards")?;
        let defense_ids = ids("defense")?;
        let goalie_rows = stats["goalies"].as_array().ok_or_else(|| {
            HistoricalOfficialEdgeError::Invalid(format!("game {game_id} {team} has no goalies"))
        })?;
        let mut goalie_ids = goalie_rows
            .iter()
            .filter_map(|row| row["playerId"].as_u64().map(|id| id as u32))
            .collect::<Vec<_>>();
        goalie_ids.sort_unstable();
        goalie_ids.dedup();
        let starters = goalie_rows
            .iter()
            .filter(|row| row["starter"].as_bool() == Some(true))
            .filter_map(|row| row["playerId"].as_u64().map(|id| id as u32))
            .collect::<Vec<_>>();
        let skater_count = forward_ids.len() + defense_ids.len();
        if !(15..=19).contains(&skater_count)
            || goalie_ids.is_empty()
            || starters.len() != 1
            || !goalie_ids.contains(&starters[0])
        {
            return Err(HistoricalOfficialEdgeError::Invalid(format!(
                "game {game_id} {team} has invalid dressed/starter identity counts"
            )));
        }
        Ok(TeamPregameIdentity {
            team,
            forward_ids,
            defense_ids,
            goalie_ids,
            starter_id: starters[0],
        })
    };
    Ok(PregameIdentitySnapshot {
        game_id,
        game_date,
        start_time_utc,
        away: parse_side("away")?,
        home: parse_side("home")?,
    })
}

fn fingerprint<T: Serialize>(value: &T) -> Result<String, HistoricalOfficialEdgeError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| HistoricalOfficialEdgeError::Serialization(error.to_string()))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use icelines_core::TeamGamePredictionTeamEvidence;
    use serde_json::json;

    use super::*;

    fn players(first: u32, count: u32) -> Vec<serde_json::Value> {
        (first..first + count)
            .map(|player_id| json!({"playerId": player_id}))
            .collect()
    }

    fn raw_boxscore() -> serde_json::Value {
        json!({
            "gameDate": "2026-10-08",
            "startTimeUTC": "2026-10-08T23:00:00Z",
            "awayTeam": {"abbrev": "NYR", "score": 7},
            "homeTeam": {"abbrev": "SEA", "score": 1},
            "playerByGameStats": {
                "awayTeam": {
                    "forwards": players(1, 9),
                    "defense": players(10, 6),
                    "goalies": [
                        {"playerId": 16, "starter": true, "saves": 40, "decision": "W"},
                        {"playerId": 17, "starter": false, "saves": 0}
                    ]
                },
                "homeTeam": {
                    "forwards": players(101, 9),
                    "defense": players(110, 6),
                    "goalies": [
                        {"playerId": 116, "starter": true, "saves": 5, "decision": "L"},
                        {"playerId": 117, "starter": false, "saves": 20}
                    ]
                }
            }
        })
    }

    #[test]
    fn l0_identity_projection_excludes_results_and_performance() {
        let first = parse_identity_snapshot(1, &raw_boxscore()).unwrap();
        let mut changed = raw_boxscore();
        changed["awayTeam"]["score"] = json!(0);
        changed["homeTeam"]["score"] = json!(9);
        changed["playerByGameStats"]["awayTeam"]["goalies"][0]["saves"] = json!(0);
        changed["playerByGameStats"]["awayTeam"]["goalies"][0]["decision"] = json!("L");
        let second = parse_identity_snapshot(1, &changed).unwrap();
        assert_eq!(first, second);
        assert_eq!(fingerprint(&first).unwrap(), fingerprint(&second).unwrap());
    }

    #[test]
    fn l0_confirmed_identity_uses_only_frozen_player_values() {
        let snapshot = parse_identity_snapshot(1, &raw_boxscore()).unwrap();
        let mut values = BTreeMap::new();
        for player_id in 1..=17 {
            values.insert(
                player_id,
                TeamGameOpeningPlayerRow {
                    player_id,
                    full_name: format!("Player {player_id}"),
                    position_group: if player_id >= 16 { "goalie" } else { "forward" }.to_owned(),
                    prior_value: Some(f64::from(player_id)),
                    modeled_value: 40.0 + f64::from(player_id),
                    selected_at_opening: true,
                },
            );
        }
        let mut evidence = TeamGamePredictionTeamEvidence {
            team: "NYR".to_owned(),
            roster_strength: Some(50.0),
            roster_state: TeamGameEvidenceState::Modeled,
            availability_strength: None,
            availability_state: TeamGameEvidenceState::Unavailable,
            lineup_impact: None,
            lineup_impact_state: TeamGameEvidenceState::Unavailable,
            goalie_quality: None,
            goalie_state: TeamGameEvidenceState::Unavailable,
            goalie_player_id: None,
            goalie_form_quality: None,
            goalie_form_appearances: 0,
            goalie_form_state: TeamGameEvidenceState::Unavailable,
            goalie_workload_readiness: None,
            xg_share: Some(0.51),
            xg_games: 10,
            opponent_adjusted_xg_share: None,
            opponent_adjusted_xg_games: 0,
            special_teams_strength: Some(52.0),
            special_teams_games: 10,
            matchup_suitability: None,
            matchup_state: TeamGameEvidenceState::Unavailable,
            source_fingerprints: vec![format!("sha256:{}", "a".repeat(64))],
        };
        let enrichment = enrich_team(
            &mut evidence,
            &snapshot.away,
            &values,
            &values.values().cloned().collect::<Vec<_>>(),
            &format!("sha256:{}", "b".repeat(64)),
            1,
        );
        assert_eq!(
            evidence.availability_state,
            TeamGameEvidenceState::Confirmed
        );
        assert_eq!(evidence.goalie_state, TeamGameEvidenceState::Confirmed);
        assert_eq!(evidence.goalie_quality, Some(56.0));
        assert!(evidence.lineup_impact.is_some());
        assert_eq!(
            evidence.lineup_impact_state,
            TeamGameEvidenceState::Confirmed
        );
        assert!(enrichment.availability);
        assert!(enrichment.goalie);
        assert!(enrichment.warnings.is_empty());
    }
}
