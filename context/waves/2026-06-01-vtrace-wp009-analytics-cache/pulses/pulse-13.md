# WP-009 Pulse 13 - Postgame adjustment-review cache surface

Date: 2026-06-01

## Scope

Add a second narrow postgame Web surface for the Major Analytics Cache without
promoting blame assignment, causal win/loss explanation, automatic correction, or
a finished postgame workflow. The slice mounts active-context routes:

- `/postgame/adjustments`
- `/api/v1/postgame/adjustments`

The routes derive the default cache key from Web active config as
`postgame_adjustments:<season>:<season_type>` and default the selected postgame
metric set to `expected_goals_share`. Operators can still pass `cache_key` or
`metrics` for explicit cache inspection, but the normal adjustment-review route
does not require the generic named-cache report query contract.

## Changes

- Added postgame adjustment-review HTML and JSON handlers that reuse
  `AnalyticsCacheStore`, `analytics_cache_consumer_envelope`, and
  `AnalyticsCacheConsumerView`.
- Reused `AnalyticsCacheConsumerKind::PostgameReviewReport` so the route stays a
  postgame consumer surface instead of introducing a new local semantic contract.
- Preserved explicit unavailable behavior for missing active
  postgame-adjustments cache records, including no cache-directory creation on
  read.
- Mounted `/postgame/adjustments` and `/api/v1/postgame/adjustments`.
- Added focused L2 route evidence for active-cache defaults, missing-cache
  unavailable state, ready HTML rendering, ready JSON rendering, non-claim copy,
  and route inventory/surface-parity coverage.
- Updated WP-009 VTRACE evidence rows and surface parity to distinguish this
  adjustment-review route from the first postgame review route and the broader
  unfinished postgame workflow.

## Evidence

| Level | Evidence | Result |
| --- | --- | --- |
| L2 | `cargo test -p icelines-web --test l2_analytics_cache_report --quiet` | passed |
| L1 | `cargo test -p icelines-web --test ted_lindsay_route_inventory --quiet` | passed |
| L1 | `cargo clippy -p icelines-web --lib --tests -- -D warnings` | passed |
| L0 | `cargo fmt --check` | passed |
| VTRACE | `C:\src\proof\target\debug\proof.exe check C:\src\TRACKER\repos\applied-systems\icelines\docs\vtrace --errors-only` | passed |
| Hygiene | `git diff --check` | passed |

## Residual risk

- This is a second narrow postgame cache-backed route, not a finished postgame
  report suite, loss-cause attribution engine, automatic correction planner, or
  autonomous coaching workflow.
- Only `expected_goals_share` is the default metric for the active postgame
  adjustment-review route; broader postgame review metric families remain future
  product work.
- Agent surfaces still require their own copy review and evidence before
  cache-backed claims.
