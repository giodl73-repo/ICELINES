use std::{collections::HashMap, time::Duration};

use axum::http::{header, HeaderMap};
use icelines_core::{
    model::Season, season_stats::SeasonType, MutationResultView, ViewContext, ViewWindow,
    CURRENT_SEASON,
};

pub(crate) enum MutateOp {
    Add,
    Remove,
}

#[derive(Debug, Clone)]
pub(crate) struct WatchNote {
    pub(crate) reason: String,
    pub(crate) source: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct WatchAlertEvent {
    pub(crate) rule_id: String,
    pub(crate) entity_ref: Option<String>,
    pub(crate) message: String,
    pub(crate) fired_at: String,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct GroupApiResponse {
    pub(crate) schema_version: &'static str,
    pub(crate) route: &'static str,
    pub(crate) data: Vec<GroupApiRow>,
    pub(crate) meta: GroupApiMeta,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct GroupApiMeta {
    pub(crate) group: &'static str,
    pub(crate) count: usize,
    pub(crate) player_count: usize,
    pub(crate) team_count: usize,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct GroupApiRow {
    pub(crate) kind: String,
    pub(crate) key: String,
    pub(crate) stat_line: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct WatchlistApiResponse {
    pub(crate) schema_version: &'static str,
    pub(crate) route: &'static str,
    pub(crate) data: Vec<WatchlistApiRow>,
    pub(crate) alerts: Vec<WatchAlertEvent>,
    pub(crate) meta: WatchlistApiMeta,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct WatchlistApiMeta {
    pub(crate) group: &'static str,
    pub(crate) count: usize,
    pub(crate) player_count: usize,
    pub(crate) team_count: usize,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct WatchlistApiRow {
    pub(crate) kind: String,
    pub(crate) key: String,
    pub(crate) reason: Option<String>,
    pub(crate) source: Option<String>,
    pub(crate) updated_at: Option<String>,
}

pub(crate) fn group_api_rows_from_view(view: &icelines_core::FavoritesView) -> Vec<GroupApiRow> {
    view.rows
        .iter()
        .map(|row| GroupApiRow {
            kind: row.kind.clone(),
            key: row.key.clone(),
            stat_line: row.stat_line.clone(),
        })
        .collect()
}

pub(crate) fn watchlist_api_rows(view: &icelines_core::WatchlistView) -> Vec<WatchlistApiRow> {
    view.rows
        .iter()
        .map(|row| WatchlistApiRow {
            kind: row.kind.clone(),
            key: row.key.clone(),
            reason: row.reason.clone(),
            source: row.source.clone(),
            updated_at: row.updated_at.clone(),
        })
        .collect()
}

pub(crate) fn read_group_members(group_name: &str) -> Vec<(String, String)> {
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
                 WHERE group_name = ?1 \
                 ORDER BY entity_ref",
    ) else {
        return Vec::new();
    };
    stmt.query_map(rusqlite::params![group_name], |r| r.get::<_, String>(0))
        .ok()
        .map(|rows| {
            rows.filter_map(Result::ok)
                .map(|er| match er.split_once(':') {
                    Some(("team", k)) => ("team".into(), k.into()),
                    Some(("player", k)) => ("player".into(), k.into()),
                    _ => ("player".into(), er),
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn read_watch_notes() -> HashMap<String, WatchNote> {
    let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) else {
        return HashMap::new();
    };
    let db_path = std::path::PathBuf::from(&home)
        .join(".icelines")
        .join("icelines.db");
    if !db_path.exists() {
        return HashMap::new();
    }
    let Ok(conn) = rusqlite::Connection::open(&db_path) else {
        return HashMap::new();
    };
    let Ok(mut stmt) =
        conn.prepare("SELECT entity_ref, reason, source, updated_at FROM watch_notes")
    else {
        return HashMap::new();
    };
    stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            WatchNote {
                reason: r.get::<_, String>(1)?,
                source: r.get::<_, String>(2)?,
                updated_at: r.get::<_, String>(3)?,
            },
        ))
    })
    .ok()
    .map(|rows| rows.filter_map(Result::ok).collect())
    .unwrap_or_default()
}

pub(crate) fn read_watch_alert_events(limit: usize) -> Vec<WatchAlertEvent> {
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
        "SELECT rule_id, entity_ref, message, fired_at
         FROM watch_rule_events
         WHERE rule_id LIKE 'alert-%'
         ORDER BY fired_at DESC, id DESC
         LIMIT ?1",
    ) else {
        return Vec::new();
    };
    stmt.query_map(rusqlite::params![limit.max(1) as i64], |r| {
        Ok(WatchAlertEvent {
            rule_id: r.get(0)?,
            entity_ref: r.get(1)?,
            message: r.get(2)?,
            fired_at: r.get(3)?,
        })
    })
    .ok()
    .map(|rows| rows.filter_map(Result::ok).collect())
    .unwrap_or_default()
}

pub(crate) fn mutate_favorites(
    headers: &HeaderMap,
    key: &str,
    kind_hint: Option<&str>,
    return_to: Option<&str>,
    op: MutateOp,
) -> Result<MutationResultView, String> {
    let intent = icelines_core::FavoriteMutationIntent::resolve(
        key,
        kind_hint,
        return_to,
        referer_path(headers),
    )?;

    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .ok_or_else(|| "HOME / USERPROFILE not set.".to_owned())?;
    let dir = std::path::PathBuf::from(home).join(".icelines");
    std::fs::create_dir_all(&dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    let db_path = dir.join("icelines.db");
    let conn = rusqlite::Connection::open(&db_path).map_err(|e| format!("open db: {e}"))?;
    conn.busy_timeout(Duration::from_secs(5))
        .map_err(|e| format!("set db busy timeout: {e}"))?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS groups (
                    name        TEXT PRIMARY KEY,
                    description TEXT NOT NULL DEFAULT '',
                    created_at  TEXT NOT NULL
                 );
                 INSERT OR IGNORE INTO groups (name, description, created_at) \
                    VALUES ('Favorites', '', datetime('now'));
                 CREATE TABLE IF NOT EXISTS group_members (
                    group_name TEXT NOT NULL,
                    entity_ref TEXT NOT NULL,
                    added_at   TEXT NOT NULL,
                    PRIMARY KEY (group_name, entity_ref)
                  );",
    )
    .map_err(|e| format!("initialize favorites tables: {e}"))?;

    let result = match op {
        MutateOp::Add => conn
            .execute(
                "INSERT OR IGNORE INTO group_members \
                     (group_name, entity_ref, added_at) \
                     VALUES ('Favorites', ?1, datetime('now'))",
                rusqlite::params![intent.entity_ref],
            )
            .map(|rows| ("add".to_string(), rows)),
        MutateOp::Remove => conn
            .execute(
                "DELETE FROM group_members \
                     WHERE group_name = 'Favorites' AND entity_ref = ?1",
                rusqlite::params![intent.entity_ref],
            )
            .map(|rows| ("remove".to_string(), rows)),
    };
    let (operation, changed_rows) = result.map_err(|e| format!("db mutation: {e}"))?;
    let context = ViewContext::new(ViewWindow::new(Season(CURRENT_SEASON), SeasonType::Regular));

    Ok(intent.result_view(context, operation, changed_rows > 0))
}

fn referer_path(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::REFERER)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| {
            if let Some(rest) = s.strip_prefix("http://") {
                rest.find('/').map(|i| &rest[i..])
            } else if let Some(rest) = s.strip_prefix("https://") {
                rest.find('/').map(|i| &rest[i..])
            } else if s.starts_with('/') {
                Some(s)
            } else {
                None
            }
        })
}
