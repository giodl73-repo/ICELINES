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

// ── Schedule API fixtures (Phase 7d) ──────────────────────────────────────────

/// gameWeek fixture: 2 days, 3 games — one final regulation, one final OT, one playoff upcoming.
const FIXTURE_SCHEDULE_GAMEWEEK: &str = r#"{
  "gameWeek": [
    {
      "date": "2026-04-27",
      "games": [
        {
          "id": 2025030001,
          "gameType": 2,
          "awayTeam": {"abbrev":"SEA","placeName":{"default":"Seattle"},"score":3},
          "homeTeam": {"abbrev":"VGK","placeName":{"default":"Vegas"}, "score":2},
          "startTimeUTC": "2026-04-27T22:00:00Z",
          "gameState": "FINAL",
          "gameOutcome": {"lastPeriodType":"OT"}
        },
        {
          "id": 2025030002,
          "gameType": 2,
          "awayTeam": {"abbrev":"NYR","placeName":{"default":"New York"},"score":1},
          "homeTeam": {"abbrev":"WSH","placeName":{"default":"Washington"},"score":4},
          "startTimeUTC": "2026-04-27T23:00:00Z",
          "gameState": "FINAL",
          "gameOutcome": {"lastPeriodType":"REG"}
        }
      ]
    },
    {
      "date": "2026-04-28",
      "games": [
        {
          "id": 2025030101,
          "gameType": 3,
          "awayTeam": {"abbrev":"NYR","placeName":{"default":"New York"}},
          "homeTeam": {"abbrev":"WSH","placeName":{"default":"Washington"}},
          "startTimeUTC": "2026-04-28T23:05:00Z",
          "gameState": "FUT",
          "seriesSummary": {"gameLabel":"Game 5","awayWins":2,"homeWins":2}
        }
      ]
    }
  ]
}"#;

/// Team-season schedule fixture: one team (SEA), 3 games — 2 finals + 1 upcoming.
/// Note `gameDate` is on the game itself (not the day wrapper).
const FIXTURE_TEAM_SEASON_SCHEDULE: &str = r#"{
  "games": [
    {
      "id": 2025020100,
      "gameDate": "2026-01-15",
      "gameType": 2,
      "awayTeam": {"abbrev":"CGY","placeName":{"default":"Calgary"},"score":2},
      "homeTeam": {"abbrev":"SEA","placeName":{"default":"Seattle"},"score":4},
      "startTimeUTC": "2026-01-16T03:00:00Z",
      "gameState": "OFF",
      "gameOutcome": {"lastPeriodType":"REG"}
    },
    {
      "id": 2025020101,
      "gameDate": "2026-02-03",
      "gameType": 2,
      "awayTeam": {"abbrev":"SEA","placeName":{"default":"Seattle"},"score":3},
      "homeTeam": {"abbrev":"EDM","placeName":{"default":"Edmonton"},"score":4},
      "startTimeUTC": "2026-02-04T02:00:00Z",
      "gameState": "OFF",
      "gameOutcome": {"lastPeriodType":"SO"}
    },
    {
      "id": 2025020102,
      "gameDate": "2026-04-30",
      "gameType": 2,
      "awayTeam": {"abbrev":"VAN","placeName":{"default":"Vancouver"}},
      "homeTeam": {"abbrev":"SEA","placeName":{"default":"Seattle"}},
      "startTimeUTC": "2026-04-30T03:00:00Z",
      "gameState": "FUT"
    }
  ]
}"#;

#[tokio::test]
async fn l1_mock_fetch_schedule_for_date_parses_gameweek() {
    let server = MockServer::start();
    let _mock = server.mock(|when, then| {
        when.method(GET).path_contains("/schedule/2026-04-27");
        then.status(200)
            .header("content-type", "application/json")
            .body(FIXTURE_SCHEDULE_GAMEWEEK);
    });

    let client = NhlApiClient::new("http://unused.local", server.url(""));
    let games = client.fetch_schedule_for_date("2026-04-27").await
        .expect("mock fetch_schedule_for_date must succeed");

    // 3 games across 2 days
    assert_eq!(games.len(), 3, "must parse 3 games from fixture gameWeek");

    // Dates flow from the day wrapper
    assert_eq!(games[0].date, "2026-04-27");
    assert_eq!(games[2].date, "2026-04-28");

    // First game: SEA @ VGK final OT
    assert_eq!(games[0].away_abbrev, "SEA");
    assert_eq!(games[0].home_abbrev, "VGK");
    assert_eq!(games[0].game_state.as_deref(), Some("FINAL"));
    assert_eq!(games[0].last_period.as_deref(), Some("OT"));
    assert_eq!(games[0].away_score, Some(3));
    assert_eq!(games[0].home_score, Some(2));
    assert!(games[0].is_final());
    assert!(!games[0].is_playoff());
}

#[tokio::test]
async fn l1_mock_fetch_schedule_extracts_final_scores() {
    let server = MockServer::start();
    let _mock = server.mock(|when, then| {
        when.method(GET).path_contains("/schedule/");
        then.status(200).body(FIXTURE_SCHEDULE_GAMEWEEK);
    });

    let client = NhlApiClient::new("http://unused.local", server.url(""));
    let games = client.fetch_schedule_for_date("2026-04-27").await.unwrap();

    // Both regulation finals (Apr 27) should have explicit scores; playoff game (Apr 28) is FUT.
    let finals: Vec<_> = games.iter().filter(|g| g.is_final()).collect();
    assert_eq!(finals.len(), 2, "fixture has 2 finals");
    for g in &finals {
        assert!(g.away_score.is_some(), "final must have away_score");
        assert!(g.home_score.is_some(), "final must have home_score");
    }
    let upcoming: Vec<_> = games.iter().filter(|g| !g.is_final()).collect();
    assert_eq!(upcoming.len(), 1, "fixture has 1 future game");
    assert!(upcoming[0].away_score.is_none(), "future game has no score yet");
}

#[tokio::test]
async fn l1_mock_fetch_schedule_extracts_playoff_series() {
    let server = MockServer::start();
    let _mock = server.mock(|when, then| {
        when.method(GET).path_contains("/schedule/");
        then.status(200).body(FIXTURE_SCHEDULE_GAMEWEEK);
    });

    let client = NhlApiClient::new("http://unused.local", server.url(""));
    let games = client.fetch_schedule_for_date("2026-04-27").await.unwrap();

    let playoff = games.iter().find(|g| g.is_playoff())
        .expect("fixture must include one playoff game");
    assert_eq!(playoff.series_game.as_deref(), Some("Game 5"));
    assert_eq!(playoff.away_wins, Some(2));
    assert_eq!(playoff.home_wins, Some(2));
    let label = playoff.series_label().expect("series_label must format from fixture data");
    assert!(label.contains("Game 5"), "label must include game number, got: {label}");
    assert!(label.contains("NYR"), "label must include away abbrev");
}

#[tokio::test]
async fn l1_mock_fetch_team_season_schedule_parses_games() {
    let server = MockServer::start();
    let _mock = server.mock(|when, then| {
        when.method(GET).path_contains("/club-schedule-season/SEA/20252026");
        then.status(200)
            .header("content-type", "application/json")
            .body(FIXTURE_TEAM_SEASON_SCHEDULE);
    });

    let client = NhlApiClient::new("http://unused.local", server.url(""));
    let games = client.fetch_team_season_schedule("SEA", "20252026").await
        .expect("mock fetch_team_season_schedule must succeed");

    assert_eq!(games.len(), 3, "fixture has 3 games");

    // gameDate flows from the game itself, not from a day wrapper
    assert_eq!(games[0].date, "2026-01-15");
    assert_eq!(games[1].date, "2026-02-03");
    assert_eq!(games[2].date, "2026-04-30");

    // Two finals + one future
    assert_eq!(games.iter().filter(|g| g.is_final()).count(), 2);
    assert_eq!(games.iter().filter(|g| !g.is_final()).count(), 1);

    // SO last_period flows through
    let so_game = games.iter().find(|g| g.last_period.as_deref() == Some("SO"))
        .expect("fixture has one SO game");
    assert_eq!(so_game.away_abbrev, "SEA");
}

#[tokio::test]
async fn l1_mock_fetch_team_season_involves_helper() {
    let server = MockServer::start();
    let _mock = server.mock(|when, then| {
        when.method(GET).path_contains("/club-schedule-season/");
        then.status(200).body(FIXTURE_TEAM_SEASON_SCHEDULE);
    });

    let client = NhlApiClient::new("http://unused.local", server.url(""));
    let games = client.fetch_team_season_schedule("SEA", "20252026").await.unwrap();

    // Every fixture game must involve SEA (the team we asked for)
    for g in &games {
        assert!(g.involves("SEA"), "game must involve SEA, got {} @ {}", g.away_abbrev, g.home_abbrev);
    }
    // None should match a team that isn't in the fixture
    assert_eq!(
        games.iter().filter(|g| g.involves("WSH")).count(),
        0,
        "no fixture game involves WSH"
    );
}

// ── Boxscore fixtures (Phase 7c gap-fix) ──────────────────────────────────────

const FIXTURE_BOXSCORE: &str = r#"{
  "id": 2025020100,
  "awayTeam": {"abbrev": "NYR", "score": 2},
  "homeTeam": {"abbrev": "WSH", "score": 3},
  "gameState": "FINAL",
  "gameOutcome": {"lastPeriodType": "OT"},
  "summary": {
    "scoring": [
      {
        "periodDescriptor": {"number": 1, "periodType": "REG"},
        "goals": [
          {
            "firstName": {"default": "Alex"},
            "lastName":  {"default": "Ovechkin"},
            "teamAbbrev": {"default": "WSH"},
            "timeInPeriod": "08:14",
            "assists": [
              {"firstName": {"default": "Evgeny"}, "lastName": {"default": "Kuznetsov"}},
              {"firstName": {"default": "John"},   "lastName": {"default": "Carlson"}}
            ],
            "awayScore": 0, "homeScore": 1
          }
        ]
      },
      {
        "periodDescriptor": {"number": 4, "periodType": "OT"},
        "goals": [
          {
            "firstName": {"default": "Tom"},
            "lastName":  {"default": "Wilson"},
            "teamAbbrev": {"default": "WSH"},
            "timeInPeriod": "03:22",
            "assists": [
              {"firstName": {"default": "Alex"}, "lastName": {"default": "Ovechkin"}}
            ],
            "awayScore": 2, "homeScore": 3
          }
        ]
      }
    ]
  },
  "playerByGameStats": {
    "awayTeam": {
      "forwards": [
        {"playerId": 8478550, "name": {"default": "Mika Zibanejad"}, "position": "C",
         "toi": "21:33", "goals": 1, "assists": 1, "plusMinus": 1,
         "sog": 5, "hits": 2, "blockedShots": 0, "takeaways": 1, "giveaways": 1, "pim": 0},
        {"playerId": 8478402, "name": {"default": "Artemi Panarin"}, "position": "L",
         "toi": "20:02", "goals": 0, "assists": 1, "plusMinus": 0,
         "sog": 3, "hits": 0, "blockedShots": 1, "takeaways": 2, "giveaways": 0, "pim": 0}
      ],
      "defense": [
        {"playerId": 8482073, "name": {"default": "Adam Fox"}, "position": "D",
         "toi": "26:09", "goals": 0, "assists": 0, "plusMinus": -1,
         "sog": 2, "hits": 1, "blockedShots": 4, "takeaways": 0, "giveaways": 0, "pim": 0}
      ],
      "goalies": [
        {
          "firstName": {"default": "Igor"},
          "lastName":  {"default": "Shesterkin"},
          "saves": 32, "shotsAgainst": 35, "decision": "L"
        }
      ]
    },
    "homeTeam": {
      "forwards": [
        {"playerId": 8471214, "name": {"default": "Alex Ovechkin"}, "position": "L",
         "toi": "19:48", "goals": 1, "assists": 0, "plusMinus": 1,
         "sog": 6, "hits": 3, "blockedShots": 0, "takeaways": 0, "giveaways": 1, "pim": 0},
        {"playerId": 8478493, "name": {"default": "Tom Wilson"}, "position": "R",
         "toi": "17:21", "goals": 1, "assists": 0, "plusMinus": 1,
         "sog": 4, "hits": 5, "blockedShots": 1, "takeaways": 1, "giveaways": 0, "pim": 2}
      ],
      "defense": [
        {"playerId": 8474590, "name": {"default": "John Carlson"}, "position": "D",
         "toi": "25:11", "goals": 0, "assists": 1, "plusMinus": 1,
         "sog": 2, "hits": 0, "blockedShots": 5, "takeaways": 0, "giveaways": 1, "pim": 0}
      ],
      "goalies": [
        {
          "firstName": {"default": "Charlie"},
          "lastName":  {"default": "Lindgren"},
          "saves": 28, "shotsAgainst": 30, "decision": "W"
        }
      ]
    }
  }
}"#;

#[test]
fn l0_parse_boxscore_basic() {
    use icelines_fetch::nhl_api::parse_boxscore;
    let raw: serde_json::Value = serde_json::from_str(FIXTURE_BOXSCORE).unwrap();
    let bs = parse_boxscore(&raw, 2025020100);

    assert_eq!(bs.game_id, 2025020100);
    assert_eq!(bs.away_abbrev, "NYR");
    assert_eq!(bs.home_abbrev, "WSH");
    assert_eq!(bs.away_score, 2);
    assert_eq!(bs.home_score, 3);
    assert_eq!(bs.last_period.as_deref(), Some("OT"));

    // 2 goals in fixture (one in P1, one in OT)
    assert_eq!(bs.goals.len(), 2);
    let g1 = &bs.goals[0];
    assert_eq!(g1.scorer_name, "Alex Ovechkin");
    assert_eq!(g1.scorer_team, "WSH");
    assert_eq!(g1.assist1_name.as_deref(), Some("Evgeny Kuznetsov"));
    assert_eq!(g1.assist2_name.as_deref(), Some("John Carlson"));
    assert_eq!(g1.period, 1);

    let g_ot = &bs.goals[1];
    assert_eq!(g_ot.period_type, "OT");
    assert_eq!(g_ot.scorer_name, "Tom Wilson");

    // 2 goalies, one per team
    assert_eq!(bs.goalies.len(), 2);
    let nyr_g = bs.goalies.iter().find(|g| g.team_abbrev == "NYR").unwrap();
    assert_eq!(nyr_g.player_name, "Igor Shesterkin");
    assert_eq!(nyr_g.saves, 32);
    assert_eq!(nyr_g.shots, 35);
    assert_eq!(nyr_g.decision.as_deref(), Some("L"));
}

#[test]
fn l0_parse_boxscore_skater_lines_per_team() {
    // playerByGameStats forwards + defense flow into SkaterLine. Each
    // team's array contains both groups; goalies stay in their own
    // bucket (not duplicated into away_skaters/home_skaters).
    use icelines_fetch::nhl_api::parse_boxscore;
    let raw: serde_json::Value = serde_json::from_str(FIXTURE_BOXSCORE).unwrap();
    let bs = parse_boxscore(&raw, 2025020100);

    // Away (NYR): 2 forwards + 1 defenseman = 3 skaters
    assert_eq!(bs.away_skaters.len(), 3, "expected 3 NYR skaters: {:?}",
        bs.away_skaters.iter().map(|s| &s.player_name).collect::<Vec<_>>());
    // Home (WSH): 2 forwards + 1 defenseman = 3 skaters
    assert_eq!(bs.home_skaters.len(), 3);

    // Spot-check field shape on Adam Fox (highest TOI on NYR).
    let fox = bs.away_skaters.iter().find(|s| s.player_name == "Adam Fox")
        .expect("Fox in fixture");
    assert_eq!(fox.team_abbrev, "NYR");
    assert_eq!(fox.position, "D");
    assert_eq!(fox.toi_seconds, 26 * 60 + 9, "26:09 → 1569 seconds");
    assert_eq!(fox.blocked_shots, 4);
    assert_eq!(fox.plus_minus, -1);

    // Tom Wilson — high hits leader on WSH side.
    let wilson = bs.home_skaters.iter().find(|s| s.player_name == "Tom Wilson")
        .expect("Wilson in fixture");
    assert_eq!(wilson.hits, 5);
    assert_eq!(wilson.pim, 2);
}

#[test]
fn l0_parse_boxscore_handles_missing_player_by_game_stats() {
    // Older boxscore endpoints (pre-2024) may omit playerByGameStats
    // entirely. The parser must return empty skater vectors without
    // panicking so the game-detail screen can fall back gracefully.
    use icelines_fetch::nhl_api::parse_boxscore;
    let raw: serde_json::Value = serde_json::from_str(r#"{
        "id": 999,
        "awayTeam": {"abbrev": "BOS", "score": 0},
        "homeTeam": {"abbrev": "MTL", "score": 0},
        "gameState": "FUT"
    }"#).unwrap();
    let bs = parse_boxscore(&raw, 999);
    assert!(bs.away_skaters.is_empty());
    assert!(bs.home_skaters.is_empty());
}

#[tokio::test]
async fn l1_mock_fetch_boxscore_parses_goals() {
    let server = MockServer::start();
    let _mock = server.mock(|when, then| {
        when.method(GET).path_contains("/gamecenter/2025020100/boxscore");
        then.status(200)
            .header("content-type", "application/json")
            .body(FIXTURE_BOXSCORE);
    });

    let client = NhlApiClient::new("http://unused.local", server.url(""));
    let bs = client.fetch_boxscore(2025020100).await
        .expect("mock fetch_boxscore must succeed");
    assert_eq!(bs.goals.len(), 2);
    assert_eq!(bs.goalies.len(), 2);
}

#[tokio::test]
async fn l1_mock_fetch_boxscore_404_returns_error() {
    let server = MockServer::start();
    let _mock = server.mock(|when, then| {
        when.method(GET).path_contains("/gamecenter/");
        then.status(404).body("Not Found");
    });
    let client = NhlApiClient::new("http://unused.local", server.url(""));
    let result = client.fetch_boxscore(99).await;
    assert!(result.is_err(), "404 must surface as Err");
}

// ── Playoff bracket fixtures (Phase 7e) ───────────────────────────────────────

/// Two-round bracket fixture with a completed first round and an in-progress
/// second round. Two conferences, mixed seeding fields, mixed completion states.
const FIXTURE_PLAYOFF_BRACKET: &str = r#"{
  "season": "20252026",
  "currentRound": 2,
  "playoffRounds": [
    {
      "roundNumber": 1,
      "roundLabel": "First Round",
      "series": [
        {
          "seriesLetter": "A",
          "conferenceAbbrev": "E",
          "topSeedTeam": {"abbrev":"FLA","name":{"default":"Florida Panthers"},"wins":4,"seed":"A1"},
          "bottomSeedTeam": {"abbrev":"TBL","name":{"default":"Tampa Bay Lightning"},"wins":2,"seed":"WC2"},
          "winningTeam": {"abbrev":"FLA"}
        },
        {
          "seriesLetter": "B",
          "conferenceAbbrev": "E",
          "topSeedTeam": {"abbrev":"WSH","name":{"default":"Washington Capitals"},"wins":4,"seed":"M1"},
          "bottomSeedTeam": {"abbrev":"NYR","name":{"default":"New York Rangers"},"wins":3,"seed":"WC1"}
        },
        {
          "seriesLetter": "E",
          "conferenceAbbrev": "W",
          "topSeedTeam": {"abbrev":"EDM","name":{"default":"Edmonton Oilers"},"wins":4,"seed":"P1"},
          "bottomSeedTeam": {"abbrev":"VAN","name":{"default":"Vancouver Canucks"},"wins":1,"seed":"WC2"},
          "winningTeam": {"abbrev":"EDM"}
        }
      ]
    },
    {
      "roundNumber": 2,
      "roundLabel": "Second Round",
      "series": [
        {
          "seriesLetter": "I",
          "conferenceAbbrev": "E",
          "topSeedTeam": {"abbrev":"FLA","name":{"default":"Florida Panthers"},"wins":2,"seed":"A1"},
          "bottomSeedTeam": {"abbrev":"WSH","name":{"default":"Washington Capitals"},"wins":1,"seed":"M1"}
        }
      ]
    }
  ]
}"#;

#[test]
fn l0_parse_playoff_bracket_basic() {
    use icelines_fetch::nhl_api::parse_playoff_bracket;
    let raw: serde_json::Value = serde_json::from_str(FIXTURE_PLAYOFF_BRACKET).unwrap();
    let bracket = parse_playoff_bracket(&raw);

    assert_eq!(bracket.season, "20252026");
    assert_eq!(bracket.current_round, Some(2));
    assert_eq!(bracket.rounds.len(), 2);
    assert!(!bracket.is_empty());

    // First round has 3 series (FLA-TBL, WSH-NYR, EDM-VAN)
    let r1 = &bracket.rounds[0];
    assert_eq!(r1.round_number, 1);
    assert_eq!(r1.label, "First Round");
    assert_eq!(r1.series.len(), 3);

    // Series A (FLA-TBL) — explicit winner
    let a = &r1.series[0];
    assert_eq!(a.letter.as_deref(), Some("A"));
    assert_eq!(a.top_seed_abbrev, "FLA");
    assert_eq!(a.bottom_seed_abbrev, "TBL");
    assert_eq!(a.top_seed_wins, 4);
    assert_eq!(a.bottom_seed_wins, 2);
    assert_eq!(a.winner_abbrev.as_deref(), Some("FLA"));
    assert_eq!(a.conference.as_deref(), Some("Eastern"));
    assert!(a.is_complete());

    // Series B (WSH-NYR) — winner inferred from 4-win threshold
    let b = &r1.series[1];
    assert_eq!(b.winner_abbrev.as_deref(), Some("WSH"),
        "winner must be inferred when explicit winningTeam is absent but top_seed reaches 4 wins");

    // Second round series I — in progress, no winner
    let r2 = &bracket.rounds[1];
    let i = &r2.series[0];
    assert_eq!(i.letter.as_deref(), Some("I"));
    assert!(i.winner_abbrev.is_none());
    assert!(!i.is_complete());
    assert_eq!(i.games_played(), 3);
}

#[test]
fn l0_parse_playoff_bracket_empty_off_season() {
    use icelines_fetch::nhl_api::parse_playoff_bracket;
    let raw: serde_json::Value = serde_json::from_str(r#"{"season":"20252026","playoffRounds":[]}"#).unwrap();
    let bracket = parse_playoff_bracket(&raw);
    assert!(bracket.is_empty(), "empty rounds means off-season");
    assert_eq!(bracket.rounds.len(), 0);
}

#[test]
fn l0_parse_playoff_bracket_flat_series_current_api() {
    // Current `/v1/playoff-bracket/{year}` shape (verified 2026-04-29):
    // a flat `series` array where each entry carries its own
    // `playoffRound` and the wins live at the series level
    // (`topSeedWins` / `bottomSeedWins`), not on the team object.
    // The legacy `playoffRounds` shape does NOT appear.
    use icelines_fetch::nhl_api::parse_playoff_bracket;
    let raw: serde_json::Value = serde_json::from_str(r#"{
        "bracketTitle": "2026 Stanley Cup Playoffs",
        "series": [
            {
                "seriesLetter": "A",
                "seriesAbbrev": "R1",
                "seriesTitle": "1st Round",
                "playoffRound": 1,
                "topSeedRank": 1,
                "topSeedRankAbbrev": "D1",
                "topSeedWins": 3,
                "bottomSeedRank": 4,
                "bottomSeedRankAbbrev": "WC1",
                "bottomSeedWins": 2,
                "topSeedTeam":    {"abbrev":"BUF","name":{"default":"Buffalo Sabres"}},
                "bottomSeedTeam": {"abbrev":"BOS","name":{"default":"Boston Bruins"}}
            },
            {
                "seriesLetter": "B",
                "seriesAbbrev": "R1",
                "seriesTitle": "1st Round",
                "playoffRound": 1,
                "topSeedRank": 2,
                "topSeedRankAbbrev": "D2",
                "topSeedWins": 4,
                "bottomSeedRank": 3,
                "bottomSeedRankAbbrev": "WC2",
                "bottomSeedWins": 1,
                "topSeedTeam":    {"abbrev":"TBL","name":{"default":"Tampa Bay Lightning"}},
                "bottomSeedTeam": {"abbrev":"MTL","name":{"default":"Montreal Canadiens"}}
            }
        ]
    }"#).unwrap();
    let bracket = parse_playoff_bracket(&raw);

    // Bracket must NOT be empty — the regression that caused the TUI to
    // show "Playoffs not yet active for this season" during round 1.
    assert!(!bracket.is_empty(),
        "current-shape bracket with 2 series must not be empty");
    assert_eq!(bracket.rounds.len(), 1, "both series belong to round 1");
    assert_eq!(bracket.rounds[0].round_number, 1);
    assert_eq!(bracket.rounds[0].label, "1st Round",
        "round label should come from seriesTitle when present");
    assert_eq!(bracket.rounds[0].series.len(), 2);

    // Wins must be read from the series-level fields, not the team
    // objects (which no longer carry them in the current API).
    let a = &bracket.rounds[0].series[0];
    assert_eq!(a.top_seed_abbrev, "BUF");
    assert_eq!(a.top_seed_wins, 3,
        "topSeedWins lives at series level in the current API");
    assert_eq!(a.bottom_seed_abbrev, "BOS");
    assert_eq!(a.bottom_seed_wins, 2);
    // Rank: prefer the abbreviated form when present.
    assert_eq!(a.top_seed_rank.as_deref(), Some("D1"));
    assert_eq!(a.bottom_seed_rank.as_deref(), Some("WC1"));

    // Series B is a 4-1 sweep — `winner_abbrev` should infer from the
    // 4-win threshold even without an explicit `winningTeam` field.
    let b = bracket.find_series("B").expect("series B in fixture");
    assert_eq!(b.top_seed_wins, 4);
    assert_eq!(b.winner_abbrev.as_deref(), Some("TBL"),
        "4 top-seed wins should infer TBL as winner");
}

#[test]
fn l0_parse_playoff_bracket_groups_flat_series_by_round() {
    // Multi-round flat series — verify bucketing.
    use icelines_fetch::nhl_api::parse_playoff_bracket;
    let raw: serde_json::Value = serde_json::from_str(r#"{
        "series": [
            {"seriesLetter":"A", "playoffRound": 1, "seriesTitle":"1st Round",
             "topSeedWins": 4, "bottomSeedWins": 0,
             "topSeedTeam":{"abbrev":"BUF","name":{"default":"BUF"}},
             "bottomSeedTeam":{"abbrev":"BOS","name":{"default":"BOS"}}},
            {"seriesLetter":"I", "playoffRound": 2, "seriesTitle":"2nd Round",
             "topSeedWins": 1, "bottomSeedWins": 0,
             "topSeedTeam":{"abbrev":"BUF","name":{"default":"BUF"}},
             "bottomSeedTeam":{"abbrev":"NYR","name":{"default":"NYR"}}}
        ]
    }"#).unwrap();
    let bracket = parse_playoff_bracket(&raw);
    assert_eq!(bracket.rounds.len(), 2);
    assert_eq!(bracket.rounds[0].round_number, 1);
    assert_eq!(bracket.rounds[1].round_number, 2);
    assert_eq!(bracket.rounds[0].series.len(), 1);
    assert_eq!(bracket.rounds[1].series.len(), 1);
}

#[test]
fn l0_parse_playoff_bracket_truly_empty_returns_empty() {
    // Both shapes absent → off-season message remains correct.
    use icelines_fetch::nhl_api::parse_playoff_bracket;
    let raw: serde_json::Value = serde_json::from_str(r#"{"bracketTitle":"None"}"#).unwrap();
    let bracket = parse_playoff_bracket(&raw);
    assert!(bracket.is_empty(),
        "neither playoffRounds nor series → empty bracket");
}

#[test]
fn l0_playoff_series_summary_phrasing() {
    use icelines_fetch::nhl_api::parse_playoff_bracket;
    let raw: serde_json::Value = serde_json::from_str(FIXTURE_PLAYOFF_BRACKET).unwrap();
    let bracket = parse_playoff_bracket(&raw);

    let a = bracket.find_series("A").unwrap();
    assert!(a.summary().contains("FLA wins"), "completed series → 'wins', got: {}", a.summary());

    let i = bracket.find_series("I").unwrap();
    let s = i.summary();
    // 2-1 in progress with FLA on top → "FLA leads 2-1"
    assert!(s.contains("leads") && s.contains("2-1"),
        "in-progress lead phrasing missing, got: {s}");
}

#[test]
fn l0_playoff_bracket_find_series_by_letter() {
    use icelines_fetch::nhl_api::parse_playoff_bracket;
    let raw: serde_json::Value = serde_json::from_str(FIXTURE_PLAYOFF_BRACKET).unwrap();
    let bracket = parse_playoff_bracket(&raw);

    assert!(bracket.find_series("A").is_some(), "A is in fixture");
    assert!(bracket.find_series("I").is_some(), "I is in fixture (round 2)");
    assert!(bracket.find_series("Z").is_none(), "Z is not in fixture");
}

#[tokio::test]
async fn l1_mock_fetch_playoff_bracket_parses_two_rounds() {
    let server = MockServer::start();
    let _mock = server.mock(|when, then| {
        when.method(GET).path_contains("/playoff-bracket/2026");
        then.status(200)
            .header("content-type", "application/json")
            .body(FIXTURE_PLAYOFF_BRACKET);
    });

    let client = NhlApiClient::new("http://unused.local", server.url(""));
    let bracket = client.fetch_playoff_bracket(2026).await
        .expect("mock fetch_playoff_bracket must succeed");

    assert_eq!(bracket.rounds.len(), 2, "fixture has 2 rounds");
    assert_eq!(bracket.rounds[0].series.len(), 3, "first round has 3 series");
    assert_eq!(bracket.current_round, Some(2));

    // Spot-check that conference grouping flowed through
    let east_series_count = bracket.rounds.iter()
        .flat_map(|r| r.series.iter())
        .filter(|s| s.conference.as_deref() == Some("Eastern"))
        .count();
    assert_eq!(east_series_count, 3, "two east first-round + one east second-round");
}

#[tokio::test]
async fn l1_mock_fetch_playoff_bracket_404_returns_error() {
    let server = MockServer::start();
    let _mock = server.mock(|when, then| {
        when.method(GET).path_contains("/playoff-bracket/");
        then.status(404).body("Not Found");
    });

    let client = NhlApiClient::new("http://unused.local", server.url(""));
    let result = client.fetch_playoff_bracket(2026).await;
    assert!(result.is_err(), "404 must surface as Err");
}

#[tokio::test]
async fn l1_mock_fetch_schedule_404_returns_error() {
    let server = MockServer::start();
    let _mock = server.mock(|when, then| {
        when.method(GET);
        then.status(404).body("Not Found");
    });

    let client = NhlApiClient::new("http://unused.local", server.url(""));
    let result = client.fetch_schedule_for_date("2026-04-27").await;
    assert!(result.is_err(), "404 must surface as Err, not empty Vec");
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
