# Pulse 01: Fantasy API Local-State Read Boundary

## Scope

`WP-006` selected fantasy read/local-state safety slice for fantasy JSON GET
routes.

The observed gaps were:

- selected fantasy read routes opened `FantasyDb::open()` on a cold home, which
  creates `~/.icelines/icelines.db` even when the request only reads fantasy
  state;
- `/api/v1/fantasy/daily` and `/api/v1/fantasy/matchup` opened the writable data
  store when cached manifest state was absent, creating local data directories
  while rendering missing-cache state.

## Change

- Fantasy read helpers now open an existing local `icelines.db` only; missing DB
  state returns an explicit error instead of creating user state.
- `/api/v1/fantasy/daily` now renders missing boxscore source-state from the
  existing FantasyDb snapshot without opening the writable data store when the
  manifest directory is absent.
- `/api/v1/fantasy/matchup` now renders missing schedule/boxscore source-state
  from the existing FantasyDb snapshot and matchup rows without opening the
  writable data store when the manifest directory is absent.
- Route tests now assert selected fantasy missing-DB and missing-cache GET paths
  do not create `~/.icelines` or `~/.icelines/data`.

## Evidence

| Level | Evidence | Result |
|---|---|---|
| L0 | `cargo test -p icelines-web --test l1_router fantasy -- --nocapture` | passed 2026-05-31 |
| L1 | `cargo fmt --check` | passed 2026-05-31 |
| L1 | `cargo clippy -p icelines-web --test l1_router --no-deps -- -D warnings` | passed 2026-05-31 |

## Decision

`closed_with_risk` for this selected fantasy API read boundary.

The pulse proves selected fantasy JSON GET render paths do not create missing
local SQLite or data-cache state. It does not close the full `VAL-007`
poach/import/simulation read transcript, broader dashboard mutation-deferral
inspection, or all fantasy/local-state preservation evidence.
