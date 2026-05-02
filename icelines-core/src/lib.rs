/// Current NHL season identifier — update each October.
/// Format: YYYYZZZZ where YYYY = start year, ZZZZ = end year.
pub const CURRENT_SEASON:     u32  = 20_252_026;
pub const CURRENT_SEASON_STR: &str = "20252026";

pub mod contract;
pub mod cross_team;
pub mod depth_chart;
pub mod error;
pub mod filter;
pub mod fixtures;
pub mod history;
pub mod identity;
pub mod model;
pub mod name;
pub mod position;
pub mod position_profile;
pub mod projection;
pub mod scheme;
pub mod scoring;
pub mod season_stats;
pub mod stats_catalog;
pub mod stats_repository;
pub mod teams;
pub mod transactions;

pub use cross_team::{
    compute_all_views as compute_cross_team_metrics_views, CrossTeamMetrics, WebFitClass,
};
pub use depth_chart::DepthChartBuilder;
pub use error::IcelinesError;
pub use filter::PlayerFilter;
pub use history::{CareerSummary, SeasonLine};
pub use model::{
    DepthChart, DepthChartSlot, FitClass, GpStatus, LineAssignment, PaceScore, Position, Region,
    Season, Slot, TeamAbbr,
};
pub use name::normalize_name;
pub use position::PositionResolver;
pub use position_profile::PositionProfile;
pub use projection::{age_factor, compute_alpha, compute_projection, ProjectionMode, ProjectionResult};
pub use scheme::{compute_fantasy_score, FantasyScore, Scheme, SkaterStats as SchemeSkaterStats};
pub use scoring::{classify_fit, compute_pace_score, sort_views_by_pace};
pub use teams::CANONICAL_TEAMS;
pub use transactions::{
    classify, other_rate, sanitize_description, trade_group_id,
    Transaction, TransactionKind, CURRENT_CLASSIFIER_VERSION,
    TRANSACTIONS_EARLIEST_SEASON,
};
