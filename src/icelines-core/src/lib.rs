pub mod depth_chart;
pub mod error;
pub mod model;
pub mod name;
pub mod position;
pub mod scoring;
pub mod teams;

pub use depth_chart::DepthChartBuilder;
pub use error::IcelinesError;
pub use model::{
    DepthChart, FitClass, GpStatus, LineAssignment, PaceScore, Player, Position, Season, Slot,
    TeamAbbr,
};
pub use name::normalize_name;
pub use position::PositionResolver;
pub use scoring::{classify_fit, compute_pace_score, sort_by_pace};
pub use teams::CANONICAL_TEAMS;
