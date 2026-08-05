use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::development_calibration::{
    development_cohort_labels, DevelopmentCalibrationCohortRow, DevelopmentCalibrationView,
    DevelopmentPositionGroup, DEVELOPMENT_CALIBRATION_SCHEMA,
};
use super::prospect_arrival_calibration::{
    ProspectArrivalCalibrationView, PROSPECT_ARRIVAL_CALIBRATION_SCHEMA,
};
use super::team_season_forecast::{TeamSeasonScenario, TeamSeasonScenarioEvent};

pub const TEAM_SEASON_SCENARIO_DEVELOPMENT_CALIBRATION_INPUT_SCHEMA: &str =
    "team_season_scenario_development_calibration_input.v1";
pub const TEAM_SEASON_SCENARIO_DEVELOPMENT_CALIBRATION_SCHEMA: &str =
    "team_season_scenario_development_calibration.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonScenarioDevelopmentProfileInput {
    pub event_id: String,
    pub player_id: u32,
    pub position: DevelopmentPositionGroup,
    pub age: Option<u8>,
    /// Completed-season NHL workload entering the forecast season.
    pub prior_games_played: u32,
    /// Completed-season value under the calibration's declared value model.
    pub prior_value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonScenarioDevelopmentCalibrationInput {
    pub schema: String,
    pub scenario: TeamSeasonScenario,
    #[serde(default)]
    pub profiles: Vec<TeamSeasonScenarioDevelopmentProfileInput>,
    /// Separately calibrated arrival authorities for prospects without qualifying NHL workload.
    #[serde(default)]
    pub prospect_arrivals: Vec<ProspectArrivalCalibrationView>,
    /// Select whether a prospect-backed event represents any NHL arrival or an
    /// established NHL role. Missing bindings preserve the v1 arrival behavior.
    #[serde(default)]
    pub prospect_outcomes: Vec<TeamSeasonProspectOutcomeInput>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamSeasonProspectOutcomeKind {
    Arrival,
    EstablishedRole,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamSeasonProspectOutcomeInput {
    pub event_id: String,
    pub outcome: TeamSeasonProspectOutcomeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamSeasonScenarioProbabilityAuthorityStatus {
    Deterministic,
    HistoricalDevelopmentCohort,
    HistoricalProspectArrivalCohort,
    HistoricalProspectEstablishedRoleCohort,
    UncalibratedScenarioAssumption,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonScenarioProbabilityAuthorityRow {
    pub event_id: String,
    pub player: Option<String>,
    pub status: TeamSeasonScenarioProbabilityAuthorityStatus,
    pub configured_probability: f64,
    pub applied_probability: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cohort: Option<DevelopmentCalibrationCohortRow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prospect_arrival: Option<ProspectArrivalCalibrationView>,
    pub basis: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonScenarioDevelopmentCalibrationView {
    pub schema: String,
    pub source_calibration_schema: String,
    pub source_calibration_seasons: Vec<u32>,
    pub source_transitions: usize,
    pub scenario: TeamSeasonScenario,
    pub probability_authority: Vec<TeamSeasonScenarioProbabilityAuthorityRow>,
    pub calibrated_events: usize,
    pub uncalibrated_events: usize,
    pub disclosures: Vec<String>,
}

pub fn calibrate_team_season_scenario_development(
    input: TeamSeasonScenarioDevelopmentCalibrationInput,
    calibration: &DevelopmentCalibrationView,
) -> Result<TeamSeasonScenarioDevelopmentCalibrationView, String> {
    if input.schema != TEAM_SEASON_SCENARIO_DEVELOPMENT_CALIBRATION_INPUT_SCHEMA {
        return Err(format!(
            "scenario development calibration requires {}",
            TEAM_SEASON_SCENARIO_DEVELOPMENT_CALIBRATION_INPUT_SCHEMA
        ));
    }
    if calibration.schema != DEVELOPMENT_CALIBRATION_SCHEMA {
        return Err(format!(
            "scenario development calibration requires {DEVELOPMENT_CALIBRATION_SCHEMA}"
        ));
    }

    let event_ids = input
        .scenario
        .events
        .iter()
        .map(|event| event.id.as_str())
        .collect::<BTreeSet<_>>();
    if event_ids.len() != input.scenario.events.len()
        || input
            .scenario
            .events
            .iter()
            .any(|event| event.id.trim().is_empty())
    {
        return Err("scenario development calibration requires unique event IDs".to_owned());
    }

    let mut profiles = BTreeMap::new();
    for profile in &input.profiles {
        if profile.event_id.trim().is_empty()
            || !event_ids.contains(profile.event_id.as_str())
            || !profile.prior_value.is_finite()
            || profiles.insert(profile.event_id.clone(), profile).is_some()
        {
            return Err(
                "scenario development calibration profile is invalid or duplicate".to_owned(),
            );
        }
    }
    let mut prospect_arrivals = BTreeMap::new();
    for arrival in &input.prospect_arrivals {
        if arrival.schema != PROSPECT_ARRIVAL_CALIBRATION_SCHEMA
            || arrival.event_id.trim().is_empty()
            || !event_ids.contains(arrival.event_id.as_str())
            || profiles.contains_key(&arrival.event_id)
            || prospect_arrivals
                .insert(arrival.event_id.clone(), arrival)
                .is_some()
        {
            return Err(
                "scenario prospect arrival authority is invalid, duplicate, or overlaps an NHL development profile"
                    .to_owned(),
            );
        }
    }
    let mut prospect_outcomes = BTreeMap::new();
    for outcome in &input.prospect_outcomes {
        if outcome.event_id.trim().is_empty()
            || !event_ids.contains(outcome.event_id.as_str())
            || !prospect_arrivals.contains_key(&outcome.event_id)
            || prospect_outcomes
                .insert(outcome.event_id.clone(), outcome.outcome)
                .is_some()
        {
            return Err(
                "scenario prospect outcome binding is invalid, duplicate, or lacks an arrival authority"
                    .to_owned(),
            );
        }
    }

    let mut scenario = input.scenario;
    let mut probability_authority = Vec::with_capacity(scenario.events.len());
    for event in &mut scenario.events {
        if !event.occurrence_probability.is_finite()
            || !(0.0..=1.0).contains(&event.occurrence_probability)
        {
            return Err(format!(
                "scenario event '{}' occurrence probability must be between 0 and 1",
                event.id
            ));
        }
        let configured_probability = event.occurrence_probability;
        let row = if configured_probability == 1.0 {
            TeamSeasonScenarioProbabilityAuthorityRow {
                event_id: event.id.clone(),
                player: event.player.clone(),
                status: TeamSeasonScenarioProbabilityAuthorityStatus::Deterministic,
                configured_probability,
                applied_probability: configured_probability,
                player_id: None,
                cohort: None,
                prospect_arrival: None,
                basis: "Scenario declares this event certain; no occurrence-rate calibration is applied."
                    .to_owned(),
            }
        } else if let Some(profile) = profiles.get(&event.id) {
            let labels = development_cohort_labels(
                profile.age,
                profile.prior_games_played,
                profile.prior_value,
            );
            let cohort = calibration.cohorts.iter().find(|cohort| {
                cohort.position == profile.position
                    && cohort.age_band == labels.0
                    && cohort.experience_band == labels.1
                    && cohort.prior_value_band == labels.2
            });
            if let Some(cohort) = cohort {
                event.occurrence_probability = cohort.calibrated_breakout_rate;
                TeamSeasonScenarioProbabilityAuthorityRow {
                    event_id: event.id.clone(),
                    player: event.player.clone(),
                    status:
                        TeamSeasonScenarioProbabilityAuthorityStatus::HistoricalDevelopmentCohort,
                    configured_probability,
                    applied_probability: event.occurrence_probability,
                    player_id: Some(profile.player_id),
                    cohort: Some(cohort.clone()),
                    prospect_arrival: None,
                    basis: format!(
                        "Applied the shrunken historical breakout rate for {}/{}/{} (n={}).",
                        labels.0, labels.1, labels.2, cohort.sample_size
                    ),
                }
            } else {
                uncalibrated_row(
                    event,
                    configured_probability,
                    Some(profile.player_id),
                    "No matching historical development cohort exists; retained the configured scenario probability.",
                )
            }
        } else if let Some(arrival) = prospect_arrivals.get(&event.id) {
            if event
                .player
                .as_deref()
                .is_none_or(|player| !player.eq_ignore_ascii_case(&arrival.player))
            {
                return Err(format!(
                    "scenario event '{}' does not match prospect arrival player '{}'",
                    event.id, arrival.player
                ));
            }
            let outcome = prospect_outcomes
                .get(&event.id)
                .copied()
                .unwrap_or(TeamSeasonProspectOutcomeKind::Arrival);
            let (status, applied_probability, basis) = match outcome {
                TeamSeasonProspectOutcomeKind::Arrival => (
                    TeamSeasonScenarioProbabilityAuthorityStatus::HistoricalProspectArrivalCohort,
                    arrival
                        .horizon_adjusted_arrival_probability
                        .unwrap_or(arrival.calibrated_arrival_probability),
                    format!(
                        "Applied the shrunken same-position prospect arrival rate from {} nearest historical players{}.",
                        arrival.neighbor_players,
                        horizon_basis(arrival)
                    ),
                ),
                TeamSeasonProspectOutcomeKind::EstablishedRole => {
                    let cumulative_probability = arrival
                        .calibrated_established_probability
                        .ok_or_else(|| {
                            format!(
                                "prospect established-role event '{}' requires a calibrated establishment authority",
                                event.id
                            )
                        })?;
                    let probability = arrival
                        .horizon_adjusted_established_probability
                        .unwrap_or(cumulative_probability);
                    let position_rate = arrival.position_established_rate.ok_or_else(|| {
                        format!(
                            "prospect established-role event '{}' requires a position establishment prior",
                            event.id
                        )
                    })?;
                    let conditional_establishment_rate = arrival
                        .established_given_arrival_rate
                        .ok_or_else(|| {
                            format!(
                                "prospect established-role event '{}' lacks arriving historical neighbors",
                                event.id
                            )
                        })?;
                    if !probability.is_finite()
                        || !(0.0..=arrival
                            .horizon_adjusted_arrival_probability
                            .unwrap_or(arrival.calibrated_arrival_probability))
                            .contains(&probability)
                        || !position_rate.is_finite()
                        || !(0.0..=1.0).contains(&position_rate)
                        || !conditional_establishment_rate.is_finite()
                        || !(0.0..=1.0).contains(&conditional_establishment_rate)
                    {
                        return Err(format!(
                            "prospect established-role event '{}' has an invalid calibrated establishment authority",
                            event.id
                        ));
                    }
                    (
                        TeamSeasonScenarioProbabilityAuthorityStatus::HistoricalProspectEstablishedRoleCohort,
                        probability,
                        format!(
                            "Applied the {:.6} neighbor establishment rate shrunk toward the {:.6} same-position rate, yielding {:.6} cumulatively and {:.6} for the scenario horizon{}; {}/{} ({:.6}) established-given-arrival is retained as descriptive context.",
                            arrival.empirical_established_rate,
                            position_rate,
                            cumulative_probability,
                            probability,
                            horizon_basis(arrival),
                            arrival.neighbor_established_players,
                            arrival.neighbor_arrivals,
                            conditional_establishment_rate
                        ),
                    )
                }
            };
            event.occurrence_probability = applied_probability;
            TeamSeasonScenarioProbabilityAuthorityRow {
                event_id: event.id.clone(),
                player: event.player.clone(),
                status,
                configured_probability,
                applied_probability: event.occurrence_probability,
                player_id: Some(arrival.player_id),
                cohort: None,
                prospect_arrival: Some((*arrival).clone()),
                basis,
            }
        } else {
            uncalibrated_row(
                event,
                configured_probability,
                None,
                "No completed-season NHL development profile or prospect-arrival authority was supplied; retained the configured scenario probability.",
            )
        };
        probability_authority.push(row);
    }

    validate_correlated_probabilities(&scenario.events)?;
    let calibrated_events = probability_authority
        .iter()
        .filter(|row| {
            matches!(
                row.status,
                TeamSeasonScenarioProbabilityAuthorityStatus::HistoricalDevelopmentCohort
                    | TeamSeasonScenarioProbabilityAuthorityStatus::HistoricalProspectArrivalCohort
                    | TeamSeasonScenarioProbabilityAuthorityStatus::HistoricalProspectEstablishedRoleCohort
            )
        })
        .count();
    let uncalibrated_events = probability_authority
        .iter()
        .filter(|row| {
            row.status
                == TeamSeasonScenarioProbabilityAuthorityStatus::UncalibratedScenarioAssumption
        })
        .count();

    Ok(TeamSeasonScenarioDevelopmentCalibrationView {
        schema: TEAM_SEASON_SCENARIO_DEVELOPMENT_CALIBRATION_SCHEMA.to_owned(),
        source_calibration_schema: calibration.schema.clone(),
        source_calibration_seasons: calibration.seasons.clone(),
        source_transitions: calibration.transitions,
        scenario,
        probability_authority,
        calibrated_events,
        uncalibrated_events,
        disclosures: vec![
            "Historical development rates calibrate event occurrence only; authored scenario strength deltas remain unchanged."
                .to_owned(),
            "Development rates are conditional on the player reaching the calibration's target-season workload gate and are not NHL-arrival probabilities."
                .to_owned(),
            "Events without a matching completed-season NHL profile or prospect-arrival authority remain visibly uncalibrated scenario assumptions."
                .to_owned(),
            "Prospect arrival authorities are separate historical base-rate estimates and never reuse workload-conditional NHL development rates."
                .to_owned(),
            "Established-role prospect events use a separately shrunken unconditional establishment probability; established-given-arrival remains descriptive and arrival and impact are never treated as equivalent."
                .to_owned(),
        ],
    })
}

fn horizon_basis(arrival: &ProspectArrivalCalibrationView) -> String {
    match (
        arrival.source_horizon_seasons,
        arrival.forecast_horizon_seasons,
    ) {
        (Some(source), Some(forecast)) => {
            format!(" after constant-hazard adjustment from {source} to {forecast} season(s)")
        }
        _ => " using legacy unadjusted horizon authority".to_owned(),
    }
}

fn uncalibrated_row(
    event: &TeamSeasonScenarioEvent,
    configured_probability: f64,
    player_id: Option<u32>,
    basis: &str,
) -> TeamSeasonScenarioProbabilityAuthorityRow {
    TeamSeasonScenarioProbabilityAuthorityRow {
        event_id: event.id.clone(),
        player: event.player.clone(),
        status: TeamSeasonScenarioProbabilityAuthorityStatus::UncalibratedScenarioAssumption,
        configured_probability,
        applied_probability: configured_probability,
        player_id,
        cohort: None,
        prospect_arrival: None,
        basis: basis.to_owned(),
    }
}

fn validate_correlated_probabilities(events: &[TeamSeasonScenarioEvent]) -> Result<(), String> {
    let mut probabilities = BTreeMap::<&str, f64>::new();
    for event in events {
        let Some(key) = event.correlation_key.as_deref() else {
            continue;
        };
        if key.trim().is_empty() {
            return Err(format!(
                "scenario event '{}' correlation key cannot be empty",
                event.id
            ));
        }
        if let Some(existing) = probabilities.insert(key, event.occurrence_probability) {
            if (existing - event.occurrence_probability).abs() > f64::EPSILON {
                return Err(format!(
                    "correlated scenario events using '{key}' must share an applied probability"
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::view_model::development_calibration::{
        DevelopmentCalibrationConfig, DevelopmentCalibrationRateRow, DevelopmentValueModel,
    };
    use crate::view_model::prospect_arrival_calibration::ProspectArrivalCalibrationConfig;
    use crate::view_model::team_season_forecast::TeamSeasonScenarioEventKind;

    fn calibration() -> DevelopmentCalibrationView {
        DevelopmentCalibrationView {
            schema: DEVELOPMENT_CALIBRATION_SCHEMA.to_owned(),
            seasons: vec![20242025, 20252026],
            transitions: 140,
            config: DevelopmentCalibrationConfig {
                value_model: DevelopmentValueModel::PositionEraNormalizedMultilens,
                ..DevelopmentCalibrationConfig::default()
            },
            global: DevelopmentCalibrationRateRow {
                sample_size: 140,
                breakout_count: 40,
                downturn_count: 20,
                stable_count: 80,
                breakout_rate: 40.0 / 140.0,
                downturn_rate: 20.0 / 140.0,
                median_breakout_strength_delta: Some(3.4),
                median_downturn_strength_delta: Some(-3.0),
            },
            cohorts: vec![DevelopmentCalibrationCohortRow {
                position: DevelopmentPositionGroup::Forward,
                age_band: "22_or_younger".to_owned(),
                experience_band: "established".to_owned(),
                prior_value_band: "below_average".to_owned(),
                sample_size: 140,
                breakout_count: 54,
                downturn_count: 14,
                empirical_breakout_rate: 54.0 / 140.0,
                empirical_downturn_rate: 0.1,
                calibrated_breakout_rate: 0.401979,
                calibrated_downturn_rate: 0.12,
                median_breakout_strength_delta: Some(3.435806),
                median_downturn_strength_delta: Some(-3.1),
            }],
            latest_season_players: vec![],
            largest_breakouts: vec![],
            largest_downturns: vec![],
            disclosures: vec![],
        }
    }

    fn event(id: &str, player: &str, probability: f64) -> TeamSeasonScenarioEvent {
        TeamSeasonScenarioEvent {
            id: id.to_owned(),
            kind: TeamSeasonScenarioEventKind::Form,
            team: "NYR".to_owned(),
            player: Some(player.to_owned()),
            effective_date: NaiveDate::from_ymd_opt(2026, 9, 29).unwrap(),
            end_date: None,
            strength_delta: 4.0,
            occurrence_probability: probability,
            correlation_key: None,
            label: format!("{player} breakout"),
        }
    }

    #[test]
    fn calibrates_matching_events_and_exposes_unmatched_assumptions() {
        let input = TeamSeasonScenarioDevelopmentCalibrationInput {
            schema: TEAM_SEASON_SCENARIO_DEVELOPMENT_CALIBRATION_INPUT_SCHEMA.to_owned(),
            scenario: TeamSeasonScenario {
                name: "portfolio".to_owned(),
                trade_deadline: None,
                events: vec![
                    event("wright", "Shane Wright", 0.40),
                    event("smits", "Alberts Smits", 0.20),
                ],
                adaptive_lineup_policies: vec![],
                opening_roster_policies: vec![],
            },
            profiles: vec![TeamSeasonScenarioDevelopmentProfileInput {
                event_id: "wright".to_owned(),
                player_id: 8_483_524,
                position: DevelopmentPositionGroup::Forward,
                age: Some(21),
                prior_games_played: 74,
                prior_value: 47.0167,
            }],
            prospect_arrivals: vec![],
            prospect_outcomes: vec![],
        };

        let view = calibrate_team_season_scenario_development(input, &calibration()).unwrap();
        assert_eq!(view.calibrated_events, 1);
        assert_eq!(view.uncalibrated_events, 1);
        assert!((view.scenario.events[0].occurrence_probability - 0.401979).abs() < 1e-12);
        assert_eq!(view.scenario.events[1].occurrence_probability, 0.20);
        assert_eq!(
            view.probability_authority[1].status,
            TeamSeasonScenarioProbabilityAuthorityStatus::UncalibratedScenarioAssumption
        );
    }

    #[test]
    fn applies_a_separate_prospect_arrival_authority() {
        let arrival = ProspectArrivalCalibrationView {
            schema: PROSPECT_ARRIVAL_CALIBRATION_SCHEMA.to_owned(),
            source_schema: "prospect_conversion_board.v2".to_owned(),
            source_baseline_seasons: vec![20222023],
            source_through_season: 20252026,
            event_id: "smits".to_owned(),
            player_id: 8_485_957,
            player: "Alberts Smits".to_owned(),
            position_group: "D".to_owned(),
            observed_signal_score: 61.0,
            forecast_season: 20262027,
            candidate_players: 80,
            neighbor_players: 50,
            neighbor_signal_min: 50.0,
            neighbor_signal_max: 72.0,
            mean_signal_distance: 5.0,
            neighbor_arrivals: 18,
            neighbor_established_players: 8,
            empirical_arrival_rate: 0.36,
            position_arrival_rate: 0.30,
            calibrated_arrival_probability: 0.342857,
            empirical_established_rate: 0.16,
            position_established_rate: Some(0.12),
            calibrated_established_probability: Some(0.148571),
            established_given_arrival_rate: Some(8.0 / 18.0),
            source_horizon_seasons: None,
            forecast_horizon_seasons: None,
            horizon_adjusted_arrival_probability: None,
            horizon_adjusted_established_probability: None,
            config: ProspectArrivalCalibrationConfig::default(),
            disclosures: vec![],
        };
        let input = TeamSeasonScenarioDevelopmentCalibrationInput {
            schema: TEAM_SEASON_SCENARIO_DEVELOPMENT_CALIBRATION_INPUT_SCHEMA.to_owned(),
            scenario: TeamSeasonScenario {
                name: "prospect arrival".to_owned(),
                trade_deadline: None,
                events: vec![event("smits", "Alberts Smits", 0.20)],
                adaptive_lineup_policies: vec![],
                opening_roster_policies: vec![],
            },
            profiles: vec![],
            prospect_arrivals: vec![arrival],
            prospect_outcomes: vec![],
        };

        let view = calibrate_team_season_scenario_development(input, &calibration()).unwrap();
        assert_eq!(view.calibrated_events, 1);
        assert_eq!(view.uncalibrated_events, 0);
        assert!((view.scenario.events[0].occurrence_probability - 0.342857).abs() < 1e-12);
        assert_eq!(
            view.probability_authority[0].status,
            TeamSeasonScenarioProbabilityAuthorityStatus::HistoricalProspectArrivalCohort
        );
        assert!(view.probability_authority[0].prospect_arrival.is_some());
    }

    #[test]
    fn established_role_uses_its_own_shrunken_probability() {
        let arrival = ProspectArrivalCalibrationView {
            schema: PROSPECT_ARRIVAL_CALIBRATION_SCHEMA.to_owned(),
            source_schema: "prospect_conversion_board.v2".to_owned(),
            source_baseline_seasons: vec![20222023],
            source_through_season: 20252026,
            event_id: "smits".to_owned(),
            player_id: 8_485_957,
            player: "Alberts Smits".to_owned(),
            position_group: "D".to_owned(),
            observed_signal_score: 29.06,
            forecast_season: 20262027,
            candidate_players: 164,
            neighbor_players: 50,
            neighbor_signal_min: 19.68,
            neighbor_signal_max: 38.73,
            mean_signal_distance: 4.9478,
            neighbor_arrivals: 31,
            neighbor_established_players: 11,
            empirical_arrival_rate: 0.62,
            position_arrival_rate: 0.457317,
            calibrated_arrival_probability: 0.573519,
            empirical_established_rate: 0.22,
            position_established_rate: Some(0.15),
            calibrated_established_probability: Some(0.20),
            established_given_arrival_rate: Some(11.0 / 31.0),
            source_horizon_seasons: Some(3),
            forecast_horizon_seasons: Some(1),
            horizon_adjusted_arrival_probability: Some(0.247280),
            horizon_adjusted_established_probability: Some(0.076015),
            config: ProspectArrivalCalibrationConfig::default(),
            disclosures: vec![],
        };
        let input = TeamSeasonScenarioDevelopmentCalibrationInput {
            schema: TEAM_SEASON_SCENARIO_DEVELOPMENT_CALIBRATION_INPUT_SCHEMA.to_owned(),
            scenario: TeamSeasonScenario {
                name: "prospect established role".to_owned(),
                trade_deadline: None,
                events: vec![event("smits", "Alberts Smits", 0.20)],
                adaptive_lineup_policies: vec![],
                opening_roster_policies: vec![],
            },
            profiles: vec![],
            prospect_arrivals: vec![arrival],
            prospect_outcomes: vec![TeamSeasonProspectOutcomeInput {
                event_id: "smits".to_owned(),
                outcome: TeamSeasonProspectOutcomeKind::EstablishedRole,
            }],
        };

        let view = calibrate_team_season_scenario_development(input, &calibration()).unwrap();
        assert_eq!(view.scenario.events[0].occurrence_probability, 0.076015);
        assert_eq!(
            view.probability_authority[0].status,
            TeamSeasonScenarioProbabilityAuthorityStatus::HistoricalProspectEstablishedRoleCohort
        );
        assert!(view.probability_authority[0]
            .basis
            .contains("11/31 (0.354839)"));
    }
}
