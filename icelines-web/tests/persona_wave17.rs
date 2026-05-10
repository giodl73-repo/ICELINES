//! Persona Wave 17 — web new-grammar parity tests.
//!
//! Wave 16 caught the CLI dispatch bug where everything except
//! `needs_provider` plans fell through to the legacy parser.
//! Web lowers repeated `filter=` params through the shared query URL boundary.
//! Wave 17 verifies the web surface accepts the new grammar via
//! `/leaders?filter=`.
//!
//! Pre-fix: many of these tests should fail with HTTP 400
//! "Bad filter" because the legacy parser doesn't understand
//! the new operators. Post-fix: all should return 200.

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

async fn body_text(response: axum::http::Response<Body>) -> String {
    let body = axum::body::to_bytes(response.into_body(), 64 * 1024 * 1024)
        .await
        .expect("body fits in 64 MiB");
    String::from_utf8(body.to_vec()).expect("utf-8")
}

/// Helper — URL-encode a filter expression for `/leaders?filter=`.
fn enc(filter: &str) -> String {
    // Minimal urlencoding for the chars that show up in our
    // filter grammar.
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

async fn assert_filter_accepted(filter: &str) {
    let url = format!("/leaders?filter={}", enc(filter));
    let r = get(&url).await;
    let status = r.status();
    if status != StatusCode::OK {
        let body = body_text(r).await;
        panic!(
            "filter {filter:?} returned {status}; body:\n{}",
            &body[..body.len().min(500)]
        );
    }
}

async fn assert_filter_rejected(filter: &str) {
    let url = format!("/leaders?filter={}", enc(filter));
    let r = get(&url).await;
    assert_eq!(
        r.status(),
        StatusCode::BAD_REQUEST,
        "filter {filter:?} should be rejected as 400"
    );
}

// ── Strict comparators (10) ──────────────────────────────────

#[tokio::test]
async fn p_w17_001_strict_lt() {
    assert_filter_accepted("g<5").await;
}

#[tokio::test]
async fn p_w17_002_strict_gt() {
    assert_filter_accepted("g>5").await;
}

#[tokio::test]
async fn p_w17_003_ne() {
    assert_filter_accepted("g!=5").await;
}

#[tokio::test]
async fn p_w17_004_age_under_25_strict() {
    assert_filter_accepted("age<25").await;
}

#[tokio::test]
async fn p_w17_005_compound_strict() {
    assert_filter_accepted("g<10 AND a<10").await;
}

#[tokio::test]
async fn p_w17_006_strict_or() {
    assert_filter_accepted("g>50 OR a>50").await;
}

#[tokio::test]
async fn p_w17_007_strict_decimal() {
    assert_filter_accepted("ppg<1.5").await;
}

#[tokio::test]
async fn p_w17_008_age_strict_gt() {
    assert_filter_accepted("age>30").await;
}

#[tokio::test]
async fn p_w17_009_ne_decimal() {
    assert_filter_accepted("ppg!=1.5").await;
}

#[tokio::test]
async fn p_w17_010_strict_compound_with_bio() {
    assert_filter_accepted("country=CAN AND age<25").await;
}

// ── IN / NOT IN (10) ─────────────────────────────────────────

#[tokio::test]
async fn p_w17_011_country_in_set() {
    assert_filter_accepted("country IN (CAN, USA, SWE)").await;
}

#[tokio::test]
async fn p_w17_012_country_not_in() {
    assert_filter_accepted("country NOT IN (RUS)").await;
}

#[tokio::test]
async fn p_w17_013_pos_in_set() {
    assert_filter_accepted("pos IN (C, LW, RW)").await;
}

#[tokio::test]
async fn p_w17_014_team_in_set() {
    assert_filter_accepted("team IN (BOS, NYR)").await;
}

#[tokio::test]
async fn p_w17_015_empty_in_rejected() {
    assert_filter_rejected("country IN ()").await;
}

#[tokio::test]
async fn p_w17_016_in_with_quoted_strings() {
    assert_filter_accepted(r#"country IN ("CAN", "USA")"#).await;
}

#[tokio::test]
async fn p_w17_017_in_numeric_draft_year() {
    assert_filter_accepted("draft-year IN (2020, 2021, 2022)").await;
}

#[tokio::test]
async fn p_w17_018_in_compound() {
    assert_filter_accepted("country IN (CAN, USA) AND p>=20").await;
}

#[tokio::test]
async fn p_w17_019_in_lowercase_keyword() {
    assert_filter_accepted("country in (CAN)").await;
}

#[tokio::test]
async fn p_w17_020_stat_in_rejected() {
    // g IN (10, 20, 30) — should reject (use BETWEEN for ranges).
    assert_filter_rejected("g IN (10, 20, 30)").await;
}

// ── BETWEEN (10) ─────────────────────────────────────────────

#[tokio::test]
async fn p_w17_021_age_between() {
    assert_filter_accepted("age BETWEEN 22 AND 28").await;
}

#[tokio::test]
async fn p_w17_022_g_between() {
    assert_filter_accepted("g BETWEEN 20 AND 40").await;
}

#[tokio::test]
async fn p_w17_023_ppg_between_decimals() {
    assert_filter_accepted("ppg BETWEEN 0.5 AND 1.5").await;
}

#[tokio::test]
async fn p_w17_024_between_inverted_no_match() {
    // Inverted bounds — should not crash, just match nobody.
    assert_filter_accepted("g BETWEEN 40 AND 20").await;
}

#[tokio::test]
async fn p_w17_025_between_with_country() {
    assert_filter_accepted("age BETWEEN 22 AND 28 AND country=CAN").await;
}

#[tokio::test]
async fn p_w17_026_between_in_or() {
    assert_filter_accepted("g BETWEEN 30 AND 50 OR a BETWEEN 30 AND 50").await;
}

#[tokio::test]
async fn p_w17_027_between_under_not() {
    assert_filter_accepted("NOT (g BETWEEN 0 AND 5)").await;
}

#[tokio::test]
async fn p_w17_028_draft_round_between() {
    assert_filter_accepted("draft-round BETWEEN 1 AND 3").await;
}

#[tokio::test]
async fn p_w17_029_between_lowercase() {
    assert_filter_accepted("g between 20 and 40").await;
}

#[tokio::test]
async fn p_w17_030_between_paren_grouped() {
    assert_filter_accepted("(g BETWEEN 20 AND 40) AND age<=30").await;
}

// ── LIKE / NOT LIKE (10) ─────────────────────────────────────

#[tokio::test]
async fn p_w17_031_country_like_quoted() {
    assert_filter_accepted(r#"country LIKE "CA*""#).await;
}

#[tokio::test]
async fn p_w17_032_country_like_unquoted() {
    assert_filter_accepted("country LIKE CA*").await;
}

#[tokio::test]
async fn p_w17_033_country_not_like() {
    assert_filter_accepted(r#"country NOT LIKE "RU*""#).await;
}

#[tokio::test]
async fn p_w17_034_like_on_numeric_rejected() {
    assert_filter_rejected(r#"g LIKE "5*""#).await;
}

#[tokio::test]
async fn p_w17_035_like_with_paren() {
    assert_filter_accepted(r#"(country LIKE "CA*") AND age<=24"#).await;
}

#[tokio::test]
async fn p_w17_036_pos_like_pattern() {
    assert_filter_accepted(r#"pos LIKE "*W""#).await;
}

#[tokio::test]
async fn p_w17_037_like_in_or() {
    assert_filter_accepted(r#"country LIKE "CA*" OR country LIKE "US*""#).await;
}

#[tokio::test]
async fn p_w17_038_like_just_wildcard() {
    assert_filter_accepted(r#"country LIKE "*""#).await;
}

#[tokio::test]
async fn p_w17_039_like_lowercase_keyword() {
    assert_filter_accepted(r#"country like "CA*""#).await;
}

#[tokio::test]
async fn p_w17_040_substring_op_not_yet_wired() {
    // The `~` substring operator is documented in the spec
    // but not yet wired in the scalar-atom parser. Use LIKE
    // for now (`country LIKE "*AN*"`).
    assert_filter_rejected("country ~ AN").await;
}

// ── Sliding-window atoms (10) ────────────────────────────────

#[tokio::test]
async fn p_w17_041_last10g() {
    assert_filter_accepted("g.last10g>=5").await;
}

#[tokio::test]
async fn p_w17_042_last30d() {
    assert_filter_accepted("g.last30d>=10").await;
}

#[tokio::test]
async fn p_w17_043_last3w() {
    assert_filter_accepted("p.last3w>=5").await;
}

#[tokio::test]
async fn p_w17_044_last3m() {
    assert_filter_accepted("p.last3m>=15").await;
}

#[tokio::test]
async fn p_w17_045_last10g_allteams() {
    assert_filter_accepted("g.last10g.allteams>=5").await;
}

#[tokio::test]
async fn p_w17_046_last10g_career() {
    assert_filter_accepted("g.last10g.career>=5").await;
}

#[tokio::test]
async fn p_w17_047_last10z_rejected() {
    assert_filter_rejected("g.last10z>=5").await;
}

#[tokio::test]
async fn p_w17_048_last0g_rejected() {
    assert_filter_rejected("g.last0g>=5").await;
}

#[tokio::test]
async fn p_w17_049_killer_query_streak_age() {
    assert_filter_accepted("g.last10g>=5 AND age<=25").await;
}

#[tokio::test]
async fn p_w17_050_window_with_country_in() {
    assert_filter_accepted("g.last10g>=5 AND country IN (CAN, USA)").await;
}

// ── Career atoms (10) ────────────────────────────────────────

#[tokio::test]
async fn p_w17_051_career_lifetime_sum() {
    assert_filter_accepted("p.career>=500").await;
}

#[tokio::test]
async fn p_w17_052_career_streak() {
    assert_filter_accepted("p.streak>=15").await;
}

#[tokio::test]
async fn p_w17_053_seasons_with() {
    assert_filter_accepted("g.seasons-with>=5").await;
}

#[tokio::test]
async fn p_w17_054_any10g_ever() {
    assert_filter_accepted("g.any10g>=5 EVER").await;
}

#[tokio::test]
async fn p_w17_055_any10g_ever_at_age() {
    assert_filter_accepted("g.any10g>=5 EVER AT age<=25").await;
}

#[tokio::test]
async fn p_w17_056_career_with_at_age_range() {
    assert_filter_accepted("p.career>=300 AT age BETWEEN 22 AND 28").await;
}

#[tokio::test]
async fn p_w17_057_ever_on_non_career_rejected() {
    assert_filter_rejected("g>=5 EVER").await;
}

#[tokio::test]
async fn p_w17_058_at_clause_non_age_rejected() {
    assert_filter_rejected("p.career>=500 AT country=CAN").await;
}

#[tokio::test]
async fn p_w17_059_career_compound_with_bio() {
    assert_filter_accepted("p.career>=300 AND country=CAN").await;
}

#[tokio::test]
async fn p_w17_060_career_with_pos_in() {
    assert_filter_accepted("p.career>=500 AND pos IN (C, LW, RW)").await;
}

// ── League atoms (10) ────────────────────────────────────────

#[tokio::test]
async fn p_w17_061_league_eq() {
    assert_filter_accepted("league=OHL").await;
}

#[tokio::test]
async fn p_w17_062_league_in_set() {
    assert_filter_accepted("league IN (OHL, WHL, QMJHL)").await;
}

#[tokio::test]
async fn p_w17_063_league_not_in() {
    assert_filter_accepted("league NOT IN (NHL)").await;
}

#[tokio::test]
async fn p_w17_064_league_tier_junior() {
    assert_filter_accepted("league.tier=Junior").await;
}

#[tokio::test]
async fn p_w17_065_league_tier_pro() {
    assert_filter_accepted("league.tier=Pro").await;
}

#[tokio::test]
async fn p_w17_066_league_tier_unknown_rejected() {
    assert_filter_rejected("league.tier=Bogus").await;
}

#[tokio::test]
async fn p_w17_067_career_junior() {
    assert_filter_accepted("p.career.junior>=200").await;
}

#[tokio::test]
async fn p_w17_068_career_nhl_only() {
    assert_filter_accepted("p.career.nhl>=500").await;
}

#[tokio::test]
async fn p_w17_069_career_specific_league() {
    assert_filter_accepted("p.career.ohl>=300").await;
}

#[tokio::test]
async fn p_w17_070_team_career_rejected() {
    assert_filter_rejected("team.career=EDM").await;
}

// ── Compound + edge cases (10) ───────────────────────────────

#[tokio::test]
async fn p_w17_071_kitchen_sink() {
    assert_filter_accepted(
        "g.last10g>=5 AND age BETWEEN 22 AND 28 AND \
         country IN (CAN, USA) AND pos IN (C, LW, RW)",
    )
    .await;
}

#[tokio::test]
async fn p_w17_072_long_and_chain() {
    assert_filter_accepted("g>=1 AND a>=1 AND p>=1 AND pim>=0 AND shots>=1").await;
}

#[tokio::test]
async fn p_w17_073_long_or_chain() {
    assert_filter_accepted("g>=20 OR a>=20 OR p>=40 OR pim>=50").await;
}

#[tokio::test]
async fn p_w17_074_demorgan_compound() {
    assert_filter_accepted("NOT (country=CAN AND pos=C)").await;
}

#[tokio::test]
async fn p_w17_075_paren_changes_outcome() {
    assert_filter_accepted("(g>=10 OR a>=10) AND p>=20").await;
}

#[tokio::test]
async fn p_w17_076_double_not() {
    assert_filter_accepted("NOT NOT g>=10").await;
}

#[tokio::test]
async fn p_w17_077_unclosed_paren_rejected() {
    assert_filter_rejected("(g>=10 AND a>=10").await;
}

#[tokio::test]
async fn p_w17_078_arrow_eq_typo_rejected() {
    assert_filter_rejected("g=>5").await;
}

#[tokio::test]
async fn p_w17_079_sql_ne_typo_rejected() {
    assert_filter_rejected("g<>5").await;
}

#[tokio::test]
async fn p_w17_080_killer_query_full() {
    // The user's vision query end-to-end on the web.
    assert_filter_accepted("g.any10g>=5 EVER AT age<=25 AND country IN (CAN, USA, SWE)").await;
}
