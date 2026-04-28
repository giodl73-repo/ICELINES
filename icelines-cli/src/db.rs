//! SQLite-backed group store for `icelines group` commands.
//!
//! Opens (or creates) `~/.icelines/icelines.db` and runs embedded migrations
//! on every startup.  Use `GroupDb::open_in_memory()` in unit tests.

use anyhow::{bail, Context};
use rusqlite::Connection;

// ── Public types ──────────────────────────────────────────────────────────────

/// A row returned by `list_groups`.
pub struct GroupRow {
    pub name: String,
    pub description: String,
    pub member_count: usize,
}

/// Opaque handle to the group database.
pub struct GroupDb {
    conn: Connection,
}

// ── Migrations ────────────────────────────────────────────────────────────────

fn run_migrations(conn: &Connection) -> anyhow::Result<()> {
    // Migration 001 — groups table
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS groups (
            name        TEXT PRIMARY KEY,
            description TEXT NOT NULL DEFAULT '',
            created_at  TEXT NOT NULL
        );",
    )
    .context("migration 001: create groups table")?;

    // Migration 002 — group_members table with cascade delete
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS group_members (
            group_name        TEXT NOT NULL,
            player_normalized TEXT NOT NULL,
            added_at          TEXT NOT NULL,
            PRIMARY KEY (group_name, player_normalized),
            FOREIGN KEY (group_name) REFERENCES groups(name) ON DELETE CASCADE
        );",
    )
    .context("migration 002: create group_members table")?;

    // Enable foreign-key enforcement (off by default in rusqlite).
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .context("enable foreign keys")?;

    Ok(())
}

// ── GroupDb impl ──────────────────────────────────────────────────────────────

impl GroupDb {
    /// Open (or create) `~/.icelines/icelines.db` and run migrations.
    pub fn open() -> anyhow::Result<Self> {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(std::path::PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;

        let dir = home.join(".icelines");
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("create {}", dir.display()))?;

        let db_path = dir.join("icelines.db");
        let conn = Connection::open(&db_path)
            .with_context(|| format!("open {}", db_path.display()))?;

        // Enable WAL for better concurrent write performance.
        conn.execute_batch("PRAGMA journal_mode = WAL;")
            .context("set WAL mode")?;

        run_migrations(&conn)?;
        Ok(Self { conn })
    }

    /// Open an in-memory database for unit tests.
    #[cfg(test)]
    pub fn open_in_memory() -> anyhow::Result<Self> {
        let conn = Connection::open_in_memory().context("open in-memory db")?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .context("enable foreign keys")?;
        run_migrations(&conn)?;
        Ok(Self { conn })
    }

    // ── Write operations ──────────────────────────────────────────────────────

    /// Create a new group.  Fails if the name already exists.
    pub fn create_group(&self, name: &str, desc: &str) -> anyhow::Result<()> {
        let now = now_utc();
        self.conn
            .execute(
                "INSERT INTO groups (name, description, created_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![name, desc, now],
            )
            .with_context(|| format!("create group '{name}'"))?;
        Ok(())
    }

    /// Delete a group and all its members (cascade).
    /// Returns `true` if the group existed, `false` if it was not found.
    pub fn delete_group(&self, name: &str) -> anyhow::Result<bool> {
        let rows = self
            .conn
            .execute("DELETE FROM groups WHERE name = ?1", rusqlite::params![name])
            .with_context(|| format!("delete group '{name}'"))?;
        Ok(rows > 0)
    }

    /// Add a player (normalized name) to a group.
    /// Returns `true` if added, `false` if already a member (no-op).
    pub fn add_member(&self, group: &str, player_normalized: &str) -> anyhow::Result<bool> {
        // Verify the group exists first so we give a clear error.
        self.require_group(group)?;

        let now = now_utc();
        let rows = self.conn
            .execute(
                "INSERT OR IGNORE INTO group_members \
                 (group_name, player_normalized, added_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![group, player_normalized, now],
            )
            .with_context(|| format!("add member '{player_normalized}' to '{group}'"))?;
        Ok(rows > 0)
    }

    /// Remove a player from a group.  No-op if the player is not a member.
    pub fn remove_member(&self, group: &str, player_normalized: &str) -> anyhow::Result<()> {
        self.require_group(group)?;

        self.conn
            .execute(
                "DELETE FROM group_members WHERE group_name = ?1 AND player_normalized = ?2",
                rusqlite::params![group, player_normalized],
            )
            .with_context(|| format!("remove member '{player_normalized}' from '{group}'"))?;
        Ok(())
    }

    // ── Read operations ───────────────────────────────────────────────────────

    /// List all groups with their member counts.
    pub fn list_groups(&self) -> anyhow::Result<Vec<GroupRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT g.name, g.description, COUNT(m.player_normalized) AS member_count
             FROM groups g
             LEFT JOIN group_members m ON m.group_name = g.name
             GROUP BY g.name
             ORDER BY g.name",
        )?;

        let rows = stmt
            .query_map([], |row| {
                Ok(GroupRow {
                    name: row.get(0)?,
                    description: row.get(1)?,
                    member_count: row.get::<_, i64>(2)? as usize,
                })
            })
            .context("list_groups query")?
            .collect::<Result<Vec<_>, _>>()
            .context("list_groups collect")?;

        Ok(rows)
    }

    /// List all members of a group (normalized names).
    pub fn list_members(&self, group: &str) -> anyhow::Result<Vec<String>> {
        self.require_group(group)?;

        let mut stmt = self.conn.prepare(
            "SELECT player_normalized FROM group_members \
             WHERE group_name = ?1 ORDER BY added_at",
        )?;

        let rows = stmt
            .query_map(rusqlite::params![group], |row| row.get(0))
            .context("list_members query")?
            .collect::<Result<Vec<String>, _>>()
            .context("list_members collect")?;

        Ok(rows)
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    fn require_group(&self, name: &str) -> anyhow::Result<()> {
        let exists: bool = self
            .conn
            .query_row(
                "SELECT 1 FROM groups WHERE name = ?1",
                rusqlite::params![name],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if !exists {
            bail!("group '{name}' not found");
        }
        Ok(())
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn now_utc() -> String {
    // chrono is not in the CLI deps, so use std time + manual formatting.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // ISO-8601-ish UTC timestamp: "2025-04-25T00:00:00Z"
    let (mut rem, s) = (secs, secs % 60);
    rem /= 60;
    let m = rem % 60;
    rem /= 60;
    let h = rem % 24;
    let days = rem / 24;
    // Days since Unix epoch → calendar date (Gregorian proleptic)
    let (y, mo, d) = days_to_ymd(days);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

/// Convert days-since-Unix-epoch to (year, month, day).
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Algorithm from https://www.researchgate.net/publication/316558298
    let z = days + 719468;
    let era = z / 146097;
    let doe = z % 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

// ── Unit tests (L1) ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l1_db_create_and_list_group() {
        let db = GroupDb::open_in_memory().expect("open in-memory db");
        db.create_group("myteam", "My fantasy team").expect("create group");

        let groups = db.list_groups().expect("list groups");
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "myteam");
        assert_eq!(groups[0].description, "My fantasy team");
        assert_eq!(groups[0].member_count, 0);
    }

    #[test]
    fn l1_db_add_remove_member() {
        let db = GroupDb::open_in_memory().expect("open in-memory db");
        db.create_group("g1", "").expect("create group");

        db.add_member("g1", "connor_mcdavid").expect("add member");
        db.add_member("g1", "leon_draisaitl").expect("add member");

        let members = db.list_members("g1").expect("list members");
        assert_eq!(members.len(), 2);
        assert!(members.contains(&"connor_mcdavid".to_owned()));

        db.remove_member("g1", "leon_draisaitl").expect("remove member");
        let members = db.list_members("g1").expect("list members after remove");
        assert_eq!(members.len(), 1);
        assert_eq!(members[0], "connor_mcdavid");
    }

    #[test]
    fn l1_db_delete_group_cascades_members() {
        let db = GroupDb::open_in_memory().expect("open in-memory db");
        db.create_group("watchlist", "").expect("create group");
        db.add_member("watchlist", "player_one").expect("add member");
        db.add_member("watchlist", "player_two").expect("add member");

        let deleted = db.delete_group("watchlist").expect("delete group");
        assert!(deleted, "delete should return true when group existed");

        // The group is gone.
        let groups = db.list_groups().expect("list groups");
        assert!(groups.is_empty(), "no groups should remain");

        // Members were cascaded — confirm the member table is empty.
        let count: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM group_members", [], |r| r.get(0))
            .expect("count");
        assert_eq!(count, 0, "members should have been cascade-deleted");
    }

    #[test]
    fn l1_db_duplicate_member_is_noop() {
        let db = GroupDb::open_in_memory().expect("open in-memory db");
        db.create_group("dups", "").expect("create group");

        db.add_member("dups", "nathan_mackinnon").expect("first add");
        // Second add should NOT error.
        db.add_member("dups", "nathan_mackinnon")
            .expect("duplicate add should not error");

        let members = db.list_members("dups").expect("list members");
        assert_eq!(members.len(), 1, "duplicate member should appear only once");
    }
}
