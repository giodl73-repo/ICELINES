use chrono::{Datelike, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

use crate::timeframe::Timeframe;
use crate::view_model::context::{
    EmptyKind, EmptyState, SourceKind, SourceState, ViewContext, ViewWarning, ViewWindow,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoresView {
    pub context: ViewContext,
    pub active_date: String,
    pub today_date: String,
    pub range: String,
    pub days: Vec<ScoresDayView>,
    pub total_games: usize,
    pub warnings: Vec<ViewWarning>,
    pub empty_state: Option<EmptyState>,
}

impl ScoresView {
    pub fn from_games(
        mut context: ViewContext,
        active_date: NaiveDate,
        today_date: NaiveDate,
        timeframe: Timeframe,
        games: Vec<ScheduledGameInput>,
    ) -> Self {
        use std::collections::BTreeMap;

        context
            .source_state
            .push(SourceState::complete(SourceKind::Scores));

        let (range_start, range_end) = timeframe.range(active_date);
        let total_games = games.len();
        let mut by_date: BTreeMap<String, Vec<ScoreGameRow>> = BTreeMap::new();
        for game in games {
            let (state_class, state_label) =
                state_to_class_label(game.game_state.as_deref(), game.last_period.as_deref());
            let series_context = if game.game_type == 3 {
                series_context(&game)
            } else {
                String::new()
            };
            let row = ScoreGameRow {
                game_id: game.game_id,
                away_abbrev: game.away_abbrev,
                away_name: game.away_name,
                home_abbrev: game.home_abbrev,
                home_name: game.home_name,
                away_score_str: game
                    .away_score
                    .map(|score| score.to_string())
                    .unwrap_or_default(),
                home_score_str: game
                    .home_score
                    .map(|score| score.to_string())
                    .unwrap_or_default(),
                state_label,
                state_class,
                start_time_utc: game.start_time_utc.clone(),
                start_time_label: pretty_time_utc(&game.start_time_utc),
                is_playoff: game.game_type == 3,
                series_context,
            };
            by_date.entry(game.date).or_default().push(row);
        }

        by_date.retain(|date_str, _| match parse_date(date_str) {
            Some(date) => date >= range_start && date <= range_end,
            None => true,
        });
        let days: Vec<ScoresDayView> = by_date
            .into_iter()
            .map(|(date, rows)| {
                let date_pretty = parse_date(&date)
                    .map(pretty_day)
                    .unwrap_or_else(|| date.clone());
                ScoresDayView {
                    date,
                    date_pretty,
                    rows,
                }
            })
            .collect();
        let empty_state = if days.is_empty() {
            Some(EmptyState {
                kind: EmptyKind::NoRows,
                title: "No games".to_string(),
                detail: Some("No games matched the selected date range.".to_string()),
                recovery: Vec::new(),
            })
        } else {
            None
        };

        Self {
            context,
            active_date: active_date.format("%Y-%m-%d").to_string(),
            today_date: today_date.format("%Y-%m-%d").to_string(),
            range: timeframe_label(timeframe).to_string(),
            days,
            total_games,
            warnings: Vec::new(),
            empty_state,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduledGameInput {
    pub game_id: u64,
    pub date: String,
    pub game_type: u8,
    pub away_abbrev: String,
    pub away_name: String,
    pub home_abbrev: String,
    pub home_name: String,
    pub start_time_utc: String,
    pub away_score: Option<u8>,
    pub home_score: Option<u8>,
    pub game_state: Option<String>,
    pub last_period: Option<String>,
    pub series_game: Option<String>,
    pub away_wins: Option<u8>,
    pub home_wins: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoresDayView {
    pub date: String,
    pub date_pretty: String,
    pub rows: Vec<ScoreGameRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoreGameRow {
    pub game_id: u64,
    pub away_abbrev: String,
    pub away_name: String,
    pub home_abbrev: String,
    pub home_name: String,
    pub away_score_str: String,
    pub home_score_str: String,
    pub state_label: String,
    pub state_class: String,
    pub start_time_utc: String,
    pub start_time_label: String,
    pub is_playoff: bool,
    pub series_context: String,
}

pub fn scores_context(window: ViewWindow) -> ViewContext {
    ViewContext::new(window)
}

fn timeframe_label(timeframe: Timeframe) -> &'static str {
    match timeframe {
        Timeframe::Day => "day",
        Timeframe::Week => "week",
        Timeframe::Month => "month",
        Timeframe::Season => "season",
    }
}

fn parse_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

fn pretty_day(date: NaiveDate) -> String {
    let weekday = match date.weekday() {
        Weekday::Mon => "Mon",
        Weekday::Tue => "Tue",
        Weekday::Wed => "Wed",
        Weekday::Thu => "Thu",
        Weekday::Fri => "Fri",
        Weekday::Sat => "Sat",
        Weekday::Sun => "Sun",
    };
    let month = match date.month() {
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
    format!("{}, {} {}, {}", weekday, month, date.day(), date.year())
}

fn state_to_class_label(state: Option<&str>, last_period: Option<&str>) -> (String, String) {
    match state.unwrap_or("") {
        "FINAL" | "OFF" => {
            let label = match last_period.unwrap_or("REG") {
                "OT" => "FINAL/OT".to_string(),
                "SO" => "FINAL/SO".to_string(),
                _ => "FINAL".to_string(),
            };
            ("final".to_string(), label)
        }
        "LIVE" | "CRIT" => ("live".to_string(), "LIVE".to_string()),
        "PRE" => ("future".to_string(), "Pre-game".to_string()),
        "FUT" | "" => ("future".to_string(), "Scheduled".to_string()),
        other => ("future".to_string(), other.to_string()),
    }
}

fn series_context(game: &ScheduledGameInput) -> String {
    let series_game = game.series_game.clone().unwrap_or_default();
    let away_wins = game.away_wins.unwrap_or(0);
    let home_wins = game.home_wins.unwrap_or(0);
    let series_state = if away_wins > home_wins {
        format!("{} leads {}-{}", game.away_abbrev, away_wins, home_wins)
    } else if home_wins > away_wins {
        format!("{} leads {}-{}", game.home_abbrev, home_wins, away_wins)
    } else if away_wins == 0 {
        "series begins".to_string()
    } else {
        format!("tied {}-{}", away_wins, home_wins)
    };

    if series_game.is_empty() {
        series_state
    } else {
        format!("{series_game} · {series_state}")
    }
}

fn pretty_time_utc(timestamp: &str) -> String {
    if let Some(time) = timestamp.split('T').nth(1) {
        let hhmm: String = time.chars().take(5).collect();
        if hhmm.len() == 5 {
            return format!("{hhmm} UTC");
        }
    }
    String::new()
}
