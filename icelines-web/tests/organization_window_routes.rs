use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use icelines_core::{CardKind, OrganizationWindowBoardView, CANONICAL_TEAMS};
use icelines_web::{router, WebState};
use tower::ServiceExt;

#[tokio::test]
async fn registered_window_json_retains_all_32_and_etag() {
    let response = router(WebState::new())
        .oneshot(
            Request::builder()
                .uri("/api/v1/window/balanced.v1/20262027")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key(header::ETAG));
    let body = axum::body::to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    let board: OrganizationWindowBoardView = serde_json::from_slice(&body).unwrap();
    assert_eq!(board.organizations.len(), 32);
    assert_eq!(board.manifest.manifest_id, "balanced.v1");
    assert!(board
        .organizations
        .iter()
        .all(|row| row.overall.rank.is_none()));
}

#[tokio::test]
async fn focused_window_html_and_card_use_same_registered_artifact() {
    let app = router(WebState::new());
    let html_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/window/balanced.v1/20262027?team=NYR")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(html_response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(html_response.into_body(), 2 * 1024 * 1024)
        .await
        .unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("The Window"));
    assert!(html.contains("NYR"));
    assert!(!html.contains(">SEA</a>"));
    assert!(html.contains("class=\"skip-link\""));
    assert!(html.contains("<main id=\"main\" tabindex=\"-1\">"));
    assert!(html.contains("<caption>"));
    assert!(html.contains("aria-label=\"Organization Window standings\""));
    assert!(html.contains("<th scope=\"col\">Team</th>"));
    assert!(html.contains("Under review"));
    assert!(!html.contains("Rebuilding"));

    let card_response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cards/organization-window/20262027/NYR")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(card_response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(card_response.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    let card: icelines_core::CardDocumentView = serde_json::from_slice(&body).unwrap();
    assert_eq!(card.card_kind, CardKind::OrganizationWindow);
    assert_eq!(card.pages.len(), 2);
    assert!(card
        .subtitle
        .as_deref()
        .is_some_and(|subtitle| subtitle.starts_with("Under review · NR")));
}

#[tokio::test]
async fn every_canonical_team_has_a_dynamic_window_card() {
    let app = router(WebState::new());
    let mut fingerprints = std::collections::BTreeSet::new();
    for (team, _) in CANONICAL_TEAMS {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/cards/organization-window/20262027/{team}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{team}");
        let body = axum::body::to_bytes(response.into_body(), 4 * 1024 * 1024)
            .await
            .unwrap();
        let card: icelines_core::CardDocumentView = serde_json::from_slice(&body).unwrap();
        assert_eq!(card.context.joins.team_ids, [*team]);
        assert!(fingerprints.insert(card.fingerprint));

        let html_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/icecast/20262027/{team}/window"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(html_response.status(), StatusCode::OK, "{team}");
        let body = axum::body::to_bytes(html_response.into_body(), 2 * 1024 * 1024)
            .await
            .unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();
        for color in [
            card.theme.primary.as_deref().unwrap(),
            card.theme.secondary.as_deref().unwrap(),
            card.theme.accent.as_deref().unwrap(),
        ] {
            assert!(
                html.contains(color),
                "{team} HTML must consume the card theme token {color}"
            );
        }
    }
    assert_eq!(fingerprints.len(), CANONICAL_TEAMS.len());
}

#[tokio::test]
async fn withheld_window_html_uses_canonical_order_not_partial_score_order() {
    let response = router(WebState::new())
        .oneshot(
            Request::builder()
                .uri("/window/balanced.v1/20262027")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 2 * 1024 * 1024)
        .await
        .unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();

    let mut prior_position = 0;
    for (team, _) in CANONICAL_TEAMS {
        let marker = format!(">{team}</a>");
        let position = html.find(&marker).expect("canonical team row");
        assert!(position >= prior_position, "{team} is out of canonical order");
        prior_position = position;
    }
    assert_eq!(html.matches("Under review").count(), CANONICAL_TEAMS.len());
}
