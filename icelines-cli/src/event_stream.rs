//! Phase Foster.3 — EventStream SQLite wrapper.
//!
//! Owns inserts and reads against the `events` table created in
//! migration 007. Insert is `INSERT OR REPLACE` so re-fetching the
//! same event (same composite PK) updates the payload + created_at
//! in place rather than duplicating the row. Caller supplies the
//! event_id via the format helpers in `icelines_core::event_stream`.

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use icelines_core::entity::EntityRef;
use rusqlite::Connection;

/// One row returned by EventStream queries. `payload` is the raw
/// JSON string; callers parse via the typed payload structs in
/// `icelines_core::event_stream`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventRow {
    pub date: String,
    pub entity_kind: String,
    pub entity_key: String,
    pub event_kind: String,
    pub event_id: String,
    pub payload: String,
    pub payload_version: u32,
    pub created_at: String,
}

/// Opaque handle to the events table. Reuses the same icelines.db
/// connection pool that GroupDb opens.
#[allow(dead_code)] // Read methods land with the Foster.2 wiring path; F.3 ships the writer + schema.
pub struct EventStream {
    conn: Connection,
}

#[allow(dead_code)] // Read API surface; consumers wire in via Foster.2 dashboard render path.
impl EventStream {
    /// Open the live `~/.icelines/icelines.db` (creates the file +
    /// runs migrations on first call). Identical bootstrap to
    /// `GroupDb::open`.
    pub fn open() -> Result<Self> {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(std::path::PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
        let dir = home.join(".icelines");
        std::fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
        let db_path = dir.join("icelines.db");
        let conn =
            Connection::open(&db_path).with_context(|| format!("open {}", db_path.display()))?;
        conn.execute_batch("PRAGMA journal_mode = WAL;")
            .context("set WAL mode")?;
        crate::db::run_migrations_for_test(&conn)?;
        Ok(Self { conn })
    }

    /// Open an in-memory db (test helper). Migrations run.
    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().context("open in-memory db")?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .context("enable foreign keys")?;
        crate::db::run_migrations_for_test(&conn)?;
        Ok(Self { conn })
    }

    /// Insert or replace an event. Returns `true` when this is a
    /// fresh insert, `false` when an existing row was overwritten.
    pub fn upsert(
        &self,
        date: NaiveDate,
        entity: &EntityRef,
        event_kind: &str,
        event_id: &str,
        payload_json: &str,
        payload_version: u32,
    ) -> Result<bool> {
        let date_str = date.format("%Y-%m-%d").to_string();
        let entity_kind = entity.kind_str();
        let entity_key = entity.key_str();
        let now = now_utc();

        let existed: bool = self
            .conn
            .query_row(
                "SELECT 1 FROM events \
                 WHERE date = ?1 AND entity_kind = ?2 AND entity_key = ?3 \
                   AND event_kind = ?4 AND event_id = ?5",
                rusqlite::params![&date_str, entity_kind, &entity_key, event_kind, event_id],
                |_| Ok(true),
            )
            .unwrap_or(false);

        self.conn
            .execute(
                "INSERT INTO events \
                 (date, entity_kind, entity_key, event_kind, event_id, \
                  payload, payload_version, created_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
                 ON CONFLICT(date, entity_kind, entity_key, event_kind, event_id) \
                 DO UPDATE SET payload = excluded.payload, \
                               payload_version = excluded.payload_version, \
                               created_at = excluded.created_at",
                rusqlite::params![
                    &date_str,
                    entity_kind,
                    &entity_key,
                    event_kind,
                    event_id,
                    payload_json,
                    payload_version,
                    &now,
                ],
            )
            .context("upsert event")?;
        Ok(!existed)
    }

    /// Events for a specific date, newest-first per entity.
    pub fn list_by_date(&self, date: NaiveDate) -> Result<Vec<EventRow>> {
        let date_str = date.format("%Y-%m-%d").to_string();
        let mut stmt = self.conn.prepare(
            "SELECT date, entity_kind, entity_key, event_kind, event_id, \
                    payload, payload_version, created_at \
             FROM events WHERE date = ?1 \
             ORDER BY entity_kind, entity_key, event_kind, event_id",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![&date_str], row_to_event)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Events for one entity within an inclusive date range.
    pub fn list_by_entity(
        &self,
        entity: &EntityRef,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Vec<EventRow>> {
        let kind = entity.kind_str();
        let key = entity.key_str();
        let mut stmt = self.conn.prepare(
            "SELECT date, entity_kind, entity_key, event_kind, event_id, \
                    payload, payload_version, created_at \
             FROM events \
             WHERE entity_kind = ?1 AND entity_key = ?2 \
               AND date >= ?3 AND date <= ?4 \
             ORDER BY date DESC, event_kind, event_id",
        )?;
        let rows = stmt
            .query_map(
                rusqlite::params![
                    kind,
                    &key,
                    &start.format("%Y-%m-%d").to_string(),
                    &end.format("%Y-%m-%d").to_string(),
                ],
                row_to_event,
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Count of events on a date — used by Foster.2's favorites
    /// view header to render "N events" without loading bodies.
    pub fn count_by_date(&self, date: NaiveDate) -> Result<usize> {
        let date_str = date.format("%Y-%m-%d").to_string();
        let n: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM events WHERE date = ?1",
                rusqlite::params![&date_str],
                |r| r.get(0),
            )
            .context("count_by_date")?;
        Ok(n as usize)
    }
}

fn row_to_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<EventRow> {
    Ok(EventRow {
        date: row.get(0)?,
        entity_kind: row.get(1)?,
        entity_key: row.get(2)?,
        event_kind: row.get(3)?,
        event_id: row.get(4)?,
        payload: row.get(5)?,
        payload_version: row.get::<_, i64>(6)? as u32,
        created_at: row.get(7)?,
    })
}

fn now_utc() -> String {
    let now: DateTime<Utc> = Utc::now();
    now.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_core::event_stream as proto;
    use icelines_core::identity::{GameId, PlayerId};
    use icelines_core::model::TeamAbbr;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn score_payload() -> String {
        let p = proto::ScorePayloadV1::new(
            GameId(2025020342),
            TeamAbbr("EDM".into()),
            TeamAbbr("CGY".into()),
            7,
            3,
            "REG",
        );
        serde_json::to_string(&p).unwrap()
    }

    /// L1 / l1_foster3_eventstream_insert_then_list_by_date
    #[test]
    fn l1_foster3_eventstream_insert_then_list_by_date() {
        let es = EventStream::open_in_memory().unwrap();
        let game = GameId(2025020342);
        let entity = EntityRef::Game(game);
        let event_id = proto::score_final_event_id(game);
        let inserted = es
            .upsert(
                d(2026, 1, 15),
                &entity,
                "score",
                &event_id,
                &score_payload(),
                proto::SCORE_PAYLOAD_VERSION,
            )
            .unwrap();
        assert!(inserted, "first insert reports new row");

        let rows = es.list_by_date(d(2026, 1, 15)).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].entity_kind, "game");
        assert_eq!(rows[0].entity_key, "2025020342");
        assert_eq!(rows[0].event_kind, "score");
        assert_eq!(rows[0].event_id, event_id);
        assert_eq!(rows[0].payload_version, 1);
    }

    /// L1 / l1_foster3_eventstream_dedup_via_on_conflict
    /// — Re-inserting the same composite PK overwrites the payload
    ///   instead of duplicating the row (TAPE H3).
    #[test]
    fn l1_foster3_eventstream_dedup_via_on_conflict() {
        let es = EventStream::open_in_memory().unwrap();
        let game = GameId(2025020342);
        let entity = EntityRef::Game(game);
        let event_id = proto::score_final_event_id(game);

        let first = es
            .upsert(
                d(2026, 1, 15),
                &entity,
                "score",
                &event_id,
                "{\"schema_version\":1,\"home_score\":1,\"away_score\":0}",
                1,
            )
            .unwrap();
        assert!(first);

        let second = es
            .upsert(
                d(2026, 1, 15),
                &entity,
                "score",
                &event_id,
                "{\"schema_version\":1,\"home_score\":7,\"away_score\":3}",
                1,
            )
            .unwrap();
        assert!(!second, "second insert reports overwrite");

        let rows = es.list_by_date(d(2026, 1, 15)).unwrap();
        assert_eq!(rows.len(), 1, "PK enforces single row");
        assert!(
            rows[0].payload.contains("\"home_score\":7"),
            "payload was overwritten, got: {}",
            rows[0].payload
        );
    }

    /// L1 / l1_foster3_eventstream_distinct_event_ids_coexist
    /// — Same (date, entity, event_kind) but different event_id
    ///   (e.g. period:2 vs final) coexist as separate rows.
    #[test]
    fn l1_foster3_eventstream_distinct_event_ids_coexist() {
        let es = EventStream::open_in_memory().unwrap();
        let game = GameId(2025020342);
        let entity = EntityRef::Game(game);
        for ev_id in [
            proto::score_period_event_id(game, "1"),
            proto::score_period_event_id(game, "2"),
            proto::score_final_event_id(game),
        ] {
            es.upsert(
                d(2026, 1, 15),
                &entity,
                "score",
                &ev_id,
                &score_payload(),
                1,
            )
            .unwrap();
        }
        let rows = es.list_by_date(d(2026, 1, 15)).unwrap();
        assert_eq!(rows.len(), 3, "three distinct event_ids → three rows");
    }

    /// L1 / l1_foster3_eventstream_list_by_entity_within_range
    #[test]
    fn l1_foster3_eventstream_list_by_entity_within_range() {
        let es = EventStream::open_in_memory().unwrap();
        let player = EntityRef::Player(PlayerId(8478402));

        // Three milestone-style events for one player across three
        // distinct dates.
        for (date, value) in [(d(2026, 1, 10), 100), (d(2026, 1, 15), 200), (d(2026, 1, 20), 300)] {
            let payload = format!("{{\"schema_version\":1,\"value\":{value}}}");
            let event_id = proto::milestone_event_id(PlayerId(8478402), "goals", value);
            es.upsert(date, &player, "milestone", &event_id, &payload, 1)
                .unwrap();
        }

        // Window the middle date only.
        let rows = es
            .list_by_entity(&player, d(2026, 1, 14), d(2026, 1, 16))
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].payload.contains("\"value\":200"));

        // Window all three.
        let rows = es
            .list_by_entity(&player, d(2026, 1, 1), d(2026, 1, 31))
            .unwrap();
        assert_eq!(rows.len(), 3);
        // Newest first by date DESC.
        assert_eq!(rows[0].date, "2026-01-20");
        assert_eq!(rows[2].date, "2026-01-10");
    }

    /// L1 / l1_foster3_eventstream_count_by_date
    #[test]
    fn l1_foster3_eventstream_count_by_date() {
        let es = EventStream::open_in_memory().unwrap();
        assert_eq!(es.count_by_date(d(2026, 1, 15)).unwrap(), 0);

        let entity = EntityRef::Game(GameId(2025020001));
        es.upsert(
            d(2026, 1, 15),
            &entity,
            "score",
            &proto::score_final_event_id(GameId(2025020001)),
            &score_payload(),
            1,
        )
        .unwrap();
        assert_eq!(es.count_by_date(d(2026, 1, 15)).unwrap(), 1);
        assert_eq!(es.count_by_date(d(2026, 1, 16)).unwrap(), 0);
    }

    /// L1 / l1_foster3_eventstream_payload_version_round_trips
    #[test]
    fn l1_foster3_eventstream_payload_version_round_trips() {
        let es = EventStream::open_in_memory().unwrap();
        let entity = EntityRef::Player(PlayerId(8478402));
        es.upsert(
            d(2026, 1, 15),
            &entity,
            "milestone",
            "milestone:8478402:goals:1000",
            "{}",
            42, // arbitrary version to confirm round-trip
        )
        .unwrap();
        let rows = es.list_by_date(d(2026, 1, 15)).unwrap();
        assert_eq!(rows[0].payload_version, 42);
    }
}
