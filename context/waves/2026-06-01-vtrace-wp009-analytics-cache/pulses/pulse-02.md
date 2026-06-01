# WP-009 Pulse 02 - Initial core analytics cache contract

## Scope

Implement the first Rust contract slice for the major analytics cache without
claiming production cache storage or downstream hockey screens.

## Evidence

- Added `icelines-core::analytics_cache` with:
  - `AnalyticsCacheRecord` and schema version `1`.
  - `AnalyticsCacheConsumerEnvelope` and consumer contract version `1`.
  - Cache scope, entity, filter, source-window, metric, quality, invalidation,
    disclosure, and non-claim fields.
  - Builder/parser helpers that validate schema version, cache key, scope, source
    window, source state, supported metrics, and disclosures.
  - Consumer-envelope helper that refuses unsupported consumer contract versions
    and unsupported consumer surfaces.
- Reused existing core view vocabulary (`ViewWindow`, `SourceState`,
  `SourceProvenance`, `MetricCell`, `ViewWarning`) so future surfaces consume one
  evidence envelope instead of rebuilding source-state or methodology meaning.
- Added focused tests for:
  - serde round trip and evidence preservation;
  - local snapshot source-state preservation;
  - top-level and metric-level live-fetch-source refusal;
  - newer-schema refusal before projection;
  - unsupported metric refusal;
  - coach-dashboard consumer-envelope preservation;
  - consumer-contract mismatch refusal;
  - unsupported-consumer refusal.

## Commands

```powershell
cargo test -p icelines-core analytics_cache --quiet
cargo fmt --check
cargo clippy -p icelines-core --lib --tests -- -D warnings
C:\src\proof\target\debug\proof.exe check C:\src\TRACKER\repos\applied-systems\icelines\docs\vtrace --errors-only
git diff --check
```

## Validation disposition

| Scenario | Result | Notes |
|---|---|---|
| VAL-011 | partial | Initial in-core schema/source/consumer fixture passed; production cache storage, broad stale/partial/missing/invalidation fixtures, and downstream surfaces remain pending. |

## Decision

`WP-009` moves from `target_spec_pending` to `partial` for the core contract
slice only. ICELINES still must not claim cache-backed dashboard, report, player
card, line, goalie, practice, postgame, or agent behavior until later storage and
consumer evidence passes.
