//! Bio-form regressions: real blank submissions, calendar cohorts and fixed season age.
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use icelines_core::{fixtures, stats_repository::StatsRepository, CURRENT_SEASON};
use icelines_web::{router, WebState};
use tower::ServiceExt;

async fn get(uri: &str) -> (StatusCode, String) {
    let mut repo = StatsRepository::new();
    for (id, date) in [
        (1, Some("2001-01-01")),
        (2, Some("2001-12-31")),
        (3, Some("2002-02-01")),
        (4, None),
    ] {
        let mut identity = fixtures::identity(id)
            .name(
                &format!("Sample Player {id}"),
                &format!("sample player {id}"),
            )
            .build();
        identity.bio.birth_date = date.map(str::to_owned);
        repo.upsert_identity(identity).unwrap();
        let mut stats = fixtures::stats(id, CURRENT_SEASON, "ALP").build();
        stats.totals.pp_points = if id == 1 { 70 } else { 0 };
        stats.totals.pp_goals = if id == 1 { 25 } else { 10 };
        stats.totals.sh_points = 0;
        repo.upsert_stats(stats).unwrap();
    }
    let response = router(WebState::with_repo(repo))
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

#[tokio::test]
async fn blank_bio_fields_accept_real_age_form_submission() {
    let form = "sort=points&pos=&top=20&age-min=25&age-max=&birth-year-min=&birth-year-max=&draft-min=&draft-max=&height-min=&height-max=&weight-min=&weight-max=&country=&shoots=";
    for route in ["/leaders", "/api/v1/leaders"] {
        let (status, body) = get(&format!("{route}?{form}")).await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }
    for query in ["age-min=nope", "birth-year-min=oops"] {
        assert_eq!(
            get(&format!("/leaders?{query}")).await.0,
            StatusCode::BAD_REQUEST
        );
    }
}

#[tokio::test]
async fn birth_year_and_age_intersect_without_conflating_cohorts() {
    let (status, body) = get("/api/v1/leaders?birth-year-min=2001&birth-year-max=2001").await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["meta"]["total"], 2);
    let (status, body) =
        get("/api/v1/leaders?birth-year-min=2001&birth-year-max=2001&age-min=25&age-max=25").await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["meta"]["total"], 1);
    assert_eq!(json["data"][0]["nhl_id"], 2);
}

#[tokio::test]
async fn birth_year_form_and_sort_links_preserve_cohort_and_explain_age() {
    let (status, body) = get("/leaders?birth-year-min=2001&birth-year-max=2001").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Season age · Jan 31"));
    assert!(body.contains("January 31, 2027"));
    assert!(!body.contains("hockey-standard Sept 15"));
    assert!(body.contains("name=\"birth-year-min\""));
    assert!(body.contains("birth-year-min=2001") && body.contains("birth-year-max=2001"));
    assert!(body.contains("Minimum birth year"));
}

#[tokio::test]
async fn dashboard_workspace_keeps_birth_year_filters() {
    let (status, body) =
        get("/dashboard?workspace=%2Fleaders%3Fbirth-year-min%3D2001%26birth-year-max%3D2001")
            .await;
    assert_eq!(status, StatusCode::OK);
    let workspace = body.split("data-workspace-url=").nth(1).unwrap();
    let table = workspace
        .split("<tbody>")
        .nth(1)
        .unwrap()
        .split("</tbody>")
        .next()
        .unwrap();
    assert!(table.contains("Sample Player 1"));
    assert!(table.contains("Sample Player 2"));
    assert!(!table.contains("Sample Player 3"));
    assert!(!body.contains("hockey-standard Sept 15"));
    assert!(body.contains("name=\"birth-year-min\""));
}

#[tokio::test]
async fn even_strength_mode_counts_and_ranks_points_without_changing_other_totals() {
    let (_, body) =
        get("/api/v1/leaders?points-mode=even-strength&birth-year-min=2001&birth-year-max=2001")
            .await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["meta"]["points_mode"], "even-strength");
    assert_eq!(json["data"][0]["nhl_id"], 2);
    assert_eq!(json["data"][0]["points"], 80);
    assert_eq!(json["data"][1]["nhl_id"], 1);
    assert_eq!(json["data"][1]["points"], 10);
    assert_eq!(json["data"][1]["goals"], 30);
    assert_eq!(json["data"][0]["ev_goals"], 20);
    assert_eq!(json["data"][0]["ev_assists"], 60);
    assert_eq!(json["data"][1]["ev_goals"], 5);
    assert_eq!(json["data"][1]["ev_assists"], 5);
    let (_, body) =
        get("/api/v1/leaders?points-mode=all&birth-year-min=2001&birth-year-max=2001").await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["data"][0]["nhl_id"], 1);
    assert_eq!(json["data"][0]["points"], 80);
    let (_, body) = get("/leaders?points-mode=even-strength").await;
    assert!(body.contains("Points counted"));
    assert!(body.contains("EV P/GP"));
    assert!(body.contains(">EV G"));
    assert!(body.contains(">EV A"));
    assert!(body.contains("All G"));
    assert!(body.contains("All A"));
    assert!(body.contains("points-mode=even-strength"));
}

#[tokio::test]
async fn screenshot_caption_shows_applied_filters_above_column_headings() {
    let (status, body) = get("/leaders?age-min=25&age-max=25&birth-year-min=2001&birth-year-max=2001&points-mode=even-strength").await;
    assert_eq!(status, StatusCode::OK);
    let caption_start = body
        .find("<caption class=\"query-context-caption\">")
        .unwrap();
    let caption = body[caption_start..].split("</caption>").next().unwrap();
    assert!(caption.contains("Age: 25 – 25"));
    assert!(caption.contains("Birth year: 2001 – 2001"));
    assert!(caption.contains("Window:"));
    assert!(caption.contains("Points: Even strength"));
    assert!(caption_start < body.find("Sort by Player").unwrap());
    assert!(!get("/leaders").await.1.contains("Age = 25"));
}

#[tokio::test]
async fn configurable_rank_ranges_keep_cohort_heading_and_original_ranks() {
    let caption = |body: &str| {
        body.split("<caption class=\"query-context-caption\">")
            .nth(1)
            .unwrap()
            .split("</caption>")
            .next()
            .unwrap()
            .to_owned()
    };
    let (_, full) = get("/leaders?points-mode=even-strength&top=4").await;
    let (status, first) = get("/leaders?points-mode=even-strength&rank-from=1&rank-to=2").await;
    assert_eq!(status, StatusCode::OK);
    let (status, second) = get("/leaders?points-mode=even-strength&rank-from=3&rank-to=4").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(caption(&first), caption(&second));
    assert_eq!(caption(&full), caption(&second));
    assert!(second.contains("data-row-rank=\"3\""));
    assert!(second.contains("data-row-rank=\"4\""));
    assert!(!second.contains("data-row-rank=\"1\""));
    assert!(second.contains("&amp;rank-from=3&amp;rank-to=4"));
    let (_, all) = get("/api/v1/leaders?points-mode=even-strength&top=4").await;
    let (_, part) =
        get("/api/v1/leaders?points-mode=even-strength&top=1&rank-from=3&rank-to=4").await;
    let all: serde_json::Value = serde_json::from_str(&all).unwrap();
    let part: serde_json::Value = serde_json::from_str(&part).unwrap();
    assert_eq!(
        part["data"],
        serde_json::json!([all["data"][2], all["data"][3]])
    );
    assert_eq!(part["meta"]["total"], 4);
    for query in [
        "rank-from=0",
        "rank-from=3&rank-to=2",
        "rank-from=1&rank-to=501",
        "rank-from=nope",
    ] {
        for route in ["/leaders", "/api/v1/leaders"] {
            assert_eq!(
                get(&format!("{route}?{query}")).await.0,
                StatusCode::BAD_REQUEST
            );
        }
    }
    assert_eq!(get("/leaders?rank-from=&rank-to=").await.0, StatusCode::OK);
    assert_eq!(
        get("/leaders?rank-from=900&rank-to=910").await.0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn recovered_season_controls_do_not_silently_change_date_window_meaning() {
    for route in ["/leaders", "/api/v1/leaders"] {
        for query in [
            "birth-year-min=2001",
            "points-mode=even-strength",
            "rank-from=2",
        ] {
            assert_eq!(
                get(&format!(
                    "{route}?date-from=2026-10-01&date-to=2026-10-02&{query}"
                ))
                .await
                .0,
                StatusCode::BAD_REQUEST
            );
        }
    }
}
#[tokio::test]
async fn shared_sort_and_csv_keep_recovered_rank_and_points_context() {
    let (_, body)=get("/leaders?points-mode=even-strength&rank-from=2&rank-to=3&sort=name&order=asc&birth-year-min=2001").await;
    assert!(body.contains("query-selection-controls"));
    assert!(body.contains("data-row-rank=\"2\""));
    assert!(body.contains("points-mode=even-strength"));
    assert!(body.contains("birth-year-min=2001"));
    let (status,csv)=get("/api/v1/leaders?points-mode=even-strength&rank-from=2&rank-to=3&sort=name&order=asc&birth-year-min=2001&format=csv").await;
    assert_eq!(status, StatusCode::OK);
    assert!(csv.contains("\"EV P\""));
    assert!(csv.contains("\"EV G\""));
    assert!(csv.lines().nth(1).unwrap().starts_with("\"2\""));
}
