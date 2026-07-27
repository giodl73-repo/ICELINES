use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

pub const PROSPECT_DEVELOPMENT_STUDY_SCHEMA: &str = "prospect_development_study.v1";
pub const PROSPECT_GOALIE_DEVELOPMENT_STUDY_SCHEMA: &str = "prospect_goalie_development_study.v1";
pub const PROSPECT_DISCOVERY_BOARD_SCHEMA: &str = "prospect_discovery_board.v1";
pub const PROSPECT_PROGRAM_BOARD_SCHEMA: &str = "prospect_program_board.v2";
pub const PROSPECT_PROGRAM_SENSITIVITY_SCHEMA: &str = "prospect_program_sensitivity.v1";
pub const PROSPECT_PROGRAM_HISTORY_SCHEMA: &str = "prospect_program_history.v1";
pub const PROSPECT_PROGRAM_SCORING_METHOD: &str = "prospect_program_signal.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectOpportunityStatus {
    None,
    Monitoring,
    RecallCandidate,
    DebutPlanned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectAvailabilityStatus {
    Healthy,
    InjuryInterrupted,
    Recovered,
    Unknown,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectNhlGamesAuthority {
    Observed,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectTrajectory {
    Rising,
    Stable,
    Cooling,
    Insufficient,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectHiddenValueClass {
    InjuryObscuredRiser,
    InjuryRecoveryWatch,
    HiddenRiser,
    VisibleRiser,
    Watch,
    Cooling,
    OverexposedCooling,
    HypeAheadOfEvidence,
    SmallSampleHypeRisk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectMarketPosition {
    Underrecognized,
    Aligned,
    Overexposed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectDiscoveryLensKind {
    ProductionRiser,
    InjuryObscured,
    RecoveryUnproven,
    OpportunityBacked,
    AttentionLag,
    AttentionAheadOfEvidence,
    WorkloadUncertain,
    CoolingSignal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectDiscoveryLensDirection {
    Upside,
    Risk,
    Context,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectDevelopmentSeasonInput {
    pub season: u32,
    pub league: String,
    pub games_played: u32,
    pub goals: u32,
    pub assists: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProspectStudyEvidenceInput {
    pub label: String,
    pub source_url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectDevelopmentStudyInput {
    pub player_id: u32,
    pub player: String,
    pub organization: String,
    pub position: String,
    pub age: u8,
    pub nhl_games_played: u32,
    pub seasons: Vec<ProspectDevelopmentSeasonInput>,
    pub opportunity: ProspectOpportunityStatus,
    pub availability: ProspectAvailabilityStatus,
    /// Explicitly authored 0..1 estimate. Zero means little public attention;
    /// one means extensive attention. `attention_basis` must explain it.
    pub attention_score: f64,
    pub attention_basis: String,
    #[serde(default)]
    pub evidence: Vec<ProspectStudyEvidenceInput>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProspectDevelopmentStudyConfig {
    /// Latest same-league points/game that represents a strong pro season.
    pub production_benchmark_ppg: f64,
    /// Same-league points/game gain that represents a clear rise.
    pub rising_delta_ppg: f64,
    /// Games required before production receives full workload confidence.
    pub full_confidence_games: u32,
    /// Both same-league seasons must reach this workload before trajectory is classified.
    pub minimum_comparison_games: u32,
    pub production_weight: f64,
    pub trajectory_weight: f64,
    pub opportunity_weight: f64,
    pub attention_gap_weight: f64,
}

impl Default for ProspectDevelopmentStudyConfig {
    fn default() -> Self {
        Self {
            production_benchmark_ppg: 0.8,
            rising_delta_ppg: 0.15,
            full_confidence_games: 40,
            minimum_comparison_games: 10,
            production_weight: 0.4,
            trajectory_weight: 0.3,
            opportunity_weight: 0.2,
            attention_gap_weight: 0.1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectDevelopmentSeasonView {
    pub season: u32,
    pub league: String,
    pub games_played: u32,
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
    pub points_per_game: f64,
    pub same_league_ppg_delta: Option<f64>,
    pub same_league_ppg_change: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectSignalComponentView {
    pub id: String,
    pub score: f64,
    pub weight: f64,
    pub weighted_points: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectDiscoveryLensView {
    pub kind: ProspectDiscoveryLensKind,
    pub direction: ProspectDiscoveryLensDirection,
    pub strength: f64,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectDevelopmentStudyView {
    pub schema: String,
    pub player_id: u32,
    pub player: String,
    pub organization: String,
    pub position: String,
    pub age: u8,
    pub nhl_games_played: u32,
    #[serde(default)]
    pub nhl_games_authority: ProspectNhlGamesAuthority,
    pub seasons: Vec<ProspectDevelopmentSeasonView>,
    pub trajectory: ProspectTrajectory,
    pub workload_confidence: f64,
    pub opportunity: ProspectOpportunityStatus,
    pub availability: ProspectAvailabilityStatus,
    pub attention_score: f64,
    pub attention_basis: String,
    pub performance_attention_gap: f64,
    pub market_position: ProspectMarketPosition,
    pub hidden_value_score: f64,
    pub classification: ProspectHiddenValueClass,
    pub components: Vec<ProspectSignalComponentView>,
    pub lenses: Vec<ProspectDiscoveryLensView>,
    pub evidence: Vec<ProspectStudyEvidenceInput>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectGoalieDevelopmentSeasonInput {
    pub season: u32,
    pub league: String,
    pub games_played: u32,
    pub save_percentage: f64,
    pub goals_against_average: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProspectGoalieDevelopmentStudyConfig {
    pub save_percentage_floor: f64,
    pub save_percentage_benchmark: f64,
    pub goals_against_average_ceiling: f64,
    pub goals_against_average_benchmark: f64,
    pub rising_save_percentage_delta: f64,
    pub rising_goals_against_average_improvement: f64,
    pub full_confidence_games: u32,
    pub minimum_comparison_games: u32,
}

impl Default for ProspectGoalieDevelopmentStudyConfig {
    fn default() -> Self {
        Self {
            save_percentage_floor: 0.880,
            save_percentage_benchmark: 0.915,
            goals_against_average_ceiling: 4.0,
            goals_against_average_benchmark: 2.5,
            rising_save_percentage_delta: 0.008,
            rising_goals_against_average_improvement: 0.35,
            full_confidence_games: 30,
            minimum_comparison_games: 8,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectGoalieDevelopmentSeasonView {
    pub season: u32,
    pub league: String,
    pub games_played: u32,
    pub save_percentage: f64,
    pub goals_against_average: f64,
    pub same_league_save_percentage_delta: Option<f64>,
    /// Positive means goals-against average improved (decreased).
    pub same_league_goals_against_average_improvement: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectGoalieDevelopmentStudyView {
    pub schema: String,
    pub player_id: u32,
    pub player: String,
    pub organization: String,
    pub position: String,
    pub age: u8,
    pub nhl_games_played: u32,
    #[serde(default)]
    pub nhl_games_authority: ProspectNhlGamesAuthority,
    pub seasons: Vec<ProspectGoalieDevelopmentSeasonView>,
    pub trajectory: ProspectTrajectory,
    pub workload_confidence: f64,
    pub opportunity: ProspectOpportunityStatus,
    pub availability: ProspectAvailabilityStatus,
    pub components: Vec<ProspectSignalComponentView>,
    pub evidence: Vec<ProspectStudyEvidenceInput>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectGoalieDevelopmentStudyInput {
    pub player_id: u32,
    pub player: String,
    pub organization: String,
    pub age: u8,
    pub nhl_games_played: u32,
    pub seasons: Vec<ProspectGoalieDevelopmentSeasonInput>,
    pub opportunity: ProspectOpportunityStatus,
    pub availability: ProspectAvailabilityStatus,
    #[serde(default)]
    pub evidence: Vec<ProspectStudyEvidenceInput>,
}

pub fn build_prospect_goalie_development_study(
    mut input: ProspectGoalieDevelopmentStudyInput,
    config: ProspectGoalieDevelopmentStudyConfig,
) -> Result<ProspectGoalieDevelopmentStudyView, String> {
    if input.player_id == 0
        || input.player.trim().is_empty()
        || input.organization.trim().is_empty()
        || input.seasons.len() < 2
        || !config.save_percentage_floor.is_finite()
        || !config.save_percentage_benchmark.is_finite()
        || config.save_percentage_floor >= config.save_percentage_benchmark
        || !config.goals_against_average_ceiling.is_finite()
        || !config.goals_against_average_benchmark.is_finite()
        || config.goals_against_average_benchmark >= config.goals_against_average_ceiling
        || !config.rising_save_percentage_delta.is_finite()
        || config.rising_save_percentage_delta <= 0.0
        || !config.rising_goals_against_average_improvement.is_finite()
        || config.rising_goals_against_average_improvement <= 0.0
        || config.minimum_comparison_games == 0
        || config.full_confidence_games < config.minimum_comparison_games
        || input.seasons.iter().any(|season| {
            season.season == 0
                || season.league.trim().is_empty()
                || season.games_played == 0
                || !season.save_percentage.is_finite()
                || !(0.0..=1.0).contains(&season.save_percentage)
                || !season.goals_against_average.is_finite()
                || season.goals_against_average < 0.0
        })
        || input.evidence.iter().any(|item| {
            item.label.trim().is_empty()
                || !(item.source_url.starts_with("https://")
                    || item.source_url.starts_with("http://"))
        })
    {
        return Err("invalid prospect goalie development input or configuration".to_owned());
    }
    input.seasons.sort_by_key(|season| season.season);
    if input
        .seasons
        .windows(2)
        .any(|seasons| seasons[0].season == seasons[1].season)
    {
        return Err("prospect goalie development seasons must be unique".to_owned());
    }

    let mut seasons = Vec::with_capacity(input.seasons.len());
    for (index, season) in input.seasons.iter().enumerate() {
        let prior = (season.games_played >= config.minimum_comparison_games)
            .then(|| {
                input.seasons[..index].iter().rev().find(|prior| {
                    prior.league.eq_ignore_ascii_case(&season.league)
                        && prior.games_played >= config.minimum_comparison_games
                })
            })
            .flatten();
        seasons.push(ProspectGoalieDevelopmentSeasonView {
            season: season.season,
            league: season.league.clone(),
            games_played: season.games_played,
            save_percentage: season.save_percentage,
            goals_against_average: season.goals_against_average,
            same_league_save_percentage_delta: prior
                .map(|prior| season.save_percentage - prior.save_percentage),
            same_league_goals_against_average_improvement: prior
                .map(|prior| prior.goals_against_average - season.goals_against_average),
        });
    }
    let latest = seasons.last().expect("validated goalie seasons");
    let save_delta = latest.same_league_save_percentage_delta;
    let gaa_improvement = latest.same_league_goals_against_average_improvement;
    let trajectory = match (save_delta, gaa_improvement) {
        (Some(save), Some(gaa))
            if save >= config.rising_save_percentage_delta
                || (save >= 0.0 && gaa >= config.rising_goals_against_average_improvement) =>
        {
            ProspectTrajectory::Rising
        }
        (Some(save), Some(gaa))
            if save <= -config.rising_save_percentage_delta
                || (save <= 0.0 && gaa <= -config.rising_goals_against_average_improvement) =>
        {
            ProspectTrajectory::Cooling
        }
        (Some(_), Some(_)) => ProspectTrajectory::Stable,
        _ => ProspectTrajectory::Insufficient,
    };
    let latest_confidence =
        (f64::from(latest.games_played) / f64::from(config.full_confidence_games)).min(1.0);
    let prior_confidence = input.seasons[..input.seasons.len() - 1]
        .iter()
        .rev()
        .find(|season| season.league.eq_ignore_ascii_case(&latest.league))
        .map(|season| {
            (f64::from(season.games_played) / f64::from(config.full_confidence_games)).min(1.0)
        })
        .unwrap_or(0.0);
    let workload_confidence = latest_confidence.min(prior_confidence);
    let save_score = ((latest.save_percentage - config.save_percentage_floor)
        / (config.save_percentage_benchmark - config.save_percentage_floor))
        .clamp(0.0, 1.0);
    let gaa_score = ((config.goals_against_average_ceiling - latest.goals_against_average)
        / (config.goals_against_average_ceiling - config.goals_against_average_benchmark))
        .clamp(0.0, 1.0);
    let production_score = (0.70 * save_score + 0.30 * gaa_score) * latest_confidence;
    let save_trajectory_score = save_delta
        .map(|delta| (0.5 + delta / (2.0 * config.rising_save_percentage_delta)).clamp(0.0, 1.0))
        .unwrap_or(0.0);
    let gaa_trajectory_score = gaa_improvement
        .map(|improvement| {
            (0.5 + improvement / (2.0 * config.rising_goals_against_average_improvement))
                .clamp(0.0, 1.0)
        })
        .unwrap_or(0.0);
    let trajectory_score =
        (0.70 * save_trajectory_score + 0.30 * gaa_trajectory_score) * workload_confidence;
    let opportunity_score = match input.opportunity {
        ProspectOpportunityStatus::None => 0.0,
        ProspectOpportunityStatus::Monitoring => 0.35,
        ProspectOpportunityStatus::RecallCandidate => 0.7,
        ProspectOpportunityStatus::DebutPlanned => 1.0,
    };
    Ok(ProspectGoalieDevelopmentStudyView {
        schema: PROSPECT_GOALIE_DEVELOPMENT_STUDY_SCHEMA.to_owned(),
        player_id: input.player_id,
        player: input.player,
        organization: input.organization,
        position: "G".to_owned(),
        age: input.age,
        nhl_games_played: input.nhl_games_played,
        nhl_games_authority: if input.nhl_games_played > 0 {
            ProspectNhlGamesAuthority::Observed
        } else {
            ProspectNhlGamesAuthority::Unknown
        },
        seasons,
        trajectory,
        workload_confidence,
        opportunity: input.opportunity,
        availability: input.availability,
        components: vec![
            ProspectSignalComponentView {
                id: "production".to_owned(),
                score: production_score,
                weight: 0.50,
                weighted_points: production_score * 50.0,
            },
            ProspectSignalComponentView {
                id: "trajectory".to_owned(),
                score: trajectory_score,
                weight: 0.25,
                weighted_points: trajectory_score * 25.0,
            },
            ProspectSignalComponentView {
                id: "opportunity".to_owned(),
                score: opportunity_score,
                weight: 0.25,
                weighted_points: opportunity_score * 25.0,
            },
        ],
        evidence: input.evidence,
        disclosures: vec![
            "Goalie production combines save percentage and goals-against average, then applies latest-season workload confidence; it is not compared directly with skater points per game.".to_owned(),
            "Trajectory combines same-league save-percentage change and goals-against-average improvement only when both seasons clear the comparison workload.".to_owned(),
            "Team defense and shot quality are not yet isolated; this is an AHL results-and-workload development signal, not a complete goalie talent model.".to_owned(),
        ],
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectDiscoveryBoardLane {
    HiddenGem,
    BuyerBeware,
    Watch,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectDiscoveryBoardRow {
    pub rank: usize,
    pub player_id: u32,
    pub player: String,
    pub organization: String,
    pub position: String,
    pub lane: ProspectDiscoveryBoardLane,
    pub classification: ProspectHiddenValueClass,
    pub market_position: ProspectMarketPosition,
    pub hidden_value_score: f64,
    pub performance_attention_gap: f64,
    /// Lane-relative 0..100 ordering signal. Hidden gems use hidden value;
    /// buyer-beware rows use their strongest supported risk signal.
    pub lane_score: f64,
    pub lenses: Vec<ProspectDiscoveryLensView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectDiscoveryBoardView {
    pub schema: String,
    pub studies: usize,
    pub hidden_gems: Vec<ProspectDiscoveryBoardRow>,
    pub buyer_beware: Vec<ProspectDiscoveryBoardRow>,
    pub watch: Vec<ProspectDiscoveryBoardRow>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProspectProgramBoardConfig {
    pub pool_weight: f64,
    pub development_weight: f64,
    pub readiness_weight: f64,
    pub confidence_weight: f64,
    pub expected_depth: usize,
    /// Operational graduation boundary for reserve-system ranking. This is a
    /// transparent IceLines population rule, not NHL rookie eligibility.
    pub maximum_nhl_games_played: u32,
}

impl Default for ProspectProgramBoardConfig {
    fn default() -> Self {
        Self {
            pool_weight: 0.45,
            development_weight: 0.30,
            readiness_weight: 0.15,
            confidence_weight: 0.10,
            expected_depth: 10,
            maximum_nhl_games_played: 50,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectProgramMethodologyView {
    pub scoring_method: String,
    pub pool_weight: f64,
    pub development_weight: f64,
    pub readiness_weight: f64,
    pub confidence_weight: f64,
    pub expected_depth: usize,
}

impl ProspectProgramMethodologyView {
    fn from_config(config: ProspectProgramBoardConfig) -> Self {
        Self {
            scoring_method: PROSPECT_PROGRAM_SCORING_METHOD.to_owned(),
            pool_weight: config.pool_weight,
            development_weight: config.development_weight,
            readiness_weight: config.readiness_weight,
            confidence_weight: config.confidence_weight,
            expected_depth: config.expected_depth,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectProgramTopProspectView {
    pub player_id: u32,
    pub player: String,
    pub position: String,
    pub observed_signal_score: f64,
    pub trajectory: ProspectTrajectory,
    pub opportunity: ProspectOpportunityStatus,
    pub workload_confidence: f64,
    pub nhl_games_played: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectProgramGraduateView {
    pub player_id: u32,
    pub player: String,
    pub position: String,
    pub nhl_games_played: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectProgramPositionCountsView {
    pub forwards: usize,
    pub defensemen: usize,
    pub goalies: usize,
    pub other: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectProgramComponentsView {
    pub elite_signal: f64,
    pub quality_depth: f64,
    pub development: f64,
    pub readiness: f64,
    pub positional_balance: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectProgramOrganizationView {
    pub organization: String,
    /// Studies eligible for reserve-system scoring at the board's GP boundary.
    pub prospect_count: usize,
    /// All supplied studies before the graduation boundary is applied.
    pub supplied_study_count: usize,
    pub graduated_count: usize,
    #[serde(default)]
    pub unknown_nhl_games_count: usize,
    pub positions: ProspectProgramPositionCountsView,
    pub components: ProspectProgramComponentsView,
    pub pool_score: f64,
    pub development_score: f64,
    pub pipeline_score: f64,
    pub pool_rank: usize,
    pub development_rank: usize,
    pub pipeline_rank: usize,
    /// Positive means the organization improved relative to the prior board.
    pub pool_rank_delta: Option<i32>,
    pub development_rank_delta: Option<i32>,
    pub pipeline_rank_delta: Option<i32>,
    pub pool_score_delta: Option<f64>,
    pub development_score_delta: Option<f64>,
    pub pipeline_score_delta: Option<f64>,
    pub top_prospects: Vec<ProspectProgramTopProspectView>,
    pub graduates: Vec<ProspectProgramGraduateView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectProgramBoardView {
    pub schema: String,
    pub scope: String,
    pub source_leagues: Vec<String>,
    pub as_of_season: u32,
    /// Season of the optional comparison board used to compute deltas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior_as_of_season: Option<u32>,
    /// Frozen scoring assumptions required for comparable score/rank deltas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub methodology: Option<ProspectProgramMethodologyView>,
    pub organizations: usize,
    pub studies: usize,
    pub ranked_studies: usize,
    pub graduated_studies: usize,
    #[serde(default)]
    pub unknown_nhl_games_studies: usize,
    pub maximum_nhl_games_played: u32,
    /// Sorted by `pipeline_rank`; consumers may independently sort on either
    /// other frozen rank without recomputing scores.
    pub programs: Vec<ProspectProgramOrganizationView>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectProgramSensitivityPointView {
    pub maximum_nhl_games_played: u32,
    pub pipeline_rank: usize,
    pub pipeline_score: f64,
    pub pool_rank: usize,
    pub development_rank: usize,
    pub ranked_studies: usize,
    pub graduated_studies: usize,
    #[serde(default)]
    pub unknown_nhl_games_studies: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectProgramSensitivityOrganizationView {
    pub organization: String,
    pub best_pipeline_rank: usize,
    pub worst_pipeline_rank: usize,
    pub pipeline_rank_span: usize,
    pub minimum_pipeline_score: f64,
    pub maximum_pipeline_score: f64,
    pub pipeline_score_span: f64,
    pub points: Vec<ProspectProgramSensitivityPointView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectProgramSensitivityView {
    pub schema: String,
    pub source_schema: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub methodology: Option<ProspectProgramMethodologyView>,
    pub thresholds: Vec<u32>,
    pub organizations: usize,
    pub supplied_studies: usize,
    pub programs: Vec<ProspectProgramSensitivityOrganizationView>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectProgramHistoryPointView {
    pub as_of_season: u32,
    pub pipeline_rank: usize,
    pub pipeline_score: f64,
    pub pool_rank: usize,
    pub pool_score: f64,
    pub development_rank: usize,
    pub development_score: f64,
    pub ranked_studies: usize,
    pub graduated_studies: usize,
    pub unknown_nhl_games_studies: usize,
    pub pipeline_rank_delta: Option<i32>,
    pub pipeline_score_delta: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectProgramHistoryOrganizationView {
    pub organization: String,
    pub first_as_of_season: u32,
    pub latest_as_of_season: u32,
    pub seasons_observed: usize,
    /// Positive means improvement from the first observed board to the latest.
    pub pipeline_rank_change: i32,
    pub pipeline_score_change: f64,
    pub points: Vec<ProspectProgramHistoryPointView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectProgramHistoryView {
    pub schema: String,
    pub source_schema: String,
    pub scope: String,
    pub source_leagues: Vec<String>,
    pub methodology: ProspectProgramMethodologyView,
    pub maximum_nhl_games_played: u32,
    pub seasons: Vec<u32>,
    pub boards: usize,
    pub organizations: usize,
    pub programs: Vec<ProspectProgramHistoryOrganizationView>,
    pub disclosures: Vec<String>,
}

pub fn build_prospect_program_history(
    mut boards: Vec<ProspectProgramBoardView>,
) -> Result<ProspectProgramHistoryView, String> {
    if boards.len() < 2 {
        return Err("prospect program history requires at least two boards".to_owned());
    }
    boards.sort_by_key(|board| board.as_of_season);
    let reference = &boards[0];
    let Some(methodology) = reference.methodology.clone() else {
        return Err("prospect program history requires proven scoring methodology".to_owned());
    };
    let mut seasons = BTreeSet::new();
    for board in &boards {
        let mut organizations = BTreeSet::new();
        let mut pipeline_ranks = BTreeSet::new();
        let mut pool_ranks = BTreeSet::new();
        let mut development_ranks = BTreeSet::new();
        if board.schema != PROSPECT_PROGRAM_BOARD_SCHEMA
            || board.as_of_season == 0
            || !seasons.insert(board.as_of_season)
            || board.scope != reference.scope
            || board.source_leagues != reference.source_leagues
            || board.maximum_nhl_games_played != reference.maximum_nhl_games_played
            || board.methodology.as_ref() != Some(&methodology)
            || board.organizations != board.programs.len()
            || board.studies != board.ranked_studies + board.graduated_studies
            || board.ranked_studies
                != board
                    .programs
                    .iter()
                    .map(|row| row.prospect_count)
                    .sum::<usize>()
            || board.graduated_studies
                != board
                    .programs
                    .iter()
                    .map(|row| row.graduated_count)
                    .sum::<usize>()
            || board.unknown_nhl_games_studies
                != board
                    .programs
                    .iter()
                    .map(|row| row.unknown_nhl_games_count)
                    .sum::<usize>()
            || board.programs.iter().any(|row| {
                row.organization.trim().is_empty()
                    || !organizations.insert(row.organization.as_str())
                    || row.supplied_study_count != row.prospect_count + row.graduated_count
                    || row.graduated_count != row.graduates.len()
                    || row.unknown_nhl_games_count > row.prospect_count
                    || row.pipeline_rank == 0
                    || row.pipeline_rank > board.organizations
                    || !pipeline_ranks.insert(row.pipeline_rank)
                    || row.pool_rank == 0
                    || row.pool_rank > board.organizations
                    || !pool_ranks.insert(row.pool_rank)
                    || row.development_rank == 0
                    || row.development_rank > board.organizations
                    || !development_ranks.insert(row.development_rank)
                    || !row.pipeline_score.is_finite()
                    || !row.pool_score.is_finite()
                    || !row.development_score.is_finite()
            })
        {
            return Err(
                "prospect program history boards must be unique, valid, and methodologically comparable"
                    .to_owned(),
            );
        }
    }

    let organization_names = boards
        .iter()
        .flat_map(|board| board.programs.iter().map(|row| row.organization.clone()))
        .collect::<BTreeSet<_>>();
    let mut programs = Vec::with_capacity(organization_names.len());
    for organization in organization_names {
        let mut points = Vec::new();
        for (index, board) in boards.iter().enumerate() {
            let Some(row) = board
                .programs
                .iter()
                .find(|row| row.organization == organization)
            else {
                continue;
            };
            let previous = index.checked_sub(1).and_then(|previous_index| {
                boards[previous_index]
                    .programs
                    .iter()
                    .find(|candidate| candidate.organization == organization)
            });
            points.push(ProspectProgramHistoryPointView {
                as_of_season: board.as_of_season,
                pipeline_rank: row.pipeline_rank,
                pipeline_score: row.pipeline_score,
                pool_rank: row.pool_rank,
                pool_score: row.pool_score,
                development_rank: row.development_rank,
                development_score: row.development_score,
                ranked_studies: row.prospect_count,
                graduated_studies: row.graduated_count,
                unknown_nhl_games_studies: row.unknown_nhl_games_count,
                pipeline_rank_delta: previous
                    .map(|prior| prior.pipeline_rank as i32 - row.pipeline_rank as i32),
                pipeline_score_delta: previous
                    .map(|prior| round_program_score(row.pipeline_score - prior.pipeline_score)),
            });
        }
        let first = points.first().expect("organization came from a board");
        let latest = points.last().expect("organization came from a board");
        programs.push(ProspectProgramHistoryOrganizationView {
            organization,
            first_as_of_season: first.as_of_season,
            latest_as_of_season: latest.as_of_season,
            seasons_observed: points.len(),
            pipeline_rank_change: first.pipeline_rank as i32 - latest.pipeline_rank as i32,
            pipeline_score_change: round_program_score(
                latest.pipeline_score - first.pipeline_score,
            ),
            points,
        });
    }
    let latest_season = boards.last().expect("validated boards").as_of_season;
    programs.sort_by(|left, right| {
        let left_rank = left
            .points
            .iter()
            .find(|point| point.as_of_season == latest_season)
            .map_or(usize::MAX, |point| point.pipeline_rank);
        let right_rank = right
            .points
            .iter()
            .find(|point| point.as_of_season == latest_season)
            .map_or(usize::MAX, |point| point.pipeline_rank);
        left_rank
            .cmp(&right_rank)
            .then_with(|| left.organization.cmp(&right.organization))
    });

    Ok(ProspectProgramHistoryView {
        schema: PROSPECT_PROGRAM_HISTORY_SCHEMA.to_owned(),
        source_schema: PROSPECT_PROGRAM_BOARD_SCHEMA.to_owned(),
        scope: reference.scope.clone(),
        source_leagues: reference.source_leagues.clone(),
        methodology,
        maximum_nhl_games_played: reference.maximum_nhl_games_played,
        seasons: seasons.into_iter().collect(),
        boards: boards.len(),
        organizations: programs.len(),
        programs,
        disclosures: vec![
            "History accepts only uniquely dated boards with identical scope, source leagues, graduation boundary, and scoring methodology.".to_owned(),
            "Adjacent deltas are recomputed from consecutive supplied boards; embedded board deltas are not trusted or copied.".to_owned(),
            "A team absent from the immediately preceding board receives null adjacent deltas. First-to-latest change spans only that team's observed points.".to_owned(),
            "Program movement describes the supplied prospect population and method, not future NHL outcomes or causal development quality.".to_owned(),
        ],
    })
}

/// Rebuild the same supplied population under multiple explicit NHL-GP
/// graduation boundaries and freeze the resulting rank/score ranges. This
/// measures definition sensitivity; it does not select a preferred threshold.
pub fn build_prospect_program_sensitivity_with_goalies(
    studies: Vec<ProspectDevelopmentStudyView>,
    goalie_studies: Vec<ProspectGoalieDevelopmentStudyView>,
    mut thresholds: Vec<u32>,
    base_config: ProspectProgramBoardConfig,
) -> Result<ProspectProgramSensitivityView, String> {
    if thresholds.len() < 2 {
        return Err("prospect program sensitivity requires at least two thresholds".to_owned());
    }
    thresholds.sort_unstable();
    if thresholds.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("prospect program sensitivity thresholds must be unique".to_owned());
    }
    let mut boards = Vec::with_capacity(thresholds.len());
    for threshold in &thresholds {
        boards.push(build_prospect_program_board_with_goalies(
            studies.clone(),
            goalie_studies.clone(),
            None,
            ProspectProgramBoardConfig {
                maximum_nhl_games_played: *threshold,
                ..base_config
            },
        )?);
    }
    let supplied_studies = boards[0].studies;
    let organizations = boards[0].organizations;
    if boards.iter().any(|board| {
        board.studies != supplied_studies
            || board.organizations != organizations
            || board.schema != PROSPECT_PROGRAM_BOARD_SCHEMA
    }) {
        return Err("prospect program sensitivity boards have inconsistent populations".to_owned());
    }

    let mut programs = Vec::with_capacity(organizations);
    for organization in boards[0]
        .programs
        .iter()
        .map(|program| program.organization.as_str())
    {
        let mut points = Vec::with_capacity(boards.len());
        for board in &boards {
            let program = board
                .programs
                .iter()
                .find(|program| program.organization == organization)
                .ok_or_else(|| {
                    format!("organization {organization} missing from sensitivity board")
                })?;
            points.push(ProspectProgramSensitivityPointView {
                maximum_nhl_games_played: board.maximum_nhl_games_played,
                pipeline_rank: program.pipeline_rank,
                pipeline_score: program.pipeline_score,
                pool_rank: program.pool_rank,
                development_rank: program.development_rank,
                ranked_studies: program.prospect_count,
                graduated_studies: program.graduated_count,
                unknown_nhl_games_studies: program.unknown_nhl_games_count,
            });
        }
        let best_pipeline_rank = points
            .iter()
            .map(|point| point.pipeline_rank)
            .min()
            .expect("validated thresholds");
        let worst_pipeline_rank = points
            .iter()
            .map(|point| point.pipeline_rank)
            .max()
            .expect("validated thresholds");
        let minimum_pipeline_score = points
            .iter()
            .map(|point| point.pipeline_score)
            .fold(f64::INFINITY, f64::min);
        let maximum_pipeline_score = points
            .iter()
            .map(|point| point.pipeline_score)
            .fold(f64::NEG_INFINITY, f64::max);
        programs.push(ProspectProgramSensitivityOrganizationView {
            organization: organization.to_owned(),
            best_pipeline_rank,
            worst_pipeline_rank,
            pipeline_rank_span: worst_pipeline_rank - best_pipeline_rank,
            minimum_pipeline_score,
            maximum_pipeline_score,
            pipeline_score_span: round_program_score(
                maximum_pipeline_score - minimum_pipeline_score,
            ),
            points,
        });
    }
    programs.sort_by(|left, right| {
        right
            .pipeline_rank_span
            .cmp(&left.pipeline_rank_span)
            .then_with(|| {
                right
                    .pipeline_score_span
                    .total_cmp(&left.pipeline_score_span)
            })
            .then_with(|| left.organization.cmp(&right.organization))
    });
    Ok(ProspectProgramSensitivityView {
        schema: PROSPECT_PROGRAM_SENSITIVITY_SCHEMA.to_owned(),
        source_schema: PROSPECT_PROGRAM_BOARD_SCHEMA.to_owned(),
        methodology: Some(ProspectProgramMethodologyView::from_config(base_config)),
        thresholds,
        organizations,
        supplied_studies,
        programs,
        disclosures: vec![
            "Each point rebuilds the identical supplied study population with only the regular-season NHL-GP graduation boundary changed.".to_owned(),
            "Rank span measures sensitivity to the prospect definition across the supplied thresholds; it is not uncertainty in player performance or a confidence interval.".to_owned(),
            "No threshold is selected as correct. Consumers must display the tested boundaries when presenting a sensitivity conclusion.".to_owned(),
        ],
    })
}

/// Aggregate canonical prospect studies into organization-level Pool,
/// Development, and Pipeline rankings. Hidden-value/attention scores are
/// deliberately excluded because underrecognition is not prospect quality.
pub fn build_prospect_program_board_with_goalies(
    mut studies: Vec<ProspectDevelopmentStudyView>,
    goalie_studies: Vec<ProspectGoalieDevelopmentStudyView>,
    prior: Option<&ProspectProgramBoardView>,
    config: ProspectProgramBoardConfig,
) -> Result<ProspectProgramBoardView, String> {
    for goalie in goalie_studies {
        if goalie.schema != PROSPECT_GOALIE_DEVELOPMENT_STUDY_SCHEMA
            || goalie.position != "G"
            || goalie.seasons.len() < 2
        {
            return Err("invalid prospect goalie study supplied to program board".to_owned());
        }
        // The program board consumes only identity, league/season coverage,
        // workload, trajectory, opportunity, and named component scores. This
        // private bridge deliberately does not expose synthetic skater stats.
        studies.push(ProspectDevelopmentStudyView {
            schema: PROSPECT_DEVELOPMENT_STUDY_SCHEMA.to_owned(),
            player_id: goalie.player_id,
            player: goalie.player,
            organization: goalie.organization,
            position: goalie.position,
            age: goalie.age,
            nhl_games_played: goalie.nhl_games_played,
            nhl_games_authority: goalie.nhl_games_authority,
            seasons: goalie
                .seasons
                .into_iter()
                .map(|season| ProspectDevelopmentSeasonView {
                    season: season.season,
                    league: season.league,
                    games_played: season.games_played,
                    goals: 0,
                    assists: 0,
                    points: 0,
                    points_per_game: 0.0,
                    same_league_ppg_delta: None,
                    same_league_ppg_change: None,
                })
                .collect(),
            trajectory: goalie.trajectory,
            workload_confidence: goalie.workload_confidence,
            opportunity: goalie.opportunity,
            availability: goalie.availability,
            attention_score: 0.5,
            attention_basis:
                "Neutral internal program-board bridge; goalie attention is not scored.".to_owned(),
            performance_attention_gap: 0.0,
            market_position: ProspectMarketPosition::Aligned,
            hidden_value_score: 0.0,
            classification: ProspectHiddenValueClass::Watch,
            components: goalie.components,
            lenses: vec![],
            evidence: goalie.evidence,
            disclosures: goalie.disclosures,
        });
    }
    build_prospect_program_board(studies, prior, config)
}

pub fn build_prospect_program_board(
    studies: Vec<ProspectDevelopmentStudyView>,
    prior: Option<&ProspectProgramBoardView>,
    config: ProspectProgramBoardConfig,
) -> Result<ProspectProgramBoardView, String> {
    let weight_sum = config.pool_weight
        + config.development_weight
        + config.readiness_weight
        + config.confidence_weight;
    if studies.is_empty()
        || config.expected_depth == 0
        || [
            config.pool_weight,
            config.development_weight,
            config.readiness_weight,
            config.confidence_weight,
        ]
        .iter()
        .any(|weight| !weight.is_finite() || *weight < 0.0)
        || (weight_sum - 1.0).abs() > 1e-9
    {
        return Err("invalid prospect program board input or configuration".to_owned());
    }
    if let Some(board) = prior {
        let mut organizations = BTreeSet::new();
        let expected_methodology = ProspectProgramMethodologyView::from_config(config);
        if board.schema != PROSPECT_PROGRAM_BOARD_SCHEMA
            || board.maximum_nhl_games_played != config.maximum_nhl_games_played
            || board.organizations != board.programs.len()
            || board.studies != board.ranked_studies + board.graduated_studies
            || board.unknown_nhl_games_studies
                != board
                    .programs
                    .iter()
                    .map(|row| row.unknown_nhl_games_count)
                    .sum::<usize>()
            || board.programs.iter().any(|row| {
                row.organization.trim().is_empty()
                    || row.supplied_study_count != row.prospect_count + row.graduated_count
                    || row.graduated_count != row.graduates.len()
                    || row.unknown_nhl_games_count > row.prospect_count
                    || !organizations.insert(row.organization.as_str())
                    || row.pool_rank == 0
                    || row.development_rank == 0
                    || row.pipeline_rank == 0
            })
        {
            return Err("invalid prior prospect program board".to_owned());
        }
        if board.methodology.as_ref() != Some(&expected_methodology) {
            return Err(
                "prior prospect program board uses a different or unproven scoring methodology"
                    .to_owned(),
            );
        }
    }

    let mut player_ids = BTreeSet::new();
    let mut source_leagues = BTreeSet::new();
    let mut by_organization = BTreeMap::<String, Vec<ProspectDevelopmentStudyView>>::new();
    let mut as_of_season = 0_u32;
    for study in studies {
        if study.schema != PROSPECT_DEVELOPMENT_STUDY_SCHEMA
            || study.player_id == 0
            || !player_ids.insert(study.player_id)
            || study.player.trim().is_empty()
            || study.organization.trim().is_empty()
            || study.position.trim().is_empty()
            || !study.workload_confidence.is_finite()
            || !(0.0..=1.0).contains(&study.workload_confidence)
            || study.seasons.is_empty()
            || study.seasons.iter().any(|season| season.season == 0)
        {
            return Err("invalid or duplicate prospect study supplied to program board".to_owned());
        }
        for required in ["production", "trajectory", "opportunity"] {
            if study
                .components
                .iter()
                .filter(|row| row.id == required)
                .count()
                != 1
            {
                return Err(format!(
                    "prospect {} lacks unique {required} program component",
                    study.player_id
                ));
            }
        }
        if study
            .components
            .iter()
            .any(|row| !row.score.is_finite() || !(0.0..=1.0).contains(&row.score))
        {
            return Err("prospect program component score is outside 0..1".to_owned());
        }
        as_of_season = as_of_season.max(
            study
                .seasons
                .iter()
                .map(|season| season.season)
                .max()
                .unwrap_or(0),
        );
        source_leagues.extend(study.seasons.iter().map(|season| season.league.clone()));
        by_organization
            .entry(study.organization.clone())
            .or_default()
            .push(study);
    }

    let source_leagues = source_leagues.into_iter().collect::<Vec<_>>();
    let scope = if source_leagues.len() == 1 && source_leagues[0].eq_ignore_ascii_case("AHL") {
        "ahl_observed"
    } else {
        "multi_league_observed"
    };
    if let Some(board) = prior {
        if board.as_of_season == 0 || board.as_of_season >= as_of_season {
            return Err(
                "prior prospect program board must precede the current as-of season".to_owned(),
            );
        }
        if board.scope != scope || board.source_leagues != source_leagues {
            return Err(
                "prior prospect program board must use the same scope and source leagues"
                    .to_owned(),
            );
        }
    }

    let mut programs = Vec::with_capacity(by_organization.len());
    for (organization, organization_studies) in by_organization {
        let eligible_studies = organization_studies
            .iter()
            .filter(|study| {
                study.nhl_games_authority != ProspectNhlGamesAuthority::Observed
                    || study.nhl_games_played <= config.maximum_nhl_games_played
            })
            .collect::<Vec<_>>();
        let mut graduates = organization_studies
            .iter()
            .filter(|study| {
                study.nhl_games_authority == ProspectNhlGamesAuthority::Observed
                    && study.nhl_games_played > config.maximum_nhl_games_played
            })
            .map(|study| ProspectProgramGraduateView {
                player_id: study.player_id,
                player: study.player.clone(),
                position: study.position.clone(),
                nhl_games_played: study.nhl_games_played,
            })
            .collect::<Vec<_>>();
        graduates.sort_by(|left, right| {
            right
                .nhl_games_played
                .cmp(&left.nhl_games_played)
                .then_with(|| left.player.cmp(&right.player))
                .then_with(|| left.player_id.cmp(&right.player_id))
        });
        let unknown_nhl_games_count = eligible_studies
            .iter()
            .filter(|study| study.nhl_games_authority == ProspectNhlGamesAuthority::Unknown)
            .count();
        let mut observed = eligible_studies
            .iter()
            .map(|study| {
                let production = prospect_component_score(study, "production");
                let trajectory = prospect_component_score(study, "trajectory");
                let opportunity = prospect_component_score(study, "opportunity");
                let score = 100.0 * (0.50 * production + 0.25 * trajectory + 0.25 * opportunity);
                (study, score, trajectory, opportunity)
            })
            .collect::<Vec<_>>();
        observed.sort_by(|left, right| {
            right
                .1
                .total_cmp(&left.1)
                .then_with(|| left.0.player.cmp(&right.0.player))
                .then_with(|| left.0.player_id.cmp(&right.0.player_id))
        });

        let elite_weights = [0.50, 0.30, 0.20];
        let elite_signal = elite_weights
            .iter()
            .enumerate()
            .map(|(index, weight)| observed.get(index).map_or(0.0, |row| row.1 * weight))
            .sum::<f64>();
        let quality_depth = observed
            .iter()
            .take(config.expected_depth)
            .map(|row| row.1)
            .sum::<f64>()
            / config.expected_depth as f64;
        let development_weight = observed
            .iter()
            .filter(|row| row.0.trajectory != ProspectTrajectory::Insufficient)
            .map(|row| row.0.workload_confidence)
            .sum::<f64>();
        let development = if development_weight > 0.0 {
            observed
                .iter()
                .filter(|row| row.0.trajectory != ProspectTrajectory::Insufficient)
                .map(|row| row.2 * row.0.workload_confidence * 100.0)
                .sum::<f64>()
                / development_weight
        } else {
            0.0
        };
        let readiness = observed
            .iter()
            .take(5)
            .map(|row| row.3 * 100.0)
            .sum::<f64>()
            / 5.0;
        let confidence_mean = if observed.is_empty() {
            0.0
        } else {
            observed
                .iter()
                .map(|row| row.0.workload_confidence)
                .sum::<f64>()
                / observed.len() as f64
        };
        let breadth = (observed.len() as f64 / config.expected_depth as f64).min(1.0);
        let confidence = confidence_mean * breadth * 100.0;

        let mut positions = ProspectProgramPositionCountsView {
            forwards: 0,
            defensemen: 0,
            goalies: 0,
            other: 0,
        };
        for study in &eligible_studies {
            match prospect_position_group(&study.position) {
                "F" => positions.forwards += 1,
                "D" => positions.defensemen += 1,
                "G" => positions.goalies += 1,
                _ => positions.other += 1,
            }
        }
        let positional_balance = 100.0
            * (0.50 * (positions.forwards as f64 / 3.0).min(1.0)
                + 0.35 * (positions.defensemen as f64 / 2.0).min(1.0)
                + 0.15 * (positions.goalies as f64).min(1.0));
        let pool_score = 0.55 * elite_signal + 0.30 * quality_depth + 0.15 * positional_balance;
        let development_score = 0.80 * development * (confidence / 100.0) + 0.20 * confidence;
        let pipeline_score = config.pool_weight * pool_score
            + config.development_weight * development_score
            + config.readiness_weight * readiness
            + config.confidence_weight * confidence;
        let top_prospects = observed
            .iter()
            .take(5)
            .map(|row| ProspectProgramTopProspectView {
                player_id: row.0.player_id,
                player: row.0.player.clone(),
                position: row.0.position.clone(),
                observed_signal_score: round_program_score(row.1),
                trajectory: row.0.trajectory,
                opportunity: row.0.opportunity,
                workload_confidence: row.0.workload_confidence,
                nhl_games_played: row.0.nhl_games_played,
            })
            .collect();
        programs.push(ProspectProgramOrganizationView {
            organization,
            prospect_count: eligible_studies.len(),
            supplied_study_count: organization_studies.len(),
            graduated_count: graduates.len(),
            unknown_nhl_games_count,
            positions,
            components: ProspectProgramComponentsView {
                elite_signal: round_program_score(elite_signal),
                quality_depth: round_program_score(quality_depth),
                development: round_program_score(development),
                readiness: round_program_score(readiness),
                positional_balance: round_program_score(positional_balance),
                confidence: round_program_score(confidence),
            },
            pool_score: round_program_score(pool_score),
            development_score: round_program_score(development_score),
            pipeline_score: round_program_score(pipeline_score),
            pool_rank: 0,
            development_rank: 0,
            pipeline_rank: 0,
            pool_rank_delta: None,
            development_rank_delta: None,
            pipeline_rank_delta: None,
            pool_score_delta: None,
            development_score_delta: None,
            pipeline_score_delta: None,
            top_prospects,
            graduates,
        });
    }

    assign_program_rank(
        &mut programs,
        |row| row.pool_score,
        |row, rank| row.pool_rank = rank,
    );
    assign_program_rank(
        &mut programs,
        |row| row.development_score,
        |row, rank| row.development_rank = rank,
    );
    assign_program_rank(
        &mut programs,
        |row| row.pipeline_score,
        |row, rank| row.pipeline_rank = rank,
    );
    if let Some(prior) = prior {
        let prior_by_org = prior
            .programs
            .iter()
            .map(|row| (row.organization.as_str(), row))
            .collect::<BTreeMap<_, _>>();
        for row in &mut programs {
            if let Some(previous) = prior_by_org.get(row.organization.as_str()) {
                row.pool_rank_delta = Some(previous.pool_rank as i32 - row.pool_rank as i32);
                row.development_rank_delta =
                    Some(previous.development_rank as i32 - row.development_rank as i32);
                row.pipeline_rank_delta =
                    Some(previous.pipeline_rank as i32 - row.pipeline_rank as i32);
                row.pool_score_delta =
                    Some(round_program_score(row.pool_score - previous.pool_score));
                row.development_score_delta = Some(round_program_score(
                    row.development_score - previous.development_score,
                ));
                row.pipeline_score_delta = Some(round_program_score(
                    row.pipeline_score - previous.pipeline_score,
                ));
            }
        }
    }
    programs.sort_by_key(|row| row.pipeline_rank);

    Ok(ProspectProgramBoardView {
        schema: PROSPECT_PROGRAM_BOARD_SCHEMA.to_owned(),
        scope: scope.to_owned(),
        source_leagues,
        as_of_season,
        prior_as_of_season: prior.map(|board| board.as_of_season),
        methodology: Some(ProspectProgramMethodologyView::from_config(config)),
        organizations: programs.len(),
        studies: player_ids.len(),
        ranked_studies: programs.iter().map(|row| row.prospect_count).sum(),
        graduated_studies: programs.iter().map(|row| row.graduated_count).sum(),
        unknown_nhl_games_studies: programs
            .iter()
            .map(|row| row.unknown_nhl_games_count)
            .sum(),
        maximum_nhl_games_played: config.maximum_nhl_games_played,
        programs,
        disclosures: vec![
            format!("This foundation ranks supplied canonical studies with no more than {} regular-season NHL games. Higher-workload young players remain visible in the graduated lane but do not inflate reserve-system scores; this operational boundary is not NHL rookie eligibility.", config.maximum_nhl_games_played),
            "Graduation is applied only when NHL games are observed. Unknown workload remains ranked and is counted explicitly; it is not treated as an observed zero.".to_owned(),
            "Pool score combines top-three observed signal, quality depth, and positional balance. Development score workload-weights same-league trajectory, then applies evidence coverage so uncertainty is not treated as failure.".to_owned(),
            "Pipeline score combines Pool, Development, documented readiness, and confidence. Missing depth lowers depth and confidence rather than being silently imputed.".to_owned(),
            "Hidden-value and public-attention scores are excluded because underrecognition is not prospect quality or ceiling.".to_owned(),
            "Supplied goalie studies use a separate save-percentage, goals-against-average, and workload adapter. Multi-league input does not by itself claim complete organizational coverage; every eligible player must still be supplied through a typed fact adapter.".to_owned(),
            "Positive rank or score delta means improvement from the explicitly dated prior board; comparison requires an earlier season with identical scope, source leagues, graduation threshold, scoring method, weights, and expected depth. Organizations absent from that board retain null deltas.".to_owned(),
        ],
    })
}

fn prospect_component_score(study: &ProspectDevelopmentStudyView, id: &str) -> f64 {
    study
        .components
        .iter()
        .find(|row| row.id == id)
        .expect("validated prospect program component")
        .score
}

fn prospect_position_group(position: &str) -> &'static str {
    match position.trim().to_ascii_uppercase().as_str() {
        "D" | "LD" | "RD" => "D",
        "G" => "G",
        "F" | "C" | "LW" | "RW" | "W" => "F",
        _ => "OTHER",
    }
}

fn round_program_score(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn assign_program_rank(
    programs: &mut [ProspectProgramOrganizationView],
    score: impl Fn(&ProspectProgramOrganizationView) -> f64,
    assign: impl Fn(&mut ProspectProgramOrganizationView, usize),
) {
    let mut indices = (0..programs.len()).collect::<Vec<_>>();
    indices.sort_by(|left, right| {
        score(&programs[*right])
            .total_cmp(&score(&programs[*left]))
            .then_with(|| {
                programs[*left]
                    .organization
                    .cmp(&programs[*right].organization)
            })
    });
    for (offset, index) in indices.into_iter().enumerate() {
        assign(&mut programs[index], offset + 1);
    }
}

pub fn build_prospect_discovery_board(
    studies: Vec<ProspectDevelopmentStudyView>,
) -> Result<ProspectDiscoveryBoardView, String> {
    let mut player_ids = BTreeSet::new();
    let mut hidden_gems = Vec::new();
    let mut buyer_beware = Vec::new();
    let mut watch = Vec::new();
    for study in studies {
        if study.schema != PROSPECT_DEVELOPMENT_STUDY_SCHEMA
            || study.player_id == 0
            || study.player.trim().is_empty()
            || study.organization.trim().is_empty()
            || study.position.trim().is_empty()
            || !study.hidden_value_score.is_finite()
            || !(0.0..=100.0).contains(&study.hidden_value_score)
            || !study.performance_attention_gap.is_finite()
            || study.lenses.iter().any(|lens| {
                !lens.strength.is_finite()
                    || !(0.0..=1.0).contains(&lens.strength)
                    || lens.summary.trim().is_empty()
            })
        {
            return Err("invalid prospect development study supplied to board".to_owned());
        }
        if !player_ids.insert(study.player_id) {
            return Err(format!(
                "duplicate prospect development study for player {}",
                study.player_id
            ));
        }

        let strongest_risk = study
            .lenses
            .iter()
            .filter(|lens| lens.direction == ProspectDiscoveryLensDirection::Risk)
            .map(|lens| lens.strength)
            .fold(0.0_f64, f64::max);
        let has_upside = study
            .lenses
            .iter()
            .any(|lens| lens.direction == ProspectDiscoveryLensDirection::Upside);
        let buyer_beware_signal = study.market_position == ProspectMarketPosition::Overexposed
            || matches!(
                study.classification,
                ProspectHiddenValueClass::OverexposedCooling
                    | ProspectHiddenValueClass::HypeAheadOfEvidence
                    | ProspectHiddenValueClass::SmallSampleHypeRisk
            );
        let hidden_gem_signal =
            matches!(
                study.classification,
                ProspectHiddenValueClass::InjuryObscuredRiser
                    | ProspectHiddenValueClass::InjuryRecoveryWatch
                    | ProspectHiddenValueClass::HiddenRiser
            ) || (study.market_position == ProspectMarketPosition::Underrecognized && has_upside);
        let lane = if buyer_beware_signal {
            ProspectDiscoveryBoardLane::BuyerBeware
        } else if hidden_gem_signal {
            ProspectDiscoveryBoardLane::HiddenGem
        } else {
            ProspectDiscoveryBoardLane::Watch
        };
        let lane_score = match lane {
            ProspectDiscoveryBoardLane::BuyerBeware => {
                strongest_risk.max((-study.performance_attention_gap).clamp(0.0, 1.0)) * 100.0
            }
            ProspectDiscoveryBoardLane::HiddenGem | ProspectDiscoveryBoardLane::Watch => {
                study.hidden_value_score
            }
        };
        let row = ProspectDiscoveryBoardRow {
            rank: 0,
            player_id: study.player_id,
            player: study.player,
            organization: study.organization,
            position: study.position,
            lane,
            classification: study.classification,
            market_position: study.market_position,
            hidden_value_score: study.hidden_value_score,
            performance_attention_gap: study.performance_attention_gap,
            lane_score,
            lenses: study.lenses,
        };
        match lane {
            ProspectDiscoveryBoardLane::HiddenGem => hidden_gems.push(row),
            ProspectDiscoveryBoardLane::BuyerBeware => buyer_beware.push(row),
            ProspectDiscoveryBoardLane::Watch => watch.push(row),
        }
    }

    for rows in [&mut hidden_gems, &mut buyer_beware, &mut watch] {
        rows.sort_by(|left, right| {
            right
                .lane_score
                .total_cmp(&left.lane_score)
                .then_with(|| left.player.cmp(&right.player))
                .then_with(|| left.player_id.cmp(&right.player_id))
        });
        for (index, row) in rows.iter_mut().enumerate() {
            row.rank = index + 1;
        }
    }

    Ok(ProspectDiscoveryBoardView {
        schema: PROSPECT_DISCOVERY_BOARD_SCHEMA.to_owned(),
        studies: player_ids.len(),
        hidden_gems,
        buyer_beware,
        watch,
        disclosures: vec![
            "The board composes validated prospect-development studies and preserves their active discovery lenses; it does not rescore raw source data.".to_owned(),
            "Hidden Gems require supported upside plus underrecognition or a hidden-value classification. Buyer Beware requires overexposure or an explicit hype/cooling classification.".to_owned(),
            "Uncertain workload alone remains Watch and is never treated as negative evidence.".to_owned(),
            "Lane scores rank candidates within a lane and are not comparable across lanes.".to_owned(),
        ],
    })
}

pub fn build_prospect_development_study(
    mut input: ProspectDevelopmentStudyInput,
    config: ProspectDevelopmentStudyConfig,
) -> Result<ProspectDevelopmentStudyView, String> {
    let weight_sum = config.production_weight
        + config.trajectory_weight
        + config.opportunity_weight
        + config.attention_gap_weight;
    let weights = [
        config.production_weight,
        config.trajectory_weight,
        config.opportunity_weight,
        config.attention_gap_weight,
    ];
    if input.player_id == 0
        || input.player.trim().is_empty()
        || input.organization.trim().is_empty()
        || input.position.trim().is_empty()
        || input.seasons.len() < 2
        || !input.attention_score.is_finite()
        || !(0.0..=1.0).contains(&input.attention_score)
        || input.attention_basis.trim().is_empty()
        || !config.production_benchmark_ppg.is_finite()
        || config.production_benchmark_ppg <= 0.0
        || !config.rising_delta_ppg.is_finite()
        || config.rising_delta_ppg <= 0.0
        || config.full_confidence_games == 0
        || config.minimum_comparison_games == 0
        || config.minimum_comparison_games > config.full_confidence_games
        || weights
            .iter()
            .any(|weight| !weight.is_finite() || *weight < 0.0)
        || !weight_sum.is_finite()
        || (weight_sum - 1.0).abs() > 1e-9
        || input.seasons.iter().any(|row| {
            row.season == 0
                || row.league.trim().is_empty()
                || row.games_played == 0
                || row.goals.saturating_add(row.assists) < row.goals
        })
        || input.evidence.iter().any(|item| {
            item.label.trim().is_empty()
                || !(item.source_url.starts_with("https://")
                    || item.source_url.starts_with("http://"))
        })
    {
        return Err("invalid prospect development study input or configuration".to_owned());
    }
    input.seasons.sort_by_key(|row| row.season);
    if input
        .seasons
        .windows(2)
        .any(|rows| rows[0].season == rows[1].season)
    {
        return Err("prospect development seasons must be unique".to_owned());
    }

    let mut seasons = Vec::with_capacity(input.seasons.len());
    for (index, row) in input.seasons.iter().enumerate() {
        let points = row.goals.saturating_add(row.assists);
        let ppg = f64::from(points) / f64::from(row.games_played);
        let prior = (row.games_played >= config.minimum_comparison_games)
            .then(|| {
                input.seasons[..index].iter().rev().find(|prior| {
                    prior.league.eq_ignore_ascii_case(&row.league)
                        && prior.games_played >= config.minimum_comparison_games
                })
            })
            .flatten();
        let prior_ppg = prior.map(|prior| {
            f64::from(prior.goals.saturating_add(prior.assists)) / f64::from(prior.games_played)
        });
        let delta = prior_ppg.map(|prior| ppg - prior);
        seasons.push(ProspectDevelopmentSeasonView {
            season: row.season,
            league: row.league.clone(),
            games_played: row.games_played,
            goals: row.goals,
            assists: row.assists,
            points,
            points_per_game: ppg,
            same_league_ppg_delta: delta,
            same_league_ppg_change: prior_ppg
                .filter(|prior| *prior > 0.0)
                .map(|prior| delta.unwrap_or(0.0) / prior),
        });
    }

    let latest = seasons.last().expect("validated seasons");
    let delta = latest.same_league_ppg_delta;
    let trajectory = match delta {
        Some(value) if value >= config.rising_delta_ppg => ProspectTrajectory::Rising,
        Some(value) if value <= -config.rising_delta_ppg => ProspectTrajectory::Cooling,
        Some(_) => ProspectTrajectory::Stable,
        None => ProspectTrajectory::Insufficient,
    };
    let latest_confidence =
        (f64::from(latest.games_played) / f64::from(config.full_confidence_games)).min(1.0);
    let prior_confidence = input.seasons[..input.seasons.len() - 1]
        .iter()
        .rev()
        .find(|row| row.league.eq_ignore_ascii_case(&latest.league))
        .map(|row| (f64::from(row.games_played) / f64::from(config.full_confidence_games)).min(1.0))
        .unwrap_or(0.0);
    let workload_confidence = latest_confidence.min(prior_confidence);
    let production_score = (latest.points_per_game / config.production_benchmark_ppg)
        .clamp(0.0, 1.0)
        * latest_confidence;
    let trajectory_score = delta
        .map(|value| {
            (0.5 + value / (2.0 * config.rising_delta_ppg)).clamp(0.0, 1.0) * workload_confidence
        })
        .unwrap_or(0.0);
    let opportunity_score = match input.opportunity {
        ProspectOpportunityStatus::None => 0.0,
        ProspectOpportunityStatus::Monitoring => 0.35,
        ProspectOpportunityStatus::RecallCandidate => 0.7,
        ProspectOpportunityStatus::DebutPlanned => 1.0,
    };
    let attention_gap_score = 1.0 - input.attention_score;
    let component_values = [
        ("production", production_score, config.production_weight),
        ("trajectory", trajectory_score, config.trajectory_weight),
        ("opportunity", opportunity_score, config.opportunity_weight),
        (
            "attention_gap",
            attention_gap_score,
            config.attention_gap_weight,
        ),
    ];
    let components = component_values
        .iter()
        .map(|(id, score, weight)| ProspectSignalComponentView {
            id: (*id).to_owned(),
            score: *score,
            weight: *weight,
            weighted_points: score * weight * 100.0,
        })
        .collect::<Vec<_>>();
    let hidden_value_score = components
        .iter()
        .map(|component| component.weighted_points)
        .sum::<f64>();
    let performance_attention_gap =
        ((production_score + trajectory_score + opportunity_score) / 3.0) - input.attention_score;
    let market_position = if performance_attention_gap >= 0.2 {
        ProspectMarketPosition::Underrecognized
    } else if performance_attention_gap <= -0.2 {
        ProspectMarketPosition::Overexposed
    } else {
        ProspectMarketPosition::Aligned
    };
    let low_attention = input.attention_score <= 0.4;
    let high_attention = input.attention_score >= 0.65;
    let classification = if trajectory == ProspectTrajectory::Rising
        && low_attention
        && input.availability == ProspectAvailabilityStatus::InjuryInterrupted
        && input.opportunity == ProspectOpportunityStatus::DebutPlanned
    {
        ProspectHiddenValueClass::InjuryObscuredRiser
    } else if trajectory == ProspectTrajectory::Rising
        && low_attention
        && hidden_value_score >= 70.0
    {
        ProspectHiddenValueClass::HiddenRiser
    } else if trajectory == ProspectTrajectory::Rising {
        ProspectHiddenValueClass::VisibleRiser
    } else if input.availability == ProspectAvailabilityStatus::Recovered
        && matches!(
            input.opportunity,
            ProspectOpportunityStatus::RecallCandidate | ProspectOpportunityStatus::DebutPlanned
        )
        && production_score >= 0.75
    {
        ProspectHiddenValueClass::InjuryRecoveryWatch
    } else if trajectory == ProspectTrajectory::Cooling && high_attention {
        ProspectHiddenValueClass::OverexposedCooling
    } else if trajectory == ProspectTrajectory::Insufficient
        && latest_confidence < 0.75
        && high_attention
    {
        ProspectHiddenValueClass::SmallSampleHypeRisk
    } else if market_position == ProspectMarketPosition::Overexposed
        && production_score < 0.65
        && matches!(
            input.opportunity,
            ProspectOpportunityStatus::None | ProspectOpportunityStatus::Monitoring
        )
    {
        ProspectHiddenValueClass::HypeAheadOfEvidence
    } else if trajectory == ProspectTrajectory::Cooling {
        ProspectHiddenValueClass::Cooling
    } else {
        ProspectHiddenValueClass::Watch
    };
    let mut lenses = Vec::new();
    if trajectory == ProspectTrajectory::Rising {
        lenses.push(ProspectDiscoveryLensView {
            kind: ProspectDiscoveryLensKind::ProductionRiser,
            direction: ProspectDiscoveryLensDirection::Upside,
            strength: trajectory_score,
            summary: "Same-league scoring rate cleared the configured rising threshold.".to_owned(),
        });
    }
    if input.availability == ProspectAvailabilityStatus::InjuryInterrupted {
        lenses.push(ProspectDiscoveryLensView {
            kind: ProspectDiscoveryLensKind::InjuryObscured,
            direction: ProspectDiscoveryLensDirection::Context,
            strength: opportunity_score,
            summary: "Injury interrupted documented opportunity; it does not reduce the development signal or add score.".to_owned(),
        });
    }
    if input.availability == ProspectAvailabilityStatus::Recovered
        && trajectory == ProspectTrajectory::Insufficient
    {
        lenses.push(ProspectDiscoveryLensView {
            kind: ProspectDiscoveryLensKind::RecoveryUnproven,
            direction: ProspectDiscoveryLensDirection::Context,
            strength: 1.0 - workload_confidence,
            summary: "The return is productive, but the injured comparison season is too small to prove a trend.".to_owned(),
        });
    }
    if matches!(
        input.opportunity,
        ProspectOpportunityStatus::RecallCandidate | ProspectOpportunityStatus::DebutPlanned
    ) {
        lenses.push(ProspectDiscoveryLensView {
            kind: ProspectDiscoveryLensKind::OpportunityBacked,
            direction: ProspectDiscoveryLensDirection::Upside,
            strength: opportunity_score,
            summary: "Documented recall or debut intent supports the performance signal."
                .to_owned(),
        });
    }
    if market_position == ProspectMarketPosition::Underrecognized {
        lenses.push(ProspectDiscoveryLensView {
            kind: ProspectDiscoveryLensKind::AttentionLag,
            direction: ProspectDiscoveryLensDirection::Upside,
            strength: performance_attention_gap.clamp(0.0, 1.0),
            summary: "The authored attention estimate trails the combined performance and opportunity evidence.".to_owned(),
        });
    } else if market_position == ProspectMarketPosition::Overexposed {
        lenses.push(ProspectDiscoveryLensView {
            kind: ProspectDiscoveryLensKind::AttentionAheadOfEvidence,
            direction: ProspectDiscoveryLensDirection::Risk,
            strength: (-performance_attention_gap).clamp(0.0, 1.0),
            summary: "The authored attention estimate is ahead of the combined performance and opportunity evidence.".to_owned(),
        });
    }
    if workload_confidence < 0.75 {
        lenses.push(ProspectDiscoveryLensView {
            kind: ProspectDiscoveryLensKind::WorkloadUncertain,
            direction: ProspectDiscoveryLensDirection::Risk,
            strength: 1.0 - workload_confidence,
            summary: "The comparable same-league workload is below the confidence gate.".to_owned(),
        });
    }
    if trajectory == ProspectTrajectory::Cooling {
        lenses.push(ProspectDiscoveryLensView {
            kind: ProspectDiscoveryLensKind::CoolingSignal,
            direction: ProspectDiscoveryLensDirection::Risk,
            strength: (0.5 - trajectory_score).max(0.0) * 2.0,
            summary: "Same-league scoring rate declined beyond the configured cooling threshold."
                .to_owned(),
        });
    }

    Ok(ProspectDevelopmentStudyView {
        schema: PROSPECT_DEVELOPMENT_STUDY_SCHEMA.to_owned(),
        player_id: input.player_id,
        player: input.player,
        organization: input.organization,
        position: input.position,
        age: input.age,
        nhl_games_played: input.nhl_games_played,
        nhl_games_authority: if input.nhl_games_played > 0 {
            ProspectNhlGamesAuthority::Observed
        } else {
            ProspectNhlGamesAuthority::Unknown
        },
        seasons,
        trajectory,
        workload_confidence,
        opportunity: input.opportunity,
        availability: input.availability,
        attention_score: input.attention_score,
        attention_basis: input.attention_basis,
        performance_attention_gap,
        market_position,
        hidden_value_score,
        classification,
        components,
        lenses,
        evidence: input.evidence,
        disclosures: vec![
            "The hidden-value score combines latest production, same-league trajectory, documented opportunity, and an explicitly authored attention estimate; it is a discovery signal, not an NHL-equivalency projection.".to_owned(),
            "Injury explains interrupted opportunity but does not add points to the score; availability remains a separate labeled state.".to_owned(),
            "Raw scoring is compared only with an earlier season in the same league, preventing junior-to-pro league changes from masquerading as development decline.".to_owned(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn program_board_ranks_three_views_and_computes_prior_deltas() {
        let mut prior_sea = program_study(1, "SEA", "RW", 8, 0.2);
        let mut prior_nyr = program_study(2, "NYR", "D", 24, 0.2);
        for study in [&mut prior_sea, &mut prior_nyr] {
            for season in &mut study.seasons {
                season.season -= 10_001;
            }
        }
        let prior = build_prospect_program_board(
            vec![prior_sea, prior_nyr],
            None,
            ProspectProgramBoardConfig::default(),
        )
        .unwrap();
        let current = build_prospect_program_board(
            vec![
                program_study(1, "SEA", "RW", 26, 0.2),
                program_study(2, "NYR", "D", 7, 0.2),
            ],
            Some(&prior),
            ProspectProgramBoardConfig::default(),
        )
        .unwrap();

        assert_eq!(current.schema, PROSPECT_PROGRAM_BOARD_SCHEMA);
        assert_eq!(current.scope, "ahl_observed");
        assert_eq!(current.organizations, 2);
        assert_eq!(current.ranked_studies, 2);
        assert_eq!(current.graduated_studies, 0);
        assert_eq!(current.unknown_nhl_games_studies, 2);
        assert_eq!(current.maximum_nhl_games_played, 50);
        assert_eq!(current.prior_as_of_season, Some(20242025));
        let methodology = current.methodology.as_ref().unwrap();
        assert_eq!(methodology.scoring_method, PROSPECT_PROGRAM_SCORING_METHOD);
        assert_eq!(methodology.expected_depth, 10);
        assert_eq!(current.programs[0].organization, "SEA");
        assert_eq!(current.programs[0].pipeline_rank, 1);
        assert_eq!(current.programs[0].pipeline_rank_delta, Some(1));
        assert_eq!(current.programs[1].pipeline_rank_delta, Some(-1));
        assert!(current.programs[0].pipeline_score_delta.unwrap() > 0.0);
        assert!(current.programs[1].pipeline_score_delta.unwrap() < 0.0);
        assert!(current.programs[0].pool_score > current.programs[1].pool_score);
        assert_eq!(current.programs[0].positions.forwards, 1);
        assert_eq!(current.programs[1].positions.defensemen, 1);
    }

    #[test]
    fn program_board_rejects_nonhistorical_or_incompatible_prior() {
        let current_study = program_study(1, "SEA", "RW", 20, 0.2);
        let same_season = build_prospect_program_board(
            vec![program_study(1, "SEA", "RW", 8, 0.2)],
            None,
            ProspectProgramBoardConfig::default(),
        )
        .unwrap();
        let error = build_prospect_program_board(
            vec![current_study.clone()],
            Some(&same_season),
            ProspectProgramBoardConfig::default(),
        )
        .unwrap_err();
        assert!(error.contains("precede"));

        let mut older_study = program_study(1, "SEA", "RW", 8, 0.2);
        for season in &mut older_study.seasons {
            season.season -= 10_001;
        }
        let older = build_prospect_program_board(
            vec![older_study],
            None,
            ProspectProgramBoardConfig::default(),
        )
        .unwrap();
        let changed_methodology = ProspectProgramBoardConfig {
            expected_depth: 12,
            ..ProspectProgramBoardConfig::default()
        };
        let error = build_prospect_program_board(
            vec![current_study.clone()],
            Some(&older),
            changed_methodology,
        )
        .unwrap_err();
        assert!(error.contains("scoring methodology"));

        let mut unproven_methodology = older.clone();
        unproven_methodology.methodology = None;
        let error = build_prospect_program_board(
            vec![current_study.clone()],
            Some(&unproven_methodology),
            ProspectProgramBoardConfig::default(),
        )
        .unwrap_err();
        assert!(error.contains("unproven scoring methodology"));

        let mut incompatible = current_study;
        for season in &mut incompatible.seasons {
            season.league = "OHL".to_owned();
        }
        let error = build_prospect_program_board(
            vec![incompatible],
            Some(&older),
            ProspectProgramBoardConfig::default(),
        )
        .unwrap_err();
        assert!(error.contains("same scope and source leagues"));
    }

    #[test]
    fn program_history_recomputes_chronological_team_deltas() {
        let oldest = build_prospect_program_board(
            vec![
                program_study_years_back(1, "SEA", "RW", 8, 2),
                program_study_years_back(2, "NYR", "D", 30, 2),
            ],
            None,
            ProspectProgramBoardConfig::default(),
        )
        .unwrap();
        let middle = build_prospect_program_board(
            vec![
                program_study_years_back(1, "SEA", "RW", 30, 1),
                program_study_years_back(2, "NYR", "D", 8, 1),
            ],
            None,
            ProspectProgramBoardConfig::default(),
        )
        .unwrap();
        let latest = build_prospect_program_board(
            vec![
                program_study(1, "SEA", "RW", 40, 0.2),
                program_study(2, "NYR", "D", 7, 0.2),
            ],
            None,
            ProspectProgramBoardConfig::default(),
        )
        .unwrap();
        let history = build_prospect_program_history(vec![latest, oldest, middle]).unwrap();
        assert_eq!(history.schema, PROSPECT_PROGRAM_HISTORY_SCHEMA);
        assert_eq!(history.seasons, vec![20232024, 20242025, 20252026]);
        assert_eq!(history.boards, 3);
        assert_eq!(history.organizations, 2);
        let sea = history
            .programs
            .iter()
            .find(|program| program.organization == "SEA")
            .unwrap();
        assert_eq!(sea.seasons_observed, 3);
        assert_eq!(sea.pipeline_rank_change, 1);
        assert_eq!(sea.points[0].pipeline_rank_delta, None);
        assert_eq!(sea.points[1].pipeline_rank_delta, Some(1));
        assert_eq!(sea.points[2].pipeline_rank_delta, Some(0));
        assert!(sea.pipeline_score_change > 0.0);
    }

    #[test]
    fn program_history_rejects_duplicate_or_cross_method_boards() {
        let older = build_prospect_program_board(
            vec![program_study_years_back(1, "SEA", "RW", 8, 1)],
            None,
            ProspectProgramBoardConfig::default(),
        )
        .unwrap();
        let latest = build_prospect_program_board(
            vec![program_study(1, "SEA", "RW", 20, 0.2)],
            None,
            ProspectProgramBoardConfig::default(),
        )
        .unwrap();
        let error =
            build_prospect_program_history(vec![latest.clone(), latest.clone()]).unwrap_err();
        assert!(error.contains("unique"));

        let mut changed = latest;
        changed.methodology.as_mut().unwrap().expected_depth = 12;
        let error = build_prospect_program_history(vec![older, changed]).unwrap_err();
        assert!(error.contains("methodologically comparable"));
    }

    #[test]
    fn program_board_does_not_treat_attention_as_talent() {
        let low_attention = program_study(10, "AAA", "C", 18, 0.1);
        let high_attention = program_study(20, "BBB", "C", 18, 0.9);
        assert_ne!(
            low_attention.hidden_value_score,
            high_attention.hidden_value_score
        );
        let board = build_prospect_program_board(
            vec![low_attention, high_attention],
            None,
            ProspectProgramBoardConfig::default(),
        )
        .unwrap();
        assert_eq!(
            board.programs[0].pipeline_score,
            board.programs[1].pipeline_score
        );
        assert_eq!(board.programs[0].organization, "AAA");
        assert!(board.disclosures.iter().any(|row| row.contains("excluded")));
    }

    #[test]
    fn program_board_keeps_graduates_visible_but_out_of_reserve_scores() {
        let reserve = program_study(10, "SEA", "C", 18, 0.5);
        let mut graduate = program_study(20, "SEA", "RW", 40, 0.5);
        graduate.nhl_games_played = 51;
        graduate.nhl_games_authority = ProspectNhlGamesAuthority::Observed;
        let board = build_prospect_program_board(
            vec![reserve, graduate],
            None,
            ProspectProgramBoardConfig::default(),
        )
        .unwrap();
        let program = &board.programs[0];
        assert_eq!(board.studies, 2);
        assert_eq!(board.ranked_studies, 1);
        assert_eq!(board.graduated_studies, 1);
        assert_eq!(program.prospect_count, 1);
        assert_eq!(program.supplied_study_count, 2);
        assert_eq!(program.graduated_count, 1);
        assert_eq!(program.unknown_nhl_games_count, 1);
        assert_eq!(program.top_prospects[0].player_id, 10);
        assert_eq!(program.graduates[0].player_id, 20);
        assert_eq!(program.graduates[0].nhl_games_played, 51);
    }

    #[test]
    fn program_board_does_not_graduate_unknown_nhl_workload() {
        let mut unknown = program_study(20, "SEA", "RW", 40, 0.5);
        unknown.nhl_games_played = 200;
        assert_eq!(
            unknown.nhl_games_authority,
            ProspectNhlGamesAuthority::Unknown
        );
        let board = build_prospect_program_board(
            vec![unknown],
            None,
            ProspectProgramBoardConfig::default(),
        )
        .unwrap();
        assert_eq!(board.ranked_studies, 1);
        assert_eq!(board.graduated_studies, 0);
        assert_eq!(board.unknown_nhl_games_studies, 1);
    }

    #[test]
    fn program_sensitivity_freezes_rank_range_across_gp_boundaries() {
        let weak_reserve = program_study(10, "AAA", "C", 8, 0.5);
        let mut strong_graduate = program_study(20, "AAA", "RW", 60, 0.5);
        strong_graduate.nhl_games_played = 51;
        strong_graduate.nhl_games_authority = ProspectNhlGamesAuthority::Observed;
        let stable_program = program_study(30, "BBB", "D", 24, 0.5);
        let view = build_prospect_program_sensitivity_with_goalies(
            vec![weak_reserve, strong_graduate, stable_program],
            vec![],
            vec![82, 25, 50],
            ProspectProgramBoardConfig::default(),
        )
        .unwrap();
        assert_eq!(view.schema, PROSPECT_PROGRAM_SENSITIVITY_SCHEMA);
        assert_eq!(
            view.methodology.as_ref().unwrap().scoring_method,
            PROSPECT_PROGRAM_SCORING_METHOD
        );
        assert_eq!(view.thresholds, vec![25, 50, 82]);
        assert_eq!(view.organizations, 2);
        assert_eq!(view.supplied_studies, 3);
        let aaa = view
            .programs
            .iter()
            .find(|program| program.organization == "AAA")
            .unwrap();
        assert_eq!(aaa.best_pipeline_rank, 1);
        assert_eq!(aaa.worst_pipeline_rank, 2);
        assert_eq!(aaa.pipeline_rank_span, 1);
        assert_eq!(aaa.points[0].ranked_studies, 1);
        assert_eq!(aaa.points[0].graduated_studies, 1);
        assert_eq!(aaa.points[2].ranked_studies, 2);
        assert_eq!(aaa.points[2].graduated_studies, 0);
        assert!(aaa.pipeline_score_span > 0.0);
    }

    #[test]
    fn program_sensitivity_rejects_duplicate_thresholds() {
        let error = build_prospect_program_sensitivity_with_goalies(
            vec![program_study(10, "AAA", "C", 8, 0.5)],
            vec![],
            vec![50, 50],
            ProspectProgramBoardConfig::default(),
        )
        .unwrap_err();
        assert!(error.contains("unique"));
    }

    #[test]
    fn goalie_study_uses_goalie_rates_and_enters_program_balance() {
        let goalie = build_prospect_goalie_development_study(
            ProspectGoalieDevelopmentStudyInput {
                player_id: 30,
                player: "Goalie Prospect".to_owned(),
                organization: "SEA".to_owned(),
                age: 22,
                nhl_games_played: 0,
                seasons: vec![
                    ProspectGoalieDevelopmentSeasonInput {
                        season: 20242025,
                        league: "AHL".to_owned(),
                        games_played: 28,
                        save_percentage: 0.898,
                        goals_against_average: 3.15,
                    },
                    ProspectGoalieDevelopmentSeasonInput {
                        season: 20252026,
                        league: "AHL".to_owned(),
                        games_played: 34,
                        save_percentage: 0.914,
                        goals_against_average: 2.61,
                    },
                ],
                opportunity: ProspectOpportunityStatus::RecallCandidate,
                availability: ProspectAvailabilityStatus::Healthy,
                evidence: vec![],
            },
            ProspectGoalieDevelopmentStudyConfig::default(),
        )
        .unwrap();

        assert_eq!(goalie.schema, PROSPECT_GOALIE_DEVELOPMENT_STUDY_SCHEMA);
        assert_eq!(goalie.trajectory, ProspectTrajectory::Rising);
        assert!(goalie.seasons[1].same_league_save_percentage_delta.unwrap() > 0.015);
        assert!(
            goalie.seasons[1]
                .same_league_goals_against_average_improvement
                .unwrap()
                > 0.5
        );
        let board = build_prospect_program_board_with_goalies(
            vec![program_study(1, "SEA", "RW", 24, 0.5)],
            vec![goalie],
            None,
            ProspectProgramBoardConfig::default(),
        )
        .unwrap();
        assert_eq!(board.studies, 2);
        assert_eq!(board.programs[0].positions.goalies, 1);
        assert!(board.programs[0]
            .top_prospects
            .iter()
            .any(|row| row.position == "G"));
    }

    fn program_study(
        player_id: u32,
        organization: &str,
        position: &str,
        latest_points: u32,
        attention_score: f64,
    ) -> ProspectDevelopmentStudyView {
        build_prospect_development_study(
            ProspectDevelopmentStudyInput {
                player_id,
                player: format!("Prospect {player_id}"),
                organization: organization.to_owned(),
                position: position.to_owned(),
                age: 21,
                nhl_games_played: 0,
                seasons: vec![
                    ProspectDevelopmentSeasonInput {
                        season: 20242025,
                        league: "AHL".to_owned(),
                        games_played: 40,
                        goals: 5,
                        assists: 5,
                    },
                    ProspectDevelopmentSeasonInput {
                        season: 20252026,
                        league: "AHL".to_owned(),
                        games_played: 40,
                        goals: latest_points / 2,
                        assists: latest_points - latest_points / 2,
                    },
                ],
                opportunity: ProspectOpportunityStatus::RecallCandidate,
                availability: ProspectAvailabilityStatus::Healthy,
                attention_score,
                attention_basis: "Test attention estimate.".to_owned(),
                evidence: vec![],
            },
            ProspectDevelopmentStudyConfig::default(),
        )
        .unwrap()
    }

    fn program_study_years_back(
        player_id: u32,
        organization: &str,
        position: &str,
        latest_points: u32,
        years: u32,
    ) -> ProspectDevelopmentStudyView {
        let mut study = program_study(player_id, organization, position, latest_points, 0.2);
        for season in &mut study.seasons {
            season.season -= 10_001 * years;
        }
        study
    }

    #[test]
    fn firkus_is_an_injury_obscured_riser_with_transparent_components() {
        let view = build_prospect_development_study(
            ProspectDevelopmentStudyInput {
                player_id: 8_483_442,
                player: "Jagger Firkus".to_owned(),
                organization: "SEA".to_owned(),
                position: "RW".to_owned(),
                age: 22,
                nhl_games_played: 0,
                seasons: vec![
                    ProspectDevelopmentSeasonInput {
                        season: 20242025,
                        league: "AHL".to_owned(),
                        games_played: 69,
                        goals: 15,
                        assists: 21,
                    },
                    ProspectDevelopmentSeasonInput {
                        season: 20252026,
                        league: "AHL".to_owned(),
                        games_played: 63,
                        goals: 21,
                        assists: 35,
                    },
                ],
                opportunity: ProspectOpportunityStatus::DebutPlanned,
                availability: ProspectAvailabilityStatus::InjuryInterrupted,
                attention_score: 0.2,
                attention_basis: "Low public visibility after missing his planned NHL debut."
                    .to_owned(),
                evidence: vec![ProspectStudyEvidenceInput {
                    label: "Kraken GM said injury prevented a planned NHL debut.".to_owned(),
                    source_url: "https://www.nhl.com/kraken/news/2026-nhl-draft-behind-the-scenes-seattle-kraken-draft-room".to_owned(),
                }],
            },
            ProspectDevelopmentStudyConfig::default(),
        )
        .unwrap();

        assert_eq!(view.trajectory, ProspectTrajectory::Rising);
        assert_eq!(
            view.classification,
            ProspectHiddenValueClass::InjuryObscuredRiser
        );
        assert!((view.seasons[1].points_per_game - 56.0 / 63.0).abs() < 1e-9);
        assert!(view.seasons[1].same_league_ppg_change.unwrap() > 0.70);
        assert!(view.hidden_value_score > 90.0);
        assert_eq!(view.components.len(), 4);
        assert_eq!(
            view.market_position,
            ProspectMarketPosition::Underrecognized
        );
        assert!(view
            .lenses
            .iter()
            .any(|lens| lens.kind == ProspectDiscoveryLensKind::InjuryObscured));
    }

    #[test]
    fn cross_league_seasons_do_not_invent_a_trajectory() {
        let input = ProspectDevelopmentStudyInput {
            player_id: 1,
            player: "Prospect".to_owned(),
            organization: "SEA".to_owned(),
            position: "C".to_owned(),
            age: 20,
            nhl_games_played: 0,
            seasons: vec![
                ProspectDevelopmentSeasonInput {
                    season: 20232024,
                    league: "WHL".to_owned(),
                    games_played: 60,
                    goals: 40,
                    assists: 50,
                },
                ProspectDevelopmentSeasonInput {
                    season: 20242025,
                    league: "AHL".to_owned(),
                    games_played: 60,
                    goals: 12,
                    assists: 18,
                },
            ],
            opportunity: ProspectOpportunityStatus::Monitoring,
            availability: ProspectAvailabilityStatus::Healthy,
            attention_score: 0.5,
            attention_basis: "Estimated from current coverage.".to_owned(),
            evidence: Vec::new(),
        };
        let view =
            build_prospect_development_study(input, ProspectDevelopmentStudyConfig::default())
                .unwrap();
        assert_eq!(view.trajectory, ProspectTrajectory::Insufficient);
        assert_eq!(view.seasons[1].same_league_ppg_delta, None);
    }

    #[test]
    fn tiny_injury_season_does_not_invent_a_recovery_decline() {
        let input = ProspectDevelopmentStudyInput {
            player_id: 8_482_162,
            player: "Roby Jarventie".to_owned(),
            organization: "EDM".to_owned(),
            position: "LW".to_owned(),
            age: 23,
            nhl_games_played: 10,
            seasons: vec![
                ProspectDevelopmentSeasonInput {
                    season: 20242025,
                    league: "AHL".to_owned(),
                    games_played: 2,
                    goals: 0,
                    assists: 2,
                },
                ProspectDevelopmentSeasonInput {
                    season: 20252026,
                    league: "AHL".to_owned(),
                    games_played: 52,
                    goals: 17,
                    assists: 19,
                },
            ],
            opportunity: ProspectOpportunityStatus::RecallCandidate,
            availability: ProspectAvailabilityStatus::Recovered,
            attention_score: 0.25,
            attention_basis: "Low visibility after two long-term injuries.".to_owned(),
            evidence: vec![ProspectStudyEvidenceInput {
                label: "Oilers documented two long-term injury setbacks and his recall."
                    .to_owned(),
                source_url: "https://www.nhl.com/oilers/news/blog-jarventie-ready-for-second-nhl-opportunity-after-overcoming-injuries".to_owned(),
            }],
        };
        let view =
            build_prospect_development_study(input, ProspectDevelopmentStudyConfig::default())
                .unwrap();
        assert_eq!(view.trajectory, ProspectTrajectory::Insufficient);
        assert_eq!(view.seasons[1].same_league_ppg_delta, None);
        assert_eq!(
            view.classification,
            ProspectHiddenValueClass::InjuryRecoveryWatch
        );
        assert!(view
            .lenses
            .iter()
            .any(|lens| lens.kind == ProspectDiscoveryLensKind::RecoveryUnproven));
    }

    #[test]
    fn attention_on_a_tiny_flash_is_flagged_as_hype_risk() {
        let view = build_prospect_development_study(
            study_input(
                5,
                6,
                4,
                10,
                ProspectOpportunityStatus::Monitoring,
                ProspectAvailabilityStatus::Healthy,
                0.85,
            ),
            ProspectDevelopmentStudyConfig::default(),
        )
        .unwrap();
        assert_eq!(view.trajectory, ProspectTrajectory::Insufficient);
        assert_eq!(
            view.classification,
            ProspectHiddenValueClass::SmallSampleHypeRisk
        );
        assert_eq!(view.market_position, ProspectMarketPosition::Overexposed);
        assert!(view
            .lenses
            .iter()
            .any(|lens| { lens.kind == ProspectDiscoveryLensKind::AttentionAheadOfEvidence }));
        assert!(view
            .lenses
            .iter()
            .any(|lens| lens.kind == ProspectDiscoveryLensKind::WorkloadUncertain));
    }

    #[test]
    fn high_attention_and_real_decline_is_overexposed_cooling() {
        let view = build_prospect_development_study(
            study_input(
                60,
                60,
                60,
                30,
                ProspectOpportunityStatus::Monitoring,
                ProspectAvailabilityStatus::Healthy,
                0.8,
            ),
            ProspectDevelopmentStudyConfig::default(),
        )
        .unwrap();
        assert_eq!(view.trajectory, ProspectTrajectory::Cooling);
        assert_eq!(
            view.classification,
            ProspectHiddenValueClass::OverexposedCooling
        );
        assert!(view
            .lenses
            .iter()
            .any(|lens| lens.kind == ProspectDiscoveryLensKind::CoolingSignal));
    }

    #[test]
    fn discovery_board_separates_upside_risk_and_uncertainty() {
        let mut hidden = build_prospect_development_study(
            study_input(
                60,
                30,
                60,
                60,
                ProspectOpportunityStatus::RecallCandidate,
                ProspectAvailabilityStatus::Healthy,
                0.2,
            ),
            ProspectDevelopmentStudyConfig::default(),
        )
        .unwrap();
        hidden.player_id = 1;
        hidden.player = "Hidden Prospect".to_owned();

        let mut buyer = build_prospect_development_study(
            study_input(
                60,
                60,
                60,
                30,
                ProspectOpportunityStatus::Monitoring,
                ProspectAvailabilityStatus::Healthy,
                0.8,
            ),
            ProspectDevelopmentStudyConfig::default(),
        )
        .unwrap();
        buyer.player_id = 2;
        buyer.player = "Overexposed Prospect".to_owned();

        let mut uncertain = hidden.clone();
        uncertain.player_id = 3;
        uncertain.player = "Uncertain Prospect".to_owned();
        uncertain.classification = ProspectHiddenValueClass::Watch;
        uncertain.market_position = ProspectMarketPosition::Aligned;
        uncertain.performance_attention_gap = 0.0;
        uncertain.lenses = vec![ProspectDiscoveryLensView {
            kind: ProspectDiscoveryLensKind::WorkloadUncertain,
            direction: ProspectDiscoveryLensDirection::Risk,
            strength: 0.8,
            summary: "The comparable workload is uncertain.".to_owned(),
        }];

        let mut lower_hidden = hidden.clone();
        lower_hidden.player_id = 4;
        lower_hidden.player = "Lower Hidden Prospect".to_owned();
        lower_hidden.hidden_value_score -= 10.0;

        let board =
            build_prospect_discovery_board(vec![uncertain, lower_hidden, buyer, hidden]).unwrap();
        assert_eq!(board.schema, PROSPECT_DISCOVERY_BOARD_SCHEMA);
        assert_eq!(board.studies, 4);
        assert_eq!(board.hidden_gems[0].player, "Hidden Prospect");
        assert_eq!(board.hidden_gems[0].rank, 1);
        assert_eq!(board.hidden_gems[1].player, "Lower Hidden Prospect");
        assert_eq!(board.hidden_gems[1].rank, 2);
        assert_eq!(board.buyer_beware[0].player, "Overexposed Prospect");
        assert_eq!(board.buyer_beware[0].rank, 1);
        assert_eq!(board.watch[0].player, "Uncertain Prospect");
        assert_eq!(board.watch[0].rank, 1);
    }

    #[test]
    fn discovery_board_rejects_duplicate_player_studies() {
        let study = build_prospect_development_study(
            study_input(
                60,
                30,
                60,
                60,
                ProspectOpportunityStatus::RecallCandidate,
                ProspectAvailabilityStatus::Healthy,
                0.2,
            ),
            ProspectDevelopmentStudyConfig::default(),
        )
        .unwrap();

        let error = build_prospect_discovery_board(vec![study.clone(), study]).unwrap_err();
        assert!(error.contains("duplicate"));
    }

    #[test]
    fn discovery_board_can_represent_an_audited_empty_result() {
        let board = build_prospect_discovery_board(vec![]).unwrap();
        assert_eq!(board.studies, 0);
        assert!(board.hidden_gems.is_empty());
        assert!(board.buyer_beware.is_empty());
        assert!(board.watch.is_empty());
    }

    fn study_input(
        prior_games: u32,
        prior_points: u32,
        latest_games: u32,
        latest_points: u32,
        opportunity: ProspectOpportunityStatus,
        availability: ProspectAvailabilityStatus,
        attention_score: f64,
    ) -> ProspectDevelopmentStudyInput {
        ProspectDevelopmentStudyInput {
            player_id: 9,
            player: "Test Prospect".to_owned(),
            organization: "TST".to_owned(),
            position: "C".to_owned(),
            age: 21,
            nhl_games_played: 0,
            seasons: vec![
                ProspectDevelopmentSeasonInput {
                    season: 20242025,
                    league: "AHL".to_owned(),
                    games_played: prior_games,
                    goals: prior_points / 2,
                    assists: prior_points - prior_points / 2,
                },
                ProspectDevelopmentSeasonInput {
                    season: 20252026,
                    league: "AHL".to_owned(),
                    games_played: latest_games,
                    goals: latest_points / 2,
                    assists: latest_points - latest_points / 2,
                },
            ],
            opportunity,
            availability,
            attention_score,
            attention_basis: "Test attention basis.".to_owned(),
            evidence: Vec::new(),
        }
    }
}
