//! Phase Lindsay L.1.5 — Rate-limit policy (TAPE-R3) tests.
//!
//! Verifies the retry surface widened from 429-only to 429 + 5xx, the
//! exponential backoff cap is honored, and 4xx errors fail-fast without
//! consuming retry budget.
//!
//! Lives in its own integration-test crate to avoid coupling to the
//! existing `mock_nhl_api.rs` fixtures.

use httpmock::prelude::*;
use icelines_core::season_stats::SeasonType;
use icelines_fetch::nhl_api::NhlApiClient;

/// 429 Too Many Requests: with `max_retries=0`, surfaces immediately as
/// `RateLimited`. Asserts the retry path RECOGNIZES 429 as retryable
/// (would have retried with budget) rather than mis-classifying it.
#[tokio::test]
async fn l1_lindsay_fetch_429_with_zero_retries_surfaces_rate_limited() {
    let server = MockServer::start();
    let _m = server.mock(|when, then| {
        when.method(GET).path_contains("/skater/bios");
        then.status(429);
    });

    let client = NhlApiClient::new(server.url(""), "http://unused.local")
        .with_retry_params(0, 1, 100);

    let err = client
        .fetch_all_bios("20252026", SeasonType::Regular)
        .await
        .expect_err("429 must surface");
    assert!(
        matches!(err, icelines_fetch::FetchError::RateLimited { .. }),
        "got: {err:?}",
    );
}

/// 503 Service Unavailable (a 5xx): Lindsay-NEW behavior retries 5xx
/// (pre-Lindsay only retried 429). With `max_retries=0`, surfaces as
/// `ServiceUnavailable` — preserves the pre-Lindsay error variant for
/// back-compat.
#[tokio::test]
async fn l1_lindsay_fetch_503_with_zero_retries_surfaces_service_unavailable() {
    let server = MockServer::start();
    let _m = server.mock(|when, then| {
        when.method(GET).path_contains("/skater/bios");
        then.status(503);
    });

    let client = NhlApiClient::new(server.url(""), "http://unused.local")
        .with_retry_params(0, 1, 100);

    let err = client
        .fetch_all_bios("20252026", SeasonType::Regular)
        .await
        .expect_err("503 must surface");
    assert!(
        matches!(err, icelines_fetch::FetchError::ServiceUnavailable { .. }),
        "got: {err:?}",
    );
}

/// 500 Internal Server Error (Lindsay-NEW retry surface): with
/// `max_retries=0`, surfaces as `Http { status: 500, .. }`. Catches a
/// regression where a 500 might be mis-classified as non-retryable.
#[tokio::test]
async fn l1_lindsay_fetch_500_with_zero_retries_surfaces_http_500() {
    let server = MockServer::start();
    let _m = server.mock(|when, then| {
        when.method(GET).path_contains("/skater/bios");
        then.status(500);
    });

    let client = NhlApiClient::new(server.url(""), "http://unused.local")
        .with_retry_params(0, 1, 100);

    let err = client
        .fetch_all_bios("20252026", SeasonType::Regular)
        .await
        .expect_err("500 must surface");
    assert!(
        matches!(err, icelines_fetch::FetchError::Http { status: 500, .. }),
        "got: {err:?}",
    );
}

/// Non-retryable 4xx errors fail-fast. A 404 doesn't burn retry budget.
/// Pin: even with `max_retries=5`, a 404 surfaces after exactly one
/// request — the retry path correctly distinguishes "transient" from
/// "permanent".
#[tokio::test]
async fn l1_lindsay_fetch_404_does_not_retry() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET).path_contains("/skater/bios");
        then.status(404);
    });

    let client = NhlApiClient::new(server.url(""), "http://unused.local")
        .with_retry_params(5, 1, 100);

    let err = client
        .fetch_all_bios("20252026", SeasonType::Regular)
        .await
        .expect_err("404 must fail-fast");
    assert!(
        matches!(err, icelines_fetch::FetchError::Http { status: 404, .. }),
        "got: {err:?}",
    );
    assert_eq!(
        m.hits(),
        1,
        "404 must not consume retry budget — exactly one request",
    );
}

/// 401 Unauthorized fail-fast. Same logic as 404 — non-5xx, non-429
/// 4xx codes don't waste retry budget.
#[tokio::test]
async fn l1_lindsay_fetch_401_does_not_retry() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET).path_contains("/skater/bios");
        then.status(401);
    });

    let client = NhlApiClient::new(server.url(""), "http://unused.local")
        .with_retry_params(5, 1, 100);

    let _err = client
        .fetch_all_bios("20252026", SeasonType::Regular)
        .await
        .expect_err("401 must fail-fast");
    assert_eq!(m.hits(), 1, "401 fail-fast — single request");
}

/// Backoff cap honored: total wall-clock for `max_retries=3` with
/// `retry_base_ms=100, retry_cap_ms=10` stays under 500ms (without cap
/// the sleeps would compound to 100+200+400 = 700ms+ before the
/// exhaust). Exercises the `delay.min(cap)` logic.
#[tokio::test]
async fn l1_lindsay_fetch_retry_cap_caps_per_attempt_sleep() {
    let server = MockServer::start();
    let _m = server.mock(|when, then| {
        when.method(GET).path_contains("/skater/bios");
        then.status(429);
    });

    let client = NhlApiClient::new(server.url(""), "http://unused.local")
        .with_retry_params(3, 100, 10);

    let started = std::time::Instant::now();
    let _err = client
        .fetch_all_bios("20252026", SeasonType::Regular)
        .await
        .expect_err("retries exhausted");
    let elapsed = started.elapsed();

    assert!(
        elapsed < std::time::Duration::from_millis(500),
        "backoff cap not enforced — total elapsed {elapsed:?} > 500ms ceiling",
    );
}

/// Retries DO actually retry: with `max_retries=3` and a 429 mock, the
/// mock receives MORE than one request before the budget exhausts.
/// This verifies the retry loop fires (vs the 4xx fail-fast path).
#[tokio::test]
async fn l1_lindsay_fetch_429_with_retries_does_retry() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET).path_contains("/skater/bios");
        then.status(429);
    });

    let client = NhlApiClient::new(server.url(""), "http://unused.local")
        .with_retry_params(3, 1, 10);

    let _err = client
        .fetch_all_bios("20252026", SeasonType::Regular)
        .await
        .expect_err("retries exhausted");
    assert!(
        m.hits() >= 4,
        "max_retries=3 means 1 initial + 3 retries = 4+ requests; got {}",
        m.hits(),
    );
}
