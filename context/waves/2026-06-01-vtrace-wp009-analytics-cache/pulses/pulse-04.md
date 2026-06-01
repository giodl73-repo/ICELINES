# Pulse 04 - Downstream Consumer ViewModel Fixture

## Scope

Add the first downstream analytics-cache consumer fixture without claiming a
shipped dashboard, report, card, line, goalie, practice, or postgame surface.

## Change

- Added `icelines-core::view_model::analytics_cache_consumer`, a minimal
  dashboard-style ViewModel that projects an `AnalyticsCacheConsumerEnvelope`
  into display-ready fields.
- Re-exported the consumer ViewModel through `icelines-core`.
- Added a core fixture proving the ViewModel preserves cache key, consumer kind,
  source window/state/provenance/freshness, quality/completeness/warnings,
  methodology, disclosures, non-claims, supported metrics, and prepared metric
  rows without recomputing cache semantics.
- Added a fetch-store fixture proving a strict JSON cache record can be written,
  read, adapted to a coach-dashboard envelope, and consumed by the ViewModel
  while preserving the stored contract.

## Evidence

- `cargo test -p icelines-core analytics_cache --quiet`
- `cargo test -p icelines-fetch analytics_cache_store --quiet`
- `cargo fmt --check`
- `cargo clippy -p icelines-core --lib --tests -- -D warnings`
- `cargo clippy -p icelines-fetch --lib --tests -- -D warnings`
- `C:\src\proof\target\debug\proof.exe check C:\src\TRACKER\repos\applied-systems\icelines\docs\vtrace --errors-only`
- `git diff --check`

## Result

Passed. The WP-009 consumer-contract criterion now has an internal downstream
ViewModel fixture plus a store-backed feed fixture. Product surfaces remain
unclaimed until a later DCR implements and reviews user-facing cache-backed copy.
