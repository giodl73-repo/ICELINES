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

    // Migration 003 — saved queries
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS saved_queries (
            name        TEXT PRIMARY KEY,
            fields_json TEXT NOT NULL,
            created_at  TEXT NOT NULL
        );",
    )
    .context("migration 003: create saved_queries table")?;

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
        let db = Self { conn };
        // Seed a default "Favorites" group so users have a group to add players to immediately.
        let _ = db.conn.execute(
            "INSERT OR IGNORE INTO groups (name, description, created_at) VALUES ('Favorites', 'My favorite players', datetime('now'))",
            [],
        );
        Ok(db)
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

    /// Rename a group. Phase 8f.6.
    ///
    /// FK constraint on `group_members.group_name` cascades the change to all
    /// members. Returns Ok(()) on success; errors when `old` doesn't exist or
    /// when `new` collides with another group.
    pub fn rename_group(&self, old: &str, new: &str) -> anyhow::Result<()> {
        if old == new {
            return Ok(());
        }
        self.require_group(old)?;
        // Reject collisions explicitly so the user sees a clean message rather
        // than rusqlite's UNIQUE-constraint-violated error.
        let collides: bool = self.conn
            .query_row(
                "SELECT 1 FROM groups WHERE name = ?1",
                rusqlite::params![new],
                |_| Ok(true),
            )
            .unwrap_or(false);
        if collides {
            bail!("a group named '{new}' already exists");
        }
        // The schema has FK on group_members.group_name → groups.name without
        // ON UPDATE CASCADE, so a naive UPDATE on `groups` violates the
        // constraint mid-transaction. `defer_foreign_keys = ON` postpones
        // enforcement until COMMIT, letting us rewrite both tables in one tx.
        let tx = self.conn.unchecked_transaction()?;
        tx.execute_batch("PRAGMA defer_foreign_keys = ON;")?;
        tx.execute(
            "UPDATE groups SET name = ?1 WHERE name = ?2",
            rusqlite::params![new, old],
        )?;
        tx.execute(
            "UPDATE group_members SET group_name = ?1 WHERE group_name = ?2",
            rusqlite::params![new, old],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Bulk-insert members into a group (used by `group import`). Returns the
    /// number of new rows actually inserted (duplicates are silently skipped).
    pub fn add_members_bulk(&self, group: &str, players_normalized: &[String])
        -> anyhow::Result<usize>
    {
        self.require_group(group)?;
        let now = now_utc();
        let tx = self.conn.unchecked_transaction()?;
        let mut inserted = 0usize;
        {
            let mut stmt = tx.prepare(
                "INSERT OR IGNORE INTO group_members \
                 (group_name, player_normalized, added_at) VALUES (?1, ?2, ?3)",
            )?;
            for p in players_normalized {
                let rows = stmt.execute(rusqlite::params![group, p, now])?;
                inserted += rows;
            }
        }
        tx.commit()?;
        Ok(inserted)
    }

    /// Look up a group's description (returns Ok("") when the group has none).
    pub fn group_description(&self, name: &str) -> anyhow::Result<String> {
        self.require_group(name)?;
        let desc: String = self.conn
            .query_row(
                "SELECT description FROM groups WHERE name = ?1",
                rusqlite::params![name],
                |r| r.get(0),
            )
            .unwrap_or_default();
        Ok(desc)
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

    // ── Saved queries ─────────────────────────────────────────────────────────

    /// Save a named query (overwrites if name already exists).
    pub fn save_query(&self, name: &str, fields_json: &str) -> anyhow::Result<()> {
        let now = now_utc();
        self.conn.execute(
            "INSERT OR REPLACE INTO saved_queries (name, fields_json, created_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![name, fields_json, now],
        ).with_context(|| format!("save query '{name}'"))?;
        Ok(())
    }

    /// List all saved queries, newest first.
    pub fn list_saved_queries(&self) -> anyhow::Result<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT name, fields_json FROM saved_queries ORDER BY created_at DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Delete a saved query by name.
    #[allow(dead_code)]
    pub fn delete_saved_query(&self, name: &str) -> anyhow::Result<()> {
        self.conn.execute("DELETE FROM saved_queries WHERE name = ?1", rusqlite::params![name])?;
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

    // ── Phase 8f.6: rename + bulk import ───────────────────────────────────

    #[test]
    fn l1_db_rename_group_moves_members() {
        let db = GroupDb::open_in_memory().expect("open in-memory db");
        db.create_group("old-name", "to be renamed").expect("create");
        db.add_member("old-name", "alice").expect("add");
        db.add_member("old-name", "bob").expect("add");

        db.rename_group("old-name", "new-name").expect("rename");

        // Old name is gone.
        assert!(db.list_members("old-name").is_err(),
            "old name should no longer exist");
        // New name has both members.
        let mut members = db.list_members("new-name").expect("list new");
        members.sort();
        assert_eq!(members, vec!["alice".to_owned(), "bob".to_owned()]);
        // Description is preserved.
        assert_eq!(db.group_description("new-name").unwrap(), "to be renamed");
    }

    #[test]
    fn l1_db_rename_to_same_name_is_noop() {
        let db = GroupDb::open_in_memory().expect("open in-memory db");
        db.create_group("x", "").expect("create");
        // Should not error and should not destroy the group.
        db.rename_group("x", "x").expect("noop rename");
        assert_eq!(db.list_groups().unwrap().len(), 1);
    }

    #[test]
    fn l1_db_rename_collision_errors() {
        let db = GroupDb::open_in_memory().expect("open in-memory db");
        db.create_group("a", "").expect("create a");
        db.create_group("b", "").expect("create b");
        let err = db.rename_group("a", "b").unwrap_err().to_string();
        assert!(err.contains("already exists"),
            "collision must mention 'already exists', got: {err}");
        // Both groups still exist with their original names.
        let names: Vec<String> = db.list_groups().unwrap().into_iter().map(|g| g.name).collect();
        assert!(names.contains(&"a".to_owned()));
        assert!(names.contains(&"b".to_owned()));
    }

    #[test]
    fn l1_db_rename_unknown_group_errors() {
        let db = GroupDb::open_in_memory().expect("open in-memory db");
        let err = db.rename_group("ghost", "phantom").unwrap_err().to_string();
        assert!(err.contains("not found"),
            "unknown old name must error, got: {err}");
    }

    #[test]
    fn l1_db_add_members_bulk_dedups_and_counts() {
        let db = GroupDb::open_in_memory().expect("open in-memory db");
        db.create_group("g", "").expect("create");
        db.add_member("g", "alice").expect("seed");

        let members = vec!["alice".to_owned(), "bob".to_owned(), "carol".to_owned()];
        let inserted = db.add_members_bulk("g", &members).expect("bulk add");
        assert_eq!(inserted, 2, "alice already there, bob+carol new = 2");
        assert_eq!(db.list_members("g").unwrap().len(), 3);
    }
}
