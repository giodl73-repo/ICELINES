//! SQLite-backed fantasy league store for `icelines fantasy` commands.
//!
//! Opens (or creates) `~/.icelines/icelines.db` and runs embedded migrations
//! on every startup.  Use `FantasyDb::open_in_memory()` in unit tests.

use anyhow::{bail, Context};
use chrono::{DateTime, NaiveDate, Utc};
use icelines_core::{
    model::Position, FantasyAcquisitionKind, FantasyAssistantRules, FantasyCompetitionMode,
    FantasyCompetitionRules, FantasyGoalieStartObservation, FantasyGoalieStartState,
    FantasyObservationConfidence, FantasyPlayerAvailabilityStatus, FantasyStatusObservation,
    FantasyWaiverWindow, RosterShape, RosterShapePlayerInput, RosterShapeValidationInput,
    RosterShapeValidationView,
};
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use std::collections::{BTreeMap, BTreeSet};

pub const DEFAULT_ROSTER_SHAPE: &str = "yahoo-standard";

// ── Public types ──────────────────────────────────────────────────────────────

/// A row returned by `list_leagues`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeagueRow {
    pub id: String,
    pub name: String,
    pub scheme: String,
    pub roster_shape: String,
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
    pub roster_shape: String,
    pub teams: Vec<FantasyTeamSnapshot>,
}

/// One team plus its normalized roster names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FantasyTeamSnapshot {
    pub name: String,
    pub owner: String,
    pub roster: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FantasyMatchupScheduleRow {
    pub id: String,
    pub week_start: NaiveDate,
    pub home_team: String,
    pub away_team: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FantasyPlayerEligibilityRow {
    pub player_normalized: String,
    pub positions: Vec<Position>,
    pub source: String,
    pub fetched_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FantasyAcquisitionLedgerRow {
    pub id: String,
    pub player_added: String,
    pub player_dropped: Option<String>,
    pub kind: FantasyAcquisitionKind,
    pub effective_at: DateTime<Utc>,
    pub counts_toward_limit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct FantasyTradeHistoryRow {
    pub id: String,
    pub league_id: String,
    pub sending_team_id: String,
    pub sending_team: String,
    pub receiving_team_id: String,
    pub receiving_team: String,
    pub sends: Vec<String>,
    pub receives: Vec<String>,
    pub executed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct FantasyTradeOfferRow {
    pub id: String,
    pub league_id: String,
    pub sending_team_id: String,
    pub sending_team: String,
    pub receiving_team_id: String,
    pub receiving_team: String,
    pub sends: Vec<String>,
    pub receives: Vec<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub roster_current: bool,
    pub roster_issues: Vec<String>,
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
            roster_shape TEXT NOT NULL DEFAULT 'yahoo-standard',
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

    let has_roster_shape_column = table_has_column(conn, "fl_leagues", "roster_shape")
        .context("migration 008: inspect fl_leagues.roster_shape")?;
    if !has_roster_shape_column {
        conn.execute_batch(
            "ALTER TABLE fl_leagues ADD COLUMN roster_shape TEXT NOT NULL DEFAULT 'yahoo-standard';",
        )
        .context("migration 008: add fl_leagues.roster_shape")?;
    }

    let has_competition_mode_column = table_has_column(conn, "fl_leagues", "competition_mode")
        .context("migration 017: inspect fl_leagues.competition_mode")?;
    if !has_competition_mode_column {
        conn.execute_batch(
            "ALTER TABLE fl_leagues ADD COLUMN competition_mode TEXT NOT NULL DEFAULT 'points';",
        )
        .context("migration 017: add fl_leagues.competition_mode")?;
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

    // Migration 007 — local fantasy matchup schedule.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS fl_matchups (
            id           TEXT PRIMARY KEY,
            league_id    TEXT NOT NULL,
            week_start   TEXT NOT NULL,
            home_team_id TEXT NOT NULL,
            away_team_id TEXT,
            created_at   TEXT NOT NULL,
            FOREIGN KEY(league_id) REFERENCES fl_leagues(id) ON DELETE CASCADE,
            FOREIGN KEY(home_team_id) REFERENCES fl_teams(id) ON DELETE CASCADE,
            FOREIGN KEY(away_team_id) REFERENCES fl_teams(id) ON DELETE CASCADE,
            UNIQUE(league_id, week_start, home_team_id)
        );",
    )
    .context("migration 007: create fl_matchups table")?;

    // Migration 009 — league-specific draft/daily assistant rules.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS fl_assistant_settings (
            league_id   TEXT PRIMARY KEY,
            rules_json  TEXT NOT NULL,
            updated_at  TEXT NOT NULL,
            FOREIGN KEY(league_id) REFERENCES fl_leagues(id) ON DELETE CASCADE
        );",
    )
    .context("migration 009: create fl_assistant_settings table")?;

    // Migration 010 — fantasy-platform eligibility, separate from canonical NHL position.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS fl_player_eligibility (
            league_id          TEXT NOT NULL,
            player_normalized  TEXT NOT NULL,
            positions_json     TEXT NOT NULL,
            source             TEXT NOT NULL,
            fetched_at         TEXT NOT NULL,
            PRIMARY KEY(league_id, player_normalized),
            FOREIGN KEY(league_id) REFERENCES fl_leagues(id) ON DELETE CASCADE
        );",
    )
    .context("migration 010: create fl_player_eligibility table")?;

    // Migration 011 — acquisition ledger used for the Monday-Sunday hard limit.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS fl_acquisitions (
            id                   TEXT PRIMARY KEY,
            league_id            TEXT NOT NULL,
            player_added         TEXT NOT NULL,
            player_dropped       TEXT,
            kind                 TEXT NOT NULL,
            effective_at         TEXT NOT NULL,
            counts_toward_limit  INTEGER NOT NULL DEFAULT 1,
            FOREIGN KEY(league_id) REFERENCES fl_leagues(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_fl_acquisitions_league_effective
            ON fl_acquisitions(league_id, effective_at);",
    )
    .context("migration 011: create fl_acquisitions table")?;

    // Migration 012 — latest dropped-player waiver clearance per league.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS fl_waivers (
            league_id            TEXT NOT NULL,
            player_normalized    TEXT NOT NULL,
            dropped_at           TEXT NOT NULL,
            clears_at            TEXT NOT NULL,
            PRIMARY KEY(league_id, player_normalized),
            FOREIGN KEY(league_id) REFERENCES fl_leagues(id) ON DELETE CASCADE
        );",
    )
    .context("migration 012: create fl_waivers table")?;

    // Migration 013 — sourced, time-bounded player availability observations.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS fl_status_observations (
            id                  TEXT PRIMARY KEY,
            league_id           TEXT NOT NULL,
            player_normalized   TEXT NOT NULL,
            status              TEXT NOT NULL,
            source              TEXT NOT NULL,
            source_url          TEXT,
            observed_at         TEXT NOT NULL,
            fetched_at          TEXT NOT NULL,
            confidence          TEXT NOT NULL,
            detail              TEXT,
            FOREIGN KEY(league_id) REFERENCES fl_leagues(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_fl_status_league_player_observed
            ON fl_status_observations(league_id, player_normalized, observed_at DESC);",
    )
    .context("migration 013: create fl_status_observations table")?;

    // Migration 014 — last emitted material morning decision per league/day.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS fl_morning_briefings (
            league_id       TEXT NOT NULL,
            briefing_date   TEXT NOT NULL,
            fingerprint     TEXT NOT NULL,
            generated_at    TEXT NOT NULL,
            PRIMARY KEY(league_id, briefing_date),
            FOREIGN KEY(league_id) REFERENCES fl_leagues(id) ON DELETE CASCADE
        );",
    )
    .context("migration 014: create fl_morning_briefings table")?;

    // Migration 015 — atomic local fantasy trade audit trail.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS fl_trade_history (
            id                 TEXT PRIMARY KEY,
            league_id          TEXT NOT NULL,
            sending_team_id    TEXT NOT NULL,
            sending_team_name  TEXT NOT NULL,
            receiving_team_id  TEXT NOT NULL,
            receiving_team_name TEXT NOT NULL,
            sends_json         TEXT NOT NULL,
            receives_json      TEXT NOT NULL,
            executed_at        TEXT NOT NULL,
            FOREIGN KEY(league_id) REFERENCES fl_leagues(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_fl_trade_history_league_executed
            ON fl_trade_history(league_id, executed_at DESC);",
    )
    .context("migration 015: create fl_trade_history table")?;

    // Migration 016 — proposed fantasy trade lifecycle, separate from execution.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS fl_trade_offers (
            id                  TEXT PRIMARY KEY,
            league_id           TEXT NOT NULL,
            sending_team_id     TEXT NOT NULL,
            sending_team_name   TEXT NOT NULL,
            receiving_team_id   TEXT NOT NULL,
            receiving_team_name TEXT NOT NULL,
            sends_json          TEXT NOT NULL,
            receives_json       TEXT NOT NULL,
            status              TEXT NOT NULL,
            created_at          TEXT NOT NULL,
            updated_at          TEXT NOT NULL,
            FOREIGN KEY(league_id) REFERENCES fl_leagues(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_fl_trade_offers_league_status_updated
            ON fl_trade_offers(league_id, status, updated_at DESC);",
    )
    .context("migration 016: create fl_trade_offers table")?;

    // Migration 017 — exact points/category competition contract.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS fl_competition_settings (
            league_id   TEXT PRIMARY KEY,
            rules_json  TEXT NOT NULL,
            updated_at  TEXT NOT NULL,
            FOREIGN KEY(league_id) REFERENCES fl_leagues(id) ON DELETE CASCADE
        );",
    )
    .context("migration 017: create fl_competition_settings table")?;

    // Migration 018 — sourced, game-specific goalie starter observations.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS fl_goalie_start_observations (
            id                  TEXT PRIMARY KEY,
            league_id           TEXT NOT NULL,
            player_normalized   TEXT NOT NULL,
            game_date           TEXT NOT NULL,
            state               TEXT NOT NULL,
            source              TEXT NOT NULL,
            source_url          TEXT,
            observed_at         TEXT NOT NULL,
            fetched_at          TEXT NOT NULL,
            detail              TEXT,
            FOREIGN KEY(league_id) REFERENCES fl_leagues(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_fl_goalie_start_league_game_player_observed
            ON fl_goalie_start_observations(
                league_id, game_date, player_normalized, observed_at DESC
            );",
    )
    .context("migration 018: create fl_goalie_start_observations table")?;

    // Enable foreign-key enforcement (off by default in rusqlite).
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .context("enable foreign keys")?;

    Ok(())
}

fn table_has_column(conn: &Connection, table: &str, column: &str) -> anyhow::Result<bool> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .with_context(|| format!("inspect {table} columns"))?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .with_context(|| format!("read {table} columns"))?
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| format!("collect {table} columns"))?;
    Ok(columns.iter().any(|name| name == column))
}

fn validate_trade_offer_status(status: &str) -> anyhow::Result<()> {
    match status {
        "pending" | "accepted" | "rejected" | "cancelled" | "expired" => Ok(()),
        _ => bail!(
            "unknown trade offer status '{status}'; expected pending, accepted, rejected, cancelled, or expired"
        ),
    }
}

pub fn open_existing_sqlite_read_only_path(
    db_path: &std::path::Path,
) -> anyhow::Result<Connection> {
    let uri = sqlite_immutable_read_uri(db_path);
    Connection::open_with_flags(
        &uri,
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_URI
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("open {} read-only", db_path.display()))
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
        conn.execute_batch("PRAGMA wal_autocheckpoint = 1;")
            .context("set WAL autocheckpoint")?;

        run_migrations(&conn)?;
        Ok(Self { conn, db_path })
    }

    /// Open an existing database for read-only surfaces without creating files,
    /// changing journal mode, SQLite sidecars, or running migrations.
    pub fn open_existing_read_only_path(db_path: std::path::PathBuf) -> anyhow::Result<Self> {
        let conn = open_existing_sqlite_read_only_path(&db_path)?;

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
        self.create_league_with_shape(name, scheme, DEFAULT_ROSTER_SHAPE)
    }

    pub fn create_league_with_shape(
        &self,
        name: &str,
        scheme: &str,
        roster_shape: &str,
    ) -> anyhow::Result<String> {
        resolve_roster_shape(roster_shape)?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO fl_leagues (id, name, scheme, roster_shape, is_active, created_at) \
                 VALUES (?1, ?2, ?3, ?4, 0, ?5)",
                rusqlite::params![id, name, scheme, roster_shape, now],
            )
            .with_context(|| format!("create league '{name}'"))?;
        Ok(id)
    }

    /// List all leagues with team counts.
    pub fn list_leagues(&self) -> anyhow::Result<Vec<LeagueRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT l.id, l.name, l.scheme, l.roster_shape, l.is_active,
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
                    roster_shape: row.get(3)?,
                    is_active: row.get::<_, i64>(4)? != 0,
                    team_count: row.get::<_, i64>(5)? as usize,
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

    pub fn set_league_scheme(&self, league_id: &str, scheme: &str) -> anyhow::Result<()> {
        let rows = self
            .conn
            .execute(
                "UPDATE fl_leagues SET scheme = ?1 WHERE id = ?2",
                rusqlite::params![scheme, league_id],
            )
            .with_context(|| format!("set scoring scheme '{scheme}' for league {league_id}"))?;
        if rows == 0 {
            bail!("league id '{league_id}' not found");
        }
        Ok(())
    }

    pub fn set_competition_rules(
        &self,
        league_id: &str,
        rules: &FantasyCompetitionRules,
    ) -> anyhow::Result<()> {
        rules.validate().map_err(anyhow::Error::msg)?;
        let rules_json = serde_json::to_string(rules).context("serialize competition rules")?;
        let now = Utc::now().to_rfc3339();
        let tx = self
            .conn
            .unchecked_transaction()
            .context("begin competition-rules transaction")?;
        let updated = tx.execute(
            "UPDATE fl_leagues SET competition_mode = ?1 WHERE id = ?2",
            rusqlite::params![rules.mode.label(), league_id],
        )?;
        if updated == 0 {
            bail!("league id '{league_id}' not found");
        }
        tx.execute(
            "INSERT INTO fl_competition_settings (league_id, rules_json, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(league_id) DO UPDATE SET
               rules_json = excluded.rules_json,
               updated_at = excluded.updated_at",
            rusqlite::params![league_id, rules_json, now],
        )?;
        tx.commit().context("commit competition rules")?;
        Ok(())
    }

    pub fn get_competition_rules(
        &self,
        league_id: &str,
    ) -> anyhow::Result<FantasyCompetitionRules> {
        let raw = self
            .conn
            .query_row(
                "SELECT competition_mode,
                        (SELECT rules_json FROM fl_competition_settings WHERE league_id = l.id)
                 FROM fl_leagues l WHERE id = ?1",
                rusqlite::params![league_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()
            .context("read competition rules")?
            .ok_or_else(|| anyhow::anyhow!("league id '{league_id}' not found"))?;
        let rules = match raw {
            (mode, None) if mode == FantasyCompetitionMode::Points.label() => {
                FantasyCompetitionRules::points()
            }
            (mode, None) => {
                bail!("league competition mode is '{mode}' but its category rules are missing")
            }
            (_, Some(json)) => serde_json::from_str::<FantasyCompetitionRules>(&json)
                .context("parse persisted competition rules")?,
        };
        rules.validate().map_err(anyhow::Error::msg)?;
        Ok(rules)
    }

    /// Get the currently active league (if any).
    pub fn get_active_league(&self) -> anyhow::Result<Option<LeagueRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT l.id, l.name, l.scheme, l.roster_shape, l.is_active,
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
                    roster_shape: row.get(3)?,
                    is_active: row.get::<_, i64>(4)? != 0,
                    team_count: row.get::<_, i64>(5)? as usize,
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

    pub fn set_league_roster_shape(
        &self,
        league_id: &str,
        roster_shape: &str,
    ) -> anyhow::Result<()> {
        resolve_roster_shape(roster_shape)?;
        let rows = self
            .conn
            .execute(
                "UPDATE fl_leagues SET roster_shape = ?1 WHERE id = ?2",
                rusqlite::params![roster_shape, league_id],
            )
            .with_context(|| format!("set roster shape '{roster_shape}' for league {league_id}"))?;
        if rows == 0 {
            bail!("league '{league_id}' not found");
        }
        Ok(())
    }

    pub fn set_assistant_rules(
        &self,
        league_id: &str,
        rules: &FantasyAssistantRules,
    ) -> anyhow::Result<()> {
        rules.validate().map_err(anyhow::Error::msg)?;
        let exists = self
            .conn
            .query_row(
                "SELECT 1 FROM fl_leagues WHERE id = ?1",
                rusqlite::params![league_id],
                |_| Ok(()),
            )
            .is_ok();
        if !exists {
            bail!("league '{league_id}' not found");
        }
        let rules_json =
            serde_json::to_string(rules).context("serialize fantasy assistant rules")?;
        let now = chrono::Utc::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO fl_assistant_settings (league_id, rules_json, updated_at)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(league_id) DO UPDATE SET
                   rules_json = excluded.rules_json,
                   updated_at = excluded.updated_at",
                rusqlite::params![league_id, rules_json, now],
            )
            .with_context(|| format!("persist assistant rules for league {league_id}"))?;
        Ok(())
    }

    pub fn get_assistant_rules(
        &self,
        league_id: &str,
    ) -> anyhow::Result<Option<FantasyAssistantRules>> {
        let raw = self
            .conn
            .query_row(
                "SELECT rules_json FROM fl_assistant_settings WHERE league_id = ?1",
                rusqlite::params![league_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .context("read fantasy assistant rules")?;
        raw.map(|json| {
            let rules: FantasyAssistantRules =
                serde_json::from_str(&json).context("parse persisted fantasy assistant rules")?;
            rules.validate().map_err(anyhow::Error::msg)?;
            Ok(rules)
        })
        .transpose()
    }

    pub fn upsert_player_eligibility(
        &self,
        league_id: &str,
        player_normalized: &str,
        positions: &[Position],
        source: &str,
    ) -> anyhow::Result<()> {
        if positions.is_empty() {
            bail!("platform eligibility requires at least one position");
        }
        let positions_json =
            serde_json::to_string(positions).context("serialize platform eligibility")?;
        let fetched_at = chrono::Utc::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO fl_player_eligibility
                   (league_id, player_normalized, positions_json, source, fetched_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(league_id, player_normalized) DO UPDATE SET
                   positions_json = excluded.positions_json,
                   source = excluded.source,
                   fetched_at = excluded.fetched_at",
                rusqlite::params![
                    league_id,
                    player_normalized,
                    positions_json,
                    source,
                    fetched_at
                ],
            )
            .with_context(|| format!("persist platform eligibility for {player_normalized}"))?;
        Ok(())
    }

    pub fn list_player_eligibility(
        &self,
        league_id: &str,
    ) -> anyhow::Result<Vec<FantasyPlayerEligibilityRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT player_normalized, positions_json, source, fetched_at
             FROM fl_player_eligibility
             WHERE league_id = ?1
             ORDER BY player_normalized",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![league_id], |row| {
                let positions_json: String = row.get(1)?;
                let positions =
                    serde_json::from_str::<Vec<Position>>(&positions_json).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            positions_json.len(),
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?;
                Ok(FantasyPlayerEligibilityRow {
                    player_normalized: row.get(0)?,
                    positions,
                    source: row.get(2)?,
                    fetched_at: row.get(3)?,
                })
            })
            .context("list platform eligibility query")?;
        rows.collect::<Result<Vec<_>, _>>()
            .context("list platform eligibility collect")
    }

    pub fn record_acquisition(
        &self,
        league_id: &str,
        player_added: &str,
        player_dropped: Option<&str>,
        kind: FantasyAcquisitionKind,
        effective_at: DateTime<Utc>,
        counts_toward_limit: bool,
        waiver_days: u8,
    ) -> anyhow::Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        self.conn
            .execute(
                "INSERT INTO fl_acquisitions
                   (id, league_id, player_added, player_dropped, kind, effective_at, counts_toward_limit)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    id,
                    league_id,
                    player_added,
                    player_dropped,
                    acquisition_kind_label(kind),
                    effective_at.to_rfc3339(),
                    i64::from(counts_toward_limit)
                ],
            )
            .context("record fantasy acquisition")?;
        if let Some(dropped) = player_dropped {
            let waiver = icelines_core::fantasy_waiver_window(dropped, effective_at, waiver_days);
            self.upsert_waiver(league_id, &waiver)?;
        }
        Ok(id)
    }

    pub fn list_acquisitions(
        &self,
        league_id: &str,
        from: DateTime<Utc>,
        through: DateTime<Utc>,
    ) -> anyhow::Result<Vec<FantasyAcquisitionLedgerRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, player_added, player_dropped, kind, effective_at, counts_toward_limit
             FROM fl_acquisitions
             WHERE league_id = ?1 AND effective_at >= ?2 AND effective_at <= ?3
             ORDER BY effective_at, id",
        )?;
        let rows = stmt.query_map(
            rusqlite::params![league_id, from.to_rfc3339(), through.to_rfc3339()],
            |row| {
                let kind: String = row.get(3)?;
                let effective_at: String = row.get(4)?;
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    kind,
                    effective_at,
                    row.get::<_, i64>(5)? != 0,
                ))
            },
        )?;
        rows.map(|row| {
            let (id, player_added, player_dropped, kind, effective_at, counts_toward_limit) = row?;
            Ok(FantasyAcquisitionLedgerRow {
                id,
                player_added,
                player_dropped,
                kind: parse_acquisition_kind(&kind)?,
                effective_at: DateTime::parse_from_rfc3339(&effective_at)
                    .with_context(|| format!("parse acquisition timestamp {effective_at}"))?
                    .with_timezone(&Utc),
                counts_toward_limit,
            })
        })
        .collect()
    }

    pub fn upsert_waiver(
        &self,
        league_id: &str,
        waiver: &FantasyWaiverWindow,
    ) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT INTO fl_waivers (league_id, player_normalized, dropped_at, clears_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(league_id, player_normalized) DO UPDATE SET
               dropped_at = excluded.dropped_at,
               clears_at = excluded.clears_at",
            rusqlite::params![
                league_id,
                waiver.player_key,
                waiver.dropped_at.to_rfc3339(),
                waiver.clears_at.to_rfc3339()
            ],
        )?;
        Ok(())
    }

    pub fn get_waiver(
        &self,
        league_id: &str,
        player_normalized: &str,
    ) -> anyhow::Result<Option<FantasyWaiverWindow>> {
        let raw = self
            .conn
            .query_row(
                "SELECT dropped_at, clears_at FROM fl_waivers
             WHERE league_id = ?1 AND player_normalized = ?2",
                rusqlite::params![league_id, player_normalized],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        raw.map(
            |(dropped_at, clears_at)| -> anyhow::Result<FantasyWaiverWindow> {
                Ok(FantasyWaiverWindow {
                    player_key: player_normalized.to_owned(),
                    dropped_at: DateTime::parse_from_rfc3339(&dropped_at)?.with_timezone(&Utc),
                    clears_at: DateTime::parse_from_rfc3339(&clears_at)?.with_timezone(&Utc),
                })
            },
        )
        .transpose()
    }

    pub fn record_status_observation(
        &self,
        league_id: &str,
        observation: &FantasyStatusObservation,
    ) -> anyhow::Result<String> {
        if observation.source.trim().is_empty() {
            bail!("status observation source is required");
        }
        if observation.player_key.trim().is_empty() {
            bail!("status observation player key is required");
        }
        let id = uuid::Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO fl_status_observations
               (id, league_id, player_normalized, status, source, source_url,
                observed_at, fetched_at, confidence, detail)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                id,
                league_id,
                observation.player_key,
                availability_status_label(observation.status),
                observation.source,
                observation.source_url,
                observation.observed_at.to_rfc3339(),
                observation.fetched_at.to_rfc3339(),
                observation_confidence_label(observation.confidence),
                observation.detail,
            ],
        )?;
        Ok(id)
    }

    pub fn list_latest_status_observations(
        &self,
        league_id: &str,
    ) -> anyhow::Result<Vec<FantasyStatusObservation>> {
        let mut stmt = self.conn.prepare(
            "SELECT player_normalized, status, source, source_url, observed_at,
                    fetched_at, confidence, detail
             FROM fl_status_observations
             WHERE league_id = ?1
             ORDER BY player_normalized, observed_at DESC, fetched_at DESC, id DESC",
        )?;
        let raw = stmt
            .query_map(rusqlite::params![league_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let mut seen = std::collections::BTreeSet::new();
        raw.into_iter()
            .filter(|row| seen.insert(row.0.clone()))
            .map(
                |(
                    player_key,
                    status,
                    source,
                    source_url,
                    observed_at,
                    fetched_at,
                    confidence,
                    detail,
                )| {
                    Ok(FantasyStatusObservation {
                        player_key,
                        status: parse_availability_status(&status)?,
                        source,
                        source_url,
                        observed_at: DateTime::parse_from_rfc3339(&observed_at)?
                            .with_timezone(&Utc),
                        fetched_at: DateTime::parse_from_rfc3339(&fetched_at)?.with_timezone(&Utc),
                        confidence: parse_observation_confidence(&confidence)?,
                        detail,
                    })
                },
            )
            .collect()
    }

    pub fn record_goalie_start_observation(
        &self,
        league_id: &str,
        observation: &FantasyGoalieStartObservation,
    ) -> anyhow::Result<String> {
        if observation.source.trim().is_empty() {
            bail!("goalie start observation source is required");
        }
        if observation.player_key.trim().is_empty() {
            bail!("goalie start observation player key is required");
        }
        let id = uuid::Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO fl_goalie_start_observations
               (id, league_id, player_normalized, game_date, state, source, source_url,
                observed_at, fetched_at, detail)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                id,
                league_id,
                observation.player_key,
                observation.game_date.to_string(),
                goalie_start_state_label(observation.state),
                observation.source,
                observation.source_url,
                observation.observed_at.to_rfc3339(),
                observation.fetched_at.to_rfc3339(),
                observation.detail,
            ],
        )?;
        Ok(id)
    }

    pub fn record_goalie_start_observations(
        &self,
        league_id: &str,
        observations: &[FantasyGoalieStartObservation],
    ) -> anyhow::Result<Vec<String>> {
        for observation in observations {
            if observation.source.trim().is_empty() {
                bail!("goalie start observation source is required");
            }
            if observation.player_key.trim().is_empty() {
                bail!("goalie start observation player key is required");
            }
        }
        let transaction = self.conn.unchecked_transaction()?;
        let mut ids = Vec::with_capacity(observations.len());
        for observation in observations {
            let id = uuid::Uuid::new_v4().to_string();
            transaction.execute(
                "INSERT INTO fl_goalie_start_observations
                   (id, league_id, player_normalized, game_date, state, source, source_url,
                    observed_at, fetched_at, detail)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    id,
                    league_id,
                    observation.player_key,
                    observation.game_date.to_string(),
                    goalie_start_state_label(observation.state),
                    observation.source,
                    observation.source_url,
                    observation.observed_at.to_rfc3339(),
                    observation.fetched_at.to_rfc3339(),
                    observation.detail,
                ],
            )?;
            ids.push(id);
        }
        transaction.commit()?;
        Ok(ids)
    }

    pub fn list_latest_goalie_start_observations(
        &self,
        league_id: &str,
        from: NaiveDate,
        through: NaiveDate,
    ) -> anyhow::Result<Vec<FantasyGoalieStartObservation>> {
        if through < from {
            bail!("goalie observation end date cannot precede start date");
        }
        let mut stmt = self.conn.prepare(
            "SELECT player_normalized, game_date, state, source, source_url, observed_at,
                    fetched_at, detail
             FROM fl_goalie_start_observations
             WHERE league_id = ?1 AND game_date >= ?2 AND game_date <= ?3
             ORDER BY game_date, player_normalized, observed_at DESC, fetched_at DESC, id DESC",
        )?;
        let raw = stmt
            .query_map(
                rusqlite::params![league_id, from.to_string(), through.to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, Option<String>>(7)?,
                    ))
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;
        let mut seen = BTreeSet::new();
        raw.into_iter()
            .filter(|row| seen.insert((row.0.clone(), row.1.clone())))
            .map(
                |(
                    player_key,
                    game_date,
                    state,
                    source,
                    source_url,
                    observed_at,
                    fetched_at,
                    detail,
                )| {
                    Ok(FantasyGoalieStartObservation {
                        player_key,
                        game_date: NaiveDate::parse_from_str(&game_date, "%Y-%m-%d")?,
                        state: parse_goalie_start_state(&state)?,
                        source,
                        source_url,
                        observed_at: DateTime::parse_from_rfc3339(&observed_at)?
                            .with_timezone(&Utc),
                        fetched_at: DateTime::parse_from_rfc3339(&fetched_at)?.with_timezone(&Utc),
                        detail,
                    })
                },
            )
            .collect()
    }

    pub fn get_morning_briefing_fingerprint(
        &self,
        league_id: &str,
        date: NaiveDate,
    ) -> anyhow::Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT fingerprint FROM fl_morning_briefings
                 WHERE league_id = ?1 AND briefing_date = ?2",
                rusqlite::params![league_id, date.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn upsert_morning_briefing_fingerprint(
        &self,
        league_id: &str,
        date: NaiveDate,
        fingerprint: &str,
        generated_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT INTO fl_morning_briefings
               (league_id, briefing_date, fingerprint, generated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(league_id, briefing_date) DO UPDATE SET
               fingerprint = excluded.fingerprint,
               generated_at = excluded.generated_at",
            rusqlite::params![
                league_id,
                date.to_string(),
                fingerprint,
                generated_at.to_rfc3339()
            ],
        )?;
        Ok(())
    }

    pub fn validate_team_roster_shape(
        &self,
        league: &LeagueRow,
        team: &TeamRow,
        player_positions: &BTreeMap<String, Vec<Position>>,
    ) -> anyhow::Result<RosterShapeValidationView> {
        let shape = resolve_roster_shape(&league.roster_shape)?;
        let players = self
            .list_roster(&team.id)?
            .into_iter()
            .map(|player_key| {
                let positions = player_positions
                    .get(&player_key)
                    .cloned()
                    .unwrap_or_default();
                if positions.is_empty() {
                    RosterShapePlayerInput::unknown(player_key.clone(), player_key)
                } else {
                    RosterShapePlayerInput::known(player_key.clone(), player_key, positions)
                }
            })
            .collect();
        Ok(RosterShapeValidationView::validate(
            RosterShapeValidationInput {
                league: league.name.clone(),
                team: team.name.clone(),
                shape,
                players,
            },
        ))
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

    /// Atomically exchange player packages between two fantasy teams.
    ///
    /// Every outgoing player must still belong to the supplied team when the
    /// transaction begins. Any stale membership, duplicate player, or insert
    /// failure rolls back the complete trade.
    pub fn execute_trade(
        &self,
        sending_team_id: &str,
        sends: &[String],
        receiving_team_id: &str,
        receives: &[String],
    ) -> anyhow::Result<()> {
        if sending_team_id == receiving_team_id {
            bail!("trade teams must be different");
        }
        if sends.is_empty() || receives.is_empty() {
            bail!("trade packages must not be empty");
        }

        let sends_set = sends.iter().collect::<BTreeSet<_>>();
        let receives_set = receives.iter().collect::<BTreeSet<_>>();
        if sends_set.len() != sends.len() || receives_set.len() != receives.len() {
            bail!("trade packages must not contain duplicate players");
        }
        if sends_set.iter().any(|player| receives_set.contains(player)) {
            bail!("a player cannot appear on both sides of a trade");
        }

        let tx = self
            .conn
            .unchecked_transaction()
            .context("begin fantasy trade")?;
        let (league_id, sending_team_name, receiving_team_name) = tx
            .query_row(
                "SELECT sender.league_id, sender.name, receiver.name
                 FROM fl_teams sender
                 JOIN fl_teams receiver
                   ON receiver.id = ?2 AND receiver.league_id = sender.league_id
                 WHERE sender.id = ?1",
                rusqlite::params![sending_team_id, receiving_team_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()
            .context("verify fantasy trade teams share a league")?
            .context("trade teams must exist in the same league")?;
        for (team_id, players) in [(sending_team_id, sends), (receiving_team_id, receives)] {
            for player in players {
                let present = tx
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM fl_roster \
                         WHERE team_id = ?1 AND player_normalized = ?2)",
                        rusqlite::params![team_id, player],
                        |row| row.get::<_, bool>(0),
                    )
                    .with_context(|| format!("verify player '{player}' on team {team_id}"))?;
                if !present {
                    bail!("player '{player}' is no longer on team {team_id}");
                }
            }
        }

        for (team_id, players) in [(sending_team_id, sends), (receiving_team_id, receives)] {
            for player in players {
                tx.execute(
                    "DELETE FROM fl_roster WHERE team_id = ?1 AND player_normalized = ?2",
                    rusqlite::params![team_id, player],
                )
                .with_context(|| format!("remove traded player '{player}' from team {team_id}"))?;
            }
        }

        let now = chrono::Utc::now().to_rfc3339();
        for (team_id, players) in [(sending_team_id, receives), (receiving_team_id, sends)] {
            for player in players {
                tx.execute(
                    "INSERT INTO fl_roster (team_id, player_normalized, added_at) \
                     VALUES (?1, ?2, ?3)",
                    rusqlite::params![team_id, player, now],
                )
                .with_context(|| format!("add traded player '{player}' to team {team_id}"))?;
            }
        }
        tx.execute(
            "INSERT INTO fl_trade_history (
                id, league_id, sending_team_id, sending_team_name,
                receiving_team_id, receiving_team_name,
                sends_json, receives_json, executed_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                uuid::Uuid::new_v4().to_string(),
                league_id,
                sending_team_id,
                sending_team_name,
                receiving_team_id,
                receiving_team_name,
                serde_json::to_string(sends).context("serialize sent trade package")?,
                serde_json::to_string(receives).context("serialize received trade package")?,
                now,
            ],
        )
        .context("record fantasy trade history")?;
        tx.commit().context("commit fantasy trade")?;
        Ok(())
    }

    /// List the most recently executed local fantasy trades for a league.
    pub fn list_trade_history(
        &self,
        league_id: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<FantasyTradeHistoryRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT h.id, h.league_id,
                    h.sending_team_id, h.sending_team_name,
                    h.receiving_team_id, h.receiving_team_name,
                    h.sends_json, h.receives_json, h.executed_at
             FROM fl_trade_history h
             WHERE h.league_id = ?1
             ORDER BY h.executed_at DESC, h.id DESC
             LIMIT ?2",
        )?;
        let raw = stmt
            .query_map(rusqlite::params![league_id, limit as i64], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                ))
            })
            .context("list fantasy trade history query")?
            .collect::<Result<Vec<_>, _>>()
            .context("list fantasy trade history rows")?;
        raw.into_iter()
            .map(
                |(
                    id,
                    league_id,
                    sending_team_id,
                    sending_team,
                    receiving_team_id,
                    receiving_team,
                    sends_json,
                    receives_json,
                    executed_at,
                )| {
                    Ok(FantasyTradeHistoryRow {
                        id,
                        league_id,
                        sending_team_id,
                        sending_team,
                        receiving_team_id,
                        receiving_team,
                        sends: serde_json::from_str(&sends_json)
                            .context("parse sent trade package history")?,
                        receives: serde_json::from_str(&receives_json)
                            .context("parse received trade package history")?,
                        executed_at,
                    })
                },
            )
            .collect()
    }

    /// Save a proposed trade without changing either roster.
    pub fn save_trade_offer(
        &self,
        sending_team_id: &str,
        sends: &[String],
        receiving_team_id: &str,
        receives: &[String],
    ) -> anyhow::Result<String> {
        if sending_team_id == receiving_team_id || sends.is_empty() || receives.is_empty() {
            bail!("a trade offer requires two teams and non-empty packages");
        }
        let sends_set = sends.iter().collect::<BTreeSet<_>>();
        let receives_set = receives.iter().collect::<BTreeSet<_>>();
        if sends_set.len() != sends.len()
            || receives_set.len() != receives.len()
            || sends_set.iter().any(|player| receives_set.contains(player))
        {
            bail!("trade offer packages must be unique and disjoint");
        }
        let tx = self
            .conn
            .unchecked_transaction()
            .context("begin fantasy trade offer")?;
        let (league_id, sending_team, receiving_team) = tx
            .query_row(
                "SELECT sender.league_id, sender.name, receiver.name
                 FROM fl_teams sender
                 JOIN fl_teams receiver
                   ON receiver.id = ?2 AND receiver.league_id = sender.league_id
                 WHERE sender.id = ?1",
                rusqlite::params![sending_team_id, receiving_team_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()
            .context("verify fantasy offer teams")?
            .context("trade offer teams must exist in the same league")?;
        for (team_id, players) in [(sending_team_id, sends), (receiving_team_id, receives)] {
            for player in players {
                let present = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM fl_roster
                     WHERE team_id = ?1 AND player_normalized = ?2)",
                    rusqlite::params![team_id, player],
                    |row| row.get::<_, bool>(0),
                )?;
                if !present {
                    bail!("player '{player}' is no longer on team {team_id}");
                }
            }
        }
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        tx.execute(
            "INSERT INTO fl_trade_offers (
                id, league_id, sending_team_id, sending_team_name,
                receiving_team_id, receiving_team_name, sends_json,
                receives_json, status, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending', ?9, ?9)",
            rusqlite::params![
                id,
                league_id,
                sending_team_id,
                sending_team,
                receiving_team_id,
                receiving_team,
                serde_json::to_string(sends).context("serialize offered players")?,
                serde_json::to_string(receives).context("serialize requested players")?,
                now,
            ],
        )
        .context("save fantasy trade offer")?;
        tx.commit().context("commit fantasy trade offer")?;
        Ok(id)
    }

    pub fn list_trade_offers(
        &self,
        league_id: &str,
        status: Option<&str>,
        limit: usize,
    ) -> anyhow::Result<Vec<FantasyTradeOfferRow>> {
        if let Some(status) = status {
            validate_trade_offer_status(status)?;
        }
        let mut stmt = self.conn.prepare(
            "SELECT id, league_id, sending_team_id, sending_team_name,
                    receiving_team_id, receiving_team_name, sends_json,
                    receives_json, status, created_at, updated_at
             FROM fl_trade_offers
             WHERE league_id = ?1 AND (?2 IS NULL OR status = ?2)
             ORDER BY updated_at DESC, id DESC LIMIT ?3",
        )?;
        let raw = stmt
            .query_map(rusqlite::params![league_id, status, limit as i64], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                ))
            })
            .context("list fantasy trade offers query")?
            .collect::<Result<Vec<_>, _>>()
            .context("list fantasy trade offer rows")?;
        raw.into_iter()
            .map(|row| {
                let sends: Vec<String> =
                    serde_json::from_str(&row.6).context("parse offered players")?;
                let receives: Vec<String> =
                    serde_json::from_str(&row.7).context("parse requested players")?;
                let mut roster_issues = Vec::new();
                for (team_id, team_name, players) in [
                    (row.2.as_str(), row.3.as_str(), sends.as_slice()),
                    (row.4.as_str(), row.5.as_str(), receives.as_slice()),
                ] {
                    for player in players {
                        let present = self.conn.query_row(
                            "SELECT EXISTS(SELECT 1 FROM fl_roster
                             WHERE team_id = ?1 AND player_normalized = ?2)",
                            rusqlite::params![team_id, player],
                            |db_row| db_row.get::<_, bool>(0),
                        )?;
                        if !present {
                            roster_issues
                                .push(format!("'{player}' is no longer rostered by '{team_name}'"));
                        }
                    }
                }
                Ok(FantasyTradeOfferRow {
                    id: row.0,
                    league_id: row.1,
                    sending_team_id: row.2,
                    sending_team: row.3,
                    receiving_team_id: row.4,
                    receiving_team: row.5,
                    sends,
                    receives,
                    status: row.8,
                    created_at: row.9,
                    updated_at: row.10,
                    roster_current: roster_issues.is_empty(),
                    roster_issues,
                })
            })
            .collect()
    }

    /// Close a pending offer. Closed offers are immutable.
    pub fn close_trade_offer(
        &self,
        league_id: &str,
        id: &str,
        status: &str,
    ) -> anyhow::Result<bool> {
        validate_trade_offer_status(status)?;
        if status == "pending" {
            bail!("closing an offer requires accepted, rejected, cancelled, or expired status");
        }
        let rows = self.conn.execute(
            "UPDATE fl_trade_offers SET status = ?3, updated_at = ?4
             WHERE league_id = ?1 AND id = ?2 AND status = 'pending'",
            rusqlite::params![league_id, id, status, Utc::now().to_rfc3339()],
        )?;
        Ok(rows == 1)
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

    /// Atomically replace the complete membership of one or more fantasy rosters.
    /// Team ids must already exist. Duplicate player keys within a supplied roster
    /// are collapsed before insertion.
    pub fn replace_rosters(&self, rosters: &[(String, Vec<String>)]) -> anyhow::Result<()> {
        let tx = self
            .conn
            .unchecked_transaction()
            .context("begin fantasy roster replacement")?;
        let now = chrono::Utc::now().to_rfc3339();
        for (team_id, players) in rosters {
            tx.execute(
                "DELETE FROM fl_roster WHERE team_id = ?1",
                rusqlite::params![team_id],
            )
            .with_context(|| format!("clear roster for team {team_id}"))?;
            for player in players.iter().collect::<BTreeSet<_>>() {
                tx.execute(
                    "INSERT INTO fl_roster (team_id, player_normalized, added_at) VALUES (?1, ?2, ?3)",
                    rusqlite::params![team_id, player, now],
                )
                .with_context(|| format!("replace player '{player}' on team {team_id}"))?;
            }
        }
        tx.commit().context("commit fantasy roster replacement")?;
        Ok(())
    }

    pub fn schedule_matchup(
        &self,
        league_id: &str,
        week_start: NaiveDate,
        home_team_name: &str,
        away_team_name: Option<&str>,
    ) -> anyhow::Result<String> {
        let home = self
            .get_team_by_name(league_id, home_team_name)?
            .ok_or_else(|| anyhow::anyhow!("home team '{home_team_name}' not found"))?;
        let away = away_team_name
            .map(|name| {
                self.get_team_by_name(league_id, name)?
                    .ok_or_else(|| anyhow::anyhow!("away team '{name}' not found"))
            })
            .transpose()?;
        if away.as_ref().is_some_and(|team| team.id == home.id) {
            bail!("a fantasy matchup cannot schedule a team against itself");
        }

        let week = week_start.format("%Y-%m-%d").to_string();
        ensure_team_unscheduled(&self.conn, league_id, &week, &home.id, &home.name)?;
        if let Some(away) = &away {
            ensure_team_unscheduled(&self.conn, league_id, &week, &away.id, &away.name)?;
        }

        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO fl_matchups
                 (id, league_id, week_start, home_team_id, away_team_id, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    id,
                    league_id,
                    week,
                    home.id,
                    away.as_ref().map(|team| &team.id),
                    now
                ],
            )
            .with_context(|| format!("schedule fantasy matchup for week {week}"))?;
        Ok(id)
    }

    pub fn list_matchups(
        &self,
        league_id: &str,
        week_start: NaiveDate,
    ) -> anyhow::Result<Vec<FantasyMatchupScheduleRow>> {
        let week = week_start.format("%Y-%m-%d").to_string();
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.week_start, home.name, away.name
             FROM fl_matchups m
             JOIN fl_teams home ON home.id = m.home_team_id
             LEFT JOIN fl_teams away ON away.id = m.away_team_id
             WHERE m.league_id = ?1 AND m.week_start = ?2
             ORDER BY home.name, away.name",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![league_id, week], |row| {
                let week_start: String = row.get(1)?;
                let week_start =
                    NaiveDate::parse_from_str(&week_start, "%Y-%m-%d").map_err(|err| {
                        rusqlite::Error::FromSqlConversionFailure(
                            1,
                            rusqlite::types::Type::Text,
                            Box::new(err),
                        )
                    })?;
                Ok(FantasyMatchupScheduleRow {
                    id: row.get(0)?,
                    week_start,
                    home_team: row.get(2)?,
                    away_team: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()
            .context("list fantasy matchups")?;
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
            roster_shape: league.roster_shape,
            teams,
        })
    }
}

fn sqlite_immutable_read_uri(db_path: &std::path::Path) -> String {
    let mut path = db_path.to_string_lossy().replace('\\', "/");
    if path.as_bytes().get(1) == Some(&b':') {
        path.insert(0, '/');
    }
    let encoded = path
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' | b':' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect::<String>();
    format!("file://{encoded}?mode=ro&immutable=1")
}

pub fn resolve_roster_shape(name: &str) -> anyhow::Result<RosterShape> {
    RosterShape::builtin_named(name).with_context(|| {
        let names = RosterShape::all_builtins()
            .into_iter()
            .map(|shape| shape.name)
            .collect::<Vec<_>>()
            .join(", ");
        format!("unknown roster shape '{name}'. Try: {names}")
    })
}

fn acquisition_kind_label(kind: FantasyAcquisitionKind) -> &'static str {
    match kind {
        FantasyAcquisitionKind::FreeAgentAdd => "free_agent_add",
        FantasyAcquisitionKind::WaiverClaim => "waiver_claim",
    }
}

fn parse_acquisition_kind(value: &str) -> anyhow::Result<FantasyAcquisitionKind> {
    match value {
        "free_agent_add" => Ok(FantasyAcquisitionKind::FreeAgentAdd),
        "waiver_claim" => Ok(FantasyAcquisitionKind::WaiverClaim),
        other => bail!("unknown fantasy acquisition kind '{other}'"),
    }
}

fn availability_status_label(status: FantasyPlayerAvailabilityStatus) -> &'static str {
    match status {
        FantasyPlayerAvailabilityStatus::Healthy => "healthy",
        FantasyPlayerAvailabilityStatus::DayToDay => "day_to_day",
        FantasyPlayerAvailabilityStatus::GameTimeDecision => "game_time_decision",
        FantasyPlayerAvailabilityStatus::Out => "out",
        FantasyPlayerAvailabilityStatus::InjuredReserve => "injured_reserve",
        FantasyPlayerAvailabilityStatus::LongTermInjuredReserve => "long_term_injured_reserve",
        FantasyPlayerAvailabilityStatus::Suspended => "suspended",
        FantasyPlayerAvailabilityStatus::Personal => "personal",
        FantasyPlayerAvailabilityStatus::Unknown => "unknown",
    }
}

fn parse_availability_status(value: &str) -> anyhow::Result<FantasyPlayerAvailabilityStatus> {
    match value {
        "healthy" => Ok(FantasyPlayerAvailabilityStatus::Healthy),
        "day_to_day" => Ok(FantasyPlayerAvailabilityStatus::DayToDay),
        "game_time_decision" => Ok(FantasyPlayerAvailabilityStatus::GameTimeDecision),
        "out" => Ok(FantasyPlayerAvailabilityStatus::Out),
        "injured_reserve" => Ok(FantasyPlayerAvailabilityStatus::InjuredReserve),
        "long_term_injured_reserve" => Ok(FantasyPlayerAvailabilityStatus::LongTermInjuredReserve),
        "suspended" => Ok(FantasyPlayerAvailabilityStatus::Suspended),
        "personal" => Ok(FantasyPlayerAvailabilityStatus::Personal),
        "unknown" => Ok(FantasyPlayerAvailabilityStatus::Unknown),
        other => bail!("unknown fantasy availability status '{other}'"),
    }
}

fn observation_confidence_label(confidence: FantasyObservationConfidence) -> &'static str {
    match confidence {
        FantasyObservationConfidence::Confirmed => "confirmed",
        FantasyObservationConfidence::Reported => "reported",
        FantasyObservationConfidence::Estimated => "estimated",
        FantasyObservationConfidence::Unknown => "unknown",
    }
}

fn parse_observation_confidence(value: &str) -> anyhow::Result<FantasyObservationConfidence> {
    match value {
        "confirmed" => Ok(FantasyObservationConfidence::Confirmed),
        "reported" => Ok(FantasyObservationConfidence::Reported),
        "estimated" => Ok(FantasyObservationConfidence::Estimated),
        "unknown" => Ok(FantasyObservationConfidence::Unknown),
        other => bail!("unknown fantasy observation confidence '{other}'"),
    }
}

fn goalie_start_state_label(state: FantasyGoalieStartState) -> &'static str {
    match state {
        FantasyGoalieStartState::ConfirmedStarting => "confirmed_starting",
        FantasyGoalieStartState::ReportedStarting => "reported_starting",
        FantasyGoalieStartState::EstimatedStarting => "estimated_starting",
        FantasyGoalieStartState::ConfirmedBackup => "confirmed_backup",
        FantasyGoalieStartState::ReportedBackup => "reported_backup",
        FantasyGoalieStartState::Unknown => "unknown",
    }
}

fn parse_goalie_start_state(value: &str) -> anyhow::Result<FantasyGoalieStartState> {
    match value {
        "confirmed_starting" => Ok(FantasyGoalieStartState::ConfirmedStarting),
        "reported_starting" => Ok(FantasyGoalieStartState::ReportedStarting),
        "estimated_starting" => Ok(FantasyGoalieStartState::EstimatedStarting),
        "confirmed_backup" => Ok(FantasyGoalieStartState::ConfirmedBackup),
        "reported_backup" => Ok(FantasyGoalieStartState::ReportedBackup),
        "unknown" => Ok(FantasyGoalieStartState::Unknown),
        other => bail!("unknown fantasy goalie start state '{other}'"),
    }
}

fn ensure_team_unscheduled(
    conn: &Connection,
    league_id: &str,
    week_start: &str,
    team_id: &str,
    team_name: &str,
) -> anyhow::Result<()> {
    let already_scheduled: bool = conn
        .query_row(
            "SELECT 1
             FROM fl_matchups
             WHERE league_id = ?1
               AND week_start = ?2
               AND (home_team_id = ?3 OR away_team_id = ?3)
             LIMIT 1",
            rusqlite::params![league_id, week_start, team_id],
            |_| Ok(true),
        )
        .unwrap_or(false);
    if already_scheduled {
        bail!("team '{team_name}' already has a matchup for week {week_start}");
    }
    Ok(())
}

// ── Unit tests (L1) ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

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
    fn l1_fantasy_replace_rosters_rolls_back_every_team_on_failure() {
        let db = FantasyDb::open_in_memory().expect("open in-memory db");
        let league_id = db
            .create_league("Office League", "yahoo-standard")
            .expect("create league");
        let team_id = db
            .create_team(&league_id, "Alpha", "Alice")
            .expect("create team");
        db.add_player(&team_id, "original_player")
            .expect("seed roster");

        let result = db.replace_rosters(&[
            (team_id.clone(), vec!["replacement_player".to_owned()]),
            (
                "missing-team-id".to_owned(),
                vec!["invalid_player".to_owned()],
            ),
        ]);

        assert!(result.is_err(), "invalid team must fail replacement");
        assert_eq!(
            db.list_roster(&team_id).expect("roster after rollback"),
            vec!["original_player".to_owned()],
            "the first team deletion/insertion must roll back with the failed transaction"
        );
    }

    #[test]
    fn l1_fantasy_execute_trade_swaps_complete_packages_atomically() {
        let db = FantasyDb::open_in_memory().expect("open in-memory db");
        let league_id = db
            .create_league("Trade League", "yahoo-standard")
            .expect("create league");
        let alpha = db
            .create_team(&league_id, "Alpha", "Alice")
            .expect("create alpha");
        let beta = db
            .create_team(&league_id, "Beta", "Bob")
            .expect("create beta");
        for player in ["alpha_one", "alpha_two", "alpha_keep"] {
            db.add_player(&alpha, player).expect("seed alpha");
        }
        for player in ["beta_one", "beta_keep"] {
            db.add_player(&beta, player).expect("seed beta");
        }

        db.execute_trade(
            &alpha,
            &["alpha_one".to_owned(), "alpha_two".to_owned()],
            &beta,
            &["beta_one".to_owned()],
        )
        .expect("execute package trade");

        let alpha_roster = db.list_roster(&alpha).expect("list alpha");
        let beta_roster = db.list_roster(&beta).expect("list beta");
        assert_eq!(alpha_roster.len(), 2);
        assert!(alpha_roster.contains(&"alpha_keep".to_owned()));
        assert!(alpha_roster.contains(&"beta_one".to_owned()));
        assert_eq!(beta_roster.len(), 3);
        assert!(beta_roster.contains(&"beta_keep".to_owned()));
        assert!(beta_roster.contains(&"alpha_one".to_owned()));
        assert!(beta_roster.contains(&"alpha_two".to_owned()));
        let history = db
            .list_trade_history(&league_id, 10)
            .expect("list trade history");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].sending_team, "Alpha");
        assert_eq!(history[0].receiving_team, "Beta");
        assert_eq!(history[0].sends, vec!["alpha_one", "alpha_two"]);
        assert_eq!(history[0].receives, vec!["beta_one"]);
        assert!(db
            .delete_team(&league_id, "Beta")
            .expect("delete traded team"));
        let retained = db
            .list_trade_history(&league_id, 10)
            .expect("history survives team deletion");
        assert_eq!(retained.len(), 1);
        assert_eq!(retained[0].receiving_team, "Beta");
    }

    #[test]
    fn l1_fantasy_execute_trade_rejects_stale_package_without_mutation() {
        let db = FantasyDb::open_in_memory().expect("open in-memory db");
        let league_id = db
            .create_league("Stale Trade League", "yahoo-standard")
            .expect("create league");
        let alpha = db
            .create_team(&league_id, "Alpha", "Alice")
            .expect("create alpha");
        let beta = db
            .create_team(&league_id, "Beta", "Bob")
            .expect("create beta");
        db.add_player(&alpha, "alpha_one").expect("seed alpha");
        db.add_player(&beta, "beta_one").expect("seed beta");

        let result = db.execute_trade(
            &alpha,
            &["alpha_one".to_owned(), "already_moved".to_owned()],
            &beta,
            &["beta_one".to_owned()],
        );

        assert!(result.is_err(), "stale package must fail");
        assert_eq!(
            db.list_roster(&alpha).expect("alpha after rollback"),
            vec!["alpha_one".to_owned()]
        );
        assert_eq!(
            db.list_roster(&beta).expect("beta after rollback"),
            vec!["beta_one".to_owned()]
        );
        assert!(db
            .list_trade_history(&league_id, 10)
            .expect("history after stale trade")
            .is_empty());
    }

    #[test]
    fn l1_fantasy_execute_trade_rolls_back_after_insert_failure() {
        let db = FantasyDb::open_in_memory().expect("open in-memory db");
        let league_id = db
            .create_league("Conflicting Trade League", "yahoo-standard")
            .expect("create league");
        let alpha = db
            .create_team(&league_id, "Alpha", "Alice")
            .expect("create alpha");
        let beta = db
            .create_team(&league_id, "Beta", "Bob")
            .expect("create beta");
        db.add_player(&alpha, "alpha_one").expect("seed alpha");
        db.add_player(&beta, "beta_one").expect("seed beta");
        db.add_player(&beta, "alpha_one")
            .expect("seed conflicting duplicate ownership");
        let alpha_before = db.list_roster(&alpha).expect("alpha before trade");
        let beta_before = db.list_roster(&beta).expect("beta before trade");

        let result = db.execute_trade(
            &alpha,
            &["alpha_one".to_owned()],
            &beta,
            &["beta_one".to_owned()],
        );

        assert!(result.is_err(), "conflicting insert must fail");
        assert_eq!(
            db.list_roster(&alpha).expect("alpha after rollback"),
            alpha_before
        );
        assert_eq!(
            db.list_roster(&beta).expect("beta after rollback"),
            beta_before
        );
        assert!(db
            .list_trade_history(&league_id, 10)
            .expect("history after rollback")
            .is_empty());
    }

    #[test]
    fn l1_fantasy_trade_offer_lifecycle_does_not_mutate_rosters() {
        let db = FantasyDb::open_in_memory().expect("open in-memory db");
        let league_id = db
            .create_league("Offer League", "yahoo-standard")
            .expect("create league");
        let alpha = db.create_team(&league_id, "Alpha", "Alice").unwrap();
        let beta = db.create_team(&league_id, "Beta", "Bob").unwrap();
        db.add_player(&alpha, "alpha_one").unwrap();
        db.add_player(&beta, "beta_one").unwrap();

        let id = db
            .save_trade_offer(
                &alpha,
                &["alpha_one".to_owned()],
                &beta,
                &["beta_one".to_owned()],
            )
            .expect("save offer");
        let pending = db
            .list_trade_offers(&league_id, Some("pending"), 10)
            .expect("list pending offers");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, id);
        assert_eq!(pending[0].sends, vec!["alpha_one"]);
        assert_eq!(pending[0].receives, vec!["beta_one"]);
        assert!(pending[0].roster_current);
        assert!(pending[0].roster_issues.is_empty());
        assert!(db
            .close_trade_offer(&league_id, &id, "accepted")
            .expect("accept offer"));
        assert!(!db
            .close_trade_offer(&league_id, &id, "rejected")
            .expect("closed offer remains immutable"));
        assert!(db
            .list_trade_offers(&league_id, Some("pending"), 10)
            .unwrap()
            .is_empty());
        assert_eq!(
            db.list_trade_offers(&league_id, Some("accepted"), 10)
                .unwrap()[0]
                .status,
            "accepted"
        );
        assert_eq!(db.list_roster(&alpha).unwrap(), vec!["alpha_one"]);
        assert_eq!(db.list_roster(&beta).unwrap(), vec!["beta_one"]);
    }

    #[test]
    fn l1_fantasy_trade_offer_detects_stale_roster_membership() {
        let db = FantasyDb::open_in_memory().expect("open in-memory db");
        let league_id = db
            .create_league("Stale Offer League", "yahoo-standard")
            .unwrap();
        let alpha = db.create_team(&league_id, "Alpha", "Alice").unwrap();
        let beta = db.create_team(&league_id, "Beta", "Bob").unwrap();
        db.add_player(&alpha, "alpha_one").unwrap();
        db.add_player(&beta, "beta_one").unwrap();
        db.save_trade_offer(
            &alpha,
            &["alpha_one".to_owned()],
            &beta,
            &["beta_one".to_owned()],
        )
        .unwrap();
        db.drop_player(&beta, "beta_one").unwrap();

        let offers = db
            .list_trade_offers(&league_id, Some("pending"), 10)
            .unwrap();
        assert_eq!(offers.len(), 1);
        assert!(!offers[0].roster_current);
        assert_eq!(offers[0].roster_issues.len(), 1);
        assert!(offers[0].roster_issues[0].contains("beta_one"));
        assert!(offers[0].roster_issues[0].contains("Beta"));
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
        assert_eq!(snapshot.roster_shape, DEFAULT_ROSTER_SHAPE);
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

    #[test]
    fn l1_fantasy_roster_shape_defaults_and_rejects_unknown_presets() {
        let db = FantasyDb::open_in_memory().expect("open in-memory db");
        let league_id = db
            .create_league("Shape Defaults", "yahoo-standard")
            .expect("create league");

        let league = db
            .list_leagues()
            .expect("list leagues")
            .into_iter()
            .find(|league| league.id == league_id)
            .expect("created league is listed");

        assert_eq!(league.roster_shape, DEFAULT_ROSTER_SHAPE);
        assert!(
            db.set_league_roster_shape(&league_id, "not-a-shape")
                .is_err(),
            "unknown roster shape names must be rejected before persistence"
        );
    }

    #[test]
    fn l1_fantasy_roster_shape_validation_uses_persisted_shape() {
        let db = FantasyDb::open_in_memory().expect("open in-memory db");
        let league_id = db
            .create_league("Shape League", "yahoo-standard")
            .expect("create league");
        let team_id = db
            .create_team(&league_id, "Short Bench", "Alice")
            .expect("create team");
        db.add_player(&team_id, "connor_mcdavid")
            .expect("add player");
        let league = db.get_active_league().expect("active").unwrap_or_else(|| {
            db.list_leagues()
                .expect("list leagues")
                .into_iter()
                .find(|league| league.id == league_id)
                .expect("league")
        });
        let team = db
            .get_team_by_name(&league_id, "Short Bench")
            .expect("team query")
            .expect("team");
        let positions = BTreeMap::from([("connor_mcdavid".to_string(), vec![Position::Center])]);

        let view = db
            .validate_team_roster_shape(&league, &team, &positions)
            .expect("validate shape");

        assert_eq!(view.shape_name, DEFAULT_ROSTER_SHAPE);
        assert_eq!(view.status, icelines_core::RosterShapeStatus::Invalid);
        assert!(view.summary.missing_slots > 0);
    }

    #[test]
    fn l1_fantasy_assistant_rules_round_trip_per_league() {
        let db = FantasyDb::open_in_memory().unwrap();
        let league_id = db
            .create_league("Assistant League", "yahoo-standard")
            .unwrap();
        assert!(db.get_assistant_rules(&league_id).unwrap().is_none());

        let mut rules = FantasyAssistantRules::configured_2026();
        rules.playoff_start = Some(chrono::NaiveDate::from_ymd_opt(2027, 3, 15).unwrap());
        rules.playoff_rounds = 3;
        db.set_assistant_rules(&league_id, &rules).unwrap();
        let loaded = db.get_assistant_rules(&league_id).unwrap().unwrap();
        assert_eq!(loaded, rules);
        assert_eq!(loaded.standard_roster_capacity(), 16);
        assert_eq!(loaded.total_capacity_with_reserve(), 20);
        assert_eq!(loaded.playoff_start, rules.playoff_start);
        assert_eq!(loaded.playoff_rounds, 3);
    }

    #[test]
    fn l1_platform_eligibility_preserves_multi_position_source() {
        let db = FantasyDb::open_in_memory().unwrap();
        let league_id = db
            .create_league("Eligibility League", "yahoo-standard")
            .unwrap();
        db.upsert_player_eligibility(
            &league_id,
            "flex_forward",
            &[Position::Center, Position::LeftWing],
            "yahoo-player-pool-csv",
        )
        .unwrap();

        let rows = db.list_player_eligibility(&league_id).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].player_normalized, "flex_forward");
        assert_eq!(
            rows[0].positions,
            vec![Position::Center, Position::LeftWing]
        );
        assert_eq!(rows[0].source, "yahoo-player-pool-csv");
    }

    #[test]
    fn l1_acquisition_ledger_creates_two_day_drop_waiver() {
        let db = FantasyDb::open_in_memory().unwrap();
        let league_id = db.create_league("Weekly League", "yahoo-standard").unwrap();
        let effective_at = Utc.with_ymd_and_hms(2026, 10, 5, 16, 0, 0).unwrap();

        db.record_acquisition(
            &league_id,
            "new_player",
            Some("dropped_player"),
            FantasyAcquisitionKind::FreeAgentAdd,
            effective_at,
            true,
            2,
        )
        .unwrap();

        let rows = db
            .list_acquisitions(
                &league_id,
                effective_at - chrono::Duration::hours(1),
                effective_at + chrono::Duration::hours(1),
            )
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].player_added, "new_player");
        assert_eq!(rows[0].player_dropped.as_deref(), Some("dropped_player"));
        assert!(rows[0].counts_toward_limit);

        let waiver = db
            .get_waiver(&league_id, "dropped_player")
            .unwrap()
            .unwrap();
        assert_eq!(waiver.dropped_at, effective_at);
        assert_eq!(waiver.clears_at, effective_at + chrono::Duration::days(2));
    }

    #[test]
    fn l1_latest_status_observation_preserves_source_and_confidence() {
        let db = FantasyDb::open_in_memory().unwrap();
        let league_id = db.create_league("Status League", "yahoo-standard").unwrap();
        let first = Utc.with_ymd_and_hms(2026, 10, 5, 14, 0, 0).unwrap();
        let latest = FantasyStatusObservation {
            player_key: "injured_player".to_owned(),
            status: FantasyPlayerAvailabilityStatus::Out,
            source: "league-export".to_owned(),
            source_url: Some("https://example.test/player".to_owned()),
            observed_at: first + chrono::Duration::minutes(30),
            fetched_at: first + chrono::Duration::minutes(31),
            confidence: FantasyObservationConfidence::Confirmed,
            detail: Some("ruled out".to_owned()),
        };
        let mut older = latest.clone();
        older.status = FantasyPlayerAvailabilityStatus::DayToDay;
        older.observed_at = first;
        older.fetched_at = first;
        db.record_status_observation(&league_id, &older).unwrap();
        db.record_status_observation(&league_id, &latest).unwrap();

        let rows = db.list_latest_status_observations(&league_id).unwrap();
        assert_eq!(rows, vec![latest]);
    }

    #[test]
    fn l1_latest_goalie_start_observation_is_game_specific_and_sourced() {
        let db = FantasyDb::open_in_memory().unwrap();
        let league_id = db.create_league("Goalie League", "yahoo-standard").unwrap();
        let date = NaiveDate::from_ymd_opt(2026, 11, 9).unwrap();
        let first = Utc.with_ymd_and_hms(2026, 11, 9, 15, 0, 0).unwrap();
        let latest = FantasyGoalieStartObservation {
            player_key: "igor_shesterkin".to_owned(),
            game_date: date,
            state: FantasyGoalieStartState::ConfirmedStarting,
            source: "team-reporter".to_owned(),
            source_url: Some("https://example.test/goalie".to_owned()),
            observed_at: first + chrono::Duration::minutes(30),
            fetched_at: first + chrono::Duration::minutes(31),
            detail: Some("led morning skate".to_owned()),
        };
        let mut older = latest.clone();
        older.state = FantasyGoalieStartState::EstimatedStarting;
        older.observed_at = first;
        older.fetched_at = first;
        db.record_goalie_start_observation(&league_id, &older)
            .unwrap();
        db.record_goalie_start_observation(&league_id, &latest)
            .unwrap();

        let rows = db
            .list_latest_goalie_start_observations(&league_id, date, date)
            .unwrap();
        assert_eq!(rows, vec![latest]);
    }

    #[test]
    fn l1_goalie_start_batch_validates_before_atomic_insert() {
        let db = FantasyDb::open_in_memory().unwrap();
        let league_id = db.create_league("Goalie Batch", "yahoo-standard").unwrap();
        let date = NaiveDate::from_ymd_opt(2026, 11, 9).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 11, 9, 17, 0, 0).unwrap();
        let valid = FantasyGoalieStartObservation {
            player_key: "goalie_one".to_owned(),
            game_date: date,
            state: FantasyGoalieStartState::ConfirmedStarting,
            source: "reporter".to_owned(),
            source_url: None,
            observed_at: now,
            fetched_at: now,
            detail: None,
        };
        let mut invalid = valid.clone();
        invalid.player_key = "goalie_two".to_owned();
        invalid.source.clear();
        assert!(db
            .record_goalie_start_observations(&league_id, &[valid.clone(), invalid])
            .is_err());
        assert!(db
            .list_latest_goalie_start_observations(&league_id, date, date)
            .unwrap()
            .is_empty());

        let mut second = valid.clone();
        second.player_key = "goalie_two".to_owned();
        let ids = db
            .record_goalie_start_observations(&league_id, &[valid, second])
            .unwrap();
        assert_eq!(ids.len(), 2);
        assert_eq!(
            db.list_latest_goalie_start_observations(&league_id, date, date)
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn l1_morning_briefing_fingerprint_round_trips_and_updates() {
        let db = FantasyDb::open_in_memory().unwrap();
        let league_id = db
            .create_league("Morning League", "yahoo-standard")
            .unwrap();
        let date = NaiveDate::from_ymd_opt(2026, 10, 8).unwrap();
        let generated_at = Utc.with_ymd_and_hms(2026, 10, 8, 14, 0, 0).unwrap();

        assert_eq!(
            db.get_morning_briefing_fingerprint(&league_id, date)
                .unwrap(),
            None
        );
        db.upsert_morning_briefing_fingerprint(&league_id, date, "first", generated_at)
            .unwrap();
        assert_eq!(
            db.get_morning_briefing_fingerprint(&league_id, date)
                .unwrap()
                .as_deref(),
            Some("first")
        );
        db.upsert_morning_briefing_fingerprint(
            &league_id,
            date,
            "second",
            generated_at + chrono::Duration::minutes(5),
        )
        .unwrap();
        assert_eq!(
            db.get_morning_briefing_fingerprint(&league_id, date)
                .unwrap()
                .as_deref(),
            Some("second")
        );
    }

    #[test]
    fn l1_fantasy_matchup_schedule_persists_byes_and_rejects_duplicates() {
        let db = FantasyDb::open_in_memory().expect("open in-memory db");
        let league_id = db
            .create_league("Matchups", "yahoo-standard")
            .expect("create league");
        db.create_team(&league_id, "Alpha", "Alice")
            .expect("create alpha");
        db.create_team(&league_id, "Bravo", "Bob")
            .expect("create bravo");
        db.create_team(&league_id, "Charlie", "Cam")
            .expect("create charlie");
        let week = NaiveDate::from_ymd_opt(2026, 1, 12).expect("valid date");

        db.schedule_matchup(&league_id, week, "Alpha", Some("Bravo"))
            .expect("schedule matchup");
        db.schedule_matchup(&league_id, week, "Charlie", None)
            .expect("schedule bye");
        let rows = db.list_matchups(&league_id, week).expect("list matchups");

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].home_team, "Alpha");
        assert_eq!(rows[0].away_team.as_deref(), Some("Bravo"));
        assert_eq!(rows[1].home_team, "Charlie");
        assert_eq!(rows[1].away_team, None);
        assert!(
            db.schedule_matchup(&league_id, week, "Bravo", Some("Charlie"))
                .is_err(),
            "a team can only occupy one matchup slot per week"
        );
        assert!(
            db.schedule_matchup(&league_id, week, "Alpha", Some("Alpha"))
                .is_err(),
            "self-matchups are invalid"
        );
    }

    #[test]
    fn l1_competition_rules_default_to_points_and_round_trip_categories() {
        use icelines_core::{
            FantasyCategoryAggregation, FantasyCategoryDirection, FantasyCategoryRule,
            FantasyMatchupTiePolicy, FANTASY_COMPETITION_RULES_SCHEMA,
        };

        let db = FantasyDb::open_in_memory().expect("open in-memory db");
        let league_id = db
            .create_league("Category League", "yahoo-standard")
            .expect("create league");
        assert_eq!(
            db.get_competition_rules(&league_id).unwrap(),
            FantasyCompetitionRules::points()
        );

        let rules = FantasyCompetitionRules {
            schema: FANTASY_COMPETITION_RULES_SCHEMA.to_owned(),
            mode: FantasyCompetitionMode::Categories,
            categories: vec![
                FantasyCategoryRule {
                    key: "goals".to_owned(),
                    label: "G".to_owned(),
                    direction: FantasyCategoryDirection::HigherWins,
                    aggregation: FantasyCategoryAggregation::Sum,
                    tie_epsilon: 0.0,
                },
                FantasyCategoryRule {
                    key: "save_percentage".to_owned(),
                    label: "SV%".to_owned(),
                    direction: FantasyCategoryDirection::HigherWins,
                    aggregation: FantasyCategoryAggregation::Ratio,
                    tie_epsilon: 0.0001,
                },
            ],
            minimum_goalie_appearances: 3,
            matchup_tie_policy: FantasyMatchupTiePolicy::Tie,
        };
        db.set_competition_rules(&league_id, &rules).unwrap();
        assert_eq!(db.get_competition_rules(&league_id).unwrap(), rules);

        let points = FantasyCompetitionRules::points();
        db.set_competition_rules(&league_id, &points).unwrap();
        assert_eq!(db.get_competition_rules(&league_id).unwrap(), points);
    }

    #[test]
    fn l1_competition_migration_preserves_existing_league_as_points() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE fl_leagues (
                id TEXT PRIMARY KEY,
                name TEXT UNIQUE NOT NULL,
                scheme TEXT NOT NULL,
                roster_shape TEXT NOT NULL,
                is_active INTEGER NOT NULL,
                created_at TEXT NOT NULL
            );
            INSERT INTO fl_leagues VALUES
                ('legacy', 'Legacy League', 'yahoo-standard', 'yahoo-standard', 1, '2026-01-01');",
        )
        .unwrap();
        run_migrations(&conn).unwrap();
        let (name, mode): (String, String) = conn
            .query_row(
                "SELECT name, competition_mode FROM fl_leagues WHERE id = 'legacy'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(name, "Legacy League");
        assert_eq!(mode, "points");
    }
}
