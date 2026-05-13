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
pub(super) struct GameDetailView {
    pub(super) game_id: u64,
    pub(super) away_abbrev: String,
    pub(super) home_abbrev: String,
    pub(super) away_score: u8,
    pub(super) home_score: u8,
    pub(super) state_label: String,
    pub(super) is_live: bool,
    pub(super) auto_refresh: bool,
    pub(super) goalies: Vec<GameGoalieView>,
    pub(super) goals: Vec<GameGoalView>,
    pub(super) away_top_skaters: Vec<GameSkaterView>,
    pub(super) home_top_skaters: Vec<GameSkaterView>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(super) struct GameGoalieView {
    pub(super) player_id: u32,
    pub(super) player_name: String,
    pub(super) team_abbrev: String,
    pub(super) saves: u32,
    pub(super) shots: u32,
    pub(super) decision: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(super) struct GameGoalView {
    pub(super) period: u8,
    pub(super) period_type: String,
    pub(super) time_in_period: String,
    pub(super) scorer_team: String,
    pub(super) scorer_name: String,
    pub(super) assist1_name: Option<String>,
    pub(super) assist2_name: Option<String>,
    pub(super) away_score: u8,
    pub(super) home_score: u8,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(super) struct GameSkaterView {
    pub(super) player_id: u32,
    pub(super) player_name: String,
    pub(super) position: String,
    pub(super) goals: u32,
    pub(super) assists: u32,
    pub(super) points: u32,
    pub(super) plus_minus: i32,
}

#[derive(Debug, serde::Serialize)]
struct GameMeta {
    game_id: u64,
    source_error: Option<String>,
}

pub(super) async fn build_game_detail(state: &WebState, id: u64) -> Result<GameDetailView, String> {
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
    let boxscore = client.fetch_boxscore(id).await.map_err(|e| e.to_string())?;
    Ok(game_detail_from_view(&GameView::from_boxscore(
        ViewContext::new(ViewWindow::new(season, season_type)),
        boxscore_input(boxscore),
    )))
}

pub async fn get_game(State(state): State<WebState>, Path(id): Path<u64>) -> Response {
    let active_label = {
        let cfg = state.config.read().await;
        cfg.active_label.clone()
    };
    let rendered = match build_game_detail(&state, id).await {
        Ok(view) => GameTemplate { active_label, view }.render(),
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
    let (data, source_error) = match build_game_detail(&state, id).await {
        Ok(view) => (Some(view), None),
        Err(e) => (None, Some(e.to_string())),
    };
    crate::api::json_data_meta(
        "game",
        data,
        GameMeta {
            game_id: id,
            source_error,
        },
    )
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
                period_type: goal.period_type,
                time_in_period: goal.time_in_period,
                scorer_team: goal.scorer_team,
                scorer_name: goal.scorer_name,
                assist1_name: goal.assist1_name,
                assist2_name: goal.assist2_name,
                away_score: goal.away_score,
                home_score: goal.home_score,
            })
            .collect(),
        goalies: boxscore
            .goalies
            .into_iter()
            .map(|goalie| GameGoalieInput {
                player_id: goalie.player_id,
                player_name: goalie.player_name,
                team_abbrev: goalie.team_abbrev,
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
        toi_seconds: skater.toi_seconds,
        sog: skater.sog,
        hits: skater.hits,
        blocked_shots: skater.blocked_shots,
        takeaways: skater.takeaways,
        giveaways: skater.giveaways,
        goals: skater.goals,
        assists: skater.assists,
        plus_minus: skater.plus_minus,
    }
}

pub(super) fn game_detail_from_view(view: &GameView) -> GameDetailView {
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
        team_abbrev: goalie.team_abbrev.clone(),
        saves: goalie.saves,
        shots: goalie.shots,
        decision: goalie.decision.clone(),
    }
}

fn goal_from_view(goal: &GameGoalRow) -> GameGoalView {
    GameGoalView {
        period: goal.period,
        period_type: goal.period_type.clone(),
        time_in_period: goal.time_in_period.clone(),
        scorer_team: goal.scorer_team.clone(),
        scorer_name: goal.scorer_name.clone(),
        assist1_name: goal.assist1_name.clone(),
        assist2_name: goal.assist2_name.clone(),
        away_score: goal.away_score,
        home_score: goal.home_score,
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
