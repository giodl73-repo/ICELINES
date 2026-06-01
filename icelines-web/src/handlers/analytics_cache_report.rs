use std::path::PathBuf;

use askama::Template;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    Json,
};
use icelines_core::{
    analytics_cache::{
        analytics_cache_consumer_envelope, AnalyticsCacheConsumerKind,
        ANALYTICS_CACHE_CONSUMER_CONTRACT_VERSION,
    },
    AnalyticsCacheConsumerMetricRow, AnalyticsCacheConsumerView, MetricUnit, MetricValue, StatKey,
    ValuePrecision,
};
use icelines_fetch::analytics_cache_store::{AnalyticsCacheStore, AnalyticsCacheStoreError};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::templates::{
    AnalyticsCacheReportMetricTemplateRow, AnalyticsCacheReportSourceTemplateRow,
    AnalyticsCacheReportTemplate,
};
use crate::WebState;

#[derive(Debug, Deserialize)]
pub struct AnalyticsCacheReportQuery {
    cache_key: Option<String>,
    metrics: Option<String>,
}

#[derive(Debug, Serialize)]
struct AnalyticsCacheReportPayload {
    status: &'static str,
    cache_key: String,
    consumer: AnalyticsCacheConsumerKind,
    report: AnalyticsCacheConsumerView,
}

#[derive(Debug, Serialize)]
struct AnalyticsCacheUnavailablePayload {
    status: &'static str,
    cache_key: Option<String>,
    reason: String,
    guidance: &'static str,
}

pub async fn analytics_cache_report(
    State(state): State<WebState>,
    Query(query): Query<AnalyticsCacheReportQuery>,
) -> impl IntoResponse {
    let active_label = state.config.read().await.active_label.clone();
    let template = match load_analytics_cache_report(&query) {
        Ok(payload) => template_from_payload(active_label, payload, query.metrics.as_deref(), None),
        Err(err) => unavailable_template(active_label, &query, err),
    };

    match template.render() {
        Ok(body) => Html(body).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to render analytics cache report: {err}"),
        )
            .into_response(),
    }
}

pub async fn analytics_cache_report_json(
    Query(query): Query<AnalyticsCacheReportQuery>,
) -> impl IntoResponse {
    match load_analytics_cache_report(&query) {
        Ok(payload) => Json(json!(payload)).into_response(),
        Err(err) => (
            err.status,
            Json(json!(AnalyticsCacheUnavailablePayload {
                status: "unavailable",
                cache_key: query.cache_key.clone(),
                reason: err.message,
                guidance: "Build or restore the named analytics cache before using this report.",
            })),
        )
            .into_response(),
    }
}

#[derive(Debug)]
struct AnalyticsCacheReportError {
    status: StatusCode,
    message: String,
}

fn load_analytics_cache_report(
    query: &AnalyticsCacheReportQuery,
) -> Result<AnalyticsCacheReportPayload, AnalyticsCacheReportError> {
    let cache_key = normalized_cache_key(query)?;
    let supported_metric_keys = supported_metric_keys(query)?;
    let store = AnalyticsCacheStore::under_data_root(data_root());
    let read = store
        .read_record(&cache_key, &supported_metric_keys, chrono::Utc::now())
        .map_err(report_error_from_store)?;
    let envelope = analytics_cache_consumer_envelope(
        &read.record,
        AnalyticsCacheConsumerKind::CoachDashboard,
        ANALYTICS_CACHE_CONSUMER_CONTRACT_VERSION,
    )
    .map_err(|err| AnalyticsCacheReportError {
        status: StatusCode::UNPROCESSABLE_ENTITY,
        message: err.to_string(),
    })?;

    Ok(AnalyticsCacheReportPayload {
        status: "ready",
        cache_key,
        consumer: AnalyticsCacheConsumerKind::CoachDashboard,
        report: AnalyticsCacheConsumerView::from_envelope(&envelope, read.disposition),
    })
}

fn normalized_cache_key(
    query: &AnalyticsCacheReportQuery,
) -> Result<String, AnalyticsCacheReportError> {
    query
        .cache_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| AnalyticsCacheReportError {
            status: StatusCode::BAD_REQUEST,
            message: "analytics cache report requires a cache_key query parameter".to_string(),
        })
}

fn supported_metric_keys(
    query: &AnalyticsCacheReportQuery,
) -> Result<Vec<StatKey>, AnalyticsCacheReportError> {
    let metrics = query
        .metrics
        .as_deref()
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| StatKey(value.to_string()))
        .collect::<Vec<_>>();

    if metrics.is_empty() {
        return Err(AnalyticsCacheReportError {
            status: StatusCode::BAD_REQUEST,
            message: "analytics cache report requires a comma-separated metrics query parameter"
                .to_string(),
        });
    }

    Ok(metrics)
}

fn report_error_from_store(err: AnalyticsCacheStoreError) -> AnalyticsCacheReportError {
    let status = match err {
        AnalyticsCacheStoreError::MissingCache { .. } => StatusCode::NOT_FOUND,
        AnalyticsCacheStoreError::Contract(_) => StatusCode::UNPROCESSABLE_ENTITY,
        AnalyticsCacheStoreError::EmptyCacheKey
        | AnalyticsCacheStoreError::EmptyInvalidationKey => StatusCode::BAD_REQUEST,
        AnalyticsCacheStoreError::Io { .. } => StatusCode::INTERNAL_SERVER_ERROR,
    };

    AnalyticsCacheReportError {
        status,
        message: err.to_string(),
    }
}

fn unavailable_template(
    active_label: String,
    query: &AnalyticsCacheReportQuery,
    err: AnalyticsCacheReportError,
) -> AnalyticsCacheReportTemplate {
    let cache_key = query.cache_key.clone().unwrap_or_default();
    AnalyticsCacheReportTemplate {
        title: "Analytics Cache Report".to_string(),
        active_label,
        cache_key: cache_key.clone(),
        json_href: json_href(&cache_key, query.metrics.as_deref()),
        status: "unavailable".to_string(),
        disposition: "unavailable".to_string(),
        source_window: "not loaded".to_string(),
        scope: "not loaded".to_string(),
        methodology_version: "not loaded".to_string(),
        quality: "not loaded".to_string(),
        sample_size: "not loaded".to_string(),
        data_root: data_root().display().to_string(),
        error: err.message,
        metrics: Vec::new(),
        sources: Vec::new(),
        warnings: Vec::new(),
        limitations: Vec::new(),
        disclosures: Vec::new(),
        non_claims: vec!["This page does not compute live analytics or infer betting, injury, deployment, or linemate meaning.".to_string()],
    }
}

fn template_from_payload(
    active_label: String,
    payload: AnalyticsCacheReportPayload,
    metrics_query: Option<&str>,
    error: Option<String>,
) -> AnalyticsCacheReportTemplate {
    let report = payload.report;
    let cache_key = payload.cache_key;
    let metrics = report.metrics.iter().map(metric_row).collect::<Vec<_>>();
    let sources = report
        .sources
        .iter()
        .map(|source| AnalyticsCacheReportSourceTemplateRow {
            source: format!("{:?}", source.source),
            state: format!("{:?}", source.state),
            provenance: source
                .provenance
                .as_ref()
                .map(|value| format!("{value:?}"))
                .unwrap_or_else(|| "not recorded".to_string()),
            fetched_at: source
                .fetched_at
                .map(|value| value.to_rfc3339())
                .unwrap_or_else(|| "not recorded".to_string()),
            message: source
                .message
                .clone()
                .or_else(|| source.stale_reason.clone())
                .unwrap_or_else(|| "none".to_string()),
        })
        .collect::<Vec<_>>();
    let quality = format!("{:?}", report.quality.completeness);

    AnalyticsCacheReportTemplate {
        title: report.title,
        active_label,
        cache_key: cache_key.clone(),
        json_href: json_href(&cache_key, metrics_query),
        status: payload.status.to_string(),
        disposition: format!("{:?}", report.disposition),
        source_window: report.source_window.source_window_label,
        scope: format!("{:?}", report.scope),
        methodology_version: report.methodology_version,
        quality,
        sample_size: report
            .quality
            .sample_size
            .map(|value| value.to_string())
            .unwrap_or_else(|| "not recorded".to_string()),
        data_root: data_root().display().to_string(),
        error: error.unwrap_or_default(),
        metrics,
        sources,
        warnings: report
            .warnings
            .iter()
            .map(|warning| format!("{warning:?}"))
            .collect(),
        limitations: report.quality.limitations,
        disclosures: report.disclosures,
        non_claims: report.non_claims,
    }
}

fn metric_row(row: &AnalyticsCacheConsumerMetricRow) -> AnalyticsCacheReportMetricTemplateRow {
    AnalyticsCacheReportMetricTemplateRow {
        key: row.cell.key.0.clone(),
        label: row.cell.label.clone(),
        value: metric_value_label(&row.cell.value, row.cell.unit, row.cell.precision),
        unit: format!("{:?}", row.cell.unit),
        source_state: row
            .source_state
            .iter()
            .map(|source| format!("{:?}:{:?}", source.source, source.state))
            .collect::<Vec<_>>()
            .join(", "),
        methodology_note: row.methodology_note.clone().unwrap_or_default(),
    }
}

fn metric_value_label(value: &MetricValue, unit: MetricUnit, precision: ValuePrecision) -> String {
    match value {
        MetricValue::Integer(value) => value.to_string(),
        MetricValue::Decimal(value) => match precision {
            ValuePrecision::Integer => format!("{value:.0}"),
            ValuePrecision::OneDecimal => decimal_label(*value, unit, 1),
            ValuePrecision::TwoDecimals => decimal_label(*value, unit, 2),
            ValuePrecision::ThreeDecimals => decimal_label(*value, unit, 3),
            ValuePrecision::PercentOneDecimal => format!("{value:.1}%"),
            ValuePrecision::Raw => value.to_string(),
        },
        MetricValue::Text(value) => value.clone(),
        MetricValue::Missing => "unavailable".to_string(),
    }
}

fn decimal_label(value: f64, unit: MetricUnit, precision: usize) -> String {
    if unit == MetricUnit::Percentage {
        format!("{value:.precision$}%")
    } else {
        format!("{value:.precision$}")
    }
}

fn json_href(cache_key: &str, metrics: Option<&str>) -> String {
    let encoded_key = percent_encode_query_component(cache_key);
    let mut href = format!("/api/v1/reports/analytics-cache?cache_key={encoded_key}");
    if let Some(metrics) = metrics.filter(|value| !value.trim().is_empty()) {
        href.push_str("&metrics=");
        href.push_str(&percent_encode_query_component(metrics));
    }
    href
}

fn percent_encode_query_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char);
            }
            b' ' => encoded.push('+'),
            other => encoded.push_str(&format!("%{other:02X}")),
        }
    }
    encoded
}

fn data_root() -> PathBuf {
    if let Some(root) = std::env::var_os("ICELINES_DATA_ROOT") {
        return PathBuf::from(root);
    }

    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"));
    home.map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".icelines")
}
