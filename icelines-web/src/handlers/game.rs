use crate::state::WebState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "game.html")]
struct GameTemplate {
    active_label: String,
    view: GameDetailView,
}

#[derive(Template)]
#[template(path = "game_error.html")]
struct GameErrorTemplate {
    active_label: String,
    game_id: u64,
    error: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct GameDetailView {
    game_id: u64,
    away_abbrev: String,
    home_abbrev: String,
    away_score: u8,
    home_score: u8,
    state_label: String,
    is_live: bool,
    auto_refresh: bool,
    goalies: Vec<GameGoalieView>,
    goals: Vec<GameGoalView>,
    away_top_skaters: Vec<GameSkaterView>,
    home_top_skaters: Vec<GameSkaterView>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct GameGoalieView {
    player_id: u32,
    player_name: String,
    saves: u32,
    shots: u32,
    decision: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct GameGoalView {
    period: u8,
    time_in_period: String,
    scorer_team: String,
    scorer_name: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct GameSkaterView {
    player_id: u32,
    player_name: String,
    position: String,
    goals: u32,
    assists: u32,
    points: u32,
    plus_minus: i32,
}

#[derive(Debug, serde::Serialize)]
struct GameMeta {
    game_id: u64,
}

pub async fn get_game(State(state): State<WebState>, Path(id): Path<u64>) -> Response {
    let active_label = state.config.read().await.active_label.clone();
    let client = icelines_fetch::nhl_api::NhlApiClient::production();
    let rendered = match client.fetch_boxscore(id).await {
        Ok(boxscore) => GameTemplate {
            active_label,
            view: project_game_detail(boxscore),
        }
        .render(),
        Err(e) => GameErrorTemplate {
            active_label,
            game_id: id,
            error: e.to_string(),
        }
        .render(),
    };
    match rendered {
        Ok(html) => Html(html).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("template render failed: {e}")),
        )
            .into_response(),
    }
}

pub async fn get_game_json(Path(id): Path<u64>) -> Response {
    let client = icelines_fetch::nhl_api::NhlApiClient::production();
    let (data, error) = match client.fetch_boxscore(id).await {
        Ok(boxscore) => (Some(project_game_detail(boxscore)), None),
        Err(e) => (None, Some(e.to_string())),
    };
    crate::api::json_envelope("game", data, GameMeta { game_id: id }, error)
}

fn project_game_detail(b: icelines_fetch::nhl_api::Boxscore) -> GameDetailView {
    let state = b.game_state.as_deref().unwrap_or("");
    let last = b.last_period.as_deref().unwrap_or("");
    let state_label = match (state, last) {
        ("FINAL" | "OFF", "OT") => "Final/OT",
        ("FINAL" | "OFF", "SO") => "Final/SO",
        ("FINAL" | "OFF", _) => "Final",
        ("LIVE" | "CRIT", _) => "LIVE",
        ("PRE", _) => "Pre-game",
        _ => "",
    }
    .to_owned();
    let is_live = matches!(state, "LIVE" | "CRIT");
    let auto_refresh = matches!(state, "LIVE" | "CRIT" | "PRE");

    let mut away_skaters = b.away_skaters;
    away_skaters.sort_by_key(|s| std::cmp::Reverse(s.goals + s.assists));
    let mut home_skaters = b.home_skaters;
    home_skaters.sort_by_key(|s| std::cmp::Reverse(s.goals + s.assists));

    GameDetailView {
        game_id: b.game_id,
        away_abbrev: b.away_abbrev,
        home_abbrev: b.home_abbrev,
        away_score: b.away_score,
        home_score: b.home_score,
        state_label,
        is_live,
        auto_refresh,
        goalies: b
            .goalies
            .into_iter()
            .map(|g| GameGoalieView {
                player_id: g.player_id,
                player_name: g.player_name,
                saves: g.saves,
                shots: g.shots,
                decision: g.decision,
            })
            .collect(),
        goals: b
            .goals
            .into_iter()
            .map(|g| GameGoalView {
                period: g.period,
                time_in_period: g.time_in_period,
                scorer_team: g.scorer_team,
                scorer_name: g.scorer_name,
            })
            .collect(),
        away_top_skaters: project_top_skaters(away_skaters),
        home_top_skaters: project_top_skaters(home_skaters),
    }
}

fn project_top_skaters(skaters: Vec<icelines_fetch::nhl_api::SkaterLine>) -> Vec<GameSkaterView> {
    skaters
        .into_iter()
        .take(5)
        .map(|s| {
            let points = s.goals + s.assists;
            GameSkaterView {
                player_id: s.player_id,
                player_name: s.player_name,
                position: s.position,
                goals: s.goals,
                assists: s.assists,
                points,
                plus_minus: s.plus_minus,
            }
        })
        .collect()
}
