pub mod aggregate;
pub mod atomic_write;
pub mod boxscore_client;
pub mod boxscore_to_night_line;
pub mod bundled;
pub mod cache;
pub mod career;
pub mod career_landing;
pub mod chunkstore;
pub mod csv_loader;
pub mod datastore;
pub mod error;
pub mod fetch_lock;
pub mod manifest;
pub mod moneypuck;
pub mod nhl_api;
pub mod playoffs_bundle;
pub mod resolver;
pub mod schema;
pub mod series_momentum_builder;
pub mod shift_profile;
pub mod snapshot;
pub mod stats_loader;
pub mod sync_engine;
pub mod teams;
pub mod transactions;

// Hart.5c.7: PlayerRepository / GoalieRepository / Player / Goalie / player_builder
// all deleted. Every consumer reads StatsRepository + PlayerView<'_> via
// `stats_loader::load_into_repo`.

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
