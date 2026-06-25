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
use crate::WebConfig;
use crate::WebState;

const GENERIC_REPORT_JSON_PATH: &str = "/api/v1/reports/analytics-cache";
const COACH_DASHBOARD_JSON_PATH: &str = "/api/v1/coach/dashboard";
const PLAYER_EVIDENCE_CARD_JSON_PATH: &str = "/api/v1/player/evidence-card";
const LINE_COMBINATION_EXPLORER_JSON_PATH: &str = "/api/v1/lines/explorer";
const GOALIE_READINESS_JSON_PATH: &str = "/api/v1/goalies/readiness";
const PRACTICE_FOCUS_JSON_PATH: &str = "/api/v1/practice/focus";
const POSTGAME_REVIEW_JSON_PATH: &str = "/api/v1/postgame/review";
const POSTGAME_ADJUSTMENTS_JSON_PATH: &str = "/api/v1/postgame/adjustments";
const AGENT_EVIDENCE_JSON_PATH: &str = "/api/v1/agents/evidence";
const OPPONENT_SCOUT_JSON_PATH: &str = "/api/v1/scout/opponent";
const DEFAULT_COACH_DASHBOARD_METRICS: &str = "expected_goals_share";
const DEFAULT_PLAYER_EVIDENCE_CARD_METRICS: &str = "expected_goals_share";
const DEFAULT_LINE_COMBINATION_EXPLORER_METRICS: &str = "expected_goals_share";
const DEFAULT_GOALIE_READINESS_METRICS: &str = "expected_goals_share";
const DEFAULT_PRACTICE_FOCUS_METRICS: &str = "expected_goals_share";
const DEFAULT_POSTGAME_REVIEW_METRICS: &str = "expected_goals_share";
const DEFAULT_POSTGAME_ADJUSTMENTS_METRICS: &str = "expected_goals_share";
const DEFAULT_AGENT_EVIDENCE_METRICS: &str = "expected_goals_share";
const DEFAULT_OPPONENT_SCOUT_METRICS: &str = "expected_goals_share";

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AnalyticsCacheReportQuery {
    cache_key: Option<String>,
    metrics: Option<String>,
}

#[derive(Debug, Serialize)]
struct AnalyticsCacheReportPayload {
    status: &'static str,
    cache_key: String,
    consumer: AnalyticsCacheConsumerKind,
    consumer_boundary: &'static str,
    selected_cache_evidence_routes: Vec<AnalyticsCacheEvidenceRouteHandoff>,
    selected_cache_evidence_scope: &'static str,
    report: AnalyticsCacheConsumerView,
}

#[derive(Debug, Serialize)]
struct AnalyticsCacheEvidenceRouteHandoff {
    label: &'static str,
    html_path: &'static str,
    json_path: &'static str,
}

#[derive(Debug, Serialize)]
struct AnalyticsCacheUnavailablePayload {
    status: &'static str,
    cache_key: Option<String>,
    reason: String,
    guidance: &'static str,
    non_claims: &'static [&'static str],
}

const UNAVAILABLE_JSON_NON_CLAIMS: &[&str] = &[
    "Does not compute live analytics.",
    "Does not infer prediction, betting, injury, deployment, or linemate meaning.",
    "Does not create or fetch missing cache records.",
];

pub async fn analytics_cache_report(
    State(state): State<WebState>,
    Query(query): Query<AnalyticsCacheReportQuery>,
) -> impl IntoResponse {
    let active_label = state.config.read().await.active_label.clone();
    let template =
        match load_analytics_cache_report(&query, AnalyticsCacheConsumerKind::CoachDashboard) {
            Ok(payload) => template_from_payload(
                active_label,
                payload,
                query.metrics.as_deref(),
                None,
                GENERIC_REPORT_JSON_PATH,
            ),
            Err(err) => unavailable_template(
                "Analytics Cache Report",
                active_label,
                &query,
                err,
                GENERIC_REPORT_JSON_PATH,
            ),
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

pub async fn coach_dashboard(
    State(state): State<WebState>,
    Query(query): Query<AnalyticsCacheReportQuery>,
) -> impl IntoResponse {
    let config = state.config.read().await.clone();
    let active_label = config.active_label.clone();
    let query = surface_query(
        query,
        &config,
        "coach_dashboard",
        DEFAULT_COACH_DASHBOARD_METRICS,
    );
    let template =
        match load_analytics_cache_report(&query, AnalyticsCacheConsumerKind::CoachDashboard) {
            Ok(payload) => template_from_payload(
                active_label,
                payload,
                query.metrics.as_deref(),
                None,
                COACH_DASHBOARD_JSON_PATH,
            ),
            Err(err) => unavailable_template(
                "Coach Game-Day Dashboard",
                active_label,
                &query,
                err,
                COACH_DASHBOARD_JSON_PATH,
            ),
        };

    match template.render() {
        Ok(body) => Html(body).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to render coach dashboard: {err}"),
        )
            .into_response(),
    }
}

pub async fn opponent_scout(
    State(state): State<WebState>,
    Query(query): Query<AnalyticsCacheReportQuery>,
) -> impl IntoResponse {
    let config = state.config.read().await.clone();
    let active_label = config.active_label.clone();
    let query = surface_query(
        query,
        &config,
        "opponent_scout",
        DEFAULT_OPPONENT_SCOUT_METRICS,
    );
    let template = match load_analytics_cache_report(
        &query,
        AnalyticsCacheConsumerKind::OpponentScoutReport,
    ) {
        Ok(payload) => template_from_payload(
            active_label,
            payload,
            query.metrics.as_deref(),
            None,
            OPPONENT_SCOUT_JSON_PATH,
        ),
        Err(err) => unavailable_template(
            "Opponent Scout Report",
            active_label,
            &query,
            err,
            OPPONENT_SCOUT_JSON_PATH,
        ),
    };

    match template.render() {
        Ok(body) => Html(body).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to render opponent scout report: {err}"),
        )
            .into_response(),
    }
}

pub async fn player_evidence_card(
    State(state): State<WebState>,
    Query(query): Query<AnalyticsCacheReportQuery>,
) -> impl IntoResponse {
    let config = state.config.read().await.clone();
    let active_label = config.active_label.clone();
    let query = surface_query(
        query,
        &config,
        "player_evidence_card",
        DEFAULT_PLAYER_EVIDENCE_CARD_METRICS,
    );
    let template =
        match load_analytics_cache_report(&query, AnalyticsCacheConsumerKind::PlayerEvidenceCard) {
            Ok(payload) => template_from_payload(
                active_label,
                payload,
                query.metrics.as_deref(),
                None,
                PLAYER_EVIDENCE_CARD_JSON_PATH,
            ),
            Err(err) => unavailable_template(
                "Player Evidence Card",
                active_label,
                &query,
                err,
                PLAYER_EVIDENCE_CARD_JSON_PATH,
            ),
        };

    match template.render() {
        Ok(body) => Html(body).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to render player evidence card: {err}"),
        )
            .into_response(),
    }
}

pub async fn line_combination_explorer(
    State(state): State<WebState>,
    Query(query): Query<AnalyticsCacheReportQuery>,
) -> impl IntoResponse {
    let config = state.config.read().await.clone();
    let active_label = config.active_label.clone();
    let query = surface_query(
        query,
        &config,
        "line_combination_explorer",
        DEFAULT_LINE_COMBINATION_EXPLORER_METRICS,
    );
    let template = match load_analytics_cache_report(
        &query,
        AnalyticsCacheConsumerKind::LineCombinationExplorer,
    ) {
        Ok(payload) => template_from_payload(
            active_label,
            payload,
            query.metrics.as_deref(),
            None,
            LINE_COMBINATION_EXPLORER_JSON_PATH,
        ),
        Err(err) => unavailable_template(
            "Line Combination Explorer",
            active_label,
            &query,
            err,
            LINE_COMBINATION_EXPLORER_JSON_PATH,
        ),
    };

    match template.render() {
        Ok(body) => Html(body).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to render line combination explorer: {err}"),
        )
            .into_response(),
    }
}

pub async fn goalie_readiness(
    State(state): State<WebState>,
    Query(query): Query<AnalyticsCacheReportQuery>,
) -> impl IntoResponse {
    let config = state.config.read().await.clone();
    let active_label = config.active_label.clone();
    let query = surface_query(
        query,
        &config,
        "goalie_readiness",
        DEFAULT_GOALIE_READINESS_METRICS,
    );
    let template =
        match load_analytics_cache_report(&query, AnalyticsCacheConsumerKind::GoalieReadiness) {
            Ok(payload) => template_from_payload(
                active_label,
                payload,
                query.metrics.as_deref(),
                None,
                GOALIE_READINESS_JSON_PATH,
            ),
            Err(err) => unavailable_template(
                "Goalie Readiness & Workload View",
                active_label,
                &query,
                err,
                GOALIE_READINESS_JSON_PATH,
            ),
        };

    match template.render() {
        Ok(body) => Html(body).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to render goalie readiness view: {err}"),
        )
            .into_response(),
    }
}

pub async fn practice_focus(
    State(state): State<WebState>,
    Query(query): Query<AnalyticsCacheReportQuery>,
) -> impl IntoResponse {
    let config = state.config.read().await.clone();
    let active_label = config.active_label.clone();
    let query = surface_query(
        query,
        &config,
        "practice_focus",
        DEFAULT_PRACTICE_FOCUS_METRICS,
    );
    let template = match load_analytics_cache_report(
        &query,
        AnalyticsCacheConsumerKind::PracticeFocusReport,
    ) {
        Ok(payload) => template_from_payload(
            active_label,
            payload,
            query.metrics.as_deref(),
            None,
            PRACTICE_FOCUS_JSON_PATH,
        ),
        Err(err) => unavailable_template(
            "Practice Focus Report",
            active_label,
            &query,
            err,
            PRACTICE_FOCUS_JSON_PATH,
        ),
    };

    match template.render() {
        Ok(body) => Html(body).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to render practice focus report: {err}"),
        )
            .into_response(),
    }
}

pub async fn postgame_review(
    State(state): State<WebState>,
    Query(query): Query<AnalyticsCacheReportQuery>,
) -> impl IntoResponse {
    let config = state.config.read().await.clone();
    let active_label = config.active_label.clone();
    let query = surface_query(
        query,
        &config,
        "postgame_review",
        DEFAULT_POSTGAME_REVIEW_METRICS,
    );
    let template =
        match load_analytics_cache_report(&query, AnalyticsCacheConsumerKind::PostgameReviewReport)
        {
            Ok(payload) => template_from_payload(
                active_label,
                payload,
                query.metrics.as_deref(),
                None,
                POSTGAME_REVIEW_JSON_PATH,
            ),
            Err(err) => unavailable_template(
                "Postgame Review Report",
                active_label,
                &query,
                err,
                POSTGAME_REVIEW_JSON_PATH,
            ),
        };

    match template.render() {
        Ok(body) => Html(body).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to render postgame review report: {err}"),
        )
            .into_response(),
    }
}

pub async fn postgame_adjustments(
    State(state): State<WebState>,
    Query(query): Query<AnalyticsCacheReportQuery>,
) -> impl IntoResponse {
    let config = state.config.read().await.clone();
    let active_label = config.active_label.clone();
    let query = surface_query(
        query,
        &config,
        "postgame_adjustments",
        DEFAULT_POSTGAME_ADJUSTMENTS_METRICS,
    );
    let template =
        match load_analytics_cache_report(&query, AnalyticsCacheConsumerKind::PostgameReviewReport)
        {
            Ok(payload) => template_from_payload(
                active_label,
                payload,
                query.metrics.as_deref(),
                None,
                POSTGAME_ADJUSTMENTS_JSON_PATH,
            ),
            Err(err) => unavailable_template(
                "Postgame Adjustment Review",
                active_label,
                &query,
                err,
                POSTGAME_ADJUSTMENTS_JSON_PATH,
            ),
        };

    match template.render() {
        Ok(body) => Html(body).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to render postgame adjustment review: {err}"),
        )
            .into_response(),
    }
}

pub async fn agent_evidence(
    State(state): State<WebState>,
    Query(query): Query<AnalyticsCacheReportQuery>,
) -> impl IntoResponse {
    let config = state.config.read().await.clone();
    let active_label = config.active_label.clone();
    let query = surface_query(
        query,
        &config,
        "agent_evidence",
        DEFAULT_AGENT_EVIDENCE_METRICS,
    );
    let template =
        match load_analytics_cache_report(&query, AnalyticsCacheConsumerKind::AgentEvidence) {
            Ok(payload) => template_from_payload(
                active_label,
                payload,
                query.metrics.as_deref(),
                None,
                AGENT_EVIDENCE_JSON_PATH,
            ),
            Err(err) => unavailable_template(
                "Agent Evidence Summary",
                active_label,
                &query,
                err,
                AGENT_EVIDENCE_JSON_PATH,
            ),
        };

    match template.render() {
        Ok(body) => Html(body).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to render agent evidence summary: {err}"),
        )
            .into_response(),
    }
}

pub async fn analytics_cache_report_json(
    Query(query): Query<AnalyticsCacheReportQuery>,
) -> impl IntoResponse {
    match load_analytics_cache_report(&query, AnalyticsCacheConsumerKind::CoachDashboard) {
        Ok(payload) => Json(json!(payload)).into_response(),
        Err(err) => (
            err.status,
            Json(json!(AnalyticsCacheUnavailablePayload {
                status: "unavailable",
                cache_key: query.cache_key.clone(),
                reason: err.message,
                guidance: "Build or restore the named analytics cache before using this report.",
                non_claims: UNAVAILABLE_JSON_NON_CLAIMS,
            })),
        )
            .into_response(),
    }
}

pub async fn coach_dashboard_json(
    State(state): State<WebState>,
    Query(query): Query<AnalyticsCacheReportQuery>,
) -> impl IntoResponse {
    let config = state.config.read().await.clone();
    let query = surface_query(
        query,
        &config,
        "coach_dashboard",
        DEFAULT_COACH_DASHBOARD_METRICS,
    );
    match load_analytics_cache_report(&query, AnalyticsCacheConsumerKind::CoachDashboard) {
        Ok(payload) => Json(json!(payload)).into_response(),
        Err(err) => (
            err.status,
            Json(json!(AnalyticsCacheUnavailablePayload {
                status: "unavailable",
                cache_key: query.cache_key.clone(),
                reason: err.message,
                guidance: "Build or restore the active coach-dashboard analytics cache before using this screen.",
                non_claims: UNAVAILABLE_JSON_NON_CLAIMS,
            })),
        )
            .into_response(),
    }
}

pub async fn opponent_scout_json(
    State(state): State<WebState>,
    Query(query): Query<AnalyticsCacheReportQuery>,
) -> impl IntoResponse {
    let config = state.config.read().await.clone();
    let query = surface_query(
        query,
        &config,
        "opponent_scout",
        DEFAULT_OPPONENT_SCOUT_METRICS,
    );
    match load_analytics_cache_report(
        &query,
        AnalyticsCacheConsumerKind::OpponentScoutReport,
    ) {
        Ok(payload) => Json(json!(payload)).into_response(),
        Err(err) => (
            err.status,
            Json(json!(AnalyticsCacheUnavailablePayload {
                status: "unavailable",
                cache_key: query.cache_key.clone(),
                reason: err.message,
                guidance: "Build or restore the active opponent-scout analytics cache before using this report.",
                non_claims: UNAVAILABLE_JSON_NON_CLAIMS,
            })),
        )
            .into_response(),
    }
}

pub async fn player_evidence_card_json(
    State(state): State<WebState>,
    Query(query): Query<AnalyticsCacheReportQuery>,
) -> impl IntoResponse {
    let config = state.config.read().await.clone();
    let query = surface_query(
        query,
        &config,
        "player_evidence_card",
        DEFAULT_PLAYER_EVIDENCE_CARD_METRICS,
    );
    match load_analytics_cache_report(
        &query,
        AnalyticsCacheConsumerKind::PlayerEvidenceCard,
    ) {
        Ok(payload) => Json(json!(payload)).into_response(),
        Err(err) => (
            err.status,
            Json(json!(AnalyticsCacheUnavailablePayload {
                status: "unavailable",
                cache_key: query.cache_key.clone(),
                reason: err.message,
                guidance: "Build or restore the active player-evidence-card analytics cache before using this card.",
                non_claims: UNAVAILABLE_JSON_NON_CLAIMS,
            })),
        )
            .into_response(),
    }
}

pub async fn line_combination_explorer_json(
    State(state): State<WebState>,
    Query(query): Query<AnalyticsCacheReportQuery>,
) -> impl IntoResponse {
    let config = state.config.read().await.clone();
    let query = surface_query(
        query,
        &config,
        "line_combination_explorer",
        DEFAULT_LINE_COMBINATION_EXPLORER_METRICS,
    );
    match load_analytics_cache_report(
        &query,
        AnalyticsCacheConsumerKind::LineCombinationExplorer,
    ) {
        Ok(payload) => Json(json!(payload)).into_response(),
        Err(err) => (
            err.status,
            Json(json!(AnalyticsCacheUnavailablePayload {
                status: "unavailable",
                cache_key: query.cache_key.clone(),
                reason: err.message,
                guidance: "Build or restore the active line-combination analytics cache before using this explorer.",
                non_claims: UNAVAILABLE_JSON_NON_CLAIMS,
            })),
        )
            .into_response(),
    }
}

pub async fn goalie_readiness_json(
    State(state): State<WebState>,
    Query(query): Query<AnalyticsCacheReportQuery>,
) -> impl IntoResponse {
    let config = state.config.read().await.clone();
    let query = surface_query(
        query,
        &config,
        "goalie_readiness",
        DEFAULT_GOALIE_READINESS_METRICS,
    );
    match load_analytics_cache_report(&query, AnalyticsCacheConsumerKind::GoalieReadiness) {
        Ok(payload) => Json(json!(payload)).into_response(),
        Err(err) => (
            err.status,
            Json(json!(AnalyticsCacheUnavailablePayload {
                status: "unavailable",
                cache_key: query.cache_key.clone(),
                reason: err.message,
                guidance: "Build or restore the active goalie-readiness analytics cache before using this workload view.",
                non_claims: UNAVAILABLE_JSON_NON_CLAIMS,
            })),
        )
            .into_response(),
    }
}

pub async fn practice_focus_json(
    State(state): State<WebState>,
    Query(query): Query<AnalyticsCacheReportQuery>,
) -> impl IntoResponse {
    let config = state.config.read().await.clone();
    let query = surface_query(
        query,
        &config,
        "practice_focus",
        DEFAULT_PRACTICE_FOCUS_METRICS,
    );
    match load_analytics_cache_report(&query, AnalyticsCacheConsumerKind::PracticeFocusReport) {
        Ok(payload) => Json(json!(payload)).into_response(),
        Err(err) => (
            err.status,
            Json(json!(AnalyticsCacheUnavailablePayload {
                status: "unavailable",
                cache_key: query.cache_key.clone(),
                reason: err.message,
                guidance: "Build or restore the active practice-focus analytics cache before using this report.",
                non_claims: UNAVAILABLE_JSON_NON_CLAIMS,
            })),
        )
            .into_response(),
    }
}

pub async fn postgame_review_json(
    State(state): State<WebState>,
    Query(query): Query<AnalyticsCacheReportQuery>,
) -> impl IntoResponse {
    let config = state.config.read().await.clone();
    let query = surface_query(
        query,
        &config,
        "postgame_review",
        DEFAULT_POSTGAME_REVIEW_METRICS,
    );
    match load_analytics_cache_report(&query, AnalyticsCacheConsumerKind::PostgameReviewReport) {
        Ok(payload) => Json(json!(payload)).into_response(),
        Err(err) => (
            err.status,
            Json(json!(AnalyticsCacheUnavailablePayload {
                status: "unavailable",
                cache_key: query.cache_key.clone(),
                reason: err.message,
                guidance: "Build or restore the active postgame-review analytics cache before using this report.",
                non_claims: UNAVAILABLE_JSON_NON_CLAIMS,
            })),
        )
            .into_response(),
    }
}

pub async fn postgame_adjustments_json(
    State(state): State<WebState>,
    Query(query): Query<AnalyticsCacheReportQuery>,
) -> impl IntoResponse {
    let config = state.config.read().await.clone();
    let query = surface_query(
        query,
        &config,
        "postgame_adjustments",
        DEFAULT_POSTGAME_ADJUSTMENTS_METRICS,
    );
    match load_analytics_cache_report(&query, AnalyticsCacheConsumerKind::PostgameReviewReport) {
        Ok(payload) => Json(json!(payload)).into_response(),
        Err(err) => (
            err.status,
            Json(json!(AnalyticsCacheUnavailablePayload {
                status: "unavailable",
                cache_key: query.cache_key.clone(),
                reason: err.message,
                guidance: "Build or restore the active postgame-adjustments analytics cache before using this report.",
                non_claims: UNAVAILABLE_JSON_NON_CLAIMS,
            })),
        )
            .into_response(),
    }
}

pub async fn agent_evidence_json(
    State(state): State<WebState>,
    Query(query): Query<AnalyticsCacheReportQuery>,
) -> impl IntoResponse {
    let config = state.config.read().await.clone();
    let query = surface_query(
        query,
        &config,
        "agent_evidence",
        DEFAULT_AGENT_EVIDENCE_METRICS,
    );
    match load_analytics_cache_report(&query, AnalyticsCacheConsumerKind::AgentEvidence) {
        Ok(payload) => Json(json!(payload)).into_response(),
        Err(err) => (
            err.status,
            Json(json!(AnalyticsCacheUnavailablePayload {
                status: "unavailable",
                cache_key: query.cache_key.clone(),
                reason: err.message,
                guidance: "Build or restore the active agent-evidence analytics cache before using this summary.",
                non_claims: UNAVAILABLE_JSON_NON_CLAIMS,
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
    consumer: AnalyticsCacheConsumerKind,
) -> Result<AnalyticsCacheReportPayload, AnalyticsCacheReportError> {
    let cache_key = normalized_cache_key(query)?;
    let supported_metric_keys = supported_metric_keys(query)?;
    let store = AnalyticsCacheStore::under_data_root(data_root());
    let read = store
        .read_record(&cache_key, &supported_metric_keys, chrono::Utc::now())
        .map_err(report_error_from_store)?;
    let envelope = analytics_cache_consumer_envelope(
        &read.record,
        consumer.clone(),
        ANALYTICS_CACHE_CONSUMER_CONTRACT_VERSION,
    )
    .map_err(|err| AnalyticsCacheReportError {
        status: StatusCode::UNPROCESSABLE_ENTITY,
        message: err.to_string(),
    })?;

    Ok(AnalyticsCacheReportPayload {
        status: "ready",
        cache_key,
        consumer_boundary: analytics_cache_consumer_boundary(&consumer),
        consumer,
        selected_cache_evidence_routes: selected_cache_evidence_route_handoffs(),
        selected_cache_evidence_scope:
            "prepared analytics cache records only; does not compute live analytics, fetch missing cache records, infer predictions, or create autonomous coaching actions",
        report: AnalyticsCacheConsumerView::from_envelope(&envelope, read.disposition),
    })
}

fn analytics_cache_consumer_boundary(consumer: &AnalyticsCacheConsumerKind) -> &'static str {
    match consumer {
        AnalyticsCacheConsumerKind::CoachDashboard => {
            "Coach dashboard reads prepared analytics-cache evidence only; it does not issue coaching recommendations, deployment decisions, live analytics, predictions, or cache fetches."
        }
        AnalyticsCacheConsumerKind::OpponentScoutReport => {
            "Opponent scout reads prepared analytics-cache evidence only; it does not issue scouting recommendations, line-matchup decisions, live analytics, predictions, or cache fetches."
        }
        AnalyticsCacheConsumerKind::PlayerEvidenceCard => {
            "Player evidence card reads prepared analytics-cache evidence only; it does not issue player grades, roster recommendations, deployment decisions, live analytics, predictions, or cache fetches."
        }
        AnalyticsCacheConsumerKind::LineCombinationExplorer => {
            "Line combination explorer reads prepared analytics-cache evidence only; it does not infer line chemistry, issue deployment recommendations, compute live analytics, make predictions, or fetch cache records."
        }
        _ => {
            "Selected analytics-cache evidence surface reads prepared cache records only; it does not compute live analytics, infer predictions, or create autonomous actions."
        }
    }
}

fn selected_cache_evidence_route_handoffs() -> Vec<AnalyticsCacheEvidenceRouteHandoff> {
    vec![
        AnalyticsCacheEvidenceRouteHandoff {
            label: "Coach dashboard evidence",
            html_path: "/coach/dashboard",
            json_path: COACH_DASHBOARD_JSON_PATH,
        },
        AnalyticsCacheEvidenceRouteHandoff {
            label: "Opponent scout evidence",
            html_path: "/scout/opponent",
            json_path: OPPONENT_SCOUT_JSON_PATH,
        },
        AnalyticsCacheEvidenceRouteHandoff {
            label: "Player evidence card",
            html_path: "/player/evidence-card",
            json_path: PLAYER_EVIDENCE_CARD_JSON_PATH,
        },
        AnalyticsCacheEvidenceRouteHandoff {
            label: "Line combination explorer",
            html_path: "/lines/explorer",
            json_path: LINE_COMBINATION_EXPLORER_JSON_PATH,
        },
        AnalyticsCacheEvidenceRouteHandoff {
            label: "Goalie readiness evidence",
            html_path: "/goalies/readiness",
            json_path: GOALIE_READINESS_JSON_PATH,
        },
        AnalyticsCacheEvidenceRouteHandoff {
            label: "Practice focus evidence",
            html_path: "/practice/focus",
            json_path: PRACTICE_FOCUS_JSON_PATH,
        },
        AnalyticsCacheEvidenceRouteHandoff {
            label: "Postgame review evidence",
            html_path: "/postgame/review",
            json_path: POSTGAME_REVIEW_JSON_PATH,
        },
        AnalyticsCacheEvidenceRouteHandoff {
            label: "Agent evidence summary",
            html_path: "/agents/evidence",
            json_path: AGENT_EVIDENCE_JSON_PATH,
        },
    ]
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

fn surface_query(
    mut query: AnalyticsCacheReportQuery,
    config: &WebConfig,
    cache_prefix: &str,
    default_metrics: &str,
) -> AnalyticsCacheReportQuery {
    if query
        .cache_key
        .as_deref()
        .is_none_or(|value| value.trim().is_empty())
    {
        query.cache_key = Some(format!(
            "{cache_prefix}:{}:{}",
            config.active_season, config.active_season_type
        ));
    }
    if query
        .metrics
        .as_deref()
        .is_none_or(|value| value.trim().is_empty())
    {
        query.metrics = Some(default_metrics.to_string());
    }
    query
}

fn unavailable_template(
    title: impl Into<String>,
    active_label: String,
    query: &AnalyticsCacheReportQuery,
    err: AnalyticsCacheReportError,
    json_path: &str,
) -> AnalyticsCacheReportTemplate {
    let cache_key = query.cache_key.clone().unwrap_or_default();
    AnalyticsCacheReportTemplate {
        title: title.into(),
        active_label,
        cache_key: cache_key.clone(),
        json_href: json_href(json_path, &cache_key, query.metrics.as_deref()),
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
        non_claims: UNAVAILABLE_JSON_NON_CLAIMS
            .iter()
            .map(|claim| (*claim).to_string())
            .collect(),
    }
}

fn template_from_payload(
    active_label: String,
    payload: AnalyticsCacheReportPayload,
    metrics_query: Option<&str>,
    error: Option<String>,
    json_path: &str,
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
        json_href: json_href(json_path, &cache_key, metrics_query),
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

fn json_href(path: &str, cache_key: &str, metrics: Option<&str>) -> String {
    let encoded_key = percent_encode_query_component(cache_key);
    let mut href = format!("{path}?cache_key={encoded_key}");
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
