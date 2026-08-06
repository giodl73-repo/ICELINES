use axum::extract::{Path, Query};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

use crate::card_store::prospect_census_readiness_board;
use crate::handlers::team_card::cached_response;

#[derive(Debug, Default, Deserialize)]
pub struct ProspectCensusReadinessQuery {
    pub team: Option<String>,
}

pub async fn get_prospect_census_readiness_json(
    Path(season): Path<u32>,
    headers: HeaderMap,
) -> Response {
    match prospect_census_readiness_board(season) {
        Ok(board) => {
            let fingerprint = board.fingerprint.clone();
            cached_response(axum::Json(board).into_response(), &fingerprint, &headers)
        }
        Err(error) => (StatusCode::NOT_FOUND, error.to_string()).into_response(),
    }
}

pub async fn get_prospect_census_readiness(
    Path(season): Path<u32>,
    Query(query): Query<ProspectCensusReadinessQuery>,
    headers: HeaderMap,
) -> Response {
    let board = match prospect_census_readiness_board(season) {
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
            Html("team is absent from this prospect census readiness board".to_owned()),
        )
            .into_response();
    }

    let mut rows = String::new();
    for row in board
        .teams
        .iter()
        .filter(|row| focus.as_ref().is_none_or(|team| row.organization == *team))
    {
        rows.push_str(&format!(
            "<tr><th scope=\"row\">{team}</th><td>{authority:?}</td><td>{publication:?}</td><td>{canonical}</td><td>{controlled}</td><td>{ranked}/{target}</td><td>{shortfall}</td><td>{gaps}</td></tr>",
            team = row.organization,
            authority = row.population_authority_status,
            publication = row.publication_status,
            canonical = row.counts.canonical_identity,
            controlled = row.counts.controlled_relationship,
            ranked = row.counts.ranked,
            target = row.requested_ranking_depth,
            shortfall = row.ranking_depth_shortfall,
            gaps = row.authority_gaps.len(),
        ));
    }
    let authority_gaps = board
        .authority_gap_summary
        .iter()
        .map(|row| {
            format!(
                "<li><strong>{}</strong>: {:?} for {} organizations</li>",
                row.source_family, row.state, row.organizations
            )
        })
        .collect::<String>();
    let player_losses = board
        .loss_summary
        .iter()
        .map(|row| {
            format!(
                "<li><strong>{:?}</strong>: {} players</li>",
                row.reason, row.players
            )
        })
        .collect::<String>();
    let html = format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Prospect Census Readiness</title><link rel=\"stylesheet\" href=\"/static/style.css\"></head><body><a href=\"#main\" class=\"skip-link\">Skip to content</a><main id=\"main\" tabindex=\"-1\"><h1>Prospect Census Readiness</h1><p>Season {season} · fingerprint <code>{fingerprint}</code></p><p><strong>{population}/{organizations}</strong> population complete · <strong>{depth}/{organizations}</strong> depth complete · <strong>{published}/{organizations}</strong> published.</p><p>Funnel: {discovered} discovered → {canonical} canonical identities → {controlled} controlled relationships → {ranked} ranked.</p><h2>Authority gaps</h2><ul>{authority_gaps}</ul><h2>Player losses</h2><ul>{player_losses}</ul><div class=\"table-scroll\" role=\"region\" aria-label=\"Prospect census readiness board\" tabindex=\"0\"><table><caption>All-team prospect census publication gates</caption><thead><tr><th scope=\"col\">Team</th><th scope=\"col\">Population</th><th scope=\"col\">Publication</th><th scope=\"col\">Canonical</th><th scope=\"col\">Controlled</th><th scope=\"col\">Ranked/target</th><th scope=\"col\">Shortfall</th><th scope=\"col\">Authority gaps</th></tr></thead><tbody>{rows}</tbody></table></div><p>This readiness board does not infer organizational control or manufacture rankings.</p><p><a href=\"/api/v1/prospect-census-readiness/{season}\">Full JSON artifact</a></p></main></body></html>",
        fingerprint = board.fingerprint,
        population = board.population_complete_organizations,
        organizations = board.organizations,
        depth = board.depth_complete_organizations,
        published = board.published_organizations,
        discovered = board.league_counts.discovered,
        canonical = board.league_counts.canonical_identity,
        controlled = board.league_counts.controlled_relationship,
        ranked = board.league_counts.ranked,
    );
    let fingerprint = board.fingerprint.clone();
    cached_response(Html(html).into_response(), &fingerprint, &headers)
}
