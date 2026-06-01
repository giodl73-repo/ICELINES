# Pulse 05: Admin Data Status Cache Read Boundary

## Scope

`WP-003` selected Web/browser safety slice for the Admin data-status GET routes.

The observed gap was that `GET /admin` and `GET /api/v1/admin/data-status`
opened `DataStore` while rendering an empty data-status view. On a cold or
missing-cache home, that call creates local data and manifest directories even
though the request is read-only.

## Change

- `GET /api/v1/admin/data-status` now parses filters but returns an empty
  `DataStatusView` without opening the writable data store when the manifest
  directory is absent.
- `GET /admin` inherits the same data-status behavior while preserving its
  operational shell, snapshot, runtime config, and POST-backed admin controls.
- Existing manifest-backed data-status reads still use `DataStore` when a
  manifest directory already exists.
- Route tests now assert the selected Admin missing-cache GET paths do not create
  `~/.icelines/data`.

## Evidence

| Level | Evidence | Result |
|---|---|---|
| L0 | `cargo test -p icelines-web --test l1_router admin -- --nocapture` | passed 2026-05-31 |
| L1 | `cargo fmt --check` | passed 2026-05-31 |
| L1 | `cargo clippy -p icelines-web --test l1_router --no-deps -- -D warnings` | passed 2026-05-31 |

## Decision

`closed_with_risk` for this selected route boundary.

The pulse proves selected Admin data-status GET render paths are cache-read-only
when manifest state is absent. It does not close full browser launch, no-JS,
viewport, host/bind, URL-before-open, broader JSON-twin, or recovery inspection
for `VAL-003`.
