use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use icelines_core::{CardKind, OrganizationWindowBoardView};
use icelines_web::{router, WebState};
use tower::ServiceExt;

#[tokio::test]
async fn registered_window_json_retains_all_32_and_etag() {
    let response = router(WebState::new())
        .oneshot(
            Request::builder()
                .uri("/api/v1/window/balanced.v1/20262027")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key(header::ETAG));
    let body = axum::body::to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    let board: OrganizationWindowBoardView = serde_json::from_slice(&body).unwrap();
    assert_eq!(board.organizations.len(), 32);
    assert_eq!(board.manifest.manifest_id, "balanced.v1");
    assert!(board
        .organizations
        .iter()
        .all(|row| row.overall.rank.is_none()));
}

#[tokio::test]
async fn focused_window_html_and_card_use_same_registered_artifact() {
    let app = router(WebState::new());
    let html_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/window/balanced.v1/20262027?team=NYR")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(html_response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(html_response.into_body(), 2 * 1024 * 1024)
        .await
        .unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("The Window"));
    assert!(html.contains("NYR"));
    assert!(!html.contains(">SEA</a>"));
    assert!(html.contains("class=\"skip-link\""));
    assert!(html.contains("<main id=\"main\" tabindex=\"-1\">"));
    assert!(html.contains("<caption>"));
    assert!(html.contains("aria-label=\"Organization Window standings\""));
    assert!(html.contains("<th scope=\"col\">Team</th>"));

    let card_response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cards/organization-window/20262027/NYR")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(card_response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(card_response.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    let card: icelines_core::CardDocumentView = serde_json::from_slice(&body).unwrap();
    assert_eq!(card.card_kind, CardKind::OrganizationWindow);
    assert_eq!(card.pages.len(), 2);
}
