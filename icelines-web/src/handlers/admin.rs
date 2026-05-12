use std::path::PathBuf;

use axum::extract::{Json, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::{
    ConfigEntryInput, ConfigMutationIntent, ConfigView, DataMutationIntent, DataMutationOperation,
    DataStatusEntryInput, DataStatusView, Season, SnapshotEntryInput, SnapshotMutationIntent,
    SnapshotMutationOperation, SnapshotView, ViewContext, ViewWindow, CURRENT_SEASON,
};
use icelines_fetch::datastore::DataStore;
use icelines_fetch::manifest::{DataKey, DataKind};
use icelines_fetch::snapshot::SnapshotStore;
use serde::Deserialize;

use crate::state::WebState;

#[derive(Debug, Deserialize, Default)]
pub struct AdminDataStatusQuery {
    #[serde(default)]
    pub shard: Option<String>,
    #[serde(default)]
    pub stale_only: bool,
}

#[derive(Debug, Deserialize, Default)]
pub struct AdminSnapshotQuery {
    #[serde(default)]
    pub selected: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct AdminConfigQuery {
    #[serde(default)]
    pub selected: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AdminConfigMutationRequest {
    pub key: String,
    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AdminSnapshotMutationRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct AdminDataVerifyRequest {
    pub target: String,
}

#[derive(Debug, serde::Serialize)]
struct AdminErrorResponse {
    error: String,
}

pub async fn get_data_status_json(Query(q): Query<AdminDataStatusQuery>) -> Response {
    match build_data_status_view(q) {
        Ok(view) => axum::Json(view).into_response(),
        Err(message) => admin_error(message),
    }
}

pub async fn get_snapshots_json(Query(q): Query<AdminSnapshotQuery>) -> Response {
    match build_snapshot_view(q) {
        Ok(view) => axum::Json(view).into_response(),
        Err(message) => admin_error(message),
    }
}

pub async fn get_config_json(
    State(state): State<WebState>,
    Query(q): Query<AdminConfigQuery>,
) -> Response {
    let config = state.config.read().await.clone();
    axum::Json(build_config_view(&config, q)).into_response()
}

pub async fn get_admin(State(state): State<WebState>) -> Response {
    let config = state.config.read().await.clone();
    let data_view = match build_data_status_view(AdminDataStatusQuery::default()) {
        Ok(view) => view,
        Err(message) => return admin_error_html(message),
    };
    let snapshot_view = match build_snapshot_view(AdminSnapshotQuery::default()) {
        Ok(view) => view,
        Err(message) => return admin_error_html(message),
    };
    let config_view = build_config_view(&config, AdminConfigQuery::default());

    Html(render_admin_html(
        &config.active_label,
        &data_view,
        &snapshot_view,
        &config_view,
    ))
    .into_response()
}

pub async fn post_config_set_json(
    State(state): State<WebState>,
    Json(req): Json<AdminConfigMutationRequest>,
) -> Response {
    let Some(value) = req.value.as_deref() else {
        return admin_bad_request("config value is required");
    };
    let intent = match ConfigMutationIntent::set(&req.key, value) {
        Ok(intent) => intent,
        Err(message) => return admin_bad_request(message),
    };
    let mut config = state.config.write().await;
    match apply_web_config_set(&mut config, &intent.key, value) {
        Ok(changed) => axum::Json(intent.result_view(default_context(), changed)).into_response(),
        Err(message) => admin_bad_request(message),
    }
}

pub async fn post_config_reset_json(
    State(state): State<WebState>,
    Json(req): Json<AdminConfigMutationRequest>,
) -> Response {
    let intent = match ConfigMutationIntent::reset(&req.key) {
        Ok(intent) => intent,
        Err(message) => return admin_bad_request(message),
    };
    let mut config = state.config.write().await;
    match apply_web_config_reset(&mut config, &intent.key) {
        Ok(changed) => axum::Json(intent.result_view(default_context(), changed)).into_response(),
        Err(message) => admin_bad_request(message),
    }
}

pub async fn post_snapshot_activate_json(
    Json(req): Json<AdminSnapshotMutationRequest>,
) -> Response {
    let intent =
        match SnapshotMutationIntent::resolve(SnapshotMutationOperation::Activate, req.name) {
            Ok(intent) => intent,
            Err(message) => return admin_bad_request(message),
        };
    let store = SnapshotStore::new(SnapshotStore::default_root());
    let before = match store.load_manifest() {
        Ok(manifest) => manifest.active,
        Err(err) => return admin_error(format!("loading snapshot manifest: {err}")),
    };
    match store.set_active(&intent.name) {
        Ok(()) => {
            let changed = before.as_deref() != Some(intent.name.as_str());
            axum::Json(intent.result_view(default_context(), changed)).into_response()
        }
        Err(err) => admin_bad_request(format!("activating snapshot '{}': {err}", intent.name)),
    }
}

pub async fn post_snapshot_delete_json(Json(req): Json<AdminSnapshotMutationRequest>) -> Response {
    let intent = match SnapshotMutationIntent::resolve(SnapshotMutationOperation::Remove, req.name)
    {
        Ok(intent) => intent,
        Err(message) => return admin_bad_request(message),
    };
    let store = SnapshotStore::new(SnapshotStore::default_root());
    match store.delete(&intent.name) {
        Ok(()) => axum::Json(intent.result_view(default_context(), true)).into_response(),
        Err(err) => admin_bad_request(format!("deleting snapshot '{}': {err}", intent.name)),
    }
}

pub async fn post_data_verify_json(Json(req): Json<AdminDataVerifyRequest>) -> Response {
    let intent = match DataMutationIntent::resolve(DataMutationOperation::Verify, req.target, false)
    {
        Ok(intent) => intent,
        Err(message) => return admin_bad_request(message),
    };
    match data_target_exists(&intent.target) {
        Ok(true) => {}
        Ok(false) => {
            return admin_bad_request(format!("data target '{}' was not found", intent.target))
        }
        Err(message) => return admin_error(message),
    }
    axum::Json(intent.result_view(default_context(), false)).into_response()
}

fn build_data_status_view(q: AdminDataStatusQuery) -> Result<DataStatusView, String> {
    let home = match home_dir() {
        Some(path) => path,
        None => return Err("cannot determine home directory".to_string()),
    };
    let data_root = home.join(".icelines").join("data");
    let store = match DataStore::open(&data_root) {
        Ok(store) => store,
        Err(err) => return Err(format!("open DataStore: {err}")),
    };
    let kind_filter = q.shard.as_deref().map(parse_kind).transpose()?;
    let rows = collect_data_status_rows(&store, kind_filter, q.stale_only);
    Ok(DataStatusView::from_entries(
        default_context(),
        data_root.display().to_string(),
        q.shard,
        q.stale_only,
        rows,
    ))
}

fn data_target_exists(target: &str) -> Result<bool, String> {
    let home = match home_dir() {
        Some(path) => path,
        None => return Err("cannot determine home directory".to_string()),
    };
    let store = DataStore::open(home.join(".icelines").join("data"))
        .map_err(|err| format!("open DataStore: {err}"))?;
    Ok(DataKind::all().iter().any(|kind| {
        store
            .manifest()
            .list(*kind)
            .iter()
            .any(|entry| short_key(&entry.key) == target)
    }))
}

fn build_snapshot_view(q: AdminSnapshotQuery) -> Result<SnapshotView, String> {
    let store = SnapshotStore::new(SnapshotStore::default_root());
    let manifest = match store.load_manifest() {
        Ok(manifest) => manifest,
        Err(err) => return Err(format!("loading snapshot manifest: {err}")),
    };
    Ok(SnapshotView::from_entries(
        default_context(),
        manifest.active,
        manifest
            .snapshots
            .into_iter()
            .map(|entry| SnapshotEntryInput {
                name: entry.name,
                season: entry.season,
                tier: format!("{:?}", entry.tier),
                date: entry.date,
                created_at: entry.created_at,
                parent_key: entry.parent_key,
                file_count: entry.file_count,
                sealed: entry.sealed,
            })
            .collect(),
        q.selected.as_deref(),
    ))
}

fn build_config_view(config: &crate::WebConfig, q: AdminConfigQuery) -> ConfigView {
    ConfigView::from_entries(
        default_context(),
        vec![
            ConfigEntryInput {
                key: "web.active_season".to_string(),
                value: config.active_season.clone(),
            },
            ConfigEntryInput {
                key: "web.active_season_type".to_string(),
                value: config.active_season_type.clone(),
            },
            ConfigEntryInput {
                key: "web.active_label".to_string(),
                value: config.active_label.clone(),
            },
        ],
        q.selected,
    )
}

fn apply_web_config_set(
    config: &mut crate::WebConfig,
    key: &str,
    value: &str,
) -> Result<bool, String> {
    match key {
        "web.active_season" => {
            validate_season(value)?;
            let changed = config.active_season != value
                || config.active_label != expected_label(value, &config.active_season_type);
            *config = crate::WebConfig::new(value, config.active_season_type.clone());
            Ok(changed)
        }
        "web.active_season_type" => {
            let value = normalize_season_type(value)?;
            let changed = config.active_season_type != value
                || config.active_label != expected_label(&config.active_season, &value);
            *config = crate::WebConfig::new(config.active_season.clone(), value);
            Ok(changed)
        }
        "web.active_label" => {
            Err("web.active_label is derived from season and season type".to_string())
        }
        other => Err(format!(
            "unknown web config key '{other}' - valid: web.active_season, web.active_season_type"
        )),
    }
}

fn apply_web_config_reset(config: &mut crate::WebConfig, key: &str) -> Result<bool, String> {
    let default = crate::WebConfig::default();
    match key {
        "web.active_season" => {
            let changed = config.active_season != default.active_season;
            *config =
                crate::WebConfig::new(default.active_season, config.active_season_type.clone());
            Ok(changed)
        }
        "web.active_season_type" => {
            let changed = config.active_season_type != default.active_season_type;
            *config =
                crate::WebConfig::new(config.active_season.clone(), default.active_season_type);
            Ok(changed)
        }
        "web.active_label" => {
            Err("web.active_label is derived from season and season type".to_string())
        }
        other => Err(format!(
            "unknown web config key '{other}' - valid: web.active_season, web.active_season_type"
        )),
    }
}

fn validate_season(value: &str) -> Result<(), String> {
    if value.len() == 8 && value.chars().all(|ch| ch.is_ascii_digit()) {
        Ok(())
    } else {
        Err("web.active_season must use YYYYZZZZ form, for example 20252026".to_string())
    }
}

fn normalize_season_type(value: &str) -> Result<String, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "regular" => Ok("regular".to_string()),
        "playoff" | "playoffs" => Ok("playoff".to_string()),
        other => Err(format!(
            "unknown season type '{other}' - valid: regular, playoff"
        )),
    }
}

fn expected_label(season: &str, season_type: &str) -> String {
    crate::WebConfig::new(season, season_type).active_label
}

fn collect_data_status_rows(
    store: &DataStore,
    kind_filter: Option<DataKind>,
    stale_only: bool,
) -> Vec<DataStatusEntryInput> {
    let clock = icelines_core::freshness::SystemClock;
    let kinds: Vec<DataKind> = match kind_filter {
        Some(kind) => vec![kind],
        None => DataKind::all().to_vec(),
    };
    let mut rows = Vec::new();
    for kind in kinds {
        for entry in store.manifest().list(kind) {
            if stale_only && !entry.freshness.is_stale(&clock) {
                continue;
            }
            rows.push(DataStatusEntryInput {
                source: entry.freshness.source,
                kind: format!("{kind:?}"),
                key: short_key(&entry.key),
                freshness: entry.freshness,
            });
        }
    }
    rows
}

fn parse_kind(value: &str) -> Result<DataKind, String> {
    Ok(match value.to_ascii_lowercase().as_str() {
        "bios" => DataKind::Bios,
        "stats" => DataKind::Stats,
        "goalie_stats" | "goalies" => DataKind::GoalieStats,
        "transactions" => DataKind::Transactions,
        "boxscore" | "boxscores" => DataKind::Boxscore,
        "career_history" | "career" => DataKind::CareerHistory,
        "schedule" => DataKind::Schedule,
        "score" | "scores" => DataKind::Score,
        "playoff_bracket" | "playoffs" => DataKind::PlayoffBracket,
        other => {
            return Err(format!(
                "unknown shard '{other}' - valid: bios, stats, goalie_stats, transactions, boxscore, career_history, schedule, score, playoff_bracket"
            ));
        }
    })
}

fn short_key(key: &DataKey) -> String {
    match key {
        DataKey::Season(season) => season.as_str(),
        DataKey::SeasonType(season, season_type) => {
            format!("{}/{}", season.as_str(), season_type.label())
        }
        DataKey::Game(game) => format!("game:{}", game.0),
        DataKey::Date(date) => date.clone(),
        DataKey::Player(player) => format!("player:{}", player.0),
        DataKey::Global => "<global>".to_string(),
    }
}

fn default_context() -> ViewContext {
    ViewContext::new(ViewWindow::new(
        Season(CURRENT_SEASON),
        icelines_core::season_stats::SeasonType::Regular,
    ))
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}

fn admin_error(message: impl Into<String>) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        axum::Json(AdminErrorResponse {
            error: message.into(),
        }),
    )
        .into_response()
}

fn admin_bad_request(message: impl Into<String>) -> Response {
    (
        StatusCode::BAD_REQUEST,
        axum::Json(AdminErrorResponse {
            error: message.into(),
        }),
    )
        .into_response()
}

fn admin_error_html(message: impl Into<String>) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Html(format!(
            "<!doctype html><html><body><h1>Admin unavailable</h1><p>{}</p></body></html>",
            html_escape(&message.into())
        )),
    )
        .into_response()
}

fn render_admin_html(
    active_label: &str,
    data_view: &DataStatusView,
    snapshot_view: &SnapshotView,
    config_view: &ConfigView,
) -> String {
    let mut html = String::new();
    html.push_str("<!doctype html><html><head><meta charset=\"utf-8\">");
    html.push_str("<title>IceLines Admin</title>");
    html.push_str("<link rel=\"stylesheet\" href=\"/static/style.css\">");
    html.push_str("</head><body><nav><a href=\"/\">League</a> - <strong>Admin</strong></nav>");
    html.push_str("<main>");
    html.push_str("<h1>Admin</h1>");
    html.push_str(&format!(
        "<p class=\"season-header\">{}</p>",
        html_escape(active_label)
    ));
    render_data_status_section(&mut html, data_view);
    render_snapshot_section(&mut html, snapshot_view);
    render_config_section(&mut html, config_view);
    html.push_str("</main></body></html>");
    html
}

fn render_data_status_section(html: &mut String, view: &DataStatusView) {
    html.push_str("<section><h2>Data Status</h2>");
    html.push_str(&format!(
        "<p>{} manifest entries at <code>{}</code></p>",
        view.total,
        html_escape(&view.root)
    ));
    if view.rows.is_empty() {
        render_empty_state(
            html,
            view.empty_state
                .as_ref()
                .map(|state| (&state.title, &state.detail)),
        );
        html.push_str("</section>");
        return;
    }
    html.push_str("<table><thead><tr><th>Source</th><th>Kind</th><th>Key</th><th>Freshness</th></tr></thead><tbody>");
    for row in &view.rows {
        html.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            html_escape(&row.source),
            html_escape(&row.kind),
            html_escape(&row.key),
            html_escape(&row.freshness)
        ));
    }
    html.push_str("</tbody></table></section>");
}

fn render_snapshot_section(html: &mut String, view: &SnapshotView) {
    html.push_str("<section><h2>Snapshots</h2>");
    html.push_str(&format!("<p>{} snapshot(s)</p>", view.total));
    if view.rows.is_empty() {
        render_empty_state(
            html,
            view.empty_state
                .as_ref()
                .map(|state| (&state.title, &state.detail)),
        );
        html.push_str("</section>");
        return;
    }
    html.push_str("<table><thead><tr><th>Name</th><th>Season</th><th>Tier</th><th>Date</th><th>Sealed</th><th>Files</th></tr></thead><tbody>");
    for row in &view.rows {
        let active = if row.is_active { " active" } else { "" };
        html.push_str(&format!(
            "<tr><td>{}{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            html_escape(&row.name),
            active,
            html_escape(&row.season),
            html_escape(&row.tier),
            html_escape(&row.date),
            html_escape(&row.sealed_label),
            row.file_count
        ));
    }
    html.push_str("</tbody></table></section>");
}

fn render_config_section(html: &mut String, view: &ConfigView) {
    html.push_str("<section><h2>Runtime Config</h2>");
    html.push_str("<table><thead><tr><th>Key</th><th>Value</th></tr></thead><tbody>");
    for row in &view.rows {
        html.push_str(&format!(
            "<tr><td>{}</td><td>{}</td></tr>",
            html_escape(&row.key),
            html_escape(&row.value)
        ));
    }
    html.push_str("</tbody></table></section>");
}

fn render_empty_state(html: &mut String, state: Option<(&String, &Option<String>)>) {
    let Some((title, detail)) = state else {
        html.push_str("<p>No rows.</p>");
        return;
    };
    html.push_str(&format!("<p>{}</p>", html_escape(title)));
    if let Some(detail) = detail {
        html.push_str(&format!("<p>{}</p>", html_escape(detail)));
    }
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
