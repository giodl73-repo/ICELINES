# IceLines Cache Model — Snapshot Specification

**Version**: 0.2  
**Date**: 2026-04-26  
**Status**: Draft — replaces the TTL-file model in `icelines-fetch/src/cache.rs`  
**Inspired by**: craftworks `design/compiler/assembly/ATOMS.md` — atoms A186, A206, A208

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
