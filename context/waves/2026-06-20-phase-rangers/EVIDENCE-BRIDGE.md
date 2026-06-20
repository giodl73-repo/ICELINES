# Rangers evidence-envelope bridge

Date: 2026-06-20

## Decision

Do not bridge `signals-roster` into the existing analytics cache/evidence-card
envelope in Phase Rangers.

Use the existing WP-009 analytics cache envelope for future cached decision
surfaces, but keep Signals discovery on `PlayerSignalsView` until a later
Signals cache-promotion gate explicitly authorizes cache metric keys and
methodology.

## Rationale

WP-009's cache contract validates records against supported `StatKey` metric
keys, source-state/invalidation inputs, and declared consumer kinds. That is the
right contract for prepared analytics cache records, but Phase Rangers Pulse 03
and Pulse 04 explicitly kept Signals out of:

- stable `StatId`;
- `--filter` and `query leaders`;
- public cross-team leaderboards;
- analytics-cache metric publication.

Forcing `signals-roster` into `AnalyticsCacheConsumerView` now would either
invent cache metric keys before the Signals promotion gate or weaken the cache
contract by treating non-cached derived Signals as cached evidence.

## Current bridge posture

| Surface | Canonical evidence shape | Cache posture |
|---|---|---|
| `icelines signals "<player>"` | `PlayerSignalsView`; `signals.v1` JSON | Not cached |
| `icelines export md signals --player <name>` | `PlayerSignalsView` rendered as Markdown | Not cached |
| `icelines signals-roster --team <ABBR>` | team-scoped collection of `PlayerSignalsView` rows; `signals-roster.v1` JSON | Not cached |
| `/player/evidence-card` | `AnalyticsCacheConsumerView` from WP-009 | Existing cache evidence-card route |

## Future bridge gate

A later cache bridge may proceed only if it adds:

- supported cache metric keys for each Signal or a typed non-`StatId` cache
  extension accepted by WP-009 owners;
- source-state and invalidation semantics for Signal inputs;
- methodology versioning for the Signal formulas;
- consumer contract fixtures proving no renderer recomputes Signal meaning;
- product-copy review preserving scorer-bias, unavailable evidence, and
  non-claim text;
- explicit acceptance that cache publication does not imply prediction,
  deployment, betting, injury, player-quality grading, or autonomous coaching.

Until then, the Rangers Signals discovery lane remains a direct
`PlayerSignalsView` consumer.
