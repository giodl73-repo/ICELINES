# WP-009 Pulse 05 - Product-facing analytics cache report

Date: 2026-06-01

## Scope

Add the first narrow hockey-facing Web surface for the Major Analytics Cache
without promoting the cache into a broad dashboard. The slice mounts a named
cache-key report and JSON twin:

- `/reports/analytics-cache?cache_key=<key>&metrics=<metric,...>`
- `/api/v1/reports/analytics-cache?cache_key=<key>&metrics=<metric,...>`

The report reads an existing analytics cache record through the fetch store,
projects it through the core consumer envelope/ViewModel, and preserves source
state, quality, methodology, disclosures, non-claims, and metric evidence. It
does not recompute analytics, fetch live data, or fabricate missing cache state.

## Changes

- Added `icelines-web::handlers::analytics_cache_report` with HTML and JSON
  handlers backed by `AnalyticsCacheStore::read_record`.
- Added the Askama template and template rows for cache-backed report rendering.
- Mounted the HTML and JSON routes in `icelines-web::router`.
- Added focused L2 route evidence for explicit missing-cache unavailable state
  and successful cache-envelope rendering.
- Updated the Ted Lindsay route inventory and surface-parity matrix for the new
  product-facing report routes.

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

- This is a cache-key-driven report, not a finished coach dashboard, opponent
  scout, player evidence card, or line explorer.
- The route requires an explicit `metrics` query so store validation can verify
  the record against the supported metric contract.
- HTML unavailable states intentionally render as a readable report page; the
  JSON twin maps store errors to structured 4xx/5xx responses.
