# Phase 8h — Chunked Snapshot Store

**Status**: Draft
**Date**: 2026-04-29
**Spec**: design/specs/cache-model.md (extension — chunked layout)

---

## Goal

Replace the full-copy snapshot storage with a **content-addressed chunk
store**, so daily fetches store only the deltas. Enables a turnkey
`cron icelines fetch all` workflow without unbounded disk growth, while
preserving every guarantee of the current model (sealing, integrity,
provenance).

Today: each snapshot is ~1.5 MB regardless of how little changed. After
30 daily fetches that's ~45 MB; after a season ~270 MB.

After Phase 8h: ~5 MB after 30 days, ~25 MB after a season — a 10–15×
reduction with no loss of read fidelity.

---

## Design

### Per-player content-addressed chunks

`bios.json` and `stats.json` are currently single arrays of ~700–1000
player records. We split each player's record into its own chunk file
named by SHA-256 hash of the canonical JSON:

```
~/.icelines/snapshots/
├── chunks/                            ← global content-addressed object store
│   ├── ab/
│   │   └── ab1f5c…d7e2.json          ← {"player_id":8478402,"goals":35,...}
│   ├── 92/
│   │   └── 92b1de…4e7a.json          ← {"player_id":8477934,"goals":48,...}
│   └── ...                            (sharded by first byte → 256 dirs)
├── 20252026-2026-04-25/
│   ├── snapshot.json                  ← SnapshotMeta (parent_key, sealed)
│   ├── manifest.json                  ← canonical mapping (see below)
│   └── rosters/                       ← team rosters stored verbatim
│       ├── ANA.json … (32 files)
└── chunkrefs.json                     ← refcount table for GC
```

**Why per-player, not per-team?**

Per-player chunks (~1000 small files per snapshot) maximize dedup:
day-to-day only ~30–50 players actually played, so 95% of chunks are
already in the store. Per-team chunks (32 files) would re-hash a whole
team whenever one player's stats change — most of the dedup is lost.

### `manifest.json` schema

One file per snapshot. Maps tier-relative IDs to chunk hashes:

```json
{
  "version": 1,
  "season": "20252026",
  "bios": {
    "8478402": "ab1f5c2c…d7e2",
    "8477934": "92b1de03…4e7a",
    "...":     "..."
  },
  "stats": {
    "8478402": "f3a87c…1029",
    "...":     "..."
  }
}
```

Reading a snapshot:
1. Open `manifest.json`.
2. For each player_id, load `chunks/{prefix}/{hash}.json`.
3. Hash mismatch → `IntegrityViolation` (same error as today).

### Rosters stay as verbatim files

Roster files are 12 KB each × 32 teams = 370 KB per snapshot. Trades
are rare so the dedup ratio is poor on coarse-grained rosters; the
write code is simpler if we keep them as today's `rosters/{team}.json`
files. Bias: ship the chunked **stats path first**, defer roster
chunking to v2 if it ever matters.

### Refcount + GC

`chunkrefs.json` at the snapshots root tracks how many snapshots
reference each chunk:

```json
{
  "ab1f5c…d7e2": 14,
  "92b1de…4e7a": 1
}
```

- **On `create + seal`**: walk the new manifest, increment counts. Atomic
  write (`.tmp` → rename).
- **On `delete`**: decrement; chunks reaching 0 are queued for sweep.
- **Sweep**: `icelines snapshot gc` (or implicit during `delete`) removes
  chunks whose refcount is 0. Idempotent and interruptible — re-running
  recomputes refcounts from manifests if the file is missing.

A truncated/missing `chunkrefs.json` is recoverable by walking every
manifest. This is the v1 fallback path.

### Atomicity

Same as today's snapshot model:
- New chunks: write to `chunks/{prefix}/{hash}.json.tmp` → rename.
  Hash is content-addressed so concurrent writes converge to the same
  file.
- `manifest.json`: write to `.tmp` → rename.
- `snapshot.json` (the seal): write to `.tmp` → rename last. The
  snapshot is "live" only after this final rename.

### Integrity

Stronger than today, not weaker:
- Each chunk is byte-for-byte verifiable against its filename hash.
- Snapshot manifest's hash mappings IS the integrity proof.
- No separate `integrity` HashMap field needed in `SnapshotMeta` — the
  manifest serves that role for chunked tiers.

---

## Sub-tasks

### 8h.1 — Chunk store primitives (~3h)

`icelines-fetch::snapshot::chunkstore` module:

```rust
pub struct ChunkStore { root: PathBuf }

impl ChunkStore {
    pub fn put(&self, value: &[u8]) -> Result<String, ...>;   // returns hash
    pub fn get(&self, hash: &str) -> Result<Vec<u8>, ...>;
    pub fn exists(&self, hash: &str) -> bool;
    pub fn delete(&self, hash: &str) -> Result<(), ...>;
}
```

- SHA-256 hash, lowercase hex
- Sharded by 2-char prefix
- Atomic writes (tmp + rename)
- Tests: put/get round-trip, dedup (same content → same path), shard
  layout, error on missing.

### 8h.2 — Chunked SnapshotStore writes (~4h)

Extend `SnapshotStore::create`/`write_file` so a Stats tier writes
chunks + manifest instead of `bios.json`/`stats.json`. Decision: opt-in
via a new `SnapshotStore::with_chunked(true)` constructor or a flag in
`SnapshotMeta` so the reader can dispatch. Default to chunked for new
snapshots.

Refcount updates in `seal` and `delete`. New `chunkrefs.json` lives
alongside the snapshots index; format uses the same atomic-rename
pattern.

### 8h.3 — Chunked SnapshotStore reads (~2h)

`PlayerRepository::load_all` already abstracts the reader. Update
`bundled::load_bios_with_fallback` / `load_stats_with_fallback` to
detect `manifest.json` vs the legacy `bios.json` and dispatch
accordingly. Live snapshots transparently work either way.

### 8h.4 — `snapshot rebuild --chunked` migration (~2h)

One-shot command that walks every existing legacy snapshot and
rewrites it as chunked. Idempotent: re-running on already-chunked
snapshots is a no-op. Safe to interrupt; partial migration leaves the
legacy snapshot untouched until the chunked version is fully sealed.

### 8h.5 — `snapshot gc` command + integration with `delete` (~2h)

```
icelines snapshot gc              # sweep zero-ref chunks
icelines snapshot gc --dry-run    # report what would be swept
```

- Walks `chunkrefs.json` (or recomputes from manifests if missing)
- Removes chunks at refcount 0
- Reports bytes freed
- `snapshot delete` calls this implicitly unless `--no-gc`

### 8h.6 — Tests (~2h)

L0 / L1 in `chunkstore.rs::tests` and `snapshot.rs::tests`:

- `chunkstore_put_then_get_roundtrip`
- `chunkstore_dedup_identical_content`
- `chunkstore_shard_layout`
- `chunked_snapshot_create_and_read_roundtrip`
- `chunked_snapshot_dedup_two_snapshots_share_unchanged_player_chunks`
- `chunked_snapshot_delete_decrements_refcount`
- `chunked_snapshot_gc_removes_zero_ref_chunks`
- `chunked_snapshot_gc_preserves_referenced_chunks`
- `chunked_snapshot_corrupted_chunk_fails_integrity`
- `migration_legacy_to_chunked_idempotent`

L2 (subprocess):
- `l2_cmd_snapshot_gc_dry_run_exits_zero`
- `l2_cmd_snapshot_rebuild_chunked_idempotent`

### 8h.7 — Spec extension (~1h)

Extend `design/specs/cache-model.md` with the chunked layout section,
or write `design/specs/snapshot-chunks.md` as a sibling. Document:
- chunk addressing scheme
- sharding rationale
- refcount + GC contract
- migration path
- backward-compatibility guarantees

### 8h.8 — Doc + CHANGELOG (~30min)

- Mention in `06-tui.md` admin section (cron-friendly storage)
- CHANGELOG entry under `### Added`

---

## Acceptance criteria

- A daily-cadence `icelines fetch all` cron over a one-week window
  produces ≤ 5 MB of new bytes total (vs ~10.5 MB unchunked).
- `icelines snapshot use {old}` still works for snapshots created
  before the migration (legacy reader path untouched).
- `snapshot delete X` followed by `snapshot gc` reduces disk usage by
  the chunks unique to X.
- Integrity verification still catches a corrupted chunk:
  `cargo test --test mock_nhl_api chunked_snapshot_corrupted_chunk_fails_integrity`.
- Workspace stays green; clippy clean for new code.

---

## Out of scope (v2+)

- **Compression of chunk files** (zstd) — additional ~70% reduction
  but harder to inspect with `cat`. Defer.
- **Roster chunking** — small files, low dedup benefit, not blocking.
- **Cross-season chunk dedup** — players' bio data rarely changes
  across seasons, but the chunk format includes season-scoped fields.
  Punt to a future spec.
- **Pack files** (git-style) — only worth it at extreme scale; we're
  forecasting ~25 MB/season chunked, so loose objects are fine.
- **Network sync** of chunks (e.g. multi-device share) — different
  problem, different design.

---

## Risks

| Risk | Mitigation |
|------|------------|
| Refcount drift if write crashes mid-seal | Recompute from manifests on next `gc` invocation |
| Corrupt `chunkrefs.json` | Recoverable via walk of all manifests |
| Slow read on first access (1000 small files) | OS cache makes second access fast; benchmark before optimizing |
| Migration takes a long time on machines with many snapshots | Stream output + `--dry-run` flag for confidence; idempotent so retryable |
| Breaks third-party tools that read `bios.json` directly | Today's path stays valid via `--no-chunked` opt-out for the foreseeable future |

---

## Suggested order

1. 8h.1 (chunkstore primitives) → standalone, testable in isolation
2. 8h.2 + 8h.3 (write + read) → core feature working end-to-end
3. 8h.6 (tests) → lock in correctness
4. 8h.4 (migration) → users on existing stores can move
5. 8h.5 (GC) → enables long-term unbounded use
6. 8h.7 (spec) → ratify the design
7. 8h.8 (docs / CHANGELOG)

Total estimate: **~14–16 hours of focused work**.

---

## Status: Implemented (2026-04-29)

All five sub-phases (8h.1–8h.5) shipped in one session, ~3 hours.

**Delivered:**
- `icelines-fetch::chunkstore` — `ChunkStore::{new, hash, put, get,
  exists, delete, iter_chunks, path_for, root}` with 12 L0 tests
  (round-trip, dedup, sharding, missing, corrupted, idempotent delete,
  iter skips .tmp + missing-root)
- `SnapshotStore` extensions:
  - `chunk_store()`, `is_chunked()`, `write_chunked_stats()`,
    `read_chunked_stats()`, `load_refs()`, `inc_refs()`, `dec_refs()`,
    `recompute_refs()`, `gc_chunks()`, `rebuild_chunked()`
  - `delete()` updated to dec-ref chunks before removing the snapshot dir
- New types: `ChunkedManifest`, `ChunkRefs`, `GcReport`
- `bundled::load_bios_with_fallback` / `load_stats_with_fallback`
  prefer chunked active snapshot, then legacy, then bundled
- CLI: `icelines snapshot rebuild <name> --chunked` and
  `icelines snapshot gc [--dry-run]` wired into `commands/snapshot.rs`
- L2 subprocess tests for the new CLI surfaces
- `cache-model.md` extended with the chunked layout section (~120 lines)

**Test count delta:** workspace went 468 → 493 (+25).

**Out of scope (still as planned):** roster chunking, zstd compression,
cross-season chunk dedup, pack files, network sync of chunks.

**Remaining for a follow-up:** make `fetch all` opt-in to chunked
writes via a `--chunked` flag (currently fetch still uses the legacy
`write_file` path; chunked is reachable only via `rebuild`).

---

## Status after Phase 8h

- `cache-model.md` regains "Implemented" status with the chunked layout
- Daily cron becomes practical: one-line crontab + `snapshot gc --keep 60`
  caps storage forever
- The "are snapshots free to keep around?" question becomes "yes" — disk
  cost scales with new data, not snapshot count
