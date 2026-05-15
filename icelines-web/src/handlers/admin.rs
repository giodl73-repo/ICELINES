use std::path::PathBuf;

use axum::extract::{Form, Json, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use icelines_core::{
    ConfigEntryInput, ConfigMutationIntent, ConfigView, DataMutationIntent, DataMutationOperation,
    DataStatusEntryInput, DataStatusView, Season, SnapshotEntryInput, SnapshotMutationIntent,
    SnapshotMutationOperation, SnapshotView, ViewContext, ViewWarning, ViewWindow, WarningKind,
    CURRENT_SEASON,
};
use icelines_fetch::datastore::DataStore;
use icelines_fetch::game_cache::{
    FavoriteGameCacheLoadRequest, GameCacheArtifact, GameCacheLoadRequest,
};
use icelines_fetch::manifest::{DataKey, DataKind};
use icelines_fetch::snapshot::SnapshotStore;
use serde::Deserialize;

use super::favorites_data::read_group_members;
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

#[derive(Debug, Deserialize)]
pub struct AdminGameCacheLoadRequest {
    pub season: String,
    pub season_type: String,
    pub teams: String,
    pub artifacts: String,
    #[serde(default)]
    pub return_to: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AdminFavoritesGameCacheLoadRequest {
    pub season: String,
    pub season_type: String,
    pub artifacts: String,
    #[serde(default)]
    pub return_to: Option<String>,
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
        &config.active_season,
        &config.active_season_type,
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

pub async fn post_config_set_form(
    State(state): State<WebState>,
    Form(req): Form<AdminConfigMutationRequest>,
) -> Response {
    let Some(value) = req.value.as_deref() else {
        return admin_bad_request_html("config value is required");
    };
    let intent = match ConfigMutationIntent::set(&req.key, value) {
        Ok(intent) => intent,
        Err(message) => return admin_bad_request_html(message),
    };
    let mut config = state.config.write().await;
    match apply_web_config_set(&mut config, &intent.key, value) {
        Ok(changed) => {
            let _result = intent.result_view(default_context(), changed);
            Redirect::to("/admin").into_response()
        }
        Err(message) => admin_bad_request_html(message),
    }
}

pub async fn post_config_reset_form(
    State(state): State<WebState>,
    Form(req): Form<AdminConfigMutationRequest>,
) -> Response {
    let intent = match ConfigMutationIntent::reset(&req.key) {
        Ok(intent) => intent,
        Err(message) => return admin_bad_request_html(message),
    };
    let mut config = state.config.write().await;
    match apply_web_config_reset(&mut config, &intent.key) {
        Ok(changed) => {
            let _result = intent.result_view(default_context(), changed);
            Redirect::to("/admin").into_response()
        }
        Err(message) => admin_bad_request_html(message),
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

pub async fn post_snapshot_activate_form(
    Form(req): Form<AdminSnapshotMutationRequest>,
) -> Response {
    let intent =
        match SnapshotMutationIntent::resolve(SnapshotMutationOperation::Activate, req.name) {
            Ok(intent) => intent,
            Err(message) => return admin_bad_request_html(message),
        };
    let store = SnapshotStore::new(SnapshotStore::default_root());
    let before = match store.load_manifest() {
        Ok(manifest) => manifest.active,
        Err(err) => return admin_error_html(format!("loading snapshot manifest: {err}")),
    };
    match store.set_active(&intent.name) {
        Ok(()) => {
            let changed = before.as_deref() != Some(intent.name.as_str());
            let _result = intent.result_view(default_context(), changed);
            Redirect::to("/admin").into_response()
        }
        Err(err) => admin_bad_request_html(format!("activating snapshot '{}': {err}", intent.name)),
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

pub async fn post_snapshot_delete_form(Form(req): Form<AdminSnapshotMutationRequest>) -> Response {
    let intent = match SnapshotMutationIntent::resolve(SnapshotMutationOperation::Remove, req.name)
    {
        Ok(intent) => intent,
        Err(message) => return admin_bad_request_html(message),
    };
    let store = SnapshotStore::new(SnapshotStore::default_root());
    match store.delete(&intent.name) {
        Ok(()) => {
            let _result = intent.result_view(default_context(), true);
            Redirect::to("/admin").into_response()
        }
        Err(err) => admin_bad_request_html(format!("deleting snapshot '{}': {err}", intent.name)),
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

pub async fn post_data_verify_form(Form(req): Form<AdminDataVerifyRequest>) -> Response {
    let intent = match DataMutationIntent::resolve(DataMutationOperation::Verify, req.target, false)
    {
        Ok(intent) => intent,
        Err(message) => return admin_bad_request_html(message),
    };
    match data_target_exists(&intent.target) {
        Ok(true) => {
            let _result = intent.result_view(default_context(), false);
            Redirect::to("/admin").into_response()
        }
        Ok(false) => {
            admin_bad_request_html(format!("data target '{}' was not found", intent.target))
        }
        Err(message) => admin_error_html(message),
    }
}

pub async fn post_game_cache_load_json(Json(req): Json<AdminGameCacheLoadRequest>) -> Response {
    match load_game_cache(req).await {
        Ok(summary) => axum::Json(summary).into_response(),
        Err(message) => admin_bad_request(message),
    }
}

pub async fn post_game_cache_load_form(Form(req): Form<AdminGameCacheLoadRequest>) -> Response {
    let return_to = safe_return_to(req.return_to.as_deref())
        .unwrap_or("/admin")
        .to_string();
    match load_game_cache(req).await {
        Ok(_summary) => Redirect::to(&return_to).into_response(),
        Err(message) => admin_bad_request_html(message),
    }
}

pub async fn post_favorites_game_cache_load_json(
    Json(req): Json<AdminFavoritesGameCacheLoadRequest>,
) -> Response {
    match load_favorites_game_cache(req).await {
        Ok(summary) => axum::Json(summary).into_response(),
        Err(message) => admin_bad_request(message),
    }
}

pub async fn post_favorites_game_cache_load_form(
    Form(req): Form<AdminFavoritesGameCacheLoadRequest>,
) -> Response {
    let return_to = safe_return_to(req.return_to.as_deref())
        .unwrap_or("/admin")
        .to_string();
    match load_favorites_game_cache(req).await {
        Ok(_summary) => Redirect::to(&return_to).into_response(),
        Err(message) => admin_bad_request_html(message),
    }
}

async fn load_game_cache(
    req: AdminGameCacheLoadRequest,
) -> Result<icelines_fetch::game_cache::GameCacheLoadSummary, String> {
    let season_value = req
        .season
        .parse::<u32>()
        .map_err(|_| format!("season '{}' is not a valid YYYYZZZZ id", req.season))?;
    let season = Season::try_new(season_value).map_err(|err| err.to_string())?;
    let season_type = normalize_season_type(&req.season_type)?;
    let artifacts = GameCacheArtifact::parse_list(&req.artifacts)?;
    let teams = req
        .teams
        .split(',')
        .map(str::trim)
        .filter(|team| !team.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if teams.is_empty() {
        return Err("at least one team is required".to_string());
    }
    let data_root = home_dir()
        .ok_or_else(|| "cannot determine home directory".to_string())?
        .join(".icelines")
        .join("data");
    let summary = icelines_fetch::game_cache::ensure_team_game_cache(
        &data_root,
        GameCacheLoadRequest {
            season,
            season_type: icelines_core::season_stats::SeasonType::parse_lossy(&season_type),
            teams,
            artifacts,
        },
    )
    .await
    .map_err(|err| err.to_string())?;
    if summary.scheduled_games == 0 && !summary.errors.is_empty() {
        return Err(summary.errors.join("; "));
    }
    if summary.final_games > 0
        && summary.cached_artifacts == 0
        && summary.fetched_artifacts == 0
        && summary.failed_artifacts > 0
    {
        return Err(summary.errors.join("; "));
    }
    Ok(summary)
}

async fn load_favorites_game_cache(
    req: AdminFavoritesGameCacheLoadRequest,
) -> Result<icelines_fetch::game_cache::FavoriteGameCacheLoadSummary, String> {
    let season_value = req
        .season
        .parse::<u32>()
        .map_err(|_| format!("season '{}' is not a valid YYYYZZZZ id", req.season))?;
    let season = Season::try_new(season_value).map_err(|err| err.to_string())?;
    let season_type = normalize_season_type(&req.season_type)?;
    let artifacts = GameCacheArtifact::parse_list(&req.artifacts)?;
    let mut unresolved_players = Vec::new();
    let mut player_ids = Vec::new();
    let mut teams = Vec::new();
    for (kind, key) in read_group_members("Favorites") {
        match kind.as_str() {
            "team" => teams.push(key),
            "player" => match resolve_favorite_player_id(&key) {
                Some(pid) => player_ids.push(pid),
                None => unresolved_players.push(key),
            },
            _ => {}
        }
    }

    let data_root = home_dir()
        .ok_or_else(|| "cannot determine home directory".to_string())?
        .join(".icelines")
        .join("data");
    let mut summary = icelines_fetch::game_cache::ensure_favorites_game_cache(
        &data_root,
        FavoriteGameCacheLoadRequest {
            season,
            season_type: icelines_core::season_stats::SeasonType::parse_lossy(&season_type),
            player_ids,
            teams,
            artifacts,
        },
    )
    .await
    .map_err(|err| err.to_string())?;
    for player in unresolved_players {
        summary
            .errors
            .push(format!("favorite player '{player}' could not be resolved"));
    }
    Ok(summary)
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
    let mut view = ConfigView::from_entries(
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
    );
    view.warnings.push(persistent_report_toggle_warning());
    view
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

fn safe_return_to(value: Option<&str>) -> Option<&str> {
    let value = value?;
    if value.starts_with('/') && !value.starts_with("//") {
        Some(value)
    } else {
        None
    }
}

fn resolve_favorite_player_id(key: &str) -> Option<u32> {
    let key = key.trim();
    if let Ok(pid) = key.parse::<u32>() {
        return icelines_fetch::stats_loader::find_player_candidate_by_id(pid)
            .map(|candidate| candidate.pid);
    }
    icelines_fetch::stats_loader::resolve_player_id_by_name(key)
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

fn admin_bad_request_html(message: impl Into<String>) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Html(format!(
            "<!doctype html><html><body><h1>Admin request rejected</h1><p>{}</p><p><a href=\"/admin\">Back to admin</a></p></body></html>",
            html_escape(&message.into())
        )),
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
    active_season: &str,
    active_season_type: &str,
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
    render_game_cache_section(&mut html, active_season, active_season_type);
    render_snapshot_section(&mut html, snapshot_view);
    render_config_section(&mut html, config_view);
    html.push_str("</main></body></html>");
    html
}

fn render_game_cache_section(html: &mut String, active_season: &str, active_season_type: &str) {
    html.push_str("<section><h2>Game Cache</h2>");
    html.push_str("<p>POST-backed cache warmers for records, streaks, scoring events, and matchup pages. These may fetch official NHL game data for the requested teams/favorites, but they do not install release data bundles or remove local data.</p>");
    html.push_str("<form method=\"post\" action=\"/admin/game-cache/load-favorites\">");
    html.push_str(&format!(
        "<input type=\"hidden\" name=\"season\" value=\"{}\">",
        html_escape(active_season)
    ));
    html.push_str(&format!(
        "<input type=\"hidden\" name=\"season_type\" value=\"{}\">",
        html_escape(active_season_type)
    ));
    html.push_str("<input type=\"hidden\" name=\"artifacts\" value=\"boxscore,play-by-play\">");
    html.push_str("<input type=\"hidden\" name=\"return_to\" value=\"/admin\">");
    html.push_str("<button type=\"submit\">Load Favorites cache</button>");
    html.push_str("<span class=\"muted\"> Favorite players: career teams/seasons. Favorite teams: active year.</span>");
    html.push_str("</form>");
    html.push_str("<form method=\"post\" action=\"/admin/game-cache/load\">");
    html.push_str(&format!(
        "<input type=\"hidden\" name=\"season\" value=\"{}\">",
        html_escape(active_season)
    ));
    html.push_str(&format!(
        "<input type=\"hidden\" name=\"season_type\" value=\"{}\">",
        html_escape(active_season_type)
    ));
    html.push_str("<label>Teams <input name=\"teams\" placeholder=\"EDM,BOS\" required aria-label=\"Teams to load, comma-separated\"></label> ");
    html.push_str("<label>Artifacts <select name=\"artifacts\"><option value=\"boxscore\">Game lines</option><option value=\"scoring-events\">Scoring events / play-by-play</option><option value=\"boxscore,scoring-events\">Both</option></select></label> ");
    html.push_str("<input type=\"hidden\" name=\"return_to\" value=\"/admin\">");
    html.push_str("<button type=\"submit\">Load active-season game cache</button>");
    html.push_str("</form></section>");
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
        render_data_install_remove_deferral(html);
        html.push_str("</section>");
        return;
    }
    html.push_str("<table><thead><tr><th>Source</th><th>Kind</th><th>Key</th><th>Freshness</th><th>Action</th></tr></thead><tbody>");
    for row in &view.rows {
        html.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td><form method=\"post\" action=\"/admin/data/verify\"><input type=\"hidden\" name=\"target\" value=\"{}\"><button type=\"submit\">Verify</button></form></td></tr>",
            html_escape(&row.source),
            html_escape(&row.kind),
            html_escape(&row.key),
            html_escape(&row.freshness),
            html_escape(&row.key)
        ));
    }
    html.push_str("</tbody></table>");
    render_data_install_remove_deferral(html);
    html.push_str("</section>");
}

fn render_data_install_remove_deferral(html: &mut String) {
    html.push_str("<div class=\"muted\">");
    html.push_str("Web data install is deferred because it performs live/network release downloads; use <code>icelines data install</code> from the CLI when you intentionally want that operation. ");
    html.push_str("Web data remove is deferred because it is destructive filesystem mutation and needs a scoped confirmation contract.");
    html.push_str("</div>");
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
    html.push_str("<table><thead><tr><th>Name</th><th>Season</th><th>Tier</th><th>Date</th><th>Sealed</th><th>Files</th><th>Action</th></tr></thead><tbody>");
    for row in &view.rows {
        let active = if row.is_active { " active" } else { "" };
        html.push_str(&format!(
            "<tr><td>{}{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>",
            html_escape(&row.name),
            active,
            html_escape(&row.season),
            html_escape(&row.tier),
            html_escape(&row.date),
            html_escape(&row.sealed_label),
            row.file_count
        ));
        if row.is_active {
            html.push_str("<span class=\"muted\">Active</span>");
        } else {
            html.push_str(&format!(
                "<form method=\"post\" action=\"/admin/snapshots/delete\"><input type=\"hidden\" name=\"name\" value=\"{}\"><button type=\"submit\">Delete</button></form>",
                html_escape(&row.name)
            ));
        }
        if !row.is_active && row.sealed {
            html.push_str(&format!(
                "<form method=\"post\" action=\"/admin/snapshots/activate\"><input type=\"hidden\" name=\"name\" value=\"{}\"><button type=\"submit\">Activate</button></form>",
                html_escape(&row.name)
            ));
        } else if !row.is_active {
            html.push_str("<span class=\"muted\">Seal before activate</span>");
        }
        html.push_str("</td></tr>");
    }
    html.push_str("</tbody></table></section>");
}

fn render_config_section(html: &mut String, view: &ConfigView) {
    html.push_str("<section><h2>Runtime Web Config</h2>");
    html.push_str("<p class=\"muted\">These controls change only the running web server's active season context. They do not write <code>~/.icelines/config.toml</code>.</p>");
    for warning in &view.warnings {
        html.push_str(&format!(
            "<p class=\"muted\">{}</p>",
            html_escape(&warning.message)
        ));
    }
    html.push_str(
        "<table><thead><tr><th>Key</th><th>Value</th><th>Action</th></tr></thead><tbody>",
    );
    for row in &view.rows {
        html.push_str("<tr>");
        html.push_str(&format!(
            "<td>{}</td><td>{}</td>",
            html_escape(&row.key),
            html_escape(&row.value)
        ));
        html.push_str("<td>");
        if row.key == "web.active_season" || row.key == "web.active_season_type" {
            html.push_str(&format!(
                "<form method=\"post\" action=\"/admin/config/set\"><input type=\"hidden\" name=\"key\" value=\"{}\"><input name=\"value\" value=\"{}\" aria-label=\"{} value\"><button type=\"submit\">Set</button></form>",
                html_escape(&row.key),
                html_escape(&row.value),
                html_escape(&row.key)
            ));
            html.push_str(&format!(
                "<form method=\"post\" action=\"/admin/config/reset\"><input type=\"hidden\" name=\"key\" value=\"{}\"><button type=\"submit\">Reset</button></form>",
                html_escape(&row.key)
            ));
        } else {
            html.push_str("<span class=\"muted\">Derived</span>");
        }
        html.push_str("</td></tr>");
    }
    html.push_str("</tbody></table></section>");
    render_report_toggle_deferral(html);
}

fn render_report_toggle_deferral(html: &mut String) {
    html.push_str("<section><h2>Persistent Report Toggles</h2>");
    html.push_str("<p class=\"muted\">Persistent Tier-1 report toggles are managed by the TUI Reports overlay (<kbd>R</kbd>) and saved to <code>~/.icelines/config.toml</code>. Web admin does not expose report-toggle writes yet because the durable config contract currently lives outside the web crate.</p>");
    html.push_str("<p class=\"muted\">Use <code>icelines tui</code> then press <kbd>R</kbd> to change which realtime, time-on-ice, goals-for/against, goalie advanced, and goalie saves-by-strength columns are visible.</p>");
    html.push_str("</section>");
}

fn persistent_report_toggle_warning() -> ViewWarning {
    ViewWarning {
        kind: WarningKind::RendererProjection,
        source: None,
        message: "Persistent report toggles are deferred on web admin; use the TUI Reports overlay (R) to write ~/.icelines/config.toml.".to_string(),
        recovery: Vec::new(),
    }
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
