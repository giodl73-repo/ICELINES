use axum::extract::{Path, Query};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

use crate::card_store::prospect_arrival_board;
use crate::handlers::team_card::cached_response;

#[derive(Debug, Default, Deserialize)]
pub struct ProspectArrivalBoardQuery {
    pub team: Option<String>,
}

pub async fn get_prospect_arrival_board_json(
    Path(season): Path<u32>,
    headers: HeaderMap,
) -> Response {
    match prospect_arrival_board(season) {
        Ok(board) => {
            let fingerprint = board.fingerprint.clone();
            cached_response(axum::Json(board).into_response(), &fingerprint, &headers)
        }
        Err(error) => (StatusCode::NOT_FOUND, error.to_string()).into_response(),
    }
}

pub async fn get_prospect_arrival_board(
    Path(season): Path<u32>,
    Query(query): Query<ProspectArrivalBoardQuery>,
    headers: HeaderMap,
) -> Response {
    let board = match prospect_arrival_board(season) {
        Ok(board) => board,
        Err(error) => return (StatusCode::NOT_FOUND, Html(error.to_string())).into_response(),
    };
    let focus = query.team.map(|team| team.trim().to_ascii_uppercase());
    if focus
        .as_ref()
        .is_some_and(|team| board.team(team).is_none())
    {
        return (
            StatusCode::NOT_FOUND,
            Html("team is absent from this prospect-arrival board".to_owned()),
        )
            .into_response();
    }

    let mut rows = String::new();
    for row in board
        .teams_in_display_order()
        .into_iter()
        .filter(|row| focus.as_ref().is_none_or(|team| row.organization == *team))
    {
        let rank = row
            .rank
            .map(|rank| rank.to_string())
            .unwrap_or_else(|| "NR".to_owned());
        let top = row
            .top_arrival_probability
            .map(|value| format!("{:.1}%", value * 100.0))
            .unwrap_or_else(|| "NR".to_owned());
        rows.push_str(&format!(
            "<tr><th scope=\"row\"><a href=\"/icecast/{season}/{team}/prospect-arrivals\">{team}</a></th><td>{rank}</td><td>{calibrated}/{eligible}</td><td>{coverage:.0}%</td><td>{routed}</td><td>{blocked}</td><td>{arrivals:.2}</td><td>{roles:.2}</td><td>{top}</td></tr>",
            team = row.organization,
            calibrated = row.calibrated_skaters,
            eligible = row.eligible_skaters,
            coverage = row.calibration_coverage * 100.0,
            routed = row.routed_established_skaters,
            blocked = row.blocking_exclusions,
            arrivals = row.expected_arrivals,
            roles = row.expected_established_roles,
        ));
    }
    let blockers = board
        .rank_blockers
        .iter()
        .map(|blocker| format!("<li>{blocker}</li>"))
        .collect::<String>();
    let remediation = board
        .exclusion_summary
        .iter()
        .map(|row| {
            format!(
                "<li><strong>{:?}</strong>: {} — {}</li>",
                row.kind, row.count, row.remediation
            )
        })
        .collect::<String>();
    let html = format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Prospect Arrival Board</title><link rel=\"stylesheet\" href=\"/static/style.css\"></head><body><a href=\"#main\" class=\"skip-link\">Skip to content</a><main id=\"main\" tabindex=\"-1\"><h1>Prospect Arrival Board</h1><p>Season {season} · generated {} · fingerprint <code>{}</code></p><p>Rank state: <strong>{:?}</strong>. {}/{} eligible skaters calibrated; {} established NHL players rerouted; {} true blockers.</p><ul>{blockers}</ul><h2>Remediation</h2><ul>{remediation}</ul><div class=\"table-scroll\" role=\"region\" aria-label=\"Prospect arrival board\" tabindex=\"0\"><table><caption>All-team calibrated prospect-arrival outlook</caption><thead><tr><th scope=\"col\">Team</th><th scope=\"col\">Rank</th><th scope=\"col\">Calibrated/eligible</th><th scope=\"col\">Coverage</th><th scope=\"col\">Rerouted</th><th scope=\"col\">Blocked</th><th scope=\"col\">Expected arrivals</th><th scope=\"col\">Expected roles</th><th scope=\"col\">Top arrival</th></tr></thead><tbody>{rows}</tbody></table></div><p><a href=\"/api/v1/prospect-arrivals/{season}\">Full JSON artifact</a></p></main></body></html>",
        board.generated_at, board.fingerprint, board.rank_state,
        board.calibrated_skaters, board.eligible_skaters,
        board.routed_established_skaters, board.blocking_exclusions,
    );
    let fingerprint = board.fingerprint.clone();
    cached_response(Html(html).into_response(), &fingerprint, &headers)
}
