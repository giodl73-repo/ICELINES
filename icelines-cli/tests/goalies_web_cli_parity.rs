use std::path::PathBuf;
use std::process::Command;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_fetch::snapshot::SnapshotStore;
use icelines_fetch::stats_loader::load_into_repo;
use icelines_web::config::WebConfig;
use icelines_web::{router, WebState};
use tower::util::ServiceExt;

fn icelines_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_icelines"))
}

fn cli_goalie_rows(sort: &str, season: u32, top: usize) -> Vec<(String, String, u64, u64, u64)> {
    let home = tempfile::TempDir::new().expect("temp home");
    let output = Command::new(icelines_bin())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env("ICELINES_NO_LIVE", "1")
        .env("ICELINES_TEST_MODE", "1")
        .args([
            "query",
            "goalies",
            "--sort",
            sort,
            "--season",
            &season.to_string(),
            "--min-gp",
            "15",
            "--top",
            &top.to_string(),
            "--json",
        ])
        .output()
        .expect("run icelines query goalies");
    assert!(
        output.status.success(),
        "query goalies failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("CLI goalies JSON parses");
    json.as_array()
        .expect("CLI emits top-level row array")
        .iter()
        .map(|row| {
            (
                row["full_name"].as_str().unwrap().to_string(),
                row["team"].as_str().unwrap().to_string(),
                row["games_played"].as_u64().unwrap(),
                row["wins"].as_u64().unwrap(),
                row["saves"].as_u64().unwrap(),
            )
        })
        .collect()
}

async fn web_goalie_rows(
    sort: &str,
    season: u32,
    top: usize,
) -> Vec<(String, String, u64, u64, u64)> {
    let store = SnapshotStore::new(SnapshotStore::default_root());
    let outcome = load_into_repo(Season(season), SeasonType::Regular, &store)
        .expect("bundled goalie fixture season loads");
    let state =
        WebState::with_repo_and_config(outcome.repo, WebConfig::new(season.to_string(), "regular"));
    let app = router(state);
    let uri = format!("/api/v1/goalies?sort={sort}&gp_min=15&top={top}");
    let response = app
        .oneshot(
            Request::builder()
                .uri(uri)
                .body(Body::empty())
                .expect("request builder ok"),
        )
        .await
        .expect("oneshot dispatch ok");
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("body");
    let json: serde_json::Value = serde_json::from_slice(&body).expect("web goalies JSON parses");
    json["data"]
        .as_array()
        .expect("web goalies data array")
        .iter()
        .map(|row| {
            (
                row["name"].as_str().unwrap().to_string(),
                row["team"].as_str().unwrap().to_string(),
                row["games"].as_u64().unwrap(),
                row["wins"].as_u64().unwrap(),
                row["saves"].as_u64().unwrap(),
            )
        })
        .collect()
}

#[tokio::test]
async fn l2_query_goalies_cli_and_web_row_identity_match() {
    let season = 20242025;
    let top = 5;
    let cli_rows = cli_goalie_rows("saves", season, top);
    let web_rows = web_goalie_rows("saves", season, top).await;

    assert!(!cli_rows.is_empty(), "fixture should return goalie rows");
    assert_eq!(cli_rows, web_rows);
}
