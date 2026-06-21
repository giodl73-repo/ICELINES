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

pub(crate) enum GroupMutateOp {
    Create {
        name: String,
        description: String,
    },
    Rename {
        old_name: String,
        new_name: String,
    },
    Delete {
        name: String,
    },
    AddMember {
        group: String,
        key: String,
        kind_hint: Option<String>,
    },
    RemoveMember {
        group: String,
        key: String,
        kind_hint: Option<String>,
    },
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

#[derive(Debug, Clone)]
pub(crate) struct GroupOption {
    pub(crate) name: String,
    pub(crate) member_count: usize,
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
    pub(crate) group: String,
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

pub(crate) fn read_group_options() -> Vec<GroupOption> {
    let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) else {
        return default_group_options();
    };
    let db_path = std::path::PathBuf::from(&home)
        .join(".icelines")
        .join("icelines.db");
    if !db_path.exists() {
        return default_group_options();
    }
    let Ok(conn) = rusqlite::Connection::open(&db_path) else {
        return default_group_options();
    };
    let Ok(mut stmt) = conn.prepare(
        "SELECT g.name, COUNT(gm.entity_ref) \
         FROM groups g \
         LEFT JOIN group_members gm ON gm.group_name = g.name \
         GROUP BY g.name \
         ORDER BY CASE WHEN g.name = 'Favorites' THEN 0 ELSE 1 END, lower(g.name)",
    ) else {
        return default_group_options();
    };
    let groups: Vec<GroupOption> = stmt
        .query_map([], |r| {
            Ok(GroupOption {
                name: r.get(0)?,
                member_count: r.get::<_, i64>(1)?.max(0) as usize,
            })
        })
        .ok()
        .map(|rows| rows.filter_map(Result::ok).collect())
        .unwrap_or_default();
    if groups.is_empty() {
        default_group_options()
    } else {
        groups
    }
}

fn default_group_options() -> Vec<GroupOption> {
    vec![GroupOption {
        name: "Favorites".to_owned(),
        member_count: 0,
    }]
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

pub(crate) fn mutate_group(op: GroupMutateOp) -> Result<MutationResultView, String> {
    let conn = open_group_conn()?;
    let (operation, target, changed) = match op {
        GroupMutateOp::Create { name, description } => {
            let name = clean_group_name(&name)?;
            conn.execute(
                "INSERT INTO groups (name, description, created_at) VALUES (?1, ?2, datetime('now'))",
                rusqlite::params![name, description.trim()],
            )
            .map_err(|e| format!("create group: {e}"))?;
            ("group.create".to_owned(), name, true)
        }
        GroupMutateOp::Rename { old_name, new_name } => {
            let old_name = clean_group_name(&old_name)?;
            let new_name = clean_group_name(&new_name)?;
            if old_name == "Favorites" {
                return Err("Favorites cannot be renamed.".to_owned());
            }
            if old_name == new_name {
                ("group.rename".to_owned(), old_name, false)
            } else {
                require_group(&conn, &old_name)?;
                let exists: bool = conn
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM groups WHERE name = ?1)",
                        rusqlite::params![new_name],
                        |row| row.get(0),
                    )
                    .map_err(|e| format!("check group: {e}"))?;
                if exists {
                    return Err(format!("group '{new_name}' already exists"));
                }
                conn.execute(
                    "INSERT INTO groups (name, description, created_at) \
                     SELECT ?1, description, datetime('now') FROM groups WHERE name = ?2",
                    rusqlite::params![new_name, old_name],
                )
                .map_err(|e| format!("create renamed group: {e}"))?;
                conn.execute(
                    "UPDATE group_members SET group_name = ?1 WHERE group_name = ?2",
                    rusqlite::params![new_name, old_name],
                )
                .map_err(|e| format!("rename group members: {e}"))?;
                conn.execute(
                    "DELETE FROM groups WHERE name = ?1",
                    rusqlite::params![old_name],
                )
                .map_err(|e| format!("delete renamed group source: {e}"))?;
                ("group.rename".to_owned(), new_name, true)
            }
        }
        GroupMutateOp::Delete { name } => {
            let name = clean_group_name(&name)?;
            if name == "Favorites" {
                return Err("Favorites cannot be deleted.".to_owned());
            }
            conn.execute(
                "DELETE FROM group_members WHERE group_name = ?1",
                rusqlite::params![name],
            )
            .map_err(|e| format!("delete group members: {e}"))?;
            let rows = conn
                .execute(
                    "DELETE FROM groups WHERE name = ?1",
                    rusqlite::params![name],
                )
                .map_err(|e| format!("delete group: {e}"))?;
            ("group.delete".to_owned(), name, rows > 0)
        }
        GroupMutateOp::AddMember {
            group,
            key,
            kind_hint,
        } => {
            let group = clean_group_name(&group)?;
            require_group(&conn, &group)?;
            let entity_ref = resolve_entity_ref(&key, kind_hint.as_deref())?;
            let rows = conn
                .execute(
                    "INSERT OR IGNORE INTO group_members (group_name, entity_ref, added_at) \
                     VALUES (?1, ?2, datetime('now'))",
                    rusqlite::params![group, entity_ref],
                )
                .map_err(|e| format!("add group member: {e}"))?;
            ("group.member.add".to_owned(), entity_ref, rows > 0)
        }
        GroupMutateOp::RemoveMember {
            group,
            key,
            kind_hint,
        } => {
            let group = clean_group_name(&group)?;
            require_group(&conn, &group)?;
            let entity_ref = resolve_entity_ref(&key, kind_hint.as_deref())?;
            let rows = conn
                .execute(
                    "DELETE FROM group_members WHERE group_name = ?1 AND entity_ref = ?2",
                    rusqlite::params![group, entity_ref],
                )
                .map_err(|e| format!("remove group member: {e}"))?;
            ("group.member.remove".to_owned(), entity_ref, rows > 0)
        }
    };

    let context = ViewContext::new(ViewWindow::new(Season(CURRENT_SEASON), SeasonType::Regular));
    if changed {
        Ok(MutationResultView::applied(
            context,
            operation,
            target,
            "Group mutation applied",
            None,
        ))
    } else {
        Ok(MutationResultView::noop(
            context,
            operation,
            target,
            "No group mutation needed",
            None,
        ))
    }
}

fn open_group_conn() -> Result<rusqlite::Connection, String> {
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
         INSERT OR IGNORE INTO groups (name, description, created_at)
            VALUES ('Favorites', '', datetime('now'));
         CREATE TABLE IF NOT EXISTS group_members (
            group_name TEXT NOT NULL,
            entity_ref TEXT NOT NULL,
            added_at   TEXT NOT NULL,
            PRIMARY KEY (group_name, entity_ref),
            FOREIGN KEY (group_name) REFERENCES groups(name) ON DELETE CASCADE
         );",
    )
    .map_err(|e| format!("initialize group tables: {e}"))?;
    Ok(conn)
}

fn clean_group_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("group name is required".to_owned());
    }
    if name.len() > 80 {
        return Err("group name must be 80 characters or fewer".to_owned());
    }
    Ok(name.to_owned())
}

fn require_group(conn: &rusqlite::Connection, group: &str) -> Result<(), String> {
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM groups WHERE name = ?1)",
            rusqlite::params![group],
            |row| row.get(0),
        )
        .map_err(|e| format!("check group: {e}"))?;
    if exists {
        Ok(())
    } else {
        Err(format!("group '{group}' not found"))
    }
}

fn resolve_entity_ref(key: &str, kind_hint: Option<&str>) -> Result<String, String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("member key is required".to_owned());
    }
    match kind_hint.map(str::trim).filter(|s| !s.is_empty()) {
        Some("team") => {
            let abbr = icelines_core::TeamAbbr::parse(key).map_err(|e| e.to_string())?;
            Ok(format!("team:{}", abbr.0))
        }
        Some("player") => Ok(format!(
            "player:{}",
            icelines_core::name::normalize_name(key)
        )),
        Some(other) => Err(format!("unknown kind '{other}' - expected player or team")),
        None => match icelines_core::TeamAbbr::parse(key) {
            Ok(abbr) => Ok(format!("team:{}", abbr.0)),
            Err(_) => Ok(format!(
                "player:{}",
                icelines_core::name::normalize_name(key)
            )),
        },
    }
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
