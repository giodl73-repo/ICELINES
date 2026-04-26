pub mod cross_team;
pub mod scheme;
pub mod depth_chart;
pub mod error;
pub mod model;
pub mod name;
pub mod position;
pub mod position_profile;
pub mod scoring;
pub mod teams;

pub use cross_team::{compute_all as compute_cross_team_metrics, CrossTeamMetrics, WebFitClass};
pub use depth_chart::DepthChartBuilder;
pub use error::IcelinesError;
pub use model::{
    DepthChart, FitClass, GpStatus, LineAssignment, PaceScore, Player, Position, Season, Slot,
    TeamAbbr,
};
pub use name::normalize_name;
pub use position::PositionResolver;
pub use position_profile::PositionProfile;
pub use scheme::{compute_fantasy_score, FantasyScore, Scheme, SkaterStats as SchemeSkaterStats};
pub use scoring::{classify_fit, compute_pace_score, sort_by_pace};
pub use teams::CANONICAL_TEAMS;
