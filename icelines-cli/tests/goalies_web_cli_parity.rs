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

fn cli_leader_rows(sort: &str, season: u32, top: usize) -> Vec<(u64, String, String, u64, u64)> {
    let home = tempfile::TempDir::new().expect("temp home");
    let output = Command::new(icelines_bin())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env("ICELINES_NO_LIVE", "1")
        .env("ICELINES_TEST_MODE", "1")
        .args([
            "query",
            "leaders",
            "--sort",
            sort,
            "--season",
            &season.to_string(),
            "--top",
            &top.to_string(),
            "--json",
        ])
        .output()
        .expect("run icelines query leaders");
    assert!(
        output.status.success(),
        "query leaders failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("CLI leaders JSON parses");
    json.as_array()
        .expect("CLI emits top-level row array")
        .iter()
        .map(|row| {
            (
                row["nhl_id"].as_u64().unwrap(),
                row["name"].as_str().unwrap().to_string(),
                row["team_abbrev"].as_str().unwrap().to_string(),
                row["season_goals"].as_u64().unwrap(),
                row["season_pts"].as_u64().unwrap(),
            )
        })
        .collect()
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

fn cli_leader_source_context(sort: &str, season: u32, top: usize) -> (String, String, String) {
    let home = tempfile::TempDir::new().expect("temp home");
    let output = Command::new(icelines_bin())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env("ICELINES_NO_LIVE", "1")
        .env("ICELINES_TEST_MODE", "1")
        .args([
            "query",
            "leaders",
            "--sort",
            sort,
            "--season",
            &season.to_string(),
            "--top",
            &top.to_string(),
            "--json",
        ])
        .output()
        .expect("run icelines query leaders");
    assert!(
        output.status.success(),
        "query leaders failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("CLI leaders JSON parses");
    let row = json
        .as_array()
        .expect("CLI emits top-level row array")
        .first()
        .expect("fixture emits at least one leaders row");
    let source = &row["source_state"][0];
    (
        row["source_completeness"].as_str().unwrap().to_string(),
        source["source"].as_str().unwrap().to_string(),
        source["state"].as_str().unwrap().to_string(),
    )
}

fn cli_leader_active_context(sort: &str, season: u32, top: usize) -> (String, String) {
    let home = tempfile::TempDir::new().expect("temp home");
    let output = Command::new(icelines_bin())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env("ICELINES_NO_LIVE", "1")
        .env("ICELINES_TEST_MODE", "1")
        .args([
            "query",
            "leaders",
            "--sort",
            sort,
            "--season",
            &season.to_string(),
            "--top",
            &top.to_string(),
            "--json",
        ])
        .output()
        .expect("run icelines query leaders");
    assert!(
        output.status.success(),
        "query leaders failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("CLI leaders JSON parses");
    let row = json
        .as_array()
        .expect("CLI emits top-level row array")
        .first()
        .expect("fixture emits at least one leaders row");
    (
        row["season"].as_u64().unwrap().to_string(),
        row["season_type"].as_str().unwrap().to_string(),
    )
}

fn cli_leader_result_state(
    sort: &str,
    season: u32,
    top: usize,
) -> (u64, u64, u64, String, Vec<String>) {
    let home = tempfile::TempDir::new().expect("temp home");
    let output = Command::new(icelines_bin())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env("ICELINES_NO_LIVE", "1")
        .env("ICELINES_TEST_MODE", "1")
        .args([
            "query",
            "leaders",
            "--sort",
            sort,
            "--season",
            &season.to_string(),
            "--top",
            &top.to_string(),
            "--json",
        ])
        .output()
        .expect("run icelines query leaders");
    assert!(
        output.status.success(),
        "query leaders failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("CLI leaders JSON parses");
    let row = json
        .as_array()
        .expect("CLI emits top-level row array")
        .first()
        .expect("fixture emits at least one leaders row");
    (
        row["total"].as_u64().unwrap(),
        row["returned"].as_u64().unwrap(),
        row["top"].as_u64().unwrap(),
        sort.to_string(),
        row["active_filters"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_string())
            .collect(),
    )
}

fn cli_leader_filtered_result_state(
    sort: &str,
    season: u32,
    top: usize,
    filters: &[&str],
) -> (u64, u64, u64, String, Vec<String>) {
    let home = tempfile::TempDir::new().expect("temp home");
    let season = season.to_string();
    let top = top.to_string();
    let mut args = vec![
        "query".to_owned(),
        "leaders".to_owned(),
        "--sort".to_owned(),
        sort.to_owned(),
        "--season".to_owned(),
        season,
        "--top".to_owned(),
        top,
    ];
    for filter in filters {
        args.push("--filter".to_owned());
        args.push((*filter).to_owned());
    }
    args.push("--json-envelope".to_owned());
    let output = Command::new(icelines_bin())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env("ICELINES_NO_LIVE", "1")
        .env("ICELINES_TEST_MODE", "1")
        .args(args)
        .output()
        .expect("run icelines query leaders");
    assert!(
        output.status.success(),
        "query leaders failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("CLI leaders JSON envelope parses");
    (
        json["meta"]["total"].as_u64().unwrap(),
        json["meta"]["returned"].as_u64().unwrap(),
        json["meta"]["top"].as_u64().unwrap(),
        json["meta"]["sort"].as_str().unwrap().to_string(),
        json["meta"]["active_filters"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_string())
            .collect(),
    )
}

fn cli_leader_empty_warning_state(
    sort: &str,
    season: u32,
    top: usize,
    pos: &str,
) -> serde_json::Value {
    let home = tempfile::TempDir::new().expect("temp home");
    let output = Command::new(icelines_bin())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env("ICELINES_NO_LIVE", "1")
        .env("ICELINES_TEST_MODE", "1")
        .args([
            "query",
            "leaders",
            "--sort",
            sort,
            "--season",
            &season.to_string(),
            "--pos",
            pos,
            "--top",
            &top.to_string(),
            "--json-envelope",
        ])
        .output()
        .expect("run icelines query leaders");
    assert!(
        output.status.success(),
        "query leaders failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("CLI leaders JSON envelope parses")
}

fn cli_leader_text_output(sort: &str, season: u32, top: usize, pos: &str) -> String {
    let home = tempfile::TempDir::new().expect("temp home");
    let output = Command::new(icelines_bin())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env("ICELINES_NO_LIVE", "1")
        .env("ICELINES_TEST_MODE", "1")
        .args([
            "query",
            "leaders",
            "--sort",
            sort,
            "--season",
            &season.to_string(),
            "--pos",
            pos,
            "--top",
            &top.to_string(),
        ])
        .output()
        .expect("run icelines query leaders text");
    assert!(
        output.status.success(),
        "query leaders text failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("CLI leaders text is utf8")
}

async fn web_leader_rows(
    sort: &str,
    season: u32,
    top: usize,
) -> Vec<(u64, String, String, u64, u64)> {
    let store = SnapshotStore::new(SnapshotStore::default_root());
    let outcome = load_into_repo(Season(season), SeasonType::Regular, &store)
        .expect("bundled leaders fixture season loads");
    let state =
        WebState::with_repo_and_config(outcome.repo, WebConfig::new(season.to_string(), "regular"));
    let app = router(state);
    let uri = format!("/api/v1/leaders?sort={sort}&top={top}");
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
    let json: serde_json::Value = serde_json::from_slice(&body).expect("web leaders JSON parses");
    json["data"]
        .as_array()
        .expect("web leaders data array")
        .iter()
        .map(|row| {
            (
                row["nhl_id"].as_u64().unwrap(),
                row["name"].as_str().unwrap().to_string(),
                row["team_abbrev"].as_str().unwrap().to_string(),
                row["goals"].as_u64().unwrap(),
                row["points"].as_u64().unwrap(),
            )
        })
        .collect()
}

async fn web_leader_source_context(
    sort: &str,
    season: u32,
    top: usize,
) -> (String, String, String) {
    let store = SnapshotStore::new(SnapshotStore::default_root());
    let outcome = load_into_repo(Season(season), SeasonType::Regular, &store)
        .expect("bundled leaders fixture season loads");
    let state =
        WebState::with_repo_and_config(outcome.repo, WebConfig::new(season.to_string(), "regular"));
    let app = router(state);
    let uri = format!("/api/v1/leaders?sort={sort}&top={top}");
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
    let json: serde_json::Value = serde_json::from_slice(&body).expect("web leaders JSON parses");
    let source = &json["meta"]["source_state"][0];
    (
        json["meta"]["completeness"].as_str().unwrap().to_string(),
        source["source"].as_str().unwrap().to_string(),
        source["state"].as_str().unwrap().to_string(),
    )
}

async fn web_leader_active_context(sort: &str, season: u32, top: usize) -> (String, String) {
    let store = SnapshotStore::new(SnapshotStore::default_root());
    let outcome = load_into_repo(Season(season), SeasonType::Regular, &store)
        .expect("bundled leaders fixture season loads");
    let state =
        WebState::with_repo_and_config(outcome.repo, WebConfig::new(season.to_string(), "regular"));
    let app = router(state);
    let uri = format!("/api/v1/leaders?sort={sort}&top={top}");
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
    let json: serde_json::Value = serde_json::from_slice(&body).expect("web leaders JSON parses");
    (
        json["meta"]["season"].as_str().unwrap().to_string(),
        json["meta"]["season_type"].as_str().unwrap().to_string(),
    )
}

async fn web_leader_result_state(
    sort: &str,
    season: u32,
    top: usize,
) -> (u64, u64, u64, String, Vec<String>) {
    let store = SnapshotStore::new(SnapshotStore::default_root());
    let outcome = load_into_repo(Season(season), SeasonType::Regular, &store)
        .expect("bundled leaders fixture season loads");
    let state =
        WebState::with_repo_and_config(outcome.repo, WebConfig::new(season.to_string(), "regular"));
    let app = router(state);
    let uri = format!("/api/v1/leaders?sort={sort}&top={top}");
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
    let json: serde_json::Value = serde_json::from_slice(&body).expect("web leaders JSON parses");
    (
        json["meta"]["total"].as_u64().unwrap(),
        json["meta"]["returned"].as_u64().unwrap(),
        json["meta"]["top"].as_u64().unwrap(),
        json["meta"]["sort"].as_str().unwrap().to_string(),
        json["meta"]["active_filters"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_string())
            .collect(),
    )
}

async fn web_leader_empty_warning_state(
    sort: &str,
    season: u32,
    top: usize,
    pos: &str,
) -> serde_json::Value {
    let store = SnapshotStore::new(SnapshotStore::default_root());
    let outcome = load_into_repo(Season(season), SeasonType::Regular, &store)
        .expect("bundled leaders fixture season loads");
    let state =
        WebState::with_repo_and_config(outcome.repo, WebConfig::new(season.to_string(), "regular"));
    let app = router(state);
    let uri = format!("/api/v1/leaders?sort={sort}&pos={pos}&top={top}");
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
    serde_json::from_slice(&body).expect("web leaders JSON parses")
}

async fn web_leader_html_rows(
    sort: &str,
    season: u32,
    top: usize,
) -> Vec<(u64, String, String, u64, u64)> {
    let store = SnapshotStore::new(SnapshotStore::default_root());
    let outcome = load_into_repo(Season(season), SeasonType::Regular, &store)
        .expect("bundled leaders fixture season loads");
    let state =
        WebState::with_repo_and_config(outcome.repo, WebConfig::new(season.to_string(), "regular"));
    let app = router(state);
    let uri = format!("/leaders?sort={sort}&top={top}");
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
    let html = String::from_utf8(body.to_vec()).expect("leaders HTML is UTF-8");
    let row_re = regex::Regex::new(
        r#"<tr data-row-rank="\d+"\s+data-nhl-id="(?P<id>\d+)"\s+data-player-name="(?P<name>[^"]+)"\s+data-team-abbrev="(?P<team>[^"]+)"\s+data-goals="(?P<goals>\d+)"\s+data-points="(?P<points>\d+)">"#,
    )
    .expect("row regex compiles");

    row_re
        .captures_iter(&html)
        .map(|cap| {
            (
                cap["id"].parse().unwrap(),
                cap["name"].to_string(),
                cap["team"].to_string(),
                cap["goals"].parse().unwrap(),
                cap["points"].parse().unwrap(),
            )
        })
        .collect()
}

async fn web_leader_html_recovery(sort: &str, season: u32, top: usize, pos: &str) -> String {
    let store = SnapshotStore::new(SnapshotStore::default_root());
    let outcome = load_into_repo(Season(season), SeasonType::Regular, &store)
        .expect("bundled leaders fixture season loads");
    let state =
        WebState::with_repo_and_config(outcome.repo, WebConfig::new(season.to_string(), "regular"));
    let app = router(state);
    let uri = format!("/leaders?sort={sort}&pos={pos}&top={top}");
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
    String::from_utf8(body.to_vec()).expect("leaders HTML is UTF-8")
}

async fn web_leader_html(sort: &str, season: u32, top: usize) -> String {
    let store = SnapshotStore::new(SnapshotStore::default_root());
    let outcome = load_into_repo(Season(season), SeasonType::Regular, &store)
        .expect("bundled leaders fixture season loads");
    let state =
        WebState::with_repo_and_config(outcome.repo, WebConfig::new(season.to_string(), "regular"));
    let app = router(state);
    let uri = format!("/leaders?sort={sort}&top={top}");
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
    String::from_utf8(body.to_vec()).expect("leaders HTML is UTF-8")
}

async fn web_leader_html_with_filters(
    sort: &str,
    season: u32,
    top: usize,
    encoded_filters: &[&str],
) -> String {
    let store = SnapshotStore::new(SnapshotStore::default_root());
    let outcome = load_into_repo(Season(season), SeasonType::Regular, &store)
        .expect("bundled leaders fixture season loads");
    let state =
        WebState::with_repo_and_config(outcome.repo, WebConfig::new(season.to_string(), "regular"));
    let app = router(state);
    let filter_query = encoded_filters
        .iter()
        .map(|filter| format!("&filter={filter}"))
        .collect::<String>();
    let uri = format!("/leaders?sort={sort}&top={top}{filter_query}");
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
    String::from_utf8(body.to_vec()).expect("leaders HTML is UTF-8")
}

async fn web_leader_html_active_context(sort: &str, season: u32, top: usize) -> (String, String) {
    let html = web_leader_html(sort, season, top).await;
    let context_re = regex::Regex::new(
        r#"<p class="meta-line"\s+data-active-season="(?P<season>[^"]+)"\s+data-active-season-type="(?P<season_type>[^"]+)"[^>]*>"#,
    )
    .expect("context regex compiles");
    let captures = context_re
        .captures(&html)
        .expect("leaders HTML exposes active context");
    (
        captures["season"].to_string(),
        captures["season_type"].to_string(),
    )
}

async fn web_leader_html_source_context(
    sort: &str,
    season: u32,
    top: usize,
) -> (String, String, String) {
    let html = web_leader_html(sort, season, top).await;
    let source_re = regex::Regex::new(
        r#"data-source-kind="(?P<source>[^"]+)"\s+data-source-completeness="(?P<state>[^"]+)""#,
    )
    .expect("source regex compiles");
    let captures = source_re
        .captures(&html)
        .expect("leaders HTML exposes source context");
    (
        captures["state"].to_string(),
        captures["source"].to_string(),
        captures["state"].to_string(),
    )
}

async fn web_leader_html_result_state(
    sort: &str,
    season: u32,
    top: usize,
) -> (u64, u64, u64, String, String) {
    let html = web_leader_html(sort, season, top).await;
    let result_re = regex::Regex::new(
        r#"data-result-total="(?P<total>\d+)"\s+data-result-returned="(?P<returned>\d+)"\s+data-result-top="(?P<top>\d+)"\s+data-result-sort="(?P<sort>[^"]+)"\s+data-result-active-filters="(?P<active_filters>[^"]*)""#,
    )
    .expect("result regex compiles");
    let captures = result_re
        .captures(&html)
        .expect("leaders HTML exposes result state");
    (
        captures["total"].parse().expect("numeric total"),
        captures["returned"].parse().expect("numeric returned"),
        captures["top"].parse().expect("numeric top"),
        captures["sort"].to_string(),
        captures["active_filters"].to_string(),
    )
}

fn decode_html_attr(value: &str) -> String {
    value
        .replace("&gt;", ">")
        .replace("&#62;", ">")
        .replace("&#x3e;", ">")
        .replace("&#x3E;", ">")
        .replace("&lt;", "<")
        .replace("&#60;", "<")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&amp;", "&")
}

async fn web_leader_html_filtered_result_state(
    sort: &str,
    season: u32,
    top: usize,
    encoded_filters: &[&str],
) -> (u64, u64, u64, String, String) {
    let html = web_leader_html_with_filters(sort, season, top, encoded_filters).await;
    let result_re = regex::Regex::new(
        r#"data-result-total="(?P<total>\d+)"\s+data-result-returned="(?P<returned>\d+)"\s+data-result-top="(?P<top>\d+)"\s+data-result-sort="(?P<sort>[^"]+)"\s+data-result-active-filters="(?P<active_filters>[^"]*)""#,
    )
    .expect("result regex compiles");
    let captures = result_re
        .captures(&html)
        .expect("leaders HTML exposes filtered result state");
    (
        captures["total"].parse().expect("numeric total"),
        captures["returned"].parse().expect("numeric returned"),
        captures["top"].parse().expect("numeric top"),
        captures["sort"].to_string(),
        decode_html_attr(&captures["active_filters"]),
    )
}

async fn web_leader_html_active_filter_ui(
    sort: &str,
    season: u32,
    top: usize,
    encoded_filters: &[&str],
) -> (Vec<String>, String, String) {
    let html = web_leader_html_with_filters(sort, season, top, encoded_filters).await;
    let token_re =
        regex::Regex::new(r#"<code class="active-filter-token">(?P<filter>[^<]+)</code>"#)
            .expect("active filter token regex compiles");
    let active_tokens = token_re
        .captures_iter(&html)
        .map(|captures| decode_html_attr(&captures["filter"]))
        .collect::<Vec<_>>();
    let input_re = regex::Regex::new(
        r#"(?s)<input id="filter-1" type="text" name="filter"\s+placeholder="[^"]*"\s+value="(?P<filter>[^"]*)""#,
    )
    .expect("filter input regex compiles");
    let input = input_re
        .captures(&html)
        .expect("leaders HTML preserves active filter in filter input");
    let clear_re = regex::Regex::new(
        r#"(?s)<a href="(?P<href>\?sort=[^"]*)"\s+class="link-button-secondary">Clear</a>"#,
    )
    .expect("clear link regex compiles");
    let clear = clear_re
        .captures(&html)
        .expect("leaders HTML exposes active filter clear link");
    (
        active_tokens,
        decode_html_attr(&input["filter"]),
        decode_html_attr(&clear["href"]),
    )
}

async fn web_leader_html_empty_warning_metadata(
    sort: &str,
    season: u32,
    top: usize,
    pos: &str,
) -> (String, u64, String) {
    let html = web_leader_html_recovery(sort, season, top, pos).await;
    let state_re = regex::Regex::new(
        r#"data-empty-kind="(?P<empty>[^"]+)"\s+data-warning-count="(?P<count>\d+)"\s+data-warning-kinds="(?P<warnings>[^"]*)""#,
    )
    .expect("empty/warning regex compiles");
    let captures = state_re
        .captures(&html)
        .expect("leaders HTML exposes empty/warning metadata");
    assert!(html.contains(r#"data-empty-state="leaders" data-empty-kind="no_rows""#));
    assert!(html.contains(r#"data-warning-kind="unsupported_filter""#));
    (
        captures["empty"].to_string(),
        captures["count"].parse().expect("numeric warning count"),
        captures["warnings"].to_string(),
    )
}

async fn web_leader_html_active_position_chip(
    sort: &str,
    season: u32,
    top: usize,
    pos: &str,
) -> (String, String, u64) {
    let html = web_leader_html_recovery(sort, season, top, pos).await;
    assert_eq!(
        html.matches(r#"aria-current="true""#).count(),
        1,
        "leaders HTML should mark exactly one active position chip"
    );
    let active_chip_re = regex::Regex::new(
        r#"(?s)<a href="\?sort=[^"]*&pos=(?P<pos>[^&"]*)&top=(?P<top>\d+)[^"]*"\s+class="filter-chip fit-solid"\s+aria-current="true">\s*(?P<label>[^<\s]+)\s*</a>"#,
    )
    .expect("active chip regex compiles");
    let captures = active_chip_re
        .captures(&html)
        .expect("leaders HTML exposes active position chip");
    (
        captures["label"].to_string(),
        captures["pos"].to_string(),
        captures["top"].parse().expect("numeric top"),
    )
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
async fn l2_query_leaders_cli_and_web_stable_identity_match() {
    let season = 20242025;
    let top = 5;
    let cli_rows = cli_leader_rows("goals", season, top);
    let web_rows = web_leader_rows("goals", season, top).await;

    assert!(!cli_rows.is_empty(), "fixture should return leaders rows");
    assert_eq!(cli_rows, web_rows);
}

#[tokio::test]
async fn l2_query_leaders_cli_and_web_html_stable_identity_match() {
    let season = 20242025;
    let top = 5;
    let cli_rows = cli_leader_rows("goals", season, top);
    let web_rows = web_leader_html_rows("goals", season, top).await;

    assert!(!cli_rows.is_empty(), "fixture should return leaders rows");
    assert_eq!(cli_rows, web_rows);
}

#[tokio::test]
async fn l2_query_leaders_cli_and_web_source_state_match() {
    let season = 20242025;
    let top = 5;
    let cli_source = cli_leader_source_context("goals", season, top);
    let web_source = web_leader_source_context("goals", season, top).await;

    assert_eq!(
        cli_source,
        ("complete".into(), "roster".into(), "complete".into())
    );
    assert_eq!(cli_source, web_source);
}

#[tokio::test]
async fn l2_query_leaders_cli_and_web_active_context_match() {
    let season = 20242025;
    let top = 5;
    let cli_context = cli_leader_active_context("goals", season, top);
    let web_context = web_leader_active_context("goals", season, top).await;

    assert_eq!(cli_context, ("20242025".into(), "regular".into()));
    assert_eq!(cli_context, web_context);
}

#[tokio::test]
async fn l2_query_leaders_cli_json_and_web_html_active_context_match() {
    let season = 20242025;
    let top = 5;
    let cli_context = cli_leader_active_context("goals", season, top);
    let web_context = web_leader_html_active_context("goals", season, top).await;

    assert_eq!(cli_context, ("20242025".into(), "regular".into()));
    assert_eq!(cli_context, web_context);
}

#[tokio::test]
async fn l2_query_leaders_cli_json_and_web_html_source_state_match() {
    let season = 20242025;
    let top = 5;
    let cli_source = cli_leader_source_context("goals", season, top);
    let web_source = web_leader_html_source_context("goals", season, top).await;

    assert_eq!(
        cli_source,
        ("complete".into(), "roster".into(), "complete".into())
    );
    assert_eq!(cli_source, web_source);
}

#[tokio::test]
async fn l2_query_leaders_cli_and_web_result_state_match() {
    let season = 20242025;
    let top = 5;
    let cli_state = cli_leader_result_state("goals", season, top);
    let web_state = web_leader_result_state("goals", season, top).await;

    assert_eq!(cli_state.1, top as u64);
    assert_eq!(cli_state.2, top as u64);
    assert_eq!(cli_state.3, "goals");
    assert!(cli_state.4.is_empty());
    assert_eq!(cli_state, web_state);
}

#[tokio::test]
async fn l2_query_leaders_cli_json_and_web_html_result_state_match() {
    let season = 20242025;
    let top = 5;
    let cli_state = cli_leader_result_state("goals", season, top);
    let web_state = web_leader_html_result_state("goals", season, top).await;

    assert_eq!(cli_state.1, top as u64);
    assert_eq!(cli_state.2, top as u64);
    assert_eq!(cli_state.3, "goals");
    assert!(cli_state.4.is_empty());
    assert_eq!(
        (
            cli_state.0,
            cli_state.1,
            cli_state.2,
            cli_state.3,
            "-".to_string()
        ),
        web_state
    );
}

#[tokio::test]
async fn l2_query_leaders_cli_json_and_web_html_active_filter_state_match() {
    let season = 20242025;
    let top = 5;
    let filter = "goals>=1";
    let cli_state = cli_leader_filtered_result_state("goals", season, top, &[filter]);
    let web_state =
        web_leader_html_filtered_result_state("goals", season, top, &["goals%3E%3D1"]).await;

    assert_eq!(cli_state.1, top as u64);
    assert_eq!(cli_state.2, top as u64);
    assert_eq!(cli_state.3, "goals");
    assert_eq!(cli_state.4, vec![filter.to_owned()]);
    assert_eq!(
        (
            cli_state.0,
            cli_state.1,
            cli_state.2,
            cli_state.3,
            cli_state.4.join(";"),
        ),
        web_state
    );
}

#[tokio::test]
async fn l2_query_leaders_cli_json_and_web_html_active_filter_ui_match() {
    let season = 20242025;
    let top = 5;
    let filter = "goals>=1";
    let cli_state = cli_leader_filtered_result_state("goals", season, top, &[filter]);
    let web_state = web_leader_html_active_filter_ui("goals", season, top, &["goals%3E%3D1"]).await;

    assert_eq!(cli_state.4, vec![filter.to_owned()]);
    assert_eq!(web_state.0, cli_state.4);
    assert_eq!(web_state.1, filter);
    assert!(web_state.2.contains("sort=goals"));
    assert!(web_state.2.contains("top=5"));
    assert!(
        !web_state.2.contains("filter="),
        "clear link should remove active query filters: {}",
        web_state.2
    );
}

#[tokio::test]
async fn l2_query_leaders_cli_and_web_empty_warning_state_match() {
    let season = 20242025;
    let top = 5;
    let cli_state = cli_leader_empty_warning_state("goals", season, top, "G");
    let web_state = web_leader_empty_warning_state("goals", season, top, "G").await;

    assert!(cli_state["data"].as_array().unwrap().is_empty());
    assert_eq!(cli_state["data"], web_state["data"]);
    assert_eq!(cli_state["meta"]["total"], web_state["meta"]["total"]);
    assert_eq!(cli_state["meta"]["returned"], web_state["meta"]["returned"]);
    assert_eq!(cli_state["meta"]["top"], web_state["meta"]["top"]);
    assert_eq!(
        cli_state["meta"]["position_filter"],
        web_state["meta"]["position_filter"]
    );
    assert_eq!(
        cli_state["meta"]["empty_state"],
        web_state["meta"]["empty_state"]
    );
    assert_eq!(cli_state["meta"]["warnings"], web_state["meta"]["warnings"]);
    assert_eq!(cli_state["meta"]["empty_state"]["kind"], "no_rows");
    assert_eq!(
        cli_state["meta"]["warnings"][0]["kind"],
        "unsupported_filter"
    );
}

#[tokio::test]
async fn l2_query_leaders_cli_json_and_web_html_recovery_match() {
    let season = 20242025;
    let top = 5;
    let cli_state = cli_leader_empty_warning_state("goals", season, top, "G");
    let html = web_leader_html_recovery("goals", season, top, "G").await;

    let empty = &cli_state["meta"]["empty_state"];
    let warning = &cli_state["meta"]["warnings"][0];
    let recovery_label = empty["recovery"][0]["label"].as_str().unwrap();
    let recovery_route = empty["recovery"][0]["action"]["open_route"]["route"]
        .as_str()
        .unwrap();

    assert!(html.contains(empty["title"].as_str().unwrap()));
    assert!(html.contains(empty["detail"].as_str().unwrap()));
    assert!(html.contains(warning["message"].as_str().unwrap()));
    assert!(html.contains(recovery_label));
    assert!(html.contains(&format!(r#"href="/{recovery_route}""#)));
    assert!(!html.contains(r#"data-row-rank=""#));
}

#[test]
fn l2_query_leaders_cli_text_renders_empty_warning_recovery_state() {
    let season = 20242025;
    let top = 5;
    let cli_state = cli_leader_empty_warning_state("goals", season, top, "G");
    let text = cli_leader_text_output("goals", season, top, "G");
    let empty = &cli_state["meta"]["empty_state"];
    let warning = &cli_state["meta"]["warnings"][0];
    let recovery_label = empty["recovery"][0]["label"].as_str().unwrap();
    let recovery_route = empty["recovery"][0]["action"]["open_route"]["route"]
        .as_str()
        .unwrap();

    assert!(text.contains("Warning: unsupported_filter"));
    assert!(text.contains(warning["message"].as_str().unwrap()));
    assert!(text.contains("Empty: no_rows"));
    assert!(text.contains(empty["title"].as_str().unwrap()));
    assert!(text.contains(empty["detail"].as_str().unwrap()));
    assert!(text.contains(&format!("Recovery: {recovery_label} -> /{recovery_route}")));
}

#[tokio::test]
async fn l2_query_leaders_cli_json_and_web_html_empty_warning_metadata_match() {
    let season = 20242025;
    let top = 5;
    let cli_state = cli_leader_empty_warning_state("goals", season, top, "G");
    let web_state = web_leader_html_empty_warning_metadata("goals", season, top, "G").await;
    let cli_warnings = cli_state["meta"]["warnings"]
        .as_array()
        .expect("CLI warnings are an array");
    let cli_warning_kinds = cli_warnings
        .iter()
        .map(|warning| warning["kind"].as_str().expect("warning kind"))
        .collect::<Vec<_>>()
        .join(";");

    assert_eq!(
        (
            cli_state["meta"]["empty_state"]["kind"]
                .as_str()
                .expect("empty kind")
                .to_string(),
            cli_warnings.len() as u64,
            cli_warning_kinds,
        ),
        web_state
    );
    assert_eq!(web_state.0, "no_rows");
    assert_eq!(web_state.1, 1);
    assert_eq!(web_state.2, "unsupported_filter");
}

#[tokio::test]
async fn l2_query_leaders_cli_json_and_web_html_active_position_chip_match() {
    let season = 20242025;
    let top = 5;
    let cli_state = cli_leader_empty_warning_state("goals", season, top, "G");
    let web_state = web_leader_html_active_position_chip("goals", season, top, "G").await;
    let cli_pos = cli_state["meta"]["position_filter"]
        .as_str()
        .expect("CLI envelope carries selected position filter");

    assert_eq!(
        (cli_pos.to_string(), cli_pos.to_string(), top as u64),
        web_state
    );
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
