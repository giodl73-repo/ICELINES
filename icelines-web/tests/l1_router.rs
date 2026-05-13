//! L1 — exercise the King.1.1 router via `tower::ServiceExt::oneshot`.
//!
//! No socket binding yet; that lands in King.1.5 once the
//! `Commands::Serve` driver exists. King.1.1's router only mounts `/`
//! plus the (future) extension points, so this is the right scope.
//!
//! Per the spec's testing strategy, L1 tests live under
//! `icelines-web/tests/`. Each file is its own binary; share fixtures
//! via `tests/common/mod.rs` once King.2 introduces them.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::response::Response;
use icelines_core::career_history::{CareerGameType, CareerHistory, CareerStint, LeagueAbbrev};
use icelines_core::freshness::{FetchSource, Freshness, Ttl};
use icelines_core::identity::PlayerId;
use icelines_core::season_stats::SeasonType;
use icelines_core::view_model::{DepthGoalieSlot, DepthLine, DepthPair, DepthPlayerSlot};
use icelines_core::{
    fixtures, CareerSortKey, CareerView, CompareView, DepthLeagueView, DepthTeamStrengthRow,
    MetricCell, MetricValue, PlayerCardView, Season, SimilarPlayersView, TeamAbbr, TeamDepthView,
    ViewContext, ViewWindow,
};
use icelines_fetch::career_landing::CareerHistoryStore;
use icelines_fetch::datastore::DataStore;
use icelines_fetch::fantasy_db::FantasyDb;
use icelines_fetch::manifest::{DataKey, DataKind, ManifestEntry};
use icelines_fetch::snapshot::{SnapshotStore, SnapshotTier};
use icelines_fetch::stats_loader::{load_into_repo, load_player_career_into_repo};
use icelines_web::{router, WebConfig, WebState};
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::Mutex;
use tower::util::ServiceExt;

async fn home_env_lock() -> tokio::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().await
}

struct HomeEnvFixture {
    _dir: tempfile::TempDir,
    prev_userprofile: Option<std::ffi::OsString>,
    prev_home: Option<std::ffi::OsString>,
}

impl HomeEnvFixture {
    fn new() -> Self {
        let dir = tempfile::TempDir::new().expect("temp home");
        let prev_userprofile = std::env::var_os("USERPROFILE");
        let prev_home = std::env::var_os("HOME");
        std::env::set_var("USERPROFILE", dir.path());
        std::env::set_var("HOME", dir.path());
        Self {
            _dir: dir,
            prev_userprofile,
            prev_home,
        }
    }
}

impl Drop for HomeEnvFixture {
    fn drop(&mut self) {
        match &self.prev_userprofile {
            Some(p) => std::env::set_var("USERPROFILE", p),
            None => std::env::remove_var("USERPROFILE"),
        }
        match &self.prev_home {
            Some(p) => std::env::set_var("HOME", p),
            None => std::env::remove_var("HOME"),
        }
    }
}

fn seed_fantasy_league(name: &str, user_roster: &[&str], rival_roster: &[&str]) {
    let db = FantasyDb::open().expect("open fantasy db");
    let league_id = db
        .create_league(name, "yahoo-standard")
        .expect("create fantasy league");
    db.set_active_league(name).expect("set active league");
    let mine = db
        .create_team(&league_id, "My Team", "Me")
        .expect("create user team");
    let rival = if rival_roster.is_empty() {
        None
    } else {
        Some(
            db.create_team(&league_id, "Rival Team", "Them")
                .expect("create rival team"),
        )
    };
    db.set_user_team(&league_id, "My Team")
        .expect("set user team");
    for player in user_roster {
        db.add_player(&mine, player).expect("add user player");
    }
    if let Some(rival) = rival {
        for player in rival_roster {
            db.add_player(&rival, player).expect("add rival player");
        }
    }
}

fn repo_with_mcdavid() -> icelines_core::stats_repository::StatsRepository {
    let identity = fixtures::identity(8478402)
        .name("Connor McDavid", "connor_mcdavid")
        .build();
    let stats = fixtures::stats(8478402, 20252026, "EDM").build();
    fixtures::test_repo_with(identity, stats)
}

fn repo_with_mcdavid_and_bench_forward() -> icelines_core::stats_repository::StatsRepository {
    let mut repo = repo_with_mcdavid();
    repo.upsert_identity(
        fixtures::identity(1)
            .name("Bench Forward", "bench_forward")
            .build(),
    )
    .expect("upsert bench identity");
    repo.upsert_stats(
        fixtures::stats(1, 20252026, "SEA")
            .position(icelines_core::model::Position::Goalie)
            .build(),
    )
    .expect("upsert bench stats");
    repo
}

async fn response_json(response: Response, limit: usize) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), limit)
        .await
        .expect("body fits");
    serde_json::from_slice(&bytes).expect("response should be valid json")
}

fn assert_json_object<'a>(json: &'a Value, ctx: &str) -> &'a Map<String, Value> {
    json.as_object()
        .unwrap_or_else(|| panic!("{ctx} should be a JSON object"))
}

fn assert_data_meta_envelope<'a>(json: &'a Value, route: &str) -> &'a Map<String, Value> {
    let obj = assert_json_object(json, "data/meta envelope");
    let keys: BTreeSet<_> = obj.keys().map(String::as_str).collect();
    let want: BTreeSet<_> = ["data", "meta", "route", "schema_version"]
        .iter()
        .copied()
        .collect();
    assert_eq!(keys, want, "envelope diverged: {keys:?}");
    assert_eq!(obj["schema_version"], serde_json::json!(1));
    assert_eq!(obj["route"], serde_json::json!(route));
    obj
}

fn assert_shared_error_envelope<'a>(json: &'a Value, route: &str) -> &'a Map<String, Value> {
    let obj = assert_json_object(json, "shared error envelope");
    let keys: BTreeSet<_> = obj.keys().map(String::as_str).collect();
    let want: BTreeSet<_> = ["data", "error", "meta", "route", "schema_version"]
        .iter()
        .copied()
        .collect();
    assert_eq!(keys, want, "error envelope diverged: {keys:?}");
    assert_eq!(obj["schema_version"], serde_json::json!(1));
    assert_eq!(obj["route"], serde_json::json!(route));
    assert!(obj["error"].is_string());
    obj
}

#[derive(Debug, PartialEq, Eq)]
struct TeamDepthSkaterSnapshot {
    nhl_id: u32,
    name: String,
    position: String,
    games: u32,
    goals: u32,
    assists: u32,
    points: u32,
}

#[derive(Debug, PartialEq, Eq)]
struct TeamDepthGoalieSnapshot {
    nhl_id: u32,
    name: String,
    games: u32,
    wins: u32,
    losses: u32,
    shutouts: u32,
}

#[derive(Debug, PartialEq)]
struct DepthLeagueRowSnapshot {
    team: String,
    c_score: String,
    lw_score: String,
    rw_score: String,
    d_score: String,
    total: String,
    c_top: String,
    lw_top: String,
    rw_top: String,
    d_top: String,
}

#[derive(Debug, PartialEq, Eq)]
struct PlayerJsonSnapshot {
    nhl_id: u32,
    full_name: String,
    position: String,
    team: String,
    active: PlayerActiveSnapshot,
    career: Vec<PlayerCareerSnapshot>,
}

#[derive(Debug, PartialEq, Eq)]
struct PlayerActiveSnapshot {
    season: String,
    season_type: String,
    games: u32,
    goals: u32,
    assists: u32,
    points: u32,
    points_per_game: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
struct PlayerCareerSnapshot {
    season: String,
    season_type: String,
    team: String,
    games: u32,
    goals: u32,
    assists: u32,
    points: u32,
    points_per_game: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
struct CompareCardSnapshot {
    nhl_id: u32,
    full_name: String,
    position: String,
    team: String,
    games: u32,
    goals: u32,
    assists: u32,
    points: u32,
    points_per_game: String,
}

#[derive(Debug, PartialEq, Eq)]
struct CareerRowSnapshot {
    rank: u64,
    player_id: u32,
    name: String,
    team: String,
    games: u32,
    goals: Option<u32>,
    assists: Option<u32>,
    points: Option<u32>,
    points_per_game: Option<String>,
}

fn depth_league_row_snapshots(rows: &[DepthTeamStrengthRow]) -> Vec<DepthLeagueRowSnapshot> {
    rows.iter()
        .map(|row| DepthLeagueRowSnapshot {
            team: row.team.0.clone(),
            c_score: normalized_score(row.c_score),
            lw_score: normalized_score(row.lw_score),
            rw_score: normalized_score(row.rw_score),
            d_score: normalized_score(row.d_score),
            total: normalized_score(row.total),
            c_top: row.c_top.clone(),
            lw_top: row.lw_top.clone(),
            rw_top: row.rw_top.clone(),
            d_top: row.d_top.clone(),
        })
        .collect()
}

fn json_depth_league_row_snapshots(json: &Value) -> Vec<DepthLeagueRowSnapshot> {
    json["data"]
        .as_array()
        .expect("depth data array")
        .iter()
        .map(|row| DepthLeagueRowSnapshot {
            team: json_str(row, "team"),
            c_score: normalized_score(json_f64(row, "c_score")),
            lw_score: normalized_score(json_f64(row, "lw_score")),
            rw_score: normalized_score(json_f64(row, "rw_score")),
            d_score: normalized_score(json_f64(row, "d_score")),
            total: normalized_score(json_f64(row, "total")),
            c_top: json_str(row, "c_top"),
            lw_top: json_str(row, "lw_top"),
            rw_top: json_str(row, "rw_top"),
            d_top: json_str(row, "d_top"),
        })
        .collect()
}

fn normalized_score(value: f64) -> String {
    let value = if value.abs() < 0.000_000_5 {
        0.0
    } else {
        value
    };
    format!("{value:.6}")
}

fn player_view_snapshot(view: &PlayerCardView, season: Season) -> PlayerJsonSnapshot {
    let active_metrics = view
        .active
        .as_ref()
        .map(|active| active.metrics.as_slice())
        .unwrap_or(&[]);
    let (position, team) = view
        .active
        .as_ref()
        .map(|active| {
            (
                active.position.abbreviation().to_owned(),
                active.team_display.clone(),
            )
        })
        .unwrap_or_else(|| (String::new(), String::new()));

    PlayerJsonSnapshot {
        nhl_id: view.player_id.0,
        full_name: view.display_name.clone(),
        position,
        team,
        active: PlayerActiveSnapshot {
            season: season.0.to_string(),
            season_type: "regular".to_owned(),
            games: metric_u32(active_metrics, "gp"),
            goals: metric_u32(active_metrics, "goals"),
            assists: metric_u32(active_metrics, "assists"),
            points: metric_u32(active_metrics, "points"),
            points_per_game: metric_f64(active_metrics, "points_per_game").map(normalized_score),
        },
        career: view
            .career
            .iter()
            .map(|row| PlayerCareerSnapshot {
                season: pretty_season(row.season),
                season_type: row.season_type.label().to_owned(),
                team: row.team.0.clone(),
                games: metric_u32(&row.metrics, "gp"),
                goals: metric_u32(&row.metrics, "goals"),
                assists: metric_u32(&row.metrics, "assists"),
                points: metric_u32(&row.metrics, "points"),
                points_per_game: metric_f64(&row.metrics, "points_per_game").map(normalized_score),
            })
            .collect(),
    }
}

fn json_player_snapshot(json: &Value) -> PlayerJsonSnapshot {
    let data = &json["data"];
    let active = &data["active_season_stats"];
    PlayerJsonSnapshot {
        nhl_id: json_u32(data, "nhl_id"),
        full_name: json_str(data, "full_name"),
        position: json_str(data, "position"),
        team: json_str(data, "team"),
        active: PlayerActiveSnapshot {
            season: json_str(active, "season"),
            season_type: json_str(active, "season_type"),
            games: json_u32(active, "games"),
            goals: json_u32(active, "goals"),
            assists: json_u32(active, "assists"),
            points: json_u32(active, "points"),
            points_per_game: optional_json_f64(active, "points_per_game").map(normalized_score),
        },
        career: data["career"]
            .as_array()
            .expect("player career data array")
            .iter()
            .map(|row| PlayerCareerSnapshot {
                season: json_str(row, "season"),
                season_type: json_str(row, "season_type"),
                team: json_str(row, "team"),
                games: json_u32(row, "games"),
                goals: json_u32(row, "goals"),
                assists: json_u32(row, "assists"),
                points: json_u32(row, "points"),
                points_per_game: optional_json_f64(row, "points_per_game").map(normalized_score),
            })
            .collect(),
    }
}

fn pretty_season(season: Season) -> String {
    let raw = season.0;
    if raw < 10_000_000 {
        return raw.to_string();
    }
    let yyyy_start = raw / 10_000;
    let yy_end = raw % 100;
    format!("{yyyy_start:04}-{yy_end:02}")
}

fn compare_card_snapshot_from_view(view: &PlayerCardView) -> CompareCardSnapshot {
    let active_metrics = view
        .active
        .as_ref()
        .map(|active| active.metrics.as_slice())
        .unwrap_or(&[]);
    let (position, team) = view
        .active
        .as_ref()
        .map(|active| {
            (
                active.position.abbreviation().to_owned(),
                active.team_display.clone(),
            )
        })
        .unwrap_or_else(|| (String::new(), String::new()));

    CompareCardSnapshot {
        nhl_id: view.player_id.0,
        full_name: view.display_name.clone(),
        position,
        team,
        games: metric_u32(active_metrics, "gp"),
        goals: metric_u32(active_metrics, "goals"),
        assists: metric_u32(active_metrics, "assists"),
        points: metric_u32(active_metrics, "points"),
        points_per_game: metric_f64(active_metrics, "points_per_game")
            .map(|ppg| format!("{ppg:.2}"))
            .unwrap_or_default(),
    }
}

fn json_compare_card_snapshot(row: &Value) -> CompareCardSnapshot {
    CompareCardSnapshot {
        nhl_id: json_u32(row, "nhl_id"),
        full_name: json_str(row, "full_name"),
        position: json_str(row, "position"),
        team: json_str(row, "team"),
        games: json_u32(row, "gp"),
        goals: json_u32(row, "goals"),
        assists: json_u32(row, "assists"),
        points: json_u32(row, "points"),
        points_per_game: json_str(row, "ppg_str"),
    }
}

fn career_row_snapshots(rows: &[icelines_core::CareerRow]) -> Vec<CareerRowSnapshot> {
    rows.iter()
        .map(|row| CareerRowSnapshot {
            rank: row.rank as u64,
            player_id: row.player_id,
            name: row.name.clone(),
            team: row.team.clone(),
            games: row.gp,
            goals: row.goals,
            assists: row.assists,
            points: row.points,
            points_per_game: row.points_per_game.map(normalized_score),
        })
        .collect()
}

fn json_career_row_snapshots(json: &Value) -> Vec<CareerRowSnapshot> {
    json["data"]
        .as_array()
        .expect("career data array")
        .iter()
        .map(|row| CareerRowSnapshot {
            rank: row["rank"]
                .as_u64()
                .unwrap_or_else(|| panic!("rank should be a JSON number in row {row}")),
            player_id: json_u32(row, "player_id"),
            name: json_str(row, "name"),
            team: json_str(row, "team"),
            games: json_u32(row, "gp"),
            goals: optional_json_u32(row, "goals"),
            assists: optional_json_u32(row, "assists"),
            points: optional_json_u32(row, "points"),
            points_per_game: optional_json_f64(row, "points_per_game").map(normalized_score),
        })
        .collect()
}

fn career_history(player_id: u32, stints: Vec<CareerStint>) -> CareerHistory {
    CareerHistory { player_id, stints }
}

fn career_stint(
    season: u32,
    league: &str,
    team: &str,
    gp: u32,
    goals: u32,
    assists: u32,
) -> CareerStint {
    CareerStint {
        season: Season(season),
        league: LeagueAbbrev::new(league),
        team: team.to_owned(),
        game_type: CareerGameType::Regular,
        sequence: 0,
        gp,
        goals: Some(goals),
        assists: Some(assists),
        points: Some(goals + assists),
        pim: None,
        plus_minus: None,
        power_play_goals: None,
        power_play_points: None,
        shorthanded_goals: None,
        shorthanded_points: None,
        game_winning_goals: None,
        ot_goals: None,
        shots: None,
        shooting_pct: None,
        avg_toi_sec: None,
        faceoff_win_pct: None,
        games_started: None,
        wins: None,
        losses: None,
        ot_losses: None,
        goals_against: None,
        goals_against_avg: None,
        save_pct: None,
        shots_against: None,
        shutouts: None,
        time_on_ice_sec: None,
    }
}

fn team_depth_skater_snapshots(view: &TeamDepthView) -> Vec<TeamDepthSkaterSnapshot> {
    let mut rows: Vec<_> = team_depth_skater_slots(view)
        .into_iter()
        .map(|slot| TeamDepthSkaterSnapshot {
            nhl_id: slot.player_id.0,
            name: slot.display_name.clone(),
            position: slot.position.abbreviation().to_owned(),
            games: metric_u32(&slot.metrics, "gp"),
            goals: metric_u32(&slot.metrics, "goals"),
            assists: metric_u32(&slot.metrics, "assists"),
            points: metric_u32(&slot.metrics, "points"),
        })
        .collect();
    rows.sort_by(|a, b| {
        b.points
            .cmp(&a.points)
            .then(b.goals.cmp(&a.goals))
            .then(a.name.cmp(&b.name))
    });
    rows
}

fn team_depth_skater_slots(view: &TeamDepthView) -> Vec<&DepthPlayerSlot> {
    let mut slots = Vec::new();
    for line in &view.forward_lines {
        push_depth_line_slots(&mut slots, line);
    }
    for pair in &view.defense_pairs {
        push_depth_pair_slots(&mut slots, pair);
    }
    slots.extend(view.extras.iter());
    slots
}

fn push_depth_line_slots<'a>(out: &mut Vec<&'a DepthPlayerSlot>, line: &'a DepthLine) {
    out.extend(
        [
            line.left.as_ref(),
            line.center.as_ref(),
            line.right.as_ref(),
        ]
        .into_iter()
        .flatten(),
    );
}

fn push_depth_pair_slots<'a>(out: &mut Vec<&'a DepthPlayerSlot>, pair: &'a DepthPair) {
    out.extend(
        [pair.left.as_ref(), pair.right.as_ref()]
            .into_iter()
            .flatten(),
    );
}

fn team_depth_goalie_snapshots(goalies: &[DepthGoalieSlot]) -> Vec<TeamDepthGoalieSnapshot> {
    let mut rows: Vec<_> = goalies
        .iter()
        .map(|slot| TeamDepthGoalieSnapshot {
            nhl_id: slot.player_id.0,
            name: slot.display_name.clone(),
            games: metric_u32(&slot.metrics, "gp"),
            wins: metric_u32(&slot.metrics, "wins"),
            losses: metric_u32(&slot.metrics, "losses"),
            shutouts: metric_u32(&slot.metrics, "shutouts"),
        })
        .collect();
    rows.sort_by(|a, b| b.wins.cmp(&a.wins).then(a.name.cmp(&b.name)));
    rows
}

fn metric_u32(metrics: &[MetricCell], key: &str) -> u32 {
    metrics
        .iter()
        .find(|metric| metric.key.0 == key)
        .and_then(|metric| match metric.value {
            MetricValue::Integer(value) => u32::try_from(value).ok(),
            _ => None,
        })
        .unwrap_or(0)
}

fn metric_f64(metrics: &[MetricCell], key: &str) -> Option<f64> {
    metrics
        .iter()
        .find(|metric| metric.key.0 == key)
        .and_then(|metric| match metric.value {
            MetricValue::Decimal(value) => Some(value),
            _ => None,
        })
}

fn json_team_skater_snapshots(json: &Value) -> Vec<TeamDepthSkaterSnapshot> {
    json["data"]["skaters"]
        .as_array()
        .expect("team skaters data array")
        .iter()
        .map(|row| TeamDepthSkaterSnapshot {
            nhl_id: json_u32(row, "nhl_id"),
            name: json_str(row, "name"),
            position: json_str(row, "position"),
            games: json_u32(row, "games"),
            goals: json_u32(row, "goals"),
            assists: json_u32(row, "assists"),
            points: json_u32(row, "points"),
        })
        .collect()
}

fn json_team_goalie_snapshots(json: &Value) -> Vec<TeamDepthGoalieSnapshot> {
    json["data"]["goalies"]
        .as_array()
        .expect("team goalies data array")
        .iter()
        .map(|row| TeamDepthGoalieSnapshot {
            nhl_id: json_u32(row, "nhl_id"),
            name: json_str(row, "name"),
            games: json_u32(row, "games"),
            wins: json_u32(row, "wins"),
            losses: json_u32(row, "losses"),
            shutouts: json_u32(row, "shutouts"),
        })
        .collect()
}

fn json_u32(row: &Value, key: &str) -> u32 {
    row[key]
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or_else(|| panic!("{key} should be a u32 JSON number in row {row}"))
}

fn optional_json_u32(row: &Value, key: &str) -> Option<u32> {
    if row[key].is_null() {
        None
    } else {
        Some(json_u32(row, key))
    }
}

fn json_f64(row: &Value, key: &str) -> f64 {
    row[key]
        .as_f64()
        .unwrap_or_else(|| panic!("{key} should be a JSON number in row {row}"))
}

fn optional_json_f64(row: &Value, key: &str) -> Option<f64> {
    if row[key].is_null() {
        None
    } else {
        Some(json_f64(row, key))
    }
}

fn json_str(row: &Value, key: &str) -> String {
    row[key]
        .as_str()
        .unwrap_or_else(|| panic!("{key} should be a string in row {row}"))
        .to_owned()
}

/// l1_get_root_returns_200_html
/// — placeholder home page handler smoke. Spec promises the bare route
///   returns the full HTML page (not a fragment); fragments are routed
///   under `?partial=*` (King.2). Today we just verify 200 + HTML
///   content-type so the contract is locked from day one.
#[tokio::test]
async fn l1_get_root_returns_200_html() {
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/")
                .body(Body::empty())
                .expect("request builder should succeed"),
        )
        .await
        .expect("oneshot dispatch should not fail");

    assert_eq!(response.status(), StatusCode::OK);

    let content_type = response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .expect("home page should set Content-Type")
        .to_str()
        .expect("content-type is ascii");
    assert!(
        content_type.starts_with("text/html"),
        "home page should be HTML, got Content-Type: {content_type}"
    );
    // King.1.x patch (broadcast review): lock the charset so a future
    // template refactor doesn't accidentally serve raw bytes that the
    // browser interprets in the wrong encoding (UTF-8 is mandatory
    // for the multi-language player names like "Slafkovský").
    assert!(
        content_type.contains("charset=utf-8") || content_type.contains("charset=UTF-8"),
        "home page Content-Type must declare charset=utf-8, got: {content_type}"
    );

    let body_bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("body should fit in 64 KiB");
    let body = std::str::from_utf8(&body_bytes).expect("HTML response is utf-8");
    assert!(
        body.contains("<!doctype html>"),
        "home page should be a full HTML document; got body starting with: {}",
        &body[..body.len().min(80)]
    );
    // King.1.4 — content from the askama template should appear.
    // Originally checked for "Welcome"; the home page was rebuilt
    // with top-3 preview sections in King.8.x. The IceLines title
    // and the "Top scorers" / "Top goalies" headings are stable.
    assert!(
        body.contains("IceLines") && body.contains("Top scorers"),
        "home page should render the askama template content"
    );
    assert!(
        body.contains("/fantasy") && body.contains("roster gaps and league simulation"),
        "home page should advertise the live fantasy read/product surface"
    );
    assert!(
        !body.contains("Fantasy</a> <small>(soon"),
        "home page must not describe the mounted fantasy surface as soon/deferred"
    );
}

/// l1_html_each_route_has_active_season_header
/// — King.1.4 fence (broadcast finding, advanced from King.6 → King.1.4):
///   every HTML page must render the active-(season, season_type)
///   sticky header so time-travel via PATCH is never silent.
///
/// Today only `/` is mounted. Each future sub-phase adds its routes
/// to the route list below — King.2 adds `/leaders`, King.3 adds
/// `/player/:id`, etc. The fence catches any route that forgets to
/// thread `active_label` into its template.
#[tokio::test]
async fn l1_html_each_route_has_active_season_header() {
    let app = router(WebState::new());

    // Default WebConfig::default() uses CURRENT_SEASON_STR + "regular"
    // → label "25-26 · Regular". The fence checks for the structural
    // marker (the season-header CSS class) plus the label substring.
    let html_routes: &[&str] = &[
        "/",
        "/dashboard",
        "/leaders",
        "/compare",
        "/goalies",
        "/depth",
        "/poach",
        "/reports/poach",
        "/reports/weekly",
        "/favorites",
        "/watchlist",
        "/scores?date=2014-10-08",
        "/schedule?date=2014-10-08",
        "/playoffs?season=19931994",
        "/game/2025020342",
        "/transactions",
        "/docs",
        "/fantasy",
    ];

    for route in html_routes {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(*route)
                    .body(Body::empty())
                    .expect("request builder ok"),
            )
            .await
            .expect("dispatch ok");

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "{route} should return 200"
        );

        let bytes = axum::body::to_bytes(response.into_body(), 2 * 1024 * 1024)
            .await
            .expect("body fits");
        let body = std::str::from_utf8(&bytes).expect("html is utf-8");

        assert!(
            body.contains("season-header"),
            "{route} must include the .season-header element \
             (broadcast a11y/UX contract)"
        );
        // CURRENT_SEASON_STR is "20252026" → label "25-26 · Regular"
        assert!(
            body.contains("25-26 · Regular"),
            "{route} must render the active-season label '25-26 · Regular' \
             (got body without it — make sure the route's template extends \
             base.html and the handler passes active_label)"
        );
    }
}

#[tokio::test]
async fn l1_dashboard_shell_renders_no_js_regions() {
    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/dashboard?workspace=/poach?availability=imported-available")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let body = std::str::from_utf8(&bytes).expect("html is utf-8");

    assert!(body.contains("aria-label=\"Scores ribbon\""));
    assert!(body.contains("aria-label=\"Favorites and watchlist\""));
    assert!(body.contains("aria-label=\"Workspace\""));
    assert!(body.contains("data-workspace-url=\"/poach?availability=imported-available\""));
    assert!(body.contains("aria-label=\"Schedule\""));
    assert!(body.contains("aria-label=\"Command palette\""));
    assert!(body.contains("href=\"/dashboard?workspace=%2Fleaders\""));
    assert!(body.contains("href=\"/dashboard?workspace=%2Fpoach\""));
    assert!(body.contains("href=\"/poach?availability=imported-available\""));
    assert!(body.contains("href=\"/schedule\""));
}

#[tokio::test]
async fn l1_dashboard_rejects_unsafe_workspace_paths() {
    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/dashboard?workspace=/favorites/add")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let body = std::str::from_utf8(&bytes).expect("html is utf-8");

    assert!(body.contains("data-workspace-url=\"/leaders\""));
    assert!(body.contains("href=\"/leaders\""));
    assert!(!body.contains("data-workspace-url=\"/favorites/add\""));
}

#[tokio::test]
async fn l1_fantasy_simulation_json_projects_seeded_league() {
    let _guard = home_env_lock().await;
    let _home = HomeEnvFixture::new();

    seed_fantasy_league(
        "Route Test League",
        &["connor_mcdavid"],
        &["nathan_mackinnon"],
    );

    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/fantasy/simulate")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let json = response_json(response, 256 * 1024).await;
    assert_eq!(json["league"], "Route Test League");
    assert_eq!(json["user_team"], "My Team");
    assert_eq!(json["scoring_scheme"], "yahoo-standard");
    assert_eq!(json["rows"].as_array().expect("rows array").len(), 2);
    assert!(json["warnings"]
        .as_array()
        .expect("warnings array")
        .iter()
        .any(|warning| warning
            .as_str()
            .is_some_and(|text| text.contains("schedule unavailable"))));
}

#[tokio::test]
async fn l1_fantasy_gaps_json_projects_seeded_league() {
    let _guard = home_env_lock().await;
    let _home = HomeEnvFixture::new();

    seed_fantasy_league("Gap Route League", &[], &[]);

    let app = router(WebState::with_repo(repo_with_mcdavid()));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/fantasy/gaps?category=points&top=1")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let json = response_json(response, 256 * 1024).await;
    assert_eq!(json["league"], "Gap Route League");
    assert_eq!(json["team"], "My Team");
    assert_eq!(json["scoring_scheme"], "yahoo-standard");
    assert_eq!(json["categories"], serde_json::json!(["points"]));

    let rows = json["rows"].as_array().expect("rows array");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["category"], "points");
    assert_eq!(rows[0]["best_available"]["display_name"], "Connor McDavid");
    assert!(
        rows[0]["weighted_gap_score"]
            .as_f64()
            .expect("weighted gap score number")
            > 0.0
    );
}

#[tokio::test]
async fn l1_fantasy_simulation_json_projects_add_scenario() {
    let _guard = home_env_lock().await;
    let _home = HomeEnvFixture::new();

    seed_fantasy_league("Scenario League", &[], &[]);
    let app = router(WebState::with_repo(repo_with_mcdavid()));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/fantasy/simulate?add_player=Connor%20McDavid")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let json = response_json(response, 256 * 1024).await;
    let scenarios = json["scenarios"].as_array().expect("scenarios array");
    assert_eq!(scenarios.len(), 1);
    assert_eq!(scenarios[0]["add_player"], "Connor McDavid");
    assert_eq!(scenarios[0]["action"], "improve");
    assert!(
        scenarios[0]["projected_score_delta"]
            .as_f64()
            .expect("scenario delta number")
            > 0.0
    );
}

#[tokio::test]
async fn l1_fantasy_simulation_json_projects_swap_scenario() {
    let _guard = home_env_lock().await;
    let _home = HomeEnvFixture::new();

    seed_fantasy_league("Swap Scenario League", &["bench_forward"], &[]);
    let app = router(WebState::with_repo(repo_with_mcdavid_and_bench_forward()));
    let response = app
        .oneshot(
            Request::builder()
                .uri(
                    "/api/v1/fantasy/simulate?add_player=Connor%20McDavid&drop_player=Bench%20Forward",
                )
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let json = response_json(response, 256 * 1024).await;
    let scenarios = json["scenarios"].as_array().expect("scenarios array");
    assert_eq!(scenarios.len(), 1);
    assert_eq!(scenarios[0]["add_player"], "Connor McDavid");
    assert_eq!(scenarios[0]["drop_player"], "Bench Forward");
    assert_eq!(scenarios[0]["action"], "improve");
    assert!(scenarios[0]["explanation"]
        .as_str()
        .expect("scenario explanation")
        .contains("Connor McDavid for Bench Forward"));
    assert!(
        scenarios[0]["projected_score_delta"]
            .as_f64()
            .expect("scenario delta number")
            > 0.0
    );
}

#[tokio::test]
async fn l1_fantasy_simulation_json_projects_drop_only_scenario() {
    let _guard = home_env_lock().await;
    let _home = HomeEnvFixture::new();

    seed_fantasy_league("Drop Only Scenario League", &["bench_forward"], &[]);
    let app = router(WebState::with_repo(repo_with_mcdavid_and_bench_forward()));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/fantasy/simulate?drop_player=Bench%20Forward")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let json = response_json(response, 256 * 1024).await;
    let scenarios = json["scenarios"].as_array().expect("scenarios array");
    assert_eq!(scenarios.len(), 1);
    assert_eq!(scenarios[0]["add_player"], serde_json::Value::Null);
    assert_eq!(scenarios[0]["drop_player"], "Bench Forward");
    assert_eq!(scenarios[0]["action"], "avoid");
    assert!(
        scenarios[0]["projected_score_delta"]
            .as_f64()
            .expect("scenario delta number")
            <= 0.0
    );
}

#[tokio::test]
async fn l1_fantasy_simulation_json_rejects_unknown_drop_player() {
    let _guard = home_env_lock().await;
    let _home = HomeEnvFixture::new();

    seed_fantasy_league("Bad Drop League", &["bench_forward"], &[]);

    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/fantasy/simulate?drop_player=Ghost%20Player")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let json = response_json(response, 256 * 1024).await;
    assert!(json["error"]
        .as_str()
        .expect("error message")
        .contains("was not found on the active fantasy roster"));
}

#[tokio::test]
async fn l1_fantasy_html_shows_unknown_drop_warning() {
    let _guard = home_env_lock().await;
    let _home = HomeEnvFixture::new();

    seed_fantasy_league("Bad Drop Html League", &["bench_forward"], &[]);

    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/fantasy?drop_player=Ghost%20Player")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let body = std::str::from_utf8(&bytes).expect("html is utf-8");
    assert!(body.contains("Fantasy simulation unavailable"));
    assert!(body.contains("was not found on the active fantasy roster"));
    assert!(body.contains("value=\"Ghost Player\""));
}

#[tokio::test]
async fn l1_fantasy_html_renders_add_scenario() {
    let _guard = home_env_lock().await;
    let _home = HomeEnvFixture::new();

    seed_fantasy_league("Fantasy Html League", &[], &[]);
    let app = router(WebState::with_repo(repo_with_mcdavid()));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/fantasy?add_player=Connor%20McDavid")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let body = std::str::from_utf8(&bytes).expect("html is utf-8");
    assert!(body.contains("League Simulation"));
    assert!(body.contains("Web add/drop scenario"));
    assert!(body.contains("value=\"Connor McDavid\""));
    assert!(body.contains("improve"));
}

/// l1_depth_route_returns_200_html
/// — Phase Lady Byng follow-up. The /depth route mirrors the TUI Depth
///   tab; this fence proves it boots and renders the askama template
///   without panicking. Asserts the route resolves to 200, returns HTML
///   with the expected charset, and contains the "Depth Rankings"
///   heading from depth.html.
#[tokio::test]
async fn l1_depth_route_returns_200_html() {
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/depth")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "/depth should return 200"
    );

    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let body = std::str::from_utf8(&bytes).expect("html is utf-8");
    assert!(
        body.contains("Depth Rankings"),
        "/depth page must render its h1 heading, got start:\n{}",
        &body[..body.len().min(120)]
    );
    // The nav bar should also include the new Depth link on every page.
    assert!(
        body.contains("href=\"/depth\""),
        "/depth must be linked in the global nav"
    );
}

#[tokio::test]
async fn l1_player_json_envelope_shape() {
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/player/8478402")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);

    let json = response_json(response, 512 * 1024).await;
    let obj = assert_data_meta_envelope(&json, "player");
    assert_eq!(json["meta"]["season_type"], "regular");
    assert_eq!(obj["data"]["nhl_id"], 8478402);
    assert_eq!(json["data"]["active_season_stats"]["season"], "20252026");
    assert!(json["data"]["career"].is_array());
}

#[tokio::test]
async fn l1_player_json_rows_match_player_card_view() {
    let season = Season(20242025);
    let season_type = SeasonType::Regular;
    let pid = PlayerId(8478402);
    let store = SnapshotStore::new(SnapshotStore::default_root());
    let mut load = load_into_repo(season, season_type, &store)
        .expect("bundled regular-season repo should load");
    load_player_career_into_repo(&mut load.repo, pid).expect("bundled career should load");
    let expected_view = PlayerCardView::from_repository(&load.repo, pid, season, season_type)
        .expect("player should exist in fixture repo");
    let expected = player_view_snapshot(&expected_view, season);
    assert!(
        !expected.career.is_empty(),
        "fixture should include player career rows for {pid:?}"
    );

    let state = WebState::with_repo_and_config(load.repo, WebConfig::new("20242025", "regular"));
    let app = router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/player/8478402")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");
    assert_eq!(response.status(), StatusCode::OK);

    let json = response_json(response, 1024 * 1024).await;
    assert_data_meta_envelope(&json, "player");
    assert_eq!(json["meta"]["season"], serde_json::json!("20242025"));
    assert_eq!(json["meta"]["season_type"], serde_json::json!("regular"));
    assert_eq!(
        json["meta"]["career_rows"],
        serde_json::json!(expected.career.len())
    );
    assert_eq!(json_player_snapshot(&json), expected);
}

#[tokio::test]
async fn l1_player_json_bad_active_season_uses_shared_envelope_shape() {
    let state = WebState::new();
    {
        let mut cfg = state.config.write().await;
        *cfg = icelines_web::WebConfig::new("not-a-season", "regular");
    }
    let app = router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/player/8478402")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let json = response_json(response, 512 * 1024).await;
    let obj = assert_shared_error_envelope(&json, "player");
    assert_eq!(obj["data"]["nhl_id"], 8478402);
    assert_eq!(obj["meta"]["career_rows"], 0);
    assert_eq!(obj["meta"]["pre_nhl_career_rows"], 0);
}

#[tokio::test]
async fn l1_player_json_missing_player_uses_shared_envelope_shape() {
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/player/999999999")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let json = response_json(response, 512 * 1024).await;
    let obj = assert_shared_error_envelope(&json, "player");
    assert_eq!(obj["data"]["nhl_id"], 999999999);
    assert_eq!(obj["meta"]["career_rows"], 0);
    assert_eq!(obj["meta"]["pre_nhl_career_rows"], 0);
}

#[tokio::test]
async fn l1_watchlist_route_returns_200_html() {
    let _guard = home_env_lock().await;
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/watchlist")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let body = std::str::from_utf8(&bytes).expect("html is utf-8");

    assert!(body.contains("Watchlist"));
    assert!(body.contains("href=\"/poach\""));
    assert!(body.contains("icelines tui poach"));
}

#[tokio::test]
async fn l1_watchlist_route_renders_watch_reason_metadata() {
    let _guard = home_env_lock().await;
    let dir = tempfile::TempDir::new().expect("temp home");
    let prev_userprofile = std::env::var_os("USERPROFILE");
    let prev_home = std::env::var_os("HOME");
    std::env::set_var("USERPROFILE", dir.path());
    std::env::set_var("HOME", dir.path());

    let db_dir = dir.path().join(".icelines");
    std::fs::create_dir_all(&db_dir).expect("create db dir");
    let conn = rusqlite::Connection::open(db_dir.join("icelines.db")).expect("open db");
    conn.execute_batch(
        "CREATE TABLE groups (
            name TEXT PRIMARY KEY,
            description TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
         );
         CREATE TABLE group_members (
            group_name TEXT NOT NULL,
            entity_ref TEXT NOT NULL,
            added_at TEXT NOT NULL,
            PRIMARY KEY (group_name, entity_ref)
         );
         CREATE TABLE watch_notes (
            entity_ref TEXT PRIMARY KEY,
            reason TEXT NOT NULL DEFAULT '',
            source TEXT NOT NULL DEFAULT '',
            updated_at TEXT NOT NULL
         );
         CREATE TABLE watch_rule_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            rule_id TEXT NOT NULL,
            entity_ref TEXT,
            message TEXT NOT NULL,
            fired_at TEXT NOT NULL
         );
         INSERT INTO groups VALUES ('Watchlist', '', datetime('now'));
         INSERT INTO group_members VALUES ('Watchlist', 'player:matthew knies', datetime('now'));
         INSERT INTO watch_notes VALUES (
            'player:matthew knies',
            'Poach score 72.0; confidence High; PP1 promotion',
            'tui-poach',
            datetime('now')
         );
         INSERT INTO watch_rule_events (rule_id, entity_ref, message, fired_at)
            VALUES (
                'alert-watched-available',
                'player:matthew knies',
                'Matthew Knies is available.',
                '2026-05-09T13:00:00Z'
            );",
    )
    .expect("seed watchlist db");

    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/watchlist")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    match prev_userprofile {
        Some(p) => std::env::set_var("USERPROFILE", p),
        None => std::env::remove_var("USERPROFILE"),
    }
    match prev_home {
        Some(p) => std::env::set_var("HOME", p),
        None => std::env::remove_var("HOME"),
    }

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let body = std::str::from_utf8(&bytes).expect("html is utf-8");

    assert!(body.contains("matthew knies"));
    assert!(body.contains("Poach score 72.0"));
    assert!(body.contains("Recent Alerts"));
    assert!(body.contains("alert-watched-available"));
    assert!(body.contains("Matthew Knies is available."));
}

#[tokio::test]
async fn l1_favorites_json_returns_group_members() {
    let _guard = home_env_lock().await;
    let dir = tempfile::TempDir::new().expect("temp home");
    let prev_userprofile = std::env::var_os("USERPROFILE");
    let prev_home = std::env::var_os("HOME");
    std::env::set_var("USERPROFILE", dir.path());
    std::env::set_var("HOME", dir.path());

    let db_dir = dir.path().join(".icelines");
    std::fs::create_dir_all(&db_dir).expect("create db dir");
    let conn = rusqlite::Connection::open(db_dir.join("icelines.db")).expect("open db");
    conn.execute_batch(
        "CREATE TABLE groups (
            name TEXT PRIMARY KEY,
            description TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
         );
         CREATE TABLE group_members (
            group_name TEXT NOT NULL,
            entity_ref TEXT NOT NULL,
            added_at TEXT NOT NULL,
            PRIMARY KEY (group_name, entity_ref)
         );
         INSERT INTO groups VALUES ('Favorites', '', datetime('now'));
         INSERT INTO group_members VALUES ('Favorites', 'player:connor mcdavid', datetime('now'));
         INSERT INTO group_members VALUES ('Favorites', 'team:EDM', datetime('now'));",
    )
    .expect("seed favorites db");

    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/favorites")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    match prev_userprofile {
        Some(p) => std::env::set_var("USERPROFILE", p),
        None => std::env::remove_var("USERPROFILE"),
    }
    match prev_home {
        Some(p) => std::env::set_var("HOME", p),
        None => std::env::remove_var("HOME"),
    }

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");

    assert_eq!(json["schema_version"], "favorites.v1");
    assert_eq!(json["route"], "favorites");
    assert_eq!(json["meta"]["group"], "Favorites");
    assert_eq!(json["meta"]["count"], 2);
    assert_eq!(json["meta"]["player_count"], 1);
    assert_eq!(json["meta"]["team_count"], 1);
    let player_row = json["data"]
        .as_array()
        .expect("data array")
        .iter()
        .find(|row| row["kind"] == "player" && row["key"] == "connor mcdavid")
        .expect("favorites player row");
    assert!(
        player_row
            .as_object()
            .expect("favorites row object")
            .contains_key("stat_line"),
        "favorites JSON rows should expose the shared FavoritesView stat_line slot"
    );
    assert!(player_row["stat_line"].is_null());
    assert!(json["data"]
        .as_array()
        .expect("data array")
        .iter()
        .any(|row| row["kind"] == "team" && row["key"] == "EDM"));
}

#[tokio::test]
async fn l1_watchlist_json_returns_watch_reason_metadata() {
    let _guard = home_env_lock().await;
    let dir = tempfile::TempDir::new().expect("temp home");
    let prev_userprofile = std::env::var_os("USERPROFILE");
    let prev_home = std::env::var_os("HOME");
    std::env::set_var("USERPROFILE", dir.path());
    std::env::set_var("HOME", dir.path());

    let db_dir = dir.path().join(".icelines");
    std::fs::create_dir_all(&db_dir).expect("create db dir");
    let conn = rusqlite::Connection::open(db_dir.join("icelines.db")).expect("open db");
    conn.execute_batch(
        "CREATE TABLE groups (
            name TEXT PRIMARY KEY,
            description TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
         );
         CREATE TABLE group_members (
            group_name TEXT NOT NULL,
            entity_ref TEXT NOT NULL,
            added_at TEXT NOT NULL,
            PRIMARY KEY (group_name, entity_ref)
         );
         CREATE TABLE watch_notes (
            entity_ref TEXT PRIMARY KEY,
            reason TEXT NOT NULL DEFAULT '',
            source TEXT NOT NULL DEFAULT '',
            updated_at TEXT NOT NULL
         );
         CREATE TABLE watch_rule_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            rule_id TEXT NOT NULL,
            entity_ref TEXT,
            message TEXT NOT NULL,
            fired_at TEXT NOT NULL
         );
         INSERT INTO groups VALUES ('Watchlist', '', datetime('now'));
         INSERT INTO group_members VALUES ('Watchlist', 'player:matthew knies', datetime('now'));
         INSERT INTO watch_notes VALUES (
            'player:matthew knies',
            'Poach score 72.0; confidence High; PP1 promotion',
            'tui-poach',
            '2026-05-09T12:00:00Z'
         );
         INSERT INTO watch_rule_events (rule_id, entity_ref, message, fired_at)
            VALUES (
                'alert-watched-available',
                'player:matthew knies',
                'Matthew Knies is available.',
                '2026-05-09T13:00:00Z'
            );",
    )
    .expect("seed watchlist db");

    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/watchlist")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    match prev_userprofile {
        Some(p) => std::env::set_var("USERPROFILE", p),
        None => std::env::remove_var("USERPROFILE"),
    }
    match prev_home {
        Some(p) => std::env::set_var("HOME", p),
        None => std::env::remove_var("HOME"),
    }

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let body: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");

    assert_eq!(body["schema_version"], "watchlist.v1");
    assert_eq!(body["route"], "watchlist");
    assert_eq!(body["meta"]["group"], "Watchlist");
    assert_eq!(body["meta"]["player_count"], 1);
    assert_eq!(body["data"][0]["kind"], "player");
    assert_eq!(body["data"][0]["key"], "matthew knies");
    assert_eq!(
        body["data"][0]["reason"],
        "Poach score 72.0; confidence High; PP1 promotion"
    );
    assert_eq!(body["data"][0]["source"], "tui-poach");
    assert_eq!(body["data"][0]["updated_at"], "2026-05-09T12:00:00Z");
    assert_eq!(body["alerts"][0]["rule_id"], "alert-watched-available");
    assert_eq!(body["alerts"][0]["entity_ref"], "player:matthew knies");
    assert_eq!(body["alerts"][0]["message"], "Matthew Knies is available.");
}

#[tokio::test]
async fn l1_poach_route_returns_200_html() {
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/poach?category=hits,blocks&availability=imported-available&top=5")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let body = std::str::from_utf8(&bytes).expect("html is utf-8");

    assert!(body.contains("Fantasy Poacher"));
    assert!(body.contains("href=\"/poach\""));
    assert!(body.contains("imported-available"));
    assert!(body.contains("Missing poacher source data"));
}

#[tokio::test]
async fn l1_poach_report_route_returns_report_html() {
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/reports/poach?category=hits&top=5")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let body = std::str::from_utf8(&bytes).expect("html is utf-8");

    assert!(body.contains("Fantasy Poacher Report"));
    assert!(body.contains("Top Adds"));
    assert!(body.contains("Source Omissions"));
    assert!(body.contains("href=\"/poach\""));
}

#[tokio::test]
async fn l1_weekly_report_route_returns_prep_sections() {
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/reports/weekly?league=Main%20League&category=hits,blocks&top=5")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let body = std::str::from_utf8(&bytes).expect("html is utf-8");

    assert!(body.contains("Weekly Fantasy Prep Report"));
    assert!(body.contains("Category Specialists"));
    assert!(body.contains("Deployment Risers"));
    assert!(body.contains("Risk Discounts"));
    assert!(body.contains("Watched Player Alerts"));
}

#[tokio::test]
async fn l1_poach_json_returns_view_model_contract() {
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/poach?category=hits,blocks&pos=LW&availability=imported-available&top=5")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");

    assert_eq!(json["scoring_scheme"], "yahoo-standard");
    assert_eq!(json["query"]["categories"][0], "hits");
    assert_eq!(json["query"]["positions"][0], "LeftWing");
    assert_eq!(json["query"]["availability_filter"], "imported_available");
    assert_eq!(json["empty_state"]["kind"], "missing_source");
}

#[tokio::test]
async fn l1_watch_rules_json_returns_shared_contract() {
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/watch-rules")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");

    assert_eq!(json["context"]["completeness"], "partial");
    assert_eq!(json["rules"][0]["id"], "category-hits-pace");
    assert_eq!(json["rules"][2]["id"], "deployment-promotion");
    assert_eq!(json["rules"][2]["unsupported_sources"][0], "shifts");
    assert_eq!(json["rules"][4]["unsupported_sources"][0], "fantasy_import");
}

#[tokio::test]
async fn l1_watch_rules_json_bad_active_season_returns_typed_error() {
    let state = WebState::new();
    {
        let mut cfg = state.config.write().await;
        *cfg = icelines_web::WebConfig::new("not-a-season", "regular");
    }
    let app = router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/watch-rules")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");
    let obj = json.as_object().expect("error response is object");
    let keys: std::collections::BTreeSet<_> = obj.keys().map(String::as_str).collect();
    let want: std::collections::BTreeSet<_> = ["error"].iter().copied().collect();
    assert_eq!(keys, want);
    assert!(json["error"].is_string());
}

#[tokio::test]
async fn l1_watch_rules_json_includes_persisted_rules() {
    let _guard = home_env_lock().await;
    let dir = tempfile::TempDir::new().expect("temp home");
    let prev_userprofile = std::env::var_os("USERPROFILE");
    let prev_home = std::env::var_os("HOME");
    std::env::set_var("USERPROFILE", dir.path());
    std::env::set_var("HOME", dir.path());

    let db_dir = dir.path().join(".icelines");
    std::fs::create_dir_all(&db_dir).expect("create db dir");
    let conn = rusqlite::Connection::open(db_dir.join("icelines.db")).expect("open db");
    conn.execute_batch(
        "CREATE TABLE watch_rules (
            id TEXT PRIMARY KEY,
            label TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            trigger_json TEXT NOT NULL,
            unsupported_sources_json TEXT NOT NULL DEFAULT '[]',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
         );
         INSERT INTO watch_rules VALUES (
            'player-matthew-knies',
            'Watch Matthew Knies when pp1',
            1,
            '{\"kind\":\"player_promoted\",\"player_id\":null,\"evidence\":{\"kind\":\"unknown\"}}',
            '[\"shifts\"]',
            '2026-05-09T12:00:00Z',
            '2026-05-09T12:00:00Z'
         );
         CREATE TABLE watch_rule_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            rule_id TEXT NOT NULL,
            entity_ref TEXT,
            message TEXT NOT NULL DEFAULT '',
            fired_at TEXT NOT NULL
         );
         INSERT INTO watch_rule_events (rule_id, entity_ref, message, fired_at)
         VALUES (
            'player-matthew-knies',
            'player:matthew knies',
            'PP1 usage crossed threshold',
            '2026-05-09T13:00:00Z'
         );",
    )
    .expect("seed watch rules db");

    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/watch-rules")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    match prev_userprofile {
        Some(p) => std::env::set_var("USERPROFILE", p),
        None => std::env::remove_var("USERPROFILE"),
    }
    match prev_home {
        Some(p) => std::env::set_var("HOME", p),
        None => std::env::remove_var("HOME"),
    }

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");

    let rules = json["rules"].as_array().expect("rules array");
    let persisted = rules
        .iter()
        .find(|rule| rule["id"] == "player-matthew-knies")
        .expect("persisted rule present");
    assert_eq!(persisted["label"], "Watch Matthew Knies when pp1");
    assert_eq!(persisted["unsupported_sources"][0], "shifts");
    assert_eq!(persisted["last_fired"], "2026-05-09T13:00:00Z");
}

#[tokio::test]
async fn l1_watch_rule_toggle_json_returns_mutation_result_view() {
    let _guard = home_env_lock().await;
    let _home = HomeEnvFixture::new();

    let home = std::env::var_os("USERPROFILE").expect("temp userprofile");
    let db_dir = std::path::PathBuf::from(home).join(".icelines");
    std::fs::create_dir_all(&db_dir).expect("create db dir");
    let conn = rusqlite::Connection::open(db_dir.join("icelines.db")).expect("open db");
    conn.execute_batch(
        "CREATE TABLE watch_rules (
            id TEXT PRIMARY KEY,
            label TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            trigger_json TEXT NOT NULL,
            unsupported_sources_json TEXT NOT NULL DEFAULT '[]',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
         );
         INSERT INTO watch_rules VALUES (
            'player-matthew-knies',
            'Watch Matthew Knies when pp1',
            1,
            '{\"kind\":\"player_promoted\",\"player_id\":null,\"evidence\":{\"kind\":\"unknown\"}}',
            '[\"shifts\"]',
            '2026-05-09T12:00:00Z',
            '2026-05-09T12:00:00Z'
         );",
    )
    .expect("seed watch rules db");

    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/watch-rules/set-enabled")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"rule_id":"player-matthew-knies","enabled":false}"#,
                ))
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");

    assert_eq!(json["operation"], "disable");
    assert_eq!(json["target"], "player-matthew-knies");
    assert_eq!(json["status"], "applied");

    let enabled: i64 = conn
        .query_row(
            "SELECT enabled FROM watch_rules WHERE id = 'player-matthew-knies'",
            [],
            |row| row.get(0),
        )
        .expect("enabled flag");
    assert_eq!(enabled, 0);
}

#[tokio::test]
async fn l1_watchlist_html_renders_watch_rule_toggle_form() {
    let _guard = home_env_lock().await;
    let _home = HomeEnvFixture::new();

    let home = std::env::var_os("USERPROFILE").expect("temp userprofile");
    let db_dir = std::path::PathBuf::from(home).join(".icelines");
    std::fs::create_dir_all(&db_dir).expect("create db dir");
    let conn = rusqlite::Connection::open(db_dir.join("icelines.db")).expect("open db");
    conn.execute_batch(
        "CREATE TABLE watch_rules (
            id TEXT PRIMARY KEY,
            label TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            trigger_json TEXT NOT NULL,
            unsupported_sources_json TEXT NOT NULL DEFAULT '[]',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
         );
         INSERT INTO watch_rules VALUES (
            'player-matthew-knies',
            'Watch Matthew Knies when pp1',
            1,
            '{\"kind\":\"player_promoted\",\"player_id\":null,\"evidence\":{\"kind\":\"unknown\"}}',
            '[\"shifts\"]',
            '2026-05-09T12:00:00Z',
            '2026-05-09T12:00:00Z'
         );",
    )
    .expect("seed watch rules db");

    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/watchlist")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let html = String::from_utf8(bytes.to_vec()).expect("utf8 html");

    assert!(html.contains("<h2>Rules</h2>"));
    assert!(html.contains("action=\"/watch-rules/create\""));
    assert!(html.contains(">Add rule</button>"));
    assert!(html.contains("player-matthew-knies"));
    assert!(html.contains("Watch Matthew Knies when pp1"));
    assert!(html.contains("action=\"/watch-rules/set-enabled\""));
    assert!(html.contains("action=\"/watch-rules/delete\""));
    assert!(html.contains(">Disable</button>"));
    assert!(html.contains(">Delete</button>"));
}

#[tokio::test]
async fn l1_watch_rule_toggle_form_redirects_and_updates_rule() {
    let _guard = home_env_lock().await;
    let _home = HomeEnvFixture::new();

    let home = std::env::var_os("USERPROFILE").expect("temp userprofile");
    let db_dir = std::path::PathBuf::from(home).join(".icelines");
    std::fs::create_dir_all(&db_dir).expect("create db dir");
    let conn = rusqlite::Connection::open(db_dir.join("icelines.db")).expect("open db");
    conn.execute_batch(
        "CREATE TABLE watch_rules (
            id TEXT PRIMARY KEY,
            label TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            trigger_json TEXT NOT NULL,
            unsupported_sources_json TEXT NOT NULL DEFAULT '[]',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
         );
         INSERT INTO watch_rules VALUES (
            'player-matthew-knies',
            'Watch Matthew Knies when pp1',
            1,
            '{\"kind\":\"player_promoted\",\"player_id\":null,\"evidence\":{\"kind\":\"unknown\"}}',
            '[\"shifts\"]',
            '2026-05-09T12:00:00Z',
            '2026-05-09T12:00:00Z'
         );",
    )
    .expect("seed watch rules db");

    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/watch-rules/set-enabled")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("rule_id=player-matthew-knies&enabled=false"))
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        response
            .headers()
            .get("location")
            .and_then(|h| h.to_str().ok()),
        Some("/watchlist")
    );
    let enabled: i64 = conn
        .query_row(
            "SELECT enabled FROM watch_rules WHERE id = 'player-matthew-knies'",
            [],
            |row| row.get(0),
        )
        .expect("enabled flag");
    assert_eq!(enabled, 0);
}

#[tokio::test]
async fn l1_watch_rule_create_form_redirects_and_persists_rule() {
    let _guard = home_env_lock().await;
    let _home = HomeEnvFixture::new();

    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/watch-rules/create")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("player=Matthew+Knies&trigger=available"))
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        response
            .headers()
            .get("location")
            .and_then(|h| h.to_str().ok()),
        Some("/watchlist")
    );

    let home = std::env::var_os("USERPROFILE").expect("temp userprofile");
    let db_path = std::path::PathBuf::from(home)
        .join(".icelines")
        .join("icelines.db");
    let conn = rusqlite::Connection::open(db_path).expect("open db");
    let (label, enabled, trigger_json): (String, i64, String) = conn
        .query_row(
            "SELECT label, enabled, trigger_json FROM watch_rules WHERE id = 'player-matthew-knies'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("persisted watch rule");
    assert_eq!(label, "Watch Matthew Knies when available");
    assert_eq!(enabled, 1);
    assert!(trigger_json.contains("availability_changed"));
}

#[tokio::test]
async fn l1_watch_rule_delete_form_redirects_and_removes_rule() {
    let _guard = home_env_lock().await;
    let _home = HomeEnvFixture::new();

    let home = std::env::var_os("USERPROFILE").expect("temp userprofile");
    let db_dir = std::path::PathBuf::from(home).join(".icelines");
    std::fs::create_dir_all(&db_dir).expect("create db dir");
    let conn = rusqlite::Connection::open(db_dir.join("icelines.db")).expect("open db");
    conn.execute_batch(
        "CREATE TABLE watch_rules (
            id TEXT PRIMARY KEY,
            label TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            trigger_json TEXT NOT NULL,
            unsupported_sources_json TEXT NOT NULL DEFAULT '[]',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
         );
         INSERT INTO watch_rules VALUES (
            'player-matthew-knies',
            'Watch Matthew Knies when pp1',
            1,
            '{\"kind\":\"player_promoted\",\"player_id\":null,\"evidence\":{\"kind\":\"unknown\"}}',
            '[\"shifts\"]',
            '2026-05-09T12:00:00Z',
            '2026-05-09T12:00:00Z'
         );",
    )
    .expect("seed watch rules db");

    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/watch-rules/delete")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("rule_id=player-matthew-knies"))
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        response
            .headers()
            .get("location")
            .and_then(|h| h.to_str().ok()),
        Some("/watchlist")
    );
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM watch_rules WHERE id = 'player-matthew-knies'",
            [],
            |row| row.get(0),
        )
        .expect("rule count");
    assert_eq!(count, 0);
}

#[tokio::test]
async fn l1_admin_data_status_json_returns_viewmodel_contract() {
    let _guard = home_env_lock().await;
    let _home = HomeEnvFixture::new();
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/data-status")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");

    assert!(json["root"].as_str().is_some());
    assert_eq!(json["total"], 0);
    assert_eq!(json["empty_state"]["kind"], "missing_source");
}

#[tokio::test]
async fn l1_admin_html_renders_operational_viewmodels() {
    let _guard = home_env_lock().await;
    let _home = HomeEnvFixture::new();
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/admin")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let html = String::from_utf8(bytes.to_vec()).expect("utf8 html");

    assert!(html.contains("<h1>Admin</h1>"));
    assert!(html.contains("Data Status"));
    assert!(html.contains("Snapshots"));
    assert!(html.contains("Runtime Config"));
    assert!(html.contains("web.active_season"));
    assert!(html.contains("action=\"/admin/config/set\""));
    assert!(html.contains("action=\"/admin/config/reset\""));
}

#[tokio::test]
async fn l1_admin_html_renders_data_verify_form_for_manifest_rows() {
    let _guard = home_env_lock().await;
    let _home = HomeEnvFixture::new();
    let data_root = std::env::var("USERPROFILE")
        .map(std::path::PathBuf::from)
        .expect("temp home")
        .join(".icelines")
        .join("data");
    let store = DataStore::open(&data_root).expect("open data store");
    store
        .manifest()
        .upsert(
            DataKind::Bios,
            ManifestEntry {
                key: DataKey::Season(Season(20252026)),
                path: data_root.join("bios.json"),
                freshness: Freshness {
                    fetched_at: chrono::Utc::now(),
                    source: FetchSource::Manual,
                    ttl: Ttl::After(Duration::from_secs(3600)),
                },
            },
        )
        .expect("seed manifest");
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/admin")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let html = String::from_utf8(bytes.to_vec()).expect("utf8 html");

    assert!(html.contains("action=\"/admin/data/verify\""));
    assert!(html.contains("name=\"target\" value=\"20252026\""));
}

#[tokio::test]
async fn l1_admin_snapshots_json_returns_viewmodel_contract() {
    let _guard = home_env_lock().await;
    let _home = HomeEnvFixture::new();
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/snapshots")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");

    assert_eq!(json["total"], 0);
    assert_eq!(json["empty_state"]["kind"], "missing_source");
}

#[tokio::test]
async fn l1_admin_html_renders_snapshot_activate_form_for_sealed_inactive_rows() {
    let _guard = home_env_lock().await;
    let _home = HomeEnvFixture::new();
    let store = SnapshotStore::new(SnapshotStore::default_root());
    store
        .create(
            "stats-a",
            "20252026",
            SnapshotTier::Stats,
            None,
            "2026-05-10",
        )
        .expect("create snapshot a");
    store.seal("stats-a").expect("seal snapshot a");
    store
        .create(
            "stats-b",
            "20252026",
            SnapshotTier::Stats,
            None,
            "2026-05-11",
        )
        .expect("create snapshot b");
    store.seal("stats-b").expect("seal snapshot b");
    store.set_active("stats-a").expect("set active snapshot");
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/admin")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let html = String::from_utf8(bytes.to_vec()).expect("utf8 html");

    assert!(html.contains("action=\"/admin/snapshots/activate\""));
    assert!(html.contains("action=\"/admin/snapshots/delete\""));
    assert!(html.contains("name=\"name\" value=\"stats-b\""));
}

#[tokio::test]
async fn l1_admin_config_json_returns_runtime_config_viewmodel() {
    let state = WebState::new();
    {
        let mut cfg = state.config.write().await;
        *cfg = WebConfig::new("20242025", "playoff");
    }
    let app = router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/config?selected=web.active_season_type")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");

    assert_eq!(json["rows"].as_array().map(Vec::len), Some(3));
    assert_eq!(json["rows"][1]["key"], "web.active_season_type");
    assert_eq!(json["rows"][1]["value"], "playoff");
    assert_eq!(json["rows"][1]["selected"], true);
}

#[tokio::test]
async fn l1_admin_config_set_json_returns_mutation_result_view() {
    let state = WebState::new();
    let app = router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/config/set")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"key":"web.active_season_type","value":"playoff"}"#,
                ))
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");

    assert_eq!(json["operation"], "config_set");
    assert_eq!(json["target"], "web.active_season_type");
    assert_eq!(json["status"], "applied");
    assert_eq!(state.config.read().await.active_season_type, "playoff");
}

#[tokio::test]
async fn l1_admin_config_set_form_redirects_and_updates_runtime_config() {
    let state = WebState::new();
    let app = router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/config/set")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("key=web.active_season_type&value=playoff"))
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        response
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok()),
        Some("/admin")
    );
    assert_eq!(state.config.read().await.active_season_type, "playoff");
}

#[tokio::test]
async fn l1_admin_config_reset_form_redirects_and_restores_runtime_config() {
    let state = WebState::new();
    {
        let mut cfg = state.config.write().await;
        *cfg = WebConfig::new("20242025", "playoff");
    }
    let app = router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/config/reset")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("key=web.active_season_type"))
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        response
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok()),
        Some("/admin")
    );
    assert_eq!(state.config.read().await.active_season_type, "regular");
}

#[tokio::test]
async fn l1_admin_config_reset_json_returns_noop_when_already_default() {
    let state = WebState::new();
    let app = router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/config/reset")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"key":"web.active_season_type"}"#))
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");

    assert_eq!(json["operation"], "config_reset");
    assert_eq!(json["status"], "noop");
}

#[tokio::test]
async fn l1_admin_snapshot_activate_json_returns_mutation_result_view() {
    let _guard = home_env_lock().await;
    let _home = HomeEnvFixture::new();
    let store = SnapshotStore::new(SnapshotStore::default_root());
    store
        .create(
            "stats-a",
            "20252026",
            SnapshotTier::Stats,
            None,
            "2026-05-10",
        )
        .expect("create snapshot a");
    store.seal("stats-a").expect("seal snapshot a");
    store
        .create(
            "stats-b",
            "20252026",
            SnapshotTier::Stats,
            None,
            "2026-05-11",
        )
        .expect("create snapshot b");
    store.seal("stats-b").expect("seal snapshot b");
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/snapshots/activate")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"stats-a"}"#))
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");

    assert_eq!(json["operation"], "snapshot_activate");
    assert_eq!(json["target"], "stats-a");
    assert_eq!(json["status"], "applied");
    assert_eq!(
        store.load_manifest().expect("manifest").active.as_deref(),
        Some("stats-a")
    );
}

#[tokio::test]
async fn l1_admin_snapshot_activate_form_redirects_and_sets_active_snapshot() {
    let _guard = home_env_lock().await;
    let _home = HomeEnvFixture::new();
    let store = SnapshotStore::new(SnapshotStore::default_root());
    store
        .create(
            "stats-a",
            "20252026",
            SnapshotTier::Stats,
            None,
            "2026-05-10",
        )
        .expect("create snapshot a");
    store.seal("stats-a").expect("seal snapshot a");
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/snapshots/activate")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("name=stats-a"))
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        response
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok()),
        Some("/admin")
    );
    assert_eq!(
        store.load_manifest().expect("manifest").active.as_deref(),
        Some("stats-a")
    );
}

#[tokio::test]
async fn l1_admin_snapshot_delete_json_returns_mutation_result_view() {
    let _guard = home_env_lock().await;
    let _home = HomeEnvFixture::new();
    let store = SnapshotStore::new(SnapshotStore::default_root());
    store
        .create(
            "stats-a",
            "20252026",
            SnapshotTier::Stats,
            None,
            "2026-05-10",
        )
        .expect("create snapshot a");
    store.seal("stats-a").expect("seal snapshot a");
    store
        .create(
            "stats-b",
            "20252026",
            SnapshotTier::Stats,
            None,
            "2026-05-11",
        )
        .expect("create snapshot b");
    store.seal("stats-b").expect("seal snapshot b");
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/snapshots/delete")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"stats-a"}"#))
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");

    assert_eq!(json["operation"], "snapshot_remove");
    assert_eq!(json["target"], "stats-a");
    assert_eq!(json["status"], "applied");
    let names: BTreeSet<String> = store
        .load_manifest()
        .expect("manifest")
        .snapshots
        .into_iter()
        .map(|entry| entry.name)
        .collect();
    assert!(!names.contains("stats-a"));
    assert!(names.contains("stats-b"));
}

#[tokio::test]
async fn l1_admin_snapshot_delete_form_redirects_and_removes_inactive_snapshot() {
    let _guard = home_env_lock().await;
    let _home = HomeEnvFixture::new();
    let store = SnapshotStore::new(SnapshotStore::default_root());
    store
        .create(
            "stats-a",
            "20252026",
            SnapshotTier::Stats,
            None,
            "2026-05-10",
        )
        .expect("create snapshot a");
    store.seal("stats-a").expect("seal snapshot a");
    store
        .create(
            "stats-b",
            "20252026",
            SnapshotTier::Stats,
            None,
            "2026-05-11",
        )
        .expect("create snapshot b");
    store.seal("stats-b").expect("seal snapshot b");
    store.set_active("stats-b").expect("set active snapshot");
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/snapshots/delete")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("name=stats-a"))
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        response
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok()),
        Some("/admin")
    );
    let names: BTreeSet<String> = store
        .load_manifest()
        .expect("manifest")
        .snapshots
        .into_iter()
        .map(|entry| entry.name)
        .collect();
    assert!(!names.contains("stats-a"));
    assert!(names.contains("stats-b"));
}

#[tokio::test]
async fn l1_admin_data_verify_json_returns_mutation_result_view() {
    let _guard = home_env_lock().await;
    let _home = HomeEnvFixture::new();
    let data_root = std::env::var("USERPROFILE")
        .map(std::path::PathBuf::from)
        .expect("temp home")
        .join(".icelines")
        .join("data");
    let store = DataStore::open(&data_root).expect("open data store");
    store
        .manifest()
        .upsert(
            DataKind::Bios,
            ManifestEntry {
                key: DataKey::Season(Season(20252026)),
                path: data_root.join("bios.json"),
                freshness: Freshness {
                    fetched_at: chrono::Utc::now(),
                    source: FetchSource::Manual,
                    ttl: Ttl::After(Duration::from_secs(3600)),
                },
            },
        )
        .expect("seed manifest");
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/data/verify")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"target":"20252026"}"#))
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");

    assert_eq!(json["operation"], "data_verify");
    assert_eq!(json["target"], "20252026");
    assert_eq!(json["status"], "noop");
}

#[tokio::test]
async fn l1_admin_data_verify_form_redirects_for_known_target() {
    let _guard = home_env_lock().await;
    let _home = HomeEnvFixture::new();
    let data_root = std::env::var("USERPROFILE")
        .map(std::path::PathBuf::from)
        .expect("temp home")
        .join(".icelines")
        .join("data");
    let store = DataStore::open(&data_root).expect("open data store");
    store
        .manifest()
        .upsert(
            DataKind::Bios,
            ManifestEntry {
                key: DataKey::Season(Season(20252026)),
                path: data_root.join("bios.json"),
                freshness: Freshness {
                    fetched_at: chrono::Utc::now(),
                    source: FetchSource::Manual,
                    ttl: Ttl::After(Duration::from_secs(3600)),
                },
            },
        )
        .expect("seed manifest");
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/data/verify")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("target=20252026"))
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        response
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok()),
        Some("/admin")
    );
}

#[tokio::test]
async fn l1_admin_data_verify_json_rejects_unknown_target() {
    let _guard = home_env_lock().await;
    let _home = HomeEnvFixture::new();
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/data/verify")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"target":"19001901"}"#))
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");
    assert!(json["error"]
        .as_str()
        .expect("error string")
        .contains("was not found"));
}

#[tokio::test]
async fn l1_favorites_add_json_returns_mutation_result_view() {
    let _guard = home_env_lock().await;
    let _home = HomeEnvFixture::new();
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/favorites/add")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"key":"EDM","kind":"team","return_to":"/favorites"}"#,
                ))
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");

    assert_eq!(json["operation"], "add");
    assert_eq!(json["target"], "team:EDM");
    assert_eq!(json["status"], "applied");
    assert_eq!(json["redirect_to"], "/favorites");
}

#[tokio::test]
async fn l1_favorites_remove_json_returns_mutation_result_view() {
    let _guard = home_env_lock().await;
    let _home = HomeEnvFixture::new();
    let app = router(WebState::new());

    let add_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/favorites/add")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"key":"EDM","kind":"team"}"#))
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");
    assert_eq!(add_response.status(), StatusCode::OK);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/favorites/remove")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"key":"EDM","kind":"team"}"#))
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");

    assert_eq!(json["operation"], "remove");
    assert_eq!(json["target"], "team:EDM");
    assert_eq!(json["status"], "applied");
}

/// l1_career_route_missing_league_returns_400 (Calder.4)
/// — `/career` without `?league=…` rejects with 400 + helpful body.
#[tokio::test]
async fn l1_career_route_missing_league_returns_400() {
    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/career")
                .body(Body::empty())
                .expect("ok"),
        )
        .await
        .expect("dispatch ok");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("body");
    let body = std::str::from_utf8(&bytes).unwrap_or("");
    assert!(
        body.contains("league") && body.contains("OHL"),
        "error body should hint at the right call shape, got:\n{body}"
    );
}

#[tokio::test]
async fn l1_career_html_uses_shared_page_shell() {
    let _guard = home_env_lock().await;
    let dir = tempfile::tempdir().expect("temp home");
    let prev_userprofile = std::env::var_os("USERPROFILE");
    let prev_home = std::env::var_os("HOME");
    std::env::set_var("USERPROFILE", dir.path());
    std::env::set_var("HOME", dir.path());

    let mut store = CareerHistoryStore::new();
    store.upsert(career_history(
        990001,
        vec![career_stint(20142015, "OHL", "ER", 60, 40, 50)],
    ));
    store.upsert(career_history(
        990002,
        vec![career_stint(20142015, "OHL", "LDN", 62, 30, 45)],
    ));
    let path = dir.path().join(".icelines").join("career_history.json");
    store.save(&path).expect("save career store");

    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/career?league=OHL&season=20142015&sort=points&top=2")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    match prev_userprofile {
        Some(value) => std::env::set_var("USERPROFILE", value),
        None => std::env::remove_var("USERPROFILE"),
    }
    match prev_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let body = std::str::from_utf8(&bytes).expect("html is utf-8");

    assert!(body.contains("season-header"));
    assert!(body.contains("global-nav"));
    assert!(body.contains("OHL Leaders"));
    assert!(body.contains("2014-15"));
    assert!(body.contains("/api/v1/career?league=OHL"));
    assert!(body.contains("career-table"));
}

/// l1_api_career_envelope_shape (Calder.4)
/// — `/api/v1/career` envelope. When the local store is empty the
///   handler returns 400 with the same envelope shape and a helpful
///   error string.
#[tokio::test]
async fn l1_api_career_envelope_shape() {
    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/career?league=OHL&season=20142015")
                .body(Body::empty())
                .expect("ok"),
        )
        .await
        .expect("dispatch ok");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("body");
    let v: serde_json::Value = serde_json::from_slice(&bytes).expect("valid json");
    if status == StatusCode::OK {
        // Store populated — assert envelope shape.
        let obj = assert_data_meta_envelope(&v, "career");
        assert!(obj["data"].is_array());
    } else {
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let obj = assert_shared_error_envelope(&v, "career");
        assert!(obj["data"].as_array().is_some_and(Vec::is_empty));
    }
}

#[tokio::test]
async fn l1_api_career_rows_match_career_view() {
    let _guard = home_env_lock().await;
    let dir = tempfile::tempdir().expect("temp home");
    let prev_userprofile = std::env::var_os("USERPROFILE");
    let prev_home = std::env::var_os("HOME");
    std::env::set_var("USERPROFILE", dir.path());
    std::env::set_var("HOME", dir.path());

    let mut store = CareerHistoryStore::new();
    store.upsert(career_history(
        990001,
        vec![career_stint(20142015, "OHL", "ER", 60, 40, 50)],
    ));
    store.upsert(career_history(
        990002,
        vec![career_stint(20142015, "OHL", "LDN", 62, 30, 45)],
    ));
    store.upsert(career_history(
        990003,
        vec![career_stint(20142015, "OHL", "OSH", 55, 28, 30)],
    ));
    let histories: Vec<_> = store
        .histories
        .iter()
        .filter_map(|(pid, history)| pid.parse::<u32>().ok().map(|pid| (pid, history.clone())))
        .collect();
    let expected_view = CareerView::from_histories(
        ViewContext::new(ViewWindow::new(Season(0), SeasonType::Regular)),
        "OHL".to_owned(),
        Some(20142015),
        CareerSortKey::Points,
        2,
        histories,
        HashMap::new(),
    );
    let expected_rows = career_row_snapshots(&expected_view.rows);
    let path = dir.path().join(".icelines").join("career_history.json");
    store.save(&path).expect("save career store");

    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/career?league=OHL&season=20142015&sort=points&top=2")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    match prev_userprofile {
        Some(value) => std::env::set_var("USERPROFILE", value),
        None => std::env::remove_var("USERPROFILE"),
    }
    match prev_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }

    assert_eq!(response.status(), StatusCode::OK);
    let json = response_json(response, 1024 * 1024).await;
    assert_data_meta_envelope(&json, "career");
    assert_eq!(json["meta"]["league"], serde_json::json!("OHL"));
    assert_eq!(json["meta"]["season"], serde_json::json!(20142015));
    assert_eq!(json["meta"]["sort"], serde_json::json!("points"));
    assert_eq!(json["meta"]["count"], serde_json::json!(2));
    assert_eq!(json["meta"]["total"], serde_json::json!(3));
    assert_eq!(json_career_row_snapshots(&json), expected_rows);
}

/// l1_depth_json_envelope_shape (T3)
/// — `/api/v1/depth` is the JSON twin of `/depth`. Every list page on
///   the web surface gets one (King.2.4 convention) so external scripts
///   don't have to scrape HTML. This fence pins the literal envelope
///   keys + types so a schema bump can't slip in unannounced.
#[tokio::test]
async fn l1_depth_json_envelope_shape() {
    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/depth")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");
    assert_eq!(response.status(), StatusCode::OK);
    let ct = response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        ct.starts_with("application/json"),
        "expected JSON content-type, got {ct:?}"
    );

    let v = response_json(response, 1024 * 1024).await;
    let obj = assert_data_meta_envelope(&v, "depth");
    assert!(obj["data"].is_array(), "data must be an array");
    let meta_keys: BTreeSet<_> = obj["meta"]
        .as_object()
        .expect("meta is an object")
        .keys()
        .map(String::as_str)
        .collect();
    let want_meta: BTreeSet<_> = ["count", "scoring_mode", "season", "season_type"]
        .iter()
        .copied()
        .collect();
    assert_eq!(meta_keys, want_meta, "meta keys diverged: {meta_keys:?}");
}

#[tokio::test]
async fn l1_depth_json_rows_match_depth_league_view() {
    let season = Season(20242025);
    let season_type = SeasonType::Regular;
    let store = SnapshotStore::new(SnapshotStore::default_root());
    let load = load_into_repo(season, season_type, &store)
        .expect("bundled regular-season repo should load");
    let expected_view = DepthLeagueView::pace_from_repository(&load.repo, season, season_type);
    let expected_rows = depth_league_row_snapshots(&expected_view.rows);
    assert!(
        !expected_rows.is_empty(),
        "fixture should include depth rows for {season:?}"
    );

    let state = WebState::with_repo_and_config(load.repo, WebConfig::new("20242025", "regular"));
    let app = router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/depth")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");
    assert_eq!(response.status(), StatusCode::OK);

    let json = response_json(response, 1024 * 1024).await;
    assert_data_meta_envelope(&json, "depth");
    assert_eq!(json["meta"]["season"], serde_json::json!("20242025"));
    assert_eq!(json["meta"]["season_type"], serde_json::json!("regular"));
    assert_eq!(json["meta"]["scoring_mode"], serde_json::json!("pace"));
    assert_eq!(
        json["meta"]["count"],
        serde_json::json!(expected_rows.len()),
        "depth meta count should match row count"
    );
    assert_eq!(json_depth_league_row_snapshots(&json), expected_rows);
}

#[tokio::test]
async fn l1_depth_json_error_uses_shared_envelope_shape() {
    let state = WebState::new();
    {
        let mut cfg = state.config.write().await;
        *cfg = icelines_web::WebConfig::new("not-a-season", "regular");
    }
    let app = router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/depth")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let v = response_json(response, 1024 * 1024).await;
    let obj = assert_shared_error_envelope(&v, "depth");
    assert!(obj["data"].as_array().is_some_and(Vec::is_empty));
}

#[tokio::test]
async fn l1_compare_json_envelope_shape() {
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/compare")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let json: serde_json::Value =
        serde_json::from_slice(&bytes).expect("compare response should be valid json");

    let obj = assert_data_meta_envelope(&json, "compare");
    assert!(
        !obj.contains_key("error"),
        "successful compare envelope should not carry error"
    );
    assert_eq!(json["meta"]["season_type"], "regular");
    assert!(json["data"]["a"].is_null());
    assert!(json["data"]["b"].is_null());
    assert!(json["data"]["winners"].is_object());
}

#[tokio::test]
async fn l1_compare_json_cards_match_compare_view() {
    let season = Season(20242025);
    let season_type = SeasonType::Regular;
    let a_id = PlayerId(8478402);
    let b_id = PlayerId(8477934);
    let store = SnapshotStore::new(SnapshotStore::default_root());
    let mut load = load_into_repo(season, season_type, &store)
        .expect("bundled regular-season repo should load");
    load_player_career_into_repo(&mut load.repo, a_id).expect("player a career should load");
    load_player_career_into_repo(&mut load.repo, b_id).expect("player b career should load");
    let expected_view =
        CompareView::from_repository(&load.repo, Some(a_id), Some(b_id), season, season_type);
    let expected_a = expected_view
        .a
        .as_ref()
        .map(compare_card_snapshot_from_view)
        .expect("player a compare card should exist");
    let expected_b = expected_view
        .b
        .as_ref()
        .map(compare_card_snapshot_from_view)
        .expect("player b compare card should exist");

    let state = WebState::with_repo_and_config(load.repo, WebConfig::new("20242025", "regular"));
    let app = router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/compare?a=8478402&b=8477934")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");
    assert_eq!(response.status(), StatusCode::OK);

    let json = response_json(response, 1024 * 1024).await;
    assert_data_meta_envelope(&json, "compare");
    assert_eq!(json["meta"]["season"], serde_json::json!("20242025"));
    assert_eq!(json["meta"]["season_type"], serde_json::json!("regular"));
    assert_eq!(json_compare_card_snapshot(&json["data"]["a"]), expected_a);
    assert_eq!(json_compare_card_snapshot(&json["data"]["b"]), expected_b);
}

#[tokio::test]
async fn l1_compare_json_similarity_matches_similar_players_view() {
    let season = Season(20242025);
    let season_type = SeasonType::Regular;
    let target_id = PlayerId(8478402);
    let store = SnapshotStore::new(SnapshotStore::default_root());
    let load = load_into_repo(season, season_type, &store)
        .expect("bundled regular-season repo should load");
    let views: Vec<_> = load.repo.skaters(season, season_type).collect();
    let target = views
        .iter()
        .find(|view| view.identity.id == target_id)
        .expect("target skater should exist");
    let expected = SimilarPlayersView::from_player_views(
        &views,
        target,
        3,
        season,
        season_type,
        load.repo.has_window(season, season_type),
    );
    let expected_ids: Vec<u32> = expected.rows.iter().map(|row| row.player_id.0).collect();

    let state = WebState::with_repo_and_config(load.repo, WebConfig::new("20242025", "regular"));
    let app = router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/compare?a=8478402&similar=3")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");
    assert_eq!(response.status(), StatusCode::OK);

    let json = response_json(response, 1024 * 1024).await;
    assert_data_meta_envelope(&json, "compare");
    assert!(json["data"]["b"].is_null());
    assert_eq!(
        json["data"]["similar"]["target"]["player_id"],
        serde_json::json!(8478402)
    );
    let actual_ids: Vec<u32> = json["data"]["similar"]["rows"]
        .as_array()
        .expect("similar rows should be an array")
        .iter()
        .map(|row| {
            row["player_id"]
                .as_u64()
                .expect("row player_id should be numeric") as u32
        })
        .collect();
    assert_eq!(actual_ids, expected_ids);
}

#[tokio::test]
async fn l1_compare_html_similarity_renders_similar_players_section() {
    let season = Season(20242025);
    let season_type = SeasonType::Regular;
    let store = SnapshotStore::new(SnapshotStore::default_root());
    let load = load_into_repo(season, season_type, &store)
        .expect("bundled regular-season repo should load");
    let state = WebState::with_repo_and_config(load.repo, WebConfig::new("20242025", "regular"));
    let app = router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/compare?a=8478402&similar=3")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("body fits");
    let html = String::from_utf8(bytes.to_vec()).expect("html should be utf8");

    assert!(html.contains("Similar players"), "html: {html}");
    assert!(
        html.contains("Connor McDavid") || html.contains("player cohort"),
        "html should render the similarity target context: {html}"
    );
    assert!(
        !html.contains("Pick two players to compare"),
        "similarity mode should not render the empty compare hint: {html}"
    );
}

#[tokio::test]
async fn l1_compare_json_bad_input_uses_shared_error_envelope() {
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/compare?a=Connor%20McDavid")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let json = response_json(response, 256 * 1024).await;
    let obj = assert_shared_error_envelope(&json, "compare");
    assert!(obj["data"]["a"].is_object() || obj["data"]["a"].is_null());
    assert!(obj["data"]["b"].is_null());
}

#[tokio::test]
async fn l1_goalies_json_envelope_shape() {
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/goalies?sort=wins&top=5")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let json = response_json(response, 256 * 1024).await;
    let obj = assert_data_meta_envelope(&json, "goalies");
    assert!(
        !obj.contains_key("error"),
        "successful goalies envelope should not carry error"
    );
    assert_eq!(json["meta"]["sort"], "wins");
    assert_eq!(json["meta"]["top"], 5);
    assert!(json["meta"]["returned"].is_number());
    assert!(json["data"].is_array());
}

#[tokio::test]
async fn l1_goalies_json_accepts_cli_parity_saves_sort_and_gp_min() {
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/goalies?sort=saves&gp_min=15&top=5")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let json = response_json(response, 256 * 1024).await;
    assert_data_meta_envelope(&json, "goalies");
    assert_eq!(json["meta"]["sort"], "saves");
    assert_eq!(json["meta"]["qualified_gp_min"], 15);
    assert_eq!(json["meta"]["top"], 5);
    let rows = json["data"].as_array().expect("goalies data array");
    assert!(
        rows.iter()
            .all(|row| row["games"].as_u64().is_some_and(|gp| gp >= 15)),
        "gp_min should apply to every returned goalie row"
    );
    if let Some(first) = rows.first() {
        assert!(first["saves"].is_number());
    }
}

#[tokio::test]
async fn l1_team_json_unknown_team_uses_shared_envelope_shape() {
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/team/ZZZ")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let v = response_json(response, 1024 * 1024).await;
    let obj = assert_shared_error_envelope(&v, "team");
    assert_eq!(obj["data"]["team_abbrev"], serde_json::json!("ZZZ"));
    assert!(obj["data"]["skaters"].as_array().is_some_and(Vec::is_empty));
    assert!(obj["data"]["goalies"].as_array().is_some_and(Vec::is_empty));
}

#[tokio::test]
async fn l1_team_json_bad_active_season_uses_shared_envelope_shape() {
    let state = WebState::new();
    {
        let mut cfg = state.config.write().await;
        *cfg = icelines_web::WebConfig::new("not-a-season", "regular");
    }
    let app = router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/team/EDM")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("body fits");
    let v: serde_json::Value = serde_json::from_slice(&bytes).expect("valid json");
    assert_eq!(v["route"], serde_json::json!("team"));
    assert_eq!(v["meta"]["team_abbrev"], serde_json::json!("EDM"));
    assert_eq!(v["meta"]["skater_count"], serde_json::json!(0));
    assert_eq!(v["meta"]["goalie_count"], serde_json::json!(0));
    assert!(v["data"]["skaters"].as_array().is_some_and(Vec::is_empty));
    assert!(v["error"].is_string());
}

#[tokio::test]
async fn l1_team_json_rows_match_team_depth_view() {
    let season = Season(20242025);
    let season_type = SeasonType::Regular;
    let team = TeamAbbr::parse("EDM").expect("known team abbrev");
    let store = SnapshotStore::new(SnapshotStore::default_root());
    let load = load_into_repo(season, season_type, &store)
        .expect("bundled regular-season repo should load");
    let expected_view =
        TeamDepthView::from_repository(&load.repo, team.clone(), season, season_type);
    let expected_skaters = team_depth_skater_snapshots(&expected_view);
    let expected_goalies = team_depth_goalie_snapshots(&expected_view.goalies);
    assert!(
        !expected_skaters.is_empty(),
        "fixture should include EDM skaters for {season:?}"
    );
    assert!(
        !expected_goalies.is_empty(),
        "fixture should include EDM goalies for {season:?}"
    );

    let state = WebState::with_repo_and_config(load.repo, WebConfig::new("20242025", "regular"));
    let app = router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/team/EDM")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");
    assert_eq!(response.status(), StatusCode::OK);

    let json = response_json(response, 1024 * 1024).await;
    assert_data_meta_envelope(&json, "team");
    assert_eq!(json["meta"]["team_abbrev"], serde_json::json!("EDM"));
    assert_eq!(json["meta"]["season"], serde_json::json!("20242025"));
    assert_eq!(json["meta"]["season_type"], serde_json::json!("regular"));
    assert_eq!(json_team_skater_snapshots(&json), expected_skaters);
    assert_eq!(json_team_goalie_snapshots(&json), expected_goalies);
}

#[tokio::test]
async fn l1_scores_json_envelope_shape() {
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/scores?date=2014-10-08&range=day")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let json: serde_json::Value =
        serde_json::from_slice(&bytes).expect("scores response should be valid json");

    let obj = assert_data_meta_envelope(&json, "scores");
    assert!(
        !obj.contains_key("error"),
        "successful scores envelope should not carry error"
    );
    assert_eq!(json["meta"]["active_date"], "2014-10-08");
    assert_eq!(json["meta"]["range"], "day");
    assert!(json["meta"]
        .as_object()
        .is_some_and(|meta| meta.contains_key("source_error")));
    assert!(json["data"].is_array());
}

#[tokio::test]
async fn l1_schedule_json_envelope_shape() {
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/schedule?date=2014-10-08")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let json: serde_json::Value =
        serde_json::from_slice(&bytes).expect("schedule response should be valid json");

    let obj = assert_data_meta_envelope(&json, "schedule");
    assert!(
        !obj.contains_key("error"),
        "successful schedule envelope should not carry error"
    );
    assert_eq!(json["meta"]["active_date"], "2014-10-08");
    assert_eq!(json["meta"]["active_team"], "");
    assert!(json["meta"]["team_chips"].is_array());
    assert!(json["meta"]
        .as_object()
        .is_some_and(|meta| meta.contains_key("source_error")));
    assert!(json["data"].is_array());
}

#[tokio::test]
async fn l1_playoffs_json_envelope_shape() {
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/playoffs")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let json: serde_json::Value =
        serde_json::from_slice(&bytes).expect("playoffs response should be valid json");

    let obj = assert_data_meta_envelope(&json, "playoffs");
    assert!(
        !obj.contains_key("error"),
        "successful playoffs envelope should not carry error"
    );
    assert!(json["meta"]["season"].is_string());
    assert!(json["meta"]["round_count"].is_number());
    assert!(json["meta"]["series_count"].is_number());
    assert!(json["meta"]
        .as_object()
        .is_some_and(|meta| meta.contains_key("source_error")));
    assert!(json["data"].is_array());
}

#[tokio::test]
async fn l1_transactions_json_envelope_shape() {
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/transactions?kind=trade&team=TOR")
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let json: serde_json::Value =
        serde_json::from_slice(&bytes).expect("transactions response should be valid json");

    let obj = assert_data_meta_envelope(&json, "transactions");
    assert!(
        !obj.contains_key("error"),
        "successful transactions envelope should not carry error"
    );
    assert_eq!(json["meta"]["active_kind"], "trade");
    assert_eq!(json["meta"]["active_team"], "TOR");
    assert!(json["meta"]["total"].is_number());
    assert!(json["data"].is_array());
}

/// l1_unknown_route_returns_404
/// — axum's default not-found handler. Once King.1.6 adds host-header
///   validation we'll add a 421 case for DNS rebinding, but the basic
///   404 contract starts here.
#[tokio::test]
async fn l1_unknown_route_returns_404() {
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/this-route-does-not-exist")
                .body(Body::empty())
                .expect("request builder should succeed"),
        )
        .await
        .expect("oneshot dispatch should not fail");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ── Season-type toggle (UX.E, 2026-05-04) ───────────────────────────
//
// `/season-type/:kind` flips `WebState.config.active_season_type` and
// redirects back to where the user came from. The route is the
// only writer of season-type today (the Reports overlay's PATCH
// /api/v1/active-season is the long-term destination per the spec).
//
// Locked behavior:
// - `playoff` and `playoffs` both normalize to "playoff".
// - `regular` and anything-else (including injection attempts)
//   normalize to "regular" — the whitelist is the security boundary
//   so a malformed URL can't poison config.
// - Response is 303 See Other with a Location header (per HTTP, GET
//   handlers redirect with 303, not 302, when the result is a new
//   resource view).
// - Location preserves the user's previous page when Referer is set
//   to a same-origin URL; falls back to "/" otherwise.

/// Helper — dispatch one request and return (status, location header).
async fn flip_season_type(
    state: WebState,
    kind: &str,
    referer: Option<&str>,
) -> (StatusCode, Option<String>) {
    let app = router(state);
    let mut req = Request::builder().uri(format!("/season-type/{kind}"));
    if let Some(r) = referer {
        req = req.header(axum::http::header::REFERER, r);
    }
    let response = app
        .oneshot(req.body(Body::empty()).expect("build request"))
        .await
        .expect("oneshot");
    let location = response
        .headers()
        .get(axum::http::header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_owned());
    (response.status(), location)
}

/// l1_season_type_playoff_flips_state_and_redirects_303
/// — happy path: `/season-type/playoff` flips state.config.active_season_type
///   from default ("regular") to "playoff" AND returns 303.
#[tokio::test]
async fn l1_season_type_playoff_flips_state_and_redirects_303() {
    let state = WebState::new();
    let captured = state.config.clone();

    let (status, location) = flip_season_type(state, "playoff", None).await;

    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location.as_deref(), Some("/"));
    let cfg = captured.read().await;
    assert_eq!(cfg.active_season_type, "playoff");
    assert!(
        cfg.active_label.contains("Playoff"),
        "active_label should reflect the new type, got: {}",
        cfg.active_label
    );
}

/// l1_season_type_regular_flips_back
/// — round-trip: after a flip to playoff, flipping to "regular" must
///   return state.config to "regular". Active label re-formats too.
#[tokio::test]
async fn l1_season_type_regular_flips_back() {
    let state = WebState::new();
    {
        let mut cfg = state.config.write().await;
        *cfg = icelines_web::WebConfig::new("20252026", "playoff");
    }
    let captured = state.config.clone();

    let (status, _) = flip_season_type(state, "regular", None).await;

    assert_eq!(status, StatusCode::SEE_OTHER);
    let cfg = captured.read().await;
    assert_eq!(cfg.active_season_type, "regular");
    assert!(cfg.active_label.contains("Regular"));
}

/// l1_season_type_plural_playoffs_normalizes_to_singular
/// — both "playoff" and "playoffs" must work. The path token may
///   read more naturally as "playoffs" but the canonical config
///   value is the singular form (lockstep with the CLI's
///   `--season-type` flag).
#[tokio::test]
async fn l1_season_type_plural_playoffs_normalizes_to_singular() {
    let state = WebState::new();
    let captured = state.config.clone();

    let (status, _) = flip_season_type(state, "playoffs", None).await;

    assert_eq!(status, StatusCode::SEE_OTHER);
    let cfg = captured.read().await;
    assert_eq!(cfg.active_season_type, "playoff");
}

/// l1_season_type_unknown_kind_falls_back_to_regular
/// — security boundary: a bogus path component MUST NOT poison the
///   config (e.g. /season-type/<script>alert(1)</script>). Whitelist
///   on the way in: anything not "playoff*" → "regular". This test
///   also covers the case where a user follows a stale link with
///   "Regular" capitalized — case-insensitive.
#[tokio::test]
async fn l1_season_type_unknown_kind_falls_back_to_regular() {
    let state = WebState::new();
    {
        let mut cfg = state.config.write().await;
        *cfg = icelines_web::WebConfig::new("20252026", "playoff");
    }
    let captured = state.config.clone();

    let (status, _) = flip_season_type(state, "garbage-input", None).await;

    assert_eq!(status, StatusCode::SEE_OTHER);
    let cfg = captured.read().await;
    assert_eq!(cfg.active_season_type, "regular");
}

/// l1_season_type_redirect_honors_relative_referer
/// — when the user clicks the toggle while on /leaders, they should
///   land back on /leaders (not /). Relative referers pass through.
#[tokio::test]
async fn l1_season_type_redirect_honors_relative_referer() {
    let state = WebState::new();

    let (status, location) = flip_season_type(state, "playoff", Some("/leaders?sort=hits")).await;

    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location.as_deref(), Some("/leaders?sort=hits"));
}

/// l1_season_type_redirect_strips_localhost_origin
/// — same-origin absolute URLs (http://127.0.0.1:8000/leaders) are
///   common when browsers send the full Referer. The handler strips
///   the origin to keep the redirect relative — open-redirect
///   protection by construction.
#[tokio::test]
async fn l1_season_type_redirect_strips_localhost_origin() {
    let state = WebState::new();

    let (status, location) = flip_season_type(
        state,
        "playoff",
        Some("http://127.0.0.1:8000/player/8478402"),
    )
    .await;

    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location.as_deref(), Some("/player/8478402"));
}

/// l1_season_type_redirect_drops_off_site_referer
/// — open-redirect defense: a referer pointing somewhere external
///   (https://evil.example/x) MUST NOT become the Location target.
///   Falls through to "/" instead.
#[tokio::test]
async fn l1_season_type_redirect_drops_off_site_referer() {
    let state = WebState::new();

    let (status, location) =
        flip_season_type(state, "playoff", Some("https://evil.example/leaders")).await;

    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location.as_deref(), Some("/"));
}

/// l1_season_type_toggle_visible_in_global_nav
/// — render-time fence: the global-nav strip on every page MUST
///   show both options (Regular | Playoffs) with the active one
///   bolded. If the base.html toggle is ever removed by accident
///   this catches it.
#[tokio::test]
async fn l1_season_type_toggle_visible_in_global_nav() {
    let app = router(WebState::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");

    let body_bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("body");
    let body = std::str::from_utf8(&body_bytes).expect("utf-8");

    // Active option is bolded; inactive option is a link to the flip.
    assert!(
        body.contains("<strong>Regular</strong>"),
        "Default state has Regular active (bolded)"
    );
    assert!(
        body.contains("/season-type/playoff"),
        "Inactive option is a link to flip"
    );
    // Class hook for CSS — the toggle has its own class so a future
    // CSS refactor that drops the styling is detectable by other
    // means than a visual scan.
    assert!(body.contains("season-type-toggle"));
}

// ── Phase Foster.1 — date-anchored route smokes ────────────────────────────
//
// Network-free smokes: the handlers may fail to reach the NHL API in
// CI / offline test runs, but the page must still render (the
// fetch_error path lands in the template, not as a 500). What we
// pin here is "the route accepts ?date= / ?season= and returns
// 200 HTML". Future work can layer on httpmock-backed L1 fetches
// once we extract a NhlClient injection point.

#[tokio::test]
async fn l1_foster1_scores_accepts_past_date_query() {
    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/scores?date=2014-10-08")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "/scores must accept ?date= and render 200"
    );
}

#[tokio::test]
async fn l1_foster1_schedule_accepts_past_date_query() {
    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/schedule?date=2014-10-08")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "/schedule must accept ?date= (date-anchored slate path) and render 200"
    );
}

#[tokio::test]
async fn l1_foster1_playoffs_accepts_season_query() {
    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/playoffs?season=19931994")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "/playoffs must accept ?season= and render 200"
    );
}

// ── Phase Foster +9 — `?range=` smokes ──────────────────────────────────────

#[tokio::test]
async fn l1_foster_plus9_scores_accepts_range_week() {
    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/scores?date=2014-10-08&range=week")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "/scores must accept ?range=week"
    );
}

#[tokio::test]
async fn l1_foster_plus9_scores_accepts_range_month() {
    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/scores?date=2014-10-08&range=month")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn l1_foster_plus9_scores_accepts_range_day_default() {
    // Bare ?date= without ?range= should still 200 — `range=day` is
    // the implicit default per the spec convention.
    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/scores?date=2014-10-08")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");
    assert_eq!(response.status(), StatusCode::OK);
}

// ── Phase Conn Smythe C.3 — /game/:id smokes ────────────────────────────────

#[tokio::test]
async fn l1_conn_smythe_c3_game_route_accepts_id_and_returns_200() {
    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/game/2025020342")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");
    // Network is available or not — the handler renders an error
    // page in either case so the route always 200s.
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn l1_conn_smythe_c3_game_route_renders_html() {
    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/game/2025020342")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");
    let ct = response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .expect("content-type")
        .to_str()
        .unwrap();
    assert!(ct.starts_with("text/html"), "got: {ct}");
}

#[tokio::test]
async fn l1_conn_smythe_c3_game_json_envelope_shape() {
    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/game/2025020342")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body fits");
    let json: serde_json::Value =
        serde_json::from_slice(&bytes).expect("game response should be valid json");

    let obj = assert_data_meta_envelope(&json, "game");
    assert!(
        !obj.contains_key("error"),
        "game JSON envelope should carry fetch failures in meta.source_error"
    );
    assert_eq!(json["meta"]["game_id"], 2025020342_u64);
    assert!(json["meta"]
        .as_object()
        .is_some_and(|meta| meta.contains_key("source_error")));
    assert!(json["data"].is_object() || json["data"].is_null());
}

#[tokio::test]
async fn l1_foster_plus9_scores_unknown_range_defaults_to_day() {
    // Unknown range value should fall back to Day rather than 400.
    let app = router(WebState::new());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/scores?date=2014-10-08&range=garbage")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "unknown range falls back to Day, must still 200"
    );
}
