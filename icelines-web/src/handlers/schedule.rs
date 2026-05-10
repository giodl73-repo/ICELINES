use crate::state::WebState;
use crate::templates::{ScheduleRow, ScheduleTemplate, TeamChip};
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct ScheduleQuery {
    #[serde(default)]
    pub team: Option<String>,
    /// Phase Foster.1 — anchor date `YYYY-MM-DD` for the
    /// date-windowed slate. Mutually exclusive with `?team=`
    /// in v1: when `team` is set, returns the team's full
    /// season; when only `date` is set, returns that day's
    /// slate via `fetch_schedule_for_date`. Drops the older
    /// `?start=` (which never shipped on this route — the
    /// CLI's `--start` is the deprecated surface).
    #[serde(default)]
    pub date: Option<String>,
}

struct ScheduleResult {
    active_label: String,
    season_pretty: String,
    active_team: String,
    active_date: Option<String>,
    team_chips: Vec<TeamChip>,
    rows: Vec<ScheduleRow>,
    total: usize,
    fetch_error: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct ScheduleEnvelope {
    schema_version: u32,
    route: &'static str,
    data: Vec<ScheduleRow>,
    meta: ScheduleMeta,
    error: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct ScheduleMeta {
    season: String,
    active_team: String,
    active_date: Option<String>,
    total: usize,
    team_chips: Vec<TeamChip>,
}

fn pretty_season(s: &str) -> String {
    if s.len() == 8 {
        format!("{}-{}", &s[0..4], &s[6..8])
    } else {
        s.to_owned()
    }
}

/// 32 active NHL franchises. Used to populate the team
/// picker chip strip. Uppercase, alphabetical.
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
    let envelope = ScheduleEnvelope {
        schema_version: 1,
        route: "schedule",
        data: result.rows,
        meta: ScheduleMeta {
            season: result.season_pretty,
            active_team: result.active_team,
            active_date: result.active_date,
            total: result.total,
            team_chips: result.team_chips,
        },
        error: result.fetch_error,
    };
    axum::Json(envelope).into_response()
}

async fn build_schedule_result(state: &WebState, q: &ScheduleQuery) -> ScheduleResult {
    let (active_label, season_str) = {
        let cfg = state.config.read().await;
        (cfg.active_label.clone(), cfg.active_season.clone())
    };

    let team_upper = q
        .team
        .as_deref()
        .map(|t| t.trim().to_ascii_uppercase())
        .filter(|t| !t.is_empty())
        .unwrap_or_default();

    let team_chips: Vec<TeamChip> = ALL_TEAM_ABBREVS
        .iter()
        .map(|a| TeamChip {
            abbrev: (*a).to_owned(),
            is_active: a.eq_ignore_ascii_case(&team_upper),
        })
        .collect();

    // Phase Foster.1 — `?date=` anchors a single-day slate fetch
    // when no team is set. Existing team-season path takes
    // precedence so bookmarks like `/schedule?team=EDM` keep
    // working.
    let (rows, total, fetch_error) = if team_upper.is_empty() {
        if let Some(date) = q.date.as_deref().filter(|d| !d.is_empty()) {
            let client = super::nhl_client();
            match client.fetch_schedule_for_date(date).await {
                Ok(games) => {
                    let mut rows: Vec<ScheduleRow> = games
                        .into_iter()
                        .map(|g| ScheduleRow {
                            date: g.date,
                            away_abbrev: g.away_abbrev.clone(),
                            home_abbrev: g.home_abbrev.clone(),
                            away_score_str: g.away_score.map(|s| s.to_string()).unwrap_or_default(),
                            home_score_str: g.home_score.map(|s| s.to_string()).unwrap_or_default(),
                            state_label: g.game_state.clone().unwrap_or_else(|| "Scheduled".into()),
                            home_or_away: "—".to_owned(),
                            opponent_abbrev: String::new(),
                            is_playoff: g.game_type == 3,
                        })
                        .collect();
                    rows.sort_by(|a, b| a.date.cmp(&b.date));
                    let total = rows.len();
                    (rows, total, None)
                }
                Err(e) => (Vec::new(), 0, Some(e.to_string())),
            }
        } else {
            (Vec::new(), 0, None)
        }
    } else {
        let client = super::nhl_client();
        match client
            .fetch_team_season_schedule(&team_upper, &season_str)
            .await
        {
            Ok(games) => {
                let mut rows: Vec<ScheduleRow> = games
                    .into_iter()
                    .map(|g| {
                        let is_home = g.home_abbrev.eq_ignore_ascii_case(&team_upper);
                        let opponent = if is_home {
                            g.away_abbrev.clone()
                        } else {
                            g.home_abbrev.clone()
                        };
                        let state_label = match g.game_state.as_deref() {
                            Some("FINAL") | Some("OFF") => match g.last_period.as_deref() {
                                Some("OT") => "FINAL/OT".to_owned(),
                                Some("SO") => "FINAL/SO".to_owned(),
                                _ => "FINAL".to_owned(),
                            },
                            Some("LIVE") | Some("CRIT") => "LIVE".to_owned(),
                            Some("PRE") => "Pre-game".to_owned(),
                            Some("FUT") | None => "Scheduled".to_owned(),
                            Some(s) => s.to_owned(),
                        };
                        ScheduleRow {
                            date: g.date,
                            away_abbrev: g.away_abbrev.clone(),
                            home_abbrev: g.home_abbrev.clone(),
                            away_score_str: g.away_score.map(|s| s.to_string()).unwrap_or_default(),
                            home_score_str: g.home_score.map(|s| s.to_string()).unwrap_or_default(),
                            state_label,
                            home_or_away: if is_home {
                                "Home".to_owned()
                            } else {
                                "Away".to_owned()
                            },
                            opponent_abbrev: opponent,
                            is_playoff: g.game_type == 3,
                        }
                    })
                    .collect();
                rows.sort_by(|a, b| a.date.cmp(&b.date));
                let total = rows.len();
                (rows, total, None)
            }
            Err(e) => (Vec::new(), 0, Some(e.to_string())),
        }
    };

    let active_date = if team_upper.is_empty() {
        q.date
            .as_deref()
            .map(str::trim)
            .filter(|d| !d.is_empty())
            .map(str::to_owned)
    } else {
        None
    };

    ScheduleResult {
        active_label,
        season_pretty: pretty_season(&season_str),
        active_team: team_upper,
        active_date,
        team_chips,
        rows,
        total,
        fetch_error,
    }
}
