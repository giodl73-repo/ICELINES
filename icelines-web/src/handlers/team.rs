use crate::state::WebState;
use crate::templates::{GoalieRow, LeaderRow, TeamTemplate};
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::model::{Season, TeamAbbr};
use icelines_core::season_stats::SeasonType;

pub async fn get_team(State(state): State<WebState>, Path(abbrev_raw): Path<String>) -> Response {
    let abbrev_upper = abbrev_raw.to_ascii_uppercase();
    let team = match TeamAbbr::parse(&abbrev_upper) {
        Ok(t) => t,
        Err(e) => {
            return (
                StatusCode::NOT_FOUND,
                Html(format!(
                    "<!doctype html><html><body><h1>Unknown team</h1>\
                             <p>'{abbrev_upper}' is not a recognized NHL team abbrev: {e}</p>\
                             <p><a href=\"/leaders\">← back to leaders</a></p>\
                             </body></html>"
                )),
            )
                .into_response();
        }
    };

    let (season_str, season_type, active_label) = {
        let cfg = state.config.read().await;
        let st = match cfg.active_season_type.as_str() {
            "playoff" | "playoffs" => SeasonType::Playoff,
            _ => SeasonType::Regular,
        };
        (cfg.active_season.clone(), st, cfg.active_label.clone())
    };
    let season_u32: u32 = match season_str.parse() {
        Ok(n) => n,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(format!(
                    "<!doctype html><html><body><h1>500</h1>\
                             <p>Active season '{season_str}' is not a YYYYZZZZ id</p>\
                             </body></html>"
                )),
            )
                .into_response();
        }
    };
    let season = Season(season_u32);

    let (skaters, goalies) = {
        let repo = state.repo.read().await;
        let roster = repo.team_roster(&team, season, season_type);

        let mut skaters: Vec<LeaderRow> = roster
            .iter()
            .filter(|v| !v.is_goalie())
            .map(|v| super::shared::project_leader_row(v))
            .collect();
        skaters.sort_by(|a, b| {
            b.points
                .cmp(&a.points)
                .then(b.goals.cmp(&a.goals))
                .then(a.name.cmp(&b.name))
        });

        let mut goalies: Vec<GoalieRow> = roster
            .iter()
            .filter(|v| v.is_goalie())
            .filter_map(|v| {
                let g = v.stats.goalie.as_ref()?;
                let save_pct_str = match g.save_pct {
                    Some(p) => format!("{:.3}", p),
                    None => "—".to_owned(),
                };
                let gaa_str = match g.goals_against_average {
                    Some(a) => format!("{:.2}", a),
                    None => "—".to_owned(),
                };
                let team_display = v.team_display().to_owned();
                let headshot_url = super::shared::build_headshot_url_for_display(
                    v.season().0,
                    &team_display,
                    v.id().0,
                );
                Some(GoalieRow {
                    nhl_id: v.id().0,
                    name: v.full_name().to_owned(),
                    team: team_display,
                    gp: v.gp(),
                    wins: g.wins,
                    losses: g.losses,
                    shutouts: g.shutouts,
                    save_pct_str,
                    gaa_str,
                    headshot_url,
                    headshot_fallback_url: format!(
                        "https://assets.nhle.com/mugs/nhl/default/{}.png",
                        v.id().0
                    ),
                })
            })
            .collect();
        goalies.sort_by(|a, b| b.wins.cmp(&a.wins).then(a.name.cmp(&b.name)));

        (skaters, goalies)
    };

    let tmpl = TeamTemplate {
        active_label,
        team_abbrev: team.0.to_string(),
        skaters,
        goalies,
    };
    match tmpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("template render failed: {e}")),
        )
            .into_response(),
    }
}

// ── King.4.2 — JSON twin ──────────────────────────────────────

#[derive(Debug, serde::Serialize)]
pub struct TeamData {
    pub team_abbrev: String,
    pub skaters: Vec<TeamSkaterRow>,
    pub goalies: Vec<TeamGoalieRow>,
}

#[derive(Debug, serde::Serialize)]
pub struct TeamSkaterRow {
    pub nhl_id: u32,
    pub name: String,
    pub position: String,
    pub games: u32,
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
    pub points_per_game: Option<f64>,
}

#[derive(Debug, serde::Serialize)]
pub struct TeamGoalieRow {
    pub nhl_id: u32,
    pub name: String,
    pub games: u32,
    pub wins: u32,
    pub losses: u32,
    pub shutouts: u32,
    pub save_pct: Option<f64>,
    pub goals_against_average: Option<f64>,
}

#[derive(Debug, serde::Serialize)]
pub struct TeamMeta {
    pub team_abbrev: String,
    pub season: String,
    pub season_type: String,
    pub skater_count: usize,
    pub goalie_count: usize,
}

/// `GET /api/v1/team/:abbrev` — JSON twin of `/team/:abbrev`.
pub async fn get_team_json(
    State(state): State<WebState>,
    Path(abbrev_raw): Path<String>,
) -> Response {
    let abbrev_upper = abbrev_raw.to_ascii_uppercase();
    let team = match TeamAbbr::parse(&abbrev_upper) {
        Ok(t) => t,
        Err(e) => {
            return (
                StatusCode::NOT_FOUND,
                axum::Json(serde_json::json!({
                    "error": "unknown_team",
                    "message": format!(
                        "'{abbrev_upper}' is not a recognized NHL team abbrev: {e}"
                    ),
                    "team_abbrev": abbrev_upper,
                })),
            )
                .into_response();
        }
    };

    let (season_str, season_type) = {
        let cfg = state.config.read().await;
        let st = match cfg.active_season_type.as_str() {
            "playoff" | "playoffs" => SeasonType::Playoff,
            _ => SeasonType::Regular,
        };
        (cfg.active_season.clone(), st)
    };
    let season_u32: u32 = match season_str.parse() {
        Ok(n) => n,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "error": "bad_active_season",
                    "message": format!("Season '{season_str}' is not a valid YYYYZZZZ id"),
                })),
            )
                .into_response();
        }
    };
    let season = Season(season_u32);

    let (skaters, goalies) = {
        let repo = state.repo.read().await;
        let roster = repo.team_roster(&team, season, season_type);

        let mut skaters: Vec<TeamSkaterRow> = roster
            .iter()
            .filter(|v| !v.is_goalie())
            .map(|v| {
                let gp = v.gp();
                let points = v.points();
                let ppg = if gp > 0 {
                    Some((points as f64) / (gp as f64))
                } else {
                    None
                };
                TeamSkaterRow {
                    nhl_id: v.id().0,
                    name: v.full_name().to_owned(),
                    position: v.position().abbreviation().to_owned(),
                    games: gp,
                    goals: v.goals(),
                    assists: v.assists(),
                    points,
                    points_per_game: ppg,
                }
            })
            .collect();
        skaters.sort_by(|a, b| {
            b.points
                .cmp(&a.points)
                .then(b.goals.cmp(&a.goals))
                .then(a.name.cmp(&b.name))
        });

        let mut goalies: Vec<TeamGoalieRow> = roster
            .iter()
            .filter(|v| v.is_goalie())
            .filter_map(|v| {
                let g = v.stats.goalie.as_ref()?;
                Some(TeamGoalieRow {
                    nhl_id: v.id().0,
                    name: v.full_name().to_owned(),
                    games: v.gp(),
                    wins: g.wins,
                    losses: g.losses,
                    shutouts: g.shutouts,
                    save_pct: g.save_pct.map(f64::from),
                    goals_against_average: g.goals_against_average.map(f64::from),
                })
            })
            .collect();
        goalies.sort_by(|a, b| b.wins.cmp(&a.wins).then(a.name.cmp(&b.name)));

        (skaters, goalies)
    };

    let meta = TeamMeta {
        team_abbrev: team.0.to_string(),
        season: season_str,
        season_type: match season_type {
            SeasonType::Regular => "regular".to_owned(),
            SeasonType::Playoff => "playoff".to_owned(),
        },
        skater_count: skaters.len(),
        goalie_count: goalies.len(),
    };
    let data = TeamData {
        team_abbrev: team.0.to_string(),
        skaters,
        goalies,
    };
    crate::api::json_data_meta("team", data, meta)
}
