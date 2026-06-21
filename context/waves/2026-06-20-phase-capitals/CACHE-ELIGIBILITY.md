# Capitals Cache Eligibility Decision

## Decision

Signals are not eligible for WP-009 analytics cache publication yet.

They remain direct `PlayerSignalsView` projections for now. This preserves the
Phase Hurricane and Phase Rangers rule that Signals do not enter analytics cache
until a separate promotion gate accepts the required cache contract.

## Why

The WP-009 analytics cache contract is not just a reusable JSON envelope. It
requires:

- supported metric keys;
- source-state for record-level and metric-level evidence;
- invalidation keys;
- methodology versioning;
- disclosure and non-claim copy;
- supported consumer kinds.

The current Signals surfaces have strong methodology, unavailable-state, and
non-claim copy through `PlayerSignalsView`, but they do not yet define accepted
cache metric keys, cache invalidation semantics, or metric-level source-state for
Signal publication.

Forcing Signals into `AnalyticsCacheConsumerView` now would either invent those
cache semantics after the fact or weaken the cache contract that WP-009 uses for
prepared analytics records.

## Required Future Gate

A later cache promotion can proceed only if it adds:

- a supported Signal cache metric key set, probably derived from
  `SignalMetricId` but explicitly accepted as cache keys;
- metric-level `SourceState` rules for complete, partial, stale, and unavailable
  Signal inputs;
- invalidation keys for the roster/stat/realtime inputs each Signal consumes;
- a methodology version string tied to the Signal formula set;
- fixtures proving unavailable Signal values remain missing, not zero-filled;
- consumer-kind acceptance for the target cached surface;
- product-copy review preserving descriptive-only non-claims.

## Non-Claims

This decision does not add:

- analytics-cache metric publication;
- `AnalyticsCacheConsumerKind` variants;
- `StatId` rows;
- filter keys;
- public cross-team Signal leaderboards;
- prediction, betting, injury, deployment, player-grade, or autonomous coaching
  recommendations.
