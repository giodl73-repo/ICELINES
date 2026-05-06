//! Phase Calder.1 — L1 integration test for the career-history HTTP path.
//!
//! Mounts the frozen McDavid landing fixture under a httpmock server,
//! points NhlApiClient at it, and asserts the parsed result matches
//! what the L0 parser produces directly. Catches drift between the
//! HTTP layer (URL formation, get_json plumbing) and the parser.

use httpmock::prelude::*;
use icelines_fetch::nhl_api::NhlApiClient;

fn fixture_path(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("landing")
        .join(name)
}

#[tokio::test]
async fn l1_fetch_player_career_history_routes_through_landing() {
    let server = MockServer::start();
    let body = std::fs::read_to_string(fixture_path("mcdavid_8478402.json"))
        .expect("McDavid fixture readable");

    let mock = server.mock(|when, then| {
        when.method(GET).path("/player/8478402/landing");
        then.status(200)
            .header("content-type", "application/json")
            .body(&body);
    });

    let client = NhlApiClient::new("http://unused", server.base_url()).with_retry_params(0, 1, 10);
    let history = client
        .fetch_player_career_history(8478402)
        .await
        .expect("fetch ok");

    mock.assert();
    assert_eq!(history.player_id, 8478402);
    // McDavid's career covers ≥12 stints (GTHL, OHL regular + playoff
    // across 3 seasons, WJ tournaments, NHL multiple seasons).
    assert!(
        history.stints.len() >= 12,
        "expected many stints, got {}",
        history.stints.len()
    );
    // OHL is represented (he played for Erie 2012-15).
    assert!(
        history.stints.iter().any(|s| s.league.0 == "OHL"),
        "OHL stint missing"
    );
    // McDavid is an active NHL'er — most recent NHL stint should be
    // the current bundled season (20252026). We don't assert "last" is
    // NHL because Olympics / international tournaments share that
    // season slot and can sort after by sequence.
    let nhl_latest = history
        .stints
        .iter()
        .filter(|s| s.league.0 == "NHL")
        .map(|s| s.season.0)
        .max()
        .expect("at least one NHL stint");
    assert!(
        nhl_latest >= 20252026,
        "most recent NHL stint should be current season; got {nhl_latest}"
    );
}

#[tokio::test]
async fn l1_fetch_all_career_histories_collects_and_skips() {
    let server = MockServer::start();
    let mcdavid_body = std::fs::read_to_string(fixture_path("mcdavid_8478402.json"))
        .expect("McDavid fixture readable");
    let bedard_body = std::fs::read_to_string(fixture_path("bedard_8484144.json"))
        .expect("Bedard fixture readable");

    server.mock(|when, then| {
        when.method(GET).path("/player/8478402/landing");
        then.status(200)
            .header("content-type", "application/json")
            .body(&mcdavid_body);
    });
    server.mock(|when, then| {
        when.method(GET).path("/player/8484144/landing");
        then.status(200)
            .header("content-type", "application/json")
            .body(&bedard_body);
    });
    // Player 9999999 is never registered — the client should skip it.
    server.mock(|when, then| {
        when.method(GET).path("/player/9999999/landing");
        then.status(404).body("not found");
    });

    let client = NhlApiClient::new("http://unused", server.base_url()).with_retry_params(0, 1, 10);
    let pids = [8478402u32, 8484144, 9999999];
    let (histories, skipped) = client.fetch_all_career_histories(&pids).await;

    assert_eq!(histories.len(), 2, "two of three should succeed");
    assert_eq!(skipped.len(), 1, "404 must be skipped not panicked");
    assert_eq!(skipped[0].0, 9999999);

    // Both succeeding histories carry their proper player_id.
    let pids_returned: std::collections::HashSet<u32> =
        histories.iter().map(|h| h.player_id).collect();
    assert!(pids_returned.contains(&8478402));
    assert!(pids_returned.contains(&8484144));
}

#[tokio::test]
async fn l1_fetch_player_career_history_surfaces_schema_error() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/player/9999999/landing");
        then.status(200)
            .header("content-type", "application/json")
            // Object without seasonTotals — typed parse error path.
            .body(r#"{"playerId":9999999}"#);
    });

    let client = NhlApiClient::new("http://unused", server.base_url()).with_retry_params(0, 1, 10);
    let err = client
        .fetch_player_career_history(9999999)
        .await
        .expect_err("must surface as error");
    let msg = format!("{err}");
    assert!(
        msg.contains("schema") || msg.contains("seasonTotals"),
        "expected schema-related error, got: {msg}"
    );
}
