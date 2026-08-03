use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

use crate::model::Position;

use super::team_lineup::{
    rebuild_team_lineup_projection, LineupForwardPosition, TeamLineupPlayerInput,
    TeamLineupPlayerView, TeamLineupProjectionView, TeamLineupRequestedSlot,
    TeamLineupRosterChangeInput,
};
use super::team_season_forecast::{
    compare_team_season_forecast_scenarios, TeamSeasonForecastView, TeamSeasonScenarioImpactRow,
};
use super::training_camp::{
    TrainingCampLeagueForecastView, TrainingCampPlayerView, TRAINING_CAMP_FORECAST_SCHEMA,
    TRAINING_CAMP_LEAGUE_FORECAST_SCHEMA,
};

pub const DRAFT_PICK_VALUE_CURVE_SCHEMA: &str = "draft_pick_value_curve.v1";
pub const DRAFT_PICK_VALUE_METHOD: &str =
    "mature-cohort fixed-horizon outcomes with weighted monotone regression";
pub const TRADE_MARKET_EVALUATION_SCHEMA: &str = "trade_market_evaluation.v1";
pub const TRADE_LINEUP_BOARD_SCHEMA: &str = "trade_lineup_board.v1";
pub const TRADE_SCOUT_SCHEMA: &str = "trade_scout.v1";
pub const TRADE_SCOUT_LEAGUE_SCHEMA: &str = "trade_scout_league.v1";
pub const TRADE_SCOUT_POPULATION_SCHEMA: &str = "trade_scout_population.v1";
pub const TRADE_SCOUT_DRAFT_PICK_POPULATION_SCHEMA: &str = "trade_scout_draft_pick_population.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DraftPickOutcomeObservation {
    pub draft_year: u16,
    pub overall_pick: u16,
    pub outcome_value: f64,
    pub observed_horizon_years: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DraftPickValueConfig {
    pub training_cutoff_year: u16,
    pub outcome_horizon_years: u8,
    pub max_overall_pick: u16,
    pub outcome_measure: String,
    pub annual_future_discount: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DraftPickValueRow {
    pub overall_pick: u16,
    pub expected_value: f64,
    pub expected_value_low: f64,
    pub expected_value_high: f64,
    pub observations: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DraftPickValueCurve {
    pub schema: String,
    pub method: String,
    pub training_cutoff_year: u16,
    pub outcome_horizon_years: u8,
    pub max_overall_pick: u16,
    pub outcome_measure: String,
    pub annual_future_discount: f64,
    pub observations: usize,
    pub values: Vec<DraftPickValueRow>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DraftPickSlotOutcome {
    pub overall_pick: u16,
    pub probability: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DraftPickAssetInput {
    pub id: String,
    pub draft_year: u16,
    /// Lottery, standings, protection, and deferral uncertainty is expanded
    /// into mutually exclusive overall-pick outcomes by the source adapter.
    pub slot_outcomes: Vec<DraftPickSlotOutcome>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DraftPickAssetValue {
    pub id: String,
    pub draft_year: u16,
    pub expected_overall_pick: f64,
    pub expected_value: f64,
    pub expected_value_low: f64,
    pub expected_value_high: f64,
    pub future_discount: f64,
    pub uncertainty_width: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeAvailabilityKind {
    ReportedRequest,
    ClubShopping,
    ContractPressure,
    DepthSurplus,
    SpeculativeFit,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeAvailabilityEvidence {
    pub kind: TradeAvailabilityKind,
    pub probability: f64,
    pub source_url: Option<String>,
    pub observed_at: Option<String>,
    /// `Some(false)` represents an evidenced clause/destination gate. `None`
    /// is unknown and must not be described as approval.
    pub destination_allowed: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradeValueBasis {
    /// Outcome currency used by every scalar value in the package.
    pub outcome_measure: String,
    /// Post-acquisition horizon over which that currency is accumulated.
    pub horizon_years: u8,
    /// Reproducible model or artifact identifier, not a display-only label.
    pub method: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeAssetValueInput {
    pub id: String,
    pub label: String,
    pub position: Option<String>,
    /// Every asset in both directions must declare the same basis. Package
    /// evaluation rejects mixed units instead of silently comparing them.
    pub value_basis: TradeValueBasis,
    pub market_value: f64,
    pub current_value: f64,
    pub future_value: f64,
    /// Expected standings-points contribution over the next full season.
    /// This remains a separate axis from the control-value basis above.
    pub season_points_impact: f64,
    pub uncertainty: f64,
    pub availability: TradeAvailabilityEvidence,
    pub draft_pick: Option<DraftPickAssetValue>,
    #[serde(default)]
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeTeamPreferenceInput {
    pub team: String,
    pub current_weight: f64,
    pub future_weight: f64,
    /// Utility assigned to one million dollars of additional cap space.
    pub cap_weight: f64,
    /// Club-specific preference for immediate standings impact.
    pub season_impact_weight: f64,
    /// Position/role need on a 0-100 scale. Missing means no fit bonus.
    pub needs: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradePackageInput {
    pub buyer: TradeTeamPreferenceInput,
    pub seller: TradeTeamPreferenceInput,
    pub assets_to_buyer: Vec<TradeAssetValueInput>,
    pub assets_to_seller: Vec<TradeAssetValueInput>,
    /// Post-trade cap-space change in millions; positive means space created.
    #[serde(default)]
    pub buyer_cap_space_delta: Option<f64>,
    #[serde(default)]
    pub seller_cap_space_delta: Option<f64>,
    #[serde(default)]
    pub transaction_gates: TradeTransactionGates,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TradeTransactionGates {
    /// Confirmed against the authoritative cap snapshot for both clubs.
    pub cap_compliant: Option<bool>,
    /// Confirmed against active-roster limits and required assignments.
    pub roster_compliant: Option<bool>,
    /// Confirms either no retention or valid retained salary and club slots.
    #[serde(default)]
    pub retention_compliant: Option<bool>,
    /// All player contracts and trade clauses in the package were sourced.
    pub contract_authority_complete: bool,
    /// Required only when the package contains a draft pick.
    pub pick_ownership_confirmed: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DraftPickPackageRole {
    Rounding,
    Principal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradePackagePickRoleView {
    pub asset_id: String,
    pub direction: String,
    pub package_share: f64,
    pub role: DraftPickPackageRole,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradePackageEvaluationView {
    pub buyer: String,
    pub seller: String,
    pub assets_to_buyer: Vec<String>,
    pub assets_to_seller: Vec<String>,
    pub buyer_utility_delta: f64,
    pub seller_utility_delta: f64,
    pub market_value_to_buyer: f64,
    pub market_value_to_seller: f64,
    pub market_value_gap: f64,
    pub buyer_season_points_delta: f64,
    pub seller_season_points_delta: f64,
    pub buyer_cap_space_delta: Option<f64>,
    pub seller_cap_space_delta: Option<f64>,
    pub fairness_score: f64,
    pub feasibility_probability: f64,
    pub transaction_ready: bool,
    pub transaction_gates: TradeTransactionGates,
    pub mutually_beneficial: bool,
    pub pick_roles: Vec<TradePackagePickRoleView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub season_forecast_impact: Option<TradePackageSeasonForecastImpactView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lineup_impact: Option<TradePackageLineupImpactView>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeLineupPosition {
    Center,
    LeftWing,
    RightWing,
    Defense,
    Goalie,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeLineupPlayerInput {
    pub player_id: u32,
    pub label: String,
    pub natural_positions: Vec<TradeLineupPosition>,
    #[serde(default)]
    pub alternate_positions: Vec<TradeLineupPosition>,
    /// Comparable confidence-weighted IceLines score from 0 through 100.
    pub projected_score: f64,
    /// Score cost when assigned only through `alternate_positions`.
    #[serde(default)]
    pub alternate_position_penalty: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradeLineupLimits {
    pub centers: u8,
    pub left_wings: u8,
    pub right_wings: u8,
    pub defense: u8,
    pub goalies: u8,
}

impl Default for TradeLineupLimits {
    fn default() -> Self {
        Self {
            centers: 4,
            left_wings: 4,
            right_wings: 4,
            defense: 6,
            goalies: 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeLineupScenarioInput {
    pub team: String,
    pub baseline_roster: Vec<TradeLineupPlayerInput>,
    pub incoming_players: Vec<TradeLineupPlayerInput>,
    #[serde(default)]
    pub outgoing_player_ids: Vec<u32>,
    #[serde(default)]
    pub limits: TradeLineupLimits,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeLineupProjectionChangeInput {
    pub incoming_players: Vec<TradeLineupPlayerInput>,
    /// Full player evidence used to regenerate lines and special teams. When
    /// supplied, IDs must match `incoming_players` exactly.
    #[serde(default)]
    pub projection_incoming_players: Vec<TeamLineupPlayerInput>,
    #[serde(default)]
    pub outgoing_player_ids: Vec<u32>,
    #[serde(default)]
    pub limits: TradeLineupLimits,
    #[serde(default)]
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeLineupAssignmentKind {
    Natural,
    Alternate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeLineupAssignmentView {
    pub player_id: u32,
    pub label: String,
    pub position: TradeLineupPosition,
    pub assignment_kind: TradeLineupAssignmentKind,
    pub effective_score: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeLineupScenarioView {
    pub team: String,
    pub before: Vec<TradeLineupAssignmentView>,
    pub after: Vec<TradeLineupAssignmentView>,
    pub added_to_lineup: Vec<String>,
    pub explicitly_removed_from_lineup: Vec<String>,
    pub displaced_by_competition: Vec<String>,
    pub before_strength: f64,
    pub after_strength: f64,
    pub strength_delta: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projected_lineup: Option<TeamLineupProjectionView>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeLineupBoardCandidateInput {
    pub candidate_id: String,
    pub label: String,
    pub change: TradeLineupProjectionChangeInput,
    /// Candidate availability before cap, clause, roster, and ownership gates.
    pub availability_probability: f64,
    pub feasibility_probability: f64,
    pub transaction_ready: bool,
    /// Required authority when `transaction_ready` is true. The board verifies
    /// buyer, probability, and gate result rather than trusting the flag alone.
    #[serde(default)]
    pub package_evaluation: Option<TradePackageEvaluationView>,
    #[serde(default)]
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeLineupBoardInput {
    pub candidates: Vec<TradeLineupBoardCandidateInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeLineupBoardRowView {
    pub candidate_id: String,
    pub label: String,
    pub hockey_rank: u16,
    pub actionable_rank: Option<u16>,
    pub availability_probability: f64,
    pub feasibility_probability: f64,
    pub transaction_ready: bool,
    /// Positive lineup strength multiplied by feasibility. This ranks only
    /// candidates that have passed the transaction gate.
    pub actionable_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_evaluation: Option<TradePackageEvaluationView>,
    pub scenario: TradeLineupScenarioView,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeLineupBoardView {
    pub schema: String,
    pub team: String,
    pub rows: Vec<TradeLineupBoardRowView>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradePackageLineupImpactView {
    pub buyer: TradeLineupScenarioView,
    pub seller: TradeLineupScenarioView,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeTeamSeasonForecastDeltaView {
    pub team: String,
    pub average_points_delta: f64,
    pub playoff_probability_delta: f64,
    pub stanley_cup_probability_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradePackageSeasonForecastImpactView {
    pub buyer: TradeTeamSeasonForecastDeltaView,
    pub seller: TradeTeamSeasonForecastDeltaView,
    /// Paired simulation minus the package's prior isolated buyer estimate.
    pub buyer_points_residual_vs_isolated: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeMarketInput {
    pub as_of: String,
    pub proposals: Vec<TradePackageInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeScoutAssetInput {
    pub organization: String,
    pub asset: TradeAssetValueInput,
    /// Protected assets remain visible in the audit but are never placed into
    /// a generated offer.
    #[serde(default)]
    pub protected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TradeScoutConfig {
    /// Smallest opening package as a share of the target's market value.
    pub opening_offer_ratio: f64,
    /// Hard buyer walk-away boundary as a share of target market value.
    pub maximum_price_ratio: f64,
    /// Maximum number of buyer assets in one generated package.
    pub maximum_assets_per_package: u8,
    /// Limit the published candidate board after deterministic ranking.
    pub maximum_candidates: usize,
}

impl Default for TradeScoutConfig {
    fn default() -> Self {
        Self {
            opening_offer_ratio: 0.82,
            maximum_price_ratio: 1.15,
            maximum_assets_per_package: 2,
            maximum_candidates: 12,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeScoutInput {
    pub as_of: String,
    pub buyer: TradeTeamPreferenceInput,
    pub sellers: Vec<TradeTeamPreferenceInput>,
    /// Player or other current assets that could satisfy the buyer's needs.
    pub targets: Vec<TradeScoutAssetInput>,
    /// Picks, prospects, or roster assets controlled by the buyer.
    pub buyer_assets: Vec<TradeScoutAssetInput>,
    #[serde(default)]
    pub config: TradeScoutConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeNegotiationTier {
    OpeningOffer,
    FairMidpoint,
    MaximumAcceptable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeNegotiationPackageView {
    pub tier: TradeNegotiationTier,
    pub assets_to_seller: Vec<String>,
    pub market_value: f64,
    pub target_value_ratio: f64,
    pub evaluation: TradePackageEvaluationView,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeNegotiationLadderView {
    pub target_market_value: f64,
    pub opening_offer: TradeNegotiationPackageView,
    pub fair_midpoint: TradeNegotiationPackageView,
    pub maximum_acceptable: TradeNegotiationPackageView,
    pub walk_away_market_value: f64,
    pub protected_buyer_assets: Vec<String>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeScoutCandidateView {
    pub rank: u16,
    pub target_id: String,
    pub label: String,
    pub seller: String,
    pub role: Option<String>,
    /// Buyer-specific current/future/need utility before market probability.
    pub hockey_fit_score: f64,
    /// Hockey fit multiplied by sourced or explicitly speculative availability.
    pub discovery_score: f64,
    pub availability: TradeAvailabilityEvidence,
    pub negotiation: TradeNegotiationLadderView,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeScoutView {
    pub schema: String,
    pub as_of: String,
    pub buyer: String,
    pub candidates: Vec<TradeScoutCandidateView>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeScoutLeagueAssetKind {
    NhlPlayer,
    Prospect,
    DraftPick,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeScoutLeagueAssetInput {
    pub kind: TradeScoutLeagueAssetKind,
    pub asset: TradeAssetValueInput,
    /// Organization-relative evidence that this asset is movable, from 0-100.
    /// Stronger sourced availability can independently make an NHL player a
    /// target, but never overrides an explicit destination veto.
    pub surplus_score: f64,
    #[serde(default)]
    pub protected: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeScoutLeagueOrganizationInput {
    pub preference: TradeTeamPreferenceInput,
    pub assets: Vec<TradeScoutLeagueAssetInput>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TradeScoutLeagueConfig {
    #[serde(default)]
    pub scout: TradeScoutConfig,
    pub minimum_surplus_score: f64,
    pub expected_organizations: usize,
    #[serde(default)]
    pub allow_partial_inventory: bool,
}

impl Default for TradeScoutLeagueConfig {
    fn default() -> Self {
        Self {
            scout: TradeScoutConfig::default(),
            minimum_surplus_score: 60.0,
            expected_organizations: 32,
            allow_partial_inventory: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeScoutLeagueInput {
    pub as_of: String,
    pub buyer: String,
    pub organizations: Vec<TradeScoutLeagueOrganizationInput>,
    #[serde(default)]
    pub config: TradeScoutLeagueConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeScoutLeagueView {
    pub schema: String,
    pub as_of: String,
    pub buyer: String,
    pub organizations_supplied: usize,
    pub expected_organizations: usize,
    pub inventory_complete: bool,
    pub derived_targets: usize,
    pub derived_buyer_assets: usize,
    pub scout: TradeScoutView,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeScoutAvailabilityOverlayInput {
    pub player_id: u32,
    pub evidence: TradeAvailabilityEvidence,
    #[serde(default)]
    pub surplus_score: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeScoutPopulationConfig {
    pub value_basis: TradeValueBasis,
    pub control_value_per_score: f64,
    pub season_points_per_score: f64,
    pub top_six_forward_score: f64,
    pub top_four_defense_score: f64,
    pub starting_goalie_score: f64,
    #[serde(default)]
    pub league: TradeScoutLeagueConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeScoutPopulationInput {
    pub as_of: String,
    pub buyer: String,
    pub preferences: Vec<TradeTeamPreferenceInput>,
    #[serde(default)]
    pub availability: Vec<TradeScoutAvailabilityOverlayInput>,
    #[serde(default)]
    pub protected_player_ids: Vec<u32>,
    /// Already-valued, ownership-scoped picks from the draft-pick source.
    #[serde(default)]
    pub draft_pick_assets: Vec<TradeScoutAssetInput>,
    pub config: TradeScoutPopulationConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeScoutPopulationView {
    pub schema: String,
    pub as_of: String,
    pub season: u32,
    pub teams_requested: usize,
    pub teams_populated: usize,
    pub players_populated: usize,
    pub prospects_populated: usize,
    pub picks_populated: usize,
    pub teams_without_forecast: Vec<String>,
    pub league_input: TradeScoutLeagueInput,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeDraftPickOwnershipStatus {
    ConfirmedUnconditional,
    Conditional,
    Encumbered,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradeDraftPickOwnershipInput {
    pub asset_id: String,
    pub owner: String,
    pub original_team: String,
    pub draft_year: u16,
    pub round: u8,
    pub status: TradeDraftPickOwnershipStatus,
    #[serde(default)]
    pub conditions: Option<String>,
    pub source_url: String,
    pub observed_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeScoutDraftPickPopulationInput {
    pub as_of: String,
    pub current_draft_year: u16,
    pub value_basis: TradeValueBasis,
    pub ownership: Vec<TradeDraftPickOwnershipInput>,
    #[serde(default)]
    pub protected_asset_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeScoutDraftPickPopulationView {
    pub schema: String,
    pub as_of: String,
    pub picks_supplied: usize,
    pub picks_populated: usize,
    pub unresolved_asset_ids: Vec<String>,
    pub assets: Vec<TradeScoutAssetInput>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeMarketAssemblyInput {
    pub as_of: String,
    pub current_draft_year: u16,
    pub authority: TradeExecutionAuthorityInput,
    pub proposals: Vec<TradePackageAssemblyInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradePackageAssemblyInput {
    pub buyer: TradeTeamPreferenceInput,
    pub seller: TradeTeamPreferenceInput,
    pub assets_to_buyer: Vec<TradeAssetAssemblyInput>,
    pub assets_to_seller: Vec<TradeAssetAssemblyInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TradeAssetAssemblyInput {
    Player {
        player_id: u32,
        asset: Box<TradeAssetValueInput>,
        /// Cap hit retained by the sending club for this contract.
        #[serde(default)]
        retained_cap_hit: Option<u64>,
    },
    DraftPick {
        asset: DraftPickAssetInput,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeExecutionAuthorityInput {
    pub teams: Vec<TradeTeamExecutionAuthority>,
    pub players: Vec<TradePlayerExecutionAuthority>,
    pub draft_picks: Vec<TradeDraftPickExecutionAuthority>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradeTeamExecutionAuthority {
    pub team: String,
    pub upper_limit: Option<u64>,
    pub committed_cap_hit: Option<u64>,
    pub active_roster_players: Option<u8>,
    #[serde(default = "default_max_active_roster_players")]
    pub max_active_roster_players: u8,
    /// Number of additional retained-salary transactions the club may carry.
    #[serde(default)]
    pub retained_salary_slots_available: Option<u8>,
    pub source_url: Option<String>,
    pub observed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradePlayerExecutionAuthority {
    pub player_id: u32,
    pub organization: String,
    pub cap_hit: Option<u64>,
    pub contract_confirmed: bool,
    pub clause_reviewed: bool,
    pub destination_allowed: Option<bool>,
    pub source_url: Option<String>,
    pub observed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradeDraftPickExecutionAuthority {
    pub asset_id: String,
    pub owner: String,
    pub confirmed: bool,
    pub source_url: Option<String>,
    pub observed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeMarketEvaluationView {
    pub schema: String,
    pub as_of: String,
    pub value_basis: TradeValueBasis,
    pub proposals: Vec<TradePackageEvaluationView>,
    pub disclosures: Vec<String>,
}

pub fn assemble_trade_market(
    input: TradeMarketAssemblyInput,
    curve: &DraftPickValueCurve,
) -> Result<TradeMarketEvaluationView, DraftPickValueError> {
    if input.proposals.is_empty() {
        return Err(DraftPickValueError::InvalidDistribution(
            "trade market assembly requires at least one proposal".to_owned(),
        ));
    }
    validate_execution_authority(&input.authority)?;
    let proposals = input
        .proposals
        .into_iter()
        .map(|proposal| {
            assemble_trade_package(proposal, &input.authority, curve, input.current_draft_year)
        })
        .collect::<Result<Vec<_>, _>>()?;
    evaluate_trade_market(TradeMarketInput {
        as_of: input.as_of,
        proposals,
    })
}

fn assemble_trade_package(
    proposal: TradePackageAssemblyInput,
    authority: &TradeExecutionAuthorityInput,
    curve: &DraftPickValueCurve,
    current_draft_year: u16,
) -> Result<TradePackageInput, DraftPickValueError> {
    if proposal.buyer.team == proposal.seller.team {
        return Err(DraftPickValueError::InvalidDistribution(
            "assembled trade requires distinct buyer and seller".to_owned(),
        ));
    }
    let mut player_movements = Vec::new();
    let mut pick_checks = Vec::new();
    let assets_to_buyer = assemble_assets(
        proposal.assets_to_buyer,
        &proposal.seller.team,
        &proposal.buyer.team,
        authority,
        curve,
        current_draft_year,
        &mut player_movements,
        &mut pick_checks,
    )?;
    let assets_to_seller = assemble_assets(
        proposal.assets_to_seller,
        &proposal.buyer.team,
        &proposal.seller.team,
        authority,
        curve,
        current_draft_year,
        &mut player_movements,
        &mut pick_checks,
    )?;
    let contract_authority_complete = player_movements.iter().all(|movement| {
        movement.authority.is_some_and(|row| {
            row.contract_confirmed
                && row.clause_reviewed
                && row.cap_hit.is_some()
                && row.destination_allowed.is_some()
                && valid_execution_evidence(&row.source_url, &row.observed_at)
        })
    });
    let package_has_pick = !pick_checks.is_empty();
    let pick_ownership_confirmed = package_has_pick.then(|| {
        pick_checks.iter().all(|check| {
            check.authority.is_some_and(|row| {
                row.confirmed
                    && row.owner == check.from
                    && valid_execution_evidence(&row.source_url, &row.observed_at)
            })
        })
    });
    let cap_compliant = derive_cap_compliance(
        &proposal.buyer.team,
        &proposal.seller.team,
        authority,
        &player_movements,
    );
    let buyer_cap_space_delta = derive_cap_space_delta(&proposal.buyer.team, &player_movements);
    let seller_cap_space_delta = derive_cap_space_delta(&proposal.seller.team, &player_movements);
    let roster_compliant = derive_roster_compliance(
        &proposal.buyer.team,
        &proposal.seller.team,
        authority,
        &player_movements,
    );
    let retention_compliant = derive_retention_compliance(
        &proposal.buyer.team,
        &proposal.seller.team,
        authority,
        &player_movements,
    );
    Ok(TradePackageInput {
        buyer: proposal.buyer,
        seller: proposal.seller,
        assets_to_buyer,
        assets_to_seller,
        buyer_cap_space_delta,
        seller_cap_space_delta,
        transaction_gates: TradeTransactionGates {
            cap_compliant,
            roster_compliant,
            retention_compliant,
            contract_authority_complete,
            pick_ownership_confirmed,
        },
    })
}

#[derive(Clone, Copy)]
struct PlayerMovement<'a> {
    from: &'a str,
    to: &'a str,
    authority: Option<&'a TradePlayerExecutionAuthority>,
    retained_cap_hit: u64,
}

#[derive(Clone, Copy)]
struct PickCheck<'a> {
    from: &'a str,
    authority: Option<&'a TradeDraftPickExecutionAuthority>,
}

#[allow(clippy::too_many_arguments)]
fn assemble_assets<'a>(
    assets: Vec<TradeAssetAssemblyInput>,
    from: &'a str,
    to: &'a str,
    authority: &'a TradeExecutionAuthorityInput,
    curve: &DraftPickValueCurve,
    current_draft_year: u16,
    player_movements: &mut Vec<PlayerMovement<'a>>,
    pick_checks: &mut Vec<PickCheck<'a>>,
) -> Result<Vec<TradeAssetValueInput>, DraftPickValueError> {
    assets
        .into_iter()
        .map(|asset| match asset {
            TradeAssetAssemblyInput::Player {
                player_id,
                mut asset,
                retained_cap_hit,
            } => {
                if player_id == 0 || asset.draft_pick.is_some() {
                    return Err(DraftPickValueError::InvalidDistribution(
                        "player assets require a nonzero player ID and cannot contain draft-pick value"
                            .to_owned(),
                    ));
                }
                let player_authority = authority
                    .players
                    .iter()
                    .find(|row| row.player_id == player_id && row.organization == from);
                if let Some(row) = player_authority {
                    asset.availability.destination_allowed = row.destination_allowed;
                }
                player_movements.push(PlayerMovement {
                    from,
                    to,
                    authority: player_authority,
                    retained_cap_hit: retained_cap_hit.unwrap_or_default(),
                });
                Ok(*asset)
            }
            TradeAssetAssemblyInput::DraftPick { asset } => {
                let pick_authority = authority
                    .draft_picks
                    .iter()
                    .find(|row| row.asset_id == asset.id);
                pick_checks.push(PickCheck {
                    from,
                    authority: pick_authority,
                });
                value_draft_pick_asset(curve, &asset, current_draft_year)
                    .map(|value| draft_pick_trade_asset(value, curve))
            }
        })
        .collect()
}

fn derive_cap_compliance(
    buyer: &str,
    seller: &str,
    authority: &TradeExecutionAuthorityInput,
    movements: &[PlayerMovement<'_>],
) -> Option<bool> {
    [buyer, seller].into_iter().try_fold(true, |all, team| {
        let team_authority = authority.teams.iter().find(|row| row.team == team)?;
        if !valid_execution_evidence(&team_authority.source_url, &team_authority.observed_at) {
            return None;
        }
        let upper = team_authority.upper_limit?;
        let mut post_trade = i128::from(team_authority.committed_cap_hit?);
        for movement in movements {
            let cap_hit = movement.authority?.cap_hit?;
            if movement.retained_cap_hit > cap_hit {
                return None;
            }
            let transferred_cap_hit = i128::from(cap_hit - movement.retained_cap_hit);
            if movement.from == team {
                post_trade -= transferred_cap_hit;
            }
            if movement.to == team {
                post_trade += transferred_cap_hit;
            }
        }
        Some(all && post_trade >= 0 && post_trade <= i128::from(upper))
    })
}

fn derive_cap_space_delta(team: &str, movements: &[PlayerMovement<'_>]) -> Option<f64> {
    let delta = movements.iter().try_fold(0_i128, |delta, movement| {
        let cap_hit = movement.authority?.cap_hit?;
        if movement.retained_cap_hit > cap_hit {
            return None;
        }
        let transferred = i128::from(cap_hit - movement.retained_cap_hit);
        Some(if movement.from == team {
            delta + transferred
        } else if movement.to == team {
            delta - transferred
        } else {
            delta
        })
    })?;
    Some(delta as f64 / 1_000_000.0)
}

fn derive_retention_compliance(
    buyer: &str,
    seller: &str,
    authority: &TradeExecutionAuthorityInput,
    movements: &[PlayerMovement<'_>],
) -> Option<bool> {
    let retained = movements
        .iter()
        .filter(|movement| movement.retained_cap_hit > 0)
        .collect::<Vec<_>>();
    if retained.is_empty() {
        return Some(true);
    }
    [buyer, seller].into_iter().try_fold(true, |all, team| {
        let team_row = authority.teams.iter().find(|row| row.team == team)?;
        if !valid_execution_evidence(&team_row.source_url, &team_row.observed_at) {
            return None;
        }
        let used = retained
            .iter()
            .filter(|movement| movement.from == team)
            .count();
        let slots = usize::from(team_row.retained_salary_slots_available?);
        let amounts_valid = retained
            .iter()
            .filter(|movement| movement.from == team)
            .all(|movement| {
                movement
                    .authority
                    .and_then(|row| row.cap_hit)
                    .is_some_and(|cap_hit| movement.retained_cap_hit <= cap_hit / 2)
            });
        Some(all && used <= slots && amounts_valid)
    })
}

fn derive_roster_compliance(
    buyer: &str,
    seller: &str,
    authority: &TradeExecutionAuthorityInput,
    movements: &[PlayerMovement<'_>],
) -> Option<bool> {
    [buyer, seller].into_iter().try_fold(true, |all, team| {
        let row = authority.teams.iter().find(|row| row.team == team)?;
        if !valid_execution_evidence(&row.source_url, &row.observed_at) {
            return None;
        }
        let mut count = i16::from(row.active_roster_players?);
        for movement in movements {
            if movement.from == team {
                count -= 1;
            }
            if movement.to == team {
                count += 1;
            }
        }
        Some(all && count >= 0 && count <= i16::from(row.max_active_roster_players))
    })
}

fn validate_execution_authority(
    authority: &TradeExecutionAuthorityInput,
) -> Result<(), DraftPickValueError> {
    let unique_teams = authority
        .teams
        .iter()
        .map(|row| row.team.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let unique_players = authority
        .players
        .iter()
        .map(|row| row.player_id)
        .collect::<std::collections::BTreeSet<_>>();
    let unique_picks = authority
        .draft_picks
        .iter()
        .map(|row| row.asset_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if unique_teams.len() != authority.teams.len()
        || unique_players.len() != authority.players.len()
        || unique_picks.len() != authority.draft_picks.len()
        || authority
            .teams
            .iter()
            .any(|row| row.team.trim().is_empty() || row.max_active_roster_players == 0)
        || authority.players.iter().any(|row| {
            row.player_id == 0
                || row.organization.trim().is_empty()
                || row.source_url.as_deref().is_some_and(str::is_empty)
        })
        || authority
            .draft_picks
            .iter()
            .any(|row| row.asset_id.trim().is_empty() || row.owner.trim().is_empty())
    {
        return Err(DraftPickValueError::InvalidDistribution(
            "trade execution authority requires unique, non-empty, internally valid rows"
                .to_owned(),
        ));
    }
    Ok(())
}

const fn default_max_active_roster_players() -> u8 {
    23
}

fn valid_execution_evidence(source_url: &Option<String>, observed_at: &Option<String>) -> bool {
    source_url.as_deref().is_some_and(|value| {
        let value = value.trim();
        value.starts_with("https://") || value.starts_with("http://")
    }) && observed_at.as_deref().is_some_and(|value| {
        chrono::DateTime::parse_from_rfc3339(value).is_ok()
            || chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok()
    })
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum DraftPickValueError {
    #[error("draft-pick curve configuration is invalid: {0}")]
    InvalidConfig(String),
    #[error("draft-pick observations are invalid: {0}")]
    InvalidObservations(String),
    #[error("draft-pick slot distribution is invalid: {0}")]
    InvalidDistribution(String),
}

pub fn draft_pick_trade_asset(
    value: DraftPickAssetValue,
    curve: &DraftPickValueCurve,
) -> TradeAssetValueInput {
    TradeAssetValueInput {
        id: value.id.clone(),
        label: format!("{} draft selection", value.draft_year),
        position: None,
        value_basis: TradeValueBasis {
            outcome_measure: curve.outcome_measure.clone(),
            horizon_years: curve.outcome_horizon_years,
            method: curve.method.clone(),
        },
        market_value: value.expected_value,
        current_value: 0.0,
        future_value: value.expected_value,
        season_points_impact: 0.0,
        uncertainty: value.uncertainty_width,
        availability: TradeAvailabilityEvidence {
            kind: TradeAvailabilityKind::DepthSurplus,
            probability: 1.0,
            source_url: None,
            observed_at: None,
            destination_allowed: Some(true),
        },
        draft_pick: Some(value),
        disclosures: vec![format!(
            "Pick control value uses {} over {} post-draft years.",
            curve.outcome_measure, curve.outcome_horizon_years
        )],
    }
}

pub fn evaluate_trade_package(
    input: TradePackageInput,
) -> Result<TradePackageEvaluationView, DraftPickValueError> {
    validate_trade_package(&input)?;
    let to_buyer_market = package_market_value(&input.assets_to_buyer);
    let to_seller_market = package_market_value(&input.assets_to_seller);
    let buyer_utility_delta = package_utility(&input.assets_to_buyer, &input.buyer)
        - package_utility(&input.assets_to_seller, &input.buyer)
        + input.buyer_cap_space_delta.unwrap_or_default() * input.buyer.cap_weight;
    let seller_utility_delta = package_utility(&input.assets_to_seller, &input.seller)
        - package_utility(&input.assets_to_buyer, &input.seller)
        + input.seller_cap_space_delta.unwrap_or_default() * input.seller.cap_weight;
    let buyer_season_points_delta = package_season_points(&input.assets_to_buyer)
        - package_season_points(&input.assets_to_seller);
    let seller_season_points_delta = -buyer_season_points_delta;
    let larger = to_buyer_market.max(to_seller_market);
    let fairness_score = if larger <= f64::EPSILON {
        0.0
    } else {
        (1.0 - (to_buyer_market - to_seller_market).abs() / larger).clamp(0.0, 1.0)
    };
    let asset_feasibility_probability = input
        .assets_to_buyer
        .iter()
        .chain(&input.assets_to_seller)
        .map(asset_feasibility)
        .product::<f64>();
    let package_has_pick = input
        .assets_to_buyer
        .iter()
        .chain(&input.assets_to_seller)
        .any(|asset| asset.draft_pick.is_some());
    let transaction_ready = transaction_gates_ready(&input.transaction_gates, package_has_pick);
    let feasibility_probability = if transaction_ready {
        asset_feasibility_probability
    } else {
        0.0
    };
    let mut pick_roles = package_pick_roles(&input.assets_to_buyer, "to_buyer");
    pick_roles.extend(package_pick_roles(&input.assets_to_seller, "to_seller"));
    let mut disclosures = vec![
        "Mutual utility reflects each club's supplied timeline, need, and cap weights; it is not evidence that either club discussed the proposal."
            .to_owned(),
        "A draft pick below 40% of its receiving package is labeled rounding; larger shares are principal assets."
            .to_owned(),
        "Unknown cap, roster, retention, contract/clauses, or required pick ownership authority blocks transaction-ready and mutual-benefit labels."
            .to_owned(),
    ];
    disclosures.extend(
        input
            .assets_to_buyer
            .iter()
            .chain(&input.assets_to_seller)
            .flat_map(|asset| {
                asset
                    .disclosures
                    .iter()
                    .map(|note| format!("{}: {note}", asset.label))
            }),
    );
    Ok(TradePackageEvaluationView {
        buyer: input.buyer.team,
        seller: input.seller.team,
        assets_to_buyer: input
            .assets_to_buyer
            .iter()
            .map(|asset| asset.label.clone())
            .collect(),
        assets_to_seller: input
            .assets_to_seller
            .iter()
            .map(|asset| asset.label.clone())
            .collect(),
        buyer_utility_delta,
        seller_utility_delta,
        market_value_to_buyer: to_buyer_market,
        market_value_to_seller: to_seller_market,
        market_value_gap: to_buyer_market - to_seller_market,
        buyer_season_points_delta,
        seller_season_points_delta,
        buyer_cap_space_delta: input.buyer_cap_space_delta,
        seller_cap_space_delta: input.seller_cap_space_delta,
        fairness_score,
        feasibility_probability,
        transaction_ready,
        transaction_gates: input.transaction_gates,
        mutually_beneficial: buyer_utility_delta >= 0.0
            && seller_utility_delta >= 0.0
            && feasibility_probability > 0.0,
        pick_roles,
        season_forecast_impact: None,
        lineup_impact: None,
        disclosures,
    })
}

pub fn attach_trade_package_season_forecast(
    market: &mut TradeMarketEvaluationView,
    buyer: &str,
    seller: &str,
    baseline: &TeamSeasonForecastView,
    scenario: &TeamSeasonForecastView,
) -> Result<(), DraftPickValueError> {
    let impacts = compare_team_season_forecast_scenarios(baseline, scenario)
        .map_err(DraftPickValueError::InvalidDistribution)?;
    attach_trade_package_season_impacts(market, buyer, seller, &impacts)
}

fn attach_trade_package_season_impacts(
    market: &mut TradeMarketEvaluationView,
    buyer: &str,
    seller: &str,
    impacts: &[TeamSeasonScenarioImpactRow],
) -> Result<(), DraftPickValueError> {
    let matching = market
        .proposals
        .iter_mut()
        .filter(|proposal| proposal.buyer == buyer && proposal.seller == seller)
        .collect::<Vec<_>>();
    if matching.len() != 1 {
        return Err(DraftPickValueError::InvalidDistribution(format!(
            "paired season forecast requires exactly one {buyer}/{seller} proposal"
        )));
    }
    let buyer_impact = impacts
        .iter()
        .find(|row| row.team == buyer)
        .ok_or_else(|| {
            DraftPickValueError::InvalidDistribution(format!(
                "paired season forecast is missing buyer {buyer}"
            ))
        })?;
    let seller_impact = impacts
        .iter()
        .find(|row| row.team == seller)
        .ok_or_else(|| {
            DraftPickValueError::InvalidDistribution(format!(
                "paired season forecast is missing seller {seller}"
            ))
        })?;
    let proposal = matching.into_iter().next().expect("one proposal");
    proposal.season_forecast_impact = Some(TradePackageSeasonForecastImpactView {
        buyer: trade_team_season_delta(buyer_impact),
        seller: trade_team_season_delta(seller_impact),
        buyer_points_residual_vs_isolated: buyer_impact.average_points_delta
            - proposal.buyer_season_points_delta,
    });
    proposal.disclosures.push(
        "Season deltas use a paired baseline/scenario simulation with the same schedule, trials, and seed; they supersede the isolated points estimate for forecast interpretation."
            .to_owned(),
    );
    Ok(())
}

fn trade_team_season_delta(
    impact: &TeamSeasonScenarioImpactRow,
) -> TradeTeamSeasonForecastDeltaView {
    TradeTeamSeasonForecastDeltaView {
        team: impact.team.clone(),
        average_points_delta: impact.average_points_delta,
        playoff_probability_delta: impact.playoff_probability_delta,
        stanley_cup_probability_delta: impact.stanley_cup_probability_delta,
    }
}

pub fn build_trade_lineup_scenario(
    input: TradeLineupScenarioInput,
) -> Result<TradeLineupScenarioView, DraftPickValueError> {
    validate_trade_lineup_input(&input)?;
    let before = optimize_trade_lineup(&input.baseline_roster, input.limits);
    let outgoing = input
        .outgoing_player_ids
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let mut after_roster = input
        .baseline_roster
        .iter()
        .filter(|player| !outgoing.contains(&player.player_id))
        .cloned()
        .collect::<Vec<_>>();
    after_roster.extend(input.incoming_players.iter().cloned());
    let after = optimize_trade_lineup(&after_roster, input.limits);
    let before_ids = before
        .iter()
        .map(|row| row.player_id)
        .collect::<std::collections::BTreeSet<_>>();
    let after_ids = after
        .iter()
        .map(|row| row.player_id)
        .collect::<std::collections::BTreeSet<_>>();
    let labels = input
        .baseline_roster
        .iter()
        .chain(&input.incoming_players)
        .map(|player| (player.player_id, player.label.as_str()))
        .collect::<BTreeMap<_, _>>();
    let names = |ids: Vec<u32>| {
        ids.into_iter()
            .filter_map(|id| labels.get(&id).map(|label| (*label).to_owned()))
            .collect::<Vec<_>>()
    };
    let added_to_lineup = names(after_ids.difference(&before_ids).copied().collect());
    let explicitly_removed_from_lineup = names(
        before_ids
            .difference(&after_ids)
            .filter(|id| outgoing.contains(id))
            .copied()
            .collect(),
    );
    let displaced_by_competition = names(
        before_ids
            .difference(&after_ids)
            .filter(|id| !outgoing.contains(id))
            .copied()
            .collect(),
    );
    let before_strength = before.iter().map(|row| row.effective_score).sum::<f64>();
    let after_strength = after.iter().map(|row| row.effective_score).sum::<f64>();
    Ok(TradeLineupScenarioView {
        team: input.team,
        before,
        after,
        added_to_lineup,
        explicitly_removed_from_lineup,
        displaced_by_competition,
        before_strength,
        after_strength,
        strength_delta: after_strength - before_strength,
        projected_lineup: None,
        disclosures: vec![
            "Dressed-player assignment maximizes the supplied confidence-weighted IceLines scores across C/LW/RW/D/G capacity and declared multi-position eligibility."
                .to_owned(),
            "Alternate-side assignments apply each player's explicit penalty; chemistry, special teams, opponent matchups, and waiver consequences require downstream recomputation."
                .to_owned(),
        ],
    })
}

pub fn build_trade_lineup_scenario_from_projection(
    projection: &TeamLineupProjectionView,
    change: TradeLineupProjectionChangeInput,
) -> Result<TradeLineupScenarioView, DraftPickValueError> {
    if projection.team.trim().is_empty() {
        return Err(DraftPickValueError::InvalidDistribution(
            "trade lineup projection requires a team".to_owned(),
        ));
    }
    let mut players = BTreeMap::<u32, TradeLineupPlayerInput>::new();
    let mut skipped_unscored_extras = Vec::new();
    for player in projection
        .forward_lines
        .iter()
        .flat_map(|line| [&line.left_wing, &line.center, &line.right_wing])
        .chain(
            projection
                .defense_pairs
                .iter()
                .flat_map(|pair| [&pair.left, &pair.right]),
        )
        .chain([&projection.goalies.starter, &projection.goalies.backup])
        .flatten()
    {
        let converted = trade_lineup_player_from_projection(player)?;
        insert_projected_lineup_player(&mut players, player, converted)?;
    }
    for player in &projection.extras {
        let Some(_) = player.score.value else {
            skipped_unscored_extras.push(player.display_name.clone());
            continue;
        };
        let converted = trade_lineup_player_from_projection(player)?;
        insert_projected_lineup_player(&mut players, player, converted)?;
    }
    if change
        .disclosures
        .iter()
        .any(|disclosure| disclosure.trim().is_empty())
    {
        return Err(DraftPickValueError::InvalidDistribution(
            "trade lineup change disclosures cannot be empty".to_owned(),
        ));
    }
    let incoming_ids = change
        .incoming_players
        .iter()
        .map(|player| player.player_id)
        .collect::<std::collections::BTreeSet<_>>();
    let projection_incoming_ids = change
        .projection_incoming_players
        .iter()
        .map(|player| player.player_id)
        .collect::<std::collections::BTreeSet<_>>();
    if !projection_incoming_ids.is_empty() && projection_incoming_ids != incoming_ids {
        return Err(DraftPickValueError::InvalidDistribution(
            "full-projection incoming player IDs must exactly match trade-lineup incoming player IDs"
                .to_owned(),
        ));
    }
    if !change.projection_incoming_players.is_empty()
        && change.limits != TradeLineupLimits::default()
    {
        return Err(DraftPickValueError::InvalidDistribution(
            "full team-lineup projection requires the NHL 4C/4LW/4RW/6D/2G shape".to_owned(),
        ));
    }
    let projection_incoming_players = change.projection_incoming_players;
    let outgoing_player_ids = change.outgoing_player_ids;
    let mut view = build_trade_lineup_scenario(TradeLineupScenarioInput {
        team: projection.team.clone(),
        baseline_roster: players.into_values().collect(),
        incoming_players: change.incoming_players,
        outgoing_player_ids: outgoing_player_ids.clone(),
        limits: change.limits,
    })?;
    let projected_lineup = if projection_incoming_players.is_empty() {
        None
    } else {
        Some(
            rebuild_team_lineup_projection(
                projection,
                TeamLineupRosterChangeInput {
                    incoming_players: projection_incoming_players,
                    outgoing_player_ids,
                    requested_slots: trade_lineup_requested_slots(&view.after),
                },
            )
            .map_err(|error| DraftPickValueError::InvalidDistribution(error.to_string()))?,
        )
    };
    view.disclosures.push(format!(
        "Baseline roster and scores were adapted from {} using {}.",
        projection.schema, projection.score_method
    ));
    if !skipped_unscored_extras.is_empty() {
        skipped_unscored_extras.sort();
        view.disclosures.push(format!(
            "Unscored extras were excluded rather than valued as zero: {}.",
            skipped_unscored_extras.join(", ")
        ));
    }
    view.disclosures.extend(change.disclosures);
    if projected_lineup.is_some() {
        view.disclosures.push(
            "Four forward lines, three defense pairs, goalies, PP1/PP2, and PK1/PK2 were regenerated by the core team-lineup primitive."
                .to_owned(),
        );
    }
    view.projected_lineup = projected_lineup;
    Ok(view)
}

pub fn build_trade_lineup_board(
    projection: &TeamLineupProjectionView,
    input: TradeLineupBoardInput,
) -> Result<TradeLineupBoardView, DraftPickValueError> {
    if input.candidates.is_empty() {
        return Err(DraftPickValueError::InvalidDistribution(
            "trade lineup board requires at least one candidate".to_owned(),
        ));
    }
    let mut candidate_ids = BTreeSet::new();
    let mut rows = Vec::with_capacity(input.candidates.len());
    for candidate in input.candidates {
        if candidate.candidate_id.trim().is_empty()
            || candidate.label.trim().is_empty()
            || !candidate_ids.insert(candidate.candidate_id.clone())
        {
            return Err(DraftPickValueError::InvalidDistribution(
                "trade lineup board candidate IDs and labels must be nonempty and IDs unique"
                    .to_owned(),
            ));
        }
        if !candidate.availability_probability.is_finite()
            || !(0.0..=1.0).contains(&candidate.availability_probability)
            || !candidate.feasibility_probability.is_finite()
            || !(0.0..=1.0).contains(&candidate.feasibility_probability)
            || candidate
                .disclosures
                .iter()
                .any(|disclosure| disclosure.trim().is_empty())
        {
            return Err(DraftPickValueError::InvalidDistribution(format!(
                "trade lineup board candidate {} has invalid feasibility or disclosures",
                candidate.candidate_id
            )));
        }
        if candidate.transaction_ready && candidate.package_evaluation.is_none() {
            return Err(DraftPickValueError::InvalidDistribution(format!(
                "trade lineup board candidate {} claims transaction readiness without a package evaluation",
                candidate.candidate_id
            )));
        }
        if let Some(package) = &candidate.package_evaluation {
            if package.buyer != projection.team
                || package.transaction_ready != candidate.transaction_ready
                || (package.feasibility_probability - candidate.feasibility_probability).abs()
                    > 1e-9
            {
                return Err(DraftPickValueError::InvalidDistribution(format!(
                    "trade lineup board candidate {} does not match its package evaluation",
                    candidate.candidate_id
                )));
            }
        }
        let scenario = build_trade_lineup_scenario_from_projection(projection, candidate.change)?;
        let actionable_score = candidate
            .transaction_ready
            .then(|| scenario.strength_delta.max(0.0) * candidate.feasibility_probability);
        rows.push(TradeLineupBoardRowView {
            candidate_id: candidate.candidate_id,
            label: candidate.label,
            hockey_rank: 0,
            actionable_rank: None,
            availability_probability: candidate.availability_probability,
            feasibility_probability: candidate.feasibility_probability,
            transaction_ready: candidate.transaction_ready,
            actionable_score,
            package_evaluation: candidate.package_evaluation,
            scenario,
            disclosures: candidate.disclosures,
        });
    }

    rows.sort_by(|a, b| {
        b.scenario
            .strength_delta
            .total_cmp(&a.scenario.strength_delta)
            .then_with(|| a.candidate_id.cmp(&b.candidate_id))
    });
    for (index, row) in rows.iter_mut().enumerate() {
        row.hockey_rank = (index + 1) as u16;
    }
    let mut actionable = rows
        .iter()
        .enumerate()
        .filter_map(|(index, row)| row.actionable_score.map(|score| (index, score)))
        .collect::<Vec<_>>();
    actionable.sort_by(|(a_index, a_score), (b_index, b_score)| {
        b_score.total_cmp(a_score).then_with(|| {
            rows[*a_index]
                .candidate_id
                .cmp(&rows[*b_index].candidate_id)
        })
    });
    for (rank, (index, _)) in actionable.into_iter().enumerate() {
        rows[index].actionable_rank = Some((rank + 1) as u16);
    }

    Ok(TradeLineupBoardView {
        schema: TRADE_LINEUP_BOARD_SCHEMA.to_owned(),
        team: projection.team.clone(),
        rows,
        disclosures: vec![
            "Hockey rank orders modeled dressed-lineup strength without claiming a trade is available or executable."
                .to_owned(),
            "Actionable rank is published only after transaction readiness passes and multiplies positive lineup impact by the supplied feasibility probability."
                .to_owned(),
        ],
    })
}

fn trade_lineup_requested_slots(
    assignments: &[TradeLineupAssignmentView],
) -> BTreeMap<u32, TeamLineupRequestedSlot> {
    let mut slots = BTreeMap::new();
    for position in [
        TradeLineupPosition::LeftWing,
        TradeLineupPosition::Center,
        TradeLineupPosition::RightWing,
        TradeLineupPosition::Defense,
        TradeLineupPosition::Goalie,
    ] {
        let mut players = assignments
            .iter()
            .filter(|row| row.position == position)
            .collect::<Vec<_>>();
        players.sort_by(|a, b| {
            b.effective_score
                .total_cmp(&a.effective_score)
                .then_with(|| a.player_id.cmp(&b.player_id))
        });
        for (index, player) in players.into_iter().enumerate() {
            let slot = match position {
                TradeLineupPosition::LeftWing
                | TradeLineupPosition::Center
                | TradeLineupPosition::RightWing => {
                    let forward_position = match position {
                        TradeLineupPosition::LeftWing => LineupForwardPosition::LeftWing,
                        TradeLineupPosition::Center => LineupForwardPosition::Center,
                        TradeLineupPosition::RightWing => LineupForwardPosition::RightWing,
                        _ => unreachable!(),
                    };
                    if player.assignment_kind == TradeLineupAssignmentKind::Alternate {
                        TeamLineupRequestedSlot::FlexibleForward {
                            line: (index + 1) as u8,
                            position: forward_position,
                        }
                    } else {
                        TeamLineupRequestedSlot::Forward {
                            line: (index + 1) as u8,
                            position: forward_position,
                        }
                    }
                }
                TradeLineupPosition::Defense => TeamLineupRequestedSlot::Defense {
                    pair: (index / 2 + 1) as u8,
                    right_side: index % 2 == 1,
                },
                TradeLineupPosition::Goalie => TeamLineupRequestedSlot::Goalie {
                    starter: index == 0,
                },
            };
            slots.insert(player.player_id, slot);
        }
    }
    slots
}

fn insert_projected_lineup_player(
    players: &mut BTreeMap<u32, TradeLineupPlayerInput>,
    player: &TeamLineupPlayerView,
    converted: TradeLineupPlayerInput,
) -> Result<(), DraftPickValueError> {
    if players.insert(player.player_id, converted).is_some() {
        return Err(DraftPickValueError::InvalidDistribution(format!(
            "trade lineup projection repeats player {}",
            player.player_id
        )));
    }
    Ok(())
}

fn trade_lineup_player_from_projection(
    player: &TeamLineupPlayerView,
) -> Result<TradeLineupPlayerInput, DraftPickValueError> {
    let projected_score = player.score.value.ok_or_else(|| {
        DraftPickValueError::InvalidDistribution(format!(
            "trade lineup projection player {} has no scored value",
            player.player_id
        ))
    })?;
    let natural_positions = player
        .eligible_positions
        .iter()
        .copied()
        .map(trade_lineup_position)
        .collect::<Vec<_>>();
    Ok(TradeLineupPlayerInput {
        player_id: player.player_id,
        label: player.display_name.clone(),
        natural_positions,
        alternate_positions: Vec::new(),
        projected_score,
        alternate_position_penalty: 0.0,
    })
}

const fn trade_lineup_position(position: Position) -> TradeLineupPosition {
    match position {
        Position::Center => TradeLineupPosition::Center,
        Position::LeftWing => TradeLineupPosition::LeftWing,
        Position::RightWing => TradeLineupPosition::RightWing,
        Position::Defense => TradeLineupPosition::Defense,
        Position::Goalie => TradeLineupPosition::Goalie,
    }
}

pub fn attach_trade_package_lineup_impacts(
    market: &mut TradeMarketEvaluationView,
    buyer: &str,
    seller: &str,
    buyer_lineup: TradeLineupScenarioView,
    seller_lineup: TradeLineupScenarioView,
) -> Result<(), DraftPickValueError> {
    if buyer_lineup.team != buyer || seller_lineup.team != seller {
        return Err(DraftPickValueError::InvalidDistribution(
            "trade lineup teams must match the package buyer and seller".to_owned(),
        ));
    }
    let matching = market
        .proposals
        .iter_mut()
        .filter(|proposal| proposal.buyer == buyer && proposal.seller == seller)
        .collect::<Vec<_>>();
    if matching.len() != 1 {
        return Err(DraftPickValueError::InvalidDistribution(format!(
            "lineup attachment requires exactly one {buyer}/{seller} proposal"
        )));
    }
    let proposal = matching.into_iter().next().expect("one proposal");
    proposal.lineup_impact = Some(TradePackageLineupImpactView {
        buyer: buyer_lineup,
        seller: seller_lineup,
    });
    proposal.disclosures.push(
        "Lineup displacement is optimized in core from explicit eligibility and score inputs; paired season simulation must consume the resulting net lineup state."
            .to_owned(),
    );
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TradeLineupState {
    centers: u8,
    left_wings: u8,
    right_wings: u8,
    defense: u8,
    goalies: u8,
}

impl TradeLineupState {
    const ZERO: Self = Self {
        centers: 0,
        left_wings: 0,
        right_wings: 0,
        defense: 0,
        goalies: 0,
    };

    const fn dressed(self) -> u8 {
        self.centers + self.left_wings + self.right_wings + self.defense + self.goalies
    }

    fn assign(self, position: TradeLineupPosition, limits: TradeLineupLimits) -> Option<Self> {
        let mut next = self;
        let (count, limit) = match position {
            TradeLineupPosition::Center => (&mut next.centers, limits.centers),
            TradeLineupPosition::LeftWing => (&mut next.left_wings, limits.left_wings),
            TradeLineupPosition::RightWing => (&mut next.right_wings, limits.right_wings),
            TradeLineupPosition::Defense => (&mut next.defense, limits.defense),
            TradeLineupPosition::Goalie => (&mut next.goalies, limits.goalies),
        };
        if *count >= limit {
            return None;
        }
        *count += 1;
        Some(next)
    }
}

#[derive(Clone)]
struct TradeLineupSelection {
    score: f64,
    assignments: Vec<TradeLineupAssignmentView>,
}

fn optimize_trade_lineup(
    players: &[TradeLineupPlayerInput],
    limits: TradeLineupLimits,
) -> Vec<TradeLineupAssignmentView> {
    let mut players = players.iter().collect::<Vec<_>>();
    players.sort_by_key(|player| player.player_id);
    let mut states = BTreeMap::from([(
        TradeLineupState::ZERO,
        TradeLineupSelection {
            score: 0.0,
            assignments: Vec::new(),
        },
    )]);
    for player in players {
        let mut next = states.clone();
        let mut positions = player
            .natural_positions
            .iter()
            .copied()
            .map(|position| (position, TradeLineupAssignmentKind::Natural))
            .chain(
                player
                    .alternate_positions
                    .iter()
                    .copied()
                    .filter(|position| !player.natural_positions.contains(position))
                    .map(|position| (position, TradeLineupAssignmentKind::Alternate)),
            )
            .collect::<Vec<_>>();
        positions.sort_by_key(|(position, kind)| (*position, *kind));
        positions.dedup_by_key(|(position, _)| *position);
        for (state, selection) in &states {
            for (position, assignment_kind) in &positions {
                let Some(next_state) = state.assign(*position, limits) else {
                    continue;
                };
                let penalty = if *assignment_kind == TradeLineupAssignmentKind::Alternate {
                    player.alternate_position_penalty
                } else {
                    0.0
                };
                let effective_score = (player.projected_score - penalty).max(0.0);
                let candidate_score = selection.score + effective_score;
                if next
                    .get(&next_state)
                    .is_some_and(|existing| existing.score >= candidate_score)
                {
                    continue;
                }
                let mut assignments = selection.assignments.clone();
                assignments.push(TradeLineupAssignmentView {
                    player_id: player.player_id,
                    label: player.label.clone(),
                    position: *position,
                    assignment_kind: *assignment_kind,
                    effective_score,
                });
                next.insert(
                    next_state,
                    TradeLineupSelection {
                        score: candidate_score,
                        assignments,
                    },
                );
            }
        }
        states = next;
    }
    let mut assignments = states
        .into_iter()
        .max_by(|(left_state, left), (right_state, right)| {
            left_state
                .dressed()
                .cmp(&right_state.dressed())
                .then_with(|| left.score.total_cmp(&right.score))
        })
        .map(|(_, selection)| selection.assignments)
        .unwrap_or_default();
    assignments.sort_by(|left, right| {
        left.position
            .cmp(&right.position)
            .then_with(|| right.effective_score.total_cmp(&left.effective_score))
            .then_with(|| left.player_id.cmp(&right.player_id))
    });
    assignments
}

fn validate_trade_lineup_input(
    input: &TradeLineupScenarioInput,
) -> Result<(), DraftPickValueError> {
    let players = input
        .baseline_roster
        .iter()
        .chain(&input.incoming_players)
        .collect::<Vec<_>>();
    let unique_ids = players
        .iter()
        .map(|player| player.player_id)
        .collect::<std::collections::BTreeSet<_>>();
    let baseline_ids = input
        .baseline_roster
        .iter()
        .map(|player| player.player_id)
        .collect::<std::collections::BTreeSet<_>>();
    let unique_outgoing = input
        .outgoing_player_ids
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let valid_limits = [
        input.limits.centers,
        input.limits.left_wings,
        input.limits.right_wings,
        input.limits.defense,
        input.limits.goalies,
    ]
    .into_iter()
    .all(|limit| limit > 0);
    if input.team.trim().is_empty()
        || input.baseline_roster.is_empty()
        || (input.incoming_players.is_empty() && input.outgoing_player_ids.is_empty())
        || players.len() > 50
        || unique_ids.len() != players.len()
        || unique_outgoing.len() != input.outgoing_player_ids.len()
        || !valid_limits
        || input
            .outgoing_player_ids
            .iter()
            .any(|player_id| !baseline_ids.contains(player_id))
        || players.iter().any(|player| {
            let natural = player
                .natural_positions
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>();
            let alternate = player
                .alternate_positions
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>();
            player.player_id == 0
                || player.label.trim().is_empty()
                || player.natural_positions.is_empty()
                || natural.len() != player.natural_positions.len()
                || alternate.len() != player.alternate_positions.len()
                || !player.projected_score.is_finite()
                || !(0.0..=100.0).contains(&player.projected_score)
                || !player.alternate_position_penalty.is_finite()
                || !(0.0..=100.0).contains(&player.alternate_position_penalty)
        })
    {
        return Err(DraftPickValueError::InvalidDistribution(
            "trade lineup requires unique valid players, positive limits, and baseline-owned outgoing IDs"
                .to_owned(),
        ));
    }
    Ok(())
}

pub fn evaluate_trade_market(
    input: TradeMarketInput,
) -> Result<TradeMarketEvaluationView, DraftPickValueError> {
    if chrono::DateTime::parse_from_rfc3339(&input.as_of).is_err()
        && chrono::NaiveDate::parse_from_str(&input.as_of, "%Y-%m-%d").is_err()
    {
        return Err(DraftPickValueError::InvalidDistribution(
            "trade market as_of must be YYYY-MM-DD or RFC 3339".to_owned(),
        ));
    }
    if input.proposals.is_empty() {
        return Err(DraftPickValueError::InvalidDistribution(
            "trade market requires at least one proposal".to_owned(),
        ));
    }
    let value_basis = common_market_value_basis(&input.proposals)?;
    let mut proposals = input
        .proposals
        .into_iter()
        .map(evaluate_trade_package)
        .collect::<Result<Vec<_>, _>>()?;
    proposals.sort_by(|left, right| {
        right
            .mutually_beneficial
            .cmp(&left.mutually_beneficial)
            .then_with(|| {
                right
                    .feasibility_probability
                    .total_cmp(&left.feasibility_probability)
            })
            .then_with(|| {
                right
                    .buyer_utility_delta
                    .min(right.seller_utility_delta)
                    .total_cmp(&left.buyer_utility_delta.min(left.seller_utility_delta))
            })
            .then_with(|| right.fairness_score.total_cmp(&left.fairness_score))
            .then_with(|| left.seller.cmp(&right.seller))
    });
    Ok(TradeMarketEvaluationView {
        schema: TRADE_MARKET_EVALUATION_SCHEMA.to_owned(),
        as_of: input.as_of,
        value_basis,
        proposals,
        disclosures: vec![
            "Proposals are ranked scenarios, not reported negotiations or transaction advice."
                .to_owned(),
            "Feasibility and mutual utility are separate: a hockey fit can still fail a clause, cap, roster, or evidence gate."
                .to_owned(),
        ],
    })
}

pub fn populate_trade_scout_league_from_camp(
    camp: &TrainingCampLeagueForecastView,
    input: TradeScoutPopulationInput,
) -> Result<TradeScoutPopulationView, DraftPickValueError> {
    validate_trade_scout_population(camp, &input)?;
    let preferences = input
        .preferences
        .iter()
        .map(|preference| (preference.team.as_str(), preference))
        .collect::<BTreeMap<_, _>>();
    let availability = input
        .availability
        .iter()
        .map(|row| (row.player_id, row))
        .collect::<BTreeMap<_, _>>();
    let protected = input
        .protected_player_ids
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut picks_by_team = BTreeMap::<String, Vec<&TradeScoutAssetInput>>::new();
    for pick in &input.draft_pick_assets {
        picks_by_team
            .entry(pick.organization.clone())
            .or_default()
            .push(pick);
    }
    let mut organizations = Vec::new();
    let mut teams_without_forecast = Vec::new();
    let mut players_populated = 0;
    let mut prospects_populated = 0;
    for team in &camp.teams {
        let Some(forecast) = team.forecast.as_ref() else {
            teams_without_forecast.push(team.team.clone());
            continue;
        };
        let preference = preferences.get(team.team.as_str()).ok_or_else(|| {
            DraftPickValueError::InvalidDistribution(format!(
                "trade scout population is missing preference for {}",
                team.team
            ))
        })?;
        let mut assets = forecast
            .players
            .iter()
            .map(|player| {
                players_populated += 1;
                prospects_populated += usize::from(player.prospect);
                camp_player_trade_asset(
                    &team.team,
                    player,
                    availability.get(&player.player_id).copied(),
                    protected.contains(&player.player_id),
                    &input.config,
                )
            })
            .collect::<Result<Vec<_>, DraftPickValueError>>()?;
        if let Some(picks) = picks_by_team.get(&team.team) {
            assets.extend(picks.iter().map(|pick| TradeScoutLeagueAssetInput {
                kind: TradeScoutLeagueAssetKind::DraftPick,
                asset: pick.asset.clone(),
                surplus_score: 100.0,
                protected: pick.protected,
            }));
        }
        organizations.push(TradeScoutLeagueOrganizationInput {
            preference: (*preference).clone(),
            assets,
        });
    }
    if !organizations
        .iter()
        .any(|organization| organization.preference.team == input.buyer)
    {
        return Err(DraftPickValueError::InvalidDistribution(
            "trade scout population has no usable forecast for the buyer".to_owned(),
        ));
    }
    let teams_populated = organizations.len();
    let mut league_config = input.config.league;
    league_config.expected_organizations = camp.teams_requested;
    league_config.allow_partial_inventory = teams_populated != camp.teams_requested;
    let league_input = TradeScoutLeagueInput {
        as_of: input.as_of.clone(),
        buyer: input.buyer,
        organizations,
        config: league_config,
    };
    Ok(TradeScoutPopulationView {
        schema: TRADE_SCOUT_POPULATION_SCHEMA.to_owned(),
        as_of: input.as_of,
        season: camp.season,
        teams_requested: camp.teams_requested,
        teams_populated,
        players_populated,
        prospects_populated,
        picks_populated: input.draft_pick_assets.len(),
        teams_without_forecast,
        league_input,
        disclosures: vec![
            "NHL-player and prospect inventory comes directly from training_camp_league_forecast.v1; missing team forecasts remain explicit coverage gaps."
                .to_owned(),
            "Score-to-control-value translation is an explicit supplied policy, not a learned trade-value claim; contract and pick execution authority remain downstream gates."
                .to_owned(),
        ],
    })
}

pub fn populate_trade_scout_draft_picks(
    curve: &DraftPickValueCurve,
    input: TradeScoutDraftPickPopulationInput,
) -> Result<TradeScoutDraftPickPopulationView, DraftPickValueError> {
    validate_trade_scout_draft_pick_population(curve, &input)?;
    let protected = input
        .protected_asset_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut unresolved_asset_ids = Vec::new();
    let mut assets = Vec::new();
    for ownership in &input.ownership {
        if ownership.status != TradeDraftPickOwnershipStatus::ConfirmedUnconditional {
            unresolved_asset_ids.push(ownership.asset_id.clone());
            continue;
        }
        let curve_round_width = if curve.max_overall_pick >= 7 {
            curve.max_overall_pick / 7
        } else {
            curve.max_overall_pick
        };
        if curve_round_width == 0 || (curve.max_overall_pick < 7 && ownership.round > 1) {
            return Err(DraftPickValueError::InvalidDistribution(format!(
                "pick {} round is outside the supplied curve",
                ownership.asset_id
            )));
        }
        let mut normalized = BTreeMap::<u16, f64>::new();
        for current_round_slot in 1_u16..=32 {
            let historical_round_slot = ((current_round_slot - 1) * curve_round_width / 32) + 1;
            let overall_pick =
                u16::from(ownership.round - 1) * curve_round_width + historical_round_slot;
            *normalized.entry(overall_pick).or_default() += 1.0 / 32.0;
        }
        let slot_outcomes = normalized
            .into_iter()
            .map(|(overall_pick, probability)| DraftPickSlotOutcome {
                overall_pick,
                probability,
            })
            .collect();
        let value = value_draft_pick_asset(
            curve,
            &DraftPickAssetInput {
                id: ownership.asset_id.clone(),
                draft_year: ownership.draft_year,
                slot_outcomes,
            },
            input.current_draft_year,
        )?;
        let mut asset = draft_pick_trade_asset(value, curve);
        asset.label = format!(
            "{} {} round {} pick",
            ownership.original_team, ownership.draft_year, ownership.round
        );
        asset.value_basis = input.value_basis.clone();
        asset.disclosures.push(format!(
            "Ownership was reviewed from {} at {}; future slot value is uniform across the current 32-slot round {} and mapped by within-round percentile to the curve's {}-slot historical era.",
            ownership.source_url, ownership.observed_at, ownership.round, curve_round_width
        ));
        assets.push(TradeScoutAssetInput {
            organization: ownership.owner.clone(),
            protected: protected.contains(ownership.asset_id.as_str()),
            asset,
        });
    }
    assets.sort_by(|left, right| {
        left.organization
            .cmp(&right.organization)
            .then_with(|| left.asset.id.cmp(&right.asset.id))
    });
    unresolved_asset_ids.sort();
    Ok(TradeScoutDraftPickPopulationView {
        schema: TRADE_SCOUT_DRAFT_PICK_POPULATION_SCHEMA.to_owned(),
        as_of: input.as_of,
        picks_supplied: input.ownership.len(),
        picks_populated: assets.len(),
        unresolved_asset_ids,
        assets,
        disclosures: vec![
            "Only reviewed confirmed-unconditional ownership becomes offer inventory; conditional and encumbered rights remain unresolved."
                .to_owned(),
            "Full-round uniform slot outcomes are a conservative preseason baseline, normalized by within-round percentile when the curve's historical league size differs; this is not a prediction of final draft order."
                .to_owned(),
        ],
    })
}

fn validate_trade_scout_draft_pick_population(
    curve: &DraftPickValueCurve,
    input: &TradeScoutDraftPickPopulationInput,
) -> Result<(), DraftPickValueError> {
    let asset_ids = input
        .ownership
        .iter()
        .map(|row| row.asset_id.as_str())
        .collect::<BTreeSet<_>>();
    let protected = input
        .protected_asset_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let known = input
        .ownership
        .iter()
        .map(|row| row.asset_id.as_str())
        .collect::<BTreeSet<_>>();
    let valid_as_of = chrono::DateTime::parse_from_rfc3339(&input.as_of).is_ok()
        || chrono::NaiveDate::parse_from_str(&input.as_of, "%Y-%m-%d").is_ok();
    if curve.schema != DRAFT_PICK_VALUE_CURVE_SCHEMA
        || !valid_as_of
        || input.current_draft_year == 0
        || input.value_basis.outcome_measure.trim().is_empty()
        || input.value_basis.horizon_years == 0
        || input.value_basis.method.trim().is_empty()
        || input.ownership.is_empty()
        || asset_ids.len() != input.ownership.len()
        || protected.len() != input.protected_asset_ids.len()
        || protected.iter().any(|asset_id| !known.contains(asset_id))
        || input.ownership.iter().any(|row| {
            row.asset_id.trim().is_empty()
                || row.owner.trim().is_empty()
                || row.original_team.trim().is_empty()
                || row.draft_year < input.current_draft_year
                || !(1..=7).contains(&row.round)
                || !valid_execution_evidence(
                    &Some(row.source_url.clone()),
                    &Some(row.observed_at.clone()),
                )
                || (row.status != TradeDraftPickOwnershipStatus::ConfirmedUnconditional
                    && row
                        .conditions
                        .as_deref()
                        .is_none_or(|conditions| conditions.trim().is_empty()))
                || (row.status == TradeDraftPickOwnershipStatus::ConfirmedUnconditional
                    && row
                        .conditions
                        .as_deref()
                        .is_some_and(|conditions| !conditions.trim().is_empty()))
        })
    {
        return Err(DraftPickValueError::InvalidDistribution(
            "draft-pick population requires a valid curve/basis/date, unique known protection, future rounds 1-7, dated ownership provenance, and explicit conditions for unresolved rights"
                .to_owned(),
        ));
    }
    Ok(())
}

fn camp_player_trade_asset(
    team: &str,
    player: &TrainingCampPlayerView,
    overlay: Option<&TradeScoutAvailabilityOverlayInput>,
    protected: bool,
    config: &TradeScoutPopulationConfig,
) -> Result<TradeScoutLeagueAssetInput, DraftPickValueError> {
    if !player.projected_score.is_finite()
        || player.projected_score < 0.0
        || !player.gp_confidence.is_finite()
        || !(0.0..=1.0).contains(&player.gp_confidence)
    {
        return Err(DraftPickValueError::InvalidDistribution(format!(
            "camp player {} has a negative/non-finite score or invalid confidence",
            player.player_id
        )));
    }
    let inferred_surplus = if player.incumbent {
        (player.cut_probability + player.healthy_scratch_probability * 0.5) * 100.0
    } else {
        player.selection_loss_probability * 50.0
    }
    .clamp(0.0, 100.0);
    let surplus_score = overlay
        .and_then(|row| row.surplus_score)
        .unwrap_or(inferred_surplus);
    let availability = overlay.map_or_else(
        || TradeAvailabilityEvidence {
            kind: TradeAvailabilityKind::DepthSurplus,
            probability: (surplus_score / 100.0).clamp(0.0, 1.0),
            source_url: None,
            observed_at: None,
            destination_allowed: None,
        },
        |row| row.evidence.clone(),
    );
    let market_value = player.projected_score * config.control_value_per_score;
    let current_share = if player.prospect {
        (0.20 + player.make_probability * 0.45).clamp(0.20, 0.65)
    } else {
        0.75
    };
    let (position, minimum_score) = if player.primary_position.is_forward() {
        ("top_six_forward", config.top_six_forward_score)
    } else if player.primary_position.is_defense() {
        ("top_four_defense", config.top_four_defense_score)
    } else {
        ("starting_goalie", config.starting_goalie_score)
    };
    let role = if player.projected_score >= minimum_score {
        position
    } else if player.primary_position.is_forward() {
        "depth_forward"
    } else if player.primary_position.is_defense() {
        "depth_defense"
    } else {
        "goalie_depth"
    };
    Ok(TradeScoutLeagueAssetInput {
        kind: if player.prospect {
            TradeScoutLeagueAssetKind::Prospect
        } else {
            TradeScoutLeagueAssetKind::NhlPlayer
        },
        asset: TradeAssetValueInput {
            id: player.player_id.to_string(),
            label: player.display_name.clone(),
            position: Some(role.to_owned()),
            value_basis: config.value_basis.clone(),
            market_value,
            current_value: market_value * current_share,
            future_value: market_value * (1.0 - current_share),
            season_points_impact: player.projected_score * config.season_points_per_score,
            uncertainty: market_value * (1.0 - player.gp_confidence),
            availability,
            draft_pick: None,
            disclosures: vec![format!(
                "Camp score {:.2}, GP confidence {:.2}, and prospect={} were translated by the supplied Trade Scout population policy for {team}.",
                player.projected_score, player.gp_confidence, player.prospect
            )],
        },
        surplus_score,
        protected,
    })
}

fn validate_trade_scout_population(
    camp: &TrainingCampLeagueForecastView,
    input: &TradeScoutPopulationInput,
) -> Result<(), DraftPickValueError> {
    let team_ids = camp
        .teams
        .iter()
        .map(|team| team.team.as_str())
        .collect::<BTreeSet<_>>();
    let camp_player_ids = camp
        .teams
        .iter()
        .filter_map(|team| team.forecast.as_ref())
        .flat_map(|forecast| forecast.players.iter().map(|player| player.player_id))
        .collect::<Vec<_>>();
    let camp_player_id_set = camp_player_ids.iter().copied().collect::<BTreeSet<_>>();
    let preferences = input
        .preferences
        .iter()
        .map(|row| row.team.as_str())
        .collect::<BTreeSet<_>>();
    let overlays = input
        .availability
        .iter()
        .map(|row| row.player_id)
        .collect::<BTreeSet<_>>();
    let protected = input
        .protected_player_ids
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let pick_ids = input
        .draft_pick_assets
        .iter()
        .map(|row| row.asset.id.as_str())
        .collect::<BTreeSet<_>>();
    let valid_as_of = chrono::DateTime::parse_from_rfc3339(&input.as_of).is_ok()
        || chrono::NaiveDate::parse_from_str(&input.as_of, "%Y-%m-%d").is_ok();
    if camp.schema != TRAINING_CAMP_LEAGUE_FORECAST_SCHEMA
        || camp.teams.is_empty()
        || camp.teams.len() != camp.teams_requested
        || team_ids.len() != camp.teams.len()
        || camp_player_id_set.len() != camp_player_ids.len()
        || camp.teams.iter().any(|team| {
            team.forecast.as_ref().is_some_and(|forecast| {
                forecast.schema != TRAINING_CAMP_FORECAST_SCHEMA || forecast.team != team.team
            })
        })
        || !valid_as_of
        || input.buyer.trim().is_empty()
        || input.preferences.is_empty()
        || preferences.len() != input.preferences.len()
        || preferences.iter().any(|team| !team_ids.contains(team))
        || overlays.len() != input.availability.len()
        || protected.len() != input.protected_player_ids.len()
        || pick_ids.len() != input.draft_pick_assets.len()
        || input.config.value_basis.outcome_measure.trim().is_empty()
        || input.config.value_basis.horizon_years == 0
        || input.config.value_basis.method.trim().is_empty()
        || [
            input.config.control_value_per_score,
            input.config.season_points_per_score,
            input.config.top_six_forward_score,
            input.config.top_four_defense_score,
            input.config.starting_goalie_score,
        ]
        .into_iter()
        .any(|value| !value.is_finite() || value < 0.0)
        || input.availability.iter().any(|row| {
            row.player_id == 0
                || !camp_player_id_set.contains(&row.player_id)
                || !row.evidence.probability.is_finite()
                || !(0.0..=1.0).contains(&row.evidence.probability)
                || row
                    .surplus_score
                    .is_some_and(|value| !value.is_finite() || !(0.0..=100.0).contains(&value))
                || (!matches!(
                    row.evidence.kind,
                    TradeAvailabilityKind::DepthSurplus | TradeAvailabilityKind::SpeculativeFit
                ) && !valid_execution_evidence(
                    &row.evidence.source_url,
                    &row.evidence.observed_at,
                ))
        })
        || input
            .protected_player_ids
            .iter()
            .any(|player_id| !camp_player_id_set.contains(player_id))
        || input.draft_pick_assets.iter().any(|row| {
            row.organization.trim().is_empty()
                || !team_ids.contains(row.organization.as_str())
                || row.asset.draft_pick.is_none()
                || row.asset.value_basis != input.config.value_basis
        })
    {
        return Err(DraftPickValueError::InvalidDistribution(
            "trade scout population requires a valid as_of and unique camp teams/players, unique known preferences/overlays/protection, 0-1 dated sourced availability, ownership-scoped picks, non-negative translation policy, and one common value basis"
                .to_owned(),
        ));
    }
    Ok(())
}

pub fn build_trade_scout_from_league(
    input: TradeScoutLeagueInput,
) -> Result<TradeScoutLeagueView, DraftPickValueError> {
    validate_trade_scout_league(&input)?;
    let buyer_organization = input
        .organizations
        .iter()
        .find(|organization| organization.preference.team == input.buyer)
        .expect("buyer organization validated");
    let buyer_preference = buyer_organization.preference.clone();
    let targets = input
        .organizations
        .iter()
        .filter(|organization| organization.preference.team != input.buyer)
        .flat_map(|organization| {
            organization.assets.iter().filter_map(|row| {
                let need_score = row
                    .asset
                    .position
                    .as_ref()
                    .and_then(|position| buyer_preference.needs.get(position))
                    .copied()
                    .unwrap_or_default();
                let strong_availability = matches!(
                    row.asset.availability.kind,
                    TradeAvailabilityKind::ReportedRequest
                        | TradeAvailabilityKind::ClubShopping
                        | TradeAvailabilityKind::ContractPressure
                );
                (row.kind == TradeScoutLeagueAssetKind::NhlPlayer
                    && need_score > 0.0
                    && asset_feasibility(&row.asset) > 0.0
                    && (strong_availability
                        || row.surplus_score >= input.config.minimum_surplus_score))
                    .then(|| TradeScoutAssetInput {
                        organization: organization.preference.team.clone(),
                        asset: row.asset.clone(),
                        protected: row.protected,
                    })
            })
        })
        .collect::<Vec<_>>();
    let buyer_assets = buyer_organization
        .assets
        .iter()
        .filter(|row| {
            row.kind != TradeScoutLeagueAssetKind::NhlPlayer
                || row.surplus_score >= input.config.minimum_surplus_score
        })
        .map(|row| TradeScoutAssetInput {
            organization: input.buyer.clone(),
            asset: row.asset.clone(),
            protected: row.protected,
        })
        .collect::<Vec<_>>();
    if targets.is_empty() || buyer_assets.is_empty() {
        return Err(DraftPickValueError::InvalidDistribution(
            "league trade scout derived no need-matched targets or no buyer offer assets"
                .to_owned(),
        ));
    }
    let organizations_supplied = input.organizations.len();
    let inventory_complete = organizations_supplied == input.config.expected_organizations;
    let derived_targets = targets.len();
    let derived_buyer_assets = buyer_assets.len();
    let sellers = input
        .organizations
        .iter()
        .filter(|organization| organization.preference.team != input.buyer)
        .map(|organization| organization.preference.clone())
        .collect();
    let scout = build_trade_scout(TradeScoutInput {
        as_of: input.as_of.clone(),
        buyer: buyer_preference,
        sellers,
        targets,
        buyer_assets,
        config: input.config.scout,
    })?;
    Ok(TradeScoutLeagueView {
        schema: TRADE_SCOUT_LEAGUE_SCHEMA.to_owned(),
        as_of: input.as_of,
        buyer: input.buyer,
        organizations_supplied,
        expected_organizations: input.config.expected_organizations,
        inventory_complete,
        derived_targets,
        derived_buyer_assets,
        scout,
        disclosures: vec![
            "League discovery selects NHL-player targets only where the buyer declares a matching role need and the seller supplies sufficient surplus or stronger dated availability evidence."
                .to_owned(),
            "Buyer offer inventory includes supplied picks and prospects plus NHL players above the same surplus threshold; protected assets remain audit-only downstream."
                .to_owned(),
        ],
    })
}

pub fn build_trade_scout(input: TradeScoutInput) -> Result<TradeScoutView, DraftPickValueError> {
    validate_trade_scout(&input)?;
    let seller_by_team = input
        .sellers
        .iter()
        .map(|seller| (seller.team.as_str(), seller))
        .collect::<BTreeMap<_, _>>();
    let protected_buyer_assets = input
        .buyer_assets
        .iter()
        .filter(|row| row.protected)
        .map(|row| row.asset.label.clone())
        .collect::<Vec<_>>();
    let available_buyer_assets = input
        .buyer_assets
        .iter()
        .filter(|row| !row.protected)
        .map(|row| row.asset.clone())
        .collect::<Vec<_>>();
    let packages = asset_combinations(
        &available_buyer_assets,
        usize::from(input.config.maximum_assets_per_package),
    );
    let mut candidates = Vec::new();
    for target in input.targets.iter().filter(|target| {
        !target.protected
            && target.organization != input.buyer.team
            && asset_feasibility(&target.asset) > 0.0
    }) {
        let seller = seller_by_team
            .get(target.organization.as_str())
            .ok_or_else(|| {
                DraftPickValueError::InvalidDistribution(format!(
                    "trade scout target {} has no seller preference for {}",
                    target.asset.id, target.organization
                ))
            })?;
        if target.asset.market_value <= f64::EPSILON {
            continue;
        }
        let mut evaluated = packages
            .iter()
            .map(|assets| {
                let evaluation = evaluate_trade_package(TradePackageInput {
                    buyer: input.buyer.clone(),
                    seller: (*seller).clone(),
                    assets_to_buyer: vec![target.asset.clone()],
                    assets_to_seller: assets.clone(),
                    buyer_cap_space_delta: None,
                    seller_cap_space_delta: None,
                    transaction_gates: TradeTransactionGates::default(),
                })?;
                Ok((package_market_value(assets), assets, evaluation))
            })
            .collect::<Result<Vec<_>, DraftPickValueError>>()?;
        let value_tolerance = target.asset.market_value.abs().max(1.0) * 1e-9;
        evaluated.retain(|(value, _, _)| {
            *value + value_tolerance >= target.asset.market_value * input.config.opening_offer_ratio
                && *value
                    <= target.asset.market_value * input.config.maximum_price_ratio
                        + value_tolerance
        });
        if evaluated.is_empty() {
            continue;
        }
        evaluated.sort_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then_with(|| package_asset_ids(left.1).cmp(&package_asset_ids(right.1)))
        });
        let opening = evaluated.first().expect("non-empty packages");
        let fair = evaluated
            .iter()
            .min_by(|left, right| {
                (left.0 - target.asset.market_value)
                    .abs()
                    .total_cmp(&(right.0 - target.asset.market_value).abs())
                    .then_with(|| left.0.total_cmp(&right.0))
                    .then_with(|| package_asset_ids(left.1).cmp(&package_asset_ids(right.1)))
            })
            .expect("non-empty packages");
        let maximum = evaluated
            .iter()
            .rev()
            .find(|(_, _, evaluation)| evaluation.buyer_utility_delta >= 0.0)
            .unwrap_or(fair);
        let ladder = TradeNegotiationLadderView {
            target_market_value: target.asset.market_value,
            opening_offer: negotiation_package(
                TradeNegotiationTier::OpeningOffer,
                target.asset.market_value,
                opening,
            ),
            fair_midpoint: negotiation_package(
                TradeNegotiationTier::FairMidpoint,
                target.asset.market_value,
                fair,
            ),
            maximum_acceptable: negotiation_package(
                TradeNegotiationTier::MaximumAcceptable,
                target.asset.market_value,
                maximum,
            ),
            walk_away_market_value: target.asset.market_value
                * input.config.maximum_price_ratio,
            protected_buyer_assets: protected_buyer_assets.clone(),
            disclosures: vec![
                "The ladder enumerates only supplied, unprotected buyer assets and does not infer seller acceptance."
                    .to_owned(),
                "Generated packages remain transaction-blocked until IceLines attaches cap, roster, retention, contract/clause, and pick-ownership authority."
                    .to_owned(),
            ],
        };
        let hockey_fit_score = package_utility(std::slice::from_ref(&target.asset), &input.buyer);
        candidates.push(TradeScoutCandidateView {
            rank: 0,
            target_id: target.asset.id.clone(),
            label: target.asset.label.clone(),
            seller: target.organization.clone(),
            role: target.asset.position.clone(),
            hockey_fit_score,
            discovery_score: hockey_fit_score * asset_feasibility(&target.asset),
            availability: target.asset.availability.clone(),
            negotiation: ladder,
            disclosures: vec![
                "Candidate discovery is a buyer-need and timeline fit, not evidence of trade talks."
                    .to_owned(),
            ],
        });
    }
    candidates.sort_by(|left, right| {
        right
            .discovery_score
            .total_cmp(&left.discovery_score)
            .then_with(|| right.hockey_fit_score.total_cmp(&left.hockey_fit_score))
            .then_with(|| left.label.cmp(&right.label))
            .then_with(|| left.target_id.cmp(&right.target_id))
    });
    candidates.truncate(input.config.maximum_candidates);
    for (index, candidate) in candidates.iter_mut().enumerate() {
        candidate.rank = index as u16 + 1;
    }
    Ok(TradeScoutView {
        schema: TRADE_SCOUT_SCHEMA.to_owned(),
        as_of: input.as_of,
        buyer: input.buyer.team,
        candidates,
        disclosures: vec![
            "The Automatic Trade Scout ranks supplied league assets by buyer-specific hockey utility multiplied by availability; it does not manufacture availability evidence."
                .to_owned(),
            "Protected assets are audited but excluded from every generated offer, and the published maximum is a walk-away boundary rather than a recommendation to pay it."
                .to_owned(),
        ],
    })
}

fn validate_trade_scout_league(input: &TradeScoutLeagueInput) -> Result<(), DraftPickValueError> {
    if input.buyer.trim().is_empty()
        || input.config.expected_organizations < 2
        || !input.config.minimum_surplus_score.is_finite()
        || !(0.0..=100.0).contains(&input.config.minimum_surplus_score)
        || input.organizations.len() < 2
        || (!input.config.allow_partial_inventory
            && input.organizations.len() != input.config.expected_organizations)
        || (chrono::DateTime::parse_from_rfc3339(&input.as_of).is_err()
            && chrono::NaiveDate::parse_from_str(&input.as_of, "%Y-%m-%d").is_err())
    {
        return Err(DraftPickValueError::InvalidDistribution(
            "league trade scout requires a dated buyer, at least two organizations, a 0-100 surplus threshold, and complete expected coverage unless partial inventory is explicit"
                .to_owned(),
        ));
    }
    let teams = input
        .organizations
        .iter()
        .map(|organization| organization.preference.team.as_str())
        .collect::<BTreeSet<_>>();
    let asset_ids = input
        .organizations
        .iter()
        .flat_map(|organization| organization.assets.iter())
        .map(|row| row.asset.id.as_str())
        .collect::<BTreeSet<_>>();
    let asset_count = input
        .organizations
        .iter()
        .map(|organization| organization.assets.len())
        .sum::<usize>();
    if teams.len() != input.organizations.len()
        || !teams.contains(input.buyer.as_str())
        || input.organizations.iter().any(|organization| {
            organization.preference.team.trim().is_empty()
                || organization.assets.is_empty()
                || organization.assets.iter().any(|row| {
                    !row.surplus_score.is_finite() || !(0.0..=100.0).contains(&row.surplus_score)
                })
        })
        || asset_ids.len() != asset_count
    {
        return Err(DraftPickValueError::InvalidDistribution(
            "league trade scout requires unique teams, globally unique assets, a present buyer, non-empty inventories, and 0-100 surplus evidence"
                .to_owned(),
        ));
    }
    Ok(())
}

fn negotiation_package(
    tier: TradeNegotiationTier,
    target_market_value: f64,
    package: &(f64, &Vec<TradeAssetValueInput>, TradePackageEvaluationView),
) -> TradeNegotiationPackageView {
    TradeNegotiationPackageView {
        tier,
        assets_to_seller: package.1.iter().map(|asset| asset.label.clone()).collect(),
        market_value: package.0,
        target_value_ratio: package.0 / target_market_value,
        evaluation: package.2.clone(),
    }
}

fn asset_combinations(
    assets: &[TradeAssetValueInput],
    maximum_assets: usize,
) -> Vec<Vec<TradeAssetValueInput>> {
    fn visit(
        assets: &[TradeAssetValueInput],
        maximum_assets: usize,
        start: usize,
        current: &mut Vec<TradeAssetValueInput>,
        output: &mut Vec<Vec<TradeAssetValueInput>>,
    ) {
        if !current.is_empty() {
            output.push(current.clone());
        }
        if current.len() == maximum_assets {
            return;
        }
        for index in start..assets.len() {
            current.push(assets[index].clone());
            visit(assets, maximum_assets, index + 1, current, output);
            current.pop();
        }
    }
    let mut output = Vec::new();
    visit(assets, maximum_assets, 0, &mut Vec::new(), &mut output);
    output
}

fn package_asset_ids(assets: &[TradeAssetValueInput]) -> Vec<&str> {
    assets.iter().map(|asset| asset.id.as_str()).collect()
}

fn validate_trade_scout(input: &TradeScoutInput) -> Result<(), DraftPickValueError> {
    if (chrono::DateTime::parse_from_rfc3339(&input.as_of).is_err()
        && chrono::NaiveDate::parse_from_str(&input.as_of, "%Y-%m-%d").is_err())
        || input.buyer.team.trim().is_empty()
        || input.sellers.is_empty()
        || input.targets.is_empty()
        || input.buyer_assets.is_empty()
        || !input.config.opening_offer_ratio.is_finite()
        || !input.config.maximum_price_ratio.is_finite()
        || !(0.0..=1.0).contains(&input.config.opening_offer_ratio)
        || input.config.maximum_price_ratio < 1.0
        || input.config.maximum_price_ratio > 2.0
        || input.config.maximum_assets_per_package == 0
        || input.config.maximum_assets_per_package > 3
        || input.config.maximum_candidates == 0
        || input.buyer_assets.len() > 24
    {
        return Err(DraftPickValueError::InvalidDistribution(
            "trade scout requires dated assets, 0-1 opening ratio, 1-2 maximum ratio, one to three assets per package, and bounded candidate inputs"
                .to_owned(),
        ));
    }
    let seller_teams = input
        .sellers
        .iter()
        .map(|seller| seller.team.as_str())
        .collect::<BTreeSet<_>>();
    let all_assets = input.targets.iter().chain(&input.buyer_assets);
    let asset_ids = all_assets
        .clone()
        .map(|row| row.asset.id.as_str())
        .collect::<BTreeSet<_>>();
    if seller_teams.len() != input.sellers.len()
        || seller_teams.contains(input.buyer.team.as_str())
        || asset_ids.len() != input.targets.len() + input.buyer_assets.len()
        || input
            .buyer_assets
            .iter()
            .any(|row| row.organization != input.buyer.team)
        || all_assets.into_iter().any(|row| {
            row.organization.trim().is_empty()
                || row.asset.id.trim().is_empty()
                || row.asset.label.trim().is_empty()
        })
    {
        return Err(DraftPickValueError::InvalidDistribution(
            "trade scout requires unique assets, unique external sellers, and buyer-controlled offer assets"
                .to_owned(),
        ));
    }
    let first_buyer_asset = &input.buyer_assets[0].asset;
    for target in &input.targets {
        let seller = input
            .sellers
            .iter()
            .find(|seller| seller.team == target.organization)
            .ok_or_else(|| {
                DraftPickValueError::InvalidDistribution(format!(
                    "trade scout target {} has no seller preference for {}",
                    target.asset.id, target.organization
                ))
            })?;
        validate_trade_package(&TradePackageInput {
            buyer: input.buyer.clone(),
            seller: seller.clone(),
            assets_to_buyer: vec![target.asset.clone()],
            assets_to_seller: vec![first_buyer_asset.clone()],
            buyer_cap_space_delta: None,
            seller_cap_space_delta: None,
            transaction_gates: TradeTransactionGates::default(),
        })?;
    }
    let first_target = &input.targets[0];
    let first_seller = input
        .sellers
        .iter()
        .find(|seller| seller.team == first_target.organization)
        .expect("target seller presence validated");
    for buyer_asset in &input.buyer_assets {
        validate_trade_package(&TradePackageInput {
            buyer: input.buyer.clone(),
            seller: first_seller.clone(),
            assets_to_buyer: vec![first_target.asset.clone()],
            assets_to_seller: vec![buyer_asset.asset.clone()],
            buyer_cap_space_delta: None,
            seller_cap_space_delta: None,
            transaction_gates: TradeTransactionGates::default(),
        })?;
    }
    Ok(())
}

pub fn closest_balancing_pick(
    market_value_gap: f64,
    candidates: &[DraftPickAssetValue],
) -> Option<&DraftPickAssetValue> {
    candidates.iter().min_by(|left, right| {
        (left.expected_value - market_value_gap.abs())
            .abs()
            .total_cmp(&(right.expected_value - market_value_gap.abs()).abs())
            .then_with(|| left.id.cmp(&right.id))
    })
}

fn validate_trade_package(input: &TradePackageInput) -> Result<(), DraftPickValueError> {
    let preferences = [&input.buyer, &input.seller];
    if input.buyer.team.trim().is_empty()
        || input.seller.team.trim().is_empty()
        || input.buyer.team == input.seller.team
        || input.assets_to_buyer.is_empty()
        || input.assets_to_seller.is_empty()
        || [input.buyer_cap_space_delta, input.seller_cap_space_delta]
            .into_iter()
            .flatten()
            .any(|value| !value.is_finite())
        || preferences.iter().any(|team| {
            [
                team.current_weight,
                team.future_weight,
                team.cap_weight,
                team.season_impact_weight,
            ]
            .into_iter()
            .any(|value| !value.is_finite() || value < 0.0)
                || team
                    .needs
                    .values()
                    .any(|value| !value.is_finite() || !(0.0..=100.0).contains(value))
        })
    {
        return Err(DraftPickValueError::InvalidDistribution(
            "trade requires distinct teams, assets in both directions, and non-negative team weights"
                .to_owned(),
        ));
    }
    let mut common_basis: Option<&TradeValueBasis> = None;
    for asset in input.assets_to_buyer.iter().chain(&input.assets_to_seller) {
        if asset.id.trim().is_empty()
            || asset.label.trim().is_empty()
            || asset.value_basis.outcome_measure.trim().is_empty()
            || asset.value_basis.horizon_years == 0
            || asset.value_basis.method.trim().is_empty()
            || [
                asset.market_value,
                asset.current_value,
                asset.future_value,
                asset.season_points_impact,
                asset.uncertainty,
                asset.availability.probability,
            ]
            .into_iter()
            .any(|value| !value.is_finite() || value < 0.0)
            || asset.availability.probability > 1.0
            || asset
                .disclosures
                .iter()
                .any(|disclosure| disclosure.trim().is_empty())
        {
            return Err(DraftPickValueError::InvalidDistribution(
                "trade asset values and 0-1 availability must be finite and non-negative"
                    .to_owned(),
            ));
        }
        if common_basis.is_some_and(|basis| basis != &asset.value_basis) {
            return Err(DraftPickValueError::InvalidDistribution(
                "trade assets cannot mix valuation measures, horizons, or methods".to_owned(),
            ));
        }
        common_basis.get_or_insert(&asset.value_basis);
        let sourced_kind = !matches!(
            asset.availability.kind,
            TradeAvailabilityKind::DepthSurplus | TradeAvailabilityKind::SpeculativeFit
        );
        if sourced_kind
            && !valid_execution_evidence(
                &asset.availability.source_url,
                &asset.availability.observed_at,
            )
        {
            return Err(DraftPickValueError::InvalidDistribution(format!(
                "asset {} requires a sourced URL and observation time for its availability state",
                asset.id
            )));
        }
    }
    Ok(())
}

fn common_market_value_basis(
    proposals: &[TradePackageInput],
) -> Result<TradeValueBasis, DraftPickValueError> {
    let mut basis: Option<&TradeValueBasis> = None;
    for asset in proposals.iter().flat_map(|proposal| {
        proposal
            .assets_to_buyer
            .iter()
            .chain(&proposal.assets_to_seller)
    }) {
        if basis.is_some_and(|existing| existing != &asset.value_basis) {
            return Err(DraftPickValueError::InvalidDistribution(
                "trade market proposals must share one valuation basis".to_owned(),
            ));
        }
        basis.get_or_insert(&asset.value_basis);
    }
    basis.cloned().ok_or_else(|| {
        DraftPickValueError::InvalidDistribution(
            "trade market requires a declared valuation basis".to_owned(),
        )
    })
}

fn package_market_value(assets: &[TradeAssetValueInput]) -> f64 {
    assets.iter().map(|asset| asset.market_value).sum()
}

fn package_season_points(assets: &[TradeAssetValueInput]) -> f64 {
    assets.iter().map(|asset| asset.season_points_impact).sum()
}

fn package_utility(assets: &[TradeAssetValueInput], team: &TradeTeamPreferenceInput) -> f64 {
    assets
        .iter()
        .map(|asset| {
            let need_bonus = asset
                .position
                .as_ref()
                .and_then(|position| team.needs.get(position))
                .copied()
                .unwrap_or(0.0)
                / 200.0;
            asset.current_value * team.current_weight * (1.0 + need_bonus)
                + asset.future_value * team.future_weight
                + asset.season_points_impact * team.season_impact_weight
        })
        .sum()
}

fn asset_feasibility(asset: &TradeAssetValueInput) -> f64 {
    if asset.availability.destination_allowed == Some(false)
        || asset.availability.kind == TradeAvailabilityKind::Unavailable
    {
        0.0
    } else {
        asset.availability.probability
    }
}

fn transaction_gates_ready(gates: &TradeTransactionGates, package_has_pick: bool) -> bool {
    gates.cap_compliant == Some(true)
        && gates.roster_compliant == Some(true)
        && gates.retention_compliant == Some(true)
        && gates.contract_authority_complete
        && (!package_has_pick || gates.pick_ownership_confirmed == Some(true))
}

fn package_pick_roles(
    assets: &[TradeAssetValueInput],
    direction: &str,
) -> Vec<TradePackagePickRoleView> {
    let package_value = package_market_value(assets);
    assets
        .iter()
        .filter(|asset| asset.draft_pick.is_some())
        .map(|asset| {
            let package_share = if package_value <= f64::EPSILON {
                0.0
            } else {
                asset.market_value / package_value
            };
            TradePackagePickRoleView {
                asset_id: asset.id.clone(),
                direction: direction.to_owned(),
                package_share,
                role: if package_share < 0.40 {
                    DraftPickPackageRole::Rounding
                } else {
                    DraftPickPackageRole::Principal
                },
            }
        })
        .collect()
}

#[derive(Debug, Clone)]
struct PickAggregate {
    count: usize,
    sum: f64,
    sum_squares: f64,
}

#[derive(Debug, Clone)]
struct MonotoneBlock {
    start: usize,
    end: usize,
    weight: usize,
    weighted_sum: f64,
}

impl MonotoneBlock {
    fn mean(&self) -> f64 {
        self.weighted_sum / self.weight as f64
    }
}

pub fn build_draft_pick_value_curve(
    observations: Vec<DraftPickOutcomeObservation>,
    config: DraftPickValueConfig,
) -> Result<DraftPickValueCurve, DraftPickValueError> {
    validate_curve_config(&config)?;
    let eligible = observations
        .into_iter()
        .filter(|row| {
            row.draft_year <= config.training_cutoff_year
                && row.observed_horizon_years >= config.outcome_horizon_years
        })
        .collect::<Vec<_>>();
    if eligible.is_empty() {
        return Err(DraftPickValueError::InvalidObservations(
            "no mature observation predates the training cutoff".to_owned(),
        ));
    }
    if eligible.iter().any(|row| {
        row.overall_pick == 0
            || row.overall_pick > config.max_overall_pick
            || !row.outcome_value.is_finite()
            || row.outcome_value < 0.0
    }) {
        return Err(DraftPickValueError::InvalidObservations(
            "pick must be in range and outcome must be finite and non-negative".to_owned(),
        ));
    }
    let mut by_pick = BTreeMap::<u16, PickAggregate>::new();
    for row in &eligible {
        let aggregate = by_pick.entry(row.overall_pick).or_insert(PickAggregate {
            count: 0,
            sum: 0.0,
            sum_squares: 0.0,
        });
        aggregate.count += 1;
        aggregate.sum += row.outcome_value;
        aggregate.sum_squares += row.outcome_value * row.outcome_value;
    }
    let missing = (1..=config.max_overall_pick)
        .filter(|pick| !by_pick.contains_key(pick))
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(DraftPickValueError::InvalidObservations(format!(
            "mature cohort has no observation for {} of {} slots (first missing: {})",
            missing.len(),
            config.max_overall_pick,
            missing[0]
        )));
    }
    let aggregates = (1..=config.max_overall_pick)
        .map(|pick| by_pick.get(&pick).expect("coverage checked").clone())
        .collect::<Vec<_>>();
    let monotone = decreasing_isotonic_means(&aggregates);
    let values = aggregates
        .iter()
        .zip(monotone)
        .enumerate()
        .map(|(index, (aggregate, expected_value))| {
            let variance = if aggregate.count > 1 {
                ((aggregate.sum_squares - aggregate.sum * aggregate.sum / aggregate.count as f64)
                    / (aggregate.count - 1) as f64)
                    .max(0.0)
            } else {
                0.0
            };
            let margin = 1.96 * (variance / aggregate.count as f64).sqrt();
            DraftPickValueRow {
                overall_pick: index as u16 + 1,
                expected_value,
                expected_value_low: (expected_value - margin).max(0.0),
                expected_value_high: expected_value + margin,
                observations: aggregate.count,
            }
        })
        .collect();
    Ok(DraftPickValueCurve {
        schema: DRAFT_PICK_VALUE_CURVE_SCHEMA.to_owned(),
        method: DRAFT_PICK_VALUE_METHOD.to_owned(),
        training_cutoff_year: config.training_cutoff_year,
        outcome_horizon_years: config.outcome_horizon_years,
        max_overall_pick: config.max_overall_pick,
        outcome_measure: config.outcome_measure,
        annual_future_discount: config.annual_future_discount,
        observations: eligible.len(),
        values,
        disclosures: vec![
            "Expected pick value does not predict the identity or ceiling of the eventual selection."
                .to_owned(),
            "Earlier picks are constrained to retain at least the expected value of later picks; uncertainty remains visible."
                .to_owned(),
        ],
    })
}

pub fn value_draft_pick_asset(
    curve: &DraftPickValueCurve,
    asset: &DraftPickAssetInput,
    current_draft_year: u16,
) -> Result<DraftPickAssetValue, DraftPickValueError> {
    if asset.id.trim().is_empty() || asset.draft_year < current_draft_year {
        return Err(DraftPickValueError::InvalidDistribution(
            "asset requires an ID and a current or future draft year".to_owned(),
        ));
    }
    if asset.slot_outcomes.is_empty()
        || asset.slot_outcomes.iter().any(|row| {
            row.overall_pick == 0
                || row.overall_pick > curve.max_overall_pick
                || !row.probability.is_finite()
                || row.probability < 0.0
        })
    {
        return Err(DraftPickValueError::InvalidDistribution(
            "slot outcomes must be non-empty, in range, and non-negative".to_owned(),
        ));
    }
    let probability = asset
        .slot_outcomes
        .iter()
        .map(|row| row.probability)
        .sum::<f64>();
    if (probability - 1.0).abs() > 1e-6 {
        return Err(DraftPickValueError::InvalidDistribution(format!(
            "slot probabilities sum to {probability:.6}, expected 1"
        )));
    }
    let years_away = asset.draft_year - current_draft_year;
    let future_discount = curve.annual_future_discount.powi(i32::from(years_away));
    let mut expected_pick = 0.0;
    let mut expected_value = 0.0;
    let mut expected_low = 0.0;
    let mut expected_high = 0.0;
    for outcome in &asset.slot_outcomes {
        let row = &curve.values[usize::from(outcome.overall_pick - 1)];
        expected_pick += f64::from(outcome.overall_pick) * outcome.probability;
        expected_value += row.expected_value * outcome.probability;
        expected_low += row.expected_value_low * outcome.probability;
        expected_high += row.expected_value_high * outcome.probability;
    }
    expected_value *= future_discount;
    expected_low *= future_discount;
    expected_high *= future_discount;
    Ok(DraftPickAssetValue {
        id: asset.id.clone(),
        draft_year: asset.draft_year,
        expected_overall_pick: expected_pick,
        expected_value,
        expected_value_low: expected_low,
        expected_value_high: expected_high,
        future_discount,
        uncertainty_width: expected_high - expected_low,
    })
}

fn validate_curve_config(config: &DraftPickValueConfig) -> Result<(), DraftPickValueError> {
    if config.training_cutoff_year < 1970
        || config.outcome_horizon_years == 0
        || config.max_overall_pick == 0
        || config.outcome_measure.trim().is_empty()
        || !config.annual_future_discount.is_finite()
        || !(0.0..=1.0).contains(&config.annual_future_discount)
    {
        return Err(DraftPickValueError::InvalidConfig(
            "cutoff, horizon, draft size, measure, and 0-1 discount are required".to_owned(),
        ));
    }
    Ok(())
}

fn decreasing_isotonic_means(aggregates: &[PickAggregate]) -> Vec<f64> {
    let mut blocks = Vec::<MonotoneBlock>::new();
    for (index, aggregate) in aggregates.iter().enumerate() {
        blocks.push(MonotoneBlock {
            start: index,
            end: index,
            weight: aggregate.count,
            weighted_sum: aggregate.sum,
        });
        while blocks.len() >= 2 {
            let right = blocks.len() - 1;
            let left = right - 1;
            if blocks[left].mean() >= blocks[right].mean() {
                break;
            }
            let latter = blocks.pop().expect("right block exists");
            let former = blocks.pop().expect("left block exists");
            blocks.push(MonotoneBlock {
                start: former.start,
                end: latter.end,
                weight: former.weight + latter.weight,
                weighted_sum: former.weighted_sum + latter.weighted_sum,
            });
        }
    }
    let mut output = vec![0.0; aggregates.len()];
    for block in blocks {
        output[block.start..=block.end].fill(block.mean());
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(year: u16, pick: u16, value: f64) -> DraftPickOutcomeObservation {
        DraftPickOutcomeObservation {
            draft_year: year,
            overall_pick: pick,
            outcome_value: value,
            observed_horizon_years: 7,
        }
    }

    fn curve() -> DraftPickValueCurve {
        build_draft_pick_value_curve(
            vec![
                observation(2010, 1, 100.0),
                observation(2011, 1, 80.0),
                observation(2010, 2, 60.0),
                observation(2011, 2, 80.0),
                observation(2010, 3, 65.0),
                observation(2011, 3, 75.0),
                observation(2010, 4, 20.0),
                observation(2011, 4, 30.0),
            ],
            DraftPickValueConfig {
                training_cutoff_year: 2018,
                outcome_horizon_years: 7,
                max_overall_pick: 4,
                outcome_measure: "seven-year IceLines value".to_owned(),
                annual_future_discount: 0.90,
            },
        )
        .unwrap()
    }

    fn player_asset(
        id: &str,
        position: &str,
        market: f64,
        current: f64,
        future: f64,
    ) -> TradeAssetValueInput {
        TradeAssetValueInput {
            id: id.to_owned(),
            label: id.to_owned(),
            position: Some(position.to_owned()),
            value_basis: TradeValueBasis {
                outcome_measure: "seven-year IceLines value".to_owned(),
                horizon_years: 7,
                method: DRAFT_PICK_VALUE_METHOD.to_owned(),
            },
            market_value: market,
            current_value: current,
            future_value: future,
            season_points_impact: current / 100.0,
            uncertainty: 5.0,
            availability: TradeAvailabilityEvidence {
                kind: TradeAvailabilityKind::SpeculativeFit,
                probability: 0.5,
                source_url: None,
                observed_at: None,
                destination_allowed: None,
            },
            draft_pick: None,
            disclosures: vec!["Test fixture player valuation.".to_owned()],
        }
    }

    fn preference(team: &str, current: f64, future: f64) -> TradeTeamPreferenceInput {
        TradeTeamPreferenceInput {
            team: team.to_owned(),
            current_weight: current,
            future_weight: future,
            cap_weight: 0.0,
            season_impact_weight: 1.0,
            needs: BTreeMap::from([("top_six_forward".to_owned(), 100.0)]),
        }
    }

    fn offer_asset(id: &str, market: f64) -> TradeAssetValueInput {
        let mut asset = player_asset(id, "future_asset", market, 0.0, market);
        asset.availability = TradeAvailabilityEvidence {
            kind: TradeAvailabilityKind::DepthSurplus,
            probability: 1.0,
            source_url: None,
            observed_at: None,
            destination_allowed: Some(true),
        };
        asset
    }

    #[test]
    fn automatic_scout_builds_a_bounded_ladder_and_excludes_protected_assets() {
        let mut target = player_asset("impact-wing", "top_six_forward", 100.0, 85.0, 15.0);
        target.availability.probability = 0.2;
        let view = build_trade_scout(TradeScoutInput {
            as_of: "2026-08-03".to_owned(),
            buyer: preference("SEA", 1.4, 0.5),
            sellers: vec![preference("PIT", 0.5, 1.4)],
            targets: vec![TradeScoutAssetInput {
                organization: "PIT".to_owned(),
                asset: target,
                protected: false,
            }],
            buyer_assets: vec![
                TradeScoutAssetInput {
                    organization: "SEA".to_owned(),
                    asset: offer_asset("second-round-pick", 75.0),
                    protected: false,
                },
                TradeScoutAssetInput {
                    organization: "SEA".to_owned(),
                    asset: offer_asset("secondary-prospect", 15.0),
                    protected: false,
                },
                TradeScoutAssetInput {
                    organization: "SEA".to_owned(),
                    asset: offer_asset("depth-prospect", 25.0),
                    protected: false,
                },
                TradeScoutAssetInput {
                    organization: "SEA".to_owned(),
                    asset: offer_asset("Berkly Catton", 150.0),
                    protected: true,
                },
            ],
            config: TradeScoutConfig {
                maximum_assets_per_package: 3,
                ..TradeScoutConfig::default()
            },
        })
        .unwrap();

        assert_eq!(view.schema, TRADE_SCOUT_SCHEMA);
        assert_eq!(view.candidates.len(), 1);
        let candidate = &view.candidates[0];
        assert_eq!(candidate.rank, 1);
        assert_eq!(candidate.negotiation.opening_offer.market_value, 90.0);
        assert_eq!(candidate.negotiation.fair_midpoint.market_value, 100.0);
        assert_eq!(candidate.negotiation.maximum_acceptable.market_value, 115.0);
        assert_eq!(
            candidate.negotiation.protected_buyer_assets,
            vec!["Berkly Catton"]
        );
        assert!(
            !candidate
                .negotiation
                .maximum_acceptable
                .evaluation
                .transaction_ready
        );
        assert!(candidate
            .negotiation
            .maximum_acceptable
            .assets_to_seller
            .iter()
            .all(|asset| asset != "Berkly Catton"));
    }

    #[test]
    fn sourced_availability_requires_an_observation_time() {
        let mut target = player_asset("reported-target", "top_six_forward", 80.0, 60.0, 20.0);
        target.availability = TradeAvailabilityEvidence {
            kind: TradeAvailabilityKind::ClubShopping,
            probability: 0.6,
            source_url: Some("https://example.test/report".to_owned()),
            observed_at: None,
            destination_allowed: None,
        };
        let error = evaluate_trade_package(TradePackageInput {
            buyer: preference("SEA", 1.4, 0.5),
            seller: preference("PIT", 0.5, 1.4),
            assets_to_buyer: vec![target],
            assets_to_seller: vec![offer_asset("future", 80.0)],
            buyer_cap_space_delta: None,
            seller_cap_space_delta: None,
            transaction_gates: TradeTransactionGates::default(),
        })
        .unwrap_err();

        assert!(error.to_string().contains("observation time"));
    }

    #[test]
    fn league_inventory_derives_only_need_matched_surplus_targets() {
        let mut rust = player_asset("rust", "top_six_forward", 100.0, 85.0, 15.0);
        rust.availability.probability = 0.2;
        let defense = player_asset("defense", "defense", 90.0, 75.0, 15.0);
        let view = build_trade_scout_from_league(TradeScoutLeagueInput {
            as_of: "2026-08-03".to_owned(),
            buyer: "SEA".to_owned(),
            organizations: vec![
                TradeScoutLeagueOrganizationInput {
                    preference: preference("SEA", 1.4, 0.5),
                    assets: vec![
                        TradeScoutLeagueAssetInput {
                            kind: TradeScoutLeagueAssetKind::DraftPick,
                            asset: offer_asset("second", 80.0),
                            surplus_score: 100.0,
                            protected: false,
                        },
                        TradeScoutLeagueAssetInput {
                            kind: TradeScoutLeagueAssetKind::Prospect,
                            asset: offer_asset("prospect", 25.0),
                            surplus_score: 40.0,
                            protected: false,
                        },
                    ],
                },
                TradeScoutLeagueOrganizationInput {
                    preference: preference("PIT", 0.5, 1.4),
                    assets: vec![
                        TradeScoutLeagueAssetInput {
                            kind: TradeScoutLeagueAssetKind::NhlPlayer,
                            asset: rust,
                            surplus_score: 75.0,
                            protected: false,
                        },
                        TradeScoutLeagueAssetInput {
                            kind: TradeScoutLeagueAssetKind::NhlPlayer,
                            asset: defense,
                            surplus_score: 90.0,
                            protected: false,
                        },
                    ],
                },
            ],
            config: TradeScoutLeagueConfig {
                expected_organizations: 32,
                allow_partial_inventory: true,
                ..TradeScoutLeagueConfig::default()
            },
        })
        .unwrap();

        assert_eq!(view.schema, TRADE_SCOUT_LEAGUE_SCHEMA);
        assert!(!view.inventory_complete);
        assert_eq!(view.derived_targets, 1);
        assert_eq!(view.derived_buyer_assets, 2);
        assert_eq!(view.scout.candidates[0].target_id, "rust");
    }

    fn camp_player(
        player_id: u32,
        name: &str,
        prospect: bool,
        incumbent: bool,
        projected_score: f64,
        cut_probability: f64,
    ) -> TrainingCampPlayerView {
        TrainingCampPlayerView {
            player_id,
            display_name: name.to_owned(),
            primary_position: Position::RightWing,
            eligible_positions: vec![Position::RightWing],
            source_league: if prospect { "AHL" } else { "NHL" }.to_owned(),
            incumbent,
            rookie_eligible: prospect,
            prospect,
            pre_camp_make_probability: Some(if incumbent { 0.9 } else { 0.4 }),
            pre_camp_track: super::super::training_camp::TrainingCampPreCampTrack::InsideTrack,
            roster_prior_delta: 0.0,
            minimum_forward_role: None,
            waiver_exempt: prospect,
            cap_hit: None,
            cap_hit_source: None,
            projected_score,
            gp_confidence: 0.8,
            camp_mean: projected_score,
            management_behavior_delta: 0.0,
            average_sampled_camp_score: projected_score,
            make_probability: if incumbent { 0.9 } else { 0.4 },
            cut_probability,
            unavailable_probability: 0.0,
            selection_loss_probability: if incumbent { 0.1 } else { 0.6 },
            dressed_probability: if incumbent { 0.8 } else { 0.3 },
            healthy_scratch_probability: 0.0,
            waiver_exposure_probability: 0.0,
            status: super::super::training_camp::TrainingCampRosterStatus::InsideTrack,
            displaced_incumbents: Vec::new(),
            evidence_label: crate::view_model::EvidenceLabel::Estimated,
        }
    }

    fn camp_team(
        team: &str,
        players: Vec<TrainingCampPlayerView>,
    ) -> super::super::training_camp::TrainingCampLeagueTeamView {
        super::super::training_camp::TrainingCampLeagueTeamView {
            team: team.to_owned(),
            authority_status:
                super::super::training_camp::TrainingCampAuthorityStatus::ConfirmedPool,
            competition_pool_status:
                super::super::training_camp::TrainingCampCompetitionPoolStatus::Authored,
            current_roster_candidates: players.len(),
            sourced_overlay_candidates: 0,
            fallback_candidates: 0,
            forecast: Some(super::super::training_camp::TrainingCampForecastView {
                schema: super::super::training_camp::TRAINING_CAMP_FORECAST_SCHEMA.to_owned(),
                method: "test".to_owned(),
                team: team.to_owned(),
                season: 20262027,
                trials: 10,
                seed: 1,
                decision_profile_id: None,
                valid_trials: 10,
                incomplete_trials: 0,
                roster_shape: "test".to_owned(),
                opening_roster_size: 1,
                dressed_roster_size: 1,
                salary_cap_upper_limit: None,
                salary_cap_status: Default::default(),
                players,
                most_common_rosters: Vec::new(),
                modal_opening_roster_ids: Vec::new(),
                warnings: Vec::new(),
                disclosures: Vec::new(),
            }),
            error: None,
            authority_warnings: Vec::new(),
        }
    }

    #[test]
    fn camp_population_reuses_roster_and_prospect_authority() {
        let camp = TrainingCampLeagueForecastView {
            schema: TRAINING_CAMP_LEAGUE_FORECAST_SCHEMA.to_owned(),
            season: 20262027,
            teams_requested: 2,
            teams_simulated: 2,
            teams_degraded: 0,
            teams_augmented: 0,
            teams_failed: 0,
            teams: vec![
                camp_team(
                    "SEA",
                    vec![
                        camp_player(1, "Prospect", true, false, 60.0, 0.6),
                        camp_player(3, "Elite scorer", true, false, 130.0, 0.7),
                    ],
                ),
                camp_team(
                    "PIT",
                    vec![camp_player(2, "Veteran", false, true, 60.0, 0.7)],
                ),
            ],
            disclosures: Vec::new(),
        };
        let population = populate_trade_scout_league_from_camp(
            &camp,
            TradeScoutPopulationInput {
                as_of: "2026-08-03".to_owned(),
                buyer: "SEA".to_owned(),
                preferences: vec![preference("SEA", 1.4, 0.5), preference("PIT", 0.5, 1.4)],
                availability: Vec::new(),
                protected_player_ids: Vec::new(),
                draft_pick_assets: Vec::new(),
                config: TradeScoutPopulationConfig {
                    value_basis: player_asset("basis", "x", 1.0, 1.0, 0.0).value_basis,
                    control_value_per_score: 1.5,
                    season_points_per_score: 0.01,
                    top_six_forward_score: 45.0,
                    top_four_defense_score: 45.0,
                    starting_goalie_score: 50.0,
                    league: TradeScoutLeagueConfig {
                        expected_organizations: 2,
                        ..TradeScoutLeagueConfig::default()
                    },
                },
            },
        )
        .unwrap();
        let scout = build_trade_scout_from_league(population.league_input.clone()).unwrap();

        assert_eq!(population.schema, TRADE_SCOUT_POPULATION_SCHEMA);
        assert_eq!(population.players_populated, 3);
        assert_eq!(population.prospects_populated, 2);
        assert_eq!(scout.scout.candidates[0].label, "Veteran");
        assert_eq!(
            population.league_input.organizations[0].assets[0].kind,
            TradeScoutLeagueAssetKind::Prospect
        );
    }

    #[test]
    fn camp_population_rejects_undated_sourced_availability() {
        let camp = TrainingCampLeagueForecastView {
            schema: TRAINING_CAMP_LEAGUE_FORECAST_SCHEMA.to_owned(),
            season: 20262027,
            teams_requested: 1,
            teams_simulated: 1,
            teams_degraded: 0,
            teams_augmented: 0,
            teams_failed: 0,
            teams: vec![camp_team(
                "SEA",
                vec![camp_player(1, "Prospect", true, false, 60.0, 0.6)],
            )],
            disclosures: Vec::new(),
        };
        let error = populate_trade_scout_league_from_camp(
            &camp,
            TradeScoutPopulationInput {
                as_of: "2026-08-03".to_owned(),
                buyer: "SEA".to_owned(),
                preferences: vec![preference("SEA", 1.4, 0.5)],
                availability: vec![TradeScoutAvailabilityOverlayInput {
                    player_id: 1,
                    evidence: TradeAvailabilityEvidence {
                        kind: TradeAvailabilityKind::ClubShopping,
                        probability: 0.8,
                        source_url: None,
                        observed_at: None,
                        destination_allowed: None,
                    },
                    surplus_score: None,
                }],
                protected_player_ids: Vec::new(),
                draft_pick_assets: Vec::new(),
                config: TradeScoutPopulationConfig {
                    value_basis: player_asset("basis", "x", 1.0, 1.0, 0.0).value_basis,
                    control_value_per_score: 1.5,
                    season_points_per_score: 0.01,
                    top_six_forward_score: 45.0,
                    top_four_defense_score: 45.0,
                    starting_goalie_score: 50.0,
                    league: TradeScoutLeagueConfig::default(),
                },
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("dated sourced availability"));
    }

    #[test]
    fn draft_pick_population_values_only_confirmed_unconditional_rights() {
        let view = populate_trade_scout_draft_picks(
            &curve(),
            TradeScoutDraftPickPopulationInput {
                as_of: "2026-08-03".to_owned(),
                current_draft_year: 2027,
                value_basis: TradeValueBasis {
                    outcome_measure: "nhl_regular_season_games_played".to_owned(),
                    horizon_years: 7,
                    method: "camp-score and pick-curve bridge v1".to_owned(),
                },
                ownership: vec![
                    TradeDraftPickOwnershipInput {
                        asset_id: "TBL-2027-1".to_owned(),
                        owner: "SEA".to_owned(),
                        original_team: "TBL".to_owned(),
                        draft_year: 2027,
                        round: 1,
                        status: TradeDraftPickOwnershipStatus::ConfirmedUnconditional,
                        conditions: None,
                        source_url: "https://example.test/tbl-pick".to_owned(),
                        observed_at: "2026-08-03T12:00:00Z".to_owned(),
                    },
                    TradeDraftPickOwnershipInput {
                        asset_id: "CBJ-WPG-2027-2-lower".to_owned(),
                        owner: "SEA".to_owned(),
                        original_team: "CBJ".to_owned(),
                        draft_year: 2027,
                        round: 2,
                        status: TradeDraftPickOwnershipStatus::Conditional,
                        conditions: Some("SEA retains the lower selection".to_owned()),
                        source_url: "https://example.test/conditional-pick".to_owned(),
                        observed_at: "2026-08-03T12:00:00Z".to_owned(),
                    },
                ],
                protected_asset_ids: vec!["TBL-2027-1".to_owned()],
            },
        )
        .unwrap();

        assert_eq!(view.schema, TRADE_SCOUT_DRAFT_PICK_POPULATION_SCHEMA);
        assert_eq!(view.picks_supplied, 2);
        assert_eq!(view.picks_populated, 1);
        assert_eq!(view.unresolved_asset_ids, ["CBJ-WPG-2027-2-lower"]);
        assert!(view.assets[0].protected);
        assert_eq!(
            view.assets[0].asset.draft_pick.as_ref().unwrap().draft_year,
            2027
        );
        assert_eq!(
            view.assets[0].asset.value_basis.method,
            "camp-score and pick-curve bridge v1"
        );
    }

    fn cleared_gates(has_pick: bool) -> TradeTransactionGates {
        TradeTransactionGates {
            cap_compliant: Some(true),
            roster_compliant: Some(true),
            retention_compliant: Some(true),
            contract_authority_complete: true,
            pick_ownership_confirmed: has_pick.then_some(true),
        }
    }

    fn player_authority(
        player_id: u32,
        organization: &str,
        cap_hit: Option<u64>,
    ) -> TradePlayerExecutionAuthority {
        TradePlayerExecutionAuthority {
            player_id,
            organization: organization.to_owned(),
            cap_hit,
            contract_confirmed: true,
            clause_reviewed: true,
            destination_allowed: Some(true),
            source_url: Some(format!("https://example.test/contracts/{player_id}")),
            observed_at: Some("2026-08-02T12:00:00Z".to_owned()),
        }
    }

    fn season_impact(
        team: &str,
        points: f64,
        playoffs: f64,
        cup: f64,
    ) -> TeamSeasonScenarioImpactRow {
        TeamSeasonScenarioImpactRow {
            team: team.to_owned(),
            average_points_delta: points,
            playoff_probability_delta: playoffs,
            second_round_probability_delta: 0.0,
            conference_final_probability_delta: 0.0,
            stanley_cup_final_probability_delta: 0.0,
            stanley_cup_probability_delta: cup,
            presidents_trophy_probability_delta: 0.0,
            average_longest_win_streak_delta: 0.0,
        }
    }

    fn lineup_player(
        player_id: u32,
        label: &str,
        position: TradeLineupPosition,
        score: f64,
    ) -> TradeLineupPlayerInput {
        TradeLineupPlayerInput {
            player_id,
            label: label.to_owned(),
            natural_positions: vec![position],
            alternate_positions: vec![],
            projected_score: score,
            alternate_position_penalty: 0.0,
        }
    }

    #[test]
    fn l0_pick_curve_is_monotone_and_uses_only_mature_prior_cohorts() {
        let curve = curve();
        assert_eq!(curve.observations, 8);
        assert!(curve
            .values
            .windows(2)
            .all(|rows| rows[0].expected_value >= rows[1].expected_value));
        assert_eq!(curve.values[1].expected_value, 70.0);
        assert_eq!(curve.values[2].expected_value, 70.0);
    }

    #[test]
    fn l0_pick_asset_integrates_slot_risk_and_future_discount() {
        let value = value_draft_pick_asset(
            &curve(),
            &DraftPickAssetInput {
                id: "NYR-2027-1".to_owned(),
                draft_year: 2027,
                slot_outcomes: vec![
                    DraftPickSlotOutcome {
                        overall_pick: 1,
                        probability: 0.25,
                    },
                    DraftPickSlotOutcome {
                        overall_pick: 4,
                        probability: 0.75,
                    },
                ],
            },
            2026,
        )
        .unwrap();
        assert_eq!(value.expected_overall_pick, 3.25);
        assert!((value.expected_value - 37.125).abs() < 1e-9);
        assert_eq!(value.future_discount, 0.9);
    }

    #[test]
    fn l0_pick_curve_refuses_leaky_training_data() {
        let error = build_draft_pick_value_curve(
            vec![observation(2025, 1, 100.0)],
            DraftPickValueConfig {
                training_cutoff_year: 2020,
                outcome_horizon_years: 7,
                max_overall_pick: 1,
                outcome_measure: "GP".to_owned(),
                annual_future_discount: 0.9,
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("no mature observation"));
    }

    #[test]
    fn l0_trade_package_can_create_distinct_positive_team_utilities() {
        let pick = value_draft_pick_asset(
            &curve(),
            &DraftPickAssetInput {
                id: "NYR-2026-1".to_owned(),
                draft_year: 2026,
                slot_outcomes: vec![DraftPickSlotOutcome {
                    overall_pick: 2,
                    probability: 1.0,
                }],
            },
            2026,
        )
        .unwrap();
        let view = evaluate_trade_package(TradePackageInput {
            buyer: preference("NYR", 1.4, 0.4),
            seller: preference("DET", 0.4, 1.4),
            assets_to_buyer: vec![player_asset(
                "top-six-forward",
                "top_six_forward",
                70.0,
                70.0,
                15.0,
            )],
            assets_to_seller: vec![draft_pick_trade_asset(pick, &curve())],
            buyer_cap_space_delta: Some(0.0),
            seller_cap_space_delta: Some(0.0),
            transaction_gates: cleared_gates(true),
        })
        .unwrap();
        assert!(view.buyer_utility_delta > 0.0);
        assert!(view.seller_utility_delta > 0.0);
        assert!(view.mutually_beneficial);
        assert_eq!(view.pick_roles[0].role, DraftPickPackageRole::Principal);
    }

    #[test]
    fn l0_destination_control_blocks_even_an_available_player() {
        let mut player = player_asset("requested-player", "top_six_forward", 70.0, 70.0, 15.0);
        player.availability = TradeAvailabilityEvidence {
            kind: TradeAvailabilityKind::ReportedRequest,
            probability: 0.8,
            source_url: Some("https://example.test/report".to_owned()),
            observed_at: Some("2026-06-04T12:00:00Z".to_owned()),
            destination_allowed: Some(false),
        };
        let view = evaluate_trade_package(TradePackageInput {
            buyer: preference("NYR", 1.4, 0.4),
            seller: preference("DET", 0.4, 1.4),
            assets_to_buyer: vec![player],
            assets_to_seller: vec![player_asset("young-player", "prospect", 70.0, 5.0, 70.0)],
            buyer_cap_space_delta: Some(0.0),
            seller_cap_space_delta: Some(0.0),
            transaction_gates: cleared_gates(false),
        })
        .unwrap();
        assert_eq!(view.feasibility_probability, 0.0);
        assert!(!view.mutually_beneficial);
    }

    #[test]
    fn l0_missing_transaction_authority_blocks_ready_and_mutual_labels() {
        let view = evaluate_trade_package(TradePackageInput {
            buyer: preference("NYR", 1.4, 0.4),
            seller: preference("SJS", 0.4, 1.4),
            assets_to_buyer: vec![player_asset(
                "top-six-forward",
                "top_six_forward",
                70.0,
                70.0,
                15.0,
            )],
            assets_to_seller: vec![player_asset("prospect", "prospect", 70.0, 5.0, 70.0)],
            buyer_cap_space_delta: None,
            seller_cap_space_delta: None,
            transaction_gates: TradeTransactionGates::default(),
        })
        .unwrap();
        assert!(view.buyer_utility_delta > 0.0);
        assert!(view.seller_utility_delta > 0.0);
        assert!(!view.transaction_ready);
        assert_eq!(view.feasibility_probability, 0.0);
        assert!(!view.mutually_beneficial);
    }

    #[test]
    fn l0_balancing_pick_uses_expected_value_not_round_label() {
        let values = [
            DraftPickAssetValue {
                id: "late-first".to_owned(),
                draft_year: 2026,
                expected_overall_pick: 28.0,
                expected_value: 25.0,
                expected_value_low: 10.0,
                expected_value_high: 40.0,
                future_discount: 1.0,
                uncertainty_width: 30.0,
            },
            DraftPickAssetValue {
                id: "early-second".to_owned(),
                draft_year: 2026,
                expected_overall_pick: 34.0,
                expected_value: 23.0,
                expected_value_low: 12.0,
                expected_value_high: 34.0,
                future_discount: 1.0,
                uncertainty_width: 22.0,
            },
        ];
        assert_eq!(
            closest_balancing_pick(22.5, &values).unwrap().id,
            "early-second"
        );
    }

    #[test]
    fn l0_market_ranks_feasible_mutual_package_before_blocked_fit() {
        let package = |seller: &str, destination_allowed| {
            let mut target = player_asset(seller, "top_six_forward", 70.0, 70.0, 15.0);
            target.availability.destination_allowed = destination_allowed;
            TradePackageInput {
                buyer: preference("NYR", 1.4, 0.4),
                seller: preference(seller, 0.4, 1.4),
                assets_to_buyer: vec![target],
                assets_to_seller: vec![player_asset(
                    &format!("{seller}-prospect"),
                    "prospect",
                    70.0,
                    5.0,
                    70.0,
                )],
                buyer_cap_space_delta: Some(0.0),
                seller_cap_space_delta: Some(0.0),
                transaction_gates: cleared_gates(false),
            }
        };
        let view = evaluate_trade_market(TradeMarketInput {
            as_of: "2026-08-02".to_owned(),
            proposals: vec![package("DET", Some(false)), package("NSH", Some(true))],
        })
        .unwrap();
        assert_eq!(view.schema, TRADE_MARKET_EVALUATION_SCHEMA);
        assert_eq!(view.proposals[0].seller, "NSH");
        assert!(view.proposals[0].mutually_beneficial);
        assert_eq!(view.proposals[1].feasibility_probability, 0.0);
    }

    #[test]
    fn l0_package_rejects_mixed_player_and_pick_value_units() {
        let mut prospect = player_asset("prospect", "prospect", 70.0, 5.0, 70.0);
        prospect.value_basis.outcome_measure = "seven-year NHL games played".to_owned();
        let error = evaluate_trade_package(TradePackageInput {
            buyer: preference("NYR", 1.4, 0.4),
            seller: preference("SJS", 0.4, 1.4),
            assets_to_buyer: vec![player_asset(
                "top-six-forward",
                "top_six_forward",
                70.0,
                70.0,
                15.0,
            )],
            assets_to_seller: vec![prospect],
            buyer_cap_space_delta: Some(0.0),
            seller_cap_space_delta: Some(0.0),
            transaction_gates: cleared_gates(false),
        })
        .unwrap_err();
        assert!(error.to_string().contains("cannot mix valuation"));
    }

    #[test]
    fn l0_market_rejects_individually_valid_proposals_with_different_bases() {
        let package = |seller: &str, measure: &str| {
            let mut target = player_asset(seller, "top_six_forward", 70.0, 70.0, 15.0);
            let mut prospect = player_asset("prospect", "prospect", 70.0, 5.0, 70.0);
            target.value_basis.outcome_measure = measure.to_owned();
            prospect.value_basis.outcome_measure = measure.to_owned();
            TradePackageInput {
                buyer: preference("NYR", 1.4, 0.4),
                seller: preference(seller, 0.4, 1.4),
                assets_to_buyer: vec![target],
                assets_to_seller: vec![prospect],
                buyer_cap_space_delta: Some(0.0),
                seller_cap_space_delta: Some(0.0),
                transaction_gates: cleared_gates(false),
            }
        };
        let error = evaluate_trade_market(TradeMarketInput {
            as_of: "2026-08-02".to_owned(),
            proposals: vec![
                package("SJS", "seven-year IceLines value"),
                package("NSH", "seven-year NHL games played"),
            ],
        })
        .unwrap_err();
        assert!(error.to_string().contains("share one valuation basis"));
    }

    #[test]
    fn l0_paired_season_impacts_attach_to_exact_trade_package() {
        let mut market = evaluate_trade_market(TradeMarketInput {
            as_of: "2026-08-02".to_owned(),
            proposals: vec![TradePackageInput {
                buyer: preference("NYR", 1.4, 0.4),
                seller: preference("NSH", 0.4, 1.4),
                assets_to_buyer: vec![player_asset("target", "top_six_forward", 70.0, 70.0, 15.0)],
                assets_to_seller: vec![player_asset("prospect", "prospect", 70.0, 5.0, 70.0)],
                buyer_cap_space_delta: Some(-2.0),
                seller_cap_space_delta: Some(2.0),
                transaction_gates: cleared_gates(false),
            }],
        })
        .unwrap();
        attach_trade_package_season_impacts(
            &mut market,
            "NYR",
            "NSH",
            &[
                season_impact("NYR", 0.4, 0.02, 0.003),
                season_impact("NSH", -0.7, -0.03, -0.001),
            ],
        )
        .unwrap();
        let impact = market.proposals[0].season_forecast_impact.as_ref().unwrap();
        assert_eq!(impact.buyer.average_points_delta, 0.4);
        assert_eq!(impact.seller.average_points_delta, -0.7);
        assert!((impact.buyer_points_residual_vs_isolated + 0.25).abs() < 1e-9);
        assert!(market.proposals[0]
            .disclosures
            .iter()
            .any(|note| note.contains("paired baseline/scenario")));
    }

    #[test]
    fn l0_lineup_optimizer_finds_off_side_upgrade_and_displaced_player() {
        let baseline_roster = vec![
            lineup_player(1, "Center", TradeLineupPosition::Center, 70.0),
            lineup_player(2, "Left wing", TradeLineupPosition::LeftWing, 90.0),
            lineup_player(3, "Right wing", TradeLineupPosition::RightWing, 40.0),
            lineup_player(4, "Defense", TradeLineupPosition::Defense, 75.0),
            lineup_player(5, "Goalie", TradeLineupPosition::Goalie, 80.0),
        ];
        let mut incoming = lineup_player(6, "Flexible scorer", TradeLineupPosition::LeftWing, 70.0);
        incoming.alternate_positions = vec![TradeLineupPosition::RightWing];
        incoming.alternate_position_penalty = 5.0;
        let view = build_trade_lineup_scenario(TradeLineupScenarioInput {
            team: "NYR".to_owned(),
            baseline_roster,
            incoming_players: vec![incoming],
            outgoing_player_ids: vec![],
            limits: TradeLineupLimits {
                centers: 1,
                left_wings: 1,
                right_wings: 1,
                defense: 1,
                goalies: 1,
            },
        })
        .unwrap();
        assert_eq!(view.added_to_lineup, vec!["Flexible scorer"]);
        assert_eq!(view.displaced_by_competition, vec!["Right wing"]);
        assert!(view.explicitly_removed_from_lineup.is_empty());
        assert_eq!(view.strength_delta, 25.0);
        let assignment = view.after.iter().find(|row| row.player_id == 6).unwrap();
        assert_eq!(assignment.position, TradeLineupPosition::RightWing);
        assert_eq!(
            assignment.assignment_kind,
            TradeLineupAssignmentKind::Alternate
        );
        assert_eq!(assignment.effective_score, 65.0);
    }

    #[test]
    fn l0_lineup_optimizer_separates_trade_removal_from_competition() {
        let view = build_trade_lineup_scenario(TradeLineupScenarioInput {
            team: "NSH".to_owned(),
            baseline_roster: vec![
                lineup_player(1, "Center", TradeLineupPosition::Center, 70.0),
                lineup_player(2, "Left wing", TradeLineupPosition::LeftWing, 60.0),
                lineup_player(3, "Traded wing", TradeLineupPosition::RightWing, 50.0),
                lineup_player(4, "Defense", TradeLineupPosition::Defense, 75.0),
                lineup_player(5, "Goalie", TradeLineupPosition::Goalie, 80.0),
            ],
            incoming_players: vec![],
            outgoing_player_ids: vec![3],
            limits: TradeLineupLimits {
                centers: 1,
                left_wings: 1,
                right_wings: 1,
                defense: 1,
                goalies: 1,
            },
        })
        .unwrap();
        assert_eq!(view.explicitly_removed_from_lineup, vec!["Traded wing"]);
        assert!(view.displaced_by_competition.is_empty());
        assert_eq!(view.strength_delta, -50.0);
    }

    #[test]
    fn l0_projection_adapter_finds_rangers_center_displaced_by_oreilly() {
        let projection: TeamLineupProjectionView = serde_json::from_str(include_str!(
            "../../../examples/team-lineup-nyr-2026-27.json"
        ))
        .unwrap();
        let mut oreilly = lineup_player(
            8_475_158,
            "Ryan O'Reilly",
            TradeLineupPosition::Center,
            60.0,
        );
        oreilly.alternate_positions = vec![
            TradeLineupPosition::LeftWing,
            TradeLineupPosition::RightWing,
        ];
        oreilly.alternate_position_penalty = 3.0;
        let projection_oreilly = TeamLineupPlayerInput {
            player_id: 8_475_158,
            display_name: "Ryan O'Reilly".to_owned(),
            team: "NYR".to_owned(),
            prior_team: Some("NSH".to_owned()),
            primary_position: Position::Center,
            eligible_positions: vec![Position::LeftWing, Position::Center, Position::RightWing],
            headshot_canonical_url: None,
            games_played: 81,
            lens_scores: BTreeMap::from([
                (crate::view_model::TeamCeilingLens::PointsPace, Some(72.0)),
                (crate::view_model::TeamCeilingLens::GoalScoring, Some(36.0)),
                (crate::view_model::TeamCeilingLens::Fantasy, Some(270.0)),
                (crate::view_model::TeamCeilingLens::Upside, Some(72.0)),
            ]),
            score_evidence: crate::view_model::EvidenceLabel::Simulated,
            power_play_role_score: Some(161.0),
            penalty_kill_role_score: Some(109.0),
            special_teams_evidence: Some(crate::view_model::EvidenceLabel::Reported),
            requested_slot: None,
            assignment_evidence: crate::view_model::LineupAssignmentEvidence::Scenario,
        };
        let view = build_trade_lineup_scenario_from_projection(
            &projection,
            TradeLineupProjectionChangeInput {
                incoming_players: vec![oreilly],
                projection_incoming_players: vec![projection_oreilly],
                outgoing_player_ids: vec![],
                limits: TradeLineupLimits::default(),
                disclosures: vec!["Ryan O'Reilly uses a scenario score of 60.".to_owned()],
            },
        )
        .unwrap();
        assert_eq!(view.added_to_lineup, vec!["Ryan O'Reilly"]);
        assert_eq!(view.displaced_by_competition, vec!["Joe Veleno"]);
        assert!((view.strength_delta - 47.3).abs() < 1e-9);
        let assignment = view
            .after
            .iter()
            .find(|row| row.player_id == 8_475_158)
            .unwrap();
        assert_eq!(assignment.position, TradeLineupPosition::Center);
        assert_eq!(
            assignment.assignment_kind,
            TradeLineupAssignmentKind::Natural
        );
        let rebuilt = view.projected_lineup.as_ref().unwrap();
        let dressed_ids = rebuilt
            .forward_lines
            .iter()
            .flat_map(|line| [&line.left_wing, &line.center, &line.right_wing])
            .flatten()
            .map(|player| player.player_id)
            .collect::<std::collections::BTreeSet<_>>();
        assert!(dressed_ids.contains(&8_475_158));
        assert!(!dressed_ids.contains(&8_480_813));
        assert!(rebuilt
            .extras
            .iter()
            .any(|player| player.player_id == 8_480_813));
    }

    #[test]
    fn l0_trade_lineup_board_separates_hockey_rank_from_actionability() {
        let set: crate::view_model::TrainingCampLineupSetView = serde_json::from_str(include_str!(
            "../../../examples/icecast-nyr-training-camp-lineups.json"
        ))
        .unwrap();
        let input: TradeLineupBoardInput = serde_json::from_str(include_str!(
            "../../../examples/icecast-nyr-forward-trade-board-2026-27.json"
        ))
        .unwrap();
        let board = build_trade_lineup_board(&set.branches[0].lineup, input).unwrap();

        assert_eq!(board.schema, TRADE_LINEUP_BOARD_SCHEMA);
        assert_eq!(board.rows[0].label, "Ryan O'Reilly");
        assert_eq!(board.rows[0].hockey_rank, 1);
        assert_eq!(board.rows[0].actionable_rank, Some(1));
        let rust = board
            .rows
            .iter()
            .find(|row| row.label == "Bryan Rust")
            .unwrap();
        assert_eq!(rust.actionable_rank, Some(2));
        assert!(rust.transaction_ready);
        assert!(board
            .rows
            .iter()
            .filter(|row| row.label != "Ryan O'Reilly" && row.label != "Bryan Rust")
            .all(|row| row.actionable_rank.is_none() && !row.transaction_ready));
        let vatrano = board
            .rows
            .iter()
            .find(|row| row.label == "Frank Vatrano")
            .unwrap();
        assert_eq!(vatrano.scenario.strength_delta, 0.0);
        assert!(vatrano.scenario.added_to_lineup.is_empty());
    }

    #[test]
    fn l0_assembly_values_pick_and_derives_execution_gates() {
        let view = assemble_trade_market(
            TradeMarketAssemblyInput {
                as_of: "2026-08-02".to_owned(),
                current_draft_year: 2026,
                authority: TradeExecutionAuthorityInput {
                    teams: vec![
                        TradeTeamExecutionAuthority {
                            team: "NYR".to_owned(),
                            upper_limit: Some(100_000_000),
                            committed_cap_hit: Some(90_000_000),
                            active_roster_players: Some(23),
                            max_active_roster_players: 23,
                            retained_salary_slots_available: Some(3),
                            source_url: Some("https://example.test/cap/NYR".to_owned()),
                            observed_at: Some("2026-08-02T12:00:00Z".to_owned()),
                        },
                        TradeTeamExecutionAuthority {
                            team: "SJS".to_owned(),
                            upper_limit: Some(100_000_000),
                            committed_cap_hit: Some(70_000_000),
                            active_roster_players: Some(23),
                            max_active_roster_players: 23,
                            retained_salary_slots_available: Some(3),
                            source_url: Some("https://example.test/cap/SJS".to_owned()),
                            observed_at: Some("2026-08-02T12:00:00Z".to_owned()),
                        },
                    ],
                    players: vec![
                        player_authority(10, "SJS", Some(5_000_000)),
                        player_authority(20, "NYR", Some(1_000_000)),
                    ],
                    draft_picks: vec![TradeDraftPickExecutionAuthority {
                        asset_id: "NYR-2026-1".to_owned(),
                        owner: "NYR".to_owned(),
                        confirmed: true,
                        source_url: Some("https://example.test/picks/NYR-2026-1".to_owned()),
                        observed_at: Some("2026-08-02T12:00:00Z".to_owned()),
                    }],
                },
                proposals: vec![TradePackageAssemblyInput {
                    buyer: preference("NYR", 1.4, 0.4),
                    seller: preference("SJS", 0.4, 1.4),
                    assets_to_buyer: vec![TradeAssetAssemblyInput::Player {
                        player_id: 10,
                        retained_cap_hit: None,
                        asset: Box::new(player_asset(
                            "top-six-forward",
                            "top_six_forward",
                            100.0,
                            80.0,
                            30.0,
                        )),
                    }],
                    assets_to_seller: vec![
                        TradeAssetAssemblyInput::Player {
                            player_id: 20,
                            retained_cap_hit: None,
                            asset: Box::new(player_asset("prospect", "prospect", 30.0, 5.0, 40.0)),
                        },
                        TradeAssetAssemblyInput::DraftPick {
                            asset: DraftPickAssetInput {
                                id: "NYR-2026-1".to_owned(),
                                draft_year: 2026,
                                slot_outcomes: vec![DraftPickSlotOutcome {
                                    overall_pick: 2,
                                    probability: 1.0,
                                }],
                            },
                        },
                    ],
                }],
            },
            &curve(),
        )
        .unwrap();
        let package = &view.proposals[0];
        assert!(package.transaction_ready);
        assert_eq!(package.transaction_gates.cap_compliant, Some(true));
        assert_eq!(package.transaction_gates.roster_compliant, Some(true));
        assert_eq!(package.buyer_cap_space_delta, Some(-4.0));
        assert_eq!(package.seller_cap_space_delta, Some(4.0));
        assert_eq!(
            package.transaction_gates.pick_ownership_confirmed,
            Some(true)
        );
        assert!(package.transaction_gates.contract_authority_complete);
        assert!(package.feasibility_probability > 0.0);
        assert_eq!(package.pick_roles.len(), 1);
    }

    #[test]
    fn l0_assembly_keeps_missing_cap_as_unknown() {
        let mut player = player_authority(10, "SJS", None);
        player.destination_allowed = Some(true);
        let proposal = TradePackageAssemblyInput {
            buyer: preference("NYR", 1.4, 0.4),
            seller: preference("SJS", 0.4, 1.4),
            assets_to_buyer: vec![TradeAssetAssemblyInput::Player {
                player_id: 10,
                retained_cap_hit: None,
                asset: Box::new(player_asset("target", "top_six_forward", 70.0, 70.0, 15.0)),
            }],
            assets_to_seller: vec![TradeAssetAssemblyInput::Player {
                player_id: 20,
                retained_cap_hit: None,
                asset: Box::new(player_asset("prospect", "prospect", 70.0, 5.0, 70.0)),
            }],
        };
        let view = assemble_trade_market(
            TradeMarketAssemblyInput {
                as_of: "2026-08-02".to_owned(),
                current_draft_year: 2026,
                authority: TradeExecutionAuthorityInput {
                    teams: vec![
                        TradeTeamExecutionAuthority {
                            team: "NYR".to_owned(),
                            upper_limit: Some(100_000_000),
                            committed_cap_hit: Some(90_000_000),
                            active_roster_players: Some(23),
                            max_active_roster_players: 23,
                            retained_salary_slots_available: Some(3),
                            source_url: Some("https://example.test/cap/NYR".to_owned()),
                            observed_at: Some("2026-08-02T12:00:00Z".to_owned()),
                        },
                        TradeTeamExecutionAuthority {
                            team: "SJS".to_owned(),
                            upper_limit: Some(100_000_000),
                            committed_cap_hit: Some(70_000_000),
                            active_roster_players: Some(23),
                            max_active_roster_players: 23,
                            retained_salary_slots_available: Some(3),
                            source_url: Some("https://example.test/cap/SJS".to_owned()),
                            observed_at: Some("2026-08-02T12:00:00Z".to_owned()),
                        },
                    ],
                    players: vec![player, player_authority(20, "NYR", Some(1_000_000))],
                    draft_picks: vec![],
                },
                proposals: vec![proposal],
            },
            &curve(),
        )
        .unwrap();
        assert_eq!(view.proposals[0].transaction_gates.cap_compliant, None);
        assert!(!view.proposals[0].transaction_ready);
        assert_eq!(view.proposals[0].feasibility_probability, 0.0);
    }

    #[test]
    fn l0_salary_retention_changes_both_clubs_cap_math_and_checks_slots() {
        let authority = TradeExecutionAuthorityInput {
            teams: vec![
                TradeTeamExecutionAuthority {
                    team: "NYR".to_owned(),
                    upper_limit: Some(100_000_000),
                    committed_cap_hit: Some(96_000_000),
                    active_roster_players: Some(22),
                    max_active_roster_players: 23,
                    retained_salary_slots_available: Some(3),
                    source_url: Some("https://example.test/cap/NYR".to_owned()),
                    observed_at: Some("2026-08-02".to_owned()),
                },
                TradeTeamExecutionAuthority {
                    team: "SJS".to_owned(),
                    upper_limit: Some(100_000_000),
                    committed_cap_hit: Some(90_000_000),
                    active_roster_players: Some(22),
                    max_active_roster_players: 23,
                    retained_salary_slots_available: Some(1),
                    source_url: Some("https://example.test/cap/SJS".to_owned()),
                    observed_at: Some("2026-08-02".to_owned()),
                },
            ],
            players: vec![player_authority(10, "SJS", Some(6_000_000))],
            draft_picks: vec![],
        };
        let movement = PlayerMovement {
            from: "SJS",
            to: "NYR",
            authority: authority.players.first(),
            retained_cap_hit: 3_000_000,
        };
        assert_eq!(
            derive_cap_compliance("NYR", "SJS", &authority, &[movement]),
            Some(true)
        );
        assert_eq!(derive_cap_space_delta("NYR", &[movement]), Some(-3.0));
        assert_eq!(derive_cap_space_delta("SJS", &[movement]), Some(3.0));
        assert_eq!(
            derive_retention_compliance("NYR", "SJS", &authority, &[movement]),
            Some(true)
        );

        let invalid = PlayerMovement {
            retained_cap_hit: 3_000_001,
            ..movement
        };
        assert_eq!(
            derive_retention_compliance("NYR", "SJS", &authority, &[invalid]),
            Some(false)
        );
    }
}
