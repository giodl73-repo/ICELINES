# WP-009 Pulse 07 - Opponent scout cache surface

Date: 2026-06-01

## Scope

Add the first opponent-scout Web surface for the Major Analytics Cache without
promoting a full scouting suite. The slice mounts active-context routes:

- `/scout/opponent`
- `/api/v1/scout/opponent`

The routes derive the default cache key from Web active config as
`opponent_scout:<season>:<season_type>` and default the selected scout metric set
to `expected_goals_share`. Operators can still pass `cache_key` or `metrics` for
explicit cache inspection, but the normal scout route no longer requires the
generic named-cache report query contract.

## Changes

- Added opponent scout HTML and JSON handlers that reuse `AnalyticsCacheStore`,
  `analytics_cache_consumer_envelope`, and `AnalyticsCacheConsumerView`.
- Passed `AnalyticsCacheConsumerKind::OpponentScoutReport` through the Web
  consumer path so ready JSON and HTML render the scout-specific ViewModel title
  and consumer identity.
- Preserved explicit unavailable behavior for missing active opponent-scout cache
  records, including no cache-directory creation on read.
- Mounted `/scout/opponent` and `/api/v1/scout/opponent`.
- Added focused L2 route evidence for active-cache defaults, missing-cache
  unavailable state, ready HTML rendering, ready JSON rendering, non-claim copy,
  and route inventory/surface-parity coverage.
- Updated WP-009 VTRACE evidence rows and surface parity to distinguish this
  opponent-scout route from both the generic named-cache report and coach route.

## Evidence

| Level | Evidence | Result |
| --- | --- | --- |
| L2 | `cargo test -p icelines-web --test l2_analytics_cache_report --quiet` | passed 2026-06-01 |
| L1 | `cargo test -p icelines-web --test ted_lindsay_route_inventory --quiet` | passed 2026-06-01 |
| L1 | `cargo clippy -p icelines-web --lib --tests -- -D warnings` | passed 2026-06-01 |
| L0 | `cargo fmt --check` | passed 2026-06-01 |
| VTRACE | `C:\src\proof\target\debug\proof.exe check C:\src\TRACKER\repos\applied-systems\icelines\docs\vtrace --errors-only` | passed 2026-06-01 |
| Hygiene | `git diff --check` | passed 2026-06-01 |

## Residual risk

- This is the first opponent-scout cache-backed route, not a finished scouting
  workflow or opponent-specific game plan.
- Only `expected_goals_share` is the default metric for the active scout route;
  broader scout metric families and opponent discovery remain future product
  work.
- Player evidence card, line explorer, goalie readiness, practice, postgame, and
  agent surfaces still require their own copy review and evidence before
  cache-backed claims.
