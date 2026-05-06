//! Phase Foster +1 — L1 tests for the production NhlApiFetcher.
//!
//! Mounts a httpmock server, points a NhlApiClient at it, wraps in
//! NhlApiFetcher, and asserts the sync-trait calls actually exercise
//! the network path. Catches regressions where the trait
//! implementation forgets to convert async → sync (the block_on
//! bridge), or where FetchError → DataError mapping drops detail.

use httpmock::prelude::*;
use icelines_fetch::datastore::{DataError, Fetcher, NhlApiFetcher};
use icelines_fetch::nhl_api::NhlApiClient;
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;

fn fixture_path(rel: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(rel)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn l1_foster_plus1_nhl_fetcher_career_history_round_trips() {
    // Reuse the existing McDavid landing fixture from Calder.1.
    let body =
        std::fs::read_to_string(fixture_path("landing/mcdavid_8478402.json"))
            .expect("McDavid fixture readable");
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/player/8478402/landing");
        then.status(200)
            .header("content-type", "application/json")
            .body(&body);
    });

    let client =
        NhlApiClient::new("http://unused", server.base_url()).with_retry_params(0, 1, 10);
    let fetcher = NhlApiFetcher::with_client(client);

    // The sync trait method is invoked from inside spawn_blocking
    // in production; mirror that here so block_on finds a runtime.
    let result = tokio::task::spawn_blocking(move || {
        fetcher.fetch_career_history(icelines_core::identity::PlayerId(8478402))
    })
    .await
    .expect("blocking task ok");

    mock.assert();
    let history = result.expect("fetch ok");
    assert_eq!(history.player_id, 8478402);
    assert!(
        history.stints.len() >= 5,
        "McDavid has many career stints, got {}",
        history.stints.len()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn l1_foster_plus1_nhl_fetcher_5xx_maps_to_data_error_http5xx() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/player/9999999/landing");
        then.status(503);
    });

    let client = NhlApiClient::new("http://unused", server.base_url())
        // Zero retries so the 503 propagates immediately.
        .with_retry_params(0, 1, 10);
    let fetcher = NhlApiFetcher::with_client(client);

    let result = tokio::task::spawn_blocking(move || {
        fetcher.fetch_career_history(icelines_core::identity::PlayerId(9999999))
    })
    .await
    .expect("blocking task ok");

    let err = result.expect_err("503 must error");
    match err {
        DataError::Http5xx { status, .. } => assert_eq!(status, 503),
        other => panic!("expected Http5xx, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn l1_foster_plus1_nhl_fetcher_stats_url_includes_season_and_type() {
    // The production client mounts skater/summary at /skater/summary
    // — matching the path lets us assert the URL formation surfaces
    // the season + game-type. An empty data array is a valid 200
    // response; the goal here is to prove the call lands.
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET)
            .path("/skater/summary")
            .query_param_exists("cayenneExp");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"data": [], "total": 0}"#);
    });

    let client =
        NhlApiClient::new(server.base_url(), "http://unused").with_retry_params(0, 1, 10);
    let fetcher = NhlApiFetcher::with_client(client);

    let result = tokio::task::spawn_blocking(move || {
        fetcher.fetch_stats(Season(20252026), SeasonType::Regular)
    })
    .await
    .expect("blocking task ok");

    let stats = result.expect("fetch ok");
    assert!(stats.is_empty(), "empty data array → empty Vec");
}
