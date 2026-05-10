use crate::state::WebState;
use crate::templates::{DepthRow, DepthTemplate};
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::cross_team::{compute_team_strength_views, ScoringMode};
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;

pub async fn get_depth(State(state): State<WebState>) -> Response {
    let (season_str, season_type, active_label) = {
        let cfg = state.config.read().await;
        (
            cfg.active_season.clone(),
            super::leaders::parse_season_type(&cfg.active_season_type),
            cfg.active_label.clone(),
        )
    };
    let season_u32: u32 = match season_str.parse() {
        Ok(n) => n,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(format!(
                    "<!doctype html><body><h1>500</h1><p>active season \
                             '{season_str}' is not a valid YYYYZZZZ id: {e}</p></body>"
                )),
            )
                .into_response();
        }
    };
    let season = Season(season_u32);

    // Brief read of the repo. Project inside the lock scope so
    // PlayerView refs don't escape (same convention as
    // `/leaders` and `/goalies`).
    let rows: Vec<DepthRow> = {
        let repo = state.repo.read().await;
        let views: Vec<_> = repo.skaters(season, season_type).collect();
        let strength = compute_team_strength_views(&views, ScoringMode::Pace);
        let mut ranked: Vec<_> = strength.into_iter().collect();
        // Newest team rank first; tie-break alphabetical for
        // determinism.
        ranked.sort_by(|a, b| {
            b.1.total
                .partial_cmp(&a.1.total)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        ranked
            .into_iter()
            .map(|(team, s)| DepthRow {
                team,
                c_score: format!("{:.0}", s.c_score),
                lw_score: format!("{:.0}", s.lw_score),
                rw_score: format!("{:.0}", s.rw_score),
                d_score: format!("{:.0}", s.d_score),
                total: format!("{:.0}", s.total),
                c_top: s.c_top,
                lw_top: s.lw_top,
                rw_top: s.rw_top,
                d_top: s.d_top,
            })
            .collect()
    };

    let tmpl = DepthTemplate { active_label, rows };
    match tmpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!(
                "<!doctype html><body><h1>500</h1><p>{e}</p></body>"
            )),
        )
            .into_response(),
    }
}

// ── JSON twin ────────────────────────────────────────────────
// T3 (post-LP test gap): every list page on the web surface
// gets a JSON twin so external scripts don't have to scrape
// HTML. Mirrors the King.2.4 envelope `{schema_version, route,
// data, meta}` already used by /api/v1/leaders + /api/v1/goalies.

#[derive(serde::Serialize)]
struct DepthJsonRow {
    team: String,
    c_score: f64,
    lw_score: f64,
    rw_score: f64,
    d_score: f64,
    total: f64,
    c_top: String,
    lw_top: String,
    rw_top: String,
    d_top: String,
}

#[derive(serde::Serialize)]
struct DepthMeta {
    season: String,
    season_type: String,
    count: usize,
    scoring_mode: &'static str,
}

#[derive(serde::Serialize)]
struct DepthEnvelope {
    schema_version: u32,
    route: &'static str,
    data: Vec<DepthJsonRow>,
    meta: DepthMeta,
}

pub async fn get_depth_json(State(state): State<WebState>) -> Response {
    let (season_str, season_type) = {
        let cfg = state.config.read().await;
        (
            cfg.active_season.clone(),
            super::leaders::parse_season_type(&cfg.active_season_type),
        )
    };
    let season_u32: u32 = match season_str.parse() {
        Ok(n) => n,
        Err(e) => {
            return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        axum::Json(serde_json::json!({
                            "error": format!("active season '{season_str}' is not a valid YYYYZZZZ id: {e}"),
                        })),
                    )
                        .into_response();
        }
    };
    let season = Season(season_u32);

    let rows: Vec<DepthJsonRow> = {
        let repo = state.repo.read().await;
        let views: Vec<_> = repo.skaters(season, season_type).collect();
        let strength = compute_team_strength_views(&views, ScoringMode::Pace);
        let mut ranked: Vec<_> = strength.into_iter().collect();
        ranked.sort_by(|a, b| {
            b.1.total
                .partial_cmp(&a.1.total)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        ranked
            .into_iter()
            .map(|(team, s)| DepthJsonRow {
                team,
                c_score: s.c_score,
                lw_score: s.lw_score,
                rw_score: s.rw_score,
                d_score: s.d_score,
                total: s.total,
                c_top: s.c_top,
                lw_top: s.lw_top,
                rw_top: s.rw_top,
                d_top: s.d_top,
            })
            .collect()
    };

    let envelope = DepthEnvelope {
        schema_version: 1,
        route: "depth",
        meta: DepthMeta {
            season: season_str,
            season_type: match season_type {
                SeasonType::Regular => "regular".to_owned(),
                SeasonType::Playoff => "playoff".to_owned(),
            },
            count: rows.len(),
            scoring_mode: "pace",
        },
        data: rows,
    };
    axum::Json(envelope).into_response()
}
