# Snapshot Operations — Specification

**Version**: 1.0
**Date**: 2026-04-28
**Status**: Implemented
**Extends**: `cache-model.md` (snapshot model, tiers, integrity)

---

## Purpose

Manage **fetched data snapshots** from the CLI: list what's stored,
inspect details, switch the active snapshot, verify integrity, and
delete old snapshots.

`cache-model.md` defines the snapshot model itself — tiered file
layout, sealing, hashing, parent-chain provenance. This spec covers
the CLI surface that operates on those snapshots.

---

## Snapshot lifecycle (recap)

```
draft  ─── icelines fetch all ─────► sealed
                                       │
                                       ├── active (one at a time)
                                       │
                                       └── inactive
```

A snapshot starts in **draft** status while `icelines fetch` is
writing to it. When the fetch completes successfully, the snapshot is
**sealed** (manifest written, files hashed). At most one sealed
snapshot is **active** — that's the one query commands read.
Inactive snapshots are kept for provenance / rollback until the user
deletes them.

---

## CLI commands

```
icelines snapshot list
icelines snapshot show   <NAME>
icelines snapshot use    <NAME>
icelines snapshot verify [<NAME>]    # default: verify active snapshot
icelines snapshot delete <NAME>
```

### `snapshot list`

```
$ icelines snapshot list
NAME                    TIER     DATE                 SEALED  ACTIVE
─────────────────────── ──────── ─────────────────── ─────── ───────
bios-2026-04-26         Stats    2026-04-26T00:42Z   ✓       ▶
bios-2026-04-19         Stats    2026-04-19T01:00Z   ✓
realtime-2026-04-26     Realtime 2026-04-26T00:51Z   ✓
draft-incomplete        Stats    2026-04-28T14:08Z   …
```

Columns:
- **NAME** — user-chosen or fetch-generated identifier
- **TIER** — `Stats`, `Realtime`, `Rosters`, `Contracts`, `MoneyPuck`,
  `Boxscore`, `Schedule`. See `cache-model.md` §3 for the tier
  taxonomy.
- **DATE** — ISO-8601 UTC of seal time (or draft creation time)
- **SEALED** — `✓` if sealed, `…` if draft
- **ACTIVE** — `▶` for the active snapshot in its tier

Sort order: tier, then date descending.

### `snapshot show <NAME>`

Full detail dump:

```
$ icelines snapshot show bios-2026-04-26
Snapshot:   bios-2026-04-26
Tier:       Stats
Created:    2026-04-26T00:42:11Z
Sealed:     true
Active:     true
Parent:     bios-2026-04-19   (inherited fields auto-merged)

Files (4):
  bios.json        2.1 MB   sha256=a3f5...c12d
  stats.json       1.5 MB   sha256=92b1...4e7a
  manifest.json    3 KB     sha256=...
  provenance.json  1 KB     sha256=...

Integrity: ✓ all 4 files match recorded hashes
```

The **Parent** line is the previous sealed snapshot of the same tier;
fields it provides are inherited unless overridden by this snapshot
(see `cache-model.md` §4 on the provenance chain).

### `snapshot use <NAME>`

Promote a sealed snapshot to **active** within its tier. The previous
active snapshot of that tier becomes inactive but is not deleted.

```
$ icelines snapshot use bios-2026-04-19
Switched active Stats snapshot:
  was: bios-2026-04-26
  now: bios-2026-04-19
```

Errors:
- Snapshot doesn't exist
- Snapshot is in **draft** state (only sealed snapshots can be active)
- Snapshot is already active (no-op with friendly message)

This is mainly used for **rollback** — pinning to a specific date or
reverting after a bad fetch.

### `snapshot verify [<NAME>]`

Re-run integrity check: re-hash every file and compare to recorded
hashes in the manifest.

```
$ icelines snapshot verify bios-2026-04-26
Verifying bios-2026-04-26 (4 files)…
  bios.json        ✓
  stats.json       ✓
  manifest.json    ✓
  provenance.json  ✓
✓ All 4 files verified
```

If a hash mismatches, exit code is non-zero and the offending file is
listed. The snapshot is **not** auto-quarantined — that's a user
decision (typically `snapshot delete` then re-fetch).

Without an argument, verifies the **active snapshot of every tier**.

### `snapshot delete <NAME>`

Removes the snapshot directory + its entries from the manifest.

```
$ icelines snapshot delete bios-2026-04-19
✓ Deleted bios-2026-04-19 (3.6 MB freed)
```

Errors:
- Snapshot doesn't exist
- Snapshot is **active** (must `snapshot use` a different one first)

There is no `--force` in v1; deleting a draft snapshot is allowed
(useful when a fetch crashed mid-way).

---

## Interaction with `icelines fetch`

`icelines fetch all` (and the per-tier variants) creates a new draft
snapshot, writes files, then seals it on success. The freshly-sealed
snapshot is **automatically activated** (becomes the new active one
for its tier).

If a fetch fails mid-stream, the draft is left on disk. The user can
inspect it with `snapshot show` or delete it with `snapshot delete`.
A subsequent `fetch` does not resume — it creates a fresh draft.

---

## Storage layout

```
~/.icelines/snapshots/
├── stats/                   ← one directory per tier
│   ├── bios-2026-04-26/
│   │   ├── bios.json
│   │   ├── stats.json
│   │   ├── manifest.json    ← {tier, name, created_at, sealed, hashes}
│   │   └── provenance.json  ← {parent: bios-2026-04-19, fetched_from: ...}
│   └── bios-2026-04-19/
├── realtime/
├── rosters/
└── manifest.toml            ← {active: {stats: bios-2026-04-26, realtime: ..., ...}}
```

`manifest.toml` at the snapshot store root tracks which snapshot is
active per tier. `snapshot use` rewrites this file atomically (write
to `manifest.toml.tmp`, rename).

---

## Decisions (Open Questions resolved)

1. **Active is per-tier, not global**: Stats and Realtime are
   independent — switching one doesn't switch the other. Reason: a
   user may want fresh stats but stable realtime (or vice versa).

2. **Sealed snapshots are immutable**: Once sealed, files cannot be
   modified. To "edit" a snapshot, fetch a fresh one. The integrity
   guarantee depends on this.

3. **No `snapshot rename`**: Names are user-meaningful and stable;
   renaming would invalidate the parent chain. If a name is bad,
   delete and re-fetch.

4. **Draft snapshots count as "exists"** for `use` rejection
   (cannot activate a draft) but as deletable for `delete`. This is
   asymmetric on purpose: `delete` is for cleanup, `use` is for
   promotion.

5. **`verify` is read-only**: Never repairs, never re-downloads. If
   a hash fails, the user decides what to do.

6. **No `--auto-prune` flag**: Old snapshots are kept by default for
   rollback / forensics. A `snapshot prune --keep N` is backlog.

7. **Active snapshot stored in `manifest.toml`, not as a symlink**:
   Cross-platform (Windows). Symlinks would also break the
   integrity check (hash of symlink target ≠ hash of file).

---

## Test coverage

L0 in `icelines-fetch/src/snapshot.rs::tests`:
- `l0_snapshot_create_and_seal`
- `l0_snapshot_integrity_verified_on_read`
- `l0_snapshot_read_active_requires_seal`
- `l0_snapshot_verify_catches_corruption`
- `l0_snapshot_manifest_atomic_write`
- `l0_snapshot_delete_non_active`

L2 (subprocess) in `system_tests.rs`:
- `l2_cmd_snapshot_list_exits_zero`
- `l2_cmd_snapshot_verify_no_active_exits_gracefully`

---

## Future work (v2+)

| Item | Priority | Why deferred |
|------|----------|--------------|
| `snapshot prune --keep N` | MED | Storage is cheap; not yet a problem |
| `snapshot diff <A> <B>` | MED | Useful for tracking weekly changes |
| `snapshot export <name>` to tar.gz | LOW | Sharing / backup; not blocking |
| `snapshot import <path>` | LOW | Pair with export |
| Garbage-collect orphan parent chains | LOW | Edge case |
| Auto-active newest sealed snapshot at load | DONE | Already happens after fetch |
| Per-tier `--keep N` policies | LOW | One global is enough |
