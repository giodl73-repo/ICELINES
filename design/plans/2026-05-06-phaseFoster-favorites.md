# Phase Foster — Favorites + Boxscores plan (Foster.2 + Foster.3)

**Spec**: `design/specs/foster-favorites-dashboard.md`
**Test budget**: 18 + 6 personas (F.2) + 12 (F.3) = **36 tests + 6 personas**

---

## Foster.2 — Favorites dashboard

### F.2.1 — Core projection (icelines-core/src/favorites.rs)

- `FavoritesView { date, range, players, teams, events, aggregate }`
- `PlayerNightRow::{Skater | Goalie | DidNotPlay}` enum
- `SkaterNightLine` + `GoalieNightLine` distinct schemas (SCOUT B1)
- `DnpReason::{Scratched | InjuredReserve | TeamBye | Recalled | DataPending}`
- `compute_favorites_view(group: &Group, date: NaiveDate, range: Timeframe, store: &DataStore, db: &GroupDb) -> FavoritesView`
- Mid-day trade attribution: source-of-truth = boxscore (SCOUT H3)
- Goalie-pull rule: prefer non-None decision, else longest TOI; surface both if both > 5 min (SCOUT H5)
- Hits/blocks gating on `game_state` (SCOUT B2)
- DNP classification: diff team-roster against boxscore appearances (SCOUT H4)
- `AggregateView::games_played` counts only games with state ∈ {OFF, FINAL}
- **Tests (8 L0)**: SkaterNightLine vs GoalieNightLine routing, hits/blocks gating, DNP classification (scratched / team-bye / data-pending), mid-day trade attribution, goalie-pull decision picking, aggregate excludes DNPs, empty group, week aggregate sums on the fly

### F.2.2 — CLI command (icelines-cli/src/commands/favorites.rs)

- `icelines favorites [--date D] [--range day|week|month] [--filter EXPR] [--json|--csv]`
- `--filter` scope: per-night stat lines (EDGE H1)
- Empty store → "No favorites yet — add via `group add Favorites <player|team>`"
- Newly-favorited player not in career_history store: lazy-fetch (SCOUT M7)
- **Tests (6 L2)**: empty store, populated, --date past, --range week, --json envelope, --filter "p>=2"

### F.2.3 — TUI tab (icelines-cli/src/tui/screens/favorites.rs)

- New tab between Goalies and Scores
- Keybind: `Shift+F` (admin overlay moves from `F` to `Shift+A` — GLASS H3+H4)
- Empty-state instructional card (GLASS M6)
- Loading affordance for lazy-fetch on date change (GLASS M7)
- `Shift+D` opens date picker overlay (shared with Scores/Schedule)
- `v` cycles timeframe (Foster.5)
- **Tests (3 L1 render smokes)**: with players, with teams, empty group

### F.2.4 — Web routes (icelines-web/src/lib.rs::handlers::favorites)

- `/favorites?date=…&range=…` HTML
- `/api/v1/favorites` JSON twin with **heterogeneous `data` object** (WIRE B1):
  ```json
  { "schema_version": 1, "route": "favorites",
    "data": { "players": [...], "teams": [...], "events": [...] },
    "meta": { "date", "range", "group_id", "group_name", "counts", "active_filters" } }
  ```
- 200 with empty group; 200 with populated; 400 for bad date format
- **Tests (4 L1)**: empty/populated/bad-date/envelope-shape

### F.2.5 — Personas (6 scenarios in `persona_foster.rs`)

1. Setup-then-favorite (drop ~/.icelines, run setup wizard, group add, view)
2. Mid-day trade for a favorited player (Marchand BOS→FLA simulation)
3. Goalie favorite with relief appearance (no decision; multi-goalie row)
4. Past-date favorites view (`favorites --date 2014-10-08`)
5. Empty group week aggregate
6. Newly-favorited rookie with empty career history

## Foster.3 — Boxscores + EventStream

### F.3.1 — EventStream table (icelines-core/src/event_stream.rs)

- SQLite table with PK `(date, entity_kind, entity_key, event_kind, event_id)` (TAPE H3 + FORGE M3)
- `payload_version INTEGER` per event_kind (WIRE M4)
- Indexes `events_by_date(date DESC)` + `events_by_entity(entity_kind, entity_key, date DESC)` (PACE M5)
- Insertion via `INSERT … ON CONFLICT DO UPDATE SET payload, created_at`
- **Tests (4 L0 + 2 L0 PK)**: insert, dedup, update-on-conflict, payload_version round-trip

### F.3.2 — Event payload schemas v1

Frozen schemas in `design/specs/event-stream-payloads.md` (new sibling spec):

- `score:GAMEID:final` — full game result
- `score:GAMEID:period:N` — period-end snapshots (optional, for live tracking)
- `trade:DATE:teams_sorted_alpha`
- `signing:DATE:player_id`
- `milestone:player_id:metric:value`
- `streak:entity_ref:start_date`

**Tests (2 L0)**: score v1 schema, trade v1 schema

### F.3.3 — Boxscore fetcher

- `NhlApiClient::fetch_boxscore_with_events(game_id) -> (Boxscore, Vec<Event>)`
- Writes JSON to `data/boxscores/<date>/<game_id>.json` via DataStore
- Inserts events to EventStream **transactionally** (TAPE H4): SQLite tx → fsync JSON → manifest entry as commit point
- **Tests (2 L1 mock)**: boxscore fetch + store, manifest entry visible after commit

### F.3.4 — `icelines fetch boxscore` command

- `--date YYYY-MM-DD [--for-favorites]` flags
- Without `--for-favorites`: fetches all games for the date
- With `--for-favorites`: filters to games involving favorited entities
- **Tests (2 L2)**: writes events to disk; --for-favorites filters

## Files added

```
icelines-core/src/favorites.rs                       ~250 lines
icelines-core/src/event_stream.rs                    ~150 lines
icelines-cli/src/commands/favorites.rs               ~200 lines
icelines-cli/src/commands/fetch.rs (extended)        +60 lines for `fetch boxscore`
icelines-cli/src/tui/screens/favorites.rs            ~250 lines
icelines-web/src/lib.rs (extended)                   ~200 lines for favorites handlers
icelines-web/templates/favorites.html                ~150 lines
design/specs/event-stream-payloads.md                ~200 lines (new sibling spec)
icelines-cli/tests/foster_favorites.rs               ~400 lines
icelines-cli/tests/persona_foster.rs                 ~250 lines (6 scenarios)
```

## Acceptance for Foster.2

- `icelines favorites` shows favorited players + teams for today
- `--date 2014-10-08` shows past-date data after lazy boxscore fetch
- `--range week` aggregates correctly, excludes DNPs from games_played
- `/favorites` HTML + `/api/v1/favorites` JSON twin both render
- TUI tab `Shift+F` works; admin moved to `Shift+A`; empty-state card displays
- 18 tests + 6 personas pass

## Acceptance for Foster.3

- `fetch boxscore --date 2026-01-15 --for-favorites` populates EventStream + boxscore JSON
- Re-running the same command is idempotent (ON CONFLICT updates)
- Multi-game day correctly attributes events per game_id
- 12 F.3 tests pass
