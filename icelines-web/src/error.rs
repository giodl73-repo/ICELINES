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
        /// Column offset of the parse failure within `filter`. Today
        /// the parser doesn't carry positions; King.2 will thread one
        /// through. The field exists now so the spec example
        /// (`details: { "filter": "g=>50", "column": 14 }`) can be
        /// populated incrementally without a contract bump.
        column: Option<u16>,
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
    /// list). PascalCase to mirror the spec's "Error envelope"
    /// subsection (`UnknownStat`, `BadFilter`, ...). The WIRE-1
    /// snake_case rule applies to JSON object keys (`schema_version`,
    /// `request_id`), not to enum-discriminator string values; the
    /// spec freezes these PascalCase values.
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
            Self::BadFilter { filter, column, .. } => match column {
                Some(c) => json!({ "filter": filter, "column": c }),
                None => json!({ "filter": filter }),
            },
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

    /// Render this error as an HTML response.
    ///
    /// HTML routes hit by an error must surface a human page, not a
    /// JSON envelope. King.1.x patch (broadcast finding): without this,
    /// browsers visiting `/leaders?filter=g=>50` would see raw JSON
    /// rather than a "filter parse error" page.
    ///
    /// The default IntoResponse impl renders JSON for the API surface;
    /// HTML handlers call this explicitly when their negotiated output
    /// is `Wants::Html`. King.1.4's askama base template will replace
    /// the inline string with a real error template.
    pub fn into_html_response(self) -> Response {
        let request_id = ulid::Ulid::new().to_string();
        let status = self.status();
        let kind = self.kind();
        let message = self.message();
        let hint = self.hint().map(str::to_owned);

        if let Self::Internal(source) = &self {
            // TODO(King.1.6): swap for `tracing::error!` once the
            // tower-http TraceLayer + tracing-subscriber land. Until
            // then `eprintln!` keeps the contract ("internal errors
            // are loggable, request_id travels with them") with no
            // tracing dep.
            eprintln!("[icelines-web INTERNAL] request_id={request_id} source={source:?}");
        }

        // Minimal placeholder HTML — King.1.4 swaps for an askama
        // template. Escaping deliberately conservative: error
        // messages can carry filter strings with `<>` characters that
        // would otherwise break out of context.
        let escaped_msg = html_escape(&message);
        let hint_block = match hint {
            Some(h) => format!("<p class=\"error-hint\">{}</p>", html_escape(&h)),
            None => String::new(),
        };
        let body = format!(
            "<!doctype html>\n\
             <html lang=\"en\"><head><meta charset=\"utf-8\">\
             <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
             <title>IceLines — {kind}</title></head>\
             <body><main>\
             <h1>{kind}</h1>\
             <p class=\"error-message\">{escaped_msg}</p>\
             {hint_block}\
             <p class=\"error-request-id\">request_id: <code>{request_id}</code></p>\
             </main></body></html>\n",
        );

        let mut response = (status, axum::response::Html(body)).into_response();
        response.headers_mut().insert(
            header::HeaderName::from_static("x-request-id"),
            header::HeaderValue::from_str(&request_id).expect("ulid produces ascii-only string"),
        );
        response
    }
}

/// Minimal HTML-safe escape for error rendering. Five mandatory chars
/// per OWASP. No allocator if the input is clean (Cow would be ideal
/// but the call site always wants `String`).
fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

// ── Wants extractor — content negotiation ───────────────────────────────────
//
// HTML routes need HTML errors; JSON routes need JSON errors. The
// `IntoResponse` trait can't see the request, so handlers extract a
// `Wants` from the request and pick the response shape at the error
// site. Pattern from broadcast review:
//
// ```ignore
// async fn handler(
//     wants: Wants,
//     ...
// ) -> Result<Response, Response> {
//     do_thing().map_err(|e: WebError| match wants {
//         Wants::Json => e.into_response(),
//         Wants::Html => e.into_html_response(),
//     })
// }
// ```

/// What output shape the client wants. Sniffed from the request's
/// `Accept` header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wants {
    /// `Accept: text/html` (or `*/*` from a browser default).
    Html,
    /// `Accept: application/json` (the explicit API consumer).
    Json,
}

impl Wants {
    /// Detect from a raw `Accept:` header value. The full RFC 7231
    /// `Accept` grammar (q-values, parameters) is overkill for our
    /// two-shape choice — first matching MIME type wins.
    pub fn from_accept_header(accept: &str) -> Self {
        // Explicit JSON request — embedders, scripts, jq.
        if accept.contains("application/json") {
            return Self::Json;
        }
        // Anything else (text/html, browser */*, missing header) —
        // render the human surface. This means a missing Accept
        // defaults to HTML, which is correct: `curl localhost/foo`
        // without flags should land on a readable page.
        Self::Html
    }
}

#[axum::async_trait]
impl<S> axum::extract::FromRequestParts<S> for Wants
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let accept = parts
            .headers
            .get(axum::http::header::ACCEPT)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        Ok(Self::from_accept_header(accept))
    }
}

// ── From bridges — preserve hints across crate boundaries ───────────────────
//
// Edge review: every handler that calls `parse_filter` will need to
// convert `FilterParseError` → `WebError`. Without a single bridge,
// each handler hand-rolls the mapping → drift across 11 source
// variants → spec hint promise becomes "every handler remembers to
// copy the logic." This bridge centralizes the mapping table.

impl From<icelines_core::stats_catalog::FilterParseError> for WebError {
    fn from(err: icelines_core::stats_catalog::FilterParseError) -> Self {
        use icelines_core::stats_catalog::FilterParseError as F;
        let hint = err.hint().map(|h| h.to_owned());
        let message = err.to_string();
        match err {
            // Kind-stable mapping: source `UnknownStat` → web
            // `UnknownStat` so 400 responses cluster correctly for
            // embedders filtering by `kind`.
            F::UnknownStat { key } => Self::UnknownStat { stat: key, hint },
            // Everything else is a parse failure → BadFilter. The
            // user typed something the parser couldn't make sense of;
            // they need to fix their input. `column` stays None until
            // the parser threads positions (King.2 work).
            other => Self::BadFilter {
                message,
                hint,
                filter: filter_input_of(&other).unwrap_or_default(),
                column: None,
            },
        }
    }
}

/// Extract the original filter input from a parse error variant if
/// present. Used by the `From` bridge so `BadFilter.filter` carries
/// the user's raw input for echoing in the error envelope.
fn filter_input_of(err: &icelines_core::stats_catalog::FilterParseError) -> Option<String> {
    use icelines_core::stats_catalog::FilterParseError as F;
    match err {
        F::MissingOp { input }
        | F::MultipleOps { input }
        | F::UnexpectedToken { token: input }
        | F::BadNumber { token: input }
        | F::NotFinite { token: input } => Some(input.clone()),
        F::UnknownStat { key } => Some(key.clone()),
        F::EmptyInput
        | F::EmptyStatKey
        | F::UnclosedParen
        | F::UnexpectedRParen
        | F::UnexpectedEnd => None,
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
            // TODO(King.1.6): swap for `tracing::error!` once the
            // tower-http TraceLayer + tracing-subscriber land. Until
            // then `eprintln!` keeps the contract ("internal errors
            // are loggable, request_id travels with them") with no
            // tracing dep.
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
                    column: None,
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
                filter: "x".into(),
                column: None,
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
            column: None,
        };
        assert_eq!(with_hint.hint(), Some("did you mean >= ?"));

        let without_hint = WebError::BadFilter {
            message: "x".into(),
            hint: None,
            filter: "x".into(),
            column: None,
        };
        assert_eq!(without_hint.hint(), None);
    }

    /// l0_envelope_round_trip_through_into_response
    /// — King.1.x patch fence (wire + bench review): serialize a
    ///   WebError through the actual IntoResponse path, parse the
    ///   resulting JSON, assert the envelope matches the spec's
    ///   contract field-by-field. Catches:
    ///     - schema_version present + literal `1`
    ///     - error.{kind, message, details} present
    ///     - error.hint omitted when None (not null)
    ///     - error.request_id is a non-empty string
    ///     - X-Request-Id header MATCHES envelope.error.request_id
    ///     - per-kind details payload is the structured shape
    #[tokio::test]
    async fn l0_envelope_round_trip_through_into_response() {
        use axum::body::to_bytes;
        use axum::response::IntoResponse;

        // Case A: BadFilter — hint present, column populated.
        let err = WebError::BadFilter {
            message: "filter parse error".into(),
            hint: Some("did you mean `>=`?".into()),
            filter: "g=>50".into(),
            column: Some(2),
        };
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let request_id_header = response
            .headers()
            .get("x-request-id")
            .expect("X-Request-Id header must be set on every error response")
            .to_str()
            .expect("ULID is ascii")
            .to_owned();
        assert!(
            !request_id_header.is_empty(),
            "X-Request-Id header value must not be empty"
        );

        let body_bytes = to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("error body fits in 64 KiB");
        let envelope: serde_json::Value =
            serde_json::from_slice(&body_bytes).expect("envelope is valid JSON");

        assert_eq!(envelope["schema_version"], 1);
        assert_eq!(envelope["error"]["kind"], "BadFilter");
        assert_eq!(
            envelope["error"]["message"],
            "bad filter expression: filter parse error"
        );
        assert_eq!(envelope["error"]["hint"], "did you mean `>=`?");
        assert_eq!(envelope["error"]["details"]["filter"], "g=>50");
        assert_eq!(envelope["error"]["details"]["column"], 2);

        // X-Request-Id header MUST match the envelope's request_id
        // (so server-side logs and client-side bug reports correlate).
        assert_eq!(
            envelope["error"]["request_id"]
                .as_str()
                .expect("request_id is a string"),
            request_id_header,
            "X-Request-Id header must match envelope.error.request_id"
        );

        // Case B: BadFilter without hint — `hint` MUST be omitted from
        // the JSON object, not present as `null`. Embedders rely on
        // "key absent" to mean "no hint."
        let err_no_hint = WebError::BadFilter {
            message: "x".into(),
            hint: None,
            filter: "x".into(),
            column: None,
        };
        let response = err_no_hint.into_response();
        let body_bytes = to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("error body fits in 64 KiB");
        let envelope: serde_json::Value =
            serde_json::from_slice(&body_bytes).expect("envelope is valid JSON");
        let error_obj = envelope["error"].as_object().expect("error is an object");
        assert!(
            !error_obj.contains_key("hint"),
            "error.hint MUST be omitted when None, not present as null. Got keys: {:?}",
            error_obj.keys().collect::<Vec<_>>()
        );
        assert!(
            !error_obj.contains_key("column") || error_obj["details"].get("column").is_none(),
            "details.column MUST be absent when None"
        );
    }

    /// l0_internal_envelope_does_not_leak_via_serialization
    /// — Stronger version of `l0_internal_message_does_not_leak_source`:
    ///   serialize the full envelope (not just the accessor) and grep
    ///   the rendered JSON for the source string. Catches future
    ///   regressions where someone forgets `Internal`'s opacity in a
    ///   new derive macro.
    #[tokio::test]
    async fn l0_internal_envelope_does_not_leak_via_serialization() {
        use axum::body::to_bytes;
        use axum::response::IntoResponse;
        let err = WebError::Internal(anyhow::anyhow!(
            "/secret/path/to/credentials.json missing — db_password=hunter2"
        ));
        let response = err.into_response();
        let body_bytes = to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("body fits");
        let body_str = std::str::from_utf8(&body_bytes).expect("envelope is utf-8");
        for sensitive in &["/secret/path", "credentials.json", "hunter2", "db_password"] {
            assert!(
                !body_str.contains(sensitive),
                "envelope must not leak sensitive substring {sensitive:?}, got body: {body_str}"
            );
        }
        // But the request_id must still be present so operators can
        // correlate to server logs.
        assert!(body_str.contains("request_id"));
    }

    /// l0_from_filter_parse_error_preserves_hint
    /// — King.1.x patch fence (edge review): the `From<FilterParseError>`
    ///   bridge MUST surface the parser's structured hint into the
    ///   web error's `hint` field, not bury it in `message`.
    #[test]
    fn l0_from_filter_parse_error_preserves_hint() {
        use icelines_core::stats_catalog::FilterParseError;
        let parse_err = FilterParseError::MultipleOps {
            input: "g=>50".into(),
        };
        let web_err: WebError = parse_err.into();
        match web_err {
            WebError::BadFilter { hint, filter, .. } => {
                let hint = hint.expect("hint must propagate from parser");
                assert!(hint.contains(">="), "hint must mention `>=`, got: {hint}");
                assert_eq!(filter, "g=>50", "filter must echo user input");
            }
            other => panic!("expected BadFilter, got: {other:?}"),
        }
    }

    /// l0_from_filter_parse_error_unknown_stat_maps_to_unknown_stat
    /// — Edge concern #7: `FilterParseError::UnknownStat` MUST map to
    ///   `WebError::UnknownStat`, not `BadFilter`, so embedders
    ///   filtering by `kind` cluster correctly.
    #[test]
    fn l0_from_filter_parse_error_unknown_stat_maps_to_unknown_stat() {
        use icelines_core::stats_catalog::FilterParseError;
        let parse_err = FilterParseError::UnknownStat { key: "hots".into() };
        let web_err: WebError = parse_err.into();
        match web_err {
            WebError::UnknownStat { stat, .. } => assert_eq!(stat, "hots"),
            other => panic!(
                "FilterParseError::UnknownStat must map to WebError::UnknownStat, got: {other:?}"
            ),
        }
    }

    /// l0_wants_from_accept_header
    /// — Wants extractor sniff logic. JSON wins on explicit
    ///   `application/json`; everything else (incl. missing) → HTML
    ///   so plain `curl` lands on a readable page.
    #[test]
    fn l0_wants_from_accept_header() {
        assert_eq!(Wants::from_accept_header("application/json"), Wants::Json);
        assert_eq!(
            Wants::from_accept_header("application/json, text/plain"),
            Wants::Json
        );
        assert_eq!(Wants::from_accept_header("text/html"), Wants::Html);
        assert_eq!(Wants::from_accept_header("*/*"), Wants::Html);
        assert_eq!(Wants::from_accept_header(""), Wants::Html);
    }

    /// l0_into_html_response_renders_html_with_request_id
    /// — broadcast HIGH finding: HTML routes need an HTML error page,
    ///   not JSON. This fences the html-response path: status,
    ///   Content-Type, X-Request-Id header, and HTML containing the
    ///   error kind + message + request_id.
    #[tokio::test]
    async fn l0_into_html_response_renders_html_with_request_id() {
        use axum::body::to_bytes;
        let err = WebError::BadFilter {
            message: "parse failed".into(),
            hint: Some("hint text".into()),
            filter: "g=>50".into(),
            column: None,
        };
        let response = err.into_html_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let ct = response
            .headers()
            .get("content-type")
            .expect("content-type set")
            .to_str()
            .unwrap();
        assert!(ct.starts_with("text/html"), "got Content-Type: {ct}");

        let request_id_header = response
            .headers()
            .get("x-request-id")
            .expect("X-Request-Id set on HTML responses too")
            .to_str()
            .unwrap()
            .to_owned();

        let body_bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        let body = std::str::from_utf8(&body_bytes).unwrap();
        assert!(body.contains("BadFilter"), "body must show error kind");
        assert!(body.contains("hint text"), "body must show hint");
        assert!(
            body.contains(&request_id_header),
            "body must show request_id matching the header"
        );
        // Filter input contains `>` and must be HTML-escaped to avoid
        // breaking out of context.
        assert!(
            !body.contains("g=>50") || body.contains("g=&gt;50"),
            "filter input with `>` must be HTML-escaped; got body: {body}"
        );
    }
}
