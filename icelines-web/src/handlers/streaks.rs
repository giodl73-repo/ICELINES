use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::identity::PlayerId;
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::{
    Completeness, PlayerStreaksView, SourceKind, SourceState, ViewContext, ViewWindow,
};

#[derive(Debug, Clone, serde::Serialize)]
struct StreakSourceAuthority {
    source: &'static str,
    source_kind: SourceKind,
    state: Completeness,
    coverage_state: &'static str,
    basis: &'static str,
    covered_metrics: Vec<&'static str>,
    limitations: Vec<&'static str>,
    label: String,
}

pub async fn get_player_streaks(
    State(state): State<crate::WebState>,
    Path(id): Path<u32>,
) -> Response {
    match build_streaks_view(&state, id).await {
        Ok((active_label, view, cache_teams)) => {
            Html(render_streaks_html(&active_label, &view, &cache_teams)).into_response()
        }
        Err(response) => response,
    }
}

pub async fn get_player_streaks_json(
    State(state): State<crate::WebState>,
    Path(id): Path<u32>,
) -> Response {
    match build_streaks_view(&state, id).await {
        Ok((_active_label, view, _cache_teams)) => {
            let meta = serde_json::json!({
                "player_id": view.player_id,
                "games_loaded": view.games_loaded,
                "source_state": view.context.source_state,
                "source_authorities": player_streak_source_authorities(&view.context.source_state),
            });
            crate::api::json_data_meta("player-streaks", view, meta)
        }
        Err(response) => response,
    }
}

async fn build_streaks_view(
    state: &crate::WebState,
    id: u32,
) -> Result<(String, PlayerStreaksView, Vec<String>), Response> {
    let (active_label, context) = active_context(state).await?;
    let pid = PlayerId(id);
    {
        let mut repo = state.repo.write().await;
        if let Err(e) = icelines_fetch::stats_loader::load_player_career_into_repo(&mut repo, pid) {
            eprintln!("warn: streaks career fan-out for pid={id} failed: {e}");
        }
    }
    let (player_name, cache_teams) = {
        let repo = state.repo.read().await;
        match repo.identity(pid) {
            Some(identity) => (
                identity.full_name.clone(),
                player_cache_teams(
                    &repo,
                    pid,
                    context.window.season,
                    context.window.season_type,
                ),
            ),
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
    let (lines, shot_lines, play_by_play_source_loaded) = if data_root.join("manifest").is_dir() {
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
        let (shot_lines, source_loaded) =
            icelines_fetch::streaks_provider::load_player_shot_lines(&store, id);
        (lines, shot_lines, source_loaded)
    } else {
        (Vec::new(), Vec::new(), false)
    };
    let player_name = lines
        .first()
        .map(|line| line.player_name.clone())
        .or_else(|| shot_lines.first().map(|line| line.player_name.clone()))
        .unwrap_or(player_name);
    let view = PlayerStreaksView::from_game_and_shot_lines(
        context,
        id,
        player_name,
        &lines,
        &shot_lines,
        play_by_play_source_loaded,
    );
    Ok((active_label, view, cache_teams))
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

fn render_streaks_html(
    active_label: &str,
    view: &PlayerStreaksView,
    cache_teams: &[String],
) -> String {
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
        rows.push_str("<tr><td colspan=\"5\">No per-game rows are loaded yet for this player. Streaks need game order, so they read the local game cache instead of season totals.");
        if !cache_teams.is_empty() {
            rows.push_str(&format!(
                "<form method=\"post\" action=\"/admin/game-cache/load\" class=\"inline-form\"><input type=\"hidden\" name=\"season\" value=\"{}\"><input type=\"hidden\" name=\"season_type\" value=\"{}\"><input type=\"hidden\" name=\"teams\" value=\"{}\"><input type=\"hidden\" name=\"artifacts\" value=\"boxscore,play-by-play\"><input type=\"hidden\" name=\"return_to\" value=\"/player/{}/streaks\"><button type=\"submit\">Load game cache for this player</button></form>",
                view.context.window.season.0,
                html_escape(view.context.window.season_type.label()).as_str(),
                html_escape(&cache_teams.join(",")),
                view.player_id,
            ));
        }
        rows.push_str("</td></tr>");
    }
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{name} Streaks</title><link rel=\"stylesheet\" href=\"/static/style.css\"></head><body><header><a href=\"/\">IceLines</a> <span>{active}</span></header><main id=\"main\"><p><a href=\"/player/{pid}\">Back to player card</a> | <a href=\"/api/v1/player/{pid}/streaks\">JSON</a></p><h1>{name} Streaks</h1><p>{games} loaded game lines. Source: per-game boxscore and play-by-play rows; no streaks are inferred from season totals.</p><p>{authority}</p><table><thead><tr><th>Metric</th><th>Current</th><th>Longest</th><th>Start</th><th>End</th></tr></thead><tbody>{rows}</tbody></table></main></body></html>",
        name = html_escape(&view.player_name),
        active = html_escape(active_label),
        pid = view.player_id,
        games = view.games_loaded,
        authority = html_escape(&player_streak_authority_label(&view.context.source_state)),
        rows = rows
    )
}

fn player_streak_source_authorities(states: &[SourceState]) -> Vec<StreakSourceAuthority> {
    vec![
        streak_authority(
            states,
            SourceKind::Boxscore,
            "cached per-game boxscore skater rows",
            "boxscore game lines ordered by game date for player goal, assist, and point streaks",
            vec!["goal_streaks", "assist_streaks", "point_streaks"],
            vec![
                "does_not_include_shot_streaks",
                "does_not_include_shift_time",
                "does_not_include_expected_goals",
            ],
        ),
        streak_authority(
            states,
            SourceKind::PlayByPlay,
            "cached official NHL play-by-play shot rows",
            "play-by-play shot lines ordered by game date for player shot and attempt streaks",
            vec!["shots_on_goal_streaks", "shot_attempt_streaks"],
            vec![
                "does_not_include_goal_assist_point_streaks",
                "does_not_include_shift_time",
                "does_not_include_expected_goals",
            ],
        ),
    ]
}

fn streak_authority(
    states: &[SourceState],
    source_kind: SourceKind,
    source: &'static str,
    basis: &'static str,
    covered_metrics: Vec<&'static str>,
    limitations: Vec<&'static str>,
) -> StreakSourceAuthority {
    let state = states
        .iter()
        .find(|state| state.source == source_kind)
        .map(|state| state.state)
        .unwrap_or(Completeness::Unavailable);
    let coverage_state = match state {
        Completeness::Complete => "covered",
        Completeness::Partial => "partial",
        Completeness::Stale => "stale",
        Completeness::Unavailable => "unavailable",
    };
    let label = match state {
        Completeness::Complete => format!("Authority: {source} loaded for streak metrics"),
        Completeness::Partial => format!("Authority: partial {source} loaded for streak metrics"),
        Completeness::Stale => format!("Authority: stale {source} loaded for streak metrics"),
        Completeness::Unavailable => format!("Authority: {source} not loaded for streak metrics"),
    };
    StreakSourceAuthority {
        source,
        source_kind,
        state,
        coverage_state,
        basis,
        covered_metrics,
        limitations,
        label,
    }
}

fn player_streak_authority_label(states: &[SourceState]) -> String {
    player_streak_source_authorities(states)
        .into_iter()
        .map(|authority| authority.label)
        .collect::<Vec<_>>()
        .join("; ")
}

fn player_cache_teams(
    repo: &icelines_core::stats_repository::StatsRepository,
    pid: PlayerId,
    season: Season,
    season_type: SeasonType,
) -> Vec<String> {
    let Some(stats) = repo.season(pid, season, season_type) else {
        let Some(stats) = repo.career_all(pid).and_then(|rows| {
            rows.filter(|row| row.season <= season && row.season_type == season_type)
                .max_by_key(|row| row.season)
        }) else {
            return Vec::new();
        };
        return sorted_teams_from_stats(stats);
    };
    sorted_teams_from_stats(stats)
}

fn sorted_teams_from_stats(stats: &icelines_core::season_stats::SeasonStats) -> Vec<String> {
    let mut teams: Vec<String> = stats
        .team_stints
        .iter()
        .map(|stint| stint.team.0.clone())
        .collect();
    teams.sort();
    teams.dedup();
    teams
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
