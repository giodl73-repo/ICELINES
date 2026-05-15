use crate::state::WebState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use chrono::{NaiveDate, Utc};
use icelines_core::identity::PlayerId;
use icelines_core::model::{Season, TeamAbbr};
use icelines_core::season_stats::SeasonType;
use icelines_core::{
    Completeness, GameScoringReportView, PlayerScoringPaceMetric, PlayerScoringPaceRow,
    PlayerScoringPaceSampleStatus, PlayerScoringPaceView, ScoringEventInput, ScoringEventSummary,
    ScoringShooterSummary, ScoringSplitSummary, ShotEventKind, SourceKind, SourceState,
    TeamScoringOutlookMetric, TeamScoringOutlookRow, TeamScoringOutlookSourceStatus,
    TeamScoringOutlookView, TeamScoringProfileView, ViewContext, ViewWindow,
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

#[derive(Debug, serde::Deserialize)]
pub struct TonightIntelQuery {
    #[serde(default)]
    date: Option<String>,
}

#[derive(Template)]
#[template(path = "tonight_intel.html")]
struct TonightIntelTemplate {
    active_label: String,
    page: TonightIntelPage,
}

#[derive(Debug, Clone)]
struct TonightIntelPage {
    date: String,
    api_href: String,
    source_loaded: bool,
    source_label: String,
    games_loaded: usize,
    events_loaded: usize,
    summary: ScoringSummaryTemplateRow,
    favorite_teams: Vec<TonightTeamIntelRow>,
    favorite_players: Vec<TonightPlayerIntelRow>,
    load_form: ScoringLoadForm,
}

#[derive(Debug, Clone)]
struct TonightTeamIntelRow {
    team: String,
    summary: ScoringSummaryTemplateRow,
}

#[derive(Debug, Clone)]
struct TonightPlayerIntelRow {
    label: String,
    player_id: String,
    summary: ScoringSummaryTemplateRow,
}

#[derive(Template)]
#[template(path = "scoring_outlook.html")]
struct ScoringOutlookTemplate {
    active_label: String,
    page: ScoringOutlookPage,
}

#[derive(Debug, Clone)]
struct ScoringOutlookPage {
    title: String,
    subtitle: String,
    back_href: String,
    back_label: String,
    api_href: String,
    source_label: String,
    rows: Vec<ScoringOutlookTemplateRow>,
    has_recent_form: bool,
    recent_label: String,
    recent_games_loaded: u32,
    recent_goals_for: u32,
    recent_goals_against: u32,
    recent_goal_differential: String,
    recent_goals_for_per_game: String,
    recent_goals_against_per_game: String,
}

#[derive(Debug, Clone)]
struct ScoringOutlookTemplateRow {
    label: String,
    current_total: u32,
    games_played: u32,
    per_game: String,
    pace_82: String,
    projected_finish: String,
    status_label: String,
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

pub async fn get_player_scoring(State(state): State<WebState>, Path(id): Path<u32>) -> Response {
    match build_player_scoring_view(&state, id).await {
        Ok((active_label, season, season_type, view)) => {
            let page = player_scoring_page(&view, season, season_type);
            render_scoring_template(active_label, page)
        }
        Err(response) => *response,
    }
}

pub async fn get_player_outlook(State(state): State<WebState>, Path(id): Path<u32>) -> Response {
    match build_player_outlook_view(&state, id).await {
        Ok((active_label, view)) => {
            let page = player_outlook_page(&view);
            render_outlook_template(active_label, page)
        }
        Err(response) => *response,
    }
}

pub async fn get_player_outlook_json(
    State(state): State<WebState>,
    Path(id): Path<u32>,
) -> Response {
    match build_player_outlook_view(&state, id).await {
        Ok((_active_label, view)) => {
            let meta = serde_json::json!({
                "player_id": view.player_id,
                "season": view.context.window.season.0.to_string(),
                "season_type": view.context.window.season_type.label(),
                "source_state": view.context.source_state,
            });
            crate::api::json_data_meta("player-outlook", view, meta)
        }
        Err(response) => *response,
    }
}

pub async fn get_player_scoring_json(
    State(state): State<WebState>,
    Path(id): Path<u32>,
) -> Response {
    match build_player_scoring_view(&state, id).await {
        Ok((_active_label, _season, _season_type, view)) => {
            let meta = serde_json::json!({
                "player_id": view.player_id,
                "season": view.context.window.season.0.to_string(),
                "season_type": view.context.window.season_type.label(),
                "source_state": view.context.source_state,
            });
            crate::api::json_data_meta("player-scoring", view, meta)
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

pub async fn get_team_outlook(
    State(state): State<WebState>,
    Path(abbrev_raw): Path<String>,
) -> Response {
    match build_team_outlook_view(&state, &abbrev_raw).await {
        Ok((active_label, view)) => {
            let page = team_outlook_page(&view);
            render_outlook_template(active_label, page)
        }
        Err(response) => *response,
    }
}

pub async fn get_team_outlook_json(
    State(state): State<WebState>,
    Path(abbrev_raw): Path<String>,
) -> Response {
    match build_team_outlook_view(&state, &abbrev_raw).await {
        Ok((_active_label, view)) => {
            let meta = serde_json::json!({
                "team_abbrev": view.team,
                "season": view.context.window.season.0.to_string(),
                "season_type": view.context.window.season_type.label(),
                "source_state": view.context.source_state,
            });
            crate::api::json_data_meta("team-outlook", view, meta)
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

pub async fn get_tonight_intel(
    State(state): State<WebState>,
    Query(q): Query<TonightIntelQuery>,
) -> Response {
    match build_tonight_intel_view(&state, &q).await {
        Ok((active_label, view)) => {
            let page = tonight_intel_page(&view);
            match (TonightIntelTemplate { active_label, page }).render() {
                Ok(html) => Html(html).into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Html(format!("template render failed: {e}")),
                )
                    .into_response(),
            }
        }
        Err(response) => *response,
    }
}

pub async fn get_tonight_intel_json(
    State(state): State<WebState>,
    Query(q): Query<TonightIntelQuery>,
) -> Response {
    match build_tonight_intel_view(&state, &q).await {
        Ok((_active_label, view)) => {
            let meta = serde_json::json!({
                "date": view.date,
                "source_state": view.context.source_state,
                "favorite_team_count": view.favorite_teams.len(),
                "favorite_player_count": view.favorite_players.len(),
            });
            crate::api::json_data_meta("tonight-intel", view, meta)
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

fn render_outlook_template(active_label: String, page: ScoringOutlookPage) -> Response {
    match (ScoringOutlookTemplate { active_label, page }).render() {
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

async fn build_team_outlook_view(
    state: &WebState,
    abbrev_raw: &str,
) -> Result<(String, TeamScoringOutlookView), Box<Response>> {
    let team = parse_team(abbrev_raw)?;
    let (active_label, season, season_type) = active_window(state).await?;
    let store = open_data_store("team-outlook")?;
    let context = ViewContext::new(ViewWindow::new(season, season_type));
    let view = icelines_fetch::scoring_outlook_provider::load_team_scoring_outlook(
        &store, context, &team.0,
    );
    Ok((active_label, view))
}

async fn build_player_scoring_view(
    state: &WebState,
    player_id: u32,
) -> Result<
    (
        String,
        Season,
        SeasonType,
        icelines_core::PlayerScoringProfileView,
    ),
    Box<Response>,
> {
    let (active_label, season, season_type) = active_window(state).await?;
    let store = open_data_store("player-scoring")?;
    let context = ViewContext::new(ViewWindow::new(season, season_type));
    let player_name = icelines_fetch::stats_loader::find_player_candidate_by_id(player_id)
        .map(|candidate| candidate.full_name)
        .unwrap_or_else(|| player_id.to_string());
    let view = icelines_fetch::scoring_provider::load_player_scoring_profile(
        &store,
        context,
        player_id,
        player_name,
    );
    Ok((active_label, season, season_type, view))
}

async fn build_player_outlook_view(
    state: &WebState,
    player_id: u32,
) -> Result<(String, PlayerScoringPaceView), Box<Response>> {
    let (active_label, season, season_type) = active_window(state).await?;
    let store = open_data_store("player-outlook")?;
    let view = {
        let repo = state.repo.read().await;
        let player = repo
            .view(PlayerId(player_id), season, season_type)
            .ok_or_else(|| {
                Box::new(
                    (
                        StatusCode::NOT_FOUND,
                        Html(format!(
                            "<!doctype html><html><body><h1>Player not found</h1>\
                         <p>No player with NHL id {player_id} in the active repository.</p>\
                         <p><a href=\"/leaders\">back to leaders</a></p></body></html>"
                        )),
                    )
                        .into_response(),
                )
            })?;
        let (remaining_games, schedule_status) =
            icelines_fetch::scoring_outlook_provider::schedule_remaining_for_team(
                &store,
                season,
                player.team_display(),
            );
        let mut context = ViewContext::new(ViewWindow::new(season, season_type));
        context
            .source_state
            .push(schedule_source_state(schedule_status));
        PlayerScoringPaceView::from_player(context, &player, remaining_games)
    };
    Ok((active_label, view))
}

async fn build_tonight_intel_view(
    state: &WebState,
    q: &TonightIntelQuery,
) -> Result<(String, icelines_core::TonightScoringIntelView), Box<Response>> {
    let (active_label, season, season_type) = active_window(state).await?;
    let date = q
        .date
        .as_deref()
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .unwrap_or_else(|| Utc::now().date_naive())
        .format("%Y-%m-%d")
        .to_string();
    let favorites = crate::handlers::favorites_data::read_group_members("Favorites");
    let favorite_teams: Vec<String> = favorites
        .iter()
        .filter(|(kind, _key)| kind == "team")
        .map(|(_kind, key)| key.to_ascii_uppercase())
        .collect();
    let favorite_players: Vec<(String, Option<u32>)> = favorites
        .iter()
        .filter(|(kind, _key)| kind == "player")
        .map(|(_kind, key)| (key.clone(), resolve_favorite_player_id(key)))
        .collect();
    let store = open_data_store("tonight-intel")?;
    let context = ViewContext::new(ViewWindow::new(season, season_type));
    let view = icelines_fetch::scoring_provider::load_tonight_scoring_intel(
        &store,
        context,
        &date,
        &favorite_teams,
        &favorite_players,
    );
    Ok((active_label, view))
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

fn player_scoring_page(
    view: &icelines_core::PlayerScoringProfileView,
    season: Season,
    season_type: SeasonType,
) -> ScoringReportPage {
    let source_loaded = source_loaded(&view.context.source_state);
    ScoringReportPage {
        title: format!("{} Scoring Profile", view.player_name),
        subtitle: format!(
            "{} · {} · official NHL play-by-play",
            pretty_season_label(season.0),
            season_type.label()
        ),
        back_href: format!("/player/{}", view.player_id),
        back_label: "player card".to_string(),
        api_href: format!("/api/v1/player/{}/scoring", view.player_id),
        source_loaded,
        source_label: source_label(source_loaded),
        summary: summary_row(view.summary),
        team_summaries: Vec::new(),
        period_summaries: split_rows(&view.period_summaries),
        situation_summaries: split_rows(&view.situation_summaries),
        top_shooters: Vec::new(),
        events: event_rows(&view.events),
        load_form: None,
    }
}

fn player_outlook_page(view: &PlayerScoringPaceView) -> ScoringOutlookPage {
    ScoringOutlookPage {
        title: format!("{} Scoring Outlook", view.player_name),
        subtitle: format!(
            "{} · {} · descriptive 82-game pace",
            pretty_season_label(view.context.window.season.0),
            view.context.window.season_type.label()
        ),
        back_href: format!("/player/{}", view.player_id),
        back_label: "player card".to_string(),
        api_href: format!("/api/v1/player/{}/outlook", view.player_id),
        source_label: player_outlook_source_label(view),
        rows: view.rows.iter().map(player_outlook_row).collect(),
        has_recent_form: false,
        recent_label: String::new(),
        recent_games_loaded: 0,
        recent_goals_for: 0,
        recent_goals_against: 0,
        recent_goal_differential: String::new(),
        recent_goals_for_per_game: String::new(),
        recent_goals_against_per_game: String::new(),
    }
}

fn team_outlook_page(view: &TeamScoringOutlookView) -> ScoringOutlookPage {
    ScoringOutlookPage {
        title: format!("{} Scoring Outlook", view.team),
        subtitle: format!(
            "{} · {} · goals for / against pace",
            pretty_season_label(view.context.window.season.0),
            view.context.window.season_type.label()
        ),
        back_href: format!("/team/{}", view.team),
        back_label: "team page".to_string(),
        api_href: format!("/api/v1/team/{}/outlook", view.team),
        source_label: team_outlook_source_label(view.source_status),
        rows: view.rows.iter().map(team_outlook_row).collect(),
        has_recent_form: true,
        recent_label: view.recent_form.label.clone(),
        recent_games_loaded: view.recent_form.games_loaded,
        recent_goals_for: view.recent_form.goals_for,
        recent_goals_against: view.recent_form.goals_against,
        recent_goal_differential: signed_i32(view.recent_form.goal_differential),
        recent_goals_for_per_game: format_opt_one_decimal(view.recent_form.goals_for_per_game),
        recent_goals_against_per_game: format_opt_one_decimal(
            view.recent_form.goals_against_per_game,
        ),
    }
}

fn tonight_intel_page(view: &icelines_core::TonightScoringIntelView) -> TonightIntelPage {
    let source_loaded = source_loaded(&view.context.source_state);
    TonightIntelPage {
        date: view.date.clone(),
        api_href: format!("/api/v1/tonight/intel?date={}", view.date),
        source_loaded,
        source_label: source_label(source_loaded),
        games_loaded: view.games_loaded,
        events_loaded: view.events_loaded,
        summary: summary_row(view.summary),
        favorite_teams: view
            .favorite_teams
            .iter()
            .map(|row| TonightTeamIntelRow {
                team: row.team.clone(),
                summary: summary_row(row.summary),
            })
            .collect(),
        favorite_players: view
            .favorite_players
            .iter()
            .map(|row| TonightPlayerIntelRow {
                label: row.player_key.clone(),
                player_id: row
                    .player_id
                    .map(|pid| pid.to_string())
                    .unwrap_or_else(|| "unresolved".to_string()),
                summary: summary_row(row.summary),
            })
            .collect(),
        load_form: ScoringLoadForm {
            season: view.context.window.season.0.to_string(),
            season_type: view.context.window.season_type.label().to_string(),
            teams: view
                .favorite_teams
                .iter()
                .map(|row| row.team.as_str())
                .collect::<Vec<_>>()
                .join(","),
            return_to: format!("/tonight/intel?date={}", view.date),
        },
    }
}

fn player_outlook_row(row: &PlayerScoringPaceRow) -> ScoringOutlookTemplateRow {
    let status_label = match row.sample_status {
        PlayerScoringPaceSampleStatus::ZeroGames => "below sample floor",
        PlayerScoringPaceSampleStatus::BelowThreshold => "below sample floor",
        PlayerScoringPaceSampleStatus::Eligible => match row.metric {
            PlayerScoringPaceMetric::Goals => "on pace",
            PlayerScoringPaceMetric::Points | PlayerScoringPaceMetric::Shots => "tracking toward",
        },
    };
    ScoringOutlookTemplateRow {
        label: row.label.clone(),
        current_total: row.current_total,
        games_played: row.games_played,
        per_game: format_opt_two_decimal(row.per_game),
        pace_82: format_opt_one_decimal(row.pace_82),
        projected_finish: format_opt_one_decimal(row.projected_finish),
        status_label: status_label.to_string(),
    }
}

fn team_outlook_row(row: &TeamScoringOutlookRow) -> ScoringOutlookTemplateRow {
    let status_label = match row.source_status {
        TeamScoringOutlookSourceStatus::MissingSource => "source missing",
        TeamScoringOutlookSourceStatus::PartialSource => "partial source",
        TeamScoringOutlookSourceStatus::Loaded => match row.metric {
            TeamScoringOutlookMetric::GoalsFor => "tracking toward",
            TeamScoringOutlookMetric::GoalsAgainst => "recent pressure",
        },
    };
    ScoringOutlookTemplateRow {
        label: row.label.clone(),
        current_total: row.current_total,
        games_played: row.games_played,
        per_game: format_opt_two_decimal(row.per_game),
        pace_82: format_opt_one_decimal(row.pace_82),
        projected_finish: format_opt_one_decimal(row.projected_finish),
        status_label: status_label.to_string(),
    }
}

fn player_outlook_source_label(view: &PlayerScoringPaceView) -> String {
    let schedule_status = view
        .context
        .source_state
        .iter()
        .find(|state| state.source == SourceKind::Schedule)
        .map(|state| state.state)
        .unwrap_or(Completeness::Unavailable);
    match schedule_status {
        Completeness::Complete => "season stats loaded · schedule loaded".to_string(),
        Completeness::Partial => "season stats loaded · partial source".to_string(),
        Completeness::Unavailable => {
            "season stats loaded · schedule missing, projected finish unavailable".to_string()
        }
        Completeness::Stale => "season stats loaded · schedule stale".to_string(),
    }
}

fn team_outlook_source_label(status: TeamScoringOutlookSourceStatus) -> String {
    match status {
        TeamScoringOutlookSourceStatus::Loaded => "schedule loaded".to_string(),
        TeamScoringOutlookSourceStatus::PartialSource => "partial source".to_string(),
        TeamScoringOutlookSourceStatus::MissingSource => {
            "schedule missing, projected finish unavailable".to_string()
        }
    }
}

fn format_opt_one_decimal(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.1}"))
        .unwrap_or_else(|| "-".to_string())
}

fn format_opt_two_decimal(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.2}"))
        .unwrap_or_else(|| "-".to_string())
}

fn signed_i32(value: i32) -> String {
    format!("{value:+}")
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

fn schedule_source_state(status: TeamScoringOutlookSourceStatus) -> SourceState {
    match status {
        TeamScoringOutlookSourceStatus::Loaded => SourceState::complete(SourceKind::Schedule),
        TeamScoringOutlookSourceStatus::PartialSource => SourceState {
            source: SourceKind::Schedule,
            state: Completeness::Partial,
            provenance: None,
            fetched_at: None,
            stale_reason: None,
            message: Some("loaded schedule/score window is partial".to_string()),
        },
        TeamScoringOutlookSourceStatus::MissingSource => SourceState::missing(SourceKind::Schedule),
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
    if let Some(root) = std::env::var_os("ICELINES_DATA_ROOT") {
        return Some(std::path::PathBuf::from(root));
    }
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .map(|home| home.join(".icelines").join("data"))
}

fn resolve_favorite_player_id(key: &str) -> Option<u32> {
    let key = key.trim();
    if let Ok(pid) = key.parse::<u32>() {
        return Some(pid);
    }
    icelines_fetch::stats_loader::resolve_player_id_by_name(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_core::model::Season;
    use icelines_core::season_stats::SeasonType;

    #[test]
    fn l0_player_outlook_page_preserves_sample_floor_and_json_source_state() {
        let mut context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        context.source_state.push(schedule_source_state(
            TeamScoringOutlookSourceStatus::MissingSource,
        ));
        let view = PlayerScoringPaceView {
            context,
            player_id: 8478402,
            player_name: "Connor McDavid".to_string(),
            team: "EDM".to_string(),
            position: "C".to_string(),
            games_played: 9,
            sample_status: PlayerScoringPaceSampleStatus::BelowThreshold,
            min_games: 10,
            pace_games: 82,
            remaining_games: None,
            shot_pct: None,
            rows: vec![PlayerScoringPaceRow::new(
                PlayerScoringPaceMetric::Goals,
                4,
                9,
                None,
            )],
        };

        let page = player_outlook_page(&view);
        let json = serde_json::to_value(&view).expect("view json");

        assert_eq!(page.rows[0].status_label, "below sample floor");
        assert_eq!(page.rows[0].pace_82, "-");
        assert!(page.source_label.contains("schedule missing"));
        assert_eq!(
            json["context"]["source_state"][0]["source"],
            serde_json::json!("schedule")
        );
    }

    #[test]
    fn l0_team_outlook_page_preserves_partial_source_and_recent_pressure() {
        let view = TeamScoringOutlookView::from_schedule_games(
            ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular)),
            "EDM",
            true,
            true,
            Vec::new(),
            None,
        );

        let page = team_outlook_page(&view);
        let json = serde_json::to_value(&view).expect("view json");

        assert_eq!(page.source_label, "partial source");
        assert_eq!(page.rows[0].status_label, "partial source");
        assert!(page.has_recent_form);
        assert_eq!(page.recent_label, "recent pressure - last 10 games");
        assert_eq!(
            json["context"]["source_state"][0]["state"],
            serde_json::json!("partial")
        );
    }
}
