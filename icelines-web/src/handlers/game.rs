use crate::state::WebState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::{
    GameBoxscoreInput, GameGoalInput, GameGoalRow, GameGoalieInput, GameGoalieRow, GameSkaterInput,
    GameSkaterRow, GameView, ViewContext, ViewWindow,
};

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
    let (active_label, season, season_type) = {
        let cfg = state.config.read().await;
        (
            cfg.active_label.clone(),
            cfg.active_season
                .parse::<u32>()
                .map(Season)
                .unwrap_or(Season(0)),
            SeasonType::parse_lossy(&cfg.active_season_type),
        )
    };
    let client = icelines_fetch::nhl_api::NhlApiClient::production();
    let rendered = match client.fetch_boxscore(id).await {
        Ok(boxscore) => GameTemplate {
            active_label,
            view: game_detail_from_view(&GameView::from_boxscore(
                ViewContext::new(ViewWindow::new(season, season_type)),
                boxscore_input(boxscore),
            )),
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

pub async fn get_game_json(State(state): State<WebState>, Path(id): Path<u64>) -> Response {
    let (season, season_type) = {
        let cfg = state.config.read().await;
        (
            cfg.active_season
                .parse::<u32>()
                .map(Season)
                .unwrap_or(Season(0)),
            SeasonType::parse_lossy(&cfg.active_season_type),
        )
    };
    let client = icelines_fetch::nhl_api::NhlApiClient::production();
    let (data, error) = match client.fetch_boxscore(id).await {
        Ok(boxscore) => {
            let view = GameView::from_boxscore(
                ViewContext::new(ViewWindow::new(season, season_type)),
                boxscore_input(boxscore),
            );
            (Some(game_detail_from_view(&view)), None)
        }
        Err(e) => (None, Some(e.to_string())),
    };
    crate::api::json_envelope("game", data, GameMeta { game_id: id }, error)
}

fn boxscore_input(boxscore: icelines_fetch::nhl_api::Boxscore) -> GameBoxscoreInput {
    GameBoxscoreInput {
        game_id: boxscore.game_id,
        away_abbrev: boxscore.away_abbrev,
        home_abbrev: boxscore.home_abbrev,
        away_score: boxscore.away_score,
        home_score: boxscore.home_score,
        game_state: boxscore.game_state,
        last_period: boxscore.last_period,
        goals: boxscore
            .goals
            .into_iter()
            .map(|goal| GameGoalInput {
                period: goal.period,
                time_in_period: goal.time_in_period,
                scorer_team: goal.scorer_team,
                scorer_name: goal.scorer_name,
            })
            .collect(),
        goalies: boxscore
            .goalies
            .into_iter()
            .map(|goalie| GameGoalieInput {
                player_id: goalie.player_id,
                player_name: goalie.player_name,
                saves: goalie.saves,
                shots: goalie.shots,
                decision: goalie.decision,
            })
            .collect(),
        away_skaters: boxscore
            .away_skaters
            .into_iter()
            .map(skater_input)
            .collect(),
        home_skaters: boxscore
            .home_skaters
            .into_iter()
            .map(skater_input)
            .collect(),
    }
}

fn skater_input(skater: icelines_fetch::nhl_api::SkaterLine) -> GameSkaterInput {
    GameSkaterInput {
        player_id: skater.player_id,
        player_name: skater.player_name,
        position: skater.position,
        goals: skater.goals,
        assists: skater.assists,
        plus_minus: skater.plus_minus,
    }
}

fn game_detail_from_view(view: &GameView) -> GameDetailView {
    GameDetailView {
        game_id: view.game_id.0,
        away_abbrev: view.away_abbrev.clone(),
        home_abbrev: view.home_abbrev.clone(),
        away_score: view.away_score,
        home_score: view.home_score,
        state_label: view.state_label.clone(),
        is_live: view.is_live,
        auto_refresh: view.auto_refresh,
        goalies: view.goalies.iter().map(goalie_from_view).collect(),
        goals: view.goals.iter().map(goal_from_view).collect(),
        away_top_skaters: view.away_top_skaters.iter().map(skater_from_view).collect(),
        home_top_skaters: view.home_top_skaters.iter().map(skater_from_view).collect(),
    }
}

fn goalie_from_view(goalie: &GameGoalieRow) -> GameGoalieView {
    GameGoalieView {
        player_id: goalie.player_id,
        player_name: goalie.player_name.clone(),
        saves: goalie.saves,
        shots: goalie.shots,
        decision: goalie.decision.clone(),
    }
}

fn goal_from_view(goal: &GameGoalRow) -> GameGoalView {
    GameGoalView {
        period: goal.period,
        time_in_period: goal.time_in_period.clone(),
        scorer_team: goal.scorer_team.clone(),
        scorer_name: goal.scorer_name.clone(),
    }
}

fn skater_from_view(skater: &GameSkaterRow) -> GameSkaterView {
    GameSkaterView {
        player_id: skater.player_id,
        player_name: skater.player_name.clone(),
        position: skater.position.clone(),
        goals: skater.goals,
        assists: skater.assists,
        points: skater.points,
        plus_minus: skater.plus_minus,
    }
}
