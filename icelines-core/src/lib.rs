#![deny(unsafe_code)]

/// Current NHL season identifier — update each October.
/// Format: YYYYZZZZ where YYYY = start year, ZZZZ = end year.
pub const CURRENT_SEASON: u32 = 20_262_027;
pub const CURRENT_SEASON_STR: &str = "20262027";

pub mod analytics_cache;
pub mod career_history;
pub mod contract;
pub mod cross_team;
pub mod depth_chart;
pub mod entity;
pub mod error;
pub mod event_stream;
pub mod favorites;
pub mod filter;
pub mod fixtures;
pub mod freshness;
pub mod history;
pub mod identity;
pub mod live_game;
pub mod model;
pub mod name;
pub mod playoff_run;
pub mod position;
pub mod position_profile;
pub mod projection;
pub mod roster_shape;
pub mod scheme;
pub mod scoring;
pub mod season_stats;
pub mod series_momentum;
pub mod signal_metrics;
pub mod stats_catalog;
pub mod stats_repository;
pub mod teams;
pub mod timeframe;
pub mod transactions;
pub mod view_model;
pub mod workbench;
pub mod workbench_layout;

pub use analytics_cache::{
    analytics_cache_consumer_envelope, analytics_cache_read_disposition,
    build_analytics_cache_record, parse_analytics_cache_record_json, AnalyticsCacheBuildInput,
    AnalyticsCacheConsumerEnvelope, AnalyticsCacheConsumerKind, AnalyticsCacheEntity,
    AnalyticsCacheError, AnalyticsCacheFilter, AnalyticsCacheInvalidation, AnalyticsCacheMetric,
    AnalyticsCacheQuality, AnalyticsCacheReadDisposition, AnalyticsCacheRecord,
    AnalyticsCacheScope, AnalyticsCacheSourceWindow, ANALYTICS_CACHE_CONSUMER_CONTRACT_VERSION,
    ANALYTICS_CACHE_SCHEMA_VERSION,
};
pub use career_history::{CareerGameType, CareerHistory, CareerStint, LeagueAbbrev, LeagueTier};
pub use cross_team::{
    compute_all_views as compute_cross_team_metrics_views, CrossTeamMetrics, WebFitClass,
};
pub use depth_chart::DepthChartBuilder;
pub use entity::{EntityRef, EntityRefError};
pub use error::IcelinesError;
pub use filter::PlayerFilter;
pub use freshness::{Clock, FetchSource, Freshness, MockClock, SystemClock, Ttl};
pub use history::{CareerSummary, SeasonLine};
pub use identity::{GameIdError, PlayerIdError};
pub use model::{
    DepthChart, DepthChartSlot, FitClass, GpStatus, LineAssignment, PaceScore, Position, Region,
    Season, SeasonParseError, Slot, TeamAbbr,
};
pub use name::normalize_name;
pub use position::PositionResolver;
pub use position_profile::PositionProfile;
pub use projection::{
    age_factor, compute_alpha, compute_projection, ProjectionMode, ProjectionResult,
};
pub use roster_shape::{
    RosterPositionGroup, RosterShape, RosterShapeIssueKind, RosterShapePlayerInput,
    RosterShapePlayerIssue, RosterShapeSlotRow, RosterShapeStatus, RosterShapeSummary,
    RosterShapeValidationInput, RosterShapeValidationView, RosterSlotRule, RosterSlotStatus,
};
pub use scheme::{compute_fantasy_score, FantasyScore, Scheme, SkaterStats as SchemeSkaterStats};
pub use scoring::{classify_fit, compute_pace_score, sort_views_by_pace};
pub use season_stats::SeasonStatsBuildError;
pub use signal_metrics::{
    SignalEvidence, SignalEvidenceTier, SignalInput, SignalMetricDescriptor, SignalMetricId,
    SignalMetricUnit, SignalPolarity,
};
pub use teams::CANONICAL_TEAMS;
pub use transactions::{
    classify, other_rate, sanitize_description, trade_group_id, Transaction, TransactionKind,
    CURRENT_CLASSIFIER_VERSION, TRANSACTIONS_EARLIEST_SEASON,
};
pub use view_model::{
    analytics_cache_consumer_title, apply_fantasy_pickup_reserve, build_cap_projection,
    build_fantasy_category_matchup, build_fantasy_daily_lineup, build_fantasy_draft_board,
    build_fantasy_goalie_plan, build_fantasy_matchup_strategy, build_fantasy_morning_briefing,
    build_fantasy_schedule_view, build_fantasy_simulation_view, build_fantasy_sleeper_board,
    build_fantasy_week_budget, build_fantasy_weekly_pickups,
    build_fantasy_weekly_pickups_with_reserve_override, build_team_ceiling, classify_cap_role,
    fantasy_acquisition_availability, fantasy_roster_games_played, fantasy_roster_games_remaining,
    fantasy_waiver_window, find_fantasy_roster_player, goalie_scheme_stats_from_view,
    import_fantasy_platform_eligibility, import_fantasy_taken_players,
    project_fantasy_roster_score, project_fantasy_scenario, resolve_fantasy_goalie_start,
    resolve_fantasy_player_status, resolve_fantasy_scenario_roster,
    resolve_fantasy_scenario_roster_details, score_fantasy_roster, scouting_report_sections,
    simulate_fantasy_season, skater_scheme_stats_from_view, watch_rules_view_with_persisted,
    AnalyticsCacheConsumerMetricRow, AnalyticsCacheConsumerView, AppliedFilter, CapLimitAuthority,
    CapLimitProjection, CapPressure, CapProjectionAssumptions, CapProjectionContractInput,
    CapProjectionError, CapProjectionPlayerInput, CapProjectionRole, CapProjectionView, CareerRow,
    CareerSortKey, CareerView, CompareView, Completeness, ConfigEntryInput, ConfigEntryRow,
    ConfigMutationIntent, ConfigView, DataMutationIntent, DataMutationOperation,
    DataStatusEntryInput, DataStatusRow, DataStatusView, DepthGoalieSlot, DepthLeagueView,
    DepthTeamStrengthRow, DocsView, EmptyKind, EmptyState, FantasyAcquisitionAvailability,
    FantasyAcquisitionInput, FantasyAcquisitionKind, FantasyActiveSlot, FantasyActiveSlotKind,
    FantasyAssistantRules, FantasyCategoryAggregation, FantasyCategoryClassification,
    FantasyCategoryDirection, FantasyCategoryMatchupInput, FantasyCategoryMatchupRow,
    FantasyCategoryMatchupView, FantasyCategoryPlayerInput, FantasyCategoryProjectedResult,
    FantasyCategoryProjectedValue, FantasyCategoryRateInput, FantasyCategoryRule,
    FantasyCategoryScope, FantasyCategorySnapshotComponents, FantasyCategorySnapshotInput,
    FantasyCategorySnapshotRow, FantasyCategoryTeamInput, FantasyCompetitionMode,
    FantasyCompetitionRules, FantasyDailyDeltaInput, FantasyDailyDeltaView, FantasyDailyLineInput,
    FantasyDailyLineupView, FantasyDailyPlayerInput, FantasyDailyPlayerRow,
    FantasyDailyPlayerStatus, FantasyDailyScore, FantasyDailySlateRow, FantasyDailyTeamInput,
    FantasyDailyTeamRow, FantasyDraftBoardView, FantasyDraftCandidateInput,
    FantasyDraftCandidateRow, FantasyDraftIdentityInput, FantasyDraftPositionLeader,
    FantasyDraftValueComponents, FantasyEligibilityImportRow, FantasyEligibilityImportStatus,
    FantasyEligibilityImportView, FantasyGoalieGameInput, FantasyGoaliePlanAction,
    FantasyGoaliePlanInput, FantasyGoaliePlanPlayerInput, FantasyGoaliePlanRow,
    FantasyGoaliePlanView, FantasyGoaliePortfolioComparison, FantasyGoalieRefreshUrgency,
    FantasyGoalieStartObservation, FantasyGoalieStartState, FantasyGoalieStreamCandidateRow,
    FantasyImportMode, FantasyImportPlayerRow, FantasyImportRowInput, FantasyImportRowStatus,
    FantasyImportSummary, FantasyImportTeamInput, FantasyImportTeamRow, FantasyImportTeamStatus,
    FantasyImportView, FantasyImportViewInput, FantasyInjuryPlanView, FantasyLeagueInput,
    FantasyLeagueRow, FantasyLeagueTeamInput, FantasyLeagueTeamRow, FantasyLeagueView,
    FantasyLineupAssignmentRow, FantasyLineupPlayerInput, FantasyMarketStatus,
    FantasyMatchupOutcome, FantasyMatchupPointsSnapshotInput, FantasyMatchupRow,
    FantasyMatchupScheduleInput, FantasyMatchupSideRow, FantasyMatchupStrategy,
    FantasyMatchupStrategyInput, FantasyMatchupStrategyPlayerInput,
    FantasyMatchupStrategyTeamInput, FantasyMatchupStrategyView, FantasyMatchupSwingInput,
    FantasyMatchupTeamProjection, FantasyMatchupTeamRow, FantasyMatchupTeamTotalInput,
    FantasyMatchupTiePolicy, FantasyMatchupWeekInput, FantasyMatchupWeekView, FantasyMorningAction,
    FantasyMorningActionKind, FantasyMorningBriefingView, FantasyObservationConfidence,
    FantasyObservationFreshness, FantasyPlayerAvailabilityStatus, FantasyReserveAssignmentRow,
    FantasyResolvedGoalieStart, FantasyResolvedPlayerStatus, FantasyRosterGapAction,
    FantasyRosterGapCandidate, FantasyRosterGapInput, FantasyRosterGapReplacement,
    FantasyRosterGapRow, FantasyRosterGapView, FantasyRosterScheduleView, FantasyRosterScore,
    FantasyScenarioRosterResolution, FantasyScheduleClassRow, FantasyScheduleComplementRow,
    FantasyScheduleGameInput, FantasyScheduleOverlapRow, FantasyScheduleTeamRow,
    FantasyScheduleView, FantasyScheduleWeekRow, FantasySeasonEventKind, FantasySeasonEventRow,
    FantasySeasonSimConfig, FantasySeasonSimPlayerInput, FantasySeasonSimTeamRow,
    FantasySeasonSimView, FantasySimulationAction, FantasySimulationBuildInput,
    FantasySimulationConfidence, FantasySimulationHorizon, FantasySimulationInput,
    FantasySimulationRosterTeamInput, FantasySimulationScenarioInput,
    FantasySimulationScenarioRosterInput, FantasySimulationScenarioRow, FantasySimulationTeamInput,
    FantasySimulationTeamRow, FantasySimulationView, FantasySleeperBoardView,
    FantasySleeperComponents, FantasySleeperConfidence, FantasySleeperInput, FantasySleeperRow,
    FantasyStatusObservation, FantasyTakenImportView, FantasyTakenPlayerRow,
    FantasyTakenResolutionStatus, FantasyWaiverWindow, FantasyWeekBudgetView,
    FantasyWeeklyMoveInput, FantasyWeeklyMoveRow, FantasyWeeklyPickupView, FantasyWeeklyTeamRow,
    FavoriteMemberInput, FavoriteMemberRow, FavoriteMutationIntent, FavoritesView,
    FightRecordInput, FilterKey, FilterOp, GameBoxscoreInput, GameGoalInput, GameGoalRow,
    GameGoalieInput, GameGoalieRow, GameScoringReportView, GameSkaterInput, GameSkaterRow,
    GameView, GoalieLeaderboardSort, GoalieRoleFilter, GoalieRoleSignal, GoalieRow, GoaliesView,
    HomeGoalieRow, HomeSkaterRow, HomeView, InsideShotBucket, InsideShotBucketCounts,
    InsideShotProxy, LeaderKind, LeaderRow, LeadersView, MetricCell, MetricUnit, MetricValue,
    MutationResultView, MutationStatus, OpponentTierBreakdown, PlayerAwardRow,
    PlayerAwardSeasonRow, PlayerAwardsView, PlayerCapProjection, PlayerCardView,
    PlayerCareerSummary, PlayerGameLineInput, PlayerGoalRecordInput, PlayerPreNhlCareerRow,
    PlayerRecordsView, PlayerScoringPaceMetric, PlayerScoringPaceRow,
    PlayerScoringPaceSampleStatus, PlayerScoringPaceView, PlayerScoringProfileView,
    PlayerScoringTrendRow, PlayerScoringTrendWindow, PlayerSeasonSummary, PlayerShotLineInput,
    PlayerStreakRow, PlayerStreaksView, PlayoffsBracketInput, PlayoffsGameInput, PlayoffsGameRow,
    PlayoffsRoundInput, PlayoffsRoundRow, PlayoffsSeriesInput, PlayoffsSeriesRow, PlayoffsView,
    RecordsOpponentRow, RecoveryAction, ReportContext, ReportFormat, ReportKind, ReportSectionRef,
    ReportView, SalaryBasis, ScheduleGameRow, ScheduleMatchupRecord, ScheduleMatchupView,
    ScheduleRecord, ScheduleTeamView, ScheduleView, ScheduledGameInput, ScoreGameRow,
    ScoresDayView, ScoresView, ScoringEventInput, ScoringEventSummary, ScoringShooterSummary,
    ScoringSplitSummary, SeasonTypeMutationIntent, SemanticToken, ShotEventKind, ShotLocation,
    SignalRosterEvidenceFilter, SignalsRosterView, SimilarPlayerRow, SimilarPlayerTarget,
    SimilarPlayersView, SnapshotEntryInput, SnapshotMutationIntent, SnapshotMutationOperation,
    SnapshotRow, SnapshotView, SortDirection, SortKey, SortState, SourceKind, SourceProvenance,
    SourceState, StatKey, StrengthState, TeamCapProjection, TeamCeilingError, TeamCeilingLens,
    TeamCeilingLensScore, TeamCeilingPlayerInput, TeamCeilingPlayerRow, TeamCeilingRow,
    TeamCeilingView, TeamChipView, TeamDepthChartColumn, TeamDepthChartPlayer, TeamDepthChartView,
    TeamDepthView, TeamPlayerStreakLeaderRow, TeamPlayerStreaksView, TeamQualityLedger,
    TeamRecentForm, TeamRecordsView, TeamRemainingSchedule, TeamScheduleStrength,
    TeamScoringOutlookMetric, TeamScoringOutlookRecentForm, TeamScoringOutlookRow,
    TeamScoringOutlookSampleStatus, TeamScoringOutlookSourceStatus, TeamScoringOutlookView,
    TeamScoringProfileView, TeamSeasonCapProjection, TeamSeasonGameRow, TeamSeasonHeadline,
    TeamSeasonSplit, TeamSeasonSplits, TeamSeasonVenue, TeamSeasonView, TeamSide,
    TeamStandingInput, TeamStandingsContext, TeamTradeImpactView, TonightFavoritePlayerScoringRow,
    TonightFavoriteTeamScoringRow, TonightScoringIntelView, TradeImpactLine, TradeImpactPair,
    TradeImpactPlayer, TradeImpactSlot, TransactionViewRow, TransactionsView, ValuePrecision,
    ViewContext, ViewWarning, ViewWindow, WarningKind, WatchNoteInput, WatchRuleMutationIntent,
    WatchRuleMutationOperation, WatchlistMemberRow, WatchlistView, ALL_SEMANTIC_TOKENS,
    CAREER_HISTORY_FETCH_COMMAND, CAREER_HISTORY_MISSING_STORE_MESSAGE, CAREER_HISTORY_STORE_PATH,
    FANTASY_CATEGORY_MATCHUP_SCHEMA, FANTASY_CATEGORY_SNAPSHOT_SCHEMA,
    FANTASY_COMPETITION_RULES_SCHEMA, FANTASY_GOALIE_PLAN_SCHEMA,
};
pub use workbench::{
    workbench_entry, workbench_experience, workbench_field, workbench_pane_binding,
    workbench_pane_model, WorkbenchDocumentKind, WorkbenchEntry, WorkbenchExperience,
    WorkbenchExperienceId, WorkbenchField, WorkbenchFieldId, WorkbenchFieldOperator,
    WorkbenchFieldScope, WorkbenchFieldSource, WorkbenchFieldSummary, WorkbenchGroup, WorkbenchId,
    WorkbenchPaneBinding, WorkbenchPaneBindingId, WorkbenchPaneInteraction, WorkbenchPaneKind,
    WorkbenchPaneModel, WorkbenchPaneModelId, WorkbenchRibbonScope, WorkbenchStatusScope,
    WorkbenchSurface, WorkbenchValueKind, WorkbenchZone, WORKBENCH_CATALOG, WORKBENCH_EXPERIENCES,
    WORKBENCH_FIELDS, WORKBENCH_PANE_BINDINGS, WORKBENCH_PANE_MODELS,
};
pub use workbench_layout::{
    normalize_layout_name, parse_experience_id, parse_pane_binding_id, parse_workbench_id,
    WorkbenchLayoutContextPolicy, WorkbenchLayoutError, WorkbenchLayoutRecord,
    WorkbenchLayoutStore, WORKBENCH_LAYOUT_SCHEMA_VERSION,
};
