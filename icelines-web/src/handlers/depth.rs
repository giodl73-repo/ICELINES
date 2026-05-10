use crate::state::WebState;
use crate::templates::{DepthRow, DepthTemplate};
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::{DepthLeagueView, DepthTeamStrengthRow};

pub async fn get_depth(State(state): State<WebState>) -> Response {
    let (season_str, season_type, active_label) = {
        let cfg = state.config.read().await;
        (
            cfg.active_season.clone(),
            SeasonType::parse_lossy(&cfg.active_season_type),
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

    let view = {
        let repo = state.repo.read().await;
        DepthLeagueView::pace_from_repository(&repo, season, season_type)
    };
    let rows = view.rows.iter().map(depth_row_from_view).collect();

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

pub async fn get_depth_json(State(state): State<WebState>) -> Response {
    let (season_str, season_type) = {
        let cfg = state.config.read().await;
        (
            cfg.active_season.clone(),
            SeasonType::parse_lossy(&cfg.active_season_type),
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

    let view = {
        let repo = state.repo.read().await;
        DepthLeagueView::pace_from_repository(&repo, season, season_type)
    };
    let rows: Vec<DepthJsonRow> = view.rows.iter().map(depth_json_row_from_view).collect();

    let count = rows.len();
    crate::api::json_data_meta(
        "depth",
        rows,
        DepthMeta {
            season: season_str,
            season_type: season_type.label().to_owned(),
            count,
            scoring_mode: "pace",
        },
    )
}

fn depth_row_from_view(row: &DepthTeamStrengthRow) -> DepthRow {
    DepthRow {
        team: row.team.0.clone(),
        c_score: format!("{:.0}", row.c_score),
        lw_score: format!("{:.0}", row.lw_score),
        rw_score: format!("{:.0}", row.rw_score),
        d_score: format!("{:.0}", row.d_score),
        total: format!("{:.0}", row.total),
        c_top: row.c_top.clone(),
        lw_top: row.lw_top.clone(),
        rw_top: row.rw_top.clone(),
        d_top: row.d_top.clone(),
    }
}

fn depth_json_row_from_view(row: &DepthTeamStrengthRow) -> DepthJsonRow {
    DepthJsonRow {
        team: row.team.0.clone(),
        c_score: row.c_score,
        lw_score: row.lw_score,
        rw_score: row.rw_score,
        d_score: row.d_score,
        total: row.total,
        c_top: row.c_top.clone(),
        lw_top: row.lw_top.clone(),
        rw_top: row.rw_top.clone(),
        d_top: row.d_top.clone(),
    }
}
