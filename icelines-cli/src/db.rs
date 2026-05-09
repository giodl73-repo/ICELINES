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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchNote {
    pub entity_ref: String,
    pub reason: String,
    pub source: String,
    pub updated_at: String,
}

/// Migration 005 — discriminator on `group_members.kind`. A favorites
/// group can carry both player normalized names AND team abbrevs;
/// downstream code branches on this to load the right thing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberKind {
    Player,
    Team,
}

impl MemberKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Player => "player",
            Self::Team => "team",
        }
    }

    /// Lossy parse — anything we don't recognize buckets as Player so
    /// pre-migration rows (which have NULL/no kind value, defaulted
    /// to 'player' by migration 005) round-trip cleanly. Migration
    /// 006 made the on-disk shape `entity_ref` (where the kind is
    /// the prefix), so this is now used for hand-built kind strings
    /// in tests and external callers — kept as a public helper.
    #[allow(dead_code)]
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "team" => Self::Team,
            _ => Self::Player,
        }
    }
}

/// Opaque handle to the group database.
pub struct GroupDb {
    conn: Connection,
}

// ── Migrations ────────────────────────────────────────────────────────────────

/// Phase Foster.3 — `pub(crate)` re-export so `event_stream.rs` can
/// initialize the events table on first open without forking the
/// migration list.
pub(crate) fn run_migrations_for_test(conn: &Connection) -> anyhow::Result<()> {
    run_migrations(conn)
}

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

    // Migration 004 — games-I-attended log
    //
    // Personal record of NHL games the user attended in person. Keyed by
    // the NHL game_id so we can rejoin it back to the boxscore feed and
    // render rich team stats. `note` is freeform — let users record
    // "took my dad to his first NHL game", "Ovechkin's 800th", etc.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS attended_games (
            game_id     INTEGER PRIMARY KEY,
            game_date   TEXT,         -- YYYY-MM-DD captured at add time
            away_abbrev TEXT,
            home_abbrev TEXT,
            away_score  INTEGER,
            home_score  INTEGER,
            note        TEXT NOT NULL DEFAULT '',
            attended_at TEXT NOT NULL  -- ISO-8601 of the row insert
        );",
    )
    .context("migration 004: create attended_games table")?;

    // Enable foreign-key enforcement (off by default in rusqlite).
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .context("enable foreign keys")?;

    // Migration 005 — `kind` discriminator on group_members.
    //
    // Existing rows are players; new rows can be either 'player' (a
    // normalized name) or 'team' (a 3-letter NHL abbrev). The
    // discriminator decouples the namespace so adding "EDM" as a team
    // doesn't shadow a future player whose normalized name happens to
    // be "edm".
    //
    // sqlite ALTER TABLE only supports adding a column at the end with
    // a default; we don't change the PK (collisions are vanishingly
    // rare given player names are lowercase ASCII and team abbrevs
    // are 3-uppercase, but stuck PK is the safe choice).
    let cols: Vec<String> = conn
        .prepare("PRAGMA table_info(group_members);")?
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(Result::ok)
        .collect();
    if !cols.iter().any(|c| c == "kind") {
        conn.execute_batch(
            "ALTER TABLE group_members ADD COLUMN kind TEXT NOT NULL DEFAULT 'player';",
        )
        .context("migration 005: add kind column to group_members")?;
    }

    // Migration 006 — collapse (kind, player_normalized) into entity_ref.
    //
    // After 006: group_members has one stringly-typed column
    // `entity_ref` matching the EntityRef Display form
    // (`player:<key>` / `team:<ABBR>`). MemberKind is derived from
    // the prefix; the legacy `kind` + `player_normalized` columns
    // are dropped.
    //
    // Idempotent — `entity_ref` column presence is the gate. Rerun
    // with the new schema is a no-op.
    let cols: Vec<String> = conn
        .prepare("PRAGMA table_info(group_members);")?
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(Result::ok)
        .collect();
    if !cols.iter().any(|c| c == "entity_ref") {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch("PRAGMA defer_foreign_keys = ON;")?;
        tx.execute_batch(
            "ALTER TABLE group_members ADD COLUMN entity_ref TEXT;
             UPDATE group_members
                SET entity_ref = CASE kind
                    WHEN 'team' THEN 'team:' || player_normalized
                    ELSE 'player:' || player_normalized
                END
              WHERE entity_ref IS NULL;
             CREATE TABLE group_members_new (
                 group_name TEXT NOT NULL,
                 entity_ref TEXT NOT NULL,
                 added_at   TEXT NOT NULL,
                 PRIMARY KEY (group_name, entity_ref),
                 FOREIGN KEY (group_name) REFERENCES groups(name) ON DELETE CASCADE
             );
             INSERT INTO group_members_new (group_name, entity_ref, added_at)
                 SELECT group_name, entity_ref, added_at FROM group_members;
             DROP TABLE group_members;
             ALTER TABLE group_members_new RENAME TO group_members;",
        )
        .context("migration 006: collapse kind into entity_ref")?;
        tx.commit().context("migration 006: commit")?;
    }

    // Migration 007 — Phase Foster.3 EventStream table.
    //
    // Per `design/specs/foster-favorites-dashboard.md` §EventStream:
    // the PK includes `event_id` (caller-supplied dedup key) so
    // re-fetched events update via INSERT … ON CONFLICT instead of
    // duplicating rows. Indexes match the two read paths Foster.2's
    // favorites view and any future timeline surface care about:
    // by date (newest first) and by entity (newest first).
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS events (
            date            TEXT NOT NULL,
            entity_kind     TEXT NOT NULL,
            entity_key      TEXT NOT NULL,
            event_kind      TEXT NOT NULL,
            event_id        TEXT NOT NULL,
            payload         TEXT NOT NULL,
            payload_version INTEGER NOT NULL,
            created_at      TEXT NOT NULL,
            PRIMARY KEY (date, entity_kind, entity_key, event_kind, event_id)
         );
         CREATE INDEX IF NOT EXISTS events_by_date ON events(date DESC);
         CREATE INDEX IF NOT EXISTS events_by_entity \
            ON events(entity_kind, entity_key, date DESC);",
    )
    .context("migration 007: events table")?;

    // Migration 008 — Selke watch metadata.
    //
    // Membership remains in `group_members` so Watchlist still behaves
    // like any other group. This table stores optional user/system
    // context for why a player was watched.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS watch_notes (
            entity_ref TEXT PRIMARY KEY,
            reason     TEXT NOT NULL DEFAULT '',
            source     TEXT NOT NULL DEFAULT '',
            updated_at TEXT NOT NULL
         );",
    )
    .context("migration 008: watch_notes table")?;

    Ok(())
}

/// Build a stringly-typed entity_ref for a (kind, key) pair. Mirrors
/// the on-disk format produced by migration 006 backfill.
fn entity_ref_for(kind: MemberKind, key: &str) -> String {
    format!("{}:{}", kind.as_str(), key)
}

/// Inverse of `entity_ref_for` — split a stored `entity_ref` back
/// into `(MemberKind, key)`. Lossy on unknown prefixes (default to
/// Player, like `MemberKind::from_str_lossy`).
fn entity_ref_split(entity_ref: &str) -> (MemberKind, String) {
    match entity_ref.split_once(':') {
        Some(("team", k)) => (MemberKind::Team, k.to_string()),
        Some(("player", k)) => (MemberKind::Player, k.to_string()),
        Some((_, k)) => (MemberKind::Player, k.to_string()),
        None => (MemberKind::Player, entity_ref.to_string()),
    }
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
        std::fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;

        let db_path = dir.join("icelines.db");
        let conn =
            Connection::open(&db_path).with_context(|| format!("open {}", db_path.display()))?;

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
            .execute(
                "DELETE FROM groups WHERE name = ?1",
                rusqlite::params![name],
            )
            .with_context(|| format!("delete group '{name}'"))?;
        Ok(rows > 0)
    }

    /// Add a player (normalized name) to a group.
    /// Returns `true` if added, `false` if already a member (no-op).
    pub fn add_member(&self, group: &str, player_normalized: &str) -> anyhow::Result<bool> {
        self.add_member_kind(group, player_normalized, MemberKind::Player)
    }

    /// Migration 005 / 006 — kind-aware member insert. After
    /// migration 006 the row stores a single `entity_ref` (e.g.
    /// `player:connor mcdavid` / `team:EDM`); `MemberKind` is
    /// derived from the prefix on read.
    pub fn add_member_kind(
        &self,
        group: &str,
        key: &str,
        kind: MemberKind,
    ) -> anyhow::Result<bool> {
        self.require_group(group)?;
        let now = now_utc();
        let entity_ref = entity_ref_for(kind, key);
        let rows = self
            .conn
            .execute(
                "INSERT OR IGNORE INTO group_members \
                 (group_name, entity_ref, added_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![group, entity_ref, now],
            )
            .with_context(|| format!("add member '{key}' ({}) to '{group}'", kind.as_str()))?;
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
        let collides: bool = self
            .conn
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
    pub fn add_members_bulk(
        &self,
        group: &str,
        players_normalized: &[String],
    ) -> anyhow::Result<usize> {
        self.require_group(group)?;
        let now = now_utc();
        let tx = self.conn.unchecked_transaction()?;
        let mut inserted = 0usize;
        {
            let mut stmt = tx.prepare(
                "INSERT OR IGNORE INTO group_members \
                 (group_name, entity_ref, added_at) VALUES (?1, ?2, ?3)",
            )?;
            for p in players_normalized {
                let entity_ref = entity_ref_for(MemberKind::Player, p);
                let rows = stmt.execute(rusqlite::params![group, entity_ref, now])?;
                inserted += rows;
            }
        }
        tx.commit()?;
        Ok(inserted)
    }

    /// Look up a group's description (returns Ok("") when the group has none).
    pub fn group_description(&self, name: &str) -> anyhow::Result<String> {
        self.require_group(name)?;
        let desc: String = self
            .conn
            .query_row(
                "SELECT description FROM groups WHERE name = ?1",
                rusqlite::params![name],
                |r| r.get(0),
            )
            .unwrap_or_default();
        Ok(desc)
    }

    /// Remove a player from a group.  No-op if the player is not a member.
    /// Backward-compat: takes the bare key (e.g. normalized name) and
    /// removes the corresponding `player:<key>` entity_ref.
    pub fn remove_member(&self, group: &str, player_normalized: &str) -> anyhow::Result<()> {
        self.remove_member_kind(group, player_normalized, MemberKind::Player)
    }

    /// Kind-aware remove. After migration 006 the row is keyed by
    /// `(group_name, entity_ref)` so removing a team uses
    /// `MemberKind::Team` to build the right entity_ref string.
    pub fn remove_member_kind(
        &self,
        group: &str,
        key: &str,
        kind: MemberKind,
    ) -> anyhow::Result<()> {
        self.require_group(group)?;
        let entity_ref = entity_ref_for(kind, key);
        self.conn
            .execute(
                "DELETE FROM group_members WHERE group_name = ?1 AND entity_ref = ?2",
                rusqlite::params![group, entity_ref],
            )
            .with_context(|| format!("remove member '{key}' ({}) from '{group}'", kind.as_str()))?;
        Ok(())
    }

    pub fn upsert_watch_note(
        &self,
        kind: MemberKind,
        key: &str,
        reason: &str,
        source: &str,
    ) -> anyhow::Result<()> {
        let now = now_utc();
        let entity_ref = entity_ref_for(kind, key);
        self.conn
            .execute(
                "INSERT INTO watch_notes (entity_ref, reason, source, updated_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(entity_ref) DO UPDATE SET
                    reason = excluded.reason,
                    source = excluded.source,
                    updated_at = excluded.updated_at",
                rusqlite::params![entity_ref, reason, source, now],
            )
            .with_context(|| format!("upsert watch note for '{key}'"))?;
        Ok(())
    }

    pub fn delete_watch_note(&self, kind: MemberKind, key: &str) -> anyhow::Result<()> {
        let entity_ref = entity_ref_for(kind, key);
        self.conn
            .execute(
                "DELETE FROM watch_notes WHERE entity_ref = ?1",
                rusqlite::params![entity_ref],
            )
            .with_context(|| format!("delete watch note for '{key}'"))?;
        Ok(())
    }

    // ── Read operations ───────────────────────────────────────────────────────

    /// List all groups with their member counts.
    pub fn list_groups(&self) -> anyhow::Result<Vec<GroupRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT g.name, g.description, COUNT(m.entity_ref) AS member_count
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

    /// List all PLAYER members of a group (normalized names). Backward-
    /// compat surface — strips the `player:` prefix so existing callers
    /// still see bare normalized names. New code should prefer
    /// `list_members_with_kind`.
    pub fn list_members(&self, group: &str) -> anyhow::Result<Vec<String>> {
        self.require_group(group)?;

        let mut stmt = self.conn.prepare(
            "SELECT entity_ref FROM group_members \
             WHERE group_name = ?1 AND entity_ref LIKE 'player:%' ORDER BY added_at",
        )?;

        let rows = stmt
            .query_map(rusqlite::params![group], |row| row.get::<_, String>(0))
            .context("list_members query")?
            .filter_map(Result::ok)
            .map(|er| entity_ref_split(&er).1)
            .collect::<Vec<String>>();

        Ok(rows)
    }

    /// Post-006 — list every member with its derived `MemberKind`.
    /// `MemberKind::from(&entity_ref_prefix)` is the single source of
    /// truth for the player/team discriminator. Used by `group show`
    /// to render Players + Teams sections.
    pub fn list_members_with_kind(&self, group: &str) -> anyhow::Result<Vec<(String, MemberKind)>> {
        self.require_group(group)?;
        let mut stmt = self.conn.prepare(
            "SELECT entity_ref FROM group_members \
             WHERE group_name = ?1 ORDER BY entity_ref, added_at",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![group], |row| row.get::<_, String>(0))
            .context("list_members_with_kind query")?
            .collect::<Result<Vec<_>, _>>()
            .context("list_members_with_kind collect")?;
        let mapped = rows
            .into_iter()
            .map(|er| {
                let (kind, key) = entity_ref_split(&er);
                (key, kind)
            })
            .collect();
        Ok(mapped)
    }

    pub fn watch_note(&self, kind: MemberKind, key: &str) -> anyhow::Result<Option<WatchNote>> {
        let entity_ref = entity_ref_for(kind, key);
        let mut stmt = self.conn.prepare(
            "SELECT entity_ref, reason, source, updated_at
             FROM watch_notes
             WHERE entity_ref = ?1",
        )?;
        let mut rows = stmt.query(rusqlite::params![entity_ref])?;
        if let Some(row) = rows.next()? {
            Ok(Some(WatchNote {
                entity_ref: row.get(0)?,
                reason: row.get(1)?,
                source: row.get(2)?,
                updated_at: row.get(3)?,
            }))
        } else {
            Ok(None)
        }
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
        let mut stmt = self
            .conn
            .prepare("SELECT name, fields_json FROM saved_queries ORDER BY created_at DESC")?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Delete a saved query by name.
    #[allow(dead_code)]
    pub fn delete_saved_query(&self, name: &str) -> anyhow::Result<()> {
        self.conn.execute(
            "DELETE FROM saved_queries WHERE name = ?1",
            rusqlite::params![name],
        )?;
        Ok(())
    }

    // ── Attended games ──────────────────────────────────────────────────

    /// Record one NHL game the user attended in person. Idempotent —
    /// duplicate calls for the same `game_id` overwrite the row so the
    /// note can be edited after the fact. The metadata fields (date,
    /// abbrevs, score) are captured at add time so the row reads
    /// usefully even after the API rotates older boxscores out.
    pub fn add_attended_game(&self, row: &AttendedGameInput) -> anyhow::Result<()> {
        let now = now_utc();
        self.conn
            .execute(
                "INSERT OR REPLACE INTO attended_games \
             (game_id, game_date, away_abbrev, home_abbrev, \
              away_score, home_score, note, attended_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    row.game_id,
                    row.game_date,
                    row.away_abbrev,
                    row.home_abbrev,
                    row.away_score,
                    row.home_score,
                    row.note,
                    now,
                ],
            )
            .with_context(|| format!("record attended game {}", row.game_id))?;
        Ok(())
    }

    /// Remove a game from the attended list. Returns `true` if a row
    /// was actually deleted, `false` if the game wasn't on the list.
    pub fn remove_attended_game(&self, game_id: u64) -> anyhow::Result<bool> {
        let n = self
            .conn
            .execute(
                "DELETE FROM attended_games WHERE game_id = ?1",
                rusqlite::params![game_id],
            )
            .with_context(|| format!("delete attended game {game_id}"))?;
        Ok(n > 0)
    }

    /// True iff this game is already on the attended list.
    #[allow(dead_code)] // Public API surface; reserved for future "already-attended" UI marker.
    pub fn is_attended(&self, game_id: u64) -> anyhow::Result<bool> {
        let exists: bool = self
            .conn
            .query_row(
                "SELECT 1 FROM attended_games WHERE game_id = ?1",
                rusqlite::params![game_id],
                |_| Ok(true),
            )
            .unwrap_or(false);
        Ok(exists)
    }

    /// List every attended game, newest first by `game_date`. Rows
    /// without a date sort to the bottom.
    pub fn list_attended_games(&self) -> anyhow::Result<Vec<AttendedGameRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT game_id, game_date, away_abbrev, home_abbrev,
                    away_score, home_score, note, attended_at
             FROM attended_games
             ORDER BY game_date DESC NULLS LAST, attended_at DESC",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(AttendedGameRow {
                    game_id: r.get::<_, i64>(0)? as u64,
                    game_date: r.get::<_, Option<String>>(1)?,
                    away_abbrev: r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    home_abbrev: r.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    away_score: r.get::<_, Option<i64>>(4)?.map(|n| n as u8),
                    home_score: r.get::<_, Option<i64>>(5)?.map(|n| n as u8),
                    note: r.get::<_, String>(6)?,
                    attended_at: r.get::<_, String>(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

// ── Attended games — row types (Phase 8 follow-up) ─────────────────────────

/// What a caller hands `add_attended_game` — the metadata captured at
/// add time so list views work even after the API rotates older
/// boxscores.
#[derive(Debug, Clone)]
pub struct AttendedGameInput {
    pub game_id: u64,
    pub game_date: Option<String>, // "YYYY-MM-DD"
    pub away_abbrev: Option<String>,
    pub home_abbrev: Option<String>,
    pub away_score: Option<u8>,
    pub home_score: Option<u8>,
    pub note: String,
}

/// One row in the attended-games list view.
#[derive(Debug, Clone)]
#[allow(dead_code)] // `attended_at` populated for future audit / sort-by-date UI; not yet displayed.
pub struct AttendedGameRow {
    pub game_id: u64,
    pub game_date: Option<String>,
    pub away_abbrev: String,
    pub home_abbrev: String,
    pub away_score: Option<u8>,
    pub home_score: Option<u8>,
    pub note: String,
    pub attended_at: String,
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

    /// Migration 005 / l1_db_member_kind_player_default
    /// — old `add_member` keeps player kind; round-trips via
    ///   list_members_with_kind.
    #[test]
    fn l1_db_member_kind_player_default() {
        let db = GroupDb::open_in_memory().expect("open");
        db.create_group("Favorites", "").expect("create");
        db.add_member("Favorites", "connor_mcdavid")
            .expect("add player");
        let kinded = db.list_members_with_kind("Favorites").expect("list kinded");
        assert_eq!(kinded.len(), 1);
        assert_eq!(kinded[0].0, "connor_mcdavid");
        assert_eq!(kinded[0].1, MemberKind::Player);
    }

    /// Migration 005 / l1_db_add_team_member_round_trips
    /// — the new `add_member_kind` writes Team rows that surface
    ///   only on the kinded reader, NOT on the legacy
    ///   `list_members` (which is now player-scoped).
    #[test]
    fn l1_db_add_team_member_round_trips() {
        let db = GroupDb::open_in_memory().expect("open");
        db.create_group("Favorites", "").expect("create");
        db.add_member("Favorites", "connor_mcdavid").expect("p");
        db.add_member_kind("Favorites", "EDM", MemberKind::Team)
            .expect("t");
        // Legacy reader only sees the player.
        let players_only = db.list_members("Favorites").expect("legacy");
        assert_eq!(players_only.len(), 1);
        assert_eq!(players_only[0], "connor_mcdavid");
        // Kinded reader sees both.
        let kinded = db.list_members_with_kind("Favorites").expect("kinded");
        assert_eq!(kinded.len(), 2);
        let teams: Vec<&str> = kinded
            .iter()
            .filter(|(_, k)| *k == MemberKind::Team)
            .map(|(k, _)| k.as_str())
            .collect();
        assert_eq!(teams, vec!["EDM"]);
    }

    /// Migration 005 / l1_db_remove_team_member_works
    /// — remove_member doesn't filter on kind; it deletes by key.
    ///   Adding "EDM" as team and removing "EDM" should drop it.
    #[test]
    fn l1_db_remove_team_member_works() {
        let db = GroupDb::open_in_memory().expect("open");
        db.create_group("Favorites", "").expect("create");
        db.add_member_kind("Favorites", "EDM", MemberKind::Team)
            .expect("t");
        // Migration 006: removing a team needs the kind-aware variant
        // because (group, key) alone is ambiguous when the same key
        // could exist under both player: and team: prefixes.
        db.remove_member_kind("Favorites", "EDM", MemberKind::Team)
            .expect("rm");
        let kinded = db.list_members_with_kind("Favorites").expect("k");
        assert!(kinded.is_empty(), "team row should be gone, got {kinded:?}");
    }

    /// Migration 005 / l1_db_member_kind_idempotent_add
    /// — adding the same (group, key, kind) twice is a no-op.
    #[test]
    fn l1_db_member_kind_idempotent_add() {
        let db = GroupDb::open_in_memory().expect("open");
        db.create_group("Favorites", "").expect("create");
        let first = db
            .add_member_kind("Favorites", "EDM", MemberKind::Team)
            .expect("first");
        let second = db
            .add_member_kind("Favorites", "EDM", MemberKind::Team)
            .expect("second");
        assert!(first, "first add should report inserted");
        assert!(!second, "second add should report no-op");
    }

    /// Migration 005 / l0_member_kind_from_str_lossy
    /// — pre-migration NULLs default to player; unknown strings
    ///   bucket as player (defensive).
    #[test]
    fn l0_member_kind_from_str_lossy() {
        assert_eq!(MemberKind::from_str_lossy("player"), MemberKind::Player);
        assert_eq!(MemberKind::from_str_lossy("team"), MemberKind::Team);
        assert_eq!(MemberKind::from_str_lossy(""), MemberKind::Player);
        assert_eq!(MemberKind::from_str_lossy("garbage"), MemberKind::Player);
    }

    #[test]
    fn l1_db_create_and_list_group() {
        let db = GroupDb::open_in_memory().expect("open in-memory db");
        db.create_group("myteam", "My fantasy team")
            .expect("create group");

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

        db.remove_member("g1", "leon_draisaitl")
            .expect("remove member");
        let members = db.list_members("g1").expect("list members after remove");
        assert_eq!(members.len(), 1);
        assert_eq!(members[0], "connor_mcdavid");
    }

    #[test]
    fn l1_db_watch_note_round_trips_and_deletes() {
        let db = GroupDb::open_in_memory().expect("open in-memory db");

        db.upsert_watch_note(
            MemberKind::Player,
            "matthew knies",
            "Poach score 72.0; confidence High; PP1 promotion",
            "tui-poach",
        )
        .expect("upsert watch note");

        let note = db
            .watch_note(MemberKind::Player, "matthew knies")
            .expect("read watch note")
            .expect("note exists");
        assert_eq!(note.entity_ref, "player:matthew knies");
        assert!(note.reason.contains("Poach score"));
        assert_eq!(note.source, "tui-poach");

        db.delete_watch_note(MemberKind::Player, "matthew knies")
            .expect("delete watch note");
        assert!(db
            .watch_note(MemberKind::Player, "matthew knies")
            .expect("read after delete")
            .is_none());
    }

    #[test]
    fn l1_db_delete_group_cascades_members() {
        let db = GroupDb::open_in_memory().expect("open in-memory db");
        db.create_group("watchlist", "").expect("create group");
        db.add_member("watchlist", "player_one")
            .expect("add member");
        db.add_member("watchlist", "player_two")
            .expect("add member");

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

        db.add_member("dups", "nathan_mackinnon")
            .expect("first add");
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
        db.create_group("old-name", "to be renamed")
            .expect("create");
        db.add_member("old-name", "alice").expect("add");
        db.add_member("old-name", "bob").expect("add");

        db.rename_group("old-name", "new-name").expect("rename");

        // Old name is gone.
        assert!(
            db.list_members("old-name").is_err(),
            "old name should no longer exist"
        );
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
        assert!(
            err.contains("already exists"),
            "collision must mention 'already exists', got: {err}"
        );
        // Both groups still exist with their original names.
        let names: Vec<String> = db
            .list_groups()
            .unwrap()
            .into_iter()
            .map(|g| g.name)
            .collect();
        assert!(names.contains(&"a".to_owned()));
        assert!(names.contains(&"b".to_owned()));
    }

    #[test]
    fn l1_db_rename_unknown_group_errors() {
        let db = GroupDb::open_in_memory().expect("open in-memory db");
        let err = db.rename_group("ghost", "phantom").unwrap_err().to_string();
        assert!(
            err.contains("not found"),
            "unknown old name must error, got: {err}"
        );
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

    // ── Attended games ─────────────────────────────────────────────────────

    fn fixture_attended(
        game_id: u64,
        date: &str,
        away: &str,
        home: &str,
        score: (u8, u8),
        note: &str,
    ) -> AttendedGameInput {
        AttendedGameInput {
            game_id,
            game_date: Some(date.to_owned()),
            away_abbrev: Some(away.to_owned()),
            home_abbrev: Some(home.to_owned()),
            away_score: Some(score.0),
            home_score: Some(score.1),
            note: note.to_owned(),
        }
    }

    #[test]
    fn l1_db_attended_add_query_round_trip() {
        let db = GroupDb::open_in_memory().expect("in-memory");
        let g = fixture_attended(
            2025020100,
            "2026-01-15",
            "SEA",
            "VGK",
            (3, 2),
            "Kraken first home win of 2026",
        );

        assert!(
            !db.is_attended(g.game_id).unwrap(),
            "fresh db: not attended"
        );
        db.add_attended_game(&g).expect("record");
        assert!(db.is_attended(g.game_id).unwrap(), "after add: attended");

        let rows = db.list_attended_games().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].game_id, 2025020100);
        assert_eq!(rows[0].game_date.as_deref(), Some("2026-01-15"));
        assert_eq!(rows[0].away_abbrev, "SEA");
        assert_eq!(rows[0].home_abbrev, "VGK");
        assert_eq!(rows[0].away_score, Some(3));
        assert_eq!(rows[0].home_score, Some(2));
        assert_eq!(rows[0].note, "Kraken first home win of 2026");
    }

    #[test]
    fn l1_db_attended_add_is_idempotent_overwrites_note() {
        // Re-adding the same game_id should overwrite the row so the
        // user can edit the note after the fact.
        let db = GroupDb::open_in_memory().expect("in-memory");
        db.add_attended_game(&fixture_attended(
            1,
            "2026-01-15",
            "BOS",
            "MTL",
            (1, 0),
            "first attempt",
        ))
        .expect("first add");
        db.add_attended_game(&fixture_attended(
            1,
            "2026-01-15",
            "BOS",
            "MTL",
            (1, 0),
            "edited note",
        ))
        .expect("second add");
        let rows = db.list_attended_games().unwrap();
        assert_eq!(rows.len(), 1, "still one row after overwrite");
        assert_eq!(rows[0].note, "edited note");
    }

    #[test]
    fn l1_db_attended_remove_returns_false_when_absent() {
        let db = GroupDb::open_in_memory().expect("in-memory");
        let removed = db.remove_attended_game(99).unwrap();
        assert!(!removed, "remove of unknown game returns false");
    }

    #[test]
    fn l1_db_attended_remove_then_list_excludes_row() {
        let db = GroupDb::open_in_memory().expect("in-memory");
        db.add_attended_game(&fixture_attended(1, "2026-01-15", "BOS", "MTL", (1, 0), ""))
            .expect("add");
        let removed = db.remove_attended_game(1).unwrap();
        assert!(removed);
        assert!(!db.is_attended(1).unwrap());
        assert!(db.list_attended_games().unwrap().is_empty());
    }

    #[test]
    fn l1_db_attended_list_orders_newest_first() {
        // Sort by game_date DESC — earlier dates render below later ones.
        let db = GroupDb::open_in_memory().expect("in-memory");
        db.add_attended_game(&fixture_attended(1, "2025-12-01", "A", "B", (1, 0), ""))
            .expect("add");
        db.add_attended_game(&fixture_attended(2, "2026-03-15", "C", "D", (2, 1), ""))
            .expect("add");
        db.add_attended_game(&fixture_attended(3, "2026-01-15", "E", "F", (3, 2), ""))
            .expect("add");
        let rows = db.list_attended_games().unwrap();
        let dates: Vec<_> = rows
            .iter()
            .map(|r| r.game_date.clone().unwrap_or_default())
            .collect();
        assert_eq!(
            dates,
            vec!["2026-03-15", "2026-01-15", "2025-12-01"],
            "newest first by game_date, got: {dates:?}"
        );
    }

    // ── Migration 006 — kind→entity_ref backfill (Phase Foster.0.6) ──────────

    /// Open a connection and run only migrations up through 005, so
    /// the test can plant pre-006 fixture rows before letting 006
    /// run on top.
    fn open_pre_006() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        conn.execute_batch(
            "CREATE TABLE groups (
                name        TEXT PRIMARY KEY,
                description TEXT NOT NULL DEFAULT '',
                created_at  TEXT NOT NULL
             );
             CREATE TABLE group_members (
                group_name        TEXT NOT NULL,
                player_normalized TEXT NOT NULL,
                added_at          TEXT NOT NULL,
                kind              TEXT NOT NULL DEFAULT 'player',
                PRIMARY KEY (group_name, player_normalized),
                FOREIGN KEY (group_name) REFERENCES groups(name) ON DELETE CASCADE
             );",
        )
        .unwrap();
        conn
    }

    /// 006.1 — round-trip with a pre-006 fixture: planted rows
    /// (player + team) survive the migration and surface with the
    /// correct kind under the new entity_ref column.
    #[test]
    fn l1_db_006_round_trip_with_pre_migration_fixture() {
        let conn = open_pre_006();
        conn.execute(
            "INSERT INTO groups (name, description, created_at) VALUES (?1, ?2, ?3)",
            rusqlite::params!["Favorites", "", "2026-01-01T00:00:00Z"],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO group_members (group_name, player_normalized, added_at, kind) \
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                "Favorites",
                "connor mcdavid",
                "2026-01-01T00:00:00Z",
                "player"
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO group_members (group_name, player_normalized, added_at, kind) \
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params!["Favorites", "EDM", "2026-01-01T00:00:00Z", "team"],
        )
        .unwrap();

        // Migration 006 applies via run_migrations.
        super::run_migrations(&conn).expect("006 runs cleanly");

        let mut stmt = conn
            .prepare("SELECT entity_ref FROM group_members ORDER BY entity_ref")
            .unwrap();
        let refs: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        assert_eq!(
            refs,
            vec!["player:connor mcdavid", "team:EDM"],
            "backfill produced canonical entity_ref strings"
        );
    }

    /// 006.2 — re-running the migration on an already-migrated db is
    /// a no-op (idempotent).
    #[test]
    fn l1_db_006_idempotent_re_run() {
        let db = GroupDb::open_in_memory().expect("first migrate");
        db.create_group("Favorites", "").unwrap();
        db.add_member_kind("Favorites", "EDM", MemberKind::Team)
            .unwrap();

        // run_migrations again — entity_ref column already exists,
        // so the rebuild branch must not fire.
        super::run_migrations(&db.conn).expect("rerun ok");

        let kinded = db.list_members_with_kind("Favorites").unwrap();
        assert_eq!(kinded, vec![("EDM".to_string(), MemberKind::Team)]);
    }

    /// 006.3 — mixed kind fixture: 5 players + 3 teams across two
    /// groups all backfill to the right entity_ref shape.
    #[test]
    fn l1_db_006_mixed_kind_backfill() {
        let conn = open_pre_006();
        for g in ["Favorites", "Watchlist"] {
            conn.execute(
                "INSERT INTO groups (name, description, created_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![g, "", "2026-01-01T00:00:00Z"],
            )
            .unwrap();
        }
        let rows: &[(&str, &str, &str)] = &[
            ("Favorites", "connor mcdavid", "player"),
            ("Favorites", "leon draisaitl", "player"),
            ("Favorites", "EDM", "team"),
            ("Favorites", "FLA", "team"),
            ("Watchlist", "auston matthews", "player"),
            ("Watchlist", "william nylander", "player"),
            ("Watchlist", "mitch marner", "player"),
            ("Watchlist", "TOR", "team"),
        ];
        for (g, k, kind) in rows {
            conn.execute(
                "INSERT INTO group_members (group_name, player_normalized, added_at, kind) \
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![g, k, "2026-01-01T00:00:00Z", kind],
            )
            .unwrap();
        }
        super::run_migrations(&conn).expect("006 runs cleanly");

        let count_players: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM group_members WHERE entity_ref LIKE 'player:%'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let count_teams: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM group_members WHERE entity_ref LIKE 'team:%'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count_players, 5, "5 player rows backfilled");
        assert_eq!(count_teams, 3, "3 team rows backfilled");
    }

    /// 006.4 — FK cascade still fires after the table rebuild.
    /// Deleting the group must cascade-delete its members.
    #[test]
    fn l1_db_006_fk_cascade_survives_rebuild() {
        let db = GroupDb::open_in_memory().expect("open");
        db.create_group("Favorites", "").unwrap();
        db.add_member("Favorites", "connor mcdavid").unwrap();
        db.add_member_kind("Favorites", "EDM", MemberKind::Team)
            .unwrap();
        assert_eq!(db.list_members_with_kind("Favorites").unwrap().len(), 2);

        db.delete_group("Favorites").unwrap();

        let count: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM group_members", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0, "FK cascade emptied group_members");
    }

    /// 006.5 — new PK semantics: the same key under two different
    /// kinds is now legal (player:EDM and team:EDM can coexist),
    /// where pre-006 they collided on PK (group_name,
    /// player_normalized).
    #[test]
    fn l1_db_006_same_key_different_kinds_coexist() {
        let db = GroupDb::open_in_memory().expect("open");
        db.create_group("Edge", "").unwrap();
        // Hypothetical: a player whose normalized name is "EDM" (no
        // such NHL player exists, but the schema must allow it now).
        db.add_member_kind("Edge", "EDM", MemberKind::Player)
            .unwrap();
        db.add_member_kind("Edge", "EDM", MemberKind::Team).unwrap();
        let kinded = db.list_members_with_kind("Edge").unwrap();
        assert_eq!(kinded.len(), 2, "same-key different-kind both stored");
    }
}
