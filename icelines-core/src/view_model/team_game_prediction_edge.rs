use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::team_game_forecast::{
    TeamGameForecastFactorRow, TeamGameForecastSummaryRow, TeamGameForecastView,
    TEAM_GAME_FORECAST_SCHEMA,
};

pub const TEAM_GAME_PREDICTION_EDGE_SCHEMA: &str = "team_game_prediction_edge.v1";
pub const TEAM_GAME_PREDICTION_EDGE_JSON_SCHEMA: &str =
    include_str!("../../../design/schemas/team_game_prediction_edge.v1.schema.json");
pub const TEAM_GAME_PREDICTION_EDGE_METHOD: &str = "elo-evidence-logit.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamGameForecastVintage {
    Preseason,
    GameMorning,
    PregameConfirmed,
}

impl TeamGameForecastVintage {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Preseason => "preseason",
            Self::GameMorning => "game_morning",
            Self::PregameConfirmed => "pregame_confirmed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamGameEvidenceState {
    Confirmed,
    Reported,
    Modeled,
    Unavailable,
}

impl TeamGameEvidenceState {
    fn reliability(self) -> f64 {
        match self {
            Self::Confirmed => 1.0,
            Self::Reported => 0.70,
            Self::Modeled => 0.40,
            Self::Unavailable => 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamGamePredictionModelAuthority {
    Evaluation,
    Production,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGamePredictionModel {
    pub model_id: String,
    pub method: String,
    pub authority: TeamGamePredictionModelAuthority,
    pub elo_weight: f64,
    pub intercept: f64,
    pub baseline_logit_weight: f64,
    pub roster_weight: f64,
    pub availability_weight: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub lineup_impact_weight: f64,
    pub goalie_weight: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub goalie_schedule_weight: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub goalie_form_weight: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub goalie_workload_weight: f64,
    pub xg_weight: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub opponent_adjusted_xg_weight: f64,
    pub special_teams_weight: f64,
    pub matchup_weight: f64,
    pub xg_prior_games: f64,
    pub special_teams_prior_games: f64,
    #[serde(
        default = "default_goalie_form_prior_appearances",
        skip_serializing_if = "is_default_goalie_form_prior_appearances"
    )]
    pub goalie_form_prior_appearances: f64,
    /// Chronological post-model logit calibration. Defaults preserve legacy models.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub calibration_intercept: f64,
    #[serde(default = "default_calibration_slope", skip_serializing_if = "is_one")]
    pub calibration_slope: f64,
    pub trained_through_season: Option<u32>,
    pub training_fingerprint: Option<String>,
}

fn is_zero(value: &f64) -> bool {
    *value == 0.0
}

fn is_one(value: &f64) -> bool {
    *value == 1.0
}

fn is_zero_usize(value: &usize) -> bool {
    *value == 0
}

fn default_goalie_form_prior_appearances() -> f64 {
    5.0
}

fn is_default_goalie_form_prior_appearances(value: &f64) -> bool {
    *value == default_goalie_form_prior_appearances()
}

fn unavailable_evidence_state() -> TeamGameEvidenceState {
    TeamGameEvidenceState::Unavailable
}

fn is_unavailable_evidence_state(value: &TeamGameEvidenceState) -> bool {
    *value == TeamGameEvidenceState::Unavailable
}

impl TeamGamePredictionModel {
    pub fn evaluation_challenger() -> Self {
        Self {
            model_id: "icecast-edge-evaluation-v1".to_owned(),
            method: TEAM_GAME_PREDICTION_EDGE_METHOD.to_owned(),
            authority: TeamGamePredictionModelAuthority::Evaluation,
            elo_weight: 0.90,
            intercept: 0.0,
            baseline_logit_weight: 1.0,
            roster_weight: 0.08,
            availability_weight: 0.10,
            lineup_impact_weight: 0.0,
            goalie_weight: 0.12,
            goalie_schedule_weight: 0.0,
            goalie_form_weight: 0.0,
            goalie_workload_weight: 0.0,
            xg_weight: 0.15,
            opponent_adjusted_xg_weight: 0.0,
            special_teams_weight: 0.08,
            matchup_weight: 0.05,
            xg_prior_games: 10.0,
            special_teams_prior_games: 20.0,
            goalie_form_prior_appearances: 5.0,
            calibration_intercept: 0.0,
            calibration_slope: 1.0,
            trained_through_season: None,
            training_fingerprint: None,
        }
    }

    pub fn validate(&self) -> Result<(), TeamGamePredictionEdgeError> {
        if self.model_id.trim().is_empty() || self.method != TEAM_GAME_PREDICTION_EDGE_METHOD {
            return Err(TeamGamePredictionEdgeError::InvalidModel(
                "model identity or method is invalid".to_owned(),
            ));
        }
        let finite = [
            self.elo_weight,
            self.intercept,
            self.baseline_logit_weight,
            self.roster_weight,
            self.availability_weight,
            self.lineup_impact_weight,
            self.goalie_weight,
            self.goalie_schedule_weight,
            self.goalie_form_weight,
            self.goalie_workload_weight,
            self.xg_weight,
            self.opponent_adjusted_xg_weight,
            self.special_teams_weight,
            self.matchup_weight,
            self.xg_prior_games,
            self.special_teams_prior_games,
            self.goalie_form_prior_appearances,
            self.calibration_intercept,
            self.calibration_slope,
        ]
        .into_iter()
        .all(f64::is_finite);
        if !finite
            || !(0.0..=1.0).contains(&self.elo_weight)
            || self.baseline_logit_weight < 0.0
            || self.xg_prior_games < 0.0
            || self.special_teams_prior_games < 0.0
            || self.goalie_form_prior_appearances < 0.0
            || self.calibration_slope <= 0.0
            || self.calibration_slope > 2.0
            || self.calibration_intercept.abs() > 2.0
        {
            return Err(TeamGamePredictionEdgeError::InvalidModel(
                "model weights and priors must be finite and in range".to_owned(),
            ));
        }
        if self.authority == TeamGamePredictionModelAuthority::Production {
            let Some(season) = self.trained_through_season else {
                return Err(TeamGamePredictionEdgeError::InvalidModel(
                    "production model requires trained_through_season".to_owned(),
                ));
            };
            if season < 20_000_000 || !valid_sha256(self.training_fingerprint.as_deref()) {
                return Err(TeamGamePredictionEdgeError::InvalidModel(
                    "production model requires a valid season and training fingerprint".to_owned(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGamePredictionTeamEvidence {
    pub team: String,
    pub roster_strength: Option<f64>,
    pub roster_state: TeamGameEvidenceState,
    pub availability_strength: Option<f64>,
    pub availability_state: TeamGameEvidenceState,
    /// Dressed-skater value above/below the team's frozen expected 12F/6D,
    /// measured on the shared player-value scale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lineup_impact: Option<f64>,
    #[serde(
        default = "unavailable_evidence_state",
        skip_serializing_if = "is_unavailable_evidence_state"
    )]
    pub lineup_impact_state: TeamGameEvidenceState,
    pub goalie_quality: Option<f64>,
    pub goalie_state: TeamGameEvidenceState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goalie_player_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goalie_form_quality: Option<f64>,
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub goalie_form_appearances: usize,
    #[serde(
        default = "unavailable_evidence_state",
        skip_serializing_if = "is_unavailable_evidence_state"
    )]
    pub goalie_form_state: TeamGameEvidenceState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goalie_workload_readiness: Option<f64>,
    pub xg_share: Option<f64>,
    pub xg_games: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opponent_adjusted_xg_share: Option<f64>,
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub opponent_adjusted_xg_games: usize,
    pub special_teams_strength: Option<f64>,
    pub special_teams_games: usize,
    pub matchup_suitability: Option<f64>,
    pub matchup_state: TeamGameEvidenceState,
    pub source_fingerprints: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGamePredictionEvidenceInput {
    pub game_id: u64,
    pub forecast_at: DateTime<Utc>,
    pub captured_at: DateTime<Utc>,
    pub away: TeamGamePredictionTeamEvidence,
    pub home: TeamGamePredictionTeamEvidence,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGamePredictionFactorRow {
    pub key: String,
    pub available: bool,
    pub raw_home_minus_away: Option<f64>,
    pub effective_home_minus_away: Option<f64>,
    pub reliability: f64,
    pub log_odds_contribution: f64,
    pub home_win_probability_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGamePredictionEdgeGameRow {
    pub game_id: u64,
    pub date: NaiveDate,
    pub away_team: String,
    pub home_team: String,
    pub forecast_at: Option<DateTime<Utc>>,
    pub captured_at: Option<DateTime<Utc>>,
    pub base_home_win_probability: f64,
    pub elo_home_win_probability: f64,
    pub blended_home_win_probability: f64,
    pub enhanced_home_win_probability: f64,
    /// Weight-adjusted reliability across active model evidence features.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_confidence: Option<f64>,
    /// Sensitivity range from a fixed half-unit perturbation of uncertain
    /// active evidence. This is not a statistical confidence interval.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stability_low_home_win_probability: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stability_high_home_win_probability: Option<f64>,
    pub available_features: usize,
    pub expected_features: usize,
    pub factors: Vec<TeamGamePredictionFactorRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGamePredictionEdgeView {
    pub schema: String,
    pub season: u32,
    pub vintage: TeamGameForecastVintage,
    pub generated_at: DateTime<Utc>,
    pub model: TeamGamePredictionModel,
    pub source_forecast_fingerprint: String,
    pub games: Vec<TeamGamePredictionEdgeGameRow>,
    pub enhanced_forecast: TeamGameForecastView,
    pub warnings: Vec<String>,
    pub disclosures: Vec<String>,
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TeamGamePredictionEdgeError {
    #[error("invalid source forecast: {0}")]
    InvalidSource(String),
    #[error("invalid model: {0}")]
    InvalidModel(String),
    #[error("invalid evidence for game {game_id}: {message}")]
    InvalidEvidence { game_id: u64, message: String },
    #[error("duplicate evidence for game {0}")]
    DuplicateEvidence(u64),
    #[error("prediction edge fingerprint mismatch")]
    FingerprintMismatch,
    #[error("prediction edge serialization failed: {0}")]
    Serialization(String),
}

pub fn build_team_game_prediction_edge(
    source: &TeamGameForecastView,
    vintage: TeamGameForecastVintage,
    generated_at: DateTime<Utc>,
    model: TeamGamePredictionModel,
    evidence: Vec<TeamGamePredictionEvidenceInput>,
) -> Result<TeamGamePredictionEdgeView, TeamGamePredictionEdgeError> {
    model.validate()?;
    validate_source(source)?;
    let source_forecast_fingerprint = fingerprint(source)?;
    let source_by_id = source
        .games
        .iter()
        .map(|game| (game.game_id, game))
        .collect::<BTreeMap<_, _>>();
    let mut evidence_by_id = BTreeMap::new();
    for row in evidence {
        if evidence_by_id.contains_key(&row.game_id) {
            return Err(TeamGamePredictionEdgeError::DuplicateEvidence(row.game_id));
        }
        let Some(game) = source_by_id.get(&row.game_id) else {
            return Err(TeamGamePredictionEdgeError::InvalidEvidence {
                game_id: row.game_id,
                message: "game is absent from the source forecast".to_owned(),
            });
        };
        validate_evidence(game, vintage, generated_at, &row)?;
        evidence_by_id.insert(row.game_id, row);
    }

    let mut enhanced_forecast = source.clone();
    enhanced_forecast.forecast_mode = format!("prediction_edge::{}", vintage.as_str());
    enhanced_forecast.accuracy = None;
    let mut game_rows = Vec::with_capacity(enhanced_forecast.games.len());
    let mut warnings = Vec::new();
    for game in &mut enhanced_forecast.games {
        let evidence = evidence_by_id.get(&game.game_id);
        let base = probability(game.home_overall_win_probability, game.game_id, "base")?;
        let elo = if game.elo_home_win_probability > 0.0 {
            probability(game.elo_home_win_probability, game.game_id, "Elo")?
        } else {
            warnings.push(format!(
                "game {} has no Elo probability; the IceLines baseline was reused",
                game.game_id
            ));
            base
        };
        let blended = base * (1.0 - model.elo_weight) + elo * model.elo_weight;
        let mut current = logistic(model.intercept + model.baseline_logit_weight * logit(blended));
        let mut factors = Vec::with_capacity(12);
        let mut active_weight = 0.0;
        let mut reliability_weight = 0.0;
        let mut uncertain_log_odds = 0.0;
        let mut available_features = 0;
        let mut expected_features = 0;

        let feature_specs = build_features(game, evidence, &model);
        for (key, raw, effective, reliability, weight) in feature_specs {
            let absolute_weight = weight.abs();
            if absolute_weight > 0.0 {
                expected_features += 1;
                available_features += usize::from(effective.is_some());
                active_weight += absolute_weight;
                reliability_weight += absolute_weight * reliability;
                uncertain_log_odds += absolute_weight * (1.0 - reliability) * 0.5;
            }
            let before = current;
            let contribution = effective.unwrap_or(0.0) * weight;
            current = logistic(logit(current) + contribution);
            factors.push(TeamGamePredictionFactorRow {
                key: key.to_owned(),
                available: effective.is_some(),
                raw_home_minus_away: raw,
                effective_home_minus_away: effective,
                reliability,
                log_odds_contribution: contribution,
                home_win_probability_delta: current - before,
            });
        }
        let before_calibration = current;
        let calibration_log_odds = model.calibration_intercept
            + (model.calibration_slope - 1.0) * logit(before_calibration);
        current = logistic(logit(before_calibration) + calibration_log_odds);
        let calibrated = model.calibration_intercept != 0.0 || model.calibration_slope != 1.0;
        factors.push(TeamGamePredictionFactorRow {
            key: "calibration".to_owned(),
            available: calibrated,
            raw_home_minus_away: calibrated.then_some(logit(before_calibration)),
            effective_home_minus_away: calibrated.then_some(calibration_log_odds),
            reliability: if calibrated { 1.0 } else { 0.0 },
            log_odds_contribution: calibration_log_odds,
            home_win_probability_delta: current - before_calibration,
        });
        let evidence_confidence =
            (active_weight > 0.0).then_some((reliability_weight / active_weight).clamp(0.0, 1.0));
        let calibrated_uncertainty = uncertain_log_odds * model.calibration_slope;
        let stability_low_home_win_probability = evidence_confidence
            .map(|_| logistic(logit(current) - calibrated_uncertainty).clamp(0.08, 0.92));
        let stability_high_home_win_probability = evidence_confidence
            .map(|_| logistic(logit(current) + calibrated_uncertainty).clamp(0.08, 0.92));
        current = current.clamp(0.08, 0.92);
        if evidence.is_none() {
            warnings.push(format!(
                "game {} {} at {} has no {:?} evidence package; only the frozen baseline/Elo blend was applied",
                game.away_team, game.home_team, game.date, vintage
            ));
        }
        current = update_forecast_game(game, current, base, blended);
        for factor in factors.iter().filter(|row| row.available) {
            game.factors.push(TeamGameForecastFactorRow {
                key: format!("edge.{}", factor.key),
                label: format!(
                    "prediction-edge {} contribution ({:+.3} log odds)",
                    factor.key, factor.log_odds_contribution
                ),
                home_win_probability_delta: factor.home_win_probability_delta,
            });
        }
        game_rows.push(TeamGamePredictionEdgeGameRow {
            game_id: game.game_id,
            date: game.date,
            away_team: game.away_team.clone(),
            home_team: game.home_team.clone(),
            forecast_at: evidence.map(|row| row.forecast_at),
            captured_at: evidence.map(|row| row.captured_at),
            base_home_win_probability: base,
            elo_home_win_probability: elo,
            blended_home_win_probability: blended,
            enhanced_home_win_probability: current,
            evidence_confidence,
            stability_low_home_win_probability,
            stability_high_home_win_probability,
            available_features,
            expected_features,
            factors,
        });
    }
    enhanced_forecast.teams = summarize_teams(&enhanced_forecast.games);
    enhanced_forecast.warnings.extend(warnings.iter().cloned());
    enhanced_forecast.disclosures.push(
        "Prediction-edge probabilities are applied in core from a frozen model and dated evidence; renderers and season simulation do not recompute them."
            .to_owned(),
    );

    let mut view = TeamGamePredictionEdgeView {
        schema: TEAM_GAME_PREDICTION_EDGE_SCHEMA.to_owned(),
        season: source.season,
        vintage,
        generated_at,
        model,
        source_forecast_fingerprint,
        games: game_rows,
        enhanced_forecast,
        warnings,
        disclosures: vec![
            "The checked default is an evaluation challenger, not a promoted production model.".to_owned(),
            "Unavailable roster, goalie identity/quality/form/workload, xG, special-teams, and matchup evidence remains missing rather than zero-filled.".to_owned(),
            "Final outcomes are labels only and cannot enter a forecast vintage before its probability is frozen.".to_owned(),
            "Evidence-stability ranges perturb uncertain active features by one-half normalized unit; they are sensitivity ranges, not statistical confidence intervals, and exclude model-selection and outcome uncertainty.".to_owned(),
        ],
        fingerprint: String::new(),
    };
    view.fingerprint = fingerprint(&view)?;
    view.validate()?;
    Ok(view)
}

impl TeamGamePredictionEdgeView {
    pub fn validate(&self) -> Result<(), TeamGamePredictionEdgeError> {
        if self.schema != TEAM_GAME_PREDICTION_EDGE_SCHEMA
            || self.enhanced_forecast.schema != TEAM_GAME_FORECAST_SCHEMA
            || self.season != self.enhanced_forecast.season
            || self.games.len() != self.enhanced_forecast.games.len()
            || !valid_sha256(Some(&self.source_forecast_fingerprint))
        {
            return Err(TeamGamePredictionEdgeError::InvalidSource(
                "edge/source identity or cohort mismatch".to_owned(),
            ));
        }
        self.model.validate()?;
        let mut ids = BTreeSet::new();
        for (edge, game) in self.games.iter().zip(&self.enhanced_forecast.games) {
            if !ids.insert(edge.game_id)
                || edge.game_id != game.game_id
                || edge.home_team != game.home_team
                || edge.away_team != game.away_team
                || (edge.enhanced_home_win_probability - game.home_overall_win_probability).abs()
                    > 1e-12
                || edge.available_features > edge.expected_features
                || !valid_stability(edge)
            {
                return Err(TeamGamePredictionEdgeError::InvalidSource(
                    "edge game rows do not reconcile to the enhanced forecast".to_owned(),
                ));
            }
        }
        if self.fingerprint != fingerprint(self)? {
            return Err(TeamGamePredictionEdgeError::FingerprintMismatch);
        }
        Ok(())
    }
}

fn valid_stability(edge: &TeamGamePredictionEdgeGameRow) -> bool {
    match (
        edge.evidence_confidence,
        edge.stability_low_home_win_probability,
        edge.stability_high_home_win_probability,
    ) {
        (None, None, None) => true,
        (Some(confidence), Some(low), Some(high)) => {
            confidence.is_finite()
                && (0.0..=1.0).contains(&confidence)
                && low.is_finite()
                && high.is_finite()
                && (0.0..=1.0).contains(&low)
                && (0.0..=1.0).contains(&high)
                && low <= edge.enhanced_home_win_probability
                && edge.enhanced_home_win_probability <= high
        }
        _ => false,
    }
}

fn validate_source(source: &TeamGameForecastView) -> Result<(), TeamGamePredictionEdgeError> {
    if source.schema != TEAM_GAME_FORECAST_SCHEMA
        || source.games.is_empty()
        || source.schedule_games != source.games.len()
    {
        return Err(TeamGamePredictionEdgeError::InvalidSource(
            "source must be a non-empty complete team_game_forecast.v1".to_owned(),
        ));
    }
    let mut ids = BTreeSet::new();
    if source.games.iter().any(|game| {
        !ids.insert(game.game_id)
            || game.home_team == game.away_team
            || game.home_team.trim().is_empty()
            || game.away_team.trim().is_empty()
    }) {
        return Err(TeamGamePredictionEdgeError::InvalidSource(
            "source contains duplicate IDs or invalid teams".to_owned(),
        ));
    }
    Ok(())
}

fn validate_evidence(
    game: &&super::team_game_forecast::TeamGameForecastRow,
    vintage: TeamGameForecastVintage,
    generated_at: DateTime<Utc>,
    evidence: &TeamGamePredictionEvidenceInput,
) -> Result<(), TeamGamePredictionEdgeError> {
    let invalid = |message: &str| TeamGamePredictionEdgeError::InvalidEvidence {
        game_id: evidence.game_id,
        message: message.to_owned(),
    };
    if evidence.captured_at > evidence.forecast_at || evidence.forecast_at > generated_at {
        return Err(invalid(
            "capture/forecast timestamps exceed their evidence boundary",
        ));
    }
    let forecast_date = evidence.forecast_at.date_naive();
    match vintage {
        TeamGameForecastVintage::Preseason if forecast_date >= game.date => {
            return Err(invalid(
                "preseason evidence must be frozen before the game date",
            ));
        }
        TeamGameForecastVintage::GameMorning if forecast_date != game.date => {
            return Err(invalid(
                "game-morning evidence must be frozen on the NHL game date",
            ));
        }
        TeamGameForecastVintage::PregameConfirmed
            if forecast_date != game.date && forecast_date != game.date.succ_opt().unwrap() =>
        {
            return Err(invalid(
                "pregame evidence must be frozen on the NHL game date or its following UTC date",
            ));
        }
        _ => {}
    }
    if evidence.home.team.trim().to_ascii_uppercase() != game.home_team
        || evidence.away.team.trim().to_ascii_uppercase() != game.away_team
    {
        return Err(invalid("evidence teams do not match the scheduled game"));
    }
    validate_team_evidence(&evidence.home).map_err(|message| invalid(&message))?;
    validate_team_evidence(&evidence.away).map_err(|message| invalid(&message))?;
    Ok(())
}

fn validate_team_evidence(row: &TeamGamePredictionTeamEvidence) -> Result<(), String> {
    if row.team.trim().is_empty() {
        return Err("team is empty".to_owned());
    }
    for (label, value) in [
        ("roster_strength", row.roster_strength),
        ("availability_strength", row.availability_strength),
        ("goalie_quality", row.goalie_quality),
        ("goalie_form_quality", row.goalie_form_quality),
        ("goalie_workload_readiness", row.goalie_workload_readiness),
        ("special_teams_strength", row.special_teams_strength),
    ] {
        if value.is_some_and(|value| !value.is_finite() || !(0.0..=100.0).contains(&value)) {
            return Err(format!("{label} must be finite and between 0 and 100"));
        }
    }
    if row
        .lineup_impact
        .is_some_and(|value| !value.is_finite() || !(-55.0..=55.0).contains(&value))
    {
        return Err("lineup_impact must be finite and between -55 and 55".to_owned());
    }
    if row
        .xg_share
        .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
    {
        return Err("xg_share must be finite and between 0 and 1".to_owned());
    }
    if row
        .opponent_adjusted_xg_share
        .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
    {
        return Err("opponent_adjusted_xg_share must be finite and between 0 and 1".to_owned());
    }
    if row
        .matchup_suitability
        .is_some_and(|value| !value.is_finite() || !(-1.0..=1.0).contains(&value))
    {
        return Err("matchup_suitability must be finite and between -1 and 1".to_owned());
    }
    if row.goalie_state == TeamGameEvidenceState::Unavailable && row.goalie_quality.is_some() {
        return Err("unavailable goalie evidence cannot carry quality".to_owned());
    }
    if row.goalie_state != TeamGameEvidenceState::Unavailable && row.goalie_quality.is_none() {
        return Err("available goalie evidence requires quality".to_owned());
    }
    if row.goalie_player_id == Some(0) {
        return Err("goalie player ID must be non-zero".to_owned());
    }
    let goalie_form_available = row.goalie_form_state != TeamGameEvidenceState::Unavailable;
    if goalie_form_available
        != (row.goalie_form_quality.is_some()
            && row.goalie_workload_readiness.is_some()
            && row.goalie_form_appearances > 0
            && row.goalie_player_id.is_some())
    {
        return Err(
            "goalie form state requires identity, form, workload, and appearances".to_owned(),
        );
    }
    for (label, state, value) in [
        ("roster", row.roster_state, row.roster_strength),
        (
            "availability",
            row.availability_state,
            row.availability_strength,
        ),
        ("lineup impact", row.lineup_impact_state, row.lineup_impact),
        ("matchup", row.matchup_state, row.matchup_suitability),
    ] {
        if (state == TeamGameEvidenceState::Unavailable) != value.is_none() {
            return Err(format!(
                "{label} evidence state and value availability are inconsistent"
            ));
        }
    }
    if row.xg_share.is_none() && row.xg_games != 0 {
        return Err("missing xG share must have zero games".to_owned());
    }
    if row.xg_share.is_some() && row.xg_games == 0 {
        return Err("available xG share requires at least one game".to_owned());
    }
    if row.opponent_adjusted_xg_share.is_none() && row.opponent_adjusted_xg_games != 0 {
        return Err("missing opponent-adjusted xG share must have zero games".to_owned());
    }
    if row.opponent_adjusted_xg_share.is_some() && row.opponent_adjusted_xg_games == 0 {
        return Err("available opponent-adjusted xG share requires at least one game".to_owned());
    }
    if row.special_teams_strength.is_none() && row.special_teams_games != 0 {
        return Err("missing special-teams strength must have zero games".to_owned());
    }
    if row.special_teams_strength.is_some() && row.special_teams_games == 0 {
        return Err("available special-teams strength requires at least one game".to_owned());
    }
    if row.source_fingerprints.is_empty() {
        return Err("evidence requires at least one source fingerprint".to_owned());
    }
    if row
        .source_fingerprints
        .iter()
        .any(|value| !valid_sha256(Some(value)))
    {
        return Err("source fingerprints must be sha256 values".to_owned());
    }
    Ok(())
}

type FeatureSpec = (&'static str, Option<f64>, Option<f64>, f64, f64);

fn build_features(
    game: &super::team_game_forecast::TeamGameForecastRow,
    evidence: Option<&TeamGamePredictionEvidenceInput>,
    model: &TeamGamePredictionModel,
) -> [FeatureSpec; 11] {
    let Some(evidence) = evidence else {
        return [
            ("roster", None, None, 0.0, model.roster_weight),
            ("availability", None, None, 0.0, model.availability_weight),
            ("lineup_impact", None, None, 0.0, model.lineup_impact_weight),
            ("goalie", None, None, 0.0, model.goalie_weight),
            (
                "goalie_schedule",
                None,
                None,
                0.0,
                model.goalie_schedule_weight,
            ),
            ("goalie_form", None, None, 0.0, model.goalie_form_weight),
            (
                "goalie_workload",
                None,
                None,
                0.0,
                model.goalie_workload_weight,
            ),
            ("xg_form", None, None, 0.0, model.xg_weight),
            (
                "opponent_adjusted_xg",
                None,
                None,
                0.0,
                model.opponent_adjusted_xg_weight,
            ),
            ("special_teams", None, None, 0.0, model.special_teams_weight),
            ("matchup", None, None, 0.0, model.matchup_weight),
        ];
    };
    let roster_raw = difference(
        evidence.home.roster_strength,
        evidence.away.roster_strength,
        10.0,
    );
    let roster_reliability =
        paired_state_reliability(evidence.home.roster_state, evidence.away.roster_state);
    let roster = roster_raw.map(|value| value * roster_reliability);
    let availability_raw = difference(
        evidence.home.availability_strength,
        evidence.away.availability_strength,
        10.0,
    );
    let availability_reliability = paired_state_reliability(
        evidence.home.availability_state,
        evidence.away.availability_state,
    );
    let availability = availability_raw.map(|value| value * availability_reliability);
    let lineup_impact_raw = difference(
        evidence.home.lineup_impact,
        evidence.away.lineup_impact,
        25.0,
    );
    let lineup_impact_reliability = paired_state_reliability(
        evidence.home.lineup_impact_state,
        evidence.away.lineup_impact_state,
    );
    let lineup_impact = lineup_impact_raw.map(|value| value * lineup_impact_reliability);
    let goalie_raw = difference(
        evidence.home.goalie_quality,
        evidence.away.goalie_quality,
        10.0,
    );
    let goalie_reliability = evidence
        .home
        .goalie_state
        .reliability()
        .min(evidence.away.goalie_state.reliability());
    let goalie = goalie_raw.map(|value| value * goalie_reliability);
    let goalie_schedule_raw = evidence
        .home
        .goalie_quality
        .zip(evidence.away.goalie_quality)
        .map(|(home, away)| {
            (home - 50.0) / 25.0 * schedule_load(&game.home_context)
                - (away - 50.0) / 25.0 * schedule_load(&game.away_context)
        });
    let goalie_schedule = goalie_schedule_raw.map(|value| value * goalie_reliability);
    let goalie_form_raw = difference(
        evidence.home.goalie_form_quality,
        evidence.away.goalie_form_quality,
        20.0,
    );
    let goalie_form_reliability = paired_state_reliability(
        evidence.home.goalie_form_state,
        evidence.away.goalie_form_state,
    ) * paired_sample_reliability(
        evidence.home.goalie_form_appearances,
        evidence.away.goalie_form_appearances,
        model.goalie_form_prior_appearances,
    );
    let goalie_form = goalie_form_raw.map(|value| value * goalie_form_reliability);
    let goalie_workload_raw = difference(
        evidence.home.goalie_workload_readiness,
        evidence.away.goalie_workload_readiness,
        20.0,
    );
    let goalie_workload_reliability = paired_state_reliability(
        evidence.home.goalie_form_state,
        evidence.away.goalie_form_state,
    );
    let goalie_workload = goalie_workload_raw.map(|value| value * goalie_workload_reliability);
    let xg_raw = difference(evidence.home.xg_share, evidence.away.xg_share, 0.10);
    let xg_reliability = paired_sample_reliability(
        evidence.home.xg_games,
        evidence.away.xg_games,
        model.xg_prior_games,
    );
    let xg = xg_raw.map(|value| value * xg_reliability);
    let opponent_adjusted_xg_raw = difference(
        evidence.home.opponent_adjusted_xg_share,
        evidence.away.opponent_adjusted_xg_share,
        0.10,
    );
    let opponent_adjusted_xg_reliability = paired_sample_reliability(
        evidence.home.opponent_adjusted_xg_games,
        evidence.away.opponent_adjusted_xg_games,
        model.xg_prior_games,
    );
    let opponent_adjusted_xg =
        opponent_adjusted_xg_raw.map(|value| value * opponent_adjusted_xg_reliability);
    let special_raw = difference(
        evidence.home.special_teams_strength,
        evidence.away.special_teams_strength,
        10.0,
    );
    let special_reliability = paired_sample_reliability(
        evidence.home.special_teams_games,
        evidence.away.special_teams_games,
        model.special_teams_prior_games,
    );
    let special = special_raw.map(|value| value * special_reliability);
    let matchup_raw = difference(
        evidence.home.matchup_suitability,
        evidence.away.matchup_suitability,
        1.0,
    );
    let matchup_reliability =
        paired_state_reliability(evidence.home.matchup_state, evidence.away.matchup_state);
    let matchup = matchup_raw.map(|value| value * matchup_reliability);
    [
        (
            "roster",
            roster_raw,
            roster,
            roster_reliability,
            model.roster_weight,
        ),
        (
            "availability",
            availability_raw,
            availability,
            availability_reliability,
            model.availability_weight,
        ),
        (
            "lineup_impact",
            lineup_impact_raw,
            lineup_impact,
            lineup_impact_reliability,
            model.lineup_impact_weight,
        ),
        (
            "goalie",
            goalie_raw,
            goalie,
            goalie_reliability,
            model.goalie_weight,
        ),
        (
            "goalie_schedule",
            goalie_schedule_raw,
            goalie_schedule,
            goalie_reliability,
            model.goalie_schedule_weight,
        ),
        (
            "goalie_form",
            goalie_form_raw,
            goalie_form,
            goalie_form_reliability,
            model.goalie_form_weight,
        ),
        (
            "goalie_workload",
            goalie_workload_raw,
            goalie_workload,
            goalie_workload_reliability,
            model.goalie_workload_weight,
        ),
        ("xg_form", xg_raw, xg, xg_reliability, model.xg_weight),
        (
            "opponent_adjusted_xg",
            opponent_adjusted_xg_raw,
            opponent_adjusted_xg,
            opponent_adjusted_xg_reliability,
            model.opponent_adjusted_xg_weight,
        ),
        (
            "special_teams",
            special_raw,
            special,
            special_reliability,
            model.special_teams_weight,
        ),
        (
            "matchup",
            matchup_raw,
            matchup,
            matchup_reliability,
            model.matchup_weight,
        ),
    ]
}

fn schedule_load(context: &super::team_game_forecast::TeamGameScheduleContext) -> f64 {
    let congestion = f64::from(u8::from(context.back_to_back))
        + 0.5 * f64::from(u8::from(context.three_in_four))
        + 0.5 * f64::from(u8::from(context.four_in_six));
    let travel = (context.travel_km / 4_000.0).clamp(0.0, 1.0);
    let timezone =
        (f64::from(context.timezone_displacement_hours.abs()) / 3.0).clamp(0.0, 1.0) * 0.5;
    ((congestion + travel + timezone) / 3.5).clamp(0.0, 1.0)
}

fn paired_state_reliability(home: TeamGameEvidenceState, away: TeamGameEvidenceState) -> f64 {
    home.reliability().min(away.reliability())
}

fn difference(home: Option<f64>, away: Option<f64>, scale: f64) -> Option<f64> {
    home.zip(away).map(|(home, away)| (home - away) / scale)
}

fn paired_sample_reliability(home: usize, away: usize, prior: f64) -> f64 {
    let sample = home.min(away) as f64;
    if sample == 0.0 {
        0.0
    } else {
        sample / (sample + prior)
    }
}

fn update_forecast_game(
    game: &mut super::team_game_forecast::TeamGameForecastRow,
    home_probability: f64,
    base_probability: f64,
    blended_probability: f64,
) -> f64 {
    let overtime = game.overtime_probability;
    let edge = ((home_probability - 0.5) / (1.0 - overtime * 0.5)).clamp(-0.49, 0.49);
    let home_reg = (1.0 - overtime) * (0.5 + edge);
    let away_reg = (1.0 - overtime) * (0.5 - edge);
    let home_ot = (0.5 + edge * 0.5).clamp(0.01, 0.99);
    let reconciled_home = home_reg + overtime * home_ot;
    game.home_regulation_win_probability = home_reg;
    game.away_regulation_win_probability = away_reg;
    game.home_overtime_win_probability = home_ot;
    game.home_overall_win_probability = reconciled_home;
    game.away_overall_win_probability = 1.0 - reconciled_home;
    game.home_expected_standings_points = home_reg * 2.0 + overtime * (1.0 + home_ot);
    game.away_expected_standings_points = away_reg * 2.0 + overtime * (2.0 - home_ot);
    game.favored_team = if reconciled_home >= 0.5 {
        game.home_team.clone()
    } else {
        game.away_team.clone()
    };
    let favorite = reconciled_home.max(1.0 - reconciled_home);
    game.confidence = if favorite >= 0.62 {
        "strong"
    } else if favorite >= 0.55 {
        "lean"
    } else {
        "toss_up"
    }
    .to_owned();
    if let Some(winner) = &game.actual_winner {
        let home_won = winner == &game.home_team;
        game.pick_correct = Some((reconciled_home >= 0.5) == home_won);
        game.brier_score = Some((reconciled_home - f64::from(home_won)).powi(2));
        let observed = if home_won {
            reconciled_home
        } else {
            1.0 - reconciled_home
        };
        game.binary_log_loss = Some(-observed.clamp(1e-15, 1.0).ln());
        game.multiclass_log_loss = match game.actual_ending.as_deref() {
            Some("OT" | "SO") => Some(-overtime.clamp(1e-15, 1.0).ln()),
            Some("REG") if home_won => Some(-home_reg.clamp(1e-15, 1.0).ln()),
            Some("REG") => Some(-away_reg.clamp(1e-15, 1.0).ln()),
            _ => None,
        };
    }
    game.factors.push(TeamGameForecastFactorRow {
        key: "edge.elo_blend".to_owned(),
        label: "frozen IceLines/Elo challenger blend".to_owned(),
        home_win_probability_delta: blended_probability - base_probability,
    });
    reconciled_home
}

fn summarize_teams(
    games: &[super::team_game_forecast::TeamGameForecastRow],
) -> Vec<TeamGameForecastSummaryRow> {
    let mut rows = BTreeMap::<String, TeamGameForecastSummaryRow>::new();
    for game in games {
        for (team, home, expected) in [
            (&game.away_team, false, game.away_expected_standings_points),
            (&game.home_team, true, game.home_expected_standings_points),
        ] {
            let row = rows
                .entry(team.clone())
                .or_insert_with(|| TeamGameForecastSummaryRow {
                    team: team.clone(),
                    games: 0,
                    home_games: 0,
                    away_games: 0,
                    favored_games: 0,
                    expected_standings_points: 0.0,
                });
            row.games += 1;
            row.home_games += usize::from(home);
            row.away_games += usize::from(!home);
            row.favored_games += usize::from(game.favored_team == *team);
            row.expected_standings_points += expected;
        }
    }
    rows.into_values().collect()
}

fn probability(value: f64, game_id: u64, label: &str) -> Result<f64, TeamGamePredictionEdgeError> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(TeamGamePredictionEdgeError::InvalidEvidence {
            game_id,
            message: format!("{label} probability is invalid"),
        });
    }
    Ok(value.clamp(1e-6, 1.0 - 1e-6))
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

fn default_calibration_slope() -> f64 {
    1.0
}

fn fingerprint<T: Serialize>(value: &T) -> Result<String, TeamGamePredictionEdgeError> {
    let mut material = serde_json::to_value(value)
        .map_err(|error| TeamGamePredictionEdgeError::Serialization(error.to_string()))?;
    if let Some(object) = material.as_object_mut() {
        object.insert(
            "fingerprint".to_owned(),
            serde_json::Value::String(String::new()),
        );
    }
    let bytes = serde_json::to_vec(&material)
        .map_err(|error| TeamGamePredictionEdgeError::Serialization(error.to_string()))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn valid_sha256(value: Option<&str>) -> bool {
    value
        .and_then(|value| value.strip_prefix("sha256:"))
        .is_some_and(|hash| hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;
    use crate::{
        build_team_game_forecast, TeamForecastGameInput, TeamForecastParameters,
        TeamForecastStrengthInput,
    };

    fn source() -> TeamGameForecastView {
        build_team_game_forecast(
            20_262_027,
            vec![TeamForecastGameInput {
                game_id: 1,
                date: NaiveDate::from_ymd_opt(2026, 10, 10).unwrap(),
                away_team: "SEA".to_owned(),
                home_team: "NYR".to_owned(),
                away_score: None,
                home_score: None,
                final_result: false,
                last_period: None,
            }],
            vec![
                TeamForecastStrengthInput {
                    team: "SEA".to_owned(),
                    strength: 49.0,
                },
                TeamForecastStrengthInput {
                    team: "NYR".to_owned(),
                    strength: 52.0,
                },
            ],
            TeamForecastParameters::default(),
            None,
            None,
        )
        .unwrap()
    }

    fn team(
        team: &str,
        roster: f64,
        goalie: f64,
        state: TeamGameEvidenceState,
    ) -> TeamGamePredictionTeamEvidence {
        TeamGamePredictionTeamEvidence {
            team: team.to_owned(),
            roster_strength: Some(roster),
            roster_state: TeamGameEvidenceState::Modeled,
            availability_strength: Some(roster),
            availability_state: TeamGameEvidenceState::Reported,
            lineup_impact: None,
            lineup_impact_state: TeamGameEvidenceState::Unavailable,
            goalie_quality: Some(goalie),
            goalie_state: state,
            goalie_player_id: Some(if team == "NYR" { 1 } else { 2 }),
            goalie_form_quality: Some(goalie),
            goalie_form_appearances: 5,
            goalie_form_state: state,
            goalie_workload_readiness: Some(if team == "NYR" { 90.0 } else { 70.0 }),
            xg_share: Some(if team == "NYR" { 0.54 } else { 0.49 }),
            xg_games: 12,
            opponent_adjusted_xg_share: None,
            opponent_adjusted_xg_games: 0,
            special_teams_strength: Some(if team == "NYR" { 54.0 } else { 48.0 }),
            special_teams_games: 20,
            matchup_suitability: Some(if team == "NYR" { 0.2 } else { -0.1 }),
            matchup_state: TeamGameEvidenceState::Modeled,
            source_fingerprints: vec![format!("sha256:{}", "a".repeat(64))],
        }
    }

    fn evidence() -> TeamGamePredictionEvidenceInput {
        TeamGamePredictionEvidenceInput {
            game_id: 1,
            forecast_at: Utc.with_ymd_and_hms(2026, 9, 30, 12, 0, 0).unwrap(),
            captured_at: Utc.with_ymd_and_hms(2026, 9, 30, 11, 0, 0).unwrap(),
            away: team("SEA", 49.0, 48.0, TeamGameEvidenceState::Modeled),
            home: team("NYR", 54.0, 56.0, TeamGameEvidenceState::Reported),
        }
    }

    #[test]
    fn edge_overlay_reconciles_features_into_simulation_ready_forecast() {
        let view = build_team_game_prediction_edge(
            &source(),
            TeamGameForecastVintage::Preseason,
            Utc.with_ymd_and_hms(2026, 9, 30, 13, 0, 0).unwrap(),
            TeamGamePredictionModel::evaluation_challenger(),
            vec![evidence()],
        )
        .unwrap();
        assert_eq!(view.games[0].available_features, 6);
        assert_eq!(view.games[0].expected_features, 6);
        assert!(view.games[0].evidence_confidence.is_some());
        assert!(
            view.games[0].stability_low_home_win_probability.unwrap()
                <= view.games[0].enhanced_home_win_probability
        );
        assert!(
            view.games[0].enhanced_home_win_probability
                <= view.games[0].stability_high_home_win_probability.unwrap()
        );
        assert_eq!(
            view.enhanced_forecast.forecast_mode,
            "prediction_edge::preseason"
        );
        assert!(
            view.games[0].enhanced_home_win_probability
                > view.games[0].blended_home_win_probability
        );
        assert_eq!(
            view.enhanced_forecast.games[0].home_overall_win_probability,
            view.games[0].enhanced_home_win_probability
        );
        assert_eq!(view.enhanced_forecast.teams[0].games, 1);
        assert!(view.fingerprint.starts_with("sha256:"));
        view.validate().unwrap();
    }

    #[test]
    fn frozen_edge_becomes_training_observation_only_when_outcome_is_joined_later() {
        let view = build_team_game_prediction_edge(
            &source(),
            TeamGameForecastVintage::Preseason,
            Utc.with_ymd_and_hms(2026, 9, 30, 13, 0, 0).unwrap(),
            TeamGamePredictionModel::evaluation_challenger(),
            vec![evidence()],
        )
        .unwrap();
        let observation = super::super::team_game_prediction_training::build_team_game_prediction_training_observation(
            &view,
            1,
            Utc.with_ymd_and_hms(2026, 10, 10, 23, 0, 0).unwrap(),
            true,
        )
        .unwrap();
        assert_eq!(observation.home_team, "NYR");
        assert_eq!(
            observation.roster_difference,
            view.games[0].factors[0].effective_home_minus_away
        );
        assert_eq!(observation.source_fingerprints[1], view.fingerprint);

        let set = super::super::team_game_prediction_training::build_team_game_prediction_observation_set(
            std::slice::from_ref(&view),
            &[super::super::team_game_prediction_training::TeamGamePredictionOutcomeInput {
                season: 20_262_027,
                game_id: 1,
                outcome_recorded_at: Utc.with_ymd_and_hms(2026, 10, 10, 23, 0, 0).unwrap(),
                home_won: true,
                source_fingerprint: format!("sha256:{}", "b".repeat(64)),
            }],
            Utc.with_ymd_and_hms(2026, 10, 11, 12, 0, 0).unwrap(),
        )
        .unwrap();
        assert_eq!(set.observations.len(), 1);
        set.validate().unwrap();
    }

    #[test]
    fn missing_game_day_evidence_stays_unavailable_instead_of_zero_filled() {
        let view = build_team_game_prediction_edge(
            &source(),
            TeamGameForecastVintage::Preseason,
            Utc.with_ymd_and_hms(2026, 9, 30, 13, 0, 0).unwrap(),
            TeamGamePredictionModel::evaluation_challenger(),
            Vec::new(),
        )
        .unwrap();
        assert_eq!(view.games[0].available_features, 0);
        assert_eq!(view.games[0].expected_features, 6);
        assert_eq!(view.games[0].evidence_confidence, Some(0.0));
        assert!(view.games[0].factors.iter().all(|factor| !factor.available));
        assert_eq!(view.warnings.len(), 1);
    }

    #[test]
    fn late_evidence_is_refused() {
        let mut row = evidence();
        row.captured_at = Utc.with_ymd_and_hms(2026, 10, 1, 0, 0, 0).unwrap();
        let error = build_team_game_prediction_edge(
            &source(),
            TeamGameForecastVintage::Preseason,
            Utc.with_ymd_and_hms(2026, 10, 1, 1, 0, 0).unwrap(),
            TeamGamePredictionModel::evaluation_challenger(),
            vec![row],
        )
        .unwrap_err();
        assert!(error.to_string().contains("evidence boundary"));
    }

    #[test]
    fn morning_vintage_requires_the_game_date() {
        let error = build_team_game_prediction_edge(
            &source(),
            TeamGameForecastVintage::GameMorning,
            Utc.with_ymd_and_hms(2026, 9, 30, 13, 0, 0).unwrap(),
            TeamGamePredictionModel::evaluation_challenger(),
            vec![evidence()],
        )
        .unwrap_err();
        assert!(error.to_string().contains("game date"));
    }

    #[test]
    fn production_model_requires_training_authority() {
        let mut model = TeamGamePredictionModel::evaluation_challenger();
        model.authority = TeamGamePredictionModelAuthority::Production;
        assert!(model.validate().is_err());
    }

    #[test]
    fn identity_calibration_preserves_legacy_edge_fingerprints() {
        let view = build_team_game_prediction_edge(
            &source(),
            TeamGameForecastVintage::Preseason,
            Utc.with_ymd_and_hms(2026, 9, 30, 13, 0, 0).unwrap(),
            TeamGamePredictionModel::evaluation_challenger(),
            vec![evidence()],
        )
        .unwrap();
        let mut wire = serde_json::to_value(&view).unwrap();
        let model = wire["model"].as_object_mut().unwrap();
        assert!(model.remove("calibration_intercept").is_none());
        assert!(model.remove("calibration_slope").is_none());
        let decoded: TeamGamePredictionEdgeView = serde_json::from_value(wire).unwrap();
        decoded.validate().unwrap();
    }
}
