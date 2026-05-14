use crate::state::WebState;
use crate::templates::{RecordsTemplate, RecordsTemplateRow};
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::identity::PlayerId;
use icelines_core::model::{Season, TeamAbbr};
use icelines_core::season_stats::SeasonType;
use icelines_core::{
    PlayerRecordsView, RecordsOpponentRow, TeamRecordsView, ViewContext, ViewWindow,
};

pub async fn get_player_records(State(state): State<WebState>, Path(id): Path<u32>) -> Response {
    let (active_label, view) = match build_player_records_view(&state, id).await {
        Ok(result) => result,
        Err(response) => return response,
    };
    let template = records_template(RecordsTemplateInput {
        active_label,
        title: format!("{} Records", view.player_name),
        subtitle: "NHL teams scored against".to_string(),
        back_href: format!("/player/{id}"),
        back_label: "player card".to_string(),
        json_href: format!("/api/v1/records/player/{id}"),
        subject_label: "opponent team".to_string(),
        empty_hint: "No goal records found in local boxscores. Run `icelines fetch boxscore --date YYYY-MM-DD` to populate this record.".to_string(),
        rows: &view.rows,
    });
    render_template(template)
}

pub async fn get_player_records_json(
    State(state): State<WebState>,
    Path(id): Path<u32>,
) -> Response {
    match build_player_records_view(&state, id).await {
        Ok((_active_label, view)) => {
            let meta = serde_json::json!({
                "player_id": view.player_id,
                "player_name": view.player_name.clone(),
                "metric": view.metric.clone(),
                "rows": view.rows.len(),
                "incomplete_goal_rows": view.incomplete_goal_rows,
                "source_state": view.context.source_state.clone(),
            });
            crate::api::json_data_meta("records-player", view, meta)
        }
        Err(response) => response,
    }
}

pub async fn get_team_records(
    State(state): State<WebState>,
    Path(abbrev): Path<String>,
) -> Response {
    let (active_label, view) = match build_team_records_view(&state, &abbrev).await {
        Ok(result) => result,
        Err(response) => return response,
    };
    let template = records_template(RecordsTemplateInput {
        active_label,
        title: format!("{} Records", view.team),
        subtitle: "Players who scored against this team".to_string(),
        back_href: format!("/team/{}/season", view.team),
        back_label: "team season".to_string(),
        json_href: format!("/api/v1/records/team/{}", view.team),
        subject_label: "player".to_string(),
        empty_hint: "No team goal records found in local boxscores. Run `icelines fetch boxscore --date YYYY-MM-DD` to populate this record.".to_string(),
        rows: &view.rows,
    });
    render_template(template)
}

pub async fn get_team_records_json(
    State(state): State<WebState>,
    Path(abbrev): Path<String>,
) -> Response {
    match build_team_records_view(&state, &abbrev).await {
        Ok((_active_label, view)) => {
            let meta = serde_json::json!({
                "team": view.team.clone(),
                "metric": view.metric.clone(),
                "rows": view.rows.len(),
                "incomplete_goal_rows": view.incomplete_goal_rows,
                "source_state": view.context.source_state.clone(),
            });
            crate::api::json_data_meta("records-team", view, meta)
        }
        Err(response) => response,
    }
}

async fn build_player_records_view(
    state: &WebState,
    id: u32,
) -> Result<(String, PlayerRecordsView), Response> {
    let (active_label, context) = active_context(state, "records-player").await?;
    let pid = PlayerId(id);
    {
        let mut repo = state.repo.write().await;
        if let Err(e) = icelines_fetch::stats_loader::load_player_career_into_repo(&mut repo, pid) {
            eprintln!("warn: records career fan-out for pid={id} failed: {e}");
        }
    }
    let player_name = {
        let repo = state.repo.read().await;
        match repo.identity(pid) {
            Some(identity) => identity.full_name.clone(),
            None => {
                return Err(crate::api::json_error_meta(
                    StatusCode::NOT_FOUND,
                    "records-player",
                    serde_json::json!({ "player_id": id }),
                    serde_json::json!({}),
                    format!("No player with NHL id {id} in the active repository."),
                ));
            }
        }
    };
    let goals = icelines_fetch::records_provider::load_goal_record_inputs_from_default_store()
        .map_err(|err| server_error("records-player", err))?;
    Ok((
        active_label,
        PlayerRecordsView::teams_scored_against(context, id, player_name, &goals),
    ))
}

async fn build_team_records_view(
    state: &WebState,
    abbrev: &str,
) -> Result<(String, TeamRecordsView), Response> {
    let (active_label, context) = active_context(state, "records-team").await?;
    let team = TeamAbbr::parse(abbrev).map_err(|_| {
        crate::api::json_error_meta(
            StatusCode::NOT_FOUND,
            "records-team",
            serde_json::json!({ "team": abbrev.to_ascii_uppercase() }),
            serde_json::json!({}),
            format!("'{}' is not a valid NHL team abbreviation", abbrev),
        )
    })?;
    let goals = icelines_fetch::records_provider::load_goal_record_inputs_from_default_store()
        .map_err(|err| server_error("records-team", err))?;
    Ok((
        active_label,
        TeamRecordsView::players_scored_against_team(context, team.0, &goals),
    ))
}

async fn active_context(
    state: &WebState,
    route: &'static str,
) -> Result<(String, ViewContext), Response> {
    let cfg = state.config.read().await;
    let season = cfg.active_season.parse::<u32>().map(Season).map_err(|_| {
        crate::api::json_error_meta(
            StatusCode::BAD_REQUEST,
            route,
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

struct RecordsTemplateInput<'a> {
    active_label: String,
    title: String,
    subtitle: String,
    back_href: String,
    back_label: String,
    json_href: String,
    subject_label: String,
    empty_hint: String,
    rows: &'a [RecordsOpponentRow],
}

fn records_template(input: RecordsTemplateInput<'_>) -> RecordsTemplate {
    RecordsTemplate {
        active_label: input.active_label,
        title: input.title,
        subtitle: input.subtitle,
        back_href: input.back_href,
        back_label: input.back_label,
        json_href: input.json_href,
        subject_label: input.subject_label,
        empty_hint: input.empty_hint,
        total: input.rows.len(),
        rows: input.rows.iter().map(record_row).collect(),
    }
}

fn record_row(row: &RecordsOpponentRow) -> RecordsTemplateRow {
    RecordsTemplateRow {
        key: row.key.clone(),
        label: row.label.clone(),
        count: row.count,
        first_game_id: row.first_game_id,
        first_date: row.first_date.clone().unwrap_or_default(),
        last_game_id: row.last_game_id,
        last_date: row.last_date.clone().unwrap_or_default(),
    }
}

fn render_template(template: RecordsTemplate) -> Response {
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("template render failed: {e}")),
        )
            .into_response(),
    }
}

fn server_error(route: &'static str, err: anyhow::Error) -> Response {
    crate::api::json_error_meta(
        StatusCode::INTERNAL_SERVER_ERROR,
        route,
        serde_json::json!({}),
        serde_json::json!({}),
        err.to_string(),
    )
}
