#![deny(unsafe_code)]

/// Current NHL season identifier — update each October.
/// Format: YYYYZZZZ where YYYY = start year, ZZZZ = end year.
pub const CURRENT_SEASON: u32 = 20_252_026;
pub const CURRENT_SEASON_STR: &str = "20252026";

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
pub mod scheme;
pub mod scoring;
pub mod season_stats;
pub mod series_momentum;
pub mod stats_catalog;
pub mod stats_repository;
pub mod teams;
pub mod timeframe;
pub mod transactions;
pub mod view_model;

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
pub use scheme::{compute_fantasy_score, FantasyScore, Scheme, SkaterStats as SchemeSkaterStats};
pub use scoring::{classify_fit, compute_pace_score, sort_views_by_pace};
pub use season_stats::SeasonStatsBuildError;
pub use teams::CANONICAL_TEAMS;
pub use transactions::{
    classify, other_rate, sanitize_description, trade_group_id, Transaction, TransactionKind,
    CURRENT_CLASSIFIER_VERSION, TRANSACTIONS_EARLIEST_SEASON,
};
pub use view_model::{
    build_fantasy_simulation_view, fantasy_roster_games_played, fantasy_roster_games_remaining,
    find_fantasy_roster_player, goalie_scheme_stats_from_view, project_fantasy_roster_score,
    project_fantasy_scenario, resolve_fantasy_scenario_roster,
    resolve_fantasy_scenario_roster_details, score_fantasy_roster, scouting_report_sections,
    skater_scheme_stats_from_view, watch_rules_view_with_persisted, AppliedFilter, CareerRow,
    CareerSortKey, CareerView, CompareView, Completeness, ConfigEntryInput, ConfigEntryRow,
    ConfigMutationIntent, ConfigView, DataMutationIntent, DataMutationOperation,
    DataStatusEntryInput, DataStatusRow, DataStatusView, DepthGoalieSlot, DepthLeagueView,
    DepthTeamStrengthRow, DocsView, EmptyKind, EmptyState, FantasyLeagueInput, FantasyLeagueRow,
    FantasyLeagueTeamInput, FantasyLeagueTeamRow, FantasyLeagueView, FantasyRosterGapAction,
    FantasyRosterGapCandidate, FantasyRosterGapInput, FantasyRosterGapReplacement,
    FantasyRosterGapRow, FantasyRosterGapView, FantasyRosterScore, FantasyScenarioRosterResolution,
    FantasySimulationAction, FantasySimulationBuildInput, FantasySimulationConfidence,
    FantasySimulationHorizon, FantasySimulationInput, FantasySimulationRosterTeamInput,
    FantasySimulationScenarioInput, FantasySimulationScenarioRosterInput,
    FantasySimulationScenarioRow, FantasySimulationTeamInput, FantasySimulationTeamRow,
    FantasySimulationView, FavoriteMemberInput, FavoriteMemberRow, FavoriteMutationIntent,
    FavoritesView, FilterKey, FilterOp, GameBoxscoreInput, GameGoalInput, GameGoalRow,
    GameGoalieInput, GameGoalieRow, GameSkaterInput, GameSkaterRow, GameView,
    GoalieLeaderboardSort, GoalieRoleFilter, GoalieRoleSignal, GoalieRow, GoaliesView,
    HomeGoalieRow, HomeSkaterRow, HomeView, LeaderKind, LeaderRow, LeadersView, MetricCell,
    MetricUnit, MetricValue, MutationResultView, MutationStatus, OpponentTierBreakdown,
    PlayerCardView, PlayerCareerSummary, PlayerPreNhlCareerRow, PlayerSeasonSummary,
    PlayoffsBracketInput, PlayoffsGameInput, PlayoffsGameRow, PlayoffsRoundInput, PlayoffsRoundRow,
    PlayoffsSeriesInput, PlayoffsSeriesRow, PlayoffsView, RecoveryAction, ReportContext,
    ReportFormat, ReportKind, ReportSectionRef, ReportView, ScheduleGameRow, ScheduleMatchupRecord,
    ScheduleMatchupView, ScheduleRecord, ScheduleTeamView, ScheduleView, ScheduledGameInput,
    ScoreGameRow, ScoresDayView, ScoresView, SeasonTypeMutationIntent, SemanticToken,
    SimilarPlayerRow, SimilarPlayerTarget, SimilarPlayersView, SnapshotEntryInput,
    SnapshotMutationIntent, SnapshotMutationOperation, SnapshotRow, SnapshotView, SortDirection,
    SortKey, SortState, SourceKind, SourceProvenance, SourceState, StatKey, TeamChipView,
    TeamDepthChartColumn, TeamDepthChartPlayer, TeamDepthChartView, TeamDepthView,
    TeamQualityLedger, TeamRecentForm, TeamRemainingSchedule, TeamScheduleStrength,
    TeamSeasonGameRow, TeamSeasonHeadline, TeamSeasonSplit, TeamSeasonSplits, TeamSeasonVenue,
    TeamSeasonView, TeamStandingInput, TeamStandingsContext, TeamTradeImpactView, TradeImpactLine,
    TradeImpactPair, TradeImpactPlayer, TradeImpactSlot, TransactionViewRow, TransactionsView,
    ValuePrecision, ViewContext, ViewWarning, ViewWindow, WarningKind, WatchNoteInput,
    WatchRuleMutationIntent, WatchRuleMutationOperation, WatchlistMemberRow, WatchlistView,
    ALL_SEMANTIC_TOKENS, CAREER_HISTORY_FETCH_COMMAND, CAREER_HISTORY_MISSING_STORE_MESSAGE,
    CAREER_HISTORY_STORE_PATH,
};
