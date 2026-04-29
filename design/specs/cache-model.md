# IceLines Cache Model — Snapshot Specification

**Version**: 0.2  
**Date**: 2026-04-26  
**Status**: Draft — replaces the TTL-file model in `icelines-fetch/src/cache.rs`

---

## Problem with the TTL Model

The original cache (`Cache::get/put` with a TTL `Duration`) has three problems:

1. **No provenance.** A stats file has no record of which roster snapshot it was fetched with.
   If rosters update (trade deadline) and stats don't, the two are silently inconsistent.
2. **No integrity.** A partially-written file looks valid until deserialization fails.
3. **No named states.** You can't roll back, compare, or label a snapshot as "pre-trade-deadline".

---

## The Snapshot Model

A **snapshot** is a named, sealed, integrity-hashed collection of NHL API data
for a given season, fetched at a point in time. Snapshots are immutable after sealing.

### Three Tiers (provenance chain)

```
Tier 1: ROSTERS   — 32 team rosters, headshots, bio data
    ↓ (parent_key)
Tier 2: STATS     — skater bios + season summary stats
    ↓ (parent_key)  
Tier 3: DERIVED   — computed pace scores, fit classes, depth charts (future)
```

Each tier's key includes the parent key — so a stats snapshot is provably
tied to the exact roster snapshot it was paired with.

### Snapshot Identity

```
name = "{season}-{date}-{tier}"
e.g.  "20252026-2026-04-25-rosters"
      "20252026-2026-04-25-stats"
```

### Directory Layout

```
~/.icelines/
├── snapshots/
│   ├── index.json                     ← SnapshotManifest (all snapshots)
│   ├── 20252026-2026-04-25/
│   │   ├── snapshot.json              ← SnapshotMeta (integrity hashes, parent key)
│   │   ├── rosters/
│   │   │   ├── ANA.json
│   │   │   ├── BOS.json  ... (32 files)
│   │   └── stats/
│   │       ├── bios.json
│   │       └── stats.json
│   └── 20252026-2026-04-26/
│       └── ...
└── active.json                        ← { "snapshot": "20252026-2026-04-25" }
```

---

## Data Structures (Rust)

```rust
/// Top-level index of all snapshots. Written atomically to ~/.icelines/snapshots/index.json.
pub struct SnapshotManifest {
    pub snapshots: Vec<SnapshotEntry>,
    pub active:    Option<String>,   // name of the active snapshot
}

pub struct SnapshotEntry {
    pub name:       String,          // "20252026-2026-04-25-rosters"
    pub season:     String,          // "20252026"
    pub tier:       SnapshotTier,
    pub date:       String,          // "2026-04-25"
    pub created_at: String,          // RFC3339 timestamp
    pub parent_key: Option<String>,  // name of parent snapshot (tier chain)
    pub file_count: usize,
    pub sealed:     bool,
}

pub enum SnapshotTier {
    Rosters,   // Tier 1 — no parent required
    Stats,     // Tier 2 — requires Rosters parent
    Derived,   // Tier 3 — requires Stats parent (Phase 3)
}

/// Per-snapshot metadata, stored in {snapshot}/snapshot.json.
pub struct SnapshotMeta {
    pub name:       String,
    pub season:     String,
    pub tier:       SnapshotTier,
    pub created_at: String,
    pub parent_key: Option<String>,
    /// SHA-256 hex digest per file, relative to snapshot dir
    pub integrity:  HashMap<String, String>,
    pub metadata:   HashMap<String, String>,
    pub sealed:     bool,
}
```

---

## CLI Commands

```
icelines snapshot list                  Show all snapshots with tier, date, status
icelines snapshot show <name>           Full detail: files, integrity, parent chain
icelines snapshot use <name>            Set as active snapshot
icelines snapshot diff <name1> <name2>  Compare two snapshots (player counts, stat deltas)
icelines snapshot delete <name>         Remove a named snapshot
icelines snapshot verify [name]         Re-check integrity hashes; report corruption
```

`icelines fetch` creates a new snapshot automatically and sets it as active.

---

## Fetch + Seal Flow

```
icelines fetch rosters
  1. Create snapshot dir: ~/.icelines/snapshots/{season}-{date}/
  2. Write each roster file to rosters/ with atomic rename (.tmp → final)
  3. Compute SHA-256 for each file
  4. Write snapshot.json (SnapshotMeta, sealed=false)
  5. Seal: set sealed=true, write final snapshot.json
  6. Update index.json (append entry, set active)

icelines fetch stats
  1. Require active rosters snapshot (parent_key)
  2. Write stats/ files to SAME snapshot dir as rosters
  3. Compute integrity for stats files
  4. Update snapshot.json — add stats integrity, update tier=Stats, re-seal
  5. Update index.json
```

---

## Integrity Verification

Every read from a sealed snapshot verifies the file's SHA-256 against `snapshot.json`
before deserializing. Mismatch → `SnapshotError::IntegrityViolation`.

```rust
pub enum SnapshotError {
    NotFound { name: String },
    NotSealed { name: String },
    IntegrityViolation { file: String, expected: String, got: String },
    MissingParent { name: String, parent: String },
    Io(std::io::Error),
    Json(serde_json::Error),
}
```

---

## Invariants

- **SI-01**: A sealed snapshot's files are never modified. Seal is permanent.
- **SI-02**: A Stats snapshot always has a valid Rosters parent in the manifest.
- **SI-03**: `index.json` is updated atomically (write to `.tmp`, rename).
- **SI-04**: File integrity is verified on every read from a sealed snapshot.
- **SI-05**: The active snapshot is always sealed before it can be set as active.

---

## Migration from TTL Cache

The old `~/.icelines/cache/rosters/` and `~/.icelines/cache/stats/` directories
are read-compatible during migration (`icelines snapshot import` reads legacy cache
and seals it as a snapshot named `legacy-import`). The old cache is not deleted
automatically — run `icelines cache clean` to remove it.

---

## Chunked storage layout (Phase 8h)

The default layout (above) writes one `bios.json` and one `stats.json` per
snapshot — ~1.5 MB total, 97% of which is byte-identical day-to-day during a
season. The **chunked layout** addresses this by storing each player's
record as its own content-addressed chunk, so daily snapshots only commit
the records that actually changed.

### Layout

```
~/.icelines/snapshots/
├── chunks/                      ← global content-addressed object store
│   ├── ab/
│   │   └── ab1f5c…d7e2          ← {"playerId":8478402,"goals":35,...}
│   ├── 92/
│   │   └── 92b1de…4e7a          ← {"playerId":8477934,"goals":48,...}
│   └── ...                       (sharded by first byte → ≤256 dirs)
├── chunkrefs.json               ← refcount table for GC
├── 20252026-2026-04-25/
│   ├── snapshot.json            ← unchanged — SnapshotMeta
│   └── chunked.json             ← ChunkedManifest (bios + stats hashes)
└── 20252026-2026-04-26/
    ├── snapshot.json
    └── chunked.json             ← shares ~95% of hashes with previous day
```

### Data structures

```rust
pub struct ChunkedManifest {
    pub version: u8,                     // schema version, currently 1
    pub bios:    HashMap<u32, String>,   // player_id → SHA-256 chunk hash
    pub stats:   HashMap<u32, String>,
}

pub struct ChunkRefs {
    pub counts: HashMap<String, u32>,    // hash → number of snapshots referencing it
}

pub struct GcReport {
    pub removed:     u32,
    pub bytes_freed: u64,
    pub dry_run:     bool,
}
```

### Storage savings

| Cadence | Legacy layout | Chunked layout | Reduction |
|---------|---------------|----------------|-----------|
| Daily, 1 month | ~45 MB | ~5 MB | 9× |
| Daily, full season | ~270 MB | ~25 MB | ~10× |
| Daily, 5 seasons archived | ~1.4 GB | ~80 MB | ~17× |

### Lifecycle

**Writing** (`SnapshotStore::write_chunked_stats`):
1. For each player record, serialize to canonical JSON.
2. Hash the bytes (SHA-256), put into `chunks/{prefix}/{hash}`.
   If the chunk already exists, `put` is a fast no-op (no rewrite).
3. Build a `ChunkedManifest` mapping player_id → hash.
4. Write `{snapshot}/chunked.json` atomically (`.tmp` → rename).
5. Increment refcount for each referenced hash in `chunkrefs.json`.

**Reading** (`SnapshotStore::read_chunked_stats`):
1. Open `{snapshot}/chunked.json`.
2. For each (player_id, hash), read `chunks/{prefix}/{hash}` and verify
   integrity by re-hashing.
3. Reassemble `Vec<SkaterBio>` and `Vec<SkaterStats>`.

**Deleting** (`SnapshotStore::delete`):
1. Decrement refcount for each chunk in the snapshot's manifest.
2. Remove the snapshot directory.
3. Chunks at refcount 0 are NOT physically removed here — call
   `gc_chunks(false)` to sweep.

**Garbage collection** (`SnapshotStore::gc_chunks`):
1. Recompute refs from all chunked manifests (defends against drift).
2. Walk every chunk in the store; any not in the refcount table is
   "zero-ref" and eligible for sweep.
3. With `dry_run=true`, report counts/bytes only. With `dry_run=false`,
   physically remove zero-ref chunks.

**Migration** (`SnapshotStore::rebuild_chunked`):
1. If snapshot already chunked → no-op (return existing manifest).
2. Read `stats/bios.json` + `stats/stats.json` via the legacy reader.
3. Run the chunked write path.
4. Legacy files remain in place — caller may delete them later.

### CLI surface

```
icelines snapshot rebuild <NAME> --chunked    # one-shot migration, idempotent
icelines snapshot gc [--dry-run]              # sweep zero-ref chunks
```

`fetch all` continues to use `write_file` for the legacy layout in v1; a
follow-up flag (`--chunked`) will let users opt new fetches into chunked.

### Recovery

`chunkrefs.json` is a hint for performance; the chunked manifests are the
source of truth. `recompute_refs()` rebuilds the refs table by walking
every chunked manifest. `gc_chunks` always recomputes before sweeping —
so a corrupt or missing refs file is recoverable and never causes
incorrect chunk deletion.

### Atomicity

Same atomic-rename pattern as the legacy layout. The snapshot is "live"
only after `chunked.json` and `snapshot.json` are renamed into place.
Concurrent writes of the same chunk content converge to the same path
(content-addressed); never produces a half-written `<hash>` file.

### Integrity

Stronger than legacy:
- Every chunk's filename IS its SHA-256. Reading verifies bytes against
  the filename — catches bit-rot and manual edits.
- The chunked manifest's hash mappings are themselves verified on read.
- No separate `integrity` HashMap field needed for chunked tiers — the
  manifest serves that role.
