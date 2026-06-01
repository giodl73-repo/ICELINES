# Pulse 04: Scoring Cache Read Boundary

## Scope

`WP-003` selected Web/browser safety slice for Rocket Richard scoring and
outlook GET routes.

The observed gap was that scoring report pages and JSON twins opened
`DataStore` while rendering. On a cold or missing-cache home, that call creates
the local data and manifest directories, so a browser GET could mutate local
cache state just to display an empty or recovery report.

## Change

- `GET /game/:id/scoring` and `GET /api/v1/game/:id/scoring` now render a
  missing-source scoring report without opening the writable data store when the
  data manifest is absent.
- `GET /team/:abbrev/scoring`, `GET /team/:abbrev/outlook`, and their JSON twins
  use the same missing-cache behavior.
- `GET /player/:id/scoring`, `GET /player/:id/outlook`, and their JSON twins use
  the same missing-cache behavior.
- `GET /tonight/intel` and `GET /api/v1/tonight/intel` render favorites-first
  empty/missing-source intel from local favorites state without creating cache
  directories.
- Existing cached play-by-play reads still use `DataStore` when a manifest
  directory already exists.
- Route tests now assert the selected missing-cache scoring paths do not create
  `~/.icelines/data` state.

## Evidence

| Level | Evidence | Result |
|---|---|---|
| L0 | `cargo test -p icelines-web --test l1_router rocket -- --nocapture` | passed 2026-05-31 |
| L1 | `cargo fmt --check` | passed 2026-05-31 |
| L1 | `cargo clippy -p icelines-web --test l1_router --no-deps -- -D warnings` | passed 2026-05-31 |

## Decision

`closed_with_risk` for this selected route boundary.

The pulse proves selected scoring, outlook, and tonight-intel GET render paths
are cache-read-only when manifest state is absent. It does not close full
browser launch, no-JS, viewport, host/bind, URL-before-open, broader JSON-twin,
or recovery inspection for `VAL-003`.
