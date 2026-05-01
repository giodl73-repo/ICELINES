pub mod aggregate;
pub mod boxscore_client;
pub mod bundled;
pub mod cache;
pub mod career;
pub mod chunkstore;
pub mod csv_loader;
pub mod error;
pub mod moneypuck;
pub mod nhl_api;
pub mod player_builder;
pub mod playoffs_bundle;
pub mod resolver;
pub mod schema;
pub mod shift_profile;
pub mod snapshot;
pub mod stats_loader;
pub mod teams;
pub mod transactions;

// Hart.5b1: PlayerRepository / GoalieRepository deleted. Every consumer
// uses load_into_repo + StatsRepository (via flat_view_legacy /
// flat_view_legacy_goalies during the Hart.5b transition; Hart.5b2 will
// refactor consumers off the legacy Player/Goalie types entirely).

pub use boxscore_client::{aggregate_profiles, aggregate_shift_profiles, BoxscoreClient};
pub use bundled::{
    get_bios as get_bundled_bios, get_playoffs as get_bundled_playoffs,
    get_stats as get_bundled_stats, load_bios_with_fallback, load_playoffs,
    load_stats_with_fallback, BUNDLED_SEASONS,
};
pub use cache::Cache;
pub use career::load_career;
pub use error::FetchError;
pub use moneypuck::{parse_csv as parse_moneypuck_csv, MoneyPuckStats};
pub use nhl_api::NhlApiClient;
pub use playoffs_bundle::PlayoffsBundle;
pub use resolver::PlayerResolver;
pub use shift_profile::{LinematePair, ShiftProfile};
pub use teams::ALL_NHL_TEAMS;
