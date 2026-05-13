use crate::state::WebState;
use crate::templates::{ScheduleRow, ScheduleTemplate, TeamChip};
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::{
    ScheduleGameRow, ScheduleView, ScheduledGameInput, TeamChipView, ViewContext, ViewWindow,
};
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct ScheduleQuery {
    #[serde(default)]
    pub team: Option<String>,
    /// Phase Foster.1 - anchor date `YYYY-MM-DD` for the date-windowed slate.
    /// Team-season mode takes precedence when `?team=` is present.
    #[serde(default)]
    pub date: Option<String>,
}

pub(super) struct ScheduleResult {
    pub(super) active_label: String,
    pub(super) season_pretty: String,
    pub(super) active_team: String,
    pub(super) active_date: Option<String>,
    pub(super) team_chips: Vec<TeamChip>,
    pub(super) rows: Vec<ScheduleRow>,
    pub(super) total: usize,
    pub(super) fetch_error: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct ScheduleMeta {
    season: String,
    active_team: String,
    active_date: Option<String>,
    total: usize,
    team_chips: Vec<TeamChip>,
    source_error: Option<String>,
}

/// 32 active NHL franchises. Used to populate the team picker chip strip.
const ALL_TEAM_ABBREVS: &[&str] = &[
    "ANA", "BOS", "BUF", "CAR", "CBJ", "CGY", "CHI", "COL", "DAL", "DET", "EDM", "FLA", "LAK",
    "MIN", "MTL", "NJD", "NSH", "NYI", "NYR", "OTT", "PHI", "PIT", "SEA", "SJS", "STL", "TBL",
    "TOR", "UTA", "VAN", "VGK", "WPG", "WSH",
];

pub async fn get_schedule(
    State(state): State<WebState>,
    Query(q): Query<ScheduleQuery>,
) -> Response {
    let result = build_schedule_result(&state, &q).await;
    let tmpl = ScheduleTemplate {
        active_label: result.active_label,
        season_pretty: result.season_pretty,
        active_team: result.active_team,
        team_chips: result.team_chips,
        rows: result.rows,
        total: result.total,
        fetch_error: result.fetch_error,
    };
    match tmpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("template render failed: {e}")),
        )
            .into_response(),
    }
}

pub async fn get_schedule_json(
    State(state): State<WebState>,
    Query(q): Query<ScheduleQuery>,
) -> Response {
    let result = build_schedule_result(&state, &q).await;
    crate::api::json_data_meta(
        "schedule",
        result.rows,
        ScheduleMeta {
            season: result.season_pretty,
            active_team: result.active_team,
            active_date: result.active_date,
            total: result.total,
            team_chips: result.team_chips,
            source_error: result.fetch_error,
        },
    )
}

pub(super) async fn build_schedule_result(state: &WebState, q: &ScheduleQuery) -> ScheduleResult {
    let (active_label, season_str, season, season_type) = {
        let cfg = state.config.read().await;
        (
            cfg.active_label.clone(),
            cfg.active_season.clone(),
            cfg.active_season
                .parse::<u32>()
                .map(Season)
                .unwrap_or(Season(0)),
            SeasonType::parse_lossy(&cfg.active_season_type),
        )
    };

    let team_upper = q
        .team
        .as_deref()
        .map(|team| team.trim().to_ascii_uppercase())
        .filter(|team| !team.is_empty())
        .unwrap_or_default();
    let active_date = if team_upper.is_empty() {
        q.date
            .as_deref()
            .map(str::trim)
            .filter(|date| !date.is_empty())
            .map(str::to_owned)
    } else {
        None
    };

    let (games, fetch_error) = if team_upper.is_empty() {
        match active_date.as_deref() {
            Some(date) => match super::nhl_client().fetch_schedule_for_date(date).await {
                Ok(games) => (games.into_iter().map(scheduled_game_input).collect(), None),
                Err(e) => (Vec::new(), Some(e.to_string())),
            },
            None => (Vec::new(), None),
        }
    } else {
        match super::nhl_client()
            .fetch_team_season_schedule(&team_upper, &season_str)
            .await
        {
            Ok(games) => (games.into_iter().map(scheduled_game_input).collect(), None),
            Err(e) => (Vec::new(), Some(e.to_string())),
        }
    };

    let view = ScheduleView::from_games(
        ViewContext::new(ViewWindow::new(season, season_type)),
        season_str,
        team_upper,
        active_date,
        ALL_TEAM_ABBREVS,
        games,
    );

    ScheduleResult {
        active_label,
        season_pretty: view.season_pretty,
        active_team: view.active_team,
        active_date: view.active_date,
        team_chips: view.team_chips.iter().map(team_chip_from_view).collect(),
        rows: view.rows.iter().map(schedule_row_from_view).collect(),
        total: view.total,
        fetch_error,
    }
}

pub(crate) fn scheduled_game_input(
    game: icelines_fetch::nhl_api::ScheduledGame,
) -> ScheduledGameInput {
    ScheduledGameInput {
        game_id: game.game_id,
        date: game.date,
        game_type: game.game_type,
        away_abbrev: game.away_abbrev,
        away_name: game.away_name,
        home_abbrev: game.home_abbrev,
        home_name: game.home_name,
        start_time_utc: game.start_time_utc,
        away_score: game.away_score,
        home_score: game.home_score,
        game_state: game.game_state,
        last_period: game.last_period,
        series_game: game.series_game,
        away_wins: game.away_wins,
        home_wins: game.home_wins,
    }
}

fn team_chip_from_view(chip: &TeamChipView) -> TeamChip {
    TeamChip {
        abbrev: chip.abbrev.clone(),
        is_active: chip.is_active,
    }
}

fn schedule_row_from_view(row: &ScheduleGameRow) -> ScheduleRow {
    ScheduleRow {
        date: row.date.clone(),
        away_abbrev: row.away_abbrev.clone(),
        home_abbrev: row.home_abbrev.clone(),
        away_score_str: row.away_score_str.clone(),
        home_score_str: row.home_score_str.clone(),
        state_label: row.state_label.clone(),
        home_or_away: row.home_or_away.clone(),
        opponent_abbrev: row.opponent_abbrev.clone(),
        is_playoff: row.is_playoff,
    }
}
