use axum::extract::{Path, Query};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

use crate::card_store::organization_window_board;
use crate::handlers::team_card::cached_response;

#[derive(Debug, Default, Deserialize)]
pub struct WindowQuery {
    pub team: Option<String>,
}

pub async fn get_window_json(
    Path((frame, season)): Path<(String, u32)>,
    headers: HeaderMap,
) -> Response {
    match organization_window_board(&frame, season) {
        Ok(board) => {
            let fingerprint = board.fingerprint.clone();
            cached_response(axum::Json(board).into_response(), &fingerprint, &headers)
        }
        Err(error) => (StatusCode::NOT_FOUND, error.to_string()).into_response(),
    }
}

pub async fn get_window(
    Path((frame, season)): Path<(String, u32)>,
    Query(query): Query<WindowQuery>,
    headers: HeaderMap,
) -> Response {
    let board = match organization_window_board(&frame, season) {
        Ok(board) => board,
        Err(error) => return (StatusCode::NOT_FOUND, Html(error.to_string())).into_response(),
    };
    let focus = query.team.map(|team| team.trim().to_ascii_uppercase());
    if focus
        .as_ref()
        .is_some_and(|team| board.organization(team).is_none())
    {
        return (
            StatusCode::NOT_FOUND,
            Html("team is absent from this Window".to_owned()),
        )
            .into_response();
    }
    let mut rows = String::new();
    for row in board
        .organizations
        .iter()
        .filter(|row| focus.as_ref().is_none_or(|team| row.organization == *team))
    {
        let score = row
            .overall
            .score
            .map(|value| format!("{value:.1}"))
            .unwrap_or_else(|| "NR".to_owned());
        let rank = row
            .overall
            .rank
            .map(|value| value.to_string())
            .unwrap_or_else(|| "NR".to_owned());
        rows.push_str(&format!(
            "<tr><th scope=\"row\"><a href=\"/icecast/{season}/{}/window\">{}</a></th><td>{score}</td><td>{rank}</td><td>{:.0}%</td><td>{:.0}%</td><td>{:?}</td></tr>",
            row.organization,
            row.organization,
            row.overall.confidence * 100.0,
            row.overall.coverage * 100.0,
            row.overall.classification
        ));
    }
    let html = format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>The Window</title><link rel=\"stylesheet\" href=\"/static/style.css\"></head><body><a href=\"#main\" class=\"skip-link\">Skip to content</a><main id=\"main\" tabindex=\"-1\"><h1>The Window</h1><p>Season {season} · as of {} · Frame {} · fingerprint <code>{}</code></p><p>Scores, confidence, and coverage are separate. NR means a declared comparability gate withheld rank.</p><div class=\"table-scroll\" role=\"region\" aria-label=\"Organization Window standings\" tabindex=\"0\"><table><caption>Organization health for the sealed {season} cohort</caption><thead><tr><th scope=\"col\">Team</th><th scope=\"col\">Score</th><th scope=\"col\">Rank</th><th scope=\"col\">Confidence</th><th scope=\"col\">Coverage</th><th scope=\"col\">Classification</th></tr></thead><tbody>{rows}</tbody></table></div><p><a href=\"/api/v1/window/{frame}/{season}\">Full JSON artifact</a></p></main></body></html>",
        board.as_of,
        board.manifest.label,
        board.fingerprint,
    );
    let fingerprint = board.fingerprint.clone();
    cached_response(Html(html).into_response(), &fingerprint, &headers)
}
