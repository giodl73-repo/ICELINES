use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use icelines_core::{parse_card_document, CardDocumentView, CANONICAL_TEAMS};
use icelines_web::{router, WebState};
use tower::util::ServiceExt;

async fn body(response: axum::response::Response) -> String {
    let bytes = to_bytes(response.into_body(), 2 * 1024 * 1024)
        .await
        .expect("team card response body");
    String::from_utf8(bytes.to_vec()).expect("UTF-8 response")
}

#[tokio::test]
async fn json_route_returns_the_complete_sealed_core_document() {
    let app = router(WebState::new());
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/cards/team-prognosis/20262027/NYR?scenario=nyr-development-variance")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let etag = response
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert_eq!(
        response.headers()["cache-control"],
        "public, max-age=300, must-revalidate"
    );
    let json = body(response).await;
    let card = parse_card_document(&json).expect("route should preserve the document seal");
    let fixture: CardDocumentView = serde_json::from_str(include_str!(
        "../../examples/team-prognosis-card-nyr-2026-27.json"
    ))
    .unwrap();
    assert_eq!(card, fixture);
    assert_eq!(card.pages.len(), 2);
    assert_eq!(card.context.joins.team_ids, ["NYR"]);

    let not_modified = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cards/team-prognosis/20262027/NYR?scenario=nyr-development-variance")
                .header("if-none-match", etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(not_modified.status(), StatusCode::NOT_MODIFIED);
}

#[tokio::test]
async fn html_tabs_are_bookmarkable_and_render_from_the_same_card() {
    let app = router(WebState::new());
    let depth = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(
                    "/icecast/20262027/NYR/card?scenario=nyr-development-variance&page=depth-chart",
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(depth.status(), StatusCode::OK);
    let depth = body(depth).await;
    assert!(depth.contains("The Depth Chart"));
    assert!(depth.contains("Tye Kartye"));
    assert!(depth.contains("player:8481789") || depth.contains("8481789.png"));
    assert!(depth.contains("aria-current=\"page\">The Depth Chart"));
    assert!(depth.contains("View source JSON"));

    let insider = app
        .oneshot(
            Request::builder()
                .uri("/icecast/20262027/NYR/card?scenario=nyr-development-variance&page=insider")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(insider.status(), StatusCode::OK);
    let insider = body(insider).await;
    assert!(insider.contains("The Insider"));
    assert!(insider.contains("Internal ceiling — +15 Path"));
    assert!(insider.contains("Best breakout upside"));
    assert!(insider.contains("aria-current=\"page\">The Insider"));
}

#[tokio::test]
async fn kraken_and_unsupported_dimensions_are_explicit() {
    let app = router(WebState::new());
    let kraken = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/cards/team-prognosis/20262027/SEA")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(kraken.status(), StatusCode::OK);
    let kraken: serde_json::Value = serde_json::from_str(&body(kraken).await).unwrap();
    assert_eq!(kraken["context"]["joins"]["team_ids"][0], "SEA");

    for (uri, status) in [
        (
            "/api/v1/cards/team-prognosis/20252026/NYR",
            StatusCode::NOT_FOUND,
        ),
        (
            "/api/v1/cards/team-prognosis/20262027/BOS",
            StatusCode::NOT_FOUND,
        ),
        (
            "/api/v1/cards/team-prognosis/20262027/NYR?scenario=sea-development-variance",
            StatusCode::BAD_REQUEST,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), status, "{uri}");
        let error: serde_json::Value = serde_json::from_str(&body(response).await).unwrap();
        assert_eq!(error["schema"], "card_error.v1");
    }
}

#[tokio::test]
async fn prospect_arrival_routes_render_the_same_sealed_card() {
    let app = router(WebState::new());
    let uri = "/api/v1/cards/prospect-arrival/20262027/NYR";
    let response = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let etag = response.headers()["etag"].to_str().unwrap().to_owned();
    let routed = parse_card_document(&body(response).await).unwrap();
    let fixture = parse_card_document(include_str!(
        "../../examples/prospect-arrival-card-nyr-2026-27.json"
    ))
    .unwrap();
    assert_eq!(routed, fixture);
    assert_eq!(routed.card_kind, icelines_core::CardKind::ProspectArrival);
    assert_eq!(etag, format!("\"{}\"", fixture.fingerprint));

    let depth = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/icecast/20262027/NYR/prospect-arrivals?page=depth-chart")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(depth.status(), StatusCode::OK);
    let depth = body(depth).await;
    assert!(depth.contains("The Depth Chart"));
    assert!(depth.contains("Cole Beaudoin"));
    assert!(depth.contains("Calibrated arrival outlook"));
    assert!(depth.contains("View source JSON"));

    let insider = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/icecast/20262027/NYR/prospect-arrivals?page=insider")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(insider.status(), StatusCode::OK);
    let insider = body(insider).await;
    assert!(insider.contains("The Insider"));
    assert!(insider.contains("Exclusion ledger"));
    assert!(insider.contains("No sealed prospect population source package was supplied"));
    assert!(insider.contains("Source authority"));

    for (team, team_name) in CANONICAL_TEAMS {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/cards/prospect-arrival/20262027/{team}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{team}");
        let card = parse_card_document(&body(response).await).unwrap();
        assert_eq!(card.context.joins.team_ids, [*team]);
        assert_eq!(card.title, format!("{team_name} prospect arrivals"));
    }

    let unsupported = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cards/prospect-arrival/20262027/XYZ")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unsupported.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn prospect_arrival_board_routes_preserve_withheld_ranks_and_team_drilldown() {
    let app = router(WebState::new());
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/prospect-arrivals/20262027")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let etag = response.headers()["etag"].to_str().unwrap().to_owned();
    let routed: icelines_core::ProspectArrivalBoardView =
        serde_json::from_str(&body(response).await).unwrap();
    let fixture: icelines_core::ProspectArrivalBoardView = serde_json::from_str(include_str!(
        "../../examples/prospect-arrival-board-2026-27.json"
    ))
    .unwrap();
    assert_eq!(routed, fixture);
    assert_eq!(routed.teams.len(), CANONICAL_TEAMS.len());
    assert!(routed.teams.iter().all(|team| team.rank.is_none()));
    assert_eq!(etag, format!("\"{}\"", fixture.fingerprint));

    let html = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/icecast/20262027/prospect-arrivals")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(html.status(), StatusCode::OK);
    let html = body(html).await;
    assert!(html.contains("Prospect Arrival Board"));
    assert!(html.contains("Rank state: <strong>Withheld</strong>"));
    assert!(html.contains("2 eligible skaters lack a comparable arrival calibration"));
    assert!(html.contains("8/10 eligible skaters calibrated"));
    assert!(html.contains("157 established NHL players rerouted"));
    assert!(html.contains("CalibrationDistance</strong>: 2"));
    assert!(html.contains("/icecast/20262027/NYR/prospect-arrivals"));
    assert!(html.contains("Expected arrivals"));

    let focused = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/icecast/20262027/prospect-arrivals?team=SEA")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let focused = body(focused).await;
    assert!(focused.contains("/icecast/20262027/SEA/prospect-arrivals"));
    assert!(!focused.contains("/icecast/20262027/NYR/prospect-arrivals"));

    let unsupported = app
        .oneshot(
            Request::builder()
                .uri("/icecast/20262027/prospect-arrivals?team=XYZ")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unsupported.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn prospect_census_readiness_routes_preserve_gates_and_team_focus() {
    let app = router(WebState::new());
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/prospect-census-readiness/20262027")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let etag = response.headers()["etag"].to_str().unwrap().to_owned();
    let routed: icelines_core::ProspectCensusReadinessBoardView =
        serde_json::from_str(&body(response).await).unwrap();
    let fixture: icelines_core::ProspectCensusReadinessBoardView = serde_json::from_str(
        include_str!("../../examples/prospect-census-readiness-2026-27.json"),
    )
    .unwrap();
    assert_eq!(routed, fixture);
    assert_eq!(etag, format!("\"{}\"", fixture.fingerprint));
    assert_eq!(routed.teams.len(), CANONICAL_TEAMS.len());
    assert_eq!(routed.population_complete_organizations, 0);
    assert_eq!(routed.published_organizations, 0);

    let html = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/icecast/20262027/prospect-census-readiness")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(html.status(), StatusCode::OK);
    let html = body(html).await;
    assert!(html.contains("Prospect Census Readiness"));
    assert!(html.contains("<strong>0/32</strong> population complete"));
    assert!(html.contains("2477 discovered → 807 canonical identities → 0 controlled"));
    assert!(html.contains("ahl_current_assignment"));
    assert!(html.contains("UnresolvedIdentity</strong>: 1670 players"));
    assert!(html.contains("NYR"));
    assert!(html.contains("SEA"));

    let focused = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/icecast/20262027/prospect-census-readiness?team=SEA")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let focused = body(focused).await;
    assert!(focused.contains("<th scope=\"row\">SEA</th>"));
    assert!(!focused.contains("<th scope=\"row\">NYR</th>"));

    let unsupported = app
        .oneshot(
            Request::builder()
                .uri("/icecast/20262027/prospect-census-readiness?team=XYZ")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unsupported.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn stylesheet_contains_phone_and_tablet_card_layouts() {
    let response = router(WebState::new())
        .oneshot(
            Request::builder()
                .uri("/static/style.css")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let css = body(response).await;
    assert!(css.contains(".team-prognosis-card"));
    assert!(css.contains("max-width: 760px"));
    assert!(css.contains("max-width: 420px"));
}

#[tokio::test]
async fn fantasy_json_route_returns_the_exact_sealed_roster_document_and_etag() {
    let app = router(WebState::new());
    let uri = "/api/v1/cards/fantasy-roster/dexters-dawgs";
    let response = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let etag = response.headers()["etag"].to_str().unwrap().to_string();
    let routed = parse_card_document(&body(response).await).unwrap();
    let fixture = parse_card_document(include_str!(
        "../../examples/fantasy-roster-card-dexters-dawgs-2026-10-05.json"
    ))
    .unwrap();
    assert_eq!(routed, fixture);
    assert_eq!(etag, format!("\"{}\"", fixture.fingerprint));

    let not_modified = app
        .oneshot(
            Request::builder()
                .uri(uri)
                .header("if-none-match", etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(not_modified.status(), StatusCode::NOT_MODIFIED);
}

#[tokio::test]
async fn fantasy_html_pages_preserve_lineup_rules_classes_and_fixture_warning() {
    let app = router(WebState::new());
    let roster = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/fantasy/cards/roster/dexters-dawgs?page=roster")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(roster.status(), StatusCode::OK);
    let roster = body(roster).await;
    assert!(roster.contains("Dexter&#x27;s Dawgs"));
    assert!(roster.contains("Nathan MacKinnon"));
    assert!(roster.contains("BN4"));
    assert!(roster.contains("IR+2"));
    assert!(roster.contains("aria-current=\"page\">The Lineup"));
    assert!(roster.contains("View source JSON"));

    let insider = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/fantasy/cards/roster/dexters-dawgs?page=insider")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(insider.status(), StatusCode::OK);
    let insider = body(insider).await;
    assert!(insider.contains("Pickups remaining"));
    assert!(insider.contains("Same day"));
    assert!(insider.contains("Best calendar complement: WSH (Class 8)"));
    assert!(insider.contains("Class 1: BOS, COL, DET, NYR"));
    assert!(insider.contains("deterministic examples, not current claims"));
    assert!(insider.contains("aria-current=\"page\">The Insider"));

    let missing = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cards/fantasy-roster/unknown")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn fantasy_draft_routes_preserve_the_pick_fallback_components_and_etag() {
    let app = router(WebState::new());
    let uri = "/api/v1/cards/fantasy-draft/dexters-dawgs";
    let response = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let etag = response.headers()["etag"].to_str().unwrap().to_string();
    let routed = parse_card_document(&body(response).await).unwrap();
    let fixture = parse_card_document(include_str!(
        "../../examples/fantasy-draft-card-dexters-dawgs-pick-7.json"
    ))
    .unwrap();
    assert_eq!(routed, fixture);
    assert_eq!(etag, format!("\"{}\"", fixture.fingerprint));

    let board = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/fantasy/cards/draft/dexters-dawgs?page=draft-board")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(board.status(), StatusCode::OK);
    let board = body(board).await;
    assert!(board.contains("THE BENCH · FANTASY DRAFT"));
    assert!(board.contains("Draft Jason Robertson"));
    assert!(board.contains("Fallback: William Nylander"));
    assert!(board.contains("LW/RW"));
    assert!(board.contains("Priority slots"));
    assert!(board.contains("aria-current=\"page\">The Draft Board"));

    let insider = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/fantasy/cards/draft/dexters-dawgs?page=insider")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(insider.status(), StatusCode::OK);
    let insider = body(insider).await;
    assert!(insider.contains("Schedule diversity"));
    assert!(insider.contains("Taken matched"));
    assert!(insider.contains(">2</strong>"));
    assert!(insider.contains("deterministic fixture inputs, not current claims"));
    assert!(insider.contains("aria-current=\"page\">The Insider"));

    let not_modified = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(uri)
                .header("if-none-match", etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(not_modified.status(), StatusCode::NOT_MODIFIED);

    let missing = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cards/fantasy-draft/unknown")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn fantasy_morning_routes_preserve_actions_goalie_timeline_pickup_and_etag() {
    let app = router(WebState::new());
    let uri = "/api/v1/cards/fantasy-morning/dexters-dawgs";
    let response = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let etag = response.headers()["etag"].to_str().unwrap().to_string();
    let routed = parse_card_document(&body(response).await).unwrap();
    let fixture = parse_card_document(include_str!(
        "../../examples/fantasy-morning-card-dexters-dawgs-2026-10-08.json"
    ))
    .unwrap();
    assert_eq!(routed, fixture);
    assert_eq!(etag, format!("\"{}\"", fixture.fingerprint));

    let morning = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/fantasy/cards/morning/dexters-dawgs?page=morning-skate")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(morning.status(), StatusCode::OK);
    let morning = body(morning).await;
    assert!(morning.contains("THE INSIDER · MORNING SKATE"));
    assert!(morning.contains("Move Justin Brazeau to IR+1"));
    assert!(morning.contains("Nathan MacKinnon"));
    assert!(morning.contains("aria-current=\"page\">The Morning Skate"));

    let insider = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/fantasy/cards/morning/dexters-dawgs?page=insider")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(insider.status(), StatusCode::OK);
    let insider = body(insider).await;
    assert!(insider.contains("Darren Raddysh"));
    assert!(insider.contains("Goalie start evidence"));
    assert!(insider.contains("goalie checkpoints"));
    assert!(insider.contains("Final goalie safety check"));
    assert!(insider.contains("Safe proactive adds"));
    assert!(insider.contains("aria-current=\"page\">The Insider"));

    let not_modified = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(uri)
                .header("if-none-match", etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(not_modified.status(), StatusCode::NOT_MODIFIED);

    let missing = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cards/fantasy-morning/unknown")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn fantasy_trade_routes_preserve_packages_fairness_impacts_and_etag() {
    let app = router(WebState::new());
    let uri = "/api/v1/cards/fantasy-trade/dexters-dawgs";
    let response = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let etag = response.headers()["etag"].to_str().unwrap().to_string();
    let routed = parse_card_document(&body(response).await).unwrap();
    let fixture = parse_card_document(include_str!(
        "../../examples/fantasy-trade-card-dexters-dawgs-fox-rantanen.json"
    ))
    .unwrap();
    assert_eq!(routed, fixture);
    assert_eq!(etag, format!("\"{}\"", fixture.fingerprint));

    let board = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/fantasy/cards/trade/dexters-dawgs?page=trade-board")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(board.status(), StatusCode::OK);
    let board = body(board).await;
    assert!(board.contains("THE BOARDS · TRADE ANALYSIS"));
    assert!(board.contains("Adam Fox"));
    assert!(board.contains("Mikko Rantanen"));
    assert!(board.contains("Reasonable offer range"));
    assert!(board.contains("Value gap percent"));
    assert!(board.contains("aria-current=\"page\">The Trade Board"));

    let insider = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/fantasy/cards/trade/dexters-dawgs?page=insider")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(insider.status(), StatusCode::OK);
    let insider = body(insider).await;
    assert!(insider.contains("Before and after"));
    assert!(insider.contains("Roster 16/16 · Legal"));
    assert!(insider.contains("Open slots after"));
    assert!(insider.contains("not current trade advice"));
    assert!(insider.contains("aria-current=\"page\">The Insider"));

    let not_modified = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(uri)
                .header("if-none-match", etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(not_modified.status(), StatusCode::NOT_MODIFIED);

    let missing = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cards/fantasy-trade/unknown")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn season_simulation_routes_share_one_league_run_and_render_both_pages() {
    let app = router(WebState::new());
    let mut cards = Vec::new();
    for team in ["NYR", "SEA"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/cards/season-simulation/20262027/{team}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        cards.push(parse_card_document(&body(response).await).unwrap());
    }
    assert_eq!(
        cards[0].context.simulation.parameter_fingerprint,
        cards[1].context.simulation.parameter_fingerprint
    );
    assert_ne!(cards[0].fingerprint, cards[1].fingerprint);

    let scoreboard = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/icecast/20262027/NYR/simulation?page=scoreboard")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(scoreboard.status(), StatusCode::OK);
    let scoreboard = body(scoreboard).await;
    assert!(scoreboard.contains("ICECAST · SEASON SIMULATION"));
    assert!(scoreboard.contains("Stanley Cup"));
    assert!(scoreboard.contains("Points distribution"));
    assert!(scoreboard.contains("Scenario delta from baseline"));

    let insider = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/icecast/20262027/NYR/simulation?page=insider")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(insider.status(), StatusCode::OK);
    let insider = body(insider).await;
    assert!(insider.contains("Schedule pressure"));
    assert!(insider.contains("Pivotal games"));
    assert!(insider.contains("IceCast Monte Carlo"));
    assert!(insider.contains("1344-game league schedule"));

    let missing = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cards/season-simulation/20262027/BOS")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn completed_season_replay_renders_actual_results_and_calibration() {
    let app = router(WebState::new());
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/cards/season-simulation/20242025/NYR")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let card = parse_card_document(&body(response).await).unwrap();
    assert_eq!(card.context.view.window.season.0, 20242025);
    assert_eq!(card.context.simulation.trials, Some(1_000));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/icecast/20242025/NYR/simulation?page=insider")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let html = body(response).await;
    assert!(html.contains("Actual team result"));
    assert!(html.contains("Completed-season calibration"));
    assert!(html.contains("League picks correct"));
    assert!(html.contains("Calibration intercept · ideal 0"));
    assert!(html.contains("-0.378"));
    assert!(html.contains("Calibration slope · ideal 1"));
    assert!(html.contains("4.417"));
    assert!(html.contains("Calibration slope 95% lower"));
    assert!(html.contains("2.380"));
    assert!(html.contains("Calibration slope 95% upper"));
    assert!(html.contains("6.454"));
    assert!(html.contains("Best chronological Elo blend"));
}

#[tokio::test]
async fn forecast_movement_routes_share_sealed_sources_and_render_both_pages() {
    let app = router(WebState::new());
    let mut cards = Vec::new();
    for team in ["NYR", "SEA"] {
        let uri = format!("/api/v1/cards/forecast-movement/20242025/{team}");
        let response = app
            .clone()
            .oneshot(Request::builder().uri(&uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let etag = response.headers()["etag"].to_str().unwrap().to_string();
        let card = parse_card_document(&body(response).await).unwrap();
        assert_eq!(etag, format!("\"{}\"", card.fingerprint));
        cards.push(card);
    }
    assert_eq!(
        cards[0].context.simulation.parameter_fingerprint,
        cards[1].context.simulation.parameter_fingerprint
    );
    assert_eq!(cards[0].provenance, cards[1].provenance);

    let shift = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/icecast/20242025/NYR/movement?page=shift")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(shift.status(), StatusCode::OK);
    let shift = body(shift).await;
    assert!(shift.contains("ICECAST · FORECAST MOVEMENT"));
    assert!(shift.contains("Projected points"));
    assert!(shift.contains("Observed standings points"));
    assert!(shift.contains("aria-current=\"page\">The Shift"));

    let insider = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/icecast/20242025/SEA/movement?page=insider")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(insider.status(), StatusCode::OK);
    let insider = body(insider).await;
    assert!(insider.contains("Sealed checkpoint delta"));
    assert!(insider.contains("Both league runs must share season"));
    assert!(insider.contains("aria-current=\"page\">The Insider"));

    let missing = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cards/forecast-movement/20242025/BOS")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn forecast_history_routes_share_sealed_sources_and_render_both_pages() {
    let app = router(WebState::new());
    let mut cards = Vec::new();
    for team in ["NYR", "SEA"] {
        let uri = format!("/api/v1/cards/forecast-history/20242025/{team}");
        let response = app
            .clone()
            .oneshot(Request::builder().uri(&uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let etag = response.headers()["etag"].to_str().unwrap().to_string();
        let card = parse_card_document(&body(response).await).unwrap();
        assert_eq!(etag, format!("\"{}\"", card.fingerprint));
        cards.push(card);
    }
    assert_eq!(
        cards[0].context.simulation.parameter_fingerprint,
        cards[1].context.simulation.parameter_fingerprint
    );
    assert_eq!(cards[0].provenance, cards[1].provenance);

    let tape = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/icecast/20242025/NYR/history?page=tape")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(tape.status(), StatusCode::OK);
    let tape = body(tape).await;
    assert!(tape.contains("ICECAST · FORECAST HISTORY"));
    assert!(tape.contains("Projected points"));
    assert!(tape.contains("Change in projected points"));
    assert!(tape.contains("Net change in projected points"));
    assert!(tape.contains("Through 2025-03-31"));
    assert!(tape.contains("19 of 32"));
    assert!(tape.contains("Trajectory"));
    assert!(tape.contains("mixed"));
    assert!(tape.contains("Largest checkpoint swing"));
    assert!(tape.contains("Projected points P10 / P50 / P90"));
    assert!(tape.contains("Movement materiality"));
    assert!(tape.contains("moderate"));
    assert!(tape.contains("Confirmed standings points gained"));
    assert!(tape.contains("+25.00"));
    assert!(tape.contains("Change in expected remaining points"));
    assert!(tape.contains("-26.75"));
    assert!(tape.contains("Prior expected points for 24 newly completed games"));
    assert!(tape.contains("+26.59"));
    assert!(tape.contains("Realized points versus prior expected pace"));
    assert!(tape.contains("-1.59"));
    assert!(tape.contains("Still-unplayed outlook revaluation"));
    assert!(tape.contains("-0.16"));
    assert!(tape.contains("Pace-normalized attribution"));
    assert!(tape.contains("Realized points versus prior checkpoint pace"));
    assert!(tape.contains("+0.03"));
    assert!(tape.contains("Still-unplayed outlook revaluation from prior checkpoint"));
    assert!(tape.contains("-0.17"));
    assert!(tape.contains("aria-current=\"page\">The Tape"));

    let insider = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/icecast/20242025/SEA/history?page=insider")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(insider.status(), StatusCode::OK);
    let insider = body(insider).await;
    assert!(insider.contains("Chronological checkpoint history"));
    assert!(insider.contains("Absolute values come from each sealed league forecast"));
    assert!(insider.contains("aria-current=\"page\">The Insider"));

    let missing = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cards/forecast-history/20242025/BOS")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}
