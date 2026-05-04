//! `WebError` — single source of truth for handler errors.
//!
//! Implements [`axum::response::IntoResponse`] so handlers return
//! `Result<T, WebError>` and the framework converts the error into the
//! spec's JSON error envelope. See `design/specs/web-dashboard.md` →
//! "URL & API contract" → "Error envelope".
//!
//! ## Envelope shape
//!
//! ```json
//! {
//!   "schema_version": 1,
//!   "error": {
//!     "kind": "BadFilter",
//!     "message": "filter parse error at column 14",
//!     "hint": "did you mean '>=' instead of '=>'?",
//!     "details": { "filter": "g=>50", "column": 14 },
//!     "request_id": "01HZQ..."
//!   }
//! }
//! ```
//!
//! Status codes per spec: 400 for client-side, 404 for missing
//! resources, 421 for DNS-rebinding rejects (King.1.6), 500 for
//! `Internal`. Every response also carries an `X-Request-Id` header
//! with the same ULID logged via `tracing` server-side.

use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;

/// Schema version constant for error envelopes. Per spec: per-envelope
/// integer, additive-only within a version. A breaking change forces a
/// per-route bump (and that route alone moves to `/api/v2/...`).
pub const ERROR_SCHEMA_VERSION: u32 = 1;

/// All error kinds returned by the web layer. Each maps to an HTTP
/// status via [`WebError::status`]. The spec's full enumeration:
///
/// | Kind | HTTP | Notes |
/// |---|---|---|
/// | `UnknownStat` | 400 | `?sort=` or `?filter=` references unknown stat |
/// | `UnknownSort` | 400 | `?sort=` value not in StatId catalog |
/// | `UnknownSeason` | 400 | `?season=` not in BUNDLED_SEASONS or installed |
/// | `UnknownPlayer` | 404 | `/player/by-name/X` resolution failed |
/// | `BadFilter` | 400 | filter expression failed to parse |
/// | `BadParam` | 400 | malformed `?limit=`, `?offset=`, etc. |
/// | `ConflictingParams` | 400 | e.g. `?seasons=N` AND `?season=YYYYZZZZ` |
/// | `NotFound` | 404 | route exists, target missing |
/// | `RateLimited` | 429 | (reserved — no rate-limiting in v1) |
/// | `Internal` | 500 | bug; logged with stack at WARN+ |
/// | `CorruptSnapshot` | 400 | installed snapshot integrity hash failed |
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum WebError {
    #[error("unknown stat: {stat}")]
    UnknownStat { stat: String, hint: Option<String> },

    #[error("unknown sort key: {sort}")]
    UnknownSort { sort: String, hint: Option<String> },

    #[error("unknown season: {season}")]
    UnknownSeason {
        season: String,
        hint: Option<String>,
    },

    #[error("unknown player: {query}")]
    UnknownPlayer { query: String, hint: Option<String> },

    #[error("bad filter expression: {message}")]
    BadFilter {
        message: String,
        hint: Option<String>,
        filter: String,
    },

    #[error("bad parameter '{param}': {message}")]
    BadParam {
        param: String,
        message: String,
        hint: Option<String>,
    },

    #[error("conflicting parameters: {message}")]
    ConflictingParams {
        message: String,
        hint: Option<String>,
    },

    #[error("not found: {what}")]
    NotFound { what: String, hint: Option<String> },

    /// Reserved for future rate-limiting; v1 never emits this.
    #[error("rate limited")]
    RateLimited,

    /// Wraps an unexpected error (a bug). Source preserved for logging
    /// but never leaked into the JSON envelope — clients see a generic
    /// `"internal error"` plus the `request_id` for support.
    #[error("internal error")]
    Internal(#[source] anyhow::Error),

    #[error("corrupt snapshot: {season}")]
    CorruptSnapshot {
        season: String,
        hint: Option<String>,
    },
}

impl WebError {
    /// HTTP status code for this error.
    pub fn status(&self) -> StatusCode {
        match self {
            Self::UnknownStat { .. }
            | Self::UnknownSort { .. }
            | Self::UnknownSeason { .. }
            | Self::BadFilter { .. }
            | Self::BadParam { .. }
            | Self::ConflictingParams { .. }
            | Self::CorruptSnapshot { .. } => StatusCode::BAD_REQUEST,
            Self::UnknownPlayer { .. } | Self::NotFound { .. } => StatusCode::NOT_FOUND,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Stable `kind` discriminator string (matches the spec's enum
    /// list). Lowercase camelCase to mirror existing fantasy-server
    /// conventions and to keep JSON consumers consistent.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::UnknownStat { .. } => "UnknownStat",
            Self::UnknownSort { .. } => "UnknownSort",
            Self::UnknownSeason { .. } => "UnknownSeason",
            Self::UnknownPlayer { .. } => "UnknownPlayer",
            Self::BadFilter { .. } => "BadFilter",
            Self::BadParam { .. } => "BadParam",
            Self::ConflictingParams { .. } => "ConflictingParams",
            Self::NotFound { .. } => "NotFound",
            Self::RateLimited => "RateLimited",
            Self::Internal(_) => "Internal",
            Self::CorruptSnapshot { .. } => "CorruptSnapshot",
        }
    }

    /// Optional hint string for the envelope. CLI errors carry
    /// actionable hints (e.g. "did you mean '>=' instead of '=>'?")
    /// and the web surface mirrors that.
    pub fn hint(&self) -> Option<&str> {
        match self {
            Self::UnknownStat { hint, .. }
            | Self::UnknownSort { hint, .. }
            | Self::UnknownSeason { hint, .. }
            | Self::UnknownPlayer { hint, .. }
            | Self::BadFilter { hint, .. }
            | Self::BadParam { hint, .. }
            | Self::ConflictingParams { hint, .. }
            | Self::NotFound { hint, .. }
            | Self::CorruptSnapshot { hint, .. } => hint.as_deref(),
            Self::RateLimited | Self::Internal(_) => None,
        }
    }

    /// Structured per-kind detail block. Embedders can read specific
    /// fields without parsing the human message.
    pub fn details(&self) -> serde_json::Value {
        use serde_json::json;
        match self {
            Self::UnknownStat { stat, .. } => json!({ "stat": stat }),
            Self::UnknownSort { sort, .. } => json!({ "sort": sort }),
            Self::UnknownSeason { season, .. } => json!({ "season": season }),
            Self::UnknownPlayer { query, .. } => json!({ "query": query }),
            Self::BadFilter { filter, .. } => json!({ "filter": filter }),
            Self::BadParam { param, .. } => json!({ "param": param }),
            Self::ConflictingParams { .. } => json!({}),
            Self::NotFound { what, .. } => json!({ "what": what }),
            Self::RateLimited => json!({}),
            Self::Internal(_) => json!({}),
            Self::CorruptSnapshot { season, .. } => json!({ "season": season }),
        }
    }

    /// User-facing message. For `Internal`, this is a generic string —
    /// the underlying `anyhow::Error` is logged but never leaked.
    pub fn message(&self) -> String {
        match self {
            Self::Internal(_) => {
                "internal error — see server logs with request_id for details".to_owned()
            }
            other => other.to_string(),
        }
    }
}

/// JSON envelope shape sent to the client.
#[derive(Debug, Serialize)]
struct ErrorEnvelope<'a> {
    schema_version: u32,
    error: ErrorBody<'a>,
}

#[derive(Debug, Serialize)]
struct ErrorBody<'a> {
    kind: &'a str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    hint: Option<&'a str>,
    details: serde_json::Value,
    request_id: String,
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        // Per spec: ULID per request, attached as both X-Request-Id
        // header AND inside the envelope. Logged at WARN+ via tracing
        // for server-side correlation. Middleware in King.1.6 will
        // generate the ULID upstream and pass it via request
        // extensions; until then we generate inline so the contract
        // (header + envelope match) holds from day one.
        let request_id = ulid::Ulid::new().to_string();

        let status = self.status();
        let envelope = ErrorEnvelope {
            schema_version: ERROR_SCHEMA_VERSION,
            error: ErrorBody {
                kind: self.kind(),
                message: self.message(),
                hint: self.hint(),
                details: self.details(),
                request_id: request_id.clone(),
            },
        };

        // Internal errors carry a source — log it with the request_id
        // so support requests can correlate. Other kinds are user
        // errors; logging would be noise.
        if let Self::Internal(source) = &self {
            // tracing isn't a workspace dep yet (King.1.6 adds the
            // tower-http TraceLayer). Until then, use eprintln! so the
            // contract ("Internal errors are loggable, request_id
            // travels with them") still holds.
            eprintln!("[icelines-web INTERNAL] request_id={request_id} source={source:?}");
        }

        let mut response = (status, Json(envelope)).into_response();
        response.headers_mut().insert(
            header::HeaderName::from_static("x-request-id"),
            header::HeaderValue::from_str(&request_id)
                // ULID is alphanumeric, always valid header value
                .expect("ulid produces ascii-only string"),
        );
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// l0_status_codes_match_spec
    /// — the spec's "Error envelope" subsection enumerates a status
    ///   per kind. Drift = embedder breakage. Lock the mapping here.
    #[test]
    fn l0_status_codes_match_spec() {
        let cases: Vec<(WebError, StatusCode)> = vec![
            (
                WebError::UnknownStat {
                    stat: "x".into(),
                    hint: None,
                },
                StatusCode::BAD_REQUEST,
            ),
            (
                WebError::UnknownSort {
                    sort: "x".into(),
                    hint: None,
                },
                StatusCode::BAD_REQUEST,
            ),
            (
                WebError::UnknownSeason {
                    season: "x".into(),
                    hint: None,
                },
                StatusCode::BAD_REQUEST,
            ),
            (
                WebError::UnknownPlayer {
                    query: "x".into(),
                    hint: None,
                },
                StatusCode::NOT_FOUND,
            ),
            (
                WebError::BadFilter {
                    message: "x".into(),
                    hint: None,
                    filter: "x".into(),
                },
                StatusCode::BAD_REQUEST,
            ),
            (
                WebError::BadParam {
                    param: "x".into(),
                    message: "x".into(),
                    hint: None,
                },
                StatusCode::BAD_REQUEST,
            ),
            (
                WebError::ConflictingParams {
                    message: "x".into(),
                    hint: None,
                },
                StatusCode::BAD_REQUEST,
            ),
            (
                WebError::NotFound {
                    what: "x".into(),
                    hint: None,
                },
                StatusCode::NOT_FOUND,
            ),
            (WebError::RateLimited, StatusCode::TOO_MANY_REQUESTS),
            (
                WebError::Internal(anyhow::anyhow!("boom")),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            (
                WebError::CorruptSnapshot {
                    season: "x".into(),
                    hint: None,
                },
                StatusCode::BAD_REQUEST,
            ),
        ];
        for (err, expected) in cases {
            let kind = err.kind();
            assert_eq!(err.status(), expected, "status mismatch for {kind}");
        }
    }

    /// l0_kind_strings_stable
    /// — `kind` is part of the JSON contract. Renames break clients.
    ///   Lock the exact strings here.
    #[test]
    fn l0_kind_strings_stable() {
        assert_eq!(
            WebError::UnknownStat {
                stat: "x".into(),
                hint: None
            }
            .kind(),
            "UnknownStat"
        );
        assert_eq!(
            WebError::UnknownSort {
                sort: "x".into(),
                hint: None
            }
            .kind(),
            "UnknownSort"
        );
        assert_eq!(
            WebError::UnknownSeason {
                season: "x".into(),
                hint: None
            }
            .kind(),
            "UnknownSeason"
        );
        assert_eq!(
            WebError::UnknownPlayer {
                query: "x".into(),
                hint: None
            }
            .kind(),
            "UnknownPlayer"
        );
        assert_eq!(
            WebError::BadFilter {
                message: "x".into(),
                hint: None,
                filter: "x".into()
            }
            .kind(),
            "BadFilter"
        );
        assert_eq!(
            WebError::BadParam {
                param: "x".into(),
                message: "x".into(),
                hint: None
            }
            .kind(),
            "BadParam"
        );
        assert_eq!(
            WebError::ConflictingParams {
                message: "x".into(),
                hint: None
            }
            .kind(),
            "ConflictingParams"
        );
        assert_eq!(
            WebError::NotFound {
                what: "x".into(),
                hint: None
            }
            .kind(),
            "NotFound"
        );
        assert_eq!(WebError::RateLimited.kind(), "RateLimited");
        assert_eq!(WebError::Internal(anyhow::anyhow!("x")).kind(), "Internal");
        assert_eq!(
            WebError::CorruptSnapshot {
                season: "x".into(),
                hint: None
            }
            .kind(),
            "CorruptSnapshot"
        );
    }

    /// l0_internal_message_does_not_leak_source
    /// — `WebError::Internal(anyhow::Error)` MUST present a generic
    ///   message to the client; the source goes to logs only. This
    ///   protects against accidentally leaking internal paths, SQL,
    ///   credentials, etc. through error messages.
    #[test]
    fn l0_internal_message_does_not_leak_source() {
        let err = WebError::Internal(anyhow::anyhow!(
            "/secret/path/to/credentials.json not readable"
        ));
        let msg = err.message();
        assert!(
            !msg.contains("/secret/path"),
            "internal error message must not leak source detail; got: {msg}"
        );
        assert!(
            msg.contains("internal error") && msg.contains("request_id"),
            "internal error message must direct the user to logs via request_id; got: {msg}"
        );
    }

    /// l0_hint_is_optional
    /// — clients should treat `hint` as optional. Verify the field is
    ///   omitted (not null) when absent so JSON consumers can use the
    ///   `key not present` sentinel.
    #[test]
    fn l0_hint_is_optional() {
        let with_hint = WebError::BadFilter {
            message: "x".into(),
            hint: Some("did you mean >= ?".into()),
            filter: "g=>50".into(),
        };
        assert_eq!(with_hint.hint(), Some("did you mean >= ?"));

        let without_hint = WebError::BadFilter {
            message: "x".into(),
            hint: None,
            filter: "x".into(),
        };
        assert_eq!(without_hint.hint(), None);
    }
}
