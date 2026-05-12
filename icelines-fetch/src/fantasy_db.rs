//! SQLite-backed fantasy league store for `icelines fantasy` commands.
//!
//! Opens (or creates) `~/.icelines/icelines.db` and runs embedded migrations
//! on every startup.  Use `FantasyDb::open_in_memory()` in unit tests.

use anyhow::{bail, Context};
use rusqlite::Connection;

// ── Public types ──────────────────────────────────────────────────────────────

/// A row returned by `list_leagues`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeagueRow {
    pub id: String,
    pub name: String,
    pub scheme: String,
    pub is_active: bool,
    pub team_count: usize,
}

/// A row returned by `list_teams` / `get_team_by_name`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamRow {
    pub id: String,
    pub name: String,
    pub owner: String,
    pub is_user_team: bool,
    pub player_count: usize,
}

/// A full fantasy league snapshot: league metadata, user team, teams, and rosters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FantasyLeagueSnapshot {
    pub league: String,
    pub user_team: String,
    pub scoring_scheme: String,
    pub teams: Vec<FantasyTeamSnapshot>,
}

/// One team plus its normalized roster names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FantasyTeamSnapshot {
    pub name: String,
    pub owner: String,
    pub roster: Vec<String>,
}

impl FantasyLeagueSnapshot {
    pub fn all_rostered(&self) -> Vec<String> {
        self.teams
            .iter()
            .flat_map(|team| team.roster.iter().cloned())
            .collect()
    }

    pub fn user_rostered(&self) -> Vec<String> {
        self.teams
            .iter()
            .find(|team| team.name == self.user_team)
            .map(|team| team.roster.clone())
            .unwrap_or_default()
    }
}

/// Opaque handle to the fantasy database.
pub struct FantasyDb {
    conn: Connection,
    /// Absolute path used to re-open this DB (needed by the HTTP server).
    pub db_path: std::path::PathBuf,
}

// ── Migrations ────────────────────────────────────────────────────────────────

fn run_migrations(conn: &Connection) -> anyhow::Result<()> {
    // Prior migrations (001, 002) live in db.rs / GroupDb — we share the same
    // file.  Our fantasy migrations are 003-005.

    // Migration 003 — fantasy leagues
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS fl_leagues (
            id         TEXT PRIMARY KEY,
            name       TEXT UNIQUE NOT NULL,
            scheme     TEXT NOT NULL DEFAULT 'yahoo-standard',
            is_active  INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        );",
    )
    .context("migration 003: create fl_leagues table")?;

    // Migration 004 — fantasy teams
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS fl_teams (
            id         TEXT PRIMARY KEY,
            league_id  TEXT NOT NULL,
            name       TEXT NOT NULL,
            owner      TEXT NOT NULL DEFAULT '',
            is_user_team INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            FOREIGN KEY(league_id) REFERENCES fl_leagues(id) ON DELETE CASCADE,
            UNIQUE(league_id, name)
        );",
    )
    .context("migration 004: create fl_teams table")?;

    let has_user_team_column: bool = conn
        .prepare("PRAGMA table_info(fl_teams)")
        .context("migration 006: inspect fl_teams")?
        .query_map([], |row| row.get::<_, String>(1))
        .context("migration 006: read fl_teams columns")?
        .collect::<Result<Vec<_>, _>>()
        .context("migration 006: collect fl_teams columns")?
        .iter()
        .any(|name| name == "is_user_team");
    if !has_user_team_column {
        conn.execute_batch(
            "ALTER TABLE fl_teams ADD COLUMN is_user_team INTEGER NOT NULL DEFAULT 0;",
        )
        .context("migration 006: add fl_teams.is_user_team")?;
    }

    // Migration 005 — fantasy rosters
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS fl_roster (
            team_id          TEXT NOT NULL,
            player_normalized TEXT NOT NULL,
            added_at         TEXT NOT NULL,
            PRIMARY KEY(team_id, player_normalized),
            FOREIGN KEY(team_id) REFERENCES fl_teams(id) ON DELETE CASCADE
        );",
    )
    .context("migration 005: create fl_roster table")?;

    // Enable foreign-key enforcement (off by default in rusqlite).
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .context("enable foreign keys")?;

    Ok(())
}

// ── FantasyDb impl ────────────────────────────────────────────────────────────

impl FantasyDb {
    /// Open (or create) `~/.icelines/icelines.db` and run migrations.
    pub fn open() -> anyhow::Result<Self> {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(std::path::PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;

        let dir = home.join(".icelines");
        std::fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;

        let db_path = dir.join("icelines.db");
        Self::open_path(db_path)
    }

    /// Open a database at the given path (used by the HTTP server).
    pub fn open_path(db_path: std::path::PathBuf) -> anyhow::Result<Self> {
        let conn =
            Connection::open(&db_path).with_context(|| format!("open {}", db_path.display()))?;

        // Enable WAL for better concurrent write performance.
        conn.execute_batch("PRAGMA journal_mode = WAL;")
            .context("set WAL mode")?;

        run_migrations(&conn)?;
        Ok(Self { conn, db_path })
    }

    /// Open an in-memory database for unit tests.
    #[cfg(test)]
    pub fn open_in_memory() -> anyhow::Result<Self> {
        let conn = Connection::open_in_memory().context("open in-memory db")?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .context("enable foreign keys")?;
        run_migrations(&conn)?;
        Ok(Self {
            conn,
            db_path: std::path::PathBuf::from(":memory:"),
        })
    }

    // ── League operations ──────────────────────────────────────────────────────

    /// Create a new fantasy league.  Returns its UUID.
    pub fn create_league(&self, name: &str, scheme: &str) -> anyhow::Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO fl_leagues (id, name, scheme, is_active, created_at) \
                 VALUES (?1, ?2, ?3, 0, ?4)",
                rusqlite::params![id, name, scheme, now],
            )
            .with_context(|| format!("create league '{name}'"))?;
        Ok(id)
    }

    /// List all leagues with team counts.
    pub fn list_leagues(&self) -> anyhow::Result<Vec<LeagueRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT l.id, l.name, l.scheme, l.is_active,
                    COUNT(t.id) AS team_count
             FROM fl_leagues l
             LEFT JOIN fl_teams t ON t.league_id = l.id
             GROUP BY l.id
             ORDER BY l.name",
        )?;

        let rows = stmt
            .query_map([], |row| {
                Ok(LeagueRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    scheme: row.get(2)?,
                    is_active: row.get::<_, i64>(3)? != 0,
                    team_count: row.get::<_, i64>(4)? as usize,
                })
            })
            .context("list_leagues query")?
            .collect::<Result<Vec<_>, _>>()
            .context("list_leagues collect")?;

        Ok(rows)
    }

    /// Set the active league by name (clears all others).
    pub fn set_active_league(&self, name: &str) -> anyhow::Result<()> {
        // Verify it exists first.
        let exists: bool = self
            .conn
            .query_row(
                "SELECT 1 FROM fl_leagues WHERE name = ?1",
                rusqlite::params![name],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if !exists {
            bail!("league '{name}' not found");
        }

        self.conn
            .execute_batch("UPDATE fl_leagues SET is_active = 0")
            .context("clear active flag")?;
        self.conn
            .execute(
                "UPDATE fl_leagues SET is_active = 1 WHERE name = ?1",
                rusqlite::params![name],
            )
            .with_context(|| format!("set active league '{name}'"))?;
        Ok(())
    }

    /// Get the currently active league (if any).
    pub fn get_active_league(&self) -> anyhow::Result<Option<LeagueRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT l.id, l.name, l.scheme, l.is_active,
                    COUNT(t.id) AS team_count
             FROM fl_leagues l
             LEFT JOIN fl_teams t ON t.league_id = l.id
             WHERE l.is_active = 1
             GROUP BY l.id",
        )?;

        let mut rows = stmt
            .query_map([], |row| {
                Ok(LeagueRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    scheme: row.get(2)?,
                    is_active: row.get::<_, i64>(3)? != 0,
                    team_count: row.get::<_, i64>(4)? as usize,
                })
            })
            .context("get_active_league query")?;

        rows.next().transpose().context("get_active_league row")
    }

    /// Delete a league by name (cascades to teams and rosters).
    /// Returns `true` if it existed, `false` if not found.
    pub fn delete_league(&self, name: &str) -> anyhow::Result<bool> {
        let rows = self
            .conn
            .execute(
                "DELETE FROM fl_leagues WHERE name = ?1",
                rusqlite::params![name],
            )
            .with_context(|| format!("delete league '{name}'"))?;
        Ok(rows > 0)
    }

    // ── Team operations ────────────────────────────────────────────────────────

    /// Create a team inside the given league.  Returns its UUID.
    pub fn create_team(&self, league_id: &str, name: &str, owner: &str) -> anyhow::Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO fl_teams (id, league_id, name, owner, created_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![id, league_id, name, owner, now],
            )
            .with_context(|| format!("create team '{name}'"))?;
        Ok(id)
    }

    /// List all teams in a league with player counts.
    pub fn list_teams(&self, league_id: &str) -> anyhow::Result<Vec<TeamRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.name, t.owner, t.is_user_team, COUNT(r.player_normalized) AS player_count
             FROM fl_teams t
             LEFT JOIN fl_roster r ON r.team_id = t.id
             WHERE t.league_id = ?1
             GROUP BY t.id
             ORDER BY t.name",
        )?;

        let rows = stmt
            .query_map(rusqlite::params![league_id], |row| {
                Ok(TeamRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    owner: row.get(2)?,
                    is_user_team: row.get::<_, i64>(3)? != 0,
                    player_count: row.get::<_, i64>(4)? as usize,
                })
            })
            .context("list_teams query")?
            .collect::<Result<Vec<_>, _>>()
            .context("list_teams collect")?;

        Ok(rows)
    }

    /// Look up a team by name within a league.
    pub fn get_team_by_name(&self, league_id: &str, name: &str) -> anyhow::Result<Option<TeamRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.name, t.owner, t.is_user_team, COUNT(r.player_normalized) AS player_count
             FROM fl_teams t
             LEFT JOIN fl_roster r ON r.team_id = t.id
             WHERE t.league_id = ?1 AND t.name = ?2
             GROUP BY t.id",
        )?;

        let mut rows = stmt
            .query_map(rusqlite::params![league_id, name], |row| {
                Ok(TeamRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    owner: row.get(2)?,
                    is_user_team: row.get::<_, i64>(3)? != 0,
                    player_count: row.get::<_, i64>(4)? as usize,
                })
            })
            .context("get_team_by_name query")?;

        rows.next().transpose().context("get_team_by_name row")
    }

    /// Mark one team in a league as the user's roster.
    pub fn set_user_team(&self, league_id: &str, name: &str) -> anyhow::Result<bool> {
        let exists = self.get_team_by_name(league_id, name)?.is_some();
        if !exists {
            return Ok(false);
        }
        self.conn
            .execute(
                "UPDATE fl_teams SET is_user_team = 0 WHERE league_id = ?1",
                rusqlite::params![league_id],
            )
            .context("clear user team")?;
        self.conn
            .execute(
                "UPDATE fl_teams SET is_user_team = 1 WHERE league_id = ?1 AND name = ?2",
                rusqlite::params![league_id, name],
            )
            .with_context(|| format!("set user team '{name}'"))?;
        Ok(true)
    }

    /// Return the user's team for a league, if configured.
    pub fn get_user_team(&self, league_id: &str) -> anyhow::Result<Option<TeamRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.name, t.owner, t.is_user_team, COUNT(r.player_normalized) AS player_count
             FROM fl_teams t
             LEFT JOIN fl_roster r ON r.team_id = t.id
             WHERE t.league_id = ?1 AND t.is_user_team = 1
             GROUP BY t.id
             LIMIT 1",
        )?;

        let mut rows = stmt
            .query_map(rusqlite::params![league_id], |row| {
                Ok(TeamRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    owner: row.get(2)?,
                    is_user_team: row.get::<_, i64>(3)? != 0,
                    player_count: row.get::<_, i64>(4)? as usize,
                })
            })
            .context("get_user_team query")?;

        rows.next().transpose().context("get_user_team row")
    }

    /// Delete a team (cascades to its roster).
    /// Returns `true` if it existed, `false` if not found.
    pub fn delete_team(&self, league_id: &str, name: &str) -> anyhow::Result<bool> {
        let rows = self
            .conn
            .execute(
                "DELETE FROM fl_teams WHERE league_id = ?1 AND name = ?2",
                rusqlite::params![league_id, name],
            )
            .with_context(|| format!("delete team '{name}'"))?;
        Ok(rows > 0)
    }

    // ── Roster operations ──────────────────────────────────────────────────────

    /// Add a player (by normalized name) to a team roster.
    pub fn add_player(&self, team_id: &str, player_normalized: &str) -> anyhow::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT OR IGNORE INTO fl_roster (team_id, player_normalized, added_at) \
                 VALUES (?1, ?2, ?3)",
                rusqlite::params![team_id, player_normalized, now],
            )
            .with_context(|| format!("add player '{player_normalized}' to team {team_id}"))?;
        Ok(())
    }

    /// Drop a player from a team roster.
    /// Returns `true` if they were on the roster, `false` otherwise.
    pub fn drop_player(&self, team_id: &str, player_normalized: &str) -> anyhow::Result<bool> {
        let rows = self
            .conn
            .execute(
                "DELETE FROM fl_roster WHERE team_id = ?1 AND player_normalized = ?2",
                rusqlite::params![team_id, player_normalized],
            )
            .with_context(|| format!("drop player '{player_normalized}' from team {team_id}"))?;
        Ok(rows > 0)
    }

    /// List all normalized player names on a team's roster.
    pub fn list_roster(&self, team_id: &str) -> anyhow::Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT player_normalized FROM fl_roster WHERE team_id = ?1 ORDER BY added_at",
        )?;

        let rows = stmt
            .query_map(rusqlite::params![team_id], |row| row.get(0))
            .context("list_roster query")?
            .collect::<Result<Vec<String>, _>>()
            .context("list_roster collect")?;

        Ok(rows)
    }

    /// Check whether a player (by normalized name) is already on any team in
    /// the given league.  Returns `Some(team_name)` if taken, `None` if free.
    pub fn is_on_any_team(
        &self,
        league_id: &str,
        player_normalized: &str,
    ) -> anyhow::Result<Option<String>> {
        let result = self
            .conn
            .query_row(
                "SELECT t.name
                 FROM fl_roster r
                 JOIN fl_teams t ON t.id = r.team_id
                 WHERE t.league_id = ?1 AND r.player_normalized = ?2
                 LIMIT 1",
                rusqlite::params![league_id, player_normalized],
                |row| row.get::<_, String>(0),
            )
            .ok();
        Ok(result)
    }

    /// Load a full fantasy league snapshot for projection/surface viewmodels.
    pub fn league_snapshot(
        &self,
        league_name: Option<&str>,
    ) -> anyhow::Result<FantasyLeagueSnapshot> {
        let league = if let Some(name) = league_name {
            self.list_leagues()?
                .into_iter()
                .find(|league| league.name == name)
                .ok_or_else(|| anyhow::anyhow!("fantasy league '{name}' not found"))?
        } else {
            self.get_active_league()?
                .ok_or_else(|| anyhow::anyhow!("no active fantasy league found"))?
        };
        let user_team = self.get_user_team(&league.id)?.ok_or_else(|| {
            anyhow::anyhow!(
                "no user team marked in '{}'; run `icelines fantasy team-use <name>`",
                league.name
            )
        })?;
        let teams = self
            .list_teams(&league.id)?
            .into_iter()
            .map(|team| {
                let roster = self.list_roster(&team.id)?;
                Ok(FantasyTeamSnapshot {
                    name: team.name,
                    owner: team.owner,
                    roster,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        if !teams.iter().any(|team| team.name == user_team.name) {
            bail!(
                "user team '{}' was not found in '{}'",
                user_team.name,
                league.name
            );
        }
        Ok(FantasyLeagueSnapshot {
            league: league.name,
            user_team: user_team.name,
            scoring_scheme: league.scheme,
            teams,
        })
    }
}

// ── Unit tests (L1) ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l1_fantasy_create_league() {
        let db = FantasyDb::open_in_memory().expect("open in-memory db");
        let id = db
            .create_league("My League", "yahoo-standard")
            .expect("create league");
        assert!(!id.is_empty(), "id should be a non-empty UUID");

        let leagues = db.list_leagues().expect("list leagues");
        assert_eq!(leagues.len(), 1);
        assert_eq!(leagues[0].name, "My League");
        assert_eq!(leagues[0].scheme, "yahoo-standard");
        assert!(!leagues[0].is_active);
        assert_eq!(leagues[0].team_count, 0);
    }

    #[test]
    fn l1_fantasy_create_and_list_teams() {
        let db = FantasyDb::open_in_memory().expect("open in-memory db");
        let league_id = db
            .create_league("Test League", "espn-standard")
            .expect("create league");

        let t1 = db
            .create_team(&league_id, "Rangers Fans", "Gio")
            .expect("create team 1");
        let t2 = db
            .create_team(&league_id, "Oilers Army", "Wayne")
            .expect("create team 2");
        assert_ne!(t1, t2, "team IDs should be unique");

        let teams = db.list_teams(&league_id).expect("list teams");
        assert_eq!(teams.len(), 2);
        assert_eq!(teams[0].player_count, 0);
        assert!(!teams[0].is_user_team);

        // Confirm get_team_by_name works.
        let found = db
            .get_team_by_name(&league_id, "Rangers Fans")
            .expect("get team by name");
        assert!(found.is_some());
        assert_eq!(found.unwrap().owner, "Gio");

        let not_found = db
            .get_team_by_name(&league_id, "Canucks Fans")
            .expect("lookup missing team");
        assert!(not_found.is_none());
    }

    #[test]
    fn l1_fantasy_marks_one_user_team_per_league() {
        let db = FantasyDb::open_in_memory().expect("open in-memory db");
        let league_id = db
            .create_league("User Team League", "yahoo-standard")
            .expect("create league");
        db.create_team(&league_id, "My Team", "Me")
            .expect("create my team");
        db.create_team(&league_id, "Other Team", "Them")
            .expect("create other team");

        assert!(db
            .set_user_team(&league_id, "My Team")
            .expect("set user team"));
        let user_team = db
            .get_user_team(&league_id)
            .expect("get user team")
            .expect("user team exists");
        assert_eq!(user_team.name, "My Team");
        assert!(user_team.is_user_team);

        assert!(db
            .set_user_team(&league_id, "Other Team")
            .expect("switch user team"));
        let teams = db.list_teams(&league_id).expect("list teams");
        assert_eq!(teams.iter().filter(|team| team.is_user_team).count(), 1);
        assert_eq!(
            teams
                .iter()
                .find(|team| team.is_user_team)
                .map(|team| team.name.as_str()),
            Some("Other Team")
        );

        assert!(!db
            .set_user_team(&league_id, "Ghost Team")
            .expect("missing team returns false"));
    }

    #[test]
    fn l1_fantasy_add_drop_player() {
        let db = FantasyDb::open_in_memory().expect("open in-memory db");
        let league_id = db
            .create_league("Draft League", "yahoo-standard")
            .expect("create league");
        let team_id = db
            .create_team(&league_id, "My Team", "Me")
            .expect("create team");

        db.add_player(&team_id, "connor_mcdavid")
            .expect("add player");
        db.add_player(&team_id, "leon_draisaitl")
            .expect("add player");

        let roster = db.list_roster(&team_id).expect("list roster");
        assert_eq!(roster.len(), 2);
        assert!(roster.contains(&"connor_mcdavid".to_owned()));

        let dropped = db
            .drop_player(&team_id, "leon_draisaitl")
            .expect("drop player");
        assert!(dropped);

        let roster2 = db.list_roster(&team_id).expect("list roster after drop");
        assert_eq!(roster2.len(), 1);
        assert_eq!(roster2[0], "connor_mcdavid");

        // Dropping a player not on the team returns false.
        let noop = db
            .drop_player(&team_id, "auston_matthews")
            .expect("drop non-existent");
        assert!(!noop);
    }

    #[test]
    fn l1_fantasy_active_league() {
        let db = FantasyDb::open_in_memory().expect("open in-memory db");
        db.create_league("Alpha", "yahoo-standard")
            .expect("create alpha");
        db.create_league("Beta", "espn-standard")
            .expect("create beta");

        // Nothing active yet.
        let active = db.get_active_league().expect("get active");
        assert!(active.is_none());

        db.set_active_league("Beta").expect("set active");
        let active = db.get_active_league().expect("get active");
        assert!(active.is_some());
        assert_eq!(active.unwrap().name, "Beta");

        // Switch to Alpha — Beta should be cleared.
        db.set_active_league("Alpha").expect("switch active");
        let active = db.get_active_league().expect("get active after switch");
        assert_eq!(active.unwrap().name, "Alpha");

        // Delete Alpha — active goes away.
        db.delete_league("Alpha").expect("delete alpha");
        let active = db.get_active_league().expect("get active after delete");
        assert!(active.is_none());
    }

    #[test]
    fn l1_fantasy_player_already_taken() {
        let db = FantasyDb::open_in_memory().expect("open in-memory db");
        let league_id = db
            .create_league("Keeper League", "yahoo-standard")
            .expect("create league");
        let team1 = db
            .create_team(&league_id, "Team One", "Alice")
            .expect("create team 1");
        let _team2 = db
            .create_team(&league_id, "Team Two", "Bob")
            .expect("create team 2");

        db.add_player(&team1, "matty_beniers")
            .expect("add player to team1");

        // is_on_any_team should return Some("Team One") for matty_beniers.
        let taken = db
            .is_on_any_team(&league_id, "matty_beniers")
            .expect("is_on_any_team");
        assert!(taken.is_some(), "player should be taken");
        assert_eq!(taken.unwrap(), "Team One");

        // Free agent is not on any team.
        let free = db
            .is_on_any_team(&league_id, "free_agent_guy")
            .expect("free agent check");
        assert!(free.is_none());
    }

    // ── Additional L1: gaps ───────────────────────────────────────────────────

    #[test]
    fn l1_fantasy_duplicate_league_name_errors() {
        let db = FantasyDb::open_in_memory().expect("open in-memory db");
        db.create_league("Same Name", "yahoo-standard")
            .expect("first create should succeed");
        let result = db.create_league("Same Name", "espn-standard");
        assert!(
            result.is_err(),
            "creating league with duplicate name must return Err"
        );
    }

    #[test]
    fn l1_fantasy_delete_league_cascades_to_teams_and_rosters() {
        let db = FantasyDb::open_in_memory().expect("open in-memory db");
        let league_id = db
            .create_league("Cascade League", "yahoo-standard")
            .expect("create league");
        let team_id = db
            .create_team(&league_id, "Alpha Team", "Alice")
            .expect("create team");
        db.add_player(&team_id, "player_one").expect("add player");
        db.add_player(&team_id, "player_two").expect("add player");

        // Verify they exist before deletion
        let roster_before = db.list_roster(&team_id).expect("list roster before");
        assert_eq!(roster_before.len(), 2);

        // Delete the league — should cascade
        let deleted = db.delete_league("Cascade League").expect("delete league");
        assert!(deleted, "delete must return true when league existed");

        // League is gone
        let leagues = db.list_leagues().expect("list leagues after delete");
        assert!(
            leagues.iter().all(|l| l.name != "Cascade League"),
            "league must not appear after deletion"
        );

        // Roster rows are cascade-deleted (foreign key cascade)
        let roster_count: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM fl_roster", [], |r| r.get(0))
            .expect("count roster");
        assert_eq!(
            roster_count, 0,
            "roster rows must cascade-delete with league"
        );
    }

    #[test]
    fn l1_fantasy_delete_nonexistent_league_returns_false() {
        let db = FantasyDb::open_in_memory().expect("open in-memory db");
        let deleted = db.delete_league("No Such League").expect("delete call");
        assert!(!deleted, "deleting nonexistent league should return false");
    }

    #[test]
    fn l1_fantasy_delete_team_cascades_roster() {
        let db = FantasyDb::open_in_memory().expect("open in-memory db");
        let league_id = db
            .create_league("Team Cascade", "espn-standard")
            .expect("create league");
        let team_id = db
            .create_team(&league_id, "Drop Team", "Bob")
            .expect("create team");
        db.add_player(&team_id, "connor_mcdavid")
            .expect("add player");

        let deleted = db
            .delete_team(&league_id, "Drop Team")
            .expect("delete team");
        assert!(deleted, "delete_team should return true when team existed");

        // Team is gone — list_teams should be empty
        let teams = db.list_teams(&league_id).expect("list teams after delete");
        assert!(teams.is_empty(), "no teams should remain");

        // Roster rows cascade-deleted
        let roster_count: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM fl_roster", [], |r| r.get(0))
            .expect("count roster");
        assert_eq!(roster_count, 0, "roster must cascade-delete with team");
    }

    #[test]
    fn l1_fantasy_duplicate_player_on_same_team_is_noop() {
        let db = FantasyDb::open_in_memory().expect("open in-memory db");
        let league_id = db
            .create_league("Dup Player League", "yahoo-standard")
            .expect("create league");
        let team_id = db
            .create_team(&league_id, "My Team", "Alice")
            .expect("create team");

        db.add_player(&team_id, "auston_matthews")
            .expect("first add");
        db.add_player(&team_id, "auston_matthews")
            .expect("duplicate add must not error (INSERT OR IGNORE)");

        let roster = db.list_roster(&team_id).expect("list roster");
        assert_eq!(roster.len(), 1, "duplicate player should appear only once");
    }

    #[test]
    fn l1_fantasy_list_leagues_empty_initially() {
        let db = FantasyDb::open_in_memory().expect("open in-memory db");
        let leagues = db.list_leagues().expect("list leagues");
        assert!(leagues.is_empty(), "new db should have no leagues");
    }

    #[test]
    fn l1_fantasy_set_active_unknown_league_errors() {
        let db = FantasyDb::open_in_memory().expect("open in-memory db");
        let result = db.set_active_league("Ghost League");
        assert!(
            result.is_err(),
            "setting active on nonexistent league must error"
        );
    }

    #[test]
    fn l1_fantasy_multiple_leagues_independent() {
        // Two leagues must not share teams or players
        let db = FantasyDb::open_in_memory().expect("open in-memory db");
        let l1 = db
            .create_league("League One", "yahoo-standard")
            .expect("create league 1");
        let l2 = db
            .create_league("League Two", "espn-standard")
            .expect("create league 2");

        let t1 = db.create_team(&l1, "Alpha", "Alice").expect("team in l1");
        let t2 = db.create_team(&l2, "Beta", "Bob").expect("team in l2");

        db.add_player(&t1, "player_a").expect("add to l1");
        db.add_player(&t2, "player_b").expect("add to l2");

        // l1 has player_a, not player_b
        let taken_l1 = db.is_on_any_team(&l1, "player_a").expect("check l1");
        let not_taken_l1 = db.is_on_any_team(&l1, "player_b").expect("check l1");
        assert!(taken_l1.is_some(), "player_a must be in l1");
        assert!(not_taken_l1.is_none(), "player_b must NOT be in l1");

        // l2 has player_b, not player_a
        let taken_l2 = db.is_on_any_team(&l2, "player_b").expect("check l2");
        let not_taken_l2 = db.is_on_any_team(&l2, "player_a").expect("check l2");
        assert!(taken_l2.is_some(), "player_b must be in l2");
        assert!(not_taken_l2.is_none(), "player_a must NOT be in l2");
    }

    #[test]
    fn l1_fantasy_league_snapshot_loads_user_team_and_rosters() {
        let db = FantasyDb::open_in_memory().expect("open in-memory db");
        let league_id = db
            .create_league("Snapshot League", "yahoo-standard")
            .expect("create league");
        db.set_active_league("Snapshot League")
            .expect("set active league");
        let mine = db
            .create_team(&league_id, "My Team", "Me")
            .expect("create my team");
        let rival = db
            .create_team(&league_id, "Rival Team", "Them")
            .expect("create rival team");
        db.set_user_team(&league_id, "My Team")
            .expect("set user team");
        db.add_player(&mine, "connor_mcdavid")
            .expect("add mine player");
        db.add_player(&rival, "nathan_mackinnon")
            .expect("add rival player");

        let snapshot = db.league_snapshot(None).expect("snapshot");

        assert_eq!(snapshot.league, "Snapshot League");
        assert_eq!(snapshot.user_team, "My Team");
        assert_eq!(snapshot.scoring_scheme, "yahoo-standard");
        assert_eq!(snapshot.teams.len(), 2);
        assert!(snapshot
            .teams
            .iter()
            .any(|team| team.name == "My Team" && team.roster == ["connor_mcdavid"]));
        assert!(snapshot
            .teams
            .iter()
            .any(|team| team.name == "Rival Team" && team.roster == ["nathan_mackinnon"]));
        assert_eq!(snapshot.user_rostered(), vec!["connor_mcdavid"]);
        assert_eq!(snapshot.all_rostered().len(), 2);
    }
}
