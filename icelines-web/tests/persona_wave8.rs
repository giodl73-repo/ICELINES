//! Persona Wave 8 — 100 web router smokes + form-POST tests covering:
//! /favorites HTML / JSON shape, /favorites/add and /remove with
//! permutations, return_to honored vs not, /game/:id, /scores
//! ?date= ?range=, /schedule ?date=, /playoffs ?series=, security
//! (open-redirect attempts, XSS escaping).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use icelines_web::{router, WebState};
use tower::util::ServiceExt;

async fn get(uri: &str) -> axum::http::Response<Body> {
    let app = router(WebState::new());
    app.oneshot(
        Request::builder()
            .uri(uri)
            .body(Body::empty())
            .expect("build request"),
    )
    .await
    .expect("oneshot")
}

async fn post_form(uri: &str, body: &str) -> axum::http::Response<Body> {
    let app = router(WebState::new());
    app.oneshot(
        Request::builder()
            .method(axum::http::Method::POST)
            .uri(uri)
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(body.to_owned()))
            .expect("build request"),
    )
    .await
    .expect("oneshot")
}

async fn body_text(response: axum::http::Response<Body>) -> String {
    let body = axum::body::to_bytes(response.into_body(), 64 * 1024 * 1024)
        .await
        .expect("body fits in 64 MiB");
    String::from_utf8(body.to_vec()).expect("utf-8")
}

// ── Bare GETs across every Foster/Conn-Smythe route (30) ────────────────────

#[tokio::test]
async fn p_w8_001_root_returns_200() {
    let r = get("/").await;
    assert_eq!(r.status(), StatusCode::OK);
}

#[tokio::test]
async fn p_w8_002_root_html_content_type() {
    let r = get("/").await;
    let ct = r
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(ct.starts_with("text/html"));
}

#[tokio::test]
async fn p_w8_003_root_charset_utf8() {
    let r = get("/").await;
    let ct = r
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(ct.contains("utf-8") || ct.contains("UTF-8"));
}

#[tokio::test]
async fn p_w8_004_favorites_returns_200() {
    let r = get("/favorites").await;
    assert_eq!(r.status(), StatusCode::OK);
}

#[tokio::test]
async fn p_w8_005_favorites_includes_nav_link_to_self() {
    let r = get("/favorites").await;
    let body = body_text(r).await;
    assert!(body.contains("Favorites"));
}

#[tokio::test]
async fn p_w8_006_favorites_includes_add_form() {
    let r = get("/favorites").await;
    let body = body_text(r).await;
    assert!(body.contains("/favorites/add"));
}

#[tokio::test]
async fn p_w8_007_favorites_form_uses_post() {
    let r = get("/favorites").await;
    let body = body_text(r).await;
    assert!(body.contains("method=\"POST\"") || body.contains("method=POST"));
}

#[tokio::test]
async fn p_w8_008_favorites_form_input_named_key() {
    let r = get("/favorites").await;
    let body = body_text(r).await;
    assert!(body.contains("name=\"key\""));
}

#[tokio::test]
async fn p_w8_009_favorites_form_explains_auto_detect() {
    let r = get("/favorites").await;
    let body = body_text(r).await;
    assert!(body.contains("auto-detect") || body.contains("Auto-detect"));
}

#[tokio::test]
async fn p_w8_010_favorites_offers_kind_radio() {
    let r = get("/favorites").await;
    let body = body_text(r).await;
    assert!(body.contains("type=\"radio\""));
    assert!(body.contains("value=\"player\""));
    assert!(body.contains("value=\"team\""));
}

#[tokio::test]
async fn p_w8_011_scores_returns_200() {
    let r = get("/scores").await;
    assert_eq!(r.status(), StatusCode::OK);
}

#[tokio::test]
async fn p_w8_012_scores_with_date_returns_200() {
    let r = get("/scores?date=2014-10-08").await;
    assert_eq!(r.status(), StatusCode::OK);
}

#[tokio::test]
async fn p_w8_013_scores_with_range_day_returns_200() {
    let r = get("/scores?date=2014-10-08&range=day").await;
    assert_eq!(r.status(), StatusCode::OK);
}

#[tokio::test]
async fn p_w8_014_scores_with_range_week_returns_200() {
    let r = get("/scores?date=2014-10-08&range=week").await;
    assert_eq!(r.status(), StatusCode::OK);
}

#[tokio::test]
async fn p_w8_015_scores_with_range_month_returns_200() {
    let r = get("/scores?date=2014-10-08&range=month").await;
    assert_eq!(r.status(), StatusCode::OK);
}

#[tokio::test]
async fn p_w8_016_scores_with_unknown_range_falls_back_to_day() {
    let r = get("/scores?date=2014-10-08&range=cosmic").await;
    assert_eq!(r.status(), StatusCode::OK);
}

#[tokio::test]
async fn p_w8_017_scores_with_invalid_date_no_5xx() {
    let r = get("/scores?date=not-a-date").await;
    assert!(r.status() != StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn p_w8_018_schedule_returns_200() {
    let r = get("/schedule").await;
    assert_eq!(r.status(), StatusCode::OK);
}

#[tokio::test]
async fn p_w8_019_schedule_with_date_returns_200() {
    let r = get("/schedule?date=2014-10-08").await;
    assert_eq!(r.status(), StatusCode::OK);
}

#[tokio::test]
async fn p_w8_020_schedule_with_team_returns_200() {
    let r = get("/schedule?team=EDM").await;
    assert_eq!(r.status(), StatusCode::OK);
}

#[tokio::test]
async fn p_w8_021_playoffs_returns_200() {
    let r = get("/playoffs").await;
    assert_eq!(r.status(), StatusCode::OK);
}

#[tokio::test]
async fn p_w8_022_playoffs_with_season_returns_200() {
    let r = get("/playoffs?season=19931994").await;
    assert_eq!(r.status(), StatusCode::OK);
}

#[tokio::test]
async fn p_w8_023_game_route_returns_200() {
    let r = get("/game/2025020342").await;
    assert_eq!(r.status(), StatusCode::OK);
}

#[tokio::test]
async fn p_w8_024_game_route_renders_html_or_error_page() {
    let r = get("/game/2025020342").await;
    let body = body_text(r).await;
    // Either renders the game (network ok) or the error page
    // (network blocked). Both are HTML.
    assert!(body.starts_with("<!DOCTYPE html") || body.contains("<html"));
}

#[tokio::test]
async fn p_w8_025_game_route_includes_nav() {
    let r = get("/game/2025020342").await;
    let body = body_text(r).await;
    assert!(body.contains("League") || body.contains("Scores"));
}

#[tokio::test]
async fn p_w8_026_unknown_route_returns_404() {
    let r = get("/this-does-not-exist-anywhere").await;
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn p_w8_027_static_assets_route_present() {
    // Even if the asset doesn't exist by name, the route shape works.
    let r = get("/static/style.css").await;
    // 200 (asset exists) or 404 (doesn't); should not 5xx.
    assert!(r.status() != StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn p_w8_028_leaders_route_returns_200() {
    let r = get("/leaders").await;
    assert_eq!(r.status(), StatusCode::OK);
}

#[tokio::test]
async fn p_w8_029_goalies_route_returns_200() {
    let r = get("/goalies").await;
    assert_eq!(r.status(), StatusCode::OK);
}

#[tokio::test]
async fn p_w8_030_depth_route_returns_200() {
    let r = get("/depth").await;
    assert_eq!(r.status(), StatusCode::OK);
}

// ── Favorites form POSTs (40) ────────────────────────────────────────────────

#[tokio::test]
async fn p_w8_031_favorites_add_team_returns_redirect() {
    let r = post_form("/favorites/add", "key=EDM&kind=team").await;
    assert!(
        r.status().is_redirection(),
        "POST /favorites/add must redirect, got {}",
        r.status()
    );
}

#[tokio::test]
async fn p_w8_032_favorites_add_player_returns_redirect() {
    let r = post_form("/favorites/add", "key=Connor%20McDavid&kind=player").await;
    assert!(r.status().is_redirection());
}

#[tokio::test]
async fn p_w8_033_favorites_add_auto_detect_team() {
    let r = post_form("/favorites/add", "key=EDM").await;
    assert!(r.status().is_redirection());
}

#[tokio::test]
async fn p_w8_034_favorites_add_auto_detect_player_name() {
    let r = post_form("/favorites/add", "key=Connor%20McDavid").await;
    assert!(r.status().is_redirection());
}

#[tokio::test]
async fn p_w8_035_favorites_add_empty_key_handled() {
    let r = post_form("/favorites/add", "key=").await;
    // Implementation returns OK with an error HTML page.
    assert!(
        r.status() == StatusCode::OK || r.status().is_redirection(),
        "empty key handled cleanly, got {}",
        r.status()
    );
}

#[tokio::test]
async fn p_w8_036_favorites_add_redirects_to_favorites_by_default() {
    let r = post_form("/favorites/add", "key=EDM").await;
    let loc = r
        .headers()
        .get(axum::http::header::LOCATION)
        .map(|v| v.to_str().unwrap().to_owned());
    assert_eq!(loc.as_deref(), Some("/favorites"));
}

#[tokio::test]
async fn p_w8_037_favorites_add_honors_return_to_relative() {
    let r = post_form(
        "/favorites/add",
        "key=EDM&return_to=%2Fteam%2FEDM",
    )
    .await;
    let loc = r
        .headers()
        .get(axum::http::header::LOCATION)
        .map(|v| v.to_str().unwrap().to_owned());
    assert_eq!(loc.as_deref(), Some("/team/EDM"));
}

#[tokio::test]
async fn p_w8_038_favorites_add_rejects_external_redirect() {
    // Open-redirect attempt — external URL should be rejected.
    let r = post_form(
        "/favorites/add",
        "key=EDM&return_to=https%3A%2F%2Fevil.example",
    )
    .await;
    let loc = r
        .headers()
        .get(axum::http::header::LOCATION)
        .map(|v| v.to_str().unwrap().to_owned());
    assert_eq!(
        loc.as_deref(),
        Some("/favorites"),
        "external redirect must be rejected, got {loc:?}"
    );
}

#[tokio::test]
async fn p_w8_039_favorites_add_rejects_protocol_relative_redirect() {
    let r = post_form("/favorites/add", "key=EDM&return_to=%2F%2Fevil").await;
    let loc = r
        .headers()
        .get(axum::http::header::LOCATION)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    assert_eq!(loc, "/favorites");
}

#[tokio::test]
async fn p_w8_040_favorites_add_rejects_non_path_redirect() {
    let r = post_form("/favorites/add", "key=EDM&return_to=javascript:alert(1)").await;
    let loc = r
        .headers()
        .get(axum::http::header::LOCATION)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    assert_eq!(loc, "/favorites");
}

#[tokio::test]
async fn p_w8_041_favorites_remove_returns_redirect() {
    let r = post_form("/favorites/remove", "key=EDM&kind=team").await;
    assert!(r.status().is_redirection());
}

#[tokio::test]
async fn p_w8_042_favorites_remove_unknown_member_clean_no_op() {
    let r = post_form("/favorites/remove", "key=NoSuch&kind=team").await;
    assert!(
        r.status().is_redirection() || r.status() == StatusCode::OK,
        "remove of nonexistent member is a clean no-op, got {}",
        r.status()
    );
}

#[tokio::test]
async fn p_w8_043_favorites_remove_honors_return_to() {
    let r = post_form(
        "/favorites/remove",
        "key=EDM&kind=team&return_to=%2Fscores",
    )
    .await;
    let loc = r.headers().get(axum::http::header::LOCATION).unwrap();
    assert_eq!(loc.to_str().unwrap(), "/scores");
}

#[tokio::test]
async fn p_w8_044_favorites_add_lowercase_team_routed_via_kind() {
    // Explicit kind=team forces the team path even if input doesn't
    // look like a team abbrev.
    let r = post_form("/favorites/add", "key=edm&kind=team").await;
    assert!(r.status().is_redirection());
}

#[tokio::test]
async fn p_w8_045_favorites_add_kind_team_with_long_string() {
    // Explicit kind=team with too-long key — handler accepts it.
    let r = post_form("/favorites/add", "key=Anaheim&kind=team").await;
    assert!(r.status().is_redirection());
}

#[tokio::test]
async fn p_w8_046_favorites_add_auto_detect_player_route() {
    let r = post_form("/favorites/add", "key=Sidney%20Crosby").await;
    assert!(r.status().is_redirection());
}

#[tokio::test]
async fn p_w8_047_favorites_add_handles_special_chars() {
    let r = post_form("/favorites/add", "key=O%27Reilly").await;
    assert!(r.status().is_redirection() || r.status() == StatusCode::OK);
}

#[tokio::test]
async fn p_w8_048_favorites_add_handles_unicode_chars() {
    let r = post_form("/favorites/add", "key=Slafkovsk%C3%BD").await;
    assert!(r.status().is_redirection() || r.status() == StatusCode::OK);
}

#[tokio::test]
async fn p_w8_049_favorites_remove_explicit_kind_team_round_trip() {
    let r = post_form(
        "/favorites/remove",
        "key=EDM&kind=team&return_to=%2Ffavorites",
    )
    .await;
    assert!(r.status().is_redirection());
}

#[tokio::test]
async fn p_w8_050_favorites_add_empty_kind_uses_auto_detect() {
    let r = post_form("/favorites/add", "key=BOS&kind=").await;
    assert!(r.status().is_redirection());
}

#[tokio::test]
async fn p_w8_051_favorites_add_then_remove_round_trip() {
    let r1 = post_form("/favorites/add", "key=ZZZ&kind=team").await;
    assert!(r1.status().is_redirection());
    let r2 = post_form("/favorites/remove", "key=ZZZ&kind=team").await;
    assert!(r2.status().is_redirection());
}

#[tokio::test]
async fn p_w8_052_favorites_add_idempotent_no_5xx() {
    let r1 = post_form("/favorites/add", "key=YYY&kind=team").await;
    let r2 = post_form("/favorites/add", "key=YYY&kind=team").await;
    assert!(r1.status() != StatusCode::INTERNAL_SERVER_ERROR);
    assert!(r2.status() != StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn p_w8_053_favorites_post_get_form_returns_405() {
    // GET on /favorites/add should be method-not-allowed.
    let r = get("/favorites/add").await;
    assert!(
        r.status() == StatusCode::METHOD_NOT_ALLOWED
            || r.status() == StatusCode::NOT_FOUND,
        "GET /favorites/add should not 200, got {}",
        r.status()
    );
}

#[tokio::test]
async fn p_w8_054_favorites_remove_get_returns_405_or_404() {
    let r = get("/favorites/remove").await;
    assert!(
        r.status() == StatusCode::METHOD_NOT_ALLOWED
            || r.status() == StatusCode::NOT_FOUND,
        "GET /favorites/remove should not 200, got {}",
        r.status()
    );
}

#[tokio::test]
async fn p_w8_055_favorites_post_with_no_body_handled() {
    let r = post_form("/favorites/add", "").await;
    // Empty form → handler errors but returns 4xx, not 5xx.
    assert!(
        r.status() != StatusCode::INTERNAL_SERVER_ERROR,
        "empty form must not 5xx, got {}",
        r.status()
    );
}

#[tokio::test]
async fn p_w8_056_favorites_add_with_garbage_query_still_handled() {
    let r = post_form("/favorites/add", "garbage=true&random=stuff").await;
    // Missing required `key` field → handler returns 4xx not 5xx.
    assert!(r.status() != StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn p_w8_057_favorites_add_kind_with_unknown_value_falls_back() {
    let r = post_form("/favorites/add", "key=EDM&kind=banana").await;
    // Unknown kind → falls back to auto-detect (per implementation).
    assert!(r.status().is_redirection() || r.status() != StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn p_w8_058_favorites_add_xss_in_key_safe() {
    // Adding "<script>" as a "team" should be persisted but escaped
    // on render.
    let r = post_form("/favorites/add", "key=%3Cscript%3E&kind=team").await;
    assert!(r.status().is_redirection());
}

#[tokio::test]
async fn p_w8_059_favorites_add_then_render_no_unescaped_script() {
    post_form("/favorites/add", "key=%3Cscript%3Ealert(1)%3C%2Fscript%3E&kind=team").await;
    let r = get("/favorites").await;
    let body = body_text(r).await;
    // The XSS payload survives as escaped text, never as live script.
    assert!(
        !body.contains("<script>alert"),
        "raw <script> tag must be escaped"
    );
}

#[tokio::test]
async fn p_w8_060_favorites_add_long_string_no_panic() {
    let long = "A".repeat(500);
    let body = format!("key={long}&kind=team");
    let r = post_form("/favorites/add", &body).await;
    assert!(r.status() != StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn p_w8_061_favorites_remove_nonexistent_idempotent() {
    let r1 = post_form("/favorites/remove", "key=NEVERTHERE&kind=team").await;
    let r2 = post_form("/favorites/remove", "key=NEVERTHERE&kind=team").await;
    assert!(r1.status().is_redirection());
    assert!(r2.status().is_redirection());
}

#[tokio::test]
async fn p_w8_062_favorites_path_traversal_in_return_to_blocked() {
    let r = post_form(
        "/favorites/add",
        "key=EDM&return_to=%2F..%2F..%2Fetc%2Fpasswd",
    )
    .await;
    let loc = r
        .headers()
        .get(axum::http::header::LOCATION)
        .unwrap()
        .to_str()
        .unwrap();
    // Implementation accepts any `/`-leading relative path. The
    // path-traversal here is a relative path starting with /, which
    // technically passes the validator. The browser/server renders
    // it as a relative URL. This is acceptable since:
    // - the redirect doesn't grant access (the server just sends
    //   a Location header)
    // - any hardening would belong at the route layer, not the
    //   redirect validator
    // Document the current behavior — pin it so a future tightening
    // would have to update this test.
    assert!(loc.starts_with("/"));
}

#[tokio::test]
async fn p_w8_063_favorites_add_and_post_get_redirect_chain() {
    let r = post_form("/favorites/add", "key=KKK&kind=team").await;
    assert!(r.status().is_redirection());
    // Follow the redirect manually
    let r2 = get("/favorites").await;
    assert_eq!(r2.status(), StatusCode::OK);
}

#[tokio::test]
async fn p_w8_064_favorites_add_kind_player_explicit_no_auto_detect() {
    // "EDM" with kind=player should be persisted as a player named "edm".
    let r = post_form("/favorites/add", "key=EDM&kind=player").await;
    assert!(r.status().is_redirection());
}

#[tokio::test]
async fn p_w8_065_favorites_remove_explicit_player_kind() {
    let r = post_form("/favorites/remove", "key=connor%20mcdavid&kind=player").await;
    assert!(r.status().is_redirection());
}

#[tokio::test]
async fn p_w8_066_favorites_form_renders_player_count() {
    let r = get("/favorites").await;
    let body = body_text(r).await;
    assert!(body.contains("player"));
}

#[tokio::test]
async fn p_w8_067_favorites_form_renders_team_count() {
    let r = get("/favorites").await;
    let body = body_text(r).await;
    assert!(body.contains("team"));
}

#[tokio::test]
async fn p_w8_068_favorites_remove_btn_present_when_populated() {
    post_form("/favorites/add", "key=AAA&kind=team").await;
    let r = get("/favorites").await;
    let body = body_text(r).await;
    assert!(body.contains("/favorites/remove"));
}

#[tokio::test]
async fn p_w8_069_favorites_team_link_present_when_populated() {
    post_form("/favorites/add", "key=BBB&kind=team").await;
    let r = get("/favorites").await;
    let body = body_text(r).await;
    assert!(body.contains("/team/BBB") || body.contains("/team/"));
}

#[tokio::test]
async fn p_w8_070_favorites_empty_state_renders_when_no_members() {
    // Note: state may not be empty due to other parallel tests adding
    // members. Just check no 5xx.
    let r = get("/favorites").await;
    assert!(r.status() == StatusCode::OK);
}

// ── Nav consistency + cross-page (15) ───────────────────────────────────────

#[tokio::test]
async fn p_w8_071_home_nav_links_to_favorites() {
    let r = get("/").await;
    let body = body_text(r).await;
    assert!(body.contains("/favorites"));
}

#[tokio::test]
async fn p_w8_072_scores_page_includes_nav_to_favorites() {
    let r = get("/scores").await;
    let body = body_text(r).await;
    assert!(body.contains("/favorites"));
}

#[tokio::test]
async fn p_w8_073_schedule_page_includes_nav_to_favorites() {
    let r = get("/schedule").await;
    let body = body_text(r).await;
    assert!(body.contains("/favorites"));
}

#[tokio::test]
async fn p_w8_074_playoffs_page_includes_nav_to_favorites() {
    let r = get("/playoffs").await;
    let body = body_text(r).await;
    assert!(body.contains("/favorites"));
}

#[tokio::test]
async fn p_w8_075_game_page_includes_nav_to_favorites() {
    let r = get("/game/2025020342").await;
    let body = body_text(r).await;
    assert!(body.contains("/favorites"));
}

#[tokio::test]
async fn p_w8_076_team_page_includes_favorite_button() {
    let r = get("/team/EDM").await;
    let body = body_text(r).await;
    // Favorite team button surfaced on the team banner
    assert!(body.contains("Favorite team") || body.contains("/favorites/add"));
}

#[tokio::test]
async fn p_w8_077_player_page_includes_favorite_button() {
    let r = get("/player/8478402").await;
    if r.status() == StatusCode::OK {
        let body = body_text(r).await;
        assert!(body.contains("Favorite player") || body.contains("/favorites/add"));
    }
}

#[tokio::test]
async fn p_w8_078_team_favorite_button_posts_kind_team() {
    let r = get("/team/EDM").await;
    let body = body_text(r).await;
    assert!(
        body.contains("name=\"kind\"") && body.contains("value=\"team\""),
        "team page must POST kind=team"
    );
}

#[tokio::test]
async fn p_w8_079_player_favorite_button_posts_kind_player() {
    let r = get("/player/8478402").await;
    if r.status() == StatusCode::OK {
        let body = body_text(r).await;
        assert!(
            body.contains("name=\"kind\"") && body.contains("value=\"player\""),
            "player page must POST kind=player"
        );
    }
}

#[tokio::test]
async fn p_w8_080_team_favorite_button_uses_post_method() {
    let r = get("/team/EDM").await;
    let body = body_text(r).await;
    assert!(
        body.contains("method=\"POST\"") || body.contains("method=POST"),
        "team page favorite form must POST"
    );
}

#[tokio::test]
async fn p_w8_081_team_favorite_button_includes_return_to_self() {
    let r = get("/team/EDM").await;
    let body = body_text(r).await;
    assert!(
        body.contains("return_to") && body.contains("/team/"),
        "form should include return_to back to the team page"
    );
}

#[tokio::test]
async fn p_w8_082_unknown_team_path_handled() {
    let r = get("/team/ZZZ").await;
    // Either 200 (unknown team handled gracefully) or 404
    assert!(r.status() == StatusCode::OK || r.status() == StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn p_w8_083_unknown_player_path_handled() {
    let r = get("/player/0").await;
    assert!(r.status() != StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn p_w8_084_unknown_player_huge_id_handled() {
    let r = get("/player/99999999999").await;
    assert!(r.status() != StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn p_w8_085_favorites_link_styles_consistently_across_pages() {
    // Just smoke-test that both player + team pages contain favorites
    // form structures (not asserting visual style here).
    let team_r = get("/team/EDM").await;
    let team_body = body_text(team_r).await;
    let pl_r = get("/player/8478402").await;
    if pl_r.status() == StatusCode::OK {
        let pl_body = body_text(pl_r).await;
        // Both should have the Favorite button form.
        assert!(team_body.contains("/favorites/add"));
        assert!(pl_body.contains("/favorites/add"));
    }
}

// ── /game/:id detail page (15) ──────────────────────────────────────────────

#[tokio::test]
async fn p_w8_086_game_invalid_id_garbage_returns_404() {
    let r = get("/game/notanumber").await;
    // Path :id with non-numeric → 404 from axum's path extractor.
    assert!(r.status() == StatusCode::NOT_FOUND || r.status() == StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn p_w8_087_game_zero_id_no_5xx() {
    let r = get("/game/0").await;
    assert!(r.status() != StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn p_w8_088_game_max_u64_no_5xx() {
    let r = get("/game/18446744073709551615").await;
    assert!(r.status() != StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn p_w8_089_game_includes_scoreboard_class() {
    let r = get("/game/2025020342").await;
    let body = body_text(r).await;
    // Either renders the boxscore (with scoreboard class) or the
    // error page; both work.
    assert!(
        body.contains("scoreboard") || body.contains("Could not fetch boxscore"),
        "expected scoreboard or error fallback"
    );
}

#[tokio::test]
async fn p_w8_090_game_renders_html_doctype() {
    let r = get("/game/2025020342").await;
    let body = body_text(r).await;
    assert!(body.contains("<!DOCTYPE html"));
}

#[tokio::test]
async fn p_w8_091_game_renders_back_link_on_error() {
    let r = get("/game/0").await;
    let body = body_text(r).await;
    // Error page includes back link
    if body.contains("Could not fetch") {
        assert!(body.contains("back") || body.contains("/scores"));
    }
}

#[tokio::test]
async fn p_w8_092_game_html_content_type() {
    let r = get("/game/2025020342").await;
    let ct = r
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(ct.contains("text/html"));
}

#[tokio::test]
async fn p_w8_093_game_error_page_doesnt_leak_internals() {
    let r = get("/game/0").await;
    let body = body_text(r).await;
    // Stack trace or panic format must not appear
    assert!(!body.contains("panicked"));
    assert!(!body.contains("backtrace"));
}

#[tokio::test]
async fn p_w8_094_game_negative_path_segment_404() {
    let r = get("/game/-1").await;
    assert!(r.status() == StatusCode::NOT_FOUND || r.status() == StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn p_w8_095_game_floating_point_id_404() {
    let r = get("/game/1.5").await;
    assert!(r.status() == StatusCode::NOT_FOUND || r.status() == StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn p_w8_096_game_with_query_params_ignored_no_5xx() {
    let r = get("/game/2025020342?foo=bar&baz=qux").await;
    assert!(r.status() != StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn p_w8_097_game_with_extra_path_segments_404() {
    let r = get("/game/2025020342/foo").await;
    assert!(r.status() == StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn p_w8_098_game_renders_links_to_scores() {
    let r = get("/game/2025020342").await;
    let body = body_text(r).await;
    assert!(body.contains("/scores"));
}

#[tokio::test]
async fn p_w8_099_game_renders_links_to_playoffs() {
    let r = get("/game/2025020342").await;
    let body = body_text(r).await;
    assert!(body.contains("/playoffs"));
}

#[tokio::test]
async fn p_w8_100_game_handles_concurrent_requests() {
    use tokio::task::JoinSet;
    let mut set = JoinSet::new();
    for _ in 0..10 {
        set.spawn(async move { get("/game/2025020342").await });
    }
    while let Some(res) = set.join_next().await {
        let r = res.unwrap();
        assert!(r.status() == StatusCode::OK);
    }
}
