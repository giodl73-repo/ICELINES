# EventStream — frozen v1 payload schemas

**Status**: Spec — implemented in `icelines-core::event_stream` (Phase Foster.3)
**Parent**: `foster-favorites-dashboard.md`
**Date**: 2026-05-06

---

## Why

The EventStream SQLite table holds heterogeneous events
(`score` / `trade` / `signing` / `milestone` / `streak`) for the
favorites timeline view. Each row carries the event_kind + a
caller-supplied event_id (composite PK) plus a JSON `payload` body
versioned independently per kind. Bumping one schema doesn't break
readers of the others.

This document **freezes the v1 shape** of each payload kind. Any
incompatible change must bump the kind's `payload_version` const in
`icelines-core::event_stream` AND add a `_v2` struct that
deserializes against the new shape. Old rows continue to round-trip
via the `_v1` struct as long as their stored version matches.

---

## Versioning constants

Defined in `icelines-core::event_stream`:

| Const | Value | Bump when |
|---|---|---|
| `SCORE_PAYLOAD_VERSION` | 1 | Change to `home`/`away`/`result`/`favorited_*` shape |
| `TRADE_PAYLOAD_VERSION` | 1 | Change to `from_team`/`to_team`/`players_*` shape |
| `SIGNING_PAYLOAD_VERSION` | 1 | Change to `player`/`term`/`AAV` shape |
| `MILESTONE_PAYLOAD_VERSION` | 1 | Change to `metric`/`value`/`in_game` shape |
| `STREAK_PAYLOAD_VERSION` | 1 | Change to `kind`/`length`/`start`/`end` shape |

---

## event_id formats

Caller-supplied dedup keys. The composite PK
`(date, entity_kind, entity_key, event_kind, event_id)` enforces
uniqueness; `INSERT … ON CONFLICT DO UPDATE` overwrites on collision
so re-fetched events update in place.

| event_kind | event_id format | Helper | Example |
|---|---|---|---|
| `score` | `score:GAMEID:final` | `score_final_event_id(GameId)` | `score:2025020342:final` |
| `score` | `score:GAMEID:period:N` | `score_period_event_id(GameId, &str)` | `score:2025020342:period:2` |
| `trade` | `trade:DATE:teams_sorted_alpha` | `trade_event_id(date, &TeamAbbr, &TeamAbbr)` | `trade:2026-01-15:bos-fla` |
| `signing` | `signing:DATE:player_id` | `signing_event_id(date, PlayerId)` | `signing:2025-07-01:8478402` |
| `milestone` | `milestone:player_id:metric:value` | `milestone_event_id(PlayerId, &str, u32)` | `milestone:8478402:goals:1000` |
| `streak` | `streak:ENTITY_REF:start_date` | `streak_event_id(&EntityRef, date)` | `streak:player:8478402:2025-12-01` |

**Trade dedup invariant**: `trade_event_id(date, A, B) == trade_event_id(date, B, A)`. The helper sorts + lowercases the abbrevs so the same trade entered with teams in either order produces the same key.

---

## Payload shapes

### `score` (v1)

```rust
pub struct ScorePayloadV1 {
    pub schema_version: u32,        // = 1
    pub game_id: GameId,
    pub home_team: TeamAbbr,
    pub away_team: TeamAbbr,
    pub home_score: u32,
    pub away_score: u32,
    pub result: String,             // "REG" | "OT" | "SO" | "LIVE" | "PRE" | "FUT"
    pub lead_changes: u32,          // optional, defaults 0
    pub favorited_skater_lines: Vec<EntityRef>,
    pub favorited_goalie_lines: Vec<EntityRef>,
}
```

**Wire example**:
```jsonc
{
  "schema_version": 1,
  "game_id": 2025020342,
  "home_team": "EDM",
  "away_team": "CGY",
  "home_score": 7,
  "away_score": 3,
  "result": "REG",
  "lead_changes": 2,
  "favorited_skater_lines": ["player:8478402"],
  "favorited_goalie_lines": ["player:8475670"]
}
```

The `favorited_*` arrays are populated when Foster.3+ wires the
boxscore parse path; today they're empty and the score event still
records the slate-level summary.

### `trade` (v1)

```rust
pub struct TradePayloadV1 {
    pub schema_version: u32,                // = 1
    pub from_team: TeamAbbr,
    pub to_team: TeamAbbr,
    pub players_sent: Vec<EntityRef>,
    pub players_received: Vec<EntityRef>,
    pub draft_picks_sent: Vec<String>,      // e.g. "2026-1st", "2027-3rd"
    pub draft_picks_received: Vec<String>,
    pub description: String,
}
```

`from_team` and `to_team` are perspective-bound — the perspective
follows the favorited team's side of the trade. Trades involving
two favorited teams generate two rows (one per perspective).

### `signing` (v1)

Reserved shape — production wiring pending. Expected fields:

```rust
pub struct SigningPayloadV1 {
    pub schema_version: u32,
    pub player: EntityRef,
    pub team: TeamAbbr,
    pub years: u8,
    pub aav: u64,                   // average annual value, dollars
    pub total: u64,
    pub structure: Option<String>,  // ELC, RFA, UFA, extension
}
```

### `milestone` (v1)

```rust
pub struct MilestonePayloadV1 {
    pub schema_version: u32,        // = 1
    pub player: EntityRef,
    pub metric: String,             // "goals" | "points" | "wins" | "shutouts" | …
    pub value: u32,                 // the threshold reached (1000, 500, …)
    pub in_game: Option<GameId>,    // game in which the milestone fired
}
```

### `streak` (v1)

```rust
pub struct StreakPayloadV1 {
    pub schema_version: u32,        // = 1
    pub entity: EntityRef,
    pub kind: String,               // "point_streak" | "win_streak" | "shutout_streak"
    pub length: u32,                // games / appearances
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,// None = live streak
    pub active: bool,
}
```

---

## Reader contract

When loading a row, callers MUST gate parse on `payload_version`:

```rust
let row: EventRow = es.list_by_date(date)?[0].clone();
match (row.event_kind.as_str(), row.payload_version) {
    ("score", 1) => {
        let p: ScorePayloadV1 = serde_json::from_str(&row.payload)?;
        // …render p
    }
    ("score", n) => {
        // Future v2+: parse via ScorePayloadV2; here surface as
        // "newer schema, please upgrade" if reader is older.
        eprintln!("score v{n} not supported by this reader");
    }
    _ => {}
}
```

`payload_version` is per-kind; a `trade` row with version 1 isn't
related to a `score` row with version 1.

---

## Migration policy

Schema-bump checklist:

1. Bump the const (e.g. `SCORE_PAYLOAD_VERSION = 2`).
2. Add `ScorePayloadV2` struct with the new shape.
3. Keep `ScorePayloadV1` defined and the old reader path live for
   at least one minor version so existing on-disk rows still parse.
4. Update the matrix in this doc.
5. Bump `MIN_READER_VERSION` in `icelines-fetch::manifest` ONLY if
   readers below the new floor would silently mis-parse. (Most
   payload bumps don't require this — readers gate per-row.)

---

## Tests

L0 round-trip tests live in
`icelines-core::event_stream::tests::l0_foster3_*_payload_v1_round_trip`.
Each one builds a struct, serializes to JSON, parses back, and
asserts equality on all required fields. Adding a new payload kind
adds the matching `l0_foster3_<kind>_payload_v1_round_trip` test.
