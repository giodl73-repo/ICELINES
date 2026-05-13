use crate::state::WebState;
use crate::templates::{ScoreRow, ScoresDay, ScoresTemplate};
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use chrono::{Duration, NaiveDate, Utc};
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::{ScheduledGameInput, ScoresDayView, ScoresView, ViewContext, ViewWindow};
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct ScoresQuery {
    /// YYYY-MM-DD. The NHL API returns a 7-day window starting
    /// from this date. Default: today.
    #[serde(default)]
    pub date: Option<String>,
    /// Phase Foster +9 — `day` (default) | `week` | `month`.
    /// Widens the rendered window around `date`. The default
    /// `day` collapses to the existing single-date behavior;
    /// `week` and `month` use Timeframe::range to bound the
    /// `by_date` group. Spec §"Web URL convention".
    #[serde(default)]
    pub range: Option<String>,
}

pub(super) struct ScoresResult {
    pub(super) active_label: String,
    pub(super) active_date: String,
    pub(super) prev_date: String,
    pub(super) next_date: String,
    pub(super) today_date: String,
    pub(super) range: String,
    pub(super) days: Vec<ScoresDay>,
    pub(super) total_games: usize,
    pub(super) fetch_error: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct ScoresMeta {
    active_date: String,
    today_date: String,
    range: String,
    total_games: usize,
    source_error: Option<String>,
}

fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

/// Phase Foster +9 — parse `?range=` into a Timeframe.
/// Defaults to Day (matches the spec convention "range=day
/// is implicit"). Unknown values fall back to Day for safety.
pub(crate) fn parse_range_to_timeframe(s: Option<&str>) -> icelines_core::timeframe::Timeframe {
    use icelines_core::timeframe::Timeframe;
    match s.map(str::trim).filter(|s| !s.is_empty()) {
        None | Some("day") => Timeframe::Day,
        Some("week") => Timeframe::Week,
        Some("month") => Timeframe::Month,
        Some("season") => Timeframe::Season,
        Some(_) => Timeframe::Day,
    }
}

pub(super) async fn build_scores_result(state: &WebState, q: &ScoresQuery) -> ScoresResult {
    let (active_label, active_season, active_season_type) = {
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

    let today = Utc::now().date_naive();
    let active_date = q.date.as_deref().and_then(parse_date).unwrap_or(today);
    // Phase Foster +9 — `?range=` resolves the timeframe.
    // Day narrows the rendered grouping to the anchor date;
    // Week / Month surface the natural 7-day gameWeek
    // window the API already returns.
    let timeframe = parse_range_to_timeframe(q.range.as_deref());
    let prev_date = active_date - Duration::days(7);
    let next_date = active_date + Duration::days(7);

    let client = super::nhl_client();
    let fetch_result = if q.date.is_some() {
        client
            .fetch_schedule_for_date(&active_date.format("%Y-%m-%d").to_string())
            .await
    } else {
        client.fetch_today_schedule().await
    };

    let context = ViewContext::new(ViewWindow::new(active_season, active_season_type));
    let (view, fetch_error) = match fetch_result {
        Ok(games) => (
            ScoresView::from_games(
                context,
                active_date,
                today,
                timeframe,
                games.into_iter().map(scheduled_game_input).collect(),
            ),
            None,
        ),
        Err(e) => (
            ScoresView::from_games(context, active_date, today, timeframe, Vec::new()),
            Some(e.to_string()),
        ),
    };

    ScoresResult {
        active_label,
        active_date: view.active_date,
        prev_date: prev_date.format("%Y-%m-%d").to_string(),
        next_date: next_date.format("%Y-%m-%d").to_string(),
        today_date: view.today_date,
        range: view.range,
        days: view.days.iter().map(scores_day_from_view).collect(),
        total_games: view.total_games,
        fetch_error,
    }
}

fn scheduled_game_input(game: icelines_fetch::nhl_api::ScheduledGame) -> ScheduledGameInput {
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

fn scores_day_from_view(day: &ScoresDayView) -> ScoresDay {
    ScoresDay {
        date: day.date.clone(),
        date_pretty: day.date_pretty.clone(),
        rows: day
            .rows
            .iter()
            .map(|row| ScoreRow {
                away_abbrev: row.away_abbrev.clone(),
                away_name: row.away_name.clone(),
                home_abbrev: row.home_abbrev.clone(),
                home_name: row.home_name.clone(),
                away_score_str: row.away_score_str.clone(),
                home_score_str: row.home_score_str.clone(),
                state_label: row.state_label.clone(),
                state_class: row.state_class.clone(),
                start_time_label: row.start_time_label.clone(),
                is_playoff: row.is_playoff,
                series_context: row.series_context.clone(),
            })
            .collect(),
    }
}

pub async fn get_scores(State(state): State<WebState>, Query(q): Query<ScoresQuery>) -> Response {
    let result = build_scores_result(&state, &q).await;
    let tmpl = ScoresTemplate {
        active_label: result.active_label,
        active_date: result.active_date,
        prev_date: result.prev_date,
        next_date: result.next_date,
        today_date: result.today_date,
        days: result.days,
        total_games: result.total_games,
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

pub async fn get_scores_json(
    State(state): State<WebState>,
    Query(q): Query<ScoresQuery>,
) -> Response {
    let result = build_scores_result(&state, &q).await;
    crate::api::json_data_meta(
        "scores",
        result.days,
        ScoresMeta {
            active_date: result.active_date,
            today_date: result.today_date,
            range: result.range,
            total_games: result.total_games,
            source_error: result.fetch_error,
        },
    )
}
