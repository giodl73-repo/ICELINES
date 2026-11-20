use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use icelines_web::{router, WebState};
use tower::ServiceExt;
#[tokio::test]
async fn entry_and_invalid_external_selector_are_local_and_safe() {
    let app = router(WebState::new());
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/goal-analysis")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = String::from_utf8(
        to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(body.contains("Analyze a goal") && body.contains("goal-analysis-data"));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/goal-analysis?target=https%3A%2F%2Fevil.test%2Fgoal%2F1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
#[tokio::test]
async fn disabled_live_api_cannot_invent_a_missing_goal() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = WebState::new();
    state.goal_analysis_root = std::sync::Arc::new(Some(dir.path().into()));
    let response = router(state)
        .oneshot(
            Request::builder()
                .uri("/api/v1/goal-analysis?target=2025021167%2F996")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = String::from_utf8(
        to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(body.contains("disabled") && !body.contains("\"audit\""));
}
