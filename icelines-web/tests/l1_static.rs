//! L1 — `/static/*` end-to-end via `tower::ServiceExt::oneshot`.
//!
//! King.1.3 vendored asset pipeline. Each test fires a real GET
//! through the router, asserts status / Content-Type / Cache-Control
//! / ETag — the full HTTP-layer contract.

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use icelines_web::{router, WebState};
use tower::util::ServiceExt;

async fn fire(path: &str) -> axum::http::Response<Body> {
    let app = router(WebState::new());
    app.oneshot(
        Request::builder()
            .uri(path)
            .body(Body::empty())
            .expect("request builder ok"),
    )
    .await
    .expect("dispatch ok")
}

async fn body_text(path: &str) -> String {
    let resp = fire(path).await;
    let bytes = axum::body::to_bytes(resp.into_body(), 256 * 1024)
        .await
        .expect("body bytes");
    String::from_utf8(bytes.to_vec()).expect("utf-8 body")
}

/// l1_static_htmx_serves_javascript_with_cache_headers
/// — full HTTP round-trip on `/static/htmx.min.js`.
#[tokio::test]
async fn l1_static_htmx_serves_javascript_with_cache_headers() {
    let resp = fire("/static/htmx.min.js").await;
    assert_eq!(resp.status(), StatusCode::OK);

    let ct = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .expect("content-type set")
        .to_str()
        .unwrap();
    assert!(
        ct.starts_with("application/javascript"),
        "htmx.min.js Content-Type must be application/javascript, got: {ct}"
    );

    let cc = resp
        .headers()
        .get(header::CACHE_CONTROL)
        .expect("cache-control set")
        .to_str()
        .unwrap();
    assert!(cc.contains("immutable"));
    assert!(cc.contains("max-age=31536000"));

    let etag = resp
        .headers()
        .get(header::ETAG)
        .expect("etag set")
        .to_str()
        .unwrap();
    assert!(etag.starts_with('"') && etag.ends_with('"'));
}

#[tokio::test]
async fn l1_static_dashboard_js_serves_javascript_with_cache_headers() {
    let resp = fire("/static/dashboard.js").await;
    assert_eq!(resp.status(), StatusCode::OK);

    let ct = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .expect("content-type set")
        .to_str()
        .unwrap();
    assert!(
        ct.starts_with("application/javascript"),
        "dashboard.js Content-Type must be application/javascript, got: {ct}"
    );

    let cc = resp
        .headers()
        .get(header::CACHE_CONTROL)
        .expect("cache-control set")
        .to_str()
        .unwrap();
    assert!(cc.contains("immutable"));
}

/// l1_static_css_serves_text_css_with_cache_headers
#[tokio::test]
async fn l1_static_css_serves_text_css_with_cache_headers() {
    let resp = fire("/static/style.css").await;
    assert_eq!(resp.status(), StatusCode::OK);

    let ct = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        ct.starts_with("text/css"),
        "style.css Content-Type must be text/css, got: {ct}"
    );
    assert!(
        ct.contains("charset=utf-8"),
        "style.css must declare utf-8 charset, got: {ct}"
    );

    let cc = resp
        .headers()
        .get(header::CACHE_CONTROL)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(cc.contains("immutable"));
}

/// Prince.4 shared route layout primitives.
#[tokio::test]
async fn l1_static_css_contains_prince_route_layout_classes() {
    let css = body_text("/static/style.css").await;

    for class in [
        ".page-section",
        ".section-heading",
        ".section-link-row",
        ".filter-panel",
        ".filter-grid",
        ".form-actions",
        ".inline-form-row",
        ".filter-label",
        ".filter-help",
        ".filter-input-wide",
        ".chip-strip",
        ".filter-chip",
        ".active-filter-line",
        ".accordion-summary",
        ".bio-filter-grid",
        ".range-input",
        ".link-button-secondary",
        ".sort-link",
        ".action-row",
        ".inline-compare",
        ".outline-button",
        ".player-identity",
        ".player-headshot",
        ".favorite-button",
        ".stat-grid",
        ".stat-label",
        ".stat-value",
        ".playoff-round",
        ".playoff-series-grid",
        ".playoff-series-card",
        ".playoff-series-score",
        ".numeric",
        ".rank-cell",
        ".brand-link",
        ".lede",
        ".not-found-panel",
        ".centered-form",
        ".back-link-row",
        ".picker-nav",
        ".inline-separator",
        ".table-score",
        ".kind-chip",
        ".game-live",
        ".team-banner-meta",
        ".team-banner-button",
        ".jaw-shell",
        ".jaw-workbench-nav",
        ".jaw-body",
        ".jaw-workspace",
        ".jaw-command-examples",
        ".jaw-command-status",
        "data-dashboard-pane-collapsed",
        "position: sticky",
    ] {
        assert!(css.contains(class), "style.css missing {class}");
    }
}

/// l1_static_svg_serves_image_svg_xml
#[tokio::test]
async fn l1_static_svg_serves_image_svg_xml() {
    let resp = fire("/static/icelines.svg").await;
    assert_eq!(resp.status(), StatusCode::OK);

    let ct = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        ct.starts_with("image/svg+xml"),
        "icelines.svg Content-Type must be image/svg+xml, got: {ct}"
    );
}

/// l1_static_unknown_asset_returns_404
/// — Spec rule: dispatch by name, not extension. A typo or unknown
///   asset returns 404 (not the wrong content with the wrong MIME).
#[tokio::test]
async fn l1_static_unknown_asset_returns_404() {
    let resp = fire("/static/unknown.js").await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// l1_static_etag_matches_workspace_version
/// — King.1.3 contract: the strong ETag value contains the workspace
///   version. New release → new ETag → all client caches bust.
#[tokio::test]
async fn l1_static_etag_matches_workspace_version() {
    let resp = fire("/static/style.css").await;
    let etag = resp.headers().get(header::ETAG).unwrap().to_str().unwrap();
    let version = env!("CARGO_PKG_VERSION");
    assert!(
        etag.contains(version),
        "ETag must contain workspace version {version}, got: {etag}"
    );
}

/// l1_static_etag_consistent_across_assets_within_release
/// — Within a single binary build, all `/static/*` assets share an
///   ETag (the workspace version). This is intentional: they're
///   versioned as a unit.
#[tokio::test]
async fn l1_static_etag_consistent_across_assets_within_release() {
    let css_etag = fire("/static/style.css")
        .await
        .headers()
        .get(header::ETAG)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let js_etag = fire("/static/htmx.min.js")
        .await
        .headers()
        .get(header::ETAG)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let svg_etag = fire("/static/icelines.svg")
        .await
        .headers()
        .get(header::ETAG)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let dashboard_js_etag = fire("/static/dashboard.js")
        .await
        .headers()
        .get(header::ETAG)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    assert_eq!(css_etag, js_etag, "css and js must share ETag");
    assert_eq!(js_etag, svg_etag, "js and svg must share ETag");
    assert_eq!(
        svg_etag, dashboard_js_etag,
        "dashboard.js must share release ETag"
    );
}
