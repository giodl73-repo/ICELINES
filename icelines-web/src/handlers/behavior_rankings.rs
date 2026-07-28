use std::sync::OnceLock;

use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use icelines_fetch::TeamBehaviorLeagueEvidenceView;
use serde::Deserialize;

use crate::{
    state::WebState,
    templates::{BehaviorRankingRowTemplate, BehaviorRankingsTemplate, BehaviorScaleTemplate},
};

const EVIDENCE_20262027: &str =
    include_str!("../../../examples/team-behavior-rankings-2026-27.json");

#[derive(Debug, Default, Deserialize)]
pub struct BehaviorRankingsQuery {
    pub scale: Option<String>,
}

pub async fn get_behavior_rankings(
    State(state): State<WebState>,
    Path(season): Path<u32>,
    Query(query): Query<BehaviorRankingsQuery>,
) -> Response {
    let view = match evidence(season) {
        Some(view) => view,
        None => return (StatusCode::NOT_FOUND, "behavior rankings unavailable").into_response(),
    };
    let scale = query
        .scale
        .as_deref()
        .filter(|candidate| {
            view.rankings
                .rows
                .iter()
                .any(|row| row.trait_key == *candidate)
        })
        .unwrap_or("rookie_opportunity");
    let rows = view
        .rankings
        .rows
        .iter()
        .filter(|row| row.trait_key == scale)
        .map(|row| BehaviorRankingRowTemplate {
            team: row.team.clone(),
            rank: row
                .rank
                .map_or_else(|| "—".to_owned(), |rank| rank.to_string()),
            percentile: row
                .percentile
                .map_or_else(|| "NoRead".to_owned(), |value| format!("{value:.0}")),
            tendency: row
                .effective_value
                .map_or_else(|| "—".to_owned(), |value| format!("{value:+.3}")),
            opportunities: row.evidence_opportunities.to_string(),
            evidence: format!("{:?}", row.evidence_label),
        })
        .collect();
    let active_label = state.config.read().await.active_label.clone();
    let template = BehaviorRankingsTemplate {
        active_label,
        target_season: season.to_string(),
        scale: scale.to_owned(),
        rows,
        scales: ranked_scales(view)
            .into_iter()
            .map(|key| BehaviorScaleTemplate {
                active: key == scale,
                key,
            })
            .collect(),
        coverage: format!(
            "{:.1}%",
            view.rankings
                .coverage
                .iter()
                .map(|row| row.coverage_pct)
                .sum::<f64>()
                / view.rankings.coverage.len().max(1) as f64
        ),
    };
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

pub async fn get_behavior_rankings_json(Path(season): Path<u32>) -> Response {
    evidence(season).map_or_else(
        || (StatusCode::NOT_FOUND, "behavior rankings unavailable").into_response(),
        |view| axum::Json(view).into_response(),
    )
}

fn evidence(season: u32) -> Option<&'static TeamBehaviorLeagueEvidenceView> {
    static VIEW: OnceLock<TeamBehaviorLeagueEvidenceView> = OnceLock::new();
    (season == 20262027).then(|| {
        VIEW.get_or_init(|| {
            serde_json::from_str(EVIDENCE_20262027)
                .expect("sealed 2026-27 management behavior ranking artifact")
        })
    })
}

fn ranked_scales(view: &TeamBehaviorLeagueEvidenceView) -> Vec<String> {
    let mut scales = view
        .rankings
        .rows
        .iter()
        .filter(|row| row.rank.is_some())
        .map(|row| row.trait_key.clone())
        .collect::<Vec<_>>();
    scales.sort();
    scales.dedup();
    scales
}
