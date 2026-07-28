use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const DEVELOPMENT_CALIBRATION_SCHEMA: &str = "development_calibration.v2";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DevelopmentValueModel {
    PositionEraNormalizedMultilens,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DevelopmentPositionGroup {
    Forward,
    Defense,
    Goalie,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DevelopmentTransitionInput {
    pub player_id: u32,
    pub player: String,
    pub prior_season: u32,
    pub target_season: u32,
    pub position: DevelopmentPositionGroup,
    pub age: Option<u8>,
    pub prior_games_played: u32,
    pub target_games_played: u32,
    pub prior_value: f64,
    pub target_value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DevelopmentCalibrationConfig {
    pub value_model: DevelopmentValueModel,
    /// A player-value change is translated to team strength at this scale.
    pub team_strength_scale: f64,
    pub breakout_strength_threshold: f64,
    pub downturn_strength_threshold: f64,
    /// Global pseudo-observations used to stabilize small cohort rates.
    pub prior_sample_size: f64,
}

impl Default for DevelopmentCalibrationConfig {
    fn default() -> Self {
        Self {
            value_model: DevelopmentValueModel::PositionEraNormalizedMultilens,
            team_strength_scale: 0.5,
            breakout_strength_threshold: 2.0,
            downturn_strength_threshold: -2.0,
            prior_sample_size: 20.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DevelopmentCalibrationRateRow {
    pub sample_size: usize,
    pub breakout_count: usize,
    pub downturn_count: usize,
    pub stable_count: usize,
    pub breakout_rate: f64,
    pub downturn_rate: f64,
    pub median_breakout_strength_delta: Option<f64>,
    pub median_downturn_strength_delta: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DevelopmentCalibrationCohortRow {
    pub position: DevelopmentPositionGroup,
    pub age_band: String,
    pub experience_band: String,
    pub prior_value_band: String,
    pub sample_size: usize,
    pub breakout_count: usize,
    pub downturn_count: usize,
    pub empirical_breakout_rate: f64,
    pub empirical_downturn_rate: f64,
    pub calibrated_breakout_rate: f64,
    pub calibrated_downturn_rate: f64,
    pub median_breakout_strength_delta: Option<f64>,
    pub median_downturn_strength_delta: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DevelopmentCalibrationExampleRow {
    pub player_id: u32,
    pub player: String,
    pub prior_season: u32,
    pub target_season: u32,
    pub position: DevelopmentPositionGroup,
    pub age: Option<u8>,
    pub prior_games_played: u32,
    pub target_games_played: u32,
    pub prior_value: f64,
    pub target_value: f64,
    pub strength_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DevelopmentCalibrationView {
    pub schema: String,
    pub seasons: Vec<u32>,
    pub transitions: usize,
    pub config: DevelopmentCalibrationConfig,
    pub global: DevelopmentCalibrationRateRow,
    pub cohorts: Vec<DevelopmentCalibrationCohortRow>,
    /// All workload-qualified players in the newest target season, retained so
    /// a preseason scenario can reproducibly select the matching next-season cohort.
    pub latest_season_players: Vec<DevelopmentCalibrationExampleRow>,
    pub largest_breakouts: Vec<DevelopmentCalibrationExampleRow>,
    pub largest_downturns: Vec<DevelopmentCalibrationExampleRow>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CohortKey {
    position: DevelopmentPositionGroup,
    age_band: String,
    experience_band: String,
    prior_value_band: String,
}

pub fn build_development_calibration(
    mut transitions: Vec<DevelopmentTransitionInput>,
    config: DevelopmentCalibrationConfig,
) -> Result<DevelopmentCalibrationView, String> {
    if transitions.is_empty() {
        return Err("development calibration requires historical transitions".to_owned());
    }
    if !config.team_strength_scale.is_finite()
        || config.team_strength_scale <= 0.0
        || !config.breakout_strength_threshold.is_finite()
        || !config.downturn_strength_threshold.is_finite()
        || config.breakout_strength_threshold <= 0.0
        || config.downturn_strength_threshold >= 0.0
        || !config.prior_sample_size.is_finite()
        || config.prior_sample_size < 0.0
    {
        return Err("development calibration configuration is invalid".to_owned());
    }
    if transitions.iter().any(|row| {
        row.player.trim().is_empty()
            || row.prior_season >= row.target_season
            || !row.prior_value.is_finite()
            || !row.target_value.is_finite()
    }) {
        return Err("development calibration transition is invalid".to_owned());
    }
    transitions.sort_by(|a, b| {
        a.target_season
            .cmp(&b.target_season)
            .then_with(|| a.player_id.cmp(&b.player_id))
    });
    let deltas = transitions
        .iter()
        .map(|row| strength_delta(row, config.team_strength_scale))
        .collect::<Vec<_>>();
    let global = rate_row(&deltas, &config);
    let mut grouped = BTreeMap::<CohortKey, Vec<f64>>::new();
    for (row, delta) in transitions.iter().zip(&deltas) {
        grouped
            .entry(CohortKey {
                position: row.position,
                age_band: age_band(row.age),
                experience_band: experience_band(row.prior_games_played),
                prior_value_band: value_band(row.prior_value),
            })
            .or_default()
            .push(*delta);
    }
    let cohorts = grouped
        .into_iter()
        .map(|(key, deltas)| {
            let raw = rate_row(&deltas, &config);
            let sample = raw.sample_size as f64;
            let denominator = sample + config.prior_sample_size;
            DevelopmentCalibrationCohortRow {
                position: key.position,
                age_band: key.age_band,
                experience_band: key.experience_band,
                prior_value_band: key.prior_value_band,
                sample_size: raw.sample_size,
                breakout_count: raw.breakout_count,
                downturn_count: raw.downturn_count,
                empirical_breakout_rate: raw.breakout_rate,
                empirical_downturn_rate: raw.downturn_rate,
                calibrated_breakout_rate: (raw.breakout_count as f64
                    + global.breakout_rate * config.prior_sample_size)
                    / denominator,
                calibrated_downturn_rate: (raw.downturn_count as f64
                    + global.downturn_rate * config.prior_sample_size)
                    / denominator,
                median_breakout_strength_delta: raw
                    .median_breakout_strength_delta
                    .or(global.median_breakout_strength_delta),
                median_downturn_strength_delta: raw
                    .median_downturn_strength_delta
                    .or(global.median_downturn_strength_delta),
            }
        })
        .collect::<Vec<_>>();
    let mut examples = transitions
        .iter()
        .map(|row| DevelopmentCalibrationExampleRow {
            player_id: row.player_id,
            player: row.player.clone(),
            prior_season: row.prior_season,
            target_season: row.target_season,
            position: row.position,
            age: row.age,
            prior_games_played: row.prior_games_played,
            target_games_played: row.target_games_played,
            prior_value: row.prior_value,
            target_value: row.target_value,
            strength_delta: strength_delta(row, config.team_strength_scale),
        })
        .collect::<Vec<_>>();
    let latest_target_season = examples
        .iter()
        .map(|row| row.target_season)
        .max()
        .unwrap_or_default();
    let mut latest_season_players = examples
        .iter()
        .filter(|row| row.target_season == latest_target_season)
        .cloned()
        .collect::<Vec<_>>();
    latest_season_players.sort_by(|a, b| {
        a.position
            .cmp(&b.position)
            .then_with(|| a.player.cmp(&b.player))
            .then_with(|| a.player_id.cmp(&b.player_id))
    });
    examples.sort_by(|a, b| {
        b.strength_delta
            .total_cmp(&a.strength_delta)
            .then_with(|| a.target_season.cmp(&b.target_season))
            .then_with(|| a.player_id.cmp(&b.player_id))
    });
    let largest_breakouts = examples.iter().take(20).cloned().collect();
    let largest_downturns = examples.iter().rev().take(20).cloned().collect();
    let seasons = transitions
        .iter()
        .flat_map(|row| [row.prior_season, row.target_season])
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    Ok(DevelopmentCalibrationView {
        schema: DEVELOPMENT_CALIBRATION_SCHEMA.to_owned(),
        seasons,
        transitions: transitions.len(),
        config,
        global,
        cohorts,
        latest_season_players,
        largest_breakouts,
        largest_downturns,
        disclosures: vec![
            "Every transition uses only one completed season and its immediately following completed season; no later player results enter an earlier label.".to_owned(),
            "Player value is position- and season-normalized across multiple available lenses, credibility-shrunk toward league average, then converts half of the year-over-year player-value change to team strength.".to_owned(),
            "Skater lenses are scoring, ice time, shot generation, power-play production, and plus/minus context; goalie lenses are save percentage, inverse goals-against average, starts, and shutout rate.".to_owned(),
            "The embedded historical summary bundle does not contain complete blocks, expected goals, possession, matchup-quality, or special-teams deployment history, so those lenses are not represented in this calibration.".to_owned(),
            "Breakouts and downturns are performance outcomes conditional on meeting the target-season workload gate; injuries, retirement, and failure to earn that workload require separate availability modeling.".to_owned(),
            "Entry cohorts have fewer than 10 prior-season NHL games. Their rates are conditional on reaching the target workload and must not be interpreted as draft-prospect NHL-arrival probabilities.".to_owned(),
            "Cohort rates use global empirical-rate shrinkage so small age/position/experience/value cells do not produce unjustified zero or one probabilities.".to_owned(),
            "The latest-season player table is a reproducible cohort-lookup aid, not a ranking: use its target age, workload, and value as the prior-season inputs for the following preseason.".to_owned(),
        ],
    })
}

pub fn development_cohort_labels(
    age: Option<u8>,
    prior_games_played: u32,
    prior_value: f64,
) -> (String, String, String) {
    (
        age_band(age),
        experience_band(prior_games_played),
        value_band(prior_value),
    )
}

fn strength_delta(row: &DevelopmentTransitionInput, scale: f64) -> f64 {
    ((row.target_value - row.prior_value) * scale).clamp(-8.0, 8.0)
}

fn rate_row(
    deltas: &[f64],
    config: &DevelopmentCalibrationConfig,
) -> DevelopmentCalibrationRateRow {
    let breakout = deltas
        .iter()
        .copied()
        .filter(|delta| *delta >= config.breakout_strength_threshold)
        .collect::<Vec<_>>();
    let downturn = deltas
        .iter()
        .copied()
        .filter(|delta| *delta <= config.downturn_strength_threshold)
        .collect::<Vec<_>>();
    let sample = deltas.len();
    DevelopmentCalibrationRateRow {
        sample_size: sample,
        breakout_count: breakout.len(),
        downturn_count: downturn.len(),
        stable_count: sample - breakout.len() - downturn.len(),
        breakout_rate: breakout.len() as f64 / sample as f64,
        downturn_rate: downturn.len() as f64 / sample as f64,
        median_breakout_strength_delta: median(breakout),
        median_downturn_strength_delta: median(downturn),
    }
}

fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    Some(if values.len().is_multiple_of(2) {
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    })
}

fn age_band(age: Option<u8>) -> String {
    match age {
        Some(0..=22) => "22_or_younger",
        Some(23..=25) => "23_to_25",
        Some(26..=29) => "26_to_29",
        Some(30..=32) => "30_to_32",
        Some(_) => "33_or_older",
        None => "unknown",
    }
    .to_owned()
}

fn experience_band(games: u32) -> String {
    match games {
        0..=9 => "entry",
        10..=39 => "limited",
        _ => "established",
    }
    .to_owned()
}

fn value_band(value: f64) -> String {
    if value < 48.0 {
        "below_average"
    } else if value < 56.0 {
        "average"
    } else {
        "impact"
    }
    .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: u32, delta: f64, age: Option<u8>, gp: u32) -> DevelopmentTransitionInput {
        DevelopmentTransitionInput {
            player_id: id,
            player: format!("Player {id}"),
            prior_season: 20232024,
            target_season: 20242025,
            position: DevelopmentPositionGroup::Forward,
            age,
            prior_games_played: gp,
            target_games_played: 70,
            prior_value: 50.0,
            target_value: 50.0 + delta * 2.0,
        }
    }

    #[test]
    fn calibration_classifies_thresholds_and_shrinks_cohorts() {
        let view = build_development_calibration(
            vec![
                row(1, 3.0, Some(21), 5),
                row(2, -3.0, Some(21), 5),
                row(3, 0.5, Some(28), 70),
                row(4, 2.0, Some(28), 70),
            ],
            DevelopmentCalibrationConfig::default(),
        )
        .unwrap();
        assert_eq!(view.global.breakout_count, 2);
        assert_eq!(view.global.downturn_count, 1);
        assert_eq!(view.global.stable_count, 1);
        assert_eq!(
            view.config.value_model,
            DevelopmentValueModel::PositionEraNormalizedMultilens
        );
        assert_eq!(
            serde_json::to_value(view.config.value_model).unwrap(),
            "position_era_normalized_multilens"
        );
        assert_eq!(view.cohorts.len(), 2);
        assert_eq!(view.latest_season_players.len(), 4);
        assert!(view.cohorts.iter().all(|cohort| {
            (0.0..=1.0).contains(&cohort.calibrated_breakout_rate)
                && (0.0..=1.0).contains(&cohort.calibrated_downturn_rate)
        }));
        assert_eq!(view.largest_breakouts[0].player_id, 1);
        assert_eq!(view.largest_downturns[0].player_id, 2);
    }

    #[test]
    fn cohort_labels_are_stable_at_boundaries() {
        assert_eq!(
            development_cohort_labels(Some(22), 9, 47.99),
            (
                "22_or_younger".to_owned(),
                "entry".to_owned(),
                "below_average".to_owned()
            )
        );
        assert_eq!(
            development_cohort_labels(None, 40, 56.0),
            (
                "unknown".to_owned(),
                "established".to_owned(),
                "impact".to_owned()
            )
        );
    }
}
