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
    CsvParse {
        row: usize,
        field: String,
        detail: String,
    },
    #[error("player not found: {name}")]
    PlayerNotFound { name: String },
    #[error("ambiguous name '{name}': {candidates:?}")]
    NameAmbiguous {
        name: String,
        candidates: Vec<(u32, String, String)>,
    },
    #[error("IO: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON: {0}")]
    Json(#[from] serde_json::Error),
    /// Phase 8h: chunk requested by hash is not present on disk.
    #[error("missing chunk: {hash}")]
    MissingChunk { hash: String },
    /// Phase 8h: on-disk bytes for a chunk hashed to a different value.
    #[error("integrity violation — expected {expected}, got {actual}")]
    IntegrityViolation { expected: String, actual: String },

    // ── Phase T.2 — transactions fetcher reliability ─────────────────────────
    /// Circuit breaker tripped: ≥N consecutive non-200 responses inside a
    /// single fetch run. Caller MUST NOT overwrite a richer snapshot when
    /// this fires — the partial result is suspect.
    #[error("circuit breaker tripped after {after_failures} consecutive failures: {url}")]
    CircuitBreakerTripped { url: String, after_failures: usize },

    /// Source returned 200 with an empty data array AND a non-empty snapshot
    /// already exists on disk. The caller (T.3 fetcher) refuses the
    /// overwrite to avoid silently zeroing out a season's transactions.
    #[error(
        "source returned empty array; refusing to overwrite non-empty snapshot for season {season}"
    )]
    EmptyResponseRefused { season: String },

    /// HTTP 200 but the body is HTML (Cloudflare challenge, endpoint
    /// removed, region-blocked content). Detected via Content-Type before
    /// feeding to serde — never let HTML deserialize as JSON.
    #[error("source returned HTML instead of JSON ({content_type}): {url}")]
    HtmlBodyResponse { url: String, content_type: String },
}
