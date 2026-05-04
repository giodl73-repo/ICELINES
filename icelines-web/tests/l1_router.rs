//! L1 — exercise the King.1.1 router via `tower::ServiceExt::oneshot`.
//!
//! No socket binding yet; that lands in King.1.5 once the
//! `Commands::Serve` driver exists. King.1.1's router only mounts `/`
//! plus the (future) extension points, so this is the right scope.
//!
//! Per the spec's testing strategy, L1 tests live under
//! `icelines-web/tests/`. Each file is its own binary; share fixtures
//! via `tests/common/mod.rs` once King.2 introduces them.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use icelines_web::{router, WebState};
use tower::util::ServiceExt;

/// l1_get_root_returns_200_html
/// — placeholder home page handler smoke. Spec promises the bare route
///   returns the full HTML page (not a fragment); fragments are routed
///   under `?partial=*` (King.2). Today we just verify 200 + HTML
///   content-type so the contract is locked from day one.
#[tokio::test]
async fn l1_get_root_returns_200_html() {
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/")
                .body(Body::empty())
                .expect("request builder should succeed"),
        )
        .await
        .expect("oneshot dispatch should not fail");

    assert_eq!(response.status(), StatusCode::OK);

    let content_type = response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .expect("home page should set Content-Type")
        .to_str()
        .expect("content-type is ascii");
    assert!(
        content_type.starts_with("text/html"),
        "home page should be HTML, got Content-Type: {content_type}"
    );

    let body_bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("body should fit in 64 KiB");
    let body = std::str::from_utf8(&body_bytes).expect("HTML response is utf-8");
    assert!(
        body.contains("<!doctype html>"),
        "home page should be a full HTML document; got body starting with: {}",
        &body[..body.len().min(80)]
    );
}

/// l1_unknown_route_returns_404
/// — axum's default not-found handler. Once King.1.6 adds host-header
///   validation we'll add a 421 case for DNS rebinding, but the basic
///   404 contract starts here.
#[tokio::test]
async fn l1_unknown_route_returns_404() {
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/this-route-does-not-exist")
                .body(Body::empty())
                .expect("request builder should succeed"),
        )
        .await
        .expect("oneshot dispatch should not fail");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
