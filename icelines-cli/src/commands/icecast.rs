use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, Utc};
use icelines_core::{
    adapt_prospect_arrival_calibration_input, adapt_prospect_conversion_input,
    adapt_team_season_window_scenario_authorities, adapt_training_camp_window_scenario_authorities,
    apply_bench_game_plan_to_player_line_matchup, apply_team_behavior_research,
    audit_organization_profile_history, audit_organization_window_source_package,
    build_adaptive_lineup_policy, build_ahl_affiliate_projection,
    build_balanced_organization_window_board_from_package, build_development_calibration,
    build_forecast_history_card, build_forecast_movement_card, build_isolated_scenario_impact,
    build_isolated_scenario_impact_as_of, build_line_combination_forecast,
    build_organization_lineup_forecast, build_organization_profile_history,
    build_organization_window_history, build_player_line_matchup_forecast,
    build_player_line_matchup_validation, build_prospect_arrival_card,
    build_prospect_conversion_archive, build_prospect_conversion_board,
    build_prospect_development_study, build_prospect_discovery_board,
    build_prospect_nhl_performance_document, build_prospect_program_board_with_goalies,
    build_prospect_program_history, build_prospect_program_sensitivity_with_goalies,
    build_season_simulation_card, build_team_game_forecast, build_team_game_forecast_validation,
    build_team_game_prediction_edge, build_team_game_prediction_edge_card,
    build_team_game_prediction_observation_set,
    build_team_game_rolling_replay_with_opening_strengths, build_team_player_matchup_role_evidence,
    build_team_season_auto_personnel_scenario, build_team_season_forecast_history,
    build_team_season_forecast_movement, build_team_season_game_plan_schedule_from_evidence,
    build_team_season_plausible_trade_scenario, build_training_camp_blender_set,
    build_training_camp_exposure_board_with_context, build_training_camp_lineup_set,
    build_training_camp_opening_roster_policy, calibrate_prospect_arrival,
    calibrate_prospect_arrival_league, calibrate_team_season_scenario_development,
    compare_organization_profile_history, compare_organization_window_scenario,
    compare_organization_window_snapshots, compare_organization_window_snapshots_with_bridge,
    compare_organization_window_typed_scenario, compare_player_line_matchup_scenarios,
    compare_team_season_forecast_scenarios, confidence_weighted_player_value,
    current_ahl_affiliation_catalog, load_organization_window_registry_lifecycle, model::Position,
    model::Season, model::TeamAbbr, normalize_name, project_organization_profile_history_card,
    project_organization_window_card, register_team_game_prediction_holdout,
    rehydrate_team_game_forecast, seal_new_organization_window_manifest,
    seal_organization_profile_history, season_stats::SeasonType,
    simulate_organization_window_scenario_distribution,
    simulate_team_season_forecast_as_of_with_scenario, simulate_team_season_forecast_with_scenario,
    simulate_training_camp, simulate_training_camp_league, train_team_game_prediction_model,
    validate_prospect_conversion_archive, validate_team_game_prediction_model_with_registration,
    AhlAffiliateProjectionInput, AhlAffiliateProjectionView, AhlAffiliationCatalogView,
    AhlCrossLeagueValuePolicy, AhlLineUnitKind, AhlPlayerValuePolicy, AhlRecallReadinessPolicy,
    AhlRosterPoolAuthorityKind, BenchGamePlanView, DevelopmentCalibrationConfig,
    DevelopmentCalibrationView, DevelopmentPositionGroup, DevelopmentTransitionInput,
    DevelopmentValueModel, EvidenceLabel, ForecastHistoryCardInput, ForecastMovementCardInput,
    LineCombinationForecastConfig, LineCombinationForecastView, LineCombinationPairEvidenceInput,
    NhlGoalieTranslationPolicy, OpponentStyleEvidenceRow, OrganizationLevel,
    OrganizationLineupForecastInput, OrganizationLineupForecastView, OrganizationPositionGroup,
    OrganizationProfileHistoryCheckpointView, OrganizationProfileHistoryDeltaView,
    OrganizationProfileHistoryView, OrganizationUnitKind, OrganizationWindowBoardView,
    OrganizationWindowBridgeView, OrganizationWindowManifestView,
    OrganizationWindowScenarioDistributionInput, OrganizationWindowSourceCoverageView,
    OrganizationWindowSourcePackageView, OrganizationalProspectPolicy,
    PlayerLineMatchupAblationObservation, PlayerLineMatchupForecastInput,
    PlayerLineMatchupForecastView, PlayerLineMatchupScenarioInput,
    ProspectArrivalCalibrationConfig, ProspectArrivalCalibrationInput,
    ProspectArrivalCalibrationView, ProspectArrivalCardInput,
    ProspectArrivalLeaguePopulationAuthorityView, ProspectArrivalLeagueSourceExclusionInput,
    ProspectConversionArchive, ProspectConversionBoardView, ProspectConversionConfig,
    ProspectConversionPerformanceDocument, ProspectDevelopmentStudyConfig,
    ProspectDevelopmentStudyInput, ProspectDevelopmentStudyView, ProspectDiscoveryBoardRow,
    ProspectDiscoveryBoardView, ProspectGoalieDevelopmentStudyConfig,
    ProspectGoalieDevelopmentStudyView, ProspectNhlGamesAuthority, ProspectProgramBoardConfig,
    ProspectProgramBoardView, ProspectProgramHistoryView, ProspectProgramPopulationAuthorityView,
    ProspectProgramSensitivityView, ScenarioScopeView, ScheduleRestProfileView,
    SeasonSimulationCardInput, TeamBehaviorResearchInput, TeamCeilingPlayerRow,
    TeamDecisionProfile, TeamForecastGameInput, TeamForecastParameters,
    TeamForecastPersonnelEvidenceInput, TeamForecastPersonnelPlayerInput, TeamForecastReplayConfig,
    TeamForecastStrengthInput, TeamGameForecastCalibrationObservation, TeamGameForecastRow,
    TeamGameForecastValidationInput, TeamGameForecastView, TeamGameOpeningPlayerRow,
    TeamGameOpeningRosterAuthorityRow, TeamGameOpeningStrengthRow, TeamGamePredictionEdgeCardInput,
    TeamGamePredictionEdgeView, TeamGamePredictionHoldoutRegistration,
    TeamGamePredictionMarketBenchmarkInput, TeamGamePredictionModel,
    TeamGamePredictionObservationSet, TeamGamePredictionTrainingConfig,
    TeamGamePredictionTrainingObservation, TeamLineupProjectionView,
    TeamPlayerMatchupRoleEvidenceView, TeamSeasonAutoPersonnelConfig,
    TeamSeasonForecastHistoryView, TeamSeasonForecastMovementView, TeamSeasonForecastView,
    TeamSeasonPersonnelInput, TeamSeasonPlausibleTradeConfig, TeamSeasonScenario,
    TeamSeasonScenarioDevelopmentCalibrationInput, TeamSeasonScenarioDevelopmentCalibrationView,
    TeamSeasonScenarioEventKind, TeamSeasonSimulationConfig, TeamSeasonStretchKind,
    TeamSeasonTradeTeamInput, TrainingCampAuthorityStatus, TrainingCampCompetitionPoolStatus,
    TrainingCampConfig, TrainingCampExposureBoardView, TrainingCampExposureLane,
    TrainingCampForecastView, TrainingCampLeagueForecastView, TrainingCampLeagueSimulationInput,
    TrainingCampLeagueTeamInput, TrainingCampPlayerInput, TrainingCampSalaryCapStatus,
    TrainingCampSimulationInput, TrainingCampTransactionAuthorityStatus,
    TrainingCampTransactionContextInput, ViewContext, ViewWindow, WindowHorizon,
    WindowManifestLifecyclePolicy, WindowScenarioAuthorityView, CANONICAL_TEAMS, CURRENT_SEASON,
    ORGANIZATION_WINDOW_SOURCE_PACKAGE_SCHEMA, PROSPECT_CONVERSION_PERFORMANCE_SCHEMA,
    TEAM_SEASON_SCENARIO_DEVELOPMENT_CALIBRATION_SCHEMA,
};
use icelines_core::{
    attribute_organization_window_personnel_movement,
    build_later_counterfactual_personnel_attribution_input,
    calibrate_organization_window_rolling_origins, evaluate_organization_window_origins,
    load_organization_window_profile_inventory, rebase_organization_window_board,
    require_ranked_balanced_organization_window_board, seal_organization_window_source_package,
    summarize_organization_window_personnel_evidence, validate_organization_window_board,
    OrganizationWindowMovementView, OrganizationWindowPersonnelAttributionInputView,
    WindowCalibrationEvaluationOriginInput, WindowCalibrationOriginInput,
    WindowCalibrationOriginRole,
};
use icelines_fetch::{
    ahl::{
        affiliate_projection_input_from_reviewed_crosswalk, ahl_identity_search_name_variants,
        apply_ahl_identity_league_birth_date_correction, apply_ahl_identity_league_collision_remap,
        apply_ahl_identity_league_conflict_review, apply_ahl_identity_league_rejection_review,
        apply_ahl_identity_league_routine_review, apply_ahl_identity_review_decisions,
        build_ahl_alias_identity_review, build_ahl_exact_identity_review,
        build_ahl_identity_crosswalk, build_ahl_identity_exception_board,
        build_ahl_identity_league_crosswalk, build_ahl_identity_league_review,
        build_ahl_identity_league_review_draft, build_ahl_identity_rejection_review,
        build_ahl_identity_review_draft_with_options, build_ahl_identity_review_inspection,
        enrich_official_nhl_landing_candidate, merge_ahl_canonical_identity_catalogs,
        normalize_ahl_identity_name, parse_official_nhl_search_candidates,
        parse_official_nhl_search_candidates_by_surname, AhlCanonicalIdentityCandidate,
        AhlCanonicalIdentityCatalog, AhlIdentityCrosswalkView, AhlIdentityExceptionBoardView,
        AhlIdentityInspectionScope, AhlIdentityLeagueCrosswalkView, AhlIdentityLeagueReviewView,
        AhlIdentityLeagueRoutineReviewKind, AhlIdentityMatchBasis, AhlIdentityReviewDecisions,
        AhlIdentityReviewDraftOptions, AhlIdentityReviewInspectionView, AhlIdentityReviewStatus,
        AhlProjectionPlayerFacts, AhlRosterStatsSnapshot, AHL_CANONICAL_IDENTITY_CATALOG_SCHEMA,
        AHL_IDENTITY_CROSSWALK_SCHEMA, AHL_IDENTITY_LEAGUE_CROSSWALK_SCHEMA,
    },
    ahl_cross_league_value::{
        apply_ahl_cross_league_value_ledger, build_ahl_cross_league_value_ledger,
        AhlCrossLeagueValueApplicationView, AhlCrossLeagueValueLedgerView,
    },
    ahl_organization_status::{
        apply_ahl_organization_status_ledger, build_ahl_organization_status_ledger,
        AhlOrganizationStatusLedgerView,
    },
    ahl_player_value::{
        apply_ahl_player_value_ledger, build_ahl_player_value_ledger,
        AhlPlayerValueApplicationView, AhlPlayerValueLedgerView,
    },
    ahl_preseason_facts::{
        apply_ahl_preseason_league_facts_overlay, build_ahl_preseason_league_facts_overlay_draft,
        build_ahl_preseason_league_facts_workboard, build_ahl_preseason_league_projection_inputs,
        build_prior_season_organization_profile_history, AhlPreseasonLeagueFactsApplicationView,
        AhlPreseasonLeagueFactsOverlay, AhlPreseasonLeagueFactsWorkboardView,
        AhlPreseasonLeagueProjectionInputsView, AHL_PRESEASON_LEAGUE_PROJECTION_INPUTS_SCHEMA,
    },
    ahl_professional_games::{
        apply_ahl_professional_game_ledger_to_facts, build_ahl_professional_game_ledger,
        build_ahl_professional_game_ledger_with_candidates, AhlProfessionalGameCandidate,
        AhlProfessionalGameFactsApplicationView, AhlProfessionalGameLedgerView,
        AhlProfessionalGamePolicy, AHL_PROFESSIONAL_GAME_FACTS_SCHEMA,
    },
    ahl_prospect_status::{
        apply_ahl_prospect_status_ledger, build_ahl_prospect_status_ledger,
        AhlProspectStatusApplicationView, AhlProspectStatusLedgerView,
    },
    ahl_recall_readiness::{
        apply_ahl_recall_readiness_ledger, build_ahl_recall_readiness_ledger,
        AhlRecallReadinessApplicationView, AhlRecallReadinessLedgerView,
    },
    ahl_rollover::{
        apply_ahl_preseason_league_organization_review, apply_ahl_preseason_organization_review,
        build_ahl_preseason_league_organization_review_draft, build_ahl_preseason_league_rollover,
        build_ahl_preseason_league_rollover_config_draft,
        build_ahl_preseason_organization_review_draft, build_ahl_preseason_rollover,
        AhlPreseasonLeagueOrganizationReview, AhlPreseasonLeagueRolloverConfig,
        AhlPreseasonLeagueRolloverView, AhlPreseasonOrganizationReview, AhlPreseasonPositionGroup,
        AhlPreseasonRolloverConfig, AhlPreseasonRolloverView,
        AHL_PRESEASON_ORGANIZATION_REVIEW_SCHEMA,
    },
    ahl_transaction_state::{
        apply_ahl_transaction_state_ledger, build_ahl_transaction_state_ledger,
        AhlTransactionStateApplicationView, AhlTransactionStateLedgerView,
    },
    ahl_transactions::AhlTransactionSnapshot,
    ahl_waiver_clearance::{
        apply_ahl_waiver_clearance_review, build_ahl_waiver_clearance_review_draft,
        finalize_ahl_waiver_clearance_review, AhlWaiverClearanceApplicationView,
        AhlWaiverClearanceDecisionsView, AhlWaiverClearanceReviewView,
    },
    attach_player_line_matchup_forecast, build_game_prediction_edge_evidence_package,
    build_historical_confirmed_edge_package, build_historical_goalie_edge_package,
    build_historical_moneypuck_edge_package, build_historical_organization_window_origin,
    build_moneypuck_line_chemistry, build_moneypuck_line_chemistry_from_package,
    build_official_game_outcome_set, build_organization_window_completion_status,
    build_organization_window_future_holdout_registration,
    build_organization_window_standings_snapshot, build_player_line_matchup_profiles,
    build_preseason_game_prediction_edge_evidence_package, build_prospect_career_context_draft,
    build_prospect_career_discovery, build_prospect_league_context_draft,
    build_prospect_league_discovery, build_prospect_program_from_camp_and_career_store,
    build_shift_adjusted_chemistry_evidence, build_shift_overlap_report,
    bundled::{
        get_bios, get_bios_installed, get_goalie_stats, get_goalie_stats_installed, get_stats,
        get_stats_installed, load_transactions_with_fallback,
    },
    career_landing::CareerHistoryStore,
    complete_lineup_goalies_with_training_camp, fetch_lock,
    fetch_official_player_landing_cachelines, fetch_official_player_search_cachelines,
    fetch_team_behavior_league_evidence,
    fletch::{player_landing_url, roster_url, FletchPlayerLandingArtifact},
    load_game_prediction_edge_evidence_package, moneypuck_goalie_game_url, moneypuck_team_game_url,
    nhl_api::ScheduledGame,
    parse_moneypuck_line_games,
    schema::{GoalieStats, LocalizedString, RosterPlayer, RosterResponse, SkaterBio, SkaterStats},
    score_organization_window_future_holdout, select_controlled_prospect_studies,
    snapshot::{
        OfficialNhlRosterCaptureManifest, SnapshotEntry, SnapshotStore, SnapshotTier,
        OFFICIAL_NHL_LIVE_ROSTER_MANIFEST_FILE, OFFICIAL_NHL_LIVE_ROSTER_SCHEMA,
        OFFICIAL_NHL_LIVE_ROSTER_SOURCE,
    },
    stats_loader::load_into_repo,
    ControlledProspectStudySelection, GamePredictionEvidencePackageBuildInput,
    GamePredictionEvidenceSource, GamePredictionEvidenceSourceAuthority,
    HistoricalMoneyPuckGoalieInput, HistoricalMoneyPuckTeamInput, HistoricalOfficialBoxscoreInput,
    MoneyPuckLineChemistrySourcePackage, MoneyPuckUnitBaselineConfig, NhlApiClient,
    OfficialGameOutcomeSet, OfficialShiftChartRow, OrganizationWindowFutureHoldoutRegistration,
    OrganizationWindowFutureHoldoutResult, OrganizationWindowHistoricalOriginArtifact,
    OrganizationWindowStandingsSnapshot, PregameUnitXgBaseline, ProspectCareerContextDraftConfig,
    ProspectCareerContextIdentityInput, ProspectCareerDiscoveryView, ProspectCareerProgramConfig,
    ProspectLeagueContext, ProspectLeagueContextDraftConfig, ProspectLeagueContextExclusionReason,
    ProspectLeagueContextExclusionView, ProspectLeagueDiscoveryView,
    ProspectPopulationOverlay as LeagueCampCandidateOverlay, ScenarioRegistryStore,
    ShiftAdjustedUnitOutcomeInput, ShiftOverlapReport,
    ORGANIZATION_WINDOW_HISTORICAL_ORIGIN_SCHEMA, PROSPECT_CAREER_DISCOVERY_SCHEMA,
    PROSPECT_LEAGUE_DISCOVERY_SCHEMA,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{fantasy::load_fantasy_schedule, report::load_team_ceiling_view_with_candidates};
use crate::config::Config;

pub struct IceCastSeasonArgs {
    pub season: u32,
    pub stats_season: u32,
    pub candidate_overlay: Option<PathBuf>,
    pub teams: Vec<String>,
    pub trials: u32,
    pub seed: u64,
    pub scenario: Option<PathBuf>,
    pub scenario_id: Option<String>,
    pub isolated_impacts: bool,
    pub auto_personnel: bool,
    pub trade_mode: String,
    pub replay_mode: String,
    pub ignore_replay_personnel_after: Option<NaiveDate>,
    pub through: Option<NaiveDate>,
    pub retrospective_opening_lineups: bool,
    pub all_games: bool,
    pub refresh: bool,
    pub json: bool,
    pub out: Option<PathBuf>,
    pub game_forecast_out: Option<PathBuf>,
}

#[allow(clippy::too_many_arguments)]
pub async fn run_draft_pick_curve(
    start_year: u16,
    cutoff_year: u16,
    completed_season_start_year: u16,
    horizon: u8,
    max_pick: u16,
    annual_future_discount: f64,
    generated_at: Option<String>,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let generated_at = generated_at
        .map(|value| {
            DateTime::parse_from_rfc3339(&value)
                .map(|value| value.with_timezone(&Utc))
                .with_context(|| format!("parse --generated-at {value}"))
        })
        .transpose()?
        .unwrap_or_else(Utc::now);
    let view = icelines_fetch::acquire_draft_pick_value_curve(
        &NhlApiClient::production(),
        icelines_fetch::DraftPickOutcomeBuildConfig {
            training_start_year: start_year,
            training_cutoff_year: cutoff_year,
            completed_season_start_year,
            outcome_horizon_years: horizon,
            max_overall_pick: max_pick,
        },
        annual_future_discount,
        generated_at,
    )
    .await
    .map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        let mut rendered = format!(
            "THE TRADE DESK · DRAFT-PICK CURVE\n{}-{} drafts · {}-season NHL GP horizon · {} selections · {} zero outcomes\n\n",
            view.outcome_set.training_start_year,
            view.outcome_set.training_cutoff_year,
            view.outcome_set.outcome_horizon_years,
            view.outcome_set.selections,
            view.outcome_set.zero_outcomes
        );
        for pick in [1_u16, 16, 32, 64, 96, 128, 160, 192, max_pick] {
            if let Some(row) = view
                .curve
                .values
                .iter()
                .find(|row| row.overall_pick == pick)
            {
                let _ = writeln!(
                    rendered,
                    "#{pick:<3} {:>6.1} expected NHL GP  ({:>5.1}-{:>5.1}) · n={}",
                    row.expected_value,
                    row.expected_value_low,
                    row.expected_value_high,
                    row.observations
                );
            }
        }
        for disclosure in view
            .outcome_set
            .disclosures
            .iter()
            .chain(view.curve.disclosures.iter())
        {
            let _ = writeln!(rendered, "- {disclosure}");
        }
        rendered
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "draft-pick value curve")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_trade_market(input: PathBuf, json: bool, out: Option<PathBuf>) -> anyhow::Result<()> {
    let market: icelines_core::TradeMarketInput = serde_json::from_slice(
        &std::fs::read(&input)
            .with_context(|| format!("read trade market input {}", input.display()))?,
    )
    .with_context(|| format!("parse trade market input {}", input.display()))?;
    let view = icelines_core::evaluate_trade_market(market).map_err(anyhow::Error::msg)?;
    write_trade_market_view(&view, json, out)
}

pub fn run_trade_scout(input: PathBuf, json: bool, out: Option<PathBuf>) -> anyhow::Result<()> {
    let scout: icelines_core::TradeScoutInput = read_icecast_json(&input, "trade scout input")?;
    let view = icelines_core::build_trade_scout(scout).map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_trade_scout_view(&view)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "trade scout")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_trade_scout_league(
    input: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let inventory: icelines_core::TradeScoutLeagueInput =
        read_icecast_json(&input, "league trade scout input")?;
    let view =
        icelines_core::build_trade_scout_from_league(inventory).map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        let coverage = if view.inventory_complete {
            "complete"
        } else {
            "partial"
        };
        format!(
            "LEAGUE INVENTORY — {}/{} ({coverage}); derived {} targets and {} buyer assets\n{}",
            view.organizations_supplied,
            view.expected_organizations,
            view.derived_targets,
            view.derived_buyer_assets,
            render_trade_scout_view(&view.scout)
        )
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "league trade scout")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_trade_scout_populate(
    camp: PathBuf,
    input: PathBuf,
    pick_assets: Option<PathBuf>,
    board_out: Option<PathBuf>,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let camp: icelines_core::TrainingCampLeagueForecastView =
        read_icecast_json(&camp, "training camp league forecast")?;
    let mut policy: icelines_core::TradeScoutPopulationInput =
        read_icecast_json(&input, "trade scout population input")?;
    if let Some(path) = pick_assets.as_deref() {
        let picks: icelines_core::TradeScoutDraftPickPopulationView =
            read_icecast_json(path, "trade scout draft-pick population")?;
        if picks.schema != icelines_core::TRADE_SCOUT_DRAFT_PICK_POPULATION_SCHEMA {
            bail!("pick assets must use trade_scout_draft_pick_population.v1");
        }
        policy.draft_pick_assets.extend(picks.assets);
    }
    let view = icelines_core::populate_trade_scout_league_from_camp(&camp, policy)
        .map_err(anyhow::Error::msg)?;
    if let Some(path) = board_out.as_deref() {
        let board = icelines_core::build_trade_scout_from_league(view.league_input.clone())
            .map_err(anyhow::Error::msg)?;
        let board_json = format!("{}\n", serde_json::to_string_pretty(&board)?);
        write_icecast_file(path, board_json.as_bytes(), "populated league trade scout")?;
    }
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        format!(
            "TRADE SCOUT POPULATION — {}/{} teams, {} players ({} prospects), {} picks; {} coverage gaps\n",
            view.teams_populated,
            view.teams_requested,
            view.players_populated,
            view.prospects_populated,
            view.picks_populated,
            view.teams_without_forecast.len()
        )
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "trade scout population")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_trade_calibrate(input: PathBuf, json: bool, out: Option<PathBuf>) -> anyhow::Result<()> {
    let input: icelines_core::TradeCompletionCalibrationInput =
        read_icecast_json(&input, "trade completion calibration input")?;
    let view = icelines_core::calibrate_trade_completion(input).map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        format!(
            "TRADE COMPLETION CALIBRATION — {}/{} completed ({:.1}%); Brier {:.4}, log loss {:.4}, ECE {:.4}\n",
            view.completions,
            view.observations,
            view.completion_rate * 100.0,
            view.brier_score,
            view.log_loss,
            view.expected_calibration_error
        )
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "trade completion calibration")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_trade_features(input: PathBuf, json: bool, out: Option<PathBuf>) -> anyhow::Result<()> {
    let scout: icelines_core::TradeScoutView = read_icecast_json(&input, "trade scout view")?;
    let view =
        icelines_core::build_trade_completion_feature_set(&scout).map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        format!(
            "TRADE COMPLETION FEATURES — {} candidates, {} negotiation rows; labels=absent completion_probability=absent\n",
            view.candidates,
            view.rows.len()
        )
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "trade completion features")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct TradePickPopulationPolicy {
    as_of: String,
    current_draft_year: u16,
    value_basis: icelines_core::TradeValueBasis,
    #[serde(default)]
    protected_asset_ids: Vec<String>,
    #[serde(default)]
    conditional_rights: Vec<icelines_core::TradeDraftPickConditionalRightInput>,
    #[serde(default)]
    protection_chains: Vec<icelines_core::TradeDraftPickProtectionChainInput>,
}

pub fn run_trade_pick_populate(
    ownership: PathBuf,
    curve: PathBuf,
    policy: PathBuf,
    season_forecast: Option<PathBuf>,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let policy: TradePickPopulationPolicy =
        read_icecast_json(&policy, "trade draft-pick population policy")?;
    let ownership_bytes = std::fs::read(&ownership)
        .with_context(|| format!("read draft-pick ownership {}", ownership.display()))?;
    let ownership = icelines_fetch::parse_draft_pick_ownership_csv(
        &ownership_bytes,
        policy.current_draft_year,
    )?;
    let curve_value: serde_json::Value = read_icecast_json(&curve, "draft-pick value curve")?;
    let curve = serde_json::from_value::<icelines_core::DraftPickValueCurve>(curve_value.clone())
        .or_else(|_| {
            serde_json::from_value::<icelines_fetch::DraftPickCurveAcquisitionView>(curve_value)
                .map(|view| view.curve)
        })
        .with_context(|| {
            "curve must be draft_pick_value_curve.v1 or draft_pick_curve_acquisition.v1"
        })?;
    let season_forecast = season_forecast
        .as_deref()
        .map(|path| {
            read_icecast_json::<icelines_core::TeamSeasonForecastView>(path, "team season forecast")
        })
        .transpose()?;
    let view = icelines_core::populate_trade_scout_draft_picks_with_forecast(
        &curve,
        icelines_core::TradeScoutDraftPickPopulationInput {
            as_of: policy.as_of,
            current_draft_year: policy.current_draft_year,
            value_basis: policy.value_basis,
            ownership,
            protected_asset_ids: policy.protected_asset_ids,
            conditional_rights: policy.conditional_rights,
            protection_chains: policy.protection_chains,
        },
        season_forecast.as_ref(),
    )
    .map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        format!(
            "TRADE PICK POPULATION — {}/{} offer-ready valued; {} conditional or encumbered rights unresolved ({} indicative conditional analyses)\n",
            view.picks_populated,
            view.picks_supplied,
            view.unresolved_asset_ids.len(),
            view.conditional_valuations.len() + view.protection_chain_valuations.len()
        )
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "trade draft-pick population")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_trade_pick_coverage(
    ownership: PathBuf,
    draft_year: u16,
    as_of: String,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let ownership_bytes = std::fs::read(&ownership)
        .with_context(|| format!("read draft-pick ownership {}", ownership.display()))?;
    let ownership = icelines_fetch::parse_draft_pick_ownership_csv(&ownership_bytes, draft_year)?;
    let view = icelines_core::build_trade_draft_pick_ownership_coverage(
        icelines_core::TradeDraftPickOwnershipCoverageInput {
            as_of,
            draft_year,
            expected_teams: icelines_core::CANONICAL_TEAMS
                .iter()
                .map(|(abbreviation, _)| (*abbreviation).to_owned())
                .collect(),
            ownership,
        },
    )
    .map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        format!(
            "TRADE PICK COVERAGE — {}/{} coordinates; {} confirmed, {} conditional, {} encumbered; coordinate_complete={} offer_ready={}\n",
            view.coordinates_supplied,
            view.coordinates_expected,
            view.confirmed_unconditional,
            view.conditional,
            view.encumbered,
            view.coordinate_inventory_complete,
            view.offer_ready_inventory_complete
        )
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(
            path,
            output.as_bytes(),
            "trade draft-pick ownership coverage",
        )?;
    } else {
        print!("{output}");
    }
    Ok(())
}

fn render_trade_scout_view(view: &icelines_core::TradeScoutView) -> String {
    let mut rendered = format!("THE TRADE SCOUT — {}\n", view.buyer);
    for candidate in &view.candidates {
        let _ = writeln!(
            rendered,
            "{} {} ({})  fit={:.2}  avail={:.0}%",
            candidate.rank,
            candidate.label,
            candidate.seller,
            candidate.hockey_fit_score,
            candidate.availability.probability * 100.0
        );
        for package in [
            &candidate.negotiation.opening_offer,
            &candidate.negotiation.fair_midpoint,
            &candidate.negotiation.maximum_acceptable,
        ] {
            let tier = match package.tier {
                icelines_core::TradeNegotiationTier::OpeningOffer => "opening",
                icelines_core::TradeNegotiationTier::FairMidpoint => "fair",
                icelines_core::TradeNegotiationTier::MaximumAcceptable => "maximum",
            };
            let _ = writeln!(
                rendered,
                "   {tier}: {:.0}% value [{}]",
                package.target_value_ratio * 100.0,
                package.assets_to_seller.join(", ")
            );
        }
        let _ = writeln!(
            rendered,
            "   walk away above {:.2}; transaction authority pending",
            candidate.negotiation.walk_away_market_value
        );
    }
    for disclosure in &view.disclosures {
        let _ = writeln!(rendered, "- {disclosure}");
    }
    rendered
}

pub fn run_trade_lineup(
    lineup: PathBuf,
    change: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let (lineup, lineup_adapter_disclosure) = read_trade_lineup_baseline(&lineup)?;
    let change: icelines_core::TradeLineupProjectionChangeInput =
        read_icecast_json(&change, "trade lineup change")?;
    let mut view = icelines_core::build_trade_lineup_scenario_from_projection(&lineup, change)
        .map_err(anyhow::Error::msg)?;
    if let Some(disclosure) = lineup_adapter_disclosure {
        view.disclosures.push(disclosure);
    }
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        let mut rendered = format!(
            "THE LINE CHANGE — {}\nStrength {:+.2} ({:.2} → {:.2})\nAdded: {}\nTraded out: {}\nDisplaced: {}\n",
            view.team,
            view.strength_delta,
            view.before_strength,
            view.after_strength,
            display_names_or_none(&view.added_to_lineup),
            display_names_or_none(&view.explicitly_removed_from_lineup),
            display_names_or_none(&view.displaced_by_competition)
        );
        for disclosure in &view.disclosures {
            let _ = writeln!(rendered, "- {disclosure}");
        }
        if let Some(lineup) = &view.projected_lineup {
            render_trade_projected_lineup(&mut rendered, lineup);
        }
        rendered
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "trade lineup impact")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_trade_lineup_board(
    lineup: PathBuf,
    input: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let (lineup, adapter_disclosure) = read_trade_lineup_baseline(&lineup)?;
    let input: icelines_core::TradeLineupBoardInput =
        read_icecast_json(&input, "trade lineup board")?;
    let mut view =
        icelines_core::build_trade_lineup_board(&lineup, input).map_err(anyhow::Error::msg)?;
    if let Some(disclosure) = adapter_disclosure {
        view.disclosures.push(disclosure);
    }
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        let mut rendered = format!("THE TRADE BOARD — {}\n", view.team);
        for row in &view.rows {
            let actionable = row
                .actionable_rank
                .map_or_else(|| "blocked".to_owned(), |rank| format!("A{rank}"));
            let _ = writeln!(
                rendered,
                "H{} {:<20} {:+.2}  {:>7}  avail={:.0}% exec={:.0}%  adds {}  displaces {}",
                row.hockey_rank,
                row.label,
                row.scenario.strength_delta,
                actionable,
                row.availability_probability * 100.0,
                row.feasibility_probability * 100.0,
                display_names_or_none(&row.scenario.added_to_lineup),
                display_names_or_none(&row.scenario.displaced_by_competition)
            );
            if let Some(impact) = row
                .package_evaluation
                .as_ref()
                .and_then(|package| package.season_forecast_impact.as_ref())
            {
                let _ = writeln!(
                    rendered,
                    "   paired season: {:+.2} pts  {:+.2}pp playoffs  {:+.2}pp Cup",
                    impact.buyer.average_points_delta,
                    impact.buyer.playoff_probability_delta * 100.0,
                    impact.buyer.stanley_cup_probability_delta * 100.0
                );
            }
        }
        for disclosure in &view.disclosures {
            let _ = writeln!(rendered, "- {disclosure}");
        }
        rendered
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "trade lineup board")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

fn read_trade_lineup_baseline(
    lineup: &std::path::Path,
) -> anyhow::Result<(icelines_core::TeamLineupProjectionView, Option<String>)> {
    let lineup_bytes = std::fs::read(lineup)
        .with_context(|| format!("read baseline trade lineup {}", lineup.display()))?;
    match serde_json::from_slice::<icelines_core::TeamLineupProjectionView>(&lineup_bytes) {
        Ok(lineup) => Ok((lineup, None)),
        Err(projection_error) => {
            let set: icelines_core::TrainingCampLineupSetView =
                serde_json::from_slice(&lineup_bytes).with_context(|| {
                    format!(
                        "parse baseline {} as team lineup projection ({projection_error}) or training-camp lineup set",
                        lineup.display()
                    )
                })?;
            let branch = set.branches.first().ok_or_else(|| {
                anyhow::anyhow!("training-camp lineup set has no retained branches")
            })?;
            Ok((
                branch.lineup.clone(),
                Some(format!(
                    "Baseline selected training-camp branch {} ({:.1}% probability) from {}.",
                    branch.rank,
                    branch.probability * 100.0,
                    set.schema
                )),
            ))
        }
    }
}

fn render_trade_projected_lineup(
    rendered: &mut String,
    lineup: &icelines_core::TeamLineupProjectionView,
) {
    let _ = writeln!(rendered, "\nPROJECTED LINES");
    for line in &lineup.forward_lines {
        let _ = writeln!(
            rendered,
            "F{}  {} | {} | {}",
            line.line,
            trade_lineup_player_label(line.left_wing.as_ref()),
            trade_lineup_player_label(line.center.as_ref()),
            trade_lineup_player_label(line.right_wing.as_ref())
        );
    }
    for pair in &lineup.defense_pairs {
        let _ = writeln!(
            rendered,
            "D{}  {} | {}",
            pair.pair,
            trade_lineup_player_label(pair.left.as_ref()),
            trade_lineup_player_label(pair.right.as_ref())
        );
    }
    let _ = writeln!(
        rendered,
        "G   {} | {}",
        trade_lineup_player_label(lineup.goalies.starter.as_ref()),
        trade_lineup_player_label(lineup.goalies.backup.as_ref())
    );

    let names = trade_lineup_player_names(lineup);
    let _ = writeln!(rendered, "\nSPECIAL TEAMS");
    for unit in lineup
        .special_teams
        .power_play
        .iter()
        .chain(&lineup.special_teams.penalty_kill)
    {
        let kind = match unit.kind {
            icelines_core::TeamLineupSpecialTeamsKind::PowerPlay => "PP",
            icelines_core::TeamLineupSpecialTeamsKind::PenaltyKill => "PK",
        };
        let players = unit
            .player_ids
            .iter()
            .map(|id| {
                names
                    .get(id)
                    .map_or_else(|| id.to_string(), |name| (*name).clone())
            })
            .collect::<Vec<_>>()
            .join(" | ");
        let quarterback = unit
            .quarterback_id
            .and_then(|id| names.get(&id))
            .map_or(String::new(), |name| format!(" (QB: {name})"));
        let _ = writeln!(rendered, "{kind}{} {players}{quarterback}", unit.unit);
    }
    for warning in &lineup.special_teams.warnings {
        let _ = writeln!(rendered, "- {warning}");
    }
}

fn trade_lineup_player_label(player: Option<&icelines_core::TeamLineupPlayerView>) -> String {
    player.map_or_else(
        || "open".to_owned(),
        |player| format!("{} ({})", player.display_name, player.score.display),
    )
}

fn trade_lineup_player_names(
    lineup: &icelines_core::TeamLineupProjectionView,
) -> std::collections::BTreeMap<u32, String> {
    lineup
        .forward_lines
        .iter()
        .flat_map(|line| [&line.left_wing, &line.center, &line.right_wing])
        .chain(
            lineup
                .defense_pairs
                .iter()
                .flat_map(|pair| [&pair.left, &pair.right]),
        )
        .chain([&lineup.goalies.starter, &lineup.goalies.backup])
        .flatten()
        .chain(&lineup.extras)
        .map(|player| (player.player_id, player.display_name.clone()))
        .collect()
}

fn display_names_or_none(names: &[String]) -> String {
    if names.is_empty() {
        "none".to_owned()
    } else {
        names.join(", ")
    }
}

pub struct IceCastTradeMarketAssembleArgs {
    pub input: PathBuf,
    pub curve: PathBuf,
    pub baseline_forecast: Option<PathBuf>,
    pub scenario_forecast: Option<PathBuf>,
    pub buyer_lineup: Option<PathBuf>,
    pub seller_lineup: Option<PathBuf>,
    pub json: bool,
    pub out: Option<PathBuf>,
}

pub fn run_trade_market_assemble(args: IceCastTradeMarketAssembleArgs) -> anyhow::Result<()> {
    let assembly: icelines_core::TradeMarketAssemblyInput =
        serde_json::from_slice(&std::fs::read(&args.input).with_context(|| {
            format!("read trade market assembly input {}", args.input.display())
        })?)
        .with_context(|| format!("parse trade market assembly input {}", args.input.display()))?;
    let curve_value: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&args.curve)
            .with_context(|| format!("read draft-pick curve {}", args.curve.display()))?,
    )
    .with_context(|| format!("parse draft-pick curve {}", args.curve.display()))?;
    let curve = serde_json::from_value::<icelines_core::DraftPickValueCurve>(curve_value.clone())
        .or_else(|_| {
            serde_json::from_value::<icelines_fetch::DraftPickCurveAcquisitionView>(curve_value)
                .map(|view| view.curve)
        })
        .with_context(|| {
            "curve must be draft_pick_value_curve.v1 or draft_pick_curve_acquisition.v1"
        })?;
    let mut view =
        icelines_core::assemble_trade_market(assembly, &curve).map_err(anyhow::Error::msg)?;
    if let (Some(baseline_path), Some(scenario_path)) = (
        args.baseline_forecast.as_deref(),
        args.scenario_forecast.as_deref(),
    ) {
        if view.proposals.len() != 1 {
            anyhow::bail!(
                "paired season forecast attachment currently requires exactly one assembled proposal"
            );
        }
        let baseline: icelines_core::TeamSeasonForecastView =
            read_icecast_json(baseline_path, "baseline team-season forecast")?;
        let scenario: icelines_core::TeamSeasonForecastView =
            read_icecast_json(scenario_path, "trade-scenario team-season forecast")?;
        let buyer = view.proposals[0].buyer.clone();
        let seller = view.proposals[0].seller.clone();
        icelines_core::attach_trade_package_season_forecast(
            &mut view, &buyer, &seller, &baseline, &scenario,
        )
        .map_err(anyhow::Error::msg)?;
    }
    if let (Some(buyer_path), Some(seller_path)) =
        (args.buyer_lineup.as_deref(), args.seller_lineup.as_deref())
    {
        if view.proposals.len() != 1 {
            anyhow::bail!(
                "lineup impact attachment currently requires exactly one assembled proposal"
            );
        }
        let buyer_input: icelines_core::TradeLineupScenarioInput =
            read_icecast_json(buyer_path, "buyer trade-lineup scenario")?;
        let seller_input: icelines_core::TradeLineupScenarioInput =
            read_icecast_json(seller_path, "seller trade-lineup scenario")?;
        let buyer_lineup =
            icelines_core::build_trade_lineup_scenario(buyer_input).map_err(anyhow::Error::msg)?;
        let seller_lineup =
            icelines_core::build_trade_lineup_scenario(seller_input).map_err(anyhow::Error::msg)?;
        let buyer = view.proposals[0].buyer.clone();
        let seller = view.proposals[0].seller.clone();
        icelines_core::attach_trade_package_lineup_impacts(
            &mut view,
            &buyer,
            &seller,
            buyer_lineup,
            seller_lineup,
        )
        .map_err(anyhow::Error::msg)?;
    }
    write_trade_market_view(&view, args.json, args.out)
}

fn write_trade_market_view(
    view: &icelines_core::TradeMarketEvaluationView,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        let mut rendered = format!(
            "THE TRADE DESK\nAs of {}\nValue basis: {} over {} years ({})\n\n",
            view.as_of,
            view.value_basis.outcome_measure,
            view.value_basis.horizon_years,
            view.value_basis.method
        );
        for (index, proposal) in view.proposals.iter().enumerate() {
            let _ = writeln!(
                rendered,
                "{}. {} ⇄ {}  feasibility {:>5.1}%  fairness {:>5.1}%  mutual {}",
                index + 1,
                proposal.buyer,
                proposal.seller,
                proposal.feasibility_probability * 100.0,
                proposal.fairness_score * 100.0,
                if proposal.mutually_beneficial {
                    "yes"
                } else {
                    "no"
                }
            );
            let _ = writeln!(
                rendered,
                "   to {}: {}",
                proposal.buyer,
                proposal.assets_to_buyer.join(", ")
            );
            let _ = writeln!(
                rendered,
                "   to {}: {}",
                proposal.seller,
                proposal.assets_to_seller.join(", ")
            );
            let _ = writeln!(
                rendered,
                "   utility {:+.2} / {:+.2} · market gap {:+.2} · next-season points {:+.2} / {:+.2}\n",
                proposal.buyer_utility_delta,
                proposal.seller_utility_delta,
                proposal.market_value_gap,
                proposal.buyer_season_points_delta,
                proposal.seller_season_points_delta
            );
            let _ = writeln!(
                rendered,
                "   cap-space delta (millions) {} / {}\n",
                proposal
                    .buyer_cap_space_delta
                    .map_or_else(|| "unknown".to_owned(), |value| format!("{value:+.2}")),
                proposal
                    .seller_cap_space_delta
                    .map_or_else(|| "unknown".to_owned(), |value| format!("{value:+.2}"))
            );
            if let Some(impact) = &proposal.season_forecast_impact {
                let _ = writeln!(
                    rendered,
                    "   paired season: {} {:+.2} pts / {:+.2}pp playoffs / {:+.2}pp Cup; {} {:+.2} pts / {:+.2}pp playoffs / {:+.2}pp Cup",
                    impact.buyer.team,
                    impact.buyer.average_points_delta,
                    impact.buyer.playoff_probability_delta * 100.0,
                    impact.buyer.stanley_cup_probability_delta * 100.0,
                    impact.seller.team,
                    impact.seller.average_points_delta,
                    impact.seller.playoff_probability_delta * 100.0,
                    impact.seller.stanley_cup_probability_delta * 100.0
                );
                let _ = writeln!(
                    rendered,
                    "   paired-vs-isolated buyer points residual {:+.2}\n",
                    impact.buyer_points_residual_vs_isolated
                );
            }
            if let Some(impact) = &proposal.lineup_impact {
                for lineup in [&impact.buyer, &impact.seller] {
                    let _ = writeln!(
                        rendered,
                        "   {} lineup strength {:+.2} · added [{}] · traded out [{}] · displaced [{}]",
                        lineup.team,
                        lineup.strength_delta,
                        lineup.added_to_lineup.join(", "),
                        lineup.explicitly_removed_from_lineup.join(", "),
                        lineup.displaced_by_competition.join(", ")
                    );
                }
                rendered.push('\n');
            }
            if !proposal.transaction_ready {
                let _ = writeln!(rendered, "   transaction authority incomplete\n");
            }
            for disclosure in &proposal.disclosures {
                let _ = writeln!(rendered, "   - {disclosure}");
            }
            rendered.push('\n');
        }
        for disclosure in &view.disclosures {
            let _ = writeln!(rendered, "- {disclosure}");
        }
        rendered
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "trade market evaluation")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub struct IceCastEdgeArgs {
    pub forecast: PathBuf,
    pub evidence: PathBuf,
    pub model: Option<PathBuf>,
    pub json: bool,
    pub out: Option<PathBuf>,
    pub enhanced_forecast_out: Option<PathBuf>,
}

pub struct IceCastEdgeTrainArgs {
    pub observations: PathBuf,
    pub config: Option<PathBuf>,
    pub registration: Option<PathBuf>,
    pub validate: bool,
    pub model_out: Option<PathBuf>,
    pub out: Option<PathBuf>,
}

pub struct IceCastBenchArgs {
    pub forecast: PathBuf,
    pub lineup: PathBuf,
    pub profile: PathBuf,
    pub style_evidence: PathBuf,
    pub stats_season: u32,
    pub scenario_out: Option<PathBuf>,
    pub json: bool,
    pub out: Option<PathBuf>,
}

pub struct IceCastBlenderArgs {
    pub lineup: PathBuf,
    pub pair_evidence: Option<PathBuf>,
    pub shift_season: Option<u32>,
    pub refresh_shifts: bool,
    pub shift_report_out: Option<PathBuf>,
    pub max_candidates: usize,
    pub allow_off_wing: bool,
    pub review_games: u8,
    pub minimum_points_percentage: f64,
    pub max_changes: u8,
    pub max_choices: usize,
    pub scenario_out: Option<PathBuf>,
    pub json: bool,
    pub out: Option<PathBuf>,
}

pub async fn run_behavior_rankings(
    target_season: u32,
    window: u8,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let view =
        fetch_team_behavior_league_evidence(&NhlApiClient::production(), target_season, window)
            .await
            .with_context(|| {
                format!("gather {window}-season league management behavior evidence")
            })?;
    let document = format!("{}\n", serde_json::to_string_pretty(&view)?);
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, document.as_bytes(), "management behavior rankings")?;
    }
    if json {
        print!("{document}");
    } else {
        println!(
            "Management behavior rankings: {} teams, {} seasons, {:.0}% average trait coverage",
            view.rankings.teams,
            view.window_seasons,
            view.rankings
                .coverage
                .iter()
                .map(|row| row.coverage_pct)
                .sum::<f64>()
                / view.rankings.coverage.len().max(1) as f64,
        );
        println!("Evidence rows: {}", view.season_evidence.len());
        println!("Ranked scales:");
        let mut traits = view
            .rankings
            .rows
            .iter()
            .filter(|row| row.rank == Some(1))
            .collect::<Vec<_>>();
        traits.sort_by(|a, b| a.trait_key.cmp(&b.trait_key));
        for row in traits {
            println!("  {:35} {}", row.trait_key, row.team);
        }
        if let Some(path) = out {
            println!("Full UI-neutral document: {}", path.display());
        }
    }
    Ok(())
}

pub fn run_behavior_research(
    rankings_path: PathBuf,
    research_path: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let rankings_bytes = std::fs::read(&rankings_path)
        .with_context(|| format!("read behavior rankings {}", rankings_path.display()))?;
    let league: icelines_fetch::TeamBehaviorLeagueEvidenceView =
        serde_json::from_slice(&rankings_bytes)
            .with_context(|| format!("parse behavior rankings {}", rankings_path.display()))?;
    let research_bytes = std::fs::read(&research_path)
        .with_context(|| format!("read behavior research {}", research_path.display()))?;
    let research: TeamBehaviorResearchInput = serde_json::from_slice(&research_bytes)
        .with_context(|| format!("parse behavior research {}", research_path.display()))?;
    let calibration = league
        .calibrations
        .iter()
        .find(|row| row.team == research.team)
        .with_context(|| format!("no {} calibration in league artifact", research.team))?;
    let view = apply_team_behavior_research(&calibration.profile, &research)
        .map_err(anyhow::Error::msg)?;
    let document = format!("{}\n", serde_json::to_string_pretty(&view)?);
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, document.as_bytes(), "leadership behavior research")?;
    }
    if json {
        print!("{document}");
    } else {
        println!("The Front Office research — {}", view.team);
        println!(
            "Active GM: {}",
            view.active_general_manager
                .as_ref()
                .map_or("NoRead", |row| row.person_name.as_str())
        );
        println!(
            "Active coach: {}",
            view.active_head_coach
                .as_ref()
                .map_or("NoRead", |row| row.person_name.as_str())
        );
        let accepted = view
            .marker_decisions
            .iter()
            .filter(|row| row.accepted)
            .count();
        println!(
            "Markers accepted: {accepted}/{}",
            view.marker_decisions.len()
        );
        if let Some(path) = out {
            println!("UI-neutral research profile: {}", path.display());
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn run_camp(
    input_path: PathBuf,
    trials: Option<u32>,
    seed: Option<u64>,
    json: bool,
    out: Option<PathBuf>,
    lineup_set_out: Option<PathBuf>,
    max_lineup_branches: usize,
    blender_set_out: Option<PathBuf>,
    season_scenario_out: Option<PathBuf>,
    season_max_roster_branches: usize,
    camp_max_candidates: usize,
) -> anyhow::Result<()> {
    let bytes = std::fs::read(&input_path)
        .with_context(|| format!("read The Cut input {}", input_path.display()))?;
    let mut input: TrainingCampSimulationInput = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse The Cut input {}", input_path.display()))?;
    if let Some(trials) = trials {
        input.config.trials = trials;
    }
    if let Some(seed) = seed {
        input.config.seed = seed;
    }
    let view = simulate_training_camp(&input).map_err(anyhow::Error::msg)?;
    if lineup_set_out.is_some() || blender_set_out.is_some() {
        let lineup_set = build_training_camp_lineup_set(&input, &view, max_lineup_branches)
            .map_err(anyhow::Error::msg)?;
        if let Some(path) = lineup_set_out.as_deref() {
            let bytes = serde_json::to_vec_pretty(&lineup_set)?;
            write_icecast_file(path, &bytes, "training camp lineup set")?;
        }
        if blender_set_out.is_some() {
            let blender_set = build_training_camp_blender_set(
                &lineup_set,
                LineCombinationForecastConfig {
                    max_candidates: camp_max_candidates,
                    allow_off_wing: true,
                },
            )
            .map_err(anyhow::Error::msg)?;
            if let Some(path) = blender_set_out.as_deref() {
                let bytes = serde_json::to_vec_pretty(&blender_set)?;
                write_icecast_file(path, &bytes, "training camp Blender set")?;
            }
        }
    }
    if let Some(path) = season_scenario_out.as_deref() {
        let season_lineups =
            build_training_camp_lineup_set(&input, &view, season_max_roster_branches)
                .map_err(anyhow::Error::msg)?;
        let policy = build_training_camp_opening_roster_policy(
            &season_lineups,
            LineCombinationForecastConfig {
                max_candidates: camp_max_candidates,
                allow_off_wing: true,
            },
        )
        .map_err(anyhow::Error::msg)?;
        let scenario = TeamSeasonScenario {
            name: format!("The Cut — {} opening roster", view.team),
            trade_deadline: None,
            events: Vec::new(),
            adaptive_lineup_policies: Vec::new(),
            opening_roster_policies: vec![policy],
        };
        let bytes = serde_json::to_vec_pretty(&scenario)?;
        write_icecast_file(path, &bytes, "training camp season scenario")?;
    }
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_camp(&view)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "training camp forecast")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

#[derive(Debug, Clone, Deserialize)]
struct LeagueRosterIdentity {
    player_id: u32,
    full_name: String,
    nhl_team: String,
    position: String,
    birth_date: Option<String>,
}

#[allow(clippy::too_many_arguments)]
pub fn run_camp_league(
    rosters_path: PathBuf,
    bios_path: PathBuf,
    stats_path: PathBuf,
    goalie_stats_path: PathBuf,
    candidate_overlay_path: Option<PathBuf>,
    authored_paths: Vec<PathBuf>,
    season: u32,
    trials: u32,
    seed: u64,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let roster_map: BTreeMap<String, LeagueRosterIdentity> = serde_json::from_slice(
        &std::fs::read(&rosters_path)
            .with_context(|| format!("read league camp rosters {}", rosters_path.display()))?,
    )
    .with_context(|| format!("parse league camp rosters {}", rosters_path.display()))?;
    let bios: Vec<SkaterBio> = serde_json::from_slice(
        &std::fs::read(&bios_path)
            .with_context(|| format!("read league camp bios {}", bios_path.display()))?,
    )?;
    let stats: Vec<SkaterStats> = serde_json::from_slice(
        &std::fs::read(&stats_path)
            .with_context(|| format!("read league camp stats {}", stats_path.display()))?,
    )?;
    let goalie_stats: Vec<GoalieStats> =
        serde_json::from_slice(&std::fs::read(&goalie_stats_path).with_context(|| {
            format!(
                "read league camp goalie stats {}",
                goalie_stats_path.display()
            )
        })?)?;
    let skater_stats = stats
        .iter()
        .map(|row| (row.player_id, row))
        .collect::<BTreeMap<_, _>>();
    let goalie_stats = goalie_stats
        .iter()
        .map(|row| (row.player_id, row))
        .collect::<BTreeMap<_, _>>();
    let overlay = if let Some(path) = candidate_overlay_path.as_deref() {
        let overlay: LeagueCampCandidateOverlay =
            serde_json::from_slice(&std::fs::read(path).with_context(|| {
                format!("read league camp candidate overlay {}", path.display())
            })?)
            .with_context(|| format!("parse league camp candidate overlay {}", path.display()))?;
        validate_league_camp_candidate_overlay(&overlay).map_err(anyhow::Error::msg)?;
        Some(overlay)
    } else {
        None
    };
    let mut authored = BTreeMap::new();
    for path in authored_paths {
        let mut input: TrainingCampSimulationInput = serde_json::from_slice(
            &std::fs::read(&path)
                .with_context(|| format!("read authored league camp input {}", path.display()))?,
        )
        .with_context(|| format!("parse authored league camp input {}", path.display()))?;
        input.config.trials = trials;
        input.config.seed = seed ^ team_seed(&input.team);
        let team = input.team.trim().to_ascii_uppercase();
        if authored.insert(team.clone(), input).is_some() {
            bail!("duplicate authored league camp input for {team}");
        }
    }

    // Resolve one authoritative organization per player before adding
    // prior-season fallback depth. Current roster identities are the floor;
    // dated candidate overlays supersede them, and authored camp pools have
    // final priority. This prevents traded players from surviving in their
    // prior club's automatic pool through stale bio team labels.
    let mut authoritative_team_by_player = BTreeMap::new();
    for identity in roster_map.values() {
        authoritative_team_by_player.insert(identity.player_id, identity.nhl_team.clone());
    }
    if let Some(overlay) = overlay.as_ref() {
        for candidate in &overlay.candidates {
            authoritative_team_by_player.insert(candidate.player_id, candidate.team.clone());
        }
    }
    for (team, simulation) in &authored {
        for player in &simulation.players {
            authoritative_team_by_player.insert(player.player_id, team.clone());
        }
    }

    let mut teams = Vec::new();
    for team in icelines_fetch::nhl_teams_for_season(&season.to_string()) {
        if let Some(simulation) = authored.remove(team) {
            teams.push(TrainingCampLeagueTeamInput {
                current_roster_candidates: simulation.players.len(),
                sourced_overlay_candidates: 0,
                fallback_candidates: 0,
                simulation,
                authority_status: TrainingCampAuthorityStatus::ConfirmedPool,
                competition_pool_status: TrainingCampCompetitionPoolStatus::Authored,
                authority_warnings: vec![
                    "Authored team camp pool replaces the automatic league pool".to_owned(),
                ],
            });
            continue;
        }
        let mut identities = roster_map
            .values()
            .filter(|player| {
                player.nhl_team.eq_ignore_ascii_case(team)
                    && authoritative_team_by_player
                        .get(&player.player_id)
                        .is_some_and(|assigned| assigned.eq_ignore_ascii_case(team))
            })
            .cloned()
            .collect::<Vec<_>>();
        identities.sort_by_key(|player| player.player_id);
        let current_roster_candidates = identities.len();
        let current_ids = identities
            .iter()
            .map(|player| player.player_id)
            .collect::<std::collections::BTreeSet<_>>();
        let mut sourced_overlay_urls = BTreeMap::new();
        if let Some(overlay) = overlay.as_ref() {
            for candidate in overlay
                .candidates
                .iter()
                .filter(|candidate| candidate.team.eq_ignore_ascii_case(team))
            {
                if current_ids.contains(&candidate.player_id) {
                    continue;
                }
                if authoritative_team_by_player
                    .get(&candidate.player_id)
                    .is_some_and(|assigned| !assigned.eq_ignore_ascii_case(team))
                {
                    continue;
                }
                identities.push(LeagueRosterIdentity {
                    player_id: candidate.player_id,
                    full_name: candidate.display_name.clone(),
                    nhl_team: team.to_owned(),
                    position: candidate.position.clone(),
                    birth_date: candidate.birth_date.clone(),
                });
                sourced_overlay_urls.insert(candidate.player_id, candidate.source_url.clone());
            }
        }
        identities.sort_by_key(|player| player.player_id);
        let sourced_overlay_candidates = sourced_overlay_urls.len();
        let opening_missing_before_fallback = camp_shape_missing(&identities);
        let mut authoritative_ids = current_ids.clone();
        authoritative_ids.extend(sourced_overlay_urls.keys().copied());
        let mut known_ids = authoritative_ids.clone();
        let mut fallback_candidates = 0usize;
        let invite_missing = camp_invite_pool_missing(&identities);
        if invite_missing != (0, 0, 0) {
            let mut fallback = bios
                .iter()
                .filter(|bio| {
                    bio.current_team_abbrev
                        .as_deref()
                        .is_some_and(|value| value.eq_ignore_ascii_case(team))
                        && !known_ids.contains(&bio.player_id)
                        && authoritative_team_by_player
                            .get(&bio.player_id)
                            .is_none_or(|assigned| assigned.eq_ignore_ascii_case(team))
                })
                .collect::<Vec<_>>();
            fallback.sort_by(|a, b| {
                b.games_played
                    .cmp(&a.games_played)
                    .then_with(|| a.player_id.cmp(&b.player_id))
            });
            for bio in fallback {
                if camp_invite_pool_missing(&identities) == (0, 0, 0) {
                    break;
                }
                let group_missing = camp_invite_group_missing(&identities, &bio.position_code);
                if !group_missing {
                    continue;
                }
                known_ids.insert(bio.player_id);
                identities.push(LeagueRosterIdentity {
                    player_id: bio.player_id,
                    full_name: bio.skater_full_name.clone(),
                    nhl_team: team.to_owned(),
                    position: bio.position_code.clone(),
                    birth_date: bio.birth_date.clone(),
                });
                fallback_candidates += 1;
            }
        }
        let remaining_missing = camp_shape_missing(&identities);
        let authority_status = if remaining_missing != (0, 0, 0) {
            TrainingCampAuthorityStatus::InsufficientAuthority
        } else if opening_missing_before_fallback != (0, 0, 0) {
            TrainingCampAuthorityStatus::DegradedFallback
        } else {
            TrainingCampAuthorityStatus::ConfirmedPool
        };
        let competition_pool_status = if remaining_missing != (0, 0, 0) {
            TrainingCampCompetitionPoolStatus::Thin
        } else if fallback_candidates > 0 {
            TrainingCampCompetitionPoolStatus::PriorSeasonAugmented
        } else {
            TrainingCampCompetitionPoolStatus::CurrentRosterOnly
        };
        let mut authority_warnings = match authority_status {
            TrainingCampAuthorityStatus::ConfirmedPool => Vec::new(),
            TrainingCampAuthorityStatus::DegradedFallback => vec![
                "The current roster snapshot plus sourced candidate overlay could not fill the opening 14F/7D/2G shape without prior-season organizational fallback candidates"
                    .to_owned(),
            ],
            TrainingCampAuthorityStatus::InsufficientAuthority => vec![format!(
                "Candidate pool remains short by {}F/{}D/{}G",
                remaining_missing.0, remaining_missing.1, remaining_missing.2
            )],
        };
        if fallback_candidates > 0 {
            authority_warnings.push(format!(
                "Added {fallback_candidates} prior-season organizational candidate(s) to expand the concept camp pool toward 17F/9D/3G"
            ));
        }
        if sourced_overlay_candidates > 0 {
            let checked_at = overlay
                .as_ref()
                .map(|overlay| overlay.checked_at.as_str())
                .unwrap_or("unknown");
            authority_warnings.push(format!(
                "Added {sourced_overlay_candidates} explicitly sourced organizational candidate(s) from the candidate overlay checked {checked_at}"
            ));
        }
        authority_warnings.push(
            "Automatic concept run uses GP-based pre-camp priors and age-based prospect/waiver estimates; it is not an authored scouting forecast"
                .to_owned(),
        );
        let players = identities
            .iter()
            .map(|identity| {
                automatic_camp_player(
                    identity,
                    authoritative_ids.contains(&identity.player_id),
                    sourced_overlay_urls
                        .get(&identity.player_id)
                        .map(String::as_str),
                    skater_stats.get(&identity.player_id).copied(),
                    goalie_stats.get(&identity.player_id).copied(),
                )
            })
            .collect::<Result<Vec<_>, String>>()
            .map_err(anyhow::Error::msg)?;
        teams.push(TrainingCampLeagueTeamInput {
            simulation: TrainingCampSimulationInput {
                team: team.to_owned(),
                season,
                config: TrainingCampConfig {
                    trials,
                    seed: seed ^ team_seed(team),
                    forward_slots: 14,
                    defense_slots: 7,
                    goalie_slots: 2,
                    minimum_centers: 4,
                    ..TrainingCampConfig::default()
                },
                decision_profile: None,
                players,
            },
            authority_status,
            competition_pool_status,
            current_roster_candidates,
            sourced_overlay_candidates,
            fallback_candidates,
            authority_warnings,
        });
    }
    if !authored.is_empty() {
        bail!(
            "authored league camp inputs contain teams outside the selected season: {}",
            authored.keys().cloned().collect::<Vec<_>>().join(", ")
        );
    }
    let view = simulate_training_camp_league(&TrainingCampLeagueSimulationInput { season, teams })
        .map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_camp_league(&view)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "league training camp forecast")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_bubble(
    input_path: PathBuf,
    transaction_context_path: Option<PathBuf>,
    top: usize,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let league: TrainingCampLeagueForecastView = serde_json::from_slice(
        &std::fs::read(&input_path)
            .with_context(|| format!("read Bubble input {}", input_path.display()))?,
    )
    .with_context(|| format!("parse Bubble input {}", input_path.display()))?;
    let transaction_context = transaction_context_path
        .as_deref()
        .map(|path| {
            serde_json::from_slice::<TrainingCampTransactionContextInput>(
                &std::fs::read(path).with_context(|| {
                    format!("read Bubble transaction context {}", path.display())
                })?,
            )
            .with_context(|| format!("parse Bubble transaction context {}", path.display()))
        })
        .transpose()?;
    let view =
        build_training_camp_exposure_board_with_context(&league, top, transaction_context.as_ref())
            .map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_bubble(&view)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "training camp exposure board")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate(input_path: PathBuf, json: bool, out: Option<PathBuf>) -> anyhow::Result<()> {
    let input: AhlAffiliateProjectionInput = serde_json::from_slice(
        &std::fs::read(&input_path)
            .with_context(|| format!("read affiliate input {}", input_path.display()))?,
    )
    .with_context(|| format!("parse affiliate input {}", input_path.display()))?;
    let view = build_ahl_affiliate_projection(&input).map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_affiliate(&view)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL affiliate projection")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn run_affiliate_identities(
    snapshot_path: PathBuf,
    team: String,
    candidates_path: Option<PathBuf>,
    discover_official: bool,
    refresh: bool,
    json: bool,
    out: Option<PathBuf>,
    cfg: &Config,
) -> anyhow::Result<()> {
    let snapshot: AhlRosterStatsSnapshot =
        read_icecast_json(&snapshot_path, "AHL roster/stat snapshot")?;
    let mut catalogs = Vec::new();
    if let Some(candidates_path) = candidates_path.as_deref() {
        catalogs.push(read_affiliate_identity_catalog(candidates_path)?);
    }
    let mut discovery_note = None;
    if discover_official {
        let (catalog, exact_search_candidates, surname_search_candidates, landing_enriched) =
            discover_official_affiliate_identities(&snapshot, &team, refresh, cfg).await?;
        catalogs.push(catalog);
        discovery_note = Some(format!(
            "Official NHL discovery proposed {exact_search_candidates} exact-name candidate(s) and {surname_search_candidates} surname fallback candidate(s); {landing_enriched} received player-landing birth-date corroboration. All proposals remain pending explicit review."
        ));
    }
    if catalogs.is_empty() {
        bail!("affiliate identity review requires --candidates or --discover-official");
    }
    let checked_at = Utc::now().date_naive().to_string();
    let candidates =
        merge_ahl_canonical_identity_catalogs(checked_at, &catalogs).map_err(anyhow::Error::msg)?;
    let mut view =
        build_ahl_identity_crosswalk(&snapshot, &team, &candidates).map_err(anyhow::Error::msg)?;
    if let Some(note) = discovery_note {
        view.disclosures.push(note);
    }
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        let inspection =
            build_ahl_identity_review_inspection(&view, AhlIdentityInspectionScope::All)
                .map_err(anyhow::Error::msg)?;
        render_affiliate_identities(&inspection)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL identity review")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub async fn run_affiliate_identities_league(
    snapshot_path: PathBuf,
    candidates_path: Option<PathBuf>,
    discover_official: bool,
    refresh: bool,
    as_of: Option<String>,
    max_age: Option<u8>,
    json: bool,
    out: Option<PathBuf>,
    cfg: &Config,
) -> anyhow::Result<()> {
    let snapshot: AhlRosterStatsSnapshot =
        read_icecast_json(&snapshot_path, "AHL roster/stat snapshot")?;
    snapshot.validate().map_err(anyhow::Error::msg)?;
    let mut catalogs = Vec::new();
    if let Some(candidates_path) = candidates_path.as_deref() {
        catalogs.push(read_affiliate_identity_catalog(candidates_path)?);
    }
    let mut discovery_note = None;
    if discover_official {
        let age_policy = match (as_of.as_deref(), max_age) {
            (Some(value), Some(max_age)) => Some((
                NaiveDate::parse_from_str(value, "%Y-%m-%d").with_context(|| {
                    format!("invalid AHL identity discovery as-of date `{value}`")
                })?,
                max_age,
            )),
            (None, None) => None,
            _ => bail!("AHL identity discovery age policy requires both --as-of and --max-age"),
        };
        let teams = snapshot
            .teams
            .iter()
            .map(|team| team.team_name.clone())
            .collect::<Vec<_>>();
        let (catalog, exact_search_candidates, surname_search_candidates, landing_enriched) =
            discover_official_affiliate_identities_for_teams(
                &snapshot, &teams, refresh, cfg, age_policy,
            )
            .await?;
        catalogs.push(catalog);
        let age_note = age_policy.map_or_else(String::new, |(as_of, max_age)| {
            format!(" Discovery was restricted to players age {max_age} or younger on {as_of}.")
        });
        discovery_note = Some(format!(
            "Deduplicated official NHL discovery across {} AHL team(s) proposed {exact_search_candidates} exact-name candidate(s) and {surname_search_candidates} surname fallback candidate(s); {landing_enriched} received player-landing birth-date corroboration. All proposals remain pending explicit review.",
            teams.len()
        ) + &age_note);
    }
    if catalogs.is_empty() {
        bail!("league affiliate identity review requires --candidates or --discover-official");
    }
    let candidates =
        merge_ahl_canonical_identity_catalogs(Utc::now().date_naive().to_string(), &catalogs)
            .map_err(anyhow::Error::msg)?;
    let mut view =
        build_ahl_identity_league_crosswalk(&snapshot, &candidates).map_err(anyhow::Error::msg)?;
    if let Some(note) = discovery_note {
        for crosswalk in &mut view.crosswalks {
            crosswalk.disclosures.push(note.clone());
        }
        view.disclosures.push(note);
    }
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        let review =
            build_ahl_identity_league_review(&view.crosswalks).map_err(anyhow::Error::msg)?;
        render_affiliate_identity_league(&review)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL league identity crosswalk")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_review_draft(
    crosswalk_path: PathBuf,
    include_aliases: bool,
    include_conflicts: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let crosswalk: AhlIdentityCrosswalkView =
        read_icecast_json(&crosswalk_path, "AHL identity crosswalk")?;
    let draft = build_ahl_identity_review_draft_with_options(
        &crosswalk,
        AhlIdentityReviewDraftOptions {
            include_aliases,
            include_conflicts,
        },
    )
    .map_err(anyhow::Error::msg)?;
    let output = format!("{}\n", serde_json::to_string_pretty(&draft)?);
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL identity review draft")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_review_draft_league(
    league_crosswalk_path: PathBuf,
    include_aliases: bool,
    include_conflicts: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let league: AhlIdentityLeagueCrosswalkView =
        read_icecast_json(&league_crosswalk_path, "AHL league identity crosswalk")?;
    let draft = build_ahl_identity_league_review_draft(
        &league,
        AhlIdentityReviewDraftOptions {
            include_aliases,
            include_conflicts,
        },
    )
    .map_err(anyhow::Error::msg)?;
    let output = format!("{}\n", serde_json::to_string_pretty(&draft)?);
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL league identity review draft")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_review_exact(
    crosswalk_path: PathBuf,
    reviewer: String,
    reviewed_at: String,
    decisions_out: Option<PathBuf>,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let crosswalk: AhlIdentityCrosswalkView =
        read_icecast_json(&crosswalk_path, "AHL identity crosswalk")?;
    let decisions = build_ahl_exact_identity_review(&crosswalk, reviewer, reviewed_at)
        .map_err(anyhow::Error::msg)?;
    if let Some(path) = decisions_out.as_deref() {
        let bytes = format!("{}\n", serde_json::to_string_pretty(&decisions)?);
        write_icecast_file(
            path,
            bytes.as_bytes(),
            "exact AHL identity review decisions",
        )?;
    }
    let reviewed =
        apply_ahl_identity_review_decisions(&crosswalk, &decisions).map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&reviewed)?)
    } else {
        let inspection =
            build_ahl_identity_review_inspection(&reviewed, AhlIdentityInspectionScope::All)
                .map_err(anyhow::Error::msg)?;
        render_affiliate_identities(&inspection)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(
            path,
            output.as_bytes(),
            "exact-reviewed AHL identity crosswalk",
        )?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_review_aliases(
    crosswalk_path: PathBuf,
    reviewer: String,
    reviewed_at: String,
    decisions_out: Option<PathBuf>,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let crosswalk: AhlIdentityCrosswalkView =
        read_icecast_json(&crosswalk_path, "AHL identity crosswalk")?;
    let decisions = build_ahl_alias_identity_review(&crosswalk, reviewer, reviewed_at)
        .map_err(anyhow::Error::msg)?;
    if let Some(path) = decisions_out.as_deref() {
        let bytes = format!("{}\n", serde_json::to_string_pretty(&decisions)?);
        write_icecast_file(
            path,
            bytes.as_bytes(),
            "alias AHL identity review decisions",
        )?;
    }
    let reviewed =
        apply_ahl_identity_review_decisions(&crosswalk, &decisions).map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&reviewed)?)
    } else {
        let inspection =
            build_ahl_identity_review_inspection(&reviewed, AhlIdentityInspectionScope::All)
                .map_err(anyhow::Error::msg)?;
        render_affiliate_identities(&inspection)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(
            path,
            output.as_bytes(),
            "alias-reviewed AHL identity crosswalk",
        )?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_review_exact_league(
    league_crosswalk_path: PathBuf,
    reviewer: String,
    reviewed_at: String,
    decisions_out: Option<PathBuf>,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    run_affiliate_review_routine_league(
        league_crosswalk_path,
        AhlIdentityLeagueRoutineReviewKind::Exact,
        reviewer,
        reviewed_at,
        decisions_out,
        json,
        out,
    )
}

pub fn run_affiliate_review_aliases_league(
    league_crosswalk_path: PathBuf,
    reviewer: String,
    reviewed_at: String,
    decisions_out: Option<PathBuf>,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    run_affiliate_review_routine_league(
        league_crosswalk_path,
        AhlIdentityLeagueRoutineReviewKind::Aliases,
        reviewer,
        reviewed_at,
        decisions_out,
        json,
        out,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn run_affiliate_review_conflicts_league(
    league_crosswalk_path: PathBuf,
    nhl_player_ids: Vec<u32>,
    evidence_urls: Vec<String>,
    reviewer: String,
    reviewed_at: String,
    note: String,
    decisions_out: Option<PathBuf>,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let league: AhlIdentityLeagueCrosswalkView =
        read_icecast_json(&league_crosswalk_path, "AHL league identity crosswalk")?;
    let (reviewed, decisions) = apply_ahl_identity_league_conflict_review(
        &league,
        &nhl_player_ids,
        &evidence_urls,
        reviewer,
        reviewed_at,
        note,
    )
    .map_err(anyhow::Error::msg)?;
    if let Some(path) = decisions_out.as_deref() {
        let bytes = format!("{}\n", serde_json::to_string_pretty(&decisions)?);
        write_icecast_file(
            path,
            bytes.as_bytes(),
            "AHL league identity conflict review decisions",
        )?;
    }
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&reviewed)?)
    } else {
        let review =
            build_ahl_identity_league_review(&reviewed.crosswalks).map_err(anyhow::Error::msg)?;
        render_affiliate_identity_league(&review)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(
            path,
            output.as_bytes(),
            "conflict-reviewed AHL league identity crosswalk",
        )?;
    } else {
        print!("{output}");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn run_affiliate_review_birth_date_league(
    league_crosswalk_path: PathBuf,
    nhl_player_id: u32,
    canonical_birth_date: String,
    evidence_urls: Vec<String>,
    reviewer: String,
    reviewed_at: String,
    note: String,
    decisions_out: Option<PathBuf>,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let league: AhlIdentityLeagueCrosswalkView =
        read_icecast_json(&league_crosswalk_path, "AHL league identity crosswalk")?;
    let (reviewed, decisions) = apply_ahl_identity_league_birth_date_correction(
        &league,
        nhl_player_id,
        canonical_birth_date,
        &evidence_urls,
        reviewer,
        reviewed_at,
        note,
    )
    .map_err(anyhow::Error::msg)?;
    if let Some(path) = decisions_out.as_deref() {
        let bytes = format!("{}\n", serde_json::to_string_pretty(&decisions)?);
        write_icecast_file(
            path,
            bytes.as_bytes(),
            "AHL league identity birth-date correction decisions",
        )?;
    }
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&reviewed)?)
    } else {
        let review =
            build_ahl_identity_league_review(&reviewed.crosswalks).map_err(anyhow::Error::msg)?;
        render_affiliate_identity_league(&review)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(
            path,
            output.as_bytes(),
            "birth-date-corrected AHL league identity crosswalk",
        )?;
    } else {
        print!("{output}");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn run_affiliate_review_collision_league(
    league_crosswalk_path: PathBuf,
    proposed_nhl_player_id: u32,
    canonical_nhl_player_id: u32,
    canonical_name: String,
    canonical_birth_date: String,
    evidence_urls: Vec<String>,
    reviewer: String,
    reviewed_at: String,
    note: String,
    decisions_out: Option<PathBuf>,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let league: AhlIdentityLeagueCrosswalkView =
        read_icecast_json(&league_crosswalk_path, "AHL league identity crosswalk")?;
    let (reviewed, decisions) = apply_ahl_identity_league_collision_remap(
        &league,
        proposed_nhl_player_id,
        canonical_nhl_player_id,
        canonical_name,
        canonical_birth_date,
        &evidence_urls,
        reviewer,
        reviewed_at,
        note,
    )
    .map_err(anyhow::Error::msg)?;
    if let Some(path) = decisions_out.as_deref() {
        let bytes = format!("{}\n", serde_json::to_string_pretty(&decisions)?);
        write_icecast_file(
            path,
            bytes.as_bytes(),
            "AHL league identity collision-remap decisions",
        )?;
    }
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&reviewed)?)
    } else {
        let review =
            build_ahl_identity_league_review(&reviewed.crosswalks).map_err(anyhow::Error::msg)?;
        render_affiliate_identity_league(&review)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(
            path,
            output.as_bytes(),
            "collision-remapped AHL league identity crosswalk",
        )?;
    } else {
        print!("{output}");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_affiliate_review_routine_league(
    league_crosswalk_path: PathBuf,
    kind: AhlIdentityLeagueRoutineReviewKind,
    reviewer: String,
    reviewed_at: String,
    decisions_out: Option<PathBuf>,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let league: AhlIdentityLeagueCrosswalkView =
        read_icecast_json(&league_crosswalk_path, "AHL league identity crosswalk")?;
    let (reviewed, decisions) =
        apply_ahl_identity_league_routine_review(&league, kind, reviewer, reviewed_at)
            .map_err(anyhow::Error::msg)?;
    if let Some(path) = decisions_out.as_deref() {
        let bytes = format!("{}\n", serde_json::to_string_pretty(&decisions)?);
        write_icecast_file(
            path,
            bytes.as_bytes(),
            "AHL league identity review decisions",
        )?;
    }
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&reviewed)?)
    } else {
        let review =
            build_ahl_identity_league_review(&reviewed.crosswalks).map_err(anyhow::Error::msg)?;
        render_affiliate_identity_league(&review)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(
            path,
            output.as_bytes(),
            "reviewed AHL league identity crosswalk",
        )?;
    } else {
        print!("{output}");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn run_affiliate_review_reject(
    crosswalk_path: PathBuf,
    provider_player_ids: Vec<String>,
    evidence_urls: Vec<String>,
    reviewer: String,
    reviewed_at: String,
    note: String,
    decisions_out: Option<PathBuf>,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let crosswalk: AhlIdentityCrosswalkView =
        read_icecast_json(&crosswalk_path, "AHL identity crosswalk")?;
    let decisions = build_ahl_identity_rejection_review(
        &crosswalk,
        &provider_player_ids,
        &evidence_urls,
        reviewer,
        reviewed_at,
        note,
    )
    .map_err(anyhow::Error::msg)?;
    if let Some(path) = decisions_out.as_deref() {
        let bytes = format!("{}\n", serde_json::to_string_pretty(&decisions)?);
        write_icecast_file(
            path,
            bytes.as_bytes(),
            "rejected AHL identity mapping decisions",
        )?;
    }
    let reviewed =
        apply_ahl_identity_review_decisions(&crosswalk, &decisions).map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&reviewed)?)
    } else {
        let inspection =
            build_ahl_identity_review_inspection(&reviewed, AhlIdentityInspectionScope::All)
                .map_err(anyhow::Error::msg)?;
        render_affiliate_identities(&inspection)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(
            path,
            output.as_bytes(),
            "rejection-reviewed AHL identity crosswalk",
        )?;
    } else {
        print!("{output}");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn run_affiliate_review_reject_league(
    league_crosswalk_path: PathBuf,
    provider_player_ids: Vec<String>,
    evidence_urls: Vec<String>,
    reviewer: String,
    reviewed_at: String,
    note: String,
    decisions_out: Option<PathBuf>,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let league: AhlIdentityLeagueCrosswalkView =
        read_icecast_json(&league_crosswalk_path, "AHL league identity crosswalk")?;
    let (reviewed, decisions) = apply_ahl_identity_league_rejection_review(
        &league,
        &provider_player_ids,
        &evidence_urls,
        reviewer,
        reviewed_at,
        note,
    )
    .map_err(anyhow::Error::msg)?;
    if let Some(path) = decisions_out.as_deref() {
        let bytes = format!("{}\n", serde_json::to_string_pretty(&decisions)?);
        write_icecast_file(
            path,
            bytes.as_bytes(),
            "AHL league identity rejection decisions",
        )?;
    }
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&reviewed)?)
    } else {
        let review =
            build_ahl_identity_league_review(&reviewed.crosswalks).map_err(anyhow::Error::msg)?;
        render_affiliate_identity_league(&review)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(
            path,
            output.as_bytes(),
            "rejection-reviewed AHL league identity crosswalk",
        )?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_review_league(
    crosswalk_paths: Vec<PathBuf>,
    league_crosswalk_paths: Vec<PathBuf>,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    if crosswalk_paths.is_empty() && league_crosswalk_paths.is_empty() {
        bail!("affiliate league review requires --crosswalk or --league-crosswalk");
    }
    let mut crosswalks = crosswalk_paths
        .iter()
        .map(|path| read_icecast_json(path, "AHL identity crosswalk"))
        .collect::<anyhow::Result<Vec<AhlIdentityCrosswalkView>>>()?;
    for path in &league_crosswalk_paths {
        let envelope: AhlIdentityLeagueCrosswalkView =
            read_icecast_json(path, "AHL league identity crosswalk")?;
        crosswalks.extend(envelope.crosswalks);
    }
    let view = build_ahl_identity_league_review(&crosswalks).map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_affiliate_identity_league(&view)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL league identity review")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_review_board(
    review_path: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let review: AhlIdentityLeagueReviewView =
        read_icecast_json(&review_path, "AHL league identity review")?;
    let board = build_ahl_identity_exception_board(&review).map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&board)?)
    } else {
        render_affiliate_identity_exception_board(&board)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL identity exception board")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_review_show(
    crosswalk_path: PathBuf,
    attention_only: bool,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let crosswalk: AhlIdentityCrosswalkView =
        read_icecast_json(&crosswalk_path, "AHL identity crosswalk")?;
    if crosswalk.schema != AHL_IDENTITY_CROSSWALK_SCHEMA {
        bail!(
            "unsupported AHL identity crosswalk schema `{}`",
            crosswalk.schema
        );
    }
    let scope = if attention_only {
        AhlIdentityInspectionScope::Attention
    } else {
        AhlIdentityInspectionScope::All
    };
    let inspection =
        build_ahl_identity_review_inspection(&crosswalk, scope).map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&inspection)?)
    } else {
        render_affiliate_identities(&inspection)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL identity review inspection")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_review_apply(
    crosswalk_path: PathBuf,
    decisions_path: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let crosswalk: AhlIdentityCrosswalkView =
        read_icecast_json(&crosswalk_path, "AHL identity crosswalk")?;
    let decisions: AhlIdentityReviewDecisions =
        read_icecast_json(&decisions_path, "AHL identity review decisions")?;
    let reviewed =
        apply_ahl_identity_review_decisions(&crosswalk, &decisions).map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&reviewed)?)
    } else {
        let inspection =
            build_ahl_identity_review_inspection(&reviewed, AhlIdentityInspectionScope::All)
                .map_err(anyhow::Error::msg)?;
        render_affiliate_identities(&inspection)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "reviewed AHL identity crosswalk")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_status_draft(
    prior_snapshot_path: PathBuf,
    crosswalk_path: PathBuf,
    camp_path: PathBuf,
    nhl_team: String,
    ahl_team: String,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let snapshot: AhlRosterStatsSnapshot =
        read_icecast_json(&prior_snapshot_path, "prior AHL roster/stat snapshot")?;
    let crosswalk: AhlIdentityCrosswalkView =
        read_icecast_json(&crosswalk_path, "AHL identity crosswalk")?;
    let camp: TrainingCampSimulationInput =
        read_icecast_json(&camp_path, "current training camp input")?;
    let draft = build_ahl_preseason_organization_review_draft(
        &snapshot, &crosswalk, &camp, &nhl_team, &ahl_team,
    )
    .map_err(anyhow::Error::msg)?;
    let output = format!("{}\n", serde_json::to_string_pretty(&draft)?);
    if let Some(path) = out.as_deref() {
        write_icecast_file(
            path,
            output.as_bytes(),
            "AHL organization-status review draft",
        )?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_status_draft_league(
    prior_snapshot_path: PathBuf,
    league_crosswalk_path: PathBuf,
    camp_forecast_path: PathBuf,
    config_path: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let prior_snapshot: AhlRosterStatsSnapshot =
        read_icecast_json(&prior_snapshot_path, "prior AHL roster/stat snapshot")?;
    let league_crosswalk: AhlIdentityLeagueCrosswalkView = read_icecast_json(
        &league_crosswalk_path,
        "reviewed AHL league identity crosswalk",
    )?;
    let camp_forecast: TrainingCampLeagueForecastView =
        read_icecast_json(&camp_forecast_path, "league training camp forecast")?;
    let config: AhlPreseasonLeagueRolloverConfig =
        read_icecast_json(&config_path, "AHL league preseason rollover config")?;
    let review = build_ahl_preseason_league_organization_review_draft(
        &prior_snapshot,
        &league_crosswalk,
        &camp_forecast,
        &config,
    )
    .map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&review)?)
    } else {
        render_affiliate_status_review_league(&review)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(
            path,
            output.as_bytes(),
            "AHL league organization-status review",
        )?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_status_show(
    review_path: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let review: AhlPreseasonOrganizationReview =
        read_icecast_json(&review_path, "AHL organization-status review")?;
    if review.schema != AHL_PRESEASON_ORGANIZATION_REVIEW_SCHEMA {
        bail!(
            "unsupported AHL organization-status review schema `{}`",
            review.schema
        );
    }
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&review)?)
    } else {
        render_affiliate_status_review(&review)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL organization-status review")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_status_evidence(
    review_path: PathBuf,
    career_history_path: PathBuf,
    as_of: String,
    maximum_fact_age_days: u32,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let review: AhlPreseasonLeagueOrganizationReview =
        read_icecast_json(&review_path, "AHL league organization-status review")?;
    let career_store = CareerHistoryStore::load(&career_history_path)
        .with_context(|| format!("read career history {}", career_history_path.display()))?;
    let ledger =
        build_ahl_organization_status_ledger(&review, &career_store, as_of, maximum_fact_age_days)
            .map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&ledger)?)
    } else {
        format!(
            "AHL organization status evidence\nRequired: {}\nResolved: {} ({} retained, {} departed)\nUnresolved: {}\n",
            ledger.counts.decisions_required,
            ledger.counts.resolved,
            ledger.counts.retained,
            ledger.counts.departed,
            ledger.counts.unresolved
        )
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL organization-status evidence")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_status_evidence_apply(
    review_path: PathBuf,
    ledger_path: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let review: AhlPreseasonLeagueOrganizationReview =
        read_icecast_json(&review_path, "AHL league organization-status review")?;
    let ledger: AhlOrganizationStatusLedgerView =
        read_icecast_json(&ledger_path, "AHL organization-status evidence ledger")?;
    let application =
        apply_ahl_organization_status_ledger(&review, &ledger).map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&application)?)
    } else {
        format!(
            "AHL organization status evidence applied\nApplied: {}\nRemaining: {}\nReview remains draft: {}\n",
            application.decisions_applied,
            application.decisions_remaining,
            application.review.draft
        )
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(
            path,
            output.as_bytes(),
            "AHL organization-status evidence application",
        )?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_transaction_state(
    transactions_path: PathBuf,
    league_crosswalk_path: PathBuf,
    affiliations_path: PathBuf,
    cutoff: String,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let transactions: AhlTransactionSnapshot =
        read_icecast_json(&transactions_path, "official AHL transaction snapshot")?;
    let identities: AhlIdentityLeagueCrosswalkView = read_icecast_json(
        &league_crosswalk_path,
        "reviewed AHL league identity crosswalk",
    )?;
    let affiliations: AhlAffiliationCatalogView =
        read_icecast_json(&affiliations_path, "dated AHL affiliation catalog")?;
    let ledger =
        build_ahl_transaction_state_ledger(&transactions, &identities, &affiliations, cutoff)
            .map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&ledger)?)
    } else {
        render_affiliate_transaction_state(&ledger)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL transaction-state ledger")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_transaction_state_apply(
    workboard_path: PathBuf,
    ledger_path: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let workboard = read_affiliate_workboard(&workboard_path)?;
    let ledger: AhlTransactionStateLedgerView =
        read_icecast_json(&ledger_path, "AHL transaction-state ledger")?;
    let application =
        apply_ahl_transaction_state_ledger(&workboard, &ledger).map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&application)?)
    } else {
        render_affiliate_transaction_state_application(&application)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL transaction-state application")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_waivers_draft(
    workboard_path: PathBuf,
    cutoff: String,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let workboard = read_affiliate_workboard(&workboard_path)?;
    let review =
        build_ahl_waiver_clearance_review_draft(&workboard, cutoff).map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&review)?)
    } else {
        render_affiliate_waiver_review(&review)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL waiver review draft")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_waivers_finalize(
    draft_path: PathBuf,
    decisions_path: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let draft: AhlWaiverClearanceReviewView =
        read_icecast_json(&draft_path, "AHL waiver review draft")?;
    let decisions: AhlWaiverClearanceDecisionsView =
        read_icecast_json(&decisions_path, "AHL waiver review decisions")?;
    let review =
        finalize_ahl_waiver_clearance_review(&draft, &decisions).map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&review)?)
    } else {
        render_affiliate_waiver_review(&review)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "final AHL waiver review")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_waivers_apply(
    workboard_path: PathBuf,
    review_path: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let workboard = read_affiliate_workboard(&workboard_path)?;
    let review: AhlWaiverClearanceReviewView =
        read_icecast_json(&review_path, "final AHL waiver review")?;
    let application =
        apply_ahl_waiver_clearance_review(&workboard, &review).map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&application)?)
    } else {
        render_affiliate_waiver_application(&application)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL waiver application")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_status_apply(
    prior_snapshot_path: PathBuf,
    crosswalk_path: PathBuf,
    camp_path: PathBuf,
    review_path: PathBuf,
    config_path: PathBuf,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let snapshot: AhlRosterStatsSnapshot =
        read_icecast_json(&prior_snapshot_path, "prior AHL roster/stat snapshot")?;
    let crosswalk: AhlIdentityCrosswalkView =
        read_icecast_json(&crosswalk_path, "reviewed AHL identity crosswalk")?;
    let camp: TrainingCampSimulationInput =
        read_icecast_json(&camp_path, "current training camp input")?;
    let review: AhlPreseasonOrganizationReview =
        read_icecast_json(&review_path, "AHL organization-status review")?;
    let config: AhlPreseasonRolloverConfig =
        read_icecast_json(&config_path, "AHL preseason rollover config")?;
    let applied =
        apply_ahl_preseason_organization_review(&snapshot, &crosswalk, &camp, &config, &review)
            .map_err(anyhow::Error::msg)?;
    let output = format!("{}\n", serde_json::to_string_pretty(&applied)?);
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "sourced AHL rollover config")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_status_apply_league(
    prior_snapshot_path: PathBuf,
    league_crosswalk_path: PathBuf,
    camp_forecast_path: PathBuf,
    review_path: PathBuf,
    config_path: PathBuf,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let prior_snapshot: AhlRosterStatsSnapshot =
        read_icecast_json(&prior_snapshot_path, "prior AHL roster/stat snapshot")?;
    let league_crosswalk: AhlIdentityLeagueCrosswalkView = read_icecast_json(
        &league_crosswalk_path,
        "reviewed AHL league identity crosswalk",
    )?;
    let camp_forecast: TrainingCampLeagueForecastView =
        read_icecast_json(&camp_forecast_path, "league training camp forecast")?;
    let review: AhlPreseasonLeagueOrganizationReview =
        read_icecast_json(&review_path, "AHL league organization-status review")?;
    let config: AhlPreseasonLeagueRolloverConfig =
        read_icecast_json(&config_path, "AHL league preseason rollover config")?;
    let applied = apply_ahl_preseason_league_organization_review(
        &prior_snapshot,
        &league_crosswalk,
        &camp_forecast,
        &config,
        &review,
    )
    .map_err(anyhow::Error::msg)?;
    let output = format!("{}\n", serde_json::to_string_pretty(&applied)?);
    if let Some(path) = out.as_deref() {
        write_icecast_file(
            path,
            output.as_bytes(),
            "sourced AHL league rollover config",
        )?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_professional_games(
    league_crosswalk_path: PathBuf,
    career_history_path: PathBuf,
    policy_path: PathBuf,
    camp_forecast_path: Option<PathBuf>,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let league_crosswalk: AhlIdentityLeagueCrosswalkView = read_icecast_json(
        &league_crosswalk_path,
        "reviewed AHL league identity crosswalk",
    )?;
    let career_store = CareerHistoryStore::load(&career_history_path)
        .with_context(|| format!("read career history {}", career_history_path.display()))?;
    let policy: AhlProfessionalGamePolicy =
        read_icecast_json(&policy_path, "AHL professional-game policy")?;
    let ledger = if let Some(path) = camp_forecast_path.as_deref() {
        let camp: TrainingCampLeagueForecastView =
            read_icecast_json(path, "league training camp forecast")?;
        if camp.schema != icelines_core::TRAINING_CAMP_LEAGUE_FORECAST_SCHEMA
            || camp.season != policy.target_season
            || camp.teams_requested != CANONICAL_TEAMS.len()
            || camp.teams.len() != camp.teams_requested
            || camp.teams_failed != 0
        {
            bail!(
                "professional-game camp completion requires a complete canonical target-season forecast"
            );
        }
        let mut candidates = BTreeMap::<u32, String>::new();
        for player in camp
            .teams
            .iter()
            .filter_map(|team| team.forecast.as_ref())
            .flat_map(|forecast| &forecast.players)
        {
            if let Some(existing) = candidates.insert(player.player_id, player.display_name.clone()) {
                if !existing.eq_ignore_ascii_case(&player.display_name) {
                    bail!(
                        "training camp player {} has conflicting canonical names",
                        player.player_id
                    );
                }
            }
        }
        let candidates = candidates
            .into_iter()
            .map(|(nhl_player_id, display_name)| AhlProfessionalGameCandidate {
                nhl_player_id,
                display_name,
            })
            .collect::<Vec<_>>();
        build_ahl_professional_game_ledger_with_candidates(
            &league_crosswalk,
            &career_store,
            &policy,
            &candidates,
        )
    } else {
        build_ahl_professional_game_ledger(&league_crosswalk, &career_store, &policy)
    }
    .map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&ledger)?)
    } else {
        render_affiliate_professional_games(&ledger)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL professional-game ledger")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_values(
    snapshot_path: PathBuf,
    league_crosswalk_path: PathBuf,
    policy_path: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let snapshot: AhlRosterStatsSnapshot =
        read_icecast_json(&snapshot_path, "official AHL roster/stats snapshot")?;
    let league_crosswalk: AhlIdentityLeagueCrosswalkView = read_icecast_json(
        &league_crosswalk_path,
        "reviewed AHL league identity crosswalk",
    )?;
    let policy: AhlPlayerValuePolicy = read_icecast_json(&policy_path, "AHL player-value policy")?;
    let ledger = build_ahl_player_value_ledger(&snapshot, &league_crosswalk, &policy)
        .map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&ledger)?)
    } else {
        render_affiliate_values(&ledger)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL player-value ledger")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_values_apply(
    workboard_path: PathBuf,
    ledger_path: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let workboard = read_affiliate_workboard(&workboard_path)?;
    let ledger: AhlPlayerValueLedgerView =
        read_icecast_json(&ledger_path, "AHL player-value ledger")?;
    let application =
        apply_ahl_player_value_ledger(&workboard, &ledger).map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&application)?)
    } else {
        render_affiliate_values_application(&application)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL player-value application")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_values_cross_league(
    workboard_path: PathBuf,
    career_history_path: PathBuf,
    policy_path: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let workboard = read_affiliate_workboard(&workboard_path)?;
    let career_store = CareerHistoryStore::load(&career_history_path)
        .with_context(|| format!("read career history {}", career_history_path.display()))?;
    let policy: AhlCrossLeagueValuePolicy =
        read_icecast_json(&policy_path, "AHL cross-league value policy")?;
    let ledger = build_ahl_cross_league_value_ledger(&workboard, &career_store, &policy)
        .map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&ledger)?)
    } else {
        render_affiliate_values_cross_league(&ledger)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL cross-league value ledger")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_values_cross_league_apply(
    workboard_path: PathBuf,
    ledger_path: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let workboard = read_affiliate_workboard(&workboard_path)?;
    let ledger: AhlCrossLeagueValueLedgerView =
        read_icecast_json(&ledger_path, "AHL cross-league value ledger")?;
    let application =
        apply_ahl_cross_league_value_ledger(&workboard, &ledger).map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&application)?)
    } else {
        render_affiliate_values_cross_league_application(&application)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(
            path,
            output.as_bytes(),
            "AHL cross-league value application",
        )?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_prospects(
    workboard_path: PathBuf,
    career_history_path: PathBuf,
    policy_path: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let workboard = read_affiliate_workboard(&workboard_path)?;
    let career_store = CareerHistoryStore::load(&career_history_path)
        .with_context(|| format!("read career history {}", career_history_path.display()))?;
    let policy: OrganizationalProspectPolicy =
        read_icecast_json(&policy_path, "organizational prospect policy")?;
    let ledger = build_ahl_prospect_status_ledger(&workboard, &career_store, &policy)
        .map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&ledger)?)
    } else {
        render_affiliate_prospects(&ledger)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL prospect-status ledger")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_prospects_apply(
    workboard_path: PathBuf,
    ledger_path: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let workboard = read_affiliate_workboard(&workboard_path)?;
    let ledger: AhlProspectStatusLedgerView =
        read_icecast_json(&ledger_path, "AHL prospect-status ledger")?;
    let application =
        apply_ahl_prospect_status_ledger(&workboard, &ledger).map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&application)?)
    } else {
        render_affiliate_prospects_application(&application)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL prospect-status application")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_readiness(
    workboard_path: PathBuf,
    career_history_path: PathBuf,
    camp_forecast_path: PathBuf,
    policy_path: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let workboard = read_affiliate_workboard(&workboard_path)?;
    let career_store = CareerHistoryStore::load(&career_history_path)
        .with_context(|| format!("read career history {}", career_history_path.display()))?;
    let camp_forecast: TrainingCampLeagueForecastView =
        read_icecast_json(&camp_forecast_path, "training-camp league forecast")?;
    let policy: AhlRecallReadinessPolicy =
        read_icecast_json(&policy_path, "AHL recall-readiness policy")?;
    let ledger =
        build_ahl_recall_readiness_ledger(&workboard, &career_store, &camp_forecast, &policy)
            .map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&ledger)?)
    } else {
        render_affiliate_readiness(&ledger)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL recall-readiness ledger")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_readiness_apply(
    workboard_path: PathBuf,
    ledger_path: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let workboard = read_affiliate_workboard(&workboard_path)?;
    let ledger: AhlRecallReadinessLedgerView =
        read_icecast_json(&ledger_path, "AHL recall-readiness ledger")?;
    let application =
        apply_ahl_recall_readiness_ledger(&workboard, &ledger).map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&application)?)
    } else {
        render_affiliate_readiness_application(&application)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL recall-readiness application")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_facts_board(
    rollover_path: PathBuf,
    professional_games_path: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let rollover: AhlPreseasonLeagueRolloverView =
        read_icecast_json(&rollover_path, "AHL preseason league rollover")?;
    let professional_games: AhlProfessionalGameLedgerView =
        read_icecast_json(&professional_games_path, "AHL professional-game ledger")?;
    let workboard = build_ahl_preseason_league_facts_workboard(&rollover, &professional_games)
        .map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&workboard)?)
    } else {
        render_affiliate_facts_board(&workboard)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL preseason facts workboard")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_facts_status(
    input_path: PathBuf,
    require_ready: bool,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let workboard = read_affiliate_workboard(&input_path)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&workboard)?)
    } else {
        render_affiliate_facts_board(&workboard)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL preseason facts status")?;
    } else {
        print!("{output}");
    }
    if require_ready {
        require_affiliate_facts_ready(&workboard)?;
    }
    Ok(())
}

fn require_affiliate_facts_ready(
    workboard: &AhlPreseasonLeagueFactsWorkboardView,
) -> anyhow::Result<()> {
    if workboard.teams == CANONICAL_TEAMS.len()
        && workboard.candidates > 0
        && workboard.facts_ready_candidates == workboard.candidates
        && workboard.blocker_counts.is_empty()
    {
        return Ok(());
    }
    let blockers = workboard
        .blocker_counts
        .iter()
        .map(|(blocker, count)| format!("{blocker:?}={count}"))
        .collect::<Vec<_>>()
        .join(", ");
    bail!(
        "AHL preseason facts are not ready: {} teams, {}/{} candidates ready; blockers: {}",
        workboard.teams,
        workboard.facts_ready_candidates,
        workboard.candidates,
        if blockers.is_empty() {
            "none reported"
        } else {
            &blockers
        }
    )
}

pub fn run_affiliate_facts_apply(
    workboard_path: PathBuf,
    overlay_path: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let workboard = read_affiliate_workboard(&workboard_path)?;
    let overlay: AhlPreseasonLeagueFactsOverlay =
        read_icecast_json(&overlay_path, "AHL preseason facts overlay")?;
    let application = apply_ahl_preseason_league_facts_overlay(&workboard, &overlay)
        .map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&application)?)
    } else {
        render_affiliate_facts_application(&application)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL preseason facts application")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_facts_draft(
    workboard_path: PathBuf,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let workboard = read_affiliate_workboard(&workboard_path)?;
    let draft =
        build_ahl_preseason_league_facts_overlay_draft(&workboard).map_err(anyhow::Error::msg)?;
    let output = format!("{}\n", serde_json::to_string_pretty(&draft)?);
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL preseason facts overlay draft")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_inputs_league(
    application_path: PathBuf,
    rule_path: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let application: AhlPreseasonLeagueFactsApplicationView =
        read_icecast_json(&application_path, "AHL preseason facts application")?;
    let rule: icelines_core::AhlDevelopmentRuleInput =
        read_icecast_json(&rule_path, "AHL development rule")?;
    let view = build_ahl_preseason_league_projection_inputs(&application, &rule)
        .map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_affiliate_inputs_league(&view)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL preseason league inputs")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_professional_games_apply(
    crosswalk_path: PathBuf,
    ledger_path: PathBuf,
    facts_path: PathBuf,
    nhl_team: String,
    ahl_team: String,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let crosswalk: AhlIdentityCrosswalkView =
        read_icecast_json(&crosswalk_path, "reviewed AHL identity crosswalk")?;
    let ledger: AhlProfessionalGameLedgerView =
        read_icecast_json(&ledger_path, "AHL professional-game ledger")?;
    let facts: Vec<AhlProjectionPlayerFacts> =
        read_icecast_json(&facts_path, "AHL projection facts")?;
    let applied = apply_ahl_professional_game_ledger_to_facts(
        &crosswalk, &ledger, &nhl_team, &ahl_team, &facts,
    )
    .map_err(anyhow::Error::msg)?;
    let output = format!("{}\n", serde_json::to_string_pretty(&applied)?);
    if let Some(path) = out.as_deref() {
        write_icecast_file(
            path,
            output.as_bytes(),
            "AHL professional-game facts application",
        )?;
    } else {
        print!("{output}");
    }
    Ok(())
}

fn read_affiliate_identity_catalog(path: &Path) -> anyhow::Result<AhlCanonicalIdentityCatalog> {
    let candidate_bytes = std::fs::read(path)
        .with_context(|| format!("read canonical NHL identity candidates {}", path.display()))?;
    if let Ok(catalog) = serde_json::from_slice(&candidate_bytes) {
        return Ok(catalog);
    }
    if let Ok(overlay) = serde_json::from_slice::<LeagueCampCandidateOverlay>(&candidate_bytes) {
        return Ok(AhlCanonicalIdentityCatalog {
            schema: AHL_CANONICAL_IDENTITY_CATALOG_SCHEMA.to_owned(),
            checked_at: overlay.checked_at,
            candidates: overlay
                .candidates
                .into_iter()
                .map(|row| AhlCanonicalIdentityCandidate {
                    nhl_player_id: row.player_id,
                    display_name: row.display_name,
                    birth_date: row.birth_date,
                    evidence_urls: vec![row.source_url],
                })
                .collect(),
        });
    }
    if let Ok(league) = serde_json::from_slice::<AhlIdentityLeagueCrosswalkView>(&candidate_bytes) {
        if league.schema != AHL_IDENTITY_LEAGUE_CROSSWALK_SCHEMA {
            bail!("invalid reviewed AHL league identity envelope schema");
        }
        let catalog = AhlCanonicalIdentityCatalog {
            schema: AHL_CANONICAL_IDENTITY_CATALOG_SCHEMA.to_owned(),
            checked_at: league.candidates_checked_at,
            candidates: league
                .crosswalks
                .into_iter()
                .flat_map(|crosswalk| crosswalk.rows)
                .filter(|row| row.review_status == AhlIdentityReviewStatus::Reviewed)
                .map(|row| {
                    let nhl_player_id = row.nhl_player_id.with_context(|| {
                        format!(
                            "reviewed AHL identity {} lacks canonical player ID",
                            row.provider_player_id
                        )
                    })?;
                    let display_name = row.nhl_display_name.with_context(|| {
                        format!(
                            "reviewed AHL identity {} lacks canonical display name",
                            row.provider_player_id
                        )
                    })?;
                    Ok(AhlCanonicalIdentityCandidate {
                        nhl_player_id,
                        display_name,
                        birth_date: row.nhl_birth_date,
                        evidence_urls: row.evidence_urls,
                    })
                })
                .collect::<anyhow::Result<Vec<_>>>()?,
        };
        return merge_ahl_canonical_identity_catalogs(catalog.checked_at.clone(), &[catalog])
            .map_err(anyhow::Error::msg);
    }
    bail!(
        "parse canonical identity catalog, camp candidate overlay, or reviewed AHL league identity envelope {}",
        path.display()
    )
}

async fn discover_official_affiliate_identities(
    snapshot: &AhlRosterStatsSnapshot,
    team: &str,
    refresh: bool,
    cfg: &Config,
) -> anyhow::Result<(AhlCanonicalIdentityCatalog, usize, usize, usize)> {
    discover_official_affiliate_identities_for_teams(
        snapshot,
        &[team.to_owned()],
        refresh,
        cfg,
        None,
    )
    .await
}

async fn discover_official_affiliate_identities_for_teams(
    snapshot: &AhlRosterStatsSnapshot,
    teams: &[String],
    refresh: bool,
    cfg: &Config,
    age_policy: Option<(NaiveDate, u8)>,
) -> anyhow::Result<(AhlCanonicalIdentityCatalog, usize, usize, usize)> {
    snapshot.validate().map_err(anyhow::Error::msg)?;
    if teams.is_empty() {
        bail!("official AHL identity discovery requires at least one team");
    }
    let mut roster = Vec::new();
    for team in teams {
        let team = snapshot
            .teams
            .iter()
            .find(|row| row.team_name == *team)
            .with_context(|| format!("AHL snapshot has no team named `{team}`"))?;
        roster.extend(team.roster.iter().filter(|player| {
            age_policy.is_none_or(|(as_of, max_age)| {
                age_on_date(&player.birthdate, as_of).is_some_and(|age| age <= max_age)
            })
        }));
    }
    if roster.is_empty() {
        bail!("official AHL identity discovery age policy selected no roster players");
    }
    let icelines_home = cfg
        .snapshot_dir()
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| cfg.snapshot_dir());
    let _lock = fetch_lock::acquire(&icelines_home, std::time::Duration::from_secs(120))
        .context("acquiring official identity discovery fetch lock")?;
    let cache_root = icelines_home.join("data").join(".fletch");
    let mut request_context = BTreeMap::new();
    let mut requests = Vec::new();
    for player in &roster {
        for search_name in ahl_identity_search_name_variants(&player.name) {
            let (dataset_id, source_url) =
                icelines_fetch::official_player_search_request(&search_name)?;
            if !request_context.contains_key(&dataset_id) {
                request_context.insert(
                    dataset_id.clone(),
                    (player.name.clone(), source_url.clone()),
                );
                requests.push((dataset_id, source_url));
            }
        }
    }
    let search_results =
        fetch_official_player_search_cachelines(requests, cache_root.clone(), refresh, 6).await;
    let mut search_candidates = Vec::new();
    for (dataset_id, result) in search_results {
        let (name, source_url) = request_context
            .get(&dataset_id)
            .with_context(|| format!("missing request context for {dataset_id}"))?;
        let bytes = result.with_context(|| format!("official NHL player search for {name}"))?;
        search_candidates.extend(
            parse_official_nhl_search_candidates(name, source_url, &bytes)
                .map_err(anyhow::Error::msg)?,
        );
    }
    let exact_search_candidates = search_candidates.len();
    let exact_names = search_candidates
        .iter()
        .map(|candidate| normalize_ahl_identity_name(&candidate.display_name))
        .collect::<std::collections::BTreeSet<_>>();
    let mut surname_context = BTreeMap::new();
    let mut surname_requests = Vec::new();
    for player in roster
        .iter()
        .filter(|player| !exact_names.contains(&normalize_ahl_identity_name(&player.name)))
    {
        let normalized_name = normalize_name(&player.name);
        let Some(surname) = normalized_name.split_whitespace().last().map(str::to_owned) else {
            continue;
        };
        let mut url = reqwest::Url::parse("https://search.d3.nhle.com/api/v1/search/player")?;
        url.query_pairs_mut()
            .append_pair("culture", "en-us")
            .append_pair("limit", "20")
            .append_pair("q", &surname);
        let source_url = url.to_string();
        let digest = format!("{:x}", Sha256::digest(normalized_name.as_bytes()));
        let dataset_id = format!("icelines.nhl.player-search-surname.{}", &digest[..20]);
        if !surname_context.contains_key(&dataset_id) {
            surname_context.insert(
                dataset_id.clone(),
                (player.name.clone(), source_url.clone()),
            );
            surname_requests.push((dataset_id, source_url));
        }
    }
    let surname_results =
        fetch_official_player_search_cachelines(surname_requests, cache_root.clone(), refresh, 6)
            .await;
    let mut surname_search_candidates = 0;
    for (dataset_id, result) in surname_results {
        let (name, source_url) = surname_context
            .get(&dataset_id)
            .with_context(|| format!("missing surname request context for {dataset_id}"))?;
        let bytes = result.with_context(|| format!("official NHL surname search for {name}"))?;
        let candidates = parse_official_nhl_search_candidates_by_surname(name, source_url, &bytes)
            .map_err(anyhow::Error::msg)?;
        surname_search_candidates += candidates.len();
        search_candidates.extend(candidates);
    }
    let search_catalog = AhlCanonicalIdentityCatalog {
        schema: AHL_CANONICAL_IDENTITY_CATALOG_SCHEMA.to_owned(),
        checked_at: Utc::now().date_naive().to_string(),
        candidates: search_candidates,
    };
    let search_catalog = merge_ahl_canonical_identity_catalogs(
        Utc::now().date_naive().to_string(),
        &[search_catalog],
    )
    .map_err(anyhow::Error::msg)?;
    let ids = search_catalog
        .candidates
        .iter()
        .map(|candidate| candidate.nhl_player_id)
        .collect::<Vec<_>>();
    let landing_bytes =
        fetch_official_player_landing_cachelines(ids, cache_root, refresh, 50).await?;
    let mut landing_enriched = 0;
    let mut discovered = Vec::with_capacity(search_catalog.candidates.len());
    for candidate in &search_catalog.candidates {
        if let Some(bytes) = landing_bytes.get(&candidate.nhl_player_id) {
            let landing_url = player_landing_url(
                "https://api-web.nhle.com/v1",
                candidate.nhl_player_id,
                FletchPlayerLandingArtifact::Landing,
            );
            discovered.push(
                enrich_official_nhl_landing_candidate(candidate, &landing_url, bytes)
                    .map_err(anyhow::Error::msg)?,
            );
            landing_enriched += 1;
        } else {
            discovered.push(candidate.clone());
        }
    }
    Ok((
        AhlCanonicalIdentityCatalog {
            schema: AHL_CANONICAL_IDENTITY_CATALOG_SCHEMA.to_owned(),
            checked_at: Utc::now().date_naive().to_string(),
            candidates: discovered,
        },
        exact_search_candidates,
        surname_search_candidates,
        landing_enriched,
    ))
}

fn age_on_date(birth_date: &str, as_of: NaiveDate) -> Option<u8> {
    let birth = NaiveDate::parse_from_str(birth_date, "%Y-%m-%d").ok()?;
    if birth > as_of {
        return None;
    }
    let before_birthday = (as_of.month(), as_of.day()) < (birth.month(), birth.day());
    let age = as_of.year() - birth.year() - i32::from(before_birthday);
    u8::try_from(age).ok()
}

pub fn run_affiliate_input(
    snapshot_path: PathBuf,
    crosswalk_path: PathBuf,
    facts_path: PathBuf,
    nhl_team: String,
    ahl_team: String,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let snapshot: AhlRosterStatsSnapshot =
        read_icecast_json(&snapshot_path, "AHL roster/stat snapshot")?;
    let crosswalk: AhlIdentityCrosswalkView =
        read_icecast_json(&crosswalk_path, "reviewed AHL identity crosswalk")?;
    let facts_bytes = std::fs::read(&facts_path)
        .with_context(|| format!("read AHL projection facts {}", facts_path.display()))?;
    let facts: Vec<AhlProjectionPlayerFacts> = if let Ok(facts) =
        serde_json::from_slice(&facts_bytes)
    {
        facts
    } else {
        let application: AhlProfessionalGameFactsApplicationView =
            serde_json::from_slice(&facts_bytes)
                .with_context(|| format!("parse AHL projection facts {}", facts_path.display()))?;
        if application.schema != AHL_PROFESSIONAL_GAME_FACTS_SCHEMA
            || application.nhl_team != nhl_team
            || application.ahl_team != ahl_team
        {
            bail!("professional-game facts application does not match requested affiliate");
        }
        application.facts
    };
    let input = affiliate_projection_input_from_reviewed_crosswalk(
        &snapshot,
        &nhl_team,
        &ahl_team,
        icelines_core::AhlDevelopmentRuleInput::default(),
        &crosswalk,
        &facts,
    )
    .map_err(anyhow::Error::msg)?;
    let output = format!("{}\n", serde_json::to_string_pretty(&input)?);
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL affiliate projection input")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn run_affiliate_rollover(
    prior_snapshot_path: PathBuf,
    crosswalk_path: PathBuf,
    camp_path: PathBuf,
    camp_forecast_path: PathBuf,
    config_path: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let prior_snapshot: AhlRosterStatsSnapshot =
        read_icecast_json(&prior_snapshot_path, "prior AHL roster/stat snapshot")?;
    let crosswalk: AhlIdentityCrosswalkView =
        read_icecast_json(&crosswalk_path, "prior AHL identity crosswalk")?;
    let camp: TrainingCampSimulationInput =
        read_icecast_json(&camp_path, "current training camp input")?;
    let camp_forecast: TrainingCampForecastView =
        read_icecast_json(&camp_forecast_path, "current training camp forecast")?;
    let config: AhlPreseasonRolloverConfig =
        read_icecast_json(&config_path, "AHL preseason rollover config")?;
    let view =
        build_ahl_preseason_rollover(&prior_snapshot, &crosswalk, &camp, &camp_forecast, &config)
            .map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_affiliate_rollover(&view)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL preseason rollover")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_rollover_league(
    prior_snapshot_path: PathBuf,
    league_crosswalk_path: PathBuf,
    camp_forecast_path: PathBuf,
    config_path: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let prior_snapshot: AhlRosterStatsSnapshot =
        read_icecast_json(&prior_snapshot_path, "prior AHL roster/stat snapshot")?;
    let league_crosswalk: AhlIdentityLeagueCrosswalkView = read_icecast_json(
        &league_crosswalk_path,
        "reviewed AHL league identity crosswalk",
    )?;
    let camp_forecast: TrainingCampLeagueForecastView =
        read_icecast_json(&camp_forecast_path, "league training camp forecast")?;
    let config: AhlPreseasonLeagueRolloverConfig =
        read_icecast_json(&config_path, "AHL league preseason rollover config")?;
    let view = build_ahl_preseason_league_rollover(
        &prior_snapshot,
        &league_crosswalk,
        &camp_forecast,
        &config,
    )
    .map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_affiliate_rollover_league(&view)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL league preseason rollover")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_rollover_config_league(
    league_crosswalk_path: PathBuf,
    camp_forecast_path: PathBuf,
    prior_affiliations_path: PathBuf,
    affiliations_path: PathBuf,
    as_of: String,
    source_urls: Vec<String>,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let league_crosswalk: AhlIdentityLeagueCrosswalkView = read_icecast_json(
        &league_crosswalk_path,
        "reviewed AHL league identity crosswalk",
    )?;
    let camp_forecast: TrainingCampLeagueForecastView =
        read_icecast_json(&camp_forecast_path, "league training camp forecast")?;
    let prior_affiliations: AhlAffiliationCatalogView =
        read_icecast_json(&prior_affiliations_path, "prior AHL affiliation catalog")?;
    let affiliations: AhlAffiliationCatalogView =
        read_icecast_json(&affiliations_path, "target AHL affiliation catalog")?;
    let config = build_ahl_preseason_league_rollover_config_draft(
        &league_crosswalk,
        &camp_forecast,
        &prior_affiliations,
        &affiliations,
        as_of,
        source_urls,
    )
    .map_err(anyhow::Error::msg)?;
    let output = format!("{}\n", serde_json::to_string_pretty(&config)?);
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL league rollover config draft")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_organization(
    input_path: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let input: OrganizationLineupForecastInput =
        read_icecast_json(&input_path, "organization lineup input")?;
    let view = build_organization_lineup_forecast(&input).map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_organization(&view)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "organization lineup forecast")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_affiliate_map(json: bool, out: Option<PathBuf>) -> anyhow::Result<()> {
    let view = current_ahl_affiliation_catalog();
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_affiliate_map(&view)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "AHL affiliation catalog")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

fn validate_league_camp_candidate_overlay(
    overlay: &LeagueCampCandidateOverlay,
) -> Result<(), String> {
    overlay.validate()
}

fn camp_shape_missing(players: &[LeagueRosterIdentity]) -> (usize, usize, usize) {
    camp_pool_missing(players, 14, 7, 2)
}

fn camp_invite_pool_missing(players: &[LeagueRosterIdentity]) -> (usize, usize, usize) {
    camp_pool_missing(players, 17, 9, 3)
}

fn camp_pool_missing(
    players: &[LeagueRosterIdentity],
    forward_target: usize,
    defense_target: usize,
    goalie_target: usize,
) -> (usize, usize, usize) {
    let forwards = players
        .iter()
        .filter(|player| matches!(player.position.as_str(), "C" | "L" | "R" | "LW" | "RW"))
        .count();
    let defense = players
        .iter()
        .filter(|player| player.position == "D")
        .count();
    let goalies = players
        .iter()
        .filter(|player| player.position == "G")
        .count();
    (
        forward_target.saturating_sub(forwards),
        defense_target.saturating_sub(defense),
        goalie_target.saturating_sub(goalies),
    )
}

fn camp_invite_group_missing(players: &[LeagueRosterIdentity], position: &str) -> bool {
    let missing = camp_invite_pool_missing(players);
    match position {
        "C" | "L" | "R" | "LW" | "RW" => missing.0 > 0,
        "D" => missing.1 > 0,
        "G" => missing.2 > 0,
        _ => false,
    }
}

fn automatic_camp_player(
    identity: &LeagueRosterIdentity,
    authoritative_identity: bool,
    overlay_source_url: Option<&str>,
    skater: Option<&SkaterStats>,
    goalie: Option<&GoalieStats>,
) -> Result<TrainingCampPlayerInput, String> {
    let primary_position = match identity.position.as_str() {
        "C" => Position::Center,
        "L" | "LW" => Position::LeftWing,
        "R" | "RW" => Position::RightWing,
        "D" => Position::Defense,
        "G" => Position::Goalie,
        value => return Err(format!("unsupported camp position {value}")),
    };
    let games = skater
        .map(|row| row.games_played)
        .or_else(|| goalie.map(|row| row.games_played))
        .unwrap_or(0);
    let age = identity
        .birth_date
        .as_deref()
        .and_then(|date| date.get(..4))
        .and_then(|year| year.parse::<u32>().ok())
        .map(|year| 2026u32.saturating_sub(year));
    let prospect = age.is_some_and(|age| age <= 23);
    let pre_camp_make_probability = Some(if authoritative_identity {
        match games {
            60.. => 0.95,
            30..=59 => 0.85,
            10..=29 => 0.65,
            1..=9 => 0.35,
            _ => 0.20,
        }
    } else {
        0.15
    });
    let projected_score = if primary_position == Position::Goalie {
        goalie
            .and_then(|row| row.save_pct)
            .map(|value| f64::from(value) * 100.0)
            .unwrap_or(50.0)
    } else {
        skater
            .map(|row| {
                if row.games_played == 0 {
                    0.0
                } else {
                    f64::from(row.points) * 82.0 / f64::from(row.games_played)
                }
            })
            .unwrap_or(if primary_position == Position::Defense {
                5.0
            } else {
                10.0
            })
    };
    Ok(TrainingCampPlayerInput {
        player_id: identity.player_id,
        display_name: identity.full_name.clone(),
        primary_position,
        eligible_positions: vec![primary_position],
        source_league: if let Some(url) = overlay_source_url {
            if skater.is_some() || goalie.is_some() {
                format!("Sourced organizational candidate ({url}) plus completed 2025-26 NHL statistics")
            } else {
                format!("Sourced organizational candidate ({url}); no NHL statistical sample")
            }
        } else if authoritative_identity && (skater.is_some() || goalie.is_some()) {
            "Current roster identity plus completed 2025-26 NHL statistics".to_owned()
        } else if authoritative_identity {
            "Current roster identity; no NHL statistical sample".to_owned()
        } else if skater.is_some() || goalie.is_some() {
            "Prior-season organizational fallback plus completed 2025-26 NHL statistics".to_owned()
        } else {
            "Prior-season organizational fallback; no NHL statistical sample".to_owned()
        },
        incumbent: games >= 20,
        rookie_eligible: age.is_some_and(|age| age <= 26) && games < 25,
        prospect,
        pre_camp_make_probability,
        minimum_forward_role: None,
        waiver_exempt: prospect,
        cap_hit: None,
        cap_hit_source: None,
        projected_score,
        translated_sample_games: games,
        camp_std_dev: if games >= 40 { 4.0 } else { 8.0 },
        readiness_delta: 0.0,
        management_delta: 0.0,
        availability_probability: 1.0,
        evidence_label: if skater.is_some() || goalie.is_some() {
            EvidenceLabel::Confirmed
        } else {
            EvidenceLabel::Estimated
        },
        power_play_role_score: None,
        penalty_kill_role_score: None,
        requested_slot: None,
    })
}

fn team_seed(team: &str) -> u64 {
    team.bytes().fold(0u64, |value, byte| {
        value.wrapping_mul(257) ^ u64::from(byte)
    })
}

fn render_camp_league(view: &TrainingCampLeagueForecastView) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "THE CUT — LEAGUE {}  simulated {}/{}  degraded {}  augmented {}  failed {}",
        view.season,
        view.teams_simulated,
        view.teams_requested,
        view.teams_degraded,
        view.teams_augmented,
        view.teams_failed
    );
    let _ = writeln!(
        out,
        "\nTEAM  AUTHORITY             POOL                  C/O/F       VALID     BREAK-CAMP WATCH"
    );
    let _ = writeln!(
        out,
        "      C/O/F = current roster / sourced overlay / prior-season fallback"
    );
    for team in &view.teams {
        let watch = team
            .forecast
            .as_ref()
            .map(|forecast| {
                let rows = forecast
                    .players
                    .iter()
                    .filter(|player| {
                        player.prospect
                            && !player.incumbent
                            && (0.1..0.9).contains(&player.make_probability)
                    })
                    .take(3)
                    .map(|player| {
                        format!(
                            "{} {:.0}%/{:.0}%",
                            player.display_name,
                            player.make_probability * 100.0,
                            player.dressed_probability * 100.0
                        )
                    })
                    .collect::<Vec<_>>();
                if rows.is_empty() {
                    "no bubble prospect read".to_owned()
                } else {
                    rows.join("; ")
                }
            })
            .unwrap_or_else(|| team.error.clone().unwrap_or_else(|| "no read".to_owned()));
        let valid = team
            .forecast
            .as_ref()
            .map(|forecast| format!("{}/{}", forecast.valid_trials, forecast.trials))
            .unwrap_or_else(|| "—".to_owned());
        let _ = writeln!(
            out,
            "{:<5} {:<21} {:<21} {:<11} {:<9} {}",
            team.team,
            format!("{:?}", team.authority_status),
            format!("{:?}", team.competition_pool_status),
            format!(
                "{}/{}/{}",
                team.current_roster_candidates,
                team.sourced_overlay_candidates,
                team.fallback_candidates
            ),
            valid,
            watch
        );
    }
    out
}

fn render_bubble(view: &TrainingCampExposureBoardView) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "THE BUBBLE — {}", view.season);
    let _ = writeln!(
        out,
        "TEAM RK PLAYER                    POS LANE                   AUTH     SCORE  EXIT SCRATCH WAIVER PRESSURE"
    );
    for team in &view.teams {
        for row in &team.rows {
            let pressure = if row.pressure_from.is_empty() {
                "—".to_owned()
            } else {
                row.pressure_from
                    .iter()
                    .map(|source| {
                        format!("{} {:.0}%", source.display_name, source.probability * 100.0)
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let lane = match row.lane {
                TrainingCampExposureLane::TransactionReview => "TRANSACTION REVIEW",
                TrainingCampExposureLane::ContractProtected => "CONTRACT PROTECTED",
                TrainingCampExposureLane::RosterDecisionReview => "ROSTER DECISION",
                TrainingCampExposureLane::WaiverWatch => "WAIVER WATCH",
                TrainingCampExposureLane::DevelopmentAssignment => "DEVELOPMENT ASSIGN",
                TrainingCampExposureLane::HealthyScratchRotation => "SCRATCH ROTATION",
                TrainingCampExposureLane::RosterSecure => "ROSTER SECURE",
            };
            let authority = match row.transaction_authority_status {
                TrainingCampTransactionAuthorityStatus::NoRead => "NO READ",
                TrainingCampTransactionAuthorityStatus::Partial => "PARTIAL",
                TrainingCampTransactionAuthorityStatus::Sourced => "SOURCED",
            };
            let position = match row.primary_position {
                Position::Center => "C",
                Position::LeftWing => "LW",
                Position::RightWing => "RW",
                Position::Defense => "D",
                Position::Goalie => "G",
            };
            let _ = writeln!(
                out,
                "{:<4} {:>2} {:<25} {:<3} {:<22} {:<8} {:>5.1}% {:>5.1}% {:>6.1}% {:>5.1}% {}",
                team.team,
                row.rank,
                row.display_name,
                position,
                lane,
                authority,
                row.exposure_score * 100.0,
                row.selection_loss_probability * 100.0,
                row.healthy_scratch_probability * 100.0,
                row.waiver_exposure_probability * 100.0,
                pressure
            );
        }
    }
    out
}

fn render_affiliate(view: &AhlAffiliateProjectionView) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{} → {} — {}",
        view.nhl_team, view.ahl_team, view.season
    );
    let _ = writeln!(
        out,
        "POOL AUTHORITY: {}",
        ahl_pool_authority_label(view.pool_authority.kind)
    );
    let _ = writeln!(
        out,
        "DEVELOPMENT RULE: {} development + {} veterans (max {}) | {} veteran slots unused | {} veterans scratched",
        view.development_skaters,
        view.veteran_skaters,
        view.maximum_veteran_skaters,
        view.unused_veteran_slots,
        view.available_veterans_not_dressed
    );
    for line in &view.lines {
        let label = match line.kind {
            AhlLineUnitKind::Forward => format!("F{}", line.unit),
            AhlLineUnitKind::Defense => format!("D{}", line.unit),
            AhlLineUnitKind::Goalie => "G".to_owned(),
        };
        let _ = writeln!(out, "{label:<3} {}", line.player_names.join(" — "));
    }
    let _ = writeln!(
        out,
        "\nPROSPECT POOL: {} assigned | {} dressed",
        view.assigned_prospects, view.dressed_prospects
    );
    for prospect in &view.prospect_pool {
        let role = prospect
            .line_assignment
            .as_deref()
            .or(prospect.blocked_reason.as_deref())
            .unwrap_or("unassigned");
        let readiness = prospect
            .recall_readiness
            .map(|value| format!(" | recall {:.0}%", value * 100.0))
            .unwrap_or_default();
        let _ = writeln!(
            out,
            "#{:<2} {:<24} {:>5.1} | {}{}",
            prospect.rank, prospect.display_name, prospect.projected_score, role, readiness
        );
    }
    let blocked = view
        .players
        .iter()
        .filter(|player| player.assigned_to_affiliate && !player.dressed)
        .collect::<Vec<_>>();
    if !blocked.is_empty() {
        let _ = writeln!(out, "\nOUT OF THE DRESSED LINEUP");
        for player in blocked {
            let _ = writeln!(
                out,
                "- {} ({:?}): {}",
                player.display_name,
                player.primary_position,
                player.blocked_reason.as_deref().unwrap_or("unknown")
            );
        }
    }
    out
}

fn render_affiliate_identities(view: &AhlIdentityReviewInspectionView) -> String {
    let mut out = String::new();
    let counts = &view.computed_counts;
    let _ = writeln!(
        out,
        "AHL IDENTITY REVIEW — {} — {}",
        view.ahl_team, view.season
    );
    let _ = writeln!(
        out,
        "{} roster | {} exact name+birth | {} surname+birth | {} name-only | {} ambiguous | {} conflicts | {} unmatched | {} reviewed",
        view.total_rows,
        counts.exact_name_and_birth_date,
        counts.surname_and_birth_date,
        counts.exact_name_only,
        counts.ambiguous,
        counts.conflicts,
        counts.unmatched,
        counts.reviewed
    );
    if view.declared_counts_stale {
        let _ = writeln!(out, "WARNING: declared identity counts are stale");
    }
    if view.scope == AhlIdentityInspectionScope::Attention {
        let _ = writeln!(
            out,
            "ATTENTION: {} non-routine row(s)",
            view.attention_count
        );
    }
    for row in &view.rows {
        let basis = match row.match_basis {
            AhlIdentityMatchBasis::ExactNameAndBirthDate => "NAME+BIRTH",
            AhlIdentityMatchBasis::SurnameAndBirthDate => "SURNAME+BD",
            AhlIdentityMatchBasis::ExactNameOnly => "NAME ONLY",
            AhlIdentityMatchBasis::BirthDateConflict => "CONFLICT",
            AhlIdentityMatchBasis::Ambiguous => "AMBIGUOUS",
            AhlIdentityMatchBasis::Unmatched => "UNMATCHED",
            AhlIdentityMatchBasis::ReviewedOverride => "OVERRIDE",
        };
        let review = match row.review_status {
            AhlIdentityReviewStatus::Pending => "PENDING",
            AhlIdentityReviewStatus::Reviewed => "REVIEWED",
            AhlIdentityReviewStatus::Rejected => "REJECTED",
        };
        let proposal = match (row.nhl_player_id, row.nhl_display_name.as_deref()) {
            (Some(id), Some(name)) => format!(" → {id} {name}"),
            (Some(id), None) => format!(" → {id}"),
            (None, _) => String::new(),
        };
        let evidence = row.evidence_urls.len();
        let _ = writeln!(
            out,
            "{:<24} {:<10} {:<10}{} | {} source(s) | {}",
            row.ahl_display_name, basis, review, proposal, evidence, row.note
        );
        if view.scope == AhlIdentityInspectionScope::Attention {
            let nhl_birth_date = row.nhl_birth_date.as_deref().unwrap_or("—");
            let _ = writeln!(
                out,
                "  BIRTH  AHL {} | NHL {}",
                row.ahl_birth_date, nhl_birth_date
            );
            for url in &row.evidence_urls {
                let _ = writeln!(out, "  SOURCE {url}");
            }
        }
    }
    if !view.disclosures.is_empty() {
        let _ = writeln!(out, "DISCLOSURES");
        for disclosure in &view.disclosures {
            let _ = writeln!(out, "- {disclosure}");
        }
    }
    out
}

fn render_affiliate_identity_league(view: &AhlIdentityLeagueReviewView) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "AHL IDENTITY LEAGUE REVIEW");
    let _ = writeln!(
        out,
        "{} crosswalks | {} appearances | {} reviewed | {} rejected | {} pending | {:.2}% resolved | {:.2}% canonical",
        view.crosswalks,
        view.roster_appearances,
        view.reviewed,
        view.rejected,
        view.pending,
        f64::from(view.resolved_basis_points) / 100.0,
        f64::from(view.canonical_identity_basis_points) / 100.0,
    );
    let _ = writeln!(out, "TEAM-SEASON COVERAGE");
    for summary in &view.summaries {
        let stale = if summary.declared_counts_stale {
            " | STALE COUNTS"
        } else {
            ""
        };
        let affiliate = summary.nhl_affiliate.as_deref().unwrap_or("—");
        let _ = writeln!(
            out,
            "{} {:<28} NHL {:<4} | {:>3} roster | {:>3} reviewed | {:>2} rejected | {:>2} pending{}",
            summary.season,
            summary.ahl_team,
            affiliate,
            summary.roster_players,
            summary.reviewed,
            summary.rejected,
            summary.pending,
            stale,
        );
    }
    let _ = writeln!(out, "ATTENTION GROUPS ({})", view.attention_groups.len());
    for group in &view.attention_groups {
        let proposal = group
            .nhl_player_id
            .map(|id| format!(" → NHL {id}"))
            .unwrap_or_default();
        let _ = writeln!(
            out,
            "{}{} | {} appearance(s) | {:?} | {:?} | {} source(s)",
            group.ahl_display_name,
            proposal,
            group.occurrences,
            group.review_statuses,
            group.match_bases,
            group.evidence_urls.len(),
        );
        for appearance in &group.appearances {
            let _ = writeln!(
                out,
                "  {} {} #{} | AHL {} / NHL {} | {}",
                appearance.season,
                appearance.ahl_team,
                appearance.provider_player_id,
                appearance.ahl_birth_date,
                appearance.nhl_birth_date.as_deref().unwrap_or("—"),
                appearance.note,
            );
        }
    }
    if !view.disclosures.is_empty() {
        let _ = writeln!(out, "DISCLOSURES");
        for disclosure in &view.disclosures {
            let _ = writeln!(out, "- {disclosure}");
        }
    }
    out
}

fn render_affiliate_identity_exception_board(view: &AhlIdentityExceptionBoardView) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "AHL IDENTITY EXCEPTION BOARD");
    let _ = writeln!(
        out,
        "{} group(s) | {} appearance(s) | ranked review leverage",
        view.groups, view.appearances
    );
    for row in &view.rows {
        let proposal = row
            .nhl_player_id
            .map(|id| format!(" → NHL {id}"))
            .unwrap_or_default();
        let _ = writeln!(
            out,
            "#{:<3} {:>3} pts | {}{} | {:?} | {} appearance(s), {} season(s), {} team(s)",
            row.rank,
            row.priority_score,
            row.ahl_display_name,
            proposal,
            row.recommended_action,
            row.occurrences,
            row.seasons.len(),
            row.ahl_teams.len(),
        );
        for dates in &row.conflict_date_pairs {
            let day_label = if dates.absolute_delta_days == 1 {
                "day"
            } else {
                "days"
            };
            let _ = writeln!(
                out,
                "      dates AHL {} / NHL {} | Δ {} {} | {} appearance(s)",
                dates.ahl_birth_date,
                dates.nhl_birth_date,
                dates.absolute_delta_days,
                day_label,
                dates.appearances
            );
        }
        let _ = writeln!(
            out,
            "      seasons {} | teams {} | {} retained source(s)",
            row.seasons
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(", "),
            row.ahl_teams.join(", "),
            row.evidence_urls.len(),
        );
    }
    if !view.disclosures.is_empty() {
        let _ = writeln!(out, "DISCLOSURES");
        for disclosure in &view.disclosures {
            let _ = writeln!(out, "- {disclosure}");
        }
    }
    out
}

fn render_affiliate_rollover(view: &AhlPreseasonRolloverView) -> String {
    let mut out = String::new();
    let readiness = if view.counts.projection_ready {
        "READY"
    } else {
        "NOT READY"
    };
    let _ = writeln!(
        out,
        "AHL PRESEASON ROLLOVER — {} → {} — {}",
        view.prior_season, view.target_season, readiness
    );
    let _ = writeln!(out, "{} / {}", view.nhl_team, view.ahl_team);
    let _ = writeln!(
        out,
        "PROJECTABLE: {}F / {}D / {}G | NEED: {}F / {}D / {}G",
        view.counts.projectable_forwards,
        view.counts.projectable_defensemen,
        view.counts.projectable_goalies,
        view.counts.forwards_needed,
        view.counts.defensemen_needed,
        view.counts.goalies_needed
    );
    let _ = writeln!(
        out,
        "REVIEW: {} unresolved identities | {} organization statuses | {} waiver gates",
        view.counts.unresolved_prior_identities,
        view.counts.prior_players_needing_organization_review,
        view.counts.waiver_gated_candidates
    );
    for row in &view.players {
        let group = match row.position_group {
            AhlPreseasonPositionGroup::Forward => "F",
            AhlPreseasonPositionGroup::Defense => "D",
            AhlPreseasonPositionGroup::Goalie => "G",
            AhlPreseasonPositionGroup::Unknown => "?",
        };
        let state = if row.projectable_affiliate_candidate {
            "POOL"
        } else if row.blockers.is_empty() {
            "OUT"
        } else {
            "REVIEW"
        };
        let blockers = if row.blockers.is_empty() {
            String::new()
        } else {
            format!(" — {}", row.blockers.join(", "))
        };
        let _ = writeln!(out, "{group:<2} {state:<6} {}{blockers}", row.display_name);
    }
    out
}

fn render_affiliate_rollover_league(view: &AhlPreseasonLeagueRolloverView) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "AHL LEAGUE ROLLOVER — {} → {}",
        view.prior_season, view.target_season
    );
    let _ = writeln!(
        out,
        "BUILT: {}/{} | PROJECTION READY: {} | FAILURES: {}",
        view.teams_built,
        view.teams_requested,
        view.teams_projection_ready,
        view.failures.len()
    );
    for rollover in &view.rollovers {
        let state = if rollover.counts.projection_ready {
            "READY"
        } else {
            "REVIEW"
        };
        let _ = writeln!(
            out,
            "{:<3} {:<6} {:>2}F/{:>2}D/{:>2}G — {}",
            rollover.nhl_team,
            state,
            rollover.counts.projectable_forwards,
            rollover.counts.projectable_defensemen,
            rollover.counts.projectable_goalies,
            rollover.ahl_team
        );
    }
    for failure in &view.failures {
        let _ = writeln!(
            out,
            "{:<3} FAILED {} → {} — {}",
            failure.nhl_team, failure.prior_ahl_team, failure.ahl_team, failure.reason
        );
    }
    out
}

fn render_affiliate_status_review(view: &AhlPreseasonOrganizationReview) -> String {
    let mut out = String::new();
    let state = if view.draft { "DRAFT" } else { "FINAL" };
    let identity_blockers = view
        .rows
        .iter()
        .filter(|row| !row.identity_reviewed)
        .count();
    let decisions_required = view
        .rows
        .iter()
        .filter(|row| row.identity_reviewed && row.in_current_camp == Some(false))
        .count();
    let _ = writeln!(
        out,
        "AHL ORGANIZATION STATUS — {} → {} — {state}",
        view.prior_season, view.target_season
    );
    let _ = writeln!(out, "{} / {}", view.nhl_team, view.ahl_team);
    let _ = writeln!(
        out,
        "{} prior players | {} identity blockers | {} decisions required",
        view.rows.len(),
        identity_blockers,
        decisions_required
    );
    if identity_blockers != view.identity_blockers || decisions_required != view.decisions_required
    {
        let _ = writeln!(
            out,
            "WARNING: declared counts are stale ({} identity / {} decisions)",
            view.identity_blockers, view.decisions_required
        );
    }
    let reviewer = view.reviewer.as_deref().unwrap_or("—");
    let reviewed_at = view.reviewed_at.as_deref().unwrap_or("—");
    let _ = writeln!(out, "REVIEWER {reviewer} | REVIEWED AT {reviewed_at}");
    let _ = writeln!(out, "CROSSWALK {}", view.crosswalk_fingerprint);
    for row in &view.rows {
        let status = if !row.identity_reviewed {
            "IDENTITY"
        } else if row.in_current_camp == Some(true) {
            "IN CAMP"
        } else if let Some(kind) = row.decision_kind {
            match kind {
                icelines_fetch::ahl_rollover::AhlPreseasonDecisionKind::Retained => "RETAINED",
                icelines_fetch::ahl_rollover::AhlPreseasonDecisionKind::Departed => "DEPARTED",
                icelines_fetch::ahl_rollover::AhlPreseasonDecisionKind::OtherLeague => "OTHER",
            }
        } else {
            "DECIDE"
        };
        let nhl_id = row
            .nhl_player_id
            .map_or_else(|| "—".to_owned(), |id| id.to_string());
        let _ = writeln!(
            out,
            "{:<24} {:<10} {:<10} {}",
            row.display_name, status, nhl_id, row.note
        );
    }
    out
}

fn render_affiliate_status_review_league(view: &AhlPreseasonLeagueOrganizationReview) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "AHL LEAGUE ORGANIZATION STATUS — {} → {} — DRAFT",
        view.prior_season, view.target_season
    );
    let _ = writeln!(
        out,
        "BUILT: {}/{} | IDENTITY BLOCKERS: {} | DECISIONS REQUIRED: {} | FAILURES: {}",
        view.teams_built,
        view.teams_requested,
        view.identity_blockers,
        view.decisions_required,
        view.failures.len()
    );
    for review in &view.reviews {
        let _ = writeln!(
            out,
            "{:<3} {:>3} decisions | {:>2} identity blockers — {}",
            review.nhl_team, review.decisions_required, review.identity_blockers, review.ahl_team
        );
    }
    for failure in &view.failures {
        let _ = writeln!(
            out,
            "{:<3} FAILED {} — {}",
            failure.nhl_team, failure.prior_ahl_team, failure.reason
        );
    }
    out
}

fn render_affiliate_professional_games(view: &AhlProfessionalGameLedgerView) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "AHL PROFESSIONAL-GAME LEDGER — {} — policy {} ({:?})",
        view.target_season, view.policy_id, view.policy_authority_status
    );
    let _ = writeln!(
        out,
        "COMPLETE: {}/{} | MISSING HISTORIES: {} | UNRESOLVED: {} | THRESHOLD: {}",
        view.complete_players,
        view.canonical_players,
        view.missing_histories,
        view.unresolved_players,
        view.threshold
    );
    for player in view
        .players
        .iter()
        .filter(|player| !player.blockers.is_empty())
    {
        let leagues = if player.unresolved_professional_leagues.is_empty() {
            "—".to_owned()
        } else {
            player.unresolved_professional_leagues.join(",")
        };
        let _ = writeln!(
            out,
            "{:>7}  {:<28} {:<34} {}",
            player.nhl_player_id,
            player.display_name,
            player.blockers.join(","),
            leagues
        );
    }
    out
}

fn render_affiliate_transaction_state(view: &AhlTransactionStateLedgerView) -> String {
    format!(
        "AHL TRANSACTION STATE — {} through {}\nSource events: {} | Through cutoff: {} | Players: {}\nAssigned: {} | Removed: {} | Ambiguous: {}\nCanonical identities: {} | Identity unavailable: {}\nMethod: {}\nFingerprint: {}\n",
        view.season,
        view.cutoff,
        view.counts.source_events,
        view.counts.events_through_cutoff,
        view.counts.players_with_events,
        view.counts.assigned,
        view.counts.removed,
        view.counts.ambiguous,
        view.counts.canonically_identified,
        view.counts.identity_unavailable,
        view.method,
        view.source_fingerprint
    )
}

fn render_affiliate_transaction_state_application(
    view: &AhlTransactionStateApplicationView,
) -> String {
    format!(
        "AHL TRANSACTION STATE APPLIED — {}\nAssigned true: {} | Assigned false: {}\nAmbiguous skipped: {} | Provider-only skipped: {} | Without candidate: {}\nCandidates still missing assignment: {}\nLedger: {}\n",
        view.target_season,
        view.assigned_true_applied,
        view.assigned_false_applied,
        view.ambiguous_states_skipped,
        view.provider_only_states_skipped,
        view.canonical_states_without_candidate,
        view.candidates_missing_assignment_authority,
        view.transaction_state_fingerprint
    )
}

fn render_affiliate_waiver_review(view: &AhlWaiverClearanceReviewView) -> String {
    format!(
        "AHL WAIVER REVIEW — {} through {} — {}\nRequired: {} | Resolved: {} ({} cleared, {} claimed) | Pending: {}\nFingerprint: {}\n",
        view.target_season,
        view.cutoff,
        if view.draft { "DRAFT" } else { "FINAL" },
        view.counts.decisions_required,
        view.counts.resolved,
        view.counts.cleared,
        view.counts.claimed,
        view.counts.pending,
        view.source_fingerprint
    )
}

fn render_affiliate_waiver_application(view: &AhlWaiverClearanceApplicationView) -> String {
    format!(
        "AHL WAIVERS APPLIED — {}\nCleared: {} | Claimed: {} | Pending review: {}\nCandidates still missing waiver clearance: {}\nReview: {}\n",
        view.target_season,
        view.cleared_applied,
        view.claimed_applied,
        view.pending_review_rows,
        view.candidates_missing_waiver_clearance,
        view.waiver_review_fingerprint
    )
}

fn render_affiliate_values(view: &AhlPlayerValueLedgerView) -> String {
    format!(
        "AHL PLAYER VALUES\nSeason: {}\nPlayers scored: {}\nMethod: {}\nFingerprint: {}\nStatus: EVALUATION — confidence-weighted affiliate ordering, not an NHL equivalency or calibrated forecast\n",
        view.prior_season,
        view.players_scored,
        view.policy.method_version,
        view.source_fingerprint
    )
}

fn render_affiliate_values_application(view: &AhlPlayerValueApplicationView) -> String {
    format!(
        "AHL PLAYER VALUES APPLIED\nSeason: {} -> {}\nScores filled: {}\nCandidates still missing score: {}\nWorkboard fingerprint: {}\nLedger fingerprint: {}\n",
        view.prior_season,
        view.target_season,
        view.rows_applied,
        view.candidates_without_value,
        view.source_workboard_fingerprint,
        view.value_ledger_fingerprint
    )
}

fn render_affiliate_values_cross_league(view: &AhlCrossLeagueValueLedgerView) -> String {
    format!(
        "AHL CROSS-LEAGUE VALUE FALLBACK\nSeason: {} -> {}\nMissing candidates requested: {}\nEstimated: {} | Unavailable: {}\nSupported league/position calibrations: {}\nMethod: {}\nFingerprint: {}\nStatus: EVALUATION — paired career translation, not a universal NHLe or NHL projection\n",
        view.prior_season,
        view.target_season,
        view.candidates_requested,
        view.candidates_estimated,
        view.candidates_unavailable,
        view.calibrations_supported,
        view.policy.method_version,
        view.source_fingerprint
    )
}

fn render_affiliate_values_cross_league_application(
    view: &AhlCrossLeagueValueApplicationView,
) -> String {
    format!(
        "AHL CROSS-LEAGUE VALUES APPLIED\nSeason: {} -> {}\nScores filled: {}\nCandidates still missing score: {}\nWorkboard fingerprint: {}\nLedger fingerprint: {}\n",
        view.prior_season,
        view.target_season,
        view.rows_applied,
        view.candidates_without_value,
        view.source_workboard_fingerprint,
        view.value_ledger_fingerprint
    )
}

fn render_affiliate_prospects(view: &AhlProspectStatusLedgerView) -> String {
    format!(
        "AHL PROSPECT STATUS\nSeason: {} -> {}\nCandidate appearances: {}\nCanonical candidates: {}\nClassified: {}\nUnavailable: {}\nMethod: {}\nFingerprint: {}\n",
        view.prior_season,
        view.target_season,
        view.candidate_appearances,
        view.candidates_requested,
        view.candidates_classified,
        view.candidates_unavailable,
        view.policy.method_version,
        view.source_fingerprint
    )
}

fn render_affiliate_prospects_application(view: &AhlProspectStatusApplicationView) -> String {
    format!(
        "AHL PROSPECT STATUS APPLIED\nSeason: {} -> {}\nStatuses filled: {}\nCandidates still missing status: {}\nWorkboard fingerprint: {}\nLedger fingerprint: {}\n",
        view.prior_season,
        view.target_season,
        view.rows_applied,
        view.candidates_without_prospect_status,
        view.source_workboard_fingerprint,
        view.prospect_ledger_fingerprint
    )
}

fn render_affiliate_readiness(view: &AhlRecallReadinessLedgerView) -> String {
    format!(
        "AHL RECALL READINESS\nSeason: {} -> {}\nCandidate appearances: {}\nCanonical candidates: {}\nEstimated: {}\nUnavailable: {}\nMethod: {}\nFingerprint: {}\n",
        view.prior_season,
        view.target_season,
        view.candidate_appearances,
        view.candidates_requested,
        view.candidates_estimated,
        view.candidates_unavailable,
        view.policy.method_version,
        view.source_fingerprint
    )
}

fn render_affiliate_readiness_application(view: &AhlRecallReadinessApplicationView) -> String {
    format!(
        "AHL RECALL READINESS APPLIED\nSeason: {} -> {}\nReadiness rows filled: {}\nCandidates still missing readiness: {}\nWorkboard fingerprint: {}\nLedger fingerprint: {}\n",
        view.prior_season,
        view.target_season,
        view.rows_applied,
        view.candidates_without_recall_readiness,
        view.source_workboard_fingerprint,
        view.readiness_ledger_fingerprint
    )
}

fn render_affiliate_facts_board(view: &AhlPreseasonLeagueFactsWorkboardView) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "AHL PRESEASON FACTS WORKBOARD — {} — {} teams",
        view.target_season, view.teams
    );
    let _ = writeln!(
        out,
        "CANDIDATES: {} | FACTS READY: {} | PRO-GAME POLICY: {} ({})",
        view.candidates,
        view.facts_ready_candidates,
        view.professional_game_policy_id,
        view.professional_game_policy_authority
    );
    for team in &view.team_workboards {
        let _ = writeln!(
            out,
            "{:<3} {:>3} candidates | {:>3} ready | {:>3} assignment | {:>3} status | {:>3} waivers | {}",
            team.nhl_team,
            team.counts.candidates,
            team.counts.facts_ready_candidates,
            team.counts.missing_assignment_authority,
            team.counts.missing_organization_status,
            team.counts.missing_waiver_clearance,
            team.ahl_team
        );
    }
    out
}

fn render_affiliate_facts_application(view: &AhlPreseasonLeagueFactsApplicationView) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "AHL PRESEASON FACTS APPLICATION — {} — {} rows applied",
        view.target_season, view.rows_applied
    );
    let _ = writeln!(
        out,
        "CANDIDATES: {} | FACTS READY: {} | SOURCE: {} | OVERLAY: {}",
        view.candidates,
        view.facts_ready_candidates,
        view.source_workboard_fingerprint,
        view.overlay_fingerprint
    );
    for line in render_affiliate_facts_board(&view.workboard)
        .lines()
        .skip(2)
    {
        let _ = writeln!(out, "{line}");
    }
    out
}

fn render_affiliate_inputs_league(view: &AhlPreseasonLeagueProjectionInputsView) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "AHL PRESEASON PROJECTION INPUTS — {} — {}/{} teams built",
        view.target_season, view.teams_built, view.teams_requested
    );
    for input in &view.inputs {
        let _ = writeln!(
            out,
            "{:<3} READY {:>3} players — {}",
            input.nhl_team,
            input.players.len(),
            input.ahl_team
        );
    }
    for failure in &view.failures {
        let _ = writeln!(
            out,
            "{:<3} BLOCKED — {} — {}",
            failure.nhl_team, failure.ahl_team, failure.reason
        );
    }
    out
}

fn render_affiliate_map(view: &AhlAffiliationCatalogView) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "AHL AFFILIATIONS — {} | checked {}",
        view.season, view.checked_at
    );
    for row in &view.affiliations {
        let _ = writeln!(out, "{:<3}  {}", row.nhl_team, row.ahl_team);
    }
    let _ = writeln!(out, "SOURCE  {}", view.source_url);
    out
}

fn render_organization(view: &OrganizationLineupForecastView) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "THE SYSTEM — {} / {} — {}",
        view.nhl_team, view.ahl_team, view.season
    );
    let _ = writeln!(
        out,
        "{} FORWARD LINES  {} DEFENSE PAIRS  {} GOALTENDERS",
        view.counts.forward_lines, view.counts.defense_pairs, view.counts.goalies
    );
    let _ = writeln!(
        out,
        "AHL POOL: {}",
        ahl_pool_authority_label(view.ahl_pool_authority.kind)
    );
    for level in [OrganizationLevel::Nhl, OrganizationLevel::Ahl] {
        let heading = match level {
            OrganizationLevel::Nhl => "NHL",
            OrganizationLevel::Ahl => "AHL",
        };
        let team = if level == OrganizationLevel::Nhl {
            &view.nhl_team
        } else {
            &view.ahl_team
        };
        let _ = writeln!(out, "\n{heading} — {team}");
        for unit in view.units.iter().filter(|unit| unit.level == level) {
            let label = match unit.kind {
                OrganizationUnitKind::ForwardLine => format!("F{}", unit.unit),
                OrganizationUnitKind::DefensePair => format!("D{}", unit.unit),
                OrganizationUnitKind::Goalies => "G".to_owned(),
            };
            let score = unit
                .average_score
                .map(|value| format!("  [{value:.1}]"))
                .unwrap_or_default();
            let _ = writeln!(out, "{label:<3} {}{score}", unit.player_names.join(" — "));
        }
    }
    let _ = writeln!(out, "\nFIRST RECALL");
    for plan in &view.recall_plan {
        let group = match plan.position_group {
            OrganizationPositionGroup::Forward => "F",
            OrganizationPositionGroup::Defense => "D",
            OrganizationPositionGroup::Goalie => "G",
        };
        let name = plan.first_recall_name.as_deref().unwrap_or("no candidate");
        let _ = writeln!(
            out,
            "{group:<2} {name}  ({} candidates)",
            plan.candidate_count
        );
    }
    if !view.blocked_players.is_empty() {
        let _ = writeln!(out, "\nAHL DEPTH OUTSIDE DRESSED LINEUP");
        for player in &view.blocked_players {
            let _ = writeln!(
                out,
                "- {} ({}) — {}",
                player.display_name,
                player.primary_position.abbreviation(),
                player.blocked_reason
            );
        }
    }
    out
}

fn ahl_pool_authority_label(kind: AhlRosterPoolAuthorityKind) -> &'static str {
    match kind {
        AhlRosterPoolAuthorityKind::OfficialSnapshot => "OFFICIAL SNAPSHOT",
        AhlRosterPoolAuthorityKind::PreseasonProjection => "PRESEASON PROJECTION",
        AhlRosterPoolAuthorityKind::AuthoredScenario => "AUTHORED SCENARIO",
        AhlRosterPoolAuthorityKind::Unspecified => "NO READ",
    }
}

fn render_camp(view: &TrainingCampForecastView) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "THE CUT — {} {}  {} trials  {}",
        view.team, view.season, view.trials, view.roster_shape
    );
    let _ = writeln!(
        out,
        "CAP: {}",
        match view.salary_cap_status {
            TrainingCampSalaryCapStatus::Enforced => view
                .salary_cap_upper_limit
                .map(|limit| format!("ENFORCED — ${limit}"))
                .unwrap_or_else(|| "ENFORCED".to_owned()),
            TrainingCampSalaryCapStatus::NoRead =>
                "NO READ — cap compliance was not simulated".to_owned(),
        }
    );
    let _ = writeln!(
        out,
        "\nPLAYER                         POS   PRE-CAMP           ACTIVE   DRESS  SCRATCH  WAIVER    CAMP   RESULT"
    );
    for player in &view.players {
        let _ = writeln!(
            out,
            "{:<30} {:<5} {:<18} {:>6.1}% {:>6.1}% {:>7.1}% {:>6.1}% {:>7.1}  {:?}",
            player.display_name,
            player.primary_position.abbreviation(),
            format!("{:?}", player.pre_camp_track),
            player.make_probability * 100.0,
            player.dressed_probability * 100.0,
            player.healthy_scratch_probability * 100.0,
            player.waiver_exposure_probability * 100.0,
            player.camp_mean,
            player.status
        );
    }
    if let Some(branch) = view.most_common_rosters.first() {
        let _ = writeln!(
            out,
            "\nMost common opening roster: {:.1}% of valid trials",
            branch.probability * 100.0
        );
    }
    for warning in &view.warnings {
        let _ = writeln!(out, "WARNING: {warning}");
    }
    out
}

pub async fn run_blender(args: IceCastBlenderArgs) -> anyhow::Result<()> {
    let lineup_bytes = std::fs::read(&args.lineup)
        .with_context(|| format!("read IceLines lineup {}", args.lineup.display()))?;
    let lineup: TeamLineupProjectionView = serde_json::from_slice(&lineup_bytes)
        .with_context(|| format!("parse IceLines lineup {}", args.lineup.display()))?;
    let mut pair_evidence = args
        .pair_evidence
        .as_deref()
        .map(|path| {
            let bytes = std::fs::read(path)
                .with_context(|| format!("read Blender pair evidence {}", path.display()))?;
            serde_json::from_slice::<Vec<LineCombinationPairEvidenceInput>>(&bytes)
                .with_context(|| format!("parse Blender pair evidence {}", path.display()))
        })
        .transpose()?
        .unwrap_or_default();
    if let Some(season) = args.shift_season {
        let report = load_official_shift_overlap(&lineup, season, args.refresh_shifts).await?;
        pair_evidence = report
            .pairs
            .iter()
            .filter(|pair| pair.shared_games >= 5 && pair.shared_seconds >= 300)
            .map(|pair| LineCombinationPairEvidenceInput {
                player_one_id: pair.player_one_id,
                player_two_id: pair.player_two_id,
                fit: pair.lower_player_overlap_pct.clamp(0.0, 1.0),
                sample: pair.shared_games,
                kind: icelines_core::LineCombinationPairEvidenceKind::ObservedDeployment,
            })
            .collect();
        if let Some(path) = args.shift_report_out.as_deref() {
            let bytes = format!("{}\n", serde_json::to_string_pretty(&report)?);
            write_icecast_file(path, bytes.as_bytes(), "official shift overlap report")?;
        }
    }
    let forecast = build_line_combination_forecast(
        &lineup,
        &pair_evidence,
        LineCombinationForecastConfig {
            max_candidates: args.max_candidates,
            allow_off_wing: args.allow_off_wing,
        },
    )
    .map_err(anyhow::Error::msg)?;
    let policy = build_adaptive_lineup_policy(
        &forecast,
        args.review_games,
        args.minimum_points_percentage,
        args.max_changes,
        args.max_choices,
    )
    .map_err(anyhow::Error::msg)?;
    let scenario = TeamSeasonScenario {
        name: format!("The Bench — {} adaptive lineup", forecast.team),
        trade_deadline: None,
        events: Vec::new(),
        adaptive_lineup_policies: vec![policy],
        opening_roster_policies: Vec::new(),
    };
    if let Some(path) = args.scenario_out.as_deref() {
        let bytes = format!("{}\n", serde_json::to_string_pretty(&scenario)?);
        write_icecast_file(path, bytes.as_bytes(), "adaptive lineup scenario")?;
    }
    let output = if args.json {
        format!("{}\n", serde_json::to_string_pretty(&forecast)?)
    } else {
        render_blender(&forecast)
    };
    if let Some(path) = args.out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "line combination forecast")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_bench(args: IceCastBenchArgs) -> anyhow::Result<()> {
    let forecast: TeamGameForecastView = read_icecast_json(&args.forecast, "team game forecast")?;
    let lineup: TeamLineupProjectionView = read_icecast_json(&args.lineup, "team lineup")?;
    let profile: TeamDecisionProfile = read_icecast_json(&args.profile, "team decision profile")?;
    let styles: Vec<OpponentStyleEvidenceRow> =
        read_icecast_json(&args.style_evidence, "opponent style evidence")?;
    let config = Config::load()?;
    let store = SnapshotStore::new(config.snapshot_dir());
    let mut loaded = load_into_repo(Season(args.stats_season), SeasonType::Regular, &store)
        .with_context(|| format!("load {} player-role evidence", args.stats_season))?;
    let team = TeamAbbr(lineup.team.trim().to_ascii_uppercase());
    let roster_ids = lineup
        .forward_lines
        .iter()
        .flat_map(|line| [&line.left_wing, &line.center, &line.right_wing])
        .chain(
            lineup
                .defense_pairs
                .iter()
                .flat_map(|pair| [&pair.left, &pair.right]),
        )
        .chain([&lineup.goalies.starter, &lineup.goalies.backup])
        .flatten()
        .map(|player| icelines_core::identity::PlayerId(player.player_id))
        .collect::<Vec<_>>();
    loaded.repo.set_current_roster(
        team.clone(),
        Season(args.stats_season),
        SeasonType::Regular,
        roster_ids,
    );
    let roles = build_team_player_matchup_role_evidence(
        &loaded.repo,
        &team,
        Season(args.stats_season),
        SeasonType::Regular,
    )
    .map_err(anyhow::Error::msg)?;
    let view = build_team_season_game_plan_schedule_from_evidence(
        &forecast, &lineup, &profile, &roles, &styles,
    )
    .map_err(anyhow::Error::msg)?;
    if let Some(path) = args.scenario_out.as_deref() {
        let bytes = serde_json::to_vec_pretty(&view.scenario)?;
        write_icecast_file(path, &bytes, "Bench season scenario")?;
    }
    let output = if args.json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_bench(&view, &roles)
    };
    if let Some(path) = args.out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "Bench game-plan schedule")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub struct IceCastLineMatchupArgs {
    pub input: PathBuf,
    pub away_bench_plan: Option<PathBuf>,
    pub home_bench_plan: Option<PathBuf>,
    pub json: bool,
    pub out: Option<PathBuf>,
}

pub fn run_line_matchup(args: IceCastLineMatchupArgs) -> anyhow::Result<()> {
    let mut input: PlayerLineMatchupForecastInput =
        read_icecast_json(&args.input, "player-line matchup input")?;
    for (path, side) in [
        (args.away_bench_plan.as_deref(), &mut input.away),
        (args.home_bench_plan.as_deref(), &mut input.home),
    ] {
        if let Some(path) = path {
            let plan: BenchGamePlanView = read_icecast_json(path, "Bench game plan")?;
            apply_bench_game_plan_to_player_line_matchup(side, &plan)
                .map_err(anyhow::Error::msg)?;
            let plan_fingerprint =
                format!("sha256:{:x}", Sha256::digest(serde_json::to_vec(&plan)?));
            if !side.source_fingerprints.contains(&plan_fingerprint) {
                side.source_fingerprints.push(plan_fingerprint);
            }
        }
    }
    let view = build_player_line_matchup_forecast(input).map_err(anyhow::Error::msg)?;
    let output = if args.json || args.out.is_some() {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_line_matchup(&view)
    };
    if let Some(path) = args.out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "player-line matchup forecast")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub struct IceCastLineMatchupProfilesArgs {
    pub lineup: PathBuf,
    pub role_evidence: PathBuf,
    pub shift_report: PathBuf,
    pub evidence_cutoff_at: String,
    pub recency: f64,
    pub source_fingerprints: Vec<String>,
    pub json: bool,
    pub out: Option<PathBuf>,
}

pub fn run_line_matchup_profiles(args: IceCastLineMatchupProfilesArgs) -> anyhow::Result<()> {
    let lineup: TeamLineupProjectionView = read_icecast_json(&args.lineup, "team lineup")?;
    let roles: TeamPlayerMatchupRoleEvidenceView =
        read_icecast_json(&args.role_evidence, "player matchup role evidence")?;
    let shifts: ShiftOverlapReport =
        read_icecast_json(&args.shift_report, "official shift report")?;
    let evidence_cutoff_at = DateTime::parse_from_rfc3339(&args.evidence_cutoff_at)
        .context("--evidence-cutoff-at must be RFC 3339, for example 2026-04-18T23:00:00Z")?
        .with_timezone(&Utc);
    let view = build_player_line_matchup_profiles(
        &lineup,
        &roles,
        &shifts,
        evidence_cutoff_at,
        args.recency,
        args.source_fingerprints,
    )
    .map_err(anyhow::Error::msg)?;
    let output = if args.json || args.out.is_some() {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        format!(
            "THE MATCHUP PROFILES — {}\n{} of {} skaters ({:.1}%)  {} deployment-affinity rows\n",
            view.team,
            view.profiles_built,
            view.dressed_skaters,
            view.coverage * 100.0,
            view.deployment_chemistry.len(),
        )
    };
    if let Some(path) = args.out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "player-line matchup profiles")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_line_chemistry(
    team: String,
    forecast_at: String,
    input: PathBuf,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let rows: Vec<ShiftAdjustedUnitOutcomeInput> =
        read_icecast_json(&input, "shift-adjusted unit outcomes")?;
    let forecast_at = DateTime::parse_from_rfc3339(&forecast_at)
        .context("--forecast-at must be RFC 3339, for example 2026-04-19T12:00:00Z")?
        .with_timezone(&Utc);
    let view = build_shift_adjusted_chemistry_evidence(&team, forecast_at, rows)
        .map_err(anyhow::Error::msg)?;
    let output = format!("{}\n", serde_json::to_string_pretty(&view)?);
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "shift-adjusted chemistry evidence")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_line_chemistry_moneypuck(
    team: String,
    forecast_at: String,
    line_games: Vec<PathBuf>,
    baselines: PathBuf,
    minimum_shared_minutes: f64,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let forecast_at = DateTime::parse_from_rfc3339(&forecast_at)
        .context("--forecast-at must be RFC 3339, for example 2026-04-19T12:00:00Z")?
        .with_timezone(&Utc);
    let baselines: Vec<PregameUnitXgBaseline> =
        read_icecast_json(&baselines, "pregame unit xG baselines")?;
    let mut rows = Vec::new();
    for path in line_games {
        let csv_text = std::fs::read_to_string(&path)
            .with_context(|| format!("read MoneyPuck line-game file {}", path.display()))?;
        rows.extend(
            parse_moneypuck_line_games(&csv_text)
                .with_context(|| format!("parse MoneyPuck line-game file {}", path.display()))?,
        );
    }
    let view = build_moneypuck_line_chemistry(
        &team,
        forecast_at,
        &rows,
        baselines,
        minimum_shared_minutes,
    )
    .map_err(anyhow::Error::msg)?;
    let output = format!("{}\n", serde_json::to_string_pretty(&view)?);
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "MoneyPuck line chemistry")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn run_line_chemistry_moneypuck_auto(
    team: String,
    season_start: u32,
    forecast_at: String,
    trailing_games: usize,
    minimum_player_games: usize,
    minimum_shared_minutes: f64,
    summary: PathBuf,
    line_game_dir: PathBuf,
    skater_game_dir: PathBuf,
    team_game_dir: PathBuf,
    rights_basis: String,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let forecast_at = DateTime::parse_from_rfc3339(&forecast_at)
        .context("--forecast-at must be RFC 3339, for example 2026-04-19T12:00:00Z")?
        .with_timezone(&Utc);
    let summary_csv = std::fs::read_to_string(&summary)
        .with_context(|| format!("read MoneyPuck line summary {}", summary.display()))?;
    let package = MoneyPuckLineChemistrySourcePackage {
        schema: icelines_fetch::MONEYPUCK_LINE_CHEMISTRY_SOURCE_PACKAGE_SCHEMA.to_owned(),
        summary_csv,
        line_game_csvs: read_csv_directory(&line_game_dir, "MoneyPuck line-game")?,
        skater_game_csvs: read_csv_directory(&skater_game_dir, "MoneyPuck skater-game")?,
        team_game_csvs: read_csv_directory(&team_game_dir, "MoneyPuck team-game")?,
        rights_basis,
    };
    let baseline_config = MoneyPuckUnitBaselineConfig {
        trailing_games,
        minimum_player_games,
        ..MoneyPuckUnitBaselineConfig::default()
    };
    let view = build_moneypuck_line_chemistry_from_package(
        &team,
        season_start,
        forecast_at,
        package,
        minimum_shared_minutes,
        baseline_config,
    )
    .map_err(anyhow::Error::msg)?;
    let output = format!("{}\n", serde_json::to_string_pretty(&view)?);
    if let Some(path) = out.as_deref() {
        write_icecast_file(
            path,
            output.as_bytes(),
            "automatic MoneyPuck line chemistry",
        )?;
    } else {
        print!("{output}");
    }
    Ok(())
}

fn read_csv_directory(path: &std::path::Path, label: &str) -> anyhow::Result<Vec<String>> {
    let mut paths = std::fs::read_dir(path)
        .with_context(|| format!("read {label} directory {}", path.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|path| {
        path.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("csv"))
    });
    paths.sort();
    if paths.is_empty() {
        bail!("{label} directory {} contains no CSV files", path.display());
    }
    paths
        .into_iter()
        .map(|path| {
            std::fs::read_to_string(&path)
                .with_context(|| format!("read {label} file {}", path.display()))
        })
        .collect()
}

pub fn run_line_matchup_validate(
    input: PathBuf,
    created_at: String,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let observations: Vec<PlayerLineMatchupAblationObservation> =
        read_icecast_json(&input, "player-line matchup ablation observations")?;
    let created_at = DateTime::parse_from_rfc3339(&created_at)
        .context("--created-at must be RFC 3339, for example 2026-04-20T12:00:00Z")?
        .with_timezone(&Utc);
    let view = build_player_line_matchup_validation(observations, created_at)
        .map_err(anyhow::Error::msg)?;
    let output = format!("{}\n", serde_json::to_string_pretty(&view)?);
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "player-line matchup validation")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_line_matchup_compare(
    input: PathBuf,
    focus_team: String,
    baseline: String,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let scenarios: Vec<PlayerLineMatchupScenarioInput> =
        read_icecast_json(&input, "player-line matchup scenarios")?;
    let view = compare_player_line_matchup_scenarios(&focus_team, &baseline, scenarios)
        .map_err(anyhow::Error::msg)?;
    let output = format!("{}\n", serde_json::to_string_pretty(&view)?);
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "player-line matchup comparison")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

fn render_line_matchup(view: &PlayerLineMatchupForecastView) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "THE MATCHUP — {} at {}  {}",
        view.away.team, view.home.team, view.game_date
    );
    for team in [&view.away, &view.home] {
        let suitability = team
            .matchup_suitability
            .map(|value| format!("{value:+.3}"))
            .unwrap_or_else(|| "NO READ".to_owned());
        let _ = writeln!(
            out,
            "{}  5v5 {:>5.1}  suitability {}  profiles {:>5.1}%  chemistry {:+.2}",
            team.team,
            team.five_on_five_matchup_score,
            suitability,
            team.profile_coverage * 100.0,
            team.chemistry_effect
        );
        for warning in &team.warnings {
            let _ = writeln!(out, "WARNING: {}: {warning}", team.team);
        }
    }
    let _ = writeln!(out, "Fingerprint: {}", view.fingerprint);
    out
}

fn read_icecast_json<T: for<'de> Deserialize<'de>>(
    path: &std::path::Path,
    label: &str,
) -> anyhow::Result<T> {
    let bytes = std::fs::read(path).with_context(|| format!("read {label} {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("parse {label} {}", path.display()))
}

fn read_affiliate_workboard(
    path: &std::path::Path,
) -> anyhow::Result<AhlPreseasonLeagueFactsWorkboardView> {
    let value: serde_json::Value = read_icecast_json(path, "AHL preseason workboard authority")?;
    let workboard = if value.get("schema").and_then(serde_json::Value::as_str)
        == Some(icelines_fetch::ahl_preseason_facts::AHL_PRESEASON_LEAGUE_FACTS_WORKBOARD_SCHEMA)
    {
        value
    } else {
        value.get("workboard").cloned().with_context(|| {
            format!(
                "{} is neither a preseason workboard nor an application containing one",
                path.display()
            )
        })?
    };
    let workboard = serde_json::from_value(workboard)
        .with_context(|| format!("parse nested AHL preseason workboard {}", path.display()))?;
    icelines_fetch::ahl_preseason_facts::validate_workboard(&workboard)
        .map_err(anyhow::Error::msg)?;
    Ok(workboard)
}

fn render_bench(
    view: &icelines_core::TeamSeasonGamePlanScheduleView,
    roles: &icelines_core::TeamPlayerMatchupRoleEvidenceView,
) -> String {
    let mut out = String::new();
    let average_edge = view
        .games
        .iter()
        .map(|game| game.plan.tactical_matchup_edge)
        .sum::<f64>()
        / view.games.len() as f64;
    let _ = writeln!(out, "THE BENCH — {} {}", view.team, view.season);
    let _ = writeln!(
        out,
        "{} game plans  {} of {} roster skaters rated  average tactical edge {:+.2}",
        view.games.len(),
        roles.rated_skaters,
        roles.roster_skaters,
        average_edge
    );
    for warning in &roles.warnings {
        let _ = writeln!(out, "WARNING: {warning}");
    }
    for disclosure in &view.disclosures {
        let _ = writeln!(out, "- {disclosure}");
    }
    out
}

async fn load_official_shift_overlap(
    lineup: &TeamLineupProjectionView,
    season: u32,
    refresh: bool,
) -> anyhow::Result<ShiftOverlapReport> {
    let team = lineup.team.trim().to_ascii_uppercase();
    let player_ids = lineup
        .forward_lines
        .iter()
        .flat_map(|line| [&line.left_wing, &line.center, &line.right_wing])
        .flatten()
        .chain(
            lineup
                .defense_pairs
                .iter()
                .flat_map(|pair| [&pair.left, &pair.right])
                .flatten(),
        )
        .map(|player| player.player_id)
        .collect::<std::collections::BTreeSet<_>>();
    let client = NhlApiClient::production();
    let schedule = client
        .fetch_team_season_schedule(&team, &season.to_string())
        .await
        .with_context(|| format!("fetch official {team} {season} schedule for shifts"))?;
    let games = schedule
        .into_iter()
        .filter(|game| game.game_type == 2 && game.is_final())
        .collect::<Vec<_>>();
    if games.is_empty() {
        bail!("official shift ingestion requires completed regular-season games");
    }
    let cache_root = Config::load()?
        .cache_dir
        .join("shiftcharts")
        .join(season.to_string());
    std::fs::create_dir_all(&cache_root)
        .with_context(|| format!("create shift cache {}", cache_root.display()))?;
    let mut loaded = Vec::with_capacity(games.len());
    for game in &games {
        let cache_path = cache_root.join(format!("{}.json", game.game_id));
        let rows = if cache_path.exists() && !refresh {
            let bytes = std::fs::read(&cache_path)
                .with_context(|| format!("read shift cache {}", cache_path.display()))?;
            serde_json::from_slice::<Vec<OfficialShiftChartRow>>(&bytes)
                .with_context(|| format!("parse shift cache {}", cache_path.display()))?
        } else {
            let rows = client
                .fetch_shift_chart(game.game_id)
                .await
                .with_context(|| format!("fetch official shifts for game {}", game.game_id))?;
            let bytes = serde_json::to_vec_pretty(&rows)?;
            icelines_fetch::snapshot::atomic_write_bytes(&cache_path, &bytes)
                .with_context(|| format!("write shift cache {}", cache_path.display()))?;
            rows
        };
        loaded.push((game.game_id, rows));
    }
    build_shift_overlap_report(&team, season, games.len(), &player_ids, &loaded)
        .map_err(anyhow::Error::msg)
}

fn render_blender(view: &LineCombinationForecastView) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "THE BLENDER — {} {}", view.team, view.roster_season);
    let _ = writeln!(
        out,
        "RANK  COMBINATION                         FIT    DELTA"
    );
    for candidate in view.candidates.iter().take(12) {
        let _ = writeln!(
            out,
            "{:>4}  {:<34} {:>5.1}  {:+.2}",
            candidate.rank, candidate.label, candidate.score.total, candidate.strength_delta
        );
    }
    let _ = writeln!(out, "\nBEST OVERALL");
    for player in view.player_leaderboards.best_overall.iter().take(10) {
        let raw = player
            .overall_score
            .map(|value| format!("{value:.1}"))
            .unwrap_or_else(|| "NR".to_owned());
        let adjusted = player
            .reliability_adjusted_score
            .map(|value| format!("{value:.1}"))
            .unwrap_or_else(|| "NR".to_owned());
        let _ = writeln!(
            out,
            "  {:<28} adj {adjusted:>5}  raw {raw:>5}  GP {:>3}",
            player.display_name, player.sample_games
        );
    }
    if !view.player_leaderboards.deployment_anchors.is_empty() {
        let _ = writeln!(out, "\nPRIOR DEPLOYMENT ANCHORS");
        for player in view.player_leaderboards.deployment_anchors.iter().take(10) {
            let _ = writeln!(
                out,
                "  {:<28} {:>5.1}%  pairs {:>2}",
                player.display_name,
                player.deployment_affinity_score.unwrap_or_default(),
                player.deployment_observations
            );
        }
    }
    if view.player_leaderboards.positive_multipliers.is_empty()
        && view.player_leaderboards.negative_multipliers.is_empty()
    {
        let _ = writeln!(
            out,
            "\nLINE DRIVERS / DRAGS: NO READ — supply labeled pair evidence"
        );
    } else {
        let _ = writeln!(out, "\nPOSITIVE MULTIPLIERS");
        for player in view
            .player_leaderboards
            .positive_multipliers
            .iter()
            .take(10)
        {
            let _ = writeln!(
                out,
                "  {:<28} {:+.1}",
                player.display_name,
                player.teammate_effect_score.unwrap_or_default()
            );
        }
        let _ = writeln!(out, "\nNEGATIVE MULTIPLIERS");
        for player in view
            .player_leaderboards
            .negative_multipliers
            .iter()
            .take(10)
        {
            let _ = writeln!(
                out,
                "  {:<28} {:+.1}",
                player.display_name,
                player.teammate_effect_score.unwrap_or_default()
            );
        }
    }
    for warning in &view.warnings {
        let _ = writeln!(out, "WARNING: {warning}");
    }
    out
}

pub async fn run_season(args: IceCastSeasonArgs) -> anyhow::Result<()> {
    let (view, focus, _) = build_season_view(&args).await?;
    let output = if args.json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render(&view, &focus, args.all_games)
    };
    if let Some(path) = args.out {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        std::fs::write(&path, output.as_bytes())
            .with_context(|| format!("write {}", path.display()))?;
        println!("Wrote IceCast forecast to {}", path.display());
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_edge(args: IceCastEdgeArgs) -> anyhow::Result<()> {
    let forecast_bytes = std::fs::read(&args.forecast)
        .with_context(|| format!("read baseline forecast {}", args.forecast.display()))?;
    let forecast: TeamGameForecastView = serde_json::from_slice(&forecast_bytes)
        .with_context(|| format!("parse baseline forecast {}", args.forecast.display()))?;
    let evidence = load_game_prediction_edge_evidence_package(&args.evidence)
        .with_context(|| format!("load edge evidence {}", args.evidence.display()))?;
    let forecast_fingerprint = format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(&forecast)?)
    );
    if evidence.source_forecast_fingerprint != forecast_fingerprint {
        bail!(
            "edge evidence binds forecast {}, but input forecast is {}",
            evidence.source_forecast_fingerprint,
            forecast_fingerprint
        );
    }
    if evidence.season != forecast.season {
        bail!(
            "edge evidence season {} does not match forecast season {}",
            evidence.season,
            forecast.season
        );
    }
    let model = args
        .model
        .as_ref()
        .map(|path| -> anyhow::Result<TeamGamePredictionModel> {
            let bytes = std::fs::read(path)
                .with_context(|| format!("read edge model {}", path.display()))?;
            serde_json::from_slice(&bytes)
                .with_context(|| format!("parse edge model {}", path.display()))
        })
        .transpose()?
        .unwrap_or_else(TeamGamePredictionModel::evaluation_challenger);
    let edge = build_team_game_prediction_edge(
        &forecast,
        evidence.vintage,
        evidence.created_at,
        model,
        evidence.games,
    )?;
    if let Some(path) = args.enhanced_forecast_out.as_ref() {
        let bytes = serde_json::to_vec_pretty(&edge.enhanced_forecast)?;
        write_icecast_file(path, &bytes, "enhanced game forecast")?;
    }
    let output = if args.json {
        format!("{}\n", serde_json::to_string_pretty(&edge)?)
    } else {
        render_edge(&edge)
    };
    if let Some(path) = args.out.as_ref() {
        write_icecast_file(path, output.as_bytes(), "IceCast prediction edge")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_edge_card(
    inputs: Vec<PathBuf>,
    game_id: u64,
    team: String,
    team_name: Option<String>,
    generated_at: Option<String>,
    market_benchmark: Option<PathBuf>,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let edges = inputs
        .iter()
        .map(|path| -> anyhow::Result<TeamGamePredictionEdgeView> {
            let bytes = std::fs::read(path)
                .with_context(|| format!("read prediction edge {}", path.display()))?;
            serde_json::from_slice(&bytes)
                .with_context(|| format!("parse prediction edge {}", path.display()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let season = edges
        .first()
        .map(|edge| edge.season)
        .context("--input requires at least one edge document")?;
    let evidence_at = generated_at
        .as_deref()
        .map(DateTime::parse_from_rfc3339)
        .transpose()
        .context("--generated-at must be RFC 3339")?
        .map(|value| value.with_timezone(&Utc));
    let mut view = ViewContext::new(ViewWindow::new(Season(season), SeasonType::Regular));
    view.generated_at = evidence_at;
    let team = team.trim().to_ascii_uppercase();
    let market_benchmark = market_benchmark
        .as_deref()
        .map(|path| {
            read_icecast_json::<TeamGamePredictionMarketBenchmarkInput>(
                path,
                "closing-market benchmark",
            )
        })
        .transpose()?;
    let card = build_team_game_prediction_edge_card(TeamGamePredictionEdgeCardInput {
        edges,
        game_id,
        focus_team: team.clone(),
        team_name: team_name.unwrap_or_else(|| team.clone()),
        view,
        evidence_at,
        market_benchmark,
    })?;
    let output = format!("{}\n", serde_json::to_string_pretty(&card)?);
    if let Some(path) = out.as_ref() {
        write_icecast_file(path, output.as_bytes(), "IceCast prediction edge card")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_season_simulate(
    forecast_path: PathBuf,
    trials: u32,
    seed: u64,
    scenario_path: Option<PathBuf>,
    through: Option<NaiveDate>,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let bytes = std::fs::read(&forecast_path)
        .with_context(|| format!("read season simulation input {}", forecast_path.display()))?;
    let schema = serde_json::from_slice::<serde_json::Value>(&bytes)
        .with_context(|| format!("parse season simulation input {}", forecast_path.display()))?
        .get("schema")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let forecast: TeamGameForecastView = match schema.as_str() {
        "team_game_forecast.v1" => serde_json::from_slice(&bytes)
            .with_context(|| format!("parse team game forecast {}", forecast_path.display()))?,
        "team_season_forecast.v1" => {
            let archived: TeamSeasonForecastView =
                serde_json::from_slice(&bytes).with_context(|| {
                    format!("parse archived season forecast {}", forecast_path.display())
                })?;
            rehydrate_team_game_forecast(&archived, TeamForecastParameters::default())
                .map_err(anyhow::Error::msg)?
        }
        _ => bail!(
            "season simulation input {} must use team_game_forecast.v1 or team_season_forecast.v1",
            forecast_path.display()
        ),
    };
    let scenario = scenario_path
        .as_deref()
        .map(|path| load_scenario(path, forecast.season))
        .transpose()?;
    let config = TeamSeasonSimulationConfig { trials, seed };
    let mut simulation = match through {
        Some(cutoff) => {
            simulate_team_season_forecast_as_of_with_scenario(&forecast, config, scenario, cutoff)
        }
        None => simulate_team_season_forecast_with_scenario(&forecast, config, scenario),
    }
    .map_err(anyhow::Error::msg)?;
    simulation.disclosures.push(format!(
        "Season simulation consumed the supplied {} game forecast without recomputing game probabilities.",
        forecast.forecast_mode
    ));
    let output = format!("{}\n", serde_json::to_string_pretty(&simulation)?);
    if let Some(path) = out.as_ref() {
        write_icecast_file(path, output.as_bytes(), "IceCast season simulation")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_edge_evidence(
    forecast_path: PathBuf,
    input_path: PathBuf,
    line_matchup_paths: Vec<PathBuf>,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let forecast_bytes = std::fs::read(&forecast_path)
        .with_context(|| format!("read baseline forecast {}", forecast_path.display()))?;
    let forecast: TeamGameForecastView = serde_json::from_slice(&forecast_bytes)
        .with_context(|| format!("parse baseline forecast {}", forecast_path.display()))?;
    let input_bytes = std::fs::read(&input_path)
        .with_context(|| format!("read edge assembly input {}", input_path.display()))?;
    let mut input: GamePredictionEvidencePackageBuildInput =
        serde_json::from_slice(&input_bytes)
            .with_context(|| format!("parse edge assembly input {}", input_path.display()))?;
    if input.season != forecast.season {
        bail!(
            "edge assembly season {} does not match forecast season {}",
            input.season,
            forecast.season
        );
    }
    let mut attached_games = BTreeSet::new();
    for path in line_matchup_paths {
        let matchup: PlayerLineMatchupForecastView =
            read_icecast_json(&path, "player-line matchup forecast")?;
        if matchup.season != input.season || !attached_games.insert(matchup.game_id) {
            bail!(
                "line matchup {} has a mismatched season or duplicate game {}",
                path.display(),
                matchup.game_id
            );
        }
        let game = input
            .games
            .iter_mut()
            .find(|game| game.game_id == matchup.game_id)
            .with_context(|| {
                format!(
                    "line matchup {} references game {} absent from edge assembly",
                    path.display(),
                    matchup.game_id
                )
            })?;
        attach_player_line_matchup_forecast(game, &matchup)?;
        input.sources.push(GamePredictionEvidenceSource {
            source_key: format!("icelines.player_line_matchup.{}", matchup.game_id),
            evidence_cutoff_at: matchup.captured_at,
            retrieved_at: matchup.captured_at,
            authority: GamePredictionEvidenceSourceAuthority::LiveCapture,
            source_uri: format!("icelines://game/{}/player-line-matchup", matchup.game_id),
            fingerprint: matchup.fingerprint,
        });
    }
    let forecast_fingerprint = format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(&forecast)?)
    );
    let result = build_game_prediction_edge_evidence_package(forecast_fingerprint, input)?;
    for warning in &result.warnings {
        eprintln!("WARNING: {warning}");
    }
    let output = format!("{}\n", serde_json::to_string_pretty(&result.package)?);
    if let Some(path) = out.as_ref() {
        write_icecast_file(path, output.as_bytes(), "IceCast edge evidence package")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_edge_preseason_evidence(
    forecast_path: PathBuf,
    created_at: String,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let forecast: TeamGameForecastView =
        read_icecast_json(&forecast_path, "preseason team game forecast")?;
    let created_at = DateTime::parse_from_rfc3339(&created_at)
        .context("--created-at must be RFC 3339")?
        .with_timezone(&Utc);
    let result = build_preseason_game_prediction_edge_evidence_package(&forecast, created_at)?;
    for warning in &result.warnings {
        eprintln!("WARNING: {warning}");
    }
    let output = format!("{}\n", serde_json::to_string_pretty(&result.package)?);
    if let Some(path) = out.as_ref() {
        write_icecast_file(path, output.as_bytes(), "IceCast preseason edge evidence")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_edge_observe(
    edge_paths: Vec<PathBuf>,
    outcome_paths: Vec<PathBuf>,
    created_at: String,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let created_at = DateTime::parse_from_rfc3339(&created_at)
        .context("--created-at must be RFC 3339")?
        .with_timezone(&Utc);
    let edges = edge_paths
        .iter()
        .map(|path| -> anyhow::Result<TeamGamePredictionEdgeView> {
            let bytes = std::fs::read(path)
                .with_context(|| format!("read frozen edge {}", path.display()))?;
            serde_json::from_slice(&bytes)
                .with_context(|| format!("parse frozen edge {}", path.display()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut outcomes = Vec::new();
    for outcomes_path in &outcome_paths {
        let outcome_bytes = std::fs::read(outcomes_path)
            .with_context(|| format!("read outcomes {}", outcomes_path.display()))?;
        let mut rows = match serde_json::from_slice::<OfficialGameOutcomeSet>(&outcome_bytes) {
            Ok(set) => {
                set.validate().with_context(|| {
                    format!("validate official outcomes {}", outcomes_path.display())
                })?;
                set.outcomes
            }
            Err(_) => serde_json::from_slice(&outcome_bytes)
                .with_context(|| format!("parse outcomes {}", outcomes_path.display()))?,
        };
        outcomes.append(&mut rows);
    }
    let set = build_team_game_prediction_observation_set(&edges, &outcomes, created_at)?;
    let output = format!("{}\n", serde_json::to_string_pretty(&set)?);
    if let Some(path) = out.as_ref() {
        write_icecast_file(path, output.as_bytes(), "IceCast edge observation set")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub async fn run_edge_outcomes(
    season: u32,
    captured_at: String,
    refresh: bool,
    allow_partial: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let captured_at = DateTime::parse_from_rfc3339(&captured_at)
        .context("--captured-at must be RFC 3339")?
        .with_timezone(&Utc);
    let schedule = load_fantasy_schedule(Season(season), refresh).await?;
    let source_url = format!("https://api-web.nhle.com/v1/club-schedule-season/{{team}}/{season}");
    let set = build_official_game_outcome_set(
        &schedule,
        season,
        captured_at,
        source_url,
        !allow_partial,
    )?;
    let output = format!("{}\n", serde_json::to_string_pretty(&set)?);
    if let Some(path) = out.as_ref() {
        write_icecast_file(path, output.as_bytes(), "official IceCast game outcomes")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_edge_replay_xg(
    forecast_path: PathBuf,
    moneypuck_dir: PathBuf,
    retrieved_at: String,
    trailing_games: usize,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let retrieved_at = DateTime::parse_from_rfc3339(&retrieved_at)
        .context("--retrieved-at must be RFC 3339")?
        .with_timezone(&Utc);
    let forecast_bytes = std::fs::read(&forecast_path)
        .with_context(|| format!("read historical forecast {}", forecast_path.display()))?;
    let forecast: TeamGameForecastView = serde_json::from_slice(&forecast_bytes)
        .with_context(|| format!("parse historical forecast {}", forecast_path.display()))?;
    let teams = forecast
        .games
        .iter()
        .flat_map(|game| [game.away_team.clone(), game.home_team.clone()])
        .collect::<BTreeSet<_>>();
    let inputs = teams
        .into_iter()
        .map(|team| -> anyhow::Result<HistoricalMoneyPuckTeamInput> {
            let path = moneypuck_dir.join(format!("{team}.csv"));
            let csv_text = std::fs::read_to_string(&path)
                .with_context(|| format!("read MoneyPuck team file {}", path.display()))?;
            Ok(HistoricalMoneyPuckTeamInput {
                team: team.clone(),
                csv_text,
                source_uri: moneypuck_team_game_url(&team),
                retrieved_at,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let result =
        build_historical_moneypuck_edge_package(&forecast, inputs, retrieved_at, trailing_games)?;
    eprintln!(
        "IceCast historical edge coverage: {} games, roster {}/{}, xG {}/{}, opponent-adjusted xG {}/{}, special teams {}/{} sides",
        result.games,
        result.roster_sides,
        result.games * 2,
        result.xg_sides,
        result.games * 2,
        result.opponent_adjusted_xg_sides,
        result.games * 2,
        result.special_teams_sides,
        result.games * 2
    );
    for warning in result.warnings.iter().take(20) {
        eprintln!("WARNING: {warning}");
    }
    if result.warnings.len() > 20 {
        eprintln!(
            "WARNING: {} additional coverage warnings omitted from text",
            result.warnings.len() - 20
        );
    }
    let output = format!("{}\n", serde_json::to_string_pretty(&result.package)?);
    if let Some(path) = out.as_ref() {
        write_icecast_file(path, output.as_bytes(), "historical IceCast xG evidence")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn run_edge_replay_confirmed(
    forecast_path: PathBuf,
    morning_evidence_path: PathBuf,
    boxscore_dir: PathBuf,
    retrieved_at: String,
    refresh: bool,
    concurrency: usize,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    if !(1..=32).contains(&concurrency) {
        bail!("--concurrency must be between 1 and 32");
    }
    let retrieved_at = DateTime::parse_from_rfc3339(&retrieved_at)
        .context("--retrieved-at must be RFC 3339")?
        .with_timezone(&Utc);
    let forecast_bytes = std::fs::read(&forecast_path)
        .with_context(|| format!("read historical forecast {}", forecast_path.display()))?;
    let forecast: TeamGameForecastView = serde_json::from_slice(&forecast_bytes)
        .with_context(|| format!("parse historical forecast {}", forecast_path.display()))?;
    let morning =
        load_game_prediction_edge_evidence_package(&morning_evidence_path).with_context(|| {
            format!(
                "load game-morning evidence {}",
                morning_evidence_path.display()
            )
        })?;
    std::fs::create_dir_all(&boxscore_dir)
        .with_context(|| format!("create boxscore cache {}", boxscore_dir.display()))?;

    let game_ids = forecast
        .games
        .iter()
        .map(|game| game.game_id)
        .collect::<BTreeSet<_>>();
    let mut raw_by_game = BTreeMap::new();
    if !refresh {
        for game_id in &game_ids {
            let path = boxscore_dir.join(format!("{game_id}.json"));
            let bytes = std::fs::read(&path)
                .with_context(|| format!("read cached official boxscore {}", path.display()))?;
            let raw = serde_json::from_slice(&bytes)
                .with_context(|| format!("parse cached official boxscore {}", path.display()))?;
            raw_by_game.insert(*game_id, raw);
        }
    } else {
        let client = NhlApiClient::production();
        let mut tasks = tokio::task::JoinSet::<anyhow::Result<(u64, serde_json::Value)>>::new();
        for game_id in game_ids.iter().copied() {
            while tasks.len() >= concurrency {
                let completed = tasks
                    .join_next()
                    .await
                    .context("official boxscore task set closed unexpectedly")??;
                let (completed_id, raw) = completed?;
                raw_by_game.insert(completed_id, raw);
                if raw_by_game.len() % 250 == 0 {
                    eprintln!(
                        "IceCast confirmed replay: {}/{} official boxscores cached",
                        raw_by_game.len(),
                        game_ids.len()
                    );
                }
            }
            let client = client.clone();
            let path = boxscore_dir.join(format!("{game_id}.json"));
            tasks.spawn(async move {
                match client.fetch_boxscore_with_raw(game_id).await {
                    Ok((_, raw)) => {
                        let bytes = serde_json::to_vec(&raw)?;
                        icelines_fetch::snapshot::atomic_write_bytes(&path, &bytes)
                            .with_context(|| format!("cache official boxscore {game_id}"))?;
                        Ok::<_, anyhow::Error>((game_id, raw))
                    }
                    Err(fetch_error) if path.exists() => {
                        let bytes = std::fs::read(&path).with_context(|| {
                            format!("read fallback cached official boxscore {game_id}")
                        })?;
                        let raw = serde_json::from_slice(&bytes).with_context(|| {
                            format!("parse fallback cached official boxscore {game_id}")
                        })?;
                        eprintln!(
                            "WARNING: official boxscore {game_id} refresh failed; using cache: {fetch_error}"
                        );
                        Ok((game_id, raw))
                    }
                    Err(error) => Err(anyhow::anyhow!(
                        "fetch official boxscore {game_id}: {error}"
                    )),
                }
            });
        }
        while let Some(result) = tasks.join_next().await {
            let (game_id, raw) = result??;
            raw_by_game.insert(game_id, raw);
            if raw_by_game.len() % 250 == 0 || raw_by_game.len() == game_ids.len() {
                eprintln!(
                    "IceCast confirmed replay: {}/{} official boxscores cached",
                    raw_by_game.len(),
                    game_ids.len()
                );
            }
        }
    }

    let inputs = raw_by_game
        .into_iter()
        .map(|(game_id, raw)| HistoricalOfficialBoxscoreInput {
            game_id,
            raw,
            source_uri: format!("https://api-web.nhle.com/v1/gamecenter/{game_id}/boxscore"),
            retrieved_at,
        })
        .collect();
    let result =
        build_historical_confirmed_edge_package(&forecast, &morning, inputs, retrieved_at)?;
    eprintln!(
        "IceCast confirmed edge coverage: {} games, availability {}/{}, goalies {}/{} sides",
        result.games,
        result.availability_sides,
        result.games * 2,
        result.goalie_sides,
        result.games * 2
    );
    for warning in result.warnings.iter().take(20) {
        eprintln!("WARNING: {warning}");
    }
    if result.warnings.len() > 20 {
        eprintln!(
            "WARNING: {} additional coverage warnings omitted from text",
            result.warnings.len() - 20
        );
    }
    let output = format!("{}\n", serde_json::to_string_pretty(&result.package)?);
    if let Some(path) = out.as_ref() {
        write_icecast_file(
            path,
            output.as_bytes(),
            "historical confirmed IceCast evidence",
        )?;
    } else {
        print!("{output}");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn run_edge_replay_goalies(
    forecast_path: PathBuf,
    confirmed_evidence_path: PathBuf,
    goalie_dir: PathBuf,
    retrieved_at: String,
    trailing_appearances: usize,
    refresh: bool,
    concurrency: usize,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    if !(1..=32).contains(&concurrency) {
        bail!("--concurrency must be between 1 and 32");
    }
    if trailing_appearances == 0 {
        bail!("--trailing-appearances must be greater than zero");
    }
    let retrieved_at = DateTime::parse_from_rfc3339(&retrieved_at)
        .context("--retrieved-at must be RFC 3339")?
        .with_timezone(&Utc);
    let forecast_bytes = std::fs::read(&forecast_path)
        .with_context(|| format!("read historical forecast {}", forecast_path.display()))?;
    let forecast: TeamGameForecastView = serde_json::from_slice(&forecast_bytes)
        .with_context(|| format!("parse historical forecast {}", forecast_path.display()))?;
    let confirmed = load_game_prediction_edge_evidence_package(&confirmed_evidence_path)
        .with_context(|| {
            format!(
                "load pregame-confirmed evidence {}",
                confirmed_evidence_path.display()
            )
        })?;
    let goalie_ids = confirmed
        .games
        .iter()
        .flat_map(|game| [&game.away, &game.home])
        .map(|team| {
            team.goalie_player_id.ok_or_else(|| {
                anyhow::anyhow!(
                    "confirmed evidence for game-side {} lacks a starter player ID; rerun edge-replay-confirmed with the current IceLines build",
                    team.team
                )
            })
        })
        .collect::<anyhow::Result<BTreeSet<_>>>()?;
    std::fs::create_dir_all(&goalie_dir)
        .with_context(|| format!("create goalie cache {}", goalie_dir.display()))?;

    let mut csv_by_goalie = BTreeMap::new();
    let mut ids_to_fetch = BTreeSet::new();
    for player_id in &goalie_ids {
        let path = goalie_dir.join(format!("{player_id}.csv"));
        if !refresh && path.exists() {
            let csv_text = std::fs::read_to_string(&path)
                .with_context(|| format!("read cached MoneyPuck goalie file {}", path.display()))?;
            csv_by_goalie.insert(*player_id, csv_text);
        } else {
            ids_to_fetch.insert(*player_id);
        }
    }
    if !ids_to_fetch.is_empty() {
        let client = reqwest::Client::builder()
            .user_agent("IceLines/edge-replay-goalies")
            .build()
            .context("build MoneyPuck HTTP client")?;
        let mut tasks = tokio::task::JoinSet::<anyhow::Result<(u32, String)>>::new();
        for player_id in ids_to_fetch.iter().copied() {
            while tasks.len() >= concurrency {
                let (completed_id, csv_text) = tasks
                    .join_next()
                    .await
                    .context("MoneyPuck goalie task set closed unexpectedly")???;
                csv_by_goalie.insert(completed_id, csv_text);
            }
            let client = client.clone();
            let path = goalie_dir.join(format!("{player_id}.csv"));
            tasks.spawn(async move {
                let url = moneypuck_goalie_game_url(player_id);
                let fetched = fetch_bytes_with_backoff(&client, &url, 8).await;
                let bytes = match fetched {
                    Ok(bytes) => {
                        icelines_fetch::snapshot::atomic_write_bytes(&path, &bytes)
                            .with_context(|| format!("cache MoneyPuck goalie {player_id}"))?;
                        bytes
                    }
                    Err(fetch_error) if path.exists() => {
                        eprintln!(
                            "WARNING: MoneyPuck goalie {player_id} refresh failed; using cache: {fetch_error}"
                        );
                        std::fs::read(&path).with_context(|| {
                            format!("read fallback cached MoneyPuck goalie {player_id}")
                        })?
                    }
                    Err(error) => {
                        return Err(anyhow::anyhow!(
                            "fetch MoneyPuck goalie {player_id} from {url}: {error}"
                        ));
                    }
                };
                let csv_text = String::from_utf8(bytes)
                    .with_context(|| format!("MoneyPuck goalie {player_id} CSV is not UTF-8"))?;
                Ok((player_id, csv_text))
            });
        }
        while let Some(result) = tasks.join_next().await {
            let (player_id, csv_text) = result??;
            csv_by_goalie.insert(player_id, csv_text);
            if csv_by_goalie.len() % 50 == 0 || csv_by_goalie.len() == goalie_ids.len() {
                eprintln!(
                    "IceCast goalie replay: {}/{} career files cached",
                    csv_by_goalie.len(),
                    goalie_ids.len()
                );
            }
        }
    }

    let inputs = csv_by_goalie
        .into_iter()
        .map(|(player_id, csv_text)| HistoricalMoneyPuckGoalieInput {
            player_id,
            csv_text,
            source_uri: moneypuck_goalie_game_url(player_id),
            retrieved_at,
        })
        .collect();
    let result = build_historical_goalie_edge_package(
        &forecast,
        &confirmed,
        inputs,
        trailing_appearances,
        retrieved_at,
    )?;
    eprintln!(
        "IceCast goalie edge coverage: {} games, {} goalies, form {}/{}, workload {}/{} sides",
        result.games,
        result.requested_goalies,
        result.form_sides,
        result.games * 2,
        result.workload_sides,
        result.games * 2
    );
    for warning in result.warnings.iter().take(20) {
        eprintln!("WARNING: {warning}");
    }
    if result.warnings.len() > 20 {
        eprintln!(
            "WARNING: {} additional coverage warnings omitted from text",
            result.warnings.len() - 20
        );
    }
    let output = format!("{}\n", serde_json::to_string_pretty(&result.package)?);
    if let Some(path) = out.as_ref() {
        write_icecast_file(
            path,
            output.as_bytes(),
            "historical goalie-form IceCast evidence",
        )?;
    } else {
        print!("{output}");
    }
    Ok(())
}

async fn fetch_bytes_with_backoff(
    client: &reqwest::Client,
    url: &str,
    attempts: usize,
) -> anyhow::Result<Vec<u8>> {
    let mut last_error = None;
    for attempt in 0..attempts.max(1) {
        match client.get(url).send().await {
            Ok(response) if response.status().is_success() => {
                let bytes = response.bytes().await?.to_vec();
                // MoneyPuck career endpoints throttle bursts aggressively.
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                return Ok(bytes);
            }
            Ok(response)
                if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS
                    || response.status().is_server_error() =>
            {
                let status = response.status();
                let retry_after = response
                    .headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.parse::<u64>().ok())
                    .unwrap_or(1_u64 << attempt.min(4))
                    .min(30);
                last_error = Some(anyhow::anyhow!("HTTP status {status}"));
                if attempt + 1 < attempts {
                    tokio::time::sleep(std::time::Duration::from_secs(retry_after)).await;
                }
            }
            Ok(response) => return Err(response.error_for_status().unwrap_err().into()),
            Err(error) => {
                last_error = Some(error.into());
                if attempt + 1 < attempts {
                    tokio::time::sleep(std::time::Duration::from_secs(1_u64 << attempt.min(4)))
                        .await;
                }
            }
        }
    }
    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("HTTP fetch failed without a response")))
}

pub fn run_edge_register_holdout(
    season: u32,
    registered_at: String,
    outcome_not_before: String,
    config_path: Option<PathBuf>,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let registered_at = DateTime::parse_from_rfc3339(&registered_at)
        .context("--registered-at must be RFC 3339")?
        .with_timezone(&Utc);
    let outcome_not_before = DateTime::parse_from_rfc3339(&outcome_not_before)
        .context("--outcome-not-before must be RFC 3339")?
        .with_timezone(&Utc);
    let config = config_path
        .as_ref()
        .map(|path| -> anyhow::Result<TeamGamePredictionTrainingConfig> {
            let bytes = std::fs::read(path)
                .with_context(|| format!("read training config {}", path.display()))?;
            serde_json::from_slice(&bytes)
                .with_context(|| format!("parse training config {}", path.display()))
        })
        .transpose()?
        .unwrap_or_default();
    let registration =
        register_team_game_prediction_holdout(&config, season, registered_at, outcome_not_before)?;
    let output = format!("{}\n", serde_json::to_string_pretty(&registration)?);
    if let Some(path) = out.as_ref() {
        write_icecast_file(
            path,
            output.as_bytes(),
            "IceCast prospective holdout registration",
        )?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_edge_train(args: IceCastEdgeTrainArgs) -> anyhow::Result<()> {
    let observation_bytes = std::fs::read(&args.observations)
        .with_context(|| format!("read observations {}", args.observations.display()))?;
    let observations: Vec<TeamGamePredictionTrainingObservation> =
        match serde_json::from_slice::<TeamGamePredictionObservationSet>(&observation_bytes) {
            Ok(set) => {
                set.validate().with_context(|| {
                    format!("validate observations {}", args.observations.display())
                })?;
                set.observations
            }
            Err(_) => serde_json::from_slice(&observation_bytes)
                .with_context(|| format!("parse observations {}", args.observations.display()))?,
        };
    let supplied_config = args
        .config
        .as_ref()
        .map(|path| -> anyhow::Result<TeamGamePredictionTrainingConfig> {
            let bytes = std::fs::read(path)
                .with_context(|| format!("read training config {}", path.display()))?;
            serde_json::from_slice(&bytes)
                .with_context(|| format!("parse training config {}", path.display()))
        })
        .transpose()?;
    let mut config = supplied_config.clone().unwrap_or_default();
    if supplied_config.is_none() {
        if let Some(observation) = observations.first() {
            config.vintage = observation.vintage;
        }
    }
    let registration = args
        .registration
        .as_ref()
        .map(
            |path| -> anyhow::Result<TeamGamePredictionHoldoutRegistration> {
                let bytes = std::fs::read(path)
                    .with_context(|| format!("read holdout registration {}", path.display()))?;
                serde_json::from_slice(&bytes)
                    .with_context(|| format!("parse holdout registration {}", path.display()))
            },
        )
        .transpose()?;
    if registration.is_some() && !args.validate {
        bail!("--registration requires --validate");
    }
    let (value, model) = if args.validate {
        let view = validate_team_game_prediction_model_with_registration(
            &observations,
            config,
            registration.as_ref(),
        )?;
        let model = view.final_model.clone();
        (serde_json::to_value(view)?, model)
    } else {
        let view = train_team_game_prediction_model(&observations, config)?;
        let model = view.model.clone();
        (serde_json::to_value(view)?, model)
    };
    if let Some(path) = args.model_out.as_ref() {
        let model_output = format!("{}\n", serde_json::to_string_pretty(&model)?);
        write_icecast_file(path, model_output.as_bytes(), "IceCast edge model")?;
    }
    let output = format!("{}\n", serde_json::to_string_pretty(&value)?);
    if let Some(path) = args.out.as_ref() {
        write_icecast_file(path, output.as_bytes(), "IceCast edge training result")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

fn render_edge(view: &icelines_core::TeamGamePredictionEdgeView) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "IceCast prediction edge · {} · {:?}",
        view.season, view.vintage
    );
    let _ = writeln!(
        out,
        "Model: {} ({:?}) · {} games",
        view.model.model_id,
        view.model.authority,
        view.games.len()
    );
    for game in &view.games {
        let _ = writeln!(
            out,
            "{}  {} at {}  {:>5.1}% -> {:>5.1}%  ({}/{} evidence)",
            game.date,
            game.away_team,
            game.home_team,
            game.blended_home_win_probability * 100.0,
            game.enhanced_home_win_probability * 100.0,
            game.available_features,
            game.expected_features
        );
    }
    for warning in view.warnings.iter().take(10) {
        let _ = writeln!(out, "WARNING: {warning}");
    }
    if view.warnings.len() > 10 {
        let _ = writeln!(
            out,
            "WARNING: {} additional warnings are retained in JSON",
            view.warnings.len() - 10
        );
    }
    out
}

pub fn run_season_card(
    input: PathBuf,
    team: String,
    team_name: Option<String>,
    generated_at: Option<String>,
    calendar_fingerprint: Option<String>,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let bytes = std::fs::read(&input)
        .with_context(|| format!("read IceCast forecast {}", input.display()))?;
    let forecast: TeamSeasonForecastView = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse IceCast forecast {}", input.display()))?;
    let evidence_at = generated_at
        .as_deref()
        .map(DateTime::parse_from_rfc3339)
        .transpose()
        .context("--generated-at must be RFC 3339, for example 2026-07-22T12:00:00Z")?
        .map(|value| value.with_timezone(&Utc));
    let mut view = ViewContext::new(ViewWindow::new(
        Season(forecast.season),
        SeasonType::Regular,
    ));
    view.generated_at = evidence_at;
    let team = team.trim().to_ascii_uppercase();
    let card = build_season_simulation_card(SeasonSimulationCardInput {
        forecast,
        focus_team: team.clone(),
        team_name: team_name.unwrap_or_else(|| team.clone()),
        view,
        evidence_at,
        calendar_fingerprint,
    })?;
    let output = format!("{}\n", serde_json::to_string_pretty(&card)?);
    if let Some(path) = out {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        std::fs::write(&path, output.as_bytes())
            .with_context(|| format!("write {}", path.display()))?;
        println!("Wrote IceCast season card to {}", path.display());
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_movement(
    earlier: PathBuf,
    later: PathBuf,
    earlier_label: Option<String>,
    later_label: Option<String>,
    teams: Vec<String>,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let load = |path: &PathBuf| -> anyhow::Result<TeamSeasonForecastView> {
        let bytes = std::fs::read(path)
            .with_context(|| format!("read IceCast forecast {}", path.display()))?;
        serde_json::from_slice(&bytes)
            .with_context(|| format!("parse IceCast forecast {}", path.display()))
    };
    let mut movement = build_team_season_forecast_movement(&load(&earlier)?, &load(&later)?)
        .map_err(anyhow::Error::msg)?;
    movement.earlier_label = normalize_movement_label(earlier_label, "--earlier-label")?;
    movement.later_label = normalize_movement_label(later_label, "--later-label")?;
    let focus = if teams.is_empty() {
        ["NYR", "SEA"]
            .into_iter()
            .filter(|team| movement.teams.iter().any(|row| row.team == *team))
            .map(str::to_owned)
            .collect::<Vec<_>>()
    } else {
        teams
            .into_iter()
            .map(|team| team.trim().to_ascii_uppercase())
            .collect::<Vec<_>>()
    };
    for team in &focus {
        if !movement.teams.iter().any(|row| row.team == *team) {
            bail!("team {team} is absent from the compared IceCast runs");
        }
    }
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&movement)?)
    } else {
        render_movement(&movement, &focus)
    };
    if let Some(path) = out {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        std::fs::write(&path, output.as_bytes())
            .with_context(|| format!("write {}", path.display()))?;
        println!("Wrote IceCast movement to {}", path.display());
    } else {
        print!("{output}");
    }
    Ok(())
}

fn normalize_movement_label(
    label: Option<String>,
    argument: &str,
) -> anyhow::Result<Option<String>> {
    label
        .map(|label| {
            let label = label.trim();
            if label.is_empty() || label.chars().count() > 80 {
                bail!("{argument} must contain 1 to 80 characters");
            }
            Ok(label.to_owned())
        })
        .transpose()
}

pub fn run_movement_card(
    input: PathBuf,
    team: String,
    team_name: Option<String>,
    generated_at: Option<String>,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let bytes = std::fs::read(&input)
        .with_context(|| format!("read IceCast movement {}", input.display()))?;
    let movement: TeamSeasonForecastMovementView = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse IceCast movement {}", input.display()))?;
    let evidence_at = generated_at
        .as_deref()
        .map(DateTime::parse_from_rfc3339)
        .transpose()
        .context("--generated-at must be RFC 3339, for example 2027-02-15T12:00:00Z")?
        .map(|value| value.with_timezone(&Utc));
    let mut view = ViewContext::new(ViewWindow::new(
        Season(movement.season),
        SeasonType::Regular,
    ));
    view.generated_at = evidence_at;
    let team = team.trim().to_ascii_uppercase();
    let card = build_forecast_movement_card(ForecastMovementCardInput {
        movement,
        focus_team: team.clone(),
        team_name: team_name.unwrap_or_else(|| team.clone()),
        view,
        evidence_at,
    })?;
    let output = format!("{}\n", serde_json::to_string_pretty(&card)?);
    if let Some(path) = out {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        std::fs::write(&path, output.as_bytes())
            .with_context(|| format!("write {}", path.display()))?;
        println!("Wrote IceCast movement card to {}", path.display());
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_history(
    inputs: Vec<PathBuf>,
    teams: Vec<String>,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    if inputs.len() < 2 {
        bail!("icecast history requires at least two --input forecast artifacts");
    }
    let forecasts = inputs
        .iter()
        .map(|path| {
            let bytes = std::fs::read(path)
                .with_context(|| format!("read IceCast forecast {}", path.display()))?;
            serde_json::from_slice(&bytes)
                .with_context(|| format!("parse IceCast forecast {}", path.display()))
        })
        .collect::<anyhow::Result<Vec<TeamSeasonForecastView>>>()?;
    let history = build_team_season_forecast_history(&forecasts).map_err(anyhow::Error::msg)?;
    let focus = if teams.is_empty() {
        ["NYR", "SEA"]
            .into_iter()
            .filter(|team| history.teams.iter().any(|row| row.team == *team))
            .map(str::to_owned)
            .collect::<Vec<_>>()
    } else {
        teams
            .into_iter()
            .map(|team| team.trim().to_ascii_uppercase())
            .collect::<Vec<_>>()
    };
    for team in &focus {
        if !history.teams.iter().any(|row| row.team == *team) {
            bail!("team {team} is absent from the IceCast history");
        }
    }
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&history)?)
    } else {
        render_history(&history, &focus)
    };
    if let Some(path) = out {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        std::fs::write(&path, output.as_bytes())
            .with_context(|| format!("write {}", path.display()))?;
        println!("Wrote IceCast history to {}", path.display());
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_history_card(
    input: PathBuf,
    team: String,
    team_name: Option<String>,
    generated_at: Option<String>,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let bytes = std::fs::read(&input)
        .with_context(|| format!("read IceCast history {}", input.display()))?;
    let history: TeamSeasonForecastHistoryView = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse IceCast history {}", input.display()))?;
    let evidence_at = generated_at
        .as_deref()
        .map(DateTime::parse_from_rfc3339)
        .transpose()
        .context("--generated-at must be RFC 3339, for example 2025-03-01T12:00:00Z")?
        .map(|value| value.with_timezone(&Utc));
    let mut view = ViewContext::new(ViewWindow::new(Season(history.season), SeasonType::Regular));
    view.generated_at = evidence_at;
    let team = team.trim().to_ascii_uppercase();
    let card = build_forecast_history_card(ForecastHistoryCardInput {
        history,
        focus_team: team.clone(),
        team_name: team_name.unwrap_or_else(|| team.clone()),
        view,
        evidence_at,
    })?;
    let output = format!("{}\n", serde_json::to_string_pretty(&card)?);
    if let Some(path) = out {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        std::fs::write(&path, output.as_bytes())
            .with_context(|| format!("write {}", path.display()))?;
        println!("Wrote IceCast history card to {}", path.display());
    } else {
        print!("{output}");
    }
    Ok(())
}

pub struct WindowBuildArgs {
    pub season: u32,
    pub as_of: NaiveDate,
    pub generated_at: String,
    pub source_package: Option<PathBuf>,
    pub team_season_forecast: Option<PathBuf>,
    pub team_game_forecast: Option<PathBuf>,
    pub team_lineups: Vec<PathBuf>,
    pub ahl_affiliates: Vec<PathBuf>,
    pub organization_lineups: Vec<PathBuf>,
    pub prospect_program: Option<PathBuf>,
    pub prospect_conversion: Option<PathBuf>,
    pub training_camp: Option<PathBuf>,
    pub schedule_rest: Vec<PathBuf>,
    pub profile_history: Option<PathBuf>,
    pub require_ranked: bool,
    pub out: PathBuf,
}

pub fn run_window_profile_history_build(
    boards: Vec<PathBuf>,
    history_id: String,
    created_at: String,
    out: PathBuf,
) -> anyhow::Result<()> {
    DateTime::parse_from_rfc3339(&created_at)
        .context("--created-at must be RFC 3339, for example 2026-07-29T12:00:00Z")?;
    if boards.is_empty() {
        bail!("at least one --board is required");
    }
    let inventory = load_organization_window_profile_inventory()?;
    let boards = boards
        .iter()
        .map(|path| {
            let board: OrganizationWindowBoardView =
                read_icecast_json(path, "organization Window board")?;
            validate_organization_window_board(&board, &inventory)?;
            Ok(board)
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let history = build_organization_profile_history(history_id, created_at, &boards)?;
    let output = format!("{}\n", serde_json::to_string_pretty(&history)?);
    write_icecast_file(&out, output.as_bytes(), "organization profile history")?;
    Ok(())
}

pub fn run_window_profile_history_backfill(
    origins: Vec<PathBuf>,
    history_id: String,
    created_at: String,
    out: PathBuf,
) -> anyhow::Result<()> {
    DateTime::parse_from_rfc3339(&created_at)
        .context("--created-at must be RFC 3339, for example 2026-07-30T18:00:00Z")?;
    if origins.is_empty() {
        bail!("at least one --origin is required");
    }
    let artifacts = origins
        .iter()
        .map(|path| {
            let artifact: OrganizationWindowHistoricalOriginArtifact =
                read_icecast_json(path, "organization Window historical origin")?;
            artifact.validate()?;
            Ok(artifact)
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let boards = artifacts
        .into_iter()
        .map(|artifact| artifact.origin.board)
        .collect::<Vec<_>>();
    let history = build_organization_profile_history(history_id, created_at, &boards)?;
    let output = format!("{}\n", serde_json::to_string_pretty(&history)?);
    write_icecast_file(
        &out,
        output.as_bytes(),
        "organization profile history backfill",
    )?;
    Ok(())
}

pub fn run_window_profile_history_baseline(
    source_package: PathBuf,
    ahl_workboard: PathBuf,
    history_id: String,
    created_at: String,
    out: PathBuf,
) -> anyhow::Result<()> {
    DateTime::parse_from_rfc3339(&created_at)
        .context("--created-at must be RFC 3339, for example 2026-07-29T12:00:00Z")?;
    let package: OrganizationWindowSourcePackageView =
        read_icecast_json(&source_package, "prior-season Window source package")?;
    let workboard = read_affiliate_workboard(&ahl_workboard)?;
    let history = build_prior_season_organization_profile_history(
        &package, &workboard, history_id, created_at,
    )?;
    let output = format!("{}\n", serde_json::to_string_pretty(&history)?);
    write_icecast_file(
        &out,
        output.as_bytes(),
        "organization profile history baseline",
    )?;
    Ok(())
}

pub fn run_window_profile_history_audit(
    input: PathBuf,
    generated_at: String,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    DateTime::parse_from_rfc3339(&generated_at)
        .context("--generated-at must be RFC 3339, for example 2026-07-30T12:00:00Z")?;
    let history: OrganizationProfileHistoryView =
        read_icecast_json(&input, "organization profile history")?;
    let coverage = audit_organization_profile_history(&history, generated_at)?;
    let output = format!("{}\n", serde_json::to_string_pretty(&coverage)?);
    if let Some(path) = out {
        write_icecast_file(
            &path,
            output.as_bytes(),
            "organization profile history coverage",
        )?;
    } else {
        print!("{output}");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn run_window_profile_history_delta(
    input: PathBuf,
    earlier_season: u32,
    earlier_as_of: NaiveDate,
    later_season: u32,
    later_as_of: NaiveDate,
    horizon: String,
    generated_at: String,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    DateTime::parse_from_rfc3339(&generated_at)
        .context("--generated-at must be RFC 3339, for example 2026-07-30T18:00:00Z")?;
    let horizon = match horizon.trim().to_ascii_lowercase().as_str() {
        "current" => WindowHorizon::Current,
        "one_year" | "one-year" => WindowHorizon::OneYear,
        "three_year" | "three-year" => WindowHorizon::ThreeYear,
        "five_year" | "five-year" => WindowHorizon::FiveYear,
        other => bail!(
            "unsupported --horizon '{other}'; use current, one_year, three_year, or five_year"
        ),
    };
    let history: OrganizationProfileHistoryView =
        read_icecast_json(&input, "organization profile history")?;
    let delta = compare_organization_profile_history(
        &history,
        OrganizationProfileHistoryCheckpointView {
            season: earlier_season,
            as_of: earlier_as_of,
            horizon,
        },
        OrganizationProfileHistoryCheckpointView {
            season: later_season,
            as_of: later_as_of,
            horizon,
        },
        generated_at,
    )?;
    let output = format!("{}\n", serde_json::to_string_pretty(&delta)?);
    if let Some(path) = out {
        write_icecast_file(
            &path,
            output.as_bytes(),
            "organization profile history delta",
        )?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_window_profile_history_card(
    input: PathBuf,
    team: String,
    team_name: Option<String>,
    generated_at: Option<String>,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let evidence_at = generated_at
        .as_deref()
        .map(DateTime::parse_from_rfc3339)
        .transpose()
        .context("--generated-at must be RFC 3339, for example 2026-07-30T18:00:00Z")?
        .map(|value| value.with_timezone(&Utc));
    let delta: OrganizationProfileHistoryDeltaView =
        read_icecast_json(&input, "organization profile history delta")?;
    let card =
        project_organization_profile_history_card(delta, &team, team_name.as_deref(), evidence_at)?;
    let output = format!("{}\n", serde_json::to_string_pretty(&card)?);
    if let Some(path) = out {
        write_icecast_file(
            &path,
            output.as_bytes(),
            "organization profile history card",
        )?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub struct WindowSourcePackageArgs {
    pub season: u32,
    pub as_of: NaiveDate,
    pub team_season_forecast: Option<PathBuf>,
    pub team_game_forecast: Option<PathBuf>,
    pub cache_team_lineups: bool,
    pub stats_season: String,
    pub team_lineups: Vec<PathBuf>,
    pub ahl_affiliates: Vec<PathBuf>,
    pub ahl_projection_inputs: Option<PathBuf>,
    pub organization_lineups: Vec<PathBuf>,
    pub prospect_program: Option<PathBuf>,
    pub cache_prospect_program: bool,
    pub career_history: Option<PathBuf>,
    pub prospect_conversion: Option<PathBuf>,
    pub training_camp: Option<PathBuf>,
    pub schedule_rest: Vec<PathBuf>,
    pub profile_history: Option<PathBuf>,
    pub out: PathBuf,
}

pub struct WindowSourceRefreshLineupsArgs {
    pub input: PathBuf,
    pub stats_season: String,
    pub training_camp: Option<PathBuf>,
    pub career_history: Option<PathBuf>,
    pub out: PathBuf,
}

pub struct WindowSourceRefreshAffiliatesArgs {
    pub input: PathBuf,
    pub ahl_projection_inputs: Option<PathBuf>,
    pub ahl_facts_application: Option<PathBuf>,
    pub ahl_development_rule: Option<PathBuf>,
    pub out: PathBuf,
}

struct WindowSourcePaths<'a> {
    season: u32,
    as_of: NaiveDate,
    team_season_forecast: Option<&'a Path>,
    team_game_forecast: Option<&'a Path>,
    team_lineups: &'a [PathBuf],
    ahl_affiliates: &'a [PathBuf],
    ahl_projection_inputs: Option<&'a Path>,
    organization_lineups: &'a [PathBuf],
    prospect_program: Option<&'a Path>,
    prospect_conversion: Option<&'a Path>,
    training_camp: Option<&'a Path>,
    schedule_rest: &'a [PathBuf],
    profile_history: Option<&'a Path>,
}

fn load_window_source_package(
    paths: WindowSourcePaths<'_>,
) -> anyhow::Result<OrganizationWindowSourcePackageView> {
    let team_season_forecast = paths
        .team_season_forecast
        .map(|path| read_icecast_json(path, "team season forecast"))
        .transpose()?;
    let team_game_forecast = paths
        .team_game_forecast
        .map(|path| read_icecast_json(path, "team game forecast"))
        .transpose()?;
    let team_lineups = paths
        .team_lineups
        .iter()
        .map(|path| read_icecast_json(path, "team lineup"))
        .collect::<anyhow::Result<Vec<TeamLineupProjectionView>>>()?;
    let mut ahl_affiliates = paths
        .ahl_affiliates
        .iter()
        .map(|path| read_icecast_json(path, "AHL affiliate projection"))
        .collect::<anyhow::Result<Vec<AhlAffiliateProjectionView>>>()?;
    if let Some(path) = paths.ahl_projection_inputs {
        let league: AhlPreseasonLeagueProjectionInputsView =
            read_icecast_json(path, "AHL preseason league projection inputs")?;
        ahl_affiliates = build_window_affiliates_from_league_inputs(&league, paths.season)?;
    }
    let organization_lineups = paths
        .organization_lineups
        .iter()
        .map(|path| read_icecast_json(path, "organization lineup"))
        .collect::<anyhow::Result<Vec<OrganizationLineupForecastView>>>()?;
    let prospect_program = paths
        .prospect_program
        .map(|path| read_icecast_json(path, "prospect program"))
        .transpose()?;
    let prospect_conversion = paths
        .prospect_conversion
        .map(|path| read_icecast_json(path, "prospect conversion"))
        .transpose()?;
    let training_camp = paths
        .training_camp
        .map(|path| read_icecast_json(path, "training camp league forecast"))
        .transpose()?;
    let schedule_rest = paths
        .schedule_rest
        .iter()
        .map(|path| read_icecast_json(path, "schedule rest profile"))
        .collect::<anyhow::Result<Vec<ScheduleRestProfileView>>>()?;
    let profile_history = paths
        .profile_history
        .map(|path| read_icecast_json(path, "organization profile history"))
        .transpose()?;
    Ok(seal_organization_window_source_package(
        OrganizationWindowSourcePackageView {
            schema: ORGANIZATION_WINDOW_SOURCE_PACKAGE_SCHEMA.to_owned(),
            season: paths.season,
            season_type: "regular".to_owned(),
            as_of: paths.as_of,
            organization_identity_version: "nhl_32.v1".to_owned(),
            team_season_forecast,
            team_game_forecast,
            team_lineups,
            ahl_affiliates,
            organization_lineups,
            prospect_program,
            prospect_conversion,
            training_camp,
            schedule_rest,
            profile_history,
            fingerprint: String::new(),
        },
    )?)
}

fn build_window_affiliates_from_league_inputs(
    league: &AhlPreseasonLeagueProjectionInputsView,
    season: u32,
) -> anyhow::Result<Vec<AhlAffiliateProjectionView>> {
    if league.schema != AHL_PRESEASON_LEAGUE_PROJECTION_INPUTS_SCHEMA
        || league.target_season != season
        || league.teams_requested != CANONICAL_TEAMS.len()
        || league.teams_built != league.teams_requested
        || league.inputs.len() != league.teams_built
        || !league.failures.is_empty()
    {
        bail!(
            "AHL league projection inputs must be complete for Window season {} (schema {}, requested {}, built {}, failures {})",
            season,
            league.schema,
            league.teams_requested,
            league.teams_built,
            league.failures.len()
        );
    }
    league
        .inputs
        .iter()
        .map(build_ahl_affiliate_projection)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|reason| anyhow::anyhow!("build reviewed AHL league projection: {reason}"))
}

pub fn run_window_source_package(args: WindowSourcePackageArgs) -> anyhow::Result<()> {
    let mut package = load_window_source_package(WindowSourcePaths {
        season: args.season,
        as_of: args.as_of,
        team_season_forecast: args.team_season_forecast.as_deref(),
        team_game_forecast: args.team_game_forecast.as_deref(),
        team_lineups: &args.team_lineups,
        ahl_affiliates: &args.ahl_affiliates,
        ahl_projection_inputs: args.ahl_projection_inputs.as_deref(),
        organization_lineups: &args.organization_lineups,
        prospect_program: args.prospect_program.as_deref(),
        prospect_conversion: args.prospect_conversion.as_deref(),
        training_camp: args.training_camp.as_deref(),
        schedule_rest: &args.schedule_rest,
        profile_history: args.profile_history.as_deref(),
    })?;
    if args.cache_team_lineups {
        let roster_season: Season =
            args.season.to_string().parse().map_err(|error| {
                anyhow::anyhow!("invalid roster season '{}': {error}", args.season)
            })?;
        let stats_season: Season = args.stats_season.parse().map_err(|error| {
            anyhow::anyhow!("invalid stats season '{}': {error}", args.stats_season)
        })?;
        package.team_lineups =
            super::report::load_league_team_lineup_views(roster_season, stats_season)?;
        if let Some(camp) = package.training_camp.as_ref() {
            let career_history_path = args
                .career_history
                .clone()
                .unwrap_or(Config::load()?.career_history_path());
            let career_store =
                CareerHistoryStore::load(&career_history_path).with_context(|| {
                    format!(
                        "read Window goalie career cache {}",
                        career_history_path.display()
                    )
                })?;
            package.team_lineups = complete_lineup_goalies_with_training_camp(
                &package.team_lineups,
                camp,
                &career_store,
                stats_season.0,
                &NhlGoalieTranslationPolicy::default(),
            )
            .map_err(anyhow::Error::msg)?;
        }
    }
    if args.cache_prospect_program {
        let career_history_path = args
            .career_history
            .unwrap_or(Config::load()?.career_history_path());
        let store = CareerHistoryStore::load(&career_history_path).with_context(|| {
            format!(
                "read Window prospect career cache {}",
                career_history_path.display()
            )
        })?;
        if store.histories.is_empty() || store.birth_dates.is_empty() {
            bail!(
                "Window prospect composition requires populated career histories and birth dates in {}; run `icelines fetch career --camp-forecast <camp.json>` first",
                career_history_path.display()
            );
        }
        let forecast = package
            .training_camp
            .clone()
            .context("--cache-prospect-program requires a training-camp authority")?;
        let composition = build_prospect_program_from_camp_and_career_store(
            forecast,
            &store,
            ProspectCareerProgramConfig {
                context: ProspectCareerContextDraftConfig {
                    as_of_date: args.as_of,
                    ..ProspectCareerContextDraftConfig::default()
                },
                ..ProspectCareerProgramConfig::default()
            },
        )
        .map_err(anyhow::Error::msg)?;
        package.prospect_program = Some(composition.program);
    }
    if args.cache_team_lineups || args.cache_prospect_program {
        package.fingerprint.clear();
        package = seal_organization_window_source_package(package)?;
    }
    let output = format!("{}\n", serde_json::to_string_pretty(&package)?);
    write_icecast_file(
        &args.out,
        output.as_bytes(),
        "organization Window source package",
    )
}

pub fn run_window_source_refresh_lineups(
    args: WindowSourceRefreshLineupsArgs,
) -> anyhow::Result<()> {
    let package: OrganizationWindowSourcePackageView =
        read_icecast_json(&args.input, "organization Window source package")?;
    let mut package = seal_organization_window_source_package(package)?;
    if let Some(path) = args.training_camp.as_deref() {
        package.training_camp = Some(read_icecast_json(path, "training camp league forecast")?);
    }
    let roster_season: Season =
        package.season.to_string().parse().map_err(|error| {
            anyhow::anyhow!("invalid package season '{}': {error}", package.season)
        })?;
    let stats_season: Season = args.stats_season.parse().map_err(|error| {
        anyhow::anyhow!("invalid stats season '{}': {error}", args.stats_season)
    })?;
    package.team_lineups =
        super::report::load_league_team_lineup_views(roster_season, stats_season)?;
    if let Some(camp) = package.training_camp.as_ref() {
        let career_history_path = args
            .career_history
            .unwrap_or(Config::load()?.career_history_path());
        let career_store = CareerHistoryStore::load(&career_history_path).with_context(|| {
            format!(
                "read Window goalie career cache {}",
                career_history_path.display()
            )
        })?;
        package.team_lineups = complete_lineup_goalies_with_training_camp(
            &package.team_lineups,
            camp,
            &career_store,
            stats_season.0,
            &NhlGoalieTranslationPolicy::default(),
        )
        .map_err(anyhow::Error::msg)?;
    }
    package.fingerprint.clear();
    package = seal_organization_window_source_package(package)?;
    let output = format!("{}\n", serde_json::to_string_pretty(&package)?);
    write_icecast_file(
        &args.out,
        output.as_bytes(),
        "refreshed organization Window source package",
    )
}

pub fn run_window_source_refresh_affiliates(
    args: WindowSourceRefreshAffiliatesArgs,
) -> anyhow::Result<()> {
    let package: OrganizationWindowSourcePackageView =
        read_icecast_json(&args.input, "organization Window source package")?;
    let mut package = seal_organization_window_source_package(package)?;
    if !package.organization_lineups.is_empty() {
        bail!(
            "affiliate refresh refuses a package with explicit organization lineups; rebuild with one organization-lineup authority"
        );
    }
    let league = match (
        args.ahl_projection_inputs.as_deref(),
        args.ahl_facts_application.as_deref(),
        args.ahl_development_rule.as_deref(),
    ) {
        (Some(path), None, None) => {
            read_icecast_json(path, "AHL preseason league projection inputs")?
        }
        (None, Some(application_path), Some(rule_path)) => {
            let application: AhlPreseasonLeagueFactsApplicationView =
                read_icecast_json(application_path, "AHL preseason facts application")?;
            let rule: icelines_core::AhlDevelopmentRuleInput =
                read_icecast_json(rule_path, "AHL development rule")?;
            build_ahl_preseason_league_projection_inputs(&application, &rule)
                .map_err(anyhow::Error::msg)?
        }
        _ => bail!(
            "affiliate refresh requires either --ahl-projection-inputs or both --ahl-facts-application and --ahl-development-rule"
        ),
    };
    package.ahl_affiliates = build_window_affiliates_from_league_inputs(&league, package.season)?;
    package.fingerprint.clear();
    package = seal_organization_window_source_package(package)?;
    let output = format!("{}\n", serde_json::to_string_pretty(&package)?);
    write_icecast_file(
        &args.out,
        output.as_bytes(),
        "affiliate-refreshed organization Window source package",
    )
}

pub fn run_window_source_refresh_history(
    input: PathBuf,
    profile_history: PathBuf,
    out: PathBuf,
) -> anyhow::Result<()> {
    let package: OrganizationWindowSourcePackageView =
        read_icecast_json(&input, "organization Window source package")?;
    let mut package = seal_organization_window_source_package(package)?;
    let history: OrganizationProfileHistoryView =
        read_icecast_json(&profile_history, "organization profile history")?;
    package.profile_history = Some(seal_organization_profile_history(history)?);
    package.fingerprint.clear();
    package = seal_organization_window_source_package(package)?;
    let output = format!("{}\n", serde_json::to_string_pretty(&package)?);
    write_icecast_file(
        &out,
        output.as_bytes(),
        "history-refreshed organization Window source package",
    )
}

pub fn run_window_source_audit(
    input: PathBuf,
    generated_at: String,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    DateTime::parse_from_rfc3339(&generated_at)
        .context("--generated-at must be RFC 3339, for example 2026-10-01T12:00:00Z")?;
    let package: OrganizationWindowSourcePackageView =
        read_icecast_json(&input, "organization Window source package")?;
    let coverage = audit_organization_window_source_package(&package, generated_at)?;
    let output = format!("{}\n", serde_json::to_string_pretty(&coverage)?);
    if let Some(path) = out {
        write_icecast_file(&path, output.as_bytes(), "organization Window source audit")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_window_build(args: WindowBuildArgs) -> anyhow::Result<()> {
    DateTime::parse_from_rfc3339(&args.generated_at)
        .context("--generated-at must be RFC 3339, for example 2026-10-01T12:00:00Z")?;
    let package = if let Some(path) = args.source_package.as_deref() {
        let package: OrganizationWindowSourcePackageView =
            read_icecast_json(path, "organization Window source package")?;
        seal_organization_window_source_package(package)?
    } else {
        load_window_source_package(WindowSourcePaths {
            season: args.season,
            as_of: args.as_of,
            team_season_forecast: args.team_season_forecast.as_deref(),
            team_game_forecast: args.team_game_forecast.as_deref(),
            team_lineups: &args.team_lineups,
            ahl_affiliates: &args.ahl_affiliates,
            ahl_projection_inputs: None,
            organization_lineups: &args.organization_lineups,
            prospect_program: args.prospect_program.as_deref(),
            prospect_conversion: args.prospect_conversion.as_deref(),
            training_camp: args.training_camp.as_deref(),
            schedule_rest: &args.schedule_rest,
            profile_history: args.profile_history.as_deref(),
        })?
    };
    if package.season != args.season || package.as_of != args.as_of {
        bail!(
            "source package identifies season {} at {}; window-build requested {} at {}",
            package.season,
            package.as_of,
            args.season,
            args.as_of
        );
    }
    let board = build_balanced_organization_window_board_from_package(&package, args.generated_at)?;
    if args.require_ranked {
        require_ranked_balanced_organization_window_board(&board)?;
    }
    let output = format!("{}\n", serde_json::to_string_pretty(&board)?);
    write_icecast_file(
        &args.out,
        output.as_bytes(),
        "official balanced organization Window",
    )
}

pub fn run_window(
    input: PathBuf,
    team: Option<String>,
    json: bool,
    markdown: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let board: OrganizationWindowBoardView = read_icecast_json(&input, "organization Window")?;
    let inventory = load_organization_window_profile_inventory()?;
    validate_organization_window_board(&board, &inventory)?;
    let focus = team.map(|team| team.trim().to_ascii_uppercase());
    if let Some(team) = &focus {
        if board.organization(team).is_none() {
            bail!("team {team} is absent from the organization Window board");
        }
    }
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&board)?)
    } else if markdown {
        render_window_markdown(&board, focus.as_deref())
    } else {
        render_window(&board, focus.as_deref())
    };
    if let Some(path) = out {
        write_icecast_file(&path, output.as_bytes(), "organization Window")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

fn render_window_markdown(board: &OrganizationWindowBoardView, focus: Option<&str>) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "---");
    let _ = writeln!(out, "schema: organization_window_report.v1");
    let _ = writeln!(out, "season: {}", board.season);
    let _ = writeln!(out, "season_type: {}", board.season_type);
    let _ = writeln!(out, "as_of: {}", board.as_of);
    let _ = writeln!(out, "frame: {}", board.manifest.manifest_id);
    let _ = writeln!(out, "manifest_fingerprint: {}", board.manifest.fingerprint);
    let _ = writeln!(out, "board_fingerprint: {}", board.fingerprint);
    let _ = writeln!(out, "league_coverage: {:.6}", board.league_coverage);
    if let Some(team) = focus {
        let _ = writeln!(out, "focus: {team}");
    }
    let _ = writeln!(out, "---\n");
    let _ = writeln!(out, "# The Window\n");
    let _ = writeln!(
        out,
        "Frozen {}-organization cohort as of **{}** using Frame **{}**. Score, confidence, and coverage are distinct; `NR` means rank was withheld.\n",
        board.expected_organizations.len(),
        board.as_of,
        markdown_cell(&board.manifest.manifest_id)
    );
    let _ = writeln!(
        out,
        "| Rank | Team | Score | Confidence | Coverage | Classification | Rank status |"
    );
    let _ = writeln!(out, "|---:|:---|---:|---:|---:|:---|:---|");
    for row in board
        .organizations_in_display_order()
        .into_iter()
        .filter(|row| focus.is_none_or(|team| row.organization == team))
    {
        let rank = row
            .overall
            .rank
            .map(|rank| rank.to_string())
            .unwrap_or_else(|| "NR".to_owned());
        let score = row
            .overall
            .score
            .map(|score| format!("{score:.1}"))
            .unwrap_or_else(|| "NR".to_owned());
        let classification = row
            .overall
            .published_classification()
            .map(|classification| format!("{classification:?}"))
            .unwrap_or_else(|| "Under review".to_owned());
        let _ = writeln!(
            out,
            "| {rank} | {} | {score} | {:.0}% | {:.0}% | {classification} | {:?} |",
            row.organization,
            row.overall.confidence * 100.0,
            row.overall.coverage * 100.0,
            row.overall.rank_status.state
        );
    }
    for row in board
        .organizations_in_display_order()
        .into_iter()
        .filter(|row| focus.is_some_and(|team| row.organization == team))
    {
        let _ = writeln!(out, "\n## {} detail\n", row.organization);
        let _ = writeln!(out, "| Pane | Score | Confidence | Coverage | State |");
        let _ = writeln!(out, "|:---|---:|---:|---:|:---|");
        for dimension in &row.dimensions {
            let score = dimension
                .score
                .map(|score| format!("{score:.1}"))
                .unwrap_or_else(|| "NR".to_owned());
            let _ = writeln!(
                out,
                "| {} | {score} | {:.0}% | {:.0}% | {:?} |",
                markdown_cell(&dimension.label),
                dimension.confidence * 100.0,
                dimension.coverage * 100.0,
                dimension.status
            );
        }
        let _ = writeln!(out, "\n### Lines and evidence\n");
        let _ = writeln!(
            out,
            "| Pane | Profile | Method | Raw | Score | Confidence | Coverage | Status |"
        );
        let _ = writeln!(out, "|:---|:---|:---|---:|---:|---:|---:|:---|");
        for dimension in &row.dimensions {
            for profile in &dimension.profiles {
                let raw = profile
                    .raw_value
                    .map(|value| format!("{value:.3}"))
                    .unwrap_or_else(|| "—".to_owned());
                let score = profile
                    .normalized_score
                    .map(|value| format!("{value:.1}"))
                    .unwrap_or_else(|| "—".to_owned());
                let _ = writeln!(
                    out,
                    "| {} | {} | {} | {raw} | {score} | {:.0}% | {:.0}% | {:?} |",
                    markdown_cell(&dimension.key),
                    markdown_cell(&profile.profile_key),
                    markdown_cell(&profile.method_version),
                    profile.confidence * 100.0,
                    profile.coverage * 100.0,
                    profile.status
                );
            }
        }
        if !row.blockers.is_empty() {
            let _ = writeln!(out, "\n### Blockers\n");
            for blocker in &row.blockers {
                let _ = writeln!(out, "- {}", markdown_cell(blocker));
            }
        }
    }
    let _ = writeln!(out, "\n## Disclosures\n");
    for disclosure in &board.disclosures {
        let _ = writeln!(out, "- {}", markdown_cell(disclosure));
    }
    out
}

fn markdown_cell(value: &str) -> String {
    value.replace('|', "\\|").replace(['\r', '\n'], " ")
}

pub fn run_window_card(
    input: PathBuf,
    team: String,
    team_name: Option<String>,
    generated_at: Option<String>,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let board: OrganizationWindowBoardView = read_icecast_json(&input, "organization Window")?;
    let evidence_at = generated_at
        .as_deref()
        .map(DateTime::parse_from_rfc3339)
        .transpose()
        .context("--generated-at must be RFC 3339, for example 2026-10-01T12:00:00Z")?
        .map(|value| value.with_timezone(&Utc));
    let team = team.trim().to_ascii_uppercase();
    let card = project_organization_window_card(board, &team, team_name.as_deref(), evidence_at)?;
    let output = format!("{}\n", serde_json::to_string_pretty(&card)?);
    if let Some(path) = out {
        write_icecast_file(&path, output.as_bytes(), "organization Window card")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_window_movement(
    earlier: PathBuf,
    later: PathBuf,
    bridge: Option<PathBuf>,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let earlier = read_icecast_json(&earlier, "earlier organization Window")?;
    let later = read_icecast_json(&later, "later organization Window")?;
    let movement = if let Some(path) = bridge {
        let bridge: OrganizationWindowBridgeView =
            read_icecast_json(&path, "organization Window bridge")?;
        let inventory = load_organization_window_profile_inventory()?;
        compare_organization_window_snapshots_with_bridge(&earlier, &later, &bridge, &inventory)?
    } else {
        compare_organization_window_snapshots(&earlier, &later)?
    };
    write_window_json(&movement, out.as_deref(), "organization Window movement")
}

pub fn run_window_personnel_attribution(
    earlier: PathBuf,
    later: PathBuf,
    movement: PathBuf,
    input: PathBuf,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let earlier: OrganizationWindowBoardView =
        read_icecast_json(&earlier, "earlier organization Window")?;
    let later: OrganizationWindowBoardView =
        read_icecast_json(&later, "later organization Window")?;
    let movement: OrganizationWindowMovementView =
        read_icecast_json(&movement, "organization Window movement")?;
    let input: OrganizationWindowPersonnelAttributionInputView =
        read_icecast_json(&input, "organization Window personnel attribution input")?;
    let attributed =
        attribute_organization_window_personnel_movement(&earlier, &later, movement, input)?;
    write_window_json(
        &attributed,
        out.as_deref(),
        "organization Window personnel attribution",
    )
}

#[allow(clippy::too_many_arguments)]
pub fn run_window_personnel_input_build(
    actual_forecast: PathBuf,
    counterfactual_board: PathBuf,
    earlier_as_of: NaiveDate,
    later_as_of: NaiveDate,
    attribution_id: String,
    scenario_id: String,
    rationale: String,
    out: PathBuf,
) -> anyhow::Result<()> {
    let forecast: TeamSeasonForecastView =
        read_icecast_json(&actual_forecast, "actual team-season forecast")?;
    let counterfactual: OrganizationWindowBoardView =
        read_icecast_json(&counterfactual_board, "counterfactual organization Window")?;
    let input = build_later_counterfactual_personnel_attribution_input(
        attribution_id,
        scenario_id,
        rationale,
        &forecast,
        counterfactual,
        earlier_as_of,
        later_as_of,
    )?;
    write_window_json(
        &input,
        Some(&out),
        "organization Window personnel attribution input",
    )
}

pub fn run_window_personnel_summary(input: PathBuf, out: Option<PathBuf>) -> anyhow::Result<()> {
    let movement: OrganizationWindowMovementView =
        read_icecast_json(&input, "attributed organization Window movement")?;
    let summary = summarize_organization_window_personnel_evidence(&movement)?;
    write_window_json(
        &summary,
        out.as_deref(),
        "organization Window personnel evidence summary",
    )
}

pub fn run_window_rebase(
    input: PathBuf,
    target_manifest: PathBuf,
    bridge: PathBuf,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let source: OrganizationWindowBoardView =
        read_icecast_json(&input, "source organization Window")?;
    let target: OrganizationWindowManifestView =
        read_icecast_json(&target_manifest, "target organization Window manifest")?;
    let bridge: OrganizationWindowBridgeView =
        read_icecast_json(&bridge, "organization Window bridge")?;
    let inventory = load_organization_window_profile_inventory()?;
    let lifecycle = load_organization_window_registry_lifecycle(&inventory)?;
    let target = seal_new_organization_window_manifest(
        target,
        &inventory,
        &lifecycle,
        &WindowManifestLifecyclePolicy::custom(),
    )?;
    let mut rebased = rebase_organization_window_board(&source, &target, &bridge, &inventory)?;
    rebased
        .source_fingerprints
        .push(format!("registry-lifecycle:{}", lifecycle.fingerprint));
    rebased.source_fingerprints.sort();
    rebased.source_fingerprints.dedup();
    rebased.fingerprint = rebased.calculate_fingerprint()?;
    write_window_json(&rebased, out.as_deref(), "rebased organization Window")
}

pub fn run_window_history(inputs: Vec<PathBuf>, out: Option<PathBuf>) -> anyhow::Result<()> {
    let boards = inputs
        .iter()
        .map(|path| read_icecast_json(path, "organization Window checkpoint"))
        .collect::<anyhow::Result<Vec<OrganizationWindowBoardView>>>()?;
    let history = build_organization_window_history(&boards)?;
    write_window_json(&history, out.as_deref(), "organization Window history")
}

pub fn run_window_scenario(
    baseline: PathBuf,
    scenario: PathBuf,
    scenario_id: String,
    authorities: Vec<PathBuf>,
    team_season_authorities: Vec<PathBuf>,
    training_camp_authorities: Vec<PathBuf>,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let baseline = read_icecast_json(&baseline, "baseline organization Window")?;
    let scenario = read_icecast_json(&scenario, "scenario organization Window")?;
    let has_typed_authorities = !authorities.is_empty()
        || !team_season_authorities.is_empty()
        || !training_camp_authorities.is_empty();
    let impact = if !has_typed_authorities {
        compare_organization_window_scenario(&scenario_id, &baseline, &scenario)?
    } else {
        let mut authorities = authorities
            .iter()
            .map(|path| read_icecast_json(path, "organization Window scenario authority"))
            .collect::<anyhow::Result<Vec<WindowScenarioAuthorityView>>>()?;
        for path in &team_season_authorities {
            let forecast: TeamSeasonForecastView =
                read_icecast_json(path, "team-season scenario authority")?;
            authorities.extend(adapt_team_season_window_scenario_authorities(&forecast)?);
        }
        for path in &training_camp_authorities {
            let forecast: TrainingCampLeagueForecastView =
                read_icecast_json(path, "training-camp scenario authority")?;
            authorities.extend(adapt_training_camp_window_scenario_authorities(&forecast)?);
        }
        compare_organization_window_typed_scenario(&scenario_id, &baseline, &scenario, authorities)?
    };
    write_window_json(&impact, out.as_deref(), "organization Window scenario")
}

pub fn run_window_scenario_distribution(
    baseline: PathBuf,
    input: PathBuf,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let baseline: OrganizationWindowBoardView =
        read_icecast_json(&baseline, "baseline organization Window")?;
    let input: OrganizationWindowScenarioDistributionInput =
        read_icecast_json(&input, "organization Window scenario distribution input")?;
    let distribution = simulate_organization_window_scenario_distribution(&baseline, input)?;
    write_window_json(
        &distribution,
        out.as_deref(),
        "organization Window scenario distribution",
    )
}

pub fn run_window_calibrate(
    target: String,
    origins: Vec<PathBuf>,
    minimum_origins: usize,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let origins = origins
        .iter()
        .map(|path| read_icecast_json(path, "organization Window calibration origin"))
        .collect::<anyhow::Result<Vec<WindowCalibrationOriginInput>>>()?;
    let calibration =
        calibrate_organization_window_rolling_origins(&target, &origins, minimum_origins)?;
    write_window_json(
        &calibration,
        out.as_deref(),
        "organization Window rolling calibration",
    )
}

pub fn run_window_evaluate(
    target: String,
    origins: Vec<PathBuf>,
    minimum_training_origins: usize,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let origins = origins
        .iter()
        .map(|path| {
            let value: serde_json::Value =
                read_icecast_json(path, "labeled organization Window evaluation origin")?;
            if value["schema"].as_str() == Some(ORGANIZATION_WINDOW_HISTORICAL_ORIGIN_SCHEMA) {
                let artifact: OrganizationWindowHistoricalOriginArtifact =
                    serde_json::from_value(value).with_context(|| {
                        format!(
                            "parse historical organization Window origin {}",
                            path.display()
                        )
                    })?;
                artifact.validate().with_context(|| {
                    format!(
                        "validate historical organization Window origin {}",
                        path.display()
                    )
                })?;
                Ok(artifact.evaluation_input())
            } else {
                serde_json::from_value::<WindowCalibrationEvaluationOriginInput>(value)
                    .with_context(|| {
                        format!(
                            "parse labeled organization Window origin {}",
                            path.display()
                        )
                    })
            }
        })
        .collect::<anyhow::Result<Vec<WindowCalibrationEvaluationOriginInput>>>()?;
    let evaluation =
        evaluate_organization_window_origins(&target, &origins, minimum_training_origins)?;
    write_window_json(
        &evaluation,
        out.as_deref(),
        "organization Window split evaluation",
    )
}

pub async fn run_window_standings(
    target_season: u32,
    date: String,
    captured_at: String,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let effective_date = NaiveDate::parse_from_str(&date, "%Y-%m-%d")
        .with_context(|| format!("invalid standings date {date}; expected YYYY-MM-DD"))?;
    let rows = NhlApiClient::production()
        .fetch_standings_for_date(&date)
        .await
        .with_context(|| format!("fetch official NHL standings for {date}"))?;
    let snapshot = build_organization_window_standings_snapshot(
        target_season,
        effective_date,
        &captured_at,
        &rows,
    )?;
    write_window_json(
        &snapshot,
        out.as_deref(),
        "organization Window standings outcome",
    )
}

#[allow(clippy::too_many_arguments)]
pub fn run_window_origin_build(
    source_season: u32,
    target_season: u32,
    as_of: String,
    generated_at: String,
    role: String,
    standings: PathBuf,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let as_of = NaiveDate::parse_from_str(&as_of, "%Y-%m-%d")
        .with_context(|| format!("invalid feature cutoff {as_of}; expected YYYY-MM-DD"))?;
    let role = match role.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "training" => WindowCalibrationOriginRole::Training,
        "validation" => WindowCalibrationOriginRole::Validation,
        "retrospective_holdout" => WindowCalibrationOriginRole::RetrospectiveHoldout,
        value => bail!(
            "invalid Window origin role {value}; expected training, validation, or retrospective_holdout"
        ),
    };
    let outcome: OrganizationWindowStandingsSnapshot =
        read_icecast_json(&standings, "organization Window standings outcome")?;
    let source = source_season.to_string();
    let stats = get_stats(&source)
        .with_context(|| format!("bundled source season {source_season} has no stats.json"))?;
    let bios = get_bios(&source)
        .with_context(|| format!("bundled source season {source_season} has no bios.json"))?;
    let artifact = build_historical_organization_window_origin(
        source_season,
        target_season,
        as_of,
        &generated_at,
        role,
        &stats,
        &bios,
        &outcome,
    )?;
    write_window_json(
        &artifact,
        out.as_deref(),
        "historical organization Window origin",
    )
}

pub fn run_window_holdout_register(
    source_season: u32,
    target_season: u32,
    feature_cutoff: String,
    outcome_not_before: String,
    registered_at: String,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let feature_cutoff = NaiveDate::parse_from_str(&feature_cutoff, "%Y-%m-%d")
        .with_context(|| format!("invalid feature cutoff {feature_cutoff}; expected YYYY-MM-DD"))?;
    let outcome_not_before = NaiveDate::parse_from_str(&outcome_not_before, "%Y-%m-%d")
        .with_context(|| {
            format!("invalid outcome eligibility date {outcome_not_before}; expected YYYY-MM-DD")
        })?;
    let source = source_season.to_string();
    let stats = get_stats(&source)
        .with_context(|| format!("bundled source season {source_season} has no stats.json"))?;
    let bios = get_bios(&source)
        .with_context(|| format!("bundled source season {source_season} has no bios.json"))?;
    let registration = build_organization_window_future_holdout_registration(
        source_season,
        target_season,
        feature_cutoff,
        outcome_not_before,
        &registered_at,
        &stats,
        &bios,
    )?;
    write_window_json(
        &registration,
        out.as_deref(),
        "future organization Window holdout registration",
    )
}

pub fn run_window_holdout_score(
    registration: PathBuf,
    standings: PathBuf,
    scored_at: String,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let registration: OrganizationWindowFutureHoldoutRegistration = read_icecast_json(
        &registration,
        "future organization Window holdout registration",
    )?;
    let standings: OrganizationWindowStandingsSnapshot =
        read_icecast_json(&standings, "organization Window standings outcome")?;
    let result = score_organization_window_future_holdout(&registration, &standings, &scored_at)?;
    write_window_json(
        &result,
        out.as_deref(),
        "future organization Window holdout result",
    )
}

pub fn run_window_completion_status(
    source_audit: PathBuf,
    holdout_registration: PathBuf,
    holdout_result: Option<PathBuf>,
    evaluated_at: String,
    require_complete: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    DateTime::parse_from_rfc3339(&evaluated_at)
        .context("--evaluated-at must be RFC 3339, for example 2026-07-30T12:00:00Z")?;
    let source_audit: OrganizationWindowSourceCoverageView =
        read_icecast_json(&source_audit, "organization Window source audit")?;
    let registration: OrganizationWindowFutureHoldoutRegistration = read_icecast_json(
        &holdout_registration,
        "future organization Window holdout registration",
    )?;
    let result: Option<OrganizationWindowFutureHoldoutResult> = holdout_result
        .as_ref()
        .map(|path| read_icecast_json(path, "future organization Window holdout result"))
        .transpose()?;
    let status = build_organization_window_completion_status(
        &source_audit,
        &registration,
        result.as_ref(),
        evaluated_at,
    )?;
    let complete = status.project_complete;
    write_window_json(
        &status,
        out.as_deref(),
        "organization Window completion status",
    )?;
    if require_complete && !complete {
        bail!(
            "organization Window is not complete: source gate passed={}, holdout state={:?}",
            status.source_gate.passed,
            status.holdout_gate.state
        );
    }
    Ok(())
}

fn write_window_json(
    value: &impl serde::Serialize,
    out: Option<&Path>,
    label: &str,
) -> anyhow::Result<()> {
    let output = format!("{}\n", serde_json::to_string_pretty(value)?);
    if let Some(path) = out {
        write_icecast_file(path, output.as_bytes(), label)?;
    } else {
        print!("{output}");
    }
    Ok(())
}

fn render_window(board: &OrganizationWindowBoardView, focus: Option<&str>) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "THE WINDOW — ORGANIZATION HEALTH");
    let _ = writeln!(
        out,
        "season {} · as of {} · frame {} · {} organizations",
        board.season,
        board.as_of,
        board.manifest.manifest_id,
        board.organizations.len()
    );
    let rows = board
        .organizations_in_display_order()
        .into_iter()
        .filter(|row| focus.is_none_or(|team| row.organization == team));
    for row in rows {
        let score = row
            .overall
            .score
            .map(|value| format!("{value:5.1}"))
            .unwrap_or_else(|| "   NR".to_owned());
        let rank = row
            .overall
            .rank
            .map(|value| format!("#{value}"))
            .unwrap_or_else(|| "NR".to_owned());
        let classification = row
            .overall
            .published_classification()
            .map(|classification| format!("{classification:?}"))
            .unwrap_or_else(|| "Under review".to_owned());
        let _ = writeln!(
            out,
            "{}  score {}  rank {:>3}  confidence {:>3.0}%  coverage {:>3.0}%  {}",
            row.organization,
            score,
            rank,
            row.overall.confidence * 100.0,
            row.overall.coverage * 100.0,
            classification
        );
        if focus.is_some() {
            for dimension in &row.dimensions {
                let dimension_score = dimension
                    .score
                    .map(|value| format!("{value:.1}"))
                    .unwrap_or_else(|| "NR".to_owned());
                let _ = writeln!(
                    out,
                    "  {:<24} {:>5}  conf {:>3.0}%  cov {:>3.0}%  {:?}",
                    dimension.label,
                    dimension_score,
                    dimension.confidence * 100.0,
                    dimension.coverage * 100.0,
                    dimension.status
                );
            }
            for reason in &row.overall.rank_status.reasons {
                let _ = writeln!(out, "  RANK GATE: {reason}");
            }
        }
    }
    out
}

fn render_history(view: &TeamSeasonForecastHistoryView, focus: &[String]) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "THE TAPE — ICECAST FORECAST HISTORY");
    let _ = writeln!(
        out,
        "season {} · {} checkpoints · {} trials · seed {}",
        view.season,
        view.checkpoints.len(),
        view.trials,
        view.seed
    );
    for team in focus {
        let Some(team) = view.teams.iter().find(|row| row.team == *team) else {
            continue;
        };
        let _ = writeln!(out, "\n{}", team.team);
        let _ = writeln!(
            out,
            "net {} → {}: {:+.2} points · {:+.1}pp playoffs · {:+.1}pp Cup · league rank {} of {} · {} · {} materiality · largest swing {:+.2} ({} → {})",
            team.checkpoints
                .first()
                .expect("history checkpoints")
                .as_of_date,
            team.checkpoints
                .last()
                .expect("history checkpoints")
                .as_of_date,
            team.average_points_delta_first_to_last,
            team.playoff_probability_delta_first_to_last * 100.0,
            team.stanley_cup_probability_delta_first_to_last * 100.0,
            team.projected_points_movement_rank,
            team.league_team_count,
            team.projected_points_trend.as_str(),
            team.net_points_movement_materiality.as_str(),
            team.largest_projected_points_swing,
            team.largest_swing_from_date,
            team.largest_swing_to_date
        );
        let _ = writeln!(
            out,
            "bridge: confirmed {:+} + remainder {:+.2} = net {:+.2} points",
            team.observed_standings_points_delta_first_to_last,
            team.expected_remaining_points_delta_first_to_last,
            team.average_points_delta_first_to_last
        );
        if let (Some(expected), Some(realized), Some(revaluation)) = (
            team.prior_expected_points_for_completed_interval,
            team.realized_points_vs_prior_remaining_pace,
            team.remaining_outlook_revaluation,
        ) {
            let _ = writeln!(
                out,
                "pace attribution: expected {:+.2} over {} games · realized vs pace {:+.2} + remaining revaluation {:+.2} = net {:+.2}",
                expected,
                team.completed_games_delta_first_to_last,
                realized,
                revaluation,
                team.average_points_delta_first_to_last
            );
        }
        let _ = writeln!(
            out,
            "{:<10} {:>4} {:>6} {:>8} {:>8} {:>9} {:>8} {:>9}",
            "Through", "GP", "Actual", "ProjPts", "ΔPoints", "Playoffs", "ΔPO", "ΔCup"
        );
        for point in &team.checkpoints {
            let _ = writeln!(
                out,
                "{:<10} {:>4} {:>6} {:>8.2} {:>8} {:>8.1}% {:>8} {:>9}",
                point.as_of_date,
                point.completed_games,
                point.observed_standings_points,
                point.average_points,
                optional_signed(point.average_points_delta_from_previous, 2, ""),
                point.playoff_probability * 100.0,
                optional_signed(point.playoff_probability_delta_from_previous, 1, "pp"),
                optional_signed(point.stanley_cup_probability_delta_from_previous, 1, "pp")
            );
            if let (Some(games), Some(expected), Some(realized), Some(revaluation)) = (
                point.completed_games_delta_from_previous,
                point.prior_expected_points_for_completed_interval_from_previous,
                point.realized_points_vs_prior_remaining_pace_from_previous,
                point.remaining_outlook_revaluation_from_previous,
            ) {
                let _ = writeln!(
                    out,
                    "           interval: expected {:+.2} over {} games · realized vs prior pace {:+.2} + remaining revaluation {:+.2} = change {:+.2}",
                    expected,
                    games,
                    realized,
                    revaluation,
                    point.average_points_delta_from_previous
                        .expect("validated history interval delta")
                );
            }
        }
    }
    let _ = writeln!(out, "\nBIGGEST RISERS — PROJECTED POINTS");
    for row in &view.biggest_risers {
        let _ = writeln!(
            out,
            "{:>2}. {:<4} {:+.2} points · {:+.1}pp playoffs · {:+.1}pp Cup",
            row.rank,
            row.team,
            row.average_points_delta_first_to_last,
            row.playoff_probability_delta_first_to_last * 100.0,
            row.stanley_cup_probability_delta_first_to_last * 100.0
        );
    }
    let _ = writeln!(out, "BIGGEST FALLERS — PROJECTED POINTS");
    for row in &view.biggest_fallers {
        let _ = writeln!(
            out,
            "{:>2}. {:<4} {:+.2} points · {:+.1}pp playoffs · {:+.1}pp Cup",
            row.rank,
            row.team,
            row.average_points_delta_first_to_last,
            row.playoff_probability_delta_first_to_last * 100.0,
            row.stanley_cup_probability_delta_first_to_last * 100.0
        );
    }
    for disclosure in &view.disclosures {
        let _ = writeln!(out, "- {disclosure}");
    }
    out
}

fn optional_signed(value: Option<f64>, precision: usize, suffix: &str) -> String {
    value
        .map(|value| {
            let value = if suffix == "pp" { value * 100.0 } else { value };
            format!("{value:+.*}{suffix}", precision)
        })
        .unwrap_or_else(|| "—".to_owned())
}

fn render_movement(view: &TeamSeasonForecastMovementView, focus: &[String]) -> String {
    let mut out = String::new();
    let earlier = view
        .earlier_as_of_date
        .map_or_else(|| "preseason".to_owned(), |date| date.to_string());
    let later = view
        .later_as_of_date
        .map_or_else(|| "full run".to_owned(), |date| date.to_string());
    let _ = writeln!(out, "THE SHIFT — ICECAST MOVEMENT");
    let _ = writeln!(
        out,
        "season {} · {} to {} · {} trials · seed {}",
        view.season, earlier, later, view.trials, view.seed
    );
    let _ = writeln!(
        out,
        "earlier {} · later {}",
        &view.earlier_fingerprint[..12],
        &view.later_fingerprint[..12]
    );
    let _ = writeln!(
        out,
        "{:<5} {:>8} {:>9} {:>8} {:>7} {:>8} {:>10}",
        "Team", "ΔPoints", "ΔPlayoffs", "ΔCup", "New GP", "Actual Δ", "Rest ΔPts"
    );
    for team in focus {
        if let Some(row) = view.teams.iter().find(|row| row.team == *team) {
            let _ = writeln!(
                out,
                "{:<5} {:>+8.2} {:>+8.2}pp {:>+7.2}pp {:>7} {:>8} {:>10}",
                row.team,
                row.average_points_delta,
                row.playoff_probability_delta * 100.0,
                row.stanley_cup_probability_delta * 100.0,
                optional_i64(row.completed_games_delta),
                optional_i64(row.observed_standings_points_delta),
                row.expected_remaining_points_delta
                    .map(|value| format!("{value:+.2}"))
                    .unwrap_or_else(|| "—".to_owned())
            );
        }
    }
    for disclosure in &view.disclosures {
        let _ = writeln!(out, "- {disclosure}");
    }
    out
}

fn optional_i64(value: Option<i64>) -> String {
    value
        .map(|value| format!("{value:+}"))
        .unwrap_or_else(|| "—".to_owned())
}

fn preseason_opening_players(players: &[TeamCeilingPlayerRow]) -> Vec<TeamGameOpeningPlayerRow> {
    let candidates = players
        .iter()
        .map(|player| {
            let scores = player
                .lens_scores
                .values()
                .flatten()
                .copied()
                .collect::<Vec<_>>();
            let modeled_value =
                confidence_weighted_player_value(&scores, player.games_played).unwrap_or(0.0);
            let position_group = if player.position.is_forward() {
                "forward"
            } else if player.position.is_defense() {
                "defense"
            } else {
                "goalie"
            };
            (player, modeled_value.clamp(0.0, 100.0), position_group)
        })
        .collect::<Vec<_>>();
    let mut selected = BTreeSet::new();
    for (position_group, slots) in [("forward", 12), ("defense", 6), ("goalie", 2)] {
        let mut ranked = candidates
            .iter()
            .filter(|(_, _, group)| *group == position_group)
            .collect::<Vec<_>>();
        ranked.sort_by(|left, right| {
            right
                .1
                .total_cmp(&left.1)
                .then_with(|| left.0.player_id.cmp(&right.0.player_id))
        });
        selected.extend(
            ranked
                .into_iter()
                .take(slots)
                .map(|(player, _, _)| player.player_id),
        );
    }
    candidates
        .into_iter()
        .map(
            |(player, modeled_value, position_group)| TeamGameOpeningPlayerRow {
                player_id: player.player_id,
                full_name: player.player.clone(),
                position_group: position_group.to_owned(),
                prior_value: player.has_nhl_sample.then_some(modeled_value),
                modeled_value,
                selected_at_opening: selected.contains(&player.player_id),
            },
        )
        .collect()
}

pub(crate) async fn build_season_view(
    args: &IceCastSeasonArgs,
) -> anyhow::Result<(TeamSeasonForecastView, Vec<String>, String)> {
    let rolling_replay = args.replay_mode == "rolling";
    if rolling_replay && args.candidate_overlay.is_some() {
        bail!("--candidate-overlay is preseason evidence and cannot be combined with --replay-mode rolling");
    }
    if args.ignore_replay_personnel_after.is_some() && !rolling_replay {
        bail!("--ignore-replay-personnel-after requires --replay-mode rolling");
    }
    if args.retrospective_opening_lineups && !rolling_replay {
        bail!("--retrospective-opening-lineups requires --replay-mode rolling");
    }
    if args.through.is_some() && !rolling_replay {
        bail!("--through requires --replay-mode rolling");
    }
    if rolling_replay && (args.auto_personnel || args.trade_mode != "off") {
        bail!("--replay-mode rolling cannot be combined with simulated personnel or trades");
    }
    let mut focus = if args.teams.is_empty() {
        vec!["NYR".to_owned(), "SEA".to_owned()]
    } else {
        args.teams
            .iter()
            .map(|team| team.trim().to_ascii_uppercase())
            .collect::<Vec<_>>()
    };
    let schedule = load_fantasy_schedule(Season(args.season), args.refresh).await?;
    let calendar_fingerprint = schedule_calendar_fingerprint(&schedule)?;
    let mut games = schedule
        .into_iter()
        .filter(|game| game.game_type == 2)
        .map(|game| {
            let final_result = game.is_final();
            Ok(TeamForecastGameInput {
                game_id: game.game_id,
                date: NaiveDate::parse_from_str(&game.date, "%Y-%m-%d")
                    .with_context(|| format!("invalid NHL schedule date '{}'", game.date))?,
                away_team: game.away_abbrev,
                home_team: game.home_abbrev,
                away_score: game.away_score,
                home_score: game.home_score,
                final_result,
                last_period: game.last_period,
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let schedule_start = games
        .iter()
        .map(|game| game.date)
        .min()
        .ok_or_else(|| anyhow::anyhow!("IceCast schedule has no regular-season games"))?;
    let schedule_end = games
        .iter()
        .map(|game| game.date)
        .max()
        .expect("non-empty regular-season schedule has an end date");
    if let Some(cutoff) = args.through {
        if cutoff < schedule_start || cutoff > schedule_end {
            bail!(
                "--through {cutoff} must be within the regular season ({schedule_start} through {schedule_end})"
            );
        }
        for game in &mut games {
            if game.date > cutoff {
                game.away_score = None;
                game.home_score = None;
                game.final_result = false;
                game.last_period = None;
            }
        }
    }
    let expected_teams = games
        .iter()
        .flat_map(|game| [&game.away_team, &game.home_team])
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    if args.teams.is_empty() {
        focus.retain(|team| expected_teams.contains(team));
    }
    let candidate_overlay = if let Some(path) = args.candidate_overlay.as_deref() {
        let overlay: LeagueCampCandidateOverlay = serde_json::from_slice(
            &std::fs::read(path)
                .with_context(|| format!("read IceCast candidate overlay {}", path.display()))?,
        )
        .with_context(|| format!("parse IceCast candidate overlay {}", path.display()))?;
        validate_league_camp_candidate_overlay(&overlay).map_err(anyhow::Error::msg)?;
        Some(overlay)
    } else {
        None
    };
    let (mut strengths, personnel, mut preseason_opening_strengths, strength_warning) =
        if rolling_replay {
            (Vec::new(), Vec::new(), Vec::new(), None)
        } else {
            match load_team_ceiling_view_with_candidates(
                Season(args.season),
                Season(args.stats_season),
                candidate_overlay.as_ref(),
            ) {
                Ok(ceiling) => {
                    let mut strengths = Vec::new();
                    let mut personnel = Vec::new();
                    let mut opening_strengths = Vec::new();
                    for team in ceiling.teams {
                        strengths.push(TeamForecastStrengthInput {
                            team: team.team.clone(),
                            strength: team.ensemble_score,
                        });
                        let opening_players = preseason_opening_players(&team.players);
                        let selected_count = |position_group: &str| {
                            opening_players
                                .iter()
                                .filter(|player| {
                                    player.selected_at_opening
                                        && player.position_group == position_group
                                })
                                .count()
                        };
                        opening_strengths.push(TeamGameOpeningStrengthRow {
                            team: team.team.clone(),
                            as_of_date: None,
                            strength: team.ensemble_score,
                            cohort_normalization_delta: 0.0,
                            roster_players: team.roster_players as usize,
                            valued_players: team.rated_players as usize,
                            value_coverage: team.coverage_pct / 100.0,
                            forwards_used: selected_count("forward"),
                            defensemen_used: selected_count("defense"),
                            goalies_used: selected_count("goalie"),
                            players: opening_players,
                        });
                        personnel.extend(team.players.into_iter().filter_map(|player| {
                            let scores = player
                                .lens_scores
                                .values()
                                .flatten()
                                .copied()
                                .collect::<Vec<_>>();
                            let rating =
                                confidence_weighted_player_value(&scores, player.games_played)
                                    .unwrap_or(f64::NAN);
                            (rating.is_finite() && rating > 0.0).then(|| TeamSeasonPersonnelInput {
                                team: team.team.clone(),
                                player: player.player,
                                position: player.position.abbreviation().to_owned(),
                                is_goalie: player.position == Position::Goalie,
                                age: player.age,
                                games_played: player.games_played,
                                rating: rating.clamp(0.0, 100.0),
                            })
                        }));
                    }
                    (strengths, personnel, opening_strengths, None)
                }
                Err(error) => (
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Some(format!(
                        "roster/depth strength unavailable ({error}); neutral strengths were used"
                    )),
                ),
            }
        };
    let official_shape = args.season == CURRENT_SEASON;
    let (replay_personnel, replay_personnel_warning) = if rolling_replay {
        match load_replay_personnel_evidence(args.season) {
            Ok(mut events) => {
                if let Some(cutoff) = args.through {
                    events.retain(|event| event.date <= cutoff);
                }
                if let Some(counterfactual_cutoff) = args.ignore_replay_personnel_after {
                    events.retain(|event| event.date <= counterfactual_cutoff);
                    (
                        events,
                        Some(format!(
                            "dated personnel evidence after {counterfactual_cutoff} was intentionally omitted for a paired evaluation counterfactual"
                        )),
                    )
                } else {
                    (events, None)
                }
            }
            Err(error) => (
                Vec::new(),
                Some(format!(
                    "dated personnel transaction evidence unavailable ({error}); replay continues with results-only strength"
                )),
            ),
        }
    } else {
        (Vec::new(), None)
    };
    let (opening_authority, opening_strengths) = if rolling_replay || official_shape {
        match Config::load() {
            Ok(config) => {
                let store = SnapshotStore::new(config.snapshot_dir());
                if args.retrospective_opening_lineups {
                    let (authority, strengths) = load_retrospective_opening_lineups(
                        args.season,
                        &games,
                        args.refresh,
                        &store,
                        &config.cache_dir,
                    )
                    .await?;
                    (Some(authority), strengths)
                } else {
                    let mut authority = audit_opening_roster_authority(
                        &store,
                        args.season,
                        schedule_start,
                        &expected_teams,
                    );
                    let opening_strengths = if rolling_replay
                        && matches!(
                            authority.status.as_str(),
                            "authoritative" | "partial_evaluation"
                        ) {
                        let strength_teams = authority
                            .verified_team_abbrevs
                            .iter()
                            .cloned()
                            .collect::<std::collections::BTreeSet<_>>();
                        match load_opening_team_strengths(
                            &store,
                            args.season,
                            &authority,
                            &strength_teams,
                        ) {
                            Ok(strengths) => {
                                authority.player_value_effects_enabled = !strengths.is_empty();
                                strengths
                            }
                            Err(error) => {
                                authority.reason = format!(
                                    "{}; player values disabled: {error}",
                                    authority.reason
                                );
                                Vec::new()
                            }
                        }
                    } else {
                        Vec::new()
                    };
                    if !rolling_replay {
                        if authority.status == "authoritative" && !strengths.is_empty() {
                            authority.player_value_effects_enabled = true;
                        } else {
                            strengths.clear();
                        }
                    }
                    (Some(authority), opening_strengths)
                }
            }
            Err(error) => (
                Some(TeamGameOpeningRosterAuthorityRow {
                    status: "unavailable".to_owned(),
                    required_before_date: schedule_start,
                    selected_snapshot: None,
                    selected_snapshot_created_at: None,
                    latest_observed_snapshot: None,
                    latest_observed_snapshot_created_at: None,
                    expected_teams: expected_teams.len(),
                    verified_teams: 0,
                    verified_team_abbrevs: Vec::new(),
                    player_value_effects_enabled: false,
                    personnel_events_effective_after: None,
                    reason: format!("snapshot configuration unavailable: {error}"),
                }),
                Vec::new(),
            ),
        }
    } else {
        (None, Vec::new())
    };
    let mut game_view = if rolling_replay {
        build_team_game_rolling_replay_with_opening_strengths(
            args.season,
            games,
            TeamForecastParameters::default(),
            official_shape.then_some(1_344),
            official_shape.then_some(84),
            TeamForecastReplayConfig::default(),
            replay_personnel,
            opening_strengths,
        )
    } else {
        build_team_game_forecast(
            args.season,
            games,
            strengths,
            TeamForecastParameters::default(),
            official_shape.then_some(1_344),
            official_shape.then_some(84),
        )
    }
    .map_err(anyhow::Error::msg)?;
    if !rolling_replay {
        game_view.disclosures.push(
            "Preseason depth, opening-lineup selection, and simulated personnel values bound each lens to 0–100 before shrinking prior-season rate statistics by GP/(GP+20); low-sample heaters retain upside without outranking established players at full weight."
                .to_owned(),
        );
        if let Some(overlay) = candidate_overlay.as_ref() {
            game_view.disclosures.push(format!(
                "Opening-roster shape was completed from a sourced organization-candidate overlay checked {}; only ranking-eligible relationships were admitted, duplicates were ignored, and absent NHL evidence received no invented production value.",
                overlay.checked_at
            ));
        }
    }
    if let Some(authority) = opening_authority {
        if authority.status == "retrospective_evaluation" && authority.player_value_effects_enabled
        {
            game_view.warnings.push(format!(
                "rolling replay uses official first-game dressed lineups for {}/{} teams as retrospective evaluation evidence; this run cannot satisfy pregame roster authority or model promotion",
                authority.verified_teams, authority.expected_teams
            ));
        } else if authority.status == "partial_evaluation" && authority.player_value_effects_enabled
        {
            game_view.warnings.push(format!(
                "rolling replay applies player-weighted opening strength only to {}/{} verified teams; all other teams remain neutral and this run cannot satisfy model promotion",
                authority.verified_teams, authority.expected_teams
            ));
        } else if authority.status != "authoritative" {
            let scope = if rolling_replay {
                "rolling replay uses a neutral opening prior"
            } else {
                "preseason forecast uses neutral team strength"
            };
            game_view
                .warnings
                .push(format!("{scope}; {}", authority.reason));
        } else if !authority.player_value_effects_enabled {
            game_view.warnings.push(
                "a dated opening roster passed the authority gate, but prior-season value coverage was insufficient for player-weighted opening strength"
                    .to_owned(),
            );
        }
        if !rolling_replay
            && authority.status == "authoritative"
            && authority.player_value_effects_enabled
        {
            let as_of_date = authority
                .selected_snapshot_created_at
                .as_deref()
                .and_then(|captured| DateTime::parse_from_rfc3339(captured).ok())
                .map(|captured| captured.date_naive())
                .or(authority.personnel_events_effective_after);
            for row in &mut preseason_opening_strengths {
                row.as_of_date = as_of_date;
            }
            game_view.opening_strengths = preseason_opening_strengths;
        }
        game_view.opening_roster_authority = Some(authority);
    }
    if let Some(warning) = strength_warning {
        game_view.warnings.push(warning);
    }
    if let Some(warning) = replay_personnel_warning {
        game_view.warnings.push(warning);
    }
    for team in &focus {
        let games = game_view
            .games
            .iter()
            .filter(|game| game.home_team == *team || game.away_team == *team)
            .count();
        if games == 0 {
            bail!(
                "team {team} has no games in the loaded {} schedule",
                args.season
            );
        }
    }
    if let Some(path) = args.game_forecast_out.as_deref() {
        let bytes = serde_json::to_vec_pretty(&game_view)?;
        write_icecast_file(path, &bytes, "team game forecast")?;
    }
    let mut scenario_reference = None;
    let mut scenario = if let Some(id) = args.scenario_id.as_deref() {
        let expected_scope = ScenarioScopeView {
            league_id: "nhl".to_string(),
            season: args.season,
            season_type: SeasonType::Regular,
            team_ids: Vec::new(),
            calendar_fingerprint: Some(calendar_fingerprint.clone()),
        };
        let resolved = ScenarioRegistryStore::new(ScenarioRegistryStore::default_root())
            .resolve_team_season_scenario(id, &expected_scope)
            .with_context(|| format!("resolve IceCast scenario id {id}"))?;
        scenario_reference = Some(resolved.reference);
        Some(resolved.scenario)
    } else {
        args.scenario
            .as_deref()
            .map(|path| load_scenario(path, args.season))
            .transpose()?
    };
    if args.auto_personnel {
        if personnel.is_empty() {
            bail!("--auto-personnel requires player records from the roster/depth snapshot");
        }
        let automatic = build_team_season_auto_personnel_scenario(
            game_view.schedule_start,
            game_view.schedule_end,
            args.seed,
            personnel.clone(),
            TeamSeasonAutoPersonnelConfig::default(),
            (args.season == CURRENT_SEASON).then(|| NaiveDate::from_ymd_opt(2027, 3, 5).unwrap()),
        )
        .map_err(anyhow::Error::msg)?;
        scenario = Some(merge_scenarios(scenario, automatic));
    }
    let counterfactual_scenario = scenario.clone();
    if args.trade_mode == "plausible" {
        if personnel.is_empty() {
            bail!("--trade-mode plausible requires player records from the roster/depth snapshot");
        }
        if args.season != CURRENT_SEASON {
            bail!("--trade-mode plausible currently requires the 2026-27 season deadline fixture");
        }
        let deadline = NaiveDate::from_ymd_opt(2027, 3, 5).unwrap();
        let outlooks = game_view
            .teams
            .iter()
            .map(|team| TeamSeasonTradeTeamInput {
                team: team.team.clone(),
                expected_points: team.expected_standings_points,
            })
            .collect();
        let trades = build_team_season_plausible_trade_scenario(
            deadline,
            outlooks,
            personnel,
            TeamSeasonPlausibleTradeConfig::default(),
        )
        .map_err(anyhow::Error::msg)?;
        scenario = Some(merge_scenarios(scenario, trades));
    }
    let completed_trade_scenario = (args.trade_mode == "plausible")
        .then(|| scenario.clone().map(force_trade_events))
        .flatten();
    let isolated_scenario = if args.isolated_impacts {
        Some(
            scenario
                .clone()
                .context("--isolated-impacts requires --scenario, --scenario-id, automatic personnel, or plausible trades")?,
        )
    } else {
        None
    };
    let simulation_config = TeamSeasonSimulationConfig {
        trials: args.trials,
        seed: args.seed,
    };
    let mut view = if let Some(cutoff) = args.through {
        simulate_team_season_forecast_as_of_with_scenario(
            &game_view,
            simulation_config,
            scenario,
            cutoff,
        )
    } else {
        simulate_team_season_forecast_with_scenario(&game_view, simulation_config, scenario)
    }
    .map_err(anyhow::Error::msg)?;
    view.scenario_reference = scenario_reference;
    if let Some(isolated_scenario) = isolated_scenario.as_ref() {
        view.isolated_impact = Some(
            match args.through {
                Some(cutoff) => build_isolated_scenario_impact_as_of(
                    &game_view,
                    isolated_scenario,
                    simulation_config,
                    cutoff,
                ),
                None => {
                    build_isolated_scenario_impact(&game_view, isolated_scenario, simulation_config)
                }
            }
            .context("build isolated IceCast scenario impacts")?,
        );
    }
    if args.scenario.is_some() {
        view.disclosures.push(
            "Scenario was loaded from an explicit CLI-only ephemeral path; import it with `icecast scenario import` before use by web, TUI, cards, or reproducible comparisons."
                .to_string(),
        );
    }
    if args.trade_mode == "plausible" {
        let baseline = simulate_for_replay_boundary(
            &game_view,
            simulation_config,
            counterfactual_scenario,
            args.through,
        )?;
        view.scenario_impacts =
            compare_team_season_forecast_scenarios(&baseline, &view).map_err(anyhow::Error::msg)?;
        let completed = simulate_for_replay_boundary(
            &game_view,
            simulation_config,
            completed_trade_scenario,
            args.through,
        )?;
        view.conditional_scenario_impacts =
            compare_team_season_forecast_scenarios(&baseline, &completed)
                .map_err(anyhow::Error::msg)?;
        view.disclosures.push(
            "Trade impacts include market-weighted and forced-completion scenario-minus-no-trade deltas from paired runs with identical schedules, seeds, trials, and non-trade events."
                .to_owned(),
        );
    } else if counterfactual_scenario.is_some() {
        let baseline =
            simulate_for_replay_boundary(&game_view, simulation_config, None, args.through)?;
        view.scenario_impacts =
            compare_team_season_forecast_scenarios(&baseline, &view).map_err(anyhow::Error::msg)?;
        view.disclosures.push(
            "Scenario impacts are scenario-minus-no-scenario deltas from paired runs with identical schedules, seeds, and trials."
                .to_owned(),
        );
    }
    Ok((view, focus, calendar_fingerprint))
}

fn simulate_for_replay_boundary(
    forecast: &icelines_core::TeamGameForecastView,
    config: TeamSeasonSimulationConfig,
    scenario: Option<TeamSeasonScenario>,
    through: Option<NaiveDate>,
) -> anyhow::Result<TeamSeasonForecastView> {
    match through {
        Some(cutoff) => {
            simulate_team_season_forecast_as_of_with_scenario(forecast, config, scenario, cutoff)
        }
        None => simulate_team_season_forecast_with_scenario(forecast, config, scenario),
    }
    .map_err(anyhow::Error::msg)
}

pub fn run_backtest(inputs: Vec<PathBuf>, json: bool, out: Option<PathBuf>) -> anyhow::Result<()> {
    let validation_inputs = inputs
        .iter()
        .map(|path| {
            let bytes = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
            let view: TeamSeasonForecastView = serde_json::from_slice(&bytes)
                .with_context(|| format!("parse IceCast forecast {}", path.display()))?;
            let authoritative_opening_roster =
                view.opening_roster_authority
                    .as_ref()
                    .is_some_and(|authority| {
                        authority.status == "authoritative"
                            && authority.player_value_effects_enabled
                    });
            let calibration_observations = view
                .games
                .iter()
                .filter_map(|game| {
                    game.actual_winner.as_ref().map(|winner| {
                        TeamGameForecastCalibrationObservation {
                            home_win_probability: game.home_overall_win_probability,
                            home_won: winner == &game.home_team,
                        }
                    })
                })
                .collect();
            let accuracy = view.accuracy.ok_or_else(|| {
                anyhow::anyhow!("IceCast forecast {} has no graded accuracy", path.display())
            })?;
            Ok(TeamGameForecastValidationInput {
                season: view.season,
                games: accuracy.final_games,
                authoritative_opening_roster,
                elo_blend_sweep: accuracy.elo_blend_sweep,
                calibration_observations,
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let view =
        build_team_game_forecast_validation(validation_inputs).map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        let mut text = String::new();
        let best = &view.pooled_best_by_brier;
        let _ = writeln!(text, "THE REVIEW — CROSS-SEASON VALIDATION");
        let _ = writeln!(
            text,
            "{} seasons · {} games · pooled best {:.0}% Elo · {:.3} Brier · {:.3} log loss",
            view.seasons.len(),
            view.total_games,
            best.elo_weight * 100.0,
            best.brier_score,
            best.binary_log_loss
        );
        let _ = writeln!(
            text,
            "promotion status: {} · authoritative opening rosters {}/{}",
            view.promotion_status,
            view.authoritative_opening_roster_seasons,
            view.seasons.len()
        );
        for check in &view.promotion_checks {
            let _ = writeln!(
                text,
                "  {} {:<30} {}",
                if check.passed { "PASS" } else { "FAIL" },
                check.key,
                check.detail
            );
        }
        let _ = writeln!(
            text,
            "Holdout   Train weight  Accuracy  Brier  vs Model  vs Elo"
        );
        for row in &view.holdouts {
            let _ = writeln!(
                text,
                "{}      {:>3.0}%       {:>5.1}%   {:.3}   {:+.4}   {:+.4}",
                row.holdout_season,
                row.selected_elo_weight * 100.0,
                row.pick_accuracy * 100.0,
                row.brier_score,
                row.brier_improvement_vs_model,
                row.brier_improvement_vs_pure_elo
            );
        }
        let _ = writeln!(text, "Chronological calibration holdouts");
        let _ = writeln!(
            text,
            "Holdout   Training  Intercept  Slope   Brier gain  Log-loss gain"
        );
        for row in &view.calibration_holdouts {
            let _ = writeln!(
                text,
                "{}   {:>5} games   {:+.3}    {:.3}    {:+.4}      {:+.4}",
                row.holdout_season,
                row.training_games,
                row.fitted_intercept,
                row.fitted_slope,
                row.brier_improvement,
                row.binary_log_loss_improvement
            );
        }
        let calibration = &view.calibration_summary;
        let _ = writeln!(
            text,
            "Pooled: {} games | Brier {:.4} -> {:.4} ({:+.4}, {}/{}) | log loss {:.4} -> {:.4} ({:+.4}, {}/{})",
            calibration.games,
            calibration.uncalibrated_brier_score,
            calibration.recalibrated_brier_score,
            calibration.brier_improvement,
            calibration.holdouts_improved_brier,
            calibration.holdout_seasons,
            calibration.uncalibrated_binary_log_loss,
            calibration.recalibrated_binary_log_loss,
            calibration.binary_log_loss_improvement,
            calibration.holdouts_improved_binary_log_loss,
            calibration.holdout_seasons
        );
        let _ = writeln!(
            text,
            "Paired 95% intervals: Brier [{:+.4}, {:+.4}] | log loss [{:+.4}, {:+.4}]",
            calibration.brier_improvement_ci95_lower,
            calibration.brier_improvement_ci95_upper,
            calibration.binary_log_loss_improvement_ci95_lower,
            calibration.binary_log_loss_improvement_ci95_upper
        );
        let _ = writeln!(
            text,
            "Season-clustered 95% intervals: Brier [{:+.4}, {:+.4}] | log loss [{:+.4}, {:+.4}]",
            calibration.season_clustered_brier_improvement_ci95_lower,
            calibration.season_clustered_brier_improvement_ci95_upper,
            calibration.season_clustered_binary_log_loss_improvement_ci95_lower,
            calibration.season_clustered_binary_log_loss_improvement_ci95_upper
        );
        let _ = writeln!(
            text,
            "Season-clustered evidence: Brier {} | log loss {}",
            calibration.season_clustered_brier_evidence,
            calibration.season_clustered_binary_log_loss_evidence
        );
        text
    };
    if let Some(path) = out {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        std::fs::write(&path, output.as_bytes())
            .with_context(|| format!("write {}", path.display()))?;
        println!("Wrote IceCast validation to {}", path.display());
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_development_calibration(
    start_season: u32,
    end_season: u32,
    breakout_threshold: f64,
    downturn_threshold: f64,
    prior_sample_size: f64,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    validate_season_id(start_season)?;
    validate_season_id(end_season)?;
    if start_season >= end_season {
        bail!("development calibration requires start-season before end-season");
    }
    let shortened = [20122013, 20192020, 20202021]
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    let mut transitions = Vec::new();
    let mut prior_season = start_season;
    while prior_season < end_season {
        let target_season = next_season_id(prior_season)?;
        if target_season > end_season {
            break;
        }
        if !shortened.contains(&prior_season) && !shortened.contains(&target_season) {
            transitions.extend(load_development_transitions(prior_season, target_season)?);
        }
        prior_season = target_season;
    }
    let mut view = build_development_calibration(
        transitions,
        DevelopmentCalibrationConfig {
            value_model: DevelopmentValueModel::PositionEraNormalizedMultilens,
            team_strength_scale: 0.5,
            breakout_strength_threshold: breakout_threshold,
            downturn_strength_threshold: downturn_threshold,
            prior_sample_size,
        },
    )
    .map_err(anyhow::Error::msg)?;
    view.disclosures.push(
        "The 2012-13 lockout season and the 2019-20/2020-21 pandemic seasons are excluded, along with transitions touching them, so schedule-length shocks do not masquerade as player development."
            .to_owned(),
    );
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_development_calibration(&view)
    };
    if let Some(path) = out {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        std::fs::write(&path, output.as_bytes())
            .with_context(|| format!("write {}", path.display()))?;
        println!(
            "Wrote IceCast development calibration to {}",
            path.display()
        );
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_scenario_development_calibration(
    input_path: PathBuf,
    calibration_path: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let input: TeamSeasonScenarioDevelopmentCalibrationInput =
        read_icecast_json(&input_path, "scenario development calibration input")?;
    let calibration: DevelopmentCalibrationView =
        read_icecast_json(&calibration_path, "development calibration")?;
    let view = calibrate_team_season_scenario_development(input, &calibration)
        .map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_scenario_development_calibration(&view)
    };
    if let Some(path) = out {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        std::fs::write(&path, output.as_bytes())
            .with_context(|| format!("write {}", path.display()))?;
        println!(
            "Wrote IceCast scenario development calibration to {}",
            path.display()
        );
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_prospect_arrival_calibration(
    input_path: Option<PathBuf>,
    career_discovery_path: Option<PathBuf>,
    player_id: Option<u32>,
    event_id: Option<String>,
    forecast_season: Option<u32>,
    conversion_board_path: Option<PathBuf>,
    conversion_archive_path: Option<PathBuf>,
    json: bool,
    out: Option<PathBuf>,
    input_out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let input: ProspectArrivalCalibrationInput = match (input_path, career_discovery_path) {
        (Some(path), None) => read_icecast_json(&path, "prospect arrival calibration input")?,
        (None, Some(path)) => {
            let player_id = player_id.context("--player-id is required with --career-discovery")?;
            let event_id = event_id.context("--event-id is required with --career-discovery")?;
            let forecast_season =
                forecast_season.context("--forecast-season is required with --career-discovery")?;
            let discovery: ProspectCareerDiscoveryView =
                read_icecast_json(&path, "prospect career discovery")?;
            if discovery.schema != PROSPECT_CAREER_DISCOVERY_SCHEMA {
                bail!(
                    "invalid prospect career discovery schema in {}",
                    path.display()
                );
            }
            let studies = discovery
                .studies
                .iter()
                .filter(|study| study.player_id == player_id)
                .collect::<Vec<_>>();
            if studies.len() != 1 {
                let goalie_matches = discovery
                    .goalie_studies
                    .iter()
                    .filter(|study| study.player_id == player_id)
                    .count();
                bail!(
                    "career discovery must contain exactly one skater study for player {player_id}; found {} skater and {goalie_matches} goalie studies",
                    studies.len()
                );
            }
            let input = adapt_prospect_arrival_calibration_input(
                event_id,
                studies[0],
                forecast_season,
                ProspectArrivalCalibrationConfig::default(),
            )
            .map_err(anyhow::Error::msg)?;
            if let Some(path) = input_out.as_deref() {
                let bytes = format!("{}\n", serde_json::to_string_pretty(&input)?);
                write_icecast_file(
                    path,
                    bytes.as_bytes(),
                    "IceCast prospect arrival calibration input",
                )?;
            }
            input
        }
        _ => bail!("provide exactly one prospect arrival input or career discovery"),
    };
    let board = match (conversion_board_path, conversion_archive_path) {
        (Some(path), None) => read_icecast_json(&path, "prospect conversion board")?,
        (None, Some(path)) => {
            let archive: ProspectConversionArchive =
                read_icecast_json(&path, "prospect conversion archive")?;
            validate_prospect_conversion_archive(&archive).map_err(anyhow::Error::msg)?;
            archive.board
        }
        _ => bail!("provide exactly one prospect conversion board or archive"),
    };
    let view = calibrate_prospect_arrival(input, &board).map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_prospect_arrival_calibration(&view)
    };
    if let Some(path) = out {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        std::fs::write(&path, output.as_bytes())
            .with_context(|| format!("write {}", path.display()))?;
        println!(
            "Wrote IceCast prospect arrival calibration to {}",
            path.display()
        );
    } else {
        print!("{output}");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn run_prospect_arrival_league(
    camp_forecast_path: Option<PathBuf>,
    career_discovery_paths: Vec<PathBuf>,
    career_history_path: Option<PathBuf>,
    source_package_path: Option<PathBuf>,
    require_complete_population: bool,
    conversion_board_path: Option<PathBuf>,
    conversion_archive_path: Option<PathBuf>,
    forecast_season: u32,
    as_of: String,
    max_age: u8,
    json: bool,
    out: Option<PathBuf>,
    discovery_out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let organizations = CANONICAL_TEAMS
        .iter()
        .map(|(abbreviation, _)| (*abbreviation).to_owned())
        .collect::<Vec<_>>();
    let expected = CANONICAL_TEAMS
        .iter()
        .map(|(abbreviation, _)| *abbreviation)
        .collect::<BTreeSet<_>>();
    let (mut studies, goalie_studies) = match (camp_forecast_path, career_discovery_paths.is_empty()) {
        (Some(path), true) => {
            let forecast: TrainingCampLeagueForecastView =
                read_icecast_json(&path, "league training-camp forecast")?;
            let actual = forecast
                .teams
                .iter()
                .map(|team| team.team.trim().to_ascii_uppercase())
                .collect::<BTreeSet<_>>();
            let actual = actual.iter().map(String::as_str).collect::<BTreeSet<_>>();
            if forecast.teams.len() != CANONICAL_TEAMS.len() || actual != expected {
                let missing = expected.difference(&actual).copied().collect::<Vec<_>>();
                let unexpected = actual.difference(&expected).copied().collect::<Vec<_>>();
                bail!(
                    "league prospect arrival requires the {} canonical organizations; found {} rows, missing [{}], unexpected [{}]",
                    CANONICAL_TEAMS.len(),
                    forecast.teams.len(),
                    missing.join(", "),
                    unexpected.join(", ")
                );
            }
            let as_of_date = NaiveDate::parse_from_str(&as_of, "%Y-%m-%d")
                .with_context(|| format!("invalid --as-of date {as_of}; expected YYYY-MM-DD"))?;
            let career_history_path =
                career_history_path.unwrap_or(Config::load()?.career_history_path());
            let store = CareerHistoryStore::load(&career_history_path).with_context(|| {
                format!(
                    "read career history store {}",
                    career_history_path.display()
                )
            })?;
            let composition = build_prospect_program_from_camp_and_career_store(
                forecast,
                &store,
                ProspectCareerProgramConfig {
                    context: ProspectCareerContextDraftConfig {
                        as_of_date,
                        max_age,
                    },
                    ..ProspectCareerProgramConfig::default()
                },
            )
            .map_err(anyhow::Error::msg)?;
            if let Some(path) = discovery_out.as_deref() {
                let bytes = format!(
                    "{}\n",
                    serde_json::to_string_pretty(&composition.discovery)?
                );
                write_icecast_file(
                    path,
                    bytes.as_bytes(),
                    "IceCast league prospect career discovery",
                )?;
            }
            (composition.discovery.studies, composition.discovery.goalie_studies)
        }
        (None, false) => {
            if career_history_path.is_some() || discovery_out.is_some() {
                bail!(
                    "--career-history and --discovery-out apply only to --camp-forecast derivation"
                );
            }
            let mut studies = Vec::new();
            let mut goalies = Vec::new();
            for path in career_discovery_paths {
                let discovery: ProspectCareerDiscoveryView =
                    read_icecast_json(&path, "prospect career discovery")?;
                if discovery.schema != PROSPECT_CAREER_DISCOVERY_SCHEMA {
                    bail!("invalid prospect career discovery schema in {}", path.display());
                }
                studies.extend(discovery.studies);
                goalies.extend(discovery.goalie_studies);
            }
            (studies, goalies)
        }
        _ => bail!(
            "provide exactly one prospect target route: --camp-forecast or one or more --career-discovery"
        ),
    };
    let mut source_exclusions = Vec::new();
    let mut population_authority = None;
    if let Some(path) = source_package_path.as_deref() {
        let package: icelines_core::source_facts::SourcePackage =
            read_icecast_json(path, "prospect source package")?;
        let supplied_skater_ids = studies
            .iter()
            .map(|study| study.player_id)
            .collect::<BTreeSet<_>>();
        let supplied_skaters = studies.len();
        let selection = select_controlled_prospect_studies(&package, studies, goalie_studies)
            .map_err(anyhow::Error::msg)?;
        (studies, source_exclusions, population_authority) =
            apply_prospect_arrival_population_selection(
                selection,
                &supplied_skater_ids,
                supplied_skaters,
                require_complete_population,
            )?;
    } else if require_complete_population {
        bail!("--require-complete-population requires --source-package");
    }
    let board = match (conversion_board_path, conversion_archive_path) {
        (Some(path), None) => read_icecast_json(&path, "prospect conversion board")?,
        (None, Some(path)) => {
            let archive: ProspectConversionArchive =
                read_icecast_json(&path, "prospect conversion archive")?;
            validate_prospect_conversion_archive(&archive).map_err(anyhow::Error::msg)?;
            archive.board
        }
        _ => bail!("provide exactly one prospect conversion board or archive"),
    };
    let view = calibrate_prospect_arrival_league(
        organizations,
        studies,
        source_exclusions,
        population_authority,
        forecast_season,
        &board,
        ProspectArrivalCalibrationConfig::default(),
    )
    .map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_prospect_arrival_league(&view)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(
            path,
            output.as_bytes(),
            "IceCast prospect arrival league calibration",
        )?;
    } else {
        print!("{output}");
    }
    Ok(())
}

fn render_prospect_arrival_league(
    view: &icelines_core::ProspectArrivalLeagueCalibrationView,
) -> String {
    let mut output = format!(
        "PROSPECT ARRIVAL LEAGUE · {} · {}/{} calibrated skaters · {} exclusions\n",
        view.forecast_season, view.calibrated_skaters, view.target_skaters, view.excluded_skaters
    );
    if let Some(authority) = &view.population_authority {
        let status = if authority.population_complete {
            "complete"
        } else {
            "incomplete"
        };
        let _ = writeln!(
            output,
            "Population authority · {status} · {}/{} controlled skaters · {} control exclusions · package {}",
            authority.controlled_studies,
            authority.supplied_studies,
            authority.control_exclusions,
            authority.source_package_fingerprint
        );
    } else {
        let _ = writeln!(
            output,
            "Population authority · not supplied · coverage draft only"
        );
    }
    for team in &view.teams {
        let _ = writeln!(
            output,
            "{} · {}/{} calibrated · {} exclusions",
            team.organization, team.calibrated_skaters, team.target_skaters, team.excluded_skaters
        );
    }
    output
}

pub fn run_prospect_arrival_card(
    input: PathBuf,
    team: String,
    team_name: Option<String>,
    evidence_at: Option<String>,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let arrival: icelines_core::ProspectArrivalLeagueCalibrationView =
        read_icecast_json(&input, "prospect arrival league calibration")?;
    let evidence_at = evidence_at
        .as_deref()
        .map(DateTime::parse_from_rfc3339)
        .transpose()
        .context("--evidence-at must be RFC 3339, for example 2026-09-15T12:00:00Z")?
        .map(|value| value.with_timezone(&Utc));
    let mut view = ViewContext::new(ViewWindow::new(
        Season(arrival.forecast_season),
        SeasonType::Regular,
    ));
    view.generated_at = evidence_at;
    let team = team.trim().to_ascii_uppercase();
    let card = build_prospect_arrival_card(ProspectArrivalCardInput {
        arrival,
        focus_team: team.clone(),
        team_name: team_name.unwrap_or_else(|| team.clone()),
        view,
        evidence_at,
    })?;
    let output = format!("{}\n", serde_json::to_string_pretty(&card)?);
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "IceCast prospect arrival card")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

fn apply_prospect_arrival_population_selection(
    selection: ControlledProspectStudySelection,
    supplied_skater_ids: &BTreeSet<u32>,
    supplied_skaters: usize,
    require_complete_population: bool,
) -> anyhow::Result<(
    Vec<ProspectDevelopmentStudyView>,
    Vec<ProspectArrivalLeagueSourceExclusionInput>,
    Option<ProspectArrivalLeaguePopulationAuthorityView>,
)> {
    if require_complete_population && !selection.population_complete {
        bail!(
            "prospect population source package is incomplete; league arrival publication was refused"
        );
    }
    let studies = selection.studies;
    let source_exclusions = selection
        .exclusions
        .into_iter()
        .filter(|exclusion| supplied_skater_ids.contains(&exclusion.player_id))
        .map(|exclusion| ProspectArrivalLeagueSourceExclusionInput {
            organization: exclusion.supplied_organization,
            player_id: exclusion.player_id,
            player: exclusion.player,
            reason: format!("prospect source control gate: {}", exclusion.detail),
        })
        .collect::<Vec<_>>();
    let authority = ProspectArrivalLeaguePopulationAuthorityView {
        source_package_fingerprint: selection.source_package_fingerprint,
        population_complete: selection.population_complete,
        supplied_studies: supplied_skaters,
        controlled_studies: studies.len(),
        control_exclusions: source_exclusions.len(),
    };
    Ok((studies, source_exclusions, Some(authority)))
}

pub fn run_prospect_study(
    input_path: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let input: ProspectDevelopmentStudyInput =
        read_icecast_json(&input_path, "prospect development study input")?;
    let view = build_prospect_development_study(input, ProspectDevelopmentStudyConfig::default())
        .map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_prospect_study(&view)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "IceCast prospect study")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn run_prospect_context(
    snapshot_paths: Vec<PathBuf>,
    league_crosswalk_paths: Vec<PathBuf>,
    affiliations_path: PathBuf,
    as_of: String,
    max_age: u8,
    minimum_ahl_seasons: usize,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let snapshots = snapshot_paths
        .iter()
        .map(|path| read_icecast_json(path, "AHL roster-stats snapshot"))
        .collect::<anyhow::Result<Vec<AhlRosterStatsSnapshot>>>()?;
    let league_crosswalks = league_crosswalk_paths
        .iter()
        .map(|path| read_icecast_json(path, "reviewed AHL league identity crosswalk"))
        .collect::<anyhow::Result<Vec<AhlIdentityLeagueCrosswalkView>>>()?;
    let affiliations: AhlAffiliationCatalogView =
        read_icecast_json(&affiliations_path, "dated AHL affiliation catalog")?;
    let as_of_date = NaiveDate::parse_from_str(&as_of, "%Y-%m-%d")
        .with_context(|| format!("invalid --as-of date {as_of}; expected YYYY-MM-DD"))?;
    let view = build_prospect_league_context_draft(
        snapshots,
        league_crosswalks,
        affiliations,
        ProspectLeagueContextDraftConfig {
            max_age,
            as_of_date,
            minimum_ahl_seasons,
        },
    )
    .map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_prospect_context(&view)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "IceCast prospect context draft")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_prospect_league(
    snapshot_paths: Vec<PathBuf>,
    crosswalk_paths: Vec<PathBuf>,
    context_path: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let snapshots = snapshot_paths
        .iter()
        .map(|path| read_icecast_json(path, "AHL roster-stats snapshot"))
        .collect::<anyhow::Result<Vec<AhlRosterStatsSnapshot>>>()?;
    let crosswalks = read_prospect_crosswalks(&crosswalk_paths)?;
    let context: ProspectLeagueContext =
        read_icecast_json(&context_path, "prospect league context")?;
    let view = build_prospect_league_discovery(
        snapshots,
        crosswalks,
        context,
        ProspectDevelopmentStudyConfig::default(),
    )
    .map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_prospect_league(&view)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "IceCast prospect league discovery")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_prospect_population_audit(
    input_path: PathBuf,
    require_fully_classified: bool,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let overlay: LeagueCampCandidateOverlay =
        read_icecast_json(&input_path, "prospect population overlay")?;
    let view = overlay.audit().map_err(anyhow::Error::msg)?;
    if require_fully_classified && !view.fully_classified {
        bail!(
            "prospect population authority is incomplete: {} legacy relationship(s), {} unknown relationship(s)",
            view.legacy_relationships,
            view.unknown_relationships
        );
    }
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_prospect_population_audit(&view)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "prospect population audit")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_identity_review_workboard(
    source_package_path: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let package: icelines_core::source_facts::SourcePackage =
        read_icecast_json(&source_package_path, "sealed source package")?;
    let view = icelines_fetch::build_identity_review_workboard_from_source_package(&package)
        .map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_identity_review_workboard(&view)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "identity review workboard")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

fn render_identity_review_workboard(view: &icelines_core::IdentityReviewWorkboardView) -> String {
    let families = view
        .family_counts
        .iter()
        .map(|row| format!("{}={}", row.family, row.proposals))
        .collect::<Vec<_>>()
        .join(", ");
    let mut output = format!(
        "IDENTITY REVIEW WORKBOARD · {} · {} unresolved\nFamilies · {}\n",
        view.evaluation_season, view.unresolved_count, families
    );
    for row in &view.rows {
        let context = row
            .contexts
            .first()
            .expect("validated workboard rows have context");
        let organization = context.organization.as_deref().unwrap_or("LEAGUE");
        let _ = writeln!(
            output,
            "{:>4}. {} · {} · {} · {} · {}",
            row.rank,
            organization,
            row.displayed_name,
            context.family,
            context.detail,
            row.proposal_id
        );
    }
    let _ = writeln!(
        output,
        "Package {} · known {}",
        view.source_package_fingerprint, view.knowledge_cutoff
    );
    output
}

#[allow(clippy::too_many_arguments)]
pub async fn run_official_identity_candidates(
    workboard_path: PathBuf,
    refresh: bool,
    offline: bool,
    search_concurrency: u8,
    evidence_cutoff: Option<String>,
    json: bool,
    out: Option<PathBuf>,
    cfg: &Config,
) -> anyhow::Result<()> {
    let workboard: icelines_core::IdentityReviewWorkboardView =
        read_icecast_json(&workboard_path, "identity review workboard")?;
    let icelines_home = cfg
        .snapshot_dir()
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| cfg.snapshot_dir());
    let _lock = fetch_lock::acquire(&icelines_home, std::time::Duration::from_secs(120))
        .context("acquiring official identity evidence fetch lock")?;
    let options = icelines_fetch::OfficialIdentityAcquisitionOptions {
        cache_root: icelines_home.join("data").join(".fletch"),
        refresh,
        offline,
        search_concurrency: usize::from(search_concurrency),
        landing_delay_ms: 50,
        evidence_cutoff: evidence_cutoff
            .map(|cutoff| {
                DateTime::parse_from_rfc3339(&cutoff)
                    .context("--evidence-cutoff must be RFC 3339")
                    .map(|cutoff| cutoff.with_timezone(&Utc))
            })
            .transpose()?,
    };
    let view = icelines_fetch::acquire_official_identity_candidates(&workboard, options).await?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_official_identity_candidates(&view)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "official identity candidates")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

fn render_official_identity_candidates(
    view: &icelines_core::OfficialIdentityCandidateBoardView,
) -> String {
    let statuses = view
        .status_counts
        .iter()
        .map(|row| format!("{:?}={}", row.status, row.proposals))
        .collect::<Vec<_>>()
        .join(", ");
    let mut output = format!(
        "OFFICIAL IDENTITY CANDIDATES · {} eligible / {} evaluated\nStatus · {}\n",
        view.eligible_count, view.evaluated_count, statuses
    );
    for row in &view.rows {
        let eligible = row
            .eligible_player_id
            .map(|player_id| player_id.to_string())
            .unwrap_or_else(|| "—".to_owned());
        let _ = writeln!(
            output,
            "{:>4}. {} · {:?} · player {} · {}",
            row.rank, row.displayed_name, row.status, eligible, row.proposal_id
        );
    }
    let _ = writeln!(
        output,
        "Package {} · known {}",
        view.source_package_fingerprint, view.knowledge_cutoff
    );
    let _ = writeln!(output, "Evidence horizon {}", view.evidence_cutoff);
    output
}

pub fn run_official_identity_review_ledger(
    candidates_path: PathBuf,
    provider: String,
    registry_url: String,
    reviewer: String,
    reviewed_at: String,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let candidates: icelines_core::OfficialIdentityCandidateBoardView =
        read_icecast_json(&candidates_path, "official identity candidate board")?;
    let reviewed_at = DateTime::parse_from_rfc3339(&reviewed_at)
        .context("--reviewed-at must be RFC 3339")?
        .with_timezone(&Utc);
    let ledger = icelines_fetch::build_official_identity_review_ledger(
        &candidates,
        provider,
        registry_url,
        reviewer,
        reviewed_at,
    )?;
    let output = format!("{}\n", serde_json::to_string_pretty(&ledger)?);
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "official identity review ledger")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn run_prospect_census(
    source_package_path: PathBuf,
    pipeline_path: Option<PathBuf>,
    league_discovery_paths: Vec<PathBuf>,
    career_discovery_paths: Vec<PathBuf>,
    program_board_path: Option<PathBuf>,
    pipeline_out: Option<PathBuf>,
    requested_ranking_depth: usize,
    require_publishable: bool,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let package: icelines_core::source_facts::SourcePackage =
        read_icecast_json(&source_package_path, "sealed prospect source package")?;
    let supplied_pipeline = pipeline_path
        .as_deref()
        .map(|path| {
            read_icecast_json::<icelines_fetch::ProspectCensusPipelineEvidence>(
                path,
                "prospect census pipeline evidence",
            )
        })
        .transpose()?;
    if let Some(pipeline) = &supplied_pipeline {
        pipeline.validate().map_err(anyhow::Error::msg)?;
    }
    let deriving = !league_discovery_paths.is_empty()
        || !career_discovery_paths.is_empty()
        || program_board_path.is_some();
    if supplied_pipeline.is_some() && deriving {
        bail!("--pipeline cannot be combined with discovery/program derivation inputs");
    }
    if deriving && program_board_path.is_none() {
        bail!("derived census pipeline evidence requires --program-board");
    }
    let derived_pipeline = if deriving {
        let league_discoveries = league_discovery_paths
            .iter()
            .map(|path| read_icecast_json(path, "prospect league discovery"))
            .collect::<anyhow::Result<Vec<ProspectLeagueDiscoveryView>>>()?;
        let career_discoveries = career_discovery_paths
            .iter()
            .map(|path| read_icecast_json(path, "prospect career discovery"))
            .collect::<anyhow::Result<Vec<ProspectCareerDiscoveryView>>>()?;
        let program: ProspectProgramBoardView = read_icecast_json(
            program_board_path
                .as_deref()
                .expect("derivation requires board"),
            "prospect program board",
        )?;
        if program.prospects_per_team != requested_ranking_depth {
            bail!(
                "program board depth {} does not match requested census depth {}",
                program.prospects_per_team,
                requested_ranking_depth
            );
        }
        Some(
            icelines_fetch::build_prospect_census_pipeline_from_discoveries(
                &league_discoveries,
                &career_discoveries,
                &program,
                icelines_core::ORGANIZATIONAL_PROSPECT_METHOD,
            )
            .map_err(anyhow::Error::msg)?,
        )
    } else {
        None
    };
    let pipeline = supplied_pipeline.as_ref().or(derived_pipeline.as_ref());
    let eligibility_policy_version = pipeline
        .map(|pipeline| pipeline.eligibility_policy_version.as_str())
        .unwrap_or(icelines_core::ORGANIZATIONAL_PROSPECT_METHOD);
    let players = pipeline
        .map(|pipeline| pipeline.players.as_slice())
        .unwrap_or_default();
    let view = icelines_fetch::build_prospect_census_from_source_package(
        &package,
        players,
        requested_ranking_depth,
        eligibility_policy_version,
    )
    .map_err(anyhow::Error::msg)?;
    if require_publishable {
        icelines_core::require_publishable_prospect_census(&view).map_err(anyhow::Error::msg)?;
    }
    if let Some(path) = pipeline_out.as_deref() {
        let pipeline = pipeline.ok_or_else(|| {
            anyhow::anyhow!("--pipeline-out requires --pipeline or discovery derivation inputs")
        })?;
        let bytes = format!("{}\n", serde_json::to_string_pretty(pipeline)?);
        write_icecast_file(path, bytes.as_bytes(), "prospect census pipeline evidence")?;
    }
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_prospect_census(&view)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "prospect census")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

fn render_prospect_census(view: &icelines_core::ProspectCensusView) -> String {
    let mut output = format!(
        "PROSPECT CENSUS · {} · {:?} · {} discovered · {} canonical · {} controlled · {} eligible · {} studies · {} ranked\n",
        view.evaluation_season,
        view.freshness_status,
        view.league_counts.discovered,
        view.league_counts.canonical_identity,
        view.league_counts.controlled_relationship,
        view.league_counts.prospect_eligible,
        view.league_counts.study_built,
        view.league_counts.ranked,
    );
    let league_losses = prospect_census_loss_summary(&view.losses);
    if !league_losses.is_empty() {
        let _ = writeln!(output, "League losses · {league_losses}");
    }
    for organization in &view.organizations {
        let loss_summary = prospect_census_loss_summary(&organization.losses);
        let _ = writeln!(
            output,
            "{} · {:?} · {:?} · {}/{} ranked · {} loss(es){}",
            organization.organization,
            organization.population_authority_status,
            organization.publication_status,
            organization.counts.ranked,
            organization.requested_ranking_depth,
            organization.losses.len(),
            if loss_summary.is_empty() {
                String::new()
            } else {
                format!(" · {loss_summary}")
            },
        );
    }
    let _ = writeln!(
        output,
        "Package {} · effective {} · known {}",
        view.source_package_fingerprint, view.effective_cutoff, view.knowledge_cutoff
    );
    output
}

fn prospect_census_loss_summary(losses: &[icelines_core::ProspectCensusLossRow]) -> String {
    let mut counts = BTreeMap::<String, usize>::new();
    for loss in losses {
        *counts.entry(format!("{:?}", loss.reason)).or_default() += 1;
    }
    counts
        .into_iter()
        .map(|(reason, count)| format!("{reason}={count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_prospect_population_audit(view: &icelines_fetch::ProspectPopulationAuditView) -> String {
    let authority = if view.fully_classified {
        "fully classified"
    } else {
        "classification incomplete"
    };
    let mut out = format!(
        "PROSPECT POPULATION AUDIT · {} · {authority}\n{} candidates · {} ranking eligible · {} camp only · {} legacy · {} unknown\n",
        view.checked_at,
        view.candidates,
        view.ranking_eligible,
        view.camp_only,
        view.legacy_relationships,
        view.unknown_relationships
    );
    for team in &view.teams {
        let relationships = team
            .relationship_counts
            .iter()
            .map(|(relationship, count)| format!("{relationship}={count}"))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            out,
            "{} · {} candidates · {} ranking · {} camp-only · {}",
            team.team, team.candidates, team.ranking_eligible, team.camp_only, relationships
        );
    }
    out
}

#[allow(clippy::too_many_arguments)]
pub fn run_prospect_career_context(
    camp_forecast_path: PathBuf,
    rosters_path: PathBuf,
    bios_path: PathBuf,
    candidate_overlay_path: Option<PathBuf>,
    career_history_path: Option<PathBuf>,
    as_of: String,
    max_age: u8,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let forecast: TrainingCampLeagueForecastView =
        read_icecast_json(&camp_forecast_path, "league training-camp forecast")?;
    let roster_map: BTreeMap<String, LeagueRosterIdentity> =
        serde_json::from_slice(&std::fs::read(&rosters_path).with_context(|| {
            format!("read league roster identities {}", rosters_path.display())
        })?)
        .with_context(|| format!("parse league roster identities {}", rosters_path.display()))?;
    let bios: Vec<SkaterBio> = serde_json::from_slice(
        &std::fs::read(&bios_path)
            .with_context(|| format!("read prospect identity bios {}", bios_path.display()))?,
    )
    .with_context(|| format!("parse prospect identity bios {}", bios_path.display()))?;

    let current_roster_ids = roster_map
        .values()
        .map(|identity| identity.player_id)
        .collect::<std::collections::BTreeSet<_>>();
    let mut identities = BTreeMap::<u32, ProspectCareerContextIdentityInput>::new();
    let mut overlay_ranking_exclusions = BTreeMap::new();
    for bio in bios {
        if let Some(birth_date) = bio.birth_date {
            identities.insert(
                bio.player_id,
                ProspectCareerContextIdentityInput {
                    player_id: bio.player_id,
                    birth_date,
                    nhl_games_played: 0,
                    evidence: vec![],
                },
            );
        }
    }
    for identity in roster_map.into_values() {
        if let Some(birth_date) = identity.birth_date {
            identities.insert(
                identity.player_id,
                ProspectCareerContextIdentityInput {
                    player_id: identity.player_id,
                    birth_date,
                    nhl_games_played: 0,
                    evidence: vec![],
                },
            );
        }
    }
    if let Some(path) = candidate_overlay_path.as_deref() {
        let overlay: LeagueCampCandidateOverlay =
            serde_json::from_slice(&std::fs::read(path).with_context(|| {
                format!("read league camp candidate overlay {}", path.display())
            })?)
            .with_context(|| format!("parse league camp candidate overlay {}", path.display()))?;
        validate_league_camp_candidate_overlay(&overlay).map_err(anyhow::Error::msg)?;
        for candidate in overlay.candidates {
            if !candidate.relationship.supports_prospect_ranking() {
                if !current_roster_ids.contains(&candidate.player_id) {
                    overlay_ranking_exclusions.insert(
                        candidate.player_id,
                        (
                            candidate.display_name,
                            candidate.relationship.label().to_owned(),
                        ),
                    );
                }
                continue;
            }
            if let Some(birth_date) = candidate.birth_date {
                identities.insert(
                    candidate.player_id,
                    ProspectCareerContextIdentityInput {
                        player_id: candidate.player_id,
                        birth_date,
                        nhl_games_played: 0,
                        evidence: vec![icelines_core::ProspectStudyEvidenceInput {
                            label: "Sourced training-camp candidate identity".to_owned(),
                            source_url: candidate.source_url,
                        }],
                    },
                );
            }
        }
    }
    if let Some(path) = career_history_path.as_deref() {
        let store = CareerHistoryStore::load(path)
            .with_context(|| format!("read career history store {}", path.display()))?;
        for player_id in store
            .birth_dates
            .keys()
            .filter_map(|key| key.parse::<u32>().ok())
        {
            if let Some(birth_date) = store.birth_date(player_id) {
                let mut evidence = identities
                    .remove(&player_id)
                    .map(|identity| identity.evidence)
                    .unwrap_or_default();
                evidence.push(icelines_core::ProspectStudyEvidenceInput {
                    label: "Official NHL player landing identity".to_owned(),
                    source_url: format!("https://api-web.nhle.com/v1/player/{player_id}/landing"),
                });
                let nhl_games_played = store
                    .get(player_id)
                    .map(|history| {
                        history
                            .stints
                            .iter()
                            .filter(|stint| {
                                stint.game_type
                                    == icelines_core::career_history::CareerGameType::Regular
                                    && stint.league.as_str().eq_ignore_ascii_case("NHL")
                            })
                            .fold(0_u32, |total, stint| total.saturating_add(stint.gp))
                    })
                    .unwrap_or_default();
                identities.insert(
                    player_id,
                    ProspectCareerContextIdentityInput {
                        player_id,
                        birth_date: birth_date.to_owned(),
                        nhl_games_played,
                        evidence,
                    },
                );
            }
        }
    }
    let as_of_date = NaiveDate::parse_from_str(&as_of, "%Y-%m-%d")
        .with_context(|| format!("invalid --as-of date {as_of}; expected YYYY-MM-DD"))?;
    let mut forecast = forecast;
    remove_overlay_ranking_exclusions(&mut forecast, &overlay_ranking_exclusions);
    let mut view = build_prospect_career_context_draft(
        forecast,
        identities.into_values().collect(),
        ProspectCareerContextDraftConfig {
            as_of_date,
            max_age,
        },
    )
    .map_err(anyhow::Error::msg)?;
    view.exclusions.extend(overlay_ranking_exclusions.into_iter().map(
        |(player_id, (player, relationship))| ProspectLeagueContextExclusionView {
            player_id,
            player,
            reason: ProspectLeagueContextExclusionReason::UnsupportedOrganizationRelationship,
            detail: format!(
                "Sourced overlay relationship `{relationship}` supports camp participation but does not establish organization control for prospect ranking"
            ),
        },
    ));
    view.exclusions.sort_by_key(|row| row.player_id);
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_prospect_context(&view)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(
            path,
            output.as_bytes(),
            "IceCast prospect career context draft",
        )?;
    } else {
        print!("{output}");
    }
    Ok(())
}

fn remove_overlay_ranking_exclusions(
    forecast: &mut TrainingCampLeagueForecastView,
    exclusions: &BTreeMap<u32, (String, String)>,
) -> usize {
    let mut removed = 0;
    for team in &mut forecast.teams {
        if let Some(team_forecast) = team.forecast.as_mut() {
            let before = team_forecast.players.len();
            team_forecast
                .players
                .retain(|player| !exclusions.contains_key(&player.player_id));
            removed += before - team_forecast.players.len();
        }
    }
    removed
}

pub fn run_prospect_career(
    context_path: PathBuf,
    career_history_path: PathBuf,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let context: ProspectLeagueContext =
        read_icecast_json(&context_path, "prospect career context")?;
    let store = CareerHistoryStore::load(&career_history_path).with_context(|| {
        format!(
            "read career history store {}",
            career_history_path.display()
        )
    })?;
    let view = build_prospect_career_discovery(
        context,
        &store,
        ProspectDevelopmentStudyConfig::default(),
        ProspectGoalieDevelopmentStudyConfig::default(),
    )
    .map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_prospect_career(&view)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "IceCast prospect career discovery")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

fn read_prospect_crosswalks(paths: &[PathBuf]) -> anyhow::Result<Vec<AhlIdentityCrosswalkView>> {
    let mut crosswalks = Vec::new();
    for path in paths {
        let bytes = std::fs::read(path)
            .with_context(|| format!("read reviewed AHL identity artifact {}", path.display()))?;
        let value: serde_json::Value = serde_json::from_slice(&bytes)
            .with_context(|| format!("parse reviewed AHL identity artifact {}", path.display()))?;
        match value.get("schema").and_then(serde_json::Value::as_str) {
            Some(AHL_IDENTITY_CROSSWALK_SCHEMA) => {
                crosswalks.push(serde_json::from_value(value).with_context(|| {
                    format!("decode AHL identity crosswalk {}", path.display())
                })?);
            }
            Some(AHL_IDENTITY_LEAGUE_CROSSWALK_SCHEMA) => {
                let league: AhlIdentityLeagueCrosswalkView = serde_json::from_value(value)
                    .with_context(|| {
                        format!("decode AHL league identity crosswalk {}", path.display())
                    })?;
                crosswalks.extend(league.crosswalks);
            }
            schema => bail!(
                "unsupported AHL identity schema {} in {}",
                schema.unwrap_or("<missing>"),
                path.display()
            ),
        }
    }
    Ok(crosswalks)
}

pub struct ProspectProgramPublicationOptions {
    pub require_complete_rankings: bool,
    pub require_complete_population: bool,
    pub json: bool,
    pub out: Option<PathBuf>,
}

pub fn run_prospect_program(
    league_discovery_paths: Vec<PathBuf>,
    career_discovery_paths: Vec<PathBuf>,
    study_paths: Vec<PathBuf>,
    prior_board_path: Option<PathBuf>,
    source_package_path: Option<PathBuf>,
    config: ProspectProgramBoardConfig,
    publication: ProspectProgramPublicationOptions,
) -> anyhow::Result<()> {
    let (mut studies, mut goalie_studies) =
        load_prospect_program_inputs(league_discovery_paths, career_discovery_paths, study_paths)?;
    let mut population_authority = None;
    if let Some(path) = source_package_path.as_deref() {
        let package: icelines_core::source_facts::SourcePackage =
            read_icecast_json(path, "prospect source package")?;
        let selection = select_controlled_prospect_studies(&package, studies, goalie_studies)
            .map_err(anyhow::Error::msg)?;
        if publication.require_complete_population && !selection.population_complete {
            bail!(
                "prospect population source package is incomplete; authority-backed publication was refused"
            );
        }
        let excluded_studies = selection.exclusions.len();
        population_authority = Some(ProspectProgramPopulationAuthorityView {
            source_package_fingerprint: selection.source_package_fingerprint,
            population_complete: selection.population_complete,
            supplied_studies: selection.supplied_studies,
            controlled_studies: selection.studies.len() + selection.goalie_studies.len(),
            excluded_studies,
        });
        studies = selection.studies;
        goalie_studies = selection.goalie_studies;
    } else if publication.require_complete_population {
        bail!("--require-complete-population requires --source-package");
    }
    let prior = prior_board_path
        .as_deref()
        .map(|path| read_icecast_json(path, "prior prospect program board"))
        .transpose()?;
    let mut view =
        build_prospect_program_board_with_goalies(studies, goalie_studies, prior.as_ref(), config)
            .map_err(anyhow::Error::msg)?;
    if let Some(authority) = population_authority {
        view.disclosures.push(format!(
            "Source-package control gating retained {} of {} supplied studies and excluded {}; AHL affiliation or roster presence alone never established NHL organization control.",
            authority.controlled_studies,
            authority.supplied_studies,
            authority.excluded_studies
        ));
        view.population_authority = Some(authority);
    }
    if publication.require_complete_rankings && view.partial_organization_rankings > 0 {
        let shortfalls = view
            .programs
            .iter()
            .filter(|program| !program.top_prospect_ranking_complete)
            .map(|program| {
                format!(
                    "{} ({}/{}, short {})",
                    program.organization,
                    program.top_prospects.len(),
                    view.prospects_per_team,
                    program.top_prospect_ranking_shortfall
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        bail!(
            "prospect ranking publication is incomplete for {} organization(s): {shortfalls}",
            view.partial_organization_rankings
        );
    }
    let output = if publication.json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_prospect_program(&view)
    };
    if let Some(path) = publication.out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "IceCast prospect program board")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_prospect_program_sensitivity(
    league_discovery_paths: Vec<PathBuf>,
    career_discovery_paths: Vec<PathBuf>,
    study_paths: Vec<PathBuf>,
    thresholds: Vec<u32>,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let (studies, goalie_studies) =
        load_prospect_program_inputs(league_discovery_paths, career_discovery_paths, study_paths)?;
    let view = build_prospect_program_sensitivity_with_goalies(
        studies,
        goalie_studies,
        thresholds,
        ProspectProgramBoardConfig::default(),
    )
    .map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_prospect_program_sensitivity(&view)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(
            path,
            output.as_bytes(),
            "IceCast prospect program sensitivity",
        )?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_prospect_program_history(
    board_paths: Vec<PathBuf>,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let boards = board_paths
        .iter()
        .map(|path| read_icecast_json(path, "prospect program history board"))
        .collect::<anyhow::Result<Vec<ProspectProgramBoardView>>>()?;
    let view = build_prospect_program_history(boards).map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_prospect_program_history(&view)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "IceCast prospect program history")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn run_prospect_conversion(
    league_discovery_paths: Vec<PathBuf>,
    career_discovery_paths: Vec<PathBuf>,
    study_paths: Vec<PathBuf>,
    career_history_path: PathBuf,
    baseline_season: u32,
    through_season: u32,
    performance_path: Option<PathBuf>,
    performance_out: Option<PathBuf>,
    input_out: Option<PathBuf>,
    archive_out: Option<PathBuf>,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let (studies, goalie_studies) =
        load_prospect_program_inputs(league_discovery_paths, career_discovery_paths, study_paths)?;
    let store = CareerHistoryStore::load(&career_history_path).with_context(|| {
        format!(
            "loading prospect conversion career cache {}",
            career_history_path.display()
        )
    })?;
    let histories = store.histories.into_values().collect::<Vec<_>>();
    let supplied_performance = performance_path
        .as_deref()
        .map(|path| {
            read_icecast_json::<ProspectConversionPerformanceDocument>(
                path,
                "prospect conversion performance",
            )
        })
        .transpose()?;
    if supplied_performance
        .as_ref()
        .is_some_and(|document| document.schema != PROSPECT_CONVERSION_PERFORMANCE_SCHEMA)
    {
        bail!("invalid prospect conversion performance schema");
    }
    let performance = match supplied_performance {
        Some(document) => document,
        None => build_prospect_nhl_performance_document(
            &studies,
            &goalie_studies,
            &histories,
            baseline_season,
            through_season,
        )
        .map_err(anyhow::Error::msg)?,
    };
    if performance.baseline_season != baseline_season
        || performance.through_season != through_season
        || performance.players != performance.scores.len()
    {
        bail!("prospect conversion performance horizon or player count mismatch");
    }
    if let Some(path) = performance_out.as_deref() {
        let body = format!("{}\n", serde_json::to_string_pretty(&performance)?);
        write_icecast_file(path, body.as_bytes(), "IceCast NHL performance authority")?;
    }
    let input = adapt_prospect_conversion_input(
        &studies,
        &goalie_studies,
        &histories,
        baseline_season,
        through_season,
        &performance.scores,
        ProspectConversionConfig::default(),
    )
    .map_err(anyhow::Error::msg)?;
    if let Some(path) = input_out.as_deref() {
        let body = format!("{}\n", serde_json::to_string_pretty(&input)?);
        write_icecast_file(path, body.as_bytes(), "IceCast prospect conversion input")?;
    }
    let view = build_prospect_conversion_board(&input).map_err(anyhow::Error::msg)?;
    if let Some(path) = archive_out.as_deref() {
        let archive = build_prospect_conversion_archive(input.clone(), performance.clone())
            .map_err(anyhow::Error::msg)?;
        let body = format!("{}\n", serde_json::to_string_pretty(&archive)?);
        write_icecast_file(path, body.as_bytes(), "IceCast prospect conversion archive")?;
    }
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_prospect_conversion(&view)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "IceCast prospect conversion board")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn run_prospect_conversion_replay(
    input_path: Option<PathBuf>,
    archive_path: Option<PathBuf>,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let view = match (input_path, archive_path) {
        (Some(path), None) => {
            let input = read_icecast_json(&path, "prospect conversion input")?;
            build_prospect_conversion_board(&input).map_err(anyhow::Error::msg)?
        }
        (None, Some(path)) => {
            let archive: ProspectConversionArchive =
                read_icecast_json(&path, "prospect conversion archive")?;
            validate_prospect_conversion_archive(&archive).map_err(anyhow::Error::msg)?;
            archive.board
        }
        _ => bail!("provide exactly one prospect conversion input or archive"),
    };
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_prospect_conversion(&view)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(
            path,
            output.as_bytes(),
            "IceCast prospect conversion replay",
        )?;
    } else {
        print!("{output}");
    }
    Ok(())
}

fn load_prospect_program_inputs(
    league_discovery_paths: Vec<PathBuf>,
    career_discovery_paths: Vec<PathBuf>,
    study_paths: Vec<PathBuf>,
) -> anyhow::Result<(
    Vec<ProspectDevelopmentStudyView>,
    Vec<ProspectGoalieDevelopmentStudyView>,
)> {
    if league_discovery_paths.is_empty()
        && career_discovery_paths.is_empty()
        && study_paths.is_empty()
    {
        bail!("prospect program requires at least one discovery or study artifact");
    }
    let mut studies = Vec::new();
    let mut goalie_studies = Vec::new();
    let mut supplied_player_ids = BTreeSet::new();
    for path in league_discovery_paths {
        let discovery: ProspectLeagueDiscoveryView =
            read_icecast_json(&path, "prospect league discovery")?;
        if discovery.schema != PROSPECT_LEAGUE_DISCOVERY_SCHEMA {
            bail!(
                "invalid prospect league discovery schema in {}",
                path.display()
            );
        }
        for study in discovery.studies {
            supplied_player_ids.insert(study.player_id);
            studies.push(study);
        }
        for study in discovery.goalie_studies {
            supplied_player_ids.insert(study.player_id);
            goalie_studies.push(study);
        }
    }
    for path in career_discovery_paths {
        let discovery: ProspectCareerDiscoveryView =
            read_icecast_json(&path, "prospect career discovery")?;
        if discovery.schema != PROSPECT_CAREER_DISCOVERY_SCHEMA {
            bail!(
                "invalid prospect career discovery schema in {}",
                path.display()
            );
        }
        // Reviewed AHL snapshots remain authoritative for development when
        // both adapters cover the same player. Official career history still
        // enriches the retained study's NHL workload so graduation policy is
        // applied to facts rather than an AHL adapter's neutral zero.
        for mut career_study in discovery.studies {
            career_study.nhl_games_authority = ProspectNhlGamesAuthority::Observed;
            if supplied_player_ids.insert(career_study.player_id) {
                studies.push(career_study);
            } else if let Some(study) = studies
                .iter_mut()
                .find(|study| study.player_id == career_study.player_id)
            {
                study.nhl_games_played = study.nhl_games_played.max(career_study.nhl_games_played);
                study.nhl_games_authority = ProspectNhlGamesAuthority::Observed;
            } else if let Some(goalie) = goalie_studies
                .iter_mut()
                .find(|study| study.player_id == career_study.player_id)
            {
                goalie.nhl_games_played =
                    goalie.nhl_games_played.max(career_study.nhl_games_played);
                goalie.nhl_games_authority = ProspectNhlGamesAuthority::Observed;
            }
        }
        for mut career_study in discovery.goalie_studies {
            career_study.nhl_games_authority = ProspectNhlGamesAuthority::Observed;
            if supplied_player_ids.insert(career_study.player_id) {
                goalie_studies.push(career_study);
            } else if let Some(goalie) = goalie_studies
                .iter_mut()
                .find(|study| study.player_id == career_study.player_id)
            {
                goalie.nhl_games_played =
                    goalie.nhl_games_played.max(career_study.nhl_games_played);
                goalie.nhl_games_authority = ProspectNhlGamesAuthority::Observed;
            } else if let Some(study) = studies
                .iter_mut()
                .find(|study| study.player_id == career_study.player_id)
            {
                study.nhl_games_played = study.nhl_games_played.max(career_study.nhl_games_played);
                study.nhl_games_authority = ProspectNhlGamesAuthority::Observed;
            }
        }
        for exclusion in discovery.excluded {
            let Some(nhl_games_played) = exclusion.nhl_games_played else {
                continue;
            };
            if let Some(study) = studies
                .iter_mut()
                .find(|study| study.player_id == exclusion.player_id)
            {
                study.nhl_games_played = study.nhl_games_played.max(nhl_games_played);
                study.nhl_games_authority = ProspectNhlGamesAuthority::Observed;
            } else if let Some(goalie) = goalie_studies
                .iter_mut()
                .find(|study| study.player_id == exclusion.player_id)
            {
                goalie.nhl_games_played = goalie.nhl_games_played.max(nhl_games_played);
                goalie.nhl_games_authority = ProspectNhlGamesAuthority::Observed;
            }
        }
    }
    for path in study_paths {
        studies.push(read_icecast_json(&path, "prospect development study")?);
    }
    Ok((studies, goalie_studies))
}

pub fn run_prospect_board(
    study_paths: Vec<PathBuf>,
    json: bool,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let studies = study_paths
        .iter()
        .map(|path| read_icecast_json(path, "prospect development study"))
        .collect::<anyhow::Result<Vec<ProspectDevelopmentStudyView>>>()?;
    let view = build_prospect_discovery_board(studies).map_err(anyhow::Error::msg)?;
    let output = if json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_prospect_board(&view)
    };
    if let Some(path) = out.as_deref() {
        write_icecast_file(path, output.as_bytes(), "IceCast prospect discovery board")?;
    } else {
        print!("{output}");
    }
    Ok(())
}

fn load_development_transitions(
    prior_season: u32,
    target_season: u32,
) -> anyhow::Result<Vec<DevelopmentTransitionInput>> {
    let prior_key = prior_season.to_string();
    let target_key = target_season.to_string();
    let prior_bios = get_bios(&prior_key)
        .or_else(|| get_bios_installed(&prior_key))
        .with_context(|| format!("missing bios for calibration season {prior_season}"))?;
    let target_bios = get_bios(&target_key)
        .or_else(|| get_bios_installed(&target_key))
        .with_context(|| format!("missing bios for calibration season {target_season}"))?;
    let prior_stats = get_stats(&prior_key)
        .or_else(|| get_stats_installed(&prior_key))
        .with_context(|| format!("missing skater stats for calibration season {prior_season}"))?;
    let target_stats = get_stats(&target_key)
        .or_else(|| get_stats_installed(&target_key))
        .with_context(|| format!("missing skater stats for calibration season {target_season}"))?;
    let prior_goalies = get_goalie_stats(&prior_key)
        .or_else(|| get_goalie_stats_installed(&prior_key))
        .with_context(|| format!("missing goalie stats for calibration season {prior_season}"))?;
    let target_goalies = get_goalie_stats(&target_key)
        .or_else(|| get_goalie_stats_installed(&target_key))
        .with_context(|| format!("missing goalie stats for calibration season {target_season}"))?;
    let prior_bios = prior_bios
        .into_iter()
        .map(|row| (row.player_id, row))
        .collect::<BTreeMap<_, _>>();
    let target_bios = target_bios
        .into_iter()
        .map(|row| (row.player_id, row))
        .collect::<BTreeMap<_, _>>();
    let prior_skater_values = skater_development_values(&prior_stats, &prior_bios);
    let target_skater_values = skater_development_values(&target_stats, &target_bios);
    let prior_goalie_values = goalie_development_values(&prior_goalies);
    let target_goalie_values = goalie_development_values(&target_goalies);
    let prior_stats = prior_stats
        .into_iter()
        .map(|row| (row.player_id, row))
        .collect::<BTreeMap<_, _>>();
    let prior_goalies = prior_goalies
        .into_iter()
        .map(|row| (row.player_id, row))
        .collect::<BTreeMap<_, _>>();
    let mut transitions = Vec::new();
    for target in target_stats
        .into_iter()
        .filter(|row| row.games_played >= 20)
    {
        let Some(bio) = target_bios
            .get(&target.player_id)
            .or_else(|| prior_bios.get(&target.player_id))
        else {
            continue;
        };
        let prior = prior_stats.get(&target.player_id);
        transitions.push(DevelopmentTransitionInput {
            player_id: target.player_id,
            player: bio.skater_full_name.clone(),
            prior_season,
            target_season,
            position: if bio.position_code == "D" {
                DevelopmentPositionGroup::Defense
            } else {
                DevelopmentPositionGroup::Forward
            },
            age: bio
                .birth_date
                .as_deref()
                .and_then(|date| age_at_season_start(date, target_season)),
            prior_games_played: prior.map_or(0, |row| row.games_played),
            target_games_played: target.games_played,
            prior_value: prior_skater_values
                .get(&target.player_id)
                .copied()
                .unwrap_or(50.0),
            target_value: target_skater_values
                .get(&target.player_id)
                .copied()
                .unwrap_or(50.0),
        });
    }
    for target in target_goalies
        .into_iter()
        .filter(|row| row.games_played >= 15)
    {
        let prior = prior_goalies.get(&target.player_id);
        transitions.push(DevelopmentTransitionInput {
            player_id: target.player_id,
            player: target.goalie_full_name.clone(),
            prior_season,
            target_season,
            position: DevelopmentPositionGroup::Goalie,
            age: None,
            prior_games_played: prior.map_or(0, |row| row.games_played),
            target_games_played: target.games_played,
            prior_value: prior_goalie_values
                .get(&target.player_id)
                .copied()
                .unwrap_or(50.0),
            target_value: target_goalie_values
                .get(&target.player_id)
                .copied()
                .unwrap_or(50.0),
        });
    }
    Ok(transitions)
}

#[derive(Debug, Clone, Copy)]
struct FeatureMoments {
    mean: f64,
    standard_deviation: f64,
}

impl FeatureMoments {
    fn from_values(values: impl Iterator<Item = f64>) -> Self {
        let values = values.filter(|value| value.is_finite()).collect::<Vec<_>>();
        if values.is_empty() {
            return Self {
                mean: 0.0,
                standard_deviation: 1.0,
            };
        }
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>()
            / values.len() as f64;
        Self {
            mean,
            standard_deviation: variance.sqrt().max(1e-9),
        }
    }

    fn z(self, value: Option<f64>) -> f64 {
        value
            .filter(|value| value.is_finite())
            .map_or(0.0, |value| {
                ((value - self.mean) / self.standard_deviation).clamp(-3.0, 3.0)
            })
    }
}

#[derive(Debug, Clone, Copy)]
struct SkaterDevelopmentFeatures {
    points_per_game: f64,
    time_on_ice_per_game: Option<f64>,
    shots_per_game: f64,
    power_play_points_per_game: f64,
    plus_minus_per_game: f64,
}

fn skater_features(row: &icelines_fetch::schema::SkaterStats) -> SkaterDevelopmentFeatures {
    let games = f64::from(row.games_played.max(1));
    SkaterDevelopmentFeatures {
        points_per_game: f64::from(row.points_per_game),
        time_on_ice_per_game: row.time_on_ice_per_game.map(f64::from),
        shots_per_game: f64::from(row.shots) / games,
        power_play_points_per_game: f64::from(row.pp_points) / games,
        plus_minus_per_game: f64::from(row.plus_minus) / games,
    }
}

fn skater_development_values(
    rows: &[icelines_fetch::schema::SkaterStats],
    bios: &BTreeMap<u32, icelines_fetch::schema::SkaterBio>,
) -> BTreeMap<u32, f64> {
    [
        DevelopmentPositionGroup::Forward,
        DevelopmentPositionGroup::Defense,
    ]
    .into_iter()
    .flat_map(|position| {
        let position_rows = rows
            .iter()
            .filter(|row| {
                bios.get(&row.player_id).is_some_and(|bio| {
                    (bio.position_code == "D") == (position == DevelopmentPositionGroup::Defense)
                })
            })
            .collect::<Vec<_>>();
        let reference = position_rows
            .iter()
            .copied()
            .filter(|row| row.games_played >= 20)
            .map(skater_features)
            .collect::<Vec<_>>();
        let ppg = FeatureMoments::from_values(reference.iter().map(|row| row.points_per_game));
        let toi = FeatureMoments::from_values(
            reference.iter().filter_map(|row| row.time_on_ice_per_game),
        );
        let shots = FeatureMoments::from_values(reference.iter().map(|row| row.shots_per_game));
        let power_play =
            FeatureMoments::from_values(reference.iter().map(|row| row.power_play_points_per_game));
        let plus_minus =
            FeatureMoments::from_values(reference.iter().map(|row| row.plus_minus_per_game));
        position_rows.into_iter().map(move |row| {
            let features = skater_features(row);
            let weights = if position == DevelopmentPositionGroup::Defense {
                [4.0, 4.0, 2.0, 1.5, 1.5]
            } else {
                [5.0, 3.0, 2.5, 1.5, 1.0]
            };
            let raw = 50.0
                + weights[0] * ppg.z(Some(features.points_per_game))
                + weights[1] * toi.z(features.time_on_ice_per_game)
                + weights[2] * shots.z(Some(features.shots_per_game))
                + weights[3] * power_play.z(Some(features.power_play_points_per_game))
                + weights[4] * plus_minus.z(Some(features.plus_minus_per_game));
            let games = f64::from(row.games_played);
            let credibility = games / (games + 20.0);
            (
                row.player_id,
                (50.0 * (1.0 - credibility) + raw * credibility).clamp(20.0, 90.0),
            )
        })
    })
    .collect()
}

fn goalie_development_values(rows: &[icelines_fetch::schema::GoalieStats]) -> BTreeMap<u32, f64> {
    let reference = rows
        .iter()
        .filter(|row| row.games_played >= 15)
        .collect::<Vec<_>>();
    let save_pct = FeatureMoments::from_values(
        reference
            .iter()
            .filter_map(|row| row.save_pct.map(f64::from)),
    );
    let gaa = FeatureMoments::from_values(
        reference
            .iter()
            .filter_map(|row| row.goals_against_average.map(f64::from)),
    );
    let starts =
        FeatureMoments::from_values(reference.iter().map(|row| f64::from(row.games_started)));
    let shutout_rate = FeatureMoments::from_values(
        reference
            .iter()
            .map(|row| f64::from(row.shutouts) / f64::from(row.games_played.max(1))),
    );
    rows.iter()
        .map(|row| {
            let raw = 50.0 + 7.0 * save_pct.z(row.save_pct.map(f64::from))
                - 3.0 * gaa.z(row.goals_against_average.map(f64::from))
                + 2.0 * starts.z(Some(f64::from(row.games_started)))
                + shutout_rate.z(Some(
                    f64::from(row.shutouts) / f64::from(row.games_played.max(1)),
                ));
            let games = f64::from(row.games_played);
            let credibility = games / (games + 15.0);
            (
                row.player_id,
                (50.0 * (1.0 - credibility) + raw * credibility).clamp(20.0, 90.0),
            )
        })
        .collect()
}

fn age_at_season_start(birth_date: &str, season: u32) -> Option<u8> {
    let birth = NaiveDate::parse_from_str(birth_date, "%Y-%m-%d").ok()?;
    let start_year = i32::try_from(season / 10_000).ok()?;
    let reference = NaiveDate::from_ymd_opt(start_year, 10, 1)?;
    let mut age = reference.year() - birth.year();
    if (reference.month(), reference.day()) < (birth.month(), birth.day()) {
        age -= 1;
    }
    u8::try_from(age).ok()
}

fn validate_season_id(season: u32) -> anyhow::Result<()> {
    let start = season / 10_000;
    let end = season % 10_000;
    if end != start + 1 {
        bail!("invalid NHL season id {season}; expected consecutive years");
    }
    Ok(())
}

fn next_season_id(season: u32) -> anyhow::Result<u32> {
    validate_season_id(season)?;
    let start = season / 10_000 + 1;
    Ok(start * 10_000 + start + 1)
}

fn render_development_calibration(view: &DevelopmentCalibrationView) -> String {
    let mut out = String::new();
    let first = view.seasons.first().copied().unwrap_or_default();
    let last = view.seasons.last().copied().unwrap_or_default();
    let _ = writeln!(out, "THE LAB — DEVELOPMENT CALIBRATION");
    let _ = writeln!(
        out,
        "{} transitions · seasons {}–{} · breakout ≥ {:+.1} · downturn ≤ {:+.1}",
        view.transitions,
        first,
        last,
        view.config.breakout_strength_threshold,
        view.config.downturn_strength_threshold
    );
    let _ = writeln!(
        out,
        "Global: {:.1}% breakout · {:.1}% downturn · median {:+.2} / {:+.2}",
        view.global.breakout_rate * 100.0,
        view.global.downturn_rate * 100.0,
        view.global.median_breakout_strength_delta.unwrap_or(0.0),
        view.global.median_downturn_strength_delta.unwrap_or(0.0)
    );
    let _ = writeln!(
        out,
        "\n{:<8} {:<14} {:<12} {:<14} {:>5} {:>9} {:>9} {:>8} {:>8}",
        "Pos",
        "Age",
        "Experience",
        "Prior value",
        "N",
        "Breakout",
        "Downturn",
        "+Median",
        "-Median"
    );
    for row in view.cohorts.iter().filter(|row| row.sample_size >= 20) {
        let _ = writeln!(
            out,
            "{:<8} {:<14} {:<12} {:<14} {:>5} {:>8.1}% {:>8.1}% {:>+8.2} {:>+8.2}",
            format!("{:?}", row.position),
            row.age_band,
            row.experience_band,
            row.prior_value_band,
            row.sample_size,
            row.calibrated_breakout_rate * 100.0,
            row.calibrated_downturn_rate * 100.0,
            row.median_breakout_strength_delta.unwrap_or(0.0),
            row.median_downturn_strength_delta.unwrap_or(0.0)
        );
    }
    for disclosure in &view.disclosures {
        let _ = writeln!(out, "\n- {disclosure}");
    }
    out
}

fn render_scenario_development_calibration(
    view: &TeamSeasonScenarioDevelopmentCalibrationView,
) -> String {
    let mut text = String::new();
    let _ = writeln!(text, "Scenario development calibration");
    let _ = writeln!(text, "{}", view.scenario.name);
    let _ = writeln!(
        text,
        "Historical transitions: {} | calibrated: {} | uncalibrated: {}",
        view.source_transitions, view.calibrated_events, view.uncalibrated_events
    );
    for row in &view.probability_authority {
        let _ = writeln!(
            text,
            "{}: {:.1}% -> {:.1}% [{:?}]",
            row.player.as_deref().unwrap_or(&row.event_id),
            row.configured_probability * 100.0,
            row.applied_probability * 100.0,
            row.status
        );
    }
    text
}

fn render_prospect_arrival_calibration(view: &ProspectArrivalCalibrationView) -> String {
    let applied_arrival = view
        .horizon_adjusted_arrival_probability
        .unwrap_or(view.calibrated_arrival_probability);
    let applied_established = view
        .horizon_adjusted_established_probability
        .or(view.calibrated_established_probability);
    let horizon = match (
        view.source_horizon_seasons,
        view.forecast_horizon_seasons,
    ) {
        (Some(source), Some(forecast)) => format!(
            " | cumulative source: {:.1}% arrival over {source} seasons | applied horizon: {forecast} season(s)",
            view.calibrated_arrival_probability * 100.0
        ),
        _ => String::new(),
    };
    format!(
        "Prospect arrival calibration\n{} ({})\nArrival: {:.1}% | established role: {}{} | neighbors: {} | same-position candidates: {} | mean signal distance: {:.2}\n",
        view.player,
        view.position_group,
        applied_arrival * 100.0,
        applied_established
            .map(|probability| format!("{:.1}%", probability * 100.0))
            .unwrap_or_else(|| "unavailable".to_owned()),
        horizon,
        view.neighbor_players,
        view.candidate_players,
        view.mean_signal_distance
    )
}

fn render_prospect_study(view: &ProspectDevelopmentStudyView) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "THE INSIDER — PROSPECT STUDY");
    let _ = writeln!(
        out,
        "{} · {} · {} · age {}",
        view.player, view.organization, view.position, view.age
    );
    let _ = writeln!(
        out,
        "CLASS: {:?} · MARKET {:?} · SCORE {:.1}/100 · TRAJECTORY {:?} · NHL GP {}",
        view.classification,
        view.market_position,
        view.hidden_value_score,
        view.trajectory,
        view.nhl_games_played
    );
    let _ = writeln!(out, "\nDEVELOPMENT");
    for season in &view.seasons {
        let change = season
            .same_league_ppg_change
            .map(|value| format!(" · {:+.1}% vs prior {}", value * 100.0, season.league))
            .unwrap_or_default();
        let _ = writeln!(
            out,
            "{} {} · {} GP · {}-{}-{} · {:.3} P/GP{}",
            season.season,
            season.league,
            season.games_played,
            season.goals,
            season.assists,
            season.points,
            season.points_per_game,
            change
        );
    }
    let _ = writeln!(
        out,
        "\nCONTEXT\nOpportunity: {:?} · Availability: {:?}\nAttention: {:.0}% · performance-attention gap {:+.2}",
        view.opportunity,
        view.availability,
        view.attention_score * 100.0,
        view.performance_attention_gap
    );
    let _ = writeln!(out, "Basis: {}", view.attention_basis);
    let _ = writeln!(out, "\nSCORE LENSES");
    for component in &view.components {
        let _ = writeln!(
            out,
            "{:<14} {:>5.1}/100 × {:>4.0}% = {:>5.1}",
            component.id,
            component.score * 100.0,
            component.weight * 100.0,
            component.weighted_points
        );
    }
    if !view.lenses.is_empty() {
        let _ = writeln!(out, "\nDISCOVERY LENSES");
        for lens in &view.lenses {
            let _ = writeln!(
                out,
                "- {:?} / {:?} ({:.0}%): {}",
                lens.kind,
                lens.direction,
                lens.strength * 100.0,
                lens.summary
            );
        }
    }
    if !view.evidence.is_empty() {
        let _ = writeln!(out, "\nEVIDENCE");
        for item in &view.evidence {
            let _ = writeln!(out, "- {}\n  {}", item.label, item.source_url);
        }
    }
    let _ = writeln!(out, "\nDISCLOSURES");
    for disclosure in &view.disclosures {
        let _ = writeln!(out, "- {disclosure}");
    }
    out
}

fn render_prospect_context(view: &ProspectLeagueContext) -> String {
    let mut out = String::new();
    let mut by_organization = BTreeMap::<&str, usize>::new();
    for player in &view.players {
        *by_organization
            .entry(player.organization.as_str())
            .or_default() += 1;
    }
    let _ = writeln!(out, "THE SYSTEM — PROSPECT CONTEXT");
    let _ = writeln!(
        out,
        "Authority: {:?} · as of {} · {} players · {} organizations · {} exclusions",
        view.authority,
        view.as_of_date.as_deref().unwrap_or("unspecified"),
        view.players.len(),
        by_organization.len(),
        view.exclusions.len()
    );
    let _ = writeln!(out, "\nORGANIZATION COVERAGE");
    for (organization, players) in by_organization {
        let _ = writeln!(out, "{organization:<4} {players:>3} observed players");
    }
    if !view.exclusions.is_empty() {
        let _ = writeln!(out, "\nEXCLUSIONS");
        for row in &view.exclusions {
            let _ = writeln!(
                out,
                "- {} ({}) · {:?}: {}",
                row.player, row.player_id, row.reason, row.detail
            );
        }
    }
    let _ = writeln!(out, "\nDISCLOSURES");
    for disclosure in &view.disclosures {
        let _ = writeln!(out, "- {disclosure}");
    }
    out
}

fn render_prospect_league(view: &ProspectLeagueDiscoveryView) -> String {
    let mut out = String::new();
    let seasons = view
        .snapshot_seasons
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let _ = writeln!(out, "THE INSIDER — LEAGUE PROSPECT DISCOVERY");
    let _ = writeln!(
        out,
        "AHL seasons {seasons} · {} context players · {} skater studies · {} goalie studies · {} exclusions",
        view.context_players,
        view.studies.len(),
        view.goalie_studies.len(),
        view.excluded.len()
    );
    out.push_str(&render_prospect_board(&view.board));
    if !view.excluded.is_empty() {
        let _ = writeln!(out, "\nEXCLUSIONS");
        for row in &view.excluded {
            let _ = writeln!(out, "- {} · {:?}: {}", row.player, row.reason, row.detail);
        }
    }
    let _ = writeln!(out, "\nADAPTER DISCLOSURES");
    for disclosure in &view.disclosures {
        let _ = writeln!(out, "- {disclosure}");
    }
    out
}

fn render_prospect_career(view: &ProspectCareerDiscoveryView) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "THE INSIDER — MULTI-LEAGUE PROSPECT DISCOVERY");
    let _ = writeln!(
        out,
        "{} context players · {} skater studies · {} goalie studies · {} exclusions",
        view.context_players,
        view.studies.len(),
        view.goalie_studies.len(),
        view.excluded.len()
    );
    out.push_str(&render_prospect_board(&view.board));
    if !view.excluded.is_empty() {
        let _ = writeln!(out, "\nEXCLUSIONS");
        for row in &view.excluded {
            let _ = writeln!(out, "- {} · {:?}: {}", row.player, row.reason, row.detail);
        }
    }
    let _ = writeln!(out, "\nADAPTER DISCLOSURES");
    for disclosure in &view.disclosures {
        let _ = writeln!(out, "- {disclosure}");
    }
    out
}

fn render_prospect_program(view: &ProspectProgramBoardView) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "THE SYSTEM — PROSPECT PROGRAMS");
    let comparison = view
        .prior_as_of_season
        .map(|season| format!(" · deltas vs season {season}"))
        .unwrap_or_default();
    let _ = writeln!(
        out,
        "{} scope ({}) · season {}{} · {} organizations · {} ranked / {} supplied studies · {} graduates above {} NHL GP · {} unknown NHL GP",
        view.scope,
        view.source_leagues.join(", "),
        view.as_of_season,
        comparison,
        view.organizations,
        view.ranked_studies,
        view.studies,
        view.graduated_studies,
        view.maximum_nhl_games_played,
        view.unknown_nhl_games_studies
    );
    if let Some(authority) = &view.population_authority {
        let status = if authority.population_complete {
            "complete"
        } else {
            "incomplete"
        };
        let _ = writeln!(
            out,
            "Population authority · {status} · {}/{} controlled studies · {} exclusions · package {}",
            authority.controlled_studies,
            authority.supplied_studies,
            authority.excluded_studies,
            authority.source_package_fingerprint
        );
    } else {
        let _ = writeln!(out, "Population authority · not supplied");
    }
    if let Some(methodology) = &view.methodology {
        let _ = writeln!(
            out,
            "method {} · expected depth {} · weights pool {:.2} / development {:.2} / readiness {:.2} / confidence {:.2}",
            methodology.scoring_method,
            methodology.expected_depth,
            methodology.pool_weight,
            methodology.development_weight,
            methodology.readiness_weight,
            methodology.confidence_weight
        );
    }
    for (heading, rank_of, score_of, rank_delta_of, score_delta_of) in [
        (
            "THE PIPELINE — COMBINED",
            (|row: &icelines_core::ProspectProgramOrganizationView| row.pipeline_rank)
                as fn(&icelines_core::ProspectProgramOrganizationView) -> usize,
            (|row: &icelines_core::ProspectProgramOrganizationView| row.pipeline_score)
                as fn(&icelines_core::ProspectProgramOrganizationView) -> f64,
            (|row: &icelines_core::ProspectProgramOrganizationView| row.pipeline_rank_delta)
                as fn(&icelines_core::ProspectProgramOrganizationView) -> Option<i32>,
            (|row: &icelines_core::ProspectProgramOrganizationView| row.pipeline_score_delta)
                as fn(&icelines_core::ProspectProgramOrganizationView) -> Option<f64>,
        ),
        (
            "THE DEPTH CHART — POOL",
            |row| row.pool_rank,
            |row| row.pool_score,
            |row| row.pool_rank_delta,
            |row| row.pool_score_delta,
        ),
        (
            "THE FACTORY — DEVELOPMENT",
            |row| row.development_rank,
            |row| row.development_score,
            |row| row.development_rank_delta,
            |row| row.development_score_delta,
        ),
    ] {
        let _ = writeln!(out, "\n{heading}");
        let mut rows = view.programs.iter().collect::<Vec<_>>();
        rows.sort_by_key(|row| rank_of(row));
        for row in rows {
            let rank_delta = rank_delta_of(row)
                .map(|value| format!("{value:+}"))
                .unwrap_or_else(|| "new".to_owned());
            let score_delta = score_delta_of(row)
                .map(|value| format!("{value:+.2}"))
                .unwrap_or_else(|| "new".to_owned());
            let leaders = row
                .top_prospects
                .iter()
                .take(3)
                .map(|prospect| prospect.player.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            let _ = writeln!(
                out,
                "{}. {} · {:.2} · rank Δ {} · score Δ {} · {} ranked / {} supplied · {} graduates · {} NHL-GP unknown · confidence {:.1}\n   {}",
                rank_of(row),
                row.organization,
                score_of(row),
                rank_delta,
                score_delta,
                row.prospect_count,
                row.supplied_study_count,
                row.graduated_count,
                row.unknown_nhl_games_count,
                row.components.confidence,
                leaders
            );
        }
    }
    let _ = writeln!(
        out,
        "\nTOP PROSPECTS BY ORGANIZATION · {} complete · {} partial",
        view.complete_organization_rankings, view.partial_organization_rankings
    );
    let mut programs = view.programs.iter().collect::<Vec<_>>();
    programs.sort_by_key(|row| row.pipeline_rank);
    for program in programs {
        let published = program.top_prospects.len();
        let coverage = if program.top_prospect_ranking_complete {
            "complete"
        } else {
            "partial"
        };
        let _ = writeln!(
            out,
            "\n{} · pipeline #{} · {} of {} requested ({coverage}, short {})",
            program.organization,
            program.pipeline_rank,
            published,
            view.prospects_per_team,
            program.top_prospect_ranking_shortfall
        );
        for prospect in &program.top_prospects {
            let _ = writeln!(
                out,
                "{}. {} ({}) · signal {:.2} · {:?} · {:?} · confidence {:.2} · {} NHL GP",
                prospect.rank,
                prospect.player,
                prospect.position,
                prospect.observed_signal_score,
                prospect.trajectory,
                prospect.opportunity,
                prospect.workload_confidence,
                prospect.nhl_games_played
            );
        }
    }
    let graduated = view
        .programs
        .iter()
        .flat_map(|program| {
            program.graduates.iter().map(move |player| {
                (
                    program.organization.as_str(),
                    player.player.as_str(),
                    player.position.as_str(),
                    player.nhl_games_played,
                )
            })
        })
        .collect::<Vec<_>>();
    if !graduated.is_empty() {
        let _ = writeln!(out, "\nGRADUATED YOUNG NHL PLAYERS");
        for (organization, player, position, nhl_games) in graduated {
            let _ = writeln!(
                out,
                "- {organization} · {player} ({position}) · {nhl_games} NHL GP"
            );
        }
    }
    let _ = writeln!(out, "\nDISCLOSURES");
    for disclosure in &view.disclosures {
        let _ = writeln!(out, "- {disclosure}");
    }
    out
}

fn render_prospect_program_sensitivity(view: &ProspectProgramSensitivityView) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "THE SYSTEM — PROSPECT DEFINITION SENSITIVITY");
    let _ = writeln!(
        out,
        "{} supplied studies · {} organizations · NHL GP thresholds {}",
        view.supplied_studies,
        view.organizations,
        view.thresholds
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    );
    if let Some(methodology) = &view.methodology {
        let _ = writeln!(
            out,
            "method {} · expected depth {} · weights {:.2}/{:.2}/{:.2}/{:.2}",
            methodology.scoring_method,
            methodology.expected_depth,
            methodology.pool_weight,
            methodology.development_weight,
            methodology.readiness_weight,
            methodology.confidence_weight
        );
    }
    for program in &view.programs {
        let points = program
            .points
            .iter()
            .map(|point| {
                format!(
                    "{} GP: #{} / {:.2} ({} ranked, {} graduated, {} NHL-GP unknown)",
                    point.maximum_nhl_games_played,
                    point.pipeline_rank,
                    point.pipeline_score,
                    point.ranked_studies,
                    point.graduated_studies,
                    point.unknown_nhl_games_studies
                )
            })
            .collect::<Vec<_>>()
            .join(" · ");
        let _ = writeln!(
            out,
            "{} · rank {}–{} (span {}) · score {:.2}–{:.2} (span {:.2})\n   {}",
            program.organization,
            program.best_pipeline_rank,
            program.worst_pipeline_rank,
            program.pipeline_rank_span,
            program.minimum_pipeline_score,
            program.maximum_pipeline_score,
            program.pipeline_score_span,
            points
        );
    }
    let _ = writeln!(out, "\nDISCLOSURES");
    for disclosure in &view.disclosures {
        let _ = writeln!(out, "- {disclosure}");
    }
    out
}

fn render_prospect_program_history(view: &ProspectProgramHistoryView) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "THE SYSTEM — PROSPECT PROGRAM HISTORY");
    let _ = writeln!(
        out,
        "{} boards · seasons {} · {} organizations · {} NHL-GP boundary",
        view.boards,
        view.seasons
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(", "),
        view.organizations,
        view.maximum_nhl_games_played
    );
    let _ = writeln!(
        out,
        "method {} · expected depth {} · weights {:.2}/{:.2}/{:.2}/{:.2}",
        view.methodology.scoring_method,
        view.methodology.expected_depth,
        view.methodology.pool_weight,
        view.methodology.development_weight,
        view.methodology.readiness_weight,
        view.methodology.confidence_weight
    );
    for program in &view.programs {
        let points = program
            .points
            .iter()
            .map(|point| {
                let rank_delta = point
                    .pipeline_rank_delta
                    .map(|delta| format!("{delta:+}"))
                    .unwrap_or_else(|| "—".to_owned());
                let score_delta = point
                    .pipeline_score_delta
                    .map(|delta| format!("{delta:+.2}"))
                    .unwrap_or_else(|| "—".to_owned());
                format!(
                    "{}: #{} / {:.2} (Δ rank {}, score {})",
                    point.as_of_season,
                    point.pipeline_rank,
                    point.pipeline_score,
                    rank_delta,
                    score_delta
                )
            })
            .collect::<Vec<_>>()
            .join(" · ");
        let _ = writeln!(
            out,
            "{} · {} season(s) · first→latest Δ rank {:+}, score {:+.2}\n   {}",
            program.organization,
            program.seasons_observed,
            program.pipeline_rank_change,
            program.pipeline_score_change,
            points
        );
    }
    let _ = writeln!(out, "\nDISCLOSURES");
    for disclosure in &view.disclosures {
        let _ = writeln!(out, "- {disclosure}");
    }
    out
}

fn render_prospect_conversion(view: &ProspectConversionBoardView) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "THE FACTORY — PROSPECT CONVERSION");
    let _ = writeln!(
        out,
        "{} players · {} organizations · {} ranked · baselines {} → outcomes {}",
        view.players,
        view.organizations,
        view.ranked_organizations,
        view.baseline_seasons
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(", "),
        view.through_season
    );
    let _ = writeln!(
        out,
        "baseline {} · method {}",
        view.baseline_basis, view.methodology.method
    );
    let _ = writeln!(out, "\nSIGNAL CALIBRATION");
    for signal in &view.signal_calibration {
        if !signal.informative {
            let _ = writeln!(
                out,
                "{:?} · non-informative in this cohort (n={})",
                signal.signal, signal.sample_size
            );
            continue;
        }
        let correlation = |value: Option<f64>| {
            value
                .map(|value| format!("{value:+.3}"))
                .unwrap_or_else(|| "n/a".to_owned())
        };
        let quartiles = signal
            .bottom_quartile
            .as_ref()
            .zip(signal.top_quartile.as_ref())
            .map(|(bottom, top)| {
                format!(
                    "arrival {:.0}%→{:.0}% · established {:.0}%→{:.0}% · role {:.1}→{:.1}",
                    bottom.arrival_rate * 100.0,
                    top.arrival_rate * 100.0,
                    bottom.established_rate * 100.0,
                    top.established_rate * 100.0,
                    bottom.mean_role_score,
                    top.mean_role_score
                )
            })
            .unwrap_or_else(|| "quartiles unavailable".to_owned());
        let _ = writeln!(
            out,
            "{:?} · r(arrival) {} · r(established) {} · r(role) {} · {}",
            signal.signal,
            correlation(signal.arrival_correlation),
            correlation(signal.established_correlation),
            correlation(signal.role_correlation),
            quartiles
        );
    }
    let _ = writeln!(out, "\nORGANIZATIONS");
    for program in &view.programs {
        let rank = program
            .conversion_rank
            .map(|rank| format!("#{rank}"))
            .unwrap_or_else(|| "NR".to_owned());
        let leaders = program
            .player_results
            .iter()
            .take(3)
            .map(|player| {
                format!(
                    "{} {:.1} realized / {:+.1} delta / {:?}",
                    player.player,
                    player.realized_value_score,
                    player.conversion_delta,
                    player.result_class
                )
            })
            .collect::<Vec<_>>()
            .join(" · ");
        let _ = writeln!(
            out,
            "{} {} · efficiency {:.1} · realized {:.1} vs baseline {:.1} ({:+.1}) · {} established / {} players · {} hits / {} breakouts / {} misses · coverage {:.0}%",
            rank,
            program.organization,
            program.efficiency_index,
            program.realized_value_score,
            program.baseline_signal_score,
            program.conversion_delta,
            program.established_players,
            program.players,
            program.expected_hits,
            program.breakouts,
            program.misses,
            program.outcome_coverage * 100.0
        );
        if !leaders.is_empty() {
            let _ = writeln!(out, "   {leaders}");
        }
        for blocker in &program.rank_blockers {
            let _ = writeln!(out, "   no-rank: {blocker:?}");
        }
    }
    let _ = writeln!(out, "\nDISCLOSURES");
    for disclosure in &view.disclosures {
        let _ = writeln!(out, "- {disclosure}");
    }
    out
}

fn render_prospect_board(view: &ProspectDiscoveryBoardView) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "THE INSIDER — PROSPECT DISCOVERY BOARD");
    let _ = writeln!(out, "{} validated studies", view.studies);
    render_prospect_board_lane(&mut out, "HIDDEN GEMS", &view.hidden_gems);
    render_prospect_board_lane(&mut out, "BUYER BEWARE", &view.buyer_beware);
    render_prospect_board_lane(&mut out, "WATCH", &view.watch);
    let _ = writeln!(out, "\nDISCLOSURES");
    for disclosure in &view.disclosures {
        let _ = writeln!(out, "- {disclosure}");
    }
    out
}

fn render_prospect_board_lane(out: &mut String, heading: &str, rows: &[ProspectDiscoveryBoardRow]) {
    let _ = writeln!(out, "\n{heading}");
    if rows.is_empty() {
        let _ = writeln!(out, "No supported candidates.");
        return;
    }
    for row in rows {
        let lenses = row
            .lenses
            .iter()
            .map(|lens| {
                format!(
                    "{:?}/{:?} {:.0}%",
                    lens.kind,
                    lens.direction,
                    lens.strength * 100.0
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            out,
            "{}. {} · {} · {} · lane {:.1} · hidden {:.1} · gap {:+.2}\n   {:?} / {:?} · {}",
            row.rank,
            row.player,
            row.organization,
            row.position,
            row.lane_score,
            row.hidden_value_score,
            row.performance_attention_gap,
            row.classification,
            row.market_position,
            lenses
        );
    }
}

const OPENING_ROSTER_ARCHIVE_SCHEMA: &str = "icecast.opening_roster_archive.v1";
const OPENING_ROSTER_ARCHIVE_SOURCE: &str = "internet_archive_official_nhl_api";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpeningRosterArchiveManifest {
    schema: String,
    season: u32,
    opening_date: NaiveDate,
    captures: Vec<OpeningRosterArchiveCapture>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpeningRosterArchiveCapture {
    team: String,
    archive_url: String,
}

const OPENING_ROSTER_DISCOVERY_SCHEMA: &str = "icecast.opening_roster_archive_discovery.v1";

#[derive(Debug, Clone, Serialize)]
struct OpeningRosterArchiveDiscoveryView {
    schema: String,
    season: u32,
    opening_date: NaiveDate,
    expected_teams: usize,
    covered_teams: usize,
    complete: bool,
    captures: Vec<OpeningRosterArchiveCapture>,
    missing_teams: Vec<String>,
    request_errors: BTreeMap<String, String>,
    cache_fallback_teams: Vec<String>,
    import_manifest: Option<OpeningRosterArchiveManifest>,
    partial_evaluation_manifest: Option<OpeningRosterArchiveManifest>,
    disclosures: Vec<String>,
}

#[derive(Debug)]
struct OpeningRosterArchiveDiscoveryResult {
    capture: Option<OpeningRosterArchiveCapture>,
    cache_fallback: bool,
}

pub async fn run_discover_opening_rosters(
    season: u32,
    out: Option<PathBuf>,
    manifest_out: Option<PathBuf>,
    partial_manifest_out: Option<PathBuf>,
    cache_only: bool,
) -> anyhow::Result<()> {
    let schedule = load_fantasy_schedule(Season(season), false).await?;
    let opening_date = schedule
        .iter()
        .filter(|game| game.game_type == 2)
        .map(|game| NaiveDate::parse_from_str(&game.date, "%Y-%m-%d"))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .min()
        .context("season schedule has no regular-season opening date")?;
    let teams = schedule
        .iter()
        .filter(|game| game.game_type == 2)
        .flat_map(|game| [&game.away_abbrev, &game.home_abbrev])
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent("IceLines opening-roster archive discovery")
        .build()
        .context("build archive discovery HTTP client")?;
    let cache_root = Config::load()?
        .cache_dir
        .join("icecast-archive-cdx")
        .join(season.to_string());
    let mut captures = Vec::new();
    let mut missing_teams = Vec::new();
    let mut request_errors = BTreeMap::new();
    let mut cache_fallback_teams = Vec::new();
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(4));
    let mut tasks = tokio::task::JoinSet::new();
    for team in &teams {
        let client = client.clone();
        let team = team.clone();
        let semaphore = semaphore.clone();
        let cache_root = cache_root.clone();
        tasks.spawn(async move {
            let _permit = semaphore.acquire_owned().await.expect("semaphore is open");
            let result = discover_official_nhl_archive_capture(
                &client,
                &team,
                season,
                opening_date,
                &cache_root,
                cache_only,
            )
            .await;
            (team, result)
        });
    }
    while let Some(task) = tasks.join_next().await {
        let (team, result) = task.context("archive discovery worker failed")?;
        match result {
            Ok(result) => {
                if result.cache_fallback {
                    cache_fallback_teams.push(team.clone());
                }
                match result.capture {
                    Some(capture) => captures.push(capture),
                    None => missing_teams.push(team),
                }
            }
            Err(error) => {
                request_errors.insert(team, error.to_string());
            }
        }
    }
    captures.sort_by(|a, b| a.team.cmp(&b.team));
    missing_teams.sort();
    cache_fallback_teams.sort();
    let complete = captures.len() == teams.len() && request_errors.is_empty();
    let import_manifest = complete.then(|| OpeningRosterArchiveManifest {
        schema: OPENING_ROSTER_ARCHIVE_SCHEMA.to_owned(),
        season,
        opening_date,
        captures: captures.clone(),
    });
    let partial_evaluation_manifest =
        (!captures.is_empty()).then(|| OpeningRosterArchiveManifest {
            schema: OPENING_ROSTER_ARCHIVE_SCHEMA.to_owned(),
            season,
            opening_date,
            captures: captures.clone(),
        });
    let view = OpeningRosterArchiveDiscoveryView {
        schema: OPENING_ROSTER_DISCOVERY_SCHEMA.to_owned(),
        season,
        opening_date,
        expected_teams: teams.len(),
        covered_teams: captures.len(),
        complete,
        captures,
        missing_teams,
        request_errors,
        cache_fallback_teams,
        import_manifest: import_manifest.clone(),
        partial_evaluation_manifest: partial_evaluation_manifest.clone(),
        disclosures: vec![
            "Discovery queries the Internet Archive CDX index and does not treat index rows as roster authority; the importer still downloads, parses, seals, and revalidates every payload.".to_owned(),
            "The latest official season-roster capture strictly before the loaded schedule's opening date is selected independently for each team.".to_owned(),
            "Successfully parsed CDX responses are cached per season/team. A later request failure may reuse that response and is listed in cache_fallback_teams rather than request_errors.".to_owned(),
            if cache_only {
                "Cache-only mode made no Internet Archive requests; cache_fallback_teams identifies responses loaded from the local audited cache.".to_owned()
            } else {
                "Network mode queried the Internet Archive before considering a cached response.".to_owned()
            },
        ],
    };
    let json = format!("{}\n", serde_json::to_string_pretty(&view)?);
    if let Some(path) = out {
        write_icecast_file(&path, json.as_bytes(), "archive discovery")?;
    } else {
        print!("{json}");
    }
    if let Some(path) = manifest_out {
        let manifest = import_manifest.context(format!(
            "archive discovery covered {}/{} teams; refusing to write an incomplete import manifest",
            view.covered_teams, view.expected_teams
        ))?;
        let json = format!("{}\n", serde_json::to_string_pretty(&manifest)?);
        write_icecast_file(&path, json.as_bytes(), "opening-roster import manifest")?;
    }
    if let Some(path) = partial_manifest_out {
        let manifest = partial_evaluation_manifest
            .context("archive discovery found no captures for a partial evaluation manifest")?;
        let json = format!("{}\n", serde_json::to_string_pretty(&manifest)?);
        write_icecast_file(
            &path,
            json.as_bytes(),
            "partial opening-roster evaluation manifest",
        )?;
    }
    Ok(())
}

async fn discover_official_nhl_archive_capture(
    client: &reqwest::Client,
    team: &str,
    season: u32,
    opening_date: NaiveDate,
    cache_root: &std::path::Path,
    cache_only: bool,
) -> anyhow::Result<OpeningRosterArchiveDiscoveryResult> {
    let season_window = season.to_string();
    let season_cache = cache_root.join(format!("{team}-season.json"));
    let legacy_cache = cache_root.join(format!("{team}.json"));
    if !season_cache.exists() && legacy_cache.exists() {
        let legacy = std::fs::read(&legacy_cache)
            .with_context(|| format!("read legacy archive cache for {team}"))?;
        icelines_fetch::snapshot::atomic_write_bytes(&season_cache, &legacy)
            .with_context(|| format!("migrate legacy archive cache for {team}"))?;
    }
    let season_result = discover_official_nhl_archive_endpoint(
        client,
        team,
        opening_date,
        &season_window,
        &season_cache,
        cache_only,
    )
    .await;
    if let Ok(result) = &season_result {
        if result.capture.is_some() {
            return Ok(OpeningRosterArchiveDiscoveryResult {
                capture: result.capture.clone(),
                cache_fallback: result.cache_fallback,
            });
        }
    }
    let current_result = discover_official_nhl_archive_endpoint(
        client,
        team,
        opening_date,
        "current",
        &cache_root.join(format!("{team}-current.json")),
        cache_only,
    )
    .await;
    match (season_result, current_result) {
        (Ok(season), Ok(current)) => Ok(OpeningRosterArchiveDiscoveryResult {
            capture: current.capture,
            cache_fallback: season.cache_fallback || current.cache_fallback,
        }),
        (Err(_), Ok(current)) if current.capture.is_some() => Ok(current),
        (Ok(_), Err(current_error)) => Err(current_error),
        (Err(season_error), Ok(_)) => Err(season_error),
        (Err(season_error), Err(current_error)) => Err(anyhow::anyhow!(
            "season endpoint failed: {season_error:#}; current endpoint failed: {current_error:#}"
        )),
    }
}

async fn discover_official_nhl_archive_endpoint(
    client: &reqwest::Client,
    team: &str,
    opening_date: NaiveDate,
    roster_window: &str,
    cache_path: &std::path::Path,
    cache_only: bool,
) -> anyhow::Result<OpeningRosterArchiveDiscoveryResult> {
    let original = format!("https://api-web.nhle.com/v1/roster/{team}/{roster_window}");
    let network = if cache_only {
        Err(anyhow::anyhow!("cache-only archive discovery requested"))
    } else {
        async {
            let response = client
                .get("https://web.archive.org/cdx/search/cdx")
                .query(&[
                    ("url", original.as_str()),
                    ("output", "json"),
                    ("filter", "statuscode:200"),
                    ("fl", "timestamp,original,statuscode,digest"),
                ])
                .send()
                .await
                .with_context(|| format!("query archive index for {team}"))?
                .error_for_status()
                .with_context(|| format!("archive index rejected {team}"))?;
            let body = response
                .bytes()
                .await
                .with_context(|| format!("read archive index for {team}"))?;
            Ok::<_, anyhow::Error>(body.to_vec())
        }
        .await
    };
    resolve_archive_discovery_response(network, cache_path, team, opening_date, roster_window)
}

fn resolve_archive_discovery_response(
    network: anyhow::Result<Vec<u8>>,
    cache_path: &std::path::Path,
    team: &str,
    opening_date: NaiveDate,
    roster_window: &str,
) -> anyhow::Result<OpeningRosterArchiveDiscoveryResult> {
    match network {
        Ok(body) => {
            let capture = select_latest_cdx_capture(&body, team, opening_date, roster_window)?;
            icelines_fetch::snapshot::atomic_write_bytes(cache_path, &body)
                .with_context(|| format!("cache parsed archive index for {team}"))?;
            Ok(OpeningRosterArchiveDiscoveryResult {
                capture,
                cache_fallback: false,
            })
        }
        Err(network_error) => {
            let cached = std::fs::read(cache_path).with_context(|| {
                format!(
                    "{network_error:#}; no cached archive index is available for {team} at {}",
                    cache_path.display()
                )
            })?;
            let capture = select_latest_cdx_capture(&cached, team, opening_date, roster_window)
                .with_context(|| {
                    format!("{network_error:#}; cached archive index for {team} is invalid")
                })?;
            Ok(OpeningRosterArchiveDiscoveryResult {
                capture,
                cache_fallback: true,
            })
        }
    }
}

fn select_latest_cdx_capture(
    body: &[u8],
    team: &str,
    opening_date: NaiveDate,
    roster_window: &str,
) -> anyhow::Result<Option<OpeningRosterArchiveCapture>> {
    let rows: Vec<Vec<String>> =
        serde_json::from_slice(body).context("parse Internet Archive CDX response")?;
    let Some(header) = rows.first() else {
        return Ok(None);
    };
    let timestamp_index = header
        .iter()
        .position(|column| column == "timestamp")
        .context("CDX response has no timestamp column")?;
    let original_index = header
        .iter()
        .position(|column| column == "original")
        .context("CDX response has no original column")?;
    let expected = format!("https://api-web.nhle.com/v1/roster/{team}/{roster_window}");
    let preseason_start = NaiveDate::from_ymd_opt(opening_date.year(), 7, 1)
        .context("opening date has an invalid preseason year")?;
    let mut candidates = rows
        .iter()
        .skip(1)
        .filter_map(|row| {
            let timestamp = row.get(timestamp_index)?;
            let original = row.get(original_index)?;
            if original.trim_end_matches('/') != expected {
                return None;
            }
            let captured_at = NaiveDateTime::parse_from_str(timestamp, "%Y%m%d%H%M%S").ok()?;
            (captured_at.date() >= preseason_start && captured_at.date() < opening_date)
                .then_some((timestamp, captured_at))
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|row| std::cmp::Reverse(row.1));
    Ok(candidates
        .first()
        .map(|(timestamp, _)| OpeningRosterArchiveCapture {
            team: team.to_owned(),
            archive_url: format!(
                "https://web.archive.org/web/{timestamp}id_/https://api-web.nhle.com/v1/roster/{team}/{roster_window}"
            ),
        }))
}

fn write_icecast_file(path: &std::path::Path, bytes: &[u8], label: &str) -> anyhow::Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    std::fs::write(path, bytes).with_context(|| format!("write {}", path.display()))?;
    println!("Wrote {label} to {}", path.display());
    Ok(())
}

pub async fn run_import_opening_rosters(
    manifest_path: PathBuf,
    dry_run: bool,
    allow_partial_evaluation: bool,
) -> anyhow::Result<()> {
    let bytes = std::fs::read(&manifest_path)
        .with_context(|| format!("read {}", manifest_path.display()))?;
    let manifest: OpeningRosterArchiveManifest = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse {}", manifest_path.display()))?;
    let schedule = load_fantasy_schedule(Season(manifest.season), false).await?;
    let schedule_opening_date = schedule
        .iter()
        .filter(|game| game.game_type == 2)
        .map(|game| NaiveDate::parse_from_str(&game.date, "%Y-%m-%d"))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .min()
        .context("season schedule has no regular-season opening date")?;
    if manifest.opening_date != schedule_opening_date {
        bail!(
            "archive manifest opening_date {} does not match the loaded schedule boundary {}",
            manifest.opening_date,
            schedule_opening_date
        );
    }
    let captures = validate_opening_roster_archive_manifest(&manifest, !allow_partial_evaluation)?;
    let expected_teams = icelines_fetch::nhl_teams_for_season(&manifest.season.to_string()).len();
    let partial_evaluation = captures.len() < expected_teams;
    let evidence_at = captures
        .iter()
        .map(|(_, captured_at, _)| *captured_at)
        .max()
        .context("opening-roster archive manifest has no captures")?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let snapshot = format!(
        "{}-{}-opening-archive-{}",
        manifest.season,
        evidence_at.date_naive(),
        &digest[..8]
    );
    if dry_run {
        println!(
            "Would import {} of {} verified archive captures into snapshot '{}' (evidence at {}; mode {})",
            captures.len(),
            expected_teams,
            snapshot,
            evidence_at.to_rfc3339(),
            if partial_evaluation { "partial_evaluation" } else { "authoritative_candidate" }
        );
        return Ok(());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("IceLines opening-roster archive importer")
        .build()
        .context("build archive HTTP client")?;
    let mut rosters = Vec::with_capacity(captures.len());
    for (team, _, archive_url) in &captures {
        let mut roster = None;
        let mut failures = Vec::new();
        for attempt in 1..=3 {
            let result = async {
                let response = client
                    .get(archive_url)
                    .send()
                    .await
                    .with_context(|| format!("fetch archived opening roster for {team}"))?
                    .error_for_status()
                    .with_context(|| format!("archive rejected opening roster for {team}"))?;
                let content_type = response
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or("unknown")
                    .to_owned();
                let body = response
                    .bytes()
                    .await
                    .with_context(|| format!("read archived opening roster for {team}"))?;
                parse_archived_roster_payload(team, &body, &content_type)
            }
            .await;
            match result {
                Ok(value) => {
                    roster = Some(value);
                    break;
                }
                Err(error) => {
                    failures.push(format!("attempt {attempt}: {error:#}"));
                    if attempt < 3 {
                        tokio::time::sleep(std::time::Duration::from_millis(250 * attempt)).await;
                    }
                }
            }
        }
        let roster = roster.with_context(|| {
            format!(
                "archived NHL roster for {team} failed after 3 attempts: {}",
                failures.join("; ")
            )
        })?;
        rosters.push((team.clone(), serde_json::to_vec(&roster)?));
    }

    let config = Config::load()?;
    let store = SnapshotStore::new(config.snapshot_dir());
    if let Some(existing) = store
        .list()
        .unwrap_or_default()
        .into_iter()
        .find(|entry| entry.name == snapshot)
    {
        if existing.sealed {
            bail!("snapshot '{snapshot}' is already sealed; refusing to rewrite archive evidence");
        }
        println!("Resuming unsealed archive snapshot '{snapshot}'.");
    }
    store
        .create_with_evidence(
            &snapshot,
            &manifest.season.to_string(),
            SnapshotTier::Rosters,
            None,
            &evidence_at.date_naive().to_string(),
            Some(evidence_at.to_rfc3339()),
            Some(OPENING_ROSTER_ARCHIVE_SOURCE.to_owned()),
        )
        .context("create archived opening-roster snapshot")?;
    for (team, roster) in rosters {
        store
            .write_file(
                &snapshot,
                &SnapshotTier::Rosters,
                &format!("{team}.json"),
                &roster,
            )
            .with_context(|| format!("write archived opening roster for {team}"))?;
    }
    let provenance = serde_json::to_vec_pretty(&manifest)?;
    store.write_file(
        &snapshot,
        &SnapshotTier::Rosters,
        "_opening-roster-archive.json",
        &provenance,
    )?;
    store.seal(&snapshot)?;
    println!(
        "Imported {} of {} opening rosters into sealed snapshot '{}' with evidence time {} (mode {}).",
        captures.len(),
        expected_teams,
        snapshot,
        evidence_at.to_rfc3339(),
        if partial_evaluation { "partial_evaluation" } else { "authoritative_candidate" }
    );
    Ok(())
}

fn parse_archived_roster_payload(
    team: &str,
    body: &[u8],
    content_type: &str,
) -> anyhow::Result<RosterResponse> {
    const MAX_ROSTER_BYTES: u64 = 4 * 1024 * 1024;
    let decoded;
    let payload = if body.starts_with(&[0x1f, 0x8b]) {
        let mut decoder = flate2::read::GzDecoder::new(body).take(MAX_ROSTER_BYTES + 1);
        decoded = {
            let mut value = Vec::new();
            decoder
                .read_to_end(&mut value)
                .with_context(|| format!("decompress archived NHL roster for {team}"))?;
            if value.len() as u64 > MAX_ROSTER_BYTES {
                bail!("decompressed archived NHL roster for {team} exceeds 4 MiB");
            }
            value
        };
        decoded.as_slice()
    } else {
        body
    };
    let roster: RosterResponse = serde_json::from_slice(payload).with_context(|| {
        let signature = String::from_utf8_lossy(&body[..body.len().min(120)])
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            "parse archived NHL roster for {team} (content-type {content_type}, {} bytes, prefix {:?})",
            body.len(),
            signature
        )
    })?;
    if roster.forwards.is_empty() && roster.defensemen.is_empty() && roster.goalies.is_empty() {
        bail!("archived NHL roster for {team} is empty");
    }
    Ok(roster)
}

fn validate_opening_roster_archive_manifest(
    manifest: &OpeningRosterArchiveManifest,
    require_complete: bool,
) -> anyhow::Result<Vec<(String, DateTime<Utc>, String)>> {
    if manifest.schema != OPENING_ROSTER_ARCHIVE_SCHEMA {
        bail!(
            "unsupported opening-roster archive schema '{}'; expected {OPENING_ROSTER_ARCHIVE_SCHEMA}",
            manifest.schema
        );
    }
    let season_start_year = (manifest.season / 10_000) as i32;
    let preseason_start = NaiveDate::from_ymd_opt(season_start_year, 7, 1)
        .context("archive manifest season has an invalid start year")?;
    if manifest.opening_date.year() != season_start_year || manifest.opening_date <= preseason_start
    {
        bail!(
            "archive manifest has an invalid opening_date for season {}",
            manifest.season
        );
    }
    let expected = icelines_fetch::nhl_teams_for_season(&manifest.season.to_string())
        .into_iter()
        .map(str::to_owned)
        .collect::<std::collections::BTreeSet<_>>();
    let mut observed = std::collections::BTreeSet::new();
    let mut captures = Vec::with_capacity(manifest.captures.len());
    for capture in &manifest.captures {
        let team = capture.team.trim().to_ascii_uppercase();
        if !expected.contains(&team) {
            bail!("archive manifest contains unexpected team {team}");
        }
        if !observed.insert(team.clone()) {
            bail!("archive manifest contains duplicate team {team}");
        }
        let captured_at =
            parse_official_nhl_archive_url(&capture.archive_url, &team, manifest.season)?;
        if captured_at.date_naive() < preseason_start
            || captured_at.date_naive() >= manifest.opening_date
        {
            bail!(
                "archive capture for {team} must be within the preseason window {} through {}",
                preseason_start,
                manifest.opening_date.pred_opt().unwrap()
            );
        }
        if captured_at > Utc::now() {
            bail!("archive capture for {team} is in the future");
        }
        captures.push((team, captured_at, capture.archive_url.clone()));
    }
    let missing = expected.difference(&observed).cloned().collect::<Vec<_>>();
    if observed.is_empty() {
        bail!("archive manifest contains no team captures");
    }
    if require_complete && !missing.is_empty() {
        bail!(
            "archive manifest covers {}/{} teams; missing {}",
            observed.len(),
            expected.len(),
            missing.join(", ")
        );
    }
    captures.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(captures)
}

fn parse_official_nhl_archive_url(
    archive_url: &str,
    team: &str,
    season: u32,
) -> anyhow::Result<DateTime<Utc>> {
    let remainder = archive_url
        .strip_prefix("https://web.archive.org/web/")
        .context("archive URL must use https://web.archive.org/web/")?;
    let (timestamp, original) = remainder
        .split_once("id_/")
        .context("archive URL must request the immutable id_ payload")?;
    if timestamp.len() != 14 || !timestamp.bytes().all(|byte| byte.is_ascii_digit()) {
        bail!("archive URL has an invalid 14-digit capture timestamp");
    }
    let season_endpoint = format!("https://api-web.nhle.com/v1/roster/{team}/{season}");
    let current_endpoint = format!("https://api-web.nhle.com/v1/roster/{team}/current");
    let original = original.trim_end_matches('/');
    if original != season_endpoint && original != current_endpoint {
        bail!(
            "archive URL must capture the official NHL roster endpoint {season_endpoint} or {current_endpoint}"
        );
    }
    Ok(NaiveDateTime::parse_from_str(timestamp, "%Y%m%d%H%M%S")?.and_utc())
}

fn load_scenario(path: &std::path::Path, season: u32) -> anyhow::Result<TeamSeasonScenario> {
    let bytes = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse IceCast scenario {}", path.display()))?;
    let mut scenario = if value.get("schema").and_then(serde_json::Value::as_str)
        == Some(TEAM_SEASON_SCENARIO_DEVELOPMENT_CALIBRATION_SCHEMA)
    {
        serde_json::from_value::<TeamSeasonScenarioDevelopmentCalibrationView>(value)
            .with_context(|| format!("parse calibrated IceCast scenario {}", path.display()))?
            .scenario
    } else {
        serde_json::from_value::<TeamSeasonScenario>(value)
            .with_context(|| format!("parse IceCast scenario {}", path.display()))?
    };
    if season == CURRENT_SEASON && scenario.trade_deadline.is_none() {
        scenario.trade_deadline = NaiveDate::from_ymd_opt(2027, 3, 5);
    }
    Ok(scenario)
}

fn schedule_calendar_fingerprint(schedule: &[ScheduledGame]) -> anyhow::Result<String> {
    let mut calendar = schedule
        .iter()
        .filter(|game| game.game_type == 2)
        .map(|game| {
            (
                game.game_id,
                game.date.as_str(),
                game.game_type,
                game.away_abbrev.as_str(),
                game.home_abbrev.as_str(),
                game.start_time_utc.as_str(),
            )
        })
        .collect::<Vec<_>>();
    calendar.sort_unstable();
    let bytes = serde_json::to_vec(&calendar).context("serialize NHL schedule calendar")?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn parse_evidence_label(value: &str) -> anyhow::Result<EvidenceLabel> {
    match value {
        "confirmed" => Ok(EvidenceLabel::Confirmed),
        "reported" => Ok(EvidenceLabel::Reported),
        "estimated" => Ok(EvidenceLabel::Estimated),
        "simulated" => Ok(EvidenceLabel::Simulated),
        "under-review" => Ok(EvidenceLabel::UnderReview),
        "no-read" => Ok(EvidenceLabel::NoRead),
        _ => bail!("unsupported evidence label '{value}'"),
    }
}

pub async fn run_scenario_import(
    id: String,
    path: PathBuf,
    season: u32,
    evidence: String,
    json: bool,
) -> anyhow::Result<()> {
    let scenario = load_scenario(&path, season)?;
    let schedule = load_fantasy_schedule(Season(season), false).await?;
    let scope = ScenarioScopeView {
        league_id: "nhl".to_string(),
        season,
        season_type: SeasonType::Regular,
        team_ids: Vec::new(),
        calendar_fingerprint: Some(schedule_calendar_fingerprint(&schedule)?),
    };
    let source_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    let result = ScenarioRegistryStore::new(ScenarioRegistryStore::default_root())
        .import_team_season_scenario(
            &id,
            scope,
            parse_evidence_label(&evidence)?,
            &scenario,
            Utc::now(),
            source_name,
        )
        .with_context(|| format!("import IceCast scenario {id}"))?;

    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!(
            "{} {} -> {} ({})",
            match result.disposition {
                icelines_fetch::ScenarioImportDisposition::Inserted => "Imported",
                icelines_fetch::ScenarioImportDisposition::Unchanged => "Unchanged",
            },
            result.entry.id,
            result.entry.content_sha256,
            result.entry.scope.team_ids.join(", ")
        );
    }
    Ok(())
}

pub fn run_scenario_list(json: bool) -> anyhow::Result<()> {
    let registry = ScenarioRegistryStore::new(ScenarioRegistryStore::default_root())
        .list()
        .context("list IceCast scenarios")?;
    if json {
        println!("{}", serde_json::to_string_pretty(&registry)?);
    } else if registry.entries.is_empty() {
        println!("No registered IceCast scenarios.");
    } else {
        for entry in registry.entries {
            println!(
                "{}  {}  {}  {}",
                entry.id,
                entry.scope.season,
                entry.scope.team_ids.join(","),
                entry.content_sha256
            );
        }
    }
    Ok(())
}

pub fn run_scenario_show(id: String, json: bool) -> anyhow::Result<()> {
    let store = ScenarioRegistryStore::new(ScenarioRegistryStore::default_root());
    let registry = store.list().context("list IceCast scenarios")?;
    let entry = registry
        .entries
        .into_iter()
        .find(|entry| entry.id == id)
        .with_context(|| format!("scenario not found in registry: {id}"))?;
    let resolved = store
        .resolve_team_season_scenario(&id, &entry.scope)
        .with_context(|| format!("resolve IceCast scenario id {id}"))?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "entry": entry,
                "scenario": resolved.scenario,
            }))?
        );
    } else {
        println!(
            "{} — {} ({}; {} events; {})",
            entry.id,
            resolved.scenario.name,
            entry.scope.season,
            resolved.scenario.events.len(),
            entry.content_sha256
        );
        for event in resolved.scenario.events {
            println!(
                "  {} {} {}: {} ({:+.2})",
                event.effective_date, event.team, event.id, event.label, event.strength_delta
            );
        }
    }
    Ok(())
}

fn audit_opening_roster_authority(
    store: &SnapshotStore,
    season: u32,
    schedule_start: NaiveDate,
    expected_teams: &std::collections::BTreeSet<String>,
) -> TeamGameOpeningRosterAuthorityRow {
    let entries = match store.list() {
        Ok(entries) => entries,
        Err(error) => {
            return TeamGameOpeningRosterAuthorityRow {
                status: "unavailable".to_owned(),
                required_before_date: schedule_start,
                selected_snapshot: None,
                selected_snapshot_created_at: None,
                latest_observed_snapshot: None,
                latest_observed_snapshot_created_at: None,
                expected_teams: expected_teams.len(),
                verified_teams: 0,
                verified_team_abbrevs: Vec::new(),
                player_value_effects_enabled: false,
                personnel_events_effective_after: None,
                reason: format!("snapshot index could not be read: {error}"),
            };
        }
    };
    let (candidate, latest) =
        select_opening_roster_snapshot(&entries, &season.to_string(), schedule_start);
    let latest_name = latest.as_ref().map(|entry| entry.name.clone());
    let latest_created_at = latest.as_ref().and_then(snapshot_evidence_at_text);
    let Some(candidate) = candidate else {
        let reason = latest.as_ref().map_or_else(
            || format!("no sealed roster snapshot exists for season {season}"),
            |entry| {
                format!(
                    "latest roster snapshot '{}' was captured {} and is not before the {} opening-day evidence boundary",
                    entry.name,
                    snapshot_evidence_at_text(entry).unwrap_or_else(|| entry.created_at.clone()),
                    schedule_start
                )
            },
        );
        return TeamGameOpeningRosterAuthorityRow {
            status: "unavailable".to_owned(),
            required_before_date: schedule_start,
            selected_snapshot: None,
            selected_snapshot_created_at: None,
            latest_observed_snapshot: latest_name,
            latest_observed_snapshot_created_at: latest_created_at,
            expected_teams: expected_teams.len(),
            verified_teams: 0,
            verified_team_abbrevs: Vec::new(),
            player_value_effects_enabled: false,
            personnel_events_effective_after: None,
            reason,
        };
    };

    let mut verified_teams = 0;
    let mut verified_team_abbrevs = Vec::new();
    let mut failures = Vec::new();
    let mut provenance_valid = true;
    let mut archive_provenance_teams = None;
    if candidate.evidence_source.as_deref() == Some(OPENING_ROSTER_ARCHIVE_SOURCE) {
        match store.read::<OpeningRosterArchiveManifest>(
            &candidate.name,
            &SnapshotTier::Rosters,
            "_opening-roster-archive.json",
        ) {
            Ok(manifest) => match validate_opening_roster_archive_manifest(&manifest, false) {
                Ok(captures) => {
                    archive_provenance_teams = Some(
                        captures
                            .iter()
                            .map(|(team, _, _)| team.clone())
                            .collect::<std::collections::BTreeSet<_>>(),
                    );
                    let manifest_evidence_at = captures
                        .iter()
                        .map(|(_, captured_at, _)| *captured_at)
                        .max();
                    let indexed_evidence_at = snapshot_evidence_at(&candidate)
                        .ok()
                        .map(|value| value.with_timezone(&Utc));
                    if manifest.season != season
                        || manifest.opening_date != schedule_start
                        || manifest_evidence_at != indexed_evidence_at
                    {
                        provenance_valid = false;
                        failures.push(
                            "archive provenance season, opening date, or evidence timestamp does not match the replay boundary"
                                .to_owned(),
                        );
                    }
                }
                Err(error) => {
                    provenance_valid = false;
                    failures.push(format!("archive provenance is invalid: {error}"));
                }
            },
            Err(error) => {
                provenance_valid = false;
                failures.push(format!("archive provenance is unavailable: {error}"));
            }
        }
    } else if candidate.evidence_source.as_deref() == Some(OFFICIAL_NHL_LIVE_ROSTER_SOURCE) {
        match store.read::<OfficialNhlRosterCaptureManifest>(
            &candidate.name,
            &SnapshotTier::Rosters,
            OFFICIAL_NHL_LIVE_ROSTER_MANIFEST_FILE,
        ) {
            Ok(manifest) => {
                let indexed_evidence_at = candidate.evidence_at.as_deref();
                let mut captured_teams = std::collections::BTreeSet::new();
                let captures_valid = manifest.captures.iter().all(|capture| {
                    let team = capture.team.trim().to_ascii_uppercase();
                    captured_teams.insert(team.clone())
                        && expected_teams.contains(&team)
                        && capture.source_url == roster_url(&team, &season.to_string())
                });
                if manifest.schema != OFFICIAL_NHL_LIVE_ROSTER_SCHEMA
                    || manifest.season != season.to_string()
                    || indexed_evidence_at != Some(manifest.observed_at.as_str())
                    || !captures_valid
                    || captured_teams != *expected_teams
                {
                    provenance_valid = false;
                    failures.push(
                        "official live roster provenance schema, season, timestamp, team coverage, or source URLs are invalid"
                            .to_owned(),
                    );
                }
            }
            Err(error) => {
                provenance_valid = false;
                failures.push(format!(
                    "official live roster provenance is unavailable: {error}"
                ));
            }
        }
    } else {
        provenance_valid = false;
        failures.push("roster snapshot has no recognized official NHL provenance class".to_owned());
    }
    for team in expected_teams {
        if archive_provenance_teams
            .as_ref()
            .is_some_and(|teams| !teams.contains(team))
        {
            failures.push(format!(
                "{team}: no verified archive capture in provenance manifest"
            ));
            continue;
        }
        match store.read::<RosterResponse>(
            &candidate.name,
            &SnapshotTier::Rosters,
            &format!("{team}.json"),
        ) {
            Ok(roster)
                if !roster.forwards.is_empty()
                    || !roster.defensemen.is_empty()
                    || !roster.goalies.is_empty() =>
            {
                verified_teams += 1;
                verified_team_abbrevs.push(team.clone());
            }
            Ok(_) => failures.push(format!("{team} roster is empty")),
            Err(error) => failures.push(format!("{team}: {error}")),
        }
    }
    if !provenance_valid {
        verified_teams = 0;
        verified_team_abbrevs.clear();
    }
    verified_team_abbrevs.sort();
    let valid = provenance_valid && failures.is_empty() && verified_teams == expected_teams.len();
    let partial = provenance_valid && verified_teams > 0 && !valid;
    TeamGameOpeningRosterAuthorityRow {
        status: if valid {
            "authoritative"
        } else if partial {
            "partial_evaluation"
        } else {
            "invalid"
        }
        .to_owned(),
        required_before_date: schedule_start,
        selected_snapshot: Some(candidate.name.clone()),
        selected_snapshot_created_at: snapshot_evidence_at_text(&candidate),
        latest_observed_snapshot: latest_name,
        latest_observed_snapshot_created_at: latest_created_at,
        expected_teams: expected_teams.len(),
        verified_teams,
        verified_team_abbrevs,
        player_value_effects_enabled: false,
        personnel_events_effective_after: snapshot_evidence_at(&candidate)
            .ok()
            .map(|value| value.date_naive()),
        reason: if valid {
            format!(
                "snapshot '{}' has trusted {} evidence before opening day and passed integrity/non-empty roster checks for all {verified_teams} teams",
                candidate.name,
                candidate.evidence_source.as_deref().unwrap_or("local-capture")
            )
        } else if partial {
            format!(
                "snapshot '{}' has trusted partial opening-roster evidence for {verified_teams}/{} teams; missing or invalid teams remain neutral and full promotion stays blocked: {}",
                candidate.name,
                expected_teams.len(),
                failures.join("; ")
            )
        } else {
            format!(
                "snapshot '{}' failed {} opening-roster validation check(s): {}",
                candidate.name,
                failures.len(),
                failures.join("; ")
            )
        },
    }
}

fn select_opening_roster_snapshot(
    entries: &[SnapshotEntry],
    season: &str,
    schedule_start: NaiveDate,
) -> (Option<SnapshotEntry>, Option<SnapshotEntry>) {
    let mut roster_entries = entries
        .iter()
        .filter(|entry| {
            entry.season == season && entry.sealed && entry.tier == SnapshotTier::Rosters
        })
        .filter_map(|entry| snapshot_evidence_at(entry).ok().map(|value| (value, entry)))
        .collect::<Vec<_>>();
    roster_entries.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.name.cmp(&a.1.name)));
    let latest = roster_entries.first().map(|(_, entry)| (*entry).clone());
    let candidate = roster_entries
        .iter()
        .find(|(created_at, _)| created_at.date_naive() < schedule_start)
        .map(|(_, entry)| (*entry).clone());
    (candidate, latest)
}

fn snapshot_evidence_at(
    entry: &SnapshotEntry,
) -> Result<DateTime<chrono::FixedOffset>, chrono::ParseError> {
    let value = match (&entry.evidence_at, entry.evidence_source.as_deref()) {
        (Some(value), Some("internet_archive_official_nhl_api" | "official_nhl_api_live")) => value,
        (Some(_), _) => &entry.created_at,
        (None, _) => &entry.created_at,
    };
    DateTime::parse_from_rfc3339(value)
}

fn snapshot_evidence_at_text(entry: &SnapshotEntry) -> Option<String> {
    snapshot_evidence_at(entry)
        .ok()
        .map(|value| value.to_rfc3339())
}

async fn load_retrospective_opening_lineups(
    season: u32,
    games: &[TeamForecastGameInput],
    refresh: bool,
    store: &SnapshotStore,
    cache_root: &std::path::Path,
) -> anyhow::Result<(
    TeamGameOpeningRosterAuthorityRow,
    Vec<TeamGameOpeningStrengthRow>,
)> {
    let mut first_game_by_team = BTreeMap::<String, (NaiveDate, u64)>::new();
    for game in games {
        for team in [&game.away_team, &game.home_team] {
            let candidate = (game.date, game.game_id);
            first_game_by_team
                .entry(team.clone())
                .and_modify(|current| {
                    if candidate < *current {
                        *current = candidate;
                    }
                })
                .or_insert(candidate);
        }
    }
    let schedule_start = first_game_by_team
        .values()
        .map(|(date, _)| *date)
        .min()
        .context("retrospective lineup schedule has no teams")?;
    let game_ids = first_game_by_team
        .values()
        .map(|(_, game_id)| *game_id)
        .collect::<std::collections::BTreeSet<_>>();
    let client = NhlApiClient::production();
    let cache_dir = cache_root
        .join("icecast-opening-lineups")
        .join(season.to_string());
    let mut boxscores = BTreeMap::new();
    let mut cache_hits = 0usize;
    for game_id in game_ids {
        let (raw, cache_hit) = load_retrospective_boxscore(
            &client,
            game_id,
            &cache_dir.join(format!("{game_id}.json")),
            refresh,
        )
        .await?;
        cache_hits += usize::from(cache_hit);
        boxscores.insert(game_id, raw);
    }
    let mut rosters = Vec::with_capacity(first_game_by_team.len());
    let mut verified_teams = Vec::with_capacity(first_game_by_team.len());
    for (team, (date, game_id)) in &first_game_by_team {
        let raw = boxscores
            .get(game_id)
            .with_context(|| format!("retrospective boxscore {game_id} was not loaded"))?;
        let roster = parse_retrospective_opening_lineup(raw, team)
            .with_context(|| format!("parse {team} lineup from first game {game_id}"))?;
        rosters.push((team.clone(), *date, roster));
        verified_teams.push(team.clone());
    }
    let strengths = build_opening_team_strengths_from_rosters(store, season, rosters)?;
    let effects_enabled = !strengths.is_empty();
    Ok((
        TeamGameOpeningRosterAuthorityRow {
            status: "retrospective_evaluation".to_owned(),
            required_before_date: schedule_start,
            selected_snapshot: None,
            selected_snapshot_created_at: None,
            latest_observed_snapshot: None,
            latest_observed_snapshot_created_at: None,
            expected_teams: first_game_by_team.len(),
            verified_teams: verified_teams.len(),
            verified_team_abbrevs: verified_teams,
            player_value_effects_enabled: effects_enabled,
            personnel_events_effective_after: None,
            reason: format!(
                "official first-game boxscores supplied complete dressed-player identity for all {} teams ({} unique games, {} cache hits); evidence is retrospective and cannot satisfy pregame opening-roster authority",
                first_game_by_team.len(),
                boxscores.len(),
                cache_hits
            ),
        },
        strengths,
    ))
}

async fn load_retrospective_boxscore(
    client: &NhlApiClient,
    game_id: u64,
    cache_path: &std::path::Path,
    refresh: bool,
) -> anyhow::Result<(serde_json::Value, bool)> {
    let read_cache = || -> anyhow::Result<serde_json::Value> {
        let bytes = std::fs::read(cache_path)
            .with_context(|| format!("read cached retrospective boxscore {game_id}"))?;
        serde_json::from_slice(&bytes)
            .with_context(|| format!("parse cached retrospective boxscore {game_id}"))
    };
    if !refresh && cache_path.exists() {
        if let Ok(raw) = read_cache() {
            return Ok((raw, true));
        }
    }
    match client.fetch_boxscore_with_raw(game_id).await {
        Ok((_, raw)) => {
            let bytes = serde_json::to_vec(&raw)?;
            icelines_fetch::snapshot::atomic_write_bytes(cache_path, &bytes)
                .with_context(|| format!("cache retrospective boxscore {game_id}"))?;
            Ok((raw, false))
        }
        Err(error) if cache_path.exists() => read_cache()
            .map(|raw| (raw, true))
            .with_context(|| format!("live retrospective boxscore {game_id} failed: {error}")),
        Err(error) => Err(anyhow::anyhow!(
            "fetch retrospective boxscore {game_id}: {error}"
        )),
    }
}

fn parse_retrospective_opening_lineup(
    raw: &serde_json::Value,
    team: &str,
) -> anyhow::Result<RosterResponse> {
    let side = if raw["awayTeam"]["abbrev"].as_str() == Some(team) {
        "awayTeam"
    } else if raw["homeTeam"]["abbrev"].as_str() == Some(team) {
        "homeTeam"
    } else {
        bail!("boxscore does not contain team {team}");
    };
    let stats = &raw["playerByGameStats"][side];
    let parse_group = |key: &str, fallback_position: &str| -> anyhow::Result<Vec<RosterPlayer>> {
        stats[key]
            .as_array()
            .with_context(|| format!("boxscore {team} has no {key} array"))?
            .iter()
            .map(|player| retrospective_roster_player(player, fallback_position))
            .collect()
    };
    let forwards = parse_group("forwards", "F")?;
    let defensemen = parse_group("defense", "D")?;
    let goalies = parse_group("goalies", "G")?;
    let skaters = forwards.len() + defensemen.len();
    if !(15..=18).contains(&skaters) || goalies.len() != 2 {
        bail!(
            "boxscore {team} dressed {} skaters and {} goalies; expected 15-18 and 2",
            skaters,
            goalies.len()
        );
    }
    let unique_ids = forwards
        .iter()
        .chain(&defensemen)
        .chain(&goalies)
        .map(|player| player.id)
        .collect::<std::collections::BTreeSet<_>>();
    if unique_ids.len() != skaters + 2 {
        bail!("boxscore {team} lineup contains duplicate player IDs");
    }
    Ok(RosterResponse {
        forwards,
        defensemen,
        goalies,
    })
}

fn retrospective_roster_player(
    player: &serde_json::Value,
    fallback_position: &str,
) -> anyhow::Result<RosterPlayer> {
    let id = player["playerId"]
        .as_u64()
        .filter(|id| *id > 0 && *id <= u64::from(u32::MAX))
        .context("retrospective lineup player has no valid playerId")? as u32;
    let name = player["name"]["default"]
        .as_str()
        .filter(|name| !name.trim().is_empty())
        .context("retrospective lineup player has no name")?
        .trim();
    let (first_name, last_name) = name
        .split_once(' ')
        .map_or(("", name), |(first, last)| (first, last));
    Ok(RosterPlayer {
        id,
        first_name: LocalizedString::Plain(first_name.to_owned()),
        last_name: LocalizedString::Plain(last_name.to_owned()),
        sweater_number: player["sweaterNumber"].as_u64().map(|value| value as u32),
        position_code: player["position"]
            .as_str()
            .unwrap_or(fallback_position)
            .to_owned(),
        shoots_catches: None,
        birth_date: None,
        birth_country: None,
        height_in_inches: None,
        weight_in_pounds: None,
        headshot: None,
        birth_city: None,
        birth_state_province: None,
    })
}

fn load_opening_team_strengths(
    store: &SnapshotStore,
    season: u32,
    authority: &TeamGameOpeningRosterAuthorityRow,
    expected_teams: &std::collections::BTreeSet<String>,
) -> anyhow::Result<Vec<TeamGameOpeningStrengthRow>> {
    let snapshot = authority
        .selected_snapshot
        .as_deref()
        .context("opening authority did not select a roster snapshot")?;
    let as_of_date = authority
        .personnel_events_effective_after
        .context("opening authority has no snapshot evidence date")?;
    let rosters = expected_teams
        .iter()
        .map(|team| {
            let roster = store
                .read(snapshot, &SnapshotTier::Rosters, &format!("{team}.json"))
                .with_context(|| format!("reading authoritative opening roster for {team}"))?;
            Ok((team.clone(), as_of_date, roster))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    build_opening_team_strengths_from_rosters(store, season, rosters)
}

fn build_opening_team_strengths_from_rosters(
    store: &SnapshotStore,
    season: u32,
    rosters: Vec<(String, NaiveDate, RosterResponse)>,
) -> anyhow::Result<Vec<TeamGameOpeningStrengthRow>> {
    let prior_season = season
        .checked_sub(10_001)
        .context("replay season has no valid preceding NHL season identifier")?;
    let prior = load_into_repo(
        Season(prior_season),
        icelines_core::season_stats::SeasonType::Regular,
        store,
    )
    .context("loading prior-season values for opening roster")?;
    let mut rows = Vec::with_capacity(rosters.len());

    for (team, as_of_date, roster) in rosters {
        let player_value = |player_id: u32| {
            prior
                .repo
                .season(
                    icelines_core::identity::PlayerId(player_id),
                    Season(prior_season),
                    icelines_core::season_stats::SeasonType::Regular,
                )
                .map(prior_season_player_value)
        };
        let forward_values = roster
            .forwards
            .iter()
            .map(|player| player_value(player.id))
            .collect::<Vec<_>>();
        let defense_values = roster
            .defensemen
            .iter()
            .map(|player| player_value(player.id))
            .collect::<Vec<_>>();
        let goalie_values = roster
            .goalies
            .iter()
            .map(|player| player_value(player.id))
            .collect::<Vec<_>>();
        let roster_players = forward_values.len() + defense_values.len() + goalie_values.len();
        let valued_players = forward_values
            .iter()
            .chain(&defense_values)
            .chain(&goalie_values)
            .filter(|value| value.is_some())
            .count();
        let (forward_score, forwards_used) = score_opening_position_group(&forward_values, 12);
        let (defense_score, defensemen_used) = score_opening_position_group(&defense_values, 6);
        let (goalie_score, goalies_used) = score_opening_position_group(&goalie_values, 2);
        let forward_selected = opening_selected_indices(&forward_values, 12);
        let defense_selected = opening_selected_indices(&defense_values, 6);
        let goalie_selected = opening_selected_indices(&goalie_values, 2);
        let mut players = Vec::with_capacity(roster_players);
        players.extend(roster.forwards.iter().enumerate().map(|(index, player)| {
            TeamGameOpeningPlayerRow {
                player_id: player.id,
                full_name: format!("{} {}", player.first_name, player.last_name),
                position_group: "forward".to_owned(),
                prior_value: forward_values[index],
                modeled_value: forward_values[index].unwrap_or(50.0),
                selected_at_opening: forward_selected.contains(&index),
            }
        }));
        players.extend(roster.defensemen.iter().enumerate().map(|(index, player)| {
            TeamGameOpeningPlayerRow {
                player_id: player.id,
                full_name: format!("{} {}", player.first_name, player.last_name),
                position_group: "defense".to_owned(),
                prior_value: defense_values[index],
                modeled_value: defense_values[index].unwrap_or(50.0),
                selected_at_opening: defense_selected.contains(&index),
            }
        }));
        players.extend(roster.goalies.iter().enumerate().map(|(index, player)| {
            TeamGameOpeningPlayerRow {
                player_id: player.id,
                full_name: format!("{} {}", player.first_name, player.last_name),
                position_group: "goalie".to_owned(),
                prior_value: goalie_values[index],
                modeled_value: goalie_values[index].unwrap_or(50.0),
                selected_at_opening: goalie_selected.contains(&index),
            }
        }));
        let coverage = if roster_players == 0 {
            0.0
        } else {
            valued_players as f64 / roster_players as f64
        };
        let raw_strength = forward_score * 0.55 + defense_score * 0.30 + goalie_score * 0.15;
        // Missing histories already enter each group as neutral. Regress the
        // remaining team-level edge once more by total observed coverage.
        let strength = 50.0 + (raw_strength - 50.0) * coverage;
        rows.push(TeamGameOpeningStrengthRow {
            team,
            as_of_date: Some(as_of_date),
            strength: strength.clamp(0.0, 100.0),
            cohort_normalization_delta: 0.0,
            roster_players,
            valued_players,
            value_coverage: coverage,
            forwards_used,
            defensemen_used,
            goalies_used,
            players,
        });
    }
    if !rows.is_empty() {
        let cohort_mean = rows.iter().map(|row| row.strength).sum::<f64>() / rows.len() as f64;
        let normalization_delta = 50.0 - cohort_mean;
        for row in &mut rows {
            row.cohort_normalization_delta = normalization_delta;
            row.strength = (row.strength + normalization_delta).clamp(0.0, 100.0);
        }
    }
    Ok(rows)
}

fn score_opening_position_group(values: &[Option<f64>], slots: usize) -> (f64, usize) {
    let mut values = values
        .iter()
        .map(|value| value.unwrap_or(50.0))
        .collect::<Vec<_>>();
    values.sort_by(|a, b| b.total_cmp(a));
    values.truncate(slots);
    let used = values.len();
    values.resize(slots, 50.0);
    (values.iter().sum::<f64>() / slots as f64, used)
}

fn opening_selected_indices(
    values: &[Option<f64>],
    slots: usize,
) -> std::collections::BTreeSet<usize> {
    let mut ranked = values
        .iter()
        .enumerate()
        .map(|(index, value)| (index, value.unwrap_or(50.0)))
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ranked
        .into_iter()
        .take(slots)
        .map(|(index, _)| index)
        .collect()
}

fn load_replay_personnel_evidence(
    season: u32,
) -> anyhow::Result<Vec<TeamForecastPersonnelEvidenceInput>> {
    let config = Config::load()?;
    let store = SnapshotStore::new(config.snapshot_dir());
    let envelope = load_transactions_with_fallback(&season.to_string(), &store)?;
    let stats = load_into_repo(
        Season(season),
        icelines_core::season_stats::SeasonType::Regular,
        &store,
    )
    .context("loading historical identity catalog for personnel evidence")?;
    let prior_season = season.checked_sub(10_001);
    let prior_stats = prior_season.and_then(|prior| {
        load_into_repo(
            Season(prior),
            icelines_core::season_stats::SeasonType::Regular,
            &store,
        )
        .ok()
    });
    let mut identities = BTreeMap::<String, Vec<TeamForecastPersonnelPlayerInput>>::new();
    for identity in stats.repo.iter_identities() {
        let normalized = normalize_name(&identity.full_name);
        if normalized.contains(' ') {
            let prior = prior_stats.as_ref().and_then(|outcome| {
                outcome.repo.season(
                    identity.id,
                    Season(prior_season.unwrap()),
                    icelines_core::season_stats::SeasonType::Regular,
                )
            });
            identities
                .entry(normalized)
                .or_default()
                .push(TeamForecastPersonnelPlayerInput {
                    player_id: identity.id.0,
                    full_name: identity.full_name.clone(),
                    action: "unclassified".to_owned(),
                    membership_delta: 0,
                    prior_position_group: prior.map(|stats| {
                        if stats.position == Position::Goalie {
                            "goalie"
                        } else if stats.position == Position::Defense {
                            "defense"
                        } else {
                            "forward"
                        }
                        .to_owned()
                    }),
                    prior_season: prior.map(|stats| stats.season.0),
                    prior_games_played: prior.map(|stats| stats.totals.gp),
                    prior_value: prior.map(prior_season_player_value),
                });
        }
    }
    let source = format!(
        "{} transactions fetched {}",
        envelope.source, envelope.fetched_at
    );
    envelope
        .rows
        .into_iter()
        .filter_map(|event| {
            let team = event.team.clone()?;
            Some((event, team))
        })
        .map(|(event, team)| {
            let date = NaiveDate::parse_from_str(&event.date, "%Y-%m-%d")
                .with_context(|| format!("invalid personnel evidence date '{}'", event.date))?;
            let (resolved_players, ambiguous_player_names) =
                resolve_transaction_players(&event.description, &identities);
            Ok(TeamForecastPersonnelEvidenceInput {
                event_id: event.id,
                date,
                team: team.as_str().to_owned(),
                kind: event.kind.label().to_owned(),
                availability_delta: transaction_availability_delta(&event.description),
                label: event.description,
                source: source.clone(),
                resolved_players,
                ambiguous_player_names,
            })
        })
        .collect()
}

fn prior_season_player_value(stats: &icelines_core::season_stats::SeasonStats) -> f64 {
    let games = f64::from(stats.totals.gp);
    let raw = if stats.position == Position::Goalie {
        stats
            .goalie
            .as_ref()
            .and_then(|goalie| goalie.save_pct)
            .map_or(50.0, |save_pct| {
                50.0 + (f64::from(save_pct) - 0.905) * 1_000.0
            })
    } else if games > 0.0 {
        35.0 + (f64::from(stats.totals.points) / games) * 35.0
    } else {
        50.0
    }
    .clamp(20.0, 90.0);
    let prior_games = if stats.position == Position::Goalie {
        15.0
    } else {
        20.0
    };
    let credibility = games / (games + prior_games);
    50.0 * (1.0 - credibility) + raw * credibility
}

fn resolve_transaction_players(
    description: &str,
    identities: &BTreeMap<String, Vec<TeamForecastPersonnelPlayerInput>>,
) -> (Vec<TeamForecastPersonnelPlayerInput>, Vec<String>) {
    let description = normalize_name(description);
    let mut resolved = Vec::new();
    let mut ambiguous = Vec::new();
    for (name, candidates) in identities {
        if !contains_name(&description, name) {
            continue;
        }
        if candidates.len() == 1 {
            let mut player = candidates[0].clone();
            let (action, membership_delta) = personnel_player_action(&description, name);
            player.action = action.to_owned();
            player.membership_delta = membership_delta;
            resolved.push(player);
        } else {
            ambiguous.push(candidates[0].full_name.clone());
        }
    }
    resolved.sort_by_key(|player| player.player_id);
    ambiguous.sort();
    (resolved, ambiguous)
}

fn personnel_player_action(description: &str, name: &str) -> (&'static str, i8) {
    let Some(start) = description.find(name) else {
        return ("ambiguous", 0);
    };
    let clause_start = description[..start]
        .rfind(['.', ';'])
        .map_or(0, |index| index + 1);
    let clause_end = description[start..]
        .find(['.', ';'])
        .map_or(description.len(), |index| start + index);
    let prefix = &description[clause_start..start];
    let suffix = &description[start..clause_end];
    let markers = [
        ("reassigned", "assigned", -1),
        ("assigned", "assigned", -1),
        ("demoted", "assigned", -1),
        ("loaned", "assigned", -1),
        ("sent", "assigned", -1),
        // Organization changes remain dated personnel evidence, but do not by
        // themselves prove an NHL active-roster transition.
        ("released", "released", 0),
        ("traded", "traded_away", 0),
        (" for ", "traded_away", 0),
        ("recalled", "recalled", 1),
        ("acquired", "acquired", 0),
        ("claimed", "waiver_claim", 1),
        ("activated", "activated", 0),
        ("reinstated", "activated", 0),
        ("placed", "placed", 0),
        ("waived", "waiver_placement", 0),
    ];
    let Some((_, action, delta)) = markers
        .iter()
        .filter_map(|marker| {
            prefix
                .rfind(marker.0)
                .map(|index| (index, marker.1, marker.2))
        })
        .max_by_key(|marker| marker.0)
    else {
        if contains_name(prefix, "signed") {
            return ("signing_no_change", 0);
        }
        return ("ambiguous", 0);
    };
    if action == "placed" {
        if suffix.contains("injured reserve") {
            return ("ir_placed", 0);
        }
        if suffix.contains("waiver") {
            return ("waiver_placement", 0);
        }
        return ("administrative", 0);
    }
    (action, delta)
}

fn contains_name(description: &str, name: &str) -> bool {
    description.match_indices(name).any(|(start, matched)| {
        let left = description[..start].chars().next_back();
        let right = description[start + matched.len()..].chars().next();
        left.is_none_or(|value| !value.is_alphanumeric())
            && right.is_none_or(|value| !value.is_alphanumeric())
    })
}

fn transaction_availability_delta(description: &str) -> i8 {
    let description = description.to_ascii_lowercase();
    let mentions_ir = description.contains("injured reserve");
    let placement =
        mentions_ir && (description.contains("placed ") || description.contains("transferred "));
    let return_to_roster = mentions_ir
        && (description.contains("activated ")
            || description.contains("reinstated ")
            || description.contains("removed "));
    match (placement, return_to_roster) {
        (true, false) => 1,
        (false, true) => -1,
        _ => 0,
    }
}

fn merge_scenarios(
    authored: Option<TeamSeasonScenario>,
    automatic: TeamSeasonScenario,
) -> TeamSeasonScenario {
    let Some(mut authored) = authored else {
        return automatic;
    };
    authored.name = format!("{} + {}", authored.name, automatic.name);
    authored.trade_deadline = authored.trade_deadline.or(automatic.trade_deadline);
    authored.events.extend(automatic.events);
    authored
        .adaptive_lineup_policies
        .extend(automatic.adaptive_lineup_policies);
    authored
        .opening_roster_policies
        .extend(automatic.opening_roster_policies);
    authored
}

fn force_trade_events(mut scenario: TeamSeasonScenario) -> TeamSeasonScenario {
    scenario.name = format!("{} — trades completed", scenario.name);
    for event in &mut scenario.events {
        if event.kind == TeamSeasonScenarioEventKind::Trade {
            event.occurrence_probability = 1.0;
        }
    }
    scenario
}

fn render(view: &TeamSeasonForecastView, focus: &[String], all_games: bool) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{} — SEASON {}",
        if view
            .games
            .first()
            .is_some_and(|game| game.evidence_cutoff_date.is_some())
        {
            "THE FILM ROOM — ICEREPLAY"
        } else {
            "THE GOAL LINE — ICECAST"
        },
        view.season
    );
    let _ = writeln!(
        out,
        "{} games · {} trials · seed {}",
        view.schedule_games, view.trials, view.seed
    );
    if let Some(checkpoint) = &view.replay_checkpoint {
        let _ = writeln!(
            out,
            "through {} · {} league games final · {} remaining",
            checkpoint.as_of_date,
            checkpoint.league_completed_games,
            checkpoint.league_remaining_games
        );
    }
    for warning in &view.warnings {
        let _ = writeln!(out, "warning: {warning}");
    }
    if let Some(authority) = &view.opening_roster_authority {
        let _ = writeln!(out, "\nTHE CREASE — OPENING ROSTER GATE");
        let _ = writeln!(
            out,
            "{} · {}/{} teams verified · player-value effects {}",
            authority.status,
            authority.verified_teams,
            authority.expected_teams,
            if authority.player_value_effects_enabled {
                "enabled"
            } else {
                "disabled"
            }
        );
        let _ = writeln!(out, "{}", authority.reason);
        if let Some(date) = authority.personnel_events_effective_after {
            let _ = writeln!(
                out,
                "opening roster is authoritative through {date}; only later personnel events can change lineup strength"
            );
        }
        if !view.opening_strengths.is_empty() {
            let average_coverage = view
                .opening_strengths
                .iter()
                .map(|row| row.value_coverage)
                .sum::<f64>()
                / view.opening_strengths.len() as f64;
            let _ = writeln!(
                out,
                "{} team strengths · {:.1}% average prior-value coverage · 55% F / 30% D / 15% G",
                view.opening_strengths.len(),
                average_coverage * 100.0
            );
            for team in focus {
                if let Some(row) = view.opening_strengths.iter().find(|row| row.team == *team) {
                    let _ = writeln!(
                        out,
                        "{team:<5} {:.1} strength · {}/{} prior-valued players",
                        row.strength, row.valued_players, row.roster_players
                    );
                }
            }
        }
    }
    if let Some(scenario) = &view.scenario {
        let trades = scenario
            .events
            .iter()
            .filter(|event| {
                event.kind == TeamSeasonScenarioEventKind::Trade && event.strength_delta > 0.0
            })
            .collect::<Vec<_>>();
        if !trades.is_empty() {
            let _ = writeln!(out, "Plausible trade hypotheses:");
            for trade in trades {
                let _ = writeln!(
                    out,
                    "- {:.0}% · {}",
                    trade.occurrence_probability * 100.0,
                    trade.label
                );
            }
        }
    }
    let _ = writeln!(out, "\nTHE SCOREBOARD — LEAGUE PICKS");
    render_leaders(
        &mut out,
        "Presidents",
        &view.league_leaders.presidents_trophy,
    );
    render_leaders(&mut out, "Stanley Cup", &view.league_leaders.stanley_cup);
    render_leaders(
        &mut out,
        "Longest streak",
        &view.league_leaders.longest_win_streak,
    );
    if let Some(accuracy) = &view.accuracy {
        let _ = writeln!(out, "\nTHE REVIEW — PICK ACCURACY");
        let _ = writeln!(
            out,
            "{} final · {} correct · {:.1}% accuracy · {:.3} Brier ({:+.3} vs coin) · {} pending",
            accuracy.final_games,
            accuracy.correct_picks,
            accuracy.pick_accuracy * 100.0,
            accuracy.brier_score,
            accuracy.brier_skill_vs_coinflip,
            accuracy.pending_games
        );
        let _ = writeln!(
            out,
            "binary log loss {:.3} ({:+.3} vs coin) · three-way log loss {} · calibration error {:.3}",
            accuracy.binary_log_loss,
            accuracy.binary_log_loss_skill_vs_coinflip,
            accuracy
                .multiclass_log_loss
                .map(|loss| format!(
                    "{loss:.3} ({:+.3} vs uniform)",
                    accuracy.multiclass_log_loss_skill_vs_uniform.unwrap_or_default()
                ))
                .unwrap_or_else(|| "unavailable (ending metadata missing)".to_owned()),
            accuracy.expected_calibration_error
        );
        if let (Some(intercept), Some(slope)) =
            (accuracy.calibration_intercept, accuracy.calibration_slope)
        {
            let _ = writeln!(
                out,
                "calibration intercept {intercept:+.3} (ideal 0) · slope {slope:.3} (ideal 1)"
            );
            if let (
                Some(intercept_lower),
                Some(intercept_upper),
                Some(slope_lower),
                Some(slope_upper),
            ) = (
                accuracy.calibration_intercept_ci95_lower,
                accuracy.calibration_intercept_ci95_upper,
                accuracy.calibration_slope_ci95_lower,
                accuracy.calibration_slope_ci95_upper,
            ) {
                let _ = writeln!(
                    out,
                    "calibration 95% intervals: intercept [{intercept_lower:+.3}, {intercept_upper:+.3}] · slope [{slope_lower:.3}, {slope_upper:.3}]"
                );
            }
        }
        for baseline in &accuracy.baselines {
            let _ = writeln!(
                out,
                "vs {:<17} {:>5.1}% picks · {:.3} Brier ({:+.3}) · {:.3} log loss ({:+.3})",
                baseline.name,
                baseline.pick_accuracy * 100.0,
                baseline.brier_score,
                baseline.model_brier_improvement,
                baseline.binary_log_loss,
                baseline.model_log_loss_improvement
            );
        }
        if let Some(best) = &accuracy.best_elo_blend_by_brier {
            let _ = writeln!(
                out,
                "Elo blend sweep historical best: {:>3.0}% Elo · {:>5.1}% picks · {:.3} Brier ({:+.4}) · {:.3} log loss ({:+.4})",
                best.elo_weight * 100.0,
                best.pick_accuracy * 100.0,
                best.brier_score,
                best.brier_improvement_vs_model,
                best.binary_log_loss,
                best.log_loss_improvement_vs_model
            );
        }
        if !accuracy.ablations.is_empty() {
            let _ = writeln!(out, "factor ablations (positive means the factor helped):");
            let mut ablations = accuracy.ablations.iter().collect::<Vec<_>>();
            ablations.sort_by(|a, b| {
                b.model_brier_improvement
                    .total_cmp(&a.model_brier_improvement)
                    .then_with(|| a.factor.cmp(&b.factor))
            });
            for ablation in ablations {
                let _ = writeln!(
                    out,
                    "  {:<24} {:>4} affected · mean |Δp| {:.3} · Brier {:+.4} · log {:+.4}",
                    ablation.factor,
                    ablation.games_affected,
                    ablation.mean_absolute_probability_delta,
                    ablation.model_brier_improvement,
                    ablation.model_log_loss_improvement
                );
            }
        }
        for row in &accuracy.by_confidence {
            let _ = writeln!(
                out,
                "{:<7} {:>4} games · {:>5.1}% correct · {:>5.1}% mean confidence · {:.3} Brier",
                row.segment,
                row.games,
                row.pick_accuracy * 100.0,
                row.mean_favorite_probability * 100.0,
                row.brier_score
            );
        }
    }
    if !view.personnel_evidence.is_empty() {
        let _ = writeln!(out, "\nTHE WIRE — DATED PERSONNEL EVIDENCE");
        let _ = writeln!(
            out,
            "{} sourced events · {} stable player links · {} prior-season valued · {} ambiguous names · {} clear additions / {} removals · {} unambiguous IR placements · {} unambiguous activations",
            view.personnel_evidence.len(),
            view.personnel_evidence
                .iter()
                .map(|event| event.resolved_players.len())
                .sum::<usize>(),
            view.personnel_evidence
                .iter()
                .flat_map(|event| &event.resolved_players)
                .filter(|player| player.prior_value.is_some())
                .count(),
            view.personnel_evidence
                .iter()
                .map(|event| event.ambiguous_player_names.len())
                .sum::<usize>(),
            view.personnel_evidence
                .iter()
                .flat_map(|event| &event.resolved_players)
                .filter(|player| player.membership_delta > 0)
                .count(),
            view.personnel_evidence
                .iter()
                .flat_map(|event| &event.resolved_players)
                .filter(|player| player.membership_delta < 0)
                .count(),
            view.personnel_evidence
                .iter()
                .filter(|event| event.availability_delta > 0)
                .count(),
            view.personnel_evidence
                .iter()
                .filter(|event| event.availability_delta < 0)
                .count(),
        );
        let sourced_intervals = view
            .membership_intervals
            .iter()
            .filter(|interval| interval.confidence == "sourced")
            .count();
        let implied_intervals = view.membership_intervals.len() - sourced_intervals;
        let open_intervals = view
            .membership_intervals
            .iter()
            .filter(|interval| interval.end_event_date.is_none())
            .count();
        let _ = writeln!(
            out,
            "{} membership intervals · {} sourced / {} implied preexisting · {} still open · {} transition conflicts (values are metadata only)",
            view.membership_intervals.len(),
            sourced_intervals,
            implied_intervals,
            open_intervals,
            view.membership_anomalies.len()
        );
        if !view.paired_trades.is_empty() {
            let active_transfers = view
                .paired_trades
                .iter()
                .filter(|trade| trade.active_lineup_applied)
                .count();
            let _ = writeln!(
                out,
                "{} exact paired trades · {} active-lineup transfers · {} organizational only",
                view.paired_trades.len(),
                active_transfers,
                view.paired_trades.len() - active_transfers
            );
        }
        for team in focus {
            let events = view
                .personnel_evidence
                .iter()
                .filter(|event| event.team == *team)
                .collect::<Vec<_>>();
            if let Some(latest) = events.last() {
                let _ = writeln!(
                    out,
                    "{team:<5} {:>4} events · latest {} · {}",
                    events.len(),
                    latest.date,
                    latest.label
                );
            }
        }
    }
    if let Some(checkpoint) = &view.replay_checkpoint {
        let _ = writeln!(
            out,
            "\nTHE CHECKPOINT — ACTUAL THROUGH {}",
            checkpoint.as_of_date
        );
        let _ = writeln!(
            out,
            "{:<5} {:>4} {:>4} {:>4} {:>4} {:>6} {:>5}",
            "Team", "GP", "W", "L", "OTL", "Points", "Left"
        );
        for team in focus {
            if let Some(row) = checkpoint.teams.iter().find(|row| row.team == *team) {
                let _ = writeln!(
                    out,
                    "{:<5} {:>4} {:>4} {:>4} {:>4} {:>6} {:>5}",
                    row.team,
                    row.completed_games,
                    row.wins,
                    row.losses,
                    row.overtime_losses,
                    row.standings_points,
                    row.remaining_games
                );
            }
        }
        let _ = writeln!(out, "THE REST OF THE WAY — EXPECTED");
        let _ = writeln!(
            out,
            "{:<5} {:>5} {:>5} {:>5} {:>7}",
            "Team", "W", "L", "OTL", "Points"
        );
        for team in focus {
            if let Some(row) = checkpoint.teams.iter().find(|row| row.team == *team) {
                let _ = writeln!(
                    out,
                    "{:<5} {:>5.1} {:>5.1} {:>5.1} {:>7.1}",
                    row.team,
                    row.expected_remaining_wins,
                    row.expected_remaining_losses,
                    row.expected_remaining_overtime_losses,
                    row.expected_remaining_points
                );
            }
        }
    }
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "{:<5} {:>5} {:>5} {:>5} {:>7} {:>11} {:>7} {:>7} {:>7}",
        "Team", "W", "L", "OTL", "Points", "P10-P90", "PO%", "Cup%", "LWS"
    );
    for team in focus {
        if let Some(row) = view.teams.iter().find(|row| row.team == *team) {
            let _ = writeln!(
                out,
                "{:<5} {:>5.1} {:>5.1} {:>5.1} {:>7.1} {:>4}-{:>4} {:>6.1}% {:>6.1}% {:>7.1}",
                row.team,
                row.average_wins,
                row.average_losses,
                row.average_overtime_losses,
                row.average_points,
                row.points_p10,
                row.points_p90,
                row.playoff_probability * 100.0,
                row.stanley_cup_probability * 100.0,
                row.average_longest_win_streak
            );
        }
    }
    for team in focus {
        let rows = view
            .schedule_stretches
            .iter()
            .filter(|row| row.team == *team)
            .collect::<Vec<_>>();
        if !rows.is_empty() {
            let _ = writeln!(out, "\nTHE GAUNTLET — {team} FIVE-GAME WINDOWS");
            for row in rows {
                let label = match row.kind {
                    TeamSeasonStretchKind::Hardest => "Hardest",
                    TeamSeasonStretchKind::Easiest => "Easiest",
                };
                let _ = writeln!(
                    out,
                    "{label:<7} {} to {} · {:.2} expected wins · {:.1}% avg · {} away · {} B2B · {:.0} km · {}",
                    row.start_date,
                    row.end_date,
                    row.expected_wins,
                    row.average_win_probability * 100.0,
                    row.away_games,
                    row.back_to_backs,
                    row.travel_km,
                    row.opponents.join(", ")
                );
            }
        }
    }
    let _ = writeln!(out, "\nPlayoff path:");
    let _ = writeln!(
        out,
        "{:<5} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "Team", "Playoffs", "Round 2", "Conf F", "Cup F", "Cup"
    );
    for team in focus {
        if let Some(row) = view.teams.iter().find(|row| row.team == *team) {
            let _ = writeln!(
                out,
                "{:<5} {:>7.1}% {:>7.1}% {:>7.1}% {:>7.1}% {:>7.1}%",
                row.team,
                row.playoff_probability * 100.0,
                row.second_round_probability * 100.0,
                row.conference_final_probability * 100.0,
                row.stanley_cup_final_probability * 100.0,
                row.stanley_cup_probability * 100.0
            );
        }
    }
    for team in focus {
        let pivotal = view
            .pivotal_games
            .iter()
            .filter(|game| game.home_team == *team || game.away_team == *team)
            .take(5)
            .collect::<Vec<_>>();
        if !pivotal.is_empty() {
            let _ = writeln!(out, "\nTHE BUBBLE — {team} PIVOTAL GAMES");
            let _ = writeln!(
                out,
                "{:<10} {:<9} {:>8} {:>9}",
                "Date", "Matchup", "Hunt%", "Spoiler%"
            );
            for game in pivotal {
                let _ = writeln!(
                    out,
                    "{:<10} {:<9} {:>7.1}% {:>8.1}%",
                    game.date,
                    format!("{}@{}", game.away_team, game.home_team),
                    game.hunt_probability * 100.0,
                    game.spoiler_probability * 100.0
                );
            }
        }
    }
    if !view.scenario_impacts.is_empty() {
        if view.conditional_scenario_impacts.is_empty() {
            let _ = writeln!(out, "\nPaired scenario impact:");
            let _ = writeln!(
                out,
                "{:<5} {:>10} {:>10} {:>10}",
                "Team", "ΔPoints", "ΔPlayoffs", "ΔCup"
            );
            for team in focus {
                if let Some(row) = view.scenario_impacts.iter().find(|row| row.team == *team) {
                    let _ = writeln!(
                        out,
                        "{:<5} {:>+10.2} {:>+9.2}pp {:>+9.2}pp",
                        row.team,
                        row.average_points_delta,
                        row.playoff_probability_delta * 100.0,
                        row.stanley_cup_probability_delta * 100.0
                    );
                }
            }
        } else {
            let _ = writeln!(out, "\nTrade-only paired impact:");
            let _ = writeln!(
                out,
                "{:<5} {:>10} {:>10} {:>11} {:>11} {:>12}",
                "Team", "Mkt ΔPts", "Mkt ΔPO", "Done ΔPts", "Done ΔPO", "Done ΔCup"
            );
            for team in focus {
                if let Some(row) = view.scenario_impacts.iter().find(|row| row.team == *team) {
                    let completed = view
                        .conditional_scenario_impacts
                        .iter()
                        .find(|impact| impact.team == *team);
                    let _ = writeln!(
                        out,
                        "{:<5} {:>+10.2} {:>+9.2}pp {:>+11.2} {:>+10.2}pp {:>+11.2}pp",
                        row.team,
                        row.average_points_delta,
                        row.playoff_probability_delta * 100.0,
                        completed.map_or(0.0, |impact| impact.average_points_delta),
                        completed.map_or(0.0, |impact| impact.playoff_probability_delta * 100.0),
                        completed
                            .map_or(0.0, |impact| impact.stanley_cup_probability_delta * 100.0)
                    );
                }
            }
        }
    }
    if !view.scenario_outcomes.is_empty() {
        let _ = writeln!(out, "\nScenario realization buckets (minimum 1%):");
        let _ = writeln!(
            out,
            "{:<5} {:>4} {:>4} {:>8} {:>8} {:>8} {:>10} {:>8}",
            "Team", "+Ev", "-Ev", "Chance", "Avg Δ", "Points", "Playoffs", "Cup"
        );
        for team in focus {
            for row in view
                .scenario_outcomes
                .iter()
                .filter(|row| row.team == *team && row.probability >= 0.01)
            {
                let _ = writeln!(
                    out,
                    "{:<5} {:>4} {:>4} {:>7.1}% {:>+8.2} {:>8.2} {:>9.1}% {:>7.1}%",
                    row.team,
                    row.positive_events,
                    row.negative_events,
                    row.probability * 100.0,
                    row.average_sampled_strength_delta,
                    row.average_points,
                    row.playoff_probability * 100.0,
                    row.stanley_cup_probability * 100.0
                );
            }
        }
    }
    if all_games {
        for team in focus {
            let _ = writeln!(out, "\n{team} — ALL GAMES");
            let _ = writeln!(
                out,
                "{:<10} {:<4} {:<4} {:>6} {:<7} {:>5} {:>5} {:>3} {:<9} {:<4} Why",
                "Date", "H/A", "Opp", "Win%", "Pick", "Known", "Moves", "IR", "Actual", "Hit"
            );
            for game in view
                .games
                .iter()
                .filter(|game| game.home_team == *team || game.away_team == *team)
            {
                render_game(&mut out, game, team);
            }
        }
    } else {
        let _ = writeln!(
            out,
            "\nUse --all-games to print every focused-team prediction and explanation."
        );
    }
    let _ = writeln!(out);
    for disclosure in &view.disclosures {
        let _ = writeln!(out, "- {disclosure}");
    }
    out
}

fn render_leaders(
    out: &mut String,
    label: &str,
    rows: &[icelines_core::TeamSeasonProbabilityLeaderRow],
) {
    let values = rows
        .iter()
        .map(|row| format!("{} {:.1}%", row.team, row.probability * 100.0))
        .collect::<Vec<_>>()
        .join(" · ");
    let _ = writeln!(out, "{label:<15} {values}");
}

fn render_game(out: &mut String, game: &TeamGameForecastRow, team: &str) {
    let home = game.home_team == team;
    let opponent = if home {
        &game.away_team
    } else {
        &game.home_team
    };
    let win_probability = if home {
        game.home_overall_win_probability
    } else {
        game.away_overall_win_probability
    };
    let reasons = game
        .factors
        .iter()
        .map(|factor| {
            let delta = if home {
                factor.home_win_probability_delta
            } else {
                -factor.home_win_probability_delta
            };
            (
                delta.abs(),
                format!("{:+.1} {}", delta * 100.0, factor.label),
            )
        })
        .collect::<Vec<_>>();
    let mut reasons = reasons;
    reasons.sort_by(|a, b| b.0.total_cmp(&a.0));
    let why = reasons
        .into_iter()
        .take(3)
        .map(|(_, text)| text)
        .collect::<Vec<_>>()
        .join("; ");
    let _ = writeln!(
        out,
        "{:<10} {:<4} {:<4} {:>5.1}% {:<7} {:>5} {:>5} {:>3} {:<9} {:<4} {}",
        game.date,
        if home { "HOME" } else { "AWAY" },
        opponent,
        win_probability * 100.0,
        game.favored_team,
        if home {
            game.home_evidence_games
        } else {
            game.away_evidence_games
        },
        if home {
            game.home_known_personnel_events
        } else {
            game.away_known_personnel_events
        },
        if home {
            game.home_active_ir_signals
        } else {
            game.away_active_ir_signals
        },
        match (game.actual_away_score, game.actual_home_score) {
            (Some(away), Some(home)) => format!(
                "{away}-{home} {}",
                game.actual_ending.as_deref().unwrap_or("")
            ),
            _ => "—".to_owned(),
        },
        match game.pick_correct {
            Some(true) => "YES",
            Some(false) => "NO",
            None => "—",
        },
        why
    );
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use chrono::NaiveDate;
    use icelines_core::{
        build_prospect_development_study, simulate_training_camp, OrganizationWindowBoardView,
        Position, ProspectArrivalLeagueCalibrationView,
        ProspectArrivalLeaguePopulationAuthorityView, ProspectAvailabilityStatus,
        ProspectDevelopmentSeasonInput, ProspectDevelopmentStudyConfig,
        ProspectDevelopmentStudyInput, ProspectOpportunityStatus, TeamCeilingLens,
        TeamCeilingPlayerRow, TeamForecastPersonnelPlayerInput, TeamSeasonAdaptiveLineupChoice,
        TeamSeasonAdaptiveLineupPolicy, TeamSeasonForecastHistoryCheckpointRow,
        TeamSeasonForecastHistoryMateriality, TeamSeasonForecastHistoryMoverRow,
        TeamSeasonForecastHistoryPointRow, TeamSeasonForecastHistoryTeamRow,
        TeamSeasonForecastHistoryTrend, TeamSeasonForecastHistoryView,
        TeamSeasonForecastMovementRow, TeamSeasonForecastMovementView, TeamSeasonForecastView,
        TeamSeasonReplayCheckpointTeamRow, TeamSeasonReplayCheckpointView,
        TrainingCampLeagueForecastView, TrainingCampSimulationInput,
    };
    use icelines_fetch::ahl::{
        build_ahl_identity_review_inspection, AhlIdentityCrosswalkCounts, AhlIdentityCrosswalkRow,
        AhlIdentityCrosswalkView, AhlIdentityInspectionScope, AhlIdentityMatchBasis,
        AhlIdentityReviewStatus, AHL_IDENTITY_CROSSWALK_SCHEMA,
    };
    use icelines_fetch::ahl_rollover::{
        AhlPreseasonOrganizationReview, AhlPreseasonOrganizationReviewRow,
        AHL_PRESEASON_ORGANIZATION_REVIEW_SCHEMA,
    };
    use icelines_fetch::{
        ControlledProspectStudySelection,
        ProspectPopulationRelationship as LeagueCampCandidateRelationship,
        ProspectStudyControlExclusion, ProspectStudyControlExclusionReason,
    };

    fn arrival_test_study(player_id: u32) -> icelines_core::ProspectDevelopmentStudyView {
        build_prospect_development_study(
            ProspectDevelopmentStudyInput {
                player_id,
                player: format!("Prospect {player_id}"),
                organization: "SEA".to_owned(),
                position: "C".to_owned(),
                age: 20,
                nhl_games_played: 0,
                seasons: vec![ProspectDevelopmentSeasonInput {
                    season: 20_252_026,
                    league: "AHL".to_owned(),
                    games_played: 30,
                    goals: 10,
                    assists: 15,
                }],
                opportunity: ProspectOpportunityStatus::None,
                availability: ProspectAvailabilityStatus::Unknown,
                attention_score: 0.5,
                attention_basis: "Neutral test context".to_owned(),
                evidence: vec![],
            },
            ProspectDevelopmentStudyConfig::default(),
        )
        .unwrap()
    }

    #[test]
    fn prospect_arrival_population_selection_refuses_incomplete_strict_authority() {
        let error = super::apply_prospect_arrival_population_selection(
            ControlledProspectStudySelection {
                source_package_fingerprint: "a".repeat(64),
                population_complete: false,
                supplied_studies: 0,
                studies: vec![],
                goalie_studies: vec![],
                exclusions: vec![],
            },
            &std::collections::BTreeSet::new(),
            0,
            true,
        )
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("population source package is incomplete"));
    }

    #[test]
    fn prospect_arrival_population_selection_reconciles_only_skater_exclusions() {
        let selection = ControlledProspectStudySelection {
            source_package_fingerprint: "b".repeat(64),
            population_complete: false,
            supplied_studies: 3,
            studies: vec![arrival_test_study(43)],
            goalie_studies: vec![],
            exclusions: vec![
                ProspectStudyControlExclusion {
                    player_id: 42,
                    player: "Excluded Skater".to_owned(),
                    supplied_organization: "SEA".to_owned(),
                    controlled_organization: None,
                    reason: ProspectStudyControlExclusionReason::UnsupportedControl,
                    detail: "no current contract or rights fact".to_owned(),
                },
                ProspectStudyControlExclusion {
                    player_id: 99,
                    player: "Excluded Goalie".to_owned(),
                    supplied_organization: "SEA".to_owned(),
                    controlled_organization: None,
                    reason: ProspectStudyControlExclusionReason::UnsupportedControl,
                    detail: "no current contract or rights fact".to_owned(),
                },
            ],
        };

        let (studies, exclusions, authority) = super::apply_prospect_arrival_population_selection(
            selection,
            &std::collections::BTreeSet::from([42, 43]),
            2,
            false,
        )
        .unwrap();
        let authority = authority.unwrap();

        assert_eq!(studies.len(), 1);
        assert_eq!(exclusions.len(), 1);
        assert_eq!(exclusions[0].player_id, 42);
        assert!(!authority.population_complete);
        assert_eq!(authority.supplied_studies, 2);
        assert_eq!(authority.controlled_studies, 1);
        assert_eq!(authority.control_exclusions, 1);
    }

    #[test]
    fn prospect_arrival_text_distinguishes_authority_from_coverage_draft() {
        let mut view = ProspectArrivalLeagueCalibrationView {
            schema: "prospect_arrival_league_calibration.v1".to_owned(),
            source_schema: "prospect_conversion_board.v1".to_owned(),
            forecast_season: 20_262_027,
            organizations_requested: 32,
            organizations_represented: 32,
            target_skaters: 2,
            calibrated_skaters: 1,
            excluded_skaters: 1,
            population_authority: Some(ProspectArrivalLeaguePopulationAuthorityView {
                source_package_fingerprint: "c".repeat(64),
                population_complete: false,
                supplied_studies: 2,
                controlled_studies: 1,
                control_exclusions: 1,
            }),
            teams: vec![],
            disclosures: vec![],
        };

        let sourced = super::render_prospect_arrival_league(&view);
        assert!(sourced.contains("Population authority · incomplete"));
        assert!(sourced.contains("1/2 controlled skaters · 1 control exclusions"));
        assert!(sourced.contains(&"c".repeat(64)));

        view.population_authority = None;
        let draft = super::render_prospect_arrival_league(&view);
        assert!(draft.contains("Population authority · not supplied · coverage draft only"));
    }

    #[test]
    fn preseason_opening_players_keep_ranked_12f_6d_2g_evidence() {
        let mut players = Vec::new();
        for (position, count, base_id) in [
            (Position::Center, 13_u32, 100_u32),
            (Position::Defense, 7, 200),
            (Position::Goalie, 3, 300),
        ] {
            players.extend((0..count).map(|index| TeamCeilingPlayerRow {
                player_id: base_id + index,
                player: format!("Player {}", base_id + index),
                position,
                age: 25,
                games_played: 82,
                prior_team: None,
                newcomer: false,
                has_nhl_sample: true,
                lens_scores: BTreeMap::from([(
                    TeamCeilingLens::PointsPace,
                    Some(100.0 - f64::from(index)),
                )]),
            }));
        }
        let rows = super::preseason_opening_players(&players);
        for (group, expected) in [("forward", 12), ("defense", 6), ("goalie", 2)] {
            assert_eq!(
                rows.iter()
                    .filter(|row| row.position_group == group && row.selected_at_opening)
                    .count(),
                expected
            );
        }
        for excluded in [112, 206, 302] {
            assert!(
                !rows
                    .iter()
                    .find(|row| row.player_id == excluded)
                    .unwrap()
                    .selected_at_opening
            );
        }
    }

    #[test]
    fn preseason_opening_players_shrink_low_gp_goalie_and_partial_lens_samples() {
        let row = |player_id,
                   player: &str,
                   position,
                   games_played,
                   scores: BTreeMap<TeamCeilingLens, Option<f64>>| {
            TeamCeilingPlayerRow {
                player_id,
                player: player.to_owned(),
                position,
                age: 25,
                games_played,
                prior_team: Some("NYR".to_owned()),
                newcomer: false,
                has_nhl_sample: games_played > 0,
                lens_scores: scores,
            }
        };
        let one_score = |value| BTreeMap::from([(TeamCeilingLens::PointsPace, Some(value))]);
        let mut players = vec![
            row(1, "Dylan Garand", Position::Goalie, 3, one_score(98.66)),
            row(2, "Igor Shesterkin", Position::Goalie, 51, one_score(74.07)),
            row(
                3,
                "Joonas Korpisalo",
                Position::Goalie,
                31,
                one_score(51.99),
            ),
            row(
                10,
                "Drew Fortescue",
                Position::Defense,
                9,
                BTreeMap::from([
                    (TeamCeilingLens::PointsPace, None),
                    (TeamCeilingLens::Fantasy, Some(95.67)),
                ]),
            ),
            row(
                11,
                "Vladislav Gavrikov",
                Position::Defense,
                82,
                BTreeMap::from([
                    (TeamCeilingLens::PointsPace, Some(35.0)),
                    (TeamCeilingLens::GoalScoring, Some(14.0)),
                    (TeamCeilingLens::Fantasy, Some(89.5)),
                    (TeamCeilingLens::Upside, Some(31.01)),
                ]),
            ),
        ];
        players.extend((0..5).map(|index| {
            row(
                20 + index,
                "Established defenseman",
                Position::Defense,
                82,
                one_score(60.0 - f64::from(index)),
            )
        }));

        let rows = super::preseason_opening_players(&players);
        let selected = |player_id| {
            rows.iter()
                .find(|candidate| candidate.player_id == player_id)
                .unwrap()
                .selected_at_opening
        };

        assert!(!selected(1), "three-game goalie heater must be regressed");
        assert!(selected(2));
        assert!(selected(3));
        assert!(!selected(10), "partial nine-game lens must be regressed");
        assert!(selected(11));
    }

    #[test]
    fn window_package_refuses_partial_league_affiliate_inputs() {
        let league = super::AhlPreseasonLeagueProjectionInputsView {
            schema: super::AHL_PRESEASON_LEAGUE_PROJECTION_INPUTS_SCHEMA.to_owned(),
            target_season: 20262027,
            facts_application_fingerprint: "sha256:test".to_owned(),
            teams_requested: 32,
            teams_built: 0,
            inputs: Vec::new(),
            failures: Vec::new(),
            disclosures: Vec::new(),
        };

        let error = super::build_window_affiliates_from_league_inputs(&league, 20262027)
            .expect_err("partial league input must fail closed");
        assert!(error.to_string().contains("must be complete"));
    }

    #[test]
    fn affiliate_facts_ready_gate_names_exact_blockers() {
        let workboard = super::AhlPreseasonLeagueFactsWorkboardView {
            schema:
                icelines_fetch::ahl_preseason_facts::AHL_PRESEASON_LEAGUE_FACTS_WORKBOARD_SCHEMA
                    .to_owned(),
            prior_season: 20252026,
            target_season: 20262027,
            professional_game_policy_id: "policy.v1".to_owned(),
            professional_game_policy_authority: "provisional".to_owned(),
            professional_game_threshold: 260,
            source_fingerprint: "sha256:test".to_owned(),
            teams: 32,
            candidates: 3,
            facts_ready_candidates: 0,
            blocker_counts: BTreeMap::from([(
                icelines_fetch::ahl_preseason_facts::AhlPreseasonFactBlocker::AssignmentAuthority,
                3,
            )]),
            team_workboards: Vec::new(),
            disclosures: Vec::new(),
        };

        let error = super::require_affiliate_facts_ready(&workboard)
            .expect_err("blocked workboard must fail the ready gate");
        assert!(error.to_string().contains("AssignmentAuthority=3"));
    }

    #[test]
    fn window_markdown_report_preserves_sealed_context_and_partial_state() {
        let board: OrganizationWindowBoardView = serde_json::from_str(include_str!(
            "../../../examples/organization-window-board-partial-2026-07-28.json"
        ))
        .unwrap();
        let report = super::render_window_markdown(&board, Some("NYR"));
        assert!(report.contains("schema: organization_window_report.v1"));
        assert!(report.contains(&format!("board_fingerprint: {}", board.fingerprint)));
        assert!(report.contains("Frozen 32-organization cohort"));
        assert!(report.contains("| NR | NYR |"));
        assert!(report.contains("## NYR detail"));
        assert!(report.contains("### Lines and evidence"));
        assert!(report.contains("### Blockers"));
        assert!(report.contains("| Under review | Withheld |"));
        assert!(!report.contains("| Plateau | Withheld |"));
        assert!(!report.contains("| SEA |"));

        let terminal = super::render_window(&board, Some("NYR"));
        assert!(terminal.contains("coverage  85%  Under review"));
        assert!(!terminal.contains("coverage  85%  Plateau"));
    }

    #[test]
    fn affiliate_identity_renderer_recomputes_counts_and_shows_evidence() {
        let view = AhlIdentityCrosswalkView {
            schema: AHL_IDENTITY_CROSSWALK_SCHEMA.to_owned(),
            season: 2025_2026,
            provider: "ahl-api".to_owned(),
            ahl_team: "Hartford Wolf Pack".to_owned(),
            nhl_affiliate: Some("NYR".to_owned()),
            roster_fetched_at: "2026-07-24T12:00:00Z".to_owned(),
            candidates_checked_at: "2026-07-24".to_owned(),
            counts: AhlIdentityCrosswalkCounts {
                roster_players: 0,
                exact_name_and_birth_date: 0,
                surname_and_birth_date: 0,
                exact_name_only: 0,
                ambiguous: 0,
                conflicts: 0,
                unmatched: 0,
                reviewed: 0,
            },
            rows: vec![AhlIdentityCrosswalkRow {
                provider_player_id: "provider-1".to_owned(),
                ahl_display_name: "Exact Player".to_owned(),
                ahl_birth_date: "2001-01-01".to_owned(),
                match_basis: AhlIdentityMatchBasis::ExactNameAndBirthDate,
                review_status: AhlIdentityReviewStatus::Pending,
                nhl_player_id: Some(8_480_001),
                nhl_display_name: Some("Exact Player".to_owned()),
                nhl_birth_date: Some("2001-01-01".to_owned()),
                evidence_urls: vec!["https://example.test/player".to_owned()],
                note: "Exact official match; review required.".to_owned(),
            }],
            disclosures: vec!["Official discovery remains pending.".to_owned()],
        };

        let inspection =
            build_ahl_identity_review_inspection(&view, AhlIdentityInspectionScope::All).unwrap();
        let rendered = super::render_affiliate_identities(&inspection);
        assert!(rendered.contains("1 roster | 1 exact name+birth | 0 surname+birth"));
        assert!(rendered.contains("WARNING: declared identity counts are stale"));
        assert!(rendered.contains("1 source(s)"));
        assert!(rendered.contains("DISCLOSURES"));
        let inspection =
            build_ahl_identity_review_inspection(&view, AhlIdentityInspectionScope::Attention)
                .unwrap();
        let attention = super::render_affiliate_identities(&inspection);
        assert!(attention.contains("ATTENTION: 0 non-routine row(s)"));
        assert!(!attention.contains("Exact Player"));

        let mut alias = view;
        alias.rows[0].ahl_display_name = "E. Player".to_owned();
        alias.rows[0].match_basis = AhlIdentityMatchBasis::SurnameAndBirthDate;
        let inspection =
            build_ahl_identity_review_inspection(&alias, AhlIdentityInspectionScope::Attention)
                .unwrap();
        let attention = super::render_affiliate_identities(&inspection);
        assert!(attention.contains("8480001 Exact Player"));
        assert!(attention.contains("BIRTH  AHL 2001-01-01 | NHL 2001-01-01"));
        assert!(attention.contains("SOURCE https://example.test/player"));
    }

    #[test]
    fn affiliate_status_review_renderer_recomputes_stale_counts() {
        let view = AhlPreseasonOrganizationReview {
            schema: AHL_PRESEASON_ORGANIZATION_REVIEW_SCHEMA.to_owned(),
            prior_season: 2025_2026,
            target_season: 2026_2027,
            nhl_team: "NYR".to_owned(),
            ahl_team: "Hartford Wolf Pack".to_owned(),
            provider: "ahl-api".to_owned(),
            roster_fetched_at: "2026-07-24T12:00:00Z".to_owned(),
            crosswalk_fingerprint: "sha256:test".to_owned(),
            draft: true,
            reviewer: None,
            reviewed_at: None,
            identity_blockers: 0,
            decisions_required: 0,
            rows: vec![AhlPreseasonOrganizationReviewRow {
                provider_player_id: "provider-1".to_owned(),
                display_name: "Pending Player".to_owned(),
                nhl_player_id: None,
                identity_reviewed: false,
                in_current_camp: None,
                decision_kind: None,
                evidence_urls: Vec::new(),
                note: String::new(),
            }],
        };

        let rendered = super::render_affiliate_status_review(&view);
        assert!(rendered.contains("1 identity blockers"));
        assert!(rendered.contains("WARNING: declared counts are stale"));
        assert!(rendered.contains("Pending Player"));
        assert!(rendered.contains("IDENTITY"));
    }

    #[test]
    fn merge_scenarios_preserves_adaptive_lineup_policies() {
        let policy = |team: &str| TeamSeasonAdaptiveLineupPolicy {
            team: team.to_owned(),
            review_games: 6,
            minimum_points_percentage: 0.5,
            max_changes: 0,
            choices: vec![TeamSeasonAdaptiveLineupChoice {
                id: "baseline".to_owned(),
                label: "Submitted lineup".to_owned(),
                strength_delta: 0.0,
            }],
        };
        let authored = TeamSeasonScenario {
            name: "authored".to_owned(),
            trade_deadline: None,
            events: Vec::new(),
            adaptive_lineup_policies: vec![policy("NYR")],
            opening_roster_policies: Vec::new(),
        };
        let automatic = TeamSeasonScenario {
            name: "automatic".to_owned(),
            trade_deadline: None,
            events: Vec::new(),
            adaptive_lineup_policies: vec![policy("SEA")],
            opening_roster_policies: Vec::new(),
        };
        let merged = merge_scenarios(Some(authored), automatic);
        assert_eq!(merged.adaptive_lineup_policies.len(), 2);
        assert_eq!(merged.adaptive_lineup_policies[0].team, "NYR");
        assert_eq!(merged.adaptive_lineup_policies[1].team, "SEA");
    }
    use icelines_fetch::{
        nhl_api::ScheduledGame,
        snapshot::{OfficialNhlRosterCapture, SnapshotEntry, SnapshotStore, SnapshotTier},
    };
    use tempfile::tempdir;

    use super::{
        audit_opening_roster_authority, automatic_camp_player, camp_invite_pool_missing,
        merge_scenarios, opening_selected_indices, parse_archived_roster_payload,
        parse_official_nhl_archive_url, parse_retrospective_opening_lineup,
        personnel_player_action, prior_season_player_value, remove_overlay_ranking_exclusions,
        render, render_camp, render_history, render_movement, resolve_archive_discovery_response,
        resolve_transaction_players, run_prospect_population_audit, schedule_calendar_fingerprint,
        score_opening_position_group, select_latest_cdx_capture, select_opening_roster_snapshot,
        snapshot_evidence_at_text, transaction_availability_delta,
        validate_league_camp_candidate_overlay, validate_opening_roster_archive_manifest,
        FeatureMoments, LeagueCampCandidateOverlay, LeagueRosterIdentity,
        OfficialNhlRosterCaptureManifest, OpeningRosterArchiveCapture,
        OpeningRosterArchiveManifest, TeamSeasonScenario, OFFICIAL_NHL_LIVE_ROSTER_MANIFEST_FILE,
        OFFICIAL_NHL_LIVE_ROSTER_SCHEMA, OFFICIAL_NHL_LIVE_ROSTER_SOURCE,
        OPENING_ROSTER_ARCHIVE_SCHEMA, OPENING_ROSTER_ARCHIVE_SOURCE,
    };

    #[test]
    fn league_camp_candidate_overlay_is_sourced_and_rejects_duplicates() {
        let overlay: LeagueCampCandidateOverlay = serde_json::from_str(include_str!(
            "../../../examples/icecast-league-candidate-overlay-2026-27.json"
        ))
        .unwrap();
        validate_league_camp_candidate_overlay(&overlay).unwrap();
        let audit = overlay.audit().unwrap();
        assert!(audit.fully_classified);
        assert_eq!(audit.legacy_relationships, 0);
        assert_eq!(audit.unknown_relationships, 0);
        assert_eq!(audit.ranking_eligible, 10);
        assert_eq!(audit.camp_only, 1);
        assert!(overlay
            .candidates
            .iter()
            .any(|candidate| candidate.display_name == "Luca Cagnoni"
                && candidate.team == "SJS"
                && candidate.relationship.supports_prospect_ranking()));
        assert_eq!(
            overlay
                .candidates
                .iter()
                .find(|candidate| candidate.display_name == "Reid Schaefer")
                .unwrap()
                .relationship,
            LeagueCampCandidateRelationship::DevelopmentCampParticipant
        );

        let mut attendee = overlay.clone();
        attendee.candidates[0].relationship =
            LeagueCampCandidateRelationship::DevelopmentCampParticipant;
        validate_league_camp_candidate_overlay(&attendee).unwrap();
        assert!(!attendee.candidates[0]
            .relationship
            .supports_prospect_ranking());

        let mut controlled = overlay.clone();
        controlled.candidates[0].relationship = LeagueCampCandidateRelationship::OrganizationRights;
        assert!(controlled.candidates[0]
            .relationship
            .supports_prospect_ranking());

        let mut duplicate = overlay.clone();
        duplicate.candidates.push(duplicate.candidates[0].clone());
        assert!(validate_league_camp_candidate_overlay(&duplicate)
            .unwrap_err()
            .contains("duplicate player"));

        let mut unsourced = overlay;
        unsourced.candidates[0].source_url = "red-wings-camp".to_owned();
        assert!(validate_league_camp_candidate_overlay(&unsourced)
            .unwrap_err()
            .contains("absolute http(s)"));
    }

    #[test]
    fn prospect_population_strict_gate_fails_before_writing_partial_authority() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("legacy-population.json");
        let output = dir.path().join("audit.json");
        std::fs::write(
            &input,
            r#"{"schema":"prospect_population_overlay.v1","checked_at":"2026-07-31","candidates":[{"player_id":1,"display_name":"Legacy Candidate","team":"SEA","position":"C","birth_date":"2005-01-01","source_url":"https://www.nhl.com/kraken"}]}"#,
        )
        .unwrap();

        let error = run_prospect_population_audit(input, true, true, Some(output.clone()))
            .unwrap_err()
            .to_string();
        assert!(error.contains("1 legacy relationship(s)"));
        assert!(!output.exists());
    }

    #[test]
    fn camp_only_relationship_is_removed_from_prospect_context_forecast() {
        let mut forecast: TrainingCampLeagueForecastView = serde_json::from_str(include_str!(
            "../../../examples/icecast-league-training-camp-2026-27.json"
        ))
        .unwrap();
        let player_id = forecast
            .teams
            .iter()
            .filter_map(|team| team.forecast.as_ref())
            .flat_map(|team| &team.players)
            .next()
            .unwrap()
            .player_id;
        let exclusions = BTreeMap::from([(
            player_id,
            ("Camp Invite".to_owned(), "free_agent_invite".to_owned()),
        )]);

        assert_eq!(
            remove_overlay_ranking_exclusions(&mut forecast, &exclusions),
            1
        );
        assert!(forecast
            .teams
            .iter()
            .filter_map(|team| team.forecast.as_ref())
            .flat_map(|team| &team.players)
            .all(|player| player.player_id != player_id));
    }

    #[test]
    fn automatic_league_camp_uses_age_not_low_gp_for_prospect_status() {
        let veteran = LeagueRosterIdentity {
            player_id: 1,
            full_name: "Depth Veteran".to_owned(),
            nhl_team: "BOS".to_owned(),
            position: "C".to_owned(),
            birth_date: Some("1990-01-01".to_owned()),
        };
        let prospect = LeagueRosterIdentity {
            player_id: 2,
            full_name: "Young Prospect".to_owned(),
            nhl_team: "BOS".to_owned(),
            position: "C".to_owned(),
            birth_date: Some("2005-01-01".to_owned()),
        };

        let veteran = automatic_camp_player(&veteran, false, None, None, None).unwrap();
        let prospect = automatic_camp_player(&prospect, false, None, None, None).unwrap();
        assert!(!veteran.prospect);
        assert!(prospect.prospect && prospect.rookie_eligible);
        assert!(veteran.source_league.contains("fallback"));
        assert_eq!(camp_invite_pool_missing(&[]), (17, 9, 3));
    }

    #[test]
    fn camp_text_distinguishes_active_dressed_scratch_and_cap_no_read() {
        let input: TrainingCampSimulationInput = serde_json::from_str(include_str!(
            "../../../examples/icecast-sea-training-camp.json"
        ))
        .unwrap();
        let view = simulate_training_camp(&input).unwrap();
        let text = render_camp(&view);

        assert!(text.contains("ACTIVE   DRESS  SCRATCH  WAIVER"));
        assert!(text.contains("CAP: NO READ"));
        assert!(text.contains("Ben Meyers"));
    }

    #[test]
    fn as_of_text_report_exposes_observed_and_remaining_games() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/icecast-2024-25-replay-1000-result.json"
        );
        let mut view: TeamSeasonForecastView =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let cutoff = NaiveDate::from_ymd_opt(2025, 1, 31).unwrap();
        view.as_of_date = Some(cutoff);
        view.replay_checkpoint = Some(TeamSeasonReplayCheckpointView {
            as_of_date: cutoff,
            league_completed_games: 800,
            league_remaining_games: 512,
            teams: vec![TeamSeasonReplayCheckpointTeamRow {
                team: "NYR".to_string(),
                completed_games: 50,
                remaining_games: 32,
                wins: 24,
                losses: 20,
                overtime_losses: 6,
                standings_points: 54,
                expected_remaining_wins: 16.5,
                expected_remaining_losses: 12.0,
                expected_remaining_overtime_losses: 3.5,
                expected_remaining_points: 36.5,
            }],
        });

        let text = render(&view, &["NYR".to_string()], false);
        assert!(text.contains("through 2025-01-31 · 800 league games final · 512 remaining"));
        assert!(text.contains("THE CHECKPOINT — ACTUAL THROUGH 2025-01-31"));
        assert!(text.contains("NYR     50   24   20    6     54    32"));
        assert!(text.contains("THE REST OF THE WAY — EXPECTED"));
        assert!(text.contains("NYR    16.5  12.0   3.5    36.5"));
    }

    #[test]
    fn movement_text_report_exposes_projection_and_checkpoint_deltas() {
        let view = TeamSeasonForecastMovementView {
            schema: "team_season_forecast_movement.v1".to_string(),
            season: 20242025,
            trials: 1_000,
            seed: 20_242_025,
            earlier_label: None,
            later_label: None,
            earlier_as_of_date: Some(NaiveDate::from_ymd_opt(2025, 1, 31).unwrap()),
            later_as_of_date: Some(NaiveDate::from_ymd_opt(2025, 2, 28).unwrap()),
            earlier_fingerprint: "a".repeat(64),
            later_fingerprint: "b".repeat(64),
            teams: vec![TeamSeasonForecastMovementRow {
                team: "NYR".to_string(),
                average_points_delta: 2.25,
                playoff_probability_delta: 0.041,
                stanley_cup_probability_delta: 0.006,
                average_longest_win_streak_delta: 0.2,
                completed_games_delta: Some(12),
                observed_standings_points_delta: Some(17),
                expected_remaining_points_delta: Some(-14.75),
            }],
            disclosures: vec!["Movement is later minus earlier.".to_string()],
        };
        let text = render_movement(&view, &["NYR".to_string()]);
        assert!(text.contains("THE SHIFT — ICECAST MOVEMENT"));
        assert!(text.contains("2025-01-31 to 2025-02-28"));
        assert!(text.contains("+2.25"));
        assert!(text.contains("+4.10pp"));
        assert!(text.contains("+17"));
        assert!(text.contains("-14.75"));
    }

    #[test]
    fn history_text_report_exposes_levels_and_consecutive_deltas() {
        let dates = [
            NaiveDate::from_ymd_opt(2025, 1, 31).unwrap(),
            NaiveDate::from_ymd_opt(2025, 2, 28).unwrap(),
        ];
        let view = TeamSeasonForecastHistoryView {
            schema: "team_season_forecast_history.v1".to_string(),
            season: 20242025,
            trials: 1_000,
            seed: 20_242_025,
            checkpoints: dates
                .into_iter()
                .enumerate()
                .map(
                    |(index, as_of_date)| TeamSeasonForecastHistoryCheckpointRow {
                        as_of_date,
                        fingerprint: if index == 0 { "a" } else { "b" }.repeat(64),
                        league_completed_games: 800 + index * 100,
                        league_remaining_games: 512 - index * 100,
                    },
                )
                .collect(),
            teams: vec![TeamSeasonForecastHistoryTeamRow {
                team: "NYR".to_string(),
                checkpoints: vec![
                    TeamSeasonForecastHistoryPointRow {
                        as_of_date: dates[0],
                        average_points: 90.0,
                        points_p10: 80,
                        points_p50: 90,
                        points_p90: 100,
                        playoff_probability: 0.45,
                        stanley_cup_probability: 0.02,
                        average_longest_win_streak: 5.0,
                        completed_games: 50,
                        remaining_games: 32,
                        observed_standings_points: 54,
                        expected_remaining_points: 36.0,
                        average_points_delta_from_previous: None,
                        playoff_probability_delta_from_previous: None,
                        stanley_cup_probability_delta_from_previous: None,
                        completed_games_delta_from_previous: None,
                        prior_expected_points_for_completed_interval_from_previous: None,
                        realized_points_vs_prior_remaining_pace_from_previous: None,
                        remaining_outlook_revaluation_from_previous: None,
                        pace_attribution_reconciliation_error_from_previous: None,
                    },
                    TeamSeasonForecastHistoryPointRow {
                        as_of_date: dates[1],
                        average_points: 92.25,
                        points_p10: 82,
                        points_p50: 92,
                        points_p90: 102,
                        playoff_probability: 0.491,
                        stanley_cup_probability: 0.027,
                        average_longest_win_streak: 5.2,
                        completed_games: 59,
                        remaining_games: 23,
                        observed_standings_points: 64,
                        expected_remaining_points: 28.25,
                        average_points_delta_from_previous: Some(2.25),
                        playoff_probability_delta_from_previous: Some(0.041),
                        stanley_cup_probability_delta_from_previous: Some(0.007),
                        completed_games_delta_from_previous: Some(9),
                        prior_expected_points_for_completed_interval_from_previous: Some(10.125),
                        realized_points_vs_prior_remaining_pace_from_previous: Some(-0.125),
                        remaining_outlook_revaluation_from_previous: Some(2.375),
                        pace_attribution_reconciliation_error_from_previous: Some(0.0),
                    },
                ],
                average_points_delta_first_to_last: 2.25,
                playoff_probability_delta_first_to_last: 0.041,
                stanley_cup_probability_delta_first_to_last: 0.007,
                projected_points_movement_rank: 1,
                league_team_count: 1,
                projected_points_trend: TeamSeasonForecastHistoryTrend::Improving,
                largest_projected_points_swing: 2.25,
                largest_swing_from_date: dates[0],
                largest_swing_to_date: dates[1],
                average_first_last_points_range_width: 20.0,
                net_points_movement_share_of_range: Some(0.1125),
                net_points_movement_materiality: TeamSeasonForecastHistoryMateriality::Moderate,
                observed_standings_points_delta_first_to_last: 10,
                expected_remaining_points_delta_first_to_last: -7.75,
                points_movement_reconciliation_error: 0.0,
                completed_games_delta_first_to_last: 9,
                prior_expected_points_per_remaining_game: Some(1.125),
                prior_expected_points_for_completed_interval: Some(10.125),
                realized_points_vs_prior_remaining_pace: Some(-0.125),
                remaining_outlook_revaluation: Some(2.375),
                pace_attribution_reconciliation_error: Some(0.0),
            }],
            biggest_risers: vec![TeamSeasonForecastHistoryMoverRow {
                rank: 1,
                team: "NYR".to_string(),
                average_points_delta_first_to_last: 2.25,
                playoff_probability_delta_first_to_last: 0.041,
                stanley_cup_probability_delta_first_to_last: 0.007,
            }],
            biggest_fallers: vec![TeamSeasonForecastHistoryMoverRow {
                rank: 1,
                team: "NYR".to_string(),
                average_points_delta_first_to_last: 2.25,
                playoff_probability_delta_first_to_last: 0.041,
                stanley_cup_probability_delta_first_to_last: 0.007,
            }],
            disclosures: vec!["Chronological sealed checkpoints.".to_string()],
        };
        let text = render_history(&view, &["NYR".to_string()]);
        assert!(text.contains("THE TAPE — ICECAST FORECAST HISTORY"));
        assert!(text.contains("2025-01-31"));
        assert!(text.contains("2025-02-28"));
        assert!(text.contains("+2.25"));
        assert!(text.contains("+4.1pp"));
        assert!(text.contains("+0.7pp"));
        assert!(text.contains("league rank 1 of 1"));
        assert!(text.contains("improving · moderate materiality · largest swing +2.25"));
        assert!(text.contains("bridge: confirmed +10 + remainder -7.75 = net +2.25 points"));
        assert!(text.contains(
            "pace attribution: expected +10.12 over 9 games · realized vs pace -0.12 + remaining revaluation +2.38 = net +2.25"
        ));
        assert!(text.contains(
            "interval: expected +10.12 over 9 games · realized vs prior pace -0.12 + remaining revaluation +2.38 = change +2.25"
        ));
        assert!(text.contains("BIGGEST RISERS"));
        assert!(text.contains("BIGGEST FALLERS"));
    }

    fn scheduled_game(game_id: u64, away: &str, home: &str) -> ScheduledGame {
        ScheduledGame {
            game_id,
            date: "2026-10-08".to_owned(),
            game_type: 2,
            away_abbrev: away.to_owned(),
            away_name: away.to_owned(),
            home_abbrev: home.to_owned(),
            home_name: home.to_owned(),
            start_time_utc: "2026-10-09T02:00:00Z".to_owned(),
            away_score: None,
            home_score: None,
            game_state: Some("FUT".to_owned()),
            last_period: None,
            series_game: None,
            away_wins: None,
            home_wins: None,
        }
    }

    #[test]
    fn calendar_fingerprint_is_ordered_and_ignores_results() {
        let first = scheduled_game(2, "SEA", "VGK");
        let mut second = scheduled_game(1, "NYR", "BOS");
        let expected = schedule_calendar_fingerprint(&[first.clone(), second.clone()]).unwrap();
        second.away_score = Some(4);
        second.home_score = Some(2);
        second.game_state = Some("FINAL".to_owned());
        assert_eq!(
            schedule_calendar_fingerprint(&[second, first]).unwrap(),
            expected
        );
    }

    #[test]
    fn development_feature_normalization_is_bounded_and_missing_is_neutral() {
        let moments = FeatureMoments::from_values([1.0, 2.0, 3.0].into_iter());
        assert!(moments.z(Some(3.0)) > 0.0);
        assert_eq!(moments.z(None), 0.0);
        assert_eq!(moments.z(Some(1_000.0)), 3.0);
    }

    #[test]
    fn ir_signal_requires_one_unambiguous_direction() {
        assert_eq!(
            transaction_availability_delta("Placed RW Example on injured reserve."),
            1
        );
        assert_eq!(
            transaction_availability_delta("Activated D Example from injured reserve."),
            -1
        );
        assert_eq!(
            transaction_availability_delta(
                "Placed C One on injured reserve. Activated D Two from injured reserve."
            ),
            0
        );
        assert_eq!(transaction_availability_delta("Recalled G Example."), 0);
    }

    #[test]
    fn transaction_players_resolve_by_stable_id_and_preserve_ambiguity() {
        let mut identities = BTreeMap::new();
        identities.insert(
            "jared mccann".to_owned(),
            vec![TeamForecastPersonnelPlayerInput {
                player_id: 8477955,
                full_name: "Jared McCann".to_owned(),
                action: "unclassified".to_owned(),
                membership_delta: 0,
                prior_position_group: None,
                prior_season: None,
                prior_games_played: None,
                prior_value: None,
            }],
        );
        identities.insert(
            "sebastian aho".to_owned(),
            vec![
                TeamForecastPersonnelPlayerInput {
                    player_id: 1,
                    full_name: "Sebastian Aho".to_owned(),
                    action: "unclassified".to_owned(),
                    membership_delta: 0,
                    prior_position_group: None,
                    prior_season: None,
                    prior_games_played: None,
                    prior_value: None,
                },
                TeamForecastPersonnelPlayerInput {
                    player_id: 2,
                    full_name: "Sebastian Aho".to_owned(),
                    action: "unclassified".to_owned(),
                    membership_delta: 0,
                    prior_position_group: None,
                    prior_season: None,
                    prior_games_played: None,
                    prior_value: None,
                },
            ],
        );
        let (resolved, ambiguous) = resolve_transaction_players(
            "Placed LW/C Jared McCann and Sebastian Aho on injured reserve.",
            &identities,
        );
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].player_id, 8477955);
        assert_eq!(resolved[0].action, "ir_placed");
        assert_eq!(resolved[0].membership_delta, 0);
        assert_eq!(ambiguous, ["Sebastian Aho"]);
    }

    #[test]
    fn membership_direction_is_player_specific_inside_mixed_rows() {
        let identities = [
            ("alpha player", 1, "Alpha Player"),
            ("beta player", 2, "Beta Player"),
            ("gamma player", 3, "Gamma Player"),
        ]
        .into_iter()
        .map(|(key, player_id, full_name)| {
            (
                key.to_owned(),
                vec![TeamForecastPersonnelPlayerInput {
                    player_id,
                    full_name: full_name.to_owned(),
                    action: "unclassified".to_owned(),
                    membership_delta: 0,
                    prior_position_group: None,
                    prior_season: None,
                    prior_games_played: None,
                    prior_value: None,
                }],
            )
        })
        .collect();
        let (players, _) = resolve_transaction_players(
            "Recalled F Alpha Player from Hartford. Assigned D Beta Player to Hartford. Placed G Gamma Player on injured reserve.",
            &identities,
        );
        let actions = players
            .iter()
            .map(|player| {
                (
                    player.player_id,
                    player.action.as_str(),
                    player.membership_delta,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            actions,
            [(1, "recalled", 1), (2, "assigned", -1), (3, "ir_placed", 0)]
        );
    }

    #[test]
    fn organization_changes_do_not_claim_active_roster_membership() {
        assert_eq!(
            personnel_player_action("acquired f test player from ottawa", "test player"),
            ("acquired", 0)
        );
        assert_eq!(
            personnel_player_action("traded f test player to ottawa", "test player"),
            ("traded_away", 0)
        );
        assert_eq!(
            personnel_player_action("released f test player", "test player"),
            ("released", 0)
        );
    }

    #[test]
    fn opening_roster_gate_requires_a_sealed_snapshot_before_opening_day() {
        let entry =
            |name: &str, created_at: &str, tier: SnapshotTier, sealed: bool| SnapshotEntry {
                name: name.to_owned(),
                season: "20252026".to_owned(),
                tier,
                date: created_at[..10].to_owned(),
                created_at: created_at.to_owned(),
                evidence_at: None,
                evidence_source: None,
                parent_key: None,
                file_count: 32,
                sealed,
            };
        let entries = vec![
            entry(
                "safe-opening",
                "2025-10-06T23:00:00Z",
                SnapshotTier::Rosters,
                true,
            ),
            entry(
                "same-day-too-late",
                "2025-10-07T01:00:00Z",
                SnapshotTier::Rosters,
                true,
            ),
            entry(
                "newer-stats",
                "2025-10-08T01:00:00Z",
                SnapshotTier::Stats,
                true,
            ),
            entry(
                "unsealed-roster",
                "2025-10-09T01:00:00Z",
                SnapshotTier::Rosters,
                false,
            ),
        ];
        let opening_day = NaiveDate::from_ymd_opt(2025, 10, 7).unwrap();

        let (selected, latest) = select_opening_roster_snapshot(&entries, "20252026", opening_day);

        assert_eq!(selected.unwrap().name, "safe-opening");
        assert_eq!(latest.unwrap().name, "same-day-too-late");
    }

    #[test]
    fn opening_roster_gate_uses_trusted_archive_evidence_time() {
        let entry = SnapshotEntry {
            name: "imported-later".to_owned(),
            season: "20242025".to_owned(),
            tier: SnapshotTier::Rosters,
            date: "2024-09-30".to_owned(),
            created_at: "2026-07-20T12:00:00Z".to_owned(),
            evidence_at: Some("2024-09-30T07:46:03Z".to_owned()),
            evidence_source: Some(OPENING_ROSTER_ARCHIVE_SOURCE.to_owned()),
            parent_key: None,
            file_count: 33,
            sealed: true,
        };
        let opening_day = NaiveDate::from_ymd_opt(2024, 10, 4).unwrap();

        let (selected, latest) = select_opening_roster_snapshot(&[entry], "20242025", opening_day);

        assert_eq!(selected.unwrap().name, "imported-later");
        assert_eq!(
            snapshot_evidence_at_text(&latest.unwrap()).as_deref(),
            Some("2024-09-30T07:46:03+00:00")
        );
    }

    #[test]
    fn archive_manifest_requires_exact_season_coverage_and_official_urls() {
        let capture = |team: &str| {
            OpeningRosterArchiveCapture {
            team: team.to_owned(),
            archive_url: format!(
                "https://web.archive.org/web/20230930074603id_/https://api-web.nhle.com/v1/roster/{team}/20232024"
            ),
        }
        };
        let mut manifest = OpeningRosterArchiveManifest {
            schema: OPENING_ROSTER_ARCHIVE_SCHEMA.to_owned(),
            season: 20232024,
            opening_date: NaiveDate::from_ymd_opt(2023, 10, 10).unwrap(),
            captures: icelines_fetch::nhl_teams_for_season("20232024")
                .into_iter()
                .map(capture)
                .collect(),
        };
        assert_eq!(
            validate_opening_roster_archive_manifest(&manifest, true)
                .unwrap()
                .len(),
            32
        );
        manifest.captures[0].archive_url = manifest.captures[0]
            .archive_url
            .replace("/20232024", "/current");
        assert_eq!(
            validate_opening_roster_archive_manifest(&manifest, true)
                .unwrap()
                .len(),
            32
        );

        manifest.captures.pop();
        assert!(validate_opening_roster_archive_manifest(&manifest, true)
            .unwrap_err()
            .to_string()
            .contains("missing"));
        assert_eq!(
            validate_opening_roster_archive_manifest(&manifest, false)
                .unwrap()
                .len(),
            31
        );

        assert!(parse_official_nhl_archive_url(
            "https://web.archive.org/web/20230930074603id_/https://example.com/roster/NYR/20232024",
            "NYR",
            20232024,
        )
        .unwrap_err()
        .to_string()
        .contains("official NHL roster endpoint"));
    }

    #[test]
    fn archived_roster_payload_reports_non_json_response_signature() {
        let error = parse_archived_roster_payload(
            "CBJ",
            b"<html><body>temporary archive response</body></html>",
            "text/html",
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("content-type text/html"));
        assert!(error.contains("temporary archive response"));
    }

    #[test]
    fn archived_roster_payload_decodes_headerless_gzip() {
        use std::io::Write as _;

        let json = br#"{
            "forwards": [{
                "id": 1,
                "firstName": "Test",
                "lastName": "Player",
                "positionCode": "C"
            }],
            "defensemen": [],
            "goalies": []
        }"#;
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(json).unwrap();
        let compressed = encoder.finish().unwrap();

        let roster = parse_archived_roster_payload("CBJ", &compressed, "application/json").unwrap();

        assert_eq!(roster.forwards.len(), 1);
        assert_eq!(roster.forwards[0].id, 1);
    }

    #[test]
    fn retrospective_lineup_accepts_short_bench_but_requires_unique_players_and_two_goalies() {
        let players = |start: u32, count: u32, position: &str| {
            (start..start + count)
                .map(|id| {
                    serde_json::json!({
                        "playerId": id,
                        "name": {"default": format!("P. Player{id}")},
                        "position": position,
                        "sweaterNumber": id % 100
                    })
                })
                .collect::<Vec<_>>()
        };
        let mut raw = serde_json::json!({
            "awayTeam": {"abbrev": "NYR"},
            "homeTeam": {"abbrev": "WSH"},
            "playerByGameStats": {
                "awayTeam": {
                    "forwards": players(1, 12, "C"),
                    "defense": players(20, 6, "D"),
                    "goalies": players(30, 2, "G")
                },
                "homeTeam": {
                    "forwards": [], "defense": [], "goalies": []
                }
            }
        });

        let roster = parse_retrospective_opening_lineup(&raw, "NYR").unwrap();
        assert_eq!(roster.forwards.len(), 12);
        assert_eq!(roster.defensemen.len(), 6);
        assert_eq!(roster.goalies.len(), 2);
        assert_eq!(roster.forwards[0].id, 1);

        raw["playerByGameStats"]["awayTeam"]["defense"]
            .as_array_mut()
            .unwrap()
            .pop();
        assert_eq!(
            parse_retrospective_opening_lineup(&raw, "NYR")
                .unwrap()
                .defensemen
                .len(),
            5
        );

        raw["playerByGameStats"]["awayTeam"]["goalies"]
            .as_array_mut()
            .unwrap()
            .pop();
        assert!(parse_retrospective_opening_lineup(&raw, "NYR")
            .unwrap_err()
            .to_string()
            .contains("expected 15-18 and 2"));
    }

    #[test]
    fn sealed_partial_archive_enables_only_manifest_verified_teams_for_evaluation() {
        let dir = tempdir().unwrap();
        let store = SnapshotStore::new(dir.path().join("snapshots"));
        store
            .create_with_evidence(
                "partial-archive",
                "20242025",
                SnapshotTier::Rosters,
                None,
                "2024-10-03",
                Some("2024-10-03T07:00:00Z".to_owned()),
                Some(OPENING_ROSTER_ARCHIVE_SOURCE.to_owned()),
            )
            .unwrap();
        let manifest = OpeningRosterArchiveManifest {
            schema: OPENING_ROSTER_ARCHIVE_SCHEMA.to_owned(),
            season: 20242025,
            opening_date: NaiveDate::from_ymd_opt(2024, 10, 4).unwrap(),
            captures: vec![OpeningRosterArchiveCapture {
                team: "NYR".to_owned(),
                archive_url: "https://web.archive.org/web/20241003070000id_/https://api-web.nhle.com/v1/roster/NYR/20242025".to_owned(),
            }],
        };
        store
            .write_file(
                "partial-archive",
                &SnapshotTier::Rosters,
                "_opening-roster-archive.json",
                &serde_json::to_vec(&manifest).unwrap(),
            )
            .unwrap();
        let roster = br#"{
            "forwards": [{
                "id": 1,
                "firstName": "Test",
                "lastName": "Player",
                "sweaterNumber": 10,
                "positionCode": "C",
                "shootsCatches": "L",
                "birthDate": null,
                "birthCountry": null,
                "heightInInches": null,
                "weightInPounds": null,
                "headshot": null,
                "birthCity": null,
                "birthStateProvince": null
            }],
            "defensemen": [],
            "goalies": []
        }"#;
        store
            .write_file(
                "partial-archive",
                &SnapshotTier::Rosters,
                "NYR.json",
                roster,
            )
            .unwrap();
        // A roster file without a matching manifest capture is deliberately
        // not promoted into the verified team set.
        store
            .write_file(
                "partial-archive",
                &SnapshotTier::Rosters,
                "BOS.json",
                roster,
            )
            .unwrap();
        store.seal("partial-archive").unwrap();

        let authority = audit_opening_roster_authority(
            &store,
            20242025,
            NaiveDate::from_ymd_opt(2024, 10, 4).unwrap(),
            &["BOS".to_owned(), "NYR".to_owned()].into_iter().collect(),
        );

        assert_eq!(authority.status, "partial_evaluation");
        assert_eq!(authority.verified_teams, 1);
        assert_eq!(authority.verified_team_abbrevs, ["NYR"]);
        assert!(!authority.player_value_effects_enabled);
        assert!(authority.reason.contains("full promotion stays blocked"));
    }

    #[test]
    fn archive_index_claim_without_sealed_provenance_is_rejected() {
        let dir = tempdir().unwrap();
        let store = SnapshotStore::new(dir.path().join("snapshots"));
        store
            .create_with_evidence(
                "unproved-archive",
                "20242025",
                SnapshotTier::Rosters,
                None,
                "2024-09-30",
                Some("2024-09-30T07:46:03Z".to_owned()),
                Some(OPENING_ROSTER_ARCHIVE_SOURCE.to_owned()),
            )
            .unwrap();
        store.seal("unproved-archive").unwrap();

        let authority = audit_opening_roster_authority(
            &store,
            20242025,
            NaiveDate::from_ymd_opt(2024, 10, 4).unwrap(),
            &["NYR".to_owned()].into_iter().collect(),
        );

        assert_eq!(authority.status, "invalid");
        assert!(authority
            .reason
            .contains("archive provenance is unavailable"));
    }

    #[test]
    fn official_live_roster_manifest_is_required_for_authority() {
        let dir = tempdir().unwrap();
        let store = SnapshotStore::new(dir.path().join("snapshots"));
        store
            .create_with_evidence(
                "official-live",
                "20262027",
                SnapshotTier::Rosters,
                None,
                "2026-07-21",
                Some("2026-07-21T12:00:00Z".to_owned()),
                Some(OFFICIAL_NHL_LIVE_ROSTER_SOURCE.to_owned()),
            )
            .unwrap();
        let roster = br#"{
            "forwards": [{
                "id": 1,
                "firstName": "Test",
                "lastName": "Player",
                "sweaterNumber": 10,
                "positionCode": "C",
                "shootsCatches": "L",
                "birthDate": null,
                "birthCountry": null,
                "heightInInches": null,
                "weightInPounds": null,
                "headshot": null,
                "birthCity": null,
                "birthStateProvince": null
            }],
            "defensemen": [],
            "goalies": []
        }"#;
        store
            .write_file("official-live", &SnapshotTier::Rosters, "NYR.json", roster)
            .unwrap();
        let manifest = OfficialNhlRosterCaptureManifest {
            schema: OFFICIAL_NHL_LIVE_ROSTER_SCHEMA.to_owned(),
            season: "20262027".to_owned(),
            observed_at: "2026-07-21T12:00:00Z".to_owned(),
            captures: vec![OfficialNhlRosterCapture {
                team: "NYR".to_owned(),
                source_url: "https://api-web.nhle.com/v1/roster/NYR/current".to_owned(),
            }],
        };
        store
            .write_file(
                "official-live",
                &SnapshotTier::Rosters,
                OFFICIAL_NHL_LIVE_ROSTER_MANIFEST_FILE,
                &serde_json::to_vec(&manifest).unwrap(),
            )
            .unwrap();
        store.seal("official-live").unwrap();

        let authority = audit_opening_roster_authority(
            &store,
            20262027,
            NaiveDate::from_ymd_opt(2026, 9, 29).unwrap(),
            &["NYR".to_owned()].into_iter().collect(),
        );

        assert_eq!(authority.status, "authoritative");

        let dir = tempdir().unwrap();
        let store = SnapshotStore::new(dir.path().join("snapshots"));
        store
            .create(
                "unproved-local",
                "20262027",
                SnapshotTier::Rosters,
                None,
                "2026-07-21",
            )
            .unwrap();
        store
            .write_file("unproved-local", &SnapshotTier::Rosters, "NYR.json", roster)
            .unwrap();
        store.seal("unproved-local").unwrap();
        let authority = audit_opening_roster_authority(
            &store,
            20262027,
            NaiveDate::from_ymd_opt(2026, 9, 29).unwrap(),
            &["NYR".to_owned()].into_iter().collect(),
        );
        assert_eq!(authority.status, "invalid");
        assert!(authority
            .reason
            .contains("no recognized official NHL provenance"));
    }

    #[test]
    fn archive_discovery_selects_latest_capture_strictly_before_opening_day() {
        let cdx = br#"[
            ["timestamp", "original", "statuscode", "digest"],
            ["20240928070000", "https://api-web.nhle.com/v1/roster/NYR/20242025", "200", "old"],
            ["20241003070000", "https://api-web.nhle.com/v1/roster/NYR/20242025", "200", "latest-safe"],
            ["20241004010000", "https://api-web.nhle.com/v1/roster/NYR/20242025", "200", "opening-day"],
            ["20241002070000", "https://api-web.nhle.com/v1/roster/BOS/20242025", "200", "wrong-team"]
        ]"#;

        let capture = select_latest_cdx_capture(
            cdx,
            "NYR",
            NaiveDate::from_ymd_opt(2024, 10, 4).unwrap(),
            "20242025",
        )
        .unwrap()
        .unwrap();

        assert_eq!(capture.team, "NYR");
        assert!(capture.archive_url.contains("20241003070000id_"));

        let current = br#"[
            ["timestamp", "original", "statuscode", "digest"],
            ["20241003080000", "https://api-web.nhle.com/v1/roster/NYR/current", "200", "current-safe"]
        ]"#;
        let capture = select_latest_cdx_capture(
            current,
            "NYR",
            NaiveDate::from_ymd_opt(2024, 10, 4).unwrap(),
            "current",
        )
        .unwrap()
        .unwrap();
        assert!(capture.archive_url.ends_with("/roster/NYR/current"));

        let before_preseason = br#"[
            ["timestamp", "original", "statuscode", "digest"],
            ["20240630080000", "https://api-web.nhle.com/v1/roster/NYR/current", "200", "too-early"]
        ]"#;
        assert!(select_latest_cdx_capture(
            before_preseason,
            "NYR",
            NaiveDate::from_ymd_opt(2024, 10, 4).unwrap(),
            "current",
        )
        .unwrap()
        .is_none());
    }

    #[test]
    fn archive_discovery_uses_valid_cached_response_after_network_failure() {
        let dir = tempdir().unwrap();
        let cache = dir.path().join("NYR.json");
        let cdx = br#"[
            ["timestamp", "original", "statuscode", "digest"],
            ["20241003070000", "https://api-web.nhle.com/v1/roster/NYR/20242025", "200", "safe"]
        ]"#;
        icelines_fetch::snapshot::atomic_write_bytes(&cache, cdx).unwrap();

        let result = resolve_archive_discovery_response(
            Err(anyhow::anyhow!("archive unavailable")),
            &cache,
            "NYR",
            NaiveDate::from_ymd_opt(2024, 10, 4).unwrap(),
            "20242025",
        )
        .unwrap();

        assert!(result.cache_fallback);
        assert_eq!(result.capture.unwrap().team, "NYR");
    }

    #[test]
    fn opening_position_score_treats_missing_history_as_neutral() {
        let (score, used) = score_opening_position_group(&[Some(80.0), None, Some(60.0)], 3);
        assert_eq!(used, 3);
        assert!((score - (190.0 / 3.0)).abs() < 1e-9);
        assert_eq!(score_opening_position_group(&[], 2), (50.0, 0));
        assert_eq!(
            opening_selected_indices(&[Some(40.0), None, Some(70.0)], 2),
            [1, 2].into_iter().collect()
        );
    }

    #[test]
    fn prior_player_value_is_bounded_for_skaters_and_goalies() {
        let (_, skater) = icelines_core::fixtures::stat_catalog_variants::skater_modern();
        let (_, goalie) = icelines_core::fixtures::stat_catalog_variants::goalie();
        for value in [
            prior_season_player_value(&skater),
            prior_season_player_value(&goalie),
        ] {
            assert!(value.is_finite());
            assert!((20.0..=90.0).contains(&value));
        }
    }

    #[test]
    fn reviewed_league_crosswalk_can_seed_historical_identity_candidates() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("reviewed-league.json");
        let row = |provider_player_id: &str, review_status: &str, source: &str| {
            serde_json::json!({
                "provider_player_id": provider_player_id,
                "ahl_display_name": "Example Player",
                "ahl_birth_date": "2001-02-03",
                "match_basis": "exact_name_and_birth_date",
                "review_status": review_status,
                "nhl_player_id": 8480001,
                "nhl_display_name": "Example Player",
                "nhl_birth_date": "2001-02-03",
                "evidence_urls": [source],
                "note": "fixture"
            })
        };
        let crosswalk = |team: &str, rows: Vec<serde_json::Value>| {
            serde_json::json!({
                "schema": "ahl_identity_crosswalk.v1",
                "season": 20232024,
                "provider": "ahl_hockeytech_statview",
                "ahl_team": team,
                "nhl_affiliate": null,
                "roster_fetched_at": "2024-06-01T00:00:00Z",
                "candidates_checked_at": "2024-06-02",
                "counts": {
                    "roster_players": rows.len(),
                    "exact_name_and_birth_date": rows.len(),
                    "surname_and_birth_date": 0,
                    "exact_name_only": 0,
                    "ambiguous": 0,
                    "conflicts": 0,
                    "unmatched": 0,
                    "reviewed": rows.iter().filter(|row| row["review_status"] == "reviewed").count()
                },
                "rows": rows,
                "disclosures": []
            })
        };
        let document = serde_json::json!({
            "schema": "ahl_identity_league_crosswalk.v1",
            "season": 20232024,
            "provider": "ahl_hockeytech_statview",
            "roster_fetched_at": "2024-06-01T00:00:00Z",
            "candidates_checked_at": "2024-06-02",
            "teams": 2,
            "roster_appearances": 3,
            "unique_provider_players": 2,
            "crosswalks": [
                crosswalk("Team One", vec![
                    row("one", "reviewed", "https://example.com/one"),
                    row("pending", "pending", "https://example.com/pending")
                ]),
                crosswalk("Team Two", vec![
                    row("two", "reviewed", "https://example.com/two")
                ])
            ],
            "disclosures": []
        });
        icelines_fetch::snapshot::atomic_write_bytes(
            &path,
            serde_json::to_vec_pretty(&document).unwrap().as_slice(),
        )
        .unwrap();

        let catalog = super::read_affiliate_identity_catalog(&path).unwrap();

        assert_eq!(catalog.checked_at, "2024-06-02");
        assert_eq!(catalog.candidates.len(), 1);
        assert_eq!(catalog.candidates[0].nhl_player_id, 8480001);
        assert_eq!(
            catalog.candidates[0].evidence_urls,
            ["https://example.com/one", "https://example.com/two"]
        );
    }
}
