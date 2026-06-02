# WP-009 Pulse 09 - Line-combination explorer cache surface

Date: 2026-06-01

## Scope

Add the first line-combination explorer Web surface for the Major Analytics Cache
without promoting a finished lineup builder, deployment recommendation, or
line-chemistry claim. The slice mounts active-context routes:

- `/lines/explorer`
- `/api/v1/lines/explorer`

The routes derive the default cache key from Web active config as
`line_combination_explorer:<season>:<season_type>` and default the selected line
metric set to `expected_goals_share`. Operators can still pass `cache_key` or
`metrics` for explicit cache inspection, but the normal line explorer route no
longer requires the generic named-cache report query contract.

## Changes

- Added line-combination explorer HTML and JSON handlers that reuse
  `AnalyticsCacheStore`, `analytics_cache_consumer_envelope`, and
  `AnalyticsCacheConsumerView`.
- Passed `AnalyticsCacheConsumerKind::LineCombinationExplorer` through the Web
  consumer path so ready JSON and HTML render the line-specific ViewModel title
  and consumer identity.
- Preserved explicit unavailable behavior for missing active line-combination
  cache records, including no cache-directory creation on read.
- Mounted `/lines/explorer` and `/api/v1/lines/explorer`.
- Added focused L2 route evidence for active-cache defaults, missing-cache
  unavailable state, ready HTML rendering, ready JSON rendering, non-claim copy,
  and route inventory/surface-parity coverage.
- Updated WP-009 VTRACE evidence rows and surface parity to distinguish this
  line-combination route from the generic named-cache report, coach route,
  opponent-scout route, and player evidence-card route.

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

- This is the first line-combination cache-backed route, not a finished lineup
  builder, deployment recommendation, or player-chemistry workflow.
- Only `expected_goals_share` is the default metric for the active line explorer
  route; broader line metric families and line discovery remain future product
  work.
- Goalie readiness, practice, postgame, and agent surfaces still require their
  own copy review and evidence before cache-backed claims.
