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
use icelines_web::{router, WebConfig, WebState};
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

fn sample_record(
    cache_key: &str,
    scope_kind: &str,
    supported_consumers: Vec<AnalyticsCacheConsumerKind>,
) -> icelines_core::AnalyticsCacheRecord {
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
        scope: AnalyticsCacheScope::new(scope_kind, Season(20252026), SeasonType::Regular),
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
        supported_consumers,
    })
    .expect("sample analytics cache record")
}

fn coach_record(cache_key: &str) -> icelines_core::AnalyticsCacheRecord {
    sample_record(
        cache_key,
        "coach_dashboard",
        vec![
            AnalyticsCacheConsumerKind::CoachDashboard,
            AnalyticsCacheConsumerKind::PlayerEvidenceCard,
        ],
    )
}

fn opponent_scout_record(cache_key: &str) -> icelines_core::AnalyticsCacheRecord {
    sample_record(
        cache_key,
        "opponent_scout",
        vec![AnalyticsCacheConsumerKind::OpponentScoutReport],
    )
}

fn player_evidence_card_record(cache_key: &str) -> icelines_core::AnalyticsCacheRecord {
    sample_record(
        cache_key,
        "player_evidence_card",
        vec![AnalyticsCacheConsumerKind::PlayerEvidenceCard],
    )
}

fn line_combination_explorer_record(cache_key: &str) -> icelines_core::AnalyticsCacheRecord {
    sample_record(
        cache_key,
        "line_combination_explorer",
        vec![AnalyticsCacheConsumerKind::LineCombinationExplorer],
    )
}

fn goalie_readiness_record(cache_key: &str) -> icelines_core::AnalyticsCacheRecord {
    sample_record(
        cache_key,
        "goalie_readiness",
        vec![AnalyticsCacheConsumerKind::GoalieReadiness],
    )
}

fn practice_focus_record(cache_key: &str) -> icelines_core::AnalyticsCacheRecord {
    sample_record(
        cache_key,
        "practice_focus",
        vec![AnalyticsCacheConsumerKind::PracticeFocusReport],
    )
}

fn postgame_review_record(cache_key: &str) -> icelines_core::AnalyticsCacheRecord {
    sample_record(
        cache_key,
        "postgame_review",
        vec![AnalyticsCacheConsumerKind::PostgameReviewReport],
    )
}

fn postgame_adjustments_record(cache_key: &str) -> icelines_core::AnalyticsCacheRecord {
    sample_record(
        cache_key,
        "postgame_adjustments",
        vec![AnalyticsCacheConsumerKind::PostgameReviewReport],
    )
}

fn agent_evidence_record(cache_key: &str) -> icelines_core::AnalyticsCacheRecord {
    sample_record(
        cache_key,
        "agent_evidence",
        vec![AnalyticsCacheConsumerKind::AgentEvidence],
    )
}

async fn response_text(response: Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body bytes");
    String::from_utf8(bytes.to_vec()).expect("utf8 body")
}

async fn app_with_config(season: &str, season_type: &str) -> axum::Router {
    let state = WebState::new();
    {
        let mut config = state.config.write().await;
        *config = WebConfig::new(season, season_type);
    }
    router(state)
}

fn assert_unavailable_json_non_claims(json: &serde_json::Value) {
    let non_claims = json["non_claims"]
        .as_array()
        .expect("unavailable JSON carries non_claims");
    assert!(
        non_claims
            .iter()
            .any(|claim| claim == "Does not compute live analytics."),
        "{json}"
    );
    assert!(
        non_claims.iter().any(|claim| claim
            == "Does not infer prediction, betting, injury, deployment, or linemate meaning."),
        "{json}"
    );
    assert!(
        non_claims
            .iter()
            .any(|claim| claim == "Does not create or fetch missing cache records."),
        "{json}"
    );
}

fn assert_unavailable_html_non_claims(body: &str) {
    assert!(body.contains("Non-claims"), "{body}");
    assert!(body.contains("Does not compute live analytics."), "{body}");
    assert!(
        body.contains(
            "Does not infer prediction, betting, injury, deployment, or linemate meaning."
        ),
        "{body}"
    );
    assert!(
        body.contains("Does not create or fetch missing cache records."),
        "{body}"
    );
}

fn assert_cache_evidence_route_handoffs(body: &str) {
    assert!(body.contains("Selected cache evidence routes"), "{body}");
    assert!(
        body.contains("prepared analytics cache records only"),
        "{body}"
    );
    assert!(body.contains("compute live analytics"), "{body}");
    assert!(body.contains("fetch missing cache records"), "{body}");
    assert!(body.contains("infer predictions"), "{body}");
    assert!(body.contains("autonomous coaching actions"), "{body}");
    assert!(body.contains(r#"href="/coach/dashboard""#), "{body}");
    assert!(body.contains(r#"href="/scout/opponent""#), "{body}");
    assert!(body.contains(r#"href="/player/evidence-card""#), "{body}");
    assert!(body.contains(r#"href="/lines/explorer""#), "{body}");
    assert!(body.contains(r#"href="/goalies/readiness""#), "{body}");
    assert!(body.contains(r#"href="/practice/focus""#), "{body}");
    assert!(body.contains(r#"href="/postgame/review""#), "{body}");
    assert!(body.contains(r#"href="/agents/evidence""#), "{body}");
}

fn assert_cache_json_evidence_route_handoffs(json: &serde_json::Value) {
    assert_eq!(
        json["selected_cache_evidence_scope"],
        "prepared analytics cache records only; does not compute live analytics, fetch missing cache records, infer predictions, or create autonomous coaching actions"
    );
    let routes = json["selected_cache_evidence_routes"]
        .as_array()
        .expect("ready cache JSON carries selected evidence route handoffs");
    assert_eq!(routes.len(), 8, "{json}");
    assert!(
        routes
            .iter()
            .any(|route| route["label"] == "Coach dashboard evidence"
                && route["html_path"] == "/coach/dashboard"
                && route["json_path"] == "/api/v1/coach/dashboard"),
        "{json}"
    );
    assert!(
        routes
            .iter()
            .any(|route| route["label"] == "Opponent scout evidence"
                && route["html_path"] == "/scout/opponent"
                && route["json_path"] == "/api/v1/scout/opponent"),
        "{json}"
    );
    assert!(
        routes
            .iter()
            .any(|route| route["label"] == "Agent evidence summary"
                && route["html_path"] == "/agents/evidence"
                && route["json_path"] == "/api/v1/agents/evidence"),
        "{json}"
    );
}

fn assert_cache_health_interpretation(body: &str) {
    assert!(body.contains("Cache health interpretation"), "{body}");
    assert!(body.contains("record that was read"), "{body}");
    assert!(body.contains("evidence health signals"), "{body}");
    assert!(body.contains("confidence"), "{body}");
    assert!(body.contains("predictions"), "{body}");
    assert!(body.contains("live fetch results"), "{body}");
    assert!(body.contains("missing cache records"), "{body}");
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
    assert_unavailable_html_non_claims(&body);
    assert_cache_evidence_route_handoffs(&body);
    assert_cache_health_interpretation(&body);

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
    assert_unavailable_json_non_claims(&json);
    assert!(!fixture.path().join("analytics_cache").exists());
}

#[tokio::test]
async fn l2_wp009_coach_dashboard_defaults_to_active_cache_and_explicit_unavailable_state() {
    let _guard = env_lock().await;
    let fixture = DataRootFixture::new();
    let app = app_with_config("20252026", "regular").await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/coach/dashboard")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_text(response).await;
    assert!(body.contains("Coach Game-Day Dashboard"));
    assert!(body.contains("Report unavailable"));
    assert!(body.contains("coach_dashboard:20252026:regular"));
    assert!(body.contains("analytics cache entry is missing: coach_dashboard:20252026:regular"));
    assert_unavailable_html_non_claims(&body);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/coach/dashboard")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let json: serde_json::Value =
        serde_json::from_str(&response_text(response).await).expect("json payload");
    assert_eq!(json["status"], "unavailable");
    assert_eq!(json["cache_key"], "coach_dashboard:20252026:regular");
    assert!(json["guidance"]
        .as_str()
        .expect("guidance")
        .contains("coach-dashboard analytics cache"));
    assert_unavailable_json_non_claims(&json);
    assert!(!fixture.path().join("analytics_cache").exists());
}

#[tokio::test]
async fn l2_wp009_coach_dashboard_renders_default_cache_without_generic_query_contract() {
    let _guard = env_lock().await;
    let fixture = DataRootFixture::new();
    let store = AnalyticsCacheStore::under_data_root(fixture.path());
    let record = coach_record("coach_dashboard:20252026:regular");
    store
        .write_record(&record, &supported_metric_keys())
        .expect("write analytics cache record");
    let app = app_with_config("20252026", "regular").await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/coach/dashboard")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_text(response).await;
    assert!(body.contains("Coach Game-Day Dashboard"));
    assert!(body.contains("coach_dashboard:20252026:regular"));
    assert!(body.contains("xG Share"));
    assert!(body.contains("55.1%"));
    assert!(body.contains("/api/v1/coach/dashboard?cache_key=coach_dashboard%3A20252026%3Aregular"));
    assert!(body.contains("Not a prediction, betting, injury, or autonomous coaching claim."));
    assert_cache_evidence_route_handoffs(&body);
    assert_cache_health_interpretation(&body);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/coach/dashboard")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json: serde_json::Value =
        serde_json::from_str(&response_text(response).await).expect("json payload");
    assert_eq!(json["status"], "ready");
    assert_eq!(json["cache_key"], "coach_dashboard:20252026:regular");
    assert_eq!(json["consumer"], "coach_dashboard");
    assert_eq!(
        json["consumer_boundary"],
        "Coach dashboard reads prepared analytics-cache evidence only; it does not issue coaching recommendations, deployment decisions, live analytics, predictions, or cache fetches."
    );
    assert_eq!(json["report"]["title"], "Coach Game-Day Dashboard");
    assert_eq!(json["report"]["metrics"][0]["cell"]["label"], "xG Share");
    assert_cache_json_evidence_route_handoffs(&json);
}

#[tokio::test]
async fn l2_wp009_opponent_scout_defaults_to_active_cache_and_explicit_unavailable_state() {
    let _guard = env_lock().await;
    let fixture = DataRootFixture::new();
    let app = app_with_config("20252026", "regular").await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/scout/opponent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_text(response).await;
    assert!(body.contains("Opponent Scout Report"));
    assert!(body.contains("Report unavailable"));
    assert!(body.contains("opponent_scout:20252026:regular"));
    assert!(body.contains("analytics cache entry is missing: opponent_scout:20252026:regular"));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/scout/opponent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let json: serde_json::Value =
        serde_json::from_str(&response_text(response).await).expect("json payload");
    assert_eq!(json["status"], "unavailable");
    assert_eq!(json["cache_key"], "opponent_scout:20252026:regular");
    assert!(json["guidance"]
        .as_str()
        .expect("guidance")
        .contains("opponent-scout analytics cache"));
    assert_unavailable_json_non_claims(&json);
    assert!(!fixture.path().join("analytics_cache").exists());
}

#[tokio::test]
async fn l2_wp009_opponent_scout_renders_cache_as_scout_consumer_view() {
    let _guard = env_lock().await;
    let fixture = DataRootFixture::new();
    let store = AnalyticsCacheStore::under_data_root(fixture.path());
    let record = opponent_scout_record("opponent_scout:20252026:regular");
    store
        .write_record(&record, &supported_metric_keys())
        .expect("write analytics cache record");
    let app = app_with_config("20252026", "regular").await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/scout/opponent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_text(response).await;
    assert!(body.contains("Opponent Scout Report"));
    assert!(body.contains("opponent_scout:20252026:regular"));
    assert!(body.contains("xG Share"));
    assert!(body.contains("55.1%"));
    assert!(body.contains("/api/v1/scout/opponent?cache_key=opponent_scout%3A20252026%3Aregular"));
    assert!(body.contains("Not a prediction, betting, injury, or autonomous coaching claim."));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/scout/opponent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json: serde_json::Value =
        serde_json::from_str(&response_text(response).await).expect("json payload");
    assert_eq!(json["status"], "ready");
    assert_eq!(json["cache_key"], "opponent_scout:20252026:regular");
    assert_eq!(json["consumer"], "opponent_scout_report");
    assert_eq!(
        json["consumer_boundary"],
        "Opponent scout reads prepared analytics-cache evidence only; it does not issue scouting recommendations, line-matchup decisions, live analytics, predictions, or cache fetches."
    );
    assert_eq!(json["report"]["title"], "Opponent Scout Report");
    assert_eq!(json["report"]["consumer"], "opponent_scout_report");
    assert_eq!(json["report"]["metrics"][0]["cell"]["label"], "xG Share");
}

#[tokio::test]
async fn l2_wp009_player_evidence_card_defaults_to_active_cache_and_explicit_unavailable_state() {
    let _guard = env_lock().await;
    let fixture = DataRootFixture::new();
    let app = app_with_config("20252026", "regular").await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/player/evidence-card")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_text(response).await;
    assert!(body.contains("Player Evidence Card"));
    assert!(body.contains("Report unavailable"));
    assert!(body.contains("player_evidence_card:20252026:regular"));
    assert!(
        body.contains("analytics cache entry is missing: player_evidence_card:20252026:regular")
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/player/evidence-card")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let json: serde_json::Value =
        serde_json::from_str(&response_text(response).await).expect("json payload");
    assert_eq!(json["status"], "unavailable");
    assert_eq!(json["cache_key"], "player_evidence_card:20252026:regular");
    assert!(json["guidance"]
        .as_str()
        .expect("guidance")
        .contains("player-evidence-card analytics cache"));
    assert_unavailable_json_non_claims(&json);
    assert!(!fixture.path().join("analytics_cache").exists());
}

#[tokio::test]
async fn l2_wp009_player_evidence_card_renders_cache_as_player_consumer_view() {
    let _guard = env_lock().await;
    let fixture = DataRootFixture::new();
    let store = AnalyticsCacheStore::under_data_root(fixture.path());
    let record = player_evidence_card_record("player_evidence_card:20252026:regular");
    store
        .write_record(&record, &supported_metric_keys())
        .expect("write analytics cache record");
    let app = app_with_config("20252026", "regular").await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/player/evidence-card")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_text(response).await;
    assert!(body.contains("Player Evidence Card"));
    assert!(body.contains("player_evidence_card:20252026:regular"));
    assert!(body.contains("xG Share"));
    assert!(body.contains("55.1%"));
    assert!(body.contains(
        "/api/v1/player/evidence-card?cache_key=player_evidence_card%3A20252026%3Aregular"
    ));
    assert!(body.contains("Not a prediction, betting, injury, or autonomous coaching claim."));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/player/evidence-card")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json: serde_json::Value =
        serde_json::from_str(&response_text(response).await).expect("json payload");
    assert_eq!(json["status"], "ready");
    assert_eq!(json["cache_key"], "player_evidence_card:20252026:regular");
    assert_eq!(json["consumer"], "player_evidence_card");
    assert_eq!(
        json["consumer_boundary"],
        "Player evidence card reads prepared analytics-cache evidence only; it does not issue player grades, roster recommendations, deployment decisions, live analytics, predictions, or cache fetches."
    );
    assert_eq!(json["report"]["title"], "Player Evidence Card");
    assert_eq!(json["report"]["consumer"], "player_evidence_card");
    assert_eq!(json["report"]["metrics"][0]["cell"]["label"], "xG Share");
}

#[tokio::test]
async fn l2_wp009_line_combination_explorer_defaults_to_active_cache_and_explicit_unavailable_state(
) {
    let _guard = env_lock().await;
    let fixture = DataRootFixture::new();
    let app = app_with_config("20252026", "regular").await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/lines/explorer")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_text(response).await;
    assert!(body.contains("Line Combination Explorer"));
    assert!(body.contains("Report unavailable"));
    assert!(body.contains("line_combination_explorer:20252026:regular"));
    assert!(body
        .contains("analytics cache entry is missing: line_combination_explorer:20252026:regular"));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/lines/explorer")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let json: serde_json::Value =
        serde_json::from_str(&response_text(response).await).expect("json payload");
    assert_eq!(json["status"], "unavailable");
    assert_eq!(
        json["cache_key"],
        "line_combination_explorer:20252026:regular"
    );
    assert!(json["guidance"]
        .as_str()
        .expect("guidance")
        .contains("line-combination analytics cache"));
    assert_unavailable_json_non_claims(&json);
    assert!(!fixture.path().join("analytics_cache").exists());
}

#[tokio::test]
async fn l2_wp009_line_combination_explorer_renders_cache_as_line_consumer_view() {
    let _guard = env_lock().await;
    let fixture = DataRootFixture::new();
    let store = AnalyticsCacheStore::under_data_root(fixture.path());
    let record = line_combination_explorer_record("line_combination_explorer:20252026:regular");
    store
        .write_record(&record, &supported_metric_keys())
        .expect("write analytics cache record");
    let app = app_with_config("20252026", "regular").await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/lines/explorer")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_text(response).await;
    assert!(body.contains("Line Combination Explorer"));
    assert!(body.contains("line_combination_explorer:20252026:regular"));
    assert!(body.contains("xG Share"));
    assert!(body.contains("55.1%"));
    assert!(body.contains(
        "/api/v1/lines/explorer?cache_key=line_combination_explorer%3A20252026%3Aregular"
    ));
    assert!(body.contains("Not a prediction, betting, injury, or autonomous coaching claim."));
    assert!(!body.contains("guaranteed chemistry"));
    assert!(!body.contains("deployment recommendation"));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/lines/explorer")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json: serde_json::Value =
        serde_json::from_str(&response_text(response).await).expect("json payload");
    assert_eq!(json["status"], "ready");
    assert_eq!(
        json["cache_key"],
        "line_combination_explorer:20252026:regular"
    );
    assert_eq!(json["consumer"], "line_combination_explorer");
    assert_eq!(
        json["consumer_boundary"],
        "Line combination explorer reads prepared analytics-cache evidence only; it does not infer line chemistry, issue deployment recommendations, compute live analytics, make predictions, or fetch cache records."
    );
    assert_eq!(json["report"]["title"], "Line Combination Explorer");
    assert_eq!(json["report"]["consumer"], "line_combination_explorer");
    assert_eq!(json["report"]["metrics"][0]["cell"]["label"], "xG Share");
}

#[tokio::test]
async fn l2_wp009_goalie_readiness_defaults_to_active_cache_and_explicit_unavailable_state() {
    let _guard = env_lock().await;
    let fixture = DataRootFixture::new();
    let app = app_with_config("20252026", "regular").await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/goalies/readiness")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_text(response).await;
    assert!(body.contains("Goalie Readiness &amp; Workload View"));
    assert!(body.contains("Report unavailable"));
    assert!(body.contains("goalie_readiness:20252026:regular"));
    assert!(body.contains("analytics cache entry is missing: goalie_readiness:20252026:regular"));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/goalies/readiness")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let json: serde_json::Value =
        serde_json::from_str(&response_text(response).await).expect("json payload");
    assert_eq!(json["status"], "unavailable");
    assert_eq!(json["cache_key"], "goalie_readiness:20252026:regular");
    assert!(json["guidance"]
        .as_str()
        .expect("guidance")
        .contains("goalie-readiness analytics cache"));
    assert_unavailable_json_non_claims(&json);
    assert!(!fixture.path().join("analytics_cache").exists());
}

#[tokio::test]
async fn l2_wp009_goalie_readiness_renders_cache_as_goalie_consumer_view() {
    let _guard = env_lock().await;
    let fixture = DataRootFixture::new();
    let store = AnalyticsCacheStore::under_data_root(fixture.path());
    let record = goalie_readiness_record("goalie_readiness:20252026:regular");
    store
        .write_record(&record, &supported_metric_keys())
        .expect("write analytics cache record");
    let app = app_with_config("20252026", "regular").await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/goalies/readiness")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_text(response).await;
    assert!(body.contains("Goalie Readiness &amp; Workload View"));
    assert!(body.contains("goalie_readiness:20252026:regular"));
    assert!(body.contains("xG Share"));
    assert!(body.contains("55.1%"));
    assert!(
        body.contains("/api/v1/goalies/readiness?cache_key=goalie_readiness%3A20252026%3Aregular")
    );
    assert!(body.contains("Not a prediction, betting, injury, or autonomous coaching claim."));
    assert!(!body.contains("injury certainty"));
    assert!(!body.contains("deployment recommendation"));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/goalies/readiness")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json: serde_json::Value =
        serde_json::from_str(&response_text(response).await).expect("json payload");
    assert_eq!(json["status"], "ready");
    assert_eq!(json["cache_key"], "goalie_readiness:20252026:regular");
    assert_eq!(json["consumer"], "goalie_readiness");
    assert_eq!(
        json["consumer_boundary"],
        "Goalie readiness reads prepared analytics-cache evidence only; it does not issue readiness recommendations, workload decisions, live analytics, predictions, or cache fetches."
    );
    assert_eq!(json["report"]["title"], "Goalie Readiness & Workload View");
    assert_eq!(json["report"]["consumer"], "goalie_readiness");
    assert_eq!(json["report"]["metrics"][0]["cell"]["label"], "xG Share");
}

#[tokio::test]
async fn l2_wp009_practice_focus_defaults_to_active_cache_and_explicit_unavailable_state() {
    let _guard = env_lock().await;
    let fixture = DataRootFixture::new();
    let app = app_with_config("20252026", "regular").await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/practice/focus")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_text(response).await;
    assert!(body.contains("Practice Focus Report"));
    assert!(body.contains("Report unavailable"));
    assert!(body.contains("practice_focus:20252026:regular"));
    assert!(body.contains("analytics cache entry is missing: practice_focus:20252026:regular"));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/practice/focus")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let json: serde_json::Value =
        serde_json::from_str(&response_text(response).await).expect("json payload");
    assert_eq!(json["status"], "unavailable");
    assert_eq!(json["cache_key"], "practice_focus:20252026:regular");
    assert!(json["guidance"]
        .as_str()
        .expect("guidance")
        .contains("practice-focus analytics cache"));
    assert_unavailable_json_non_claims(&json);
    assert!(!fixture.path().join("analytics_cache").exists());
}

#[tokio::test]
async fn l2_wp009_practice_focus_renders_cache_as_practice_consumer_view() {
    let _guard = env_lock().await;
    let fixture = DataRootFixture::new();
    let store = AnalyticsCacheStore::under_data_root(fixture.path());
    let record = practice_focus_record("practice_focus:20252026:regular");
    store
        .write_record(&record, &supported_metric_keys())
        .expect("write analytics cache record");
    let app = app_with_config("20252026", "regular").await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/practice/focus")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_text(response).await;
    assert!(body.contains("Practice Focus Report"));
    assert!(body.contains("practice_focus:20252026:regular"));
    assert!(body.contains("xG Share"));
    assert!(body.contains("55.1%"));
    assert!(body.contains("/api/v1/practice/focus?cache_key=practice_focus%3A20252026%3Aregular"));
    assert!(body.contains("Not a prediction, betting, injury, or autonomous coaching claim."));
    assert!(!body.contains("mandatory drill plan"));
    assert!(!body.contains("autonomous practice prescription"));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/practice/focus")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json: serde_json::Value =
        serde_json::from_str(&response_text(response).await).expect("json payload");
    assert_eq!(json["status"], "ready");
    assert_eq!(json["cache_key"], "practice_focus:20252026:regular");
    assert_eq!(json["consumer"], "practice_focus_report");
    assert_eq!(
        json["consumer_boundary"],
        "Practice focus reads prepared analytics-cache evidence only; it does not issue practice plans, coaching recommendations, deployment decisions, live analytics, predictions, or cache fetches."
    );
    assert_eq!(json["report"]["title"], "Practice Focus Report");
    assert_eq!(json["report"]["consumer"], "practice_focus_report");
    assert_eq!(json["report"]["metrics"][0]["cell"]["label"], "xG Share");
}

#[tokio::test]
async fn l2_wp009_postgame_review_defaults_to_active_cache_and_explicit_unavailable_state() {
    let _guard = env_lock().await;
    let fixture = DataRootFixture::new();
    let app = app_with_config("20252026", "regular").await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/postgame/review")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_text(response).await;
    assert!(body.contains("Postgame Review Report"));
    assert!(body.contains("Report unavailable"));
    assert!(body.contains("postgame_review:20252026:regular"));
    assert!(body.contains("analytics cache entry is missing: postgame_review:20252026:regular"));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/postgame/review")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let json: serde_json::Value =
        serde_json::from_str(&response_text(response).await).expect("json payload");
    assert_eq!(json["status"], "unavailable");
    assert_eq!(json["cache_key"], "postgame_review:20252026:regular");
    assert!(json["guidance"]
        .as_str()
        .expect("guidance")
        .contains("postgame-review analytics cache"));
    assert_unavailable_json_non_claims(&json);
    assert!(!fixture.path().join("analytics_cache").exists());
}

#[tokio::test]
async fn l2_wp009_postgame_review_renders_cache_as_postgame_consumer_view() {
    let _guard = env_lock().await;
    let fixture = DataRootFixture::new();
    let store = AnalyticsCacheStore::under_data_root(fixture.path());
    let record = postgame_review_record("postgame_review:20252026:regular");
    store
        .write_record(&record, &supported_metric_keys())
        .expect("write analytics cache record");
    let app = app_with_config("20252026", "regular").await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/postgame/review")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_text(response).await;
    assert!(body.contains("Postgame Review Report"));
    assert!(body.contains("postgame_review:20252026:regular"));
    assert!(body.contains("xG Share"));
    assert!(body.contains("55.1%"));
    assert!(body.contains("/api/v1/postgame/review?cache_key=postgame_review%3A20252026%3Aregular"));
    assert!(body.contains("Not a prediction, betting, injury, or autonomous coaching claim."));
    assert!(!body.contains("causal win explanation"));
    assert!(!body.contains("blame assignment"));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/postgame/review")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json: serde_json::Value =
        serde_json::from_str(&response_text(response).await).expect("json payload");
    assert_eq!(json["status"], "ready");
    assert_eq!(json["cache_key"], "postgame_review:20252026:regular");
    assert_eq!(json["consumer"], "postgame_review_report");
    assert_eq!(
        json["consumer_boundary"],
        "Postgame review reads prepared analytics-cache evidence only; it does not issue postgame conclusions, adjustment plans, blame assignments, live analytics, predictions, or cache fetches."
    );
    assert_eq!(json["report"]["title"], "Postgame Review Report");
    assert_eq!(json["report"]["consumer"], "postgame_review_report");
    assert_eq!(json["report"]["metrics"][0]["cell"]["label"], "xG Share");
}

#[tokio::test]
async fn l2_wp009_postgame_adjustments_defaults_to_active_cache_and_explicit_unavailable_state() {
    let _guard = env_lock().await;
    let fixture = DataRootFixture::new();
    let app = app_with_config("20252026", "regular").await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/postgame/adjustments")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_text(response).await;
    assert!(body.contains("Postgame Adjustment Review"));
    assert!(body.contains("Report unavailable"));
    assert!(body.contains("postgame_adjustments:20252026:regular"));
    assert!(
        body.contains("analytics cache entry is missing: postgame_adjustments:20252026:regular")
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/postgame/adjustments")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let json: serde_json::Value =
        serde_json::from_str(&response_text(response).await).expect("json payload");
    assert_eq!(json["status"], "unavailable");
    assert_eq!(json["cache_key"], "postgame_adjustments:20252026:regular");
    assert!(json["guidance"]
        .as_str()
        .expect("guidance")
        .contains("postgame-adjustments analytics cache"));
    assert_unavailable_json_non_claims(&json);
    assert!(!fixture.path().join("analytics_cache").exists());
}

#[tokio::test]
async fn l2_wp009_postgame_adjustments_renders_cache_as_postgame_consumer_view() {
    let _guard = env_lock().await;
    let fixture = DataRootFixture::new();
    let store = AnalyticsCacheStore::under_data_root(fixture.path());
    let record = postgame_adjustments_record("postgame_adjustments:20252026:regular");
    store
        .write_record(&record, &supported_metric_keys())
        .expect("write analytics cache record");
    let app = app_with_config("20252026", "regular").await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/postgame/adjustments")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_text(response).await;
    assert!(body.contains("Postgame Review Report"));
    assert!(body.contains("postgame_adjustments:20252026:regular"));
    assert!(body.contains("xG Share"));
    assert!(body.contains("55.1%"));
    assert!(body.contains(
        "/api/v1/postgame/adjustments?cache_key=postgame_adjustments%3A20252026%3Aregular"
    ));
    assert!(body.contains("Not a prediction, betting, injury, or autonomous coaching claim."));
    assert!(!body.contains("automatic correction plan"));
    assert!(!body.contains("blame assignment"));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/postgame/adjustments")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json: serde_json::Value =
        serde_json::from_str(&response_text(response).await).expect("json payload");
    assert_eq!(json["status"], "ready");
    assert_eq!(json["cache_key"], "postgame_adjustments:20252026:regular");
    assert_eq!(json["consumer"], "postgame_review_report");
    assert_eq!(
        json["consumer_boundary"],
        "Postgame review reads prepared analytics-cache evidence only; it does not issue postgame conclusions, adjustment plans, blame assignments, live analytics, predictions, or cache fetches."
    );
    assert_eq!(json["report"]["title"], "Postgame Review Report");
    assert_eq!(json["report"]["consumer"], "postgame_review_report");
    assert_eq!(json["report"]["metrics"][0]["cell"]["label"], "xG Share");
}

#[tokio::test]
async fn l2_wp009_agent_evidence_defaults_to_active_cache_and_explicit_unavailable_state() {
    let _guard = env_lock().await;
    let fixture = DataRootFixture::new();
    let app = app_with_config("20252026", "regular").await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/agents/evidence")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_text(response).await;
    assert!(body.contains("Agent Evidence Summary"));
    assert!(body.contains("Report unavailable"));
    assert!(body.contains("agent_evidence:20252026:regular"));
    assert!(body.contains("analytics cache entry is missing: agent_evidence:20252026:regular"));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/agents/evidence")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let json: serde_json::Value =
        serde_json::from_str(&response_text(response).await).expect("json payload");
    assert_eq!(json["status"], "unavailable");
    assert_eq!(json["cache_key"], "agent_evidence:20252026:regular");
    assert!(json["guidance"]
        .as_str()
        .expect("guidance")
        .contains("agent-evidence analytics cache"));
    assert_unavailable_json_non_claims(&json);
    assert!(!fixture.path().join("analytics_cache").exists());
}

#[tokio::test]
async fn l2_wp009_agent_evidence_renders_cache_as_agent_consumer_view() {
    let _guard = env_lock().await;
    let fixture = DataRootFixture::new();
    let store = AnalyticsCacheStore::under_data_root(fixture.path());
    let record = agent_evidence_record("agent_evidence:20252026:regular");
    store
        .write_record(&record, &supported_metric_keys())
        .expect("write analytics cache record");
    let app = app_with_config("20252026", "regular").await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/agents/evidence")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_text(response).await;
    assert!(body.contains("Agent Evidence Summary"));
    assert!(body.contains("agent_evidence:20252026:regular"));
    assert!(body.contains("xG Share"));
    assert!(body.contains("55.1%"));
    assert!(body.contains("/api/v1/agents/evidence?cache_key=agent_evidence%3A20252026%3Aregular"));
    assert!(body.contains("Not a prediction, betting, injury, or autonomous coaching claim."));
    assert!(!body.contains("execute recommendation"));
    assert!(!body.contains("autonomous action"));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/agents/evidence")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json: serde_json::Value =
        serde_json::from_str(&response_text(response).await).expect("json payload");
    assert_eq!(json["status"], "ready");
    assert_eq!(json["cache_key"], "agent_evidence:20252026:regular");
    assert_eq!(json["consumer"], "agent_evidence");
    assert_eq!(
        json["consumer_boundary"],
        "Agent evidence reads prepared analytics-cache evidence only; it does not execute recommendations, take autonomous actions, call agents, compute live analytics, make predictions, or fetch cache records."
    );
    assert_eq!(json["report"]["title"], "Agent Evidence Summary");
    assert_eq!(json["report"]["consumer"], "agent_evidence");
    assert_eq!(json["report"]["metrics"][0]["cell"]["label"], "xG Share");
}

#[tokio::test]
async fn l2_wp009_analytics_cache_report_renders_cache_envelope_without_recomputing() {
    let _guard = env_lock().await;
    let fixture = DataRootFixture::new();
    let store = AnalyticsCacheStore::under_data_root(fixture.path());
    let record = coach_record("coach_dashboard:20252026:regular");
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
