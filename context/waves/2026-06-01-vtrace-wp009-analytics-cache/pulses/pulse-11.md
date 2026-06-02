# WP-009 Pulse 11 - Practice focus cache surface

Date: 2026-06-01

## Scope

Add the first practice focus Web surface for the Major Analytics Cache without
promoting mandatory drill plans, practice prescriptions, or a finished practice
workflow. The slice mounts active-context routes:

- `/practice/focus`
- `/api/v1/practice/focus`

The routes derive the default cache key from Web active config as
`practice_focus:<season>:<season_type>` and default the selected practice metric
set to `expected_goals_share`. Operators can still pass `cache_key` or `metrics`
for explicit cache inspection, but the normal practice focus route no longer
requires the generic named-cache report query contract.

## Changes

- Added practice focus HTML and JSON handlers that reuse
  `AnalyticsCacheStore`, `analytics_cache_consumer_envelope`, and
  `AnalyticsCacheConsumerView`.
- Passed `AnalyticsCacheConsumerKind::PracticeFocusReport` through the Web
  consumer path so ready JSON and HTML render the practice-specific ViewModel
  title and consumer identity.
- Preserved explicit unavailable behavior for missing active practice-focus
  cache records, including no cache-directory creation on read.
- Mounted `/practice/focus` and `/api/v1/practice/focus`.
- Added focused L2 route evidence for active-cache defaults, missing-cache
  unavailable state, ready HTML rendering, ready JSON rendering, non-claim copy,
  and route inventory/surface-parity coverage.
- Updated WP-009 VTRACE evidence rows and surface parity to distinguish this
  practice-focus route from the generic named-cache report, coach route,
  opponent-scout route, player evidence-card route, line-combination route, and
  goalie-readiness route.

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

- This is the first practice-focus cache-backed route, not a finished practice
  planner, drill prescription engine, workload manager, or autonomous coaching
  workflow.
- Only `expected_goals_share` is the default metric for the active practice
  route; broader practice workload/focus metric families remain future product
  work.
- Postgame and agent surfaces still require their own copy review and evidence
  before cache-backed claims.
