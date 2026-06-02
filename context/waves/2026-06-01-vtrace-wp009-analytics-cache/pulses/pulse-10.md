# WP-009 Pulse 10 - Goalie readiness/workload cache surface

Date: 2026-06-01

## Scope

Add the first goalie readiness/workload Web surface for the Major Analytics
Cache without promoting injury certainty, deployment advice, or a finished goalie
management workflow. The slice mounts active-context routes:

- `/goalies/readiness`
- `/api/v1/goalies/readiness`

The routes derive the default cache key from Web active config as
`goalie_readiness:<season>:<season_type>` and default the selected goalie metric
set to `expected_goals_share`. Operators can still pass `cache_key` or `metrics`
for explicit cache inspection, but the normal goalie readiness route no longer
requires the generic named-cache report query contract.

## Changes

- Added goalie readiness/workload HTML and JSON handlers that reuse
  `AnalyticsCacheStore`, `analytics_cache_consumer_envelope`, and
  `AnalyticsCacheConsumerView`.
- Passed `AnalyticsCacheConsumerKind::GoalieReadiness` through the Web consumer
  path so ready JSON and HTML render the goalie-specific ViewModel title and
  consumer identity.
- Preserved explicit unavailable behavior for missing active goalie-readiness
  cache records, including no cache-directory creation on read.
- Mounted `/goalies/readiness` and `/api/v1/goalies/readiness`.
- Added focused L2 route evidence for active-cache defaults, missing-cache
  unavailable state, ready HTML rendering, ready JSON rendering, non-claim copy,
  and route inventory/surface-parity coverage.
- Updated WP-009 VTRACE evidence rows and surface parity to distinguish this
  goalie-readiness route from the generic named-cache report, coach route,
  opponent-scout route, player evidence-card route, and line-combination route.

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

- This is the first goalie-readiness cache-backed route, not a finished workload
  manager, injury model, deployment recommendation, or starter decision workflow.
- Only `expected_goals_share` is the default metric for the active goalie route;
  broader goalie workload/readiness metric families remain future product work.
- Practice, postgame, and agent surfaces still require their own copy review and
  evidence before cache-backed claims.
