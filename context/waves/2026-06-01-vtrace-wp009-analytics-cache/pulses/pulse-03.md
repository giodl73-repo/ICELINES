# WP-009 Pulse 03: Analytics cache store/read path

## Scope

Add the first production-oriented analytics cache storage/read path around the
WP-009 core contract without attaching downstream hockey screens yet.

## Implementation

- Added `icelines-fetch::analytics_cache_store` as a strict JSON store rooted
  under `<data_root>/analytics_cache`.
- Cache writes validate the core `AnalyticsCacheRecord` contract before atomic
  persistence.
- Cache reads parse and validate stored JSON through
  `parse_analytics_cache_record_json`, returning explicit missing-cache,
  unsupported-schema, unsupported-metric, stale, and rebuild-required states
  without any live fetch path.
- Invalidation can remove one encoded cache key or all records matching an
  explicit invalidation key.
- The core contract now rejects missing invalidation keys, empty invalidation
  keys, missing methodology version, and missing supported consumers, and exposes
  `analytics_cache_read_disposition` for freshness/staleness/rebuild decisions.

## Evidence

- `icelines-core/src/analytics_cache.rs`
- `icelines-fetch/src/analytics_cache_store.rs`
- `icelines-fetch/src/lib.rs`
- `cargo test -p icelines-core analytics_cache --quiet`
- `cargo test -p icelines-fetch analytics_cache_store --quiet`
- `cargo fmt --check`
- `cargo clippy -p icelines-core --lib --tests -- -D warnings`
- `cargo clippy -p icelines-fetch --lib --tests -- -D warnings`
- `C:\src\proof\target\debug\proof.exe check C:\src\TRACKER\repos\applied-systems\icelines\docs\vtrace --errors-only`
- `git diff --check`

## Result

`partial_pass`: WP-009 now has a strict cache store/read/invalidation fixture
slice in addition to the core schema/consumer contract. Downstream dashboard,
report, card, line, goalie, practice, and postgame consumers remain pending.
