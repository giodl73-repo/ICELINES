use crate::{
    goal_analysis::{script_json, GoalChoice, Page},
    WebState,
};
use askama::Template;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Json,
};
use icelines_fetch::goal_analysis::{self, GoalTarget};
use serde::Deserialize;
#[derive(Default, Deserialize)]
pub struct Params {
    target: Option<String>,
    game_id: Option<u64>,
}
fn root(s: &WebState) -> std::path::PathBuf {
    s.goal_analysis_root
        .as_ref()
        .clone()
        .unwrap_or_else(goal_analysis::default_data_root)
}
pub async fn page(State(state): State<WebState>, Query(p): Query<Params>) -> Response {
    let label = state.config.read().await.active_label.clone();
    let mut page = Page {
        active_label: label,
        data_json: "null".into(),
        target: p.target.clone().unwrap_or_default(),
        message: String::new(),
        choices: vec![],
        standalone: false,
    };
    let mut status = StatusCode::OK;
    if let Some(input) = p.target.filter(|s| !s.trim().is_empty()) {
        match GoalTarget::parse(&input) {
            Ok(target) => {
                match goal_analysis::prepare(target, &root(&state), state.goal_analysis_live, None)
                    .await
                {
                    Ok(analysis) => match script_json(&analysis) {
                        Ok(data) => page.data_json = data,
                        Err(_) => {
                            status = StatusCode::INTERNAL_SERVER_ERROR;
                            page.message = "Goal analysis could not be rendered.".into();
                        }
                    },
                    Err(e) => {
                        status = StatusCode::UNPROCESSABLE_ENTITY;
                        page.message = e.to_string();
                    }
                }
            }
            Err(e) => {
                status = StatusCode::BAD_REQUEST;
                page.message = e.to_string();
            }
        }
    } else if let Some(game) = p.game_id {
        match goal_analysis::discover_game(game, &root(&state), state.goal_analysis_live)
            .await
            .and_then(|landing| Ok(icelines_fetch::goal_visualizer::discover_goals(&landing)?))
        {
            Ok(mut goals) => {
                goals.sort_by_key(|g| (g.period, g.time_in_period.clone()));
                page.choices = goals
                    .into_iter()
                    .map(|g| GoalChoice {
                        target: format!("{game}/{}", g.event_id),
                        label: format!(
                            "P{} {} · {} · {}",
                            g.period,
                            g.time_in_period,
                            g.team_abbrev.unwrap_or_default(),
                            g.scorer_name.unwrap_or_else(|| "Unknown scorer".into())
                        ),
                        available: g.replay_url.is_some(),
                    })
                    .collect();
                if page.choices.is_empty() {
                    page.message = "No goals are recorded for this game.".into();
                }
            }
            Err(e) => {
                status = StatusCode::UNPROCESSABLE_ENTITY;
                page.message = e.to_string();
            }
        }
    }
    match page.render() {
        Ok(html) => (status, Html(html)).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Goal analysis could not be rendered",
        )
            .into_response(),
    }
}
pub async fn json(State(state): State<WebState>, Query(p): Query<Params>) -> Response {
    let result = async {
        let target = GoalTarget::parse(p.target.as_deref().unwrap_or(""))?;
        goal_analysis::prepare(target, &root(&state), state.goal_analysis_live, None).await
    }
    .await;
    match result {
        Ok(data) => Json(data).into_response(),
        Err(e) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({"status":"unavailable","message":e.to_string()})),
        )
            .into_response(),
    }
}
