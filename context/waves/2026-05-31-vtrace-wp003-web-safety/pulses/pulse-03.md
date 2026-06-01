# Pulse 03: Streaks Cache Read Boundary

## Scope

`WP-003` selected Web/browser safety slice for streaks GET routes.

The observed gap was that missing-cache streaks pages opened `DataStore`, which
creates the local data and manifest directories. That made a browser GET request
able to mutate local cache state just to render an empty recovery page.

## Change

- `GET /player/:id/streaks` now treats a missing data manifest as an empty cache
  instead of opening the writable data store.
- `GET /team/:abbrev/streaks` and `GET /api/v1/team/:abbrev/streaks` use the
  same missing-cache behavior.
- Existing cached boxscore reads still use `DataStore` when a manifest directory
  already exists.
- Route tests now assert the missing-cache paths do not create
  `~/.icelines/data` state.

## Evidence

| Level | Evidence | Result |
|---|---|---|
| L0 | `cargo test -p icelines-web --test l1_router streaks -- --nocapture` | passed 2026-05-31 |
| L1 | `cargo fmt --check` | passed 2026-05-31 |
| L1 | `cargo clippy -p icelines-web --test l1_router --no-deps -- -D warnings` | passed 2026-05-31 |

## Decision

`closed_with_risk` for this selected route boundary.

The pulse proves selected streaks GET render paths are cache-read-only when
cache state is absent. It does not close full browser launch, no-JS, viewport,
host/bind, URL-before-open, JSON-twin, or recovery inspection for `VAL-003`.
