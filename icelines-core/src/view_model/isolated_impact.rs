//! Paired, same-seed isolation of individual IceCast scenario events.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::{
    compare_team_season_forecast_scenarios, simulate_team_season_forecast_as_of_with_scenario,
    simulate_team_season_forecast_with_scenario, TeamGameForecastView, TeamSeasonForecastRow,
    TeamSeasonForecastView, TeamSeasonScenario, TeamSeasonScenarioEvent,
    TeamSeasonScenarioImpactRow, TeamSeasonSimulationConfig,
};

pub const ISOLATED_IMPACT_SCHEMA: &str = "isolated_scenario_impact.v1";
pub const ISOLATED_IMPACT_METHOD: &str = "paired_same_seed_one_event.v1";
pub const ISOLATED_IMPACT_AS_OF_METHOD: &str = "paired_same_seed_one_event_as_of.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IsolatedImpactBaselineRow {
    pub team: String,
    pub average_points: f64,
    pub playoff_probability: f64,
    pub second_round_probability: f64,
    pub conference_final_probability: f64,
    pub stanley_cup_final_probability: f64,
    pub stanley_cup_probability: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IsolatedEventImpactRow {
    pub event_id: String,
    pub team: String,
    pub player: Option<String>,
    pub label: String,
    pub occurrence_probability: f64,
    pub correlation_key: Option<String>,
    /// Model input, not standings points.
    pub raw_team_strength_delta: f64,
    /// Conditional impact when this event occurs with all unrelated events disabled.
    pub conditional_impact: TeamSeasonScenarioImpactRow,
    pub conditional_outcome: IsolatedImpactBaselineRow,
    pub isolated_scenario_fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForcedCeilingPathRow {
    pub team: String,
    /// Sum of positive event strength inputs before rounding.
    pub raw_team_strength_delta_sum: f64,
    pub display_label: String,
    pub event_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IsolatedImpactView {
    pub schema: String,
    pub method: String,
    pub season: u32,
    #[serde(default)]
    pub as_of_date: Option<NaiveDate>,
    pub trials: u32,
    pub seed: u64,
    pub input_fingerprint: String,
    pub scenario_fingerprint: String,
    pub baseline: Vec<IsolatedImpactBaselineRow>,
    pub isolated_events: Vec<IsolatedEventImpactRow>,
    pub naturally_sampled_impacts: Vec<TeamSeasonScenarioImpactRow>,
    pub forced_ceiling_paths: Vec<ForcedCeilingPathRow>,
    pub forced_ceiling_impacts: Vec<TeamSeasonScenarioImpactRow>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum IsolatedImpactError {
    #[error("serialize isolated-impact input: {0}")]
    Serialize(String),
    #[error("simulate isolated-impact scenario: {0}")]
    Simulation(String),
    #[error("scenario comparison failed: {0}")]
    Comparison(String),
    #[error("isolated event {event_id} produced no impact row for team {team}")]
    MissingTeamImpact { event_id: String, team: String },
}

#[derive(Debug, Default)]
pub struct IsolatedImpactCache {
    entries: BTreeMap<String, IsolatedImpactView>,
}

impl IsolatedImpactCache {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

pub fn build_isolated_scenario_impact(
    forecast: &TeamGameForecastView,
    scenario: &TeamSeasonScenario,
    config: TeamSeasonSimulationConfig,
) -> Result<IsolatedImpactView, IsolatedImpactError> {
    let input_fingerprint = input_fingerprint(forecast, scenario, config)?;
    build_with_fingerprint(forecast, scenario, config, input_fingerprint, None)
}

pub fn build_isolated_scenario_impact_as_of(
    forecast: &TeamGameForecastView,
    scenario: &TeamSeasonScenario,
    config: TeamSeasonSimulationConfig,
    as_of_date: NaiveDate,
) -> Result<IsolatedImpactView, IsolatedImpactError> {
    let input_fingerprint = fingerprint(&serde_json::json!({
        "method": ISOLATED_IMPACT_AS_OF_METHOD,
        "forecast": forecast,
        "scenario": scenario,
        "config": config,
        "as_of_date": as_of_date,
    }))?;
    build_with_fingerprint(
        forecast,
        scenario,
        config,
        input_fingerprint,
        Some(as_of_date),
    )
}

pub fn build_isolated_scenario_impact_cached(
    forecast: &TeamGameForecastView,
    scenario: &TeamSeasonScenario,
    config: TeamSeasonSimulationConfig,
    cache: &mut IsolatedImpactCache,
) -> Result<IsolatedImpactView, IsolatedImpactError> {
    let fingerprint = input_fingerprint(forecast, scenario, config)?;
    if let Some(view) = cache.entries.get(&fingerprint) {
        return Ok(view.clone());
    }
    let view = build_with_fingerprint(forecast, scenario, config, fingerprint.clone(), None)?;
    cache.entries.insert(fingerprint, view.clone());
    Ok(view)
}

fn build_with_fingerprint(
    forecast: &TeamGameForecastView,
    scenario: &TeamSeasonScenario,
    config: TeamSeasonSimulationConfig,
    input_fingerprint: String,
    as_of_date: Option<NaiveDate>,
) -> Result<IsolatedImpactView, IsolatedImpactError> {
    let baseline = simulate_at_boundary(forecast, config, None, as_of_date)?;
    let natural = simulate_at_boundary(forecast, config, Some(scenario.clone()), as_of_date)?;
    let naturally_sampled_impacts = compare_team_season_forecast_scenarios(&baseline, &natural)
        .map_err(IsolatedImpactError::Comparison)?;

    let mut events = scenario.events.clone();
    events.sort_by(|a, b| a.id.cmp(&b.id));
    let mut isolated_events = Vec::with_capacity(events.len());
    for event in &events {
        let isolated_scenario = TeamSeasonScenario {
            name: format!("Isolated: {}", event.label),
            trade_deadline: scenario.trade_deadline,
            events: vec![forced_event(event)],
            adaptive_lineup_policies: Vec::new(),
            opening_roster_policies: Vec::new(),
        };
        let isolated_fingerprint = fingerprint(&isolated_scenario)?;
        let isolated = simulate_at_boundary(forecast, config, Some(isolated_scenario), as_of_date)?;
        let impacts = compare_team_season_forecast_scenarios(&baseline, &isolated)
            .map_err(IsolatedImpactError::Comparison)?;
        let conditional_outcome = isolated
            .teams
            .iter()
            .find(|team| team.team == event.team)
            .map(baseline_row)
            .ok_or_else(|| IsolatedImpactError::MissingTeamImpact {
                event_id: event.id.clone(),
                team: event.team.clone(),
            })?;
        let conditional_impact = impacts
            .into_iter()
            .find(|impact| impact.team == event.team)
            .ok_or_else(|| IsolatedImpactError::MissingTeamImpact {
                event_id: event.id.clone(),
                team: event.team.clone(),
            })?;
        isolated_events.push(IsolatedEventImpactRow {
            event_id: event.id.clone(),
            team: event.team.clone(),
            player: event.player.clone(),
            label: event.label.clone(),
            occurrence_probability: event.occurrence_probability,
            correlation_key: event.correlation_key.clone(),
            raw_team_strength_delta: event.strength_delta,
            conditional_impact,
            conditional_outcome,
            isolated_scenario_fingerprint: isolated_fingerprint,
        });
    }

    let positive_events = events
        .iter()
        .filter(|event| event.strength_delta > 0.0)
        .map(forced_event)
        .collect::<Vec<_>>();
    let forced_scenario = TeamSeasonScenario {
        name: format!("{} — forced positive-event ceiling", scenario.name),
        trade_deadline: scenario.trade_deadline,
        events: positive_events.clone(),
        adaptive_lineup_policies: Vec::new(),
        opening_roster_policies: Vec::new(),
    };
    let forced = simulate_at_boundary(forecast, config, Some(forced_scenario), as_of_date)?;
    let forced_ceiling_impacts = compare_team_season_forecast_scenarios(&baseline, &forced)
        .map_err(IsolatedImpactError::Comparison)?;
    let forced_ceiling_paths = forced_paths(&positive_events);

    Ok(IsolatedImpactView {
        schema: ISOLATED_IMPACT_SCHEMA.to_string(),
        method: if as_of_date.is_some() {
            ISOLATED_IMPACT_AS_OF_METHOD
        } else {
            ISOLATED_IMPACT_METHOD
        }
        .to_string(),
        season: forecast.season,
        as_of_date,
        trials: config.trials,
        seed: config.seed,
        input_fingerprint,
        scenario_fingerprint: fingerprint(scenario)?,
        baseline: baseline.teams.iter().map(baseline_row).collect(),
        isolated_events,
        naturally_sampled_impacts,
        forced_ceiling_paths,
        forced_ceiling_impacts,
        disclosures: {
            let mut disclosures = vec![
            "Each isolated impact forces exactly one event to occur, disables every unrelated event, and compares paired runs with identical schedule, seed, trials, and model parameters.".to_string(),
            "Raw team-strength delta is a model input, not standings points; outcome deltas come only from the paired simulation.".to_string(),
            "The forced ceiling includes positive-strength events only. The naturally sampled run retains original marginal probabilities and explicit correlation keys.".to_string(),
            "Display path labels round only the summed strength input; raw sums remain authoritative.".to_string(),
            ];
            if let Some(cutoff) = as_of_date {
                disclosures.push(format!(
                    "Every paired attribution run fixes the identical final results through {cutoff} and samples only later games."
                ));
            }
            disclosures
        },
    })
}

fn simulate_at_boundary(
    forecast: &TeamGameForecastView,
    config: TeamSeasonSimulationConfig,
    scenario: Option<TeamSeasonScenario>,
    as_of_date: Option<NaiveDate>,
) -> Result<TeamSeasonForecastView, IsolatedImpactError> {
    match as_of_date {
        Some(cutoff) => {
            simulate_team_season_forecast_as_of_with_scenario(forecast, config, scenario, cutoff)
        }
        None => simulate_team_season_forecast_with_scenario(forecast, config, scenario),
    }
    .map_err(IsolatedImpactError::Simulation)
}

fn forced_event(event: &TeamSeasonScenarioEvent) -> TeamSeasonScenarioEvent {
    let mut event = event.clone();
    event.occurrence_probability = 1.0;
    event
}

fn forced_paths(events: &[TeamSeasonScenarioEvent]) -> Vec<ForcedCeilingPathRow> {
    let mut by_team = BTreeMap::<String, (f64, Vec<String>)>::new();
    for event in events {
        let row = by_team.entry(event.team.clone()).or_default();
        row.0 += event.strength_delta;
        row.1.push(event.id.clone());
    }
    by_team
        .into_iter()
        .map(|(team, (raw_team_strength_delta_sum, mut event_ids))| {
            event_ids.sort();
            ForcedCeilingPathRow {
                team,
                display_label: format!("{raw_team_strength_delta_sum:+.0} Path"),
                raw_team_strength_delta_sum,
                event_ids,
            }
        })
        .collect()
}

fn baseline_row(team: &TeamSeasonForecastRow) -> IsolatedImpactBaselineRow {
    IsolatedImpactBaselineRow {
        team: team.team.clone(),
        average_points: team.average_points,
        playoff_probability: team.playoff_probability,
        second_round_probability: team.second_round_probability,
        conference_final_probability: team.conference_final_probability,
        stanley_cup_final_probability: team.stanley_cup_final_probability,
        stanley_cup_probability: team.stanley_cup_probability,
    }
}

fn input_fingerprint(
    forecast: &TeamGameForecastView,
    scenario: &TeamSeasonScenario,
    config: TeamSeasonSimulationConfig,
) -> Result<String, IsolatedImpactError> {
    fingerprint(&serde_json::json!({
        "method": ISOLATED_IMPACT_METHOD,
        "forecast": forecast,
        "scenario": scenario,
        "config": config,
    }))
}

fn fingerprint(value: &impl Serialize) -> Result<String, IsolatedImpactError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| IsolatedImpactError::Serialize(error.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::view_model::{
        build_team_game_forecast, TeamForecastGameInput, TeamForecastParameters,
        TeamForecastStrengthInput, TeamSeasonScenarioEventKind,
    };

    fn forecast() -> TeamGameForecastView {
        let teams = [
            "BOS", "BUF", "DET", "FLA", "MTL", "OTT", "TBL", "TOR", "CAR", "CBJ", "NJD", "NYI",
            "NYR", "PHI", "PIT", "WSH", "CHI", "COL", "DAL", "MIN", "NSH", "STL", "UTA", "WPG",
            "ANA", "CGY", "EDM", "LAK", "SEA", "SJS", "VAN", "VGK",
        ];
        let games = (0..84)
            .flat_map(|round| {
                teams.chunks_exact(2).enumerate().map(move |(pair, chunk)| {
                    let reverse = round % 2 == 1;
                    TeamForecastGameInput {
                        game_id: (round * 16 + pair) as u64,
                        date: NaiveDate::from_ymd_opt(2026, 9, 29).unwrap()
                            + chrono::Duration::days(round as i64),
                        away_team: chunk[usize::from(reverse)].to_string(),
                        home_team: chunk[usize::from(!reverse)].to_string(),
                        away_score: None,
                        home_score: None,
                        final_result: false,
                        last_period: None,
                    }
                })
            })
            .collect();
        let strengths = teams
            .iter()
            .map(|team| TeamForecastStrengthInput {
                team: (*team).to_string(),
                strength: 50.0,
            })
            .collect();
        build_team_game_forecast(
            20262027,
            games,
            strengths,
            TeamForecastParameters::default(),
            None,
            None,
        )
        .unwrap()
    }

    fn scenario() -> TeamSeasonScenario {
        let date = NaiveDate::from_ymd_opt(2026, 9, 29).unwrap();
        TeamSeasonScenario {
            name: "Development range".to_string(),
            trade_deadline: None,
            events: vec![
                TeamSeasonScenarioEvent {
                    id: "nyr-breakout".to_string(),
                    kind: TeamSeasonScenarioEventKind::Form,
                    team: "NYR".to_string(),
                    player: Some("Young Ranger".to_string()),
                    effective_date: date,
                    end_date: None,
                    strength_delta: 3.4,
                    occurrence_probability: 0.25,
                    correlation_key: Some("nyr-youth".to_string()),
                    label: "Ranger breakout".to_string(),
                },
                TeamSeasonScenarioEvent {
                    id: "sea-downturn".to_string(),
                    kind: TeamSeasonScenarioEventKind::Form,
                    team: "SEA".to_string(),
                    player: Some("Kraken Veteran".to_string()),
                    effective_date: date,
                    end_date: None,
                    strength_delta: -2.0,
                    occurrence_probability: 0.40,
                    correlation_key: None,
                    label: "Kraken downturn".to_string(),
                },
            ],
            adaptive_lineup_policies: Vec::new(),
            opening_roster_policies: Vec::new(),
        }
    }

    #[test]
    fn isolates_events_and_reconciles_forced_path_units() {
        let view = build_isolated_scenario_impact(
            &forecast(),
            &scenario(),
            TeamSeasonSimulationConfig {
                trials: 20,
                seed: 7,
            },
        )
        .unwrap();
        assert_eq!(view.isolated_events.len(), 2);
        let ranger = view
            .isolated_events
            .iter()
            .find(|row| row.event_id == "nyr-breakout")
            .unwrap();
        assert_eq!(ranger.team, "NYR");
        assert_eq!(ranger.raw_team_strength_delta, 3.4);
        assert_eq!(ranger.correlation_key.as_deref(), Some("nyr-youth"));
        let baseline = view.baseline.iter().find(|row| row.team == "NYR").unwrap();
        assert!(
            (baseline.average_points + ranger.conditional_impact.average_points_delta
                - ranger.conditional_outcome.average_points)
                .abs()
                < 1e-12
        );
        assert!(
            (baseline.playoff_probability + ranger.conditional_impact.playoff_probability_delta
                - ranger.conditional_outcome.playoff_probability)
                .abs()
                < 1e-12
        );
        assert_eq!(view.forced_ceiling_paths.len(), 1);
        assert_eq!(
            view.forced_ceiling_paths[0].raw_team_strength_delta_sum,
            3.4
        );
        assert_eq!(view.forced_ceiling_paths[0].display_label, "+3 Path");
    }

    #[test]
    fn as_of_isolation_shares_the_fixed_result_boundary() {
        let mut forecast = forecast();
        let cutoff = forecast.schedule_start;
        for game in forecast.games.iter_mut().filter(|game| game.date == cutoff) {
            game.actual_away_score = Some(1);
            game.actual_home_score = Some(2);
            game.actual_winner = Some(game.home_team.clone());
            game.actual_ending = None;
        }
        let config = TeamSeasonSimulationConfig {
            trials: 20,
            seed: 19,
        };
        let expected =
            simulate_team_season_forecast_as_of_with_scenario(&forecast, config, None, cutoff)
                .unwrap();
        let view =
            build_isolated_scenario_impact_as_of(&forecast, &scenario(), config, cutoff).unwrap();

        assert_eq!(view.as_of_date, Some(cutoff));
        assert_eq!(view.method, ISOLATED_IMPACT_AS_OF_METHOD);
        assert!(view
            .disclosures
            .iter()
            .any(|value| value.contains("fixes the identical final results")));
        let expected_nyr = expected.teams.iter().find(|row| row.team == "NYR").unwrap();
        let actual_nyr = view.baseline.iter().find(|row| row.team == "NYR").unwrap();
        assert_eq!(actual_nyr.average_points, expected_nyr.average_points);
        assert_eq!(view.isolated_events.len(), 2);
    }

    #[test]
    fn cached_and_uncached_views_are_byte_equivalent() {
        let forecast = forecast();
        let scenario = scenario();
        let config = TeamSeasonSimulationConfig {
            trials: 10,
            seed: 11,
        };
        let uncached = build_isolated_scenario_impact(&forecast, &scenario, config).unwrap();
        let mut cache = IsolatedImpactCache::default();
        let first = build_isolated_scenario_impact_cached(&forecast, &scenario, config, &mut cache)
            .unwrap();
        let second =
            build_isolated_scenario_impact_cached(&forecast, &scenario, config, &mut cache)
                .unwrap();
        assert_eq!(cache.len(), 1);
        assert_eq!(uncached, first);
        assert_eq!(first, second);
        assert_eq!(
            serde_json::to_vec(&uncached).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
    }

    #[test]
    fn event_order_does_not_change_paired_outcomes() {
        let forecast = forecast();
        let scenario = scenario();
        let mut reversed = scenario.clone();
        reversed.events.reverse();
        let config = TeamSeasonSimulationConfig {
            trials: 20,
            seed: 13,
        };
        let first = build_isolated_scenario_impact(&forecast, &scenario, config).unwrap();
        let second = build_isolated_scenario_impact(&forecast, &reversed, config).unwrap();
        assert_eq!(first.isolated_events, second.isolated_events);
        assert_eq!(
            first.naturally_sampled_impacts,
            second.naturally_sampled_impacts
        );
        assert_eq!(first.forced_ceiling_impacts, second.forced_ceiling_impacts);
    }

    #[test]
    fn empty_scenario_is_identical_to_baseline() {
        let empty = TeamSeasonScenario {
            name: "No events".to_string(),
            trade_deadline: None,
            events: Vec::new(),
            adaptive_lineup_policies: Vec::new(),
            opening_roster_policies: Vec::new(),
        };
        let view = build_isolated_scenario_impact(
            &forecast(),
            &empty,
            TeamSeasonSimulationConfig {
                trials: 10,
                seed: 17,
            },
        )
        .unwrap();
        assert!(view.isolated_events.is_empty());
        assert!(view.forced_ceiling_paths.is_empty());
        assert!(view.naturally_sampled_impacts.iter().all(zero_impact));
        assert!(view.forced_ceiling_impacts.iter().all(zero_impact));
    }

    fn zero_impact(impact: &TeamSeasonScenarioImpactRow) -> bool {
        impact.average_points_delta == 0.0
            && impact.playoff_probability_delta == 0.0
            && impact.second_round_probability_delta == 0.0
            && impact.conference_final_probability_delta == 0.0
            && impact.stanley_cup_final_probability_delta == 0.0
            && impact.stanley_cup_probability_delta == 0.0
    }
}
