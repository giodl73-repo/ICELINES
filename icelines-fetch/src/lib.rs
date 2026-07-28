#![deny(unsafe_code)]

pub mod aggregate;
pub mod ahl;
pub mod ahl_organization_status;
pub mod ahl_player_value;
pub mod ahl_preseason_facts;
pub mod ahl_professional_games;
pub mod ahl_prospect_status;
pub mod ahl_recall_readiness;
pub mod ahl_rollover;
pub mod ahl_transactions;
pub mod analytics_cache_store;
pub mod atomic_write;
pub mod boxscore_client;
pub mod boxscore_to_night_line;
pub mod bundled;
pub mod cache;
pub mod capwages;
pub mod career;
pub mod career_landing;
pub mod chunkstore;
pub mod contracts_csv;
pub mod csv_loader;
pub mod datastore;
pub mod error;
pub mod fantasy_daily;
pub mod fantasy_db;
pub mod fantasy_import;
pub mod fantasy_matchup;
pub mod fetch_lock;
pub mod fletch;
pub mod game_cache;
pub mod management_behavior_source;
pub mod manifest;
pub mod moneypuck;
pub mod nhl_api;
pub mod organization_window_history;
pub mod playoffs_bundle;
pub mod prospect_career;
pub mod prospect_discovery;
pub mod query_provider;
pub mod records_provider;
pub mod resolver;
pub mod scenario_registry;
pub mod schedule_remaining;
pub mod schema;
pub mod scoring_outlook_provider;
pub mod scoring_provider;
pub mod series_momentum_builder;
pub mod shift_chart;
pub mod shift_profile;
pub mod snapshot;
pub mod stats_loader;
pub mod streaks_provider;
pub mod sync_engine;
pub mod teams;
pub mod transactions;

// Hart.5c.7: PlayerRepository / GoalieRepository / Player / Goalie / player_builder
// all deleted. Every consumer reads StatsRepository + PlayerView<'_> via
// `stats_loader::load_into_repo`.

pub use analytics_cache_store::{
    AnalyticsCacheRead, AnalyticsCacheStore, AnalyticsCacheStoreError,
};
pub use boxscore_client::{aggregate_profiles, aggregate_shift_profiles, BoxscoreClient};
pub use bundled::{
    get_bios as get_bundled_bios, get_playoffs as get_bundled_playoffs,
    get_stats as get_bundled_stats, load_bios_with_fallback, load_playoffs,
    load_stats_with_fallback, BUNDLED_SEASONS,
};
pub use cache::Cache;
pub use career::load_career;
pub use error::FetchError;
pub use management_behavior_source::{
    fetch_team_behavior_league_evidence, BehaviorEvidenceSourceView,
    TeamBehaviorLeagueEvidenceView, TeamBehaviorSeasonEvidenceView,
    TEAM_BEHAVIOR_LEAGUE_EVIDENCE_SCHEMA,
};
pub use moneypuck::{parse_csv as parse_moneypuck_csv, MoneyPuckStats};
pub use nhl_api::NhlApiClient;
pub use organization_window_history::{
    build_historical_organization_window_origin, build_organization_window_standings_snapshot,
    historical_franchise_organization, historical_organization_window_manifest,
    OrganizationWindowHistoricalOriginArtifact, OrganizationWindowHistoryError,
    OrganizationWindowStandingRow, OrganizationWindowStandingsSnapshot, NHL_STANDINGS_SOURCE_BASE,
    ORGANIZATION_WINDOW_HISTORICAL_IDENTITY_VERSION, ORGANIZATION_WINDOW_HISTORICAL_MANIFEST_ID,
    ORGANIZATION_WINDOW_HISTORICAL_ORIGIN_JSON_SCHEMA,
    ORGANIZATION_WINDOW_HISTORICAL_ORIGIN_SCHEMA, ORGANIZATION_WINDOW_STANDINGS_JSON_SCHEMA,
    ORGANIZATION_WINDOW_STANDINGS_SCHEMA,
};
pub use playoffs_bundle::PlayoffsBundle;
pub use prospect_career::{
    build_prospect_career_context_draft, build_prospect_career_discovery,
    build_prospect_program_from_camp_and_career_store, ProspectCareerContextDraftConfig,
    ProspectCareerContextIdentityInput, ProspectCareerDiscoveryView, ProspectCareerExclusionReason,
    ProspectCareerExclusionView, ProspectCareerProgramComposition, ProspectCareerProgramConfig,
    PROSPECT_CAREER_DISCOVERY_SCHEMA,
};
pub use prospect_discovery::{
    build_prospect_league_context_draft, build_prospect_league_discovery, ProspectLeagueContext,
    ProspectLeagueContextAuthority, ProspectLeagueContextDraftConfig,
    ProspectLeagueContextExclusionReason, ProspectLeagueContextExclusionView,
    ProspectLeagueDiscoveryView, ProspectLeagueExclusionReason, ProspectLeagueExclusionView,
    ProspectLeaguePlayerContext, PROSPECT_LEAGUE_CONTEXT_SCHEMA, PROSPECT_LEAGUE_DISCOVERY_SCHEMA,
};
pub use resolver::PlayerResolver;
pub use scenario_registry::{
    ResolvedTeamSeasonScenario, ScenarioImportDisposition, ScenarioImportResult,
    ScenarioRegistryStore, ScenarioRegistryStoreError,
};
pub use shift_chart::{
    build_shift_overlap_report, OfficialShiftChartResponse, OfficialShiftChartRow,
    ShiftOverlapPairRow, ShiftOverlapPlayerRow, ShiftOverlapReport, ShiftOverlapTrioRow,
    SHIFT_CHART_SOURCE, SHIFT_OVERLAP_SCHEMA,
};
pub use shift_profile::{LinematePair, ShiftProfile};
pub use teams::{nhl_teams_for_season, ALL_NHL_TEAMS};
