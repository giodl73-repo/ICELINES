# WP-009 Pulse 12 - Postgame review cache surface

Date: 2026-06-01

## Scope

Add the first postgame review Web surface for the Major Analytics Cache without
promoting blame assignment, causal win/loss explanation, automatic correction, or
a finished postgame workflow. The slice mounts active-context routes:

- `/postgame/review`
- `/api/v1/postgame/review`

The routes derive the default cache key from Web active config as
`postgame_review:<season>:<season_type>` and default the selected postgame metric
set to `expected_goals_share`. Operators can still pass `cache_key` or `metrics`
for explicit cache inspection, but the normal postgame review route no longer
requires the generic named-cache report query contract.

## Changes

- Added postgame review HTML and JSON handlers that reuse `AnalyticsCacheStore`,
  `analytics_cache_consumer_envelope`, and `AnalyticsCacheConsumerView`.
- Passed `AnalyticsCacheConsumerKind::PostgameReviewReport` through the Web
  consumer path so ready JSON and HTML render the postgame-specific ViewModel
  title and consumer identity.
- Preserved explicit unavailable behavior for missing active postgame-review
  cache records, including no cache-directory creation on read.
- Mounted `/postgame/review` and `/api/v1/postgame/review`.
- Added focused L2 route evidence for active-cache defaults, missing-cache
  unavailable state, ready HTML rendering, ready JSON rendering, non-claim copy,
  and route inventory/surface-parity coverage.
- Updated WP-009 VTRACE evidence rows and surface parity to distinguish this
  postgame-review route from the generic named-cache report, coach route,
  opponent-scout route, player evidence-card route, line-combination route,
  goalie-readiness route, and practice-focus route.

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

- This is the first postgame-review cache-backed route, not a finished postgame
  report suite, loss-cause attribution engine, automatic correction planner, or
  autonomous coaching workflow.
- Only `expected_goals_share` is the default metric for the active postgame
  route; broader postgame review metric families remain future product work.
- Agent surfaces still require their own copy review and evidence before
  cache-backed claims.
