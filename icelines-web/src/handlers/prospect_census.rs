use axum::extract::{Path, Query};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

use icelines_core::{
    ProspectAuthorityClosureDisposition, ProspectAuthorityProgressChangeKind,
    ProspectCensusAuthorityGapState, CANONICAL_TEAMS,
};

use crate::card_store::{
    prospect_authority_closure_board, prospect_authority_progress, prospect_census_readiness_board,
};
use crate::handlers::team_card::cached_response;

#[derive(Debug, Default, Deserialize)]
pub struct ProspectCensusReadinessQuery {
    pub team: Option<String>,
}

pub async fn get_prospect_authority_closure_json(
    Path(season): Path<u32>,
    headers: HeaderMap,
) -> Response {
    match prospect_authority_closure_board(season) {
        Ok(board) => {
            let fingerprint = board.fingerprint.clone();
            cached_response(axum::Json(board).into_response(), &fingerprint, &headers)
        }
        Err(error) => (StatusCode::NOT_FOUND, error.to_string()).into_response(),
    }
}

pub async fn get_prospect_authority_closure(
    Path(season): Path<u32>,
    Query(query): Query<ProspectCensusReadinessQuery>,
    headers: HeaderMap,
) -> Response {
    let board = match prospect_authority_closure_board(season) {
        Ok(board) => board,
        Err(error) => return (StatusCode::NOT_FOUND, Html(error.to_string())).into_response(),
    };
    let focus = query.team.map(|team| team.trim().to_ascii_uppercase());
    if focus.as_ref().is_some_and(|team| {
        !CANONICAL_TEAMS
            .iter()
            .any(|(candidate, _)| candidate == team)
    }) {
        return (
            StatusCode::NOT_FOUND,
            Html("team is absent from the canonical NHL organization set".to_owned()),
        )
            .into_response();
    }

    let family_summary = board
        .family_summary
        .iter()
        .map(|row| {
            format!(
                "<li><strong>{family}</strong>: {cells} cells across {organizations} teams · {gate} · <code>{artifact}</code> via <code>{option}</code></li>",
                family = row.source_family,
                cells = row.cells,
                organizations = row.organizations,
                gate = authority_gate_label(row.gate),
                artifact = row.required_artifact_schema.as_deref().unwrap_or("adapter required"),
                option = row.ingestion_option.as_deref().unwrap_or("no registered option"),
            )
        })
        .collect::<String>();
    let rows = board
        .closure_cells
        .iter()
        .filter(|row| focus.as_ref().is_none_or(|team| row.organization == *team))
        .map(|row| {
            format!(
                "<tr><th scope=\"row\">{team}</th><td>{family}</td><td>{gate}</td><td>{state}</td><td>{disposition}</td><td><code>{artifact}</code></td><td><code>{option}</code></td><td>{remediation}</td></tr>",
                team = row.organization,
                family = row.source_family,
                gate = authority_gate_label(row.gate),
                state = authority_gap_state_label(row.state),
                disposition = authority_disposition_label(row.disposition),
                artifact = row.required_artifact_schema.as_deref().unwrap_or("adapter required"),
                option = row.ingestion_option.as_deref().unwrap_or("no registered option"),
                remediation = row.remediation,
            )
        })
        .collect::<String>();
    let disclosures = board
        .disclosures
        .iter()
        .map(|disclosure| format!("<li>{disclosure}</li>"))
        .collect::<String>();
    let html = format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Prospect Authority Closure</title><link rel=\"stylesheet\" href=\"/static/style.css\"></head><body><a href=\"#main\" class=\"skip-link\">Skip to content</a><main id=\"main\" tabindex=\"-1\"><h1>Prospect Authority Closure</h1><p>Season {season} · fingerprint <code>{fingerprint}</code></p><p><strong>{cells}</strong> blocking cells across <strong>{affected}/{organizations}</strong> organizations · {population} population-authority · {control} organizational-control.</p><p>Knowledge cutoff: <time>{cutoff}</time>.</p><h2>Required source families</h2><ul>{family_summary}</ul><div class=\"table-scroll\" role=\"region\" aria-label=\"Prospect authority closure recipes\" tabindex=\"0\"><table><caption>Exact acquisition recipes; these rows are not evidence approvals</caption><thead><tr><th scope=\"col\">Team</th><th scope=\"col\">Source family</th><th scope=\"col\">Gate</th><th scope=\"col\">State</th><th scope=\"col\">Disposition</th><th scope=\"col\">Artifact</th><th scope=\"col\">Ingestion</th><th scope=\"col\">Remediation</th></tr></thead><tbody>{rows}</tbody></table></div><h2>Disclosures</h2><ul>{disclosures}</ul><p><a href=\"/api/v1/prospect-authority-closure/{season}\">Full JSON artifact</a></p></main></body></html>",
        fingerprint = board.fingerprint,
        cells = board.cells,
        affected = board.affected_organizations,
        organizations = board.organizations,
        population = board.population_blocking_cells,
        control = board.control_blocking_cells,
        cutoff = board.knowledge_cutoff,
    );
    let fingerprint = board.fingerprint.clone();
    cached_response(Html(html).into_response(), &fingerprint, &headers)
}

pub async fn get_prospect_authority_progress_json(
    Path(season): Path<u32>,
    headers: HeaderMap,
) -> Response {
    match prospect_authority_progress(season) {
        Ok(progress) => {
            let fingerprint = progress.fingerprint.clone();
            cached_response(axum::Json(progress).into_response(), &fingerprint, &headers)
        }
        Err(error) => (StatusCode::NOT_FOUND, error.to_string()).into_response(),
    }
}

pub async fn get_prospect_authority_progress(
    Path(season): Path<u32>,
    Query(query): Query<ProspectCensusReadinessQuery>,
    headers: HeaderMap,
) -> Response {
    let progress = match prospect_authority_progress(season) {
        Ok(progress) => progress,
        Err(error) => return (StatusCode::NOT_FOUND, Html(error.to_string())).into_response(),
    };
    let focus = query.team.map(|team| team.trim().to_ascii_uppercase());
    if focus.as_ref().is_some_and(|team| {
        !CANONICAL_TEAMS
            .iter()
            .any(|(candidate, _)| candidate == team)
    }) {
        return (
            StatusCode::NOT_FOUND,
            Html("team is absent from the canonical NHL organization set".to_owned()),
        )
            .into_response();
    }

    let rows = progress
        .changes
        .iter()
        .filter(|row| focus.as_ref().is_none_or(|team| row.organization == *team))
        .map(|row| {
            format!(
                "<tr><th scope=\"row\">{team}</th><td>{family}</td><td>{gate}</td><td>{kind}</td><td>{prior}</td><td>{current}</td></tr>",
                team = row.organization,
                family = row.source_family,
                gate = authority_gate_label(row.gate),
                kind = authority_change_label(row.kind),
                prior = row.prior_state.map(authority_gap_state_label).unwrap_or("absent"),
                current = row.current_state.map(authority_gap_state_label).unwrap_or("resolved"),
            )
        })
        .collect::<String>();
    let disclosures = progress
        .disclosures
        .iter()
        .map(|disclosure| format!("<li>{disclosure}</li>"))
        .collect::<String>();
    let html = format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Prospect Authority Progress</title><link rel=\"stylesheet\" href=\"/static/style.css\"></head><body><a href=\"#main\" class=\"skip-link\">Skip to content</a><main id=\"main\" tabindex=\"-1\"><h1>Prospect Authority Progress</h1><p>Season {season} · fingerprint <code>{fingerprint}</code></p><p><strong>{prior} → {current}</strong> blocking cells · <strong>{closed}</strong> closed · <strong>{opened}</strong> opened · <strong>{percent}</strong>% of the prior backlog retired.</p><p>{population} population-authority cells closed · {control} organizational-control cells closed · {changed} persisting cells changed state.</p><p>Knowledge window: <time>{prior_cutoff}</time> → <time>{current_cutoff}</time>.</p><div class=\"table-scroll\" role=\"region\" aria-label=\"Prospect authority progress changes\" tabindex=\"0\"><table><caption>Exact changes between the two sealed closure boards</caption><thead><tr><th scope=\"col\">Team</th><th scope=\"col\">Source family</th><th scope=\"col\">Gate</th><th scope=\"col\">Change</th><th scope=\"col\">Prior</th><th scope=\"col\">Current</th></tr></thead><tbody>{rows}</tbody></table></div><h2>Disclosures</h2><ul>{disclosures}</ul><p><a href=\"/api/v1/prospect-authority-progress/{season}\">Full JSON artifact</a></p></main></body></html>",
        fingerprint = progress.fingerprint,
        prior = progress.prior_cells,
        current = progress.current_cells,
        closed = progress.closed_cells,
        opened = progress.opened_cells,
        percent = format_args!("{:.2}", f64::from(progress.closure_basis_points) / 100.0),
        population = progress.population_cells_closed,
        control = progress.control_cells_closed,
        changed = progress.state_changed_cells,
        prior_cutoff = progress.prior_knowledge_cutoff,
        current_cutoff = progress.current_knowledge_cutoff,
    );
    let fingerprint = progress.fingerprint.clone();
    cached_response(Html(html).into_response(), &fingerprint, &headers)
}

fn authority_change_label(kind: ProspectAuthorityProgressChangeKind) -> &'static str {
    match kind {
        ProspectAuthorityProgressChangeKind::Closed => "closed",
        ProspectAuthorityProgressChangeKind::Opened => "opened",
        ProspectAuthorityProgressChangeKind::StateChanged => "state changed",
    }
}

fn authority_gap_state_label(state: ProspectCensusAuthorityGapState) -> &'static str {
    match state {
        ProspectCensusAuthorityGapState::Failed => "failed",
        ProspectCensusAuthorityGapState::Quarantined => "quarantined",
        ProspectCensusAuthorityGapState::IncompletePagination => "incomplete pagination",
    }
}

fn authority_gate_label(gate: icelines_core::ProspectAuthorityClosureGate) -> &'static str {
    match gate {
        icelines_core::ProspectAuthorityClosureGate::PopulationAuthority => "population authority",
        icelines_core::ProspectAuthorityClosureGate::OrganizationalControl => {
            "organizational control"
        }
    }
}

fn authority_disposition_label(disposition: ProspectAuthorityClosureDisposition) -> &'static str {
    match disposition {
        ProspectAuthorityClosureDisposition::Acquire => "acquire",
        ProspectAuthorityClosureDisposition::ResolveQuarantine => "resolve quarantine",
        ProspectAuthorityClosureDisposition::CompletePagination => "complete pagination",
    }
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
