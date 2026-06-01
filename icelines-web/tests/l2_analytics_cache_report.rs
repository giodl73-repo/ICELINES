use std::sync::OnceLock;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::response::Response;
use chrono::{DateTime, TimeZone, Utc};
use icelines_core::season_stats::SeasonType;
use icelines_core::{
    build_analytics_cache_record, AnalyticsCacheBuildInput, AnalyticsCacheConsumerKind,
    AnalyticsCacheInvalidation, AnalyticsCacheMetric, AnalyticsCacheQuality, AnalyticsCacheScope,
    AnalyticsCacheSourceWindow, Completeness, MetricCell, MetricUnit, MetricValue, Season,
    SemanticToken, SourceKind, SourceProvenance, SourceState, StatKey, ValuePrecision, ViewWindow,
};
use icelines_fetch::analytics_cache_store::AnalyticsCacheStore;
use icelines_web::{router, WebState};
use tempfile::TempDir;
use tokio::sync::Mutex;
use tower::util::ServiceExt;

async fn env_lock() -> tokio::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().await
}

struct DataRootFixture {
    dir: TempDir,
    prev_data_root: Option<std::ffi::OsString>,
}

impl DataRootFixture {
    fn new() -> Self {
        let dir = TempDir::new().expect("temp data root");
        let prev_data_root = std::env::var_os("ICELINES_DATA_ROOT");
        std::env::set_var("ICELINES_DATA_ROOT", dir.path());
        Self {
            dir,
            prev_data_root,
        }
    }

    fn path(&self) -> &std::path::Path {
        self.dir.path()
    }
}

impl Drop for DataRootFixture {
    fn drop(&mut self) {
        match &self.prev_data_root {
            Some(value) => std::env::set_var("ICELINES_DATA_ROOT", value),
            None => std::env::remove_var("ICELINES_DATA_ROOT"),
        }
    }
}

fn t() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap()
}

fn supported_metric_keys() -> Vec<StatKey> {
    vec![StatKey::from("expected_goals_share")]
}

fn source_state(state: Completeness) -> SourceState {
    SourceState {
        source: SourceKind::Snapshot,
        state,
        provenance: Some(SourceProvenance::Snapshot {
            id: "stats-2026-06-01".to_string(),
        }),
        fetched_at: Some(t()),
        stale_reason: (state == Completeness::Stale).then(|| "snapshot expired".to_string()),
        message: Some("local snapshot source".to_string()),
    }
}

fn sample_record(cache_key: &str) -> icelines_core::AnalyticsCacheRecord {
    let source = source_state(Completeness::Complete);
    let mut metric = AnalyticsCacheMetric::new(
        MetricCell {
            key: StatKey::from("expected_goals_share"),
            label: "xG Share".to_string(),
            value: MetricValue::Decimal(55.1),
            unit: MetricUnit::Percentage,
            precision: ValuePrecision::OneDecimal,
            token: Some(SemanticToken::DecisionHighlight),
        },
        vec![source.clone()],
    );
    metric.methodology_note = Some("cache-foundation-v1 preserved from record".to_string());

    build_analytics_cache_record(AnalyticsCacheBuildInput {
        cache_key: cache_key.to_string(),
        scope: AnalyticsCacheScope::new("coach_dashboard", Season(20252026), SeasonType::Regular),
        built_at: t(),
        source_window: AnalyticsCacheSourceWindow::season(
            ViewWindow::new(Season(20252026), SeasonType::Regular),
            "2025-26 regular season through 2026-06-01",
        ),
        sources: vec![source],
        quality: AnalyticsCacheQuality {
            completeness: Completeness::Complete,
            sample_size: Some(82),
            warnings: Vec::new(),
            limitations: vec!["Does not prove line chemistry causality".to_string()],
        },
        invalidation: AnalyticsCacheInvalidation::keys(vec![
            "snapshot:stats-2026-06-01".to_string(),
            "methodology:cache-foundation-v1".to_string(),
        ]),
        methodology_version: "cache-foundation-v1".to_string(),
        metrics: vec![metric],
        disclosures: vec![
            "Prepared from local snapshot evidence; stale or partial state is explicit."
                .to_string(),
        ],
        non_claims: vec![
            "Not a prediction, betting, injury, or autonomous coaching claim.".to_string(),
        ],
        supported_metric_keys: supported_metric_keys(),
        supported_consumers: vec![
            AnalyticsCacheConsumerKind::CoachDashboard,
            AnalyticsCacheConsumerKind::PlayerEvidenceCard,
        ],
    })
    .expect("sample analytics cache record")
}

async fn response_text(response: Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body bytes");
    String::from_utf8(bytes.to_vec()).expect("utf8 body")
}

#[tokio::test]
async fn l2_wp009_analytics_cache_report_missing_cache_is_explicitly_unavailable() {
    let _guard = env_lock().await;
    let fixture = DataRootFixture::new();
    let app = router(WebState::new());

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(
                    "/reports/analytics-cache?cache_key=missing:cache&metrics=expected_goals_share",
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_text(response).await;
    assert!(body.contains("Report unavailable"));
    assert!(body.contains("analytics cache entry is missing: missing:cache"));
    assert!(body.contains("does not compute live analytics"));

    let response = app
        .oneshot(
            Request::builder()
                .uri(
                    "/api/v1/reports/analytics-cache?cache_key=missing:cache&metrics=expected_goals_share",
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let json: serde_json::Value =
        serde_json::from_str(&response_text(response).await).expect("json payload");
    assert_eq!(json["status"], "unavailable");
    assert_eq!(json["cache_key"], "missing:cache");
    assert!(json["reason"]
        .as_str()
        .expect("reason")
        .contains("analytics cache entry is missing: missing:cache"));
    assert!(!fixture.path().join("analytics_cache").exists());
}

#[tokio::test]
async fn l2_wp009_analytics_cache_report_renders_cache_envelope_without_recomputing() {
    let _guard = env_lock().await;
    let fixture = DataRootFixture::new();
    let store = AnalyticsCacheStore::under_data_root(fixture.path());
    let record = sample_record("coach_dashboard:20252026:regular");
    store
        .write_record(&record, &supported_metric_keys())
        .expect("write analytics cache record");
    let app = router(WebState::new());

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/reports/analytics-cache?cache_key=coach_dashboard:20252026:regular&metrics=expected_goals_share")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_text(response).await;
    assert!(body.contains("Coach Game-Day Dashboard"));
    assert!(body.contains("xG Share"));
    assert!(body.contains("55.1%"));
    assert!(body.contains("local snapshot source"));
    assert!(body.contains("Prepared from local snapshot evidence"));
    assert!(body.contains("Not a prediction, betting, injury, or autonomous coaching claim."));
    assert!(body.contains("recompute analytics"));
    assert!(body.contains("fetch live data"));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/reports/analytics-cache?cache_key=coach_dashboard:20252026:regular&metrics=expected_goals_share")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json: serde_json::Value =
        serde_json::from_str(&response_text(response).await).expect("json payload");
    assert_eq!(json["status"], "ready");
    assert_eq!(json["report"]["metrics"][0]["cell"]["label"], "xG Share");
    assert_eq!(
        json["report"]["non_claims"][0],
        "Not a prediction, betting, injury, or autonomous coaching claim."
    );
}
