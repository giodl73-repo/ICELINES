# WP-009 Pulse 06 - Coach dashboard cache surface

Date: 2026-06-01

## Scope

Add the first coach-specific Web surface for the Major Analytics Cache without
promoting a broad analytics dashboard suite. The slice mounts active-context
routes:

- `/coach/dashboard`
- `/api/v1/coach/dashboard`

The routes derive the default cache key from Web active config as
`coach_dashboard:<season>:<season_type>` and default the selected coach metric
set to `expected_goals_share`. Operators can still pass `cache_key` or `metrics`
for explicit cache inspection, but the normal coach surface no longer requires
the generic named-cache report query contract.

## Changes

- Added coach dashboard HTML and JSON handlers that reuse
  `AnalyticsCacheStore`, `analytics_cache_consumer_envelope`, and
  `AnalyticsCacheConsumerView`.
- Preserved explicit unavailable behavior for missing active coach cache records,
  including no cache-directory creation on read.
- Mounted `/coach/dashboard` and `/api/v1/coach/dashboard`.
- Added focused L2 route evidence for active-cache defaults, missing-cache
  unavailable state, ready HTML rendering, ready JSON rendering, non-claim copy,
  and route inventory/surface-parity coverage.
- Updated WP-009 VTRACE evidence rows and surface parity to distinguish this
  coach route from the generic named-cache report.

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

- This is the first coach-specific cache-backed route, not a finished multi-panel
  coach dashboard suite.
- Only `expected_goals_share` is the default metric for the active coach route;
  broader metric families and discovery remain future product work.
- Opponent scout, player evidence card, line explorer, goalie readiness,
  practice, postgame, and agent surfaces still require their own copy review and
  evidence before cache-backed claims.
