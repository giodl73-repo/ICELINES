# Phase Foster — implementation plan (SUPERSEDED)

**Spec**: `design/specs/foster-favorites-time.md` (also superseded)
**Version**: 0.1 (draft, superseded 2026-05-06)
**Date**: 2026-05-06
**Status**: SUPERSEDED — kept for history.

Replaced by:
- `2026-05-06-phaseFoster-overview.md` — orchestrator
- `2026-05-06-phaseFoster-data.md` — F.0 + F.4
- `2026-05-06-phaseFoster-favorites.md` — F.2 + F.3
- `2026-05-06-phaseFoster-time.md` — F.1 + F.5

Two rounds of 8-role review (TAPE/FORGE/GLASS/WIRE + SCOUT/EDGE/BENCH/PACE)
surfaced 19 blockers; all addressed in the per-sub-phase plans.

---

## Pre-flight

- [x] NHL API probe — `/v1/score/{date}` and `/v1/schedule/{date}`
      return 200 for arbitrary past dates (verified 2014-10-08).
- [x] Groups+teams committed (`ab8903bf`) — Foster.2's favorites surface
      can build on the kind-discriminated groups table.
- [ ] Spec + plan review (this doc + the spec) by 4 roles before
      Foster.0 starts.

## Sub-phase tracking

Each sub-phase ships behind one commit. Tests per the existing tier
discipline (L0 unit, L1 integration, L2 subprocess). Workspace tests
must stay green at every checkpoint.

### Foster.0 — Data-model groundwork (~3-4 days)

**Goal**: land the four abstractions before any surface touches them.

- F.0.1 — `icelines-core/src/entity.rs` — `EntityRef` enum + serde +
  string round-trip helpers + 6-8 L0 tests.
- F.0.2 — `icelines-core/src/freshness.rs` — `Freshness`, `FetchSource`,
  TTL helpers + 4-5 L0 tests.
- F.0.3 — `icelines-fetch/src/datastore.rs` — `DataStore` skeleton with
  load_bios / load_stats / load_career_history routing through manifest.
  Reads from `~/.icelines/data/` first, falls back to (current) bundled
  data, lazy-fetches when allowed.
- F.0.4 — `~/.icelines/data/manifest.json` shape + read/write + atomic
  save (mirrors Calder.2 pattern).
- F.0.5 — Bundle reduction: `BUNDLED_SEASONS` shrinks to current season
  only. Drop the 33 historical season include_bytes! entries and the
  `data/seasons/<old>/...` files from the workspace's `data/` dir.
  Keep one season for offline-friendly first-run.
- F.0.6 — Existing `BUNDLED_*` callers redirect through DataStore
  (mechanical); historical-season callers learn to fetch via DataStore's
  lazy path or report "season not installed — `icelines setup --season X`".
- F.0.7 — Migration: existing `~/.icelines/snapshots/` get read-shimmed
  by DataStore so users with installed data don't lose it on the upgrade.
  Old `snapshots/<name>/stats/...` paths surface through the new manifest
  with `source: DataInstall` retained.

**Tests** (~20 new):
- L0: EntityRef serde round-trips (5), Freshness comparisons (4),
  Manifest add/remove/list (5).
- L1: DataStore reads bundled current season, falls back to fetch on
  miss, writes manifest entry (3 mock).
- L2: `icelines data status` lists manifest contents (1).

**Acceptance**: `cargo build` green, all existing workspace tests pass,
`target/release/icelines.exe` is ~5 MB, bare `icelines query leaders`
falls back to lazy-fetch (or a friendly "run setup" message) when the
data dir is empty.

### Foster.1 — Time axis on Scores/Schedule/Playoffs (~2 days)

**Goal**: every date-anchored surface accepts a date param.

- F.1.1 — `NhlApiClient::fetch_schedule_for_date(date)` already exists
  in `nhl_api.rs:373`; verify it works for arbitrary past dates.
  Probably already does — confirm and document.
- F.1.2 — CLI: `tonight --date YYYY-MM-DD`, `schedule --start
  YYYY-MM-DD`, `playoffs --season` already takes a season, no change.
- F.1.3 — TUI: `d` keybind on Scores + Schedule + Playoffs tabs opens
  a date picker overlay. Reuses the calendar overlay pattern from `y`
  season picker (LB.0 work).
- F.1.4 — Web: `/scores`, `/schedule`, `/playoffs` accept `?date=` /
  `?start=` query params. Bookmarkable URLs.

**Tests**: +6 — L0 date parsing, L1 date-routed handler, L2 CLI
subprocess for the new `--date` flag.

**Acceptance**: `icelines tonight --date 2014-10-08` prints that
night's 8 games. `/scores?date=2014-10-08` renders the same data in
HTML.

### Foster.2 — Favorites dashboard (~3 days)

**Goal**: one new surface per medium that aggregates everything about
the user's favorites for a given date.

- F.2.1 — `icelines-core/src/favorites.rs` — pure projection from
  groups + DataStore + EventStream. `compute_favorites_view(group,
  date) -> FavoritesView` returns a struct with player rows + team
  rows + recent events.
- F.2.2 — CLI: `icelines favorites [--date D] [--week] [--json]`.
- F.2.3 — TUI: new tab `f` between Stats and Scores; `d` keybind for
  date selector; `w` toggle Day↔Week.
- F.2.4 — Web: `/favorites` HTML + `/api/v1/favorites` JSON twin.
  Date + week query params.

**Tests**: +12 — L0 projection (5), L1 web routes (3), L2 CLI (4).

**Acceptance**: `icelines favorites` shows McDavid + EDM (already in
my Favorites group) with last-night stats. Works for `--date` past
dates and `--week` aggregate.

### Foster.3 — Per-game boxscore fetching for favorited entities (~2 days)

**Goal**: populate `GameLog` so favorites stat lines are accurate even
when the active stats snapshot is days stale.

- F.3.1 — `NhlApiClient::fetch_boxscore` already exists for live
  refresh. Wire it to write into DataStore at
  `data/boxscores/<date>/<game_id>.json` with manifest entry.
- F.3.2 — `icelines fetch boxscore --date YYYY-MM-DD [--for-favorites]`.
  Without `--for-favorites`, fetches all games; with the flag, only
  games involving favorited teams or rosters with favorited players.
- F.3.3 — Insert score events into the new EventStream table.

**Tests**: +6 — L0 event extraction from boxscore (3), L1 mock fetch
+ store (2), L2 CLI (1).

**Acceptance**: Run `fetch boxscore --date 2026-01-15`; `icelines
favorites --date 2026-01-15` renders accurate stat lines.

### Foster.4 — Sync layer (~2 days)

**Goal**: background freshness for favorited entities + manual `fetch
sync` for power users.

- F.4.1 — `icelines fetch sync` walks the favorites list, looks up
  freshness in manifest, refreshes anything stale (TTL-based).
- F.4.2 — TUI: `R`-overlay shows freshness summary; explicit
  `[r]efresh` action triggers `fetch sync` inline.
- F.4.3 — TUI/Web: optional auto-sync on launch (config flag
  `[sync] auto_on_launch = true`, default off).

**Tests**: +5 — L0 staleness check (3), L1 mock sync run (1), L2 CLI (1).

**Acceptance**: `icelines fetch sync --dry-run` enumerates what
would be refreshed; non-dry runs do it.

### Foster.5 — Timeframe views (~2 days)

**Goal**: Day / Week / Month aggregation for Favorites + Scores +
Schedule.

- F.5.1 — `icelines-core/src/timeframe.rs` — `Timeframe { Day, Week,
  Month, Season }` enum + `range(date) -> (start, end)` helpers.
- F.5.2 — Plumb `--week` / `--month` through favorites + scores +
  schedule CLI commands.
- F.5.3 — TUI top-bar selector shows the active timeframe; `t` cycles.
- F.5.4 — Web `?view=week|month` query param on all date-anchored routes.

**Tests**: +6 — L0 timeframe range computation (4), L1 web routes (1),
L2 CLI (1).

**Acceptance**: `icelines favorites --week` shows the last 7 days
aggregated. `/scores?date=2026-01-15&view=week` shows that week's
slate.

### Foster.6 — Setup wizard + docs + persona pass (~1 day)

**Goal**: make the first-run experience smooth + close the docs gap.

- F.6.1 — `icelines setup` interactive wizard; auto-runs from `icelines
  tui` and `icelines query leaders` (etc.) when manifest is empty,
  unless `--no-setup` is passed.
- F.6.2 — `icelines data status` — pretty-print the manifest.
- F.6.3 — COMMANDS.md / README.md / CLAUDE.md refresh (Foster surface).
- F.6.4 — Persona pass: 5 hands-on scenarios covering favorites,
  time-travel, setup-from-scratch, sync, week-aggregate.

**Tests**: +4 — L2 setup wizard subprocess (1), L0 data status format (3).

**Acceptance**: Drop `~/.icelines/`, run `icelines tui` from scratch,
prompted for setup, accept defaults, end up on the Favorites tab
showing the user's existing groups (which carry across since
`icelines.db` lives in the same directory and isn't reset).

---

## Roles to involve

- **TAPE** — data-pipeline integrity, manifest schema, fetch retry
  semantics on first-run setup
- **FORGE** — Rust code quality on EntityRef + Freshness + DataStore
- **GLASS** — TUI date picker + favorites tab UX
- **EDGE** — query/filter scoping (timeframe interaction with existing
  --filter grammar)
- **WIRE** — JSON envelope shape for `/api/v1/favorites`, manifest
  schema versioning
- **SCOUT** — favorites view correctness (right stats per night)
- **BENCH** — test plan adequacy across 6 sub-phases
- **PACE** — manifest read latency, lazy-fetch hot path

Recommend invoking 4 in parallel for the pre-Foster.0 review:
TAPE + FORGE + GLASS + WIRE. SCOUT/EDGE/BENCH/PACE join at the .2 / .3
checkpoints.

---

## Risk register

| Risk | Severity | Mitigation |
|---|---|---|
| Bundle reduction breaks offline-on-day-one experience | High | Keep current season bundled (~1.5 MB); setup wizard runs immediately on first launch |
| Manifest schema migration on existing users | Med | Read-shim from old `snapshots/` paths to manifest entries (F.0.7) |
| EntityRef serialization choice locks us | Med | Spec calls for stringly-typed in JSON, struct internally — round-trip is reversible |
| Setup wizard adds first-run latency | Med | Choice 1 (default) is ~30 MB / 2 min; choice 4 is "skip" for impatient users |
| Per-game boxscore storage bloats `~/.icelines/` | Low | ~30 KB per game × 1300 games/season = ~40 MB/season fully populated |
| TUI date picker collision with existing keybinds | Low | LB picker pattern already covers this; `d` is currently free on Scores tab |

---

## Out of plan (deferred to post-Foster)

- League axis on StatsRepository (career_history works in parallel store)
- Skater/Goalie iterator unification (cosmetic)
- WebSocket / push notifications (explicit IceLines.md non-goal)
- Multi-user / cloud sync (explicit IceLines.md non-goal)
- Pre-2014 historical scores (depends on API probe extension)
