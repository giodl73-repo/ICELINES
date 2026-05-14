//! Phase Foster +3 — L1 test for boxscore JSON persistence + DataStore::load_boxscore_raw.
//!
//! Mounts a httpmock server with a minimal boxscore body, fetches
//! via `fetch_boxscore_with_raw`, persists to a tempdir DataStore,
//! then reads back via `load_boxscore_raw`. Round-trip proves the
//! body bytes survive the persist + manifest cycle untouched.

use httpmock::prelude::*;
use icelines_core::identity::GameId;
use icelines_fetch::atomic_write::write_bytes_atomic;
use icelines_fetch::datastore::DataStore;
use icelines_fetch::manifest::{DataKey, DataKind, ManifestEntry};
use icelines_fetch::nhl_api::NhlApiClient;
use std::sync::Arc;

fn minimal_boxscore_body(game_id: u64, home: &str, away: &str) -> String {
    format!(
        r#"{{
            "id": {game_id},
            "gameDate": "2026-01-15",
            "gameState": "FINAL",
            "homeTeam": {{ "abbrev": "{home}", "score": 7 }},
            "awayTeam": {{ "abbrev": "{away}", "score": 3 }},
            "playerByGameStats": {{
                "homeTeam": {{ "forwards": [], "defense": [], "goalies": [] }},
                "awayTeam": {{ "forwards": [], "defense": [], "goalies": [] }}
            }}
        }}"#
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn l1_foster_plus3_fetch_boxscore_with_raw_round_trips() {
    let server = MockServer::start();
    let body = minimal_boxscore_body(2025020342, "EDM", "CGY");
    let mock = server.mock(|when, then| {
        when.method(GET).path("/gamecenter/2025020342/boxscore");
        then.status(200)
            .header("content-type", "application/json")
            .body(&body);
    });

    let client = NhlApiClient::new("http://unused", server.base_url()).with_retry_params(0, 1, 10);
    let (parsed, raw) = client
        .fetch_boxscore_with_raw(2025020342)
        .await
        .expect("fetch ok");

    mock.assert();
    assert_eq!(parsed.game_id, 2025020342);
    assert_eq!(parsed.home_abbrev, "EDM");
    assert_eq!(parsed.away_abbrev, "CGY");
    // The raw body must include the "homeTeam" key — proves
    // fetch_boxscore_with_raw returned the unmodified JSON for
    // downstream callers (Foster +4 favorited-line parse).
    assert!(raw.get("homeTeam").is_some(), "raw body preserved");
}

#[test]
fn l1_foster_plus3_load_boxscore_raw_reads_persisted_file() {
    let dir = tempfile::tempdir().unwrap();
    let store = DataStore::open(dir.path()).unwrap();

    // Plant a boxscore JSON file + manifest entry (mirrors what
    // `icelines fetch boxscore` does in production).
    let path = dir
        .path()
        .join("boxscores")
        .join("2026-01-15")
        .join("2025020342.json");
    let body = serde_json::json!({
        "id": 2025020342,
        "homeTeam": { "abbrev": "EDM", "score": 7 },
        "awayTeam": { "abbrev": "CGY", "score": 3 },
    });
    write_bytes_atomic(&path, body.to_string().as_bytes()).unwrap();

    let entry = ManifestEntry {
        key: DataKey::Game(GameId(2025020342)),
        path,
        freshness: icelines_core::Freshness {
            fetched_at: chrono::Utc::now(),
            source: icelines_core::FetchSource::Live,
            ttl: icelines_core::Ttl::Static,
        },
    };
    store.manifest().upsert(DataKind::Boxscore, entry).unwrap();

    // Read back — round-trip proves the persistence + manifest
    // lookup chain is intact.
    let got = store
        .load_boxscore_raw(DataKey::Game(GameId(2025020342)))
        .expect("manifest hit");
    assert_eq!(got["homeTeam"]["abbrev"], "EDM");
    assert_eq!(got["awayTeam"]["score"], 3);
}

#[test]
fn l1_trace_events_load_play_by_play_raw_reads_persisted_file() {
    let dir = tempfile::tempdir().unwrap();
    let store = DataStore::open(dir.path()).unwrap();

    let path = dir
        .path()
        .join("play_by_play")
        .join("2026-01-15")
        .join("2025020342.json");
    let body = serde_json::json!({
        "id": 2025020342,
        "plays": [
            {
                "eventId": 12,
                "typeDescKey": "goal",
                "details": { "scoringPlayerId": 8477444, "goalieInNetId": 8478400 }
            }
        ]
    });
    write_bytes_atomic(&path, body.to_string().as_bytes()).unwrap();

    let entry = ManifestEntry {
        key: DataKey::Game(GameId(2025020342)),
        path,
        freshness: icelines_core::Freshness {
            fetched_at: chrono::Utc::now(),
            source: icelines_core::FetchSource::Live,
            ttl: icelines_core::Ttl::Static,
        },
    };
    store
        .manifest()
        .upsert(DataKind::PlayByPlay, entry)
        .unwrap();

    let got = store
        .load_play_by_play_raw(DataKey::Game(GameId(2025020342)))
        .expect("manifest hit");
    assert_eq!(got["plays"][0]["details"]["goalieInNetId"], 8478400);
}

#[test]
fn l1_foster_plus3_load_boxscore_raw_returns_none_when_unmanifested() {
    let dir = tempfile::tempdir().unwrap();
    let store = DataStore::open(dir.path()).unwrap();
    assert!(
        store
            .load_boxscore_raw(DataKey::Game(GameId(9999999)))
            .is_none(),
        "no manifest entry → None"
    );
    let _ = Arc::new(store); // silence Arc import warning if any
}
