//! Phase Foster.3 — EventStream payload schemas + event_id format helpers.
//!
//! The EventStream SQLite table lives one layer up (icelines-cli's
//! db.rs); this module owns the **payload shapes** and the
//! **event_id format strings** so the dedup key + the JSON body are
//! generated in one place. Any future surface that wants to insert
//! into the stream calls these helpers.
//!
//! Payload schemas are **versioned per event_kind**. A bump to the
//! score-payload shape changes only the score `payload_version`, not
//! the others — readers gate on `payload_version` per row.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::entity::EntityRef;
use crate::identity::{GameId, PlayerId};
use crate::model::TeamAbbr;

pub const SCORE_PAYLOAD_VERSION: u32 = 1;
pub const TRADE_PAYLOAD_VERSION: u32 = 1;
pub const SIGNING_PAYLOAD_VERSION: u32 = 1;
pub const MILESTONE_PAYLOAD_VERSION: u32 = 1;
pub const STREAK_PAYLOAD_VERSION: u32 = 1;

// ── event_id formatters ─────────────────────────────────────────────────────

/// `score:GAMEID:final` — full game result snapshot.
pub fn score_final_event_id(game: GameId) -> String {
    format!("score:{}:final", game.0)
}

/// `score:GAMEID:period:N` — period-end snapshot (optional, used for
/// the live-tracking surface). N is 1, 2, 3, or "OT" / "SO".
pub fn score_period_event_id(game: GameId, period: &str) -> String {
    format!("score:{}:period:{}", game.0, period)
}

/// `trade:DATE:teams_sorted_alpha`. Caller supplies the team
/// abbreviations; this helper sorts + lowercases them so two teams
/// passed in either order produce the same dedup key.
pub fn trade_event_id(date: NaiveDate, team_a: &TeamAbbr, team_b: &TeamAbbr) -> String {
    let a = team_a.0.to_lowercase();
    let b = team_b.0.to_lowercase();
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    format!("trade:{}:{lo}-{hi}", date.format("%Y-%m-%d"))
}

/// `signing:DATE:player_id`.
pub fn signing_event_id(date: NaiveDate, player: PlayerId) -> String {
    format!("signing:{}:{}", date.format("%Y-%m-%d"), player.0)
}

/// `milestone:player_id:metric:value`. `metric` is a snake_case
/// short string (e.g. "goals", "points", "wins"); value is the
/// integer threshold reached.
pub fn milestone_event_id(player: PlayerId, metric: &str, value: u32) -> String {
    format!("milestone:{}:{metric}:{value}", player.0)
}

/// `streak:ENTITY_REF:start_date`. Uses the canonical Display form
/// of the entity ref so the player/team/game discriminator stays in
/// the dedup key.
pub fn streak_event_id(entity: &EntityRef, start: NaiveDate) -> String {
    format!("streak:{entity}:{}", start.format("%Y-%m-%d"))
}

// ── Payload schemas ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScorePayloadV1 {
    pub schema_version: u32,
    pub game_id: GameId,
    pub home_team: TeamAbbr,
    pub away_team: TeamAbbr,
    pub home_score: u32,
    pub away_score: u32,
    /// Mirrors NHL boxscore `gameOutcome.lastPeriodType` semantics:
    /// `"REG"` / `"OT"` / `"SO"` for finalized games; `"LIVE"` /
    /// `"PRE"` / `"FUT"` while in flight.
    pub result: String,
    #[serde(default)]
    pub lead_changes: u32,
    /// Skater lines for any favorited players in this game. Empty
    /// when no favorites participated.
    #[serde(default)]
    pub favorited_skater_lines: Vec<EntityRef>,
    #[serde(default)]
    pub favorited_goalie_lines: Vec<EntityRef>,
}

impl ScorePayloadV1 {
    pub fn new(
        game: GameId,
        home: TeamAbbr,
        away: TeamAbbr,
        home_score: u32,
        away_score: u32,
        result: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: SCORE_PAYLOAD_VERSION,
            game_id: game,
            home_team: home,
            away_team: away,
            home_score,
            away_score,
            result: result.into(),
            lead_changes: 0,
            favorited_skater_lines: Vec::new(),
            favorited_goalie_lines: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradePayloadV1 {
    pub schema_version: u32,
    pub from_team: TeamAbbr,
    pub to_team: TeamAbbr,
    #[serde(default)]
    pub players_sent: Vec<EntityRef>,
    #[serde(default)]
    pub players_received: Vec<EntityRef>,
    #[serde(default)]
    pub draft_picks_sent: Vec<String>,
    #[serde(default)]
    pub draft_picks_received: Vec<String>,
    #[serde(default)]
    pub description: String,
}

impl TradePayloadV1 {
    pub fn new(from: TeamAbbr, to: TeamAbbr) -> Self {
        Self {
            schema_version: TRADE_PAYLOAD_VERSION,
            from_team: from,
            to_team: to,
            players_sent: Vec::new(),
            players_received: Vec::new(),
            draft_picks_sent: Vec::new(),
            draft_picks_received: Vec::new(),
            description: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MilestonePayloadV1 {
    pub schema_version: u32,
    pub player: EntityRef,
    pub metric: String,
    pub value: u32,
    /// Game in which the milestone was reached, when known.
    #[serde(default)]
    pub in_game: Option<GameId>,
}

impl MilestonePayloadV1 {
    pub fn new(player: EntityRef, metric: impl Into<String>, value: u32) -> Self {
        Self {
            schema_version: MILESTONE_PAYLOAD_VERSION,
            player,
            metric: metric.into(),
            value,
            in_game: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreakPayloadV1 {
    pub schema_version: u32,
    pub entity: EntityRef,
    pub kind: String,           // "point_streak" / "win_streak" / "shutout_streak"
    pub length: u32,
    pub start_date: NaiveDate,
    #[serde(default)]
    pub end_date: Option<NaiveDate>,
    #[serde(default)]
    pub active: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn l0_foster3_score_event_id_final_format() {
        let id = score_final_event_id(GameId(2025020342));
        assert_eq!(id, "score:2025020342:final");
    }

    #[test]
    fn l0_foster3_score_event_id_period_format() {
        let id = score_period_event_id(GameId(2025020342), "2");
        assert_eq!(id, "score:2025020342:period:2");
        let ot = score_period_event_id(GameId(2025020342), "OT");
        assert_eq!(ot, "score:2025020342:period:OT");
    }

    #[test]
    fn l0_foster3_trade_event_id_sorts_teams_alphabetically() {
        // Same trade entered with teams in either order produces
        // the same dedup key — protects against double-recording
        // when the API surfaces both sides.
        let date = d(2026, 1, 15);
        let bos = TeamAbbr("BOS".into());
        let fla = TeamAbbr("FLA".into());
        let id1 = trade_event_id(date, &bos, &fla);
        let id2 = trade_event_id(date, &fla, &bos);
        assert_eq!(id1, id2);
        assert_eq!(id1, "trade:2026-01-15:bos-fla");
    }

    #[test]
    fn l0_foster3_signing_event_id_format() {
        let id = signing_event_id(d(2025, 7, 1), PlayerId(8478402));
        assert_eq!(id, "signing:2025-07-01:8478402");
    }

    #[test]
    fn l0_foster3_milestone_event_id_format() {
        let id = milestone_event_id(PlayerId(8478402), "goals", 1000);
        assert_eq!(id, "milestone:8478402:goals:1000");
    }

    #[test]
    fn l0_foster3_streak_event_id_includes_entity_kind() {
        let r = EntityRef::Player(PlayerId(8478402));
        let id = streak_event_id(&r, d(2025, 12, 1));
        assert_eq!(id, "streak:player:8478402:2025-12-01");
    }

    #[test]
    fn l0_foster3_score_payload_v1_round_trip() {
        let p = ScorePayloadV1::new(
            GameId(2025020342),
            TeamAbbr("EDM".into()),
            TeamAbbr("CGY".into()),
            7,
            3,
            "REG",
        );
        let s = serde_json::to_string(&p).unwrap();
        let back: ScorePayloadV1 = serde_json::from_str(&s).unwrap();
        assert_eq!(back.schema_version, 1);
        assert_eq!(back.home_score, 7);
        assert_eq!(back.result, "REG");
    }

    #[test]
    fn l0_foster3_trade_payload_v1_round_trip() {
        let p = TradePayloadV1::new(TeamAbbr("BOS".into()), TeamAbbr("FLA".into()));
        let s = serde_json::to_string(&p).unwrap();
        let back: TradePayloadV1 = serde_json::from_str(&s).unwrap();
        assert_eq!(back.schema_version, 1);
        assert_eq!(back.from_team.0, "BOS");
    }

    #[test]
    fn l0_foster3_milestone_payload_v1_round_trip() {
        let p = MilestonePayloadV1::new(
            EntityRef::Player(PlayerId(8478402)),
            "goals",
            1000,
        );
        let s = serde_json::to_string(&p).unwrap();
        let back: MilestonePayloadV1 = serde_json::from_str(&s).unwrap();
        assert_eq!(back.value, 1000);
        assert_eq!(back.metric, "goals");
    }

    #[test]
    fn l0_foster3_payload_versions_pinned_at_one() {
        // Pin: bumping any schema version means callers must
        // re-read this test and acknowledge the migration.
        assert_eq!(SCORE_PAYLOAD_VERSION, 1);
        assert_eq!(TRADE_PAYLOAD_VERSION, 1);
        assert_eq!(SIGNING_PAYLOAD_VERSION, 1);
        assert_eq!(MILESTONE_PAYLOAD_VERSION, 1);
        assert_eq!(STREAK_PAYLOAD_VERSION, 1);
    }
}
