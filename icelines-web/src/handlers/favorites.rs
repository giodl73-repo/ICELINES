use super::favorites_data::{
    group_api_rows_from_view, mutate_favorites, read_group_members, read_group_options,
    read_watch_alert_events, read_watch_notes, watchlist_api_rows, GroupApiMeta, GroupApiResponse,
    MutateOp, WatchAlertEvent, WatchlistApiMeta, WatchlistApiResponse,
};
use crate::templates::{
    FavoriteGroupOptionRow, FavoritePlayerRow, FavoriteTeamRow, FavoritesTemplate,
    WatchRuleTemplateRow, WatchlistAlertRow, WatchlistPlayerRow, WatchlistTeamRow,
    WatchlistTemplate,
};
use askama::Template;
use axum::extract::{Form, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::{
    FavoriteMemberInput, FavoriteMemberRow, FavoritesView, ViewContext, ViewWindow, WatchNoteInput,
    WatchlistMemberRow, WatchlistView,
};
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct FavoritesQuery {
    group: Option<String>,
}

pub async fn get_favorites(
    State(state): State<crate::WebState>,
    Query(query): Query<FavoritesQuery>,
) -> Response {
    let group = selected_group(query.group.as_deref());
    let can_mutate = group == "Favorites";
    let members = read_group_members(&group);
    let (active_label, context) = match favorites_context(&state, "favorites").await {
        Ok(context) => context,
        Err(response) => return response,
    };

    // Phase Foster +21 — for each favorited player resolve to
    // a PlayerId and walk the persisted boxscore JSON to pull
    // tonight's stat line. Best-effort: missing bundle bios
    // → row drops to "no resolved pid"; missing boxscore →
    // dash row.
    let stat_lines = compute_player_stat_lines(&members).await;
    let view = FavoritesView::from_members(
        context,
        group.clone(),
        favorite_member_inputs(&members),
        stat_lines,
    );

    let tmpl = FavoritesTemplate {
        active_label,
        group: group.clone(),
        can_mutate,
        groups: favorite_group_options(&group),
        active_season: view.context.window.season.as_str(),
        active_season_type: view.context.window.season_type.label().to_string(),
        player_count: view.player_count,
        team_count: view.team_count,
        players: view
            .rows
            .iter()
            .filter(|row| row.kind == "player")
            .map(favorite_player_row)
            .collect(),
        teams: view
            .rows
            .iter()
            .filter(|row| row.kind == "team")
            .map(|row| FavoriteTeamRow {
                key: row.key.clone(),
            })
            .collect(),
    };

    render_template(tmpl)
}

pub async fn get_watchlist(State(state): State<crate::WebState>) -> Response {
    let members = read_group_members("Watchlist");
    let notes = read_watch_notes();
    let alerts = read_watch_alert_events(5);
    let (active_label, context) = match favorites_context(&state, "watchlist").await {
        Ok(context) => context,
        Err(response) => return response,
    };
    let view = WatchlistView::from_members(
        context,
        "Watchlist".to_string(),
        favorite_member_inputs(&members),
        watch_note_inputs(notes),
    );
    let tmpl = WatchlistTemplate {
        active_label,
        player_count: view.player_count,
        team_count: view.team_count,
        players: view
            .rows
            .iter()
            .filter(|row| row.kind == "player")
            .map(watchlist_player_row)
            .collect(),
        teams: view
            .rows
            .iter()
            .filter(|row| row.kind == "team")
            .map(|row| WatchlistTeamRow {
                key: row.key.clone(),
            })
            .collect(),
        rules: read_watch_rule_rows(),
        alerts: alerts.iter().map(watchlist_alert_row).collect(),
    };

    render_template(tmpl)
}

pub async fn get_favorites_json(Query(query): Query<FavoritesQuery>) -> Response {
    let group = selected_group(query.group.as_deref());
    let members = read_group_members(&group);
    let stat_lines = compute_player_stat_lines(&members).await;
    let view = FavoritesView::from_members(
        ViewContext::new(ViewWindow::new(
            Season(icelines_core::CURRENT_SEASON),
            icelines_core::season_stats::SeasonType::Regular,
        )),
        group.clone(),
        favorite_member_inputs(&members),
        stat_lines,
    );
    let rows = group_api_rows_from_view(&view);
    let meta = GroupApiMeta {
        group,
        count: view.rows.len(),
        player_count: view.player_count,
        team_count: view.team_count,
    };
    axum::Json(GroupApiResponse {
        schema_version: "favorites.v1",
        route: "favorites",
        data: rows,
        meta,
    })
    .into_response()
}

fn selected_group(group: Option<&str>) -> String {
    group
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or("Favorites")
        .to_owned()
}

fn favorite_group_options(active_group: &str) -> Vec<FavoriteGroupOptionRow> {
    let mut rows: Vec<FavoriteGroupOptionRow> = read_group_options()
        .into_iter()
        .map(|group| FavoriteGroupOptionRow {
            href: if group.name == "Favorites" {
                "/favorites".to_owned()
            } else {
                format!("/favorites?group={}", url_component(&group.name))
            },
            is_active: group.name == active_group,
            name: group.name,
            member_count: group.member_count,
        })
        .collect();

    if !rows.iter().any(|row| row.name == active_group) {
        rows.push(FavoriteGroupOptionRow {
            name: active_group.to_owned(),
            href: format!("/favorites?group={}", url_component(active_group)),
            member_count: 0,
            is_active: true,
        });
    }

    rows
}

fn url_component(raw: &str) -> String {
    let mut encoded = String::new();
    for byte in raw.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(char::from(byte));
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

async fn favorites_context(
    state: &crate::WebState,
    route: &'static str,
) -> Result<(String, ViewContext), Response> {
    let cfg = state.config.read().await;
    let season = cfg.active_season.parse::<u32>().map(Season).map_err(|_| {
        crate::api::json_error_meta(
            StatusCode::BAD_REQUEST,
            route,
            Vec::<FavoriteMemberInput>::new(),
            serde_json::json!({ "season": cfg.active_season }),
            format!("Season '{}' is not a valid YYYYZZZZ id", cfg.active_season),
        )
    })?;
    let season_type = SeasonType::parse_lossy(&cfg.active_season_type);
    Ok((
        cfg.active_label.clone(),
        ViewContext::new(ViewWindow::new(season, season_type)),
    ))
}

fn favorite_member_inputs(members: &[(String, String)]) -> Vec<FavoriteMemberInput> {
    members
        .iter()
        .map(|(kind, key)| FavoriteMemberInput {
            kind: kind.clone(),
            key: key.clone(),
        })
        .collect()
}

fn favorite_player_row(row: &FavoriteMemberRow) -> FavoritePlayerRow {
    let resolved = resolve_favorite_player(&row.key);
    FavoritePlayerRow {
        key: row.key.clone(),
        display_name: resolved
            .as_ref()
            .map(|candidate| candidate.full_name.clone())
            .unwrap_or_else(|| row.key.clone()),
        player_url: resolved
            .map(|candidate| format!("/player/{}", candidate.pid))
            .unwrap_or_default(),
        stat_line: row.stat_line.clone().unwrap_or_default(),
    }
}

fn resolve_favorite_player(key: &str) -> Option<icelines_fetch::stats_loader::PlayerCandidate> {
    let key = key.trim();
    if let Ok(pid) = key.parse::<u32>() {
        return icelines_fetch::stats_loader::find_player_candidate_by_id(pid);
    }

    let needle = icelines_core::name::normalize_name(key);
    let mut exact_matches: Vec<_> = icelines_fetch::stats_loader::find_player_candidates(key)
        .into_iter()
        .filter(|candidate| icelines_core::name::normalize_name(&candidate.full_name) == needle)
        .collect();
    if exact_matches.len() == 1 {
        exact_matches.pop()
    } else {
        None
    }
}

pub async fn get_watchlist_json() -> Response {
    let members = read_group_members("Watchlist");
    let notes = read_watch_notes();
    let alerts = read_watch_alert_events(10);
    let view = WatchlistView::from_members(
        ViewContext::new(ViewWindow::new(
            Season(icelines_core::CURRENT_SEASON),
            icelines_core::season_stats::SeasonType::Regular,
        )),
        "Watchlist".to_string(),
        favorite_member_inputs(&members),
        watch_note_inputs(notes),
    );
    let rows = watchlist_api_rows(&view);
    let meta = WatchlistApiMeta {
        group: "Watchlist",
        count: view.rows.len(),
        player_count: view.player_count,
        team_count: view.team_count,
    };
    axum::Json(WatchlistApiResponse {
        schema_version: "watchlist.v1",
        route: "watchlist",
        data: rows,
        alerts,
        meta,
    })
    .into_response()
}

fn watch_note_inputs(
    notes: std::collections::HashMap<String, super::favorites_data::WatchNote>,
) -> std::collections::HashMap<String, WatchNoteInput> {
    notes
        .into_iter()
        .map(|(key, note)| {
            (
                key,
                WatchNoteInput {
                    reason: note.reason,
                    source: note.source,
                    updated_at: note.updated_at,
                },
            )
        })
        .collect()
}

fn watchlist_player_row(row: &WatchlistMemberRow) -> WatchlistPlayerRow {
    let resolved = resolve_favorite_player(&row.key);
    WatchlistPlayerRow {
        key: row.key.clone(),
        display_name: resolved
            .as_ref()
            .map(|candidate| candidate.full_name.clone())
            .unwrap_or_else(|| row.key.clone()),
        player_url: resolved
            .map(|candidate| format!("/player/{}", candidate.pid))
            .unwrap_or_default(),
        reason: row.reason.clone().unwrap_or_default(),
    }
}

fn watchlist_alert_row(row: &WatchAlertEvent) -> WatchlistAlertRow {
    WatchlistAlertRow {
        fired_at: row.fired_at.clone(),
        rule_id: row.rule_id.clone(),
        entity: row.entity_ref.clone().unwrap_or_else(|| "-".to_string()),
        message: row.message.clone(),
    }
}

fn read_watch_rule_rows() -> Vec<WatchRuleTemplateRow> {
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
    let Ok(mut stmt) = conn.prepare("SELECT id, label, enabled FROM watch_rules ORDER BY id")
    else {
        return Vec::new();
    };
    stmt.query_map([], |row| {
        let id: String = row.get(0)?;
        let label: String = row.get(1)?;
        let enabled: i64 = row.get(2)?;
        let enabled = enabled != 0;
        Ok(WatchRuleTemplateRow {
            id,
            label,
            enabled,
            enabled_label: if enabled { "enabled" } else { "disabled" }.to_string(),
            next_enabled: !enabled,
            action_label: if enabled { "Disable" } else { "Enable" }.to_string(),
        })
    })
    .ok()
    .map(|rows| rows.filter_map(Result::ok).collect())
    .unwrap_or_default()
}

/// Per-favorited-player stat-line lookup. Returns a flat
/// vec of (display_name, formatted_line) pairs the renderer
/// drops in below the player's name. Empty when no boxscore
/// data is on disk yet — caller falls back to plain listing.
pub(super) async fn compute_player_stat_lines(
    members: &[(String, String)],
) -> std::collections::HashMap<String, String> {
    use std::collections::HashMap;
    let mut out = HashMap::new();

    // Today's slate fetch (best-effort).
    let client = icelines_fetch::nhl_api::NhlApiClient::production();
    let today = chrono::Utc::now()
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();
    let slate = match client.fetch_schedule_for_date(&today).await {
        Ok(g) => g
            .into_iter()
            .filter(|g| g.date == today)
            .collect::<Vec<_>>(),
        Err(_) => Vec::new(),
    };
    if slate.is_empty() {
        return out;
    }

    let home = match std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        Some(h) => std::path::PathBuf::from(h),
        None => return out,
    };
    let data_root = home.join(".icelines").join("data");
    let store = match icelines_fetch::datastore::DataStore::open(&data_root) {
        Ok(s) => s,
        Err(_) => return out,
    };

    for (kind, name) in members {
        if kind != "player" {
            continue;
        }
        let Some(pid) = icelines_fetch::stats_loader::resolve_player_id_by_name(name) else {
            continue;
        };
        // Find the player's team, then the day's game.
        let team = match player_team(pid) {
            Some(t) => t.to_uppercase(),
            None => continue,
        };
        let game = match slate.iter().find(|g| {
            g.away_abbrev.eq_ignore_ascii_case(&team) || g.home_abbrev.eq_ignore_ascii_case(&team)
        }) {
            Some(g) => g,
            None => continue,
        };
        let key =
            icelines_fetch::manifest::DataKey::Game(icelines_core::identity::GameId(game.game_id));
        // Foster +23 — lazy-fetch the boxscore body when it's
        // not on disk so users see real numbers without a
        // separate `icelines fetch boxscore` step. Persists
        // the body to the manifest as a side effect so the
        // TUI / CLI / next page-load all benefit. Failures
        // are non-fatal (drop to "no line").
        let raw_opt = match store.load_boxscore_raw(key.clone()) {
            Some(r) => Some(r),
            None => match client.fetch_boxscore_with_raw(game.game_id).await {
                Ok((_, raw_body)) => {
                    // Best-effort persist so subsequent renders
                    // don't re-hit the network. Same write
                    // pattern as `icelines fetch boxscore`.
                    let path = data_root
                        .join("boxscores")
                        .join(&today)
                        .join(format!("{}.json", game.game_id));
                    if let Ok(bytes) = serde_json::to_vec(&raw_body) {
                        let _ = icelines_fetch::atomic_write::write_bytes_atomic(&path, &bytes);
                        let _ = store.manifest().upsert(
                            icelines_fetch::manifest::DataKind::Boxscore,
                            icelines_fetch::manifest::ManifestEntry {
                                key: key.clone(),
                                path,
                                freshness: icelines_core::Freshness {
                                    fetched_at: chrono::Utc::now(),
                                    source: icelines_core::FetchSource::Live,
                                    ttl: icelines_core::Ttl::Static,
                                },
                            },
                        );
                    }
                    Some(raw_body)
                }
                Err(_) => None,
            },
        };
        let Some(raw) = raw_opt else { continue };
        let parsed = icelines_fetch::nhl_api::parse_boxscore(&raw, game.game_id);
        if let Some(line) =
            icelines_fetch::boxscore_to_night_line::extract_skater_line(&parsed, pid)
        {
            out.insert(name.clone(), format_skater_line_html(&line));
        }
    }
    out
}

fn player_team(pid: u32) -> Option<String> {
    for season in icelines_fetch::bundled::BUNDLED_SEASONS {
        if let Some(bios) = icelines_fetch::bundled::get_bios(season) {
            if let Some(b) = bios.iter().find(|b| b.player_id == pid) {
                if let Some(team) = &b.current_team_abbrev {
                    return Some(team.clone());
                }
            }
        }
    }
    None
}

fn format_skater_line_html(line: &icelines_core::favorites::SkaterNightLine) -> String {
    use icelines_core::favorites::{GameResult, HomeAway};
    let matchup = match line.home_or_away {
        HomeAway::Home => format!("{} vs {}", line.team.0, line.opponent.0),
        HomeAway::Away => format!("{} @ {}", line.team.0, line.opponent.0),
    };
    let result = match line.result {
        GameResult::Win => "W",
        GameResult::Loss => "L",
        GameResult::OtLoss => "OTL",
        GameResult::InProgress => "LIVE",
    };
    let toi = line
        .toi_seconds
        .map(|s| format!("{}:{:02}", s / 60, s % 60))
        .unwrap_or_else(|| "—".to_string());
    format!(
        "{} {}-{} {} · {}G {}A {}P · {:+} · TOI {} · {} SOG",
        matchup,
        line.team_score,
        line.opponent_score,
        result,
        line.goals,
        line.assists,
        line.points,
        line.plus_minus,
        toi,
        line.shots
            .map(|n| n.to_string())
            .unwrap_or_else(|| "—".into()),
    )
}

fn error_response(msg: &str) -> Response {
    let body = format!(
        "<!DOCTYPE html><html><body><h1>Favorites</h1><p>Error: {}</p></body></html>",
        html_escape(msg)
    );
    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        Html(body),
    )
        .into_response()
}

fn render_template<T: Template>(tmpl: T) -> Response {
    match tmpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!(
                "<!doctype html><body><h1>500</h1><p>{}</p></body>",
                html_escape(&e.to_string())
            )),
        )
            .into_response(),
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ── Foster +18 — POST handlers for add/remove ────────────────────

#[derive(Debug, Deserialize)]
pub struct FavoritesMutation {
    /// Free-text key — auto-detected as a team if it parses as
    /// a TeamAbbr, otherwise treated as a player name and
    /// normalized via `icelines_core::name::normalize_name`.
    /// Same auto-detect as the CLI `group add` path.
    pub key: String,
    /// Optional explicit kind override (`player` / `team`). When
    /// omitted, auto-detect runs.
    #[serde(default)]
    pub kind: Option<String>,
    /// Where to send the user after the mutation. Defaults to
    /// `/favorites`. Caller-supplied so each surface (team page,
    /// player card, favorites page itself) can route back to
    /// itself.
    #[serde(default)]
    pub return_to: Option<String>,
}

pub async fn post_add(headers: HeaderMap, Form(req): Form<FavoritesMutation>) -> Response {
    // Snapshot the resolved key + display name BEFORE mutate
    // so we can fire the career-history augment off in the
    // background after the redirect is queued. Augment is
    // best-effort + non-blocking from the user's POV — they
    // get the redirect immediately; the network call
    // completes off the request path.
    let display = req.key.trim().to_string();
    let kind_hint = req.kind.clone();
    let response = match mutate_favorites(
        &headers,
        &req.key,
        req.kind.as_deref(),
        req.return_to.as_deref(),
        MutateOp::Add,
    ) {
        Ok(view) => {
            Redirect::to(view.redirect_to.as_deref().unwrap_or("/favorites")).into_response()
        }
        Err(msg) => error_response(&msg),
    };
    // Foster +18 — opportunistic career-history augment for
    // newly-favorited players. Mirrors the CLI `group add`
    // behavior so favoriting from either surface populates
    // the local store identically. Skip on team adds.
    spawn_career_augment_if_player(display, kind_hint.as_deref());
    response
}

pub async fn post_remove(headers: HeaderMap, Form(req): Form<FavoritesMutation>) -> Response {
    match mutate_favorites(
        &headers,
        &req.key,
        req.kind.as_deref(),
        req.return_to.as_deref(),
        MutateOp::Remove,
    ) {
        Ok(view) => {
            Redirect::to(view.redirect_to.as_deref().unwrap_or("/favorites")).into_response()
        }
        Err(msg) => error_response(&msg),
    }
}

pub async fn post_add_json(
    headers: HeaderMap,
    axum::Json(req): axum::Json<FavoritesMutation>,
) -> Response {
    let display = req.key.trim().to_string();
    let kind_hint = req.kind.clone();
    match mutate_favorites(
        &headers,
        &req.key,
        req.kind.as_deref(),
        req.return_to.as_deref(),
        MutateOp::Add,
    ) {
        Ok(view) => {
            spawn_career_augment_if_player(display, kind_hint.as_deref());
            axum::Json(view).into_response()
        }
        Err(msg) => json_error_response(&msg),
    }
}

pub async fn post_remove_json(
    headers: HeaderMap,
    axum::Json(req): axum::Json<FavoritesMutation>,
) -> Response {
    match mutate_favorites(
        &headers,
        &req.key,
        req.kind.as_deref(),
        req.return_to.as_deref(),
        MutateOp::Remove,
    ) {
        Ok(view) => axum::Json(view).into_response(),
        Err(msg) => json_error_response(&msg),
    }
}

fn spawn_career_augment_if_player(display: String, kind_hint: Option<&str>) {
    let is_player = match kind_hint {
        Some("team") => false,
        Some("player") => true,
        _ => icelines_core::TeamAbbr::parse(&display).is_err(),
    };
    if is_player && !display.is_empty() {
        let normalized = icelines_core::name::normalize_name(&display);
        tokio::spawn(async move {
            icelines_fetch::career_landing::augment_career_history_for_player(
                &display,
                &normalized,
                true,
            )
            .await;
        });
    }
}

fn json_error_response(message: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        axum::Json(serde_json::json!({ "error": message })),
    )
        .into_response()
}
