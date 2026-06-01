# Pulse 02: Fantasy Existing-DB Read-Only Boundary

## Scope

`WP-006` selected fantasy read/local-state safety slice for existing local
FantasyDb-backed Web GET routes.

The observed gap was that pulse 01 prevented missing database creation, but
existing database reads still used the writable FantasyDb open path, which sets
SQLite WAL mode and can create SQLite sidecar files while servicing a GET.

## Change

- FantasyDb now exposes an existing-database read-only open path for read
  surfaces.
- Web fantasy GET reads use the read-only path after confirming the local
  `icelines.db` exists.
- The read-only path uses SQLite immutable URI mode so closed local databases can
  be inspected without changing journal mode, running migrations, or creating
  WAL/SHM sidecar state.
- Route tests now assert selected fantasy gaps GET reads do not create
  `icelines.db-wal` or `icelines.db-shm`.

## Evidence

| Level | Evidence | Result |
|---|---|---|
| L0 | `cargo test -p icelines-web --test l1_router fantasy -- --nocapture` | passed 2026-05-31 |
| L1 | `cargo fmt --check` | passed 2026-05-31 |
| L1 | `cargo clippy -p icelines-web --test l1_router --no-deps -- -D warnings` | passed 2026-05-31 |

## Decision

`closed_with_risk` for this selected existing-FantasyDb read-only boundary.

The pulse proves selected fantasy Web GET reads do not create SQLite sidecar
state when reading an existing closed FantasyDb. It does not close the full
`VAL-007` poach/import/simulation transcript, browser dashboard mutation-deferral
inspection, or active-writer/concurrent-CLI database semantics.
