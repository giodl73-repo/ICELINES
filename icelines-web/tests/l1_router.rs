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
use std::sync::{Mutex, OnceLock};
use tower::util::ServiceExt;

fn home_env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

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
    // Originally checked for "Welcome"; the home page was rebuilt
    // with top-3 preview sections in King.8.x. The IceLines title
    // and the "Top scorers" / "Top goalies" headings are stable.
    assert!(
        body.contains("IceLines") && body.contains("Top scorers"),
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
        "/poach",
        "/watchlist",
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

/// l1_depth_route_returns_200_html
/// — Phase Lady Byng follow-up. The /depth route mirrors the TUI Depth
///   tab; this fence proves it boots and renders the askama template
///   without panicking. Asserts the route resolves to 200, returns HTML
///   with the expected charset, and contains the "Depth Rankings"
///   heading from depth.html.
#[tokio::test]
async fn l1_depth_route_returns_200_html() {
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/depth")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "/depth should return 200"
    );

    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let body = std::str::from_utf8(&bytes).expect("html is utf-8");
    assert!(
        body.contains("Depth Rankings"),
        "/depth page must render its h1 heading, got start:\n{}",
        &body[..body.len().min(120)]
    );
    // The nav bar should also include the new Depth link on every page.
    assert!(
        body.contains("href=\"/depth\""),
        "/depth must be linked in the global nav"
    );
}

#[tokio::test]
async fn l1_watchlist_route_returns_200_html() {
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/watchlist")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let body = std::str::from_utf8(&bytes).expect("html is utf-8");

    assert!(body.contains("Watchlist"));
    assert!(body.contains("href=\"/poach\""));
    assert!(body.contains("icelines tui poach"));
}

#[tokio::test]
async fn l1_watchlist_route_renders_watch_reason_metadata() {
    let _guard = home_env_lock();
    let dir = tempfile::TempDir::new().expect("temp home");
    let prev_userprofile = std::env::var_os("USERPROFILE");
    let prev_home = std::env::var_os("HOME");
    std::env::set_var("USERPROFILE", dir.path());
    std::env::set_var("HOME", dir.path());

    let db_dir = dir.path().join(".icelines");
    std::fs::create_dir_all(&db_dir).expect("create db dir");
    let conn = rusqlite::Connection::open(db_dir.join("icelines.db")).expect("open db");
    conn.execute_batch(
        "CREATE TABLE groups (
            name TEXT PRIMARY KEY,
            description TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
         );
         CREATE TABLE group_members (
            group_name TEXT NOT NULL,
            entity_ref TEXT NOT NULL,
            added_at TEXT NOT NULL,
            PRIMARY KEY (group_name, entity_ref)
         );
         CREATE TABLE watch_notes (
            entity_ref TEXT PRIMARY KEY,
            reason TEXT NOT NULL DEFAULT '',
            source TEXT NOT NULL DEFAULT '',
            updated_at TEXT NOT NULL
         );
         INSERT INTO groups VALUES ('Watchlist', '', datetime('now'));
         INSERT INTO group_members VALUES ('Watchlist', 'player:matthew knies', datetime('now'));
         INSERT INTO watch_notes VALUES (
            'player:matthew knies',
            'Poach score 72.0; confidence High; PP1 promotion',
            'tui-poach',
            datetime('now')
         );",
    )
    .expect("seed watchlist db");

    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/watchlist")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    match prev_userprofile {
        Some(p) => std::env::set_var("USERPROFILE", p),
        None => std::env::remove_var("USERPROFILE"),
    }
    match prev_home {
        Some(p) => std::env::set_var("HOME", p),
        None => std::env::remove_var("HOME"),
    }

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let body = std::str::from_utf8(&bytes).expect("html is utf-8");

    assert!(body.contains("matthew knies"));
    assert!(body.contains("Poach score 72.0"));
}

#[tokio::test]
async fn l1_watchlist_json_returns_watch_reason_metadata() {
    let _guard = home_env_lock();
    let dir = tempfile::TempDir::new().expect("temp home");
    let prev_userprofile = std::env::var_os("USERPROFILE");
    let prev_home = std::env::var_os("HOME");
    std::env::set_var("USERPROFILE", dir.path());
    std::env::set_var("HOME", dir.path());

    let db_dir = dir.path().join(".icelines");
    std::fs::create_dir_all(&db_dir).expect("create db dir");
    let conn = rusqlite::Connection::open(db_dir.join("icelines.db")).expect("open db");
    conn.execute_batch(
        "CREATE TABLE groups (
            name TEXT PRIMARY KEY,
            description TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
         );
         CREATE TABLE group_members (
            group_name TEXT NOT NULL,
            entity_ref TEXT NOT NULL,
            added_at TEXT NOT NULL,
            PRIMARY KEY (group_name, entity_ref)
         );
         CREATE TABLE watch_notes (
            entity_ref TEXT PRIMARY KEY,
            reason TEXT NOT NULL DEFAULT '',
            source TEXT NOT NULL DEFAULT '',
            updated_at TEXT NOT NULL
         );
         INSERT INTO groups VALUES ('Watchlist', '', datetime('now'));
         INSERT INTO group_members VALUES ('Watchlist', 'player:matthew knies', datetime('now'));
         INSERT INTO watch_notes VALUES (
            'player:matthew knies',
            'Poach score 72.0; confidence High; PP1 promotion',
            'tui-poach',
            '2026-05-09T12:00:00Z'
         );",
    )
    .expect("seed watchlist db");

    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/watchlist")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    match prev_userprofile {
        Some(p) => std::env::set_var("USERPROFILE", p),
        None => std::env::remove_var("USERPROFILE"),
    }
    match prev_home {
        Some(p) => std::env::set_var("HOME", p),
        None => std::env::remove_var("HOME"),
    }

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let body: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");

    assert_eq!(body["schema_version"], "watchlist.v1");
    assert_eq!(body["route"], "watchlist");
    assert_eq!(body["meta"]["group"], "Watchlist");
    assert_eq!(body["meta"]["player_count"], 1);
    assert_eq!(body["data"][0]["kind"], "player");
    assert_eq!(body["data"][0]["key"], "matthew knies");
    assert_eq!(
        body["data"][0]["reason"],
        "Poach score 72.0; confidence High; PP1 promotion"
    );
    assert_eq!(body["data"][0]["source"], "tui-poach");
    assert_eq!(body["data"][0]["updated_at"], "2026-05-09T12:00:00Z");
}

#[tokio::test]
async fn l1_poach_route_returns_200_html() {
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/poach?category=hits,blocks&top=5")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let body = std::str::from_utf8(&bytes).expect("html is utf-8");

    assert!(body.contains("Fantasy Poacher"));
    assert!(body.contains("href=\"/poach\""));
    assert!(body.contains("Missing poacher source data"));
}

#[tokio::test]
async fn l1_poach_report_route_returns_report_html() {
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/reports/poach?category=hits&top=5")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let body = std::str::from_utf8(&bytes).expect("html is utf-8");

    assert!(body.contains("Fantasy Poacher Report"));
    assert!(body.contains("Top Adds"));
    assert!(body.contains("Source Omissions"));
    assert!(body.contains("href=\"/poach\""));
}

#[tokio::test]
async fn l1_weekly_report_route_returns_prep_sections() {
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/reports/weekly?league=Main%20League&category=hits,blocks&top=5")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let body = std::str::from_utf8(&bytes).expect("html is utf-8");

    assert!(body.contains("Weekly Fantasy Prep Report"));
    assert!(body.contains("Category Specialists"));
    assert!(body.contains("Deployment Risers"));
    assert!(body.contains("Risk Discounts"));
    assert!(body.contains("Watched Player Alerts"));
}

#[tokio::test]
async fn l1_poach_json_returns_view_model_contract() {
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/poach?category=hits,blocks&pos=LW&top=5")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");

    assert_eq!(json["scoring_scheme"], "yahoo-standard");
    assert_eq!(json["query"]["categories"][0], "hits");
    assert_eq!(json["query"]["positions"][0], "LeftWing");
    assert_eq!(json["empty_state"]["kind"], "missing_source");
}

#[tokio::test]
async fn l1_watch_rules_json_returns_shared_contract() {
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/watch-rules")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");

    assert_eq!(json["context"]["completeness"], "partial");
    assert_eq!(json["rules"][0]["id"], "category-hits-pace");
    assert_eq!(json["rules"][2]["id"], "deployment-promotion");
    assert_eq!(json["rules"][2]["unsupported_sources"][0], "shifts");
    assert_eq!(json["rules"][4]["unsupported_sources"][0], "fantasy_import");
}

/// l1_career_route_missing_league_returns_400 (Calder.4)
/// — `/career` without `?league=…` rejects with 400 + helpful body.
#[tokio::test]
async fn l1_career_route_missing_league_returns_400() {
    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/career")
                .body(Body::empty())
                .expect("ok"),
        )
        .await
        .expect("dispatch ok");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("body");
    let body = std::str::from_utf8(&bytes).unwrap_or("");
    assert!(
        body.contains("league") && body.contains("OHL"),
        "error body should hint at the right call shape, got:\n{body}"
    );
}

/// l1_api_career_envelope_shape (Calder.4)
/// — `/api/v1/career` envelope. When the local store is empty the
///   handler returns 400 with a helpful message; we accept either
///   shape and assert the right keys for whichever side fires.
#[tokio::test]
async fn l1_api_career_envelope_shape() {
    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/career?league=OHL&season=20142015")
                .body(Body::empty())
                .expect("ok"),
        )
        .await
        .expect("dispatch ok");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("body");
    let v: serde_json::Value = serde_json::from_slice(&bytes).expect("valid json");
    let obj = v.as_object().expect("object");
    if status == StatusCode::OK {
        // Store populated — assert envelope shape.
        let keys: std::collections::BTreeSet<_> = obj.keys().map(String::as_str).collect();
        let want: std::collections::BTreeSet<_> = ["data", "meta", "route", "schema_version"]
            .iter()
            .copied()
            .collect();
        assert_eq!(keys, want, "envelope diverged: {keys:?}");
        assert_eq!(obj["route"], serde_json::json!("career"));
        assert!(obj["data"].is_array());
    } else {
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(
            obj.contains_key("error"),
            "BAD_REQUEST must carry error field"
        );
    }
}

/// l1_depth_json_envelope_shape (T3)
/// — `/api/v1/depth` is the JSON twin of `/depth`. Every list page on
///   the web surface gets one (King.2.4 convention) so external scripts
///   don't have to scrape HTML. This fence pins the literal envelope
///   keys + types so a schema bump can't slip in unannounced.
#[tokio::test]
async fn l1_depth_json_envelope_shape() {
    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/depth")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");
    assert_eq!(response.status(), StatusCode::OK);
    let ct = response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        ct.starts_with("application/json"),
        "expected JSON content-type, got {ct:?}"
    );

    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("body fits");
    let v: serde_json::Value = serde_json::from_slice(&bytes).expect("valid json");
    let obj = v.as_object().expect("envelope is an object");
    let keys: std::collections::BTreeSet<_> = obj.keys().map(String::as_str).collect();
    let want: std::collections::BTreeSet<_> = ["data", "meta", "route", "schema_version"]
        .iter()
        .copied()
        .collect();
    assert_eq!(keys, want, "envelope keys diverged: {keys:?}");
    assert_eq!(obj["schema_version"], serde_json::json!(1));
    assert_eq!(obj["route"], serde_json::json!("depth"));
    assert!(obj["data"].is_array(), "data must be an array");
    let meta_keys: std::collections::BTreeSet<_> = obj["meta"]
        .as_object()
        .expect("meta is an object")
        .keys()
        .map(String::as_str)
        .collect();
    let want_meta: std::collections::BTreeSet<_> =
        ["count", "scoring_mode", "season", "season_type"]
            .iter()
            .copied()
            .collect();
    assert_eq!(meta_keys, want_meta, "meta keys diverged: {meta_keys:?}");
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

// ── Season-type toggle (UX.E, 2026-05-04) ───────────────────────────
//
// `/season-type/:kind` flips `WebState.config.active_season_type` and
// redirects back to where the user came from. The route is the
// only writer of season-type today (the Reports overlay's PATCH
// /api/v1/active-season is the long-term destination per the spec).
//
// Locked behavior:
// - `playoff` and `playoffs` both normalize to "playoff".
// - `regular` and anything-else (including injection attempts)
//   normalize to "regular" — the whitelist is the security boundary
//   so a malformed URL can't poison config.
// - Response is 303 See Other with a Location header (per HTTP, GET
//   handlers redirect with 303, not 302, when the result is a new
//   resource view).
// - Location preserves the user's previous page when Referer is set
//   to a same-origin URL; falls back to "/" otherwise.

/// Helper — dispatch one request and return (status, location header).
async fn flip_season_type(
    state: WebState,
    kind: &str,
    referer: Option<&str>,
) -> (StatusCode, Option<String>) {
    let app = router(state);
    let mut req = Request::builder().uri(format!("/season-type/{kind}"));
    if let Some(r) = referer {
        req = req.header(axum::http::header::REFERER, r);
    }
    let response = app
        .oneshot(req.body(Body::empty()).expect("build request"))
        .await
        .expect("oneshot");
    let location = response
        .headers()
        .get(axum::http::header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_owned());
    (response.status(), location)
}

/// l1_season_type_playoff_flips_state_and_redirects_303
/// — happy path: `/season-type/playoff` flips state.config.active_season_type
///   from default ("regular") to "playoff" AND returns 303.
#[tokio::test]
async fn l1_season_type_playoff_flips_state_and_redirects_303() {
    let state = WebState::new();
    let captured = state.config.clone();

    let (status, location) = flip_season_type(state, "playoff", None).await;

    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location.as_deref(), Some("/"));
    let cfg = captured.read().await;
    assert_eq!(cfg.active_season_type, "playoff");
    assert!(
        cfg.active_label.contains("Playoff"),
        "active_label should reflect the new type, got: {}",
        cfg.active_label
    );
}

/// l1_season_type_regular_flips_back
/// — round-trip: after a flip to playoff, flipping to "regular" must
///   return state.config to "regular". Active label re-formats too.
#[tokio::test]
async fn l1_season_type_regular_flips_back() {
    let state = WebState::new();
    {
        let mut cfg = state.config.write().await;
        *cfg = icelines_web::WebConfig::new("20252026", "playoff");
    }
    let captured = state.config.clone();

    let (status, _) = flip_season_type(state, "regular", None).await;

    assert_eq!(status, StatusCode::SEE_OTHER);
    let cfg = captured.read().await;
    assert_eq!(cfg.active_season_type, "regular");
    assert!(cfg.active_label.contains("Regular"));
}

/// l1_season_type_plural_playoffs_normalizes_to_singular
/// — both "playoff" and "playoffs" must work. The path token may
///   read more naturally as "playoffs" but the canonical config
///   value is the singular form (lockstep with the CLI's
///   `--season-type` flag).
#[tokio::test]
async fn l1_season_type_plural_playoffs_normalizes_to_singular() {
    let state = WebState::new();
    let captured = state.config.clone();

    let (status, _) = flip_season_type(state, "playoffs", None).await;

    assert_eq!(status, StatusCode::SEE_OTHER);
    let cfg = captured.read().await;
    assert_eq!(cfg.active_season_type, "playoff");
}

/// l1_season_type_unknown_kind_falls_back_to_regular
/// — security boundary: a bogus path component MUST NOT poison the
///   config (e.g. /season-type/<script>alert(1)</script>). Whitelist
///   on the way in: anything not "playoff*" → "regular". This test
///   also covers the case where a user follows a stale link with
///   "Regular" capitalized — case-insensitive.
#[tokio::test]
async fn l1_season_type_unknown_kind_falls_back_to_regular() {
    let state = WebState::new();
    {
        let mut cfg = state.config.write().await;
        *cfg = icelines_web::WebConfig::new("20252026", "playoff");
    }
    let captured = state.config.clone();

    let (status, _) = flip_season_type(state, "garbage-input", None).await;

    assert_eq!(status, StatusCode::SEE_OTHER);
    let cfg = captured.read().await;
    assert_eq!(cfg.active_season_type, "regular");
}

/// l1_season_type_redirect_honors_relative_referer
/// — when the user clicks the toggle while on /leaders, they should
///   land back on /leaders (not /). Relative referers pass through.
#[tokio::test]
async fn l1_season_type_redirect_honors_relative_referer() {
    let state = WebState::new();

    let (status, location) = flip_season_type(state, "playoff", Some("/leaders?sort=hits")).await;

    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location.as_deref(), Some("/leaders?sort=hits"));
}

/// l1_season_type_redirect_strips_localhost_origin
/// — same-origin absolute URLs (http://127.0.0.1:8000/leaders) are
///   common when browsers send the full Referer. The handler strips
///   the origin to keep the redirect relative — open-redirect
///   protection by construction.
#[tokio::test]
async fn l1_season_type_redirect_strips_localhost_origin() {
    let state = WebState::new();

    let (status, location) = flip_season_type(
        state,
        "playoff",
        Some("http://127.0.0.1:8000/player/8478402"),
    )
    .await;

    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location.as_deref(), Some("/player/8478402"));
}

/// l1_season_type_redirect_drops_off_site_referer
/// — open-redirect defense: a referer pointing somewhere external
///   (https://evil.example/x) MUST NOT become the Location target.
///   Falls through to "/" instead.
#[tokio::test]
async fn l1_season_type_redirect_drops_off_site_referer() {
    let state = WebState::new();

    let (status, location) =
        flip_season_type(state, "playoff", Some("https://evil.example/leaders")).await;

    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location.as_deref(), Some("/"));
}

/// l1_season_type_toggle_visible_in_global_nav
/// — render-time fence: the global-nav strip on every page MUST
///   show both options (Regular | Playoffs) with the active one
///   bolded. If the base.html toggle is ever removed by accident
///   this catches it.
#[tokio::test]
async fn l1_season_type_toggle_visible_in_global_nav() {
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");

    let body_bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("body");
    let body = std::str::from_utf8(&body_bytes).expect("utf-8");

    // Active option is bolded; inactive option is a link to the flip.
    assert!(
        body.contains("<strong>Regular</strong>"),
        "Default state has Regular active (bolded)"
    );
    assert!(
        body.contains("/season-type/playoff"),
        "Inactive option is a link to flip"
    );
    // Class hook for CSS — the toggle has its own class so a future
    // CSS refactor that drops the styling is detectable by other
    // means than a visual scan.
    assert!(body.contains("season-type-toggle"));
}

// ── Phase Foster.1 — date-anchored route smokes ────────────────────────────
//
// Network-free smokes: the handlers may fail to reach the NHL API in
// CI / offline test runs, but the page must still render (the
// fetch_error path lands in the template, not as a 500). What we
// pin here is "the route accepts ?date= / ?season= and returns
// 200 HTML". Future work can layer on httpmock-backed L1 fetches
// once we extract a NhlClient injection point.

#[tokio::test]
async fn l1_foster1_scores_accepts_past_date_query() {
    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/scores?date=2014-10-08")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "/scores must accept ?date= and render 200"
    );
}

#[tokio::test]
async fn l1_foster1_schedule_accepts_past_date_query() {
    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/schedule?date=2014-10-08")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "/schedule must accept ?date= (date-anchored slate path) and render 200"
    );
}

#[tokio::test]
async fn l1_foster1_playoffs_accepts_season_query() {
    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/playoffs?season=19931994")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "/playoffs must accept ?season= and render 200"
    );
}

// ── Phase Foster +9 — `?range=` smokes ──────────────────────────────────────

#[tokio::test]
async fn l1_foster_plus9_scores_accepts_range_week() {
    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/scores?date=2014-10-08&range=week")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "/scores must accept ?range=week"
    );
}

#[tokio::test]
async fn l1_foster_plus9_scores_accepts_range_month() {
    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/scores?date=2014-10-08&range=month")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn l1_foster_plus9_scores_accepts_range_day_default() {
    // Bare ?date= without ?range= should still 200 — `range=day` is
    // the implicit default per the spec convention.
    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/scores?date=2014-10-08")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");
    assert_eq!(response.status(), StatusCode::OK);
}

// ── Phase Conn Smythe C.3 — /game/:id smokes ────────────────────────────────

#[tokio::test]
async fn l1_conn_smythe_c3_game_route_accepts_id_and_returns_200() {
    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/game/2025020342")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");
    // Network is available or not — the handler renders an error
    // page in either case so the route always 200s.
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn l1_conn_smythe_c3_game_route_renders_html() {
    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/game/2025020342")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");
    let ct = response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .expect("content-type")
        .to_str()
        .unwrap();
    assert!(ct.starts_with("text/html"), "got: {ct}");
}

#[tokio::test]
async fn l1_foster_plus9_scores_unknown_range_defaults_to_day() {
    // Unknown range value should fall back to Day rather than 400.
    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/scores?date=2014-10-08&range=garbage")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "unknown range falls back to Day, must still 200"
    );
}
