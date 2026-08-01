//! Historical prospect-to-NHL conversion measurement.
//!
//! This contract compares a frozen prospect baseline with later observed NHL
//! participation, role, and optional performance. It measures realized value
//! from supplied cohorts; it does not infer causal development quality.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::career_history::{CareerGameType, CareerHistory};

use super::prospect_study::{
    ProspectDevelopmentStudyView, ProspectGoalieDevelopmentStudyView, ProspectStudyEvidenceInput,
    PROSPECT_DEVELOPMENT_STUDY_SCHEMA, PROSPECT_GOALIE_DEVELOPMENT_STUDY_SCHEMA,
    PROSPECT_PROGRAM_SCORING_METHOD,
};

pub const PROSPECT_CONVERSION_INPUT_SCHEMA: &str = "prospect_conversion_input.v2";
pub const PROSPECT_CONVERSION_BOARD_SCHEMA: &str = "prospect_conversion_board.v2";
pub const PROSPECT_CONVERSION_PERFORMANCE_SCHEMA: &str = "prospect_conversion_performance.v2";
pub const PROSPECT_CONVERSION_METHOD: &str = "prospect_conversion_observed.v2";
pub const PROSPECT_NHL_PERFORMANCE_METHOD: &str = "position_normalized_nhl_horizon.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectConversionDisposition {
    Retained,
    Traded,
    Departed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProspectConversionRankBlocker {
    InsufficientCohort { observed: usize, required: usize },
    LowBaselineConfidence { observed: f64, required: f64 },
    LowOutcomeCoverage { observed: f64, required: f64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectConversionBaselineInput {
    pub player_id: u32,
    pub player: String,
    pub organization: String,
    pub position: String,
    pub baseline_season: u32,
    /// Attention-free 0..100 prospect signal frozen at the baseline season.
    pub observed_signal_score: f64,
    /// 0..1 workload confidence attached to the frozen prospect study.
    pub workload_confidence: f64,
    pub evidence: Vec<ProspectStudyEvidenceInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectConversionPerformanceInput {
    pub player_id: u32,
    pub score: f64,
    pub basis: String,
    pub position_group: String,
    pub games_played: u32,
    pub sample_confidence: f64,
    pub raw_quality_score: f64,
    pub components: Vec<ProspectNhlPerformanceComponentView>,
    pub evidence: Vec<ProspectStudyEvidenceInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectConversionPerformanceDocument {
    pub schema: String,
    pub method: String,
    pub baseline_season: u32,
    pub through_season: u32,
    pub players: usize,
    pub source_coverage: f64,
    pub scores: Vec<ProspectConversionPerformanceInput>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectNhlPerformanceComponentView {
    pub metric: String,
    pub raw_value: f64,
    pub normalized_score: f64,
    pub weight: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectConversionSignalKind {
    Overall,
    Production,
    Trajectory,
    Opportunity,
    WorkloadConfidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectConversionResultClass {
    ExpectedHit,
    Breakout,
    Miss,
    Developing,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectConversionSignalInput {
    pub player_id: u32,
    /// Frozen 0..1 component scores from the baseline prospect study.
    pub production_score: f64,
    pub trajectory_score: f64,
    pub opportunity_score: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectConversionCalibrationBandView {
    pub players: usize,
    pub arrival_rate: f64,
    pub established_rate: f64,
    pub mean_role_score: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectConversionSignalCalibrationView {
    pub signal: ProspectConversionSignalKind,
    pub sample_size: usize,
    pub informative: bool,
    pub arrival_correlation: Option<f64>,
    pub established_correlation: Option<f64>,
    pub role_correlation: Option<f64>,
    pub bottom_quartile: Option<ProspectConversionCalibrationBandView>,
    pub top_quartile: Option<ProspectConversionCalibrationBandView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectNhlOutcomeInput {
    pub player_id: u32,
    pub through_season: u32,
    pub nhl_games_played: u32,
    pub nhl_toi_seconds: u64,
    /// Optional canonical 0..100 NHL performance measure. Missing performance
    /// reduces outcome coverage and can leave the organization unranked.
    pub performance_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub performance_basis: Option<String>,
    pub disposition: ProspectConversionDisposition,
    pub evidence: Vec<ProspectStudyEvidenceInput>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProspectConversionConfig {
    pub minimum_horizon_seasons: u8,
    pub skater_established_games: u32,
    pub goalie_established_games: u32,
    pub forward_role_seconds_per_game: u32,
    pub defense_role_seconds_per_game: u32,
    pub goalie_role_seconds_per_game: u32,
    pub arrival_weight: f64,
    pub role_weight: f64,
    pub performance_weight: f64,
    pub baseline_floor: f64,
    pub maximum_efficiency_index: f64,
    pub minimum_rankable_players: usize,
    pub minimum_rankable_baseline_confidence: f64,
    pub minimum_rankable_outcome_coverage: f64,
}

impl Default for ProspectConversionConfig {
    fn default() -> Self {
        Self {
            minimum_horizon_seasons: 3,
            skater_established_games: 82,
            goalie_established_games: 40,
            forward_role_seconds_per_game: 900,
            defense_role_seconds_per_game: 1_080,
            goalie_role_seconds_per_game: 3_000,
            arrival_weight: 0.40,
            role_weight: 0.30,
            performance_weight: 0.30,
            baseline_floor: 20.0,
            maximum_efficiency_index: 200.0,
            minimum_rankable_players: 5,
            minimum_rankable_baseline_confidence: 0.50,
            minimum_rankable_outcome_coverage: 0.80,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectConversionInput {
    pub schema: String,
    pub baseline_basis: String,
    pub baselines: Vec<ProspectConversionBaselineInput>,
    pub outcomes: Vec<ProspectNhlOutcomeInput>,
    /// Optional frozen component scores used only for descriptive calibration.
    /// Older authored inputs may omit them; the overall and confidence rows
    /// remain available.
    #[serde(default)]
    pub calibration_signals: Vec<ProspectConversionSignalInput>,
    #[serde(default)]
    pub config: ProspectConversionConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectConversionMethodologyView {
    pub method: String,
    pub config: ProspectConversionConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectConversionPlayerView {
    pub player_id: u32,
    pub player: String,
    pub organization: String,
    pub position: String,
    pub baseline_season: u32,
    pub through_season: u32,
    pub horizon_seasons: u32,
    pub baseline_signal_score: f64,
    pub workload_confidence: f64,
    pub nhl_games_played: u32,
    pub nhl_toi_seconds: u64,
    pub arrival_score: f64,
    pub role_score: f64,
    pub performance_score: Option<f64>,
    pub performance_basis: Option<String>,
    pub realized_value_score: f64,
    pub outcome_coverage: f64,
    pub conversion_delta: f64,
    pub efficiency_index: f64,
    pub established: bool,
    pub result_class: ProspectConversionResultClass,
    pub disposition: ProspectConversionDisposition,
    pub evidence: Vec<ProspectStudyEvidenceInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectConversionOrganizationView {
    pub organization: String,
    pub players: usize,
    pub converted_players: usize,
    pub established_players: usize,
    pub retained_players: usize,
    pub traded_players: usize,
    pub expected_hits: usize,
    pub breakouts: usize,
    pub misses: usize,
    pub developing_players: usize,
    pub baseline_signal_score: f64,
    pub baseline_confidence: f64,
    pub realized_value_score: f64,
    pub conversion_delta: f64,
    pub efficiency_index: f64,
    pub outcome_coverage: f64,
    pub conversion_rank: Option<usize>,
    pub rank_blockers: Vec<ProspectConversionRankBlocker>,
    pub player_results: Vec<ProspectConversionPlayerView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectConversionBoardView {
    pub schema: String,
    pub source_schema: String,
    pub baseline_basis: String,
    pub methodology: ProspectConversionMethodologyView,
    pub baseline_seasons: Vec<u32>,
    pub through_season: u32,
    pub organizations: usize,
    pub players: usize,
    pub ranked_organizations: usize,
    pub signal_calibration: Vec<ProspectConversionSignalCalibrationView>,
    pub programs: Vec<ProspectConversionOrganizationView>,
    pub disclosures: Vec<String>,
}

/// Build the canonical NHL-performance authority used by conversion studies.
/// The score is position normalized, limited to the declared post-baseline
/// horizon, and reliability adjusted for games played. A complete official
/// history with no NHL games is an observed zero; a missing history is an
/// error and may not be silently imputed.
pub fn build_prospect_nhl_performance_document(
    studies: &[ProspectDevelopmentStudyView],
    goalie_studies: &[ProspectGoalieDevelopmentStudyView],
    histories: &[CareerHistory],
    baseline_season: u32,
    through_season: u32,
) -> Result<ProspectConversionPerformanceDocument, String> {
    let baseline_start = season_start_year(baseline_season)
        .ok_or_else(|| "invalid prospect performance baseline season".to_owned())?;
    let through_start = season_start_year(through_season)
        .ok_or_else(|| "invalid prospect performance through season".to_owned())?;
    if through_start <= baseline_start {
        return Err("prospect performance horizon must follow its baseline".to_owned());
    }

    let mut positions = BTreeMap::new();
    for study in studies {
        if study.schema != PROSPECT_DEVELOPMENT_STUDY_SCHEMA
            || positions
                .insert(study.player_id, study.position.clone())
                .is_some()
        {
            return Err("invalid or duplicate skater performance study".to_owned());
        }
    }
    for study in goalie_studies {
        if study.schema != PROSPECT_GOALIE_DEVELOPMENT_STUDY_SCHEMA
            || study.position != "G"
            || positions.insert(study.player_id, "G".to_owned()).is_some()
        {
            return Err("invalid or duplicate goalie performance study".to_owned());
        }
    }
    if positions.is_empty() {
        return Err("prospect performance requires frozen studies".to_owned());
    }
    let histories = histories
        .iter()
        .map(|history| (history.player_id, history))
        .collect::<BTreeMap<_, _>>();
    let mut scores = Vec::with_capacity(positions.len());
    for (player_id, position) in positions {
        let history = histories.get(&player_id).ok_or_else(|| {
            format!("missing official NHL career history for prospect {player_id}")
        })?;
        let stints = history
            .stints
            .iter()
            .filter(|stint| {
                stint.game_type == CareerGameType::Regular
                    && stint.league.as_str().eq_ignore_ascii_case("NHL")
                    && stint.season.0 > baseline_season
                    && stint.season.0 <= through_season
            })
            .collect::<Vec<_>>();
        let games_played = stints.iter().map(|stint| stint.gp).sum::<u32>();
        let group = position_group(&position).to_owned();
        let (raw_quality_score, components) = if games_played == 0 {
            (0.0, Vec::new())
        } else if group == "G" {
            goalie_performance_components(&stints)?
        } else {
            skater_performance_components(&stints, &group)?
        };
        let prior_games = if group == "G" { 20.0 } else { 30.0 };
        let sample_confidence = if games_played == 0 {
            1.0
        } else {
            f64::from(games_played) / (f64::from(games_played) + prior_games)
        };
        let score = if games_played == 0 {
            0.0
        } else {
            raw_quality_score * sample_confidence
        };
        scores.push(ProspectConversionPerformanceInput {
            player_id,
            score: round_score(score),
            basis: format!(
                "{}; {} NHL GP; sample confidence {:.3}",
                PROSPECT_NHL_PERFORMANCE_METHOD, games_played, sample_confidence
            ),
            position_group: group,
            games_played,
            sample_confidence: round_ratio(sample_confidence),
            raw_quality_score: round_score(raw_quality_score),
            components,
            evidence: vec![ProspectStudyEvidenceInput {
                label: "Official NHL player landing career statistics".to_owned(),
                source_url: format!("https://api-web.nhle.com/v1/player/{player_id}/landing"),
            }],
        });
    }
    scores.sort_by_key(|row| row.player_id);
    Ok(ProspectConversionPerformanceDocument {
        schema: PROSPECT_CONVERSION_PERFORMANCE_SCHEMA.to_owned(),
        method: PROSPECT_NHL_PERFORMANCE_METHOD.to_owned(),
        baseline_season,
        through_season,
        players: scores.len(),
        source_coverage: 1.0,
        scores,
        disclosures: vec![
            "Forward, defense, and goalie quality use separate fixed modern-horizon benchmarks; they are comparable 0..100 scores, not era-wide wins-above-replacement.".to_owned(),
            "Skater quality combines scoring, goal scoring, deployment, power-play production, and shot generation. Goalie quality combines save percentage, goals-against rate, starts, and shutouts.".to_owned(),
            "Observed quality is multiplied by GP/(GP+30) for skaters or GP/(GP+20) for goalies so small NHL samples cannot receive full credit.".to_owned(),
            "A complete official history with zero post-baseline NHL games scores zero. Missing official history is rejected rather than treated as zero.".to_owned(),
        ],
    })
}

fn skater_performance_components(
    stints: &[&crate::career_history::CareerStint],
    group: &str,
) -> Result<(f64, Vec<ProspectNhlPerformanceComponentView>), String> {
    let gp = stints.iter().map(|stint| stint.gp).sum::<u32>();
    let total = |metric: &str,
                 value: fn(&crate::career_history::CareerStint) -> Option<u32>|
     -> Result<u32, String> {
        if stints
            .iter()
            .any(|stint| stint.gp > 0 && value(stint).is_none())
        {
            return Err(format!(
                "missing NHL skater {metric} in performance horizon"
            ));
        }
        Ok(stints.iter().filter_map(|stint| value(stint)).sum::<u32>())
    };
    let points = total("points", |stint| stint.points)?;
    let goals = total("goals", |stint| stint.goals)?;
    let power_play_points = total("power-play points", |stint| stint.power_play_points)?;
    let shots = total("shots", |stint| stint.shots)?;
    let toi_games = stints
        .iter()
        .filter_map(|stint| stint.avg_toi_sec.map(|toi| (toi, stint.gp)))
        .collect::<Vec<_>>();
    if toi_games.iter().map(|(_, games)| games).sum::<u32>() != gp {
        return Err("missing NHL skater deployment in performance horizon".to_owned());
    }
    let average_toi = toi_games
        .iter()
        .map(|(toi, games)| f64::from(*toi) * f64::from(*games))
        .sum::<f64>()
        / f64::from(gp);
    let per_82 = |value: u32| 82.0 * f64::from(value) / f64::from(gp);
    let (point_target, goal_target, toi_target, pp_target, shot_target) = if group == "D" {
        (50.0, 18.0, 1_440.0, 25.0, 200.0)
    } else {
        (70.0, 35.0, 1_200.0, 25.0, 250.0)
    };
    let specs = [
        ("points_per_82", per_82(points), point_target, 0.35),
        ("goals_per_82", per_82(goals), goal_target, 0.20),
        ("average_toi_seconds", average_toi, toi_target, 0.20),
        (
            "power_play_points_per_82",
            per_82(power_play_points),
            pp_target,
            0.15,
        ),
        ("shots_per_82", per_82(shots), shot_target, 0.10),
    ];
    Ok(performance_components(&specs))
}

fn goalie_performance_components(
    stints: &[&crate::career_history::CareerStint],
) -> Result<(f64, Vec<ProspectNhlPerformanceComponentView>), String> {
    let gp = stints.iter().map(|stint| stint.gp).sum::<u32>();
    let starts = stints
        .iter()
        .filter_map(|stint| stint.games_started)
        .sum::<u32>();
    let shutouts = stints
        .iter()
        .filter_map(|stint| stint.shutouts)
        .sum::<u32>();
    let shots_against = stints
        .iter()
        .filter_map(|stint| stint.shots_against)
        .sum::<u32>();
    let weighted_saves = stints
        .iter()
        .filter_map(|stint| stint.save_pct.zip(stint.shots_against))
        .map(|(save_pct, shots)| f64::from(save_pct) * f64::from(shots))
        .sum::<f64>();
    let toi = stints
        .iter()
        .filter_map(|stint| stint.time_on_ice_sec)
        .sum::<u32>();
    let goals_against = stints
        .iter()
        .filter_map(|stint| stint.goals_against)
        .sum::<u32>();
    if shots_against == 0 || toi == 0 {
        return Err("missing NHL goalie rate inputs in performance horizon".to_owned());
    }
    let save_pct = weighted_saves / f64::from(shots_against);
    let gaa = 3_600.0 * f64::from(goals_against) / f64::from(toi);
    let start_share = f64::from(starts) / f64::from(gp);
    let shutout_rate = f64::from(shutouts) / f64::from(gp);
    let components = vec![
        goalie_component(
            "save_percentage",
            save_pct,
            normalize_range(save_pct, 0.880, 0.925),
            0.50,
        ),
        goalie_component(
            "goals_against_average",
            gaa,
            100.0 - normalize_range(gaa, 2.20, 4.00),
            0.25,
        ),
        goalie_component(
            "start_share",
            start_share,
            normalize_range(start_share, 0.0, 0.80),
            0.15,
        ),
        goalie_component(
            "shutouts_per_game",
            shutout_rate,
            normalize_range(shutout_rate, 0.0, 0.10),
            0.10,
        ),
    ];
    let score = components
        .iter()
        .map(|component| component.normalized_score * component.weight)
        .sum();
    Ok((score, components))
}

fn goalie_component(
    metric: &str,
    raw_value: f64,
    normalized_score: f64,
    weight: f64,
) -> ProspectNhlPerformanceComponentView {
    ProspectNhlPerformanceComponentView {
        metric: metric.to_owned(),
        raw_value: round_metric(raw_value),
        normalized_score: round_score(normalized_score),
        weight,
    }
}

fn performance_components(
    specs: &[(&str, f64, f64, f64)],
) -> (f64, Vec<ProspectNhlPerformanceComponentView>) {
    let components = specs
        .iter()
        .map(
            |(metric, raw_value, target, weight)| ProspectNhlPerformanceComponentView {
                metric: (*metric).to_owned(),
                raw_value: round_metric(*raw_value),
                normalized_score: round_score(normalize_range(*raw_value, 0.0, *target)),
                weight: *weight,
            },
        )
        .collect::<Vec<_>>();
    let score = components
        .iter()
        .map(|component| component.normalized_score * component.weight)
        .sum();
    (score, components)
}

fn normalize_range(value: f64, floor: f64, ceiling: f64) -> f64 {
    (100.0 * (value - floor) / (ceiling - floor)).clamp(0.0, 100.0)
}

fn round_metric(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

/// Adapt complete frozen prospect-study cohorts and official NHL landing
/// histories into the conversion contract. Only regular-season NHL stints
/// strictly after the baseline season and through the declared outcome season
/// count. Missing NHL TOI is rejected rather than converted into zero role.
pub fn adapt_prospect_conversion_input(
    studies: &[ProspectDevelopmentStudyView],
    goalie_studies: &[ProspectGoalieDevelopmentStudyView],
    histories: &[CareerHistory],
    baseline_season: u32,
    through_season: u32,
    performances: &[ProspectConversionPerformanceInput],
    config: ProspectConversionConfig,
) -> Result<ProspectConversionInput, String> {
    let baseline_start = season_start_year(baseline_season)
        .ok_or_else(|| "invalid prospect conversion baseline season".to_owned())?;
    let through_start = season_start_year(through_season)
        .ok_or_else(|| "invalid prospect conversion through season".to_owned())?;
    if through_start.saturating_sub(baseline_start) < u32::from(config.minimum_horizon_seasons) {
        return Err("prospect conversion adaptation horizon is too short".to_owned());
    }
    if studies.is_empty() && goalie_studies.is_empty() {
        return Err("prospect conversion adaptation requires frozen studies".to_owned());
    }

    let mut history_by_player = BTreeMap::new();
    for history in histories {
        if history.player_id == 0
            || history_by_player
                .insert(history.player_id, history)
                .is_some()
        {
            return Err("invalid or duplicate prospect conversion career history".to_owned());
        }
    }
    let mut performance_by_player = BTreeMap::new();
    for performance in performances {
        if performance.player_id == 0
            || !performance.score.is_finite()
            || !(0.0..=100.0).contains(&performance.score)
            || performance.basis.trim().is_empty()
            || !matches!(performance.position_group.as_str(), "F" | "D" | "G")
            || !performance.sample_confidence.is_finite()
            || !(0.0..=1.0).contains(&performance.sample_confidence)
            || !performance.raw_quality_score.is_finite()
            || !(0.0..=100.0).contains(&performance.raw_quality_score)
            || performance.evidence.is_empty()
            || performance_by_player
                .insert(performance.player_id, performance)
                .is_some()
        {
            return Err("invalid or duplicate prospect conversion performance".to_owned());
        }
    }

    let mut sources = Vec::with_capacity(studies.len() + goalie_studies.len());
    for study in studies {
        if study.schema != PROSPECT_DEVELOPMENT_STUDY_SCHEMA
            || study.seasons.is_empty()
            || study
                .seasons
                .iter()
                .any(|season| season.season > baseline_season)
        {
            return Err("invalid or post-baseline skater study".to_owned());
        }
        sources.push(ConversionBaselineSource {
            player_id: study.player_id,
            player: &study.player,
            organization: &study.organization,
            position: &study.position,
            workload_confidence: study.workload_confidence,
            components: &study.components,
            evidence: &study.evidence,
        });
    }
    for study in goalie_studies {
        if study.schema != PROSPECT_GOALIE_DEVELOPMENT_STUDY_SCHEMA
            || study.position != "G"
            || study.seasons.is_empty()
            || study
                .seasons
                .iter()
                .any(|season| season.season > baseline_season)
        {
            return Err("invalid or post-baseline goalie study".to_owned());
        }
        sources.push(ConversionBaselineSource {
            player_id: study.player_id,
            player: &study.player,
            organization: &study.organization,
            position: &study.position,
            workload_confidence: study.workload_confidence,
            components: &study.components,
            evidence: &study.evidence,
        });
    }

    let mut player_ids = BTreeSet::new();
    let mut baselines = Vec::with_capacity(sources.len());
    let mut outcomes = Vec::with_capacity(sources.len());
    let mut calibration_signals = Vec::with_capacity(sources.len());
    for source in sources {
        if source.player_id == 0 || !player_ids.insert(source.player_id) {
            return Err("invalid or duplicate prospect conversion study player".to_owned());
        }
        let history = history_by_player.get(&source.player_id).ok_or_else(|| {
            format!(
                "missing official NHL career history for prospect {}",
                source.player_id
            )
        })?;
        let observed_signal_score = observed_signal_score(source.components)?;
        calibration_signals.push(ProspectConversionSignalInput {
            player_id: source.player_id,
            production_score: component_score(source.components, "production")?,
            trajectory_score: component_score(source.components, "trajectory")?,
            opportunity_score: component_score(source.components, "opportunity")?,
        });
        baselines.push(ProspectConversionBaselineInput {
            player_id: source.player_id,
            player: source.player.clone(),
            organization: source.organization.clone(),
            position: source.position.clone(),
            baseline_season,
            observed_signal_score,
            workload_confidence: source.workload_confidence,
            evidence: source.evidence.to_vec(),
        });

        let goalie = position_group(source.position) == "G";
        let mut nhl_games_played = 0_u32;
        let mut nhl_toi_seconds = 0_u64;
        for stint in history.stints.iter().filter(|stint| {
            stint.game_type == CareerGameType::Regular
                && stint.league.as_str().eq_ignore_ascii_case("NHL")
                && stint.season.0 > baseline_season
                && stint.season.0 <= through_season
        }) {
            nhl_games_played = nhl_games_played.saturating_add(stint.gp);
            let stint_toi = if goalie {
                stint.time_on_ice_sec.map(u64::from)
            } else {
                stint
                    .avg_toi_sec
                    .map(|seconds| u64::from(seconds) * u64::from(stint.gp))
            };
            if stint.gp > 0 && stint_toi.is_none() {
                return Err(format!(
                    "missing NHL time on ice for prospect {} season {}",
                    source.player_id, stint.season.0
                ));
            }
            nhl_toi_seconds = nhl_toi_seconds.saturating_add(stint_toi.unwrap_or(0));
        }
        let performance = performance_by_player.get(&source.player_id);
        outcomes.push(ProspectNhlOutcomeInput {
            player_id: source.player_id,
            through_season,
            nhl_games_played,
            nhl_toi_seconds,
            performance_score: performance.map(|row| row.score),
            performance_basis: performance.map(|row| row.basis.clone()),
            disposition: ProspectConversionDisposition::Unknown,
            evidence: vec![ProspectStudyEvidenceInput {
                label: "Official NHL player landing career totals".to_owned(),
                source_url: format!(
                    "https://api-web.nhle.com/v1/player/{}/landing",
                    source.player_id
                ),
            }],
        });
    }
    if performance_by_player
        .keys()
        .any(|player_id| !player_ids.contains(player_id))
    {
        return Err("prospect conversion performance contains an unknown player".to_owned());
    }
    baselines.sort_by_key(|row| row.player_id);
    outcomes.sort_by_key(|row| row.player_id);
    let input = ProspectConversionInput {
        schema: PROSPECT_CONVERSION_INPUT_SCHEMA.to_owned(),
        baseline_basis: PROSPECT_PROGRAM_SCORING_METHOD.to_owned(),
        baselines,
        outcomes,
        calibration_signals,
        config,
    };
    build_prospect_conversion_board(&input)?;
    Ok(input)
}

struct ConversionBaselineSource<'a> {
    player_id: u32,
    player: &'a String,
    organization: &'a String,
    position: &'a String,
    workload_confidence: f64,
    components: &'a [super::prospect_study::ProspectSignalComponentView],
    evidence: &'a [ProspectStudyEvidenceInput],
}

pub fn build_prospect_conversion_board(
    input: &ProspectConversionInput,
) -> Result<ProspectConversionBoardView, String> {
    validate_config(input.config)?;
    if input.schema != PROSPECT_CONVERSION_INPUT_SCHEMA
        || input.baseline_basis.trim().is_empty()
        || input.baselines.is_empty()
    {
        return Err("invalid prospect conversion input".to_owned());
    }

    let mut baselines = BTreeMap::new();
    for baseline in &input.baselines {
        if baseline.player_id == 0
            || baseline.player.trim().is_empty()
            || baseline.organization.trim().is_empty()
            || baseline.position.trim().is_empty()
            || !matches!(
                baseline.position.trim().to_ascii_uppercase().as_str(),
                "F" | "C" | "LW" | "RW" | "W" | "D" | "LD" | "RD" | "G"
            )
            || season_start_year(baseline.baseline_season).is_none()
            || !baseline.observed_signal_score.is_finite()
            || !(0.0..=100.0).contains(&baseline.observed_signal_score)
            || !baseline.workload_confidence.is_finite()
            || !(0.0..=1.0).contains(&baseline.workload_confidence)
            || baseline.evidence.is_empty()
            || baseline.evidence.iter().any(|item| {
                item.label.trim().is_empty()
                    || !(item.source_url.starts_with("https://")
                        || item.source_url.starts_with("http://"))
            })
            || baselines.insert(baseline.player_id, baseline).is_some()
        {
            return Err("invalid or duplicate prospect conversion baseline".to_owned());
        }
    }

    let mut outcomes = BTreeMap::new();
    for outcome in &input.outcomes {
        if outcome.player_id == 0
            || season_start_year(outcome.through_season).is_none()
            || outcome
                .performance_score
                .is_some_and(|score| !score.is_finite() || !(0.0..=100.0).contains(&score))
            || (!matches!(
                (&outcome.performance_score, &outcome.performance_basis),
                (Some(_), Some(basis)) if !basis.trim().is_empty()
            ) && !matches!(
                (&outcome.performance_score, &outcome.performance_basis),
                (None, None)
            ))
            || outcome.evidence.is_empty()
            || outcome.evidence.iter().any(|item| {
                item.label.trim().is_empty()
                    || !(item.source_url.starts_with("https://")
                        || item.source_url.starts_with("http://"))
            })
            || outcomes.insert(outcome.player_id, outcome).is_some()
        {
            return Err("invalid or duplicate prospect NHL outcome".to_owned());
        }
    }
    if baselines.len() != outcomes.len()
        || baselines
            .keys()
            .any(|player_id| !outcomes.contains_key(player_id))
    {
        return Err(
            "prospect conversion baselines and outcomes must cover identical players".to_owned(),
        );
    }

    let mut calibration_signals = BTreeMap::new();
    for signal in &input.calibration_signals {
        if signal.player_id == 0
            || [
                signal.production_score,
                signal.trajectory_score,
                signal.opportunity_score,
            ]
            .iter()
            .any(|score| !score.is_finite() || !(0.0..=1.0).contains(score))
            || calibration_signals
                .insert(signal.player_id, signal)
                .is_some()
        {
            return Err("invalid or duplicate prospect conversion calibration signal".to_owned());
        }
    }
    if !calibration_signals.is_empty()
        && (calibration_signals.len() != baselines.len()
            || baselines
                .keys()
                .any(|player_id| !calibration_signals.contains_key(player_id)))
    {
        return Err(
            "prospect conversion calibration signals must cover every baseline player".to_owned(),
        );
    }

    let through_seasons = outcomes
        .values()
        .map(|outcome| outcome.through_season)
        .collect::<BTreeSet<_>>();
    if through_seasons.len() != 1 {
        return Err("prospect conversion outcomes require one common through season".to_owned());
    }
    let through_season = *through_seasons.iter().next().expect("validated outcomes");
    let through_start = season_start_year(through_season).expect("validated season");
    let mut baseline_seasons = BTreeSet::new();
    let mut by_organization = BTreeMap::<String, Vec<ProspectConversionPlayerView>>::new();

    for (&player_id, baseline) in &baselines {
        let outcome = outcomes[&player_id];
        let baseline_start = season_start_year(baseline.baseline_season).expect("validated season");
        let horizon_seasons = through_start.saturating_sub(baseline_start);
        if horizon_seasons < u32::from(input.config.minimum_horizon_seasons) {
            return Err(format!(
                "prospect {} has only {} completed horizon season(s)",
                player_id, horizon_seasons
            ));
        }
        baseline_seasons.insert(baseline.baseline_season);
        let goalie = position_group(&baseline.position) == "G";
        let established_games = if goalie {
            input.config.goalie_established_games
        } else {
            input.config.skater_established_games
        };
        let role_benchmark = match position_group(&baseline.position) {
            "G" => input.config.goalie_role_seconds_per_game,
            "D" => input.config.defense_role_seconds_per_game,
            _ => input.config.forward_role_seconds_per_game,
        };
        let arrival_score =
            100.0 * (f64::from(outcome.nhl_games_played) / f64::from(established_games)).min(1.0);
        let seconds_per_game = if outcome.nhl_games_played == 0 {
            0.0
        } else {
            outcome.nhl_toi_seconds as f64 / f64::from(outcome.nhl_games_played)
        };
        let role_score = 100.0 * (seconds_per_game / f64::from(role_benchmark)).min(1.0);
        let available_weight = input.config.arrival_weight
            + input.config.role_weight
            + if outcome.performance_score.is_some() {
                input.config.performance_weight
            } else {
                0.0
            };
        let realized_value_score = (input.config.arrival_weight * arrival_score
            + input.config.role_weight * role_score
            + input.config.performance_weight * outcome.performance_score.unwrap_or(0.0))
            / available_weight;
        let outcome_coverage = available_weight;
        let efficiency_index = (100.0 * realized_value_score
            / baseline
                .observed_signal_score
                .max(input.config.baseline_floor))
        .min(input.config.maximum_efficiency_index);
        let result_class =
            conversion_result_class(baseline.observed_signal_score, realized_value_score);
        by_organization
            .entry(baseline.organization.clone())
            .or_default()
            .push(ProspectConversionPlayerView {
                player_id,
                player: baseline.player.clone(),
                organization: baseline.organization.clone(),
                position: baseline.position.clone(),
                baseline_season: baseline.baseline_season,
                through_season,
                horizon_seasons,
                baseline_signal_score: round_score(baseline.observed_signal_score),
                workload_confidence: baseline.workload_confidence,
                nhl_games_played: outcome.nhl_games_played,
                nhl_toi_seconds: outcome.nhl_toi_seconds,
                arrival_score: round_score(arrival_score),
                role_score: round_score(role_score),
                performance_score: outcome.performance_score.map(round_score),
                performance_basis: outcome.performance_basis.clone(),
                realized_value_score: round_score(realized_value_score),
                outcome_coverage: round_ratio(outcome_coverage),
                conversion_delta: round_score(
                    realized_value_score - baseline.observed_signal_score,
                ),
                efficiency_index: round_score(efficiency_index),
                established: outcome.nhl_games_played >= established_games,
                result_class,
                disposition: outcome.disposition,
                evidence: outcome.evidence.clone(),
            });
    }

    let mut programs = Vec::with_capacity(by_organization.len());
    for (organization, mut players) in by_organization {
        players.sort_by(|left, right| {
            right
                .realized_value_score
                .total_cmp(&left.realized_value_score)
                .then_with(|| left.player.cmp(&right.player))
                .then_with(|| left.player_id.cmp(&right.player_id))
        });
        let confidence_weight = players
            .iter()
            .map(|player| player.workload_confidence)
            .sum::<f64>();
        let baseline_signal_score = weighted_mean(&players, confidence_weight, |player| {
            player.baseline_signal_score
        });
        let realized_value_score = weighted_mean(&players, confidence_weight, |player| {
            player.realized_value_score
        });
        let outcome_coverage = weighted_mean(&players, confidence_weight, |player| {
            player.outcome_coverage
        });
        let baseline_confidence = players
            .iter()
            .map(|player| player.workload_confidence)
            .sum::<f64>()
            / players.len() as f64;
        let efficiency_index = (100.0 * realized_value_score
            / baseline_signal_score.max(input.config.baseline_floor))
        .min(input.config.maximum_efficiency_index);
        let mut rank_blockers = Vec::new();
        if players.len() < input.config.minimum_rankable_players {
            rank_blockers.push(ProspectConversionRankBlocker::InsufficientCohort {
                observed: players.len(),
                required: input.config.minimum_rankable_players,
            });
        }
        if baseline_confidence < input.config.minimum_rankable_baseline_confidence {
            rank_blockers.push(ProspectConversionRankBlocker::LowBaselineConfidence {
                observed: round_ratio(baseline_confidence),
                required: input.config.minimum_rankable_baseline_confidence,
            });
        }
        if outcome_coverage < input.config.minimum_rankable_outcome_coverage {
            rank_blockers.push(ProspectConversionRankBlocker::LowOutcomeCoverage {
                observed: round_ratio(outcome_coverage),
                required: input.config.minimum_rankable_outcome_coverage,
            });
        }
        programs.push(ProspectConversionOrganizationView {
            organization,
            players: players.len(),
            converted_players: players
                .iter()
                .filter(|player| player.nhl_games_played > 0)
                .count(),
            established_players: players.iter().filter(|player| player.established).count(),
            retained_players: players
                .iter()
                .filter(|player| player.disposition == ProspectConversionDisposition::Retained)
                .count(),
            traded_players: players
                .iter()
                .filter(|player| player.disposition == ProspectConversionDisposition::Traded)
                .count(),
            expected_hits: players
                .iter()
                .filter(|player| player.result_class == ProspectConversionResultClass::ExpectedHit)
                .count(),
            breakouts: players
                .iter()
                .filter(|player| player.result_class == ProspectConversionResultClass::Breakout)
                .count(),
            misses: players
                .iter()
                .filter(|player| player.result_class == ProspectConversionResultClass::Miss)
                .count(),
            developing_players: players
                .iter()
                .filter(|player| player.result_class == ProspectConversionResultClass::Developing)
                .count(),
            baseline_signal_score: round_score(baseline_signal_score),
            baseline_confidence: round_ratio(baseline_confidence),
            realized_value_score: round_score(realized_value_score),
            conversion_delta: round_score(realized_value_score - baseline_signal_score),
            efficiency_index: round_score(efficiency_index),
            outcome_coverage: round_ratio(outcome_coverage),
            conversion_rank: None,
            rank_blockers,
            player_results: players,
        });
    }
    let mut rankable = programs
        .iter()
        .enumerate()
        .filter(|(_, program)| program.rank_blockers.is_empty())
        .map(|(index, program)| (index, program.efficiency_index))
        .collect::<Vec<_>>();
    rankable.sort_by(|left, right| {
        right.1.total_cmp(&left.1).then_with(|| {
            programs[left.0]
                .organization
                .cmp(&programs[right.0].organization)
        })
    });
    for (offset, (index, _)) in rankable.iter().enumerate() {
        programs[*index].conversion_rank = Some(offset + 1);
    }
    programs.sort_by(|left, right| {
        left.conversion_rank
            .unwrap_or(usize::MAX)
            .cmp(&right.conversion_rank.unwrap_or(usize::MAX))
            .then_with(|| left.organization.cmp(&right.organization))
    });
    let signal_calibration = build_signal_calibration(&baselines, &calibration_signals, &programs);

    Ok(ProspectConversionBoardView {
        schema: PROSPECT_CONVERSION_BOARD_SCHEMA.to_owned(),
        source_schema: PROSPECT_CONVERSION_INPUT_SCHEMA.to_owned(),
        baseline_basis: input.baseline_basis.clone(),
        methodology: ProspectConversionMethodologyView {
            method: PROSPECT_CONVERSION_METHOD.to_owned(),
            config: input.config,
        },
        baseline_seasons: baseline_seasons.into_iter().collect(),
        through_season,
        organizations: programs.len(),
        players: input.baselines.len(),
        ranked_organizations: rankable.len(),
        signal_calibration,
        programs,
        disclosures: vec![
            "Conversion compares a frozen attention-free prospect signal with later observed NHL arrival, role, and optional canonical performance evidence.".to_owned(),
            "Performance is never imputed. Missing performance lowers outcome coverage, and organizations below the configured coverage floor remain unranked.".to_owned(),
            "Small or weakly observed cohorts remain unranked under the configured player-count and baseline-confidence floors; rank blockers remain visible on every organization row.".to_owned(),
            "The efficiency index compares realized value with baseline signal using a disclosed denominator floor and cap; it rewards over-delivery without allowing tiny baselines to explode.".to_owned(),
            "Retention and trade disposition are reported but do not add value without a separately sourced return model.".to_owned(),
            "This is cohort conversion, not proof that an organization caused or prevented an individual outcome.".to_owned(),
            "Signal calibration is descriptive association over this frozen cohort. Correlations and quartile outcome rates do not establish causation; constant signals are marked non-informative rather than assigned a numeric result.".to_owned(),
            "Player result classes use disclosed 60-point baseline and realized-value thresholds: expected hit (both strong), breakout (realized only), miss (baseline only), and developing (neither). They are comparison labels, not scouting verdicts.".to_owned(),
        ],
    })
}

fn conversion_result_class(
    baseline_score: f64,
    realized_score: f64,
) -> ProspectConversionResultClass {
    match (baseline_score >= 60.0, realized_score >= 60.0) {
        (true, true) => ProspectConversionResultClass::ExpectedHit,
        (false, true) => ProspectConversionResultClass::Breakout,
        (true, false) => ProspectConversionResultClass::Miss,
        (false, false) => ProspectConversionResultClass::Developing,
    }
}

#[allow(clippy::type_complexity)]
fn build_signal_calibration(
    baselines: &BTreeMap<u32, &ProspectConversionBaselineInput>,
    component_signals: &BTreeMap<u32, &ProspectConversionSignalInput>,
    programs: &[ProspectConversionOrganizationView],
) -> Vec<ProspectConversionSignalCalibrationView> {
    let results = programs
        .iter()
        .flat_map(|program| &program.player_results)
        .map(|player| (player.player_id, player))
        .collect::<BTreeMap<_, _>>();
    let mut signal_sets = vec![
        (
            ProspectConversionSignalKind::Overall,
            baselines
                .iter()
                .map(|(player_id, row)| (*player_id, row.observed_signal_score / 100.0))
                .collect::<Vec<_>>(),
        ),
        (
            ProspectConversionSignalKind::WorkloadConfidence,
            baselines
                .iter()
                .map(|(player_id, row)| (*player_id, row.workload_confidence))
                .collect::<Vec<_>>(),
        ),
    ];
    if !component_signals.is_empty() {
        let components: [(
            ProspectConversionSignalKind,
            fn(&ProspectConversionSignalInput) -> f64,
        ); 3] = [
            (
                ProspectConversionSignalKind::Production,
                |row: &ProspectConversionSignalInput| row.production_score,
            ),
            (
                ProspectConversionSignalKind::Trajectory,
                |row: &ProspectConversionSignalInput| row.trajectory_score,
            ),
            (
                ProspectConversionSignalKind::Opportunity,
                |row: &ProspectConversionSignalInput| row.opportunity_score,
            ),
        ];
        for (kind, value) in components {
            signal_sets.push((
                kind,
                component_signals
                    .iter()
                    .map(|(player_id, row)| (*player_id, value(row)))
                    .collect(),
            ));
        }
    }
    signal_sets.sort_by_key(|(kind, _)| *kind);
    signal_sets
        .into_iter()
        .map(|(kind, values)| signal_calibration_row(kind, values, &results))
        .collect()
}

fn signal_calibration_row(
    signal: ProspectConversionSignalKind,
    mut values: Vec<(u32, f64)>,
    results: &BTreeMap<u32, &ProspectConversionPlayerView>,
) -> ProspectConversionSignalCalibrationView {
    values.sort_by(|left, right| {
        left.1
            .total_cmp(&right.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    let informative = values.len() >= 4
        && values
            .first()
            .zip(values.last())
            .is_some_and(|(low, high)| high.1 - low.1 > f64::EPSILON);
    let x = values.iter().map(|(_, value)| *value).collect::<Vec<_>>();
    let outcome = |player_id: u32| results[&player_id];
    let arrival = values
        .iter()
        .map(|(player_id, _)| {
            if outcome(*player_id).nhl_games_played > 0 {
                1.0
            } else {
                0.0
            }
        })
        .collect::<Vec<_>>();
    let established = values
        .iter()
        .map(|(player_id, _)| {
            if outcome(*player_id).established {
                1.0
            } else {
                0.0
            }
        })
        .collect::<Vec<_>>();
    let role = values
        .iter()
        .map(|(player_id, _)| outcome(*player_id).role_score)
        .collect::<Vec<_>>();
    let (bottom_quartile, top_quartile) = if informative {
        let last = values.len() - 1;
        let low_cutoff = values[last / 4].1;
        let high_cutoff = values[(3 * last) / 4].1;
        if low_cutoff < high_cutoff {
            (
                Some(calibration_band(
                    values
                        .iter()
                        .filter(|(_, value)| *value <= low_cutoff)
                        .map(|(player_id, _)| outcome(*player_id)),
                )),
                Some(calibration_band(
                    values
                        .iter()
                        .filter(|(_, value)| *value >= high_cutoff)
                        .map(|(player_id, _)| outcome(*player_id)),
                )),
            )
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };
    ProspectConversionSignalCalibrationView {
        signal,
        sample_size: values.len(),
        informative,
        arrival_correlation: pearson_correlation(&x, &arrival).map(round_ratio),
        established_correlation: pearson_correlation(&x, &established).map(round_ratio),
        role_correlation: pearson_correlation(&x, &role).map(round_ratio),
        bottom_quartile,
        top_quartile,
    }
}

fn calibration_band<'a>(
    players: impl Iterator<Item = &'a ProspectConversionPlayerView>,
) -> ProspectConversionCalibrationBandView {
    let players = players.collect::<Vec<_>>();
    let count = players.len();
    ProspectConversionCalibrationBandView {
        players: count,
        arrival_rate: round_ratio(
            players
                .iter()
                .filter(|player| player.nhl_games_played > 0)
                .count() as f64
                / count as f64,
        ),
        established_rate: round_ratio(
            players.iter().filter(|player| player.established).count() as f64 / count as f64,
        ),
        mean_role_score: round_score(
            players.iter().map(|player| player.role_score).sum::<f64>() / count as f64,
        ),
    }
}

fn pearson_correlation(left: &[f64], right: &[f64]) -> Option<f64> {
    if left.len() != right.len() || left.len() < 2 {
        return None;
    }
    let left_mean = left.iter().sum::<f64>() / left.len() as f64;
    let right_mean = right.iter().sum::<f64>() / right.len() as f64;
    let (mut numerator, mut left_variance, mut right_variance) = (0.0, 0.0, 0.0);
    for (left, right) in left.iter().zip(right) {
        let left_delta = left - left_mean;
        let right_delta = right - right_mean;
        numerator += left_delta * right_delta;
        left_variance += left_delta * left_delta;
        right_variance += right_delta * right_delta;
    }
    (left_variance > f64::EPSILON && right_variance > f64::EPSILON)
        .then(|| numerator / (left_variance * right_variance).sqrt())
}

fn observed_signal_score(
    components: &[super::prospect_study::ProspectSignalComponentView],
) -> Result<f64, String> {
    let production = component_score(components, "production")?;
    let trajectory = component_score(components, "trajectory")?;
    let opportunity = component_score(components, "opportunity")?;
    Ok(round_score(
        100.0 * (0.50 * production + 0.25 * trajectory + 0.25 * opportunity),
    ))
}

fn component_score(
    components: &[super::prospect_study::ProspectSignalComponentView],
    id: &str,
) -> Result<f64, String> {
    let matches = components
        .iter()
        .filter(|component| component.id == id)
        .collect::<Vec<_>>();
    if matches.len() != 1
        || !matches[0].score.is_finite()
        || !(0.0..=1.0).contains(&matches[0].score)
    {
        return Err(format!(
            "prospect conversion study lacks unique {id} component"
        ));
    }
    Ok(matches[0].score)
}

fn validate_config(config: ProspectConversionConfig) -> Result<(), String> {
    let weight_sum = config.arrival_weight + config.role_weight + config.performance_weight;
    if config.minimum_horizon_seasons == 0
        || config.skater_established_games == 0
        || config.goalie_established_games == 0
        || config.forward_role_seconds_per_game == 0
        || config.defense_role_seconds_per_game == 0
        || config.goalie_role_seconds_per_game == 0
        || [
            config.arrival_weight,
            config.role_weight,
            config.performance_weight,
        ]
        .iter()
        .any(|weight| !weight.is_finite() || *weight < 0.0)
        || (weight_sum - 1.0).abs() > 1e-9
        || config.arrival_weight + config.role_weight <= 0.0
        || !config.baseline_floor.is_finite()
        || !(0.0..=100.0).contains(&config.baseline_floor)
        || !config.maximum_efficiency_index.is_finite()
        || config.maximum_efficiency_index < 100.0
        || config.minimum_rankable_players == 0
        || !config.minimum_rankable_baseline_confidence.is_finite()
        || !(0.0..=1.0).contains(&config.minimum_rankable_baseline_confidence)
        || !config.minimum_rankable_outcome_coverage.is_finite()
        || !(0.0..=1.0).contains(&config.minimum_rankable_outcome_coverage)
    {
        return Err("invalid prospect conversion configuration".to_owned());
    }
    Ok(())
}

fn weighted_mean(
    players: &[ProspectConversionPlayerView],
    total_weight: f64,
    value: impl Fn(&ProspectConversionPlayerView) -> f64,
) -> f64 {
    if total_weight <= 0.0 {
        players.iter().map(&value).sum::<f64>() / players.len() as f64
    } else {
        players
            .iter()
            .map(|player| value(player) * player.workload_confidence)
            .sum::<f64>()
            / total_weight
    }
}

fn position_group(position: &str) -> &'static str {
    match position.trim().to_ascii_uppercase().as_str() {
        "G" => "G",
        "D" | "LD" | "RD" => "D",
        _ => "F",
    }
}

fn season_start_year(season: u32) -> Option<u32> {
    let start = season / 10_000;
    let end = season % 10_000;
    (start >= 1900 && end == start + 1).then_some(start)
}

fn round_score(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn round_ratio(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline(
        player_id: u32,
        organization: &str,
        position: &str,
        signal: f64,
    ) -> ProspectConversionBaselineInput {
        ProspectConversionBaselineInput {
            player_id,
            player: format!("Prospect {player_id}"),
            organization: organization.to_owned(),
            position: position.to_owned(),
            baseline_season: 20222023,
            observed_signal_score: signal,
            workload_confidence: 1.0,
            evidence: vec![ProspectStudyEvidenceInput {
                label: "Frozen prospect study".to_owned(),
                source_url: format!("https://example.com/prospects/{player_id}"),
            }],
        }
    }

    fn outcome(
        player_id: u32,
        games: u32,
        seconds_per_game: u64,
        performance_score: Option<f64>,
    ) -> ProspectNhlOutcomeInput {
        ProspectNhlOutcomeInput {
            player_id,
            through_season: 20252026,
            nhl_games_played: games,
            nhl_toi_seconds: u64::from(games) * seconds_per_game,
            performance_score,
            performance_basis: performance_score.map(|_| "Canonical NHL value score".to_owned()),
            disposition: ProspectConversionDisposition::Retained,
            evidence: vec![ProspectStudyEvidenceInput {
                label: "Official NHL outcome".to_owned(),
                source_url: format!("https://api-web.nhle.com/v1/player/{player_id}/landing"),
            }],
        }
    }

    #[test]
    fn ranks_complete_conversion_cohorts_by_realized_efficiency() {
        let config = ProspectConversionConfig {
            minimum_rankable_players: 1,
            ..ProspectConversionConfig::default()
        };
        let input = ProspectConversionInput {
            schema: PROSPECT_CONVERSION_INPUT_SCHEMA.to_owned(),
            baseline_basis: PROSPECT_PROGRAM_SCORING_METHOD.to_owned(),
            baselines: vec![
                baseline(1, "SEA", "RW", 60.0),
                baseline(2, "NYR", "D", 60.0),
            ],
            outcomes: vec![
                outcome(1, 82, 900, Some(80.0)),
                outcome(2, 20, 600, Some(40.0)),
            ],
            calibration_signals: vec![],
            config,
        };
        let view = build_prospect_conversion_board(&input).unwrap();
        assert_eq!(view.schema, PROSPECT_CONVERSION_BOARD_SCHEMA);
        assert_eq!(view.baseline_basis, PROSPECT_PROGRAM_SCORING_METHOD);
        assert_eq!(view.ranked_organizations, 2);
        assert_eq!(view.programs[0].organization, "SEA");
        assert_eq!(view.programs[0].conversion_rank, Some(1));
        assert!(view.programs[0].rank_blockers.is_empty());
        assert!(view.programs[0].efficiency_index > view.programs[1].efficiency_index);
        assert_eq!(view.programs[0].established_players, 1);
    }

    #[test]
    fn missing_performance_is_visible_and_leaves_program_unranked() {
        let config = ProspectConversionConfig {
            minimum_rankable_players: 1,
            ..ProspectConversionConfig::default()
        };
        let input = ProspectConversionInput {
            schema: PROSPECT_CONVERSION_INPUT_SCHEMA.to_owned(),
            baseline_basis: PROSPECT_PROGRAM_SCORING_METHOD.to_owned(),
            baselines: vec![baseline(1, "SEA", "RW", 60.0)],
            outcomes: vec![outcome(1, 82, 900, None)],
            calibration_signals: vec![],
            config,
        };
        let view = build_prospect_conversion_board(&input).unwrap();
        assert_eq!(view.ranked_organizations, 0);
        assert_eq!(view.programs[0].conversion_rank, None);
        assert_eq!(view.programs[0].outcome_coverage, 0.7);
        assert!(matches!(
            view.programs[0].rank_blockers[0],
            ProspectConversionRankBlocker::LowOutcomeCoverage {
                observed: 0.7,
                required: 0.8
            }
        ));
        assert_eq!(view.programs[0].player_results[0].performance_score, None);
    }

    #[test]
    fn calibrates_frozen_signals_without_inventing_constant_signal_results() {
        let baselines = (1..=8)
            .map(|player_id| baseline(player_id, "SEA", "RW", f64::from(player_id) * 10.0))
            .collect::<Vec<_>>();
        let outcomes = (1..=8)
            .map(|player_id| {
                if player_id >= 5 {
                    outcome(player_id, 20 * player_id, 900, Some(60.0))
                } else {
                    outcome(player_id, 0, 0, Some(40.0))
                }
            })
            .collect::<Vec<_>>();
        let calibration_signals = (1..=8)
            .map(|player_id| ProspectConversionSignalInput {
                player_id,
                production_score: f64::from(player_id) / 10.0,
                trajectory_score: f64::from(9 - player_id) / 10.0,
                opportunity_score: 0.0,
            })
            .collect();
        let view = build_prospect_conversion_board(&ProspectConversionInput {
            schema: PROSPECT_CONVERSION_INPUT_SCHEMA.to_owned(),
            baseline_basis: PROSPECT_PROGRAM_SCORING_METHOD.to_owned(),
            baselines,
            outcomes,
            calibration_signals,
            config: ProspectConversionConfig::default(),
        })
        .unwrap();

        let production = view
            .signal_calibration
            .iter()
            .find(|row| row.signal == ProspectConversionSignalKind::Production)
            .unwrap();
        assert!(production.informative);
        assert!(production.arrival_correlation.unwrap() > 0.8);
        assert!(
            production.top_quartile.as_ref().unwrap().arrival_rate
                > production.bottom_quartile.as_ref().unwrap().arrival_rate
        );
        let opportunity = view
            .signal_calibration
            .iter()
            .find(|row| row.signal == ProspectConversionSignalKind::Opportunity)
            .unwrap();
        assert!(!opportunity.informative);
        assert_eq!(opportunity.arrival_correlation, None);
        assert_eq!(opportunity.top_quartile, None);
    }

    #[test]
    fn default_rank_floors_explain_small_and_low_confidence_cohorts() {
        let mut weak_baseline = baseline(1, "SEA", "RW", 60.0);
        weak_baseline.workload_confidence = 0.4;
        let input = ProspectConversionInput {
            schema: PROSPECT_CONVERSION_INPUT_SCHEMA.to_owned(),
            baseline_basis: PROSPECT_PROGRAM_SCORING_METHOD.to_owned(),
            baselines: vec![weak_baseline],
            outcomes: vec![outcome(1, 82, 900, Some(80.0))],
            calibration_signals: vec![],
            config: ProspectConversionConfig::default(),
        };
        let view = build_prospect_conversion_board(&input).unwrap();
        assert_eq!(view.ranked_organizations, 0);
        assert_eq!(view.programs[0].rank_blockers.len(), 2);
        assert!(matches!(
            view.programs[0].rank_blockers[0],
            ProspectConversionRankBlocker::InsufficientCohort {
                observed: 1,
                required: 5
            }
        ));
        assert!(matches!(
            view.programs[0].rank_blockers[1],
            ProspectConversionRankBlocker::LowBaselineConfidence {
                observed: 0.4,
                required: 0.5
            }
        ));
    }

    #[test]
    fn rejects_short_horizons_and_mismatched_player_sets() {
        let mut input = ProspectConversionInput {
            schema: PROSPECT_CONVERSION_INPUT_SCHEMA.to_owned(),
            baseline_basis: PROSPECT_PROGRAM_SCORING_METHOD.to_owned(),
            baselines: vec![baseline(1, "SEA", "RW", 60.0)],
            outcomes: vec![outcome(1, 82, 900, Some(80.0))],
            calibration_signals: vec![],
            config: ProspectConversionConfig::default(),
        };
        input.outcomes[0].through_season = 20242025;
        assert!(build_prospect_conversion_board(&input)
            .unwrap_err()
            .contains("horizon"));
        input.outcomes[0].through_season = 20252026;
        input.outcomes[0].player_id = 2;
        assert!(build_prospect_conversion_board(&input)
            .unwrap_err()
            .contains("identical players"));
    }

    #[test]
    fn adapts_frozen_study_and_only_post_baseline_nhl_outcomes() {
        use super::super::prospect_study::{
            ProspectAvailabilityStatus, ProspectHiddenValueClass, ProspectMarketPosition,
            ProspectNhlGamesAuthority, ProspectOpportunityStatus, ProspectSignalComponentView,
            ProspectTrajectory,
        };
        use crate::career_history::{CareerStint, LeagueAbbrev};
        use crate::model::Season;

        let components = vec![
            ProspectSignalComponentView {
                id: "production".to_owned(),
                score: 0.8,
                weight: 0.4,
                weighted_points: 32.0,
            },
            ProspectSignalComponentView {
                id: "trajectory".to_owned(),
                score: 0.6,
                weight: 0.3,
                weighted_points: 18.0,
            },
            ProspectSignalComponentView {
                id: "opportunity".to_owned(),
                score: 0.7,
                weight: 0.2,
                weighted_points: 14.0,
            },
        ];
        let study = ProspectDevelopmentStudyView {
            schema: PROSPECT_DEVELOPMENT_STUDY_SCHEMA.to_owned(),
            player_id: 1,
            player: "Prospect One".to_owned(),
            organization: "SEA".to_owned(),
            position: "RW".to_owned(),
            age: 20,
            nhl_games_played: 5,
            nhl_games_authority: ProspectNhlGamesAuthority::Observed,
            seasons: vec![
                super::super::prospect_study::ProspectDevelopmentSeasonView {
                    season: 20222023,
                    league: "AHL".to_owned(),
                    games_played: 40,
                    goals: 20,
                    assists: 20,
                    points: 40,
                    points_per_game: 1.0,
                    same_league_ppg_delta: None,
                    same_league_ppg_change: None,
                },
            ],
            trajectory: ProspectTrajectory::Rising,
            workload_confidence: 1.0,
            opportunity: ProspectOpportunityStatus::RecallCandidate,
            availability: ProspectAvailabilityStatus::Healthy,
            attention_score: 0.5,
            attention_basis: "Test".to_owned(),
            performance_attention_gap: 0.1,
            market_position: ProspectMarketPosition::Aligned,
            hidden_value_score: 64.0,
            classification: ProspectHiddenValueClass::Watch,
            components,
            lenses: vec![],
            evidence: vec![ProspectStudyEvidenceInput {
                label: "Frozen study".to_owned(),
                source_url: "https://example.com/frozen/1".to_owned(),
            }],
            disclosures: vec![],
        };
        let stint = |season, gp, avg_toi_sec| CareerStint {
            season: Season(season),
            league: LeagueAbbrev::new("NHL"),
            team: "Seattle Kraken".to_owned(),
            game_type: CareerGameType::Regular,
            sequence: 1,
            gp,
            goals: Some(10),
            assists: Some(10),
            points: Some(20),
            pim: None,
            plus_minus: None,
            power_play_goals: None,
            power_play_points: Some(5),
            shorthanded_goals: None,
            shorthanded_points: None,
            game_winning_goals: None,
            ot_goals: None,
            shots: Some(80),
            shooting_pct: None,
            avg_toi_sec,
            faceoff_win_pct: None,
            games_started: None,
            wins: None,
            losses: None,
            ot_losses: None,
            goals_against: None,
            goals_against_avg: None,
            save_pct: None,
            shots_against: None,
            shutouts: None,
            time_on_ice_sec: None,
        };
        let history = CareerHistory {
            player_id: 1,
            stints: vec![
                stint(20222023, 5, Some(600)),
                stint(20232024, 40, Some(900)),
            ],
        };
        let performance_document = build_prospect_nhl_performance_document(
            std::slice::from_ref(&study),
            &[],
            std::slice::from_ref(&history),
            20222023,
            20252026,
        )
        .unwrap();
        assert_eq!(
            performance_document.schema,
            PROSPECT_CONVERSION_PERFORMANCE_SCHEMA
        );
        assert_eq!(performance_document.source_coverage, 1.0);
        assert_eq!(performance_document.scores[0].games_played, 40);
        assert_eq!(performance_document.scores[0].position_group, "F");
        assert_eq!(performance_document.scores[0].sample_confidence, 0.5714);
        assert!(
            performance_document.scores[0].score < performance_document.scores[0].raw_quality_score
        );
        let input = adapt_prospect_conversion_input(
            std::slice::from_ref(&study),
            &[],
            std::slice::from_ref(&history),
            20222023,
            20252026,
            &[ProspectConversionPerformanceInput {
                player_id: 1,
                score: 75.0,
                basis: "IceLines NHL value".to_owned(),
                position_group: "F".to_owned(),
                games_played: 40,
                sample_confidence: 0.5714,
                raw_quality_score: 80.0,
                components: Vec::new(),
                evidence: vec![ProspectStudyEvidenceInput {
                    label: "Official NHL history".to_owned(),
                    source_url: "https://example.com/player/1".to_owned(),
                }],
            }],
            ProspectConversionConfig {
                minimum_rankable_players: 1,
                ..ProspectConversionConfig::default()
            },
        )
        .unwrap();
        assert_eq!(input.baseline_basis, PROSPECT_PROGRAM_SCORING_METHOD);
        assert_eq!(input.baselines[0].observed_signal_score, 72.5);
        assert_eq!(input.outcomes[0].nhl_games_played, 40);
        assert_eq!(input.outcomes[0].nhl_toi_seconds, 36_000);
        assert_eq!(input.outcomes[0].performance_score, Some(75.0));

        let mut incomplete_history = history;
        incomplete_history.stints[1].avg_toi_sec = None;
        let error = adapt_prospect_conversion_input(
            &[study],
            &[],
            &[incomplete_history],
            20222023,
            20252026,
            &[],
            ProspectConversionConfig::default(),
        )
        .unwrap_err();
        assert!(error.contains("missing NHL time on ice"));
    }
}
