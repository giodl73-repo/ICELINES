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
    // King.1.x patch (broadcast review): lock the charset so a future
    // template refactor doesn't accidentally serve raw bytes that the
    // browser interprets in the wrong encoding (UTF-8 is mandatory
    // for the multi-language player names like "Slafkovský").
    assert!(
        content_type.contains("charset=utf-8") || content_type.contains("charset=UTF-8"),
        "home page Content-Type must declare charset=utf-8, got: {content_type}"
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
    // King.1.4 — content from the askama template should appear.
    assert!(
        body.contains("Welcome"),
        "home page should render the askama template content"
    );
}

/// l1_html_each_route_has_active_season_header
/// — King.1.4 fence (broadcast finding, advanced from King.6 → King.1.4):
///   every HTML page must render the active-(season, season_type)
///   sticky header so time-travel via PATCH is never silent.
///
/// Today only `/` is mounted. Each future sub-phase adds its routes
/// to the route list below — King.2 adds `/leaders`, King.3 adds
/// `/player/:id`, etc. The fence catches any route that forgets to
/// thread `active_label` into its template.
#[tokio::test]
async fn l1_html_each_route_has_active_season_header() {
    let app = router(WebState::new());

    // Default WebConfig::default() uses CURRENT_SEASON_STR + "regular"
    // → label "25-26 · Regular". The fence checks for the structural
    // marker (the season-header CSS class) plus the label substring.
    let html_routes: &[&str] = &[
        "/",
        // King.2: "/leaders",
        // King.3: "/player/8478402",
        // King.4: "/team/SEA", "/depth", "/class/2022",
        // King.5: "/goalies",
        // King.6: "/reports", "/seasons",
        // King.7: "/scores", "/schedule", "/playoffs",
        // King.8: "/transactions", "/groups", "/games", "/search", "/docs",
        // King.9: "/fantasy",
    ];

    for route in html_routes {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(*route)
                    .body(Body::empty())
                    .expect("request builder ok"),
            )
            .await
            .expect("dispatch ok");

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "{route} should return 200"
        );

        let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
            .await
            .expect("body fits");
        let body = std::str::from_utf8(&bytes).expect("html is utf-8");

        assert!(
            body.contains("season-header"),
            "{route} must include the .season-header element \
             (broadcast a11y/UX contract)"
        );
        // CURRENT_SEASON_STR is "20252026" → label "25-26 · Regular"
        assert!(
            body.contains("25-26 · Regular"),
            "{route} must render the active-season label '25-26 · Regular' \
             (got body without it — make sure the route's template extends \
             base.html and the handler passes active_label)"
        );
    }
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
