# Phase Foster — Favorites Dashboard (Foster.2 + Foster.3)

**Parent**: `foster-overview.md`
**Plan**: `design/plans/2026-05-06-phaseFoster-favorites.md`
**Status**: Spec — ready for implementation

---

## Goal

A new surface on every medium (CLI, TUI, Web) that aggregates
everything about the user's favorited players + teams for a given
date: stat lines from games played that night, team scores,
recent transactions, milestones, streaks. Reads off the EventStream
populated by Foster.3.

## Surfaces

**CLI** — `icelines favorites [--date YYYY-MM-DD] [--week|--month] [--json|--csv]`

```
FAVORITES — 2026-01-15 (Mon)
══════════════════════════════════════════════════════════════
Players (3 favorited)
  Connor McDavid     EDM 7-3 W vs CGY    1G 2A 3P  TOI 22:14  +2  4 SOG
  Connor Hellebuyck  WPG 4-1 W @ MIN     34/35  SV%.971  GAA 1.00  W
  Brad Marchand      FLA — DNP (rest)
Teams (2 favorited)
  EDM                7-3 W vs CGY        Skinner 32 SV  Top: McDavid 3P
  TOR                — bye
Last 7 days for your favorites:
  McDavid: 7 GP · 5G 12A 17P · +8 · 13.7 SOG/g
  Hellebuyck: 6 GP · 4-2-0 · .934 SV% · 2.05 GAA
  EDM: 5-2-0 · +9 GD · 3rd in Pacific
```

**TUI** — new tab `Shift+F`. Admin overlay (currently `F`) moves to
`Shift+A` (GLASS H3). Tab order: League / Depth / Stats / Goalies /
Favorites / Scores / Schedule / Playoffs / Transactions overlays
remain. Favorites placement: between Goalies and Scores (one slot
left of mid-cycle) — minimizes disruption to right-side tabs which
are most-used.

**Web** — `/favorites?date=YYYY-MM-DD&range=day|week|month` HTML;
`/api/v1/favorites` JSON twin (envelope detail below).

## Data shape

### `FavoritesView` (icelines-core/src/favorites.rs — new)

```rust
pub struct FavoritesView {
    pub date: NaiveDate,
    pub range: TimeRange,        // Day | Week | Month
    pub players: Vec<PlayerNightRow>,
    pub teams: Vec<TeamNightRow>,
    pub events: Vec<EventRow>,
    pub aggregate: AggregateView,    // last-7-days summary etc.
}

pub enum PlayerNightRow {
    Skater(SkaterNightLine),
    Goalie(GoalieNightLine),
    DidNotPlay { player: EntityRef, reason: DnpReason },
}

pub enum DnpReason {
    Scratched,           // in roster, not in boxscore
    InjuredReserve,      // IR list (when API exposes — defer to ad-hoc check)
    TeamBye,             // team didn't play that night
    Recalled,            // sent down/recalled (track via transactions)
    DataPending,         // boxscore not yet fetched/finalized
}
```

**Two distinct night-line schemas** (SCOUT B1) — goalie and skater
projections diverge, so one struct doesn't fit:

```rust
pub struct SkaterNightLine {
    pub player: EntityRef,
    pub team: TeamAbbr,           // team in THIS game (mid-day-trade-aware)
    pub opponent: TeamAbbr,
    pub home_or_away: HomeAway,
    pub team_score: u32,
    pub opponent_score: u32,
    pub result: GameResult,        // Win | Loss | OTLoss | InProgress
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
    pub plus_minus: i32,
    pub shots: Option<u32>,
    pub hits: Option<u32>,         // None when game not finalized
    pub blocks: Option<u32>,
    pub pim: Option<u32>,
    pub toi_seconds: Option<u32>,
    pub power_play_goals: u32,
    pub power_play_assists: u32,
    pub shorthanded_goals: u32,
    pub game_state: GameState,    // FUT | LIVE | FINAL | OFF
}

pub struct GoalieNightLine {
    pub player: EntityRef,
    pub team: TeamAbbr,
    pub opponent: TeamAbbr,
    pub home_or_away: HomeAway,
    pub team_score: u32,
    pub opponent_score: u32,
    pub games_started: bool,
    pub decision: Option<Decision>,    // W | L | OTL | None (relief w/o decision)
    pub saves: u32,
    pub shots_against: u32,
    pub goals_against: u32,
    pub save_pct: f32,                  // computed: 1 - GA/SA
    pub gaa: f32,                       // computed: GA / (TOI/3600)
    pub toi_seconds: Option<u32>,
    pub shutout: bool,
    pub game_state: GameState,
}
```

### Hits/blocks gating (SCOUT B2)

`hits`, `blocks`, `pim`, `takeaways`, `giveaways` default to 0 in the
NHL API on in-progress boxscores instead of `None`. Foster reads the
game's `game_state` and presents these fields:

- `game_state ∈ {FUT, LIVE, PRE}` → render as `—` (unfinalized)
- `game_state ∈ {OFF, FINAL}` → render the integer (real value)

Schema: `Option<u32>` carrying `None` when state-gated, `Some(0)` when
truly zero. Renderer disambiguates.

### Mid-day trade attribution (SCOUT H3)

When a player is traded on a game day, attribution goes to **the
team in the boxscore** (NHL API's source of truth), even if the
favorited team is the *other* one. A "TRADE" event lands in the
EventStream that day so the favorites view shows it inline:

```
Brad Marchand   FLA → BOS   trade · Coyle + 2026 1st (now favorited via FLA)
                BOS 2-1 W vs MTL    1G 0A 1P  TOI 17:42  +1
```

Display rule: if a favorited team's row would be empty for that night
(player traded out earlier), omit the team-row collapse and show the
TRADE event with the actual game line below.

### Goalie pull / multi-goalie handling (SCOUT H5)

When `boxscore.goalies[]` has multiple entries:

1. Prefer the goalie with `decision != None` (W/L/OTL)
2. If none has a decision (pulled mid-game on both sides), pick
   longest TOI
3. Surface BOTH if both played > 5 min (split-decision viewer hint)

### Career history augment for newly-favorited (SCOUT M7)

When user runs `group add Favorites <PlayerName>` for a player whose
NHL id isn't in the local career_history store:

1. `DataStore::load_career_history(pid)` returns `None`
2. If `live_feeds && !test_mode`: lazy-fetch via
   `NhlApiClient::fetch_player_career_history`
3. Write to `~/.icelines/data/career_history.json` (existing CHS path)
4. If fetch fails: append a stub `CareerHistory { player_id: pid,
   stints: vec![] }` so the dashboard renders "no NHL games yet" not
   an error

L1 test required: rookie pid with empty `seasonTotals` → favorites
view shows the player row with current-season-only data + "career
history pending" footer.

### Per-night vs season-total filter scope (EDGE H1)

`icelines favorites --filter "p>=2"` filters **per-night stat lines**
(2+ points tonight), NOT season totals. Season-total filtering stays
on `query leaders`. Documented in COMMANDS.md.

### Aggregate view ("Last 7 days for your favorites")

`AggregateView` contains rollups across the active range:

```rust
pub struct AggregateView {
    pub range_start: NaiveDate,
    pub range_end: NaiveDate,
    pub player_rollups: Vec<PlayerRollup>,
    pub team_rollups: Vec<TeamRollup>,
}

pub struct PlayerRollup {
    pub player: EntityRef,
    pub games_played: u32,        // counts only nights with a SkaterLine + game finalized
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
    pub plus_minus: i32,
    pub shots_per_game: f32,
    // Goalie rollup is a separate enum variant (W/L/SV%/GAA)
}
```

`games_played` counts only nights where the player has a
`PlayerNightRow::Skater | Goalie` AND `game_state ∈ {OFF, FINAL}`.
DNPs and unfinalized games don't inflate the denominator (SCOUT M8).

## EventStream (Foster.3)

SQLite table for the temporal "what happened on date X for entity Y"
view. Foster.2's favorites read off this; future surfaces (timeline
view, RSS feed) can read off the same stream.

```sql
CREATE TABLE events (
    date            TEXT NOT NULL,         -- YYYY-MM-DD
    entity_kind     TEXT NOT NULL,         -- 'player' | 'team' | 'game'
    entity_key      TEXT NOT NULL,         -- pid digits / team abbrev / game_id digits
    event_kind      TEXT NOT NULL,         -- 'score' | 'trade' | 'milestone' | 'streak'
    event_id        TEXT NOT NULL,         -- caller-supplied dedup key
    payload         TEXT NOT NULL,         -- per-event-kind JSON
    payload_version INTEGER NOT NULL,      -- per event_kind, frozen schema
    created_at      TEXT NOT NULL,
    PRIMARY KEY (date, entity_kind, entity_key, event_kind, event_id)
);
CREATE INDEX events_by_date ON events(date DESC);
CREATE INDEX events_by_entity ON events(entity_kind, entity_key, date DESC);
```

**PK fix** (TAPE H3 + FORGE M3): `event_id` replaces `payload` in the
PK. Caller supplies a stable dedup key per event. Re-fetched events
update via `INSERT … ON CONFLICT DO UPDATE SET payload=excluded.payload,
created_at=excluded.created_at`.

**Event_id formats** (frozen, per kind):

| event_kind | event_id format | Example |
|---|---|---|
| `score` | `score:GAMEID:final` | `score:2025020342:final` |
| `score` | `score:GAMEID:period:N` | `score:2025020342:period:2` (period-end snapshots) |
| `trade` | `trade:DATE:teams_sorted_alpha` | `trade:2026-01-15:bos-fla` |
| `signing` | `signing:DATE:player_id` | `signing:2025-07-01:8478402` |
| `milestone` | `milestone:player_id:metric:value` | `milestone:8478402:goals:1000` |
| `streak` | `streak:entity_ref:start_date` | `streak:player:8478402:2025-12-01` |

**Payload schemas** (frozen, versioned per `event_kind`). Full docs in
`design/specs/event-stream-payloads.md` (new doc, child of this one;
land in Foster.3). Sketch:

```jsonc
// score v1
{ "schema_version": 1, "home": {...}, "away": {...}, "result": "FINAL_OT",
  "lead_changes": 2, "favorited_skater_lines": [...], "favorited_goalie_lines": [...] }

// trade v1
{ "schema_version": 1, "from_team": "BOS", "to_team": "FLA",
  "players_sent": ["player:8470829"], "draft_picks_sent": ["2026-1st"],
  "players_received": ["player:8478402"], "description": "..." }

// milestone v1
{ "schema_version": 1, "metric": "goals", "value": 1000,
  "in_game_id": "game:2025020342" }
```

Insertions are **transactional** (TAPE H4): SQLite tx → fsync the
boxscore JSON → manifest update last (manifest is the commit point).

## JSON envelope — `/api/v1/favorites` (WIRE B1)

Heterogeneous `data` object, **explicitly breaking the homogeneous-
array convention** of King.2.4. Documented inline:

```json
{
  "schema_version": 1,
  "route": "favorites",
  "data": {
    "players": [
      { "kind": "skater", "entity_ref": "player:8478402", "team": "EDM",
        "opponent": "CGY", "home_or_away": "home", "team_score": 7,
        "opponent_score": 3, "result": "win",
        "stats": { "g": 1, "a": 2, "p": 3, "plus_minus": 2, "toi_sec": 1334,
                   "shots": 4, "hits": 2, "blocks": 0, "pim": 0,
                   "pp_g": 0, "pp_a": 1, "sh_g": 0 },
        "game_state": "FINAL" },
      { "kind": "goalie", "entity_ref": "player:8476945", "team": "WPG",
        "opponent": "MIN", "team_score": 4, "opponent_score": 1,
        "stats": { "saves": 34, "shots_against": 35, "goals_against": 1,
                   "save_pct": 0.9714, "gaa": 1.00, "decision": "W",
                   "started": true, "shutout": false },
        "game_state": "FINAL" },
      { "kind": "dnp", "entity_ref": "player:8470829", "team": "FLA",
        "reason": "scratched" }
    ],
    "teams": [
      { "entity_ref": "team:EDM", "score": "7-3", "result": "win",
        "opponent": "CGY", "top_skater": "player:8478402", "top_goalie": "player:8475670" },
      { "entity_ref": "team:TOR", "result": "bye" }
    ],
    "events": [
      { "event_kind": "trade", "date": "2026-01-15",
        "entity_ref": "player:8470829", "payload_version": 1, "payload": { ... } }
    ]
  },
  "meta": {
    "date": "2026-01-15",
    "range": "day",
    "group_id": 7,
    "group_name": "Favorites",
    "counts": { "players": 3, "teams": 2, "events": 1 },
    "active_filters": []
  }
}
```

## TUI specifics

**Tab insertion**: Favorites between Goalies and Scores (GLASS H4 —
Goalies/Scores/Schedule/Playoffs are right-side and most-used; one
slot earlier minimizes disruption).

**Empty-state card** (GLASS M6) when no favorites group exists or
the group is empty:

```
                ╭──────────── Favorites ────────────╮
                │                                    │
                │  No favorites yet.                 │
                │                                    │
                │  Press `g` on any player or team   │
                │  to add to a group, then mark      │
                │  that group as Favorites.          │
                │                                    │
                │  Or run from the CLI:              │
                │    icelines group add Favorites \  │
                │      McDavid                       │
                │    icelines group add Favorites \  │
                │      EDM                           │
                │                                    │
                ╰────────────────────────────────────╯
```

**Loading affordance** (GLASS M7) when boxscores for the active date
haven't been fetched yet:

```
Loading favorites for 2014-10-08…
Fetching 4 boxscores from NHL API
███████████░░░░░░░  3/4 (45 ms avg)
```

Mirrors UX.1 "Loading-career placeholder" pattern.

## Test plan (BENCH H1+H3)

**Foster.2 = 18 tests + 6 personas**:

- L0 projection (8): SkaterNightLine vs GoalieNightLine routing,
  hits/blocks gating on game_state, DNP classification (scratched /
  team-bye / data-pending), mid-day trade attribution, goalie-pull
  decision picking, aggregate.games_played excludes DNPs, empty
  group renders empty-state, week-aggregate sums on the fly
- L1 web (4): `/favorites` 200 with empty group, with players, with
  teams, with both; `/api/v1/favorites` envelope shape with
  heterogeneous `data` documented
- L2 CLI (6): `favorites` exits 0 with empty store, with populated
  store, `--date 2014-10-08` past-date, `--week` aggregate,
  `--json` envelope, `--filter "p>=2"` per-night scoped

**Personas (6)**:
- Setup-then-favorite (drop ~/.icelines, setup wizard, group add, view)
- Mid-day trade for a favorited player
- Goalie favorite with relief appearance
- Past-date favorites view
- Empty group week aggregate
- Newly-favorited rookie with empty career history

**Foster.3 = 12 tests**:

- L0 EventStream extraction from boxscore (4): score event, trade
  event, milestone event (1000-goal), streak event
- L0 PK + ON CONFLICT round-trip (2): fresh insert, re-insert with
  updated payload
- L0 payload_version per kind (2): score v1 schema, trade v1 schema
- L1 mock fetch + store (2): boxscore JSON → SQLite tx → manifest
  commits last
- L2 CLI (2): `fetch boxscore --date YYYY-MM-DD` writes events,
  `--for-favorites` filters

**Foster.2 + .3 total = 36 tests + 6 personas**.

## Files added

```
icelines-core/src/favorites.rs               (~250 lines)
icelines-core/src/event_stream.rs            (~150 lines)
icelines-cli/src/commands/favorites.rs       (~200 lines)
icelines-cli/src/tui/screens/favorites.rs    (~250 lines)
icelines-web/src/handlers/favorites.rs       (in lib.rs, ~200 lines)
icelines-web/templates/favorites.html        (~150 lines)
design/specs/event-stream-payloads.md        (~200 lines, new spec for v1 payloads)
icelines-cli/tests/foster_favorites.rs       (~400 lines)
```

## Open items

1. **Streak detection** — when does a "streak" event get inserted?
   At streak-start? At each game continuing it? At streak-end?
   Recommend insert at streak-end + a "live streak" derived view
   that scans recent events. Defer detail to Foster.3 implementation.
2. **Milestone detection** — same question (1000G insert at the
   game when it happens, derived from boxscore parse). Defer to
   Foster.3.
3. **Group selection** — spec assumes one "Favorites" group. What if
   user has multiple groups (Favorites + Watchlist + Daily Picks)?
   v1 reads the group named exactly "Favorites". Multi-group selector
   is Foster.6 polish.
