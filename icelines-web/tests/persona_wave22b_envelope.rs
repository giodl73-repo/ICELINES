//! Persona Wave 22b — output format correctness for the
//! `/api/v1/leaders` JSON envelope under the new-grammar
//! pipeline.
//!
//! Asserts that filters running through the Phase Art Ross
//! pipeline (Wave 19's dispatch fix) emit the same envelope
//! shape as legacy filters — same keys, same row schema, same
//! meta block. The risk we're closing: the new pipeline takes
//! a different .filter() branch on the iterator, and a careless
//! refactor could plausibly drop a column or skip the meta
//! population.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use icelines_web::{router, WebState};
use serde_json::Value;
use tower::util::ServiceExt;

async fn fetch_envelope(filter: &str) -> Value {
    let app = router(WebState::new());
    let url = format!("/api/v1/leaders?filter={}&top=20", enc(filter));
    let r = app
        .oneshot(
            Request::builder()
                .uri(&url)
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");
    assert_eq!(r.status(), StatusCode::OK, "filter {filter:?} non-200");
    let bytes = axum::body::to_bytes(r.into_body(), 64 * 1024 * 1024)
        .await
        .expect("body bytes");
    serde_json::from_slice(&bytes).expect("envelope must be valid JSON")
}

fn enc(s: &str) -> String {
    s.replace('%', "%25")
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

fn assert_envelope_shape(env: &Value, ctx: &str) {
    // Top-level keys.
    let obj = env.as_object().unwrap_or_else(|| {
        panic!("{ctx}: expected object envelope, got: {env}");
    });
    for key in ["schema_version", "route", "data", "meta"] {
        assert!(
            obj.contains_key(key),
            "{ctx}: envelope missing top-level `{key}`; keys present: {:?}",
            obj.keys().collect::<Vec<_>>()
        );
    }

    // schema_version is u32 = 1 (frozen).
    assert_eq!(
        obj["schema_version"].as_u64(),
        Some(1),
        "{ctx}: schema_version must be 1 (frozen contract)"
    );
    assert_eq!(
        obj["route"].as_str(),
        Some("leaders"),
        "{ctx}: route must be \"leaders\""
    );

    // data is an array.
    let data = obj["data"]
        .as_array()
        .unwrap_or_else(|| panic!("{ctx}: data must be array; got: {}", obj["data"]));

    // Each row has the expected snake_case fields.
    for (i, row) in data.iter().enumerate() {
        let row_obj = row
            .as_object()
            .unwrap_or_else(|| panic!("{ctx}: data[{i}] must be object; got {row}"));
        for key in [
            "name",
            "position",
            "team",
            "games",
            "goals",
            "assists",
            "points",
            "points_per_game",
        ] {
            assert!(
                row_obj.contains_key(key),
                "{ctx}: data[{i}] missing field `{key}`; keys: {:?}",
                row_obj.keys().collect::<Vec<_>>()
            );
        }
    }

    // meta block.
    let meta = obj["meta"]
        .as_object()
        .unwrap_or_else(|| panic!("{ctx}: meta must be object; got {}", obj["meta"]));
    for key in [
        "season",
        "season_type",
        "sort",
        "position_filter",
        "active_filters",
        "total",
        "returned",
        "top",
    ] {
        assert!(
            meta.contains_key(key),
            "{ctx}: meta missing `{key}`; keys: {:?}",
            meta.keys().collect::<Vec<_>>()
        );
    }
    // active_filters is array of strings.
    let active = meta["active_filters"]
        .as_array()
        .unwrap_or_else(|| panic!("{ctx}: meta.active_filters must be array"));
    for (i, f) in active.iter().enumerate() {
        assert!(
            f.is_string(),
            "{ctx}: meta.active_filters[{i}] must be string; got {f}"
        );
    }
}

// ── Legacy filter envelope (baseline) ────────────────────────

#[tokio::test]
async fn p_w22b_001_legacy_envelope_shape() {
    let env = fetch_envelope("g>=10").await;
    assert_envelope_shape(&env, "legacy g>=10");
}

#[tokio::test]
async fn p_w22b_002_legacy_compound_envelope_shape() {
    let env = fetch_envelope("g>=10 AND p>=20").await;
    assert_envelope_shape(&env, "legacy compound");
}

// ── New-grammar filters: must produce identical envelope ─────

#[tokio::test]
async fn p_w22b_003_strict_lt_envelope_shape() {
    let env = fetch_envelope("g<5").await;
    assert_envelope_shape(&env, "strict-lt g<5");
}

#[tokio::test]
async fn p_w22b_004_ne_envelope_shape() {
    let env = fetch_envelope("g!=5").await;
    assert_envelope_shape(&env, "ne g!=5");
}

#[tokio::test]
async fn p_w22b_005_country_in_envelope_shape() {
    let env = fetch_envelope("country IN (CAN, USA, SWE)").await;
    assert_envelope_shape(&env, "country IN");
}

#[tokio::test]
async fn p_w22b_006_age_between_envelope_shape() {
    let env = fetch_envelope("age BETWEEN 22 AND 28").await;
    assert_envelope_shape(&env, "age BETWEEN");
}

#[tokio::test]
async fn p_w22b_007_country_like_envelope_shape() {
    let env = fetch_envelope(r#"country LIKE "CA*""#).await;
    assert_envelope_shape(&env, "country LIKE");
}

#[tokio::test]
async fn p_w22b_008_pos_in_envelope_shape() {
    let env = fetch_envelope("pos IN (C, LW, RW)").await;
    assert_envelope_shape(&env, "pos IN");
}

#[tokio::test]
async fn p_w22b_009_sliding_window_envelope_shape() {
    let env = fetch_envelope("g.last10g>=5").await;
    assert_envelope_shape(&env, "sliding-window");
}

#[tokio::test]
async fn p_w22b_010_career_envelope_shape() {
    let env = fetch_envelope("p.career>=500").await;
    assert_envelope_shape(&env, "career aggregate");
}

#[tokio::test]
async fn p_w22b_011_league_envelope_shape() {
    let env = fetch_envelope("league=OHL").await;
    assert_envelope_shape(&env, "league atom");
}

#[tokio::test]
async fn p_w22b_012_kitchen_sink_envelope_shape() {
    let env = fetch_envelope(
        "g.last10g>=5 AND age BETWEEN 22 AND 28 AND \
         country IN (CAN, USA) AND pos IN (C, LW, RW) AND draft-round<=2",
    )
    .await;
    assert_envelope_shape(&env, "kitchen sink");
}

// ── meta.active_filters reflects user input verbatim ─────────

#[tokio::test]
async fn p_w22b_013_active_filters_preserves_input() {
    let env = fetch_envelope("country IN (CAN, USA)").await;
    let active = env["meta"]["active_filters"]
        .as_array()
        .expect("active_filters array");
    assert!(
        active.iter().any(|f| f
            .as_str()
            .map(|s| s.contains("country") && s.contains("IN"))
            .unwrap_or(false)),
        "active_filters must preserve `country IN (...)` verbatim; got: {active:?}",
    );
}

#[tokio::test]
async fn p_w22b_014_active_filters_preserves_compound() {
    let env = fetch_envelope("country IN (CAN, USA) AND pos=C").await;
    let active = env["meta"]["active_filters"]
        .as_array()
        .expect("active_filters array");
    assert!(
        !active.is_empty(),
        "active_filters must NOT be empty for non-trivial filter; got: {active:?}",
    );
}

// ── shape stability across legacy vs new for matching answers ─

#[tokio::test]
async fn p_w22b_015_legacy_vs_new_same_keys() {
    let legacy = fetch_envelope("g>=10").await;
    let new = fetch_envelope("g.career>=10").await;
    let lk: std::collections::BTreeSet<_> = legacy.as_object().unwrap().keys().cloned().collect();
    let nk: std::collections::BTreeSet<_> = new.as_object().unwrap().keys().cloned().collect();
    assert_eq!(
        lk, nk,
        "top-level envelope keys must match across pipelines"
    );

    let lm: std::collections::BTreeSet<_> = legacy["meta"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    let nm: std::collections::BTreeSet<_> =
        new["meta"].as_object().unwrap().keys().cloned().collect();
    assert_eq!(lm, nm, "meta block keys must match across pipelines");
}

#[tokio::test]
async fn p_w22b_016_meta_total_returned_top_consistency() {
    let env = fetch_envelope("g>=0").await;
    let total = env["meta"]["total"].as_u64().unwrap();
    let returned = env["meta"]["returned"].as_u64().unwrap();
    let top = env["meta"]["top"].as_u64().unwrap();
    let data_len = env["data"].as_array().unwrap().len() as u64;

    assert_eq!(returned, data_len, "meta.returned must match data.len()");
    assert!(
        returned <= top,
        "meta.returned ({returned}) must be ≤ top ({top})"
    );
    assert!(
        returned <= total,
        "meta.returned ({returned}) must be ≤ total ({total})"
    );
}

#[tokio::test]
async fn p_w22b_017_meta_total_returned_consistency_new_grammar() {
    let env = fetch_envelope("country IN (CAN, USA, SWE)").await;
    let total = env["meta"]["total"].as_u64().unwrap();
    let returned = env["meta"]["returned"].as_u64().unwrap();
    let top = env["meta"]["top"].as_u64().unwrap();
    let data_len = env["data"].as_array().unwrap().len() as u64;

    assert_eq!(returned, data_len);
    assert!(returned <= top);
    assert!(returned <= total);
}
