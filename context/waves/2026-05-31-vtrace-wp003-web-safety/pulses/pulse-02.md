# Pulse 02: Favorites Cache Read Boundary

## Scope

`WP-003` selected Web/browser safety slice for `GET /favorites`.

The observed gap was that rendering `/favorites` could lazy-fetch live NHL
boxscores and persist them through the local data manifest/cache. That made a
browser GET request a hidden network and local-state mutation path.

## Change

- Removed live `NhlApiClient::production()` schedule/boxscore calls from the
  favorites render path.
- Replaced lazy fetch/persist behavior with a cache-only read of existing
  `manifest/boxscores.json` entries for today's date.
- Kept POST-backed favorite add/remove and admin cache warming as the explicit
  mutation/write surfaces.
- Added route evidence that an empty data root remains free of `manifest` and
  `boxscores` state after `GET /favorites`.

## Evidence

| Level | Evidence | Result |
|---|---|---|
| L0 | `cargo test -p icelines-web --test l1_router l1_favorites_get_does_not_create_data_cache_when_missing -- --nocapture` | passed 2026-05-31 |
| L0 | `cargo test -p icelines-web --test l1_router favorites -- --nocapture` | passed 2026-05-31 |
| L1 | `cargo fmt --check` | passed 2026-05-31 |
| L1 | `cargo clippy -p icelines-web --test l1_router --no-deps -- -D warnings` | passed 2026-05-31 |

## Decision

`closed_with_risk` for this selected route boundary.

The pulse proves the `/favorites` GET render path is cache-read-only for the
observed manifest/boxscore mutation gap. It does not close full browser launch,
no-JS, viewport, host/bind, URL-before-open, JSON-twin, or recovery inspection
for `VAL-003`.
