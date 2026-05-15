use crate::state::WebState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::model::{Season, TeamAbbr};
use icelines_core::season_stats::SeasonType;
use icelines_core::{
    Completeness, GameScoringReportView, ScoringEventInput, ScoringEventSummary,
    ScoringShooterSummary, ScoringSplitSummary, ShotEventKind, TeamScoringProfileView, ViewContext,
    ViewWindow,
};

#[derive(Template)]
#[template(path = "scoring_report.html")]
struct ScoringReportTemplate {
    active_label: String,
    page: ScoringReportPage,
}

#[derive(Debug, Clone)]
struct ScoringReportPage {
    title: String,
    subtitle: String,
    back_href: String,
    back_label: String,
    api_href: String,
    source_loaded: bool,
    source_label: String,
    summary: ScoringSummaryTemplateRow,
    team_summaries: Vec<ScoringSplitTemplateRow>,
    period_summaries: Vec<ScoringSplitTemplateRow>,
    situation_summaries: Vec<ScoringSplitTemplateRow>,
    top_shooters: Vec<ScoringShooterTemplateRow>,
    events: Vec<ScoringEventTemplateRow>,
    load_form: Option<ScoringLoadForm>,
}

#[derive(Debug, Clone)]
struct ScoringLoadForm {
    season: String,
    season_type: String,
    teams: String,
    return_to: String,
}

#[derive(Debug, Clone)]
struct ScoringSummaryTemplateRow {
    goals: u32,
    shots_on_goal: u32,
    missed_shots: u32,
    blocked_shots: u32,
    shot_attempts: u32,
    unblocked_attempts: u32,
    shot_pct: String,
}

#[derive(Debug, Clone)]
struct ScoringSplitTemplateRow {
    label: String,
    summary: ScoringSummaryTemplateRow,
}

#[derive(Debug, Clone)]
struct ScoringShooterTemplateRow {
    player_id: u32,
    summary: ScoringSummaryTemplateRow,
}

#[derive(Debug, Clone)]
struct ScoringEventTemplateRow {
    period: String,
    time: String,
    kind: String,
    team: String,
    shooter: String,
    goalie: String,
    shot_type: String,
    location: String,
}

pub async fn get_game_scoring(State(state): State<WebState>, Path(id): Path<u64>) -> Response {
    match build_game_scoring_view(&state, id).await {
        Ok((active_label, view)) => {
            let page = game_scoring_page(&view);
            render_scoring_template(active_label, page)
        }
        Err(response) => *response,
    }
}

pub async fn get_game_scoring_json(State(state): State<WebState>, Path(id): Path<u64>) -> Response {
    match build_game_scoring_view(&state, id).await {
        Ok((_active_label, view)) => {
            let meta = serde_json::json!({
                "game_id": view.game_id,
                "source_state": view.context.source_state,
            });
            crate::api::json_data_meta("game-scoring", view, meta)
        }
        Err(response) => *response,
    }
}

pub async fn get_team_scoring(
    State(state): State<WebState>,
    Path(abbrev_raw): Path<String>,
) -> Response {
    match build_team_scoring_view(&state, &abbrev_raw).await {
        Ok((active_label, season, season_type, view)) => {
            let page = team_scoring_page(&view, season, season_type);
            render_scoring_template(active_label, page)
        }
        Err(response) => *response,
    }
}

pub async fn get_team_scoring_json(
    State(state): State<WebState>,
    Path(abbrev_raw): Path<String>,
) -> Response {
    match build_team_scoring_view(&state, &abbrev_raw).await {
        Ok((_active_label, _season, _season_type, view)) => {
            let meta = serde_json::json!({
                "team_abbrev": view.team,
                "season": view.context.window.season.0.to_string(),
                "season_type": view.context.window.season_type.label(),
                "source_state": view.context.source_state,
            });
            crate::api::json_data_meta("team-scoring", view, meta)
        }
        Err(response) => *response,
    }
}

fn render_scoring_template(active_label: String, page: ScoringReportPage) -> Response {
    match (ScoringReportTemplate { active_label, page }).render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("template render failed: {e}")),
        )
            .into_response(),
    }
}

async fn build_game_scoring_view(
    state: &WebState,
    game_id: u64,
) -> Result<(String, GameScoringReportView), Box<Response>> {
    let (active_label, season, season_type) = active_window(state).await?;
    let store = open_data_store("game-scoring")?;
    let context = ViewContext::new(ViewWindow::new(season, season_type));
    let view = icelines_fetch::scoring_provider::load_game_scoring_report(&store, context, game_id);
    Ok((active_label, view))
}

async fn build_team_scoring_view(
    state: &WebState,
    abbrev_raw: &str,
) -> Result<(String, Season, SeasonType, TeamScoringProfileView), Box<Response>> {
    let team = parse_team(abbrev_raw)?;
    let (active_label, season, season_type) = active_window(state).await?;
    let store = open_data_store("team-scoring")?;
    let context = ViewContext::new(ViewWindow::new(season, season_type));
    let view =
        icelines_fetch::scoring_provider::load_team_scoring_profile(&store, context, &team.0);
    Ok((active_label, season, season_type, view))
}

async fn active_window(state: &WebState) -> Result<(String, Season, SeasonType), Box<Response>> {
    let cfg = state.config.read().await;
    let season = cfg.active_season.parse::<u32>().map(Season).map_err(|_| {
        Box::new(crate::api::json_error_meta(
            StatusCode::BAD_REQUEST,
            "scoring",
            serde_json::json!({}),
            serde_json::json!({ "season": cfg.active_season }),
            format!("Season '{}' is not a valid YYYYZZZZ id", cfg.active_season),
        ))
    })?;
    Ok((
        cfg.active_label.clone(),
        season,
        SeasonType::parse_lossy(&cfg.active_season_type),
    ))
}

fn open_data_store(
    surface: &'static str,
) -> Result<icelines_fetch::datastore::DataStore, Box<Response>> {
    let data_root = data_root().ok_or_else(|| {
        Box::new(crate::api::json_error_meta(
            StatusCode::INTERNAL_SERVER_ERROR,
            surface,
            serde_json::json!({}),
            serde_json::json!({}),
            "cannot determine home directory".to_string(),
        ))
    })?;
    icelines_fetch::datastore::DataStore::open(&data_root).map_err(|err| {
        Box::new(crate::api::json_error_meta(
            StatusCode::INTERNAL_SERVER_ERROR,
            surface,
            serde_json::json!({}),
            serde_json::json!({ "data_root": data_root.display().to_string() }),
            err.to_string(),
        ))
    })
}

fn game_scoring_page(view: &GameScoringReportView) -> ScoringReportPage {
    let source_loaded = source_loaded(&view.context.source_state);
    ScoringReportPage {
        title: format!("Game {} Scoring Report", view.game_id),
        subtitle: "Official NHL play-by-play scoring-event summary".to_string(),
        back_href: format!("/game/{}", view.game_id),
        back_label: "game detail".to_string(),
        api_href: format!("/api/v1/game/{}/scoring", view.game_id),
        source_loaded,
        source_label: source_label(source_loaded),
        summary: summary_row(view.summary),
        team_summaries: split_rows(&view.team_summaries),
        period_summaries: split_rows(&view.period_summaries),
        situation_summaries: split_rows(&view.situation_summaries),
        top_shooters: shooter_rows(&view.top_shooters),
        events: event_rows(&view.events),
        load_form: None,
    }
}

fn team_scoring_page(
    view: &TeamScoringProfileView,
    season: Season,
    season_type: SeasonType,
) -> ScoringReportPage {
    let source_loaded = source_loaded(&view.context.source_state);
    ScoringReportPage {
        title: format!("{} Scoring Profile", view.team),
        subtitle: format!(
            "{} · {} · official NHL play-by-play",
            pretty_season_label(season.0),
            season_type.label()
        ),
        back_href: format!("/team/{}", view.team),
        back_label: "team page".to_string(),
        api_href: format!("/api/v1/team/{}/scoring", view.team),
        source_loaded,
        source_label: source_label(source_loaded),
        summary: summary_row(view.summary),
        team_summaries: Vec::new(),
        period_summaries: split_rows(&view.period_summaries),
        situation_summaries: split_rows(&view.situation_summaries),
        top_shooters: shooter_rows(&view.top_shooters),
        events: event_rows(&view.events),
        load_form: Some(ScoringLoadForm {
            season: season.0.to_string(),
            season_type: season_type.label().to_string(),
            teams: view.team.clone(),
            return_to: format!("/team/{}/scoring", view.team),
        }),
    }
}

fn summary_row(summary: ScoringEventSummary) -> ScoringSummaryTemplateRow {
    ScoringSummaryTemplateRow {
        goals: summary.goals,
        shots_on_goal: summary.shots_on_goal,
        missed_shots: summary.missed_shots,
        blocked_shots: summary.blocked_shots,
        shot_attempts: summary.shot_attempts,
        unblocked_attempts: summary.unblocked_attempts,
        shot_pct: if summary.shots_on_goal == 0 {
            "-".to_string()
        } else {
            format!(
                "{:.1}%",
                (summary.goals as f64 / summary.shots_on_goal as f64) * 100.0
            )
        },
    }
}

fn split_rows(rows: &[ScoringSplitSummary]) -> Vec<ScoringSplitTemplateRow> {
    rows.iter()
        .map(|row| ScoringSplitTemplateRow {
            label: row.label.clone(),
            summary: summary_row(row.summary),
        })
        .collect()
}

fn shooter_rows(rows: &[ScoringShooterSummary]) -> Vec<ScoringShooterTemplateRow> {
    rows.iter()
        .map(|row| ScoringShooterTemplateRow {
            player_id: row.player_id,
            summary: summary_row(row.summary),
        })
        .collect()
}

fn event_rows(events: &[ScoringEventInput]) -> Vec<ScoringEventTemplateRow> {
    events.iter().map(event_row).collect()
}

fn event_row(event: &ScoringEventInput) -> ScoringEventTemplateRow {
    ScoringEventTemplateRow {
        period: if event.period_type == "REG" {
            format!("P{}", event.period)
        } else {
            format!("{}{}", event.period_type, event.period)
        },
        time: event.time_in_period.clone(),
        kind: kind_label(event.kind).to_string(),
        team: event
            .event_owner_team_abbrev
            .clone()
            .unwrap_or_else(|| "-".to_string()),
        shooter: event
            .shooting_player_id
            .or(event.scoring_player_id)
            .map(|id| id.to_string())
            .unwrap_or_else(|| "-".to_string()),
        goalie: event
            .goalie_in_net_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "-".to_string()),
        shot_type: event.shot_type.clone().unwrap_or_else(|| "-".to_string()),
        location: match (event.location.x_coord, event.location.y_coord) {
            (Some(x), Some(y)) => format!("{x}, {y}"),
            _ => "-".to_string(),
        },
    }
}

fn kind_label(kind: ShotEventKind) -> &'static str {
    match kind {
        ShotEventKind::Goal => "Goal",
        ShotEventKind::ShotOnGoal => "Shot on goal",
        ShotEventKind::MissedShot => "Missed shot",
        ShotEventKind::BlockedShot => "Blocked shot",
    }
}

fn source_loaded(states: &[icelines_core::SourceState]) -> bool {
    states.iter().any(|state| {
        state.source == icelines_core::SourceKind::PlayByPlay
            && state.state == Completeness::Complete
    })
}

fn source_label(loaded: bool) -> String {
    if loaded {
        "play-by-play loaded".to_string()
    } else {
        "play-by-play not loaded".to_string()
    }
}

fn parse_team(abbrev_raw: &str) -> Result<TeamAbbr, Box<Response>> {
    let abbrev_upper = abbrev_raw.to_ascii_uppercase();
    TeamAbbr::parse(&abbrev_upper).map_err(|e| {
        let message = format!("'{abbrev_upper}' is not a recognized NHL team abbrev: {e}");
        Box::new(
            (
                StatusCode::NOT_FOUND,
                Html(format!(
                    "<!doctype html><html><body><h1>Unknown team</h1>\
                 <p>{message}</p>\
                 <p><a href=\"/leaders\">back to leaders</a></p>\
                </body></html>"
                )),
            )
                .into_response(),
        )
    })
}

fn pretty_season_label(season: u32) -> String {
    let start = season / 10_000;
    let end = season % 10_000;
    if start > 0 && end > 0 {
        format!("{start}-{end_short:02}", end_short = end % 100)
    } else {
        season.to_string()
    }
}

fn data_root() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .map(|home| home.join(".icelines").join("data"))
}
