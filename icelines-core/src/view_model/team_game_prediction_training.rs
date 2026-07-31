use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::team_game_prediction_edge::{
    TeamGameForecastVintage, TeamGamePredictionEdgeGameRow, TeamGamePredictionEdgeView,
    TeamGamePredictionModel, TeamGamePredictionModelAuthority, TEAM_GAME_PREDICTION_EDGE_METHOD,
};

pub const TEAM_GAME_PREDICTION_TRAINING_SCHEMA: &str = "team_game_prediction_training.v1";
pub const TEAM_GAME_PREDICTION_VALIDATION_SCHEMA: &str = "team_game_prediction_validation.v1";
pub const TEAM_GAME_PREDICTION_OBSERVATIONS_SCHEMA: &str = "team_game_prediction_observations.v1";
pub const TEAM_GAME_PREDICTION_HOLDOUT_REGISTRATION_SCHEMA: &str =
    "team_game_prediction_holdout_registration.v1";
pub const TEAM_GAME_PREDICTION_HOLDOUT_REGISTRATION_JSON_SCHEMA: &str = include_str!(
    "../../../design/schemas/team_game_prediction_holdout_registration.v1.schema.json"
);
pub const TEAM_GAME_PREDICTION_OBSERVATIONS_JSON_SCHEMA: &str =
    include_str!("../../../design/schemas/team_game_prediction_observations.v1.schema.json");
pub const TEAM_GAME_PREDICTION_TRAINING_JSON_SCHEMA: &str =
    include_str!("../../../design/schemas/team_game_prediction_training.v1.schema.json");
pub const TEAM_GAME_PREDICTION_VALIDATION_JSON_SCHEMA: &str =
    include_str!("../../../design/schemas/team_game_prediction_validation.v1.schema.json");

const FEATURE_KEYS: [&str; 11] = [
    "roster",
    "availability",
    "lineup_impact",
    "goalie",
    "goalie_schedule",
    "goalie_form",
    "goalie_workload",
    "xg_form",
    "opponent_adjusted_xg",
    "special_teams",
    "matchup",
];
pub const TEAM_GAME_PREDICTION_FEATURE_SET_V1: &str = "edge-core-v1";
pub const TEAM_GAME_PREDICTION_FEATURE_SET_V2: &str = "edge-core-v2-goalie-form";
pub const TEAM_GAME_PREDICTION_FEATURE_SET_V3: &str = "edge-core-v3-lineup-impact";
pub const TEAM_GAME_PREDICTION_FEATURE_SET_V4: &str = "edge-core-v4-goalie-schedule";
pub const TEAM_GAME_PREDICTION_FEATURE_SET_V5: &str = "edge-core-v5-opponent-adjusted-xg";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGamePredictionTrainingObservation {
    pub season: u32,
    pub game_id: u64,
    pub away_team: String,
    pub home_team: String,
    pub vintage: TeamGameForecastVintage,
    pub forecast_at: DateTime<Utc>,
    pub outcome_recorded_at: DateTime<Utc>,
    pub home_won: bool,
    pub baseline_home_probability: f64,
    pub elo_home_probability: f64,
    pub roster_difference: Option<f64>,
    pub availability_difference: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lineup_impact_difference: Option<f64>,
    pub goalie_difference: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goalie_schedule_difference: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goalie_form_difference: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goalie_workload_difference: Option<f64>,
    pub xg_difference: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opponent_adjusted_xg_difference: Option<f64>,
    pub special_teams_difference: Option<f64>,
    pub matchup_difference: Option<f64>,
    pub source_fingerprints: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGamePredictionOutcomeInput {
    pub season: u32,
    pub game_id: u64,
    pub outcome_recorded_at: DateTime<Utc>,
    pub home_won: bool,
    pub source_fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGamePredictionObservationSet {
    pub schema: String,
    pub vintage: TeamGameForecastVintage,
    pub created_at: DateTime<Utc>,
    pub seasons: Vec<u32>,
    pub edge_fingerprints: Vec<String>,
    pub outcome_source_fingerprints: Vec<String>,
    pub observations: Vec<TeamGamePredictionTrainingObservation>,
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGamePredictionHoldoutRegistration {
    pub schema: String,
    pub holdout_season: u32,
    pub registered_at: DateTime<Utc>,
    pub outcome_not_before: DateTime<Utc>,
    pub config_fingerprint: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGamePredictionTrainingConfig {
    pub model_id: String,
    /// Seals the exact feature vocabulary so prospective registrations cannot
    /// silently authorize a later model design.
    #[serde(
        default = "default_legacy_feature_set",
        skip_serializing_if = "is_legacy_feature_set"
    )]
    pub feature_set: String,
    pub vintage: TeamGameForecastVintage,
    pub minimum_training_seasons: usize,
    pub minimum_holdout_seasons: usize,
    pub minimum_brier_gain: f64,
    pub minimum_improved_holdouts: usize,
    pub l2_penalty: f64,
    pub learning_rate: f64,
    pub iterations: usize,
    pub elo_weight_grid: Vec<f64>,
    pub coefficient_prune_threshold: f64,
    pub minimum_team_coverage: usize,
    pub maximum_team_game_share: f64,
    pub minimum_roster_coverage: f64,
    pub minimum_goalie_coverage: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub minimum_candidate_feature_gain: f64,
}

impl Default for TeamGamePredictionTrainingConfig {
    fn default() -> Self {
        Self {
            model_id: "icecast-edge-trained-v1".to_owned(),
            feature_set: TEAM_GAME_PREDICTION_FEATURE_SET_V1.to_owned(),
            vintage: TeamGameForecastVintage::PregameConfirmed,
            minimum_training_seasons: 2,
            minimum_holdout_seasons: 5,
            minimum_brier_gain: 0.001,
            minimum_improved_holdouts: 4,
            l2_penalty: 0.02,
            learning_rate: 0.05,
            iterations: 2_000,
            elo_weight_grid: (0..=10).map(|step| step as f64 / 10.0).collect(),
            coefficient_prune_threshold: 0.01,
            minimum_team_coverage: 16,
            maximum_team_game_share: 0.20,
            minimum_roster_coverage: 0.80,
            minimum_goalie_coverage: 0.60,
            minimum_candidate_feature_gain: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGamePredictionTrainingView {
    pub schema: String,
    pub config: TeamGamePredictionTrainingConfig,
    pub observations: usize,
    pub seasons: Vec<u32>,
    pub model: TeamGamePredictionModel,
    pub pruned_features: Vec<String>,
    pub input_fingerprint: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGamePredictionHoldoutRow {
    pub season: u32,
    pub training_seasons: Vec<u32>,
    pub games: usize,
    pub model_id: String,
    pub elo_weight: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub calibration_intercept: f64,
    #[serde(default = "default_calibration_slope", skip_serializing_if = "is_one")]
    pub calibration_slope: f64,
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub calibration_games: usize,
    pub candidate_brier: f64,
    pub elo_brier: f64,
    pub brier_gain: f64,
    pub candidate_log_loss: f64,
    pub elo_log_loss: f64,
    pub log_loss_gain: f64,
    pub candidate_ece: f64,
    pub elo_ece: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGamePredictionAblationRow {
    pub feature: String,
    pub candidate_brier: f64,
    pub ablated_brier: f64,
    pub included_feature_gain: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGamePredictionPromotionCheck {
    pub key: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGamePredictionValidationView {
    pub schema: String,
    pub vintage: TeamGameForecastVintage,
    pub holdouts: Vec<TeamGamePredictionHoldoutRow>,
    pub games: usize,
    pub candidate_brier: f64,
    pub elo_brier: f64,
    pub brier_gain: f64,
    pub candidate_log_loss: f64,
    pub elo_log_loss: f64,
    pub log_loss_gain: f64,
    pub candidate_ece: f64,
    pub elo_ece: f64,
    pub improved_holdouts: usize,
    pub ablations: Vec<TeamGamePredictionAblationRow>,
    pub checks: Vec<TeamGamePredictionPromotionCheck>,
    pub promotion_passed: bool,
    pub final_model: TeamGamePredictionModel,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prospective_registration_fingerprint: Option<String>,
    pub input_fingerprint: String,
    pub fingerprint: String,
}

fn is_zero(value: &f64) -> bool {
    *value == 0.0
}

fn is_one(value: &f64) -> bool {
    *value == 1.0
}

fn default_calibration_slope() -> f64 {
    1.0
}

fn is_zero_usize(value: &usize) -> bool {
    *value == 0
}

fn default_legacy_feature_set() -> String {
    TEAM_GAME_PREDICTION_FEATURE_SET_V1.to_owned()
}

fn is_legacy_feature_set(value: &String) -> bool {
    value == TEAM_GAME_PREDICTION_FEATURE_SET_V1
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TeamGamePredictionTrainingError {
    #[error("invalid training configuration: {0}")]
    InvalidConfig(String),
    #[error("invalid training observation for game {game_id}: {message}")]
    InvalidObservation { game_id: u64, message: String },
    #[error("not enough seasons for rolling validation")]
    InsufficientSeasons,
    #[error("prediction training serialization failed: {0}")]
    Serialization(String),
    #[error("prediction training fingerprint mismatch")]
    FingerprintMismatch,
    #[error("invalid frozen prediction edge: {0}")]
    InvalidEdge(String),
}

pub fn register_team_game_prediction_holdout(
    config: &TeamGamePredictionTrainingConfig,
    holdout_season: u32,
    registered_at: DateTime<Utc>,
    outcome_not_before: DateTime<Utc>,
) -> Result<TeamGamePredictionHoldoutRegistration, TeamGamePredictionTrainingError> {
    validate_config(config)?;
    if holdout_season < 20_000_000 || outcome_not_before <= registered_at {
        return Err(TeamGamePredictionTrainingError::InvalidConfig(
            "prospective holdout requires a valid season and later outcome boundary".to_owned(),
        ));
    }
    let mut registration = TeamGamePredictionHoldoutRegistration {
        schema: TEAM_GAME_PREDICTION_HOLDOUT_REGISTRATION_SCHEMA.to_owned(),
        holdout_season,
        registered_at,
        outcome_not_before,
        config_fingerprint: fingerprint(config)?,
        fingerprint: String::new(),
    };
    registration.fingerprint = fingerprint(&registration)?;
    Ok(registration)
}

impl TeamGamePredictionHoldoutRegistration {
    pub fn validate(
        &self,
        config: &TeamGamePredictionTrainingConfig,
    ) -> Result<(), TeamGamePredictionTrainingError> {
        if self.schema != TEAM_GAME_PREDICTION_HOLDOUT_REGISTRATION_SCHEMA
            || self.holdout_season < 20_000_000
            || self.outcome_not_before <= self.registered_at
            || self.config_fingerprint != fingerprint(config)?
            || self.fingerprint != fingerprint(self)?
        {
            return Err(TeamGamePredictionTrainingError::FingerprintMismatch);
        }
        Ok(())
    }
}

/// Join a final outcome to one already-sealed edge row without rebuilding or
/// rereading any prediction feature. This is the only supported observation
/// bridge for historical training data.
pub fn build_team_game_prediction_training_observation(
    edge: &TeamGamePredictionEdgeView,
    game_id: u64,
    outcome_recorded_at: DateTime<Utc>,
    home_won: bool,
) -> Result<TeamGamePredictionTrainingObservation, TeamGamePredictionTrainingError> {
    edge.validate()
        .map_err(|error| TeamGamePredictionTrainingError::InvalidEdge(error.to_string()))?;
    let game = edge
        .games
        .iter()
        .find(|game| game.game_id == game_id)
        .ok_or(TeamGamePredictionTrainingError::InvalidObservation {
            game_id,
            message: "game is absent from the frozen edge".to_owned(),
        })?;
    build_training_observation_from_game(edge, game, outcome_recorded_at, home_won)
}

fn build_training_observation_from_game(
    edge: &TeamGamePredictionEdgeView,
    game: &TeamGamePredictionEdgeGameRow,
    outcome_recorded_at: DateTime<Utc>,
    home_won: bool,
) -> Result<TeamGamePredictionTrainingObservation, TeamGamePredictionTrainingError> {
    let game_id = game.game_id;
    let forecast_at =
        game.forecast_at
            .ok_or(TeamGamePredictionTrainingError::InvalidObservation {
                game_id,
                message: "game has no frozen forecast timestamp".to_owned(),
            })?;
    let feature = |key: &str| {
        game.factors
            .iter()
            .find(|factor| factor.key == key)
            .and_then(|factor| factor.effective_home_minus_away)
    };
    let observation = TeamGamePredictionTrainingObservation {
        season: edge.season,
        game_id,
        away_team: game.away_team.clone(),
        home_team: game.home_team.clone(),
        vintage: edge.vintage,
        forecast_at,
        outcome_recorded_at,
        home_won,
        baseline_home_probability: game.base_home_win_probability,
        elo_home_probability: game.elo_home_win_probability,
        roster_difference: feature("roster"),
        availability_difference: feature("availability"),
        lineup_impact_difference: feature("lineup_impact"),
        goalie_difference: feature("goalie"),
        goalie_schedule_difference: feature("goalie_schedule"),
        goalie_form_difference: feature("goalie_form"),
        goalie_workload_difference: feature("goalie_workload"),
        xg_difference: feature("xg_form"),
        opponent_adjusted_xg_difference: feature("opponent_adjusted_xg"),
        special_teams_difference: feature("special_teams"),
        matchup_difference: feature("matchup"),
        source_fingerprints: vec![
            edge.source_forecast_fingerprint.clone(),
            edge.fingerprint.clone(),
        ],
    };
    validate_observations(std::slice::from_ref(&observation), edge.vintage)?;
    Ok(observation)
}

pub fn build_team_game_prediction_observation_set(
    edges: &[TeamGamePredictionEdgeView],
    outcomes: &[TeamGamePredictionOutcomeInput],
    created_at: DateTime<Utc>,
) -> Result<TeamGamePredictionObservationSet, TeamGamePredictionTrainingError> {
    let vintage = edges
        .first()
        .map(|edge| edge.vintage)
        .ok_or(TeamGamePredictionTrainingError::InsufficientSeasons)?;
    if edges.iter().any(|edge| edge.vintage != vintage) {
        return Err(TeamGamePredictionTrainingError::InvalidEdge(
            "observation sets cannot mix forecast vintages".to_owned(),
        ));
    }
    let mut outcome_by_key = std::collections::BTreeMap::new();
    for outcome in outcomes {
        if outcome.outcome_recorded_at > created_at
            || !valid_sha256(&outcome.source_fingerprint)
            || outcome_by_key
                .insert((outcome.season, outcome.game_id), outcome)
                .is_some()
        {
            return Err(TeamGamePredictionTrainingError::InvalidObservation {
                game_id: outcome.game_id,
                message: "outcome is duplicated or postdates observation-set creation".to_owned(),
            });
        }
    }
    let mut observations = Vec::new();
    let mut consumed = BTreeSet::new();
    for edge in edges {
        edge.validate()
            .map_err(|error| TeamGamePredictionTrainingError::InvalidEdge(error.to_string()))?;
        for game in &edge.games {
            let key = (edge.season, game.game_id);
            let outcome = outcome_by_key.get(&key).ok_or(
                TeamGamePredictionTrainingError::InvalidObservation {
                    game_id: game.game_id,
                    message: "frozen edge game has no separately joined outcome".to_owned(),
                },
            )?;
            observations.push(build_training_observation_from_game(
                edge,
                game,
                outcome.outcome_recorded_at,
                outcome.home_won,
            )?);
            consumed.insert(key);
        }
    }
    if consumed.len() != outcome_by_key.len() {
        return Err(TeamGamePredictionTrainingError::InvalidEdge(
            "outcome input contains games absent from the frozen edges".to_owned(),
        ));
    }
    observations.sort_by_key(|row| (row.season, row.game_id));
    validate_observations(&observations, vintage)?;
    let seasons = unique_seasons(&observations);
    let mut edge_fingerprints = edges
        .iter()
        .map(|edge| edge.fingerprint.clone())
        .collect::<Vec<_>>();
    edge_fingerprints.sort();
    let outcome_source_fingerprints = outcomes
        .iter()
        .map(|outcome| outcome.source_fingerprint.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut set = TeamGamePredictionObservationSet {
        schema: TEAM_GAME_PREDICTION_OBSERVATIONS_SCHEMA.to_owned(),
        vintage,
        created_at,
        seasons,
        edge_fingerprints,
        outcome_source_fingerprints,
        observations,
        fingerprint: String::new(),
    };
    set.fingerprint = fingerprint(&set)?;
    Ok(set)
}

impl TeamGamePredictionObservationSet {
    pub fn validate(&self) -> Result<(), TeamGamePredictionTrainingError> {
        if self.schema != TEAM_GAME_PREDICTION_OBSERVATIONS_SCHEMA
            || self.observations.is_empty()
            || self.edge_fingerprints.is_empty()
            || self.outcome_source_fingerprints.is_empty()
            || self
                .edge_fingerprints
                .iter()
                .any(|fingerprint| !valid_sha256(fingerprint))
            || self
                .outcome_source_fingerprints
                .iter()
                .any(|fingerprint| !valid_sha256(fingerprint))
            || self.seasons != unique_seasons(&self.observations)
            || self.fingerprint != fingerprint(self)?
        {
            return Err(TeamGamePredictionTrainingError::FingerprintMismatch);
        }
        validate_observations(&self.observations, self.vintage)?;
        if self
            .observations
            .iter()
            .any(|row| row.outcome_recorded_at > self.created_at)
        {
            return Err(TeamGamePredictionTrainingError::InvalidEdge(
                "observation outcome postdates set creation".to_owned(),
            ));
        }
        Ok(())
    }
}

pub fn train_team_game_prediction_model(
    observations: &[TeamGamePredictionTrainingObservation],
    config: TeamGamePredictionTrainingConfig,
) -> Result<TeamGamePredictionTrainingView, TeamGamePredictionTrainingError> {
    validate_config(&config)?;
    validate_observations(observations, config.vintage)?;
    let seasons = unique_seasons(observations);
    if seasons.len() < config.minimum_training_seasons {
        return Err(TeamGamePredictionTrainingError::InsufficientSeasons);
    }
    let mut model = fit_best_model(observations, &config);
    let pruned_features = prune_model(&mut model, config.coefficient_prune_threshold);
    model
        .validate()
        .map_err(|error| TeamGamePredictionTrainingError::InvalidConfig(error.to_string()))?;
    let input_fingerprint = fingerprint(&(observations, &config))?;
    let mut view = TeamGamePredictionTrainingView {
        schema: TEAM_GAME_PREDICTION_TRAINING_SCHEMA.to_owned(),
        config,
        observations: observations.len(),
        seasons,
        model,
        pruned_features,
        input_fingerprint,
        fingerprint: String::new(),
    };
    view.fingerprint = fingerprint(&view)?;
    Ok(view)
}

pub fn validate_team_game_prediction_model(
    observations: &[TeamGamePredictionTrainingObservation],
    config: TeamGamePredictionTrainingConfig,
) -> Result<TeamGamePredictionValidationView, TeamGamePredictionTrainingError> {
    validate_team_game_prediction_model_with_registration(observations, config, None)
}

pub fn validate_team_game_prediction_model_with_registration(
    observations: &[TeamGamePredictionTrainingObservation],
    config: TeamGamePredictionTrainingConfig,
    registration: Option<&TeamGamePredictionHoldoutRegistration>,
) -> Result<TeamGamePredictionValidationView, TeamGamePredictionTrainingError> {
    validate_config(&config)?;
    validate_observations(observations, config.vintage)?;
    if let Some(registration) = registration {
        registration.validate(&config)?;
    }
    let seasons = unique_seasons(observations);
    if seasons.len() <= config.minimum_training_seasons {
        return Err(TeamGamePredictionTrainingError::InsufficientSeasons);
    }
    let mut holdouts = Vec::new();
    let mut predictions = Vec::new();
    let mut calibration_history = Vec::<(f64, bool)>::new();
    for (index, holdout_season) in seasons.iter().copied().enumerate() {
        if index < config.minimum_training_seasons {
            continue;
        }
        let training = observations
            .iter()
            .filter(|row| row.season < holdout_season)
            .cloned()
            .collect::<Vec<_>>();
        let holdout = observations
            .iter()
            .filter(|row| row.season == holdout_season)
            .collect::<Vec<_>>();
        if training.is_empty() || holdout.is_empty() {
            continue;
        }
        let mut model = fit_best_model(&training, &config);
        prune_model(&mut model, config.coefficient_prune_threshold);
        let (calibration_intercept, calibration_slope) =
            select_chronological_calibration(&calibration_history);
        model.calibration_intercept = calibration_intercept;
        model.calibration_slope = calibration_slope;
        let mut season_predictions = Vec::with_capacity(holdout.len());
        for observation in holdout {
            let raw_candidate = predict_uncalibrated(&model, observation, None);
            let candidate = predict(&model, observation, None);
            season_predictions.push(ScoredPrediction {
                observation: observation.clone(),
                model: model.clone(),
                candidate,
                elo: observation.elo_home_probability,
            });
            calibration_history.push((raw_candidate, observation.home_won));
        }
        let candidate_metrics = metrics(
            season_predictions
                .iter()
                .map(|row| (row.candidate, row.observation.home_won)),
        );
        let elo_metrics = metrics(
            season_predictions
                .iter()
                .map(|row| (row.elo, row.observation.home_won)),
        );
        holdouts.push(TeamGamePredictionHoldoutRow {
            season: holdout_season,
            training_seasons: unique_seasons(&training),
            games: season_predictions.len(),
            model_id: model.model_id.clone(),
            elo_weight: model.elo_weight,
            calibration_intercept,
            calibration_slope,
            calibration_games: calibration_history.len() - season_predictions.len(),
            candidate_brier: candidate_metrics.brier,
            elo_brier: elo_metrics.brier,
            brier_gain: elo_metrics.brier - candidate_metrics.brier,
            candidate_log_loss: candidate_metrics.log_loss,
            elo_log_loss: elo_metrics.log_loss,
            log_loss_gain: elo_metrics.log_loss - candidate_metrics.log_loss,
            candidate_ece: candidate_metrics.ece,
            elo_ece: elo_metrics.ece,
        });
        predictions.extend(season_predictions);
    }
    if predictions.is_empty() {
        return Err(TeamGamePredictionTrainingError::InsufficientSeasons);
    }
    let candidate_metrics = metrics(
        predictions
            .iter()
            .map(|row| (row.candidate, row.observation.home_won)),
    );
    let elo_metrics = metrics(
        predictions
            .iter()
            .map(|row| (row.elo, row.observation.home_won)),
    );
    let ablations = FEATURE_KEYS
        .into_iter()
        .map(|feature| {
            let ablated = metrics(predictions.iter().map(|row| {
                (
                    predict(&row.model, &row.observation, Some(feature)),
                    row.observation.home_won,
                )
            }));
            TeamGamePredictionAblationRow {
                feature: feature.to_owned(),
                candidate_brier: candidate_metrics.brier,
                ablated_brier: ablated.brier,
                included_feature_gain: ablated.brier - candidate_metrics.brier,
            }
        })
        .collect::<Vec<_>>();
    let improved_holdouts = holdouts.iter().filter(|row| row.brier_gain > 0.0).count();
    let roster_ablation = ablations
        .iter()
        .find(|row| row.feature == "roster")
        .expect("roster ablation");
    let goalie_ablation = ablations
        .iter()
        .find(|row| row.feature == "goalie")
        .expect("goalie ablation");
    let candidate_features: &[&str] = match config.feature_set.as_str() {
        TEAM_GAME_PREDICTION_FEATURE_SET_V2 => &["goalie_form", "goalie_workload"],
        TEAM_GAME_PREDICTION_FEATURE_SET_V3 => &["lineup_impact"],
        TEAM_GAME_PREDICTION_FEATURE_SET_V4 => &["goalie_schedule"],
        TEAM_GAME_PREDICTION_FEATURE_SET_V5 => &["opponent_adjusted_xg"],
        _ => &[],
    };
    let candidate_feature_gain = candidate_features
        .iter()
        .map(|feature| {
            ablations
                .iter()
                .find(|row| row.feature == *feature)
                .expect("candidate feature ablation")
                .included_feature_gain
        })
        .fold(f64::INFINITY, f64::min);
    let roster_coverage = observations
        .iter()
        .filter(|row| row.roster_difference.is_some())
        .count() as f64
        / observations.len() as f64;
    let goalie_coverage = observations
        .iter()
        .filter(|row| row.goalie_difference.is_some())
        .count() as f64
        / observations.len() as f64;
    let unique_teams = observations
        .iter()
        .flat_map(|row| [row.away_team.as_str(), row.home_team.as_str()])
        .collect::<BTreeSet<_>>();
    let maximum_team_share = unique_teams
        .iter()
        .map(|team| {
            observations
                .iter()
                .filter(|row| row.away_team == *team || row.home_team == *team)
                .count() as f64
                / observations.len() as f64
        })
        .fold(0.0, f64::max);
    let season_independent = holdouts.iter().all(|excluded| {
        let retained = predictions
            .iter()
            .filter(|row| row.observation.season != excluded.season);
        candidate_beats_elo(retained)
    });
    let team_independent = unique_teams.iter().all(|excluded| {
        let retained = predictions.iter().filter(|row| {
            row.observation.away_team != *excluded && row.observation.home_team != *excluded
        });
        candidate_beats_elo(retained)
    });
    let prospective_registration = registration.filter(|registration| {
        let holdout_rows = observations
            .iter()
            .filter(|row| row.season == registration.holdout_season)
            .collect::<Vec<_>>();
        registration.holdout_season == seasons.last().copied().unwrap_or_default()
            && !holdout_rows.is_empty()
            && holdout_rows
                .iter()
                .all(|row| registration.registered_at < row.forecast_at)
            && holdout_rows
                .iter()
                .all(|row| registration.outcome_not_before <= row.outcome_recorded_at)
    });
    let checks = vec![
        check(
            "minimum_holdout_seasons",
            holdouts.len() >= config.minimum_holdout_seasons,
            format!(
                "{} holdouts; require {}",
                holdouts.len(),
                config.minimum_holdout_seasons
            ),
        ),
        check(
            "pooled_brier_gain",
            elo_metrics.brier - candidate_metrics.brier >= config.minimum_brier_gain,
            format!(
                "gain {:.6}; require {:.6}",
                elo_metrics.brier - candidate_metrics.brier,
                config.minimum_brier_gain
            ),
        ),
        check(
            "pooled_log_loss_gain",
            elo_metrics.log_loss > candidate_metrics.log_loss,
            format!(
                "gain {:.6}",
                elo_metrics.log_loss - candidate_metrics.log_loss
            ),
        ),
        check(
            "improved_holdouts",
            improved_holdouts >= config.minimum_improved_holdouts,
            format!(
                "{} improved; require {}",
                improved_holdouts, config.minimum_improved_holdouts
            ),
        ),
        check(
            "calibration_not_worse",
            candidate_metrics.ece <= elo_metrics.ece,
            format!(
                "candidate {:.6}; Elo {:.6}",
                candidate_metrics.ece, elo_metrics.ece
            ),
        ),
        check(
            "roster_ablation",
            roster_ablation.included_feature_gain >= 0.0,
            format!("included gain {:.6}", roster_ablation.included_feature_gain),
        ),
        check(
            "roster_coverage",
            roster_coverage >= config.minimum_roster_coverage,
            format!(
                "coverage {:.3}; require {:.3}",
                roster_coverage, config.minimum_roster_coverage
            ),
        ),
        check(
            "goalie_ablation",
            goalie_ablation.included_feature_gain >= 0.0,
            format!("included gain {:.6}", goalie_ablation.included_feature_gain),
        ),
        check(
            "goalie_coverage",
            goalie_coverage >= config.minimum_goalie_coverage,
            format!(
                "coverage {:.3}; require {:.3}",
                goalie_coverage, config.minimum_goalie_coverage
            ),
        ),
        check(
            "candidate_feature_gain",
            candidate_features.is_empty()
                || candidate_feature_gain >= config.minimum_candidate_feature_gain,
            if candidate_features.is_empty() {
                "registered default has no experimental feature".to_owned()
            } else {
                format!(
                    "{} weakest included gain {:.6}; require {:.6}",
                    candidate_features.join(", "),
                    candidate_feature_gain,
                    config.minimum_candidate_feature_gain
                )
            },
        ),
        check(
            "season_stability",
            season_independent,
            "candidate must still beat Elo after excluding any one holdout season".to_owned(),
        ),
        check(
            "team_coverage",
            unique_teams.len() >= config.minimum_team_coverage
                && maximum_team_share <= config.maximum_team_game_share,
            format!(
                "{} teams; largest game share {:.3}; require at least {} teams and at most {:.3}",
                unique_teams.len(),
                maximum_team_share,
                config.minimum_team_coverage,
                config.maximum_team_game_share
            ),
        ),
        check(
            "team_stability",
            team_independent,
            "candidate must still beat Elo after excluding every game involving any one team"
                .to_owned(),
        ),
        check(
            "prospective_holdout_authority",
            prospective_registration.is_some(),
            prospective_registration.map_or_else(
                || {
                    "production requires a sealed registration created before every forecast in the final holdout season"
                        .to_owned()
                },
                |registration| {
                    format!(
                        "season {} preregistered at {} with outcomes not before {}",
                        registration.holdout_season,
                        registration.registered_at,
                        registration.outcome_not_before
                    )
                },
            ),
        ),
    ];
    let promotion_passed = checks.iter().all(|check| check.passed);
    let training = train_team_game_prediction_model(observations, config.clone())?;
    let mut final_model = training.model;
    (
        final_model.calibration_intercept,
        final_model.calibration_slope,
    ) = select_chronological_calibration(&calibration_history);
    let prospective_registration_fingerprint =
        prospective_registration.map(|registration| registration.fingerprint.clone());
    let input_fingerprint = fingerprint(&(
        observations,
        &config,
        prospective_registration_fingerprint.as_deref(),
    ))?;
    if promotion_passed {
        final_model.authority = TeamGamePredictionModelAuthority::Production;
        final_model.trained_through_season = seasons.last().copied();
        final_model.training_fingerprint = Some(input_fingerprint.clone());
    }
    let mut view = TeamGamePredictionValidationView {
        schema: TEAM_GAME_PREDICTION_VALIDATION_SCHEMA.to_owned(),
        vintage: config.vintage,
        holdouts,
        games: predictions.len(),
        candidate_brier: candidate_metrics.brier,
        elo_brier: elo_metrics.brier,
        brier_gain: elo_metrics.brier - candidate_metrics.brier,
        candidate_log_loss: candidate_metrics.log_loss,
        elo_log_loss: elo_metrics.log_loss,
        log_loss_gain: elo_metrics.log_loss - candidate_metrics.log_loss,
        candidate_ece: candidate_metrics.ece,
        elo_ece: elo_metrics.ece,
        improved_holdouts,
        ablations,
        checks,
        promotion_passed,
        final_model,
        prospective_registration_fingerprint,
        input_fingerprint,
        fingerprint: String::new(),
    };
    view.fingerprint = fingerprint(&view)?;
    Ok(view)
}

impl TeamGamePredictionTrainingView {
    pub fn validate(&self) -> Result<(), TeamGamePredictionTrainingError> {
        if self.schema != TEAM_GAME_PREDICTION_TRAINING_SCHEMA
            || self.observations == 0
            || self.seasons.is_empty()
            || !valid_sha256(&self.input_fingerprint)
            || self.fingerprint != fingerprint(self)?
        {
            return Err(TeamGamePredictionTrainingError::FingerprintMismatch);
        }
        self.model
            .validate()
            .map_err(|error| TeamGamePredictionTrainingError::InvalidConfig(error.to_string()))
    }
}

impl TeamGamePredictionValidationView {
    pub fn validate(&self) -> Result<(), TeamGamePredictionTrainingError> {
        if self.schema != TEAM_GAME_PREDICTION_VALIDATION_SCHEMA
            || self.games == 0
            || self.holdouts.is_empty()
            || self.promotion_passed != self.checks.iter().all(|check| check.passed)
            || !valid_sha256(&self.input_fingerprint)
            || self
                .prospective_registration_fingerprint
                .as_ref()
                .is_some_and(|fingerprint| !valid_sha256(fingerprint))
            || self.fingerprint != fingerprint(self)?
        {
            return Err(TeamGamePredictionTrainingError::FingerprintMismatch);
        }
        self.final_model
            .validate()
            .map_err(|error| TeamGamePredictionTrainingError::InvalidConfig(error.to_string()))
    }
}

#[derive(Debug, Clone)]
struct ScoredPrediction {
    observation: TeamGamePredictionTrainingObservation,
    model: TeamGamePredictionModel,
    candidate: f64,
    elo: f64,
}

#[derive(Debug, Clone, Copy)]
struct Metrics {
    brier: f64,
    log_loss: f64,
    ece: f64,
}

fn validate_config(
    config: &TeamGamePredictionTrainingConfig,
) -> Result<(), TeamGamePredictionTrainingError> {
    if config.model_id.trim().is_empty()
        || ![
            TEAM_GAME_PREDICTION_FEATURE_SET_V1,
            TEAM_GAME_PREDICTION_FEATURE_SET_V2,
            TEAM_GAME_PREDICTION_FEATURE_SET_V3,
            TEAM_GAME_PREDICTION_FEATURE_SET_V4,
            TEAM_GAME_PREDICTION_FEATURE_SET_V5,
        ]
        .contains(&config.feature_set.as_str())
        || config.minimum_training_seasons == 0
        || config.minimum_holdout_seasons == 0
        || config.iterations == 0
        || !config.minimum_brier_gain.is_finite()
        || config.minimum_brier_gain < 0.0
        || !config.l2_penalty.is_finite()
        || config.l2_penalty < 0.0
        || !config.learning_rate.is_finite()
        || config.learning_rate <= 0.0
        || !config.coefficient_prune_threshold.is_finite()
        || config.coefficient_prune_threshold < 0.0
        || config.minimum_team_coverage < 2
        || !config.maximum_team_game_share.is_finite()
        || !(0.0..=1.0).contains(&config.maximum_team_game_share)
        || !config.minimum_roster_coverage.is_finite()
        || !(0.0..=1.0).contains(&config.minimum_roster_coverage)
        || !config.minimum_goalie_coverage.is_finite()
        || !(0.0..=1.0).contains(&config.minimum_goalie_coverage)
        || !config.minimum_candidate_feature_gain.is_finite()
        || config.minimum_candidate_feature_gain < 0.0
        || config.elo_weight_grid.is_empty()
        || config
            .elo_weight_grid
            .iter()
            .any(|weight| !weight.is_finite() || !(0.0..=1.0).contains(weight))
    {
        return Err(TeamGamePredictionTrainingError::InvalidConfig(
            "training thresholds, optimizer, and Elo grid are invalid".to_owned(),
        ));
    }
    Ok(())
}

fn validate_observations(
    observations: &[TeamGamePredictionTrainingObservation],
    vintage: TeamGameForecastVintage,
) -> Result<(), TeamGamePredictionTrainingError> {
    let mut ids = BTreeSet::new();
    for row in observations {
        let invalid = |message: &str| TeamGamePredictionTrainingError::InvalidObservation {
            game_id: row.game_id,
            message: message.to_owned(),
        };
        if row.vintage != vintage {
            return Err(invalid(
                "observation vintage does not match training config",
            ));
        }
        if row.away_team.trim().is_empty()
            || row.home_team.trim().is_empty()
            || row.away_team == row.home_team
        {
            return Err(invalid("away and home team identities are invalid"));
        }
        if !ids.insert((row.season, row.game_id, row.vintage)) {
            return Err(invalid("duplicate season/game/vintage observation"));
        }
        if row.outcome_recorded_at <= row.forecast_at {
            return Err(invalid(
                "outcome must be recorded after the frozen forecast",
            ));
        }
        for (label, value) in [
            ("baseline", row.baseline_home_probability),
            ("Elo", row.elo_home_probability),
        ] {
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(invalid(&format!("{label} probability is invalid")));
            }
        }
        if feature_values(row)
            .into_iter()
            .flatten()
            .any(|value| !value.is_finite() || !(-5.0..=5.0).contains(&value))
        {
            return Err(invalid(
                "feature differences must be finite and between -5 and 5",
            ));
        }
        if row.source_fingerprints.is_empty()
            || row
                .source_fingerprints
                .iter()
                .any(|fingerprint| !valid_sha256(fingerprint))
        {
            return Err(invalid("source fingerprints are missing or invalid"));
        }
    }
    Ok(())
}

fn fit_best_model(
    observations: &[TeamGamePredictionTrainingObservation],
    config: &TeamGamePredictionTrainingConfig,
) -> TeamGamePredictionModel {
    config
        .elo_weight_grid
        .iter()
        .copied()
        .map(|elo_weight| {
            let model = fit_model(observations, config, elo_weight);
            let score = metrics(
                observations
                    .iter()
                    .map(|row| (predict(&model, row, None), row.home_won)),
            )
            .brier;
            (score, elo_weight, model)
        })
        .min_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then_with(|| left.1.total_cmp(&right.1))
        })
        .expect("validated non-empty Elo grid")
        .2
}

fn fit_model(
    observations: &[TeamGamePredictionTrainingObservation],
    config: &TeamGamePredictionTrainingConfig,
    elo_weight: f64,
) -> TeamGamePredictionModel {
    let mut weights = [
        0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ];
    let samples = observations
        .iter()
        .map(|row| {
            (
                training_features(
                    row,
                    elo_weight,
                    config.feature_set == TEAM_GAME_PREDICTION_FEATURE_SET_V2,
                    config.feature_set == TEAM_GAME_PREDICTION_FEATURE_SET_V3,
                    config.feature_set == TEAM_GAME_PREDICTION_FEATURE_SET_V4,
                    config.feature_set == TEAM_GAME_PREDICTION_FEATURE_SET_V5,
                ),
                if row.home_won { 1.0 } else { 0.0 },
            )
        })
        .collect::<Vec<_>>();
    for _ in 0..config.iterations {
        let mut gradient = [0.0; 13];
        for (features, outcome) in &samples {
            let prediction = logistic(dot(&weights, features));
            let error = prediction - *outcome;
            for index in 0..gradient.len() {
                gradient[index] += error * features[index];
            }
        }
        let count = samples.len() as f64;
        for index in 0..weights.len() {
            gradient[index] /= count;
            if index > 0 {
                gradient[index] += config.l2_penalty * weights[index];
            }
            weights[index] =
                (weights[index] - config.learning_rate * gradient[index]).clamp(-5.0, 5.0);
        }
    }
    TeamGamePredictionModel {
        model_id: config.model_id.clone(),
        method: TEAM_GAME_PREDICTION_EDGE_METHOD.to_owned(),
        authority: TeamGamePredictionModelAuthority::Evaluation,
        elo_weight,
        intercept: weights[0],
        baseline_logit_weight: weights[1].max(0.0),
        roster_weight: weights[2],
        availability_weight: weights[3],
        lineup_impact_weight: weights[4],
        goalie_weight: weights[5],
        goalie_schedule_weight: weights[6],
        goalie_form_weight: weights[7],
        goalie_workload_weight: weights[8],
        xg_weight: weights[9],
        opponent_adjusted_xg_weight: weights[10],
        special_teams_weight: weights[11],
        matchup_weight: weights[12],
        xg_prior_games: 10.0,
        special_teams_prior_games: 20.0,
        goalie_form_prior_appearances: 5.0,
        calibration_intercept: 0.0,
        calibration_slope: 1.0,
        trained_through_season: None,
        training_fingerprint: None,
    }
}

fn prune_model(model: &mut TeamGamePredictionModel, threshold: f64) -> Vec<String> {
    let mut pruned = Vec::new();
    for (key, weight) in [
        ("roster", &mut model.roster_weight),
        ("availability", &mut model.availability_weight),
        ("lineup_impact", &mut model.lineup_impact_weight),
        ("goalie", &mut model.goalie_weight),
        ("goalie_schedule", &mut model.goalie_schedule_weight),
        ("goalie_form", &mut model.goalie_form_weight),
        ("goalie_workload", &mut model.goalie_workload_weight),
        ("xg_form", &mut model.xg_weight),
        (
            "opponent_adjusted_xg",
            &mut model.opponent_adjusted_xg_weight,
        ),
        ("special_teams", &mut model.special_teams_weight),
        ("matchup", &mut model.matchup_weight),
    ] {
        if weight.abs() < threshold {
            *weight = 0.0;
            pruned.push(key.to_owned());
        }
    }
    pruned
}

fn predict(
    model: &TeamGamePredictionModel,
    observation: &TeamGamePredictionTrainingObservation,
    ablate: Option<&str>,
) -> f64 {
    let raw = predict_uncalibrated(model, observation, ablate);
    logistic(model.calibration_intercept + model.calibration_slope * logit(raw))
        .clamp(1e-6, 1.0 - 1e-6)
}

fn predict_uncalibrated(
    model: &TeamGamePredictionModel,
    observation: &TeamGamePredictionTrainingObservation,
    ablate: Option<&str>,
) -> f64 {
    let blended = observation.baseline_home_probability * (1.0 - model.elo_weight)
        + observation.elo_home_probability * model.elo_weight;
    let values = feature_values(observation).map(|value| value.unwrap_or(0.0));
    let weights = [
        model.roster_weight,
        model.availability_weight,
        model.lineup_impact_weight,
        model.goalie_weight,
        model.goalie_schedule_weight,
        model.goalie_form_weight,
        model.goalie_workload_weight,
        model.xg_weight,
        model.opponent_adjusted_xg_weight,
        model.special_teams_weight,
        model.matchup_weight,
    ];
    let mut value = model.intercept + model.baseline_logit_weight * logit(blended);
    for (index, key) in FEATURE_KEYS.iter().enumerate() {
        if ablate != Some(*key) {
            value += values[index] * weights[index];
        }
    }
    logistic(value).clamp(1e-6, 1.0 - 1e-6)
}

fn select_chronological_calibration(rows: &[(f64, bool)]) -> (f64, f64) {
    if rows.len() < 500 {
        return (0.0, 1.0);
    }
    // Preserve the fitted model's base rate. A second intercept confounds
    // calibration with season-varying home win rate; temperature scaling
    // adjusts confidence without moving that base.
    const INTERCEPTS: [f64; 1] = [0.0];
    // Calibration is deliberately bounded to a five-percent logit rescale.
    // Wider grids improved prior pooled fit but proved too unstable across the
    // next season.
    const SLOPES: [f64; 5] = [0.95, 0.975, 1.0, 1.025, 1.05];
    INTERCEPTS
        .into_iter()
        .flat_map(|intercept| {
            SLOPES.into_iter().map(move |slope| {
                let score = metrics(rows.iter().map(|(probability, outcome)| {
                    (logistic(intercept + slope * logit(*probability)), *outcome)
                }))
                .brier;
                let distance = intercept.abs() + (slope - 1.0).abs();
                (score, distance, intercept, slope)
            })
        })
        .min_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then_with(|| left.1.total_cmp(&right.1))
                .then_with(|| left.2.total_cmp(&right.2))
                .then_with(|| left.3.total_cmp(&right.3))
        })
        .map(|(_, _, intercept, slope)| (intercept, slope))
        .expect("calibration grid is non-empty")
}

fn training_features(
    row: &TeamGamePredictionTrainingObservation,
    elo_weight: f64,
    include_goalie_form: bool,
    include_lineup_impact: bool,
    include_goalie_schedule: bool,
    include_opponent_adjusted_xg: bool,
) -> [f64; 13] {
    let blended =
        row.baseline_home_probability * (1.0 - elo_weight) + row.elo_home_probability * elo_weight;
    let mut values = feature_values(row).map(|value| value.unwrap_or(0.0));
    if !include_goalie_form {
        values[5] = 0.0;
        values[6] = 0.0;
    }
    if !include_lineup_impact {
        values[2] = 0.0;
    }
    if !include_goalie_schedule {
        values[4] = 0.0;
    }
    if !include_opponent_adjusted_xg {
        values[8] = 0.0;
    }
    [
        1.0,
        logit(blended),
        values[0],
        values[1],
        values[2],
        values[3],
        values[4],
        values[5],
        values[6],
        values[7],
        values[8],
        values[9],
        values[10],
    ]
}

fn feature_values(row: &TeamGamePredictionTrainingObservation) -> [Option<f64>; 11] {
    [
        row.roster_difference,
        row.availability_difference,
        row.lineup_impact_difference,
        row.goalie_difference,
        row.goalie_schedule_difference,
        row.goalie_form_difference,
        row.goalie_workload_difference,
        row.xg_difference,
        row.opponent_adjusted_xg_difference,
        row.special_teams_difference,
        row.matchup_difference,
    ]
}

fn dot(left: &[f64; 13], right: &[f64; 13]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

fn candidate_beats_elo<'a>(rows: impl IntoIterator<Item = &'a ScoredPrediction>) -> bool {
    let rows = rows.into_iter().collect::<Vec<_>>();
    if rows.is_empty() {
        return false;
    }
    let candidate = metrics(
        rows.iter()
            .map(|row| (row.candidate, row.observation.home_won)),
    );
    let elo = metrics(rows.iter().map(|row| (row.elo, row.observation.home_won)));
    candidate.brier <= elo.brier && candidate.log_loss <= elo.log_loss
}

fn metrics<I>(rows: I) -> Metrics
where
    I: IntoIterator<Item = (f64, bool)>,
{
    let rows = rows.into_iter().collect::<Vec<_>>();
    let count = rows.len() as f64;
    let brier = rows
        .iter()
        .map(|(probability, outcome)| (probability - if *outcome { 1.0 } else { 0.0 }).powi(2))
        .sum::<f64>()
        / count;
    let log_loss = rows
        .iter()
        .map(|(probability, outcome)| {
            let observed = if *outcome {
                *probability
            } else {
                1.0 - probability
            };
            -observed.clamp(1e-15, 1.0).ln()
        })
        .sum::<f64>()
        / count;
    let ece = (0..10)
        .filter_map(|bin| {
            let lower = bin as f64 / 10.0;
            let upper = (bin + 1) as f64 / 10.0;
            let group = rows
                .iter()
                .filter(|(probability, _)| {
                    *probability >= lower
                        && (*probability < upper || (bin == 9 && *probability <= upper))
                })
                .collect::<Vec<_>>();
            if group.is_empty() {
                return None;
            }
            let mean_probability = group
                .iter()
                .map(|(probability, _)| *probability)
                .sum::<f64>()
                / group.len() as f64;
            let observed =
                group.iter().filter(|(_, outcome)| *outcome).count() as f64 / group.len() as f64;
            Some((mean_probability - observed).abs() * group.len() as f64 / count)
        })
        .sum();
    Metrics {
        brier,
        log_loss,
        ece,
    }
}

fn unique_seasons(observations: &[TeamGamePredictionTrainingObservation]) -> Vec<u32> {
    observations
        .iter()
        .map(|row| row.season)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn check(key: &str, passed: bool, detail: String) -> TeamGamePredictionPromotionCheck {
    TeamGamePredictionPromotionCheck {
        key: key.to_owned(),
        passed,
        detail,
    }
}

fn logit(value: f64) -> f64 {
    let value = value.clamp(1e-9, 1.0 - 1e-9);
    (value / (1.0 - value)).ln()
}

fn logistic(value: f64) -> f64 {
    if value >= 0.0 {
        1.0 / (1.0 + (-value).exp())
    } else {
        let exp = value.exp();
        exp / (1.0 + exp)
    }
}

fn fingerprint<T: Serialize>(value: &T) -> Result<String, TeamGamePredictionTrainingError> {
    let mut material = serde_json::to_value(value)
        .map_err(|error| TeamGamePredictionTrainingError::Serialization(error.to_string()))?;
    if let Some(object) = material.as_object_mut() {
        object.insert(
            "fingerprint".to_owned(),
            serde_json::Value::String(String::new()),
        );
    }
    let bytes = serde_json::to_vec(&material)
        .map_err(|error| TeamGamePredictionTrainingError::Serialization(error.to_string()))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn valid_sha256(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hash| hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone};

    use super::*;

    fn observations(
        seasons: usize,
        games_per_season: usize,
    ) -> Vec<TeamGamePredictionTrainingObservation> {
        let start = Utc.with_ymd_and_hms(2020, 1, 1, 12, 0, 0).unwrap();
        let mut rows = Vec::new();
        for season_index in 0..seasons {
            let season = 20_202_021 + season_index as u32 * 10_001;
            for game in 0..games_per_season {
                let signal = (game as i32 % 9 - 4) as f64 / 2.0;
                let home_won = signal + ((season_index + game) % 3) as f64 * 0.15 > 0.0;
                let forecast_at =
                    start + Duration::days((season_index * games_per_season + game) as i64);
                rows.push(TeamGamePredictionTrainingObservation {
                    season,
                    game_id: season as u64 * 10_000 + game as u64,
                    away_team: format!("T{:02}", (game + 1) % 16),
                    home_team: format!("T{:02}", game % 16),
                    vintage: TeamGameForecastVintage::PregameConfirmed,
                    forecast_at,
                    outcome_recorded_at: forecast_at + Duration::hours(4),
                    home_won,
                    baseline_home_probability: 0.5,
                    elo_home_probability: 0.5,
                    roster_difference: Some(signal),
                    availability_difference: Some(signal * 0.2),
                    lineup_impact_difference: Some(signal * 0.3),
                    goalie_difference: Some(signal * 0.8),
                    goalie_schedule_difference: Some(signal * 0.25),
                    goalie_form_difference: Some(signal * 0.4),
                    goalie_workload_difference: Some(signal * 0.2),
                    xg_difference: Some(signal * 0.5),
                    opponent_adjusted_xg_difference: Some(signal * 0.45),
                    special_teams_difference: Some(signal * 0.1),
                    matchup_difference: Some(signal * 0.2),
                    source_fingerprints: vec![format!("sha256:{}", "a".repeat(64))],
                });
            }
        }
        rows
    }

    #[test]
    fn regularized_training_is_deterministic_and_sealed() {
        let rows = observations(4, 30);
        let first =
            train_team_game_prediction_model(&rows, TeamGamePredictionTrainingConfig::default())
                .unwrap();
        let second =
            train_team_game_prediction_model(&rows, TeamGamePredictionTrainingConfig::default())
                .unwrap();
        assert_eq!(first, second);
        assert!(first.model.roster_weight > 0.0);
        assert!(first.fingerprint.starts_with("sha256:"));
        first.validate().unwrap();
    }

    #[test]
    fn rolling_origins_never_train_on_the_holdout_or_future() {
        let rows = observations(7, 30);
        let view =
            validate_team_game_prediction_model(&rows, TeamGamePredictionTrainingConfig::default())
                .unwrap();
        assert_eq!(view.holdouts.len(), 5);
        for holdout in &view.holdouts {
            assert!(holdout
                .training_seasons
                .iter()
                .all(|season| *season < holdout.season));
        }
        assert_eq!(view.holdouts[0].calibration_games, 0);
        assert_eq!(view.holdouts[1].calibration_games, 30);
        assert!(view.brier_gain > 0.0);
        assert!(view.ablations.iter().any(|row| row.feature == "goalie"));
        assert!(!view.promotion_passed);
        assert!(view
            .checks
            .iter()
            .any(|check| check.key == "prospective_holdout_authority" && !check.passed));
        view.validate().unwrap();

        let wire = serde_json::to_value(&view).unwrap();
        assert!(wire.get("prospective_registration_fingerprint").is_none());
        for holdout in wire["holdouts"].as_array().unwrap() {
            if holdout["calibration_games"] == 0 {
                assert!(holdout.get("calibration_intercept").is_none());
                assert!(holdout.get("calibration_slope").is_none());
                assert!(holdout.get("calibration_games").is_none());
            }
        }
        let decoded: TeamGamePredictionValidationView = serde_json::from_value(wire).unwrap();
        decoded.validate().unwrap();
    }

    #[test]
    fn candidate_feature_must_clear_its_own_incremental_gain_gate() {
        let rows = observations(7, 30);
        let mut config = TeamGamePredictionTrainingConfig::default();
        config.feature_set = TEAM_GAME_PREDICTION_FEATURE_SET_V4.to_owned();
        config.minimum_candidate_feature_gain = 1.0;
        let view = validate_team_game_prediction_model(&rows, config).unwrap();
        let check = view
            .checks
            .iter()
            .find(|check| check.key == "candidate_feature_gain")
            .unwrap();
        assert!(!check.passed);
        assert!(check.detail.contains("goalie_schedule"));
    }

    #[test]
    fn sealed_registration_is_required_and_must_predate_final_holdout() {
        let rows = observations(7, 30);
        let config = TeamGamePredictionTrainingConfig::default();
        let holdout_season = rows.last().unwrap().season;
        let first_holdout = rows
            .iter()
            .find(|row| row.season == holdout_season)
            .unwrap();
        let registration = register_team_game_prediction_holdout(
            &config,
            holdout_season,
            first_holdout.forecast_at - Duration::hours(1),
            first_holdout.forecast_at + Duration::hours(1),
        )
        .unwrap();
        registration.validate(&config).unwrap();
        let view = validate_team_game_prediction_model_with_registration(
            &rows,
            config,
            Some(&registration),
        )
        .unwrap();
        assert!(view
            .checks
            .iter()
            .any(|check| check.key == "prospective_holdout_authority" && check.passed));
        assert_eq!(
            view.prospective_registration_fingerprint.as_deref(),
            Some(registration.fingerprint.as_str())
        );
    }

    #[test]
    fn prospective_registration_binds_the_feature_vocabulary() {
        let legacy = TeamGamePredictionTrainingConfig::default();
        let mut candidate = legacy.clone();
        candidate.model_id = "icecast-edge-trained-v2".to_owned();
        candidate.feature_set = TEAM_GAME_PREDICTION_FEATURE_SET_V2.to_owned();
        let registered_at = Utc.with_ymd_and_hms(2026, 7, 30, 12, 0, 0).unwrap();
        let outcome_not_before = Utc.with_ymd_and_hms(2027, 4, 11, 12, 0, 0).unwrap();
        let registration = register_team_game_prediction_holdout(
            &legacy,
            20_262_027,
            registered_at,
            outcome_not_before,
        )
        .unwrap();
        assert!(registration.validate(&candidate).is_err());
    }

    #[test]
    fn chronological_calibration_is_conservative_and_brier_selected() {
        let rows = (0..600)
            .map(|index| (0.70, index % 20 < 11))
            .collect::<Vec<_>>();
        let before = metrics(rows.iter().copied()).brier;
        let (intercept, slope) = select_chronological_calibration(&rows);
        let after = metrics(rows.iter().map(|(probability, outcome)| {
            (logistic(intercept + slope * logit(*probability)), *outcome)
        }))
        .brier;
        assert!(after < before);
        assert!(intercept.abs() <= 0.10);
        assert!((0.95..=1.05).contains(&slope));
    }

    #[test]
    fn outcome_before_forecast_is_rejected_as_leakage() {
        let mut rows = observations(3, 10);
        rows[0].outcome_recorded_at = rows[0].forecast_at;
        let error =
            train_team_game_prediction_model(&rows, TeamGamePredictionTrainingConfig::default())
                .unwrap_err();
        assert!(error.to_string().contains("after the frozen forecast"));
    }

    #[test]
    fn sparse_coefficients_are_pruned_explicitly() {
        let rows = observations(4, 30);
        let config = TeamGamePredictionTrainingConfig {
            coefficient_prune_threshold: 0.5,
            ..TeamGamePredictionTrainingConfig::default()
        };
        let view = train_team_game_prediction_model(&rows, config).unwrap();
        assert!(!view.pruned_features.is_empty());
    }
}
