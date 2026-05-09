//! Phase Foster.1 — L1 mock-NHL-API tests for date-anchored schedule fetch.
//!
//! Mounts a minimal `gameWeek` JSON against httpmock at the
//! `/schedule/{date}` route and asserts `fetch_schedule_for_date`
//! routes correctly + parses the date-windowed response. Two
//! historical dates exercise the same code path with different
//! values to catch URL-formation bugs.

use httpmock::prelude::*;
use icelines_fetch::nhl_api::NhlApiClient;

fn date_fixture(date: &str, game_id: u64, away: &str, home: &str) -> String {
    format!(
        r#"{{
            "gameWeek": [{{
                "date": "{date}",
                "games": [{{
                    "id": {game_id},
                    "gameType": 2,
                    "awayTeam": {{
                        "abbrev": "{away}",
                        "placeName": {{ "default": "{away}" }}
                    }},
                    "homeTeam": {{
                        "abbrev": "{home}",
                        "placeName": {{ "default": "{home}" }}
                    }},
                    "startTimeUTC": "{date}T23:00:00Z",
                    "gameState": "OFF",
                    "awayTeam.score": null
                }}]
            }}]
        }}"#
    )
}

#[tokio::test]
async fn l1_foster1_fetch_schedule_for_date_2014_10_08() {
    let server = MockServer::start();
    let body = date_fixture("2014-10-08", 2014020001, "MTL", "TOR");
    let mock = server.mock(|when, then| {
        when.method(GET).path("/schedule/2014-10-08");
        then.status(200)
            .header("content-type", "application/json")
            .body(&body);
    });

    let client = NhlApiClient::new("http://unused", server.base_url()).with_retry_params(0, 1, 10);
    let games = client
        .fetch_schedule_for_date("2014-10-08")
        .await
        .expect("fetch ok");

    mock.assert();
    assert_eq!(games.len(), 1, "one game on 2014-10-08 fixture");
    assert_eq!(games[0].date, "2014-10-08");
    assert_eq!(games[0].away_abbrev, "MTL");
    assert_eq!(games[0].home_abbrev, "TOR");
    assert_eq!(games[0].game_id, 2014020001);
}

#[tokio::test]
async fn l1_foster1_fetch_schedule_for_date_2024_12_01() {
    let server = MockServer::start();
    let body = date_fixture("2024-12-01", 2024020342, "EDM", "VAN");
    let mock = server.mock(|when, then| {
        when.method(GET).path("/schedule/2024-12-01");
        then.status(200)
            .header("content-type", "application/json")
            .body(&body);
    });

    let client = NhlApiClient::new("http://unused", server.base_url()).with_retry_params(0, 1, 10);
    let games = client
        .fetch_schedule_for_date("2024-12-01")
        .await
        .expect("fetch ok");

    mock.assert();
    assert_eq!(games.len(), 1);
    assert_eq!(games[0].date, "2024-12-01");
    assert_eq!(games[0].away_abbrev, "EDM");
}

#[tokio::test]
async fn l1_foster1_fetch_schedule_url_includes_date_segment() {
    // Catches a regression where someone replaces /schedule/{date}
    // with a query-param style — the URL formation is the contract.
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/schedule/2026-01-15");
        then.status(200).body(r#"{"gameWeek": []}"#);
    });

    let client = NhlApiClient::new("http://unused", server.base_url()).with_retry_params(0, 1, 10);
    // If the URL formation is wrong, httpmock doesn't match and the
    // call hits the real network (or times out). The expectation
    // here is the request lands on the mounted path.
    let games = client
        .fetch_schedule_for_date("2026-01-15")
        .await
        .expect("URL must hit the mounted path");
    assert!(games.is_empty(), "empty gameWeek → empty result");
}
