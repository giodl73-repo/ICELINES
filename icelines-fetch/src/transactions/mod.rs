//! ESPN transactions fetcher (Phase T.2).
//!
//! Spec: `design/specs/transactions.md` §"Data source", §"Failure modes".
//! Plan: `design/plans/2026-04-30-phaseT-transactions.md` §T.2.
//!
//! No `TransactionSource` trait in v1 (FORGE: premature abstraction with
//! one impl). Promote when source #2 (PHR RSS) actually lands.
//!
//! WIRE-mandated reliability behaviors live here:
//! - 429 backoff with jitter, max 3 retries, honor `Retry-After`
//! - 5xx backoff
//! - Circuit-break after 3 consecutive non-200s in one paginated run
//! - HTML body detection via `Content-Type` before feeding to serde
//! - Schema-drift fallback via `serde_json::Value` extraction; the
//!   dropped field paths are surfaced in `FetchOutcome.dropped_unknown_schema`
//!   so callers can WARN-log specifics.

pub mod convert;
pub mod espn;

pub use convert::{raw_to_transaction, raw_to_transactions};
pub use espn::EspnSource;

use crate::schema::RawTransaction;

/// Result of one season fetch. The shape is rich enough for callers to
/// react to partial / degraded conditions (WIRE: never paper over
/// reliability).
#[derive(Debug, Clone)]
pub struct FetchOutcome {
    pub rows: Vec<RawTransaction>,
    /// Field paths that were dropped via the schema-drift fallback, e.g.
    /// `"team.logos[]"`. Empty = clean parse. Caller WARN-logs specifics.
    pub dropped_unknown_schema: Vec<String>,
    /// True when the source signaled partial data (circuit-breaker tripped
    /// before completion). Caller MUST NOT overwrite a richer snapshot
    /// with a partial one.
    pub partial: bool,
    /// ETag / Last-Modified for conditional re-fetch when supported.
    /// Not used in v1; kept on the struct so a follow-up can wire it
    /// without a public-API break.
    pub source_etag: Option<String>,
    /// Wall-clock at fetch start (RFC-3339). Persisted in the snapshot
    /// envelope as `fetched_at` for staleness display.
    pub fetched_at: String,
}

impl FetchOutcome {
    #[allow(dead_code)]
    pub(crate) fn new(rows: Vec<RawTransaction>, fetched_at: String) -> Self {
        Self {
            rows,
            dropped_unknown_schema: Vec::new(),
            partial: false,
            source_etag: None,
            fetched_at,
        }
    }
}
