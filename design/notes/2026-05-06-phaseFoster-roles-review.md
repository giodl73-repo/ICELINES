# Phase Foster — Two-round roles-review notes

**Date**: 2026-05-06
**Round 1**: TAPE / FORGE / GLASS / WIRE on the original
`foster-favorites-time.md` spec
**Round 2**: SCOUT / EDGE / BENCH / PACE after the capability-matrix
+ shifts/transactions opt-in decisions were locked in

19 blocking findings total; all addressed in the four-doc revision.

## Blocking-finding traceability

| # | Finding | Reviewer | Resolution |
|---|---|---|---|
| 1 | Manifest schema versioning | TAPE B1, WIRE H2 | Sharded manifest, `version.json` with `schema_version` + `min_reader_version` (foster-data-architecture §Manifest) |
| 2 | Migration table SnapshotTier→DataKind | TAPE B1 | Translation table in foster-data-architecture §Snapshot read-shim |
| 3 | Snapshot dir as immutable read-only input | FORGE B1 | Locked in foster-data-architecture §Snapshot read-shim — manifest rebuilt on every open |
| 4 | MemberKind ⊕ EntityRef divergence | FORGE B2 | Migration 006 collapses kind into entity_ref (foster-data-architecture §Migration 006) |
| 5 | Timeframe × `--filter` ambiguity | EDGE B1 | Namespaced grammar `g.week` (foster-time-and-timeframes §Timeframe × filter grammar) |
| 6 | `query career --week` rejection | EDGE B2 | Reject with clear error; L2 test required |
| 7 | `shifts` capability misnamed | SCOUT 6 | DEFERRED to a future phase; matrix reserves the slot but enforces `off`-only |
| 8 | Goalie projection path | SCOUT B1 | Separate `GoalieNightLine` schema (foster-favorites-dashboard §Two distinct night-line schemas) |
| 9 | Hits/blocks default 0 on in-progress | SCOUT B2 | Gate on `game_state ∈ {OFF, FINAL}` (foster-favorites-dashboard §Hits/blocks gating) |
| 10 | `/api/v1/favorites` heterogeneous data | WIRE B1 | Documented break in convention (foster-favorites-dashboard §JSON envelope) |
| 11 | EventStream PK includes `payload` | TAPE H3, FORGE M3 | `event_id` replaces `payload` in PK (foster-favorites-dashboard §EventStream) |
| 12 | Foster.0 test budget halved | BENCH B1 | 35 tests (foster-data-architecture §Test plan) |
| 13 | No capability matrix coverage | BENCH B2 | 24 tests in `foster_capability_matrix.rs` |
| 14 | L3 golden tests break under auto-refresh | BENCH B3 | `MockClock` injection + `ICELINES_TEST_MODE=1` |
| 15 | Eager refresh blocks alt-screen | PACE B1 | Non-blocking via `tokio::spawn` + one-shot channel |
| 16 | Manifest O(n) per call | PACE B2 | Sharded by kind, HashMap-indexed in OnceLock |
| 17 | `d` keybind globally taken | GLASS B1 | `Shift+D` for date picker (mirror Shift+P/Shift+M) |
| 18 | `t` collides on three tabs | GLASS B2 | `v` for timeframe cycle |
| 19 | Bundle reduction stance | TAPE B2 | RESOLVED by user — keep 38 seasons, status quo |

## High-priority items also addressed

- Atomic-write invariant + advisory file lock (TAPE H1)
- Lazy-fetch surfacing (stderr banner) + `live_feeds_enabled()` test gating (TAPE H2)
- EventStream + manifest writes transactional (TAPE H4)
- Data prune retention (TAPE M1) — spec'd, deferred to F.3 implementation
- `FetchSource` `#[non_exhaustive]` (FORGE H2)
- `Ttl` enum instead of `Option<Duration>` (FORGE M1)
- `trait DatasetStore` retrofit (FORGE M2)
- F.0.5 split into 5a/5b for bisect (FORGE L2) — N/A since bundle stays at 38
- `f` reserved for text-input — Favorites tab is `Shift+F`, admin → `Shift+A` (GLASS H3)
- Tab insertion order — Favorites between Goalies and Scores (GLASS H4)
- First-run setup as alt-screen modal (GLASS M5)
- Empty-state instructional card (GLASS M6)
- Loading affordance for lazy-fetch (GLASS M7)
- Active-timeframe in status bar (GLASS L8)
- `payload_version` per event_kind (WIRE M4)
- URL convention `?date=` + `?range=` (WIRE M5)
- 409 Conflict for "season not installed" (WIRE M6)
- Mid-day trade attribution rule (SCOUT H3)
- DNP/scratched/absent classification (SCOUT H4)
- Goalie pull rule (SCOUT H5)
- Career history augment for newly-favorited (SCOUT M7)
- `games_played` excludes DNPs (SCOUT M8)
- EntityRef as filter atom (EDGE H2)
- Capability mode bleed-through (EDGE H3) — allow + warn
- Time-travel test fixtures (BENCH H1)
- Stale-cache + banner tests (BENCH H2)
- 30 personas, not 5 (BENCH H3)
- Migration round-trip tests (BENCH M1)
- EventStream coverage 12, not 6 (BENCH M2)

## Reviewer-by-reviewer summaries

### TAPE (data pipeline integrity)
Most concerned with operational correctness — concurrent writes,
partial-fetch atomicity, manifest schema migration. Surfaced the
bundle reduction stance question that the user resolved.

### FORGE (Rust code quality)
Focused on type design — closed enums, Option overloading, parallel-
store proliferation, layering (DataStore vs StatsRepository).
Recommended stringly-typed EntityRef (also WIRE).

### GLASS (TUI UX)
Killed the keybind plan — `d`/`t`/`f` all taken or conflict-prone.
Forced the `Shift+*` modifier pattern across Favorites tab + date
picker + timeframe cycle.

### WIRE (API contracts)
Heterogeneous data envelope explicit break for /api/v1/favorites.
Manifest versioning floor. Stringly-typed entity refs in URL params.
409 vs 404 for unbundled seasons.

### SCOUT (correctness)
Found the `shift_profile.rs`-is-misnamed bombshell. Goalie schema
absent. Mid-day trade attribution undefined. DNP classification
missing.

### EDGE (query/filter logic)
Timeframe × filter ambiguity unresolved — would have shipped wrong
filter behavior on `--week`. Forced the namespaced grammar.

### BENCH (test coverage)
Budget was 3× too small. Capability matrix had zero tests planned.
L3 goldens would break under auto-refresh.

### PACE (perf)
"~3 sec parallel HTTP" claim was false against the existing sequential
retry policy — would block alt-screen 10-15 sec typical, 2.5 min
worst case. Forced background refresh.
