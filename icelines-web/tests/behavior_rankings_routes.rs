use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use icelines_web::{router, WebState};
use tower::ServiceExt;

#[tokio::test]
async fn behavior_rankings_html_exposes_scale_and_all_teams() {
    let response = router(WebState::new())
        .oneshot(
            Request::builder()
                .uri("/icecast/20262027/behavior-rankings?scale=rookie_opportunity")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 2 * 1024 * 1024)
        .await
        .unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("The Front Office — Team Behavior Rankings"));
    assert!(html.contains("rookie_opportunity"));
    assert!(html.contains("NYR"));
    assert!(html.contains("SEA"));
}

#[tokio::test]
async fn behavior_rankings_json_contains_three_seasons_and_32_teams() {
    let response = router(WebState::new())
        .oneshot(
            Request::builder()
                .uri("/api/v1/icecast/20262027/behavior-rankings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 2 * 1024 * 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["rankings"]["teams"], 32);
    assert_eq!(json["season_evidence"].as_array().unwrap().len(), 96);
    assert_eq!(json["window_seasons"], 3);
}
