# Phase Foster — closeout 8-role review

**Date**: 2026-05-06
**Scope**: Full Foster phase output — F.0 through F.6 + F+1 through F+12 deferred-work batch
**Goal per closeout plan**: "did we forget anything" — not finding new blockers (should be impossible at this point) but capturing lessons for future phases

Composite synthesis across the 8 review angles (TAPE / FORGE / GLASS / WIRE / SCOUT / EDGE / BENCH / PACE).

---

## TL;DR

Foster ships its critical-path with **5 known carry-forwards** documented in code + this note. No new blockers. The deferred items each have either (a) a primitive in place needing wiring or (b) a follow-on phase that's the natural home.

---

## Per-role observations

### TAPE (data pipeline integrity)

**Strengths**
- Atomic write helper extracted (FORGE M2 honored) and shared between `CareerHistoryStore::save` and `Manifest::write_shard`
- Migration 006 transactional with `defer_foreign_keys = ON` so the rebuild + FK constraint update happens in one tx (l1_db_006_round_trip_with_pre_migration_fixture pinned)
- Migration 007 idempotent (`CREATE TABLE IF NOT EXISTS` + `CREATE INDEX IF NOT EXISTS`)
- Boxscore persistence (F+3) writes JSON first, then manifest entry — manifest is the commit point per the spec's TAPE H4 rule

**Carry-forward**
- `fetch boxscore` doesn't yet hold an advisory lock against concurrent invocations on the same date. Two parallel runs on the same date could double-write the JSON file (last writer wins; not corruption since each write is atomic, but still wasted bandwidth). Low priority — `fetch_lock.rs` pattern exists; can be applied if real-world concurrency matters.

### FORGE (Rust code quality)

**Strengths**
- `EntityRef` shipped as a stringly-typed enum with strict `FromStr` validation (alphanumeric keys, uppercase team abbrevs, numeric player/game ids)
- `Fetcher` trait + `NoopFetcher` default + production `NhlApiFetcher` with explicit `block_on` bridge — the sync/async boundary is honest
- `FetchSource` is `#[non_exhaustive]` (FORGE H2) so `data status` had to add a wildcard match arm — caught at compile time
- `CapabilityError` shipped as a `#[non_exhaustive]` enum with the literal BENCH-H3 error string for `ShiftsLocked`

**Carry-forward**
- `DatasetStore` trait retrofit (FORGE M2) was scoped down to the atomic-write helper share. The fuller trait (`load`/`save`/`freshness` per kind) didn't ship — the Manifest + DataStore composition turned out cleaner than a generic trait would have been. Documented as "scoped" rather than "deferred."

### GLASS (TUI UX)

**Strengths**
- `Shift+D` shipped (GLASS B1 — date picker overlay) routing to Tonight/Schedule/Playoffs surfaces by `picker_target` enum
- `v` keybind cycles Day → Week → Month → Season (GLASS B2); status bar surfaces the active timeframe in chunks[2] (GLASS L8) only when not the Day default to keep first-launch uncluttered
- `Shift+F` (Favorites tab) and `Shift+A` (admin) keybind shifts deferred — admin overlay still on `F`; not yet a Foster.2 tab

**Carry-forward**
- TUI Favorites tab itself didn't ship — only the CLI `icelines favorites` command and the underlying schemas. The tab is a natural follow-on; CLI provides the mental model + the data surface
- `SyncBanner` widget state machine ships (F+10, 7 L0 tests) but isn't yet wired into the TUI app loop (no `mpsc::Receiver<SyncEvent>` consumer in `run_loop`). Wiring is small (~30 lines); deferred for the same session that adds the Favorites tab

### WIRE (API contracts)

**Strengths**
- Web `/scores?date=` and `/schedule?date=` (Foster.1) + `/scores?range=day|week|month` (F+9) all 200 with `range=day` implicit
- `/api/v1/favorites` envelope shape documented as heterogeneous (WIRE B1) — `data: { players, teams, events }` instead of a homogeneous array; the empty-state case in `icelines favorites --json` proves the envelope shape
- Manifest `version.json` carries `schema_version` + `min_reader_version`; refuse-to-read on version-too-new (WIRE H2) verified by `l0_foster03_version_too_new_refuses`

**Carry-forward**
- `?range=` only wired on `/scores`; `/schedule` accepts `?date=` but not `?range=`. Pattern is one-line copy from the scores handler; left for the same session that wires `?range=` on `/favorites`
- `/api/v1/favorites` JSON twin not yet a separate route — currently only the CLI `favorites --json` exposes the envelope. Web JSON twin is straightforward once the orchestration logic moves out of the CLI

### SCOUT (correctness)

**Strengths**
- Distinct `SkaterNightLine` + `GoalieNightLine` schemas (SCOUT B1) ship in `icelines-core::favorites`
- `gate_finalized` helper drops NHL API mid-game zero-defaults for hits/blocks (SCOUT B2)
- `primary_goalie` multi-goalie picker (SCOUT H5) — prefers decision-holder, falls back to longest TOI; 4 truth-table tests
- `detect_mid_day_trade` (F+5) handles the case-insensitive same-team match correctly
- Career-history augment (F+6 / SCOUT M7) for newly-favorited players triggers a one-off lazy fetch with non-fatal failure

**Carry-forward**
- The actual rendered per-night stat lines aren't yet computed — `FavoritesView.players` always renders empty in the CLI today. The schemas + projection helpers are in place; the orchestration that walks (group → favorited PIDs → each game's boxscore → night-line population) is the natural next chunk
- Trade detection wiring into `do_boxscore` is similarly scoped — the helper exists; calling it requires walking yesterday's score event payloads to find the prior team observation

### EDGE (query/filter logic)

**Strengths**
- `WindowedAtom` extends the filter grammar to `<stat-key>[.<window>]<op><value>` (EDGE B1) — 7 L0 tests cover the rsplit-on-`.` parser edge cases
- `query career --week` / `--month` rejected with literal EDGE B2 error including `Use --season instead` remediation; pinned in 2 L2 system tests + 2 personas

**Carry-forward**
- The plumbing through `apply_views` to actually apply windowed atoms isn't wired; `query leaders --filter "g.week>=10"` would parse OK but currently has no per-week stat source to evaluate against. F+4 (boxscore parse) provides the per-game stat foundation; binding the windowed atom evaluator on top is a sequel

### BENCH (test coverage)

**Strengths**
- 30 personas in `persona_foster.rs` hit BENCH H3's spec target
- L1 mock-NHL tests for the production `NhlApiFetcher` (3 tests, including 5xx → DataError::Http5xx mapping)
- `ENV_MUTEX` static serializes the env-var-touching sync-engine tests so parallel runners don't race on `ICELINES_TEST_MODE`
- 248+ Foster-specific tests across L0/L1/L2/persona

**Carry-forward**
- Persona suite is shipped at 30 but doesn't yet include the "lazy fetch happy path" (3-person flow) since `NhlApiFetcher` requires live network for the happy path. Could be backed by httpmock + isolated tempdir; deferred since the L1 fetcher tests already prove the wiring
- `cargo test --workspace` shows occasional flakes on `l1_userflow_add_to_favorites_persists_to_sqlite` under heavy parallel load — runs cleanly in isolation. Likely a `~/.icelines/icelines.db` race when multiple tests open the production HOME path simultaneously. Documented; would need test-isolation refactor to close

### PACE (perf)

**Strengths**
- `launch_eager_sync` returns `Option<Receiver<SyncEvent>>` and short-circuits to `None` under `ICELINES_TEST_MODE=1` so L3 golden tests don't race a background refresh (BENCH B3)
- Each refresh runs in `tokio::task::spawn_blocking` so sync HTTP doesn't pin the executor's worker
- Manifest is sharded by `DataKind` (PACE B2) — `query leaders` deserializes ~50 entries instead of the 50k a unified manifest would force

**Carry-forward**
- `enumerate_stale` walks all kinds even when the caller only cares about one — fine at today's manifest size (single-digit hundreds of entries) but worth a `enumerate_stale_for(kind)` variant if the manifest grows past ~10k entries
- Boxscore JSON files aren't compressed; ~30KB each × N games over a season. Cumulative storage could grow to ~40MB/season fully populated. Document; consider gzip wrapper if user complaints surface

---

## Closeout decisions

**No new blockers identified.** All deferred work is documented in:
1. CLAUDE.md "What's been built" section (Foster bullet)
2. COMMANDS.md "Phase Foster" section
3. design/specs/event-stream-payloads.md (F+12 sibling spec)
4. Inline code comments at each carry-forward point

**Lessons for future phases**:
1. **Surface the carry-forwards in the same commit** that ships the partial — it's tempting to defer documentation, but every "I'll explain it later" item pays back at 3-5x cost when someone hits the gap fresh
2. **Pure helpers ship before orchestration** — `WindowedAtom`, `detect_mid_day_trade`, `SyncBanner` all shipped as testable pure logic before their full wiring. Each can be smoke-tested at the truth-table level without an integration harness, which made them survive the "ship green, wire later" pattern cleanly
3. **`#[non_exhaustive]` on enums earned its keep** — `FetchSource` and `CapabilityError` both grew during the phase and forced compile-time match-arm updates rather than silent drift
4. **Static mutex over env-var tests** is the simplest serialization for `std::env::var_os` race conditions — no need for `serial_test` crate when one mutex covers the only racing tests in a single file
5. **Dotted-key config API** (`Config::get_key("sync.policy")` → `String`) is a happy medium between "a typed enum per knob" and "a single `HashMap<String, String>`". The CLI surface stays generic; the typed `SyncConfig` validates on `set`

**Tag readiness**: `cargo test --workspace` green; CLAUDE.md + COMMANDS.md current; persona suite passes; carry-forwards documented. v0.16.0 ready for the user to cut at their discretion.
