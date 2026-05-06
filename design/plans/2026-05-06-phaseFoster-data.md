# Phase Foster — Data architecture plan (Foster.0 + Foster.4)

**Spec**: `design/specs/foster-data-architecture.md`
**Test budget**: 35 (F.0) + 24 (capability matrix) + 15 (F.4) = **74 tests**

---

## Foster.0 sub-steps

### F.0.1 — `EntityRef` (icelines-core/src/entity.rs)

- New module + `lib.rs` re-export
- `EntityRef` enum (Player / Team / Game)
- `GameId(pub u64)` newtype in `identity.rs`
- `Display` + `FromStr` (regex-validated grammar `^(player|team|game):[A-Za-z0-9]+$`)
- Serde via `#[serde(into = "String", try_from = "String")]`
- `EntityRefError` (thiserror)
- **Tests (8)**: stringly + struct round-trips × 3 variants, hash equality, malformed string, unknown kind, bad key

### F.0.2 — `Freshness` + `Clock` (icelines-core/src/freshness.rs)

- `Freshness { fetched_at, source, ttl }`
- `FetchSource` `#[non_exhaustive]` enum (Bundle/Setup/Live/DataInstall/Manual)
- `Ttl` enum (Static / After(Duration))
- `Clock` trait + `SystemClock` + `MockClock` (test-only)
- `is_stale(now: &dyn Clock) -> bool`
- **Tests (6)**: stale/fresh/never-expire/clock-skew/Static-source-pinned/test-mode-bypass

### F.0.3 — Manifest shards (icelines-fetch/src/manifest.rs)

- Per-kind JSON files under `~/.icelines/data/manifest/<kind>.json`
- `version.json` with `schema_version` + `min_reader_version`
- `ManifestSet { shards: HashMap<DataKind, OnceLock<HashMap<DataKey, Freshness>>> }`
- Atomic writes (`tmp + rename`); `fs2` advisory file lock on `manifest/.lock`
- Schema-version refuse-to-read on `min_reader_version > MAX_SUPPORTED`
- Unknown top-level keys preserved on rewrite
- **Tests (8)**: add/remove/list/atomic-save/concurrent-writer-lock/schema-bump/missing-file/corrupt-JSON

### F.0.4 — `DataStore` (icelines-fetch/src/datastore.rs)

- `DataStore { root, manifest, clock, live_feeds, test_mode }`
- `load_bios(season)` / `load_stats(season, type)` / `load_career_history(pid)` / `load_boxscore(game_id)` / `freshness(...)` / `list_seasons(...)`
- Read priority: manifest → bundle → lazy fetch → `DataError::NotInstalled`
- Stderr banner on lazy fetch ("fetching season 2008-2009 from NHL API…")
- `live_feeds=false` → never lazy-fetch; surface `NotInstalled` instead
- **Tests (8)**: bundled-hit, manifest-hit, lazy-fetch-hit, lazy-fetch-disabled, lazy-fetch-network-fail, lazy-fetch-5xx, lazy-fetch-schema-drift, fallback-order

### F.0.5 — Snapshot read-shim

- DataStore::open() walks bundle + `~/.icelines/data/seasons/` + `~/.icelines/snapshots/<active>/` to rebuild manifest
- SnapshotTier → DataKind translation table (in spec)
- Snapshots dir is **immutable read-only input** — never mutate
- Recovery: delete `~/.icelines/data/manifest/` → next open rebuilds
- **Tests included in F.0.4** (rebuild-on-open is one of the routing tests)

### F.0.6 — Migration 006 (groups: kind → entity_ref)

- ALTER TABLE add `entity_ref TEXT`
- Backfill from `kind` + `player_normalized`
- Rebuild table with new PK `(group_name, entity_ref)`
- `MemberKind::from(&EntityRef)` becomes a derived view
- **Tests (5)**: 006 round-trip with pre-migration fixture, idempotent re-run, mixed kind→entity_ref backfill, FK cascade, partial-rollback

### F.0.7 — Capability matrix

- `[sync.capabilities]` config section in `~/.icelines/config.toml`
- 6 capabilities × 3 modes parsed into a typed struct
- `shifts` enforced as `off`-only with clear error message
- **Tests (24)** in new file `icelines-cli/tests/foster_capability_matrix.rs`:
  - 18 mode-honored: 6 capabilities × 3 modes (each toggling changes data fetched/stored)
  - 6 interaction tests (transactions=fav + boxscores=off, shifts=off blocks fetch_shifts, career_history=fav filters lazy fan-out, sync=off short-circuits Foster.4, banner verbosity, season_transition test-mode short-circuit)

### F.0.8 — Setup wizard (alt-screen modal)

- `icelines setup` command + auto-detection from `tui` and `query *` when manifest empty
- Three-question modal flow (transactions / boxscores / sync policy)
- Choices write `~/.icelines/config.toml`
- TUI version uses centered modal pattern from `render_reports_overlay`
- `--no-setup` skips for headless callers
- L2 test for the dry-run branch

## Foster.4 sub-steps

### F.4.1 — Sync engine (icelines-fetch/src/sync_engine.rs)

- `launch_eager_sync(store) -> mpsc::Receiver<SyncEvent>`
- `tokio::spawn` background task; never blocks caller
- Enumerates stale entries via `Freshness::is_stale`
- Refreshes per-capability scope (favorites-only narrows to favorited entity refs)
- `SyncEvent::Refreshed | Failed | Done`
- **Tests (10)**: spawn lifecycle, drain-channel-on-exit, banner-shown/suppressed/summary/silent/age-formatting, per-capability staleness gating × 3, MockClock + ICELINES_TEST_MODE skip

### F.4.2 — Banner widget (TUI)

- Status-bar slot for "Refreshed N · 2.1 s" or "Refreshing 3/8 …"
- Drains the `mpsc::Receiver` on every render tick
- Hides after 5 seconds idle
- **Tests (3)**: render-during-sync, render-after-done, hidden-after-timeout

### F.4.3 — `icelines fetch sync` command

- Walks the manifest, refreshes anything stale within capability scope
- `--dry-run` enumerates without fetching
- `--force` overrides DataInstall TTL=Static
- Mirrors the existing `fetch all` UX
- **Tests (2)**: dry-run lists stale entries; --force invalidates DataInstall pin

## Files added

```
icelines-core/src/entity.rs                          ~150 lines
icelines-core/src/freshness.rs                       ~100 lines
icelines-fetch/src/manifest.rs                       ~250 lines
icelines-fetch/src/datastore.rs                      ~300 lines
icelines-fetch/src/sync_engine.rs                    ~150 lines
icelines-cli/src/commands/setup.rs                   ~150 lines
icelines-cli/src/commands/data_status.rs             ~80 lines
icelines-cli/src/tui/screens/setup_overlay.rs        ~120 lines
icelines-cli/src/tui/sync_banner.rs                  ~80 lines
icelines-cli/tests/foster_capability_matrix.rs       ~400 lines
icelines-cli/tests/foster_data.rs                    ~300 lines
```

## Acceptance for Foster.0

- `cargo build --release -p icelines-cli` green
- All existing workspace tests still pass
- `target/release/icelines.exe` ~56 MB (bundle unchanged)
- `icelines query leaders` works against:
  - Fresh install (cache primed by bundle)
  - User with existing `~/.icelines/snapshots/` (read-shim populates manifest)
  - User with empty `~/.icelines/data/` and live feeds off (returns clean `NotInstalled` errors)
- 35 F.0 tests + 24 capability-matrix tests all pass

## Acceptance for Foster.4

- `icelines tui` renders alt-screen immediately on launch (no blocking)
- `icelines fetch sync --dry-run` enumerates stale entries
- Banner appears in status bar during background refresh, disappears after done
- L3 golden tests still pass (MockClock + ICELINES_TEST_MODE keeps them deterministic)
- 15 F.4 tests pass
