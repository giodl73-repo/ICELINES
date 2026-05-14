use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::identity::PlayerId;
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::{PlayerStreaksView, ViewContext, ViewWindow};

pub async fn get_player_streaks(
    State(state): State<crate::WebState>,
    Path(id): Path<u32>,
) -> Response {
    match build_streaks_view(&state, id).await {
        Ok((active_label, view)) => Html(render_streaks_html(&active_label, &view)).into_response(),
        Err(response) => response,
    }
}

pub async fn get_player_streaks_json(
    State(state): State<crate::WebState>,
    Path(id): Path<u32>,
) -> Response {
    match build_streaks_view(&state, id).await {
        Ok((_active_label, view)) => {
            let meta = serde_json::json!({
                "player_id": view.player_id,
                "games_loaded": view.games_loaded,
                "source_state": view.context.source_state,
            });
            crate::api::json_data_meta("player-streaks", view, meta)
        }
        Err(response) => response,
    }
}

async fn build_streaks_view(
    state: &crate::WebState,
    id: u32,
) -> Result<(String, PlayerStreaksView), Response> {
    let (active_label, context) = active_context(state).await?;
    let pid = PlayerId(id);
    {
        let mut repo = state.repo.write().await;
        if let Err(e) = icelines_fetch::stats_loader::load_player_career_into_repo(&mut repo, pid) {
            eprintln!("warn: streaks career fan-out for pid={id} failed: {e}");
        }
    }
    let player_name = {
        let repo = state.repo.read().await;
        match repo.identity(pid) {
            Some(identity) => identity.full_name.clone(),
            None => {
                return Err(crate::api::json_error_meta(
                    StatusCode::NOT_FOUND,
                    "player-streaks",
                    serde_json::json!({ "player_id": id }),
                    serde_json::json!({}),
                    format!("No player with NHL id {id} in the active repository."),
                ));
            }
        }
    };

    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .ok_or_else(|| {
            crate::api::json_error_meta(
                StatusCode::INTERNAL_SERVER_ERROR,
                "player-streaks",
                serde_json::json!({ "player_id": id }),
                serde_json::json!({}),
                "cannot determine home directory".to_string(),
            )
        })?;
    let data_root = home.join(".icelines").join("data");
    let store = icelines_fetch::datastore::DataStore::open(&data_root).map_err(|err| {
        crate::api::json_error_meta(
            StatusCode::INTERNAL_SERVER_ERROR,
            "player-streaks",
            serde_json::json!({ "player_id": id }),
            serde_json::json!({ "data_root": data_root.display().to_string() }),
            err.to_string(),
        )
    })?;
    let lines = icelines_fetch::streaks_provider::load_player_game_lines(&store, id);
    let player_name = lines
        .first()
        .map(|line| line.player_name.clone())
        .unwrap_or(player_name);
    let view = PlayerStreaksView::from_game_lines(context, id, player_name, &lines);
    Ok((active_label, view))
}

async fn active_context(state: &crate::WebState) -> Result<(String, ViewContext), Response> {
    let cfg = state.config.read().await;
    let season = cfg.active_season.parse::<u32>().map(Season).map_err(|_| {
        crate::api::json_error_meta(
            StatusCode::BAD_REQUEST,
            "player-streaks",
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

fn render_streaks_html(active_label: &str, view: &PlayerStreaksView) -> String {
    let mut rows = String::new();
    for row in &view.rows {
        rows.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            html_escape(&row.metric),
            row.current,
            row.longest,
            opt_str(row.longest_start_date.as_deref()),
            opt_str(row.longest_end_date.as_deref())
        ));
    }
    if view.games_loaded == 0 {
        rows.push_str("<tr><td colspan=\"5\">No cached boxscore game lines found. Run <code>icelines fetch boxscore --date YYYY-MM-DD</code>.</td></tr>");
    }
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{name} Streaks</title><link rel=\"stylesheet\" href=\"/static/style.css\"></head><body><header><a href=\"/\">IceLines</a> <span>{active}</span></header><main id=\"main\"><p><a href=\"/player/{pid}\">Back to player card</a> | <a href=\"/api/v1/player/{pid}/streaks\">JSON</a></p><h1>{name} Streaks</h1><p>{games} cached game lines. Source: boxscore skater rows; no streaks are inferred from season totals.</p><table><thead><tr><th>Metric</th><th>Current</th><th>Longest</th><th>Start</th><th>End</th></tr></thead><tbody>{rows}</tbody></table></main></body></html>",
        name = html_escape(&view.player_name),
        active = html_escape(active_label),
        pid = view.player_id,
        games = view.games_loaded,
        rows = rows
    )
}

fn opt_str(value: Option<&str>) -> String {
    value.unwrap_or("-").to_string()
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
