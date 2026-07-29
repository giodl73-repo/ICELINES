//! Seeded opening-roster selection for The Cut.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::model::Position;

use super::line_combination::{
    build_line_combination_forecast, complete_flexible_forward_shape,
    LineCombinationForecastConfig, LineCombinationForecastView,
};
use super::management_behavior::TeamDecisionProfile;
use super::team_lineup::{
    build_team_lineup_projection, LineupAssignmentEvidence, LineupForwardPosition,
    TeamLineupPlayerInput, TeamLineupPlayerView, TeamLineupProjectionView, TeamLineupRequestedSlot,
};
use super::team_season_forecast::{TeamSeasonOpeningRosterChoice, TeamSeasonOpeningRosterPolicy};
use super::{EvidenceLabel, TeamCeilingLens};

pub const TRAINING_CAMP_FORECAST_SCHEMA: &str = "training_camp_forecast.v1";
pub const TRAINING_CAMP_FORECAST_METHOD: &str = "seeded_constrained_camp.v2";
pub const TRAINING_CAMP_LINEUP_SET_SCHEMA: &str = "training_camp_lineup_set.v1";
pub const TRAINING_CAMP_BLENDER_SET_SCHEMA: &str = "training_camp_blender_set.v1";
pub const TRAINING_CAMP_LEAGUE_FORECAST_SCHEMA: &str = "training_camp_league_forecast.v1";
pub const TRAINING_CAMP_EXPOSURE_BOARD_SCHEMA: &str = "training_camp_exposure_board.v1";
pub const TRAINING_CAMP_TRANSACTION_CONTEXT_SCHEMA: &str = "training_camp_transaction_context.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingCampSimulationInput {
    pub team: String,
    pub season: u32,
    pub config: TrainingCampConfig,
    #[serde(default)]
    pub decision_profile: Option<TeamDecisionProfile>,
    pub players: Vec<TrainingCampPlayerInput>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrainingCampAuthorityStatus {
    ConfirmedPool,
    DegradedFallback,
    InsufficientAuthority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrainingCampCompetitionPoolStatus {
    Authored,
    CurrentRosterOnly,
    PriorSeasonAugmented,
    Thin,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingCampLeagueTeamInput {
    pub simulation: TrainingCampSimulationInput,
    pub authority_status: TrainingCampAuthorityStatus,
    pub competition_pool_status: TrainingCampCompetitionPoolStatus,
    pub current_roster_candidates: usize,
    #[serde(default)]
    pub sourced_overlay_candidates: usize,
    #[serde(default)]
    pub fallback_candidates: usize,
    #[serde(default)]
    pub authority_warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingCampLeagueSimulationInput {
    pub season: u32,
    pub teams: Vec<TrainingCampLeagueTeamInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingCampLeagueTeamView {
    pub team: String,
    pub authority_status: TrainingCampAuthorityStatus,
    pub competition_pool_status: TrainingCampCompetitionPoolStatus,
    pub current_roster_candidates: usize,
    pub sourced_overlay_candidates: usize,
    pub fallback_candidates: usize,
    pub forecast: Option<TrainingCampForecastView>,
    pub error: Option<String>,
    pub authority_warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingCampLeagueForecastView {
    pub schema: String,
    pub season: u32,
    pub teams_requested: usize,
    pub teams_simulated: usize,
    pub teams_degraded: usize,
    pub teams_augmented: usize,
    pub teams_failed: usize,
    pub teams: Vec<TrainingCampLeagueTeamView>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrainingCampExposureLane {
    TransactionReview,
    ContractProtected,
    RosterDecisionReview,
    WaiverWatch,
    DevelopmentAssignment,
    HealthyScratchRotation,
    RosterSecure,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrainingCampTradeProtection {
    #[default]
    Unknown,
    None,
    ModifiedNoTrade,
    NoTrade,
    NoMove,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrainingCampTransactionAuthorityStatus {
    #[default]
    NoRead,
    Partial,
    Sourced,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingCampTransactionPlayerInput {
    pub player_id: u32,
    pub display_name: String,
    pub team: String,
    #[serde(default)]
    pub cap_hit: Option<u64>,
    #[serde(default)]
    pub expiry_year: Option<u16>,
    #[serde(default)]
    pub expiry_type: Option<String>,
    #[serde(default)]
    pub trade_protection: TrainingCampTradeProtection,
    #[serde(default)]
    pub requires_waivers: Option<bool>,
    pub source_urls: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingCampTransactionContextInput {
    pub schema: String,
    pub season: u32,
    pub checked_at: String,
    pub players: Vec<TrainingCampTransactionPlayerInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingCampExposurePressureView {
    pub player_id: u32,
    pub display_name: String,
    pub probability: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingCampExposurePlayerView {
    pub rank: usize,
    pub player_id: u32,
    pub display_name: String,
    pub primary_position: Position,
    pub incumbent: bool,
    pub prospect: bool,
    pub waiver_exempt: bool,
    pub active_probability: f64,
    pub dressed_probability: f64,
    pub healthy_scratch_probability: f64,
    pub unavailable_probability: f64,
    /// Available for the trial but not selected for the active roster.
    pub selection_loss_probability: f64,
    /// A selection loss that would expose a non-exempt player to waivers.
    pub waiver_exposure_probability: f64,
    /// Selection-loss probability for a waiver-exempt development player.
    pub development_assignment_probability: f64,
    pub prospect_displacement_probability: f64,
    pub exposure_score: f64,
    pub lane: TrainingCampExposureLane,
    pub transaction_authority_status: TrainingCampTransactionAuthorityStatus,
    #[serde(default)]
    pub cap_hit: Option<u64>,
    #[serde(default)]
    pub contract_expiry_year: Option<u16>,
    #[serde(default)]
    pub contract_expiry_type: Option<String>,
    #[serde(default)]
    pub trade_protection: TrainingCampTradeProtection,
    #[serde(default)]
    pub requires_waivers: Option<bool>,
    #[serde(default)]
    pub transaction_source_urls: Vec<String>,
    #[serde(default)]
    pub transaction_warnings: Vec<String>,
    pub pressure_from: Vec<TrainingCampExposurePressureView>,
    pub source_league: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingCampExposureTeamView {
    pub team: String,
    pub authority_status: TrainingCampAuthorityStatus,
    pub competition_pool_status: TrainingCampCompetitionPoolStatus,
    pub valid_trials: u32,
    pub trials: u32,
    pub players_ranked: usize,
    pub rows: Vec<TrainingCampExposurePlayerView>,
    pub authority_warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingCampExposureBoardView {
    pub schema: String,
    pub season: u32,
    pub source_schema: String,
    pub teams: Vec<TrainingCampExposureTeamView>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TrainingCampConfig {
    pub trials: u32,
    pub seed: u64,
    /// Opening active-roster slots. These may exceed the dressed game shape.
    pub forward_slots: usize,
    pub defense_slots: usize,
    pub goalie_slots: usize,
    pub minimum_centers: usize,
    #[serde(default = "default_dressed_forward_slots")]
    pub dressed_forward_slots: usize,
    #[serde(default = "default_dressed_defense_slots")]
    pub dressed_defense_slots: usize,
    #[serde(default = "default_dressed_goalie_slots")]
    pub dressed_goalie_slots: usize,
    /// NHL active-roster ceiling; 23 unless an historical season overrides it.
    #[serde(default = "default_maximum_active_roster_players")]
    pub maximum_active_roster_players: usize,
    /// Optional cap enforcement. Missing player values fail validation rather
    /// than being treated as zero.
    #[serde(default)]
    pub salary_cap_upper_limit: Option<u64>,
    #[serde(default)]
    pub committed_non_roster_cap: u64,
}

const fn default_dressed_forward_slots() -> usize {
    12
}

const fn default_dressed_defense_slots() -> usize {
    6
}

const fn default_dressed_goalie_slots() -> usize {
    2
}

const fn default_maximum_active_roster_players() -> usize {
    23
}

impl Default for TrainingCampConfig {
    fn default() -> Self {
        Self {
            trials: 10_000,
            seed: 20_262_027,
            forward_slots: 12,
            defense_slots: 6,
            goalie_slots: 2,
            minimum_centers: 4,
            dressed_forward_slots: default_dressed_forward_slots(),
            dressed_defense_slots: default_dressed_defense_slots(),
            dressed_goalie_slots: default_dressed_goalie_slots(),
            maximum_active_roster_players: default_maximum_active_roster_players(),
            salary_cap_upper_limit: None,
            committed_non_roster_cap: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingCampPlayerInput {
    pub player_id: u32,
    pub display_name: String,
    pub primary_position: Position,
    pub eligible_positions: Vec<Position>,
    pub source_league: String,
    pub incumbent: bool,
    #[serde(default)]
    pub rookie_eligible: bool,
    /// Organizational development status. This is independent of NHL rookie
    /// eligibility and of the player's pre-camp roster track.
    #[serde(default)]
    pub prospect: bool,
    /// Evidence-backed roster expectation before camp, when one is available.
    /// It affects selection utility through a disclosed log-odds adjustment;
    /// it is not copied into the simulated make probability.
    #[serde(default)]
    pub pre_camp_make_probability: Option<f64>,
    /// Development-role floor required to keep a forward in the NHL. A
    /// scoring prospect can therefore win a top-nine job without being kept
    /// as a low-minute fourth-line spare.
    #[serde(default)]
    pub minimum_forward_role: Option<TrainingCampForwardRole>,
    pub waiver_exempt: bool,
    /// Current-season cap hit. Absent remains a no-read and may not be used
    /// with configured cap enforcement.
    #[serde(default)]
    pub cap_hit: Option<u64>,
    #[serde(default)]
    pub cap_hit_source: Option<String>,
    /// NHL-equivalent IceLines estimate before evidence-sample shrinkage.
    pub projected_score: f64,
    /// Translated games of evidence; AHL/junior samples must be discounted by
    /// the caller before entering this field.
    pub translated_sample_games: u32,
    /// One-standard-deviation camp/performance uncertainty on the score scale.
    pub camp_std_dev: f64,
    /// Explicit conditioning, injury recovery, or readiness adjustment.
    pub readiness_delta: f64,
    /// Explicit contract/waiver/incumbency preference. This is never inferred.
    pub management_delta: f64,
    pub availability_probability: f64,
    pub evidence_label: EvidenceLabel,
    #[serde(default)]
    pub power_play_role_score: Option<f64>,
    #[serde(default)]
    pub penalty_kill_role_score: Option<f64>,
    /// Reported baseline deployment. If that player makes the active roster,
    /// the game-day builder honors this slot before filling vacancies.
    #[serde(default)]
    pub requested_slot: Option<TeamLineupRequestedSlot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrainingCampForwardRole {
    TopSix,
    TopNine,
}

impl TrainingCampForwardRole {
    fn maximum_rank(self) -> usize {
        match self {
            Self::TopSix => 6,
            Self::TopNine => 9,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrainingCampRosterStatus {
    Lock,
    InsideTrack,
    Bubble,
    LongShot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrainingCampPreCampTrack {
    Lock,
    InsideTrack,
    Bubble,
    OutsideLookingIn,
    Unspecified,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrainingCampSalaryCapStatus {
    Enforced,
    #[default]
    NoRead,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingCampDisplacementView {
    pub player_id: u32,
    pub display_name: String,
    pub probability: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingCampPlayerView {
    pub player_id: u32,
    pub display_name: String,
    pub primary_position: Position,
    /// Retained in the sealed forecast so downstream roster/affiliate
    /// composition never has to reconstruct multi-position eligibility.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub eligible_positions: Vec<Position>,
    pub source_league: String,
    pub incumbent: bool,
    #[serde(default)]
    pub rookie_eligible: bool,
    #[serde(default)]
    pub prospect: bool,
    #[serde(default)]
    pub pre_camp_make_probability: Option<f64>,
    pub pre_camp_track: TrainingCampPreCampTrack,
    #[serde(default)]
    pub roster_prior_delta: f64,
    #[serde(default)]
    pub minimum_forward_role: Option<TrainingCampForwardRole>,
    pub waiver_exempt: bool,
    #[serde(default)]
    pub cap_hit: Option<u64>,
    #[serde(default)]
    pub cap_hit_source: Option<String>,
    pub projected_score: f64,
    pub gp_confidence: f64,
    pub camp_mean: f64,
    #[serde(default)]
    pub management_behavior_delta: f64,
    pub average_sampled_camp_score: f64,
    /// Probability of making the configured opening active roster.
    pub make_probability: f64,
    pub cut_probability: f64,
    /// Probability of missing the active roster because the player was not
    /// available in an otherwise valid constrained trial.
    #[serde(default)]
    pub unavailable_probability: f64,
    /// Probability of being available but losing the active-roster decision.
    #[serde(default)]
    pub selection_loss_probability: f64,
    /// Probability of both making the opening roster and dressing in the
    /// configured 12F/6D/2G game lineup.
    #[serde(default)]
    pub dressed_probability: f64,
    /// Probability of making the opening roster but beginning as a healthy
    /// scratch. This reconciles with make_probability.
    #[serde(default)]
    pub healthy_scratch_probability: f64,
    /// Probability of requiring waivers under this camp outcome. Waiver-
    /// exempt players remain zero; claim probability is not inferred.
    pub waiver_exposure_probability: f64,
    pub status: TrainingCampRosterStatus,
    pub displaced_incumbents: Vec<TrainingCampDisplacementView>,
    pub evidence_label: EvidenceLabel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingCampRosterBranchView {
    pub probability: f64,
    pub trials: u32,
    pub forward_ids: Vec<u32>,
    pub defense_ids: Vec<u32>,
    pub goalie_ids: Vec<u32>,
    #[serde(default)]
    pub total_cap_hit: Option<u64>,
    #[serde(default)]
    pub cap_space: Option<i64>,
    #[serde(default)]
    pub cap_compliant: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingCampForecastView {
    pub schema: String,
    pub method: String,
    pub team: String,
    pub season: u32,
    pub trials: u32,
    pub seed: u64,
    #[serde(default)]
    pub decision_profile_id: Option<String>,
    pub valid_trials: u32,
    pub incomplete_trials: u32,
    pub roster_shape: String,
    pub opening_roster_size: usize,
    pub dressed_roster_size: usize,
    #[serde(default)]
    pub salary_cap_upper_limit: Option<u64>,
    #[serde(default)]
    pub salary_cap_status: TrainingCampSalaryCapStatus,
    pub players: Vec<TrainingCampPlayerView>,
    pub most_common_rosters: Vec<TrainingCampRosterBranchView>,
    pub modal_opening_roster_ids: Vec<u32>,
    pub warnings: Vec<String>,
    pub disclosures: Vec<String>,
}

/// One reusable Cut-to-Blender branch. The lineup remains renderer-neutral;
/// consumers must not rebuild or reinterpret player scores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingCampLineupBranchView {
    pub rank: usize,
    pub probability: f64,
    pub trials: u32,
    pub roster_ids: Vec<u32>,
    pub dressed_ids: Vec<u32>,
    pub healthy_scratch_ids: Vec<u32>,
    pub lineup: TeamLineupProjectionView,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingCampLineupSetView {
    pub schema: String,
    pub team: String,
    pub season: u32,
    pub source_schema: String,
    pub retained_probability: f64,
    pub branches: Vec<TrainingCampLineupBranchView>,
    pub disclosures: Vec<String>,
}

/// Independent player-value authority used when a camp assignment fills an
/// otherwise empty NHL goalie slot. The camp decides assignment; this input
/// decides display value, and the two authorities remain explicit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingCampGoalieValueInput {
    pub player_id: u32,
    pub goalie_quality_score: f64,
    pub sample_games: u32,
    pub evidence_label: EvidenceLabel,
    pub source_method: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingCampBlenderBranchView {
    pub rank: usize,
    pub probability: f64,
    pub roster_ids: Vec<u32>,
    pub best_candidate_id: String,
    pub best_score: f64,
    /// Cross-roster strength change relative to the modal roster branch.
    pub opening_strength_delta: f64,
    pub forecast: LineCombinationForecastView,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingCampBlenderSetView {
    pub schema: String,
    pub team: String,
    pub season: u32,
    pub source_schema: String,
    pub retained_probability: f64,
    pub residual_probability: f64,
    pub branches: Vec<TrainingCampBlenderBranchView>,
    pub opening_roster_policy: TeamSeasonOpeningRosterPolicy,
    pub disclosures: Vec<String>,
}

pub fn build_training_camp_blender_set(
    lineup_set: &TrainingCampLineupSetView,
    config: LineCombinationForecastConfig,
) -> Result<TrainingCampBlenderSetView, String> {
    if lineup_set.branches.is_empty() {
        return Err("training-camp Blender set requires at least one lineup branch".into());
    }
    let mut scored = lineup_set
        .branches
        .iter()
        .map(|branch| {
            let forecast = build_line_combination_forecast(&branch.lineup, &[], config)?;
            let best = forecast
                .candidates
                .first()
                .ok_or_else(|| format!("camp branch {} has no Blender candidates", branch.rank))?;
            let best_candidate_id = best.id.clone();
            let best_score = best.score.total;
            Ok((branch, forecast, best_candidate_id, best_score))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let reference_score = scored[0].3;
    let branches = scored
        .drain(..)
        .map(
            |(branch, forecast, best_candidate_id, best_score)| TrainingCampBlenderBranchView {
                rank: branch.rank,
                probability: branch.probability,
                roster_ids: branch.roster_ids.clone(),
                best_candidate_id,
                best_score,
                opening_strength_delta: ((best_score - reference_score) * 0.25).clamp(-5.0, 5.0),
                forecast,
            },
        )
        .collect::<Vec<_>>();
    let retained_probability = branches
        .iter()
        .map(|branch| branch.probability)
        .sum::<f64>();
    if retained_probability > 1.0 + 1e-6 {
        return Err("training-camp lineup branch probabilities exceed 1.0".into());
    }
    let residual_probability = (1.0 - retained_probability).max(0.0);
    let mut choices = branches
        .iter()
        .map(|branch| TeamSeasonOpeningRosterChoice {
            id: format!("camp-roster-{}", branch.rank),
            label: format!("Camp roster branch {}", branch.rank),
            probability: branch.probability,
            strength_delta: branch.opening_strength_delta,
            roster_ids: branch.roster_ids.clone(),
        })
        .collect::<Vec<_>>();
    if residual_probability > 1e-9 {
        choices.push(TeamSeasonOpeningRosterChoice {
            id: "camp-roster-residual".to_owned(),
            label: "Unretained camp outcome (modal-strength fallback)".to_owned(),
            probability: residual_probability,
            strength_delta: 0.0,
            roster_ids: Vec::new(),
        });
    } else if let Some(last) = choices.last_mut() {
        last.probability += 1.0 - retained_probability;
    }
    Ok(TrainingCampBlenderSetView {
        schema: TRAINING_CAMP_BLENDER_SET_SCHEMA.to_string(),
        team: lineup_set.team.clone(),
        season: lineup_set.season,
        source_schema: lineup_set.schema.clone(),
        retained_probability,
        residual_probability,
        branches,
        opening_roster_policy: TeamSeasonOpeningRosterPolicy {
            team: lineup_set.team.clone(),
            choices,
        },
        disclosures: vec![
            "Every retained camp lineup is independently ranked by The Blender without observed pair evidence.".into(),
            "Cross-roster strength deltas use the same 0.25 Blender score-to-strength scale and the modal camp branch as zero.".into(),
            "Unretained camp probability is sampled as an explicit modal-strength fallback; its exact player roster is unknown and never fabricated.".into(),
        ],
    })
}

/// Score every supplied camp lineup while retaining only the compact season
/// choices. This lets IceCast cover thousands of long-tail camp outcomes
/// without embedding thousands of full Blender documents in the scenario.
pub fn build_training_camp_opening_roster_policy(
    lineup_set: &TrainingCampLineupSetView,
    config: LineCombinationForecastConfig,
) -> Result<TeamSeasonOpeningRosterPolicy, String> {
    if lineup_set.branches.is_empty() {
        return Err("training-camp opening-roster policy requires lineup branches".into());
    }
    let mut scored = Vec::with_capacity(lineup_set.branches.len());
    for branch in &lineup_set.branches {
        let forecast = build_line_combination_forecast(&branch.lineup, &[], config)?;
        let best_score = forecast
            .candidates
            .first()
            .ok_or_else(|| format!("camp branch {} has no Blender candidates", branch.rank))?
            .score
            .total;
        scored.push((branch, best_score));
    }
    let reference_score = scored[0].1;
    let retained_probability = scored
        .iter()
        .map(|(branch, _)| branch.probability)
        .sum::<f64>();
    if retained_probability > 1.0 + 1e-6 {
        return Err("training-camp lineup branch probabilities exceed 1.0".into());
    }
    let mut choices = scored
        .into_iter()
        .map(|(branch, best_score)| TeamSeasonOpeningRosterChoice {
            id: format!("camp-roster-{}", branch.rank),
            label: format!("Camp roster branch {}", branch.rank),
            probability: branch.probability,
            strength_delta: ((best_score - reference_score) * 0.25).clamp(-5.0, 5.0),
            roster_ids: branch.roster_ids.clone(),
        })
        .collect::<Vec<_>>();
    let residual_probability = (1.0 - retained_probability).max(0.0);
    if residual_probability > 1e-9 {
        choices.push(TeamSeasonOpeningRosterChoice {
            id: "camp-roster-residual".to_owned(),
            label: "Unretained camp outcome (modal-strength fallback)".to_owned(),
            probability: residual_probability,
            strength_delta: 0.0,
            roster_ids: Vec::new(),
        });
    } else if let Some(last) = choices.last_mut() {
        last.probability += 1.0 - retained_probability;
    }
    Ok(TeamSeasonOpeningRosterPolicy {
        team: lineup_set.team.clone(),
        choices,
    })
}

pub fn build_training_camp_lineup_set(
    input: &TrainingCampSimulationInput,
    forecast: &TrainingCampForecastView,
    max_branches: usize,
) -> Result<TrainingCampLineupSetView, String> {
    if !input.team.trim().eq_ignore_ascii_case(forecast.team.trim())
        || input.season != forecast.season
    {
        return Err("training-camp input and forecast identify different teams or seasons".into());
    }
    if max_branches == 0 {
        return Err("training-camp lineup set requires at least one branch".into());
    }
    let players = input
        .players
        .iter()
        .map(|player| (player.player_id, player))
        .collect::<BTreeMap<_, _>>();
    let mut branches = Vec::new();
    for (index, branch) in forecast
        .most_common_rosters
        .iter()
        .take(max_branches)
        .enumerate()
    {
        let mut roster_ids = branch.forward_ids.clone();
        roster_ids.extend(&branch.defense_ids);
        roster_ids.extend(&branch.goalie_ids);
        let lineup_players = roster_ids
            .iter()
            .map(|player_id| {
                let player = players.get(player_id).ok_or_else(|| {
                    format!("camp branch references unknown player id {player_id}")
                })?;
                Ok(TeamLineupPlayerInput {
                    player_id: player.player_id,
                    display_name: player.display_name.clone(),
                    team: input.team.clone(),
                    prior_team: None,
                    primary_position: player.primary_position,
                    eligible_positions: player.eligible_positions.clone(),
                    headshot_canonical_url: Some(format!(
                        "https://assets.nhle.com/mugs/nhl/{}/{}/{}.png",
                        input.season,
                        input.team.trim().to_ascii_uppercase(),
                        player.player_id
                    )),
                    games_played: player.translated_sample_games,
                    lens_scores: TeamCeilingLens::ALL
                        .into_iter()
                        .map(|lens| (lens, Some(player.projected_score)))
                        .collect(),
                    score_evidence: player.evidence_label,
                    power_play_role_score: player.power_play_role_score,
                    penalty_kill_role_score: player.penalty_kill_role_score,
                    special_teams_evidence: (player.power_play_role_score.is_some()
                        || player.penalty_kill_role_score.is_some())
                    .then_some(player.evidence_label),
                    requested_slot: player.requested_slot,
                    assignment_evidence: LineupAssignmentEvidence::Scenario,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let lineup = build_team_lineup_projection(&input.team, input.season, lineup_players)
            .map_err(|error| format!("build camp lineup branch {}: {error}", index + 1))?;
        let lineup = complete_flexible_forward_shape(&lineup);
        let healthy_scratch_ids = lineup
            .extras
            .iter()
            .map(|player| player.player_id)
            .collect::<Vec<_>>();
        let scratch_set = healthy_scratch_ids.iter().copied().collect::<BTreeSet<_>>();
        let dressed_ids = roster_ids
            .iter()
            .copied()
            .filter(|player_id| !scratch_set.contains(player_id))
            .collect::<Vec<_>>();
        branches.push(TrainingCampLineupBranchView {
            rank: index + 1,
            probability: branch.probability,
            trials: branch.trials,
            roster_ids,
            dressed_ids,
            healthy_scratch_ids,
            lineup,
        });
    }
    if branches.is_empty() {
        return Err("training-camp forecast contains no reusable roster branches".into());
    }
    let retained_probability = branches.iter().map(|branch| branch.probability).sum();
    Ok(TrainingCampLineupSetView {
        schema: TRAINING_CAMP_LINEUP_SET_SCHEMA.to_string(),
        team: forecast.team.clone(),
        season: forecast.season,
        source_schema: forecast.schema.clone(),
        retained_probability,
        branches,
        disclosures: vec![
            "Branch probabilities come directly from seeded training-camp trials.".into(),
            "Camp projected_score is bridged unchanged into every IceLines score lens; downstream Blender evidence may refine lineup ordering but must preserve this assumption disclosure.".into(),
            "Natural centers and wings may fill either wing when the selected roster lacks a strict natural-side shape.".into(),
            "Only the most common retained branches are represented; retained_probability may be less than 1.0.".into(),
        ],
    })
}

/// Fill only empty goalie slots in an existing NHL lineup from the most
/// probable sealed camp roster branch. Existing assigned goalies and all
/// skater assignments remain fixed. A separate value input is required for
/// every inserted goalie, preventing the camp score from becoming an
/// unlabeled NHL-quality proxy.
pub fn complete_lineup_goalies_from_training_camp(
    baseline: &TeamLineupProjectionView,
    forecast: &TrainingCampForecastView,
    goalie_values: &[TrainingCampGoalieValueInput],
) -> Result<TeamLineupProjectionView, String> {
    if baseline.schema != super::team_lineup::TEAM_LINEUP_PROJECTION_SCHEMA
        || forecast.schema != TRAINING_CAMP_FORECAST_SCHEMA
        || !baseline.team.eq_ignore_ascii_case(&forecast.team)
        || baseline.roster_season != forecast.season
    {
        return Err("camp goalie completion requires aligned lineup and forecast axes".to_owned());
    }
    if baseline.goalies.starter.is_some() && baseline.goalies.backup.is_some() {
        return Ok(baseline.clone());
    }
    let branch = forecast
        .most_common_rosters
        .first()
        .ok_or_else(|| "camp goalie completion requires a modal roster branch".to_owned())?;
    if branch.goalie_ids.len() != 2 {
        return Err("camp modal roster must contain exactly two goalies".to_owned());
    }
    let value_index = goalie_values
        .iter()
        .map(|value| (value.player_id, value))
        .collect::<BTreeMap<_, _>>();
    if value_index.len() != goalie_values.len()
        || goalie_values.iter().any(|value| {
            value.player_id == 0
                || value.sample_games == 0
                || !value.goalie_quality_score.is_finite()
                || !(0.0..=100.0).contains(&value.goalie_quality_score)
                || value.source_method.trim().is_empty()
        })
    {
        return Err("camp goalie completion received invalid value authority".to_owned());
    }
    let mut inputs = lineup_inputs_preserving_assignments(baseline);
    let existing_ids = inputs
        .iter()
        .map(|player| player.player_id)
        .collect::<BTreeSet<_>>();
    let missing_slots = usize::from(baseline.goalies.starter.is_none())
        + usize::from(baseline.goalies.backup.is_none());
    let mut inserted = Vec::new();
    for player_id in branch
        .goalie_ids
        .iter()
        .filter(|player_id| !existing_ids.contains(player_id))
        .take(missing_slots)
    {
        let player = forecast
            .players
            .iter()
            .find(|player| player.player_id == *player_id)
            .ok_or_else(|| format!("camp branch references unknown goalie {player_id}"))?;
        if player.primary_position != Position::Goalie {
            return Err(format!("camp branch player {player_id} is not a goalie"));
        }
        let value = value_index.get(player_id).ok_or_else(|| {
            format!("camp-assigned goalie {player_id} lacks independent value authority")
        })?;
        let requested_slot = if baseline.goalies.starter.is_none() && inserted.is_empty() {
            TeamLineupRequestedSlot::Goalie { starter: true }
        } else {
            TeamLineupRequestedSlot::Goalie { starter: false }
        };
        inputs.push(TeamLineupPlayerInput {
            player_id: player.player_id,
            display_name: player.display_name.clone(),
            team: baseline.team.clone(),
            prior_team: None,
            primary_position: Position::Goalie,
            eligible_positions: vec![Position::Goalie],
            headshot_canonical_url: Some(format!(
                "https://assets.nhle.com/mugs/nhl/{}/{}/{}.png",
                baseline.roster_season, baseline.team, player.player_id
            )),
            games_played: value.sample_games,
            lens_scores: TeamCeilingLens::ALL
                .into_iter()
                .map(|lens| (lens, Some(value.goalie_quality_score)))
                .collect(),
            score_evidence: value.evidence_label,
            power_play_role_score: None,
            penalty_kill_role_score: None,
            special_teams_evidence: None,
            requested_slot: Some(requested_slot),
            assignment_evidence: LineupAssignmentEvidence::Scenario,
        });
        inserted.push((player.player_id, value.source_method.clone()));
    }
    if inserted.len() != missing_slots {
        return Err(format!(
            "camp goalie completion filled {}/{} empty slots",
            inserted.len(),
            missing_slots
        ));
    }
    let mut completed =
        build_team_lineup_projection(&baseline.team, baseline.roster_season, inputs)
            .map_err(|error| format!("rebuild camp-completed lineup: {error}"))?;
    for warning in baseline.warnings.iter().filter(|warning| {
        !matches!(
            warning.code.as_str(),
            "incomplete_roster_shape" | "unrated_players"
        )
    }) {
        if !completed
            .warnings
            .iter()
            .any(|existing| existing.code == warning.code)
        {
            completed.warnings.push(warning.clone());
        }
    }
    completed
        .warnings
        .push(super::team_lineup::TeamLineupWarningView {
            code: "training_camp_goalie_assignment".to_owned(),
            message: format!(
                "Camp modal assignment added {} using independent value method(s): {}.",
                inserted
                    .iter()
                    .map(|(player_id, _)| player_id.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
                inserted
                    .iter()
                    .map(|(_, method)| method.as_str())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        });
    Ok(completed)
}

fn lineup_inputs_preserving_assignments(
    lineup: &TeamLineupProjectionView,
) -> Vec<TeamLineupPlayerInput> {
    let mut inputs = Vec::new();
    for line in &lineup.forward_lines {
        for (player, position) in [
            (line.left_wing.as_ref(), LineupForwardPosition::LeftWing),
            (line.center.as_ref(), LineupForwardPosition::Center),
            (line.right_wing.as_ref(), LineupForwardPosition::RightWing),
        ] {
            if let Some(player) = player {
                let slot_position = match position {
                    LineupForwardPosition::LeftWing => Position::LeftWing,
                    LineupForwardPosition::Center => Position::Center,
                    LineupForwardPosition::RightWing => Position::RightWing,
                };
                let requested_slot = if player.eligible_positions.contains(&slot_position) {
                    TeamLineupRequestedSlot::Forward {
                        line: line.line,
                        position,
                    }
                } else {
                    TeamLineupRequestedSlot::FlexibleForward {
                        line: line.line,
                        position,
                    }
                };
                inputs.push(lineup_player_input(player, requested_slot));
            }
        }
    }
    for pair in &lineup.defense_pairs {
        for (player, right_side) in [(pair.left.as_ref(), false), (pair.right.as_ref(), true)] {
            if let Some(player) = player {
                inputs.push(lineup_player_input(
                    player,
                    TeamLineupRequestedSlot::Defense {
                        pair: pair.pair,
                        right_side,
                    },
                ));
            }
        }
    }
    if let Some(player) = lineup.goalies.starter.as_ref() {
        inputs.push(lineup_player_input(
            player,
            TeamLineupRequestedSlot::Goalie { starter: true },
        ));
    }
    if let Some(player) = lineup.goalies.backup.as_ref() {
        inputs.push(lineup_player_input(
            player,
            TeamLineupRequestedSlot::Goalie { starter: false },
        ));
    }
    inputs.extend(
        lineup
            .extras
            .iter()
            .map(|player| lineup_player_input(player, TeamLineupRequestedSlot::Extra)),
    );
    inputs
}

fn lineup_player_input(
    player: &TeamLineupPlayerView,
    requested_slot: TeamLineupRequestedSlot,
) -> TeamLineupPlayerInput {
    TeamLineupPlayerInput {
        player_id: player.player_id,
        display_name: player.display_name.clone(),
        team: player.team.clone(),
        prior_team: player.prior_team.clone(),
        primary_position: player.primary_position,
        eligible_positions: player.eligible_positions.clone(),
        headshot_canonical_url: player.portrait.headshot_canonical_url.clone(),
        games_played: player.score.sample_games,
        lens_scores: player
            .score
            .components
            .iter()
            .map(|component| (component.lens, component.raw_value))
            .collect(),
        score_evidence: player.score.evidence_label,
        power_play_role_score: player.power_play_role_score,
        penalty_kill_role_score: player.penalty_kill_role_score,
        special_teams_evidence: player.special_teams_evidence,
        requested_slot: Some(requested_slot),
        assignment_evidence: player.assignment_evidence,
    }
}

pub fn simulate_training_camp_league(
    input: &TrainingCampLeagueSimulationInput,
) -> Result<TrainingCampLeagueForecastView, String> {
    let mut seen = BTreeSet::new();
    let mut teams = Vec::with_capacity(input.teams.len());
    for team in &input.teams {
        let abbreviation = team.simulation.team.trim().to_ascii_uppercase();
        if team.simulation.season != input.season {
            return Err(format!(
                "league camp team {abbreviation} has season {}, expected {}",
                team.simulation.season, input.season
            ));
        }
        if !seen.insert(abbreviation.clone()) {
            return Err(format!(
                "league camp contains duplicate team {abbreviation}"
            ));
        }
        let (forecast, error) = if team.authority_status
            == TrainingCampAuthorityStatus::InsufficientAuthority
        {
            (
                None,
                Some("candidate authority cannot fill the configured opening roster".to_owned()),
            )
        } else {
            match simulate_training_camp(&team.simulation) {
                Ok(forecast) => (Some(forecast), None),
                Err(error) => (None, Some(error)),
            }
        };
        teams.push(TrainingCampLeagueTeamView {
            team: abbreviation,
            authority_status: team.authority_status,
            competition_pool_status: team.competition_pool_status,
            current_roster_candidates: team.current_roster_candidates,
            sourced_overlay_candidates: team.sourced_overlay_candidates,
            fallback_candidates: team.fallback_candidates,
            forecast,
            error,
            authority_warnings: team.authority_warnings.clone(),
        });
    }
    teams.sort_by(|a, b| a.team.cmp(&b.team));
    let teams_simulated = teams.iter().filter(|team| team.forecast.is_some()).count();
    let teams_degraded = teams
        .iter()
        .filter(|team| team.authority_status == TrainingCampAuthorityStatus::DegradedFallback)
        .count();
    let teams_augmented = teams
        .iter()
        .filter(|team| {
            team.competition_pool_status == TrainingCampCompetitionPoolStatus::PriorSeasonAugmented
        })
        .count();
    let teams_failed = teams.len() - teams_simulated;
    Ok(TrainingCampLeagueForecastView {
        schema: TRAINING_CAMP_LEAGUE_FORECAST_SCHEMA.to_owned(),
        season: input.season,
        teams_requested: teams.len(),
        teams_simulated,
        teams_degraded,
        teams_augmented,
        teams_failed,
        teams,
        disclosures: vec![
            "Each team is simulated independently through the same seeded opening-roster and dressed-lineup contracts.".to_owned(),
            "Authority status describes whether the opening 14F/7D/2G roster can be sourced; competition_pool_status separately describes how optional camp depth was assembled.".to_owned(),
            "A failed or insufficient team remains in the league document and is never replaced by invented players.".to_owned(),
        ],
    })
}

pub fn build_training_camp_exposure_board(
    league: &TrainingCampLeagueForecastView,
    top_per_team: usize,
) -> Result<TrainingCampExposureBoardView, String> {
    build_training_camp_exposure_board_with_context(league, top_per_team, None)
}

pub fn build_training_camp_exposure_board_with_context(
    league: &TrainingCampLeagueForecastView,
    top_per_team: usize,
    context: Option<&TrainingCampTransactionContextInput>,
) -> Result<TrainingCampExposureBoardView, String> {
    if top_per_team == 0 {
        return Err("Bubble top_per_team must be greater than zero".to_owned());
    }
    let transaction_rows = validate_transaction_context(league, context)?;
    let mut league_memberships: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
    for team in &league.teams {
        if let Some(forecast) = &team.forecast {
            for player in &forecast.players {
                league_memberships
                    .entry(player.player_id)
                    .or_default()
                    .insert(team.team.clone());
            }
        }
    }
    let mut teams = Vec::with_capacity(league.teams.len());
    for team in &league.teams {
        let forecast = team
            .forecast
            .as_ref()
            .ok_or_else(|| format!("Bubble cannot rank {} without a camp forecast", team.team))?;
        let mut rows = forecast
            .players
            .iter()
            .map(|player| {
                let transaction = transaction_rows
                    .get(&(team.team.as_str(), player.player_id))
                    .copied();
                let mut transaction_warnings = Vec::new();
                if let Some(transaction) = transaction {
                    if let Some(requires_waivers) = transaction.requires_waivers {
                        if requires_waivers == player.waiver_exempt {
                            transaction_warnings.push(format!(
                                "transaction context waiver status conflicts with camp input (waiver_exempt={})",
                                player.waiver_exempt
                            ));
                        }
                    }
                }
                let transaction_authority_status = transaction
                    .map(|row| {
                        if row.requires_waivers.is_some()
                            && row.trade_protection != TrainingCampTradeProtection::Unknown
                            && !row.source_urls.is_empty()
                        {
                            TrainingCampTransactionAuthorityStatus::Sourced
                        } else {
                            TrainingCampTransactionAuthorityStatus::Partial
                        }
                    })
                    .unwrap_or_else(|| {
                        if player.cap_hit.is_some() && player.cap_hit_source.is_some() {
                            TrainingCampTransactionAuthorityStatus::Partial
                        } else {
                            TrainingCampTransactionAuthorityStatus::NoRead
                        }
                    });
                let mut pressure_from = forecast
                    .players
                    .iter()
                    .filter(|candidate| candidate.prospect && !candidate.incumbent)
                    .filter_map(|candidate| {
                        candidate
                            .displaced_incumbents
                            .iter()
                            .find(|displaced| displaced.player_id == player.player_id)
                            .map(|displaced| TrainingCampExposurePressureView {
                                player_id: candidate.player_id,
                                display_name: candidate.display_name.clone(),
                                probability: displaced.probability,
                            })
                    })
                    .collect::<Vec<_>>();
                pressure_from.sort_by(|a, b| {
                    b.probability
                        .total_cmp(&a.probability)
                        .then_with(|| a.display_name.cmp(&b.display_name))
                });
                let prospect_displacement_probability = pressure_from
                    .iter()
                    .map(|pressure| pressure.probability)
                    .sum::<f64>()
                    .min(1.0);
                pressure_from.truncate(3);
                let development_assignment_probability = if player.waiver_exempt {
                    player.selection_loss_probability
                } else {
                    0.0
                };
                let exposure_score = (player.selection_loss_probability * 0.55
                    + player.healthy_scratch_probability * 0.30
                    + prospect_displacement_probability * 0.15)
                    .clamp(0.0, 1.0);
                let waiver_confirmed = transaction
                    .and_then(|row| row.requires_waivers)
                    .filter(|requires| *requires != player.waiver_exempt);
                let lane = if transaction.is_some_and(|row| {
                    row.trade_protection == TrainingCampTradeProtection::NoMove
                }) && (player.selection_loss_probability >= 0.10
                    || player.healthy_scratch_probability >= 0.20)
                {
                    transaction_warnings.push(
                        "no-move clause blocks assignment or waivers without player consent"
                            .to_owned(),
                    );
                    TrainingCampExposureLane::ContractProtected
                } else if !player.waiver_exempt
                    && player.incumbent
                    && player.selection_loss_probability >= 0.15
                    && transaction_authority_status
                        == TrainingCampTransactionAuthorityStatus::Sourced
                {
                    TrainingCampExposureLane::TransactionReview
                } else if !player.waiver_exempt
                    && player.selection_loss_probability >= 0.10
                    && waiver_confirmed == Some(true)
                {
                    TrainingCampExposureLane::WaiverWatch
                } else if !player.waiver_exempt && player.selection_loss_probability >= 0.10 {
                    TrainingCampExposureLane::RosterDecisionReview
                } else if player.waiver_exempt && player.selection_loss_probability >= 0.20 {
                    TrainingCampExposureLane::DevelopmentAssignment
                } else if player.healthy_scratch_probability >= 0.20 {
                    TrainingCampExposureLane::HealthyScratchRotation
                } else {
                    TrainingCampExposureLane::RosterSecure
                };
                TrainingCampExposurePlayerView {
                    rank: 0,
                    player_id: player.player_id,
                    display_name: player.display_name.clone(),
                    primary_position: player.primary_position,
                    incumbent: player.incumbent,
                    prospect: player.prospect,
                    waiver_exempt: player.waiver_exempt,
                    active_probability: player.make_probability,
                    dressed_probability: player.dressed_probability,
                    healthy_scratch_probability: player.healthy_scratch_probability,
                    unavailable_probability: player.unavailable_probability,
                    selection_loss_probability: player.selection_loss_probability,
                    waiver_exposure_probability: player.waiver_exposure_probability,
                    development_assignment_probability,
                    prospect_displacement_probability,
                    exposure_score,
                    lane,
                    transaction_authority_status,
                    cap_hit: transaction
                        .and_then(|row| row.cap_hit)
                        .or(player.cap_hit),
                    contract_expiry_year: transaction.and_then(|row| row.expiry_year),
                    contract_expiry_type: transaction.and_then(|row| row.expiry_type.clone()),
                    trade_protection: transaction
                        .map(|row| row.trade_protection)
                        .unwrap_or_default(),
                    requires_waivers: transaction.and_then(|row| row.requires_waivers),
                    transaction_source_urls: transaction
                        .map(|row| row.source_urls.clone())
                        .unwrap_or_default(),
                    transaction_warnings,
                    pressure_from,
                    source_league: player.source_league.clone(),
                }
            })
            .collect::<Vec<_>>();
        rows.sort_by(|a, b| {
            b.exposure_score
                .total_cmp(&a.exposure_score)
                .then_with(|| {
                    b.selection_loss_probability
                        .total_cmp(&a.selection_loss_probability)
                })
                .then_with(|| a.display_name.cmp(&b.display_name))
        });
        let players_ranked = rows.len();
        rows.truncate(top_per_team);
        for (index, row) in rows.iter_mut().enumerate() {
            row.rank = index + 1;
        }
        let mut authority_warnings = team.authority_warnings.clone();
        for player in &forecast.players {
            let memberships = &league_memberships[&player.player_id];
            if memberships.len() > 1 {
                authority_warnings.push(format!(
                    "{} ({}) appears in multiple league camp pools: {}; transaction evidence remains team-scoped",
                    player.display_name,
                    player.player_id,
                    memberships.iter().cloned().collect::<Vec<_>>().join(", ")
                ));
            }
        }
        authority_warnings.sort();
        authority_warnings.dedup();
        teams.push(TrainingCampExposureTeamView {
            team: team.team.clone(),
            authority_status: team.authority_status,
            competition_pool_status: team.competition_pool_status,
            valid_trials: forecast.valid_trials,
            trials: forecast.trials,
            players_ranked,
            rows,
            authority_warnings,
        });
    }
    teams.sort_by(|a, b| a.team.cmp(&b.team));
    Ok(TrainingCampExposureBoardView {
        schema: TRAINING_CAMP_EXPOSURE_BOARD_SCHEMA.to_owned(),
        season: league.season,
        source_schema: league.schema.clone(),
        teams,
        disclosures: vec![
            "The Bubble ranks roster-selection loss, healthy-scratch pressure, and explicitly simulated prospect displacement; injury/unavailability is reported separately and does not create waiver exposure.".to_owned(),
            "Exposure score = 55% selection-loss probability + 30% healthy-scratch probability + 15% disclosed prospect-displacement probability.".to_owned(),
            "transaction_review is emitted only when sourced waiver and trade-protection context is present; it requests analysis and is not a trade prediction because market demand and claim probability are not inferred.".to_owned(),
            "contract_protected means roster pressure exists but a sourced no-move clause blocks assignment or waivers without player consent.".to_owned(),
            "roster_decision_review is the no-read fallback: roster pressure is material, but IceLines lacks enough sourced transaction context to name a transaction path.".to_owned(),
            "Waiver exposure means a non-exempt player was available but lost the active-roster decision; it does not predict that a club submits or another club claims the player.".to_owned(),
            "Player identity collisions across team camp pools are disclosed per team; transaction context is keyed by team and NHL player ID so stale prior-team pools cannot inherit current-team facts.".to_owned(),
        ],
    })
}

fn validate_transaction_context<'a>(
    league: &TrainingCampLeagueForecastView,
    context: Option<&'a TrainingCampTransactionContextInput>,
) -> Result<BTreeMap<(&'a str, u32), &'a TrainingCampTransactionPlayerInput>, String> {
    let Some(context) = context else {
        return Ok(BTreeMap::new());
    };
    if context.schema != TRAINING_CAMP_TRANSACTION_CONTEXT_SCHEMA {
        return Err(format!(
            "unsupported transaction context schema {}; expected {}",
            context.schema, TRAINING_CAMP_TRANSACTION_CONTEXT_SCHEMA
        ));
    }
    if context.season != league.season {
        return Err(format!(
            "transaction context season {} does not match camp season {}",
            context.season, league.season
        ));
    }
    if context.checked_at.trim().is_empty() {
        return Err("transaction context checked_at must not be empty".to_owned());
    }
    let known_players = league
        .teams
        .iter()
        .filter_map(|team| team.forecast.as_ref().map(|forecast| (team, forecast)))
        .flat_map(|(team, forecast)| {
            forecast.players.iter().map(move |player| {
                (
                    (team.team.as_str(), player.player_id),
                    player.display_name.as_str(),
                )
            })
        })
        .collect::<BTreeMap<_, _>>();
    let mut rows = BTreeMap::new();
    for row in &context.players {
        let Some(expected_name) = known_players.get(&(row.team.as_str(), row.player_id)) else {
            return Err(format!(
                "transaction context player {} is not present for {} in the camp forecast",
                row.player_id, row.team
            ));
        };
        if row.display_name != *expected_name {
            return Err(format!(
                "transaction context player {} label does not match camp forecast (expected {} {})",
                row.player_id, row.team, expected_name
            ));
        }
        if rows
            .insert((row.team.as_str(), row.player_id), row)
            .is_some()
        {
            return Err(format!(
                "transaction context contains duplicate player {} for {}",
                row.player_id, row.team
            ));
        }
        if row.source_urls.is_empty() {
            return Err(format!(
                "transaction context player {} requires at least one source URL",
                row.player_id
            ));
        }
        if row.cap_hit == Some(0) {
            return Err(format!(
                "transaction context player {} has a zero cap hit",
                row.player_id
            ));
        }
        if let Some(url) = row
            .source_urls
            .iter()
            .find(|url| !(url.starts_with("https://") || url.starts_with("http://")))
        {
            return Err(format!(
                "transaction context player {} has non-absolute source URL {}",
                row.player_id, url
            ));
        }
    }
    Ok(rows)
}

pub fn simulate_training_camp(
    input: &TrainingCampSimulationInput,
) -> Result<TrainingCampForecastView, String> {
    validate(input)?;
    let group_priors = group_priors(&input.players);
    let means = input
        .players
        .iter()
        .map(|player| {
            let confidence = sample_confidence(player.translated_sample_games);
            let prior = group_priors[&position_group(player.primary_position)];
            prior
                + (player.projected_score - prior) * confidence
                + player.readiness_delta
                + player.management_delta
                + roster_prior_delta(player)
                + input
                    .decision_profile
                    .as_ref()
                    .map(|profile| gm_camp_adjustment(player, profile))
                    .unwrap_or(0.0)
        })
        .collect::<Vec<_>>();
    let mut selected_counts = vec![0u32; input.players.len()];
    let mut available_valid_counts = vec![0u32; input.players.len()];
    let mut dressed_counts = vec![0u32; input.players.len()];
    let mut scratch_counts = vec![0u32; input.players.len()];
    let mut score_sums = vec![0.0; input.players.len()];
    let mut displacements = BTreeMap::<(usize, usize), u32>::new();
    let mut branches = BTreeMap::<(Vec<u32>, Vec<u32>, Vec<u32>), u32>::new();
    let mut valid_trials = 0u32;

    for trial in 0..input.config.trials {
        let mut scores = Vec::with_capacity(input.players.len());
        let mut available = Vec::with_capacity(input.players.len());
        for (index, player) in input.players.iter().enumerate() {
            let confidence = sample_confidence(player.translated_sample_games);
            let mut performance_rng = SimRng(keyed_seed(
                input.config.seed,
                trial,
                player.player_id,
                0x4341_4D50,
            ));
            let uncertainty = player.camp_std_dev * (1.0 + (1.0 - confidence) * 0.5);
            let score = means[index] + performance_rng.standard_normalish() * uncertainty;
            score_sums[index] += score;
            scores.push(score);
            let mut availability_rng = SimRng(keyed_seed(
                input.config.seed,
                trial,
                player.player_id,
                0x4845_414C,
            ));
            available.push(availability_rng.next_f64() < player.availability_probability);
        }

        let Some(forwards) = select_group(
            &input.players,
            &scores,
            &available,
            CampGroup::Forward,
            input.config.forward_slots,
            input.config.minimum_centers,
        ) else {
            continue;
        };
        let Some(defense) = select_group(
            &input.players,
            &scores,
            &available,
            CampGroup::Defense,
            input.config.defense_slots,
            0,
        ) else {
            continue;
        };
        let Some(goalies) = select_group(
            &input.players,
            &scores,
            &available,
            CampGroup::Goalie,
            input.config.goalie_slots,
            0,
        ) else {
            continue;
        };
        let selected = forwards
            .iter()
            .chain(&defense)
            .chain(&goalies)
            .copied()
            .collect::<BTreeSet<_>>();
        let selected_availability = (0..input.players.len())
            .map(|index| selected.contains(&index))
            .collect::<Vec<_>>();
        let Some(dressed_forwards) = select_group(
            &input.players,
            &scores,
            &selected_availability,
            CampGroup::Forward,
            input.config.dressed_forward_slots,
            input.config.minimum_centers,
        ) else {
            continue;
        };
        let Some(dressed_defense) = select_group(
            &input.players,
            &scores,
            &selected_availability,
            CampGroup::Defense,
            input.config.dressed_defense_slots,
            0,
        ) else {
            continue;
        };
        let Some(dressed_goalies) = select_group(
            &input.players,
            &scores,
            &selected_availability,
            CampGroup::Goalie,
            input.config.dressed_goalie_slots,
            0,
        ) else {
            continue;
        };
        let dressed = dressed_forwards
            .iter()
            .chain(&dressed_defense)
            .chain(&dressed_goalies)
            .copied()
            .collect::<BTreeSet<_>>();
        if let Some(upper_limit) = input.config.salary_cap_upper_limit {
            let player_cap = selected
                .iter()
                .map(|index| input.players[*index].cap_hit.unwrap_or(0))
                .sum::<u64>();
            if player_cap + input.config.committed_non_roster_cap > upper_limit {
                continue;
            }
        }
        valid_trials += 1;
        for (index, is_available) in available.iter().copied().enumerate() {
            if is_available {
                available_valid_counts[index] += 1;
            }
        }
        for index in &selected {
            selected_counts[*index] += 1;
            if dressed.contains(index) {
                dressed_counts[*index] += 1;
            } else {
                scratch_counts[*index] += 1;
            }
        }
        for group in [CampGroup::Forward, CampGroup::Defense, CampGroup::Goalie] {
            let mut selected_prospects = selected
                .iter()
                .copied()
                .filter(|index| {
                    !input.players[*index].incumbent
                        && position_group(input.players[*index].primary_position) == group
                })
                .collect::<Vec<_>>();
            let mut cut_incumbents = input
                .players
                .iter()
                .enumerate()
                .filter_map(|(index, player)| {
                    (player.incumbent
                        && !selected.contains(&index)
                        && position_group(player.primary_position) == group)
                        .then_some(index)
                })
                .collect::<Vec<_>>();
            selected_prospects.sort_by(|a, b| scores[*b].total_cmp(&scores[*a]));
            cut_incumbents.sort_by(|a, b| scores[*b].total_cmp(&scores[*a]));
            for (prospect, incumbent) in selected_prospects.into_iter().zip(cut_incumbents) {
                *displacements.entry((prospect, incumbent)).or_default() += 1;
            }
        }
        let mut forward_ids = ids(&input.players, &forwards);
        let mut defense_ids = ids(&input.players, &defense);
        let mut goalie_ids = ids(&input.players, &goalies);
        forward_ids.sort_unstable();
        defense_ids.sort_unstable();
        goalie_ids.sort_unstable();
        *branches
            .entry((forward_ids, defense_ids, goalie_ids))
            .or_default() += 1;
    }

    let probability_denominator = f64::from(valid_trials.max(1));
    let score_denominator = f64::from(input.config.trials);
    let mut players = input
        .players
        .iter()
        .enumerate()
        .map(|(index, player)| {
            let make_probability = f64::from(selected_counts[index]) / probability_denominator;
            let available_probability =
                f64::from(available_valid_counts[index]) / probability_denominator;
            let unavailable_probability = 1.0 - available_probability;
            let selection_loss_probability =
                f64::from(available_valid_counts[index] - selected_counts[index])
                    / probability_denominator;
            let dressed_probability = f64::from(dressed_counts[index]) / probability_denominator;
            let healthy_scratch_probability =
                f64::from(scratch_counts[index]) / probability_denominator;
            let mut displaced_incumbents = displacements
                .iter()
                .filter(|((prospect, _), _)| *prospect == index)
                .map(|((_, incumbent), count)| TrainingCampDisplacementView {
                    player_id: input.players[*incumbent].player_id,
                    display_name: input.players[*incumbent].display_name.clone(),
                    probability: f64::from(*count) / probability_denominator,
                })
                .collect::<Vec<_>>();
            displaced_incumbents.sort_by(|a, b| {
                b.probability
                    .total_cmp(&a.probability)
                    .then_with(|| a.display_name.cmp(&b.display_name))
            });
            TrainingCampPlayerView {
                player_id: player.player_id,
                display_name: player.display_name.clone(),
                primary_position: player.primary_position,
                eligible_positions: player.eligible_positions.clone(),
                source_league: player.source_league.clone(),
                incumbent: player.incumbent,
                rookie_eligible: player.rookie_eligible,
                prospect: player.prospect,
                pre_camp_make_probability: player.pre_camp_make_probability,
                pre_camp_track: pre_camp_track(player.pre_camp_make_probability),
                roster_prior_delta: roster_prior_delta(player),
                minimum_forward_role: player.minimum_forward_role,
                waiver_exempt: player.waiver_exempt,
                cap_hit: player.cap_hit,
                cap_hit_source: player.cap_hit_source.clone(),
                projected_score: player.projected_score,
                gp_confidence: sample_confidence(player.translated_sample_games),
                camp_mean: means[index],
                management_behavior_delta: input
                    .decision_profile
                    .as_ref()
                    .map(|profile| gm_camp_adjustment(player, profile))
                    .unwrap_or(0.0),
                average_sampled_camp_score: score_sums[index] / score_denominator,
                make_probability,
                cut_probability: 1.0 - make_probability,
                unavailable_probability,
                selection_loss_probability,
                dressed_probability,
                healthy_scratch_probability,
                waiver_exposure_probability: if player.waiver_exempt {
                    0.0
                } else {
                    selection_loss_probability
                },
                status: status(make_probability),
                displaced_incumbents,
                evidence_label: player.evidence_label,
            }
        })
        .collect::<Vec<_>>();
    players.sort_by(|a, b| {
        b.make_probability
            .total_cmp(&a.make_probability)
            .then_with(|| b.camp_mean.total_cmp(&a.camp_mean))
            .then_with(|| a.display_name.cmp(&b.display_name))
    });
    let mut most_common_rosters = branches
        .into_iter()
        .map(|((forward_ids, defense_ids, goalie_ids), trials)| {
            let total_cap_hit = input.config.salary_cap_upper_limit.map(|_| {
                forward_ids
                    .iter()
                    .chain(&defense_ids)
                    .chain(&goalie_ids)
                    .filter_map(|player_id| {
                        input
                            .players
                            .iter()
                            .find(|player| player.player_id == *player_id)
                            .and_then(|player| player.cap_hit)
                    })
                    .sum::<u64>()
                    + input.config.committed_non_roster_cap
            });
            let cap_space =
                input
                    .config
                    .salary_cap_upper_limit
                    .zip(total_cap_hit)
                    .map(|(limit, total)| {
                        i64::try_from(limit).unwrap_or(i64::MAX)
                            - i64::try_from(total).unwrap_or(i64::MAX)
                    });
            TrainingCampRosterBranchView {
                probability: if valid_trials == 0 {
                    0.0
                } else {
                    f64::from(trials) / f64::from(valid_trials)
                },
                trials,
                forward_ids,
                defense_ids,
                goalie_ids,
                total_cap_hit,
                cap_space,
                cap_compliant: cap_space.map(|space| space >= 0),
            }
        })
        .collect::<Vec<_>>();
    most_common_rosters.sort_by(|a, b| {
        b.trials
            .cmp(&a.trials)
            .then_with(|| a.forward_ids.cmp(&b.forward_ids))
    });
    let modal_opening_roster_ids = most_common_rosters
        .first()
        .map(|branch| {
            branch
                .forward_ids
                .iter()
                .chain(&branch.defense_ids)
                .chain(&branch.goalie_ids)
                .copied()
                .collect()
        })
        .unwrap_or_default();
    let incomplete_trials = input.config.trials - valid_trials;
    let mut warnings = Vec::new();
    if incomplete_trials > 0 {
        warnings.push(format!(
            "{incomplete_trials} camp trial(s) could not fill the configured healthy roster shape"
        ));
    }
    if input.config.salary_cap_upper_limit.is_none() {
        warnings.push(
            "Salary-cap authority is unavailable; cap compliance was not used to constrain this forecast"
                .to_owned(),
        );
    }
    Ok(TrainingCampForecastView {
        schema: TRAINING_CAMP_FORECAST_SCHEMA.to_owned(),
        method: TRAINING_CAMP_FORECAST_METHOD.to_owned(),
        team: input.team.trim().to_ascii_uppercase(),
        season: input.season,
        trials: input.config.trials,
        seed: input.config.seed,
        decision_profile_id: input
            .decision_profile
            .as_ref()
            .map(|profile| profile.id.clone()),
        valid_trials,
        incomplete_trials,
        roster_shape: format!(
            "opening {}F/{}D/{}G; dressed {}F/{}D/{}G",
            input.config.forward_slots, input.config.defense_slots, input.config.goalie_slots
            , input.config.dressed_forward_slots, input.config.dressed_defense_slots, input.config.dressed_goalie_slots
        ),
        opening_roster_size: input.config.forward_slots
            + input.config.defense_slots
            + input.config.goalie_slots,
        dressed_roster_size: input.config.dressed_forward_slots
            + input.config.dressed_defense_slots
            + input.config.dressed_goalie_slots,
        salary_cap_upper_limit: input.config.salary_cap_upper_limit,
        salary_cap_status: if input.config.salary_cap_upper_limit.is_some() {
            TrainingCampSalaryCapStatus::Enforced
        } else {
            TrainingCampSalaryCapStatus::NoRead
        },
        players,
        most_common_rosters,
        modal_opening_roster_ids,
        warnings,
        disclosures: vec![
            "The Cut samples an explicit NHL-equivalent camp estimate; it does not treat preseason results or scouting assumptions as confirmed NHL performance.".to_owned(),
            "GP confidence uses translated_sample_games/(translated_sample_games+20); callers must disclose league translation before supplying non-NHL evidence.".to_owned(),
            "Management, waiver, contract, readiness, and incumbency effects enter only through disclosed input deltas; IceLines does not invent them.".to_owned(),
            "When supplied, a decision profile adds disclosed GP-confidence-weighted GM opportunity, veteran, and waiver-asset tendencies; manager deployment traits do not alter The Cut.".to_owned(),
            "Each valid trial selects a constrained opening roster before line optimization; make probability is not a transaction prediction.".to_owned(),
            "Player outcome and displacement probabilities are conditioned on valid constrained trials; incomplete trials remain a separate model-quality signal and do not count as player cuts.".to_owned(),
            "cut_probability is total active-roster absence and reconciles as unavailable_probability plus selection_loss_probability; only available selection loss can create waiver exposure.".to_owned(),
            "The Cut selects the configured opening active roster first, then records each player's dressed and healthy-scratch probabilities for the configured game roster; make_probability equals dressed_probability plus healthy_scratch_probability.".to_owned(),
            "When salary_cap_upper_limit is configured, every candidate requires a sourced cap hit and over-limit trials are invalid; absent cap authority remains a no-read rather than zero.".to_owned(),
            "A disclosed minimum_forward_role cuts a development prospect when his sampled camp rank does not earn that usage tier; it never upgrades the player's score.".to_owned(),
            "When supplied, pre_camp_make_probability is converted to a disclosed log-odds roster-prior delta scaled by that player's GP-adjusted camp uncertainty. It represents the as-known roster path, while make_probability remains the constrained camp result.".to_owned(),
        ],
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CampGroup {
    Forward,
    Defense,
    Goalie,
}

fn position_group(position: Position) -> CampGroup {
    if position.is_forward() {
        CampGroup::Forward
    } else if position.is_defense() {
        CampGroup::Defense
    } else {
        CampGroup::Goalie
    }
}

fn group_priors(players: &[TrainingCampPlayerInput]) -> BTreeMap<CampGroup, f64> {
    [CampGroup::Forward, CampGroup::Defense, CampGroup::Goalie]
        .into_iter()
        .map(|group| {
            let values = players
                .iter()
                .filter(|player| position_group(player.primary_position) == group)
                .map(|player| player.projected_score)
                .collect::<Vec<_>>();
            (group, values.iter().sum::<f64>() / values.len() as f64)
        })
        .collect()
}

fn select_group(
    players: &[TrainingCampPlayerInput],
    scores: &[f64],
    available: &[bool],
    group: CampGroup,
    slots: usize,
    minimum_centers: usize,
) -> Option<Vec<usize>> {
    let mut ranked = players
        .iter()
        .enumerate()
        .filter_map(|(index, player)| {
            (available[index] && position_group(player.primary_position) == group).then_some(index)
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| {
        scores[*b]
            .total_cmp(&scores[*a])
            .then_with(|| players[*a].player_id.cmp(&players[*b].player_id))
    });
    if group == CampGroup::Forward {
        let raw_rank = ranked
            .iter()
            .enumerate()
            .map(|(rank, index)| (*index, rank + 1))
            .collect::<BTreeMap<_, _>>();
        ranked.retain(|index| {
            players[*index]
                .minimum_forward_role
                .map(|role| raw_rank[index] <= role.maximum_rank())
                .unwrap_or(true)
        });
    }
    if ranked.len() < slots {
        return None;
    }
    let mut selected = ranked.iter().take(slots).copied().collect::<Vec<_>>();
    if group == CampGroup::Forward {
        while selected
            .iter()
            .filter(|index| {
                players[**index]
                    .eligible_positions
                    .contains(&Position::Center)
            })
            .count()
            < minimum_centers
        {
            let replacement = ranked.iter().skip(slots).copied().find(|index| {
                players[*index]
                    .eligible_positions
                    .contains(&Position::Center)
                    && !selected.contains(index)
            })?;
            let remove = selected
                .iter()
                .copied()
                .filter(|index| {
                    !players[*index]
                        .eligible_positions
                        .contains(&Position::Center)
                })
                .min_by(|a, b| scores[*a].total_cmp(&scores[*b]))?;
            selected.retain(|index| *index != remove);
            selected.push(replacement);
        }
    }
    selected.sort_by(|a, b| scores[*b].total_cmp(&scores[*a]));
    Some(selected)
}

fn ids(players: &[TrainingCampPlayerInput], indices: &[usize]) -> Vec<u32> {
    indices
        .iter()
        .map(|index| players[*index].player_id)
        .collect()
}

fn sample_confidence(games: u32) -> f64 {
    let games = f64::from(games);
    games / (games + 20.0)
}

fn roster_prior_delta(player: &TrainingCampPlayerInput) -> f64 {
    player
        .pre_camp_make_probability
        .map(|probability| {
            let confidence = sample_confidence(player.translated_sample_games);
            let uncertainty = player.camp_std_dev * (1.0 + (1.0 - confidence) * 0.5);
            uncertainty * (probability / (1.0 - probability)).ln()
        })
        .unwrap_or(0.0)
}

fn pre_camp_track(probability: Option<f64>) -> TrainingCampPreCampTrack {
    match probability {
        Some(probability) if probability >= 0.9 => TrainingCampPreCampTrack::Lock,
        Some(probability) if probability >= 0.65 => TrainingCampPreCampTrack::InsideTrack,
        Some(probability) if probability >= 0.25 => TrainingCampPreCampTrack::Bubble,
        Some(_) => TrainingCampPreCampTrack::OutsideLookingIn,
        None => TrainingCampPreCampTrack::Unspecified,
    }
}

fn status(probability: f64) -> TrainingCampRosterStatus {
    if probability >= 0.9 {
        TrainingCampRosterStatus::Lock
    } else if probability >= 0.65 {
        TrainingCampRosterStatus::InsideTrack
    } else if probability >= 0.25 {
        TrainingCampRosterStatus::Bubble
    } else {
        TrainingCampRosterStatus::LongShot
    }
}

fn keyed_seed(seed: u64, trial: u32, player_id: u32, domain: u64) -> u64 {
    seed ^ (u64::from(trial) + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ u64::from(player_id).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ domain
}

struct SimRng(u64);

impl SimRng {
    fn next_f64(&mut self) -> f64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        (value >> 11) as f64 / (1u64 << 53) as f64
    }

    fn standard_normalish(&mut self) -> f64 {
        (0..12).map(|_| self.next_f64()).sum::<f64>() - 6.0
    }
}

fn validate(input: &TrainingCampSimulationInput) -> Result<(), String> {
    if input.team.trim().len() != 3 {
        return Err("The Cut requires a three-letter team abbreviation".to_owned());
    }
    if let Some(profile) = &input.decision_profile {
        profile.validate()?;
        if !profile.team.eq_ignore_ascii_case(&input.team) || profile.season != input.season {
            return Err(
                "training camp and decision profile identify different team seasons".to_owned(),
            );
        }
    }
    if !(100..=1_000_000).contains(&input.config.trials) {
        return Err("The Cut trials must be between 100 and 1,000,000".to_owned());
    }
    if input.config.forward_slots == 0
        || input.config.defense_slots == 0
        || input.config.goalie_slots == 0
        || input.config.minimum_centers > input.config.forward_slots
    {
        return Err("The Cut roster shape is invalid".to_owned());
    }
    let opening_size =
        input.config.forward_slots + input.config.defense_slots + input.config.goalie_slots;
    let dressed_size = input.config.dressed_forward_slots
        + input.config.dressed_defense_slots
        + input.config.dressed_goalie_slots;
    if opening_size > input.config.maximum_active_roster_players
        || dressed_size > opening_size
        || input.config.dressed_forward_slots > input.config.forward_slots
        || input.config.dressed_defense_slots > input.config.defense_slots
        || input.config.dressed_goalie_slots > input.config.goalie_slots
    {
        return Err("The Cut opening/dressed roster relationship is invalid".to_owned());
    }
    if input.config.dressed_forward_slots != 12
        || input.config.dressed_defense_slots != 6
        || input.config.dressed_goalie_slots != 2
    {
        return Err("The Cut currently supports a 12F/6D/2G dressed lineup".to_owned());
    }
    if input
        .config
        .salary_cap_upper_limit
        .is_some_and(|limit| limit == 0 || input.config.committed_non_roster_cap >= limit)
    {
        return Err("The Cut salary-cap configuration is invalid".to_owned());
    }
    let mut ids = BTreeSet::new();
    for player in &input.players {
        if player.player_id == 0 || !ids.insert(player.player_id) {
            return Err("The Cut requires unique non-zero player IDs".to_owned());
        }
        if player.minimum_forward_role.is_some() && !player.primary_position.is_forward() {
            return Err(format!(
                "The Cut player {} has a forward-role floor but is not a forward",
                player.player_id
            ));
        }
        if player.pre_camp_make_probability.is_some_and(|probability| {
            !probability.is_finite() || probability <= 0.0 || probability >= 1.0
        }) {
            return Err(format!(
                "The Cut player {} has an invalid pre-camp make probability",
                player.player_id
            ));
        }
        if input.config.salary_cap_upper_limit.is_some()
            && (player.cap_hit.is_none()
                || player
                    .cap_hit_source
                    .as_deref()
                    .is_none_or(|source| source.trim().is_empty()))
        {
            return Err(format!(
                "The Cut player {} lacks a sourced cap hit required for cap enforcement",
                player.player_id
            ));
        }
        if player.display_name.trim().is_empty()
            || !player.projected_score.is_finite()
            || !player.camp_std_dev.is_finite()
            || player.camp_std_dev < 0.0
            || !player.readiness_delta.is_finite()
            || !player.management_delta.is_finite()
            || !player.availability_probability.is_finite()
            || !(0.0..=1.0).contains(&player.availability_probability)
        {
            return Err(format!(
                "The Cut player {} has invalid inputs",
                player.player_id
            ));
        }
        if !player.eligible_positions.contains(&player.primary_position) {
            return Err(format!(
                "The Cut player {} primary position must be eligible",
                player.player_id
            ));
        }
    }
    for (group, slots) in [
        (CampGroup::Forward, input.config.forward_slots),
        (CampGroup::Defense, input.config.defense_slots),
        (CampGroup::Goalie, input.config.goalie_slots),
    ] {
        if input
            .players
            .iter()
            .filter(|player| position_group(player.primary_position) == group)
            .count()
            < slots
        {
            return Err(format!("The Cut invite pool cannot fill {group:?} slots"));
        }
    }
    if input
        .players
        .iter()
        .filter(|player| {
            player.primary_position.is_forward()
                && player.eligible_positions.contains(&Position::Center)
        })
        .count()
        < input.config.minimum_centers
    {
        return Err("The Cut invite pool cannot satisfy minimum centers".to_owned());
    }
    Ok(())
}

fn gm_camp_adjustment(player: &TrainingCampPlayerInput, profile: &TeamDecisionProfile) -> f64 {
    let general_manager = &profile.general_manager;
    let opportunity = if player.rookie_eligible {
        general_manager.rookie_opportunity.effective_value() * 2.0
    } else if player.incumbent {
        general_manager.veteran_preference.effective_value() * 2.0
    } else {
        0.0
    };
    let asset_direction = if player.waiver_exempt { -1.0 } else { 1.0 };
    opportunity + general_manager.waiver_asset_protection.effective_value() * asset_direction
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_model::management_behavior::{
        BehaviorTraitView, GeneralManagerBehaviorProfile, ManagerBehaviorProfile,
        TEAM_DECISION_PROFILE_SCHEMA,
    };

    fn tendency(value: f64) -> BehaviorTraitView {
        BehaviorTraitView {
            value,
            evidence_games: 82,
            evidence_label: EvidenceLabel::Estimated,
        }
    }

    fn decision_profile(rookie: f64, veteran: f64) -> TeamDecisionProfile {
        TeamDecisionProfile {
            schema: TEAM_DECISION_PROFILE_SCHEMA.to_owned(),
            id: "nyr-test-management".to_owned(),
            team: "NYR".to_owned(),
            season: 20262027,
            general_manager: GeneralManagerBehaviorProfile {
                rookie_opportunity: tendency(rookie),
                veteran_preference: tendency(veteran),
                waiver_asset_protection: tendency(0.0),
                trade_aggression: tendency(0.0),
                deadline_buying_bias: tendency(0.0),
            },
            manager: ManagerBehaviorProfile {
                matchup_intensity: tendency(0.0),
                tactical_adaptability: tendency(0.0),
                lineup_patience: tendency(0.0),
                position_flexibility: tendency(0.0),
                physical_fourth_line_preference: tendency(0.0),
                four_line_usage: tendency(0.0),
                fatigue_rotation: tendency(0.0),
            },
            disclosures: Vec::new(),
        }
    }

    fn player(id: u32, position: Position, score: f64, incumbent: bool) -> TrainingCampPlayerInput {
        TrainingCampPlayerInput {
            player_id: id,
            display_name: format!("Player {id}"),
            primary_position: position,
            eligible_positions: vec![position],
            source_league: if incumbent { "NHL" } else { "AHL" }.to_owned(),
            incumbent,
            rookie_eligible: !incumbent,
            prospect: !incumbent,
            pre_camp_make_probability: None,
            minimum_forward_role: None,
            waiver_exempt: !incumbent,
            cap_hit: None,
            cap_hit_source: None,
            projected_score: score,
            translated_sample_games: 60,
            camp_std_dev: 4.0,
            readiness_delta: 0.0,
            management_delta: 0.0,
            availability_probability: 1.0,
            evidence_label: EvidenceLabel::Estimated,
            power_play_role_score: (position != Position::Goalie).then_some(score),
            penalty_kill_role_score: (position != Position::Goalie).then_some(score),
            requested_slot: None,
        }
    }

    fn input() -> TrainingCampSimulationInput {
        let mut players = Vec::new();
        for id in 1..=14 {
            let position = if id <= 5 {
                Position::Center
            } else {
                Position::LeftWing
            };
            players.push(player(id, position, 45.0 - f64::from(id), id <= 12));
        }
        for id in 20..=26 {
            players.push(player(
                id,
                Position::Defense,
                50.0 - f64::from(id - 20),
                id < 26,
            ));
        }
        for id in 30..=32 {
            players.push(player(
                id,
                Position::Goalie,
                50.0 - f64::from(id - 30),
                id < 32,
            ));
        }
        TrainingCampSimulationInput {
            team: "NYR".to_owned(),
            season: 20262027,
            config: TrainingCampConfig {
                trials: 1_000,
                seed: 7,
                ..TrainingCampConfig::default()
            },
            decision_profile: None,
            players,
        }
    }

    #[test]
    fn camp_is_seeded_and_reconciles_roster_shape() {
        let first = simulate_training_camp(&input()).unwrap();
        let second = simulate_training_camp(&input()).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.valid_trials, 1_000);
        assert_eq!(first.modal_opening_roster_ids.len(), 20);
        assert_eq!(first.salary_cap_status, TrainingCampSalaryCapStatus::NoRead);
        assert!(first
            .warnings
            .iter()
            .any(|warning| warning.contains("Salary-cap authority is unavailable")));
        assert!(first.players.iter().all(|player| {
            (player.make_probability + player.cut_probability - 1.0).abs() < 1e-12
                && (player.make_probability
                    - player.dressed_probability
                    - player.healthy_scratch_probability)
                    .abs()
                    < 1e-12
        }));
    }

    #[test]
    fn legacy_empty_eligibility_stays_absent_from_v1_wire_shape() {
        let mut forecast = simulate_training_camp(&input()).unwrap();
        forecast.players[0].eligible_positions.clear();
        let value = serde_json::to_value(&forecast).unwrap();
        assert!(value["players"][0].get("eligible_positions").is_none());
    }

    #[test]
    fn incomplete_trials_do_not_count_as_player_cuts() {
        let mut input = input();
        for goalie in input
            .players
            .iter_mut()
            .filter(|player| player.primary_position == Position::Goalie)
        {
            goalie.availability_probability = 0.75;
        }

        let forecast = simulate_training_camp(&input).unwrap();
        assert!(forecast.valid_trials > 0);
        assert!(forecast.incomplete_trials > 0);
        assert_eq!(
            forecast.valid_trials + forecast.incomplete_trials,
            forecast.trials
        );
        let make_sum = forecast
            .players
            .iter()
            .map(|player| player.make_probability)
            .sum::<f64>();
        assert!((make_sum - forecast.opening_roster_size as f64).abs() < 1e-12);
        for player in &forecast.players {
            assert!(
                (player.make_probability
                    + player.unavailable_probability
                    + player.selection_loss_probability
                    - 1.0)
                    .abs()
                    < 1e-12
            );
            if !player.waiver_exempt {
                assert_eq!(
                    player.waiver_exposure_probability,
                    player.selection_loss_probability
                );
            }
        }
    }

    #[test]
    fn bubble_ranks_selection_and_scratch_pressure_without_inventing_trades() {
        let league = simulate_training_camp_league(&TrainingCampLeagueSimulationInput {
            season: 20262027,
            teams: vec![TrainingCampLeagueTeamInput {
                simulation: input(),
                authority_status: TrainingCampAuthorityStatus::ConfirmedPool,
                competition_pool_status: TrainingCampCompetitionPoolStatus::Authored,
                current_roster_candidates: 24,
                sourced_overlay_candidates: 0,
                fallback_candidates: 0,
                authority_warnings: Vec::new(),
            }],
        })
        .unwrap();

        let bubble = build_training_camp_exposure_board(&league, 5).unwrap();
        assert_eq!(bubble.schema, TRAINING_CAMP_EXPOSURE_BOARD_SCHEMA);
        assert_eq!(bubble.teams.len(), 1);
        assert_eq!(bubble.teams[0].rows.len(), 5);
        assert!(bubble.teams[0]
            .rows
            .windows(2)
            .all(|rows| rows[0].exposure_score >= rows[1].exposure_score));
        assert!(bubble
            .disclosures
            .iter()
            .any(|line| line.contains("not a trade prediction")));
        assert!(build_training_camp_exposure_board(&league, 0).is_err());
    }

    #[test]
    fn bubble_requires_sourced_transaction_context_before_naming_a_path() {
        let league = simulate_training_camp_league(&TrainingCampLeagueSimulationInput {
            season: 20262027,
            teams: vec![TrainingCampLeagueTeamInput {
                simulation: input(),
                authority_status: TrainingCampAuthorityStatus::ConfirmedPool,
                competition_pool_status: TrainingCampCompetitionPoolStatus::Authored,
                current_roster_candidates: 24,
                sourced_overlay_candidates: 0,
                fallback_candidates: 0,
                authority_warnings: Vec::new(),
            }],
        })
        .unwrap();
        let no_read = build_training_camp_exposure_board(&league, 24).unwrap();
        let candidate = no_read.teams[0]
            .rows
            .iter()
            .find(|row| {
                row.incumbent && !row.waiver_exempt && row.selection_loss_probability >= 0.15
            })
            .expect("fixture must contain a pressured incumbent");
        assert_eq!(
            candidate.lane,
            TrainingCampExposureLane::RosterDecisionReview
        );
        assert_eq!(
            candidate.transaction_authority_status,
            TrainingCampTransactionAuthorityStatus::NoRead
        );

        let mut context = TrainingCampTransactionContextInput {
            schema: TRAINING_CAMP_TRANSACTION_CONTEXT_SCHEMA.to_owned(),
            season: league.season,
            checked_at: "2026-07-24T00:00:00Z".to_owned(),
            players: vec![TrainingCampTransactionPlayerInput {
                player_id: candidate.player_id,
                display_name: candidate.display_name.clone(),
                team: "NYR".to_owned(),
                cap_hit: Some(1_000_000),
                expiry_year: Some(2027),
                expiry_type: Some("UFA".to_owned()),
                trade_protection: TrainingCampTradeProtection::None,
                requires_waivers: Some(true),
                source_urls: vec!["https://example.test/contract".to_owned()],
            }],
        };
        let sourced =
            build_training_camp_exposure_board_with_context(&league, 24, Some(&context)).unwrap();
        let candidate = sourced.teams[0]
            .rows
            .iter()
            .find(|row| row.player_id == candidate.player_id)
            .unwrap();
        assert_eq!(candidate.lane, TrainingCampExposureLane::TransactionReview);
        assert_eq!(
            candidate.transaction_authority_status,
            TrainingCampTransactionAuthorityStatus::Sourced
        );
        assert!(candidate.transaction_warnings.is_empty());

        context.players[0].trade_protection = TrainingCampTradeProtection::NoMove;
        let protected =
            build_training_camp_exposure_board_with_context(&league, 24, Some(&context)).unwrap();
        let protected = protected.teams[0]
            .rows
            .iter()
            .find(|row| row.player_id == candidate.player_id)
            .unwrap();
        assert_eq!(protected.lane, TrainingCampExposureLane::ContractProtected);
        assert!(protected
            .transaction_warnings
            .iter()
            .any(|warning| warning.contains("no-move clause")));
    }

    #[test]
    fn transaction_context_is_team_scoped_when_stale_pools_share_a_player() {
        let nyr = input();
        let mut mtl = nyr.clone();
        mtl.team = "MTL".to_owned();
        let league = simulate_training_camp_league(&TrainingCampLeagueSimulationInput {
            season: 20262027,
            teams: [nyr, mtl]
                .into_iter()
                .map(|simulation| TrainingCampLeagueTeamInput {
                    simulation,
                    authority_status: TrainingCampAuthorityStatus::ConfirmedPool,
                    competition_pool_status: TrainingCampCompetitionPoolStatus::Authored,
                    current_roster_candidates: 24,
                    sourced_overlay_candidates: 0,
                    fallback_candidates: 0,
                    authority_warnings: Vec::new(),
                })
                .collect(),
        })
        .unwrap();
        let candidate = league.teams[0]
            .forecast
            .as_ref()
            .unwrap()
            .players
            .iter()
            .find(|player| {
                player.incumbent
                    && !player.waiver_exempt
                    && player.selection_loss_probability >= 0.15
            })
            .unwrap();
        let context = TrainingCampTransactionContextInput {
            schema: TRAINING_CAMP_TRANSACTION_CONTEXT_SCHEMA.to_owned(),
            season: league.season,
            checked_at: "2026-07-24T00:00:00Z".to_owned(),
            players: vec![TrainingCampTransactionPlayerInput {
                player_id: candidate.player_id,
                display_name: candidate.display_name.clone(),
                team: "NYR".to_owned(),
                cap_hit: Some(1_000_000),
                expiry_year: Some(2027),
                expiry_type: Some("UFA".to_owned()),
                trade_protection: TrainingCampTradeProtection::None,
                requires_waivers: Some(true),
                source_urls: vec!["https://example.test/contract".to_owned()],
            }],
        };
        let board =
            build_training_camp_exposure_board_with_context(&league, 24, Some(&context)).unwrap();
        let nyr = board.teams.iter().find(|team| team.team == "NYR").unwrap();
        let mtl = board.teams.iter().find(|team| team.team == "MTL").unwrap();
        assert_eq!(
            nyr.rows
                .iter()
                .find(|row| row.player_id == candidate.player_id)
                .unwrap()
                .transaction_authority_status,
            TrainingCampTransactionAuthorityStatus::Sourced
        );
        assert_eq!(
            mtl.rows
                .iter()
                .find(|row| row.player_id == candidate.player_id)
                .unwrap()
                .transaction_authority_status,
            TrainingCampTransactionAuthorityStatus::NoRead
        );
        assert!(mtl
            .authority_warnings
            .iter()
            .any(|warning| warning.contains("multiple league camp pools")));
    }

    #[test]
    fn league_camp_preserves_degraded_and_insufficient_authority_rows() {
        let mut degraded = input();
        degraded.team = "BOS".to_owned();
        let mut insufficient = input();
        insufficient.team = "DET".to_owned();
        let view = simulate_training_camp_league(&TrainingCampLeagueSimulationInput {
            season: 20262027,
            teams: vec![
                TrainingCampLeagueTeamInput {
                    simulation: degraded,
                    authority_status: TrainingCampAuthorityStatus::DegradedFallback,
                    competition_pool_status:
                        TrainingCampCompetitionPoolStatus::PriorSeasonAugmented,
                    current_roster_candidates: 18,
                    sourced_overlay_candidates: 0,
                    fallback_candidates: 2,
                    authority_warnings: vec!["test fallback".to_owned()],
                },
                TrainingCampLeagueTeamInput {
                    simulation: insufficient,
                    authority_status: TrainingCampAuthorityStatus::InsufficientAuthority,
                    competition_pool_status: TrainingCampCompetitionPoolStatus::Thin,
                    current_roster_candidates: 19,
                    sourced_overlay_candidates: 0,
                    fallback_candidates: 0,
                    authority_warnings: vec!["test incomplete".to_owned()],
                },
            ],
        })
        .unwrap();

        assert_eq!(view.teams_requested, 2);
        assert_eq!(view.teams_simulated, 1);
        assert_eq!(view.teams_degraded, 1);
        assert_eq!(view.teams_augmented, 1);
        assert_eq!(view.teams_failed, 1);
        assert!(view.teams[0].forecast.is_some());
        assert!(view.teams[1].forecast.is_none());
        assert!(view.teams[1].error.is_some());
    }

    #[test]
    fn opening_roster_is_selected_before_dressed_lineup_and_cap_is_enforced() {
        let mut input = input();
        input.config.forward_slots = 14;
        input.config.defense_slots = 7;
        input.config.goalie_slots = 2;
        input.config.salary_cap_upper_limit = Some(23_000_000);
        for player in &mut input.players {
            player.cap_hit = Some(1_000_000);
            player.cap_hit_source = Some("test-contract-source".to_owned());
        }

        let forecast = simulate_training_camp(&input).unwrap();
        assert_eq!(forecast.opening_roster_size, 23);
        assert_eq!(forecast.dressed_roster_size, 20);
        assert_eq!(
            forecast.salary_cap_status,
            TrainingCampSalaryCapStatus::Enforced
        );
        assert_eq!(forecast.modal_opening_roster_ids.len(), 23);
        assert!(forecast.players.iter().all(|player| {
            (player.make_probability
                - player.dressed_probability
                - player.healthy_scratch_probability)
                .abs()
                < 1e-12
        }));
        let branch = forecast.most_common_rosters.first().unwrap();
        assert_eq!(branch.total_cap_hit, Some(23_000_000));
        assert_eq!(branch.cap_space, Some(0));
        assert_eq!(branch.cap_compliant, Some(true));

        let lineups = build_training_camp_lineup_set(&input, &forecast, 1).unwrap();
        let branch = &lineups.branches[0];
        assert_eq!(branch.roster_ids.len(), 23);
        assert_eq!(branch.dressed_ids.len(), 20);
        assert_eq!(branch.healthy_scratch_ids.len(), 3);
        assert_eq!(branch.lineup.extras.len(), 3);
        assert!(branch
            .healthy_scratch_ids
            .iter()
            .all(|id| branch.roster_ids.contains(id) && !branch.dressed_ids.contains(id)));

        input.config.salary_cap_upper_limit = Some(22_000_000);
        let over_cap = simulate_training_camp(&input).unwrap();
        assert_eq!(over_cap.valid_trials, 0);
        assert_eq!(over_cap.incomplete_trials, input.config.trials);
    }

    #[test]
    fn cap_enforcement_rejects_unsourced_contract_values() {
        let mut input = input();
        input.config.salary_cap_upper_limit = Some(100_000_000);
        let error = simulate_training_camp(&input).unwrap_err();
        assert!(error.contains("lacks a sourced cap hit"));
    }

    #[test]
    fn high_variance_prospect_can_displace_an_incumbent() {
        let mut input = input();
        let prospect = input
            .players
            .iter_mut()
            .find(|player| player.player_id == 13)
            .unwrap();
        prospect.projected_score = 43.0;
        prospect.camp_std_dev = 9.0;
        let view = simulate_training_camp(&input).unwrap();
        let prospect = view
            .players
            .iter()
            .find(|player| player.player_id == 13)
            .unwrap();
        assert!(prospect.make_probability > 0.1);
        assert!(!prospect.displaced_incumbents.is_empty());
        assert!(
            prospect
                .displaced_incumbents
                .iter()
                .map(|row| row.probability)
                .sum::<f64>()
                <= prospect.make_probability + 1e-12
        );
    }

    #[test]
    fn scoring_prospect_must_earn_disclosed_development_role() {
        let mut input = input();
        input.config.trials = 100;
        for player in &mut input.players {
            player.camp_std_dev = 0.0;
            player.availability_probability = 1.0;
        }
        let prospect = input
            .players
            .iter_mut()
            .find(|player| player.player_id == 13)
            .unwrap();
        prospect.minimum_forward_role = Some(TrainingCampForwardRole::TopNine);

        let outside_top_nine = simulate_training_camp(&input).unwrap();
        assert_eq!(
            outside_top_nine
                .players
                .iter()
                .find(|player| player.player_id == 13)
                .unwrap()
                .make_probability,
            0.0
        );

        input
            .players
            .iter_mut()
            .find(|player| player.player_id == 13)
            .unwrap()
            .projected_score = 60.0;
        let earns_top_nine = simulate_training_camp(&input).unwrap();
        let prospect = earns_top_nine
            .players
            .iter()
            .find(|player| player.player_id == 13)
            .unwrap();
        assert_eq!(prospect.make_probability, 1.0);
        assert_eq!(
            prospect.minimum_forward_role,
            Some(TrainingCampForwardRole::TopNine)
        );
    }

    #[test]
    fn rangers_fixture_keeps_greentree_and_beaudoin_as_distinct_battles() {
        let input: TrainingCampSimulationInput = serde_json::from_str(include_str!(
            "../../../examples/icecast-nyr-training-camp.json"
        ))
        .unwrap();
        let greentree = input
            .players
            .iter()
            .find(|player| player.player_id == 8_484_802)
            .unwrap();
        let beaudoin = input
            .players
            .iter()
            .find(|player| player.player_id == 8_484_786)
            .unwrap();

        assert_eq!(greentree.display_name, "Liam Greentree");
        assert_eq!(greentree.primary_position, Position::RightWing);
        assert!(greentree.prospect && greentree.rookie_eligible);
        assert_eq!(greentree.pre_camp_make_probability, Some(0.35));
        assert_eq!(
            greentree.minimum_forward_role,
            Some(TrainingCampForwardRole::TopNine)
        );
        assert_eq!(beaudoin.display_name, "Cole Beaudoin");
        assert_eq!(beaudoin.primary_position, Position::Center);
        assert!(beaudoin.prospect && beaudoin.rookie_eligible);
        assert_eq!(beaudoin.pre_camp_make_probability, Some(0.15));
        assert_eq!(beaudoin.minimum_forward_role, None);
    }

    #[test]
    fn kraken_fixture_preserves_reported_lines_and_separates_roster_from_dress() {
        let input: TrainingCampSimulationInput = serde_json::from_str(include_str!(
            "../../../examples/icecast-sea-training-camp.json"
        ))
        .unwrap();
        let forecast = simulate_training_camp(&input).unwrap();

        assert!(forecast.valid_trials > 9_500);
        assert_eq!(forecast.opening_roster_size, 23);
        assert_eq!(forecast.dressed_roster_size, 20);
        let meyers = forecast
            .players
            .iter()
            .find(|player| player.display_name == "Ben Meyers")
            .unwrap();
        assert!(meyers.make_probability > meyers.dressed_probability);
        assert!(meyers.healthy_scratch_probability > 0.0);

        let lineups = build_training_camp_lineup_set(&input, &forecast, 1).unwrap();
        let lineup = &lineups.branches[0].lineup;
        let names = |line: usize| {
            let row = &lineup.forward_lines[line];
            (
                row.left_wing.as_ref().unwrap().display_name.as_str(),
                row.center.as_ref().unwrap().display_name.as_str(),
                row.right_wing.as_ref().unwrap().display_name.as_str(),
            )
        };
        assert_eq!(names(0), ("Bobby McMann", "Matty Beniers", "Jordan Eberle"));
        assert_eq!(names(1), ("Jared McCann", "Shane Wright", "Berkly Catton"));
        assert_eq!(
            lineup.goalies.starter.as_ref().unwrap().display_name,
            "Joey Daccord"
        );
        assert_eq!(lineup.extras.len(), 3);
    }

    #[test]
    fn gm_rookie_opportunity_changes_camp_odds_without_hidden_player_edits() {
        let baseline = simulate_training_camp(&input()).unwrap();
        let mut opportunity_input = input();
        opportunity_input.decision_profile = Some(decision_profile(1.0, -1.0));
        let opportunity = simulate_training_camp(&opportunity_input).unwrap();
        let probability = |view: &TrainingCampForecastView, id| {
            view.players
                .iter()
                .find(|player| player.player_id == id)
                .unwrap()
                .make_probability
        };

        assert!(probability(&opportunity, 13) > probability(&baseline, 13));
        assert_eq!(
            opportunity.decision_profile_id.as_deref(),
            Some("nyr-test-management")
        );
        assert!(
            opportunity
                .players
                .iter()
                .find(|player| player.player_id == 13)
                .unwrap()
                .management_behavior_delta
                > 0.0
        );
    }

    #[test]
    fn pre_camp_track_is_distinct_from_rookie_and_simulated_status() {
        let baseline = simulate_training_camp(&input()).unwrap();
        let mut prior_input = input();
        prior_input
            .players
            .iter_mut()
            .find(|player| player.player_id == 13)
            .unwrap()
            .pre_camp_make_probability = Some(0.18);
        let with_prior = simulate_training_camp(&prior_input).unwrap();
        let row = with_prior
            .players
            .iter()
            .find(|player| player.player_id == 13)
            .unwrap();
        let baseline_probability = baseline
            .players
            .iter()
            .find(|player| player.player_id == 13)
            .unwrap()
            .make_probability;

        assert!(row.prospect);
        assert!(row.rookie_eligible);
        assert_eq!(
            row.pre_camp_track,
            TrainingCampPreCampTrack::OutsideLookingIn
        );
        assert!(row.roster_prior_delta < 0.0);
        assert!(row.make_probability < baseline_probability);
    }

    #[test]
    fn camp_branches_bridge_to_complete_ui_neutral_lineups() {
        let mut input = input();
        // Exercise the real camp edge case: a player selected to satisfy the
        // center minimum may have a wing primary position and C eligibility.
        input.players[3].primary_position = Position::LeftWing;
        input.players[3].eligible_positions = vec![Position::LeftWing, Position::Center];
        input.players[4].primary_position = Position::LeftWing;
        input.players[4].eligible_positions = vec![Position::LeftWing];
        let forecast = simulate_training_camp(&input).unwrap();
        let set = build_training_camp_lineup_set(&input, &forecast, 3).unwrap();

        assert_eq!(set.schema, TRAINING_CAMP_LINEUP_SET_SCHEMA);
        assert_eq!(set.branches.len(), 3);
        assert!(set.retained_probability > 0.0 && set.retained_probability <= 1.0);
        for branch in &set.branches {
            assert_eq!(branch.roster_ids.len(), 20);
            assert_eq!(branch.lineup.team, "NYR");
            assert!(branch.lineup.forward_lines.iter().all(|line| {
                line.left_wing.is_some() && line.center.is_some() && line.right_wing.is_some()
            }));
            assert!(branch
                .lineup
                .warnings
                .iter()
                .all(|warning| warning.code != "incomplete_roster_shape"));
            assert!(branch.lineup.special_teams.warnings.is_empty());
            assert_eq!(branch.lineup.special_teams.power_play.len(), 2);
            assert_eq!(branch.lineup.special_teams.penalty_kill.len(), 2);
        }
        let retained = set
            .branches
            .iter()
            .map(|branch| branch.probability)
            .sum::<f64>();
        assert!((set.retained_probability - retained).abs() < 1e-12);
        let blender = build_training_camp_blender_set(
            &set,
            LineCombinationForecastConfig {
                max_candidates: 8,
                allow_off_wing: true,
            },
        )
        .unwrap();
        assert_eq!(blender.schema, TRAINING_CAMP_BLENDER_SET_SCHEMA);
        assert_eq!(blender.branches.len(), set.branches.len());
        assert!((blender.retained_probability + blender.residual_probability - 1.0).abs() < 1e-12);
        assert!(
            (blender
                .opening_roster_policy
                .choices
                .iter()
                .map(|choice| choice.probability)
                .sum::<f64>()
                - 1.0)
                .abs()
                < 1e-12
        );
        let compact = build_training_camp_opening_roster_policy(
            &set,
            LineCombinationForecastConfig {
                max_candidates: 8,
                allow_off_wing: true,
            },
        )
        .unwrap();
        assert_eq!(compact, blender.opening_roster_policy);
    }

    #[test]
    fn camp_goalie_completion_preserves_lineup_and_requires_independent_value() {
        let input = input();
        let forecast = simulate_training_camp(&input).unwrap();
        let set = build_training_camp_lineup_set(&input, &forecast, 1).unwrap();
        let mut baseline = set.branches[0].lineup.clone();
        let removed = baseline.goalies.backup.take().unwrap();
        baseline
            .warnings
            .push(super::super::team_lineup::TeamLineupWarningView {
                code: "incomplete_roster_shape".to_owned(),
                message: "missing backup".to_owned(),
            });
        let forward_ids = baseline
            .forward_lines
            .iter()
            .flat_map(|line| [&line.left_wing, &line.center, &line.right_wing])
            .filter_map(Option::as_ref)
            .map(|player| player.player_id)
            .collect::<Vec<_>>();

        assert!(complete_lineup_goalies_from_training_camp(&baseline, &forecast, &[]).is_err());
        let completed = complete_lineup_goalies_from_training_camp(
            &baseline,
            &forecast,
            &[TrainingCampGoalieValueInput {
                player_id: removed.player_id,
                goalie_quality_score: 42.0,
                sample_games: 12,
                evidence_label: EvidenceLabel::Estimated,
                source_method: "career_paired_ahl_to_nhl_goalie.v1".to_owned(),
            }],
        )
        .unwrap();
        let backup = completed.goalies.backup.as_ref().unwrap();
        assert_eq!(backup.player_id, removed.player_id);
        assert_eq!(backup.score.value, Some(42.0));
        assert_eq!(backup.score.sample_games, 12);
        assert_eq!(backup.score.evidence_label, EvidenceLabel::Estimated);
        assert_eq!(
            backup.assignment_evidence,
            LineupAssignmentEvidence::Scenario
        );
        assert_eq!(
            completed
                .forward_lines
                .iter()
                .flat_map(|line| [&line.left_wing, &line.center, &line.right_wing])
                .filter_map(Option::as_ref)
                .map(|player| player.player_id)
                .collect::<Vec<_>>(),
            forward_ids
        );
        assert!(!completed
            .warnings
            .iter()
            .any(|warning| warning.code == "incomplete_roster_shape"));
        assert!(completed
            .warnings
            .iter()
            .any(|warning| warning.code == "training_camp_goalie_assignment"));
    }
}
