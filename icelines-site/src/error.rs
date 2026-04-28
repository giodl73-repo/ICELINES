use thiserror::Error;

#[derive(Debug, Error)]
pub enum SiteError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("snapshot error: {0}")]
    Snapshot(String),
    #[error("no players found for team {0}")]
    EmptyTeam(String),
}
