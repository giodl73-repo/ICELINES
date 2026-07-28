//! Typed presentation boundary for IceLines surfaces.
//!
//! ViewModels carry hockey semantics, source context, identity, warnings, and
//! display policy. CLI, TUI, web, JSON, and reports render these shapes without
//! recomputing hockey logic.

pub mod ahl_affiliate;
pub mod analytics_cache_consumer;
pub mod awards;
pub mod cap_projection;
pub mod card;
pub mod career;
pub mod compare;
pub mod config;
pub mod context;
pub mod data_status;
pub mod development_calibration;
pub mod docs;
pub mod fantasy_assistant;
pub mod fantasy_category_matchup;
pub mod fantasy_competition;
pub mod fantasy_daily;
pub mod fantasy_gap;
pub mod fantasy_goalie_plan;
pub mod fantasy_import;
pub mod fantasy_league;
pub mod fantasy_matchup;
pub mod fantasy_matchup_strategy;
pub mod fantasy_playoff_portfolio;
pub mod fantasy_schedule;
pub mod fantasy_season_sim;
pub mod fantasy_sim;
pub mod fantasy_trade;
pub mod favorites;
pub mod game;
pub mod goalies;
pub mod home;
pub mod isolated_impact;
pub mod leaders;
pub mod line_combination;
pub mod management_behavior;
pub mod matchup_evidence;
pub mod mutation;
pub mod organization_lineup;
pub mod organization_window;
pub mod organization_window_adapters;
pub mod organization_window_calibration;
pub mod organization_window_comparison;
pub mod organization_window_scenario_distribution;
pub mod player_card;
pub mod playoffs;
pub mod poach;
pub mod prospect_conversion;
pub mod prospect_study;
pub mod records;
pub mod report;
pub mod scenario_registry;
pub mod schedule;
pub mod scores;
pub mod scoring;
pub mod scoring_outlook;
pub mod scoring_pace;
pub mod signals;
pub mod snapshot;
pub mod streaks;
pub mod team_ceiling;
pub mod team_depth;
pub mod team_game_forecast;
pub mod team_lineup;
pub mod team_season_forecast;
pub mod tokens;
pub mod training_camp;
pub mod transactions;

pub use ahl_affiliate::{
    build_ahl_affiliate_projection, classify_ahl_development_player,
    current_ahl_affiliation_catalog, AhlAffiliatePlayerInput, AhlAffiliatePlayerView,
    AhlAffiliateProjectionInput, AhlAffiliateProjectionView, AhlAffiliationCatalogView,
    AhlAffiliationView, AhlDevelopmentClassification, AhlDevelopmentRuleInput, AhlLineUnitKind,
    AhlLineUnitView, AhlProspectPoolRowView, AhlRosterPoolAuthority, AhlRosterPoolAuthorityKind,
    AHL_AFFILIATE_PROJECTION_SCHEMA, AHL_AFFILIATION_CATALOG_SCHEMA, AHL_AFFILIATION_SOURCE_URL,
    CURRENT_AHL_AFFILIATION_SEASON,
};
pub use analytics_cache_consumer::{
    analytics_cache_consumer_title, AnalyticsCacheConsumerMetricRow, AnalyticsCacheConsumerView,
};
pub use awards::{PlayerAwardRow, PlayerAwardSeasonRow, PlayerAwardsView};
pub use cap_projection::{
    build_cap_projection, classify_cap_role, sort_team_seasons_by_pressure, CapLimitAuthority,
    CapLimitProjection, CapPressure, CapProjectionAssumptions, CapProjectionContractInput,
    CapProjectionError, CapProjectionPlayerInput, CapProjectionRole, CapProjectionView,
    PlayerCapProjection, SalaryBasis, TeamCapProjection, TeamSeasonCapProjection,
    CAP_LIMIT_SOURCE_URL, CAP_PROJECTION_METHOD, CAP_PROJECTION_SCHEMA, CARLSSON_ANCHOR_URL,
};
pub use card::{
    build_card_comparison_set, build_fantasy_draft_card, build_fantasy_morning_card,
    build_fantasy_roster_card, build_fantasy_trade_card, build_forecast_history_card,
    build_forecast_movement_card, build_organization_window_card, build_season_simulation_card,
    build_team_prognosis_card, parse_card_document, CardAlignedMetricRow, CardAssetFallback,
    CardAssetKind, CardAssetReference, CardAssetState, CardAssetView, CardComparisonError,
    CardComparisonSetView, CardComparisonWarning, CardComparisonWarningKind, CardContextView,
    CardDecisionAlternativeView, CardDocumentError, CardDocumentView, CardIdentityJoinsView,
    CardIdentityKind, CardIdentityView, CardKind, CardLineupGroupKind, CardLineupGroupView,
    CardLineupSlotView, CardMethodologyItemView, CardMetricComparisonView, CardMetricView,
    CardPageView, CardPlayerRowView, CardProbabilityRangeView, CardProvenanceView,
    CardRendererCapability, CardSectionView, CardSimulationContextView, CardThemeView,
    CardTimelineItemView, DecisionSectionView, FantasyDraftCardError, FantasyDraftCardInput,
    FantasyMorningCardError, FantasyMorningCardInput, FantasyRosterCardError,
    FantasyRosterCardInput, FantasyTradeCardError, FantasyTradeCardInput, ForecastHistoryCardError,
    ForecastHistoryCardInput, ForecastMovementCardError, ForecastMovementCardInput,
    IdentityHeaderSectionView, LineupSectionView, MethodologySectionView, MetricStripSectionView,
    OrganizationWindowCardError, OrganizationWindowCardInput, PlayerListSectionView,
    ProbabilityRangeSectionView, ProvenanceSectionView, ScenarioBridgeSectionView,
    SeasonSimulationCardError, SeasonSimulationCardInput, StateNoticeSectionView,
    TeamPrognosisCardError, TeamPrognosisCardInput, TeamPrognosisEventProjection,
    TimelineSectionView, CARD_COMPARISON_SET_SCHEMA, CARD_DOCUMENT_JSON_SCHEMA,
    CARD_DOCUMENT_SCHEMA, FANTASY_DRAFT_CARD_BUILDER_VERSION, FANTASY_MORNING_CARD_BUILDER_VERSION,
    FANTASY_ROSTER_CARD_BUILDER_VERSION, FANTASY_TRADE_CARD_BUILDER_VERSION,
    FORECAST_HISTORY_CARD_VERSION, FORECAST_MOVEMENT_CARD_VERSION,
    ORGANIZATION_WINDOW_CARD_VERSION, SEASON_SIMULATION_CARD_VERSION,
    TEAM_PROGNOSIS_BUILDER_VERSION,
};
pub use career::{
    CareerRow, CareerSortKey, CareerView, CAREER_HISTORY_FETCH_COMMAND,
    CAREER_HISTORY_MISSING_STORE_MESSAGE, CAREER_HISTORY_STORE_PATH,
};
pub use compare::{CompareView, SimilarPlayerRow, SimilarPlayerTarget, SimilarPlayersView};
pub use config::{
    ConfigEntryInput, ConfigEntryRow, ConfigMutationIntent, ConfigView, SeasonTypeMutationIntent,
};
pub use context::{
    AppliedFilter, Completeness, EmptyKind, EmptyState, EvidenceLabel, FilterKey, FilterOp,
    RecoveryAction, ReportContext, ReportKind, ReportSectionRef, SortDirection, SortKey, SortState,
    SourceKind, SourceProvenance, SourceState, ViewContext, ViewWarning, ViewWindow, WarningKind,
};
pub use data_status::{
    DataMutationIntent, DataMutationOperation, DataStatusEntryInput, DataStatusRow, DataStatusView,
};
pub use development_calibration::{
    build_development_calibration, development_cohort_labels, DevelopmentCalibrationCohortRow,
    DevelopmentCalibrationConfig, DevelopmentCalibrationExampleRow, DevelopmentCalibrationRateRow,
    DevelopmentCalibrationView, DevelopmentPositionGroup, DevelopmentTransitionInput,
    DevelopmentValueModel, DEVELOPMENT_CALIBRATION_SCHEMA,
};
pub use docs::DocsView;
pub use fantasy_assistant::{
    apply_fantasy_pickup_reserve, build_fantasy_daily_lineup, build_fantasy_draft_board,
    build_fantasy_morning_briefing, build_fantasy_sleeper_board, build_fantasy_week_budget,
    build_fantasy_weekly_pickups, build_fantasy_weekly_pickups_with_reserve_override,
    fantasy_acquisition_availability, fantasy_waiver_window, import_fantasy_platform_eligibility,
    import_fantasy_taken_players, resolve_fantasy_player_status, FantasyAcquisitionAvailability,
    FantasyAcquisitionInput, FantasyAcquisitionKind, FantasyActiveSlot, FantasyActiveSlotKind,
    FantasyAssistantRules, FantasyBenchAssignmentRow, FantasyDailyLineupView,
    FantasyDraftBoardView, FantasyDraftCandidateInput, FantasyDraftCandidateRow,
    FantasyDraftIdentityInput, FantasyDraftPositionLeader, FantasyDraftValueComponents,
    FantasyEligibilityImportRow, FantasyEligibilityImportStatus, FantasyEligibilityImportView,
    FantasyInjuryPlanView, FantasyLineupAssignmentRow, FantasyLineupPlayerInput,
    FantasyMarketStatus, FantasyMorningAction, FantasyMorningActionKind,
    FantasyMorningBriefingView, FantasyObservationConfidence, FantasyObservationFreshness,
    FantasyPlayerAvailabilityStatus, FantasyReserveAssignmentRow, FantasyResolvedPlayerStatus,
    FantasySleeperBoardView, FantasySleeperComponents, FantasySleeperConfidence,
    FantasySleeperInput, FantasySleeperRow, FantasyStatusObservation, FantasyTakenImportView,
    FantasyTakenPlayerRow, FantasyTakenResolutionStatus, FantasyWaiverWindow,
    FantasyWeekBudgetView, FantasyWeeklyMoveInput, FantasyWeeklyMoveRow, FantasyWeeklyPickupView,
    FANTASY_ASSISTANT_RULES_SCHEMA, FANTASY_DAILY_LINEUP_SCHEMA, FANTASY_DRAFT_BOARD_SCHEMA,
    FANTASY_ELIGIBILITY_IMPORT_SCHEMA, FANTASY_INJURY_PLAN_SCHEMA, FANTASY_MORNING_BRIEFING_SCHEMA,
    FANTASY_SLEEPER_BOARD_SCHEMA, FANTASY_TAKEN_IMPORT_SCHEMA, FANTASY_WEEKLY_PICKUP_SCHEMA,
    FANTASY_WEEK_BUDGET_SCHEMA,
};
pub use fantasy_category_matchup::{
    build_fantasy_category_matchup, FantasyCategoryClassification, FantasyCategoryMatchupInput,
    FantasyCategoryMatchupRow, FantasyCategoryMatchupView, FantasyCategoryPlayerInput,
    FantasyCategoryProjectedResult, FantasyCategoryProjectedValue, FantasyCategoryRateInput,
    FantasyCategoryScope, FantasyCategorySnapshotComponents, FantasyCategorySnapshotInput,
    FantasyCategorySnapshotRow, FantasyCategoryTeamInput, FANTASY_CATEGORY_MATCHUP_SCHEMA,
    FANTASY_CATEGORY_SNAPSHOT_SCHEMA,
};
pub use fantasy_competition::{
    FantasyCategoryAggregation, FantasyCategoryDirection, FantasyCategoryRule,
    FantasyCompetitionMode, FantasyCompetitionRules, FantasyMatchupTiePolicy,
    FANTASY_COMPETITION_RULES_SCHEMA,
};
pub use fantasy_daily::{
    score_daily_goalie_line, score_daily_skater_line, FantasyDailyDeltaInput,
    FantasyDailyDeltaView, FantasyDailyLineInput, FantasyDailyPlayerInput, FantasyDailyPlayerRow,
    FantasyDailyPlayerStatus, FantasyDailyScore, FantasyDailyTeamInput, FantasyDailyTeamRow,
};
pub use fantasy_gap::{
    FantasyRosterGapAction, FantasyRosterGapCandidate, FantasyRosterGapInput,
    FantasyRosterGapReplacement, FantasyRosterGapRow, FantasyRosterGapView,
};
pub use fantasy_goalie_plan::{
    build_fantasy_goalie_plan, resolve_fantasy_goalie_start, FantasyGoalieGameInput,
    FantasyGoaliePlanAction, FantasyGoaliePlanInput, FantasyGoaliePlanPlayerInput,
    FantasyGoaliePlanRow, FantasyGoaliePlanView, FantasyGoaliePortfolioComparison,
    FantasyGoalieRefreshUrgency, FantasyGoalieStartObservation, FantasyGoalieStartState,
    FantasyGoalieStreamCandidateRow, FantasyResolvedGoalieStart, FANTASY_GOALIE_PLAN_SCHEMA,
};
pub use fantasy_import::{
    FantasyImportMode, FantasyImportPlayerRow, FantasyImportRowInput, FantasyImportRowStatus,
    FantasyImportSummary, FantasyImportTeamInput, FantasyImportTeamRow, FantasyImportTeamStatus,
    FantasyImportView, FantasyImportViewInput,
};
pub use fantasy_league::{
    FantasyLeagueInput, FantasyLeagueRow, FantasyLeagueTeamInput, FantasyLeagueTeamRow,
    FantasyLeagueView,
};
pub use fantasy_matchup::{
    FantasyMatchupOutcome, FantasyMatchupRow, FantasyMatchupScheduleInput, FantasyMatchupSideRow,
    FantasyMatchupTeamRow, FantasyMatchupTeamTotalInput, FantasyMatchupWeekInput,
    FantasyMatchupWeekView,
};
pub use fantasy_matchup_strategy::{
    build_fantasy_matchup_strategy, FantasyMatchupDailyProjectionRow,
    FantasyMatchupPointsSnapshotInput, FantasyMatchupStrategy, FantasyMatchupStrategyInput,
    FantasyMatchupStrategyPlayerInput, FantasyMatchupStrategyTeamInput, FantasyMatchupStrategyView,
    FantasyMatchupSwingInput, FantasyMatchupTeamProjection, FANTASY_MATCHUP_STRATEGY_SCHEMA,
};
pub use fantasy_playoff_portfolio::{
    build_fantasy_playoff_portfolio, rank_fantasy_playoff_candidate_fits,
    FantasyPlayoffCandidateFitRow, FantasyPlayoffPlayerInput, FantasyPlayoffPlayerRoundRow,
    FantasyPlayoffPlayerRow, FantasyPlayoffPortfolioInput, FantasyPlayoffPortfolioView,
    FantasyPlayoffRoundInput, FantasyPlayoffRoundRow, FantasyPlayoffTeamRow,
    FANTASY_PLAYOFF_PORTFOLIO_SCHEMA,
};
pub use fantasy_schedule::{
    build_fantasy_schedule_view, FantasyDailySlateRow, FantasyRosterScheduleView,
    FantasyScheduleClassRow, FantasyScheduleComplementRow, FantasyScheduleGameInput,
    FantasyScheduleOverlapRow, FantasyScheduleTeamRow, FantasyScheduleView, FantasyScheduleWeekRow,
    FantasyWeeklyTeamRow, FANTASY_SCHEDULE_SCHEMA,
};
pub use fantasy_season_sim::{
    simulate_fantasy_season, FantasySeasonEventKind, FantasySeasonEventRow, FantasySeasonSimConfig,
    FantasySeasonSimPlayerInput, FantasySeasonSimTeamRow, FantasySeasonSimView,
    FANTASY_SEASON_SIM_SCHEMA,
};
pub use fantasy_sim::{
    build_fantasy_simulation_view, fantasy_roster_games_played, fantasy_roster_games_remaining,
    find_fantasy_roster_player, goalie_scheme_stats_from_view, project_fantasy_roster_score,
    project_fantasy_scenario, resolve_fantasy_scenario_roster,
    resolve_fantasy_scenario_roster_details, score_fantasy_roster, skater_scheme_stats_from_view,
    FantasyRosterScore, FantasyScenarioRosterResolution, FantasySimulationAction,
    FantasySimulationBuildInput, FantasySimulationConfidence, FantasySimulationHorizon,
    FantasySimulationInput, FantasySimulationRosterTeamInput, FantasySimulationScenarioInput,
    FantasySimulationScenarioRosterInput, FantasySimulationScenarioRow, FantasySimulationTeamInput,
    FantasySimulationTeamRow, FantasySimulationView,
};
pub use fantasy_trade::{
    FantasyTradeEvaluationView, FantasyTradePlayerEvaluation, FantasyTradeTeamEvaluation,
    FANTASY_TRADE_EVALUATION_SCHEMA,
};
pub use favorites::{
    FavoriteMemberInput, FavoriteMemberRow, FavoriteMutationIntent, FavoritesView, WatchNoteInput,
    WatchlistMemberRow, WatchlistView,
};
pub use game::{
    GameBoxscoreInput, GameGoalInput, GameGoalRow, GameGoalieInput, GameGoalieRow, GameSkaterInput,
    GameSkaterRow, GameView,
};
pub use goalies::{
    GoalieLeaderboardSort, GoalieRoleFilter, GoalieRoleSignal, GoalieRow, GoaliesView,
};
pub use home::{HomeGoalieRow, HomeSkaterRow, HomeView};
pub use isolated_impact::{
    build_isolated_scenario_impact, build_isolated_scenario_impact_as_of,
    build_isolated_scenario_impact_cached, ForcedCeilingPathRow, IsolatedEventImpactRow,
    IsolatedImpactBaselineRow, IsolatedImpactCache, IsolatedImpactError, IsolatedImpactView,
    ISOLATED_IMPACT_AS_OF_METHOD, ISOLATED_IMPACT_METHOD, ISOLATED_IMPACT_SCHEMA,
};
pub use leaders::{LeaderKind, LeaderRow, LeadersView};
pub use line_combination::{
    build_adaptive_lineup_policy, build_line_combination_forecast, LineCombinationCandidateView,
    LineCombinationForecastConfig, LineCombinationForecastView, LineCombinationPairEvidenceInput,
    LineCombinationPairEvidenceKind, LineCombinationPlayerInfluenceView,
    LineCombinationPlayerLeaderboardsView, LineCombinationScoreView, LineCombinationUnitKind,
    LineCombinationUnitView, LINE_COMBINATION_FORECAST_METHOD, LINE_COMBINATION_FORECAST_SCHEMA,
};
pub use matchup_evidence::{
    build_opponent_style_evidence, build_player_matchup_role_evidence,
    build_team_player_matchup_role_evidence, OpponentStyleEvidenceRow, OpponentStyleScoreView,
    PlayerMatchupRoleEvidenceRow, PlayerRoleSeasonFactsInput, TeamPlayerMatchupRoleEvidenceView,
    TeamStyleSeasonFactsInput, OPPONENT_STYLE_EVIDENCE_SCHEMA, PLAYER_MATCHUP_ROLE_EVIDENCE_SCHEMA,
    TEAM_PLAYER_MATCHUP_ROLE_EVIDENCE_SCHEMA,
};
pub use mutation::{MutationResultView, MutationStatus};
pub use organization_lineup::{
    build_organization_lineup_forecast, OrganizationBlockedPlayerView, OrganizationLevel,
    OrganizationLineupCountsView, OrganizationLineupForecastInput, OrganizationLineupForecastView,
    OrganizationPositionGroup, OrganizationRecallCandidateView, OrganizationRecallPlanView,
    OrganizationUnitKind, OrganizationUnitView, ORGANIZATION_LINEUP_FORECAST_SCHEMA,
};
pub use organization_window::{
    build_organization_window_board, load_organization_window_profile_inventory,
    parse_organization_window_manifest, seal_organization_window_manifest,
    validate_organization_window_board, validate_profile_inventory, OrganizationProfileInput,
    OrganizationProfileObservationView, OrganizationWindowBoardInput, OrganizationWindowBoardView,
    OrganizationWindowError, OrganizationWindowManifestView, OrganizationWindowProfileInventory,
    WindowAggregateStatus, WindowClassification, WindowCohortKind, WindowCohortManifest,
    WindowDimensionManifest, WindowDimensionView, WindowDriverView, WindowEvidenceView,
    WindowFreshness, WindowHorizon, WindowMissingPolicy, WindowNormalizationMethod,
    WindowOrganizationView, WindowOverallView, WindowProfileDescriptor, WindowProfileDirection,
    WindowProfileInventoryCounts, WindowProfileReadiness, WindowProfileStatus, WindowProfileWeight,
    WindowRankState, WindowRankStatusView, WindowSignalFamilyCap,
    ORGANIZATION_PROFILE_OBSERVATION_JSON_SCHEMA, ORGANIZATION_PROFILE_OBSERVATION_SCHEMA,
    ORGANIZATION_WINDOW_BOARD_JSON_SCHEMA, ORGANIZATION_WINDOW_BOARD_SCHEMA,
    ORGANIZATION_WINDOW_CLASSIFICATION_METHOD, ORGANIZATION_WINDOW_MANIFEST_JSON_SCHEMA,
    ORGANIZATION_WINDOW_MANIFEST_SCHEMA, ORGANIZATION_WINDOW_PROFILE_INVENTORY_JSON,
    ORGANIZATION_WINDOW_PROFILE_INVENTORY_SCHEMA, ORGANIZATION_WINDOW_REGISTRY_VERSION,
};
pub use organization_window_adapters::{
    adapt_balanced_organization_window_sources, adapt_line_combination_window_profile,
    balanced_organization_window_manifest, build_balanced_organization_window_board,
    build_forecast_history_organization_window_boards, OrganizationWindowAdapterContext,
    OrganizationWindowSourceSet, ORGANIZATION_WINDOW_BALANCED_MANIFEST_ID,
    ORGANIZATION_WINDOW_FORECAST_HISTORY_MANIFEST_ID,
};
pub use organization_window_calibration::{
    calibrate_organization_window, calibrate_organization_window_rolling_origins,
    evaluate_organization_window_origins, OrganizationWindowCalibrationError,
    OrganizationWindowCalibrationView, OrganizationWindowEvaluationView,
    OrganizationWindowRollingCalibrationView, WindowCalibrationAblationView,
    WindowCalibrationClaimStatus, WindowCalibrationEvaluationOriginInput,
    WindowCalibrationMetricView, WindowCalibrationOriginInput, WindowCalibrationOriginRole,
    WindowCalibrationOriginView, WindowCalibrationSplitView, WindowCalibrationUncertaintyView,
    WindowLeakageAuditRow, WindowOrganizationStabilityView, WindowOutcomeRow,
    WindowTrialNoiseStatus, ORGANIZATION_WINDOW_CALIBRATION_SCHEMA,
    ORGANIZATION_WINDOW_EVALUATION_JSON_SCHEMA, ORGANIZATION_WINDOW_EVALUATION_SCHEMA,
    ORGANIZATION_WINDOW_ROLLING_CALIBRATION_JSON_SCHEMA,
    ORGANIZATION_WINDOW_ROLLING_CALIBRATION_SCHEMA,
};
pub use organization_window_comparison::{
    adapt_line_combination_window_scenario_authority, adapt_team_game_window_personnel_events,
    adapt_team_season_window_scenario_authorities, adapt_training_camp_window_scenario_authorities,
    attribute_organization_window_personnel_movement, build_organization_window_history,
    compare_organization_window_scenario, compare_organization_window_snapshots,
    compare_organization_window_snapshots_with_bridge, compare_organization_window_typed_scenario,
    rebase_organization_window_board, seal_organization_window_bridge,
    OrganizationWindowBridgeView, OrganizationWindowComparisonError, OrganizationWindowHistoryView,
    OrganizationWindowMovementView, OrganizationWindowPersonnelAttributionInputView,
    OrganizationWindowScenarioImpactView, WindowDimensionDeltaView, WindowOrganizationDeltaView,
    WindowPersonnelAttributionView, WindowPersonnelEventKind, WindowPersonnelEventView,
    WindowProfileBridgeView, WindowProfileDeltaView, WindowScenarioAuthorityKind,
    WindowScenarioAuthorityView, WindowScenarioProfileImpactKind, WindowScenarioProfileImpactView,
    WindowScenarioProfileMethodView, ORGANIZATION_WINDOW_BRIDGE_JSON_SCHEMA,
    ORGANIZATION_WINDOW_BRIDGE_SCHEMA, ORGANIZATION_WINDOW_HISTORY_SCHEMA,
    ORGANIZATION_WINDOW_MOVEMENT_SCHEMA,
    ORGANIZATION_WINDOW_PERSONNEL_ATTRIBUTION_INPUT_JSON_SCHEMA,
    ORGANIZATION_WINDOW_PERSONNEL_ATTRIBUTION_INPUT_SCHEMA,
    ORGANIZATION_WINDOW_SCENARIO_IMPACT_SCHEMA,
};
pub use organization_window_scenario_distribution::{
    simulate_organization_window_scenario_distribution,
    OrganizationWindowScenarioDistributionError, OrganizationWindowScenarioDistributionInput,
    OrganizationWindowScenarioDistributionView, WindowScenarioDimensionDistributionView,
    WindowScenarioDistributionSummaryView, WindowScenarioOrganizationDistributionView,
    WindowScenarioProfileShockInput, WindowScenarioShockDistributionView,
    ORGANIZATION_WINDOW_SCENARIO_DISTRIBUTION_INPUT_JSON_SCHEMA,
    ORGANIZATION_WINDOW_SCENARIO_DISTRIBUTION_INPUT_SCHEMA,
    ORGANIZATION_WINDOW_SCENARIO_DISTRIBUTION_JSON_SCHEMA,
    ORGANIZATION_WINDOW_SCENARIO_DISTRIBUTION_SCHEMA,
};
pub use player_card::{
    PlayerCardView, PlayerCareerSummary, PlayerPreNhlCareerRow, PlayerSeasonSummary,
};
pub use playoffs::{
    PlayoffsBracketInput, PlayoffsGameInput, PlayoffsGameRow, PlayoffsRoundInput, PlayoffsRoundRow,
    PlayoffsSeriesInput, PlayoffsSeriesRow, PlayoffsView,
};
pub use poach::{
    default_watch_rules_view, evaluate_watch_alerts, poach_report_context, poach_report_from_board,
    watch_rules_view_with_persisted, weekly_poach_report_from_board,
    weekly_poach_report_from_board_with_watched, AvailabilityState, ComponentStatus,
    ConfidenceSummary, DeploymentSignal, ExplanationImpact, PoachAvailabilityFilter,
    PoachBoardView, PoachCandidateKind, PoachComponentKind, PoachConfidence, PoachExplanation,
    PoachPlayerRow, PoachQuery, PoachReportSection, PoachReportView, PoachScheduleFilter,
    PoachScore, PoachScoreComponent, PoachWindow, RecommendationKind, ScoreRange, WatchAlertRow,
    WatchAlertSeverity, WatchAlertTrigger, WatchAlertsView, WatchRule, WatchRuleMutationIntent,
    WatchRuleMutationOperation, WatchRuleTrigger, WatchRulesView,
};
pub use prospect_conversion::{
    adapt_prospect_conversion_input, build_prospect_conversion_board,
    build_prospect_nhl_performance_document, ProspectConversionBaselineInput,
    ProspectConversionBoardView, ProspectConversionCalibrationBandView, ProspectConversionConfig,
    ProspectConversionDisposition, ProspectConversionInput, ProspectConversionMethodologyView,
    ProspectConversionOrganizationView, ProspectConversionPerformanceDocument,
    ProspectConversionPerformanceInput, ProspectConversionPlayerView,
    ProspectConversionRankBlocker, ProspectConversionResultClass,
    ProspectConversionSignalCalibrationView, ProspectConversionSignalInput,
    ProspectConversionSignalKind, ProspectNhlOutcomeInput, ProspectNhlPerformanceComponentView,
    PROSPECT_CONVERSION_BOARD_SCHEMA, PROSPECT_CONVERSION_INPUT_SCHEMA, PROSPECT_CONVERSION_METHOD,
    PROSPECT_CONVERSION_PERFORMANCE_SCHEMA, PROSPECT_NHL_PERFORMANCE_METHOD,
};
pub use prospect_study::{
    build_prospect_development_study, build_prospect_discovery_board,
    build_prospect_goalie_development_study, build_prospect_program_board,
    build_prospect_program_board_with_goalies, build_prospect_program_history,
    build_prospect_program_sensitivity_with_goalies, ProspectAvailabilityStatus,
    ProspectDevelopmentSeasonInput, ProspectDevelopmentSeasonView, ProspectDevelopmentStudyConfig,
    ProspectDevelopmentStudyInput, ProspectDevelopmentStudyView, ProspectDiscoveryBoardLane,
    ProspectDiscoveryBoardRow, ProspectDiscoveryBoardView, ProspectDiscoveryLensDirection,
    ProspectDiscoveryLensKind, ProspectDiscoveryLensView, ProspectGoalieDevelopmentSeasonInput,
    ProspectGoalieDevelopmentSeasonView, ProspectGoalieDevelopmentStudyConfig,
    ProspectGoalieDevelopmentStudyInput, ProspectGoalieDevelopmentStudyView,
    ProspectHiddenValueClass, ProspectMarketPosition, ProspectNhlGamesAuthority,
    ProspectOpportunityStatus, ProspectProgramBoardConfig, ProspectProgramBoardView,
    ProspectProgramComponentsView, ProspectProgramGraduateView,
    ProspectProgramHistoryOrganizationView, ProspectProgramHistoryPointView,
    ProspectProgramHistoryView, ProspectProgramMethodologyView, ProspectProgramOrganizationView,
    ProspectProgramPositionCountsView, ProspectProgramSensitivityOrganizationView,
    ProspectProgramSensitivityPointView, ProspectProgramSensitivityView,
    ProspectProgramTopProspectView, ProspectSignalComponentView, ProspectStudyEvidenceInput,
    ProspectTrajectory, PROSPECT_DEVELOPMENT_STUDY_SCHEMA, PROSPECT_DISCOVERY_BOARD_SCHEMA,
    PROSPECT_GOALIE_DEVELOPMENT_STUDY_SCHEMA, PROSPECT_PROGRAM_BOARD_SCHEMA,
    PROSPECT_PROGRAM_HISTORY_SCHEMA, PROSPECT_PROGRAM_SCORING_METHOD,
    PROSPECT_PROGRAM_SENSITIVITY_SCHEMA,
};
pub use records::{
    FightRecordInput, PlayerGoalRecordInput, PlayerRecordsView, RecordsOpponentRow, TeamRecordsView,
};
pub use report::{scouting_report_sections, ReportFormat, ReportView};
pub use scenario_registry::{
    scenario_content_sha256, validate_scenario_id, ScenarioRegistryContractError,
    ScenarioRegistryEntryView, ScenarioRegistryReferenceView, ScenarioRegistryView,
    ScenarioScopeView, SCENARIO_REGISTRY_ENTRY_SCHEMA, SCENARIO_REGISTRY_SCHEMA,
};
pub use schedule::{
    OpponentTierBreakdown, ScheduleGameRow, ScheduleMatchupRecord, ScheduleMatchupView,
    ScheduleRecord, ScheduleTeamView, ScheduleView, TeamChipView, TeamQualityLedger,
    TeamRecentForm, TeamRemainingSchedule, TeamScheduleStrength, TeamSeasonGameRow,
    TeamSeasonHeadline, TeamSeasonSplit, TeamSeasonSplits, TeamSeasonVenue, TeamSeasonView,
    TeamStandingInput, TeamStandingsContext,
};
pub use scores::{scores_context, ScheduledGameInput, ScoreGameRow, ScoresDayView, ScoresView};
pub use scoring::{
    GameScoringReportView, InsideShotBucket, InsideShotBucketCounts, InsideShotProxy,
    PlayerScoringProfileView, PlayerScoringTrendRow, PlayerScoringTrendWindow, ScoringEventInput,
    ScoringEventSummary, ScoringShooterSummary, ScoringSplitSummary, ShotEventKind, ShotLocation,
    StrengthState, TeamScoringProfileView, TeamSide, TonightFavoritePlayerScoringRow,
    TonightFavoriteTeamScoringRow, TonightScoringIntelView,
};
pub use scoring_outlook::{
    TeamScoringOutlookMetric, TeamScoringOutlookRecentForm, TeamScoringOutlookRow,
    TeamScoringOutlookSampleStatus, TeamScoringOutlookSourceStatus, TeamScoringOutlookView,
};
pub use scoring_pace::{
    PlayerScoringPaceMetric, PlayerScoringPaceRow, PlayerScoringPaceSampleStatus,
    PlayerScoringPaceView,
};
pub use signals::{
    PlayerSignalRow, PlayerSignalsView, SignalRosterEvidenceFilter, SignalsRosterView,
};
pub use snapshot::{
    SnapshotEntryInput, SnapshotMutationIntent, SnapshotMutationOperation, SnapshotRow,
    SnapshotView,
};
pub use streaks::{
    PlayerGameLineInput, PlayerShotLineInput, PlayerStreakRow, PlayerStreaksView,
    TeamPlayerStreakLeaderRow, TeamPlayerStreaksView,
};
pub use team_ceiling::{
    build_team_ceiling, team_ceiling_player_lens_score, TeamCeilingError, TeamCeilingLens,
    TeamCeilingLensScore, TeamCeilingPlayerInput, TeamCeilingPlayerRow, TeamCeilingRow,
    TeamCeilingView, TEAM_CEILING_METHOD, TEAM_CEILING_SCHEMA,
};
pub use team_depth::{
    DeploymentEvidence, DepthGoalieSlot, DepthLeagueView, DepthLine, DepthPair, DepthPlayerSlot,
    DepthSlotKind, DepthSummary, DepthTeamStrengthRow, TeamDepthChartColumn, TeamDepthChartPlayer,
    TeamDepthChartView, TeamDepthView, TeamTradeImpactView, TradeImpactLine, TradeImpactPair,
    TradeImpactPlayer, TradeImpactSlot,
};
pub use team_game_forecast::{
    build_team_game_forecast, build_team_game_forecast_validation, build_team_game_rolling_replay,
    build_team_game_rolling_replay_with_opening_strengths,
    build_team_game_rolling_replay_with_personnel, TeamForecastGameInput, TeamForecastParameters,
    TeamForecastPersonnelEvidenceInput, TeamForecastPersonnelPlayerInput, TeamForecastReplayConfig,
    TeamForecastStrengthInput, TeamGameForecastAblationRow, TeamGameForecastAccuracyRow,
    TeamGameForecastAccuracySummary, TeamGameForecastBaselineRow, TeamGameForecastBlendRow,
    TeamGameForecastCalibrationHoldoutRow, TeamGameForecastCalibrationObservation,
    TeamGameForecastCalibrationRow, TeamGameForecastCalibrationSummary, TeamGameForecastFactorRow,
    TeamGameForecastHoldoutRow, TeamGameForecastRow, TeamGameForecastSummaryRow,
    TeamGameForecastValidationCheckRow, TeamGameForecastValidationInput,
    TeamGameForecastValidationView, TeamGameForecastView, TeamGameMembershipAnomalyRow,
    TeamGameMembershipIntervalRow, TeamGameOpeningPlayerRow, TeamGameOpeningRosterAuthorityRow,
    TeamGameOpeningStrengthRow, TeamGamePairedTradeRow, TeamGamePersonnelEvidenceRow,
    TeamGamePersonnelPlayerRow, TeamGameScheduleContext, TEAM_GAME_FORECAST_SCHEMA,
    TEAM_GAME_FORECAST_VALIDATION_SCHEMA,
};
pub use team_lineup::{
    build_team_lineup_projection, team_lineup_card_assets, team_lineup_card_section,
    IceLinesPlayerScoreComponent, IceLinesPlayerScoreView, LineupAssignmentEvidence,
    LineupForwardPosition, PlayerScorePositionGroup, TeamLineupDefensePairView,
    TeamLineupForwardLineView, TeamLineupGoaliesView, TeamLineupPlayerInput, TeamLineupPlayerView,
    TeamLineupPortraitView, TeamLineupProjectionError, TeamLineupProjectionView,
    TeamLineupRequestedSlot, TeamLineupSpecialTeamsKind, TeamLineupSpecialTeamsUnitView,
    TeamLineupSpecialTeamsView, TeamLineupWarningView, ICELINES_PLAYER_SCORE_METHOD,
    ICELINES_PLAYER_SCORE_SCHEMA, TEAM_LINEUP_PROJECTION_SCHEMA,
};
pub use team_season_forecast::{
    build_team_season_auto_personnel_scenario, build_team_season_forecast_history,
    build_team_season_forecast_movement, build_team_season_game_plan_event,
    build_team_season_game_plan_schedule, build_team_season_game_plan_schedule_from_evidence,
    build_team_season_plausible_trade_scenario, compare_team_season_forecast_scenarios,
    simulate_team_season_forecast, simulate_team_season_forecast_as_of_with_scenario,
    simulate_team_season_forecast_with_scenario, TeamSeasonAdaptiveLineupChoice,
    TeamSeasonAdaptiveLineupChoiceSummaryRow, TeamSeasonAdaptiveLineupPolicy,
    TeamSeasonAdaptiveLineupSummaryRow, TeamSeasonAutoPersonnelConfig,
    TeamSeasonForecastHistoryCheckpointRow, TeamSeasonForecastHistoryMateriality,
    TeamSeasonForecastHistoryMoverRow, TeamSeasonForecastHistoryPointRow,
    TeamSeasonForecastHistoryTeamRow, TeamSeasonForecastHistoryTrend,
    TeamSeasonForecastHistoryView, TeamSeasonForecastMovementRow, TeamSeasonForecastMovementView,
    TeamSeasonForecastRow, TeamSeasonForecastView, TeamSeasonGamePlanScheduleView,
    TeamSeasonLeagueLeaders, TeamSeasonOpeningRosterChoice,
    TeamSeasonOpeningRosterChoiceSummaryRow, TeamSeasonOpeningRosterPolicy,
    TeamSeasonOpeningRosterSummaryRow, TeamSeasonOpponentGamePlanInput, TeamSeasonPersonnelInput,
    TeamSeasonPivotalGameRow, TeamSeasonPlausibleTradeConfig, TeamSeasonProbabilityLeaderRow,
    TeamSeasonReplayCheckpointTeamRow, TeamSeasonReplayCheckpointView, TeamSeasonScenario,
    TeamSeasonScenarioEvent, TeamSeasonScenarioEventKind, TeamSeasonScenarioImpactRow,
    TeamSeasonScheduleStretchRow, TeamSeasonScheduledGamePlanRow, TeamSeasonSimulationConfig,
    TeamSeasonStretchKind, TeamSeasonTradeTeamInput, TEAM_SEASON_FORECAST_HISTORY_SCHEMA,
    TEAM_SEASON_FORECAST_MOVEMENT_SCHEMA, TEAM_SEASON_FORECAST_SCHEMA,
    TEAM_SEASON_GAME_PLAN_SCHEDULE_SCHEMA, TEAM_SEASON_SCENARIO_SCHEMA,
};
pub use tokens::{
    MetricCell, MetricUnit, MetricValue, SemanticToken, StatKey, ValuePrecision,
    ALL_SEMANTIC_TOKENS,
};
pub use training_camp::{
    build_training_camp_blender_set, build_training_camp_exposure_board,
    build_training_camp_exposure_board_with_context, build_training_camp_lineup_set,
    build_training_camp_opening_roster_policy, simulate_training_camp,
    simulate_training_camp_league, TrainingCampAuthorityStatus, TrainingCampBlenderBranchView,
    TrainingCampBlenderSetView, TrainingCampCompetitionPoolStatus, TrainingCampConfig,
    TrainingCampDisplacementView, TrainingCampExposureBoardView, TrainingCampExposureLane,
    TrainingCampExposurePlayerView, TrainingCampExposurePressureView, TrainingCampExposureTeamView,
    TrainingCampForecastView, TrainingCampLeagueForecastView, TrainingCampLeagueSimulationInput,
    TrainingCampLeagueTeamInput, TrainingCampLeagueTeamView, TrainingCampLineupBranchView,
    TrainingCampLineupSetView, TrainingCampPlayerInput, TrainingCampPlayerView,
    TrainingCampRosterBranchView, TrainingCampRosterStatus, TrainingCampSalaryCapStatus,
    TrainingCampSimulationInput, TrainingCampTradeProtection,
    TrainingCampTransactionAuthorityStatus, TrainingCampTransactionContextInput,
    TrainingCampTransactionPlayerInput, TRAINING_CAMP_BLENDER_SET_SCHEMA,
    TRAINING_CAMP_EXPOSURE_BOARD_SCHEMA, TRAINING_CAMP_FORECAST_METHOD,
    TRAINING_CAMP_FORECAST_SCHEMA, TRAINING_CAMP_LEAGUE_FORECAST_SCHEMA,
    TRAINING_CAMP_LINEUP_SET_SCHEMA, TRAINING_CAMP_TRANSACTION_CONTEXT_SCHEMA,
};
pub use transactions::{TransactionViewRow, TransactionsView};

#[cfg(test)]
mod tests {
    use crate::model::Season;
    use crate::season_stats::SeasonType;
    use crate::view_model::{
        CareerSortKey, CareerView, CompareView, Completeness, ConfigMutationIntent,
        DataMutationIntent, DataMutationOperation, DepthLeagueView, DocsView, EmptyKind,
        FavoriteMemberInput, FavoriteMutationIntent, FavoritesView, GameBoxscoreInput,
        GameGoalInput, GameGoalieInput, GameSkaterInput, GameView, HomeView, LeaderKind,
        LeadersView, MetricCell, MetricUnit, MetricValue, MutationStatus, PlayerCardView,
        PlayoffsBracketInput, PlayoffsGameInput, PlayoffsRoundInput, PlayoffsSeriesInput,
        PlayoffsView, ScheduleMatchupView, ScheduleTeamView, ScheduleView, ScheduledGameInput,
        ScoresView, SeasonTypeMutationIntent, SemanticToken, SnapshotMutationIntent,
        SnapshotMutationOperation, SourceKind, SourceProvenance, SourceState, StatKey,
        TeamSeasonView, TeamStandingInput, TransactionsView, ValuePrecision, ViewContext,
        ViewWindow, WatchNoteInput, WatchlistView,
    };

    #[test]
    fn context_source_state_survives_json_projection() {
        let mut context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        context.completeness = Completeness::Partial;
        context.data_generation = Some("fixture-generation-1".to_string());
        context.source_state.push(SourceState {
            source: SourceKind::Roster,
            state: Completeness::Partial,
            provenance: Some(SourceProvenance::Snapshot {
                id: "snapshot-2026-05-09".to_string(),
            }),
            fetched_at: None,
            stale_reason: None,
            message: Some("missing shifts".to_string()),
        });

        let view = LeadersView::new(context, LeaderKind::Skaters);
        let json = serde_json::to_string(&view).expect("serialize leaders view");

        assert!(json.contains("\"season\":20252026"));
        assert!(json.contains("\"season_type\":\"regular\""));
        assert!(json.contains("\"completeness\":\"partial\""));
        assert!(json.contains("\"source\":\"roster\""));
        assert!(json.contains("\"data_generation\":\"fixture-generation-1\""));
    }

    #[test]
    fn docs_viewmodel_carries_source_metadata_and_rendered_body() {
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        let view = DocsView::rendered(
            context,
            "COMMANDS.md",
            "IceLines Commands",
            "# IceLines\n",
            "<h1>IceLines</h1>",
        );

        assert_eq!(view.context.source_state[0].source, SourceKind::Docs);
        assert_eq!(view.source_path, "COMMANDS.md");
        assert_eq!(view.markdown, "# IceLines\n");
        assert_eq!(view.markdown_bytes, "# IceLines\n".len());
        assert!(view.rendered_html.contains("<h1>IceLines</h1>"));
    }

    #[test]
    fn l0_admin_mutation_intents_project_shared_result_views() {
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));

        let config = ConfigMutationIntent::set("web.active_season_type", "playoff")
            .expect("valid config intent")
            .result_view(context.clone(), true);
        assert_eq!(config.operation, "config_set");
        assert_eq!(config.status, MutationStatus::Applied);

        let data =
            DataMutationIntent::resolve(DataMutationOperation::Verify, "20252026/regular", false)
                .expect("valid data intent")
                .result_view(context.clone(), false);
        assert_eq!(data.operation, "data_verify");
        assert_eq!(data.status, MutationStatus::Noop);

        let snapshot = SnapshotMutationIntent::resolve(
            SnapshotMutationOperation::Activate,
            "stats-2026-05-10",
        )
        .expect("valid snapshot intent")
        .result_view(context, true);
        assert_eq!(snapshot.operation, "snapshot_activate");
        assert_eq!(snapshot.status, MutationStatus::Applied);
    }

    #[test]
    fn metric_cell_carries_precision_and_semantic_token() {
        let cell = MetricCell {
            key: StatKey::from("points_per_game"),
            label: "PPG".to_string(),
            value: MetricValue::Decimal(1.27),
            unit: MetricUnit::PerGame,
            precision: ValuePrecision::TwoDecimals,
            token: Some(SemanticToken::DecisionHighlight),
        };

        let json = serde_json::to_value(&cell).expect("serialize metric cell");

        assert_eq!(json["key"], "points_per_game");
        assert_eq!(json["unit"], "per_game");
        assert_eq!(json["precision"], "two_decimals");
        assert_eq!(json["token"], "decision_highlight");
    }

    #[test]
    fn report_context_reuses_view_context() {
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        let report = crate::view_model::ReportContext {
            kind: crate::view_model::ReportKind::Scouting,
            view_context: context,
            report_id: "scouting-8478402".to_string(),
            title: "Scouting Report".to_string(),
            sections: vec![crate::view_model::ReportSectionRef {
                id: "summary".to_string(),
                title: "Summary".to_string(),
            }],
        };

        let json = serde_json::to_value(&report).expect("serialize report context");

        assert_eq!(json["kind"], "scouting");
        assert_eq!(json["view_context"]["window"]["season"], 20252026);
        assert_eq!(json["sections"][0]["id"], "summary");
    }

    #[test]
    fn career_viewmodel_filters_latest_league_season_and_sorts() {
        use crate::career_history::{CareerGameType, CareerHistory, CareerStint, LeagueAbbrev};

        fn stint(season: u32, points: u32, gp: u32, team: &str) -> CareerStint {
            CareerStint {
                season: Season(season),
                league: LeagueAbbrev::new("OHL"),
                team: team.to_string(),
                game_type: CareerGameType::Regular,
                sequence: 0,
                gp,
                goals: Some(points / 2),
                assists: Some(points - (points / 2)),
                points: Some(points),
                pim: None,
                plus_minus: None,
                power_play_goals: None,
                power_play_points: None,
                shorthanded_goals: None,
                shorthanded_points: None,
                game_winning_goals: None,
                ot_goals: None,
                shots: None,
                shooting_pct: None,
                avg_toi_sec: None,
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
            }
        }

        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        let histories = vec![
            (
                1,
                CareerHistory {
                    player_id: 1,
                    stints: vec![stint(20142015, 100, 50, "ERI")],
                },
            ),
            (
                2,
                CareerHistory {
                    player_id: 2,
                    stints: vec![
                        stint(20142015, 80, 40, "LDN"),
                        stint(20132014, 120, 60, "LDN"),
                    ],
                },
            ),
        ];
        let mut names = std::collections::HashMap::new();
        names.insert(1, "Connor McDavid".to_string());
        names.insert(2, "Mitch Marner".to_string());

        let view = CareerView::from_histories(
            context,
            "ohl".to_string(),
            None,
            CareerSortKey::Ppg,
            10,
            histories,
            names,
        );

        assert_eq!(view.league, "ohl");
        assert_eq!(view.season, 20142015);
        assert_eq!(view.rows.len(), 2);
        assert_eq!(view.rows[0].name, "Connor McDavid");
        assert_eq!(view.rows[0].points_per_game, Some(2.0));
    }

    #[test]
    fn playoffs_viewmodel_projects_bracket_series() {
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Playoff));
        let view = PlayoffsView::from_bracket(
            context,
            "20252026".to_string(),
            "historical bundle".to_string(),
            PlayoffsBracketInput {
                rounds: vec![PlayoffsRoundInput {
                    round_number: 1,
                    label: "First Round".to_string(),
                    series: vec![PlayoffsSeriesInput {
                        letter: Some("A".to_string()),
                        top_abbrev: "EDM".to_string(),
                        top_name: "Edmonton Oilers".to_string(),
                        top_wins: 4,
                        top_seed_rank: Some("P2".to_string()),
                        bottom_abbrev: "LAK".to_string(),
                        bottom_name: "Los Angeles Kings".to_string(),
                        bottom_wins: 2,
                        bottom_seed_rank: Some("P3".to_string()),
                        winner_abbrev: Some("EDM".to_string()),
                        conference: Some("Western".to_string()),
                        games: vec![PlayoffsGameInput {
                            date: "2026-04-20".to_string(),
                            home_abbrev: "EDM".to_string(),
                            away_abbrev: "LAK".to_string(),
                            home_score: 4,
                            away_score: 2,
                            series_after: "EDM leads 1-0".to_string(),
                        }],
                    }],
                }],
            },
        );

        assert_eq!(view.season_pretty, "2025-26");
        assert_eq!(view.context.source_state[0].source, SourceKind::Playoffs);
        assert!(!view.empty);
        assert_eq!(view.rounds[0].series[0].summary, "EDM 4-2 LAK · EDM wins");
        assert!(view.rounds[0].series[0].is_complete);
        assert_eq!(
            view.rounds[0].series[0].winner_abbrev.as_deref(),
            Some("EDM")
        );
        assert_eq!(view.rounds[0].series[0].games_played, 6);
        assert_eq!(view.rounds[0].series[0].top_seed_rank, "P2");
        assert_eq!(view.rounds[0].series[0].bottom_seed_rank, "P3");
        assert_eq!(view.rounds[0].series[0].letter, "A");
        assert_eq!(view.rounds[0].series[0].games[0].game_number, 1);
    }

    #[test]
    fn favorites_viewmodel_counts_members_and_attaches_stat_lines() {
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        let mut stat_lines = std::collections::HashMap::new();
        stat_lines.insert(
            "connor mcdavid".to_string(),
            "EDM 4-2 W · 1G 2A 3P".to_string(),
        );

        let view = FavoritesView::from_members(
            context,
            "Favorites".to_string(),
            vec![
                FavoriteMemberInput {
                    kind: "player".to_string(),
                    key: "connor mcdavid".to_string(),
                },
                FavoriteMemberInput {
                    kind: "team".to_string(),
                    key: "EDM".to_string(),
                },
            ],
            stat_lines,
        );

        assert_eq!(view.context.source_state[0].source, SourceKind::Favorites);
        assert_eq!(view.player_count, 1);
        assert_eq!(view.team_count, 1);
        assert_eq!(
            view.rows[0].stat_line.as_deref(),
            Some("EDM 4-2 W · 1G 2A 3P")
        );
        assert!(view.empty_state.is_none());
    }

    #[test]
    fn favorite_mutation_intent_resolves_kind_key_and_safe_redirect() {
        let team = FavoriteMutationIntent::resolve("edm", None, Some("/team/EDM"), None)
            .expect("team intent");
        assert_eq!(team.kind, "team");
        assert_eq!(team.key, "EDM");
        assert_eq!(team.entity_ref, "team:EDM");
        assert_eq!(team.redirect_to, "/team/EDM");

        let player = FavoriteMutationIntent::resolve(
            "Connor McDavid",
            Some("player"),
            Some("//evil.example/path"),
            Some("/favorites"),
        )
        .expect("player intent");
        assert_eq!(player.kind, "player");
        assert_eq!(player.key, "connor mcdavid");
        assert_eq!(player.redirect_to, "/favorites");
    }

    #[test]
    fn season_type_mutation_intent_normalizes_kind_and_safe_redirect() {
        let playoff = SeasonTypeMutationIntent::resolve(
            "playoffs",
            Some("http://127.0.0.1:8000/player/8478402"),
        );
        assert_eq!(playoff.active_season_type, "playoff");
        assert_eq!(playoff.redirect_to, "/player/8478402");

        let regular = SeasonTypeMutationIntent::resolve("garbage", Some("https://evil.example/x"));
        assert_eq!(regular.active_season_type, "regular");
        assert_eq!(regular.redirect_to, "/");

        let scheme_relative =
            SeasonTypeMutationIntent::resolve("playoff", Some("http://localhost:8000//evil"));
        assert_eq!(scheme_relative.redirect_to, "/");

        let fake_local =
            SeasonTypeMutationIntent::resolve("playoff", Some("http://localhost.evil/path"));
        assert_eq!(fake_local.redirect_to, "/");
    }

    #[test]
    fn watchlist_viewmodel_attaches_notes_and_counts_members() {
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        let mut notes = std::collections::HashMap::new();
        notes.insert(
            "player:matthew knies".to_string(),
            WatchNoteInput {
                reason: "Poach score 72.0; confidence High".to_string(),
                source: "tui-poach".to_string(),
                updated_at: "2026-05-09T12:00:00Z".to_string(),
            },
        );

        let view = WatchlistView::from_members(
            context,
            "Watchlist".to_string(),
            vec![
                FavoriteMemberInput {
                    kind: "player".to_string(),
                    key: "matthew knies".to_string(),
                },
                FavoriteMemberInput {
                    kind: "team".to_string(),
                    key: "TOR".to_string(),
                },
            ],
            notes,
        );

        assert_eq!(view.context.source_state[0].source, SourceKind::Watchlist);
        assert_eq!(view.player_count, 1);
        assert_eq!(view.team_count, 1);
        assert_eq!(
            view.rows[0].reason.as_deref(),
            Some("Poach score 72.0; confidence High")
        );
        assert_eq!(view.rows[0].source.as_deref(), Some("tui-poach"));
        assert!(view.empty_state.is_none());
    }

    #[test]
    fn first_viewmodel_builders_read_from_repository() {
        let (identity, stats) = crate::fixtures::stat_catalog_variants::skater_modern();
        let team = stats.team_stints[0].team.clone();
        let repo = crate::fixtures::test_repo_with(identity, stats);

        let leaders = crate::view_model::LeadersView::skater_pace(
            &repo,
            Season(20242025),
            SeasonType::Regular,
        );
        assert_eq!(leaders.rows.len(), 1);
        assert_eq!(
            leaders.rows[0].primary.key,
            crate::view_model::StatKey::from("pace_82")
        );
        assert_eq!(leaders.rows[0].rank, 1);

        let depth = crate::view_model::TeamDepthView::from_repository(
            &repo,
            team,
            Season(20242025),
            SeasonType::Regular,
        );
        assert!(!depth.is_empty());
        assert_eq!(depth.forward_lines.len(), 4);
        assert!(depth.forward_lines.iter().any(|line| line
            .center
            .as_ref()
            .is_some_and(|slot| slot.display_name == "Connor McDavid")));
    }

    #[test]
    fn team_depth_view_empty_accessor_reports_no_projected_players() {
        let view = crate::view_model::TeamDepthView::from_player_views(
            crate::model::TeamAbbr("EDM".to_string()),
            Season(20242025),
            SeasonType::Regular,
            &[],
        );

        assert!(view.is_empty());
    }

    #[test]
    fn home_viewmodel_picks_preview_skaters_and_goalies() {
        let (skater_identity, skater_stats) =
            crate::fixtures::stat_catalog_variants::skater_modern();
        let (goalie_identity, goalie_stats) = crate::fixtures::stat_catalog_variants::goalie();
        let mut repo = crate::fixtures::test_repo_with(skater_identity, skater_stats);
        repo.upsert_identity(goalie_identity)
            .expect("goalie identity upsert");
        repo.upsert_stats(goalie_stats)
            .expect("goalie stats upsert");

        let view = HomeView::from_repository(&repo, Season(20242025), SeasonType::Regular, 5, 3);

        assert_eq!(view.context.source_state[0].source, SourceKind::Home);
        assert_eq!(view.top_skaters.len(), 1);
        assert_eq!(view.top_goalies.len(), 1);
        assert_eq!(view.top_goalies[0].save_pct, Some(0.915));
    }

    #[test]
    fn leaders_viewmodel_contract_fixture_serializes_surface_fields() {
        let (identity, stats) = crate::fixtures::stat_catalog_variants::skater_modern();
        let repo = crate::fixtures::test_repo_with(identity, stats);

        let mut view = crate::view_model::LeadersView::skater_pace(
            &repo,
            Season(20242025),
            SeasonType::Regular,
        );
        view.context.data_generation = Some("campbell-contract-fixture-v1".to_string());

        let json = serde_json::to_value(&view).expect("serialize leaders contract fixture");
        let row = &json["rows"][0];

        assert_eq!(json["context"]["window"]["season"], 20242025);
        assert_eq!(json["context"]["window"]["season_type"], "regular");
        assert_eq!(json["context"]["completeness"], "complete");
        assert_eq!(
            json["context"]["data_generation"],
            "campbell-contract-fixture-v1"
        );
        assert_eq!(json["kind"], "skaters");
        assert_eq!(json["sort"]["key"], "pace_82");
        assert_eq!(json["sort"]["direction"], "desc");

        assert_eq!(row["rank"], 1);
        assert_eq!(row["player_id"], 8478402);
        assert_eq!(row["display_name"], "Connor McDavid");
        assert_eq!(row["team"], "EDM");
        assert_eq!(row["position"], "Center");
        assert_eq!(row["primary"]["key"], "pace_82");
        assert_eq!(row["primary"]["value"]["decimal"], 130.0);
        assert_eq!(row["primary"]["unit"], "per82");
        assert_eq!(row["primary"]["precision"], "one_decimal");
        assert_eq!(row["primary"]["token"], "decision_highlight");

        assert_eq!(row["secondary"][0]["key"], "age");
        assert_eq!(row["secondary"][0]["value"]["integer"], 28);
        assert_eq!(row["secondary"][1]["key"], "gp");
        assert_eq!(row["secondary"][1]["value"]["integer"], 82);
        assert_eq!(row["secondary"][4]["key"], "points");
        assert_eq!(row["secondary"][4]["value"]["integer"], 130);
        assert!(
            row["catalog_metrics"]
                .as_array()
                .expect("catalog metrics array")
                .iter()
                .any(|metric| metric["key"] == "points" && metric["value"]["integer"] == 130),
            "leader row must carry catalog metrics for export/custom surfaces"
        );
        assert_eq!(row["tokens"][0], "supporting_evidence");
    }

    #[test]
    fn leaders_age_metric_uses_view_window_not_current_calendar() {
        let (identity, mut stats) = crate::fixtures::stat_catalog_variants::skater_modern();
        stats.season = Season(20222023);
        let repo = crate::fixtures::test_repo_with(identity, stats);

        let view = crate::view_model::LeadersView::skater_pace(
            &repo,
            Season(20222023),
            SeasonType::Regular,
        );
        let age = &view.rows[0].secondary[0].value;

        assert_eq!(age, &MetricValue::Integer(26));
    }

    #[test]
    fn goalie_viewmodel_builder_preserves_role_evidence() {
        let (identity, stats) = crate::fixtures::stat_catalog_variants::goalie();
        let repo = crate::fixtures::test_repo_with_goalie(identity, stats);

        let goalies = crate::view_model::GoaliesView::from_repository(
            &repo,
            Season(20242025),
            SeasonType::Regular,
        );

        assert_eq!(goalies.rows.len(), 1);
        assert_eq!(goalies.rows[0].role_signal.label, "starter");
        assert_eq!(
            goalies.rows[0].role_signal.evidence,
            crate::view_model::DeploymentEvidence::Actual
        );
    }

    #[test]
    fn goalie_viewmodel_contract_fixture_serializes_role_and_metrics() {
        let (identity, stats) = crate::fixtures::stat_catalog_variants::goalie();
        let repo = crate::fixtures::test_repo_with_goalie(identity, stats);

        let mut view = crate::view_model::GoaliesView::from_repository(
            &repo,
            Season(20242025),
            SeasonType::Regular,
        );
        view.context.data_generation = Some("campbell-contract-fixture-v1".to_string());

        let json = serde_json::to_value(&view).expect("serialize goalies contract fixture");
        let row = &json["rows"][0];

        assert_eq!(json["context"]["window"]["season"], 20242025);
        assert_eq!(json["context"]["window"]["season_type"], "regular");
        assert_eq!(json["context"]["completeness"], "complete");
        assert_eq!(
            json["context"]["data_generation"],
            "campbell-contract-fixture-v1"
        );
        assert_eq!(json["sort"]["key"], "save_pct");
        assert_eq!(json["sort"]["direction"], "desc");

        assert_eq!(row["player_id"], 8476434);
        assert_eq!(row["display_name"], "Connor McDavid");
        assert_eq!(row["team"], "FLA");
        assert_eq!(row["role_signal"]["label"], "starter");
        assert_eq!(row["role_signal"]["evidence"], "actual");
        assert_eq!(row["metrics"][0]["key"], "gp");
        assert_eq!(row["metrics"][0]["unit"], "games");
        assert_eq!(row["metrics"][0]["precision"], "integer");
        assert_eq!(row["metrics"][5]["key"], "save_pct");
        assert_eq!(row["metrics"][5]["unit"], "percentage");
        assert_eq!(row["metrics"][5]["precision"], "three_decimals");
        assert_eq!(row["metrics"][6]["key"], "gaa");
        assert_eq!(row["metrics"][2]["key"], "wins");
        assert_eq!(row["metrics"][3]["key"], "losses");
        assert_eq!(row["metrics"][7]["key"], "shutouts");
        assert_eq!(row["tokens"][0], "supporting_evidence");
    }

    #[test]
    fn missing_windows_are_not_marked_complete() {
        let repo = crate::stats_repository::StatsRepository::new();

        let leaders = crate::view_model::LeadersView::skater_pace(
            &repo,
            Season(19991998),
            SeasonType::Regular,
        );
        let goalies = crate::view_model::GoaliesView::from_repository(
            &repo,
            Season(19991998),
            SeasonType::Regular,
        );
        let depth = crate::view_model::TeamDepthView::from_repository(
            &repo,
            crate::model::TeamAbbr("EDM".to_string()),
            Season(19991998),
            SeasonType::Regular,
        );

        assert_eq!(leaders.context.completeness, Completeness::Unavailable);
        assert_eq!(
            leaders.empty_state.as_ref().map(|state| state.kind),
            Some(EmptyKind::MissingSource)
        );
        assert_eq!(goalies.context.completeness, Completeness::Unavailable);
        assert_eq!(
            goalies.empty_state.as_ref().map(|state| state.kind),
            Some(EmptyKind::MissingSource)
        );
        assert_eq!(depth.context.completeness, Completeness::Unavailable);
        assert_eq!(depth.context.source_state[0].source, SourceKind::Roster);
    }

    #[test]
    fn team_depth_preserves_goalie_section() {
        let (identity, stats) = crate::fixtures::stat_catalog_variants::goalie();
        let team = stats.team_stints[0].team.clone();
        let repo = crate::fixtures::test_repo_with_goalie(identity, stats);

        let depth = crate::view_model::TeamDepthView::from_repository(
            &repo,
            team,
            Season(20242025),
            SeasonType::Regular,
        );

        assert_eq!(depth.goalies.len(), 1);
        assert_eq!(depth.goalies[0].role, "starter");
        assert!(
            depth.extras.is_empty(),
            "goalies must not be rendered as extras"
        );
    }

    #[test]
    fn team_depth_contract_fixture_serializes_slots_and_goalie_section() {
        let (identity, stats) = crate::fixtures::stat_catalog_variants::skater_modern();
        let team = stats.team_stints[0].team.clone();
        let repo = crate::fixtures::test_repo_with(identity, stats);

        let mut depth = crate::view_model::TeamDepthView::from_repository(
            &repo,
            team,
            Season(20242025),
            SeasonType::Regular,
        );
        depth.context.data_generation = Some("campbell-contract-fixture-v1".to_string());

        let json = serde_json::to_value(&depth).expect("serialize team depth contract fixture");
        let center = &json["forward_lines"][0]["center"];

        assert_eq!(json["context"]["window"]["season"], 20242025);
        assert_eq!(json["context"]["window"]["season_type"], "regular");
        assert_eq!(json["context"]["completeness"], "complete");
        assert_eq!(
            json["context"]["data_generation"],
            "campbell-contract-fixture-v1"
        );
        assert_eq!(json["team"], "EDM");
        assert_eq!(json["summary"]["metrics"][0]["key"], "rostered");
        assert_eq!(json["summary"]["metrics"][0]["value"]["integer"], 1);
        assert_eq!(json["summary"]["tokens"][0], "supporting_evidence");

        assert_eq!(center["player_id"], 8478402);
        assert_eq!(center["display_name"], "Connor McDavid");
        assert_eq!(center["team"], "EDM");
        assert_eq!(center["slot"]["forward"]["line"], 1);
        assert_eq!(center["slot"]["forward"]["slot"], "center");
        assert_eq!(center["position"], "Center");
        assert_eq!(center["evidence"], "estimated");
        assert_eq!(center["metrics"][0]["key"], "pace_82");
        assert_eq!(center["metrics"][0]["unit"], "per82");
        assert_eq!(center["metrics"][0]["precision"], "one_decimal");
        assert_eq!(center["metrics"][2]["key"], "goals");
        assert_eq!(center["metrics"][3]["key"], "assists");
        assert_eq!(center["metrics"][4]["key"], "points");
        assert_eq!(center["metrics"][5]["key"], "gp");
    }

    #[test]
    fn player_card_viewmodel_contract_fixture_serializes_context_active_and_career() {
        use crate::career_history::{CareerGameType, CareerStint, LeagueAbbrev};
        let (identity, stats) = crate::fixtures::stat_catalog_variants::skater_modern();
        let player_id = identity.id;
        let repo = crate::fixtures::test_repo_with(identity, stats);

        let mut view = PlayerCardView::from_repository(
            &repo,
            player_id,
            Season(20242025),
            SeasonType::Regular,
        )
        .expect("player exists");
        view = view.with_pre_nhl_stints(&[CareerStint {
            season: Season(20142015),
            league: LeagueAbbrev::new("OHL"),
            team: "Erie".to_string(),
            game_type: CareerGameType::Regular,
            sequence: 1,
            gp: 47,
            goals: Some(44),
            assists: Some(76),
            points: Some(120),
            pim: None,
            plus_minus: None,
            power_play_goals: None,
            power_play_points: None,
            shorthanded_goals: None,
            shorthanded_points: None,
            game_winning_goals: None,
            ot_goals: None,
            shots: None,
            shooting_pct: None,
            avg_toi_sec: None,
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
        }]);
        view.context.data_generation = Some("campbell-player-card-fixture-v1".to_string());

        let json = serde_json::to_value(&view).expect("serialize player card view");

        assert_eq!(json["context"]["window"]["season"], 20242025);
        assert_eq!(json["context"]["window"]["season_type"], "regular");
        assert_eq!(
            json["context"]["data_generation"],
            "campbell-player-card-fixture-v1"
        );
        assert_eq!(json["player_id"], 8478402);
        assert_eq!(json["display_name"], "Connor McDavid");
        assert_eq!(
            json["headshot_url"],
            "https://assets.nhle.com/mugs/nhl/default/8478402.png"
        );
        assert_eq!(json["active"]["position"], "Center");
        assert_eq!(json["active"]["team"], "EDM");
        assert_eq!(json["active"]["team_display"], "EDM");
        assert_eq!(json["active"]["metrics"][0]["key"], "gp");
        assert_eq!(json["active"]["metrics"][3]["key"], "points");
        assert_eq!(json["active"]["metrics"][4]["key"], "points_per_game");
        assert_eq!(json["active"]["metrics"][4]["unit"], "per_game");
        assert_eq!(json["active"]["metrics"][5]["key"], "plus_minus");
        assert_eq!(json["active"]["metrics"][6]["key"], "pim");
        assert_eq!(json["active"]["metrics"][7]["key"], "shots");
        assert_eq!(json["active"]["metrics"][8]["key"], "shooting_pct");
        assert_eq!(json["active"]["metrics"][18]["key"], "toi_per_game_sec");
        assert_eq!(json["career"][0]["season"], 20242025);
        assert_eq!(json["career"][0]["metrics"][3]["value"]["integer"], 130);
        assert_eq!(json["pre_nhl_career"][0]["season_label"], "14-15");
        assert_eq!(json["pre_nhl_career"][0]["league"], "OHL");
        assert_eq!(json["pre_nhl_career"][0]["points"], 120);
    }

    #[test]
    fn compare_viewmodel_contract_fixture_serializes_player_cards() {
        let (identity, stats) = crate::fixtures::stat_catalog_variants::skater_modern();
        let player_id = identity.id;
        let repo = crate::fixtures::test_repo_with(identity, stats);

        let mut view = CompareView::from_repository(
            &repo,
            Some(player_id),
            None,
            Season(20242025),
            SeasonType::Regular,
        );
        view.context.data_generation = Some("ted-compare-fixture-v1".to_string());

        let json = serde_json::to_value(&view).expect("serialize compare view");

        assert_eq!(json["context"]["window"]["season"], 20242025);
        assert_eq!(json["context"]["data_generation"], "ted-compare-fixture-v1");
        assert_eq!(json["a"]["player_id"], 8478402);
        assert_eq!(json["a"]["display_name"], "Connor McDavid");
        assert_eq!(json["a"]["active"]["team_display"], "EDM");
        assert_eq!(json["a"]["active"]["metrics"][5]["key"], "plus_minus");
        assert!(json["b"].is_null());
    }

    #[test]
    fn depth_league_viewmodel_contract_fixture_serializes_ranked_rows() {
        use crate::fixtures;
        use crate::model::Position;
        use crate::stats_repository::StatsRepository;

        let mut repo = StatsRepository::new();
        for (id, name, team, pos, pace) in [
            (1, "Edmonton Center", "EDM", Position::Center, 120.0),
            (2, "Edmonton Left", "EDM", Position::LeftWing, 90.0),
            (3, "Seattle Center", "SEA", Position::Center, 60.0),
        ] {
            let normalized = crate::name::normalize_name(name);
            repo.upsert_identity(fixtures::identity(id).name(name, &normalized).build())
                .unwrap();
            let mut stats = fixtures::stats(id, 20242025, team).position(pos).build();
            if let Some(ref mut pace_score) = stats.totals.pace_score {
                pace_score.pace_82 = pace;
            }
            repo.upsert_stats(stats).unwrap();
        }

        let mut view =
            DepthLeagueView::pace_from_repository(&repo, Season(20242025), SeasonType::Regular);
        view.context.data_generation = Some("ted-depth-league-fixture-v1".to_string());

        let json = serde_json::to_value(&view).expect("serialize depth league view");

        assert_eq!(json["context"]["window"]["season"], 20242025);
        assert_eq!(
            json["context"]["data_generation"],
            "ted-depth-league-fixture-v1"
        );
        assert_eq!(json["scoring_mode"], "Pts/82");
        assert_eq!(json["rows"][0]["team"], "EDM");
        assert_eq!(json["rows"][0]["c_top"], "Edmonton Center");
        assert_eq!(json["rows"][0]["total"], 210.0);
        assert_eq!(json["rows"][1]["team"], "SEA");
    }

    #[test]
    fn scores_viewmodel_contract_fixture_groups_dates_and_status() {
        let context = ViewContext::new(ViewWindow::new(Season(20242025), SeasonType::Regular));
        let view = ScoresView::from_games(
            context,
            chrono::NaiveDate::from_ymd_opt(2024, 10, 8).unwrap(),
            chrono::NaiveDate::from_ymd_opt(2024, 10, 8).unwrap(),
            crate::timeframe::Timeframe::Day,
            vec![ScheduledGameInput {
                game_id: 2024030411,
                date: "2024-10-08".to_string(),
                game_type: 3,
                away_abbrev: "EDM".to_string(),
                away_name: "Oilers".to_string(),
                home_abbrev: "FLA".to_string(),
                home_name: "Panthers".to_string(),
                start_time_utc: "2024-10-08T23:00:00Z".to_string(),
                away_score: Some(3),
                home_score: Some(2),
                game_state: Some("FINAL".to_string()),
                last_period: Some("OT".to_string()),
                series_game: Some("Game 4".to_string()),
                away_wins: Some(1),
                home_wins: Some(2),
            }],
        );

        let json = serde_json::to_value(&view).expect("serialize scores view");

        assert_eq!(json["context"]["window"]["season"], 20242025);
        assert_eq!(json["range"], "day");
        assert_eq!(json["total_games"], 1);
        assert_eq!(json["days"][0]["date"], "2024-10-08");
        assert_eq!(json["days"][0]["rows"][0]["game_id"], 2024030411);
        assert_eq!(json["days"][0]["rows"][0]["state_label"], "FINAL/OT");
        assert_eq!(json["days"][0]["rows"][0]["state_class"], "final");
        assert_eq!(
            json["days"][0]["rows"][0]["start_time_utc"],
            "2024-10-08T23:00:00Z"
        );
        assert_eq!(
            json["days"][0]["rows"][0]["series_context"],
            "Game 4 · FLA leads 2-1"
        );
    }

    #[test]
    fn schedule_viewmodel_contract_fixture_projects_team_perspective() {
        let context = ViewContext::new(ViewWindow::new(Season(20242025), SeasonType::Regular));
        let view = ScheduleView::from_games(
            context,
            "20242025".to_string(),
            "EDM".to_string(),
            None,
            &["EDM", "SEA"],
            vec![ScheduledGameInput {
                game_id: 2024020001,
                date: "2024-10-08".to_string(),
                game_type: 2,
                away_abbrev: "SEA".to_string(),
                away_name: "Kraken".to_string(),
                home_abbrev: "EDM".to_string(),
                home_name: "Oilers".to_string(),
                start_time_utc: "2024-10-08T23:00:00Z".to_string(),
                away_score: Some(2),
                home_score: Some(3),
                game_state: Some("FINAL".to_string()),
                last_period: Some("SO".to_string()),
                series_game: None,
                away_wins: None,
                home_wins: None,
            }],
        );

        let json = serde_json::to_value(&view).expect("serialize schedule view");

        assert_eq!(json["season_pretty"], "2024-25");
        assert_eq!(json["active_team"], "EDM");
        assert_eq!(json["team_chips"][0]["is_active"], true);
        assert_eq!(json["rows"][0]["game_id"], 2024020001);
        assert_eq!(json["rows"][0]["start_time_utc"], "2024-10-08T23:00:00Z");
        assert_eq!(json["rows"][0]["home_or_away"], "Home");
        assert_eq!(json["rows"][0]["opponent_abbrev"], "SEA");
        assert_eq!(json["rows"][0]["state_label"], "FINAL/SO");
        assert_eq!(json["rows"][0]["series_context"], "");
    }

    #[test]
    fn schedule_team_viewmodel_contract_fixture_projects_record_and_rows() {
        let context = ViewContext::new(ViewWindow::new(Season(20242025), SeasonType::Regular));
        let view = ScheduleTeamView::from_games(
            context,
            "20242025".to_string(),
            "SEA".to_string(),
            vec![
                ScheduledGameInput {
                    game_id: 2024020001,
                    date: "2024-10-08".to_string(),
                    game_type: 2,
                    away_abbrev: "SEA".to_string(),
                    away_name: "Kraken".to_string(),
                    home_abbrev: "EDM".to_string(),
                    home_name: "Oilers".to_string(),
                    start_time_utc: "2024-10-08T23:00:00Z".to_string(),
                    away_score: Some(3),
                    home_score: Some(4),
                    game_state: Some("FINAL".to_string()),
                    last_period: Some("SO".to_string()),
                    series_game: None,
                    away_wins: None,
                    home_wins: None,
                },
                ScheduledGameInput {
                    game_id: 2024020002,
                    date: "2024-10-10".to_string(),
                    game_type: 2,
                    away_abbrev: "VAN".to_string(),
                    away_name: "Canucks".to_string(),
                    home_abbrev: "SEA".to_string(),
                    home_name: "Kraken".to_string(),
                    start_time_utc: "2024-10-10T23:00:00Z".to_string(),
                    away_score: Some(1),
                    home_score: Some(5),
                    game_state: Some("FINAL".to_string()),
                    last_period: Some("REG".to_string()),
                    series_game: None,
                    away_wins: None,
                    home_wins: None,
                },
            ],
        );

        let json = serde_json::to_value(&view).expect("serialize schedule team view");

        assert_eq!(json["team"], "SEA");
        assert_eq!(json["record"]["wins"], 1);
        assert_eq!(json["record"]["losses"], 0);
        assert_eq!(json["record"]["overtime_losses"], 1);
        assert_eq!(json["record"]["played"], 2);
        assert_eq!(json["rows"][0]["opponent_abbrev"], "EDM");
        assert_eq!(json["rows"][1]["home_or_away"], "Home");
    }

    #[test]
    fn team_season_viewmodel_contract_fixture_projects_schedule_derived_performance() {
        let context = ViewContext::new(ViewWindow::new(Season(20242025), SeasonType::Regular));
        let view = TeamSeasonView::from_games(
            context,
            "20242025".to_string(),
            "SEA".to_string(),
            vec![
                ScheduledGameInput {
                    game_id: 2024020001,
                    date: "2024-10-08".to_string(),
                    game_type: 2,
                    away_abbrev: "SEA".to_string(),
                    away_name: "Kraken".to_string(),
                    home_abbrev: "EDM".to_string(),
                    home_name: "Oilers".to_string(),
                    start_time_utc: "2024-10-08T23:00:00Z".to_string(),
                    away_score: Some(3),
                    home_score: Some(4),
                    game_state: Some("FINAL".to_string()),
                    last_period: Some("SO".to_string()),
                    series_game: None,
                    away_wins: None,
                    home_wins: None,
                },
                ScheduledGameInput {
                    game_id: 2024020002,
                    date: "2024-10-10".to_string(),
                    game_type: 2,
                    away_abbrev: "VAN".to_string(),
                    away_name: "Canucks".to_string(),
                    home_abbrev: "SEA".to_string(),
                    home_name: "Kraken".to_string(),
                    start_time_utc: "2024-10-10T23:00:00Z".to_string(),
                    away_score: Some(1),
                    home_score: Some(5),
                    game_state: Some("FINAL".to_string()),
                    last_period: Some("REG".to_string()),
                    series_game: None,
                    away_wins: None,
                    home_wins: None,
                },
                ScheduledGameInput {
                    game_id: 2024020003,
                    date: "2024-10-12".to_string(),
                    game_type: 2,
                    away_abbrev: "SEA".to_string(),
                    away_name: "Kraken".to_string(),
                    home_abbrev: "CGY".to_string(),
                    home_name: "Flames".to_string(),
                    start_time_utc: "2024-10-12T23:00:00Z".to_string(),
                    away_score: Some(1),
                    home_score: Some(3),
                    game_state: Some("FINAL".to_string()),
                    last_period: Some("REG".to_string()),
                    series_game: None,
                    away_wins: None,
                    home_wins: None,
                },
                ScheduledGameInput {
                    game_id: 2024020004,
                    date: "2024-10-14".to_string(),
                    game_type: 2,
                    away_abbrev: "SEA".to_string(),
                    away_name: "Kraken".to_string(),
                    home_abbrev: "LAK".to_string(),
                    home_name: "Kings".to_string(),
                    start_time_utc: "2024-10-14T23:00:00Z".to_string(),
                    away_score: None,
                    home_score: None,
                    game_state: Some("FUT".to_string()),
                    last_period: None,
                    series_game: None,
                    away_wins: None,
                    home_wins: None,
                },
            ],
        );

        let json = serde_json::to_value(&view).expect("serialize team season view");

        assert_eq!(json["team"], "SEA");
        assert_eq!(json["headline"]["record"]["wins"], 1);
        assert_eq!(json["headline"]["record"]["losses"], 1);
        assert_eq!(json["headline"]["record"]["overtime_losses"], 1);
        assert_eq!(json["headline"]["points"], 3);
        assert_eq!(json["headline"]["goals_for"], 9);
        assert_eq!(json["headline"]["goals_against"], 8);
        assert_eq!(json["headline"]["goal_differential"], 1);
        assert_eq!(json["splits"]["home"]["record"]["wins"], 1);
        assert_eq!(json["splits"]["away"]["record"]["losses"], 1);
        assert_eq!(json["splits"]["one_goal"]["record"]["overtime_losses"], 1);
        assert_eq!(json["form"]["last_5"]["played"], 3);
        assert_eq!(json["form"]["last_10_goal_differential"], 1);
        assert_eq!(json["remaining"]["games"], 1);
        assert_eq!(json["remaining"]["away"], 1);
        assert_eq!(json["remaining"]["next_opponents"][0], "LAK");
        assert_eq!(json["context"]["completeness"], "partial");
        assert_eq!(json["rows"][0]["venue"], "away");
        assert_eq!(json["rows"][0]["result"], "OTL");
        assert_eq!(json["rows"][1]["venue"], "home");
        assert_eq!(json["rows"][3]["result"], "SCHEDULED");
        assert_eq!(json["warnings"][0]["source"], "standings");
        assert!(view.empty_state.is_none());
    }

    #[test]
    fn team_season_viewmodel_empty_fixture_warns_and_marks_no_rows() {
        let context = ViewContext::new(ViewWindow::new(Season(20242025), SeasonType::Regular));
        let view = TeamSeasonView::from_games(
            context,
            "20242025".to_string(),
            "SEA".to_string(),
            Vec::new(),
        );

        assert_eq!(view.headline.record.played, 0);
        assert_eq!(view.headline.points, 0);
        assert_eq!(view.headline.points_percentage, 0.0);
        assert_eq!(view.remaining.games, 0);
        assert_eq!(
            view.empty_state.as_ref().map(|empty| empty.kind),
            Some(EmptyKind::NoRows)
        );
        assert_eq!(view.context.source_state[0].source, SourceKind::Schedule);
        assert_eq!(view.context.source_state[1].source, SourceKind::Standings);
    }

    #[test]
    fn team_season_viewmodel_with_standings_projects_playoff_context() {
        let context = ViewContext::new(ViewWindow::new(Season(20242025), SeasonType::Regular));
        let standings: Vec<TeamStandingInput> = (1..=8)
            .map(|rank| TeamStandingInput {
                team: format!("W{rank}"),
                conference: Some("Western".to_string()),
                division: Some("Pacific".to_string()),
                games_played: 40,
                wins: 20,
                losses: 15,
                overtime_losses: 5,
                points: 96 - rank,
                points_percentage: 0.600,
                regulation_wins: Some(18),
                goal_differential: 12,
                league_rank: Some(rank),
                conference_rank: Some(rank),
                division_rank: Some(rank.min(4)),
                wild_card_rank: (rank > 3).then(|| rank - 3),
            })
            .chain(std::iter::once(TeamStandingInput {
                team: "SEA".to_string(),
                conference: Some("Western".to_string()),
                division: Some("Pacific".to_string()),
                games_played: 40,
                wins: 22,
                losses: 13,
                overtime_losses: 5,
                points: 49,
                points_percentage: 0.613,
                regulation_wins: Some(19),
                goal_differential: 14,
                league_rank: Some(9),
                conference_rank: Some(9),
                division_rank: Some(5),
                wild_card_rank: Some(3),
            }))
            .collect();

        let view = TeamSeasonView::from_games_and_standings(
            context,
            "20242025".to_string(),
            "SEA".to_string(),
            Vec::new(),
            standings,
        );

        let standings = view.standings.as_ref().expect("standings context");
        assert_eq!(standings.conference.as_deref(), Some("Western"));
        assert_eq!(standings.points, 49);
        assert_eq!(standings.playoff_cut_points, Some(88));
        assert_eq!(standings.points_behind_cutline, Some(39));
        assert_eq!(standings.points_above_cutline, None);
        assert_eq!(standings.playoff_position_label, "wild card 3");
        assert!(view.warnings.is_empty());
        assert_eq!(view.context.source_state[1].state, Completeness::Complete);
    }

    #[test]
    fn team_season_viewmodel_with_standings_projects_sos_and_quality_ledger() {
        fn standing(team: &str, points_percentage: f32, points: u32) -> TeamStandingInput {
            TeamStandingInput {
                team: team.to_string(),
                conference: Some("Western".to_string()),
                division: Some("Pacific".to_string()),
                games_played: 40,
                wins: 20,
                losses: 15,
                overtime_losses: 5,
                points,
                points_percentage,
                regulation_wins: Some(18),
                goal_differential: 0,
                league_rank: None,
                conference_rank: None,
                division_rank: None,
                wild_card_rank: None,
            }
        }
        fn game(
            id: u64,
            date: &str,
            away: &str,
            home: &str,
            away_score: Option<u8>,
            home_score: Option<u8>,
            last_period: Option<&str>,
        ) -> ScheduledGameInput {
            ScheduledGameInput {
                game_id: id,
                date: date.to_string(),
                game_type: 2,
                away_abbrev: away.to_string(),
                away_name: away.to_string(),
                home_abbrev: home.to_string(),
                home_name: home.to_string(),
                start_time_utc: format!("{date}T23:00:00Z"),
                away_score,
                home_score,
                game_state: Some(if away_score.is_some() {
                    "FINAL".to_string()
                } else {
                    "FUT".to_string()
                }),
                last_period: last_period.map(str::to_string),
                series_game: None,
                away_wins: None,
                home_wins: None,
            }
        }

        let context = ViewContext::new(ViewWindow::new(Season(20242025), SeasonType::Regular));
        let view = TeamSeasonView::from_games_and_standings(
            context,
            "20242025".to_string(),
            "SEA".to_string(),
            vec![
                game(1, "2024-10-01", "SEA", "COL", Some(4), Some(2), Some("REG")),
                game(2, "2024-10-03", "SEA", "SJS", Some(1), Some(2), Some("REG")),
                game(3, "2024-10-05", "ANA", "SEA", Some(3), Some(2), Some("OT")),
                game(4, "2024-10-07", "SEA", "VGK", None, None, None),
            ],
            vec![
                standing("COL", 0.720, 58),
                standing("VGK", 0.690, 56),
                standing("SEA", 0.600, 48),
                standing("EDM", 0.590, 47),
                standing("ANA", 0.430, 34),
                standing("SJS", 0.350, 28),
            ],
        );

        assert_eq!(view.schedule_strength.faced_games, 3);
        assert_eq!(view.schedule_strength.remaining_games, 1);
        assert_eq!(view.schedule_strength.faced.top, 1);
        assert_eq!(view.schedule_strength.faced.bottom, 2);
        assert_eq!(view.schedule_strength.remaining.top, 1);
        assert_eq!(view.quality_ledger.quality_wins, 1);
        assert_eq!(view.quality_ledger.bad_losses, 2);
        assert_eq!(view.quality_ledger.missed_points, 3);
        assert_eq!(view.quality_ledger.expected_wins, 0);
    }

    #[test]
    fn schedule_matchup_viewmodel_contract_fixture_projects_records_and_groups() {
        let context = ViewContext::new(ViewWindow::new(Season(20242025), SeasonType::Regular));
        let view = ScheduleMatchupView::from_games(
            context,
            "20242025".to_string(),
            "NYR".to_string(),
            "WSH".to_string(),
            vec![
                ScheduledGameInput {
                    game_id: 2024020001,
                    date: "2024-11-15".to_string(),
                    game_type: 2,
                    away_abbrev: "WSH".to_string(),
                    away_name: "Capitals".to_string(),
                    home_abbrev: "NYR".to_string(),
                    home_name: "Rangers".to_string(),
                    start_time_utc: "2024-11-15T23:00:00Z".to_string(),
                    away_score: Some(1),
                    home_score: Some(4),
                    game_state: Some("FINAL".to_string()),
                    last_period: Some("REG".to_string()),
                    series_game: None,
                    away_wins: None,
                    home_wins: None,
                },
                ScheduledGameInput {
                    game_id: 2024030001,
                    date: "2025-04-24".to_string(),
                    game_type: 3,
                    away_abbrev: "NYR".to_string(),
                    away_name: "Rangers".to_string(),
                    home_abbrev: "WSH".to_string(),
                    home_name: "Capitals".to_string(),
                    start_time_utc: "2025-04-24T23:00:00Z".to_string(),
                    away_score: Some(2),
                    home_score: Some(5),
                    game_state: Some("FINAL".to_string()),
                    last_period: Some("REG".to_string()),
                    series_game: Some("Game 3".to_string()),
                    away_wins: Some(1),
                    home_wins: Some(2),
                },
            ],
        );

        let json = serde_json::to_value(&view).expect("serialize schedule matchup view");

        assert_eq!(json["team"], "NYR");
        assert_eq!(json["opponent"], "WSH");
        assert_eq!(json["regular_record"]["wins"], 1);
        assert_eq!(json["regular_record"]["losses"], 0);
        assert_eq!(json["playoff_record"]["wins"], 0);
        assert_eq!(json["playoff_record"]["losses"], 1);
        assert_eq!(json["regular_rows"][0]["game_id"], 2024020001);
        assert_eq!(json["playoff_rows"][0]["series_game"], "Game 3");
    }

    #[test]
    fn transactions_viewmodel_contract_fixture_filters_and_formats_rows() {
        use crate::model::TeamAbbr;
        use crate::transactions::{Transaction, TransactionKind};

        let context = ViewContext::new(ViewWindow::new(Season(20242025), SeasonType::Regular));
        let view = TransactionsView::from_rows(
            context,
            "20242025".to_string(),
            vec![
                Transaction {
                    date: "2024-10-09".to_string(),
                    team: Some(TeamAbbr("EDM".to_string())),
                    description: "Edmonton Oilers acquired a player".to_string(),
                    kind: TransactionKind::Trade,
                    id: "tx-edm-trade".to_string(),
                    trade_group_id: None,
                    classifier_version: crate::transactions::CURRENT_CLASSIFIER_VERSION,
                },
                Transaction {
                    date: "2024-10-08".to_string(),
                    team: Some(TeamAbbr("SEA".to_string())),
                    description: "Seattle Kraken recalled a player".to_string(),
                    kind: TransactionKind::Recall,
                    id: "tx-sea-recall".to_string(),
                    trade_group_id: None,
                    classifier_version: crate::transactions::CURRENT_CLASSIFIER_VERSION,
                },
            ],
            Some(&[TransactionKind::Trade]),
            "trade".to_string(),
            Some(TeamAbbr("EDM".to_string())),
            false,
        );

        let json = serde_json::to_value(&view).expect("serialize transactions view");

        assert_eq!(json["season_pretty"], "2024-25");
        assert_eq!(json["active_kind"], "trade");
        assert_eq!(json["active_team"], "EDM");
        assert_eq!(json["total"], 1);
        assert_eq!(json["rows"][0]["kind_label"], "trade");
        assert_eq!(json["rows"][0]["kind_pretty"], "Trade");
        assert_eq!(json["rows"][0]["team"], "EDM");
        assert_eq!(json["rows"][0]["id"], "tx-edm-trade");
    }

    #[test]
    fn transactions_viewmodel_contract_fixture_filters_league_bucket() {
        use crate::model::TeamAbbr;
        use crate::transactions::{Transaction, TransactionKind};

        let context = ViewContext::new(ViewWindow::new(Season(20242025), SeasonType::Regular));
        let view = TransactionsView::from_rows(
            context,
            "20242025".to_string(),
            vec![
                Transaction {
                    date: "2024-10-09".to_string(),
                    team: Some(TeamAbbr("EDM".to_string())),
                    description: "Edmonton Oilers acquired a player".to_string(),
                    kind: TransactionKind::Trade,
                    id: "tx-edm-trade".to_string(),
                    trade_group_id: None,
                    classifier_version: crate::transactions::CURRENT_CLASSIFIER_VERSION,
                },
                Transaction {
                    date: "2024-10-08".to_string(),
                    team: None,
                    description: "League-wide transaction".to_string(),
                    kind: TransactionKind::Other,
                    id: "tx-league".to_string(),
                    trade_group_id: None,
                    classifier_version: crate::transactions::CURRENT_CLASSIFIER_VERSION,
                },
            ],
            None,
            String::new(),
            Some(TeamAbbr("LEAGUE".to_string())),
            false,
        );

        assert_eq!(view.active_team, "LEAGUE");
        assert_eq!(view.total, 1);
        assert_eq!(view.rows[0].id, "tx-league");
        assert_eq!(view.rows[0].team, "");
    }

    #[test]
    fn game_viewmodel_contract_fixture_projects_status_and_top_skaters() {
        let context = ViewContext::new(ViewWindow::new(Season(20242025), SeasonType::Regular));
        let view = GameView::from_boxscore(
            context,
            GameBoxscoreInput {
                game_id: 2024020001,
                away_abbrev: "SEA".to_string(),
                home_abbrev: "EDM".to_string(),
                away_score: 2,
                home_score: 3,
                game_state: Some("FINAL".to_string()),
                last_period: Some("OT".to_string()),
                goals: vec![GameGoalInput {
                    period: 4,
                    period_type: "OT".to_string(),
                    time_in_period: "01:10".to_string(),
                    scorer_team: "EDM".to_string(),
                    scorer_name: "Connor McDavid".to_string(),
                    assist1_name: Some("Leon Draisaitl".to_string()),
                    assist2_name: None,
                    away_score: 2,
                    home_score: 3,
                }],
                goalies: vec![GameGoalieInput {
                    player_id: 1,
                    player_name: "Goalie".to_string(),
                    team_abbrev: "EDM".to_string(),
                    saves: 30,
                    shots: 32,
                    decision: Some("W".to_string()),
                }],
                away_skaters: vec![GameSkaterInput {
                    player_id: 2,
                    player_name: "Away Skater".to_string(),
                    position: "C".to_string(),
                    toi_seconds: 1200,
                    sog: 2,
                    hits: 1,
                    blocked_shots: 0,
                    takeaways: 0,
                    giveaways: 1,
                    goals: 1,
                    assists: 0,
                    plus_minus: 1,
                }],
                home_skaters: vec![
                    GameSkaterInput {
                        player_id: 3,
                        player_name: "Depth Skater".to_string(),
                        position: "R".to_string(),
                        toi_seconds: 900,
                        sog: 1,
                        hits: 2,
                        blocked_shots: 0,
                        takeaways: 0,
                        giveaways: 0,
                        goals: 0,
                        assists: 1,
                        plus_minus: 0,
                    },
                    GameSkaterInput {
                        player_id: 4,
                        player_name: "Top Skater".to_string(),
                        position: "C".to_string(),
                        toi_seconds: 1300,
                        sog: 4,
                        hits: 1,
                        blocked_shots: 1,
                        takeaways: 2,
                        giveaways: 0,
                        goals: 1,
                        assists: 2,
                        plus_minus: 2,
                    },
                ],
            },
        );

        let json = serde_json::to_value(&view).expect("serialize game view");

        assert_eq!(json["game_id"], 2024020001);
        assert_eq!(json["state_label"], "Final/OT");
        assert_eq!(json["is_live"], false);
        assert_eq!(json["auto_refresh"], false);
        assert_eq!(json["goals"][0]["scorer_name"], "Connor McDavid");
        assert_eq!(json["goals"][0]["period_type"], "OT");
        assert_eq!(json["goals"][0]["assist1_name"], "Leon Draisaitl");
        assert_eq!(json["goalies"][0]["team_abbrev"], "EDM");
        assert_eq!(json["home_top_skaters"][0]["player_name"], "Top Skater");
        assert_eq!(json["home_top_skaters"][0]["points"], 3);
    }
}
