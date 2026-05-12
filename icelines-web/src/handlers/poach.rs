use crate::state::WebState;
use crate::templates::{PoachRow, PoachTemplate};
use askama::Template;
use axum::extract::{Form, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use icelines_core::model::{Position, Season, TeamAbbr};
use icelines_core::season_stats::SeasonType;
use icelines_core::view_model::DeploymentSignal;
use icelines_core::{
    view_model::{
        poach_report_from_board, watch_rules_view_with_persisted,
        weekly_poach_report_from_board_with_watched, AvailabilityState, PoachAvailabilityFilter,
        PoachBoardView, PoachQuery, PoachReportView, WatchRule, WatchRuleMutationIntent,
        WatchRuleTrigger,
    },
    Completeness, EmptyKind, EmptyState, SourceKind, SourceState, ViewContext, ViewWindow,
    CURRENT_SEASON,
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
    #[serde(default)]
    pub availability: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct WatchRulesErrorResponse {
    error: String,
}

#[derive(Debug, Deserialize)]
pub struct WatchRuleMutationRequest {
    pub rule_id: String,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct WatchRuleMutationForm {
    pub rule_id: String,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct WatchRuleCreateForm {
    pub player: String,
    pub trigger: String,
}

#[derive(Debug, Deserialize)]
pub struct WatchRuleDeleteForm {
    pub rule_id: String,
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
            availability: availability_label(row.availability).to_string(),
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
        availability: q.availability.unwrap_or_else(|| "any".to_string()),
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
    let context = match watch_context_from_state(&state).await {
        Ok(context) => context,
        Err(response) => return response,
    };
    let view = watch_rules_view_with_persisted(context, read_persisted_watch_rules());
    axum::Json(view).into_response()
}

pub async fn post_watch_rule_enabled_json(
    State(state): State<WebState>,
    axum::Json(req): axum::Json<WatchRuleMutationRequest>,
) -> Response {
    let intent = match WatchRuleMutationIntent::resolve(&req.rule_id, req.enabled) {
        Ok(intent) => intent,
        Err(message) => return watch_rules_error(StatusCode::BAD_REQUEST, message),
    };
    let context = match watch_context_from_state(&state).await {
        Ok(context) => context,
        Err(response) => return response,
    };
    match set_persisted_watch_rule_enabled(&intent.rule_id, intent.enabled) {
        Ok(true) => axum::Json(intent.result_view(context, true)).into_response(),
        Ok(false) => watch_rules_error(
            StatusCode::NOT_FOUND,
            format!("unknown persisted watch rule '{}'", intent.rule_id),
        ),
        Err(message) => watch_rules_error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}

pub async fn post_watch_rule_enabled_form(
    State(state): State<WebState>,
    Form(req): Form<WatchRuleMutationForm>,
) -> Response {
    let intent = match WatchRuleMutationIntent::resolve(&req.rule_id, req.enabled) {
        Ok(intent) => intent,
        Err(message) => return bad_request_html(message),
    };
    let context = match watch_context_from_state(&state).await {
        Ok(context) => context,
        Err(response) => return response,
    };
    match set_persisted_watch_rule_enabled(&intent.rule_id, intent.enabled) {
        Ok(true) => {
            let _result = intent.result_view(context, true);
            Redirect::to("/watchlist").into_response()
        }
        Ok(false) => bad_request_html(format!("unknown persisted watch rule '{}'", intent.rule_id)),
        Err(message) => watch_rules_error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}

pub async fn post_watch_rule_create_form(Form(req): Form<WatchRuleCreateForm>) -> Response {
    let rule = match player_watch_rule(&req.player, &req.trigger) {
        Ok(rule) => rule,
        Err(message) => return bad_request_html(message),
    };
    let intent = match WatchRuleMutationIntent::create(&rule.id) {
        Ok(intent) => intent,
        Err(message) => return bad_request_html(message),
    };
    match persist_watch_rule(&rule) {
        Ok(()) => {
            let _result = intent.result_view(default_watch_context(), true);
            Redirect::to("/watchlist").into_response()
        }
        Err(message) => watch_rules_error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}

pub async fn post_watch_rule_delete_form(Form(req): Form<WatchRuleDeleteForm>) -> Response {
    let intent = match WatchRuleMutationIntent::delete(&req.rule_id) {
        Ok(intent) => intent,
        Err(message) => return bad_request_html(message),
    };
    match delete_persisted_watch_rule(&intent.rule_id) {
        Ok(true) => {
            let _result = intent.result_view(default_watch_context(), true);
            Redirect::to("/watchlist").into_response()
        }
        Ok(false) => bad_request_html(format!("unknown persisted watch rule '{}'", intent.rule_id)),
        Err(message) => watch_rules_error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}

async fn watch_context_from_state(state: &WebState) -> Result<ViewContext, Response> {
    let (season_str, season_type) = {
        let cfg = state.config.read().await;
        (
            cfg.active_season.clone(),
            SeasonType::parse_lossy(&cfg.active_season_type),
        )
    };
    let season_u32: u32 = match season_str.parse() {
        Ok(n) => n,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(WatchRulesErrorResponse {
                    error: format!("active season '{season_str}' is not a valid YYYYZZZZ id: {e}"),
                }),
            )
                .into_response());
        }
    };
    Ok(ViewContext::new(ViewWindow::new(
        Season(season_u32),
        season_type,
    )))
}

fn default_watch_context() -> ViewContext {
    ViewContext::new(ViewWindow::new(Season(CURRENT_SEASON), SeasonType::Regular))
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
    if !report.scoring_categories.is_empty() {
        body.push_str(&format!(
            "<p>Categories: <strong>{}</strong></p>",
            html_escape(&report.scoring_categories.join(", "))
        ));
    }

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
    let Some(db_path) = watch_db_path() else {
        return Vec::new();
    };
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

fn set_persisted_watch_rule_enabled(id: &str, enabled: bool) -> Result<bool, String> {
    let db_path = watch_db_path().ok_or_else(|| "HOME / USERPROFILE not set.".to_string())?;
    if !db_path.exists() {
        return Ok(false);
    }
    let conn = rusqlite::Connection::open(&db_path).map_err(|err| format!("open db: {err}"))?;
    let changed = conn
        .execute(
            "UPDATE watch_rules
             SET enabled = ?2, updated_at = datetime('now')
             WHERE id = ?1",
            rusqlite::params![id, if enabled { 1 } else { 0 }],
        )
        .map_err(|err| format!("update watch rule: {err}"))?;
    Ok(changed > 0)
}

fn delete_persisted_watch_rule(id: &str) -> Result<bool, String> {
    let db_path = watch_db_path().ok_or_else(|| "HOME / USERPROFILE not set.".to_string())?;
    if !db_path.exists() {
        return Ok(false);
    }
    let conn = rusqlite::Connection::open(&db_path).map_err(|err| format!("open db: {err}"))?;
    let changed = conn
        .execute(
            "DELETE FROM watch_rules WHERE id = ?1",
            rusqlite::params![id],
        )
        .map_err(|err| format!("delete watch rule: {err}"))?;
    Ok(changed > 0)
}

fn persist_watch_rule(rule: &WatchRule) -> Result<(), String> {
    let db_path = watch_db_path().ok_or_else(|| "HOME / USERPROFILE not set.".to_string())?;
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    let conn = rusqlite::Connection::open(&db_path).map_err(|err| format!("open db: {err}"))?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS watch_rules (
            id TEXT PRIMARY KEY,
            label TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            trigger_json TEXT NOT NULL,
            unsupported_sources_json TEXT NOT NULL DEFAULT '[]',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
         );",
    )
    .map_err(|err| format!("create watch_rules table: {err}"))?;
    let trigger_json =
        serde_json::to_string(&rule.trigger).map_err(|err| format!("encode trigger: {err}"))?;
    let unsupported_sources_json = serde_json::to_string(&rule.unsupported_sources)
        .map_err(|err| format!("encode unsupported sources: {err}"))?;
    conn.execute(
        "INSERT INTO watch_rules (
            id, label, enabled, trigger_json, unsupported_sources_json, created_at, updated_at
         )
         VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'), datetime('now'))
         ON CONFLICT(id) DO UPDATE SET
            label = excluded.label,
            enabled = excluded.enabled,
            trigger_json = excluded.trigger_json,
            unsupported_sources_json = excluded.unsupported_sources_json,
            updated_at = excluded.updated_at",
        rusqlite::params![
            rule.id,
            rule.label,
            if rule.enabled { 1_i64 } else { 0_i64 },
            trigger_json,
            unsupported_sources_json,
        ],
    )
    .map_err(|err| format!("upsert watch rule '{}': {err}", rule.id))?;
    Ok(())
}

fn player_watch_rule(player: &str, trigger: &str) -> Result<WatchRule, String> {
    let player = player.trim();
    if player.is_empty() {
        return Err("watch rule player is required".to_string());
    }
    let normalized_trigger = trigger.trim().to_ascii_lowercase();
    let (rule_trigger, unsupported_sources) = match normalized_trigger.as_str() {
        "available" | "availability" => (
            WatchRuleTrigger::AvailabilityChanged {
                player_id: None,
                state: AvailabilityState::Unknown,
            },
            vec![SourceKind::FantasyImport],
        ),
        "" | "pp1" | "pp2" | "top-six" | "promotion" | "line-change" => (
            WatchRuleTrigger::PlayerPromoted {
                player_id: None,
                evidence: DeploymentSignal::Unknown,
            },
            vec![SourceKind::Shifts],
        ),
        other => {
            return Err(format!(
                "unknown watch trigger '{other}' - valid: pp1, pp2, top-six, promotion, line-change, available"
            ));
        }
    };
    let trigger_label = if normalized_trigger.is_empty() {
        "promotion".to_string()
    } else {
        normalized_trigger
    };
    Ok(WatchRule {
        id: format!("player-{}", slug(player)),
        label: format!("Watch {player} when {trigger_label}"),
        enabled: true,
        trigger: rule_trigger,
        last_fired: None,
        unsupported_sources,
    })
}

fn slug(value: &str) -> String {
    value
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() {
                Some(ch.to_ascii_lowercase())
            } else if ch.is_whitespace() || ch == '-' || ch == '_' {
                Some('-')
            } else {
                None
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn watch_db_path() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|home| {
            std::path::PathBuf::from(&home)
                .join(".icelines")
                .join("icelines.db")
        })
}

fn watch_rules_error(status: StatusCode, message: impl Into<String>) -> Response {
    (
        status,
        axum::Json(WatchRulesErrorResponse {
            error: message.into(),
        }),
    )
        .into_response()
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
            SeasonType::parse_lossy(&cfg.active_season_type),
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
    query.positions = parse_positions(q.pos.as_deref()).map_err(bad_request_html)?;
    query.availability_filter =
        parse_availability_filter(q.availability.as_deref()).map_err(bad_request_html)?;
    query.limit = Some(q.top.unwrap_or(20).clamp(1, 100));
    query.sort = Some("poach_score".to_string());
    if let Some(rosters) = read_fantasy_rostered_player_keys(q.league.as_deref()) {
        query =
            query.with_imported_league_availability(rosters.all_rostered, rosters.user_rostered);
    }

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

struct FantasyRosterKeys {
    all_rostered: Vec<String>,
    user_rostered: Vec<String>,
}

fn read_fantasy_rostered_player_keys(league_name: Option<&str>) -> Option<FantasyRosterKeys> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    let db_path = std::path::PathBuf::from(&home)
        .join(".icelines")
        .join("icelines.db");
    if !db_path.exists() {
        return None;
    }
    let conn = rusqlite::Connection::open(&db_path).ok()?;
    let league_id: String = if let Some(name) = league_name {
        conn.query_row(
            "SELECT id FROM fl_leagues WHERE name = ?1",
            rusqlite::params![name],
            |row| row.get(0),
        )
        .ok()?
    } else {
        conn.query_row(
            "SELECT id FROM fl_leagues WHERE is_active = 1 LIMIT 1",
            [],
            |row| row.get(0),
        )
        .ok()?
    };
    let mut stmt = conn
        .prepare(
            "SELECT r.player_normalized, t.is_user_team
             FROM fl_roster r
             JOIN fl_teams t ON t.id = r.team_id
             WHERE t.league_id = ?1
             ORDER BY r.player_normalized",
        )
        .ok()?;
    stmt.query_map(rusqlite::params![league_id], |row| row.get::<_, String>(0))
        .ok()
        .map(|rows| {
            let mut all_rostered = Vec::new();
            let mut user_rostered = Vec::new();
            for row in rows.filter_map(Result::ok) {
                all_rostered.push(row.clone());
            }
            if let Ok(mut user_stmt) = conn.prepare(
                "SELECT r.player_normalized
                 FROM fl_roster r
                 JOIN fl_teams t ON t.id = r.team_id
                 WHERE t.league_id = ?1 AND t.is_user_team = 1
                 ORDER BY r.player_normalized",
            ) {
                user_rostered = user_stmt
                    .query_map(rusqlite::params![league_id], |row| row.get::<_, String>(0))
                    .ok()
                    .map(|rows| rows.filter_map(Result::ok).collect())
                    .unwrap_or_default();
            }
            FantasyRosterKeys {
                all_rostered,
                user_rostered,
            }
        })
}

fn parse_availability_filter(value: Option<&str>) -> Result<PoachAvailabilityFilter, String> {
    let Some(value) = value else {
        return Ok(PoachAvailabilityFilter::Any);
    };
    match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "" | "any" | "all" => Ok(PoachAvailabilityFilter::Any),
        "available" | "free" | "free_agent" | "free_agents" => {
            Ok(PoachAvailabilityFilter::Available)
        }
        "not_on_user_roster" | "not_user_roster" | "not_mine" => {
            Ok(PoachAvailabilityFilter::NotOnUserRoster)
        }
        "watched" | "watchlist" => Ok(PoachAvailabilityFilter::Watched),
        "imported_available" | "imported_free" | "league_available" => {
            Ok(PoachAvailabilityFilter::ImportedAvailable)
        }
        "unknown" => Ok(PoachAvailabilityFilter::Unknown),
        other => Err(format!(
            "unknown availability filter '{other}' - valid: any, available, imported_available, not_on_user_roster, watched, unknown"
        )),
    }
}

fn availability_label(state: AvailabilityState) -> &'static str {
    match state {
        AvailabilityState::Unknown => "unknown",
        AvailabilityState::Available => "available",
        AvailabilityState::RosteredByUser => "my roster",
        AvailabilityState::ImportedAvailable => "free",
        AvailabilityState::ImportedRostered => "rostered",
        AvailabilityState::Watched => "watched",
    }
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

fn parse_positions(value: Option<&str>) -> Result<Vec<Position>, String> {
    split_csv(value)
        .into_iter()
        .map(|value| match value.to_ascii_uppercase().as_str() {
            "C" => Ok(Position::Center),
            "LW" | "L" => Ok(Position::LeftWing),
            "RW" | "R" => Ok(Position::RightWing),
            "D" => Ok(Position::Defense),
            "G" => Ok(Position::Goalie),
            other => Err(format!(
                "unknown position '{other}' - valid: C, LW, RW, D, G"
            )),
        })
        .collect()
}

fn bad_request_html(message: String) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Html(format!(
            "<!doctype html><body><h1>400</h1><p>{message}</p></body>"
        )),
    )
        .into_response()
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
