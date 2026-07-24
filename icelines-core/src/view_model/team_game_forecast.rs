use std::collections::{BTreeMap, BTreeSet};

use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};

pub const TEAM_GAME_FORECAST_SCHEMA: &str = "team_game_forecast.v1";
pub const TEAM_GAME_FORECAST_VALIDATION_SCHEMA: &str = "team_game_forecast_validation.v1";

#[derive(Debug, Clone, PartialEq)]
pub struct TeamForecastGameInput {
    pub game_id: u64,
    pub date: NaiveDate,
    pub away_team: String,
    pub home_team: String,
    /// Final scores are evaluation labels only and never enter forecast features.
    pub away_score: Option<u8>,
    pub home_score: Option<u8>,
    pub final_result: bool,
    pub last_period: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TeamForecastStrengthInput {
    pub team: String,
    /// Comparable 0-100 roster/depth strength.
    pub strength: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamForecastReplayConfig {
    /// Neutral/result-regressed starting strength used when no dated roster prior exists.
    pub prior_strength: f64,
    /// Equivalent games assigned to the prior before current-season evidence dominates.
    pub prior_games: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TeamForecastPersonnelEvidenceInput {
    pub event_id: String,
    pub date: NaiveDate,
    pub team: String,
    pub kind: String,
    pub label: String,
    pub source: String,
    /// +1 for an unambiguous IR placement, -1 for an unambiguous activation.
    pub availability_delta: i8,
    pub resolved_players: Vec<TeamForecastPersonnelPlayerInput>,
    pub ambiguous_player_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TeamForecastPersonnelPlayerInput {
    pub player_id: u32,
    pub full_name: String,
    pub action: String,
    /// +1 clear NHL active-roster addition, -1 clear active-roster removal,
    /// 0 organization/availability/administrative/ambiguous evidence.
    pub membership_delta: i8,
    pub prior_position_group: Option<String>,
    pub prior_season: Option<u32>,
    pub prior_games_played: Option<u32>,
    /// Regressed 0-100 value from the completed season before the replay.
    pub prior_value: Option<f64>,
}

impl Default for TeamForecastReplayConfig {
    fn default() -> Self {
        Self {
            prior_strength: 50.0,
            prior_games: 20.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamForecastParameters {
    pub name: String,
    pub home_edge: f64,
    pub strength_edge_scale: f64,
    pub back_to_back_edge: f64,
    pub three_in_four_edge: f64,
    pub travel_edge_per_1000_km: f64,
    pub timezone_edge: f64,
    pub overtime_probability: f64,
}

impl Default for TeamForecastParameters {
    fn default() -> Self {
        Self {
            name: "icecast-baseline-v1".to_owned(),
            home_edge: 0.035,
            strength_edge_scale: 0.20,
            back_to_back_edge: 0.020,
            three_in_four_edge: 0.012,
            travel_edge_per_1000_km: 0.003,
            timezone_edge: 0.004,
            overtime_probability: 0.23,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGameScheduleContext {
    pub rest_days: Option<i64>,
    pub back_to_back: bool,
    pub three_in_four: bool,
    pub four_in_six: bool,
    pub road_trip_index: usize,
    pub home_stand_index: usize,
    pub travel_km: f64,
    pub timezone_displacement_hours: i8,
    pub post_all_star_break: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGameForecastFactorRow {
    pub key: String,
    pub label: String,
    /// Signed change to overall home-win probability, not betting value.
    pub home_win_probability_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGameForecastRow {
    pub game_id: u64,
    pub date: NaiveDate,
    pub away_team: String,
    pub home_team: String,
    pub away_strength: f64,
    pub home_strength: f64,
    pub home_regulation_win_probability: f64,
    pub away_regulation_win_probability: f64,
    pub overtime_probability: f64,
    pub home_overtime_win_probability: f64,
    pub home_overall_win_probability: f64,
    pub away_overall_win_probability: f64,
    /// Leakage-safe comparison forecast using only the configured home edge.
    #[serde(default)]
    pub home_only_home_win_probability: f64,
    /// Leakage-safe chronological Elo comparison forecast.
    #[serde(default)]
    pub elo_home_win_probability: f64,
    /// Points-only rolling comparison forecast; absent outside rolling replay.
    #[serde(default)]
    pub standings_home_win_probability: Option<f64>,
    pub home_expected_standings_points: f64,
    pub away_expected_standings_points: f64,
    pub favored_team: String,
    pub confidence: String,
    pub home_context: TeamGameScheduleContext,
    pub away_context: TeamGameScheduleContext,
    pub factors: Vec<TeamGameForecastFactorRow>,
    pub actual_away_score: Option<u8>,
    pub actual_home_score: Option<u8>,
    pub actual_winner: Option<String>,
    pub actual_ending: Option<String>,
    pub pick_correct: Option<bool>,
    pub brier_score: Option<f64>,
    #[serde(default)]
    pub binary_log_loss: Option<f64>,
    #[serde(default)]
    pub multiclass_log_loss: Option<f64>,
    /// Games completed strictly before this date that informed rolling strength.
    pub away_evidence_games: usize,
    pub home_evidence_games: usize,
    /// Exclusive evidence boundary for rolling replay; None for a frozen forecast.
    pub evidence_cutoff_date: Option<NaiveDate>,
    pub away_known_personnel_events: usize,
    pub home_known_personnel_events: usize,
    pub away_active_ir_signals: usize,
    pub home_active_ir_signals: usize,
    pub away_known_roster_additions: usize,
    pub home_known_roster_additions: usize,
    pub away_known_roster_removals: usize,
    pub home_known_roster_removals: usize,
    #[serde(default)]
    pub away_personnel_strength_delta: f64,
    #[serde(default)]
    pub home_personnel_strength_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGamePersonnelEvidenceRow {
    pub event_id: String,
    pub date: NaiveDate,
    pub team: String,
    pub kind: String,
    pub label: String,
    pub source: String,
    pub availability_delta: i8,
    pub resolved_players: Vec<TeamGamePersonnelPlayerRow>,
    pub ambiguous_player_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGamePersonnelPlayerRow {
    pub player_id: u32,
    pub full_name: String,
    pub action: String,
    pub membership_delta: i8,
    #[serde(default)]
    pub prior_position_group: Option<String>,
    pub prior_season: Option<u32>,
    pub prior_games_played: Option<u32>,
    pub prior_value: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGameMembershipIntervalRow {
    pub player_id: u32,
    pub full_name: String,
    pub team: String,
    /// Transaction date that opened the observed interval. None means a later
    /// removal only implies that membership existed before the event.
    pub start_event_date: Option<NaiveDate>,
    pub start_event_id: Option<String>,
    pub end_event_date: Option<NaiveDate>,
    pub end_event_id: Option<String>,
    pub confidence: String,
    pub opening_basis: String,
    pub prior_season: Option<u32>,
    pub prior_games_played: Option<u32>,
    pub prior_value: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGameMembershipAnomalyRow {
    pub player_id: u32,
    pub full_name: String,
    pub team: String,
    pub event_id: String,
    pub event_date: NaiveDate,
    pub prior_event_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGameOpeningRosterAuthorityRow {
    pub status: String,
    /// Calendar-date replay freezes all opening-day evidence before this day.
    pub required_before_date: NaiveDate,
    pub selected_snapshot: Option<String>,
    pub selected_snapshot_created_at: Option<String>,
    pub latest_observed_snapshot: Option<String>,
    pub latest_observed_snapshot_created_at: Option<String>,
    pub expected_teams: usize,
    pub verified_teams: usize,
    #[serde(default)]
    pub verified_team_abbrevs: Vec<String>,
    pub player_value_effects_enabled: bool,
    /// Personnel events must occur strictly after this snapshot date to be
    /// applied on top of the authoritative opening roster.
    pub personnel_events_effective_after: Option<NaiveDate>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGameOpeningStrengthRow {
    pub team: String,
    #[serde(default)]
    pub as_of_date: Option<NaiveDate>,
    pub strength: f64,
    /// Shared verified-cohort offset that makes opening strength relative to
    /// a neutral 50 without discarding between-team player-value differences.
    #[serde(default)]
    pub cohort_normalization_delta: f64,
    pub roster_players: usize,
    pub valued_players: usize,
    pub value_coverage: f64,
    pub forwards_used: usize,
    pub defensemen_used: usize,
    pub goalies_used: usize,
    #[serde(default)]
    pub players: Vec<TeamGameOpeningPlayerRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGameOpeningPlayerRow {
    pub player_id: u32,
    pub full_name: String,
    /// forward, defense, or goalie.
    pub position_group: String,
    pub prior_value: Option<f64>,
    /// Missing prior history is explicitly modeled as neutral 50.
    pub modeled_value: f64,
    pub selected_at_opening: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGamePairedTradeRow {
    pub transfer_id: String,
    pub date: NaiveDate,
    pub player_id: u32,
    pub full_name: String,
    pub from_team: String,
    pub to_team: String,
    pub source_event_ids: Vec<String>,
    pub prior_position_group: Option<String>,
    pub prior_value: Option<f64>,
    pub active_lineup_applied: bool,
    pub disposition: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGameForecastAccuracyRow {
    pub segment: String,
    pub games: usize,
    pub correct_picks: usize,
    pub pick_accuracy: f64,
    pub mean_favorite_probability: f64,
    pub brier_score: f64,
    #[serde(default)]
    pub binary_log_loss: f64,
    #[serde(default)]
    pub multiclass_log_loss: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGameForecastCalibrationRow {
    pub segment: String,
    pub games: usize,
    pub mean_home_win_probability: f64,
    pub observed_home_win_rate: f64,
    pub absolute_calibration_error: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGameForecastBaselineRow {
    pub name: String,
    pub games: usize,
    pub pick_accuracy: f64,
    pub brier_score: f64,
    pub binary_log_loss: f64,
    /// Positive values mean IceLines has lower loss than this baseline.
    pub model_brier_improvement: f64,
    /// Positive values mean IceLines has lower loss than this baseline.
    pub model_log_loss_improvement: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGameForecastAblationRow {
    pub factor: String,
    pub games: usize,
    pub games_affected: usize,
    pub pick_accuracy: f64,
    pub brier_score: f64,
    pub binary_log_loss: f64,
    pub mean_absolute_probability_delta: f64,
    /// Positive values mean the included factor improved IceLines Brier loss.
    pub model_brier_improvement: f64,
    /// Positive values mean the included factor improved IceLines log loss.
    pub model_log_loss_improvement: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGameForecastBlendRow {
    /// Weight assigned to chronological Elo; the remainder is IceLines.
    pub elo_weight: f64,
    pub games: usize,
    pub pick_accuracy: f64,
    pub brier_score: f64,
    pub binary_log_loss: f64,
    /// Positive values mean this blend improved on unblended IceLines.
    pub brier_improvement_vs_model: f64,
    /// Positive values mean this blend improved on unblended IceLines.
    pub log_loss_improvement_vs_model: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TeamGameForecastCalibrationObservation {
    pub home_win_probability: f64,
    pub home_won: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TeamGameForecastValidationInput {
    pub season: u32,
    pub games: usize,
    pub authoritative_opening_roster: bool,
    pub elo_blend_sweep: Vec<TeamGameForecastBlendRow>,
    pub calibration_observations: Vec<TeamGameForecastCalibrationObservation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGameForecastValidationCheckRow {
    pub key: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGameForecastHoldoutRow {
    pub holdout_season: u32,
    pub training_seasons: Vec<u32>,
    pub selected_elo_weight: f64,
    pub games: usize,
    pub pick_accuracy: f64,
    pub brier_score: f64,
    pub binary_log_loss: f64,
    pub brier_improvement_vs_model: f64,
    pub brier_improvement_vs_pure_elo: f64,
    pub log_loss_improvement_vs_model: f64,
    pub log_loss_improvement_vs_pure_elo: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGameForecastCalibrationHoldoutRow {
    pub holdout_season: u32,
    pub training_seasons: Vec<u32>,
    pub training_games: usize,
    pub games: usize,
    pub fitted_intercept: f64,
    pub fitted_slope: f64,
    pub uncalibrated_brier_score: f64,
    pub recalibrated_brier_score: f64,
    pub brier_improvement: f64,
    pub uncalibrated_binary_log_loss: f64,
    pub recalibrated_binary_log_loss: f64,
    pub binary_log_loss_improvement: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TeamGameForecastCalibrationSummary {
    pub holdout_seasons: usize,
    pub games: usize,
    pub holdouts_improved_brier: usize,
    pub holdouts_improved_binary_log_loss: usize,
    pub uncalibrated_brier_score: f64,
    pub recalibrated_brier_score: f64,
    pub brier_improvement: f64,
    pub brier_improvement_standard_error: f64,
    pub brier_improvement_ci95_lower: f64,
    pub brier_improvement_ci95_upper: f64,
    pub season_clustered_brier_improvement_standard_error: f64,
    pub season_clustered_brier_improvement_ci95_lower: f64,
    pub season_clustered_brier_improvement_ci95_upper: f64,
    pub season_clustered_brier_evidence: String,
    pub uncalibrated_binary_log_loss: f64,
    pub recalibrated_binary_log_loss: f64,
    pub binary_log_loss_improvement: f64,
    pub binary_log_loss_improvement_standard_error: f64,
    pub binary_log_loss_improvement_ci95_lower: f64,
    pub binary_log_loss_improvement_ci95_upper: f64,
    pub season_clustered_binary_log_loss_improvement_standard_error: f64,
    pub season_clustered_binary_log_loss_improvement_ci95_lower: f64,
    pub season_clustered_binary_log_loss_improvement_ci95_upper: f64,
    pub season_clustered_binary_log_loss_evidence: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGameForecastValidationView {
    pub schema: String,
    pub seasons: Vec<u32>,
    pub total_games: usize,
    pub pooled_sweep: Vec<TeamGameForecastBlendRow>,
    pub pooled_best_by_brier: TeamGameForecastBlendRow,
    pub holdouts: Vec<TeamGameForecastHoldoutRow>,
    pub calibration_holdouts: Vec<TeamGameForecastCalibrationHoldoutRow>,
    #[serde(default)]
    pub calibration_summary: TeamGameForecastCalibrationSummary,
    pub authoritative_opening_roster_seasons: usize,
    pub promotion_status: String,
    pub promotion_checks: Vec<TeamGameForecastValidationCheckRow>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGameForecastAccuracySummary {
    pub final_games: usize,
    pub pending_games: usize,
    pub correct_picks: usize,
    pub pick_accuracy: f64,
    pub brier_score: f64,
    #[serde(default)]
    pub binary_log_loss: f64,
    #[serde(default)]
    pub multiclass_log_loss: Option<f64>,
    /// Positive values outperform an always-50% binary winner forecast.
    #[serde(default)]
    pub brier_skill_vs_coinflip: f64,
    /// Positive values outperform an always-50% binary winner forecast.
    #[serde(default)]
    pub binary_log_loss_skill_vs_coinflip: f64,
    /// Positive values outperform an equal 1/3 regulation-home,
    /// regulation-away, and overtime forecast.
    #[serde(default)]
    pub multiclass_log_loss_skill_vs_uniform: Option<f64>,
    #[serde(default)]
    pub expected_calibration_error: f64,
    /// Ideal value is zero. Estimated by logistic recalibration of home-win
    /// outcomes on the forecast log odds.
    #[serde(default)]
    pub calibration_intercept: Option<f64>,
    /// Ideal value is one. Values below one indicate over-dispersed confidence;
    /// values above one indicate under-dispersed confidence.
    #[serde(default)]
    pub calibration_slope: Option<f64>,
    #[serde(default)]
    pub calibration_intercept_standard_error: Option<f64>,
    #[serde(default)]
    pub calibration_slope_standard_error: Option<f64>,
    #[serde(default)]
    pub calibration_intercept_ci95_lower: Option<f64>,
    #[serde(default)]
    pub calibration_intercept_ci95_upper: Option<f64>,
    #[serde(default)]
    pub calibration_slope_ci95_lower: Option<f64>,
    #[serde(default)]
    pub calibration_slope_ci95_upper: Option<f64>,
    pub by_confidence: Vec<TeamGameForecastAccuracyRow>,
    #[serde(default)]
    pub calibration_bins: Vec<TeamGameForecastCalibrationRow>,
    #[serde(default)]
    pub baselines: Vec<TeamGameForecastBaselineRow>,
    #[serde(default)]
    pub ablations: Vec<TeamGameForecastAblationRow>,
    #[serde(default)]
    pub elo_blend_sweep: Vec<TeamGameForecastBlendRow>,
    #[serde(default)]
    pub best_elo_blend_by_brier: Option<TeamGameForecastBlendRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGameForecastSummaryRow {
    pub team: String,
    pub games: usize,
    pub home_games: usize,
    pub away_games: usize,
    pub favored_games: usize,
    pub expected_standings_points: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGameForecastView {
    pub schema: String,
    pub season: u32,
    pub schedule_games: usize,
    pub schedule_start: NaiveDate,
    pub schedule_end: NaiveDate,
    pub parameters: TeamForecastParameters,
    pub forecast_mode: String,
    pub games: Vec<TeamGameForecastRow>,
    pub teams: Vec<TeamGameForecastSummaryRow>,
    pub accuracy: Option<TeamGameForecastAccuracySummary>,
    pub personnel_evidence: Vec<TeamGamePersonnelEvidenceRow>,
    pub membership_intervals: Vec<TeamGameMembershipIntervalRow>,
    pub membership_anomalies: Vec<TeamGameMembershipAnomalyRow>,
    #[serde(default)]
    pub opening_roster_authority: Option<TeamGameOpeningRosterAuthorityRow>,
    #[serde(default)]
    pub opening_strengths: Vec<TeamGameOpeningStrengthRow>,
    #[serde(default)]
    pub paired_trades: Vec<TeamGamePairedTradeRow>,
    pub warnings: Vec<String>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct TeamScheduleState {
    last_date: Option<NaiveDate>,
    recent_dates: Vec<NaiveDate>,
    away_run: usize,
    home_run: usize,
    previous_venue: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct TeamReplayState {
    games: usize,
    standings_points: usize,
    goals_for: usize,
    goals_against: usize,
    known_personnel_events: usize,
    active_ir_signals: usize,
    known_roster_additions: usize,
    known_roster_removals: usize,
}

#[derive(Debug)]
struct PendingEloResult {
    away_team: String,
    home_team: String,
    home_score: f64,
    expected_home_score: f64,
}

#[derive(Debug)]
struct PendingReplayResult {
    away_team: String,
    home_team: String,
    away_score: u8,
    home_score: u8,
    overtime: bool,
}

pub fn build_team_game_forecast(
    season: u32,
    games: Vec<TeamForecastGameInput>,
    strengths: Vec<TeamForecastStrengthInput>,
    parameters: TeamForecastParameters,
    expected_league_games: Option<usize>,
    expected_games_per_team: Option<usize>,
) -> Result<TeamGameForecastView, String> {
    build_team_game_forecast_impl(
        season,
        games,
        strengths,
        parameters,
        expected_league_games,
        expected_games_per_team,
        None,
    )
}

pub fn build_team_game_rolling_replay(
    season: u32,
    games: Vec<TeamForecastGameInput>,
    parameters: TeamForecastParameters,
    expected_league_games: Option<usize>,
    expected_games_per_team: Option<usize>,
    replay: TeamForecastReplayConfig,
) -> Result<TeamGameForecastView, String> {
    build_team_game_rolling_replay_with_personnel(
        season,
        games,
        parameters,
        expected_league_games,
        expected_games_per_team,
        replay,
        Vec::new(),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_team_game_rolling_replay_with_personnel(
    season: u32,
    games: Vec<TeamForecastGameInput>,
    parameters: TeamForecastParameters,
    expected_league_games: Option<usize>,
    expected_games_per_team: Option<usize>,
    replay: TeamForecastReplayConfig,
    personnel_evidence: Vec<TeamForecastPersonnelEvidenceInput>,
) -> Result<TeamGameForecastView, String> {
    build_team_game_rolling_replay_with_opening_strengths(
        season,
        games,
        parameters,
        expected_league_games,
        expected_games_per_team,
        replay,
        personnel_evidence,
        Vec::new(),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_team_game_rolling_replay_with_opening_strengths(
    season: u32,
    games: Vec<TeamForecastGameInput>,
    parameters: TeamForecastParameters,
    expected_league_games: Option<usize>,
    expected_games_per_team: Option<usize>,
    replay: TeamForecastReplayConfig,
    personnel_evidence: Vec<TeamForecastPersonnelEvidenceInput>,
    opening_strengths: Vec<TeamGameOpeningStrengthRow>,
) -> Result<TeamGameForecastView, String> {
    if !replay.prior_strength.is_finite() || !(0.0..=100.0).contains(&replay.prior_strength) {
        return Err("IceReplay prior strength must be finite and between 0 and 100".to_owned());
    }
    if !replay.prior_games.is_finite() || replay.prior_games <= 0.0 {
        return Err("IceReplay prior games must be finite and greater than zero".to_owned());
    }
    build_team_game_forecast_impl(
        season,
        games,
        Vec::new(),
        parameters,
        expected_league_games,
        expected_games_per_team,
        Some((replay, personnel_evidence, opening_strengths)),
    )
}

fn build_team_game_forecast_impl(
    season: u32,
    mut games: Vec<TeamForecastGameInput>,
    strengths: Vec<TeamForecastStrengthInput>,
    parameters: TeamForecastParameters,
    expected_league_games: Option<usize>,
    expected_games_per_team: Option<usize>,
    replay: Option<(
        TeamForecastReplayConfig,
        Vec<TeamForecastPersonnelEvidenceInput>,
        Vec<TeamGameOpeningStrengthRow>,
    )>,
) -> Result<TeamGameForecastView, String> {
    validate_parameters(&parameters)?;
    if games.is_empty() {
        return Err("IceCast schedule has no games".to_owned());
    }
    games.sort_by(|a, b| a.date.cmp(&b.date).then_with(|| a.game_id.cmp(&b.game_id)));
    let unique = games
        .iter()
        .map(|game| game.game_id)
        .collect::<BTreeSet<_>>();
    if unique.len() != games.len() {
        return Err("IceCast schedule contains duplicate game IDs".to_owned());
    }
    if expected_league_games.is_some_and(|expected| expected != games.len()) {
        return Err(format!(
            "IceCast expected {} league games but loaded {}",
            expected_league_games.unwrap(),
            games.len()
        ));
    }
    let mut appearances: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for game in &games {
        if game.away_team == game.home_team {
            return Err(format!(
                "game {} has the same home and away team",
                game.game_id
            ));
        }
        appearances.entry(game.away_team.clone()).or_default().1 += 1;
        appearances.entry(game.home_team.clone()).or_default().0 += 1;
    }
    if let Some(expected) = expected_games_per_team {
        for (team, (home, away)) in &appearances {
            if home + away != expected || *home != expected / 2 || *away != expected / 2 {
                return Err(format!(
                    "IceCast schedule for {team} is {home} home/{away} away; expected {}/{}",
                    expected / 2,
                    expected / 2
                ));
            }
        }
    }
    if strengths
        .iter()
        .any(|input| !input.strength.is_finite() || !(0.0..=100.0).contains(&input.strength))
    {
        return Err("IceCast team strength must be finite and between 0 and 100".to_owned());
    }
    let strength_by_team = strengths
        .into_iter()
        .map(|input| (input.team.trim().to_ascii_uppercase(), input.strength))
        .collect::<BTreeMap<_, _>>();
    let missing_strength = appearances
        .keys()
        .filter(|team| !strength_by_team.contains_key(*team))
        .cloned()
        .collect::<Vec<_>>();
    let replay_config = replay.as_ref().map(|(config, _, _)| config);
    let opening_strengths = replay
        .as_ref()
        .map(|(_, _, strengths)| strengths.clone())
        .unwrap_or_default();
    if opening_strengths.iter().any(|row| {
        !row.strength.is_finite()
            || !(0.0..=100.0).contains(&row.strength)
            || !row.cohort_normalization_delta.is_finite()
            || !row.value_coverage.is_finite()
            || !(0.0..=1.0).contains(&row.value_coverage)
            || row.valued_players > row.roster_players
            || row.players.len() != row.roster_players
            || row.players.iter().any(|player| {
                !player.modeled_value.is_finite()
                    || !(0.0..=100.0).contains(&player.modeled_value)
                    || player
                        .prior_value
                        .is_some_and(|value| !value.is_finite() || !(0.0..=100.0).contains(&value))
                    || !matches!(
                        player.position_group.as_str(),
                        "forward" | "defense" | "goalie"
                    )
            })
            || row
                .players
                .iter()
                .map(|player| player.player_id)
                .collect::<BTreeSet<_>>()
                .len()
                != row.players.len()
            || {
                let membership = row
                    .players
                    .iter()
                    .map(|player| ((row.team.clone(), player.player_id), true))
                    .collect::<BTreeMap<_, _>>();
                let roster_players = row
                    .players
                    .iter()
                    .map(|player| ((row.team.clone(), player.player_id), player.clone()))
                    .collect::<BTreeMap<_, _>>();
                (replay_roster_strength(row, &roster_players, &membership, &BTreeSet::new())
                    - row.strength)
                    .abs()
                    > 1e-9
            }
    }) {
        return Err("IceReplay opening strengths contain invalid values or coverage".to_owned());
    }
    let opening_strength_by_team = opening_strengths
        .iter()
        .map(|row| (row.team.trim().to_ascii_uppercase(), row.strength))
        .collect::<BTreeMap<_, _>>();
    let opening_strength_index = opening_strengths
        .iter()
        .enumerate()
        .map(|(index, row)| (row.team.trim().to_ascii_uppercase(), index))
        .collect::<BTreeMap<_, _>>();
    let opening_evidence_cutoff_by_team = opening_strengths
        .iter()
        .filter_map(|row| {
            row.as_of_date
                .map(|date| (row.team.trim().to_ascii_uppercase(), date))
        })
        .collect::<BTreeMap<_, _>>();
    let mut personnel_inputs = replay
        .as_ref()
        .map(|(_, evidence, _)| evidence.clone())
        .unwrap_or_default();
    personnel_inputs.sort_by(|a, b| {
        a.date
            .cmp(&b.date)
            .then_with(|| a.event_id.cmp(&b.event_id))
    });
    if personnel_inputs
        .iter()
        .any(|event| !(-1..=1).contains(&event.availability_delta))
    {
        return Err("IceReplay personnel availability delta must be -1, 0, or 1".to_owned());
    }
    let unique_personnel_ids = personnel_inputs
        .iter()
        .map(|event| event.event_id.as_str())
        .collect::<BTreeSet<_>>();
    if unique_personnel_ids.len() != personnel_inputs.len() {
        return Err("IceReplay personnel evidence contains duplicate event IDs".to_owned());
    }
    let personnel_evidence = personnel_inputs
        .iter()
        .filter(|event| event.date <= games.last().unwrap().date)
        .map(|event| TeamGamePersonnelEvidenceRow {
            event_id: event.event_id.clone(),
            date: event.date,
            team: event.team.clone(),
            kind: event.kind.clone(),
            label: event.label.clone(),
            source: event.source.clone(),
            availability_delta: event.availability_delta,
            resolved_players: event
                .resolved_players
                .iter()
                .map(|player| TeamGamePersonnelPlayerRow {
                    player_id: player.player_id,
                    full_name: player.full_name.clone(),
                    action: player.action.clone(),
                    membership_delta: player.membership_delta,
                    prior_position_group: player.prior_position_group.clone(),
                    prior_season: player.prior_season,
                    prior_games_played: player.prior_games_played,
                    prior_value: player.prior_value,
                })
                .collect(),
            ambiguous_player_names: event.ambiguous_player_names.clone(),
        })
        .collect::<Vec<_>>();
    let (membership_intervals, membership_anomalies) =
        build_membership_intervals(&personnel_inputs, games.last().unwrap().date);
    let (mut paired_trades, paired_trade_by_event_player) =
        build_paired_trades(&personnel_inputs, games.last().unwrap().date);
    let mut personnel_index = 0;
    let mut states: BTreeMap<String, TeamScheduleState> = BTreeMap::new();
    let mut replay_states: BTreeMap<String, TeamReplayState> = BTreeMap::new();
    let mut replay_membership = BTreeMap::<(String, u32), bool>::new();
    let mut replay_roster_players = BTreeMap::<(String, u32), TeamGameOpeningPlayerRow>::new();
    for row in &opening_strengths {
        for player in &row.players {
            let key = (row.team.clone(), player.player_id);
            replay_membership.insert(key.clone(), true);
            replay_roster_players.insert(key, player.clone());
        }
    }
    let mut replay_ir_players = BTreeSet::<(String, u32)>::new();
    let mut applied_paired_trades = BTreeSet::<String>::new();
    let mut pending_results = Vec::new();
    let mut elo_ratings = BTreeMap::<String, f64>::new();
    let mut pending_elo_results = Vec::new();
    let mut current_date = None;
    let mut rows = Vec::with_capacity(games.len());
    for game in &games {
        if current_date != Some(game.date) {
            apply_pending_replay_results(&mut replay_states, &mut pending_results);
            apply_pending_elo_results(&mut elo_ratings, &mut pending_elo_results);
            while personnel_index < personnel_inputs.len()
                && personnel_inputs[personnel_index].date < game.date
            {
                let event = &personnel_inputs[personnel_index];
                let already_reflected = opening_evidence_cutoff_by_team
                    .get(&event.team)
                    .is_some_and(|cutoff| event.date <= *cutoff);
                if !already_reflected {
                    apply_paired_trades_for_event(
                        &mut replay_states,
                        &mut replay_roster_players,
                        &mut replay_membership,
                        &mut replay_ir_players,
                        &mut paired_trades,
                        &paired_trade_by_event_player,
                        &mut applied_paired_trades,
                        event,
                    );
                    apply_personnel_evidence(
                        &mut replay_states,
                        &mut replay_roster_players,
                        &mut replay_membership,
                        &mut replay_ir_players,
                        event,
                    );
                }
                personnel_index += 1;
            }
            current_date = Some(game.date);
        }
        let home_state = states.get(&game.home_team).cloned().unwrap_or_default();
        let away_state = states.get(&game.away_team).cloned().unwrap_or_default();
        let home_context = schedule_context(&game.home_team, true, game.date, &home_state);
        let away_context = schedule_context(&game.home_team, false, game.date, &away_state);
        let home_evidence_games = replay_states
            .get(&game.home_team)
            .map_or(0, |state| state.games);
        let away_evidence_games = replay_states
            .get(&game.away_team)
            .map_or(0, |state| state.games);
        let home_current_opening = opening_strength_index.get(&game.home_team).map(|index| {
            replay_roster_strength(
                &opening_strengths[*index],
                &replay_roster_players,
                &replay_membership,
                &replay_ir_players,
            )
        });
        let away_current_opening = opening_strength_index.get(&game.away_team).map(|index| {
            replay_roster_strength(
                &opening_strengths[*index],
                &replay_roster_players,
                &replay_membership,
                &replay_ir_players,
            )
        });
        let home_personnel_strength_delta = home_current_opening
            .zip(opening_strength_by_team.get(&game.home_team).copied())
            .map_or(0.0, |(current, opening)| current - opening);
        let away_personnel_strength_delta = away_current_opening
            .zip(opening_strength_by_team.get(&game.away_team).copied())
            .map_or(0.0, |(current, opening)| current - opening);
        let home_strength = replay_config.map_or_else(
            || {
                strength_by_team
                    .get(&game.home_team)
                    .copied()
                    .unwrap_or(50.0)
            },
            |config| {
                rolling_replay_strength(
                    replay_states.get(&game.home_team),
                    config,
                    home_current_opening,
                )
            },
        );
        let away_strength = replay_config.map_or_else(
            || {
                strength_by_team
                    .get(&game.away_team)
                    .copied()
                    .unwrap_or(50.0)
            },
            |config| {
                rolling_replay_strength(
                    replay_states.get(&game.away_team),
                    config,
                    away_current_opening,
                )
            },
        );
        let standings_home_win_probability = replay_config.map(|config| {
            let home_standings_strength =
                rolling_standings_strength(replay_states.get(&game.home_team), config);
            let away_standings_strength =
                rolling_standings_strength(replay_states.get(&game.away_team), config);
            let edge = parameters.home_edge
                + ((home_standings_strength - away_standings_strength) / 100.0)
                    * parameters.strength_edge_scale;
            home_win_probability_from_edge(edge, &parameters)
        });
        let mut raw_factors = edge_factors(
            home_strength,
            away_strength,
            &home_context,
            &away_context,
            &parameters,
        );
        if let Some(config) = replay_config {
            let home_neutral =
                rolling_replay_strength(replay_states.get(&game.home_team), config, None);
            let away_neutral =
                rolling_replay_strength(replay_states.get(&game.away_team), config, None);
            let home_opening = rolling_replay_strength(
                replay_states.get(&game.home_team),
                config,
                opening_strength_by_team.get(&game.home_team).copied(),
            );
            let away_opening = rolling_replay_strength(
                replay_states.get(&game.away_team),
                config,
                opening_strength_by_team.get(&game.away_team).copied(),
            );
            let edge_for_strengths =
                |home: f64, away: f64| ((home - away) / 100.0) * parameters.strength_edge_scale;
            let neutral_edge = edge_for_strengths(home_neutral, away_neutral);
            let opening_edge = edge_for_strengths(home_opening, away_opening);
            let current_edge = edge_for_strengths(home_strength, away_strength);
            if let Some(strength) = raw_factors
                .iter_mut()
                .find(|(key, _, _)| *key == "strength")
            {
                strength.1 = format!(
                    "rolling results strength {:.1} vs {:.1}",
                    home_neutral, away_neutral
                );
                strength.2 = neutral_edge;
            }
            raw_factors.push((
                "opening_roster",
                format!(
                    "opening roster prior {:.1} vs {:.1}",
                    home_opening, away_opening
                ),
                opening_edge - neutral_edge,
            ));
            raw_factors.push((
                "personnel",
                format!(
                    "post-opening personnel {:.1} vs {:.1}",
                    home_strength, away_strength
                ),
                current_edge - opening_edge,
            ));
        }
        let raw_edge = raw_factors.iter().map(|(_, _, value)| *value).sum::<f64>();
        let edge = raw_edge.clamp(-0.24, 0.24);
        let overtime = parameters.overtime_probability;
        let home_reg = (1.0 - overtime) * (0.5 + edge);
        let away_reg = (1.0 - overtime) * (0.5 - edge);
        let home_ot = (0.5 + edge * 0.5).clamp(0.25, 0.75);
        let home_overall = home_reg + overtime * home_ot;
        let away_overall = 1.0 - home_overall;
        let home_only_home_win_probability = home_only_win_probability(&parameters);
        let elo_home_win_probability = elo_home_win_probability(
            elo_ratings.get(&game.home_team).copied().unwrap_or(1500.0),
            elo_ratings.get(&game.away_team).copied().unwrap_or(1500.0),
        );
        let attribution_scale = (1.0 - overtime * 0.5)
            * if raw_edge.abs() > f64::EPSILON {
                edge / raw_edge
            } else {
                1.0
            };
        let factors = raw_factors
            .into_iter()
            .filter(|(_, _, value)| value.abs() > f64::EPSILON)
            .map(|(key, label, value)| TeamGameForecastFactorRow {
                key: key.to_owned(),
                label,
                home_win_probability_delta: value * attribution_scale,
            })
            .collect::<Vec<_>>();
        let favorite_probability = home_overall.max(away_overall);
        let graded_result = grade_result(game);
        rows.push(TeamGameForecastRow {
            game_id: game.game_id,
            date: game.date,
            away_team: game.away_team.clone(),
            home_team: game.home_team.clone(),
            away_strength,
            home_strength,
            home_regulation_win_probability: home_reg,
            away_regulation_win_probability: away_reg,
            overtime_probability: overtime,
            home_overtime_win_probability: home_ot,
            home_overall_win_probability: home_overall,
            away_overall_win_probability: away_overall,
            home_only_home_win_probability,
            elo_home_win_probability,
            standings_home_win_probability,
            home_expected_standings_points: home_reg * 2.0 + overtime * (1.0 + home_ot),
            away_expected_standings_points: away_reg * 2.0 + overtime * (2.0 - home_ot),
            favored_team: if home_overall >= away_overall {
                game.home_team.clone()
            } else {
                game.away_team.clone()
            },
            confidence: if favorite_probability >= 0.62 {
                "strong"
            } else if favorite_probability >= 0.55 {
                "lean"
            } else {
                "toss_up"
            }
            .to_owned(),
            home_context,
            away_context,
            factors,
            actual_away_score: graded_result.as_ref().map(|result| result.away_score),
            actual_home_score: graded_result.as_ref().map(|result| result.home_score),
            actual_winner: graded_result.as_ref().map(|result| result.winner.clone()),
            actual_ending: graded_result
                .as_ref()
                .and_then(|result| result.ending.clone()),
            pick_correct: graded_result.as_ref().map(|result| {
                result.winner
                    == if home_overall >= away_overall {
                        game.home_team.as_str()
                    } else {
                        game.away_team.as_str()
                    }
            }),
            brier_score: graded_result.as_ref().map(|result| {
                let home_won = f64::from(result.winner == game.home_team);
                (home_overall - home_won).powi(2)
            }),
            binary_log_loss: graded_result
                .as_ref()
                .map(|result| score_binary_log_loss(home_overall, result.winner == game.home_team)),
            multiclass_log_loss: graded_result.as_ref().and_then(|result| {
                score_multiclass_log_loss(home_reg, away_reg, overtime, result, game)
            }),
            away_evidence_games,
            home_evidence_games,
            evidence_cutoff_date: replay.as_ref().map(|_| game.date),
            away_known_personnel_events: replay_states
                .get(&game.away_team)
                .map_or(0, |state| state.known_personnel_events),
            home_known_personnel_events: replay_states
                .get(&game.home_team)
                .map_or(0, |state| state.known_personnel_events),
            away_active_ir_signals: replay_states
                .get(&game.away_team)
                .map_or(0, |state| state.active_ir_signals),
            home_active_ir_signals: replay_states
                .get(&game.home_team)
                .map_or(0, |state| state.active_ir_signals),
            away_known_roster_additions: replay_states
                .get(&game.away_team)
                .map_or(0, |state| state.known_roster_additions),
            home_known_roster_additions: replay_states
                .get(&game.home_team)
                .map_or(0, |state| state.known_roster_additions),
            away_known_roster_removals: replay_states
                .get(&game.away_team)
                .map_or(0, |state| state.known_roster_removals),
            home_known_roster_removals: replay_states
                .get(&game.home_team)
                .map_or(0, |state| state.known_roster_removals),
            away_personnel_strength_delta,
            home_personnel_strength_delta,
        });
        if replay.is_some() {
            if let Some(result) = graded_result.as_ref() {
                pending_elo_results.push(PendingEloResult {
                    away_team: game.away_team.clone(),
                    home_team: game.home_team.clone(),
                    home_score: match result.ending.as_deref() {
                        Some("OT" | "SO") if result.winner == game.home_team => 0.75,
                        Some("OT" | "SO") => 0.25,
                        _ if result.winner == game.home_team => 1.0,
                        _ => 0.0,
                    },
                    expected_home_score: elo_home_win_probability,
                });
            }
        }
        if replay.is_some() {
            if let Some(result) = graded_result.as_ref() {
                pending_results.push(PendingReplayResult {
                    away_team: game.away_team.clone(),
                    home_team: game.home_team.clone(),
                    away_score: result.away_score,
                    home_score: result.home_score,
                    overtime: matches!(result.ending.as_deref(), Some("OT" | "SO")),
                });
            }
        }
        update_state(
            states.entry(game.home_team.clone()).or_default(),
            true,
            game.date,
            &game.home_team,
        );
        update_state(
            states.entry(game.away_team.clone()).or_default(),
            false,
            game.date,
            &game.home_team,
        );
    }
    let mut summaries = appearances
        .keys()
        .map(|team| {
            let relevant = rows
                .iter()
                .filter(|row| row.home_team == *team || row.away_team == *team)
                .collect::<Vec<_>>();
            TeamGameForecastSummaryRow {
                team: team.clone(),
                games: relevant.len(),
                home_games: relevant.iter().filter(|row| row.home_team == *team).count(),
                away_games: relevant.iter().filter(|row| row.away_team == *team).count(),
                favored_games: relevant
                    .iter()
                    .filter(|row| row.favored_team == *team)
                    .count(),
                expected_standings_points: relevant
                    .iter()
                    .map(|row| {
                        if row.home_team == *team {
                            row.home_expected_standings_points
                        } else {
                            row.away_expected_standings_points
                        }
                    })
                    .sum(),
            }
        })
        .collect::<Vec<_>>();
    summaries.sort_by(|a, b| {
        b.expected_standings_points
            .total_cmp(&a.expected_standings_points)
            .then_with(|| a.team.cmp(&b.team))
    });
    let accuracy = build_accuracy_summary(&rows, replay.is_some());
    Ok(TeamGameForecastView {
        schema: TEAM_GAME_FORECAST_SCHEMA.to_owned(),
        season,
        schedule_games: games.len(),
        schedule_start: games.first().unwrap().date,
        schedule_end: games.last().unwrap().date,
        parameters,
        forecast_mode: if replay.is_some() {
            "rolling_results_replay_v1"
        } else {
            "frozen_baseline_v1"
        }
        .to_owned(),
        games: rows,
        teams: summaries,
        accuracy,
        personnel_evidence,
        membership_intervals,
        membership_anomalies: membership_anomalies.clone(),
        opening_roster_authority: None,
        opening_strengths,
        paired_trades,
        warnings: {
            let mut warnings = Vec::new();
            if replay.is_none() && !missing_strength.is_empty() {
                warnings.push(format!(
                "missing roster/depth strength for {}; neutral 50.0 was used",
                missing_strength.join(", ")
                ));
            }
            if !membership_anomalies.is_empty() {
                warnings.push(format!(
                    "The Wire ignored {} duplicate membership transition(s) that would have created overlapping or repeated intervals",
                    membership_anomalies.len()
                ));
            }
            warnings
        },
        disclosures: vec![
            "IceCast baseline probabilities are model estimates, not betting odds or guarantees.".to_owned(),
            "The baseline uses roster/depth strength, home ice, rest, congestion, travel, and timezone context; personnel events and trades affect games only when the season scenario layer supplies them.".to_owned(),
            "Every factor is signed from the home-team perspective and reconciles to overall home-win probability within rounding tolerance.".to_owned(),
            "Final scores are joined only after each probability is computed; they are evaluation labels and never forecast inputs.".to_owned(),
            if replay.is_some() {
                "Rolling replay strength starts from a neutral regressed prior and uses only standings points and goal differential from games on earlier calendar dates; same-day results are withheld from one another.".to_owned()
            } else {
                "Frozen forecasts use one roster/depth strength snapshot for the scheduled season.".to_owned()
            },
            "The Wire membership intervals are an NHL active-roster evidence ledger, not a complete historical roster: recalls and waiver claims open sourced intervals, assignments close them, and removal-only intervals have an unknown start labeled implied_preexisting. Trades, acquisitions, and releases do not independently prove active-roster status; an exact paired trade transfers a lineup player only when the source is already known active. Event-date changes become forecast evidence only on later calendar dates.".to_owned(),
            "Repeated recall, assignment, IR-placement, and activation rows remain visible as sourced evidence but cannot increment the same player's replay state twice.".to_owned(),
            "Verified opening-strength rows retain exact player membership and position groups. Only personnel events strictly after the snapshot date may recompute the active 12-forward, six-defense, and two-goalie lineup; each game exposes the resulting personnel strength delta.".to_owned(),
            "Post-snapshot newcomers enter lineup strength only when stable identity resolves both a completed prior-season value and position group; otherwise the move remains roster evidence with neutral modeled impact.".to_owned(),
            "Prior-season player values change rolling opening strength only for teams The Crease verifies from pre-opening roster evidence; uncovered teams retain the neutral prior, and partial coverage remains ineligible for promotion.".to_owned(),
        ],
    })
}

#[derive(Debug, Clone)]
enum MembershipState {
    Open(usize),
    Absent(String),
}

fn build_membership_intervals(
    events: &[TeamForecastPersonnelEvidenceInput],
    schedule_end: NaiveDate,
) -> (
    Vec<TeamGameMembershipIntervalRow>,
    Vec<TeamGameMembershipAnomalyRow>,
) {
    let mut intervals: Vec<TeamGameMembershipIntervalRow> = Vec::new();
    let mut states: BTreeMap<(String, u32), MembershipState> = BTreeMap::new();
    let mut anomalies = Vec::new();

    for event in events.iter().filter(|event| event.date <= schedule_end) {
        for player in &event.resolved_players {
            let key = (event.team.clone(), player.player_id);
            match player.membership_delta {
                1 => match states.get(&key).cloned() {
                    Some(MembershipState::Open(index)) => {
                        anomalies.push(TeamGameMembershipAnomalyRow {
                            player_id: player.player_id,
                            full_name: player.full_name.clone(),
                            team: event.team.clone(),
                            event_id: event.event_id.clone(),
                            event_date: event.date,
                            prior_event_id: intervals[index]
                                .start_event_id
                                .clone()
                                .unwrap_or_default(),
                            reason: "addition_while_open".to_owned(),
                        });
                    }
                    _ => {
                        let index = intervals.len();
                        intervals.push(TeamGameMembershipIntervalRow {
                            player_id: player.player_id,
                            full_name: player.full_name.clone(),
                            team: event.team.clone(),
                            start_event_date: Some(event.date),
                            start_event_id: Some(event.event_id.clone()),
                            end_event_date: None,
                            end_event_id: None,
                            confidence: "sourced".to_owned(),
                            opening_basis: "dated_addition_event".to_owned(),
                            prior_season: player.prior_season,
                            prior_games_played: player.prior_games_played,
                            prior_value: player.prior_value,
                        });
                        states.insert(key, MembershipState::Open(index));
                    }
                },
                -1 => match states.get(&key).cloned() {
                    Some(MembershipState::Open(index)) => {
                        intervals[index].end_event_date = Some(event.date);
                        intervals[index].end_event_id = Some(event.event_id.clone());
                        states.insert(key, MembershipState::Absent(event.event_id.clone()));
                    }
                    Some(MembershipState::Absent(prior_event_id)) => {
                        anomalies.push(TeamGameMembershipAnomalyRow {
                            player_id: player.player_id,
                            full_name: player.full_name.clone(),
                            team: event.team.clone(),
                            event_id: event.event_id.clone(),
                            event_date: event.date,
                            prior_event_id,
                            reason: "removal_while_absent".to_owned(),
                        });
                    }
                    None => {
                        intervals.push(TeamGameMembershipIntervalRow {
                            player_id: player.player_id,
                            full_name: player.full_name.clone(),
                            team: event.team.clone(),
                            start_event_date: None,
                            start_event_id: None,
                            end_event_date: Some(event.date),
                            end_event_id: Some(event.event_id.clone()),
                            confidence: "implied_preexisting".to_owned(),
                            opening_basis: "removal_implies_prior_membership".to_owned(),
                            prior_season: player.prior_season,
                            prior_games_played: player.prior_games_played,
                            prior_value: player.prior_value,
                        });
                        states.insert(key, MembershipState::Absent(event.event_id.clone()));
                    }
                },
                _ => {}
            }
        }
    }

    (intervals, anomalies)
}

fn build_paired_trades(
    events: &[TeamForecastPersonnelEvidenceInput],
    schedule_end: NaiveDate,
) -> (Vec<TeamGamePairedTradeRow>, BTreeMap<(String, u32), usize>) {
    let mut candidates = BTreeMap::<
        (NaiveDate, u32),
        Vec<(String, String, String, TeamForecastPersonnelPlayerInput)>,
    >::new();
    for event in events.iter().filter(|event| event.date <= schedule_end) {
        for player in &event.resolved_players {
            if matches!(player.action.as_str(), "acquired" | "traded_away") {
                candidates
                    .entry((event.date, player.player_id))
                    .or_default()
                    .push((
                        event.event_id.clone(),
                        event.team.clone(),
                        player.action.clone(),
                        player.clone(),
                    ));
            }
        }
    }
    let mut rows = Vec::new();
    let mut by_event_player = BTreeMap::new();
    for ((date, player_id), candidates) in candidates {
        let acquired = candidates
            .iter()
            .filter(|(_, _, action, _)| action == "acquired")
            .collect::<Vec<_>>();
        let traded = candidates
            .iter()
            .filter(|(_, _, action, _)| action == "traded_away")
            .collect::<Vec<_>>();
        if acquired.len() != 1 || traded.len() != 1 || acquired[0].1 == traded[0].1 {
            continue;
        }
        let player = &acquired[0].3;
        let transfer_id = format!("{}:{}:{}:{}", date, player_id, traded[0].1, acquired[0].1);
        let mut source_event_ids = vec![traded[0].0.clone(), acquired[0].0.clone()];
        source_event_ids.sort();
        let index = rows.len();
        rows.push(TeamGamePairedTradeRow {
            transfer_id,
            date,
            player_id,
            full_name: player.full_name.clone(),
            from_team: traded[0].1.clone(),
            to_team: acquired[0].1.clone(),
            source_event_ids: source_event_ids.clone(),
            prior_position_group: player.prior_position_group.clone(),
            prior_value: player.prior_value,
            active_lineup_applied: false,
            disposition: "organizational_pair_only".to_owned(),
        });
        for event_id in source_event_ids {
            by_event_player.insert((event_id, player_id), index);
        }
    }
    (rows, by_event_player)
}

#[allow(clippy::too_many_arguments)]
fn apply_paired_trades_for_event(
    states: &mut BTreeMap<String, TeamReplayState>,
    roster_players: &mut BTreeMap<(String, u32), TeamGameOpeningPlayerRow>,
    membership: &mut BTreeMap<(String, u32), bool>,
    ir_players: &mut BTreeSet<(String, u32)>,
    paired_trades: &mut [TeamGamePairedTradeRow],
    by_event_player: &BTreeMap<(String, u32), usize>,
    applied: &mut BTreeSet<String>,
    event: &TeamForecastPersonnelEvidenceInput,
) {
    for player in &event.resolved_players {
        let Some(index) = by_event_player
            .get(&(event.event_id.clone(), player.player_id))
            .copied()
        else {
            continue;
        };
        let trade = &mut paired_trades[index];
        if !applied.insert(trade.transfer_id.clone()) {
            continue;
        }
        let from_key = (trade.from_team.clone(), trade.player_id);
        let to_key = (trade.to_team.clone(), trade.player_id);
        if membership.get(&from_key) != Some(&true) {
            trade.disposition = "source_not_known_active".to_owned();
            continue;
        }
        membership.insert(from_key.clone(), false);
        membership.insert(to_key.clone(), true);
        states
            .entry(trade.from_team.clone())
            .or_default()
            .known_roster_removals += 1;
        states
            .entry(trade.to_team.clone())
            .or_default()
            .known_roster_additions += 1;

        let roster_player = roster_players.get(&from_key).cloned().or_else(|| {
            match (&trade.prior_position_group, trade.prior_value) {
                (Some(position_group), Some(prior_value))
                    if matches!(position_group.as_str(), "forward" | "defense" | "goalie") =>
                {
                    Some(TeamGameOpeningPlayerRow {
                        player_id: trade.player_id,
                        full_name: trade.full_name.clone(),
                        position_group: position_group.clone(),
                        prior_value: Some(prior_value),
                        modeled_value: prior_value,
                        selected_at_opening: false,
                    })
                }
                _ => None,
            }
        });
        if let Some(roster_player) = roster_player {
            roster_players.insert(to_key.clone(), roster_player);
        }
        if ir_players.remove(&from_key) {
            let from_state = states.entry(trade.from_team.clone()).or_default();
            from_state.active_ir_signals = from_state.active_ir_signals.saturating_sub(1);
            ir_players.insert(to_key);
            states
                .entry(trade.to_team.clone())
                .or_default()
                .active_ir_signals += 1;
        }
        trade.active_lineup_applied = true;
        trade.disposition = "active_lineup_transferred".to_owned();
    }
}

#[derive(Debug)]
struct GradedResult {
    away_score: u8,
    home_score: u8,
    winner: String,
    ending: Option<String>,
}

fn grade_result(game: &TeamForecastGameInput) -> Option<GradedResult> {
    if !game.final_result {
        return None;
    }
    let away_score = game.away_score?;
    let home_score = game.home_score?;
    if away_score == home_score {
        return None;
    }
    Some(GradedResult {
        away_score,
        home_score,
        winner: if home_score > away_score {
            game.home_team.clone()
        } else {
            game.away_team.clone()
        },
        ending: game.last_period.clone(),
    })
}

fn score_binary_log_loss(home_win_probability: f64, home_won: bool) -> f64 {
    let observed_probability = if home_won {
        home_win_probability
    } else {
        1.0 - home_win_probability
    };
    -observed_probability.clamp(1e-15, 1.0).ln()
}

fn home_only_win_probability(parameters: &TeamForecastParameters) -> f64 {
    home_win_probability_from_edge(parameters.home_edge, parameters)
}

fn home_win_probability_from_edge(edge: f64, parameters: &TeamForecastParameters) -> f64 {
    (0.5 + edge.clamp(-0.24, 0.24) * (1.0 - parameters.overtime_probability * 0.5))
        .clamp(0.01, 0.99)
}

fn elo_home_win_probability(home_rating: f64, away_rating: f64) -> f64 {
    const HOME_ADVANTAGE_ELO: f64 = 22.0;
    1.0 / (1.0 + 10.0_f64.powf((away_rating - home_rating - HOME_ADVANTAGE_ELO) / 400.0))
}

fn apply_pending_elo_results(
    ratings: &mut BTreeMap<String, f64>,
    pending: &mut Vec<PendingEloResult>,
) {
    const K_FACTOR: f64 = 20.0;
    for result in pending.drain(..) {
        let delta = K_FACTOR * (result.home_score - result.expected_home_score);
        *ratings.entry(result.home_team).or_insert(1500.0) += delta;
        *ratings.entry(result.away_team).or_insert(1500.0) -= delta;
    }
}

fn score_multiclass_log_loss(
    home_regulation_probability: f64,
    away_regulation_probability: f64,
    overtime_probability: f64,
    result: &GradedResult,
    game: &TeamForecastGameInput,
) -> Option<f64> {
    let observed_probability = match result.ending.as_deref()? {
        "OT" | "SO" => overtime_probability,
        "REG" if result.winner == game.home_team => home_regulation_probability,
        "REG" if result.winner == game.away_team => away_regulation_probability,
        _ => return None,
    };
    Some(-observed_probability.clamp(1e-15, 1.0).ln())
}

fn replay_roster_strength(
    opening: &TeamGameOpeningStrengthRow,
    roster_players: &BTreeMap<(String, u32), TeamGameOpeningPlayerRow>,
    membership: &BTreeMap<(String, u32), bool>,
    ir_players: &BTreeSet<(String, u32)>,
) -> f64 {
    let active = roster_players
        .iter()
        .filter(|((team, _), _)| team == &opening.team)
        .filter(|(key, _)| membership.get(*key) == Some(&true) && !ir_players.contains(*key))
        .map(|(_, player)| player)
        .collect::<Vec<_>>();
    let coverage = if active.is_empty() {
        0.0
    } else {
        active
            .iter()
            .filter(|player| player.prior_value.is_some())
            .count() as f64
            / active.len() as f64
    };
    let group_score = |group: &str, slots: usize| {
        let mut values = active
            .iter()
            .filter(|player| player.position_group == group)
            .map(|player| player.modeled_value)
            .collect::<Vec<_>>();
        values.sort_by(|a, b| b.total_cmp(a));
        values.truncate(slots);
        values.resize(slots, 50.0);
        values.iter().sum::<f64>() / slots as f64
    };
    let raw = group_score("forward", 12) * 0.55
        + group_score("defense", 6) * 0.30
        + group_score("goalie", 2) * 0.15;
    (50.0 + (raw - 50.0) * coverage + opening.cohort_normalization_delta).clamp(0.0, 100.0)
}

fn rolling_replay_strength(
    state: Option<&TeamReplayState>,
    config: &TeamForecastReplayConfig,
    opening_strength: Option<f64>,
) -> f64 {
    let prior_strength = opening_strength.unwrap_or(config.prior_strength);
    let Some(state) = state.filter(|state| state.games > 0) else {
        return prior_strength;
    };
    let games = state.games as f64;
    let points_percentage = state.standings_points as f64 / (games * 2.0);
    let record_strength = 50.0 + (points_percentage - 0.5) * 40.0;
    let goal_differential_per_game = (state.goals_for as f64 - state.goals_against as f64) / games;
    let goal_strength = 50.0 + (goal_differential_per_game * 4.0).clamp(-10.0, 10.0);
    let observed_strength = record_strength * 0.75 + goal_strength * 0.25;
    let credibility = games / (games + config.prior_games);
    (prior_strength * (1.0 - credibility) + observed_strength * credibility).clamp(0.0, 100.0)
}

fn rolling_standings_strength(
    state: Option<&TeamReplayState>,
    config: &TeamForecastReplayConfig,
) -> f64 {
    let Some(state) = state.filter(|state| state.games > 0) else {
        return config.prior_strength;
    };
    let games = state.games as f64;
    let points_percentage = state.standings_points as f64 / (games * 2.0);
    let observed_strength = 50.0 + (points_percentage - 0.5) * 40.0;
    let credibility = games / (games + config.prior_games);
    (config.prior_strength * (1.0 - credibility) + observed_strength * credibility)
        .clamp(0.0, 100.0)
}

fn apply_pending_replay_results(
    states: &mut BTreeMap<String, TeamReplayState>,
    pending: &mut Vec<PendingReplayResult>,
) {
    for result in pending.drain(..) {
        let away_won = result.away_score > result.home_score;
        let home_won = result.home_score > result.away_score;
        let away = states.entry(result.away_team).or_default();
        away.games += 1;
        away.goals_for += usize::from(result.away_score);
        away.goals_against += usize::from(result.home_score);
        away.standings_points += if away_won {
            2
        } else if result.overtime {
            1
        } else {
            0
        };
        let home = states.entry(result.home_team).or_default();
        home.games += 1;
        home.goals_for += usize::from(result.home_score);
        home.goals_against += usize::from(result.away_score);
        home.standings_points += if home_won {
            2
        } else if result.overtime {
            1
        } else {
            0
        };
    }
}

fn apply_personnel_evidence(
    states: &mut BTreeMap<String, TeamReplayState>,
    roster_players: &mut BTreeMap<(String, u32), TeamGameOpeningPlayerRow>,
    membership: &mut BTreeMap<(String, u32), bool>,
    ir_players: &mut BTreeSet<(String, u32)>,
    event: &TeamForecastPersonnelEvidenceInput,
) {
    let state = states.entry(event.team.clone()).or_default();
    state.known_personnel_events += 1;
    let mut resolved_availability = false;
    for player in &event.resolved_players {
        let key = (event.team.clone(), player.player_id);
        match player.action.as_str() {
            "ir_placed" => {
                resolved_availability = true;
                if ir_players.insert(key.clone()) {
                    state.active_ir_signals += 1;
                }
            }
            "activated" => {
                resolved_availability = true;
                if ir_players.remove(&key) {
                    state.active_ir_signals = state.active_ir_signals.saturating_sub(1);
                }
            }
            _ => {}
        }
        match player.membership_delta {
            1 if membership.get(&key) != Some(&true) => {
                if !roster_players.contains_key(&key) {
                    if let (Some(position_group), Some(prior_value)) =
                        (&player.prior_position_group, player.prior_value)
                    {
                        if matches!(position_group.as_str(), "forward" | "defense" | "goalie") {
                            roster_players.insert(
                                key.clone(),
                                TeamGameOpeningPlayerRow {
                                    player_id: player.player_id,
                                    full_name: player.full_name.clone(),
                                    position_group: position_group.clone(),
                                    prior_value: Some(prior_value),
                                    modeled_value: prior_value,
                                    selected_at_opening: false,
                                },
                            );
                        }
                    }
                }
                membership.insert(key, true);
                state.known_roster_additions += 1;
            }
            -1 if membership.get(&key) != Some(&false) => {
                membership.insert(key, false);
                state.known_roster_removals += 1;
            }
            _ => {}
        }
    }
    if !resolved_availability {
        match event.availability_delta {
            1 => state.active_ir_signals += 1,
            -1 => state.active_ir_signals = state.active_ir_signals.saturating_sub(1),
            _ => {}
        }
    }
}

fn fit_logistic_calibration(rows: &[&TeamGameForecastRow]) -> Option<(f64, f64, f64, f64)> {
    let values = rows
        .iter()
        .map(|row| {
            (
                row.home_overall_win_probability,
                row.actual_winner.as_deref() == Some(row.home_team.as_str()),
            )
        })
        .collect::<Vec<_>>();
    fit_logistic_calibration_values(&values)
}

fn apply_logistic_calibration(probability: f64, intercept: f64, slope: f64) -> f64 {
    let probability = probability.clamp(1e-6, 1.0 - 1e-6);
    let log_odds = (probability / (1.0 - probability)).ln();
    let linear = (intercept + slope * log_odds).clamp(-30.0, 30.0);
    1.0 / (1.0 + (-linear).exp())
}

fn fit_logistic_calibration_values(values: &[(f64, bool)]) -> Option<(f64, f64, f64, f64)> {
    if values.len() < 20
        || values.iter().all(|(_, outcome)| *outcome)
        || values.iter().all(|(_, outcome)| !*outcome)
    {
        return None;
    }
    let mut intercept = 0.0_f64;
    let mut slope = 1.0_f64;
    for _ in 0..100 {
        let mut gradient_intercept = 0.0;
        let mut gradient_slope = 0.0;
        let mut information_intercept = 1e-9;
        let mut information_cross = 0.0;
        let mut information_slope = 1e-9;
        for (probability, outcome) in values {
            let probability = probability.clamp(1e-6, 1.0 - 1e-6);
            let log_odds = (probability / (1.0 - probability)).ln();
            let fitted = apply_logistic_calibration(probability, intercept, slope);
            let observed = if *outcome { 1.0 } else { 0.0 };
            let residual = observed - fitted;
            let weight = fitted * (1.0 - fitted);
            gradient_intercept += residual;
            gradient_slope += residual * log_odds;
            information_intercept += weight;
            information_cross += weight * log_odds;
            information_slope += weight * log_odds * log_odds;
        }
        let determinant =
            information_intercept * information_slope - information_cross * information_cross;
        if determinant.abs() < 1e-12 {
            return None;
        }
        let intercept_step = (gradient_intercept * information_slope
            - gradient_slope * information_cross)
            / determinant;
        let slope_step = (gradient_slope * information_intercept
            - gradient_intercept * information_cross)
            / determinant;
        if !intercept_step.is_finite() || !slope_step.is_finite() {
            return None;
        }
        intercept += intercept_step.clamp(-5.0, 5.0);
        slope += slope_step.clamp(-5.0, 5.0);
        if intercept_step.abs().max(slope_step.abs()) < 1e-10 {
            break;
        }
    }
    if !intercept.is_finite() || !slope.is_finite() {
        return None;
    }
    let mut information_intercept = 1e-9;
    let mut information_cross = 0.0;
    let mut information_slope = 1e-9;
    for (probability, _) in values {
        let probability = probability.clamp(1e-6, 1.0 - 1e-6);
        let log_odds = (probability / (1.0 - probability)).ln();
        let fitted = apply_logistic_calibration(probability, intercept, slope);
        let weight = fitted * (1.0 - fitted);
        information_intercept += weight;
        information_cross += weight * log_odds;
        information_slope += weight * log_odds * log_odds;
    }
    let determinant =
        information_intercept * information_slope - information_cross * information_cross;
    if determinant <= 1e-12 {
        return None;
    }
    let intercept_standard_error = (information_slope / determinant).sqrt();
    let slope_standard_error = (information_intercept / determinant).sqrt();
    (intercept_standard_error.is_finite() && slope_standard_error.is_finite()).then_some((
        intercept,
        slope,
        intercept_standard_error,
        slope_standard_error,
    ))
}

fn build_accuracy_summary(
    rows: &[TeamGameForecastRow],
    chronological_baselines: bool,
) -> Option<TeamGameForecastAccuracySummary> {
    let final_rows = rows
        .iter()
        .filter(|row| row.pick_correct.is_some())
        .collect::<Vec<_>>();
    if final_rows.is_empty() {
        return None;
    }
    let correct_picks = final_rows
        .iter()
        .filter(|row| row.pick_correct == Some(true))
        .count();
    let mean_metric = |values: Vec<f64>| {
        (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
    };
    let mut by_confidence = Vec::new();
    for segment in ["strong", "lean", "toss_up"] {
        let group = final_rows
            .iter()
            .copied()
            .filter(|row| row.confidence == segment)
            .collect::<Vec<_>>();
        if group.is_empty() {
            continue;
        }
        let group_correct = group
            .iter()
            .filter(|row| row.pick_correct == Some(true))
            .count();
        by_confidence.push(TeamGameForecastAccuracyRow {
            segment: segment.to_owned(),
            games: group.len(),
            correct_picks: group_correct,
            pick_accuracy: group_correct as f64 / group.len() as f64,
            mean_favorite_probability: group
                .iter()
                .map(|row| {
                    row.home_overall_win_probability
                        .max(row.away_overall_win_probability)
                })
                .sum::<f64>()
                / group.len() as f64,
            brier_score: group.iter().filter_map(|row| row.brier_score).sum::<f64>()
                / group.len() as f64,
            binary_log_loss: group
                .iter()
                .filter_map(|row| row.binary_log_loss)
                .sum::<f64>()
                / group.len() as f64,
            multiclass_log_loss: mean_metric(
                group
                    .iter()
                    .filter_map(|row| row.multiclass_log_loss)
                    .collect(),
            ),
        });
    }
    let mut calibration_bins = Vec::new();
    for bin in 0..10 {
        let lower = bin as f64 / 10.0;
        let upper = (bin + 1) as f64 / 10.0;
        let group = final_rows
            .iter()
            .copied()
            .filter(|row| {
                let probability = row.home_overall_win_probability;
                probability >= lower && (probability < upper || (bin == 9 && probability <= 1.0))
            })
            .collect::<Vec<_>>();
        if group.is_empty() {
            continue;
        }
        let mean_probability = group
            .iter()
            .map(|row| row.home_overall_win_probability)
            .sum::<f64>()
            / group.len() as f64;
        let observed_rate = group
            .iter()
            .filter(|row| row.actual_winner.as_deref() == Some(row.home_team.as_str()))
            .count() as f64
            / group.len() as f64;
        calibration_bins.push(TeamGameForecastCalibrationRow {
            segment: format!("{lower:.1}-{upper:.1}"),
            games: group.len(),
            mean_home_win_probability: mean_probability,
            observed_home_win_rate: observed_rate,
            absolute_calibration_error: (mean_probability - observed_rate).abs(),
        });
    }
    let brier_score = final_rows
        .iter()
        .filter_map(|row| row.brier_score)
        .sum::<f64>()
        / final_rows.len() as f64;
    let binary_log_loss = final_rows
        .iter()
        .filter_map(|row| row.binary_log_loss)
        .sum::<f64>()
        / final_rows.len() as f64;
    let multiclass_log_loss = mean_metric(
        final_rows
            .iter()
            .filter_map(|row| row.multiclass_log_loss)
            .collect(),
    );
    let expected_calibration_error = calibration_bins
        .iter()
        .map(|bin| bin.absolute_calibration_error * bin.games as f64)
        .sum::<f64>()
        / final_rows.len() as f64;
    let calibration_fit = fit_logistic_calibration(&final_rows);
    let calibration_intercept = calibration_fit.map(|fit| fit.0);
    let calibration_slope = calibration_fit.map(|fit| fit.1);
    let calibration_intercept_standard_error = calibration_fit.map(|fit| fit.2);
    let calibration_slope_standard_error = calibration_fit.map(|fit| fit.3);
    let calibration_intercept_ci95_lower = calibration_fit.map(|fit| fit.0 - 1.96 * fit.2);
    let calibration_intercept_ci95_upper = calibration_fit.map(|fit| fit.0 + 1.96 * fit.2);
    let calibration_slope_ci95_lower = calibration_fit.map(|fit| fit.1 - 1.96 * fit.3);
    let calibration_slope_ci95_upper = calibration_fit.map(|fit| fit.1 + 1.96 * fit.3);
    let mut baselines = vec![
        build_binary_baseline(
            "home_only",
            &final_rows,
            brier_score,
            binary_log_loss,
            |row| row.home_only_home_win_probability,
        ),
        build_binary_baseline(
            if chronological_baselines {
                "chronological_elo"
            } else {
                "frozen_equal_rating_elo"
            },
            &final_rows,
            brier_score,
            binary_log_loss,
            |row| row.elo_home_win_probability,
        ),
    ];
    if chronological_baselines {
        baselines.push(build_binary_baseline(
            "rolling_standings",
            &final_rows,
            brier_score,
            binary_log_loss,
            |row| row.standings_home_win_probability.unwrap_or(0.5),
        ));
    }
    let ablations = build_factor_ablations(&final_rows, brier_score, binary_log_loss);
    let elo_blend_sweep = if chronological_baselines {
        build_elo_blend_sweep(&final_rows, brier_score, binary_log_loss)
    } else {
        Vec::new()
    };
    let best_elo_blend_by_brier = elo_blend_sweep
        .iter()
        .min_by(|a, b| {
            a.brier_score
                .total_cmp(&b.brier_score)
                .then_with(|| a.elo_weight.total_cmp(&b.elo_weight))
        })
        .cloned();
    Some(TeamGameForecastAccuracySummary {
        final_games: final_rows.len(),
        pending_games: rows.len() - final_rows.len(),
        correct_picks,
        pick_accuracy: correct_picks as f64 / final_rows.len() as f64,
        brier_score,
        binary_log_loss,
        multiclass_log_loss,
        brier_skill_vs_coinflip: 0.25 - brier_score,
        binary_log_loss_skill_vs_coinflip: std::f64::consts::LN_2 - binary_log_loss,
        multiclass_log_loss_skill_vs_uniform: multiclass_log_loss.map(|loss| 3.0_f64.ln() - loss),
        expected_calibration_error,
        calibration_intercept,
        calibration_slope,
        calibration_intercept_standard_error,
        calibration_slope_standard_error,
        calibration_intercept_ci95_lower,
        calibration_intercept_ci95_upper,
        calibration_slope_ci95_lower,
        calibration_slope_ci95_upper,
        by_confidence,
        calibration_bins,
        baselines,
        ablations,
        elo_blend_sweep,
        best_elo_blend_by_brier,
    })
}

fn build_elo_blend_sweep(
    rows: &[&TeamGameForecastRow],
    model_brier: f64,
    model_log_loss: f64,
) -> Vec<TeamGameForecastBlendRow> {
    (0..=10)
        .map(|step| {
            let elo_weight = step as f64 / 10.0;
            let mut correct = 0;
            let mut brier = 0.0;
            let mut log_loss = 0.0;
            for row in rows {
                let home_probability = row.home_overall_win_probability * (1.0 - elo_weight)
                    + row.elo_home_win_probability * elo_weight;
                let home_won = row.actual_winner.as_deref() == Some(row.home_team.as_str());
                correct += usize::from((home_probability >= 0.5) == home_won);
                brier += (home_probability - f64::from(home_won)).powi(2);
                log_loss += score_binary_log_loss(home_probability, home_won);
            }
            brier /= rows.len() as f64;
            log_loss /= rows.len() as f64;
            TeamGameForecastBlendRow {
                elo_weight,
                games: rows.len(),
                pick_accuracy: correct as f64 / rows.len() as f64,
                brier_score: brier,
                binary_log_loss: log_loss,
                brier_improvement_vs_model: model_brier - brier,
                log_loss_improvement_vs_model: model_log_loss - log_loss,
            }
        })
        .collect()
}

pub fn build_team_game_forecast_validation(
    mut inputs: Vec<TeamGameForecastValidationInput>,
) -> Result<TeamGameForecastValidationView, String> {
    if inputs.len() < 3 {
        return Err("IceCast cross-validation requires at least three seasons".to_owned());
    }
    inputs.sort_by_key(|input| input.season);
    if inputs
        .windows(2)
        .any(|pair| pair[0].season == pair[1].season)
    {
        return Err("IceCast cross-validation seasons must be unique".to_owned());
    }
    let reference_weights = inputs[0]
        .elo_blend_sweep
        .iter()
        .map(|row| row.elo_weight)
        .collect::<Vec<_>>();
    if reference_weights.len() < 2
        || reference_weights.first() != Some(&0.0)
        || reference_weights.last() != Some(&1.0)
        || inputs.iter().any(|input| {
            input.games == 0
                || input.calibration_observations.len() != input.games
                || input.calibration_observations.iter().any(|observation| {
                    !observation.home_win_probability.is_finite()
                        || !(0.0..=1.0).contains(&observation.home_win_probability)
                })
                || input.elo_blend_sweep.len() != reference_weights.len()
                || input
                    .elo_blend_sweep
                    .iter()
                    .zip(&reference_weights)
                    .any(|(row, weight)| {
                        row.games != input.games
                            || (row.elo_weight - weight).abs() > 1e-12
                            || !row.pick_accuracy.is_finite()
                            || !row.brier_score.is_finite()
                            || !row.binary_log_loss.is_finite()
                    })
        })
    {
        return Err(
            "IceCast cross-validation requires compatible finite blend grids and one valid calibration observation per graded game".to_owned(),
        );
    }

    let pooled_sweep = pool_validation_sweeps(&inputs);
    let pooled_best_by_brier = best_blend(&pooled_sweep).clone();
    let mut holdouts = Vec::with_capacity(inputs.len());
    for holdout in &inputs {
        let training = inputs
            .iter()
            .filter(|input| input.season != holdout.season)
            .cloned()
            .collect::<Vec<_>>();
        let training_sweep = pool_validation_sweeps(&training);
        let selected = best_blend(&training_sweep);
        let test = holdout
            .elo_blend_sweep
            .iter()
            .find(|row| (row.elo_weight - selected.elo_weight).abs() <= 1e-12)
            .expect("validated compatible grid");
        let model = &holdout.elo_blend_sweep[0];
        let elo = holdout.elo_blend_sweep.last().unwrap();
        holdouts.push(TeamGameForecastHoldoutRow {
            holdout_season: holdout.season,
            training_seasons: training.iter().map(|input| input.season).collect(),
            selected_elo_weight: selected.elo_weight,
            games: holdout.games,
            pick_accuracy: test.pick_accuracy,
            brier_score: test.brier_score,
            binary_log_loss: test.binary_log_loss,
            brier_improvement_vs_model: model.brier_score - test.brier_score,
            brier_improvement_vs_pure_elo: elo.brier_score - test.brier_score,
            log_loss_improvement_vs_model: model.binary_log_loss - test.binary_log_loss,
            log_loss_improvement_vs_pure_elo: elo.binary_log_loss - test.binary_log_loss,
        });
    }
    let mut calibration_holdouts = Vec::with_capacity(inputs.len().saturating_sub(1));
    let mut brier_improvement_samples = Vec::new();
    let mut binary_log_loss_improvement_samples = Vec::new();
    for index in 1..inputs.len() {
        let training = &inputs[..index];
        let training_values = training
            .iter()
            .flat_map(|input| {
                input
                    .calibration_observations
                    .iter()
                    .map(|observation| (observation.home_win_probability, observation.home_won))
            })
            .collect::<Vec<_>>();
        let (intercept, slope, _, _) = fit_logistic_calibration_values(&training_values)
            .ok_or_else(|| {
                format!(
                    "IceCast chronological calibration cannot fit seasons before {}",
                    inputs[index].season
                )
            })?;
        let holdout = &inputs[index];
        let mut uncalibrated_brier = 0.0;
        let mut recalibrated_brier = 0.0;
        let mut uncalibrated_log_loss = 0.0;
        let mut recalibrated_log_loss = 0.0;
        for observation in &holdout.calibration_observations {
            let observed = if observation.home_won { 1.0 } else { 0.0 };
            let recalibrated =
                apply_logistic_calibration(observation.home_win_probability, intercept, slope);
            let uncalibrated_game_brier = (observation.home_win_probability - observed).powi(2);
            let recalibrated_game_brier = (recalibrated - observed).powi(2);
            let uncalibrated_game_log_loss =
                score_binary_log_loss(observation.home_win_probability, observation.home_won);
            let recalibrated_game_log_loss =
                score_binary_log_loss(recalibrated, observation.home_won);
            uncalibrated_brier += uncalibrated_game_brier;
            recalibrated_brier += recalibrated_game_brier;
            uncalibrated_log_loss += uncalibrated_game_log_loss;
            recalibrated_log_loss += recalibrated_game_log_loss;
            brier_improvement_samples.push(uncalibrated_game_brier - recalibrated_game_brier);
            binary_log_loss_improvement_samples
                .push(uncalibrated_game_log_loss - recalibrated_game_log_loss);
        }
        let games = holdout.calibration_observations.len();
        uncalibrated_brier /= games as f64;
        recalibrated_brier /= games as f64;
        uncalibrated_log_loss /= games as f64;
        recalibrated_log_loss /= games as f64;
        calibration_holdouts.push(TeamGameForecastCalibrationHoldoutRow {
            holdout_season: holdout.season,
            training_seasons: training.iter().map(|input| input.season).collect(),
            training_games: training_values.len(),
            games,
            fitted_intercept: intercept,
            fitted_slope: slope,
            uncalibrated_brier_score: uncalibrated_brier,
            recalibrated_brier_score: recalibrated_brier,
            brier_improvement: uncalibrated_brier - recalibrated_brier,
            uncalibrated_binary_log_loss: uncalibrated_log_loss,
            recalibrated_binary_log_loss: recalibrated_log_loss,
            binary_log_loss_improvement: uncalibrated_log_loss - recalibrated_log_loss,
        });
    }
    let calibration_games = calibration_holdouts
        .iter()
        .map(|row| row.games)
        .sum::<usize>();
    let weighted_calibration_metric =
        |metric: fn(&TeamGameForecastCalibrationHoldoutRow) -> f64| {
            calibration_holdouts
                .iter()
                .map(|row| metric(row) * row.games as f64)
                .sum::<f64>()
                / calibration_games as f64
        };
    let uncalibrated_brier_score = weighted_calibration_metric(|row| row.uncalibrated_brier_score);
    let recalibrated_brier_score = weighted_calibration_metric(|row| row.recalibrated_brier_score);
    let uncalibrated_binary_log_loss =
        weighted_calibration_metric(|row| row.uncalibrated_binary_log_loss);
    let recalibrated_binary_log_loss =
        weighted_calibration_metric(|row| row.recalibrated_binary_log_loss);
    let (_, brier_improvement_standard_error) = mean_and_standard_error(&brier_improvement_samples);
    let (_, binary_log_loss_improvement_standard_error) =
        mean_and_standard_error(&binary_log_loss_improvement_samples);
    let brier_improvement = uncalibrated_brier_score - recalibrated_brier_score;
    let binary_log_loss_improvement = uncalibrated_binary_log_loss - recalibrated_binary_log_loss;
    let season_clustered_brier_improvement_standard_error = delete_one_cluster_standard_error(
        &calibration_holdouts
            .iter()
            .map(|row| (row.games, row.brier_improvement))
            .collect::<Vec<_>>(),
    );
    let season_clustered_binary_log_loss_improvement_standard_error =
        delete_one_cluster_standard_error(
            &calibration_holdouts
                .iter()
                .map(|row| (row.games, row.binary_log_loss_improvement))
                .collect::<Vec<_>>(),
        );
    let season_clustered_brier_improvement_ci95_lower =
        brier_improvement - 1.96 * season_clustered_brier_improvement_standard_error;
    let season_clustered_brier_improvement_ci95_upper =
        brier_improvement + 1.96 * season_clustered_brier_improvement_standard_error;
    let season_clustered_binary_log_loss_improvement_ci95_lower = binary_log_loss_improvement
        - 1.96 * season_clustered_binary_log_loss_improvement_standard_error;
    let season_clustered_binary_log_loss_improvement_ci95_upper = binary_log_loss_improvement
        + 1.96 * season_clustered_binary_log_loss_improvement_standard_error;
    let calibration_summary = TeamGameForecastCalibrationSummary {
        holdout_seasons: calibration_holdouts.len(),
        games: calibration_games,
        holdouts_improved_brier: calibration_holdouts
            .iter()
            .filter(|row| row.brier_improvement > 0.0)
            .count(),
        holdouts_improved_binary_log_loss: calibration_holdouts
            .iter()
            .filter(|row| row.binary_log_loss_improvement > 0.0)
            .count(),
        uncalibrated_brier_score,
        recalibrated_brier_score,
        brier_improvement,
        brier_improvement_standard_error,
        brier_improvement_ci95_lower: brier_improvement - 1.96 * brier_improvement_standard_error,
        brier_improvement_ci95_upper: brier_improvement + 1.96 * brier_improvement_standard_error,
        season_clustered_brier_improvement_standard_error,
        season_clustered_brier_improvement_ci95_lower,
        season_clustered_brier_improvement_ci95_upper,
        season_clustered_brier_evidence: calibration_evidence_label(
            calibration_holdouts.len(),
            season_clustered_brier_improvement_ci95_lower,
            season_clustered_brier_improvement_ci95_upper,
        )
        .to_owned(),
        uncalibrated_binary_log_loss,
        recalibrated_binary_log_loss,
        binary_log_loss_improvement,
        binary_log_loss_improvement_standard_error,
        binary_log_loss_improvement_ci95_lower: binary_log_loss_improvement
            - 1.96 * binary_log_loss_improvement_standard_error,
        binary_log_loss_improvement_ci95_upper: binary_log_loss_improvement
            + 1.96 * binary_log_loss_improvement_standard_error,
        season_clustered_binary_log_loss_improvement_standard_error,
        season_clustered_binary_log_loss_improvement_ci95_lower,
        season_clustered_binary_log_loss_improvement_ci95_upper,
        season_clustered_binary_log_loss_evidence: calibration_evidence_label(
            calibration_holdouts.len(),
            season_clustered_binary_log_loss_improvement_ci95_lower,
            season_clustered_binary_log_loss_improvement_ci95_upper,
        )
        .to_owned(),
    };
    let authoritative_opening_roster_seasons = inputs
        .iter()
        .filter(|input| input.authoritative_opening_roster)
        .count();
    let weights = holdouts
        .iter()
        .map(|row| row.selected_elo_weight)
        .collect::<Vec<_>>();
    let weight_span = weights.iter().copied().fold(f64::NEG_INFINITY, f64::max)
        - weights.iter().copied().fold(f64::INFINITY, f64::min);
    let elo_holdout_wins = holdouts
        .iter()
        .filter(|row| row.brier_improvement_vs_pure_elo > 0.0)
        .count();
    let pooled_elo_brier = pooled_sweep.last().unwrap().brier_score;
    let promotion_checks = vec![
        TeamGameForecastValidationCheckRow {
            key: "minimum_seasons".to_owned(),
            passed: inputs.len() >= 5,
            detail: format!("{} supplied; at least 5 required", inputs.len()),
        },
        TeamGameForecastValidationCheckRow {
            key: "opening_roster_authority".to_owned(),
            passed: authoritative_opening_roster_seasons == inputs.len(),
            detail: format!(
                "{authoritative_opening_roster_seasons}/{} seasons authoritative",
                inputs.len()
            ),
        },
        TeamGameForecastValidationCheckRow {
            key: "all_holdouts_beat_model".to_owned(),
            passed: holdouts
                .iter()
                .all(|row| row.brier_improvement_vs_model > 0.0),
            detail: "every holdout must have positive Brier improvement versus IceLines".to_owned(),
        },
        TeamGameForecastValidationCheckRow {
            key: "majority_holdouts_beat_elo".to_owned(),
            passed: elo_holdout_wins * 5 >= holdouts.len() * 3,
            detail: format!(
                "{elo_holdout_wins}/{} holdouts beat pure Elo; at least 60% required",
                holdouts.len()
            ),
        },
        TeamGameForecastValidationCheckRow {
            key: "pooled_blend_beats_elo".to_owned(),
            passed: pooled_best_by_brier.brier_score < pooled_elo_brier,
            detail: format!(
                "pooled blend {:.5} versus pure Elo {pooled_elo_brier:.5}",
                pooled_best_by_brier.brier_score
            ),
        },
        TeamGameForecastValidationCheckRow {
            key: "holdout_weight_stability".to_owned(),
            passed: weight_span <= 0.20 + 1e-12,
            detail: format!("selected-weight span {weight_span:.2}; maximum 0.20"),
        },
    ];
    let promotion_status = if promotion_checks.iter().all(|check| check.passed) {
        "candidate_for_versioned_evaluation"
    } else if authoritative_opening_roster_seasons != inputs.len() {
        "evaluation_only_missing_roster_authority"
    } else {
        "evaluation_only_failed_generalization_gate"
    }
    .to_owned();
    Ok(TeamGameForecastValidationView {
        schema: TEAM_GAME_FORECAST_VALIDATION_SCHEMA.to_owned(),
        seasons: inputs.iter().map(|input| input.season).collect(),
        total_games: inputs.iter().map(|input| input.games).sum(),
        pooled_sweep,
        pooled_best_by_brier,
        holdouts,
        calibration_holdouts,
        calibration_summary,
        authoritative_opening_roster_seasons,
        promotion_status,
        promotion_checks,
        disclosures: vec![
            "Each holdout weight is selected only from the other supplied seasons; positive improvement means the selected blend has lower loss.".to_owned(),
            "Chronological calibration holdouts are separate: each intercept and slope is fitted only on earlier supplied seasons, then frozen before scoring the next season.".to_owned(),
            "Calibration-summary intervals include paired per-game and delete-one-holdout-season jackknife views. The season-clustered interval is conditional on the fitted chronological sequence and can be unstable with few holdouts; neither interval includes model-selection uncertainty.".to_owned(),
            "This artifact evaluates supplied forecast files and does not change production probabilities or model parameters.".to_owned(),
            "Candidate status requires at least five seasons, authoritative opening rosters for every season, all holdouts beating IceLines, at least 60% beating pure Elo, pooled improvement over Elo, and a holdout weight span no greater than 0.20.".to_owned(),
        ],
    })
}

fn mean_and_standard_error(values: &[f64]) -> (f64, f64) {
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    if values.len() < 2 {
        return (mean, 0.0);
    }
    let sample_variance = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (values.len() - 1) as f64;
    (mean, (sample_variance / values.len() as f64).sqrt())
}

fn delete_one_cluster_standard_error(clusters: &[(usize, f64)]) -> f64 {
    if clusters.len() < 2 {
        return 0.0;
    }
    let total_games = clusters.iter().map(|(games, _)| games).sum::<usize>();
    let weighted_total = clusters
        .iter()
        .map(|(games, value)| *value * *games as f64)
        .sum::<f64>();
    let delete_one_estimates = clusters
        .iter()
        .map(|(omitted_games, omitted_value)| {
            let remaining_games = total_games - omitted_games;
            (weighted_total - *omitted_value * *omitted_games as f64) / remaining_games as f64
        })
        .collect::<Vec<_>>();
    let mean = delete_one_estimates.iter().sum::<f64>() / delete_one_estimates.len() as f64;
    (((clusters.len() - 1) as f64 / clusters.len() as f64)
        * delete_one_estimates
            .iter()
            .map(|estimate| (estimate - mean).powi(2))
            .sum::<f64>())
    .sqrt()
}

fn calibration_evidence_label(holdouts: usize, ci95_lower: f64, ci95_upper: f64) -> &'static str {
    if holdouts < 4 {
        "insufficient_holdouts"
    } else if ci95_lower > 0.0 {
        "positive"
    } else if ci95_upper < 0.0 {
        "negative"
    } else {
        "inconclusive"
    }
}

fn pool_validation_sweeps(
    inputs: &[TeamGameForecastValidationInput],
) -> Vec<TeamGameForecastBlendRow> {
    let total_games = inputs.iter().map(|input| input.games).sum::<usize>();
    (0..inputs[0].elo_blend_sweep.len())
        .map(|index| {
            let weighted = |metric: fn(&TeamGameForecastBlendRow) -> f64| {
                inputs
                    .iter()
                    .map(|input| metric(&input.elo_blend_sweep[index]) * input.games as f64)
                    .sum::<f64>()
                    / total_games as f64
            };
            let brier_score = weighted(|row| row.brier_score);
            let binary_log_loss = weighted(|row| row.binary_log_loss);
            let model_brier = weighted(|row| row.brier_score + row.brier_improvement_vs_model);
            let model_log = weighted(|row| row.binary_log_loss + row.log_loss_improvement_vs_model);
            TeamGameForecastBlendRow {
                elo_weight: inputs[0].elo_blend_sweep[index].elo_weight,
                games: total_games,
                pick_accuracy: weighted(|row| row.pick_accuracy),
                brier_score,
                binary_log_loss,
                brier_improvement_vs_model: model_brier - brier_score,
                log_loss_improvement_vs_model: model_log - binary_log_loss,
            }
        })
        .collect()
}

fn best_blend(rows: &[TeamGameForecastBlendRow]) -> &TeamGameForecastBlendRow {
    rows.iter()
        .min_by(|a, b| {
            a.brier_score
                .total_cmp(&b.brier_score)
                .then_with(|| a.elo_weight.total_cmp(&b.elo_weight))
        })
        .expect("validated non-empty blend grid")
}

fn build_factor_ablations(
    rows: &[&TeamGameForecastRow],
    model_brier: f64,
    model_log_loss: f64,
) -> Vec<TeamGameForecastAblationRow> {
    let factors = rows
        .iter()
        .flat_map(|row| row.factors.iter().map(|factor| factor.key.clone()))
        .collect::<BTreeSet<_>>();
    factors
        .into_iter()
        .map(|factor| {
            let mut correct = 0;
            let mut affected = 0;
            let mut brier = 0.0;
            let mut log_loss = 0.0;
            let mut absolute_delta = 0.0;
            for row in rows {
                let delta = row
                    .factors
                    .iter()
                    .filter(|candidate| candidate.key == factor)
                    .map(|candidate| candidate.home_win_probability_delta)
                    .sum::<f64>();
                affected += usize::from(delta.abs() > f64::EPSILON);
                absolute_delta += delta.abs();
                let ablated_probability =
                    (row.home_overall_win_probability - delta).clamp(0.01, 0.99);
                let home_won = row.actual_winner.as_deref() == Some(row.home_team.as_str());
                correct += usize::from((ablated_probability >= 0.5) == home_won);
                brier += (ablated_probability - f64::from(home_won)).powi(2);
                log_loss += score_binary_log_loss(ablated_probability, home_won);
            }
            brier /= rows.len() as f64;
            log_loss /= rows.len() as f64;
            TeamGameForecastAblationRow {
                factor,
                games: rows.len(),
                games_affected: affected,
                pick_accuracy: correct as f64 / rows.len() as f64,
                brier_score: brier,
                binary_log_loss: log_loss,
                mean_absolute_probability_delta: absolute_delta / rows.len() as f64,
                model_brier_improvement: brier - model_brier,
                model_log_loss_improvement: log_loss - model_log_loss,
            }
        })
        .collect()
}

fn build_binary_baseline<F>(
    name: &str,
    rows: &[&TeamGameForecastRow],
    model_brier: f64,
    model_log_loss: f64,
    probability: F,
) -> TeamGameForecastBaselineRow
where
    F: Fn(&TeamGameForecastRow) -> f64,
{
    let mut correct = 0;
    let mut brier = 0.0;
    let mut log_loss = 0.0;
    for row in rows {
        let home_won = row.actual_winner.as_deref() == Some(row.home_team.as_str());
        let home_probability = probability(row);
        correct += usize::from((home_probability >= 0.5) == home_won);
        brier += (home_probability - f64::from(home_won)).powi(2);
        log_loss += score_binary_log_loss(home_probability, home_won);
    }
    brier /= rows.len() as f64;
    log_loss /= rows.len() as f64;
    TeamGameForecastBaselineRow {
        name: name.to_owned(),
        games: rows.len(),
        pick_accuracy: correct as f64 / rows.len() as f64,
        brier_score: brier,
        binary_log_loss: log_loss,
        model_brier_improvement: brier - model_brier,
        model_log_loss_improvement: log_loss - model_log_loss,
    }
}

fn validate_parameters(parameters: &TeamForecastParameters) -> Result<(), String> {
    let values = [
        parameters.home_edge,
        parameters.strength_edge_scale,
        parameters.back_to_back_edge,
        parameters.three_in_four_edge,
        parameters.travel_edge_per_1000_km,
        parameters.timezone_edge,
        parameters.overtime_probability,
    ];
    if values
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0)
    {
        return Err("IceCast parameters must be finite and non-negative".to_owned());
    }
    if !(0.05..=0.50).contains(&parameters.overtime_probability) {
        return Err("IceCast overtime probability must be between 0.05 and 0.50".to_owned());
    }
    Ok(())
}

fn schedule_context(
    venue_team: &str,
    is_home: bool,
    date: NaiveDate,
    state: &TeamScheduleState,
) -> TeamGameScheduleContext {
    let rest_days = state.last_date.map(|last| (date - last).num_days() - 1);
    let games_in_four = state
        .recent_dates
        .iter()
        .filter(|prior| date.signed_duration_since(**prior) <= Duration::days(3))
        .count()
        + 1;
    let games_in_six = state
        .recent_dates
        .iter()
        .filter(|prior| date.signed_duration_since(**prior) <= Duration::days(5))
        .count()
        + 1;
    let travel_km = state
        .previous_venue
        .as_deref()
        .map_or(0.0, |previous| distance_between(previous, venue_team));
    let timezone_displacement_hours = state.previous_venue.as_deref().map_or(0, |previous| {
        (arena(previous).2 - arena(venue_team).2).abs()
    });
    TeamGameScheduleContext {
        rest_days,
        back_to_back: rest_days == Some(0),
        three_in_four: games_in_four >= 3,
        four_in_six: games_in_six >= 4,
        road_trip_index: if is_home { 0 } else { state.away_run + 1 },
        home_stand_index: if is_home { state.home_run + 1 } else { 0 },
        travel_km,
        timezone_displacement_hours,
        post_all_star_break: date >= NaiveDate::from_ymd_opt(2027, 2, 8).unwrap()
            && date <= NaiveDate::from_ymd_opt(2027, 2, 14).unwrap(),
    }
}

fn update_state(state: &mut TeamScheduleState, is_home: bool, date: NaiveDate, venue: &str) {
    state.last_date = Some(date);
    state
        .recent_dates
        .retain(|prior| date.signed_duration_since(*prior) <= Duration::days(5));
    state.recent_dates.push(date);
    if is_home {
        state.home_run += 1;
        state.away_run = 0;
    } else {
        state.away_run += 1;
        state.home_run = 0;
    }
    state.previous_venue = Some(venue.to_owned());
}

fn edge_factors(
    home_strength: f64,
    away_strength: f64,
    home: &TeamGameScheduleContext,
    away: &TeamGameScheduleContext,
    p: &TeamForecastParameters,
) -> Vec<(&'static str, String, f64)> {
    let mut out = vec![
        (
            "strength",
            format!(
                "roster/depth strength {:.1} vs {:.1}",
                home_strength, away_strength
            ),
            ((home_strength - away_strength) / 100.0) * p.strength_edge_scale,
        ),
        ("home_ice", "home ice".to_owned(), p.home_edge),
    ];
    if home.back_to_back {
        out.push((
            "home_back_to_back",
            "home team back-to-back".to_owned(),
            -p.back_to_back_edge,
        ));
    }
    if away.back_to_back {
        out.push((
            "away_back_to_back",
            "away team back-to-back".to_owned(),
            p.back_to_back_edge,
        ));
    }
    if home.three_in_four {
        out.push((
            "home_three_in_four",
            "home team three games in four nights".to_owned(),
            -p.three_in_four_edge,
        ));
    }
    if away.three_in_four {
        out.push((
            "away_three_in_four",
            "away team three games in four nights".to_owned(),
            p.three_in_four_edge,
        ));
    }
    let travel_edge = ((away.travel_km - home.travel_km) / 1000.0 * p.travel_edge_per_1000_km)
        .clamp(-0.015, 0.015);
    out.push((
        "travel",
        format!(
            "travel {:.0} km home / {:.0} km away",
            home.travel_km, away.travel_km
        ),
        travel_edge,
    ));
    let timezone_edge =
        f64::from(away.timezone_displacement_hours - home.timezone_displacement_hours)
            * p.timezone_edge;
    out.push((
        "timezone",
        format!(
            "timezone displacement {}h home / {}h away",
            home.timezone_displacement_hours, away.timezone_displacement_hours
        ),
        timezone_edge,
    ));
    out
}

fn distance_between(a: &str, b: &str) -> f64 {
    let (lat1, lon1, _) = arena(a);
    let (lat2, lon2, _) = arena(b);
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let lat1 = lat1.to_radians();
    let lat2 = lat2.to_radians();
    let h = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
    6371.0 * 2.0 * h.sqrt().asin()
}

fn arena(team: &str) -> (f64, f64, i8) {
    match team {
        "ANA" => (33.8078, -117.8765, -8),
        "ARI" => (33.4255, -111.9325, -7),
        "BOS" => (42.3662, -71.0621, -5),
        "BUF" => (42.8750, -78.8764, -5),
        "CAR" => (35.8033, -78.7218, -5),
        "CBJ" => (39.9693, -83.0061, -5),
        "CGY" => (51.0375, -114.0519, -7),
        "CHI" => (41.8807, -87.6742, -6),
        "COL" => (39.7487, -105.0077, -7),
        "DAL" => (32.7905, -96.8103, -6),
        "DET" => (42.3411, -83.0550, -5),
        "EDM" => (53.5469, -113.4978, -7),
        "FLA" => (26.1584, -80.3256, -5),
        "LAK" => (34.0430, -118.2673, -8),
        "MIN" => (44.9448, -93.1011, -6),
        "MTL" => (45.4961, -73.5693, -5),
        "NJD" => (40.7335, -74.1711, -5),
        "NSH" => (36.1592, -86.7785, -6),
        "NYI" => (40.7229, -73.5907, -5),
        "NYR" => (40.7505, -73.9934, -5),
        "OTT" => (45.2969, -75.9272, -5),
        "PHI" => (39.9012, -75.1720, -5),
        "PIT" => (40.4396, -79.9892, -5),
        "SEA" => (47.6221, -122.3540, -8),
        "SJS" => (37.3328, -121.9012, -8),
        "STL" => (38.6268, -90.2026, -6),
        "TBL" => (27.9427, -82.4518, -5),
        "TOR" => (43.6435, -79.3791, -5),
        "UTA" => (40.7683, -111.9011, -7),
        "VAN" => (49.2778, -123.1089, -8),
        "VGK" => (36.1029, -115.1783, -8),
        "WPG" => (49.8927, -97.1436, -6),
        "WSH" => (38.8981, -77.0209, -5),
        _ => (0.0, 0.0, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pre_utah_arizona_arena_preserves_historical_travel_context() {
        assert_eq!(arena("ARI"), (33.4255, -111.9325, -7));
    }

    #[test]
    fn calibration_summary_accepts_pre_interval_v1_json() {
        let summary: TeamGameForecastCalibrationSummary =
            serde_json::from_value(serde_json::json!({
                "holdout_seasons": 2,
                "games": 200,
                "holdouts_improved_brier": 2,
                "holdouts_improved_binary_log_loss": 2,
                "uncalibrated_brier_score": 0.24,
                "recalibrated_brier_score": 0.22,
                "brier_improvement": 0.02,
                "uncalibrated_binary_log_loss": 0.68,
                "recalibrated_binary_log_loss": 0.64,
                "binary_log_loss_improvement": 0.04
            }))
            .expect("v1 summary without later uncertainty fields should remain readable");

        assert_eq!(summary.games, 200);
        assert_eq!(summary.brier_improvement, 0.02);
        assert_eq!(summary.brier_improvement_standard_error, 0.0);
        assert_eq!(summary.binary_log_loss_improvement_ci95_lower, 0.0);
        assert!(summary.season_clustered_brier_evidence.is_empty());
    }

    #[test]
    fn season_clustered_standard_error_uses_delete_one_holdout_jackknife() {
        let standard_error =
            delete_one_cluster_standard_error(&[(100, 0.01), (100, 0.03), (100, -0.01)]);

        assert!((standard_error - 0.011_547_005_383_792_516).abs() < 1e-12);
    }

    #[test]
    fn calibration_evidence_requires_four_holdouts_and_a_one_sided_interval() {
        assert_eq!(
            calibration_evidence_label(3, 0.01, 0.03),
            "insufficient_holdouts"
        );
        assert_eq!(calibration_evidence_label(4, 0.01, 0.03), "positive");
        assert_eq!(calibration_evidence_label(4, -0.03, -0.01), "negative");
        assert_eq!(calibration_evidence_label(4, -0.01, 0.03), "inconclusive");
    }

    #[test]
    fn cross_validation_selects_weights_without_using_the_holdout() {
        let input = |season: u32, losses: [f64; 3]| TeamGameForecastValidationInput {
            season,
            games: 100,
            authoritative_opening_roster: true,
            elo_blend_sweep: [0.0, 0.5, 1.0]
                .into_iter()
                .zip(losses)
                .map(|(elo_weight, brier_score)| TeamGameForecastBlendRow {
                    elo_weight,
                    games: 100,
                    pick_accuracy: 0.5,
                    brier_score,
                    binary_log_loss: brier_score * 2.0,
                    brier_improvement_vs_model: losses[0] - brier_score,
                    log_loss_improvement_vs_model: (losses[0] - brier_score) * 2.0,
                })
                .collect(),
            calibration_observations: (0..100)
                .map(|index| TeamGameForecastCalibrationObservation {
                    home_win_probability: if index < 50 { 0.25 } else { 0.75 },
                    home_won: if index < 50 {
                        index % 5 == 0
                    } else {
                        index % 5 != 0
                    },
                })
                .collect(),
        };
        let inputs = vec![
            input(20212022, [0.30, 0.20, 0.21]),
            input(20222023, [0.30, 0.22, 0.20]),
            input(20232024, [0.30, 0.23, 0.19]),
        ];
        let view = build_team_game_forecast_validation(inputs.clone()).unwrap();
        assert_eq!(view.schema, TEAM_GAME_FORECAST_VALIDATION_SCHEMA);
        assert_eq!(view.total_games, 300);
        assert_eq!(view.holdouts.len(), 3);
        assert_eq!(view.pooled_best_by_brier.elo_weight, 1.0);
        assert_eq!(
            view.promotion_status,
            "evaluation_only_failed_generalization_gate"
        );
        assert!(view
            .promotion_checks
            .iter()
            .any(|check| check.key == "minimum_seasons" && !check.passed));
        assert!(view.holdouts.iter().all(|row| {
            !row.training_seasons.contains(&row.holdout_season)
                && row.training_seasons.len() == 2
                && row.brier_improvement_vs_model > 0.0
        }));
        assert_eq!(view.calibration_holdouts.len(), 2);
        assert_eq!(
            view.calibration_holdouts[0].training_seasons,
            vec![20212022]
        );
        assert_eq!(
            view.calibration_holdouts[1].training_seasons,
            vec![20212022, 20222023]
        );
        assert!(view
            .calibration_holdouts
            .iter()
            .all(|row| row.brier_improvement > 0.0 && row.binary_log_loss_improvement > 0.0));
        assert_eq!(view.calibration_summary.holdout_seasons, 2);
        assert_eq!(view.calibration_summary.games, 200);
        assert_eq!(view.calibration_summary.holdouts_improved_brier, 2);
        assert_eq!(
            view.calibration_summary.holdouts_improved_binary_log_loss,
            2
        );
        let expected_brier = view
            .calibration_holdouts
            .iter()
            .map(|row| row.recalibrated_brier_score * row.games as f64)
            .sum::<f64>()
            / 200.0;
        assert!((view.calibration_summary.recalibrated_brier_score - expected_brier).abs() < 1e-12);
        assert!(view.calibration_summary.brier_improvement > 0.0);
        assert!(view.calibration_summary.binary_log_loss_improvement > 0.0);
        assert!(view.calibration_summary.brier_improvement_standard_error > 0.0);
        assert!(
            view.calibration_summary.brier_improvement_ci95_lower
                < view.calibration_summary.brier_improvement
        );
        assert!(
            view.calibration_summary.brier_improvement
                < view.calibration_summary.brier_improvement_ci95_upper
        );
        assert!(
            view.calibration_summary
                .binary_log_loss_improvement_standard_error
                > 0.0
        );
        assert!(
            view.calibration_summary
                .binary_log_loss_improvement_ci95_lower
                < view.calibration_summary.binary_log_loss_improvement
        );
        assert!(
            view.calibration_summary.binary_log_loss_improvement
                < view
                    .calibration_summary
                    .binary_log_loss_improvement_ci95_upper
        );
        assert!(
            view.calibration_summary
                .season_clustered_brier_improvement_ci95_lower
                <= view.calibration_summary.brier_improvement
        );
        assert!(
            view.calibration_summary.brier_improvement
                <= view
                    .calibration_summary
                    .season_clustered_brier_improvement_ci95_upper
        );
        assert_eq!(
            view.calibration_summary.season_clustered_brier_evidence,
            "insufficient_holdouts"
        );
        assert_eq!(
            view.calibration_summary
                .season_clustered_binary_log_loss_evidence,
            "insufficient_holdouts"
        );

        let error = build_team_game_forecast_validation(vec![
            inputs[0].clone(),
            inputs[0].clone(),
            inputs[2].clone(),
        ])
        .unwrap_err();
        assert!(error.contains("unique"));

        let mut missing_authority = inputs;
        missing_authority[1].authoritative_opening_roster = false;
        let view = build_team_game_forecast_validation(missing_authority).unwrap();
        assert_eq!(
            view.promotion_status,
            "evaluation_only_missing_roster_authority"
        );

        let candidate = build_team_game_forecast_validation(
            [20212022, 20222023, 20232024, 20242025, 20252026]
                .into_iter()
                .map(|season| input(season, [0.30, 0.20, 0.25]))
                .collect(),
        )
        .unwrap();
        assert_eq!(
            candidate.promotion_status,
            "candidate_for_versioned_evaluation"
        );
        assert!(candidate.promotion_checks.iter().all(|check| check.passed));
    }

    #[test]
    fn probabilities_normalize_and_factors_reconcile() {
        let games = vec![
            TeamForecastGameInput {
                game_id: 1,
                date: NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
                away_team: "SEA".into(),
                home_team: "NYR".into(),
                away_score: Some(2),
                home_score: Some(4),
                final_result: true,
                last_period: Some("REG".into()),
            },
            TeamForecastGameInput {
                game_id: 2,
                date: NaiveDate::from_ymd_opt(2026, 10, 2).unwrap(),
                away_team: "SEA".into(),
                home_team: "BOS".into(),
                away_score: Some(3),
                home_score: Some(2),
                final_result: true,
                last_period: Some("OT".into()),
            },
            TeamForecastGameInput {
                game_id: 3,
                date: NaiveDate::from_ymd_opt(2026, 10, 3).unwrap(),
                away_team: "NYR".into(),
                home_team: "BOS".into(),
                away_score: None,
                home_score: None,
                final_result: false,
                last_period: None,
            },
        ];
        let view = build_team_game_forecast(
            20262027,
            games,
            vec![
                TeamForecastStrengthInput {
                    team: "NYR".into(),
                    strength: 60.0,
                },
                TeamForecastStrengthInput {
                    team: "SEA".into(),
                    strength: 50.0,
                },
                TeamForecastStrengthInput {
                    team: "BOS".into(),
                    strength: 55.0,
                },
            ],
            TeamForecastParameters::default(),
            None,
            None,
        )
        .unwrap();
        for row in &view.games {
            assert!(
                (row.home_regulation_win_probability
                    + row.away_regulation_win_probability
                    + row.overtime_probability
                    - 1.0)
                    .abs()
                    < 1e-12
            );
            assert!(
                (row.home_overall_win_probability + row.away_overall_win_probability - 1.0).abs()
                    < 1e-12
            );
            let attribution: f64 = row
                .factors
                .iter()
                .map(|factor| factor.home_win_probability_delta)
                .sum();
            assert!((0.5 + attribution - row.home_overall_win_probability).abs() < 1e-12);
        }
        assert!(view.games[1].away_context.back_to_back);
        let accuracy = view.accuracy.as_ref().unwrap();
        assert_eq!(accuracy.final_games, 2);
        assert_eq!(accuracy.pending_games, 1);
        assert_eq!(accuracy.correct_picks, 1);
        assert_eq!(accuracy.pick_accuracy, 0.5);
        assert!(accuracy.brier_score.is_finite());
        assert!(accuracy.binary_log_loss.is_finite());
        assert!(accuracy.multiclass_log_loss.is_some_and(f64::is_finite));
        assert!(accuracy.brier_skill_vs_coinflip.is_finite());
        assert!(accuracy
            .multiclass_log_loss_skill_vs_uniform
            .is_some_and(f64::is_finite));
        assert!((0.0..=1.0).contains(&accuracy.expected_calibration_error));
        assert!(accuracy.calibration_intercept.is_none());
        assert!(accuracy.calibration_slope.is_none());
        assert!(accuracy.calibration_intercept_standard_error.is_none());
        assert!(accuracy.calibration_slope_standard_error.is_none());
        assert_eq!(
            accuracy
                .by_confidence
                .iter()
                .map(|row| row.games)
                .sum::<usize>(),
            2
        );
        assert_eq!(
            accuracy
                .calibration_bins
                .iter()
                .map(|row| row.games)
                .sum::<usize>(),
            2
        );
        assert_eq!(accuracy.baselines.len(), 2);
        assert!(accuracy.baselines.iter().all(|baseline| {
            baseline.games == 2
                && baseline.brier_score.is_finite()
                && baseline.binary_log_loss.is_finite()
        }));
        assert!(accuracy
            .ablations
            .iter()
            .any(|ablation| ablation.factor == "strength"));
        assert!(accuracy
            .ablations
            .iter()
            .any(|ablation| ablation.factor == "home_ice"));
        assert!(accuracy.ablations.iter().all(|ablation| {
            ablation.games == 2
                && ablation.games_affected <= ablation.games
                && ablation.brier_score.is_finite()
                && ablation.binary_log_loss.is_finite()
                && ablation.model_brier_improvement.is_finite()
        }));
        assert!(view
            .games
            .windows(2)
            .all(|pair| (pair[0].home_only_home_win_probability
                - pair[1].home_only_home_win_probability)
                .abs()
                < 1e-12));
        assert!(
            (view.games[1].elo_home_win_probability - view.games[0].elo_home_win_probability).abs()
                < 1e-12
        );
        assert_eq!(accuracy.baselines[1].name, "frozen_equal_rating_elo");
        assert!(accuracy.elo_blend_sweep.is_empty());
        assert!(accuracy.best_elo_blend_by_brier.is_none());
        assert!(view
            .games
            .iter()
            .all(|game| game.standings_home_win_probability.is_none()));
        assert_eq!(view.games[0].actual_winner.as_deref(), Some("NYR"));
        assert!(view.games[0].binary_log_loss.is_some());
        assert!(view.games[0].multiclass_log_loss.is_some());
        assert_eq!(view.games[1].actual_ending.as_deref(), Some("OT"));
        assert_eq!(view.games[1].pick_correct, Some(false));
        assert_eq!(view.games[2].pick_correct, None);
    }

    #[test]
    fn logistic_calibration_recovers_a_calibrated_probability_scale() {
        let mut values = Vec::new();
        for index in 0..50 {
            values.push((0.2, index < 10));
        }
        for index in 0..50 {
            values.push((0.8, index < 40));
        }
        let (intercept, slope, intercept_standard_error, slope_standard_error) =
            fit_logistic_calibration_values(&values).unwrap();
        assert!(intercept.abs() < 1e-8, "intercept={intercept}");
        assert!((slope - 1.0).abs() < 1e-8, "slope={slope}");
        assert!(intercept_standard_error.is_finite() && intercept_standard_error > 0.0);
        assert!(slope_standard_error.is_finite() && slope_standard_error > 0.0);
        assert!(fit_logistic_calibration_values(&values[..10]).is_none());
    }

    #[test]
    fn official_shape_accepts_1344_games_and_84_per_team() {
        let teams = (0..32)
            .map(|index| format!("T{index:02}"))
            .collect::<Vec<_>>();
        let mut games = Vec::new();
        for round in 0..84 {
            for pair in 0..16 {
                let a = &teams[pair * 2];
                let b = &teams[pair * 2 + 1];
                let (away, home) = if round % 2 == 0 { (a, b) } else { (b, a) };
                games.push(TeamForecastGameInput {
                    game_id: (round * 16 + pair) as u64,
                    date: NaiveDate::from_ymd_opt(2026, 9, 29).unwrap()
                        + Duration::days(round as i64),
                    away_team: away.clone(),
                    home_team: home.clone(),
                    away_score: None,
                    home_score: None,
                    final_result: false,
                    last_period: None,
                });
            }
        }
        let view = build_team_game_forecast(
            20262027,
            games,
            Vec::new(),
            TeamForecastParameters::default(),
            Some(1344),
            Some(84),
        )
        .unwrap();
        assert_eq!(view.schedule_games, 1344);
        assert!(view
            .teams
            .iter()
            .all(|team| team.games == 84 && team.home_games == 42 && team.away_games == 42));
    }

    #[test]
    fn rolling_replay_uses_only_results_from_earlier_dates() {
        let day_one = NaiveDate::from_ymd_opt(2025, 10, 1).unwrap();
        let games = vec![
            TeamForecastGameInput {
                game_id: 1,
                date: day_one,
                away_team: "AAA".into(),
                home_team: "BBB".into(),
                away_score: Some(1),
                home_score: Some(4),
                final_result: true,
                last_period: Some("REG".into()),
            },
            TeamForecastGameInput {
                game_id: 2,
                date: day_one + Duration::days(1),
                away_team: "AAA".into(),
                home_team: "BBB".into(),
                away_score: Some(5),
                home_score: Some(1),
                final_result: true,
                last_period: Some("REG".into()),
            },
            TeamForecastGameInput {
                game_id: 3,
                date: day_one + Duration::days(1),
                away_team: "AAA".into(),
                home_team: "CCC".into(),
                away_score: Some(2),
                home_score: Some(3),
                final_result: true,
                last_period: Some("OT".into()),
            },
            TeamForecastGameInput {
                game_id: 4,
                date: day_one + Duration::days(2),
                away_team: "AAA".into(),
                home_team: "CCC".into(),
                away_score: None,
                home_score: None,
                final_result: false,
                last_period: None,
            },
        ];
        let personnel = vec![
            TeamForecastPersonnelEvidenceInput {
                event_id: "ir-1".into(),
                date: day_one,
                team: "AAA".into(),
                kind: "ir".into(),
                label: "AAA player placed on injured reserve".into(),
                source: "fixture".into(),
                availability_delta: 1,
                resolved_players: vec![TeamForecastPersonnelPlayerInput {
                    player_id: 101,
                    full_name: "AAA Player".into(),
                    action: "ir_placed".into(),
                    membership_delta: 0,
                    prior_position_group: Some("forward".into()),
                    prior_season: Some(20242025),
                    prior_games_played: Some(82),
                    prior_value: Some(70.0),
                }],
                ambiguous_player_names: Vec::new(),
            },
            TeamForecastPersonnelEvidenceInput {
                event_id: "ir-2".into(),
                date: day_one + Duration::days(1),
                team: "AAA".into(),
                kind: "ir".into(),
                label: "AAA player activated from injured reserve".into(),
                source: "fixture".into(),
                availability_delta: -1,
                resolved_players: vec![TeamForecastPersonnelPlayerInput {
                    player_id: 101,
                    full_name: "AAA Player".into(),
                    action: "activated".into(),
                    membership_delta: 0,
                    prior_position_group: Some("forward".into()),
                    prior_season: Some(20242025),
                    prior_games_played: Some(82),
                    prior_value: Some(70.0),
                }],
                ambiguous_player_names: Vec::new(),
            },
            TeamForecastPersonnelEvidenceInput {
                event_id: "recall-1".into(),
                date: day_one,
                team: "AAA".into(),
                kind: "recall".into(),
                label: "Recalled Added Player".into(),
                source: "fixture".into(),
                availability_delta: 0,
                resolved_players: vec![TeamForecastPersonnelPlayerInput {
                    player_id: 102,
                    full_name: "Added Player".into(),
                    action: "recalled".into(),
                    membership_delta: 1,
                    prior_position_group: None,
                    prior_season: None,
                    prior_games_played: None,
                    prior_value: None,
                }],
                ambiguous_player_names: Vec::new(),
            },
        ];
        let view = build_team_game_rolling_replay_with_personnel(
            20252026,
            games,
            TeamForecastParameters::default(),
            None,
            None,
            TeamForecastReplayConfig::default(),
            personnel,
        )
        .unwrap();

        assert_eq!(view.forecast_mode, "rolling_results_replay_v1");
        assert_eq!(view.games[0].away_evidence_games, 0);
        assert_eq!(view.games[0].away_strength, 50.0);
        assert_eq!(view.games[0].away_known_personnel_events, 0);
        assert_eq!(view.games[1].away_evidence_games, 1);
        assert_eq!(view.games[1].away_known_personnel_events, 2);
        assert_eq!(view.games[1].away_active_ir_signals, 1);
        assert_eq!(view.games[1].away_known_roster_additions, 1);
        assert_eq!(view.games[2].away_evidence_games, 1);
        assert_eq!(view.games[2].away_known_personnel_events, 2);
        assert_eq!(view.games[1].away_strength, view.games[2].away_strength);
        let initial_elo_home = elo_home_win_probability(1500.0, 1500.0);
        let day_one_delta = 20.0 * (1.0 - initial_elo_home);
        assert!(
            (view.games[1].elo_home_win_probability
                - elo_home_win_probability(1500.0 + day_one_delta, 1500.0 - day_one_delta))
            .abs()
                < 1e-12
        );
        assert!(
            (view.games[2].elo_home_win_probability
                - elo_home_win_probability(1500.0, 1500.0 - day_one_delta))
            .abs()
                < 1e-12
        );
        assert_eq!(
            view.accuracy.as_ref().unwrap().baselines[1].name,
            "chronological_elo"
        );
        assert_eq!(
            view.accuracy.as_ref().unwrap().baselines[2].name,
            "rolling_standings"
        );
        let accuracy = view.accuracy.as_ref().unwrap();
        assert_eq!(accuracy.elo_blend_sweep.len(), 11);
        assert_eq!(accuracy.elo_blend_sweep[0].elo_weight, 0.0);
        assert_eq!(accuracy.elo_blend_sweep[10].elo_weight, 1.0);
        assert!(accuracy
            .elo_blend_sweep
            .iter()
            .all(|row| row.games == 3 && row.brier_score.is_finite()));
        assert!(accuracy.best_elo_blend_by_brier.is_some());
        assert!(view.games[0].standings_home_win_probability.is_some());
        assert!(
            view.games[1].standings_home_win_probability
                > view.games[2].standings_home_win_probability
        );
        assert!(
            view.games[2].standings_home_win_probability
                > Some(view.games[2].home_only_home_win_probability)
        );
        assert_eq!(view.games[3].away_evidence_games, 3);
        assert_eq!(view.games[3].away_known_personnel_events, 3);
        assert_eq!(view.games[3].away_active_ir_signals, 0);
        assert_ne!(view.games[3].away_strength, view.games[2].away_strength);
        assert_eq!(view.personnel_evidence.len(), 3);
        assert_eq!(view.membership_intervals.len(), 1);
        assert_eq!(view.membership_intervals[0].confidence, "sourced");
        assert_eq!(
            view.personnel_evidence[0].resolved_players[0].player_id,
            101
        );
        assert_eq!(
            view.games[2].evidence_cutoff_date,
            Some(day_one + Duration::days(1))
        );
    }

    #[test]
    fn membership_intervals_preserve_unknown_openings_and_never_overlap() {
        let first = NaiveDate::from_ymd_opt(2025, 10, 1).unwrap();
        let event = |id: &str, date: NaiveDate, delta: i8| TeamForecastPersonnelEvidenceInput {
            event_id: id.to_owned(),
            date,
            team: "AAA".into(),
            kind: "transaction".into(),
            label: id.to_owned(),
            source: "fixture".into(),
            availability_delta: 0,
            resolved_players: vec![TeamForecastPersonnelPlayerInput {
                player_id: 101,
                full_name: "Test Player".into(),
                action: if delta > 0 { "recalled" } else { "assigned" }.into(),
                membership_delta: delta,
                prior_position_group: Some("forward".into()),
                prior_season: Some(20242025),
                prior_games_played: Some(40),
                prior_value: Some(60.0),
            }],
            ambiguous_player_names: Vec::new(),
        };
        let events = vec![
            event("remove-opening", first, -1),
            event("duplicate-remove", first + Duration::days(1), -1),
            event("add", first + Duration::days(2), 1),
            event("duplicate-add", first + Duration::days(3), 1),
            event("remove", first + Duration::days(4), -1),
            event("re-add", first + Duration::days(5), 1),
        ];

        let (intervals, anomalies) =
            build_membership_intervals(&events, first + Duration::days(10));

        assert_eq!(anomalies.len(), 2);
        assert_eq!(anomalies[0].reason, "removal_while_absent");
        assert_eq!(anomalies[1].reason, "addition_while_open");
        assert_eq!(intervals.len(), 3);
        assert_eq!(intervals[0].start_event_date, None);
        assert_eq!(intervals[0].end_event_id.as_deref(), Some("remove-opening"));
        assert_eq!(intervals[0].confidence, "implied_preexisting");
        assert_eq!(intervals[1].start_event_id.as_deref(), Some("add"));
        assert_eq!(intervals[1].end_event_id.as_deref(), Some("remove"));
        assert_eq!(intervals[2].start_event_id.as_deref(), Some("re-add"));
        assert_eq!(intervals[2].end_event_date, None);
    }

    #[test]
    fn authoritative_opening_strength_replaces_neutral_prior_only_for_covered_team() {
        let date = NaiveDate::from_ymd_opt(2025, 10, 1).unwrap();
        let games = (0..3)
            .map(|offset| TeamForecastGameInput {
                game_id: 1 + offset,
                date: date + Duration::days(offset as i64),
                away_team: "AAA".into(),
                home_team: "BBB".into(),
                away_score: None,
                home_score: None,
                final_result: false,
                last_period: None,
            })
            .collect();
        let players = (0..20)
            .map(|index| TeamGameOpeningPlayerRow {
                player_id: 100 + index,
                full_name: format!("Player {index}"),
                position_group: if index < 12 {
                    "forward"
                } else if index < 18 {
                    "defense"
                } else {
                    "goalie"
                }
                .into(),
                prior_value: Some(64.0),
                modeled_value: 64.0,
                selected_at_opening: true,
            })
            .collect();
        let opening = vec![TeamGameOpeningStrengthRow {
            team: "AAA".into(),
            as_of_date: Some(date - Duration::days(1)),
            strength: 60.0,
            cohort_normalization_delta: -4.0,
            roster_players: 20,
            valued_players: 20,
            value_coverage: 1.0,
            forwards_used: 12,
            defensemen_used: 6,
            goalies_used: 2,
            players,
        }];

        let assignment = TeamForecastPersonnelEvidenceInput {
            event_id: "assign-100".into(),
            date,
            team: "AAA".into(),
            kind: "transaction".into(),
            label: "Assigned Player 0".into(),
            source: "fixture".into(),
            availability_delta: 0,
            resolved_players: vec![TeamForecastPersonnelPlayerInput {
                player_id: 100,
                full_name: "Player 0".into(),
                action: "assigned".into(),
                membership_delta: -1,
                prior_position_group: Some("forward".into()),
                prior_season: Some(20242025),
                prior_games_played: Some(82),
                prior_value: Some(64.0),
            }],
            ambiguous_player_names: Vec::new(),
        };
        let reflected_assignment = TeamForecastPersonnelEvidenceInput {
            event_id: "assign-101-before-snapshot".into(),
            date: date - Duration::days(1),
            team: "AAA".into(),
            kind: "transaction".into(),
            label: "Assigned Player 1 before snapshot".into(),
            source: "fixture".into(),
            availability_delta: 0,
            resolved_players: vec![TeamForecastPersonnelPlayerInput {
                player_id: 101,
                full_name: "Player 1".into(),
                action: "assigned".into(),
                membership_delta: -1,
                prior_position_group: Some("forward".into()),
                prior_season: Some(20242025),
                prior_games_played: Some(82),
                prior_value: Some(64.0),
            }],
            ambiguous_player_names: Vec::new(),
        };
        let newcomer_recall = TeamForecastPersonnelEvidenceInput {
            event_id: "recall-newcomer".into(),
            date: date + Duration::days(1),
            team: "AAA".into(),
            kind: "transaction".into(),
            label: "Recalled New Player".into(),
            source: "fixture".into(),
            availability_delta: 0,
            resolved_players: vec![TeamForecastPersonnelPlayerInput {
                player_id: 200,
                full_name: "New Player".into(),
                action: "recalled".into(),
                membership_delta: 1,
                prior_position_group: Some("forward".into()),
                prior_season: Some(20242025),
                prior_games_played: Some(82),
                prior_value: Some(80.0),
            }],
            ambiguous_player_names: Vec::new(),
        };
        let view = build_team_game_rolling_replay_with_opening_strengths(
            20252026,
            games,
            TeamForecastParameters::default(),
            None,
            None,
            TeamForecastReplayConfig::default(),
            vec![reflected_assignment, assignment, newcomer_recall],
            opening,
        )
        .unwrap();

        assert_eq!(view.games[0].away_strength, 60.0);
        assert_eq!(view.games[0].home_strength, 50.0);
        assert!(view.games[0].factors.iter().any(
            |factor| factor.key == "opening_roster" && factor.home_win_probability_delta < 0.0
        ));
        assert!(!view.games[0]
            .factors
            .iter()
            .any(|factor| factor.key == "personnel"));
        let expected_after_assignment = 60.0 - 14.0 * 0.55 / 12.0;
        assert!((view.games[1].away_strength - expected_after_assignment).abs() < 1e-9);
        assert!(
            (view.games[1].away_personnel_strength_delta - (expected_after_assignment - 60.0))
                .abs()
                < 1e-9
        );
        assert!(view.games[1]
            .factors
            .iter()
            .any(|factor| factor.key == "personnel" && factor.home_win_probability_delta > 0.0));
        let expected_after_newcomer = expected_after_assignment + 30.0 * 0.55 / 12.0;
        assert!((view.games[2].away_strength - expected_after_newcomer).abs() < 1e-9);
        assert!(view.games[2].away_personnel_strength_delta > 0.0);
        assert_eq!(view.opening_strengths.len(), 1);
    }

    #[test]
    fn repeated_player_transitions_do_not_inflate_replay_state() {
        let date = NaiveDate::from_ymd_opt(2025, 10, 1).unwrap();
        let event =
            |id: &str, action: &str, membership_delta: i8| TeamForecastPersonnelEvidenceInput {
                event_id: id.into(),
                date,
                team: "AAA".into(),
                kind: "transaction".into(),
                label: id.into(),
                source: "fixture".into(),
                availability_delta: match action {
                    "ir_placed" => 1,
                    "activated" => -1,
                    _ => 0,
                },
                resolved_players: vec![TeamForecastPersonnelPlayerInput {
                    player_id: 101,
                    full_name: "Test Player".into(),
                    action: action.into(),
                    membership_delta,
                    prior_position_group: None,
                    prior_season: None,
                    prior_games_played: None,
                    prior_value: None,
                }],
                ambiguous_player_names: Vec::new(),
            };
        let mut states = BTreeMap::new();
        let mut roster_players = BTreeMap::new();
        let mut membership = BTreeMap::new();
        let mut ir_players = BTreeSet::new();
        for evidence in [
            event("recall-1", "recalled", 1),
            event("recall-2", "recalled", 1),
            event("ir-1", "ir_placed", 0),
            event("ir-2", "ir_placed", 0),
            event("activate", "activated", 0),
            event("assign-1", "assigned", -1),
            event("assign-2", "assigned", -1),
        ] {
            apply_personnel_evidence(
                &mut states,
                &mut roster_players,
                &mut membership,
                &mut ir_players,
                &evidence,
            );
        }
        let state = states.get("AAA").unwrap();
        assert_eq!(state.known_personnel_events, 7);
        assert_eq!(state.known_roster_additions, 1);
        assert_eq!(state.known_roster_removals, 1);
        assert_eq!(state.active_ir_signals, 0);
    }

    #[test]
    fn paired_trade_moves_only_a_player_known_active_on_the_source_team() {
        let date = NaiveDate::from_ymd_opt(2026, 12, 1).unwrap();
        let event = |event_id: &str, team: &str, action: &str| TeamForecastPersonnelEvidenceInput {
            event_id: event_id.into(),
            date,
            team: team.into(),
            kind: "trade".into(),
            label: event_id.into(),
            source: "fixture".into(),
            availability_delta: 0,
            resolved_players: vec![TeamForecastPersonnelPlayerInput {
                player_id: 101,
                full_name: "Trade Player".into(),
                action: action.into(),
                membership_delta: 0,
                prior_position_group: Some("forward".into()),
                prior_season: Some(20252026),
                prior_games_played: Some(82),
                prior_value: Some(70.0),
            }],
            ambiguous_player_names: Vec::new(),
        };
        let events = vec![
            event("away", "AAA", "traded_away"),
            event("acquired", "BBB", "acquired"),
        ];
        let (mut trades, by_event) = build_paired_trades(&events, date);
        assert_eq!(trades.len(), 1);
        let source_key = ("AAA".to_owned(), 101);
        let destination_key = ("BBB".to_owned(), 101);
        let mut roster_players = BTreeMap::from([(
            source_key.clone(),
            TeamGameOpeningPlayerRow {
                player_id: 101,
                full_name: "Trade Player".into(),
                position_group: "forward".into(),
                prior_value: Some(70.0),
                modeled_value: 70.0,
                selected_at_opening: true,
            },
        )]);
        let mut membership = BTreeMap::from([(source_key.clone(), true)]);
        let mut states = BTreeMap::from([(
            "AAA".to_owned(),
            TeamReplayState {
                active_ir_signals: 1,
                ..TeamReplayState::default()
            },
        )]);
        let mut ir_players = BTreeSet::from([source_key.clone()]);
        let mut applied = BTreeSet::new();
        apply_paired_trades_for_event(
            &mut states,
            &mut roster_players,
            &mut membership,
            &mut ir_players,
            &mut trades,
            &by_event,
            &mut applied,
            &events[1],
        );

        assert_eq!(membership.get(&source_key), Some(&false));
        assert_eq!(membership.get(&destination_key), Some(&true));
        assert!(roster_players.contains_key(&destination_key));
        assert_eq!(states["AAA"].known_roster_removals, 1);
        assert_eq!(states["AAA"].active_ir_signals, 0);
        assert_eq!(states["BBB"].known_roster_additions, 1);
        assert_eq!(states["BBB"].active_ir_signals, 1);
        assert!(!ir_players.contains(&source_key));
        assert!(ir_players.contains(&destination_key));
        assert!(trades[0].active_lineup_applied);
        assert_eq!(trades[0].disposition, "active_lineup_transferred");

        let (mut unknown_trades, unknown_by_event) = build_paired_trades(&events, date);
        apply_paired_trades_for_event(
            &mut BTreeMap::new(),
            &mut BTreeMap::new(),
            &mut BTreeMap::new(),
            &mut BTreeSet::new(),
            &mut unknown_trades,
            &unknown_by_event,
            &mut BTreeSet::new(),
            &events[0],
        );
        assert!(!unknown_trades[0].active_lineup_applied);
        assert_eq!(unknown_trades[0].disposition, "source_not_known_active");

        let (unpaired, _) = build_paired_trades(&events[..1], date);
        assert!(unpaired.is_empty());
    }
}
