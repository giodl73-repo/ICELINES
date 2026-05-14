use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::identity::PlayerId;
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::{PlayerAwardsView, ViewContext, ViewWindow};

pub async fn get_player_awards(
    State(state): State<crate::WebState>,
    Path(id): Path<u32>,
) -> Response {
    match build_awards_view(&state, id).await {
        Ok((active_label, view)) => Html(render_awards_html(&active_label, &view)).into_response(),
        Err(response) => response,
    }
}

pub async fn get_player_awards_json(
    State(state): State<crate::WebState>,
    Path(id): Path<u32>,
) -> Response {
    match build_awards_view(&state, id).await {
        Ok((_active_label, view)) => {
            let meta = serde_json::json!({
                "player_id": view.player_id,
                "trophies": view.trophy_count(),
                "trophy_seasons": view.season_count(),
                "source_state": view.context.source_state,
            });
            crate::api::json_data_meta("player-awards", view, meta)
        }
        Err(response) => response,
    }
}

async fn build_awards_view(
    state: &crate::WebState,
    id: u32,
) -> Result<(String, PlayerAwardsView), Response> {
    let (active_label, context) = active_context(state).await?;
    let pid = PlayerId(id);
    {
        let mut repo = state.repo.write().await;
        if let Err(e) = icelines_fetch::stats_loader::load_player_career_into_repo(&mut repo, pid) {
            eprintln!("warn: awards career fan-out for pid={id} failed: {e}");
        }
    }
    let player_name = {
        let repo = state.repo.read().await;
        match repo.identity(pid) {
            Some(identity) => identity.full_name.clone(),
            None => {
                return Err(crate::api::json_error_meta(
                    StatusCode::NOT_FOUND,
                    "player-awards",
                    serde_json::json!({ "player_id": id }),
                    serde_json::json!({}),
                    format!("No player with NHL id {id} in the active repository."),
                ));
            }
        }
    };

    let view = super::nhl_client()
        .fetch_player_awards(id, &player_name, context)
        .await
        .map_err(|err| {
            crate::api::json_error_meta(
                StatusCode::BAD_GATEWAY,
                "player-awards",
                serde_json::json!({ "player_id": id }),
                serde_json::json!({}),
                err.to_string(),
            )
        })?;
    Ok((active_label, view))
}

async fn active_context(state: &crate::WebState) -> Result<(String, ViewContext), Response> {
    let cfg = state.config.read().await;
    let season = cfg.active_season.parse::<u32>().map(Season).map_err(|_| {
        crate::api::json_error_meta(
            StatusCode::BAD_REQUEST,
            "player-awards",
            serde_json::json!({}),
            serde_json::json!({ "season": cfg.active_season }),
            format!("Season '{}' is not a valid YYYYZZZZ id", cfg.active_season),
        )
    })?;
    let season_type = SeasonType::parse_lossy(&cfg.active_season_type);
    Ok((
        cfg.active_label.clone(),
        ViewContext::new(ViewWindow::new(season, season_type)),
    ))
}

fn render_awards_html(active_label: &str, view: &PlayerAwardsView) -> String {
    let mut rows = String::new();
    for award in &view.awards {
        if award.seasons.is_empty() {
            rows.push_str(&format!(
                "<tr><td>{}</td><td colspan=\"7\">No season rows</td></tr>",
                html_escape(&award.trophy)
            ));
            continue;
        }
        for season in &award.seasons {
            rows.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                html_escape(&award.trophy),
                season.season.0,
                game_type_label(season.game_type_id),
                opt_u32(season.games_played),
                opt_u32(season.goals),
                opt_u32(season.assists),
                opt_u32(season.points),
                season.plus_minus.map(|v| v.to_string()).unwrap_or_else(|| "-".to_string())
            ));
        }
    }
    if rows.is_empty() {
        rows.push_str("<tr><td colspan=\"8\">No NHL awards found for this player.</td></tr>");
    }
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{name} Trophy Case</title><link rel=\"stylesheet\" href=\"/static/style.css\"></head><body><header><a href=\"/\">IceLines</a> <span>{active}</span></header><main id=\"main\"><p><a href=\"/player/{pid}\">Back to player card</a> | <a href=\"/api/v1/player/{pid}/awards\">JSON</a></p><h1>{name} Trophy Case</h1><p>{trophies} trophies, {seasons} trophy seasons. Source: NHL landing awards[].</p><table><thead><tr><th>Trophy</th><th>Season</th><th>Type</th><th>GP</th><th>G</th><th>A</th><th>P</th><th>+/-</th></tr></thead><tbody>{rows}</tbody></table></main></body></html>",
        name = html_escape(&view.player_name),
        active = html_escape(active_label),
        pid = view.player_id,
        trophies = view.trophy_count(),
        seasons = view.season_count(),
        rows = rows
    )
}

fn opt_u32(value: Option<u32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn game_type_label(game_type_id: u8) -> &'static str {
    match game_type_id {
        2 => "Regular",
        3 => "Playoffs",
        _ => "Other",
    }
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
