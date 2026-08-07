//! Dated player-profile, line-chemistry, and opponent-matchup forecasting.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::teams::CANONICAL_TEAMS;

use super::management_behavior::{
    BenchGamePlanView, OpponentTacticalStyle, BENCH_GAME_PLAN_SCHEMA,
};
use super::team_game_prediction_edge::{TeamGameEvidenceState, TeamGameForecastVintage};
use super::team_lineup::{
    TeamLineupPlayerView, TeamLineupProjectionView, TeamLineupSpecialTeamsKind,
};

pub const PLAYER_LINE_MATCHUP_FORECAST_SCHEMA: &str = "player_line_matchup_forecast.v1";
pub const PLAYER_LINE_MATCHUP_FORECAST_JSON_SCHEMA: &str =
    include_str!("../../../design/schemas/player_line_matchup_forecast.v1.schema.json");
pub const PLAYER_LINE_MATCHUP_FORECAST_METHOD: &str = "profile_line_matchup.v1";
pub const PLAYER_LINE_MATCHUP_SCENARIO_COMPARISON_SCHEMA: &str =
    "player_line_matchup_scenario_comparison.v1";
pub const PLAYER_FORECAST_PROFILE_SCHEMA: &str = "player_forecast_profile.v1";
pub const LINE_CHEMISTRY_EVIDENCE_SCHEMA: &str = "line_chemistry_evidence.v1";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PlayerForecastProfileDimensions {
    pub scoring_creation: Option<f64>,
    pub finishing: Option<f64>,
    pub passing_transition: Option<f64>,
    pub forecheck_retrieval: Option<f64>,
    pub defensive_suppression: Option<f64>,
    pub physical_matchup: Option<f64>,
    pub discipline_puck_security: Option<f64>,
    pub faceoffs: Option<f64>,
    pub power_play: Option<f64>,
    pub penalty_kill: Option<f64>,
}

impl PlayerForecastProfileDimensions {
    fn values(&self) -> [Option<f64>; 10] {
        [
            self.scoring_creation,
            self.finishing,
            self.passing_transition,
            self.forecheck_retrieval,
            self.defensive_suppression,
            self.physical_matchup,
            self.discipline_puck_security,
            self.faceoffs,
            self.power_play,
            self.penalty_kill,
        ]
    }

    fn adjusted(&self, confidence: f64) -> Self {
        let adjust = |value: Option<f64>| value.map(|value| shrink(value, confidence));
        Self {
            scoring_creation: adjust(self.scoring_creation),
            finishing: adjust(self.finishing),
            passing_transition: adjust(self.passing_transition),
            forecheck_retrieval: adjust(self.forecheck_retrieval),
            defensive_suppression: adjust(self.defensive_suppression),
            physical_matchup: adjust(self.physical_matchup),
            discipline_puck_security: adjust(self.discipline_puck_security),
            faceoffs: adjust(self.faceoffs),
            power_play: adjust(self.power_play),
            penalty_kill: adjust(self.penalty_kill),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerForecastProfileInput {
    pub schema: String,
    pub player_id: u32,
    pub team: String,
    pub evidence_cutoff_at: DateTime<Utc>,
    pub games_played: u32,
    pub even_strength_minutes: f64,
    pub observed_shifts: u32,
    /// Point-in-time recency/completeness modifier from 0 through 1.
    pub recency: f64,
    pub dimensions: PlayerForecastProfileDimensions,
    pub source_fingerprints: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerForecastProfileView {
    pub schema: String,
    pub player_id: u32,
    pub display_name: String,
    pub team: String,
    pub evidence_cutoff_at: DateTime<Utc>,
    pub games_played: u32,
    pub even_strength_minutes: f64,
    pub observed_shifts: u32,
    pub component_coverage: f64,
    pub sample_confidence: f64,
    pub raw_overall_score: f64,
    pub reliability_adjusted_score: f64,
    pub adjusted_dimensions: PlayerForecastProfileDimensions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineChemistryEvidenceKind {
    ShiftAdjustedOutcome,
    ShiftDeployment,
    CoarseSameGame,
    SimulatedFit,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineChemistryEvidenceInput {
    pub schema: String,
    /// Exactly two or three stable player IDs. Ordering is immaterial.
    pub player_ids: Vec<u32>,
    pub team: String,
    pub evidence_cutoff_at: DateTime<Utc>,
    pub shared_games: u32,
    pub shared_minutes: f64,
    /// Residual performance from -1 through 1 after the declared baseline.
    /// Deployment-only evidence must leave this absent.
    pub performance_residual: Option<f64>,
    /// Exact shared-ice share from 0 through 1 when available.
    pub deployment_affinity: Option<f64>,
    pub kind: LineChemistryEvidenceKind,
    pub source_fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerLineMatchupTeamInput {
    pub lineup: TeamLineupProjectionView,
    pub lineup_state: TeamGameEvidenceState,
    pub profiles: Vec<PlayerForecastProfileInput>,
    pub chemistry: Vec<LineChemistryEvidenceInput>,
    /// The style used by this team's opponent.
    pub opponent_style: OpponentTacticalStyle,
    /// Probability from 0 through 1 that the manager can execute the intended matchups.
    pub manager_execution_confidence: f64,
    /// Optional projected 5-on-5 shares for forward lines 1 through 4.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forward_line_shares_pct: Option<[f64; 4]>,
    pub source_fingerprints: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerLineMatchupForecastInput {
    pub game_id: u64,
    pub season: u32,
    pub game_date: NaiveDate,
    pub vintage: TeamGameForecastVintage,
    pub forecast_at: DateTime<Utc>,
    pub captured_at: DateTime<Utc>,
    pub away: PlayerLineMatchupTeamInput,
    pub home: PlayerLineMatchupTeamInput,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerLineMatchupScenarioInput {
    pub scenario_id: String,
    pub forecast: PlayerLineMatchupForecastInput,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerLineMatchupScenarioRow {
    pub scenario_id: String,
    pub rank: usize,
    pub focus_five_on_five_score: f64,
    pub focus_matchup_suitability: Option<f64>,
    pub score_delta_vs_baseline: f64,
    pub forecast: PlayerLineMatchupForecastView,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerLineMatchupScenarioComparisonView {
    pub schema: String,
    pub game_id: u64,
    pub focus_team: String,
    pub baseline_scenario_id: String,
    pub rows: Vec<PlayerLineMatchupScenarioRow>,
    pub disclosures: Vec<String>,
    pub source_fingerprints: Vec<String>,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerLineMatchupUnitKind {
    ForwardLine,
    DefensePair,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerLineMatchupUnitView {
    pub kind: PlayerLineMatchupUnitKind,
    pub unit: u8,
    pub player_ids: Vec<u32>,
    pub offense_score: f64,
    pub defense_score: f64,
    pub opponent_style_response: f64,
    /// Shrunk performance effect in lineup-score points.
    pub chemistry_effect: f64,
    pub pair_chemistry_effect: f64,
    pub trio_chemistry_effect: f64,
    /// Descriptive exact shared-ice affinity; never a causal multiplier.
    pub deployment_affinity: Option<f64>,
    pub evidence_confidence: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerLineSpecialTeamsMatchupView {
    pub attacking_team: String,
    pub defending_team: String,
    pub power_play_score: Option<f64>,
    pub penalty_kill_score: Option<f64>,
    pub suitability: Option<f64>,
    pub included_in_five_on_five_matchup: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerLineMatchupTeamView {
    pub team: String,
    pub opponent: String,
    pub profiles: Vec<PlayerForecastProfileView>,
    pub units: Vec<PlayerLineMatchupUnitView>,
    pub profile_coverage: f64,
    pub average_profile_confidence: f64,
    pub offense_score: f64,
    pub defense_score: f64,
    pub opponent_style_response: f64,
    pub chemistry_effect: f64,
    pub pair_chemistry_effect: f64,
    pub trio_chemistry_effect: f64,
    pub manager_execution_confidence: f64,
    pub last_change_adjustment: f64,
    pub five_on_five_matchup_score: f64,
    /// Bounded -1 through 1 value accepted by the existing prediction edge.
    pub matchup_suitability: Option<f64>,
    pub matchup_state: TeamGameEvidenceState,
    pub special_teams: PlayerLineSpecialTeamsMatchupView,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerLineMatchupForecastView {
    pub schema: String,
    pub method: String,
    pub game_id: u64,
    pub season: u32,
    pub game_date: NaiveDate,
    pub vintage: TeamGameForecastVintage,
    pub forecast_at: DateTime<Utc>,
    pub captured_at: DateTime<Utc>,
    pub away: PlayerLineMatchupTeamView,
    pub home: PlayerLineMatchupTeamView,
    pub warnings: Vec<String>,
    pub disclosures: Vec<String>,
    pub source_fingerprints: Vec<String>,
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerLineMatchupFeatureVector {
    pub schema: String,
    pub game_id: u64,
    pub forecast_fingerprint: String,
    /// Signed home-minus-away player/profile matchup signal, normalized near -1..1.
    pub profile_fit_difference: f64,
    pub opponent_style_difference: f64,
    pub pair_chemistry_difference: f64,
    pub trio_chemistry_difference: f64,
    pub manager_execution_difference: f64,
    pub minimum_profile_coverage: f64,
    pub minimum_profile_confidence: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerLineMatchupAblationFeatureVector {
    pub stage: String,
    pub features: PlayerLineMatchupFeatureVector,
}

#[derive(Debug)]
struct TeamEvaluation {
    team: String,
    profiles: Vec<PlayerForecastProfileView>,
    units: Vec<PlayerLineMatchupUnitView>,
    profile_coverage: f64,
    average_profile_confidence: f64,
    offense_score: f64,
    defense_score: f64,
    style_response: f64,
    chemistry_effect: f64,
    pair_chemistry_effect: f64,
    trio_chemistry_effect: f64,
    manager_execution_confidence: f64,
    pp_score: Option<f64>,
    pk_score: Option<f64>,
    warnings: Vec<String>,
}

pub fn build_player_line_matchup_forecast(
    input: PlayerLineMatchupForecastInput,
) -> Result<PlayerLineMatchupForecastView, String> {
    validate_game_input(&input)?;
    let mut away = evaluate_team(&input.away, input.forecast_at)?;
    let mut home = evaluate_team(&input.home, input.forecast_at)?;
    let away_team = away.team.clone();
    let home_team = home.team.clone();
    let away_special = special_teams_matchup(&away_team, &home_team, away.pp_score, home.pk_score);
    let home_special = special_teams_matchup(&home_team, &away_team, home.pp_score, away.pk_score);

    let away_last_change = 0.0;
    let home_last_change = round9(home.manager_execution_confidence * 0.75);
    let away_score = matchup_score(&away, home.defense_score, away_last_change);
    let home_score = matchup_score(&home, away.defense_score, home_last_change);
    let away_suitability = suitability(away.profile_coverage, away_score);
    let home_suitability = suitability(home.profile_coverage, home_score);
    let away_state = evidence_state(input.away.lineup_state, away_suitability);
    let home_state = evidence_state(input.home.lineup_state, home_suitability);

    if input.away.lineup_state == TeamGameEvidenceState::Unavailable {
        away.warnings.push(
            "Away lineup authority is unavailable; matchup suitability is withheld.".to_owned(),
        );
    }
    if input.home.lineup_state == TeamGameEvidenceState::Unavailable {
        home.warnings.push(
            "Home lineup authority is unavailable; matchup suitability is withheld.".to_owned(),
        );
    }

    let mut source_fingerprints = input
        .away
        .source_fingerprints
        .iter()
        .chain(&input.home.source_fingerprints)
        .chain(
            input
                .away
                .profiles
                .iter()
                .flat_map(|row| &row.source_fingerprints),
        )
        .chain(
            input
                .home
                .profiles
                .iter()
                .flat_map(|row| &row.source_fingerprints),
        )
        .chain(
            input
                .away
                .chemistry
                .iter()
                .map(|row| &row.source_fingerprint),
        )
        .chain(
            input
                .home
                .chemistry
                .iter()
                .map(|row| &row.source_fingerprint),
        )
        .cloned()
        .collect::<Vec<_>>();
    source_fingerprints.sort();
    source_fingerprints.dedup();

    let mut warnings = Vec::new();
    warnings.extend(
        away.warnings
            .iter()
            .map(|row| format!("{away_team}: {row}")),
    );
    warnings.extend(
        home.warnings
            .iter()
            .map(|row| format!("{home_team}: {row}")),
    );
    let mut view = PlayerLineMatchupForecastView {
        schema: PLAYER_LINE_MATCHUP_FORECAST_SCHEMA.to_owned(),
        method: PLAYER_LINE_MATCHUP_FORECAST_METHOD.to_owned(),
        game_id: input.game_id,
        season: input.season,
        game_date: input.game_date,
        vintage: input.vintage,
        forecast_at: input.forecast_at,
        captured_at: input.captured_at,
        away: team_view(
            away,
            &home_team,
            away_last_change,
            away_score,
            away_suitability.filter(|_| input.away.lineup_state != TeamGameEvidenceState::Unavailable),
            away_state,
            away_special,
        ),
        home: team_view(
            home,
            &away_team,
            home_last_change,
            home_score,
            home_suitability.filter(|_| input.home.lineup_state != TeamGameEvidenceState::Unavailable),
            home_state,
            home_special,
        ),
        warnings,
        disclosures: vec![
            "Player dimensions shrink toward 50 by games, minutes, shift volume, recency, and component coverage.".to_owned(),
            "Exact shared deployment is affinity only; it contributes zero causal chemistry without a shift-aligned outcome residual.".to_owned(),
            "Special-teams unit suitability is reported separately and excluded from the 5-on-5 matchup feature to prevent double counting.".to_owned(),
            "Availability losses belong to the game edge's lineup features; The Matchup evaluates only the submitted dressed units.".to_owned(),
            "Home last change is a bounded execution adjustment, not an automatic talent bonus.".to_owned(),
        ],
        source_fingerprints,
        fingerprint: String::new(),
    };
    view.fingerprint = fingerprint(&view)?;
    Ok(view)
}

/// Apply the existing Bench manager/deployment primitive to a Matchup side.
/// Schedule fatigue can reduce execution confidence here, but the Bench's
/// separate schedule edge is intentionally not copied.
pub fn apply_bench_game_plan_to_player_line_matchup(
    input: &mut PlayerLineMatchupTeamInput,
    plan: &BenchGamePlanView,
) -> Result<(), String> {
    if plan.schema != BENCH_GAME_PLAN_SCHEMA
        || !plan.team.eq_ignore_ascii_case(&input.lineup.team)
        || !plan.hard_match_confidence.is_finite()
        || !(0.0..=1.0).contains(&plan.hard_match_confidence)
        || plan.forward_assignments.len() != 4
    {
        return Err("Bench plan and player-line matchup side do not align".into());
    }
    let mut shares = [0.0; 4];
    let mut seen = BTreeSet::new();
    for assignment in &plan.forward_assignments {
        if !(1..=4).contains(&assignment.line) || !seen.insert(assignment.line) {
            return Err("Bench plan requires one assignment for each forward line".into());
        }
        let projected = &input.lineup.forward_lines[usize::from(assignment.line - 1)];
        let projected_ids = [
            &projected.left_wing,
            &projected.center,
            &projected.right_wing,
        ]
        .into_iter()
        .flatten()
        .map(|player| player.player_id)
        .collect::<BTreeSet<_>>();
        let assigned_ids = assignment
            .player_ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if projected_ids.len() != 3
            || assigned_ids != projected_ids
            || !assignment.projected_five_on_five_share_pct.is_finite()
            || assignment.projected_five_on_five_share_pct <= 0.0
        {
            return Err("Bench plan assignments must match the submitted dressed lines".into());
        }
        shares[usize::from(assignment.line - 1)] = assignment.projected_five_on_five_share_pct;
    }
    let total = shares.iter().sum::<f64>();
    if !(99.0..=101.0).contains(&total) {
        return Err("Bench forward-line shares must total approximately 100 percent".into());
    }
    input.opponent_style = plan.opponent_style;
    input.manager_execution_confidence = plan.hard_match_confidence;
    input.forward_line_shares_pct = Some(shares);
    Ok(())
}

pub fn compare_player_line_matchup_scenarios(
    focus_team: &str,
    baseline_scenario_id: &str,
    scenarios: Vec<PlayerLineMatchupScenarioInput>,
) -> Result<PlayerLineMatchupScenarioComparisonView, String> {
    let focus_team = focus_team.trim().to_ascii_uppercase();
    if scenarios.len() < 2 || baseline_scenario_id.trim().is_empty() {
        return Err("lineup comparison requires a baseline and at least one alternative".into());
    }
    let mut ids = BTreeSet::new();
    let mut forecasts = Vec::with_capacity(scenarios.len());
    for scenario in scenarios {
        if scenario.scenario_id.trim().is_empty() || !ids.insert(scenario.scenario_id.clone()) {
            return Err("lineup comparison requires unique non-empty scenario IDs".into());
        }
        forecasts.push((
            scenario.scenario_id,
            build_player_line_matchup_forecast(scenario.forecast)?,
        ));
    }
    let first = &forecasts[0].1;
    if focus_team != first.away.team && focus_team != first.home.team {
        return Err("lineup comparison focus team is not in the game".into());
    }
    if forecasts.iter().any(|(_, row)| {
        row.game_id != first.game_id
            || row.season != first.season
            || row.game_date != first.game_date
            || row.forecast_at != first.forecast_at
            || row.captured_at != first.captured_at
            || row.away.team != first.away.team
            || row.home.team != first.home.team
    }) {
        return Err("lineup comparison scenarios must share one frozen game identity".into());
    }
    let baseline = forecasts
        .iter()
        .find(|(id, _)| id == baseline_scenario_id)
        .ok_or_else(|| "lineup comparison baseline scenario is absent".to_owned())?;
    let baseline_score = focus_view(&baseline.1, &focus_team).five_on_five_matchup_score;
    let game_id = first.game_id;
    let mut rows = forecasts
        .into_iter()
        .map(|(scenario_id, forecast)| {
            let focus = focus_view(&forecast, &focus_team);
            PlayerLineMatchupScenarioRow {
                scenario_id,
                rank: 0,
                focus_five_on_five_score: focus.five_on_five_matchup_score,
                focus_matchup_suitability: focus.matchup_suitability,
                score_delta_vs_baseline: round9(focus.five_on_five_matchup_score - baseline_score),
                forecast,
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .focus_five_on_five_score
            .total_cmp(&left.focus_five_on_five_score)
            .then_with(|| left.scenario_id.cmp(&right.scenario_id))
    });
    for (index, row) in rows.iter_mut().enumerate() {
        row.rank = index + 1;
    }
    let mut source_fingerprints = rows
        .iter()
        .flat_map(|row| row.forecast.source_fingerprints.iter().cloned())
        .collect::<Vec<_>>();
    source_fingerprints.sort();
    source_fingerprints.dedup();
    let mut view = PlayerLineMatchupScenarioComparisonView {
        schema: PLAYER_LINE_MATCHUP_SCENARIO_COMPARISON_SCHEMA.to_owned(),
        game_id,
        focus_team,
        baseline_scenario_id: baseline_scenario_id.to_owned(),
        rows,
        disclosures: vec![
            "Scenario rank compares the sealed 5-on-5 Matchup score; probability movement requires passing each scenario through the same registered game-edge model.".to_owned(),
            "Alternatives share an exact game and evidence boundary, so later lineup knowledge cannot leak into the comparison.".to_owned(),
        ],
        source_fingerprints,
        fingerprint: String::new(),
    };
    view.fingerprint = scenario_fingerprint(&view)?;
    Ok(view)
}

pub fn validate_player_line_matchup_scenario_comparison(
    view: &PlayerLineMatchupScenarioComparisonView,
) -> Result<(), String> {
    if view.schema != PLAYER_LINE_MATCHUP_SCENARIO_COMPARISON_SCHEMA
        || view.game_id == 0
        || !valid_team(&view.focus_team)
        || view.rows.len() < 2
        || view.source_fingerprints.is_empty()
        || view
            .source_fingerprints
            .iter()
            .any(|value| !valid_fingerprint(value))
    {
        return Err("invalid player-line matchup scenario comparison identity".into());
    }
    let baseline = view
        .rows
        .iter()
        .find(|row| row.scenario_id == view.baseline_scenario_id)
        .ok_or_else(|| "scenario comparison baseline row is absent".to_owned())?;
    let baseline_score = baseline.focus_five_on_five_score;
    let mut ids = BTreeSet::new();
    for (index, row) in view.rows.iter().enumerate() {
        validate_player_line_matchup_forecast(&row.forecast)?;
        if row.forecast.away.team != view.focus_team && row.forecast.home.team != view.focus_team {
            return Err("scenario comparison focus team is absent from a row".into());
        }
        let focus = focus_view(&row.forecast, &view.focus_team);
        if row.scenario_id.trim().is_empty()
            || !ids.insert(row.scenario_id.clone())
            || row.rank != index + 1
            || row.forecast.game_id != view.game_id
            || (row.focus_five_on_five_score - focus.five_on_five_matchup_score).abs() > 1e-9
            || row.focus_matchup_suitability != focus.matchup_suitability
            || (row.score_delta_vs_baseline - round9(row.focus_five_on_five_score - baseline_score))
                .abs()
                > 1e-9
        {
            return Err("invalid or tampered player-line matchup scenario row".into());
        }
    }
    if scenario_fingerprint(view)? != view.fingerprint {
        return Err("player-line matchup scenario comparison fingerprint mismatch".into());
    }
    Ok(())
}

fn focus_view<'a>(
    forecast: &'a PlayerLineMatchupForecastView,
    focus_team: &str,
) -> &'a PlayerLineMatchupTeamView {
    if forecast.home.team == focus_team {
        &forecast.home
    } else {
        &forecast.away
    }
}

pub fn validate_player_line_matchup_forecast(
    view: &PlayerLineMatchupForecastView,
) -> Result<(), String> {
    if view.schema != PLAYER_LINE_MATCHUP_FORECAST_SCHEMA
        || view.method != PLAYER_LINE_MATCHUP_FORECAST_METHOD
        || view.game_id == 0
        || view.season < 20_000_000
        || view.captured_at > view.forecast_at
        || view.away.team == view.home.team
        || !valid_team(&view.away.team)
        || !valid_team(&view.home.team)
        || view.away.opponent != view.home.team
        || view.home.opponent != view.away.team
        || view.source_fingerprints.is_empty()
        || view
            .source_fingerprints
            .iter()
            .any(|value| !valid_fingerprint(value))
    {
        return Err("invalid player-line matchup identity, method, timing, or sources".to_owned());
    }
    for team in [&view.away, &view.home] {
        let profile_ids = team
            .profiles
            .iter()
            .map(|profile| profile.player_id)
            .collect::<BTreeSet<_>>();
        let expected_units = [(0, 1), (0, 2), (0, 3), (0, 4), (1, 1), (1, 2), (1, 3)]
            .into_iter()
            .collect::<BTreeSet<_>>();
        let actual_units = team
            .units
            .iter()
            .map(|unit| {
                (
                    match unit.kind {
                        PlayerLineMatchupUnitKind::ForwardLine => 0,
                        PlayerLineMatchupUnitKind::DefensePair => 1,
                    },
                    unit.unit,
                )
            })
            .collect::<BTreeSet<_>>();
        let unit_player_ids = team
            .units
            .iter()
            .flat_map(|unit| unit.player_ids.iter().copied())
            .collect::<BTreeSet<_>>();
        if !valid_optional_range(team.matchup_suitability, -1.0, 1.0)
            || !matches!(
                (team.matchup_state, team.matchup_suitability),
                (TeamGameEvidenceState::Modeled, Some(_))
                    | (TeamGameEvidenceState::Unavailable, None)
            )
            || !valid_range(team.profile_coverage, 0.0, 1.0)
            || !valid_range(team.average_profile_confidence, 0.0, 1.0)
            || !valid_range(team.offense_score, 0.0, 100.0)
            || !valid_range(team.defense_score, 0.0, 100.0)
            || !valid_range(team.opponent_style_response, 0.0, 100.0)
            || !valid_range(team.chemistry_effect, -4.0, 4.0)
            || !valid_range(team.pair_chemistry_effect, -4.0, 4.0)
            || !valid_range(team.trio_chemistry_effect, -4.0, 4.0)
            || !valid_range(team.manager_execution_confidence, 0.0, 1.0)
            || !valid_range(team.last_change_adjustment, 0.0, 0.75)
            || !valid_range(team.five_on_five_matchup_score, 0.0, 100.0)
            || team.profiles.iter().any(|profile| {
                profile.schema != PLAYER_FORECAST_PROFILE_SCHEMA
                    || profile.player_id == 0
                    || profile.display_name.trim().is_empty()
                    || profile.evidence_cutoff_at > view.forecast_at
                    || profile.even_strength_minutes < 0.0
                    || !profile.even_strength_minutes.is_finite()
                    || !valid_range(profile.component_coverage, 0.0, 1.0)
                    || !valid_range(profile.sample_confidence, 0.0, 1.0)
                    || !valid_range(profile.raw_overall_score, 0.0, 100.0)
                    || !valid_range(profile.reliability_adjusted_score, 0.0, 100.0)
                    || !valid_dimensions(&profile.adjusted_dimensions)
            })
            || team.units.iter().any(|unit| {
                !valid_range(unit.offense_score, 0.0, 100.0)
                    || !valid_range(unit.defense_score, 0.0, 100.0)
                    || !valid_range(unit.opponent_style_response, 0.0, 100.0)
                    || !valid_range(unit.chemistry_effect, -4.0, 4.0)
                    || !valid_range(unit.pair_chemistry_effect, -4.0, 4.0)
                    || !valid_range(unit.trio_chemistry_effect, -4.0, 4.0)
                    || !valid_optional_range(unit.deployment_affinity, 0.0, 1.0)
                    || !valid_range(unit.evidence_confidence, 0.0, 1.0)
            })
            || !valid_optional_range(team.special_teams.power_play_score, 0.0, 100.0)
            || !valid_optional_range(team.special_teams.penalty_kill_score, 0.0, 100.0)
            || !valid_optional_range(team.special_teams.suitability, -1.0, 1.0)
            || team.special_teams.attacking_team != team.team
            || team.special_teams.defending_team != team.opponent
            || team.special_teams.included_in_five_on_five_matchup
            || team.units.len() != 7
            || team.profiles.len() != 18
        {
            return Err("invalid player-line matchup team scores or unit shape".to_owned());
        }
        if profile_ids.len() != team.profiles.len()
            || team
                .profiles
                .iter()
                .any(|profile| profile.team != team.team)
            || actual_units != expected_units
            || team.units.iter().any(|unit| {
                unit.player_ids.len()
                    != match unit.kind {
                        PlayerLineMatchupUnitKind::ForwardLine => 3,
                        PlayerLineMatchupUnitKind::DefensePair => 2,
                    }
                    || unit.player_ids.iter().collect::<BTreeSet<_>>().len()
                        != unit.player_ids.len()
            })
            || unit_player_ids != profile_ids
        {
            return Err("invalid player-line matchup unit/profile shape".to_owned());
        }
    }
    let away_player_ids = view
        .away
        .profiles
        .iter()
        .map(|profile| profile.player_id)
        .collect::<BTreeSet<_>>();
    if view
        .home
        .profiles
        .iter()
        .any(|profile| away_player_ids.contains(&profile.player_id))
    {
        return Err("player-line matchup teams cannot share player IDs".to_owned());
    }
    if fingerprint(view)? != view.fingerprint {
        return Err("player-line matchup fingerprint mismatch".to_owned());
    }
    Ok(())
}

fn valid_range(value: f64, minimum: f64, maximum: f64) -> bool {
    value.is_finite() && (minimum..=maximum).contains(&value)
}

fn valid_optional_range(value: Option<f64>, minimum: f64, maximum: f64) -> bool {
    value.is_none_or(|value| valid_range(value, minimum, maximum))
}

fn valid_dimensions(dimensions: &PlayerForecastProfileDimensions) -> bool {
    [
        dimensions.scoring_creation,
        dimensions.finishing,
        dimensions.passing_transition,
        dimensions.forecheck_retrieval,
        dimensions.defensive_suppression,
        dimensions.physical_matchup,
        dimensions.discipline_puck_security,
        dimensions.faceoffs,
        dimensions.power_play,
        dimensions.penalty_kill,
    ]
    .into_iter()
    .all(|value| valid_optional_range(value, 0.0, 100.0))
}

/// Project independently ablatable, frozen features from one sealed Matchup.
/// This function performs no outcome join and no coefficient fitting.
pub fn player_line_matchup_feature_vector(
    view: &PlayerLineMatchupForecastView,
) -> Result<PlayerLineMatchupFeatureVector, String> {
    validate_player_line_matchup_forecast(view)?;
    let home_profile_fit = view.home.offense_score - view.away.defense_score;
    let away_profile_fit = view.away.offense_score - view.home.defense_score;
    Ok(PlayerLineMatchupFeatureVector {
        schema: "player_line_matchup_features.v1".to_owned(),
        game_id: view.game_id,
        forecast_fingerprint: view.fingerprint.clone(),
        profile_fit_difference: round9((home_profile_fit - away_profile_fit) / 100.0),
        opponent_style_difference: round9(
            (view.home.opponent_style_response - view.away.opponent_style_response) / 100.0,
        ),
        pair_chemistry_difference: round9(
            (view.home.pair_chemistry_effect - view.away.pair_chemistry_effect) / 4.0,
        ),
        trio_chemistry_difference: round9(
            (view.home.trio_chemistry_effect - view.away.trio_chemistry_effect) / 4.0,
        ),
        manager_execution_difference: round9(
            (view.home.manager_execution_confidence - view.away.manager_execution_confidence)
                + view.home.last_change_adjustment / 3.0,
        ),
        minimum_profile_coverage: view.home.profile_coverage.min(view.away.profile_coverage),
        minimum_profile_confidence: view
            .home
            .average_profile_confidence
            .min(view.away.average_profile_confidence),
    })
}

/// Return the five registered cumulative ablations from one sealed forecast.
/// No coefficients or game outcomes are read here.
pub fn player_line_matchup_ablation_feature_vectors(
    view: &PlayerLineMatchupForecastView,
) -> Result<Vec<PlayerLineMatchupAblationFeatureVector>, String> {
    let full = player_line_matchup_feature_vector(view)?;
    let mut strength = full.clone();
    strength.profile_fit_difference = 0.0;
    strength.opponent_style_difference = 0.0;
    strength.pair_chemistry_difference = 0.0;
    strength.trio_chemistry_difference = 0.0;
    strength.manager_execution_difference = 0.0;
    let mut profiles = strength.clone();
    profiles.profile_fit_difference = full.profile_fit_difference;
    let mut pairs = profiles.clone();
    pairs.pair_chemistry_difference = full.pair_chemistry_difference;
    let mut pairs_trios = pairs.clone();
    pairs_trios.trio_chemistry_difference = full.trio_chemistry_difference;
    Ok([
        ("team_strength_only", strength),
        ("player_profiles", profiles),
        ("profiles_plus_pairs", pairs),
        ("profiles_plus_pairs_trios", pairs_trios),
        ("full_matchup_manager", full),
    ]
    .into_iter()
    .map(|(stage, features)| PlayerLineMatchupAblationFeatureVector {
        stage: stage.to_owned(),
        features,
    })
    .collect())
}

fn validate_game_input(input: &PlayerLineMatchupForecastInput) -> Result<(), String> {
    if input.game_id == 0 || input.season < 20_000_000 || input.captured_at > input.forecast_at {
        return Err(
            "player-line matchup requires a game, NHL season, and capture no later than forecast"
                .to_owned(),
        );
    }
    let away = input.away.lineup.team.trim().to_ascii_uppercase();
    let home = input.home.lineup.team.trim().to_ascii_uppercase();
    if away == home || !valid_team(&away) || !valid_team(&home) {
        return Err("player-line matchup requires two distinct canonical NHL teams".to_owned());
    }
    if input.away.lineup.roster_season != input.season
        || input.home.lineup.roster_season != input.season
    {
        return Err("player-line matchup lineups must match the forecast season".to_owned());
    }
    Ok(())
}

fn evaluate_team(
    input: &PlayerLineMatchupTeamInput,
    forecast_at: DateTime<Utc>,
) -> Result<TeamEvaluation, String> {
    let team = input.lineup.team.trim().to_ascii_uppercase();
    if !input.manager_execution_confidence.is_finite()
        || !(0.0..=1.0).contains(&input.manager_execution_confidence)
    {
        return Err(format!(
            "{team} manager execution confidence must be between 0 and 1"
        ));
    }
    let shares = input
        .forward_line_shares_pct
        .unwrap_or([31.0, 27.0, 23.0, 19.0]);
    if shares
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0)
        || (shares.iter().sum::<f64>() - 100.0).abs() > 0.01
    {
        return Err(format!(
            "{team} forward line shares must be non-negative and sum to 100"
        ));
    }
    let lineup_players = dressed_skaters(&input.lineup)?;
    let lineup_ids = lineup_players.keys().copied().collect::<BTreeSet<_>>();
    let mut profile_inputs = BTreeMap::new();
    for profile in &input.profiles {
        validate_profile(profile, &team, forecast_at)?;
        if !lineup_ids.contains(&profile.player_id)
            || profile_inputs.insert(profile.player_id, profile).is_some()
        {
            return Err(format!(
                "{team} profiles must uniquely reference dressed skaters"
            ));
        }
    }
    validate_chemistry(&input.chemistry, &team, &lineup_ids, forecast_at)?;
    validate_fingerprints(&input.source_fingerprints)?;

    let mut profiles = Vec::with_capacity(lineup_players.len());
    let mut warnings = Vec::new();
    for (player_id, player) in &lineup_players {
        if let Some(profile) = profile_inputs.get(player_id) {
            profiles.push(build_profile_view(profile, player));
        } else {
            warnings.push(format!(
                "player {player_id} has no dated profile and remains neutral"
            ));
            profiles.push(neutral_profile(player, &team, forecast_at));
        }
    }
    profiles.sort_by_key(|row| row.player_id);
    let by_player = profiles
        .iter()
        .map(|row| (row.player_id, row))
        .collect::<BTreeMap<_, _>>();

    let mut units = Vec::with_capacity(7);
    for line in &input.lineup.forward_lines {
        let ids = [&line.left_wing, &line.center, &line.right_wing]
            .into_iter()
            .flatten()
            .map(|player| player.player_id)
            .collect::<Vec<_>>();
        if ids.len() != 3 {
            return Err(format!("{team} requires four complete forward lines"));
        }
        units.push(build_unit(
            PlayerLineMatchupUnitKind::ForwardLine,
            line.line,
            ids,
            &by_player,
            &input.chemistry,
            input.opponent_style,
        ));
    }
    if units.len() != 4 {
        return Err(format!("{team} requires exactly four forward lines"));
    }
    for pair in &input.lineup.defense_pairs {
        let ids = [&pair.left, &pair.right]
            .into_iter()
            .flatten()
            .map(|player| player.player_id)
            .collect::<Vec<_>>();
        if ids.len() != 2 {
            return Err(format!("{team} requires three complete defense pairs"));
        }
        units.push(build_unit(
            PlayerLineMatchupUnitKind::DefensePair,
            pair.pair,
            ids,
            &by_player,
            &input.chemistry,
            input.opponent_style,
        ));
    }
    if units.len() != 7 {
        return Err(format!("{team} requires exactly three defense pairs"));
    }

    let forward = units
        .iter()
        .filter(|unit| unit.kind == PlayerLineMatchupUnitKind::ForwardLine)
        .collect::<Vec<_>>();
    let defense = units
        .iter()
        .filter(|unit| unit.kind == PlayerLineMatchupUnitKind::DefensePair)
        .collect::<Vec<_>>();
    let offense_score = weighted(
        &forward
            .iter()
            .enumerate()
            .map(|(index, unit)| (unit.offense_score, shares[index]))
            .collect::<Vec<_>>(),
    );
    let defense_score = weighted(
        &defense
            .iter()
            .enumerate()
            .map(|(index, unit)| (unit.defense_score, [42.0, 34.0, 24.0][index]))
            .collect::<Vec<_>>(),
    );
    let style_response = weighted(
        &units
            .iter()
            .map(|unit| {
                let weight = match unit.kind {
                    PlayerLineMatchupUnitKind::ForwardLine => shares[usize::from(unit.unit - 1)],
                    PlayerLineMatchupUnitKind::DefensePair => {
                        [42.0, 34.0, 24.0][usize::from(unit.unit - 1)]
                    }
                };
                (unit.opponent_style_response, weight)
            })
            .collect::<Vec<_>>(),
    );
    let chemistry_effect = weighted(
        &units
            .iter()
            .map(|unit| {
                let weight = match unit.kind {
                    PlayerLineMatchupUnitKind::ForwardLine => shares[usize::from(unit.unit - 1)],
                    PlayerLineMatchupUnitKind::DefensePair => {
                        [42.0, 34.0, 24.0][usize::from(unit.unit - 1)]
                    }
                };
                (unit.chemistry_effect, weight)
            })
            .collect::<Vec<_>>(),
    );
    let pair_chemistry_effect = weighted(
        &units
            .iter()
            .map(|unit| {
                let weight = match unit.kind {
                    PlayerLineMatchupUnitKind::ForwardLine => shares[usize::from(unit.unit - 1)],
                    PlayerLineMatchupUnitKind::DefensePair => {
                        [42.0, 34.0, 24.0][usize::from(unit.unit - 1)]
                    }
                };
                (unit.pair_chemistry_effect, weight)
            })
            .collect::<Vec<_>>(),
    );
    let trio_chemistry_effect = weighted(
        &units
            .iter()
            .map(|unit| {
                let weight = match unit.kind {
                    PlayerLineMatchupUnitKind::ForwardLine => shares[usize::from(unit.unit - 1)],
                    PlayerLineMatchupUnitKind::DefensePair => {
                        [42.0, 34.0, 24.0][usize::from(unit.unit - 1)]
                    }
                };
                (unit.trio_chemistry_effect, weight)
            })
            .collect::<Vec<_>>(),
    );
    let covered = input.profiles.len() as f64 / lineup_players.len() as f64;
    let average_confidence = profiles
        .iter()
        .map(|row| row.sample_confidence)
        .sum::<f64>()
        / profiles.len() as f64;
    let pp_score = special_team_profile_score(
        &input.lineup,
        &by_player,
        TeamLineupSpecialTeamsKind::PowerPlay,
    );
    let pk_score = special_team_profile_score(
        &input.lineup,
        &by_player,
        TeamLineupSpecialTeamsKind::PenaltyKill,
    );
    if covered < 0.75 {
        warnings.push(
            "fewer than 75% of dressed skaters have dated profiles; matchup is withheld".to_owned(),
        );
    }
    if input.chemistry.is_empty() {
        warnings.push("no chemistry evidence supplied; unit effects remain neutral".to_owned());
    }
    Ok(TeamEvaluation {
        team,
        profiles,
        units,
        profile_coverage: round9(covered),
        average_profile_confidence: round9(average_confidence),
        offense_score: round9(offense_score),
        defense_score: round9(defense_score),
        style_response: round9(style_response),
        chemistry_effect: round9(chemistry_effect),
        pair_chemistry_effect: round9(pair_chemistry_effect),
        trio_chemistry_effect: round9(trio_chemistry_effect),
        manager_execution_confidence: input.manager_execution_confidence,
        pp_score,
        pk_score,
        warnings,
    })
}

fn validate_profile(
    profile: &PlayerForecastProfileInput,
    team: &str,
    forecast_at: DateTime<Utc>,
) -> Result<(), String> {
    if profile.schema != PLAYER_FORECAST_PROFILE_SCHEMA
        || profile.player_id == 0
        || !profile.team.eq_ignore_ascii_case(team)
        || profile.evidence_cutoff_at > forecast_at
        || profile.games_played == 0
        || !profile.even_strength_minutes.is_finite()
        || profile.even_strength_minutes <= 0.0
        || profile.observed_shifts == 0
        || !profile.recency.is_finite()
        || !(0.0..=1.0).contains(&profile.recency)
    {
        return Err(format!("{team} has an invalid or future player profile"));
    }
    let values = profile.dimensions.values();
    if values.iter().flatten().count() < 4
        || values
            .iter()
            .flatten()
            .any(|value| !value.is_finite() || !(0.0..=100.0).contains(value))
    {
        return Err(format!(
            "{team} player profiles require at least four valid dimensions"
        ));
    }
    validate_fingerprints(&profile.source_fingerprints)
}

fn validate_chemistry(
    rows: &[LineChemistryEvidenceInput],
    team: &str,
    lineup_ids: &BTreeSet<u32>,
    forecast_at: DateTime<Utc>,
) -> Result<(), String> {
    let mut keys = BTreeSet::new();
    for row in rows {
        let mut ids = row.player_ids.clone();
        ids.sort_unstable();
        if row.schema != LINE_CHEMISTRY_EVIDENCE_SCHEMA
            || !row.team.eq_ignore_ascii_case(team)
            || row.evidence_cutoff_at > forecast_at
            || !(2..=3).contains(&ids.len())
            || ids.iter().any(|id| *id == 0 || !lineup_ids.contains(id))
            || ids.windows(2).any(|pair| pair[0] == pair[1])
            || !keys.insert((ids, row.kind))
            || row.shared_games == 0
            || !row.shared_minutes.is_finite()
            || row.shared_minutes <= 0.0
            || row
                .performance_residual
                .is_some_and(|value| !value.is_finite() || !(-1.0..=1.0).contains(&value))
            || row
                .deployment_affinity
                .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
            || !valid_fingerprint(&row.source_fingerprint)
        {
            return Err(format!(
                "{team} has invalid, duplicate, cross-roster, or future chemistry evidence"
            ));
        }
        if row.kind == LineChemistryEvidenceKind::ShiftAdjustedOutcome
            && row.performance_residual.is_none()
        {
            return Err(
                "shift-adjusted outcome chemistry requires a performance residual".to_owned(),
            );
        }
        if row.kind == LineChemistryEvidenceKind::ShiftDeployment
            && row.performance_residual.is_some()
        {
            return Err("deployment-only chemistry cannot carry a performance residual".to_owned());
        }
    }
    Ok(())
}

fn build_profile_view(
    profile: &PlayerForecastProfileInput,
    player: &TeamLineupPlayerView,
) -> PlayerForecastProfileView {
    let coverage = profile.dimensions.values().iter().flatten().count() as f64 / 10.0;
    let games = f64::from(profile.games_played) / (f64::from(profile.games_played) + 20.0);
    let minutes = profile.even_strength_minutes / (profile.even_strength_minutes + 400.0);
    let shifts = f64::from(profile.observed_shifts) / (f64::from(profile.observed_shifts) + 250.0);
    let confidence = (games * minutes * shifts).cbrt() * profile.recency * coverage;
    let raw = mean_present(&profile.dimensions.values());
    PlayerForecastProfileView {
        schema: PLAYER_FORECAST_PROFILE_SCHEMA.to_owned(),
        player_id: profile.player_id,
        display_name: player.display_name.clone(),
        team: profile.team.trim().to_ascii_uppercase(),
        evidence_cutoff_at: profile.evidence_cutoff_at,
        games_played: profile.games_played,
        even_strength_minutes: profile.even_strength_minutes,
        observed_shifts: profile.observed_shifts,
        component_coverage: round9(coverage),
        sample_confidence: round9(confidence),
        raw_overall_score: round9(raw),
        reliability_adjusted_score: round9(shrink(raw, confidence)),
        adjusted_dimensions: profile.dimensions.adjusted(confidence),
    }
}

fn neutral_profile(
    player: &TeamLineupPlayerView,
    team: &str,
    forecast_at: DateTime<Utc>,
) -> PlayerForecastProfileView {
    PlayerForecastProfileView {
        schema: PLAYER_FORECAST_PROFILE_SCHEMA.to_owned(),
        player_id: player.player_id,
        display_name: player.display_name.clone(),
        team: team.to_owned(),
        evidence_cutoff_at: forecast_at,
        games_played: 0,
        even_strength_minutes: 0.0,
        observed_shifts: 0,
        component_coverage: 0.0,
        sample_confidence: 0.0,
        raw_overall_score: 50.0,
        reliability_adjusted_score: 50.0,
        adjusted_dimensions: PlayerForecastProfileDimensions::default(),
    }
}

fn build_unit(
    kind: PlayerLineMatchupUnitKind,
    unit: u8,
    player_ids: Vec<u32>,
    profiles: &BTreeMap<u32, &PlayerForecastProfileView>,
    chemistry: &[LineChemistryEvidenceInput],
    opponent_style: OpponentTacticalStyle,
) -> PlayerLineMatchupUnitView {
    let rows = player_ids
        .iter()
        .filter_map(|id| profiles.get(id).copied())
        .collect::<Vec<_>>();
    let offense = mean(
        &rows
            .iter()
            .map(|row| offense_score(row))
            .collect::<Vec<_>>(),
    );
    let defense = mean(
        &rows
            .iter()
            .map(|row| defense_score(row))
            .collect::<Vec<_>>(),
    );
    let response = mean(
        &rows
            .iter()
            .map(|row| style_response(row, opponent_style))
            .collect::<Vec<_>>(),
    );
    let unit_ids = player_ids.iter().copied().collect::<BTreeSet<_>>();
    let relevant = chemistry
        .iter()
        .filter(|row| row.player_ids.iter().all(|id| unit_ids.contains(id)))
        .collect::<Vec<_>>();
    let pair_chemistry_effect = relevant
        .iter()
        .filter(|row| row.player_ids.len() == 2)
        .map(|row| chemistry_effect(row))
        .sum::<f64>()
        .clamp(-4.0, 4.0);
    let trio_chemistry_effect = relevant
        .iter()
        .filter(|row| row.player_ids.len() == 3)
        .map(|row| chemistry_effect(row))
        .sum::<f64>()
        .clamp(-4.0, 4.0);
    let chemistry_effect = relevant
        .iter()
        .map(|row| chemistry_effect(row))
        .sum::<f64>()
        .clamp(-4.0, 4.0);
    let affinities = relevant
        .iter()
        .filter_map(|row| row.deployment_affinity)
        .collect::<Vec<_>>();
    let evidence_confidence = if relevant.is_empty() {
        0.0
    } else {
        relevant
            .iter()
            .map(|row| chemistry_confidence(row))
            .sum::<f64>()
            / relevant.len() as f64
    };
    PlayerLineMatchupUnitView {
        kind,
        unit,
        player_ids,
        offense_score: round9(offense),
        defense_score: round9(defense),
        opponent_style_response: round9(response),
        chemistry_effect: round9(chemistry_effect),
        pair_chemistry_effect: round9(pair_chemistry_effect),
        trio_chemistry_effect: round9(trio_chemistry_effect),
        deployment_affinity: (!affinities.is_empty()).then(|| round9(mean(&affinities))),
        evidence_confidence: round9(evidence_confidence),
    }
}

fn chemistry_confidence(row: &LineChemistryEvidenceInput) -> f64 {
    let minute_prior = if row.player_ids.len() == 3 {
        600.0
    } else {
        300.0
    };
    let game_prior = if row.player_ids.len() == 3 {
        20.0
    } else {
        10.0
    };
    let minutes = row.shared_minutes / (row.shared_minutes + minute_prior);
    let games = f64::from(row.shared_games) / (f64::from(row.shared_games) + game_prior);
    let authority = match row.kind {
        LineChemistryEvidenceKind::ShiftAdjustedOutcome => 1.0,
        LineChemistryEvidenceKind::ShiftDeployment => 0.6,
        LineChemistryEvidenceKind::CoarseSameGame => 0.25,
        LineChemistryEvidenceKind::SimulatedFit => 0.15,
    };
    (minutes * games).sqrt() * authority
}

fn chemistry_effect(row: &LineChemistryEvidenceInput) -> f64 {
    if row.kind == LineChemistryEvidenceKind::ShiftDeployment {
        return 0.0;
    }
    row.performance_residual.unwrap_or(0.0) * chemistry_confidence(row) * 4.0
}

fn matchup_score(team: &TeamEvaluation, opponent_defense: f64, last_change: f64) -> f64 {
    round9(
        (50.0
            + (team.offense_score - opponent_defense) * 0.28
            + (team.style_response - 50.0) * 0.18
            + team.chemistry_effect
            + (team.manager_execution_confidence - 0.5) * 1.5
            + last_change)
            .clamp(0.0, 100.0),
    )
}

fn suitability(coverage: f64, score: f64) -> Option<f64> {
    (coverage >= 0.75).then(|| round9(((score - 50.0) / 50.0).clamp(-1.0, 1.0)))
}

fn evidence_state(
    lineup_state: TeamGameEvidenceState,
    suitability: Option<f64>,
) -> TeamGameEvidenceState {
    if lineup_state == TeamGameEvidenceState::Unavailable || suitability.is_none() {
        TeamGameEvidenceState::Unavailable
    } else {
        // Even confirmed lines feed a fitted profile/chemistry model.
        TeamGameEvidenceState::Modeled
    }
}

fn special_teams_matchup(
    attacking_team: &str,
    defending_team: &str,
    pp: Option<f64>,
    pk: Option<f64>,
) -> PlayerLineSpecialTeamsMatchupView {
    PlayerLineSpecialTeamsMatchupView {
        attacking_team: attacking_team.to_owned(),
        defending_team: defending_team.to_owned(),
        power_play_score: pp.map(round9),
        penalty_kill_score: pk.map(round9),
        suitability: pp
            .zip(pk)
            .map(|(attack, defense)| round9(((attack - defense) / 50.0).clamp(-1.0, 1.0))),
        included_in_five_on_five_matchup: false,
    }
}

fn team_view(
    team: TeamEvaluation,
    opponent: &str,
    last_change_adjustment: f64,
    score: f64,
    suitability: Option<f64>,
    state: TeamGameEvidenceState,
    special_teams: PlayerLineSpecialTeamsMatchupView,
) -> PlayerLineMatchupTeamView {
    PlayerLineMatchupTeamView {
        team: team.team,
        opponent: opponent.to_owned(),
        profiles: team.profiles,
        units: team.units,
        profile_coverage: team.profile_coverage,
        average_profile_confidence: team.average_profile_confidence,
        offense_score: team.offense_score,
        defense_score: team.defense_score,
        opponent_style_response: team.style_response,
        chemistry_effect: team.chemistry_effect,
        pair_chemistry_effect: team.pair_chemistry_effect,
        trio_chemistry_effect: team.trio_chemistry_effect,
        manager_execution_confidence: round9(team.manager_execution_confidence),
        last_change_adjustment,
        five_on_five_matchup_score: score,
        matchup_suitability: suitability,
        matchup_state: state,
        special_teams,
        warnings: team.warnings,
    }
}

fn dressed_skaters(
    lineup: &TeamLineupProjectionView,
) -> Result<BTreeMap<u32, &TeamLineupPlayerView>, String> {
    let mut players = BTreeMap::new();
    for player in lineup
        .forward_lines
        .iter()
        .flat_map(|line| [&line.left_wing, &line.center, &line.right_wing])
        .chain(
            lineup
                .defense_pairs
                .iter()
                .flat_map(|pair| [&pair.left, &pair.right]),
        )
        .flatten()
    {
        if players.insert(player.player_id, player).is_some() {
            return Err(format!(
                "{} lineup has duplicate dressed skaters",
                lineup.team
            ));
        }
    }
    Ok(players)
}

fn offense_score(profile: &PlayerForecastProfileView) -> f64 {
    mean_present_or_neutral(&[
        profile.adjusted_dimensions.scoring_creation,
        profile.adjusted_dimensions.finishing,
        profile.adjusted_dimensions.passing_transition,
        profile.adjusted_dimensions.forecheck_retrieval,
    ])
}

fn defense_score(profile: &PlayerForecastProfileView) -> f64 {
    mean_present_or_neutral(&[
        profile.adjusted_dimensions.defensive_suppression,
        profile.adjusted_dimensions.physical_matchup,
        profile.adjusted_dimensions.discipline_puck_security,
        profile.adjusted_dimensions.forecheck_retrieval,
    ])
}

fn style_response(profile: &PlayerForecastProfileView, style: OpponentTacticalStyle) -> f64 {
    let d = &profile.adjusted_dimensions;
    match style {
        OpponentTacticalStyle::NorthSouthRush => mean_present_or_neutral(&[
            d.passing_transition,
            d.defensive_suppression,
            d.discipline_puck_security,
        ]),
        OpponentTacticalStyle::EastWestPossession => mean_present_or_neutral(&[
            d.defensive_suppression,
            d.discipline_puck_security,
            d.forecheck_retrieval,
        ]),
        OpponentTacticalStyle::DumpAndChase => mean_present_or_neutral(&[
            d.forecheck_retrieval,
            d.physical_matchup,
            d.defensive_suppression,
        ]),
        OpponentTacticalStyle::HeavyCycle => mean_present_or_neutral(&[
            d.physical_matchup,
            d.defensive_suppression,
            d.forecheck_retrieval,
        ]),
        OpponentTacticalStyle::Counterattack => mean_present_or_neutral(&[
            d.passing_transition,
            d.discipline_puck_security,
            d.defensive_suppression,
        ]),
        OpponentTacticalStyle::Balanced => profile.reliability_adjusted_score,
    }
}

fn special_team_profile_score(
    lineup: &TeamLineupProjectionView,
    profiles: &BTreeMap<u32, &PlayerForecastProfileView>,
    kind: TeamLineupSpecialTeamsKind,
) -> Option<f64> {
    let units = match kind {
        TeamLineupSpecialTeamsKind::PowerPlay => &lineup.special_teams.power_play,
        TeamLineupSpecialTeamsKind::PenaltyKill => &lineup.special_teams.penalty_kill,
    };
    let first = units
        .iter()
        .find(|unit| unit.kind == kind && unit.unit == 1)?;
    let values = first
        .player_ids
        .iter()
        .filter_map(|id| profiles.get(id))
        .filter_map(|profile| match kind {
            TeamLineupSpecialTeamsKind::PowerPlay => profile.adjusted_dimensions.power_play,
            TeamLineupSpecialTeamsKind::PenaltyKill => profile.adjusted_dimensions.penalty_kill,
        })
        .collect::<Vec<_>>();
    (values.len() >= 3).then(|| round9(mean(&values)))
}

fn validate_fingerprints(fingerprints: &[String]) -> Result<(), String> {
    if fingerprints.is_empty() || fingerprints.iter().any(|value| !valid_fingerprint(value)) {
        return Err("player-line matchup requires canonical sha256 source fingerprints".to_owned());
    }
    Ok(())
}

fn valid_fingerprint(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hash| hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn valid_team(team: &str) -> bool {
    CANONICAL_TEAMS.iter().any(|(abbr, _)| *abbr == team)
}

fn mean_present(values: &[Option<f64>]) -> f64 {
    let present = values.iter().flatten().copied().collect::<Vec<_>>();
    mean(&present)
}

fn mean_present_or_neutral(values: &[Option<f64>]) -> f64 {
    let present = values.iter().flatten().copied().collect::<Vec<_>>();
    if present.is_empty() {
        50.0
    } else {
        mean(&present)
    }
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        50.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn weighted(values: &[(f64, f64)]) -> f64 {
    let denominator = values.iter().map(|(_, weight)| weight).sum::<f64>();
    if denominator <= 0.0 {
        50.0
    } else {
        values
            .iter()
            .map(|(value, weight)| value * weight)
            .sum::<f64>()
            / denominator
    }
}

fn shrink(value: f64, confidence: f64) -> f64 {
    round9(50.0 + (value - 50.0) * confidence.clamp(0.0, 1.0))
}

fn round9(value: f64) -> f64 {
    (value * 1_000_000_000.0).round() / 1_000_000_000.0
}

fn fingerprint(view: &PlayerLineMatchupForecastView) -> Result<String, String> {
    let mut material = serde_json::to_value(view).map_err(|error| error.to_string())?;
    if let Some(object) = material.as_object_mut() {
        object.insert(
            "fingerprint".to_owned(),
            serde_json::Value::String(String::new()),
        );
    }
    let bytes = serde_json::to_vec(&material).map_err(|error| error.to_string())?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn scenario_fingerprint(view: &PlayerLineMatchupScenarioComparisonView) -> Result<String, String> {
    let mut material = serde_json::to_value(view).map_err(|error| error.to_string())?;
    if let Some(object) = material.as_object_mut() {
        object.insert(
            "fingerprint".to_owned(),
            serde_json::Value::String(String::new()),
        );
    }
    let bytes = serde_json::to_vec(&material).map_err(|error| error.to_string())?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::model::Position;
    use crate::view_model::team_lineup::{
        build_team_lineup_projection, LineupAssignmentEvidence, TeamLineupPlayerInput,
        TeamLineupRequestedSlot,
    };
    use crate::{EvidenceLabel, TeamCeilingLens};

    fn seal(letter: char) -> String {
        format!("sha256:{}", letter.to_string().repeat(64))
    }

    fn player(
        id: u32,
        team: &str,
        position: Position,
        slot: TeamLineupRequestedSlot,
    ) -> TeamLineupPlayerInput {
        TeamLineupPlayerInput {
            player_id: id,
            display_name: format!("{team} Player {id}"),
            team: team.to_owned(),
            prior_team: None,
            primary_position: position,
            eligible_positions: vec![position],
            headshot_canonical_url: None,
            games_played: 82,
            lens_scores: TeamCeilingLens::ALL
                .into_iter()
                .map(|lens| (lens, Some(60.0)))
                .collect(),
            score_evidence: EvidenceLabel::Estimated,
            power_play_role_score: Some(60.0),
            penalty_kill_role_score: Some(60.0),
            special_teams_evidence: Some(EvidenceLabel::Estimated),
            requested_slot: Some(slot),
            assignment_evidence: LineupAssignmentEvidence::Scenario,
        }
    }

    fn lineup(team: &str, start: u32) -> TeamLineupProjectionView {
        let mut players = Vec::new();
        let mut id = start;
        for line in 1..=4 {
            for (position, requested) in [
                (
                    Position::LeftWing,
                    super::super::team_lineup::LineupForwardPosition::LeftWing,
                ),
                (
                    Position::Center,
                    super::super::team_lineup::LineupForwardPosition::Center,
                ),
                (
                    Position::RightWing,
                    super::super::team_lineup::LineupForwardPosition::RightWing,
                ),
            ] {
                players.push(player(
                    id,
                    team,
                    position,
                    TeamLineupRequestedSlot::Forward {
                        line,
                        position: requested,
                    },
                ));
                id += 1;
            }
        }
        for pair in 1..=3 {
            for right_side in [false, true] {
                players.push(player(
                    id,
                    team,
                    Position::Defense,
                    TeamLineupRequestedSlot::Defense { pair, right_side },
                ));
                id += 1;
            }
        }
        for starter in [true, false] {
            players.push(player(
                id,
                team,
                Position::Goalie,
                TeamLineupRequestedSlot::Goalie { starter },
            ));
            id += 1;
        }
        build_team_lineup_projection(team, 20262027, players).unwrap()
    }

    fn dimensions(value: f64) -> PlayerForecastProfileDimensions {
        PlayerForecastProfileDimensions {
            scoring_creation: Some(value),
            finishing: Some(value),
            passing_transition: Some(value),
            forecheck_retrieval: Some(value),
            defensive_suppression: Some(value),
            physical_matchup: Some(value),
            discipline_puck_security: Some(value),
            faceoffs: Some(value),
            power_play: Some(value),
            penalty_kill: Some(value),
        }
    }

    fn profiles(
        lineup: &TeamLineupProjectionView,
        value: f64,
        games: u32,
    ) -> Vec<PlayerForecastProfileInput> {
        dressed_skaters(lineup)
            .unwrap()
            .keys()
            .map(|id| PlayerForecastProfileInput {
                schema: PLAYER_FORECAST_PROFILE_SCHEMA.to_owned(),
                player_id: *id,
                team: lineup.team.clone(),
                evidence_cutoff_at: Utc.with_ymd_and_hms(2026, 10, 1, 0, 0, 0).unwrap(),
                games_played: games,
                even_strength_minutes: f64::from(games) * 12.0,
                observed_shifts: games * 18,
                recency: 1.0,
                dimensions: dimensions(value),
                source_fingerprints: vec![seal('a')],
            })
            .collect()
    }

    fn team_input(
        lineup: TeamLineupProjectionView,
        value: f64,
        games: u32,
    ) -> PlayerLineMatchupTeamInput {
        PlayerLineMatchupTeamInput {
            profiles: profiles(&lineup, value, games),
            lineup,
            lineup_state: TeamGameEvidenceState::Reported,
            chemistry: Vec::new(),
            opponent_style: OpponentTacticalStyle::Balanced,
            manager_execution_confidence: 0.5,
            forward_line_shares_pct: None,
            source_fingerprints: vec![seal('b')],
        }
    }

    fn input() -> PlayerLineMatchupForecastInput {
        PlayerLineMatchupForecastInput {
            game_id: 2026020001,
            season: 20262027,
            game_date: NaiveDate::from_ymd_opt(2026, 10, 10).unwrap(),
            vintage: TeamGameForecastVintage::GameMorning,
            forecast_at: Utc.with_ymd_and_hms(2026, 10, 10, 16, 0, 0).unwrap(),
            captured_at: Utc.with_ymd_and_hms(2026, 10, 10, 15, 0, 0).unwrap(),
            away: team_input(lineup("SEA", 101), 50.0, 82),
            home: team_input(lineup("NYR", 1), 60.0, 82),
        }
    }

    #[test]
    fn small_samples_shrink_player_profiles_more_than_full_seasons() {
        let lineup = lineup("NYR", 1);
        let player = dressed_skaters(&lineup).unwrap()[&1];
        let small = build_profile_view(&profiles(&lineup, 90.0, 11)[0], player);
        let established = build_profile_view(&profiles(&lineup, 90.0, 82)[0], player);
        assert!(small.reliability_adjusted_score < established.reliability_adjusted_score);
        assert!(small.sample_confidence < established.sample_confidence);
    }

    #[test]
    fn deployment_affinity_never_becomes_causal_chemistry() {
        let mut input = input();
        input.home.chemistry.push(LineChemistryEvidenceInput {
            schema: LINE_CHEMISTRY_EVIDENCE_SCHEMA.to_owned(),
            player_ids: vec![1, 2],
            team: "NYR".to_owned(),
            evidence_cutoff_at: input.captured_at,
            shared_games: 82,
            shared_minutes: 900.0,
            performance_residual: None,
            deployment_affinity: Some(0.9),
            kind: LineChemistryEvidenceKind::ShiftDeployment,
            source_fingerprint: seal('c'),
        });
        let view = build_player_line_matchup_forecast(input).unwrap();
        assert_eq!(view.home.chemistry_effect, 0.0);
        assert_eq!(view.home.units[0].chemistry_effect, 0.0);
        assert_eq!(view.home.units[0].deployment_affinity, Some(0.9));
    }

    #[test]
    fn shift_adjusted_outcome_changes_only_the_containing_unit() {
        let mut input = input();
        input.home.chemistry.push(LineChemistryEvidenceInput {
            schema: LINE_CHEMISTRY_EVIDENCE_SCHEMA.to_owned(),
            player_ids: vec![1, 2, 3],
            team: "NYR".to_owned(),
            evidence_cutoff_at: input.captured_at,
            shared_games: 60,
            shared_minutes: 700.0,
            performance_residual: Some(0.5),
            deployment_affinity: Some(0.8),
            kind: LineChemistryEvidenceKind::ShiftAdjustedOutcome,
            source_fingerprint: seal('c'),
        });
        let view = build_player_line_matchup_forecast(input).unwrap();
        assert!(view.home.units[0].chemistry_effect > 0.0);
        assert_eq!(view.home.units[0].pair_chemistry_effect, 0.0);
        assert!(view.home.units[0].trio_chemistry_effect > 0.0);
        assert_eq!(view.home.units[1].chemistry_effect, 0.0);
        assert!(view.home.chemistry_effect > 0.0);
        let features = player_line_matchup_feature_vector(&view).unwrap();
        assert_eq!(features.pair_chemistry_difference, 0.0);
        assert!(features.trio_chemistry_difference > 0.0);
        assert_eq!(features.forecast_fingerprint, view.fingerprint);
        let ablations = player_line_matchup_ablation_feature_vectors(&view).unwrap();
        assert_eq!(ablations.len(), 5);
        assert_eq!(ablations[0].features.trio_chemistry_difference, 0.0);
        assert_eq!(
            ablations[3].features.trio_chemistry_difference,
            features.trio_chemistry_difference
        );
        assert_eq!(
            ablations[4].features.opponent_style_difference,
            features.opponent_style_difference
        );
    }

    #[test]
    fn deployment_and_adjusted_outcome_can_coexist_for_the_same_pair() {
        let mut input = input();
        input.home.chemistry.extend([
            LineChemistryEvidenceInput {
                schema: LINE_CHEMISTRY_EVIDENCE_SCHEMA.to_owned(),
                player_ids: vec![1, 2],
                team: "NYR".to_owned(),
                evidence_cutoff_at: input.captured_at,
                shared_games: 60,
                shared_minutes: 600.0,
                performance_residual: None,
                deployment_affinity: Some(0.8),
                kind: LineChemistryEvidenceKind::ShiftDeployment,
                source_fingerprint: seal('c'),
            },
            LineChemistryEvidenceInput {
                schema: LINE_CHEMISTRY_EVIDENCE_SCHEMA.to_owned(),
                player_ids: vec![2, 1],
                team: "NYR".to_owned(),
                evidence_cutoff_at: input.captured_at,
                shared_games: 60,
                shared_minutes: 600.0,
                performance_residual: Some(0.2),
                deployment_affinity: None,
                kind: LineChemistryEvidenceKind::ShiftAdjustedOutcome,
                source_fingerprint: seal('d'),
            },
        ]);
        let view = build_player_line_matchup_forecast(input).unwrap();
        assert_eq!(view.home.units[0].deployment_affinity, Some(0.8));
        assert!(view.home.units[0].pair_chemistry_effect > 0.0);
    }

    #[test]
    fn special_teams_are_visible_but_excluded_from_five_on_five() {
        let view = build_player_line_matchup_forecast(input()).unwrap();
        assert!(view.home.special_teams.power_play_score.is_some());
        assert!(view.home.special_teams.penalty_kill_score.is_some());
        assert!(!view.home.special_teams.included_in_five_on_five_matchup);
    }

    #[test]
    fn home_last_change_is_bounded_and_matchup_state_stays_modeled() {
        let mut input = input();
        input.home.manager_execution_confidence = 0.9;
        let view = build_player_line_matchup_forecast(input).unwrap();
        assert_eq!(view.home.last_change_adjustment, 0.675);
        assert_eq!(view.away.last_change_adjustment, 0.0);
        assert_eq!(view.home.matchup_state, TeamGameEvidenceState::Modeled);
        assert!(view.fingerprint.starts_with("sha256:"));
        validate_player_line_matchup_forecast(&view).unwrap();
    }

    #[test]
    fn validator_rejects_tampered_matchup_scores() {
        let mut view = build_player_line_matchup_forecast(input()).unwrap();
        view.home.five_on_five_matchup_score += 1.0;
        assert!(validate_player_line_matchup_forecast(&view)
            .unwrap_err()
            .contains("fingerprint"));
    }

    #[test]
    fn canonical_teams_share_one_profile_and_matchup_method() {
        for (index, (team, _)) in CANONICAL_TEAMS.iter().enumerate() {
            let opponent = if *team == "NYR" { "SEA" } else { "NYR" };
            let mut case = input();
            case.home = team_input(lineup(team, 1_000 + index as u32 * 100), 55.0, 40);
            case.away = team_input(lineup(opponent, 50_000 + index as u32 * 100), 55.0, 40);
            let view = build_player_line_matchup_forecast(case).unwrap();
            assert_eq!(view.method, PLAYER_LINE_MATCHUP_FORECAST_METHOD);
            assert_eq!(view.home.team, *team);
        }
    }

    #[test]
    fn lineup_scenarios_share_one_boundary_and_rank_manager_execution() {
        let baseline = input();
        let mut alternative = baseline.clone();
        alternative.home.manager_execution_confidence = 0.9;
        let comparison = compare_player_line_matchup_scenarios(
            "NYR",
            "baseline",
            vec![
                PlayerLineMatchupScenarioInput {
                    scenario_id: "baseline".to_owned(),
                    forecast: baseline,
                },
                PlayerLineMatchupScenarioInput {
                    scenario_id: "hard-match".to_owned(),
                    forecast: alternative,
                },
            ],
        )
        .unwrap();
        assert_eq!(comparison.rows[0].scenario_id, "hard-match");
        assert!(comparison.rows[0].score_delta_vs_baseline > 0.0);
        assert_eq!(comparison.rows[1].score_delta_vs_baseline, 0.0);
        assert!(comparison.fingerprint.starts_with("sha256:"));
        validate_player_line_matchup_scenario_comparison(&comparison).unwrap();
    }

    #[test]
    fn bench_plan_supplies_manager_confidence_style_and_line_shares() {
        use super::super::management_behavior::{
            BenchForwardAssignmentView, BenchForwardRole, BenchGamePlanView, BenchTacticalResponse,
        };

        let mut case = input();
        let shares = [30.0, 27.0, 23.0, 20.0];
        let forward_assignments = case
            .home
            .lineup
            .forward_lines
            .iter()
            .enumerate()
            .map(|(index, line)| BenchForwardAssignmentView {
                line: index as u8 + 1,
                role: BenchForwardRole::PrimaryScoring,
                player_ids: [&line.left_wing, &line.center, &line.right_wing]
                    .into_iter()
                    .flatten()
                    .map(|player| player.player_id)
                    .collect(),
                suitability_score: 60.0,
                projected_five_on_five_share_pct: shares[index],
                target: None,
                evidence_label: EvidenceLabel::Estimated,
            })
            .collect();
        let plan = BenchGamePlanView {
            schema: BENCH_GAME_PLAN_SCHEMA.to_owned(),
            team: "NYR".to_owned(),
            opponent: "SEA".to_owned(),
            opponent_style: OpponentTacticalStyle::HeavyCycle,
            tactical_response: BenchTacticalResponse::LowZoneSupport,
            manager_profile_id: "nyr-manager".to_owned(),
            hard_match_confidence: 0.82,
            tactical_matchup_edge: 1.2,
            schedule_fatigue_edge: -0.5,
            forward_assignments,
            defense_assignments: Vec::new(),
            warnings: Vec::new(),
            disclosures: Vec::new(),
        };
        apply_bench_game_plan_to_player_line_matchup(&mut case.home, &plan).unwrap();
        assert_eq!(case.home.manager_execution_confidence, 0.82);
        assert_eq!(case.home.forward_line_shares_pct, Some(shares));
        assert_eq!(case.home.opponent_style, OpponentTacticalStyle::HeavyCycle);
    }
}
