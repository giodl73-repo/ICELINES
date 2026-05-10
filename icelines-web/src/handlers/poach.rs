use crate::state::WebState;
use crate::templates::{PoachRow, PoachTemplate};
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::model::{Position, Season, TeamAbbr};
use icelines_core::{
    view_model::{
        default_watch_rules_view, poach_report_from_board,
        weekly_poach_report_from_board_with_watched, PoachBoardView, PoachQuery, PoachReportView,
        WatchRule,
    },
    Completeness, EmptyKind, EmptyState, SourceKind, SourceState, ViewContext, ViewWindow,
};
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct PoachWebQuery {
    #[serde(default)]
    pub scheme: Option<String>,
    #[serde(default, rename = "category")]
    pub categories: Option<String>,
    #[serde(default)]
    pub team: Option<String>,
    #[serde(default)]
    pub pos: Option<String>,
    #[serde(default)]
    pub top: Option<u16>,
    #[serde(default)]
    pub league: Option<String>,
}

pub async fn get_poach(State(state): State<WebState>, Query(q): Query<PoachWebQuery>) -> Response {
    let result = match build_poach_view(&state, &q).await {
        Ok(result) => result,
        Err(response) => return response,
    };

    let empty = result.view.empty_state.clone();
    let rows = result
        .view
        .rows
        .iter()
        .enumerate()
        .map(|(idx, row)| PoachRow {
            rank: idx + 1,
            player_id: row.player_id.0,
            name: row.display_name.clone(),
            team: row.team.as_str().to_string(),
            position: row.position.abbreviation().to_string(),
            score: format!("{:.1}", row.score.final_score),
            confidence: format!("{:?}", row.confidence).to_ascii_lowercase(),
            category_fit: row.category_fit_summary.clone(),
            schedule: row.schedule_summary.clone(),
            risk: row.risk_summary.clone().unwrap_or_else(|| "-".to_string()),
            why: row
                .explanations
                .first()
                .map(|explanation| explanation.message.clone())
                .unwrap_or_else(|| "No explanation".to_string()),
        })
        .collect::<Vec<_>>();
    let total = result.view.rows.len();

    let tmpl = PoachTemplate {
        active_label: result.active_label,
        rows,
        total,
        scoring_scheme: result.view.scoring_scheme,
        categories: q.categories.unwrap_or_default(),
        teams: q.team.unwrap_or_default(),
        positions: q.pos.unwrap_or_default(),
        source_note: source_note(&result.view.source_state),
        empty_title: empty
            .as_ref()
            .map(|state| state.title.clone())
            .unwrap_or_default(),
        empty_detail: empty
            .and_then(|state| state.detail.clone())
            .unwrap_or_default(),
    };

    match tmpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!(
                "<!doctype html><body><h1>500</h1><p>{e}</p></body>"
            )),
        )
            .into_response(),
    }
}

pub async fn get_poach_report(
    State(state): State<WebState>,
    Query(q): Query<PoachWebQuery>,
) -> Response {
    let result = match build_poach_view(&state, &q).await {
        Ok(result) => result,
        Err(response) => return response,
    };
    let report = poach_report_from_board(result.view);
    Html(render_poach_report_html(&report, &result.active_label)).into_response()
}

pub async fn get_weekly_report(
    State(state): State<WebState>,
    Query(q): Query<PoachWebQuery>,
) -> Response {
    let result = match build_poach_view(&state, &q).await {
        Ok(result) => result,
        Err(response) => return response,
    };
    let league = q.league.as_deref().unwrap_or("default");
    let top = q.top.unwrap_or(20).clamp(1, 100);
    let watched = read_watchlist_player_keys();
    let report = weekly_poach_report_from_board_with_watched(result.view, league, top, &watched);
    Html(render_poach_report_html(&report, &result.active_label)).into_response()
}

pub async fn get_poach_json(
    State(state): State<WebState>,
    Query(q): Query<PoachWebQuery>,
) -> Response {
    let result = match build_poach_view(&state, &q).await {
        Ok(result) => result,
        Err(response) => return response,
    };
    axum::Json(result.view).into_response()
}

pub async fn get_watch_rules_json(State(state): State<WebState>) -> Response {
    let (season_str, season_type) = {
        let cfg = state.config.read().await;
        (
            cfg.active_season.clone(),
            super::leaders::parse_season_type(&cfg.active_season_type),
        )
    };
    let season_u32: u32 = match season_str.parse() {
        Ok(n) => n,
        Err(e) => {
            return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        axum::Json(serde_json::json!({
                            "error": format!("active season '{season_str}' is not a valid YYYYZZZZ id: {e}"),
                        })),
                    )
                        .into_response();
        }
    };
    let mut context = ViewContext::new(ViewWindow::new(Season(season_u32), season_type));
    context.completeness = Completeness::Partial;
    context.source_state = vec![
        SourceState::complete(SourceKind::Roster),
        SourceState::missing(SourceKind::Shifts),
        SourceState::missing(SourceKind::Schedule),
        SourceState::missing(SourceKind::FantasyImport),
    ];

    let mut view = default_watch_rules_view(context);
    view.rules.extend(read_persisted_watch_rules());
    axum::Json(view).into_response()
}

fn render_poach_report_html(report: &PoachReportView, active_label: &str) -> String {
    let mut body = String::new();
    body.push_str("<!DOCTYPE html><html><head>");
    body.push_str("<meta charset=\"utf-8\">");
    body.push_str("<title>Poach Report - IceLines</title>");
    body.push_str("<link rel=\"stylesheet\" href=\"/static/style.css\">");
    body.push_str("</head><body>");
    body.push_str(
        "<nav><a href=\"/\">League</a> - <a href=\"/poach\">Poach</a> - \
                 <a href=\"/watchlist\">Watchlist</a> - <strong>Report</strong></nav>",
    );
    body.push_str("<main>");
    body.push_str(&format!(
        "<h1>{} Report</h1>",
        html_escape(&report.context.title)
    ));
    body.push_str(&format!(
        "<p class=\"season-header\">{}</p>",
        html_escape(active_label)
    ));
    body.push_str(&format!(
        "<p>Scheme: <strong>{}</strong></p>",
        html_escape(&report.scoring_scheme)
    ));

    if !report.omissions.is_empty() {
        body.push_str("<section><h2>Source Omissions</h2><ul>");
        for omission in &report.omissions {
            body.push_str(&format!("<li>{}</li>", html_escape(omission)));
        }
        body.push_str("</ul></section>");
    }

    for section in &report.sections {
        body.push_str(&format!(
            "<section><h2>{}</h2>",
            html_escape(&section.title)
        ));
        if section.rows.is_empty() {
            body.push_str("<p>No candidates matched this report.</p></section>");
            continue;
        }
        body.push_str("<table><thead><tr><th>#</th><th>Player</th><th>Team</th><th>Pos</th><th>Score</th><th>Confidence</th><th>Why</th></tr></thead><tbody>");
        for (idx, row) in section.rows.iter().enumerate() {
            let why = row
                .explanations
                .first()
                .map(|explanation| explanation.message.as_str())
                .unwrap_or("No explanation");
            body.push_str(&format!(
                        "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{:.1}</td><td>{:?}</td><td>{}</td></tr>",
                        idx + 1,
                        html_escape(&row.display_name),
                        html_escape(row.team.as_str()),
                        html_escape(row.position.abbreviation()),
                        row.score.final_score,
                        row.confidence,
                        html_escape(why)
                    ));
        }
        body.push_str("</tbody></table></section>");
    }
    body.push_str("</main></body></html>");
    body
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn read_persisted_watch_rules() -> Vec<WatchRule> {
    let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) else {
        return Vec::new();
    };
    let db_path = std::path::PathBuf::from(&home)
        .join(".icelines")
        .join("icelines.db");
    if !db_path.exists() {
        return Vec::new();
    }
    let Ok(conn) = rusqlite::Connection::open(&db_path) else {
        return Vec::new();
    };
    let latest_fired = read_watch_rule_last_fired(&conn);
    let Ok(mut stmt) = conn.prepare(
        "SELECT id, label, enabled, trigger_json, unsupported_sources_json \
                 FROM watch_rules \
                 ORDER BY id",
    ) else {
        return Vec::new();
    };
    stmt.query_map([], |r| {
        let id: String = r.get(0)?;
        let label: String = r.get(1)?;
        let enabled: i64 = r.get(2)?;
        let trigger_json: String = r.get(3)?;
        let unsupported_sources_json: String = r.get(4)?;
        let trigger = serde_json::from_str(&trigger_json)
            .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?;
        let unsupported_sources = serde_json::from_str(&unsupported_sources_json)
            .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?;
        Ok(WatchRule {
            last_fired: latest_fired.get(&id).copied(),
            id,
            label,
            enabled: enabled != 0,
            trigger,
            unsupported_sources,
        })
    })
    .ok()
    .map(|rows| rows.filter_map(Result::ok).collect())
    .unwrap_or_default()
}

fn read_watch_rule_last_fired(
    conn: &rusqlite::Connection,
) -> std::collections::HashMap<String, chrono::DateTime<chrono::Utc>> {
    let Ok(mut stmt) = conn.prepare(
        "SELECT rule_id, MAX(fired_at)
                 FROM watch_rule_events
                 GROUP BY rule_id",
    ) else {
        return std::collections::HashMap::new();
    };
    stmt.query_map([], |r| {
        let rule_id: String = r.get(0)?;
        let fired_at: String = r.get(1)?;
        Ok((rule_id, fired_at))
    })
    .ok()
    .map(|rows| {
        rows.filter_map(Result::ok)
            .filter_map(|(rule_id, fired_at)| {
                chrono::DateTime::parse_from_rfc3339(&fired_at)
                    .ok()
                    .map(|dt| (rule_id, dt.with_timezone(&chrono::Utc)))
            })
            .collect()
    })
    .unwrap_or_default()
}

fn read_watchlist_player_keys() -> Vec<String> {
    let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) else {
        return Vec::new();
    };
    let db_path = std::path::PathBuf::from(&home)
        .join(".icelines")
        .join("icelines.db");
    if !db_path.exists() {
        return Vec::new();
    }
    let Ok(conn) = rusqlite::Connection::open(&db_path) else {
        return Vec::new();
    };
    let Ok(mut stmt) = conn.prepare(
        "SELECT entity_ref FROM group_members \
                 WHERE group_name = 'Watchlist' \
                 ORDER BY entity_ref",
    ) else {
        return Vec::new();
    };
    stmt.query_map([], |r| r.get::<_, String>(0))
        .ok()
        .map(|rows| {
            rows.filter_map(Result::ok)
                .filter_map(|entity_ref| match entity_ref.split_once(':') {
                    Some(("player", key)) => Some(key.to_string()),
                    Some(("team", _)) => None,
                    _ => Some(entity_ref),
                })
                .collect()
        })
        .unwrap_or_default()
}

struct PoachBuildResult {
    view: PoachBoardView,
    active_label: String,
}

async fn build_poach_view(
    state: &WebState,
    q: &PoachWebQuery,
) -> Result<PoachBuildResult, Response> {
    let (season_str, season_type, active_label) = {
        let cfg = state.config.read().await;
        (
            cfg.active_season.clone(),
            super::leaders::parse_season_type(&cfg.active_season_type),
            cfg.active_label.clone(),
        )
    };
    let season_u32: u32 = match season_str.parse() {
        Ok(n) => n,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(format!(
                    "<!doctype html><body><h1>500</h1><p>active season \
                             '{season_str}' is not a valid YYYYZZZZ id: {e}</p></body>"
                )),
            )
                .into_response());
        }
    };
    let season = Season(season_u32);
    let scheme = q
        .scheme
        .clone()
        .unwrap_or_else(|| "yahoo-standard".to_string());
    let mut query = PoachQuery::new(season, season_type, scheme.clone());
    query.categories = split_csv(q.categories.as_deref())
        .into_iter()
        .map(|category| category.to_ascii_lowercase())
        .collect();
    query.teams = split_csv(q.team.as_deref())
        .into_iter()
        .map(|team| TeamAbbr(team.to_ascii_uppercase()))
        .collect();
    query.positions = parse_positions(q.pos.as_deref())?;
    query.limit = Some(q.top.unwrap_or(20).clamp(1, 100));
    query.sort = Some("poach_score".to_string());

    let view = {
        let repo = state.repo.read().await;
        if repo.has_window(season, season_type) {
            PoachBoardView::from_repository(&repo, query)
        } else {
            let mut view = PoachBoardView::new(
                ViewContext::new(ViewWindow::new(season, season_type)),
                query,
                scheme,
            );
            view.context.completeness = Completeness::Unavailable;
            view.source_state = vec![SourceState::missing(SourceKind::Roster)];
            view.empty_state = Some(EmptyState {
                kind: EmptyKind::MissingSource,
                title: "Missing poacher source data".to_string(),
                detail: Some("The active season/type window is not loaded.".to_string()),
                recovery: Vec::new(),
            });
            view
        }
    };

    Ok(PoachBuildResult { view, active_label })
}

fn split_csv(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn parse_positions(value: Option<&str>) -> Result<Vec<Position>, Response> {
    split_csv(value)
                .into_iter()
                .map(|value| match value.to_ascii_uppercase().as_str() {
                    "C" => Ok(Position::Center),
                    "LW" | "L" => Ok(Position::LeftWing),
                    "RW" | "R" => Ok(Position::RightWing),
                    "D" => Ok(Position::Defense),
                    "G" => Ok(Position::Goalie),
                    other => Err((
                        StatusCode::BAD_REQUEST,
                        Html(format!(
                            "<!doctype html><body><h1>400</h1><p>unknown position '{other}' - valid: C, LW, RW, D, G</p></body>"
                        )),
                    )
                        .into_response()),
                })
                .collect()
}

fn source_note(source_state: &[SourceState]) -> String {
    let missing = source_state
        .iter()
        .filter(|state| state.state != Completeness::Complete)
        .map(|state| format!("{:?}", state.source).to_ascii_lowercase())
        .collect::<Vec<_>>();
    if missing.is_empty() {
        String::new()
    } else {
        format!(
            "Missing source data is disclosed, not scored as negative evidence: {}.",
            missing.join(", ")
        )
    }
}
