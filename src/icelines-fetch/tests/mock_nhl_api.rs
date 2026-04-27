//! Mock NHL API integration tests.
//!
//! Uses httpmock to serve realistic NHL API responses for 3 test players:
//! - Player 1 (McDavid-like): elite forward, center, EDM
//! - Player 2 (Beniers-like): young center, SEA
//! - Player 3 (Makar-like): elite defenseman, COL
//!
//! Tests verify the full fetch → parse → build pipeline without live API calls.
//! httpmock starts an in-process HTTP server; NhlApiClient is pointed at it.

use httpmock::prelude::*;
use icelines_fetch::{
    nhl_api::NhlApiClient,
    player_builder::{build_players_from_bios, index_bios, index_stats},
    schema::{PagedResponse, SkaterBio, SkaterRealtime, SkaterStats},
};
use std::collections::HashMap;

// ── Fixture data ──────────────────────────────────────────────────────────────

/// 3-player bios payload — matches SkaterBio camelCase deserialization.
const FIXTURE_BIOS_JSON: &str = r#"{
  "data": [
    {
      "playerId": 8478402,
      "skaterFullName": "Connor McPlayer",
      "gamesPlayed": 82,
      "goals": 52,
      "assists": 89,
      "points": 141,
      "currentTeamAbbrev": "EDM",
      "positionCode": "C",
      "birthDate": "1997-01-13",
      "birthCountry": "CAN",
      "nationalityCode": "CAN",
      "shootsCatches": "L",
      "draftYear": 2015,
      "draftRound": 1,
      "draftOverall": 1,
      "birthCity": "Edmonton",
      "birthStateProvinceCode": "AB",
      "height": 73,
      "weight": 193,
      "firstSeasonForGameType": 20152016,
      "isInHallOfFameYn": "N",
      "lastName": "McPlayer"
    },
    {
      "playerId": 8482665,
      "skaterFullName": "Matty Beplayer",
      "gamesPlayed": 79,
      "goals": 24,
      "assists": 38,
      "points": 62,
      "currentTeamAbbrev": "SEA",
      "positionCode": "C",
      "birthDate": "2002-11-05",
      "birthCountry": "USA",
      "nationalityCode": "USA",
      "shootsCatches": "L",
      "draftYear": 2021,
      "draftRound": 1,
      "draftOverall": 2,
      "birthCity": "Shoreline",
      "birthStateProvinceCode": "WA",
      "height": 74,
      "weight": 185,
      "firstSeasonForGameType": 20222023,
      "isInHallOfFameYn": "N",
      "lastName": "Beplayer"
    },
    {
      "playerId": 8480069,
      "skaterFullName": "Cale Makelar",
      "gamesPlayed": 77,
      "goals": 25,
      "assists": 65,
      "points": 90,
      "currentTeamAbbrev": "COL",
      "positionCode": "D",
      "birthDate": "1998-10-30",
      "birthCountry": "CAN",
      "nationalityCode": "CAN",
      "shootsCatches": "L",
      "draftYear": 2017,
      "draftRound": 1,
      "draftOverall": 4,
      "birthCity": "Sherwood Park",
      "birthStateProvinceCode": "AB",
      "height": 72,
      "weight": 187,
      "firstSeasonForGameType": 20192020,
      "isInHallOfFameYn": "N",
      "lastName": "Makelar"
    }
  ],
  "total": 3
}"#;

/// 3-player stats payload — matches SkaterStats camelCase deserialization.
const FIXTURE_STATS_JSON: &str = r#"{
  "data": [
    {
      "playerId": 8478402,
      "gamesPlayed": 82,
      "goals": 52,
      "assists": 89,
      "points": 141,
      "pointsPerGame": 1.72,
      "ppGoals": 18,
      "ppPoints": 40,
      "shGoals": 1,
      "shPoints": 2,
      "gameWinningGoals": 8,
      "otGoals": 2,
      "shots": 290,
      "shootingPct": 0.1793,
      "plusMinus": 22,
      "timeOnIcePerGame": 1335.0,
      "faceoffWinPct": 0.5212
    },
    {
      "playerId": 8482665,
      "gamesPlayed": 79,
      "goals": 24,
      "assists": 38,
      "points": 62,
      "pointsPerGame": 0.785,
      "ppGoals": 7,
      "ppPoints": 18,
      "shGoals": 0,
      "shPoints": 0,
      "gameWinningGoals": 4,
      "otGoals": 1,
      "shots": 195,
      "shootingPct": 0.1231,
      "plusMinus": 8,
      "timeOnIcePerGame": 1150.0,
      "faceoffWinPct": 0.4987
    },
    {
      "playerId": 8480069,
      "gamesPlayed": 77,
      "goals": 25,
      "assists": 65,
      "points": 90,
      "pointsPerGame": 1.169,
      "ppGoals": 11,
      "ppPoints": 36,
      "shGoals": 0,
      "shPoints": 0,
      "gameWinningGoals": 5,
      "otGoals": 1,
      "shots": 240,
      "shootingPct": 0.1042,
      "plusMinus": 30,
      "timeOnIcePerGame": 1620.0,
      "faceoffWinPct": null
    }
  ],
  "total": 3
}"#;

/// 3-player realtime stats payload — hits, blocks, giveaways, takeaways, pim.
const FIXTURE_REALTIME_JSON: &str = r#"{
  "data": [
    {
      "playerId": 8478402,
      "hits": 28,
      "blockedShots": 15,
      "missedShots": 45,
      "giveaways": 35,
      "takeaways": 42,
      "pim": 14
    },
    {
      "playerId": 8482665,
      "hits": 55,
      "blockedShots": 32,
      "missedShots": 30,
      "giveaways": 22,
      "takeaways": 30,
      "pim": 18
    },
    {
      "playerId": 8480069,
      "hits": 45,
      "blockedShots": 110,
      "missedShots": 38,
      "giveaways": 28,
      "takeaways": 38,
      "pim": 26
    }
  ],
  "total": 3
}"#;

// ── Test helpers ──────────────────────────────────────────────────────────────

/// Parse fixture JSON directly without network — pure unit verification.
fn parse_bios_fixture() -> Vec<SkaterBio> {
    let page: PagedResponse<SkaterBio> =
        serde_json::from_str(FIXTURE_BIOS_JSON).expect("fixture bios JSON must be valid");
    assert_eq!(page.total, 3, "fixture must declare 3 bios");
    page.data
}

fn parse_stats_fixture() -> Vec<SkaterStats> {
    let page: PagedResponse<SkaterStats> =
        serde_json::from_str(FIXTURE_STATS_JSON).expect("fixture stats JSON must be valid");
    assert_eq!(page.total, 3, "fixture must declare 3 stats rows");
    page.data
}

fn parse_realtime_fixture() -> Vec<SkaterRealtime> {
    let page: PagedResponse<SkaterRealtime> =
        serde_json::from_str(FIXTURE_REALTIME_JSON).expect("fixture realtime JSON must be valid");
    assert_eq!(page.total, 3, "fixture must declare 3 realtime rows");
    page.data
}

// ── L0: fixture JSON is valid and matches schema ──────────────────────────────

#[test]
fn l0_fixture_bios_parses_correctly() {
    let bios = parse_bios_fixture();
    assert_eq!(bios.len(), 3, "must have 3 bios");

    // Verify McPlayer (McDavid-like)
    let mc = bios.iter().find(|b| b.player_id == 8478402).unwrap();
    assert_eq!(mc.skater_full_name, "Connor McPlayer");
    assert_eq!(mc.current_team_abbrev.as_deref(), Some("EDM"));
    assert_eq!(mc.position_code, "C");
    assert_eq!(mc.draft_year, Some(2015));
    assert_eq!(mc.draft_overall, Some(1));
    assert_eq!(mc.birth_country.as_deref(), Some("CAN"));
}

#[test]
fn l0_fixture_stats_parses_correctly() {
    let stats = parse_stats_fixture();
    assert_eq!(stats.len(), 3, "must have 3 stats rows");

    // McPlayer stats
    let mc_stats = stats.iter().find(|s| s.player_id == 8478402).unwrap();
    assert_eq!(mc_stats.goals, 52);
    assert_eq!(mc_stats.assists, 89);
    assert_eq!(mc_stats.pp_goals, 18);
    assert_eq!(mc_stats.pp_points, 40);
    assert_eq!(mc_stats.game_winning_goals, 8);
    assert_eq!(mc_stats.shots, 290);
    assert_eq!(mc_stats.plus_minus, 22);

    // Makelar (Makar-like): no faceoff data (defenseman)
    let makar_stats = stats.iter().find(|s| s.player_id == 8480069).unwrap();
    assert!(makar_stats.faceoff_win_pct.is_none(), "defenseman must have None faceoff%");
}

#[test]
fn l0_fixture_realtime_parses_correctly() {
    let rt = parse_realtime_fixture();
    assert_eq!(rt.len(), 3, "must have 3 realtime rows");

    let mc_rt = rt.iter().find(|r| r.player_id == 8478402).unwrap();
    assert_eq!(mc_rt.hits, 28);
    assert_eq!(mc_rt.blocked_shots, 15);
    assert_eq!(mc_rt.takeaways, 42);
    assert_eq!(mc_rt.giveaways, 35);
    assert_eq!(mc_rt.pim, 14);

    // Makelar (defenseman) should have more blocks
    let makar_rt = rt.iter().find(|r| r.player_id == 8480069).unwrap();
    assert!(
        makar_rt.blocked_shots > mc_rt.blocked_shots,
        "defenseman should have more blocked shots than forward"
    );
}

// ── L1: parse + build pipeline from fixture data (no network) ────────────────

#[test]
fn l1_fixture_bios_and_stats_produce_valid_players() {
    let bios = parse_bios_fixture();
    let stats = parse_stats_fixture();
    let rt: Vec<SkaterRealtime> = parse_realtime_fixture();

    let _bio_idx = index_bios(&bios);
    let stats_idx = index_stats(&stats);
    let rt_map: HashMap<u32, SkaterRealtime> =
        rt.into_iter().map(|r| (r.player_id, r)).collect();
    let empty_mp = HashMap::new();
    let empty_contracts = HashMap::new();

    let players = build_players_from_bios(
        &bios,
        &stats_idx,
        &rt_map,
        &empty_mp,
        &empty_contracts,
        icelines_core::model::Season(20252026),
    );

    // All 3 fixture players are skaters (C, C, D) and have teams — all must be built
    assert_eq!(players.len(), 3, "all 3 fixture players must be built");

    // Verify McPlayer
    let mc = players.iter().find(|p| p.nhl_id == Some(8478402)).unwrap();
    assert_eq!(mc.season_goals, 52);
    assert_eq!(mc.season_assists, 89);
    assert_eq!(mc.pp_goals, 18);
    assert_eq!(mc.pp_points, 40);
    assert_eq!(mc.gwg, 8);
    assert_eq!(mc.shots, 290);
    assert_eq!(mc.plus_minus, 22);
    assert_eq!(mc.hits, 28);
    assert_eq!(mc.blocked_shots, 15);
    assert_eq!(mc.takeaways, 42);
    assert!(mc.toi_per_game_sec.is_some(), "toi_per_game_sec must be populated");
    assert!(
        mc.shooting_pct.is_some(),
        "shooting_pct must be populated when shots > 0"
    );
    assert!(
        mc.faceoff_win_pct.is_some(),
        "center must have faceoff_win_pct"
    );
    // Contract fields must be None (not fetched)
    assert!(mc.contract_expiry_year.is_none(), "contract_expiry_year must be None");
    assert!(mc.expiry_type.is_none(), "expiry_type must be None");
    // MoneyPuck fields must be None (not fetched)
    assert!(mc.xg.is_none(), "xg must be None when not fetched");
    assert!(mc.cf_pct_5v5.is_none(), "cf_pct_5v5 must be None when not fetched");
}

#[test]
fn l1_fixture_defenseman_has_no_faceoff_pct() {
    let bios = parse_bios_fixture();
    let stats = parse_stats_fixture();
    let stats_idx = index_stats(&stats);
    let empty_rt = HashMap::new();
    let empty_mp = HashMap::new();
    let empty_contracts = HashMap::new();

    let players = build_players_from_bios(
        &bios,
        &stats_idx,
        &empty_rt,
        &empty_mp,
        &empty_contracts,
        icelines_core::model::Season(20252026),
    );

    let makar = players
        .iter()
        .find(|p| p.nhl_id == Some(8480069))
        .expect("Makelar must be in players");
    assert!(
        makar.faceoff_win_pct.is_none(),
        "defenseman must have no faceoff_win_pct (was None in fixture)"
    );
    assert_eq!(makar.position, icelines_core::model::Position::Defense);
}

#[test]
fn l1_fixture_realtime_fields_populated_from_rt_map() {
    let bios = parse_bios_fixture();
    let stats = parse_stats_fixture();
    let rt = parse_realtime_fixture();

    let stats_idx = index_stats(&stats);
    let rt_map: HashMap<u32, SkaterRealtime> =
        rt.into_iter().map(|r| (r.player_id, r)).collect();
    let empty_mp = HashMap::new();
    let empty_contracts = HashMap::new();

    let players = build_players_from_bios(
        &bios,
        &stats_idx,
        &rt_map,
        &empty_mp,
        &empty_contracts,
        icelines_core::model::Season(20252026),
    );

    // Beniers-like (Beplayer) should have realtime stats populated
    let beniers = players
        .iter()
        .find(|p| p.nhl_id == Some(8482665))
        .expect("Beplayer must be in players");
    assert_eq!(beniers.hits, 55, "hits must come from realtime fixture");
    assert_eq!(beniers.blocked_shots, 32, "blocks must come from realtime fixture");
    assert_eq!(beniers.takeaways, 30, "takeaways must come from realtime fixture");
    assert_eq!(beniers.pim, 18, "pim must come from realtime fixture");
}

#[test]
fn l1_fixture_players_have_pace_score_when_eligible() {
    let bios = parse_bios_fixture();
    let stats = parse_stats_fixture();
    let stats_idx = index_stats(&stats);
    let empty_rt = HashMap::new();
    let empty_mp = HashMap::new();
    let empty_contracts = HashMap::new();

    let players = build_players_from_bios(
        &bios,
        &stats_idx,
        &empty_rt,
        &empty_mp,
        &empty_contracts,
        icelines_core::model::Season(20252026),
    );

    // All 3 players have GP > MIN_GP — all must have a pace score
    for p in &players {
        assert!(
            p.pace_score.is_some(),
            "player {} must have pace_score (gp={})",
            p.full_name,
            p.gp().unwrap_or(0)
        );
    }

    // McPlayer (52G+89A in 82GP) should be the top scorer
    let mc = players.iter().find(|p| p.nhl_id == Some(8478402)).unwrap();
    let expected_pace = (52.0 + 89.0) / 82.0 * 82.0; // = 141.0
    let actual_pace = mc.pace_score.unwrap().pace_82;
    assert!(
        (actual_pace - expected_pace).abs() < 0.01,
        "McPlayer pace_82 should be {:.1}, got {:.1}",
        expected_pace,
        actual_pace
    );
}

#[test]
fn l1_fixture_bio_demographics_populated() {
    let bios = parse_bios_fixture();
    let stats = parse_stats_fixture();
    let stats_idx = index_stats(&stats);
    let empty_rt = HashMap::new();
    let empty_mp = HashMap::new();
    let empty_contracts = HashMap::new();

    let players = build_players_from_bios(
        &bios,
        &stats_idx,
        &empty_rt,
        &empty_mp,
        &empty_contracts,
        icelines_core::model::Season(20252026),
    );

    let mc = players.iter().find(|p| p.nhl_id == Some(8478402)).unwrap();
    assert_eq!(mc.birth_country.as_deref(), Some("CAN"));
    assert_eq!(mc.birth_state_province.as_deref(), Some("AB"));
    assert_eq!(mc.draft_year, Some(2015));
    assert_eq!(mc.draft_round, Some(1));
    assert_eq!(mc.draft_overall, Some(1));
    assert!(mc.rookie_season.is_some(), "rookie_season must be populated");
}

// ── L1: mock HTTP server tests ────────────────────────────────────────────────

#[tokio::test]
async fn l1_mock_fetch_bios_returns_3_players() {
    let server = MockServer::start();

    // Mock the paginated bios endpoint — single page with all 3 players
    let _mock = server.mock(|when, then| {
        when.method(GET)
            .path_contains("/skater/bios");
        then.status(200)
            .header("content-type", "application/json")
            .body(FIXTURE_BIOS_JSON);
    });

    let base_stats = server.url("");
    let client = NhlApiClient::new(&base_stats, "http://unused.local");

    let bios = client
        .fetch_all_bios("20252026")
        .await
        .expect("mock fetch_all_bios must succeed");
    assert_eq!(bios.len(), 3, "must parse 3 bios from mock response");
    assert!(
        bios.iter().any(|b| b.player_id == 8478402),
        "McPlayer must be in bios"
    );
}

#[tokio::test]
async fn l1_mock_fetch_stats_parses_pp_goals() {
    let server = MockServer::start();

    let _mock = server.mock(|when, then| {
        when.method(GET)
            .path_contains("/skater/summary");
        then.status(200)
            .header("content-type", "application/json")
            .body(FIXTURE_STATS_JSON);
    });

    let base_stats = server.url("");
    let client = NhlApiClient::new(&base_stats, "http://unused.local");

    let stats = client
        .fetch_all_stats("20252026")
        .await
        .expect("mock fetch_all_stats must succeed");
    assert_eq!(stats.len(), 3, "must parse 3 stats rows");

    let mc_stats = stats.iter().find(|s| s.player_id == 8478402).unwrap();
    assert_eq!(mc_stats.pp_goals, 18, "pp_goals must be parsed from fixture");
    assert_eq!(mc_stats.pp_points, 40, "pp_points must be parsed from fixture");
    assert!(
        mc_stats.shooting_pctg.is_some(),
        "shooting_pct must be Some when shots > 0"
    );
}

#[tokio::test]
async fn l1_mock_fetch_realtime_parses_hits() {
    let server = MockServer::start();

    let _mock = server.mock(|when, then| {
        when.method(GET)
            .path_contains("/skater/realtime");
        then.status(200)
            .header("content-type", "application/json")
            .body(FIXTURE_REALTIME_JSON);
    });

    let base_stats = server.url("");
    let client = NhlApiClient::new(&base_stats, "http://unused.local");

    let realtime = client
        .fetch_all_realtime("20252026")
        .await
        .expect("mock fetch_all_realtime must succeed");
    assert_eq!(realtime.len(), 3, "must parse 3 realtime rows");

    let mc_rt = realtime.iter().find(|r| r.player_id == 8478402).unwrap();
    assert_eq!(mc_rt.hits, 28, "hits must be parsed from realtime fixture");
    assert_eq!(mc_rt.blocked_shots, 15, "blocked_shots must be parsed");
    assert_eq!(mc_rt.takeaways, 42, "takeaways must be parsed");
    assert_eq!(mc_rt.giveaways, 35, "giveaways must be parsed");
    assert_eq!(mc_rt.pim, 14, "pim must be parsed");
}

#[tokio::test]
async fn l1_mock_player_build_all_fields_populated() {
    // Build players from mock bios + stats + realtime (no contracts, no MoneyPuck)
    let bios = parse_bios_fixture();
    let stats = parse_stats_fixture();
    let rt = parse_realtime_fixture();

    let stats_idx = index_stats(&stats);
    let rt_map: HashMap<u32, SkaterRealtime> =
        rt.into_iter().map(|r| (r.player_id, r)).collect();
    let empty_mp = HashMap::new();
    let empty_contracts = HashMap::new();

    let players = build_players_from_bios(
        &bios,
        &stats_idx,
        &rt_map,
        &empty_mp,
        &empty_contracts,
        icelines_core::model::Season(20252026),
    );

    assert_eq!(players.len(), 3, "all 3 players must be built");

    let mc = players.iter().find(|p| p.nhl_id == Some(8478402)).unwrap();

    // Stats fields
    assert_eq!(mc.pp_goals, 18);
    assert!(mc.toi_per_game_sec.is_some(), "toi must be populated");
    assert!(mc.shooting_pct.is_some(), "shooting_pct must be populated");

    // Realtime fields
    assert_eq!(mc.hits, 28, "hits must flow from realtime");
    assert_eq!(mc.blocked_shots, 15, "blocks must flow from realtime");
    assert_eq!(mc.takeaways, 42, "takeaways must flow from realtime");
    assert_eq!(mc.giveaways, 35, "giveaways must flow from realtime");

    // Contract fields must be None (not fetched via contracts endpoint)
    assert!(mc.contract_expiry_year.is_none(), "contract_expiry_year must be None");
    assert!(mc.expiry_type.is_none(), "expiry_type must be None");
    assert!(mc.salary.is_none(), "salary must be None");

    // MoneyPuck fields must be None (not fetched)
    assert!(mc.xg.is_none(), "xg must be None without MoneyPuck");
    assert!(mc.cf_pct_5v5.is_none(), "cf_pct must be None without MoneyPuck");
    assert!(mc.xgf_pct_5v5.is_none(), "xgf_pct must be None without MoneyPuck");
}

#[tokio::test]
async fn l1_mock_player_build_graceful_with_missing_stats() {
    // McPlayer has bio but no stats row — should be built with 0s from bio fallback
    let bios = parse_bios_fixture();
    // Only Beniers stats (exclude McPlayer and Makar)
    let stats: Vec<SkaterStats> = parse_stats_fixture()
        .into_iter()
        .filter(|s| s.player_id == 8482665)
        .collect();
    let stats_idx = index_stats(&stats);
    let empty_rt = HashMap::new();
    let empty_mp = HashMap::new();
    let empty_contracts = HashMap::new();

    let players = build_players_from_bios(
        &bios,
        &stats_idx,
        &empty_rt,
        &empty_mp,
        &empty_contracts,
        icelines_core::model::Season(20252026),
    );

    // All 3 bios should produce players even when stats are missing for 2 of them
    assert_eq!(
        players.len(),
        3,
        "all bio players must be built even with missing stats rows"
    );

    // McPlayer without stats should fall back to bio's goals/assists/gp
    let mc = players
        .iter()
        .find(|p| p.nhl_id == Some(8478402))
        .expect("McPlayer must be in players even without stats row");
    // Bio has goals=52, assists=89, gamesPlayed=82
    assert_eq!(
        mc.season_goals, 52,
        "goals must fall back to bio when stats row is missing"
    );
    // pp_goals should be 0 (no stats row)
    assert_eq!(mc.pp_goals, 0, "pp_goals must be 0 without stats row");
}

#[tokio::test]
async fn l1_mock_http_server_404_returns_error() {
    let server = MockServer::start();

    // Return 404 for all requests
    let _mock = server.mock(|when, then| {
        when.method(GET);
        then.status(404).body("Not Found");
    });

    let base_stats = server.url("");
    let client = NhlApiClient::new(&base_stats, "http://unused.local");

    let result = client.fetch_all_bios("20252026").await;
    assert!(
        result.is_err(),
        "404 response must return Err, not empty vec"
    );
}
