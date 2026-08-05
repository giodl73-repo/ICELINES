use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::prospect_conversion::{
    ProspectConversionBoardView, PROSPECT_CONVERSION_BOARD_SCHEMA, PROSPECT_CONVERSION_INPUT_SCHEMA,
};
use super::prospect_study::{ProspectDevelopmentStudyView, PROSPECT_DEVELOPMENT_STUDY_SCHEMA};

pub const PROSPECT_ARRIVAL_CALIBRATION_INPUT_SCHEMA: &str = "prospect_arrival_calibration_input.v1";
pub const PROSPECT_ARRIVAL_CALIBRATION_SCHEMA: &str = "prospect_arrival_calibration.v1";
pub const PROSPECT_ARRIVAL_LEAGUE_CALIBRATION_SCHEMA: &str =
    "prospect_arrival_league_calibration.v1";

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProspectArrivalCalibrationConfig {
    pub neighbor_count: usize,
    pub minimum_sample_size: usize,
    pub prior_sample_size: f64,
    pub maximum_mean_signal_distance: f64,
    /// Scenario horizon receiving the cumulative source-cohort probability.
    #[serde(default = "one_forecast_horizon")]
    pub forecast_horizon_seasons: u8,
}

impl Default for ProspectArrivalCalibrationConfig {
    fn default() -> Self {
        Self {
            neighbor_count: 50,
            minimum_sample_size: 30,
            prior_sample_size: 20.0,
            maximum_mean_signal_distance: 15.0,
            forecast_horizon_seasons: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectArrivalCalibrationInput {
    pub schema: String,
    pub event_id: String,
    pub player_id: u32,
    pub player: String,
    pub position: String,
    /// Frozen attention-free prospect signal on the conversion board's 0..100 basis.
    pub observed_signal_score: f64,
    pub forecast_season: u32,
    #[serde(default)]
    pub config: ProspectArrivalCalibrationConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectArrivalCalibrationView {
    pub schema: String,
    pub source_schema: String,
    pub source_baseline_seasons: Vec<u32>,
    pub source_through_season: u32,
    pub event_id: String,
    pub player_id: u32,
    pub player: String,
    pub position_group: String,
    pub observed_signal_score: f64,
    pub forecast_season: u32,
    pub candidate_players: usize,
    pub neighbor_players: usize,
    pub neighbor_signal_min: f64,
    pub neighbor_signal_max: f64,
    pub mean_signal_distance: f64,
    pub neighbor_arrivals: usize,
    pub neighbor_established_players: usize,
    pub empirical_arrival_rate: f64,
    pub position_arrival_rate: f64,
    pub calibrated_arrival_probability: f64,
    pub empirical_established_rate: f64,
    /// Complete same-position establishment rate used as the shrinkage prior.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position_established_rate: Option<f64>,
    /// Neighbor establishment rate shrunk toward the complete position cohort.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calibrated_established_probability: Option<f64>,
    /// Descriptive establishment share among neighbors who reached the NHL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub established_given_arrival_rate: Option<f64>,
    /// Common outcome horizon represented by the historical cohort.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_horizon_seasons: Option<u32>,
    /// Requested scenario horizon receiving the adjusted probability.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forecast_horizon_seasons: Option<u8>,
    /// Constant-hazard projection of cumulative arrival into the forecast horizon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horizon_adjusted_arrival_probability: Option<f64>,
    /// Constant-hazard projection of cumulative establishment into the forecast horizon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horizon_adjusted_established_probability: Option<f64>,
    pub config: ProspectArrivalCalibrationConfig,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectArrivalLeagueExclusionView {
    pub player_id: u32,
    pub player: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProspectArrivalLeagueSourceExclusionInput {
    pub organization: String,
    pub player_id: u32,
    pub player: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProspectArrivalLeaguePopulationAuthorityView {
    pub source_package_fingerprint: String,
    pub population_complete: bool,
    pub supplied_studies: usize,
    pub controlled_studies: usize,
    pub control_exclusions: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectArrivalLeagueTeamView {
    pub organization: String,
    pub target_skaters: usize,
    pub calibrated_skaters: usize,
    pub excluded_skaters: usize,
    pub calibrations: Vec<ProspectArrivalCalibrationView>,
    pub exclusions: Vec<ProspectArrivalLeagueExclusionView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectArrivalLeagueCalibrationView {
    pub schema: String,
    pub source_schema: String,
    pub forecast_season: u32,
    pub organizations_requested: usize,
    pub organizations_represented: usize,
    pub target_skaters: usize,
    pub calibrated_skaters: usize,
    pub excluded_skaters: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub population_authority: Option<ProspectArrivalLeaguePopulationAuthorityView>,
    pub teams: Vec<ProspectArrivalLeagueTeamView>,
    pub disclosures: Vec<String>,
}

/// Apply one historical cohort and one calibration policy to every supplied
/// organization. Failures stay visible beside successful targets; an
/// organization with no eligible skater study is retained as an empty row.
pub fn calibrate_prospect_arrival_league(
    organizations: Vec<String>,
    studies: Vec<ProspectDevelopmentStudyView>,
    source_exclusions: Vec<ProspectArrivalLeagueSourceExclusionInput>,
    population_authority: Option<ProspectArrivalLeaguePopulationAuthorityView>,
    forecast_season: u32,
    board: &ProspectConversionBoardView,
    config: ProspectArrivalCalibrationConfig,
) -> Result<ProspectArrivalLeagueCalibrationView, String> {
    validate_config(config)?;
    if organizations.is_empty() || forecast_season <= board.through_season {
        return Err("prospect arrival league calibration inputs are invalid".to_owned());
    }
    let mut normalized = BTreeSet::new();
    for organization in organizations {
        let organization = organization.trim().to_ascii_uppercase();
        if organization.is_empty() || !normalized.insert(organization) {
            return Err(
                "prospect arrival league organizations are invalid or duplicated".to_owned(),
            );
        }
    }
    let mut by_organization = BTreeMap::<String, Vec<ProspectDevelopmentStudyView>>::new();
    let mut player_ids = BTreeSet::new();
    for study in studies {
        let organization = study.organization.trim().to_ascii_uppercase();
        if !normalized.contains(&organization) {
            return Err(format!(
                "prospect arrival study organization {organization} is outside the league envelope"
            ));
        }
        if !player_ids.insert(study.player_id) {
            return Err(format!(
                "prospect arrival league target player {} is duplicated",
                study.player_id
            ));
        }
        by_organization.entry(organization).or_default().push(study);
    }
    let controlled_studies = player_ids.len();
    let source_exclusion_count = source_exclusions.len();
    let mut exclusions_by_organization =
        BTreeMap::<String, Vec<ProspectArrivalLeagueExclusionView>>::new();
    for exclusion in source_exclusions {
        let organization = exclusion.organization.trim().to_ascii_uppercase();
        if !normalized.contains(&organization)
            || exclusion.player_id == 0
            || exclusion.player.trim().is_empty()
            || exclusion.reason.trim().is_empty()
        {
            return Err("prospect arrival league source exclusion is invalid".to_owned());
        }
        if !player_ids.insert(exclusion.player_id) {
            return Err(format!(
                "prospect arrival league target player {} is duplicated across controlled and excluded studies",
                exclusion.player_id
            ));
        }
        exclusions_by_organization
            .entry(organization)
            .or_default()
            .push(ProspectArrivalLeagueExclusionView {
                player_id: exclusion.player_id,
                player: exclusion.player,
                reason: exclusion.reason,
            });
    }
    if population_authority.is_none() && source_exclusion_count > 0 {
        return Err(
            "prospect arrival league source exclusions require population authority".to_owned(),
        );
    }
    if let Some(authority) = population_authority.as_ref() {
        if authority.source_package_fingerprint.trim().is_empty()
            || authority.supplied_studies
                != authority.controlled_studies + authority.control_exclusions
            || authority.controlled_studies != controlled_studies
            || authority.control_exclusions != source_exclusion_count
            || authority.supplied_studies != player_ids.len()
        {
            return Err("prospect arrival league population authority is invalid".to_owned());
        }
    }

    let mut teams = Vec::with_capacity(normalized.len());
    for organization in normalized {
        let mut calibrations = Vec::new();
        let mut exclusions = exclusions_by_organization
            .remove(&organization)
            .unwrap_or_default();
        let mut targets = by_organization.remove(&organization).unwrap_or_default();
        targets.sort_by_key(|study| study.player_id);
        let target_skaters = targets.len() + exclusions.len();
        for study in targets {
            let player_id = study.player_id;
            let player = study.player.clone();
            let event_id = format!(
                "{}-{}-prospect-arrival",
                organization.to_ascii_lowercase(),
                player_id
            );
            let result =
                adapt_prospect_arrival_calibration_input(event_id, &study, forecast_season, config)
                    .and_then(|input| calibrate_prospect_arrival(input, board));
            match result {
                Ok(view) => calibrations.push(view),
                Err(reason) => exclusions.push(ProspectArrivalLeagueExclusionView {
                    player_id,
                    player,
                    reason,
                }),
            }
        }
        calibrations.sort_by_key(|view| view.player_id);
        exclusions.sort_by_key(|view| view.player_id);
        teams.push(ProspectArrivalLeagueTeamView {
            organization,
            target_skaters,
            calibrated_skaters: calibrations.len(),
            excluded_skaters: exclusions.len(),
            calibrations,
            exclusions,
        });
    }
    let target_skaters = teams.iter().map(|team| team.target_skaters).sum();
    let calibrated_skaters = teams.iter().map(|team| team.calibrated_skaters).sum();
    let excluded_skaters = teams.iter().map(|team| team.excluded_skaters).sum();
    Ok(ProspectArrivalLeagueCalibrationView {
        schema: PROSPECT_ARRIVAL_LEAGUE_CALIBRATION_SCHEMA.to_owned(),
        source_schema: board.schema.clone(),
        forecast_season,
        organizations_requested: teams.len(),
        organizations_represented: teams.len(),
        target_skaters,
        calibrated_skaters,
        excluded_skaters,
        population_authority,
        teams,
        disclosures: vec![
            "Every requested organization remains present, including teams with no eligible skater study or no successful calibration.".to_owned(),
            "All targets use the same frozen conversion cohort, same-position neighbor policy, shrinkage prior, signal-distance gate, and forecast-horizon adjustment.".to_owned(),
            "Goalies require a separately calibrated goalie outcome cohort and are not treated as skaters in this artifact.".to_owned(),
            "A player-level failure is retained as a typed team exclusion and never silently removed from league coverage totals.".to_owned(),
            "When a source package is supplied, the existing current-control resolver gates studies before calibration and its unsupported or mismatched rows remain in the same reconciled exclusion ledger.".to_owned(),
        ],
    })
}

pub fn adapt_prospect_arrival_calibration_input(
    event_id: impl Into<String>,
    study: &ProspectDevelopmentStudyView,
    forecast_season: u32,
    config: ProspectArrivalCalibrationConfig,
) -> Result<ProspectArrivalCalibrationInput, String> {
    if study.schema != PROSPECT_DEVELOPMENT_STUDY_SCHEMA {
        return Err("prospect arrival adaptation requires a canonical skater study".to_owned());
    }
    if study.nhl_games_played > 0 {
        return Err(format!(
            "prospect arrival target already has {} NHL games; use established-role forecasting instead",
            study.nhl_games_played
        ));
    }
    let component = |id: &str| {
        let rows = study
            .components
            .iter()
            .filter(|component| component.id == id)
            .collect::<Vec<_>>();
        if rows.len() != 1 || !rows[0].score.is_finite() || !(0.0..=1.0).contains(&rows[0].score) {
            return Err(format!(
                "prospect arrival study lacks unique {id} component"
            ));
        }
        Ok(rows[0].score)
    };
    let observed_signal_score = 100.0
        * (0.50 * component("production")?
            + 0.25 * component("trajectory")?
            + 0.25 * component("opportunity")?);
    let input = ProspectArrivalCalibrationInput {
        schema: PROSPECT_ARRIVAL_CALIBRATION_INPUT_SCHEMA.to_owned(),
        event_id: event_id.into(),
        player_id: study.player_id,
        player: study.player.clone(),
        position: study.position.clone(),
        observed_signal_score: (observed_signal_score * 100.0).round() / 100.0,
        forecast_season,
        config,
    };
    validate_config(input.config)?;
    if input.event_id.trim().is_empty()
        || input.player_id == 0
        || input.player.trim().is_empty()
        || input.position.trim().is_empty()
    {
        return Err("adapted prospect arrival input is invalid".to_owned());
    }
    Ok(input)
}

pub fn calibrate_prospect_arrival(
    input: ProspectArrivalCalibrationInput,
    board: &ProspectConversionBoardView,
) -> Result<ProspectArrivalCalibrationView, String> {
    if input.schema != PROSPECT_ARRIVAL_CALIBRATION_INPUT_SCHEMA {
        return Err(format!(
            "prospect arrival calibration requires {PROSPECT_ARRIVAL_CALIBRATION_INPUT_SCHEMA}"
        ));
    }
    if board.schema != PROSPECT_CONVERSION_BOARD_SCHEMA {
        return Err(format!(
            "prospect arrival calibration requires {PROSPECT_CONVERSION_BOARD_SCHEMA}"
        ));
    }
    if board.source_schema != PROSPECT_CONVERSION_INPUT_SCHEMA
        || board.baseline_seasons.is_empty()
        || board.organizations != board.programs.len()
    {
        return Err(
            "prospect arrival calibration conversion board authority is invalid".to_owned(),
        );
    }
    if input.event_id.trim().is_empty()
        || input.player_id == 0
        || input.player.trim().is_empty()
        || !input.observed_signal_score.is_finite()
        || !(0.0..=100.0).contains(&input.observed_signal_score)
        || input.forecast_season <= board.through_season
    {
        return Err(
            "prospect arrival calibration input is invalid or leaks its forecast horizon"
                .to_owned(),
        );
    }
    validate_config(input.config)?;
    let target_position_group = position_group(&input.position)
        .ok_or_else(|| "prospect arrival calibration position is unsupported".to_owned())?;
    let all_players = board
        .programs
        .iter()
        .flat_map(|program| &program.player_results)
        .collect::<Vec<_>>();
    let player_ids = all_players
        .iter()
        .map(|player| player.player_id)
        .collect::<BTreeSet<_>>();
    if all_players.len() != board.players
        || player_ids.len() != all_players.len()
        || all_players.iter().any(|player| {
            player.player_id == 0
                || player.through_season != board.through_season
                || !board.baseline_seasons.contains(&player.baseline_season)
                || !player.baseline_signal_score.is_finite()
                || !(0.0..=100.0).contains(&player.baseline_signal_score)
                || player.horizon_seasons == 0
        })
    {
        return Err("prospect arrival calibration conversion board players are invalid".to_owned());
    }
    if all_players
        .iter()
        .any(|player| player.player_id == input.player_id)
    {
        return Err(
            "prospect arrival calibration target cannot appear in its historical outcome cohort"
                .to_owned(),
        );
    }
    let mut candidates = all_players
        .into_iter()
        .filter(|player| position_group(&player.position) == Some(target_position_group))
        .collect::<Vec<_>>();
    if candidates.len() < input.config.minimum_sample_size {
        return Err(format!(
            "prospect arrival calibration has only {} same-position candidates; {} required",
            candidates.len(),
            input.config.minimum_sample_size
        ));
    }
    let candidate_players = candidates.len();
    let source_horizons = candidates
        .iter()
        .map(|player| player.horizon_seasons)
        .collect::<BTreeSet<_>>();
    if source_horizons.len() != 1 {
        return Err(
            "prospect arrival calibration requires one common historical outcome horizon"
                .to_owned(),
        );
    }
    let source_horizon_seasons = *source_horizons
        .iter()
        .next()
        .expect("validated historical horizon");
    if u32::from(input.config.forecast_horizon_seasons) > source_horizon_seasons {
        return Err(format!(
            "prospect arrival forecast horizon {} exceeds historical horizon {source_horizon_seasons}",
            input.config.forecast_horizon_seasons
        ));
    }
    let position_arrival_rate = candidates
        .iter()
        .filter(|player| player.nhl_games_played > 0)
        .count() as f64
        / candidate_players as f64;
    let position_established_rate = candidates
        .iter()
        .filter(|player| player.established)
        .count() as f64
        / candidate_players as f64;
    candidates.sort_by(|left, right| {
        (left.baseline_signal_score - input.observed_signal_score)
            .abs()
            .total_cmp(&(right.baseline_signal_score - input.observed_signal_score).abs())
            .then_with(|| left.player_id.cmp(&right.player_id))
    });
    let neighbor_players = input.config.neighbor_count.min(candidates.len());
    if neighbor_players < input.config.minimum_sample_size {
        return Err(format!(
            "prospect arrival calibration selected only {neighbor_players} neighbors; {} required",
            input.config.minimum_sample_size
        ));
    }
    let neighbors = &candidates[..neighbor_players];
    let mean_signal_distance = neighbors
        .iter()
        .map(|player| (player.baseline_signal_score - input.observed_signal_score).abs())
        .sum::<f64>()
        / neighbor_players as f64;
    if mean_signal_distance > input.config.maximum_mean_signal_distance {
        return Err(format!(
            "prospect arrival calibration mean signal distance {mean_signal_distance:.4} exceeds {:.4}",
            input.config.maximum_mean_signal_distance
        ));
    }
    let neighbor_arrivals = neighbors
        .iter()
        .filter(|player| player.nhl_games_played > 0)
        .count();
    let neighbor_established_players = neighbors.iter().filter(|player| player.established).count();
    let empirical_arrival_rate = neighbor_arrivals as f64 / neighbor_players as f64;
    let empirical_established_rate = neighbor_established_players as f64 / neighbor_players as f64;
    let calibrated_arrival_probability = (neighbor_arrivals as f64
        + position_arrival_rate * input.config.prior_sample_size)
        / (neighbor_players as f64 + input.config.prior_sample_size);
    let calibrated_established_probability = (neighbor_established_players as f64
        + position_established_rate * input.config.prior_sample_size)
        / (neighbor_players as f64 + input.config.prior_sample_size);
    let established_given_arrival_rate = (neighbor_arrivals > 0)
        .then_some(neighbor_established_players as f64 / neighbor_arrivals as f64);
    let horizon_adjusted_arrival_probability = adjust_cumulative_probability(
        calibrated_arrival_probability,
        source_horizon_seasons,
        input.config.forecast_horizon_seasons,
    );
    let horizon_adjusted_established_probability = adjust_cumulative_probability(
        calibrated_established_probability,
        source_horizon_seasons,
        input.config.forecast_horizon_seasons,
    );
    let neighbor_signal_min = neighbors
        .iter()
        .map(|player| player.baseline_signal_score)
        .min_by(f64::total_cmp)
        .expect("validated non-empty neighbors");
    let neighbor_signal_max = neighbors
        .iter()
        .map(|player| player.baseline_signal_score)
        .max_by(f64::total_cmp)
        .expect("validated non-empty neighbors");

    Ok(ProspectArrivalCalibrationView {
        schema: PROSPECT_ARRIVAL_CALIBRATION_SCHEMA.to_owned(),
        source_schema: board.schema.clone(),
        source_baseline_seasons: board.baseline_seasons.clone(),
        source_through_season: board.through_season,
        event_id: input.event_id,
        player_id: input.player_id,
        player: input.player,
        position_group: target_position_group.to_owned(),
        observed_signal_score: round_score(input.observed_signal_score),
        forecast_season: input.forecast_season,
        candidate_players,
        neighbor_players,
        neighbor_signal_min: round_score(neighbor_signal_min),
        neighbor_signal_max: round_score(neighbor_signal_max),
        mean_signal_distance: round_score(mean_signal_distance),
        neighbor_arrivals,
        neighbor_established_players,
        empirical_arrival_rate: round_ratio(empirical_arrival_rate),
        position_arrival_rate: round_ratio(position_arrival_rate),
        calibrated_arrival_probability: round_ratio(calibrated_arrival_probability),
        empirical_established_rate: round_ratio(empirical_established_rate),
        position_established_rate: Some(round_ratio(position_established_rate)),
        calibrated_established_probability: Some(round_ratio(
            calibrated_established_probability,
        )),
        established_given_arrival_rate: established_given_arrival_rate.map(round_ratio),
        source_horizon_seasons: Some(source_horizon_seasons),
        forecast_horizon_seasons: Some(input.config.forecast_horizon_seasons),
        horizon_adjusted_arrival_probability: Some(round_ratio(
            horizon_adjusted_arrival_probability,
        )),
        horizon_adjusted_established_probability: Some(round_ratio(
            horizon_adjusted_established_probability,
        )),
        config: input.config,
        disclosures: vec![
            "Arrival means at least one post-baseline regular-season NHL game inside the frozen conversion horizon; it does not mean the player establishes an NHL role."
                .to_owned(),
            "The estimate uses nearest same-position historical prospect signals and shrinks their empirical arrival rate toward the complete same-position cohort rate."
                .to_owned(),
            "Established-role probability is calibrated independently by shrinking the neighbors' unconditional establishment rate toward the complete same-position establishment rate; established-given-arrival remains descriptive."
                .to_owned(),
            format!(
                "Cumulative historical probabilities span {source_horizon_seasons} seasons and are projected to the {}-season scenario horizon with a disclosed constant-hazard transformation.",
                input.config.forecast_horizon_seasons
            ),
            "The target is rejected if it appears in the historical outcome cohort, the board reaches the forecast season, or sample and signal-distance gates fail."
                .to_owned(),
            "This historical base-rate estimate does not incorporate current camp performance, injuries, contract status, waivers, organizational depth, or manager behavior."
                .to_owned(),
        ],
    })
}

fn validate_config(config: ProspectArrivalCalibrationConfig) -> Result<(), String> {
    if config.neighbor_count == 0
        || config.minimum_sample_size == 0
        || config.neighbor_count < config.minimum_sample_size
        || !config.prior_sample_size.is_finite()
        || config.prior_sample_size < 0.0
        || !config.maximum_mean_signal_distance.is_finite()
        || config.maximum_mean_signal_distance <= 0.0
        || config.forecast_horizon_seasons == 0
    {
        return Err("prospect arrival calibration configuration is invalid".to_owned());
    }
    Ok(())
}

fn adjust_cumulative_probability(
    probability: f64,
    source_horizon_seasons: u32,
    forecast_horizon_seasons: u8,
) -> f64 {
    1.0 - (1.0 - probability)
        .powf(f64::from(forecast_horizon_seasons) / f64::from(source_horizon_seasons))
}

const fn one_forecast_horizon() -> u8 {
    1
}

fn position_group(position: &str) -> Option<&'static str> {
    match position.trim().to_ascii_uppercase().as_str() {
        "F" | "C" | "LW" | "RW" | "W" => Some("F"),
        "D" | "LD" | "RD" => Some("D"),
        "G" => Some("G"),
        _ => None,
    }
}

fn round_score(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn round_ratio(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_model::prospect_conversion::{
        ProspectConversionConfig, ProspectConversionDisposition, ProspectConversionMethodologyView,
        ProspectConversionOrganizationView, ProspectConversionPlayerView,
        ProspectConversionResultClass, ProspectConversionSignalCalibrationView,
    };
    use crate::view_model::prospect_study::{
        build_prospect_development_study, ProspectAvailabilityStatus,
        ProspectDevelopmentSeasonInput, ProspectDevelopmentStudyConfig,
        ProspectDevelopmentStudyInput, ProspectOpportunityStatus,
    };

    fn player(id: u32, signal: f64, arrived: bool) -> ProspectConversionPlayerView {
        ProspectConversionPlayerView {
            player_id: id,
            player: format!("Historical {id}"),
            organization: "HIS".to_owned(),
            position: "D".to_owned(),
            baseline_season: 20222023,
            through_season: 20252026,
            horizon_seasons: 3,
            baseline_signal_score: signal,
            workload_confidence: 0.8,
            nhl_games_played: if arrived { 30 } else { 0 },
            nhl_toi_seconds: if arrived { 900_000 } else { 0 },
            arrival_score: if arrived { 36.5854 } else { 0.0 },
            role_score: if arrived { 55.0 } else { 0.0 },
            performance_score: Some(if arrived { 50.0 } else { 0.0 }),
            performance_basis: Some("test".to_owned()),
            realized_value_score: if arrived { 46.0 } else { 0.0 },
            outcome_coverage: 1.0,
            conversion_delta: 0.0,
            efficiency_index: 100.0,
            established: false,
            result_class: ProspectConversionResultClass::Developing,
            disposition: ProspectConversionDisposition::Unknown,
            evidence: vec![],
        }
    }

    fn board() -> ProspectConversionBoardView {
        let players = (0..40)
            .map(|index| player(1_000 + index, 40.0 + f64::from(index), index >= 15))
            .collect::<Vec<_>>();
        ProspectConversionBoardView {
            schema: PROSPECT_CONVERSION_BOARD_SCHEMA.to_owned(),
            source_schema: "prospect_conversion_input.v2".to_owned(),
            baseline_basis: "test".to_owned(),
            methodology: ProspectConversionMethodologyView {
                method: "test".to_owned(),
                config: ProspectConversionConfig::default(),
            },
            baseline_seasons: vec![20222023],
            through_season: 20252026,
            organizations: 1,
            players: 40,
            ranked_organizations: 1,
            signal_calibration: Vec::<ProspectConversionSignalCalibrationView>::new(),
            programs: vec![ProspectConversionOrganizationView {
                organization: "HIS".to_owned(),
                players: 40,
                converted_players: 25,
                established_players: 0,
                retained_players: 0,
                traded_players: 0,
                expected_hits: 0,
                breakouts: 0,
                misses: 0,
                developing_players: 40,
                baseline_signal_score: 59.5,
                baseline_confidence: 0.8,
                realized_value_score: 0.0,
                conversion_delta: 0.0,
                efficiency_index: 100.0,
                outcome_coverage: 1.0,
                conversion_rank: Some(1),
                rank_blockers: vec![],
                player_results: players,
            }],
            disclosures: vec![],
        }
    }

    #[test]
    fn arrival_uses_same_position_neighbors_and_global_shrinkage() {
        let view = calibrate_prospect_arrival(
            ProspectArrivalCalibrationInput {
                schema: PROSPECT_ARRIVAL_CALIBRATION_INPUT_SCHEMA.to_owned(),
                event_id: "smits-arrival".to_owned(),
                player_id: 8_485_957,
                player: "Alberts Smits".to_owned(),
                position: "D".to_owned(),
                observed_signal_score: 60.0,
                forecast_season: 20262027,
                config: ProspectArrivalCalibrationConfig {
                    neighbor_count: 30,
                    minimum_sample_size: 20,
                    prior_sample_size: 20.0,
                    maximum_mean_signal_distance: 15.0,
                    forecast_horizon_seasons: 1,
                },
            },
            &board(),
        )
        .unwrap();
        assert_eq!(view.candidate_players, 40);
        assert_eq!(view.neighbor_players, 30);
        assert_eq!(view.position_group, "D");
        assert!((0.0..=1.0).contains(&view.calibrated_arrival_probability));
        assert!(view.position_established_rate.is_some());
        assert!(view.calibrated_established_probability.is_some());
        assert!(view.established_given_arrival_rate.is_some());
        assert_eq!(view.source_horizon_seasons, Some(3));
        assert_eq!(view.forecast_horizon_seasons, Some(1));
        assert!(
            view.horizon_adjusted_arrival_probability.unwrap()
                < view.calibrated_arrival_probability
        );
        assert_ne!(
            view.calibrated_arrival_probability,
            view.empirical_arrival_rate
        );
    }

    #[test]
    fn arrival_input_is_derived_from_attention_free_study_components() {
        let study = build_prospect_development_study(
            ProspectDevelopmentStudyInput {
                player_id: 8_485_957,
                player: "Alberts Smits".to_owned(),
                organization: "NYR".to_owned(),
                position: "D".to_owned(),
                age: 18,
                nhl_games_played: 0,
                seasons: vec![
                    ProspectDevelopmentSeasonInput {
                        season: 20242025,
                        league: "Liiga".to_owned(),
                        games_played: 9,
                        goals: 1,
                        assists: 1,
                    },
                    ProspectDevelopmentSeasonInput {
                        season: 20252026,
                        league: "Liiga".to_owned(),
                        games_played: 38,
                        goals: 6,
                        assists: 7,
                    },
                ],
                opportunity: ProspectOpportunityStatus::Monitoring,
                availability: ProspectAvailabilityStatus::Unknown,
                attention_score: 0.5,
                attention_basis: "Neutral test attention.".to_owned(),
                evidence: vec![],
            },
            ProspectDevelopmentStudyConfig::default(),
        )
        .unwrap();
        let component = |id: &str| {
            study
                .components
                .iter()
                .find(|component| component.id == id)
                .unwrap()
                .score
        };
        let input = adapt_prospect_arrival_calibration_input(
            "nyr-smits-defense-hit",
            &study,
            20262027,
            ProspectArrivalCalibrationConfig::default(),
        )
        .unwrap();

        let expected = 100.0
            * (0.50 * component("production")
                + 0.25 * component("trajectory")
                + 0.25 * component("opportunity"));
        assert_eq!(input.event_id, "nyr-smits-defense-hit");
        assert_eq!(
            input.observed_signal_score,
            (expected * 100.0).round() / 100.0
        );
        assert_ne!(component("attention_gap"), component("opportunity"));
    }

    #[test]
    fn arrival_adapter_rejects_a_player_who_has_already_arrived() {
        let mut study = build_prospect_development_study(
            ProspectDevelopmentStudyInput {
                player_id: 8_480_001,
                player: "Already Arrived".to_owned(),
                organization: "SEA".to_owned(),
                position: "C".to_owned(),
                age: 21,
                nhl_games_played: 12,
                seasons: vec![ProspectDevelopmentSeasonInput {
                    season: 20252026,
                    league: "AHL".to_owned(),
                    games_played: 40,
                    goals: 12,
                    assists: 18,
                }],
                opportunity: ProspectOpportunityStatus::Monitoring,
                availability: ProspectAvailabilityStatus::Unknown,
                attention_score: 0.5,
                attention_basis: "Neutral test attention.".to_owned(),
                evidence: vec![],
            },
            ProspectDevelopmentStudyConfig::default(),
        )
        .unwrap();
        // Preserve the explicit authoritative workload even if the study
        // builder evolves its presentation fields.
        study.nhl_games_played = 12;
        let error = adapt_prospect_arrival_calibration_input(
            "already-arrived",
            &study,
            20262027,
            ProspectArrivalCalibrationConfig::default(),
        )
        .unwrap_err();
        assert!(error.contains("already has 12 NHL games"));
    }

    #[test]
    fn league_calibration_retains_every_team_and_player_failure() {
        let study = build_prospect_development_study(
            ProspectDevelopmentStudyInput {
                player_id: 8_485_957,
                player: "Alberts Smits".to_owned(),
                organization: "NYR".to_owned(),
                position: "D".to_owned(),
                age: 18,
                nhl_games_played: 0,
                seasons: vec![
                    ProspectDevelopmentSeasonInput {
                        season: 20242025,
                        league: "Liiga".to_owned(),
                        games_played: 9,
                        goals: 1,
                        assists: 1,
                    },
                    ProspectDevelopmentSeasonInput {
                        season: 20252026,
                        league: "Liiga".to_owned(),
                        games_played: 38,
                        goals: 6,
                        assists: 7,
                    },
                ],
                opportunity: ProspectOpportunityStatus::Monitoring,
                availability: ProspectAvailabilityStatus::Unknown,
                attention_score: 0.5,
                attention_basis: "Neutral test attention.".to_owned(),
                evidence: vec![],
            },
            ProspectDevelopmentStudyConfig::default(),
        )
        .unwrap();
        let mut historical_target = study.clone();
        historical_target.player_id = 1_000;
        historical_target.player = "Historical 1000".to_owned();
        historical_target.organization = "SEA".to_owned();

        let view = calibrate_prospect_arrival_league(
            vec!["SEA".to_owned(), "NYR".to_owned(), "VGK".to_owned()],
            vec![study, historical_target],
            vec![],
            None,
            20262027,
            &board(),
            ProspectArrivalCalibrationConfig {
                maximum_mean_signal_distance: 100.0,
                ..ProspectArrivalCalibrationConfig::default()
            },
        )
        .unwrap();

        assert_eq!(view.organizations_requested, 3);
        assert_eq!(view.organizations_represented, 3);
        assert_eq!(view.target_skaters, 2);
        assert_eq!(view.calibrated_skaters, 1);
        assert_eq!(view.excluded_skaters, 1);
        assert_eq!(view.teams[0].organization, "NYR");
        assert_eq!(view.teams[1].organization, "SEA");
        assert_eq!(view.teams[1].exclusions.len(), 1);
        assert_eq!(view.teams[2].organization, "VGK");
        assert_eq!(view.teams[2].target_skaters, 0);
    }

    #[test]
    fn league_calibration_reconciles_source_control_exclusions() {
        let view = calibrate_prospect_arrival_league(
            vec!["NYR".to_owned(), "SEA".to_owned()],
            vec![],
            vec![ProspectArrivalLeagueSourceExclusionInput {
                organization: "NYR".to_owned(),
                player_id: 8_485_957,
                player: "Alberts Smits".to_owned(),
                reason: "current organization control is unsupported".to_owned(),
            }],
            Some(ProspectArrivalLeaguePopulationAuthorityView {
                source_package_fingerprint: "sealed-source-package".to_owned(),
                population_complete: false,
                supplied_studies: 1,
                controlled_studies: 0,
                control_exclusions: 1,
            }),
            20262027,
            &board(),
            ProspectArrivalCalibrationConfig::default(),
        )
        .unwrap();

        assert_eq!(view.target_skaters, 1);
        assert_eq!(view.calibrated_skaters, 0);
        assert_eq!(view.excluded_skaters, 1);
        assert_eq!(view.teams[0].target_skaters, 1);
        assert_eq!(view.teams[0].exclusions[0].player, "Alberts Smits");
        assert_eq!(
            view.population_authority
                .as_ref()
                .unwrap()
                .controlled_studies,
            0
        );
    }

    #[test]
    fn arrival_refuses_self_leakage_and_future_outcomes() {
        let input = ProspectArrivalCalibrationInput {
            schema: PROSPECT_ARRIVAL_CALIBRATION_INPUT_SCHEMA.to_owned(),
            event_id: "event".to_owned(),
            player_id: 1_000,
            player: "Historical 1000".to_owned(),
            position: "D".to_owned(),
            observed_signal_score: 50.0,
            forecast_season: 20262027,
            config: ProspectArrivalCalibrationConfig::default(),
        };
        assert!(calibrate_prospect_arrival(input.clone(), &board())
            .unwrap_err()
            .contains("cannot appear"));
        let mut future = input;
        future.player_id = 8_485_957;
        future.forecast_season = 20252026;
        assert!(calibrate_prospect_arrival(future, &board())
            .unwrap_err()
            .contains("forecast horizon"));
    }

    #[test]
    fn cumulative_probability_is_aligned_to_the_forecast_horizon() {
        let one_year = adjust_cumulative_probability(0.573519, 3, 1);
        assert!((one_year - 0.2472803921).abs() < 1e-10);
        assert_eq!(adjust_cumulative_probability(0.573519, 3, 3), 0.573519);

        let mut mixed = board();
        mixed.programs[0].player_results[0].horizon_seasons = 2;
        let error = calibrate_prospect_arrival(
            ProspectArrivalCalibrationInput {
                schema: PROSPECT_ARRIVAL_CALIBRATION_INPUT_SCHEMA.to_owned(),
                event_id: "smits".to_owned(),
                player_id: 8_485_957,
                player: "Alberts Smits".to_owned(),
                position: "D".to_owned(),
                observed_signal_score: 29.06,
                forecast_season: 20262027,
                config: ProspectArrivalCalibrationConfig::default(),
            },
            &mixed,
        )
        .unwrap_err();
        assert!(error.contains("one common historical outcome horizon"));
    }
}
