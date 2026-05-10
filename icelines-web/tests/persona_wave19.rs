//! Persona Wave 19 — `/api/v1/leaders` JSON endpoint new-grammar
//! parity. Wave 17 covered the HTML `/leaders` route; the JSON
//! API at `/api/v1/leaders` uses a different shared data path
//! (`build_leader_result`) which had its own legacy-only
//! dispatch. Wave 19 surfaces + fixes that.

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

fn enc(filter: &str) -> String {
    filter
        .replace('%', "%25")
        .replace(' ', "%20")
        .replace('"', "%22")
        .replace('<', "%3C")
        .replace('>', "%3E")
        .replace(',', "%2C")
        .replace('(', "%28")
        .replace(')', "%29")
        .replace('!', "%21")
        .replace('&', "%26")
        .replace('+', "%2B")
}

async fn assert_api_filter_accepted(filter: &str) {
    let url = format!("/api/v1/leaders?filter={}", enc(filter));
    let r = get(&url).await;
    let status = r.status();
    if status != StatusCode::OK {
        let body = axum::body::to_bytes(r.into_body(), 64 * 1024)
            .await
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .unwrap_or_default();
        panic!("filter {filter:?} returned {status}; body: {body}");
    }
}

async fn assert_api_filter_rejected(filter: &str) {
    let url = format!("/api/v1/leaders?filter={}", enc(filter));
    let r = get(&url).await;
    let status = r.status();
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "filter {filter:?} should be rejected"
    );
    let content_type = r
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    assert!(
        content_type.starts_with("application/json"),
        "rejected API filter should return JSON, got {content_type:?}"
    );
    let body = axum::body::to_bytes(r.into_body(), 64 * 1024)
        .await
        .expect("body fits");
    let json: serde_json::Value =
        serde_json::from_slice(&body).expect("rejected API filter should be valid JSON");
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["route"], "leaders");
    assert!(json["data"].as_array().is_some_and(Vec::is_empty));
    assert_eq!(json["meta"]["returned"], 0);
    assert!(json["error"].is_string());
}

async fn api_json(uri: &str) -> serde_json::Value {
    let r = get(uri).await;
    assert_eq!(r.status(), StatusCode::OK, "{uri} should return 200");
    let body = axum::body::to_bytes(r.into_body(), 64 * 1024 * 1024)
        .await
        .expect("body fits");
    serde_json::from_slice(&body).expect("API response should be valid JSON")
}

#[tokio::test]
async fn p_w19_001_strict_lt_via_api() {
    assert_api_filter_accepted("g<5").await;
}

#[tokio::test]
async fn p_w19_002_strict_gt_via_api() {
    assert_api_filter_accepted("g>5").await;
}

#[tokio::test]
async fn p_w19_003_ne_via_api() {
    assert_api_filter_accepted("g!=5").await;
}

#[tokio::test]
async fn p_w19_004_age_strict_under_25_via_api() {
    assert_api_filter_accepted("age<25").await;
}

#[tokio::test]
async fn p_w19_005_country_in_set_via_api() {
    assert_api_filter_accepted("country IN (CAN, USA, SWE)").await;
}

#[tokio::test]
async fn p_w19_006_country_not_in_via_api() {
    assert_api_filter_accepted("country NOT IN (RUS)").await;
}

#[tokio::test]
async fn p_w19_007_pos_in_set_via_api() {
    assert_api_filter_accepted("pos IN (C, LW, RW)").await;
}

#[tokio::test]
async fn p_w19_008_age_between_via_api() {
    assert_api_filter_accepted("age BETWEEN 22 AND 28").await;
}

#[tokio::test]
async fn p_w19_009_country_like_via_api() {
    assert_api_filter_accepted(r#"country LIKE "CA*""#).await;
}

#[tokio::test]
async fn p_w19_010_country_not_like_via_api() {
    assert_api_filter_accepted(r#"country NOT LIKE "RU*""#).await;
}

#[tokio::test]
async fn p_w19_011_sliding_window_via_api() {
    assert_api_filter_accepted("g.last10g>=5").await;
}

#[tokio::test]
async fn p_w19_012_career_atom_via_api() {
    assert_api_filter_accepted("p.career>=500").await;
}

#[tokio::test]
async fn p_w19_013_ever_via_api() {
    assert_api_filter_accepted("g.any10g>=5 EVER").await;
}

#[tokio::test]
async fn p_w19_014_ever_at_age_via_api() {
    assert_api_filter_accepted("g.any10g>=5 EVER AT age<=25").await;
}

#[tokio::test]
async fn p_w19_015_league_via_api() {
    assert_api_filter_accepted("league=OHL").await;
}

#[tokio::test]
async fn p_w19_016_league_in_set_via_api() {
    assert_api_filter_accepted("league IN (OHL, WHL, QMJHL)").await;
}

#[tokio::test]
async fn p_w19_017_career_junior_via_api() {
    assert_api_filter_accepted("p.career.junior>=200").await;
}

#[tokio::test]
async fn p_w19_018_compound_kitchen_sink_via_api() {
    assert_api_filter_accepted(
        "g.last10g>=5 AND age BETWEEN 22 AND 28 AND \
         country IN (CAN, USA) AND pos IN (C, LW, RW) AND draft-round<=2",
    )
    .await;
}

#[tokio::test]
async fn p_w19_019_empty_in_rejected_via_api() {
    assert_api_filter_rejected("country IN ()").await;
}

#[tokio::test]
async fn p_w19_020_stat_in_rejected_via_api() {
    assert_api_filter_rejected("g IN (10, 20, 30)").await;
}

#[tokio::test]
async fn p_w19_021_like_on_numeric_rejected_via_api() {
    assert_api_filter_rejected(r#"g LIKE "5*""#).await;
}

#[tokio::test]
async fn p_w19_022_team_career_rejected_via_api() {
    assert_api_filter_rejected("team.career=EDM").await;
}

#[tokio::test]
async fn p_w19_023_demorgan_via_api() {
    assert_api_filter_accepted("NOT (country=CAN AND pos=C)").await;
}

#[tokio::test]
async fn p_w19_024_response_returns_valid_json() {
    let url = format!("/api/v1/leaders?filter={}", enc("country=CAN"));
    let r = get(&url).await;
    assert_eq!(r.status(), StatusCode::OK);
    let body = axum::body::to_bytes(r.into_body(), 64 * 1024 * 1024)
        .await
        .unwrap();
    let _: serde_json::Value =
        serde_json::from_slice(&body).expect("API response must be valid JSON");
}

#[tokio::test]
async fn p_w19_025_killer_query_via_api() {
    // The user's full vision query through the JSON API.
    assert_api_filter_accepted("g.any10g>=5 EVER AT age<=25 AND country IN (CAN, USA, SWE)").await;
}

#[tokio::test]
async fn p_w19_026_discrete_country_filter_via_api() {
    let json = api_json("/api/v1/leaders?country=ZZZ&top=500").await;
    assert_eq!(json["route"], "leaders");
    assert_eq!(json["meta"]["total"], 0);
    assert_eq!(json["meta"]["returned"], 0);
    assert!(json["data"].as_array().is_some_and(Vec::is_empty));
}

#[tokio::test]
async fn p_w19_027_discrete_age_filter_via_api() {
    let json = api_json("/api/v1/leaders?age-max=1&top=500").await;
    assert_eq!(json["route"], "leaders");
    assert_eq!(json["meta"]["total"], 0);
    assert_eq!(json["meta"]["returned"], 0);
    assert!(json["data"].as_array().is_some_and(Vec::is_empty));
}
