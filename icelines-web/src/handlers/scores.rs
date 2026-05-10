use crate::state::WebState;
use crate::templates::{ScoreRow, ScoresDay, ScoresTemplate};
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use chrono::{Datelike, Duration, NaiveDate, Utc, Weekday};
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

struct ScoresResult {
    active_label: String,
    active_date: String,
    prev_date: String,
    next_date: String,
    today_date: String,
    range: String,
    days: Vec<ScoresDay>,
    total_games: usize,
    fetch_error: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct ScoresMeta {
    active_date: String,
    today_date: String,
    range: String,
    total_games: usize,
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

fn pretty_day(d: NaiveDate) -> String {
    let weekday = match d.weekday() {
        Weekday::Mon => "Mon",
        Weekday::Tue => "Tue",
        Weekday::Wed => "Wed",
        Weekday::Thu => "Thu",
        Weekday::Fri => "Fri",
        Weekday::Sat => "Sat",
        Weekday::Sun => "Sun",
    };
    let month = match d.month() {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "?",
    };
    format!("{}, {} {}, {}", weekday, month, d.day(), d.year())
}

fn state_to_class_label(state: Option<&str>, last_period: Option<&str>) -> (String, String) {
    match state.unwrap_or("") {
        "FINAL" | "OFF" => {
            let label = match last_period.unwrap_or("REG") {
                "OT" => "FINAL/OT".to_owned(),
                "SO" => "FINAL/SO".to_owned(),
                _ => "FINAL".to_owned(),
            };
            ("final".to_owned(), label)
        }
        "LIVE" | "CRIT" => ("live".to_owned(), "LIVE".to_owned()),
        "PRE" => ("future".to_owned(), "Pre-game".to_owned()),
        "FUT" | "" => ("future".to_owned(), "Scheduled".to_owned()),
        other => ("future".to_owned(), other.to_owned()),
    }
}

/// Drop the date portion of an ISO-8601 timestamp and emit
/// just `HH:MM UTC`. Inputs look like `2026-05-04T19:00:00Z`.
fn pretty_time_utc(ts: &str) -> String {
    if let Some(t) = ts.split('T').nth(1) {
        let hhmm: String = t.chars().take(5).collect();
        if hhmm.len() == 5 {
            return format!("{hhmm} UTC");
        }
    }
    String::new()
}

async fn build_scores_result(state: &WebState, q: &ScoresQuery) -> ScoresResult {
    let active_label = state.config.read().await.active_label.clone();

    let today = Utc::now().date_naive();
    let active_date = q.date.as_deref().and_then(parse_date).unwrap_or(today);
    // Phase Foster +9 — `?range=` resolves the timeframe.
    // Day narrows the rendered grouping to the anchor date;
    // Week / Month surface the natural 7-day gameWeek
    // window the API already returns.
    let timeframe = parse_range_to_timeframe(q.range.as_deref());
    let (range_start, range_end) = timeframe.range(active_date);
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

    let (days, total_games, fetch_error) = match fetch_result {
        Ok(games) => {
            use std::collections::BTreeMap;
            let mut by_date: BTreeMap<String, Vec<ScoreRow>> = BTreeMap::new();
            let total = games.len();
            for g in games {
                let (state_class, state_label) =
                    state_to_class_label(g.game_state.as_deref(), g.last_period.as_deref());
                let series_context = if g.is_playoff() {
                    let series_game = g.series_game.unwrap_or_default();
                    let aw = g.away_wins.unwrap_or(0);
                    let hw = g.home_wins.unwrap_or(0);
                    let series_state = if aw > hw {
                        format!("{} leads {}-{}", g.away_abbrev, aw, hw)
                    } else if hw > aw {
                        format!("{} leads {}-{}", g.home_abbrev, hw, aw)
                    } else if aw == 0 {
                        "series begins".to_owned()
                    } else {
                        format!("tied {}-{}", aw, hw)
                    };
                    if series_game.is_empty() {
                        series_state
                    } else {
                        format!("{series_game} · {series_state}")
                    }
                } else {
                    String::new()
                };
                let row = ScoreRow {
                    away_abbrev: g.away_abbrev,
                    away_name: g.away_name,
                    home_abbrev: g.home_abbrev,
                    home_name: g.home_name,
                    away_score_str: g.away_score.map(|s| s.to_string()).unwrap_or_default(),
                    home_score_str: g.home_score.map(|s| s.to_string()).unwrap_or_default(),
                    state_label,
                    state_class,
                    start_time_label: pretty_time_utc(&g.start_time_utc),
                    is_playoff: g.game_type == 3,
                    series_context,
                };
                by_date.entry(g.date).or_default().push(row);
            }
            // Phase Foster +9 — keep only days that fall
            // inside `(range_start, range_end)`. Day collapses
            // to a single date; Week/Month widen.
            by_date.retain(|date_str, _| {
                match parse_date(date_str) {
                    Some(d) => d >= range_start && d <= range_end,
                    None => true, // unparseable date stays — defensive
                }
            });
            let days: Vec<ScoresDay> = by_date
                .into_iter()
                .map(|(date, rows)| {
                    let date_pretty = parse_date(&date)
                        .map(pretty_day)
                        .unwrap_or_else(|| date.clone());
                    ScoresDay {
                        date,
                        date_pretty,
                        rows,
                    }
                })
                .collect();
            (days, total, None)
        }
        Err(e) => (Vec::new(), 0, Some(e.to_string())),
    };

    ScoresResult {
        active_label,
        active_date: active_date.format("%Y-%m-%d").to_string(),
        prev_date: prev_date.format("%Y-%m-%d").to_string(),
        next_date: next_date.format("%Y-%m-%d").to_string(),
        today_date: today.format("%Y-%m-%d").to_string(),
        range: match timeframe {
            icelines_core::timeframe::Timeframe::Day => "day",
            icelines_core::timeframe::Timeframe::Week => "week",
            icelines_core::timeframe::Timeframe::Month => "month",
            icelines_core::timeframe::Timeframe::Season => "season",
        }
        .to_owned(),
        days,
        total_games,
        fetch_error,
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
    crate::api::json_envelope(
        "scores",
        result.days,
        ScoresMeta {
            active_date: result.active_date,
            today_date: result.today_date,
            range: result.range,
            total_games: result.total_games,
        },
        result.fetch_error,
    )
}
