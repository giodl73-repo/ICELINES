//! Shared JSON response envelope for `/api/v1/*` handlers.

use axum::response::{IntoResponse, Response};
use serde::Serialize;

pub const API_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize)]
pub struct ApiEnvelope<T, M> {
    pub schema_version: u32,
    pub route: &'static str,
    pub data: T,
    pub meta: M,
    pub error: Option<String>,
}

impl<T, M> ApiEnvelope<T, M> {
    pub fn new(route: &'static str, data: T, meta: M, error: Option<String>) -> Self {
        Self {
            schema_version: API_SCHEMA_VERSION,
            route,
            data,
            meta,
            error,
        }
    }
}

pub fn json_envelope<T, M>(route: &'static str, data: T, meta: M, error: Option<String>) -> Response
where
    T: Serialize,
    M: Serialize,
{
    axum::Json(ApiEnvelope::new(route, data, meta, error)).into_response()
}
