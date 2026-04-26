use thiserror::Error;

#[derive(Debug, Error)]
pub enum FetchError {
    #[error("HTTP {status}: {url}")]
    Http { status: u16, url: String },
    #[error("rate limited — retries exhausted: {url}")]
    RateLimited { url: String },
    #[error("service unavailable (503): {url}")]
    ServiceUnavailable { url: String },
    #[error("NHL API schema changed — unexpected field: {detail}")]
    SchemaChanged { detail: String },
    #[error("cache error: {0}")]
    Cache(String),
    #[error("CSV parse error at row {row}, field '{field}': {detail}")]
    CsvParse { row: usize, field: String, detail: String },
    #[error("player not found: {name}")]
    PlayerNotFound { name: String },
    #[error("ambiguous name '{name}': {candidates:?}")]
    NameAmbiguous { name: String, candidates: Vec<(u32, String, String)> },
    #[error("IO: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON: {0}")]
    Json(#[from] serde_json::Error),
}

pub struct PlayerResolver;
