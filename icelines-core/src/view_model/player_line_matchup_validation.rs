//! Chronological ablation evaluation for player/line matchup forecasts.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    validate_player_line_matchup_forecast, PlayerLineMatchupForecastView,
    TeamGamePredictionOutcomeInput, CANONICAL_TEAMS,
};

pub const PLAYER_LINE_MATCHUP_ABLATION_OBSERVATION_SCHEMA: &str =
    "player_line_matchup_ablation_observation.v1";
pub const PLAYER_LINE_MATCHUP_ABLATION_PREDICTION_SCHEMA: &str =
    "player_line_matchup_ablation_prediction.v1";
pub const PLAYER_LINE_MATCHUP_ABLATION_PREDICTION_JSON_SCHEMA: &str =
    include_str!("../../../design/schemas/player_line_matchup_ablation_prediction.v1.schema.json");
pub const PLAYER_LINE_MATCHUP_VALIDATION_SCHEMA: &str = "player_line_matchup_validation.v1";
pub const PLAYER_LINE_MATCHUP_VALIDATION_JSON_SCHEMA: &str =
    include_str!("../../../design/schemas/player_line_matchup_validation.v1.schema.json");

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayerLineMatchupAblationProbabilities {
    pub team_strength_only: f64,
    pub player_profiles: f64,
    pub profiles_plus_pairs: f64,
    pub profiles_plus_pairs_trios: f64,
    pub full_matchup_manager: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerLineMatchupAblationObservation {
    pub schema: String,
    pub game_id: u64,
    pub season: u32,
    pub away_team: String,
    pub home_team: String,
    pub forecast_at: DateTime<Utc>,
    pub outcome_at: DateTime<Utc>,
    pub home_win: bool,
    pub probabilities: PlayerLineMatchupAblationProbabilities,
    pub forecast_fingerprint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prediction_fingerprint: Option<String>,
    pub outcome_fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerLineMatchupAblationPredictionInput {
    pub schema: String,
    pub game_id: u64,
    pub season: u32,
    pub away_team: String,
    pub home_team: String,
    pub forecast_at: DateTime<Utc>,
    pub probabilities: PlayerLineMatchupAblationProbabilities,
    pub forecast_fingerprint: String,
    pub source_fingerprint: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PlayerLineMatchupObservationError {
    #[error("matchup observation harvesting requires frozen forecasts")]
    MissingForecasts,
    #[error("invalid frozen matchup forecast for game {game_id}: {message}")]
    InvalidForecast { game_id: u64, message: String },
    #[error("duplicate frozen matchup forecast for season {season} game {game_id}")]
    DuplicateForecast { season: u32, game_id: u64 },
    #[error("invalid ablation prediction for game {game_id}: {message}")]
    InvalidPrediction { game_id: u64, message: String },
    #[error("duplicate ablation prediction for season {season} game {game_id}")]
    DuplicatePrediction { season: u32, game_id: u64 },
    #[error("invalid official outcome for game {game_id}: {message}")]
    InvalidOutcome { game_id: u64, message: String },
    #[error("duplicate official outcome for season {season} game {game_id}")]
    DuplicateOutcome { season: u32, game_id: u64 },
    #[error("frozen matchup game {game_id} has no ablation prediction")]
    MissingPrediction { game_id: u64 },
    #[error("frozen matchup game {game_id} has no official outcome")]
    MissingOutcome { game_id: u64 },
    #[error("ablation prediction does not exactly cite frozen matchup game {game_id}")]
    PredictionMismatch { game_id: u64 },
    #[error("official outcome for game {game_id} does not postdate the frozen forecast")]
    OutcomeBeforeForecast { game_id: u64 },
    #[error("ablation prediction input contains games absent from the frozen forecasts")]
    UnmatchedPredictions,
    #[error("official outcome input contains games absent from the frozen forecasts")]
    UnmatchedOutcomes,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerLineMatchupAblationMetric {
    pub stage: String,
    pub games: usize,
    pub brier_score: f64,
    pub log_loss: f64,
    pub brier_gain_vs_strength_only: f64,
    pub log_loss_gain_vs_strength_only: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerLineMatchupStabilityRow {
    pub scope: String,
    pub key: String,
    pub games: usize,
    pub full_brier_gain: f64,
    pub full_log_loss_gain: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerLineMatchupValidationView {
    pub schema: String,
    pub created_at: DateTime<Utc>,
    pub games: usize,
    pub metrics: Vec<PlayerLineMatchupAblationMetric>,
    pub stability: Vec<PlayerLineMatchupStabilityRow>,
    pub promotion_eligible: bool,
    pub disclosures: Vec<String>,
    pub source_fingerprints: Vec<String>,
    pub fingerprint: String,
}

pub fn build_player_line_matchup_ablation_observations(
    forecasts: &[PlayerLineMatchupForecastView],
    predictions: &[PlayerLineMatchupAblationPredictionInput],
    outcomes: &[TeamGamePredictionOutcomeInput],
    created_at: DateTime<Utc>,
) -> Result<Vec<PlayerLineMatchupAblationObservation>, PlayerLineMatchupObservationError> {
    if forecasts.is_empty() {
        return Err(PlayerLineMatchupObservationError::MissingForecasts);
    }
    let mut prediction_by_key = BTreeMap::new();
    for prediction in predictions {
        let sealed_prediction = prediction_fingerprint(prediction).map_err(|message| {
            PlayerLineMatchupObservationError::InvalidPrediction {
                game_id: prediction.game_id,
                message,
            }
        })?;
        if prediction.schema != PLAYER_LINE_MATCHUP_ABLATION_PREDICTION_SCHEMA
            || prediction.game_id == 0
            || prediction.season < 20_000_000
            || prediction.away_team == prediction.home_team
            || !canonical_team(&prediction.away_team)
            || !canonical_team(&prediction.home_team)
            || prediction.forecast_at > created_at
            || probabilities_from_values(prediction.probabilities)
                .iter()
                .any(|probability| !valid_probability(*probability))
            || !valid_fingerprint(&prediction.forecast_fingerprint)
            || !valid_fingerprint(&prediction.source_fingerprint)
            || prediction.fingerprint != sealed_prediction
        {
            return Err(PlayerLineMatchupObservationError::InvalidPrediction {
                game_id: prediction.game_id,
                message: "prediction identity, time, probabilities, or provenance is invalid"
                    .to_owned(),
            });
        }
        let key = (prediction.season, prediction.game_id);
        if prediction_by_key.insert(key, prediction).is_some() {
            return Err(PlayerLineMatchupObservationError::DuplicatePrediction {
                season: prediction.season,
                game_id: prediction.game_id,
            });
        }
    }
    let mut outcome_by_key = BTreeMap::new();
    for outcome in outcomes {
        if outcome.game_id == 0
            || outcome.season < 20_000_000
            || outcome.outcome_recorded_at > created_at
            || !valid_fingerprint(&outcome.source_fingerprint)
        {
            return Err(PlayerLineMatchupObservationError::InvalidOutcome {
                game_id: outcome.game_id,
                message: "outcome identity, time, or provenance is invalid".to_owned(),
            });
        }
        let key = (outcome.season, outcome.game_id);
        if outcome_by_key.insert(key, outcome).is_some() {
            return Err(PlayerLineMatchupObservationError::DuplicateOutcome {
                season: outcome.season,
                game_id: outcome.game_id,
            });
        }
    }
    let mut forecast_keys = BTreeSet::new();
    let mut observations = Vec::with_capacity(forecasts.len());
    for forecast in forecasts {
        validate_player_line_matchup_forecast(forecast).map_err(|message| {
            PlayerLineMatchupObservationError::InvalidForecast {
                game_id: forecast.game_id,
                message,
            }
        })?;
        let key = (forecast.season, forecast.game_id);
        if !forecast_keys.insert(key) {
            return Err(PlayerLineMatchupObservationError::DuplicateForecast {
                season: forecast.season,
                game_id: forecast.game_id,
            });
        }
        let prediction = prediction_by_key.get(&key).ok_or(
            PlayerLineMatchupObservationError::MissingPrediction {
                game_id: forecast.game_id,
            },
        )?;
        if prediction.away_team != forecast.away.team
            || prediction.home_team != forecast.home.team
            || prediction.forecast_at != forecast.forecast_at
            || prediction.forecast_fingerprint != forecast.fingerprint
        {
            return Err(PlayerLineMatchupObservationError::PredictionMismatch {
                game_id: forecast.game_id,
            });
        }
        let outcome =
            outcome_by_key
                .get(&key)
                .ok_or(PlayerLineMatchupObservationError::MissingOutcome {
                    game_id: forecast.game_id,
                })?;
        if outcome.outcome_recorded_at <= forecast.forecast_at {
            return Err(PlayerLineMatchupObservationError::OutcomeBeforeForecast {
                game_id: forecast.game_id,
            });
        }
        observations.push(PlayerLineMatchupAblationObservation {
            schema: PLAYER_LINE_MATCHUP_ABLATION_OBSERVATION_SCHEMA.to_owned(),
            game_id: forecast.game_id,
            season: forecast.season,
            away_team: forecast.away.team.clone(),
            home_team: forecast.home.team.clone(),
            forecast_at: forecast.forecast_at,
            outcome_at: outcome.outcome_recorded_at,
            home_win: outcome.home_won,
            probabilities: prediction.probabilities,
            forecast_fingerprint: forecast.fingerprint.clone(),
            prediction_fingerprint: Some(prediction.fingerprint.clone()),
            outcome_fingerprint: outcome.source_fingerprint.clone(),
        });
    }
    if forecast_keys.len() != prediction_by_key.len() {
        return Err(PlayerLineMatchupObservationError::UnmatchedPredictions);
    }
    if forecast_keys.len() != outcome_by_key.len() {
        return Err(PlayerLineMatchupObservationError::UnmatchedOutcomes);
    }
    observations.sort_by_key(|row| (row.season, row.game_id));
    Ok(observations)
}

pub fn seal_player_line_matchup_ablation_prediction(
    mut prediction: PlayerLineMatchupAblationPredictionInput,
) -> Result<PlayerLineMatchupAblationPredictionInput, String> {
    prediction.fingerprint.clear();
    prediction.fingerprint = prediction_fingerprint(&prediction)?;
    Ok(prediction)
}

pub fn build_player_line_matchup_validation(
    observations: Vec<PlayerLineMatchupAblationObservation>,
    created_at: DateTime<Utc>,
) -> Result<PlayerLineMatchupValidationView, String> {
    if observations.is_empty() {
        return Err("matchup validation requires chronological observations".into());
    }
    let mut game_ids = BTreeSet::new();
    let mut sources = BTreeSet::new();
    for row in &observations {
        if row.schema != PLAYER_LINE_MATCHUP_ABLATION_OBSERVATION_SCHEMA
            || row.game_id == 0
            || row.season < 20_000_000
            || row.away_team == row.home_team
            || !canonical_team(&row.away_team)
            || !canonical_team(&row.home_team)
            || row.forecast_at >= row.outcome_at
            || row.outcome_at > created_at
            || !game_ids.insert(row.game_id)
            || probabilities(row)
                .iter()
                .any(|probability| !valid_probability(*probability))
            || !valid_fingerprint(&row.forecast_fingerprint)
            || row
                .prediction_fingerprint
                .as_deref()
                .is_some_and(|value| !valid_fingerprint(value))
            || !valid_fingerprint(&row.outcome_fingerprint)
        {
            return Err(
                "matchup validation rejects invalid, duplicate, non-chronological, or unsealed observations"
                    .into(),
            );
        }
        sources.insert(row.forecast_fingerprint.clone());
        if let Some(fingerprint) = &row.prediction_fingerprint {
            sources.insert(fingerprint.clone());
        }
        sources.insert(row.outcome_fingerprint.clone());
    }
    let stages = [
        ("team_strength_only", 0usize),
        ("player_profiles", 1),
        ("profiles_plus_pairs", 2),
        ("profiles_plus_pairs_trios", 3),
        ("full_matchup_manager", 4),
    ];
    let baseline = score(&observations, 0);
    let metrics = stages
        .iter()
        .map(|(stage, index)| {
            let scored = score(&observations, *index);
            PlayerLineMatchupAblationMetric {
                stage: (*stage).to_owned(),
                games: observations.len(),
                brier_score: round9(scored.0),
                log_loss: round9(scored.1),
                brier_gain_vs_strength_only: round9(baseline.0 - scored.0),
                log_loss_gain_vs_strength_only: round9(baseline.1 - scored.1),
            }
        })
        .collect::<Vec<_>>();
    let mut stability = Vec::new();
    let mut seasons = BTreeSet::new();
    let mut teams = BTreeSet::new();
    for row in &observations {
        seasons.insert(row.season);
        teams.insert(row.away_team.clone());
        teams.insert(row.home_team.clone());
    }
    for season in seasons {
        let rows = observations
            .iter()
            .filter(|row| row.season != season)
            .collect::<Vec<_>>();
        if !rows.is_empty() {
            stability.push(stability_row(
                "leave_one_season_out",
                season.to_string(),
                rows,
            ));
        }
    }
    for team in teams {
        let rows = observations
            .iter()
            .filter(|row| row.away_team != team && row.home_team != team)
            .collect::<Vec<_>>();
        if !rows.is_empty() {
            stability.push(stability_row("leave_one_team_out", team, rows));
        }
    }
    let full = metrics.last().expect("five fixed ablations");
    let promotion_eligible = full.brier_gain_vs_strength_only > 0.0
        && full.log_loss_gain_vs_strength_only > 0.0
        && stability
            .iter()
            .all(|row| row.full_brier_gain >= 0.0 && row.full_log_loss_gain >= 0.0);
    let mut view = PlayerLineMatchupValidationView {
        schema: PLAYER_LINE_MATCHUP_VALIDATION_SCHEMA.to_owned(),
        created_at,
        games: observations.len(),
        metrics,
        stability,
        promotion_eligible,
        disclosures: vec![
            "Every forecast must predate its outcome; later outcomes are rejected rather than backfilled into the forecast vintage.".to_owned(),
            "Promotion requires pooled Brier and log-loss gains and non-negative full-model gains in every available leave-one-season-out and leave-one-team-out slice.".to_owned(),
            "This report evaluates sealed probabilities; coefficient fitting and holdout registration remain owned by the game-prediction trainer.".to_owned(),
        ],
        source_fingerprints: sources.into_iter().collect(),
        fingerprint: String::new(),
    };
    view.fingerprint = fingerprint(&view)?;
    Ok(view)
}

pub fn validate_player_line_matchup_validation(
    view: &PlayerLineMatchupValidationView,
) -> Result<(), String> {
    let stages = [
        "team_strength_only",
        "player_profiles",
        "profiles_plus_pairs",
        "profiles_plus_pairs_trios",
        "full_matchup_manager",
    ];
    if view.schema != PLAYER_LINE_MATCHUP_VALIDATION_SCHEMA
        || view.games == 0
        || view.metrics.len() != stages.len()
        || view.source_fingerprints.is_empty()
        || view
            .source_fingerprints
            .iter()
            .any(|value| !valid_fingerprint(value))
        || view.metrics.iter().zip(stages).any(|(row, stage)| {
            row.stage != stage
                || row.games != view.games
                || !row.brier_score.is_finite()
                || !(0.0..=1.0).contains(&row.brier_score)
                || !row.log_loss.is_finite()
                || row.log_loss < 0.0
                || !row.brier_gain_vs_strength_only.is_finite()
                || !row.log_loss_gain_vs_strength_only.is_finite()
        })
        || view.stability.iter().any(|row| {
            !matches!(
                row.scope.as_str(),
                "leave_one_season_out" | "leave_one_team_out"
            ) || row.key.is_empty()
                || row.games == 0
                || !row.full_brier_gain.is_finite()
                || !row.full_log_loss_gain.is_finite()
        })
    {
        return Err("invalid player-line matchup validation report".into());
    }
    let full = view.metrics.last().expect("five validated metrics");
    let eligible = full.brier_gain_vs_strength_only > 0.0
        && full.log_loss_gain_vs_strength_only > 0.0
        && view
            .stability
            .iter()
            .all(|row| row.full_brier_gain >= 0.0 && row.full_log_loss_gain >= 0.0);
    if eligible != view.promotion_eligible || fingerprint(view)? != view.fingerprint {
        return Err("tampered player-line matchup validation report".into());
    }
    Ok(())
}

fn probabilities(row: &PlayerLineMatchupAblationObservation) -> [f64; 5] {
    probabilities_from_values(row.probabilities)
}

fn probabilities_from_values(probabilities: PlayerLineMatchupAblationProbabilities) -> [f64; 5] {
    [
        probabilities.team_strength_only,
        probabilities.player_profiles,
        probabilities.profiles_plus_pairs,
        probabilities.profiles_plus_pairs_trios,
        probabilities.full_matchup_manager,
    ]
}

fn score(rows: &[PlayerLineMatchupAblationObservation], index: usize) -> (f64, f64) {
    let games = rows.len() as f64;
    let sum = rows
        .iter()
        .map(|row| {
            let probability = probabilities(row)[index];
            let outcome = if row.home_win { 1.0 } else { 0.0 };
            let brier = (probability - outcome).powi(2);
            let probability = probability.clamp(1e-12, 1.0 - 1e-12);
            let log_loss =
                -(outcome * probability.ln() + (1.0 - outcome) * (1.0 - probability).ln());
            (brier, log_loss)
        })
        .fold((0.0, 0.0), |sum, row| (sum.0 + row.0, sum.1 + row.1));
    (sum.0 / games, sum.1 / games)
}

fn stability_row(
    scope: &str,
    key: String,
    rows: Vec<&PlayerLineMatchupAblationObservation>,
) -> PlayerLineMatchupStabilityRow {
    let owned = rows.into_iter().cloned().collect::<Vec<_>>();
    let baseline = score(&owned, 0);
    let full = score(&owned, 4);
    PlayerLineMatchupStabilityRow {
        scope: scope.to_owned(),
        key,
        games: owned.len(),
        full_brier_gain: round9(baseline.0 - full.0),
        full_log_loss_gain: round9(baseline.1 - full.1),
    }
}

fn valid_probability(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn canonical_team(team: &str) -> bool {
    CANONICAL_TEAMS.iter().any(|(abbr, _)| *abbr == team)
}

fn valid_fingerprint(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn prediction_fingerprint(
    prediction: &PlayerLineMatchupAblationPredictionInput,
) -> Result<String, String> {
    let mut canonical = prediction.clone();
    canonical.fingerprint.clear();
    let bytes = serde_json::to_vec(&canonical).map_err(|error| error.to_string())?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn fingerprint(value: &PlayerLineMatchupValidationView) -> Result<String, String> {
    let mut material = serde_json::to_value(value).map_err(|error| error.to_string())?;
    if let Some(object) = material.as_object_mut() {
        object.insert(
            "fingerprint".to_owned(),
            serde_json::Value::String(String::new()),
        );
    }
    let bytes = serde_json::to_vec(&material).map_err(|error| error.to_string())?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn round9(value: f64) -> f64 {
    (value * 1_000_000_000.0).round() / 1_000_000_000.0
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn seal(character: char) -> String {
        format!("sha256:{}", character.to_string().repeat(64))
    }

    fn observation(game_id: u64, home_win: bool) -> PlayerLineMatchupAblationObservation {
        let good = if home_win { 0.75 } else { 0.25 };
        PlayerLineMatchupAblationObservation {
            schema: PLAYER_LINE_MATCHUP_ABLATION_OBSERVATION_SCHEMA.to_owned(),
            game_id,
            season: 20252026,
            away_team: "SEA".to_owned(),
            home_team: "NYR".to_owned(),
            forecast_at: Utc.with_ymd_and_hms(2026, 3, 1, 12, 0, 0).unwrap(),
            outcome_at: Utc.with_ymd_and_hms(2026, 3, 2, 4, 0, 0).unwrap(),
            home_win,
            probabilities: PlayerLineMatchupAblationProbabilities {
                team_strength_only: 0.5,
                player_profiles: if home_win { 0.58 } else { 0.42 },
                profiles_plus_pairs: if home_win { 0.64 } else { 0.36 },
                profiles_plus_pairs_trios: if home_win { 0.7 } else { 0.3 },
                full_matchup_manager: good,
            },
            forecast_fingerprint: seal('a'),
            prediction_fingerprint: None,
            outcome_fingerprint: seal('b'),
        }
    }

    fn forecast() -> PlayerLineMatchupForecastView {
        serde_json::from_str(include_str!(
            "../../../examples/player-line-matchup-forecast-alp-vs-brv-2026-27.json"
        ))
        .unwrap()
    }

    fn prediction(
        forecast: &PlayerLineMatchupForecastView,
    ) -> PlayerLineMatchupAblationPredictionInput {
        seal_player_line_matchup_ablation_prediction(PlayerLineMatchupAblationPredictionInput {
            schema: PLAYER_LINE_MATCHUP_ABLATION_PREDICTION_SCHEMA.to_owned(),
            game_id: forecast.game_id,
            season: forecast.season,
            away_team: forecast.away.team.clone(),
            home_team: forecast.home.team.clone(),
            forecast_at: forecast.forecast_at,
            probabilities: PlayerLineMatchupAblationProbabilities {
                team_strength_only: 0.5,
                player_profiles: 0.54,
                profiles_plus_pairs: 0.56,
                profiles_plus_pairs_trios: 0.58,
                full_matchup_manager: 0.6,
            },
            forecast_fingerprint: forecast.fingerprint.clone(),
            source_fingerprint: seal('c'),
            fingerprint: String::new(),
        })
        .unwrap()
    }

    fn outcome(forecast: &PlayerLineMatchupForecastView) -> TeamGamePredictionOutcomeInput {
        TeamGamePredictionOutcomeInput {
            season: forecast.season,
            game_id: forecast.game_id,
            outcome_recorded_at: forecast.forecast_at + chrono::Duration::hours(6),
            home_won: true,
            source_fingerprint: seal('d'),
        }
    }

    #[test]
    fn sealed_inputs_harvest_one_chronological_observation() {
        let forecast = forecast();
        let prediction = prediction(&forecast);
        let outcome = outcome(&forecast);
        let rows = build_player_line_matchup_ablation_observations(
            std::slice::from_ref(&forecast),
            std::slice::from_ref(&prediction),
            std::slice::from_ref(&outcome),
            outcome.outcome_recorded_at,
        )
        .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].forecast_fingerprint, forecast.fingerprint);
        assert_eq!(
            rows[0].prediction_fingerprint.as_deref(),
            Some(prediction.fingerprint.as_str())
        );
        assert_eq!(rows[0].outcome_fingerprint, outcome.source_fingerprint);
        let report =
            build_player_line_matchup_validation(rows, outcome.outcome_recorded_at).unwrap();
        assert_eq!(report.source_fingerprints.len(), 3);
    }

    #[test]
    fn legacy_observation_without_prediction_fingerprint_remains_readable() {
        let mut value = serde_json::to_value(observation(1, true)).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .remove("prediction_fingerprint");

        let row: PlayerLineMatchupAblationObservation = serde_json::from_value(value).unwrap();

        assert_eq!(row.prediction_fingerprint, None);
    }

    #[test]
    fn mutated_prediction_probabilities_fail_the_prediction_seal() {
        let forecast = forecast();
        let mut prediction = prediction(&forecast);
        prediction.probabilities.full_matchup_manager = 0.99;
        let outcome = outcome(&forecast);

        assert!(matches!(
            build_player_line_matchup_ablation_observations(
                std::slice::from_ref(&forecast),
                std::slice::from_ref(&prediction),
                std::slice::from_ref(&outcome),
                outcome.outcome_recorded_at,
            ),
            Err(PlayerLineMatchupObservationError::InvalidPrediction { .. })
        ));
    }

    #[test]
    fn mismatched_prediction_identity_and_early_outcome_fail_explicitly() {
        let forecast = forecast();
        let mut mismatched_prediction = prediction(&forecast);
        mismatched_prediction.home_team = "BOS".to_owned();
        let mismatched_prediction =
            seal_player_line_matchup_ablation_prediction(mismatched_prediction).unwrap();
        let official_outcome = outcome(&forecast);
        assert_eq!(
            build_player_line_matchup_ablation_observations(
                std::slice::from_ref(&forecast),
                std::slice::from_ref(&mismatched_prediction),
                std::slice::from_ref(&official_outcome),
                official_outcome.outcome_recorded_at,
            ),
            Err(PlayerLineMatchupObservationError::PredictionMismatch {
                game_id: forecast.game_id
            })
        );

        let prediction = prediction(&forecast);
        let mut outcome = outcome(&forecast);
        outcome.outcome_recorded_at = forecast.forecast_at;
        assert_eq!(
            build_player_line_matchup_ablation_observations(
                std::slice::from_ref(&forecast),
                std::slice::from_ref(&prediction),
                std::slice::from_ref(&outcome),
                forecast.forecast_at,
            ),
            Err(PlayerLineMatchupObservationError::OutcomeBeforeForecast {
                game_id: forecast.game_id
            })
        );
    }

    #[test]
    fn chronological_ablation_can_promote_stable_matchup_signal() {
        let created_at = Utc.with_ymd_and_hms(2026, 3, 3, 12, 0, 0).unwrap();
        let view = build_player_line_matchup_validation(
            vec![observation(1, true), observation(2, false)],
            created_at,
        )
        .expect("chronological observations");
        assert_eq!(view.metrics.len(), 5);
        assert!(view.promotion_eligible);
        validate_player_line_matchup_validation(&view).unwrap();
        assert!(view
            .metrics
            .windows(2)
            .all(|rows| rows[1].brier_score < rows[0].brier_score));
    }

    #[test]
    fn outcome_before_forecast_or_after_report_is_rejected() {
        let mut row = observation(1, true);
        row.outcome_at = row.forecast_at;
        let created_at = Utc.with_ymd_and_hms(2026, 3, 3, 12, 0, 0).unwrap();
        assert!(build_player_line_matchup_validation(vec![row], created_at).is_err());
    }

    #[test]
    fn pooled_gain_cannot_hide_leave_one_team_out_instability() {
        let first = observation(1, true);
        let mut second = observation(2, false);
        second.away_team = "BOS".to_owned();
        second.home_team = "TOR".to_owned();
        let mut unstable = observation(3, false);
        unstable.away_team = "EDM".to_owned();
        unstable.home_team = "VAN".to_owned();
        unstable.probabilities.full_matchup_manager = 0.75;
        let created_at = Utc.with_ymd_and_hms(2026, 3, 3, 12, 0, 0).unwrap();
        let view = build_player_line_matchup_validation(vec![first, second, unstable], created_at)
            .unwrap();
        assert!(view.metrics.last().unwrap().brier_gain_vs_strength_only > 0.0);
        assert!(!view.promotion_eligible);
        assert!(view
            .stability
            .iter()
            .any(|row| { row.scope == "leave_one_team_out" && row.full_brier_gain < 0.0 }));
    }
}
