# IceLines Signals

## Scope

IceLines Signals are descriptive derived metrics built from existing stats. They
are not new official NHL stats, predictive model outputs, betting edges, injury
signals, deployment recommendations, or autonomous coaching decisions.

Signals remain separate from `StatId` until methodology, consumer fit, and
surface copy are reviewed across the product. The first core slice is exposed
through `icelines-core::signal_metrics` only.

## Initial signal set

| Signal | Key | Unit | Polarity | Formula | Required inputs | Limitations |
|---|---|---|---|---|---|---|
| Physical Engagement Rate | `physical-engagement-rate` | per 60 | neutral | `(hits + blocked shots) / TOI * 60` | minimum GP, realtime stats, TOI | Descriptive only. Hits and blocks carry rink scorer bias and do not prove puck recovery, possession value, or player quality. |
| Puck Management Differential | `puck-management-differential` | per 60 | higher is better | `(takeaways - giveaways) / TOI * 60` | minimum GP, realtime stats, TOI | Descriptive only. Takeaways and giveaways are scorer-dependent and do not isolate teammates, zone, deployment, or recovery context. |
| Penalty Drag Rate | `penalty-drag-rate` | per 60 | lower is better | `penalty minutes / TOI * 60` | minimum GP, TOI | Descriptive only. PIM mixes penalty types and does not by itself prove avoidable team harm. |

## Evidence contract

Each signal exposes a descriptor and a typed evidence outcome:

- `SignalEvidenceTier::Full`: all required inputs are present.
- `SignalEvidenceTier::Partial`: at least one required input is present and at
  least one required input is missing.
- `SignalEvidenceTier::Missing`: all required inputs are missing.

Missing realtime, missing or tiny TOI, and below-threshold sample size return
`None` instead of `0.0`. Consumers must render that as unavailable or missing
evidence, not as a player value.

## Promotion rule

Before any signal becomes a stable `StatId`, leaderboard field, report/export
column, cache metric family, or Web/CLI/TUI surface, a later pulse must add:

- product-copy review for the target surface;
- source/completeness disclosure for unavailable and partial evidence;
- parity evidence if more than one surface renders the signal;
- cache-envelope methodology if the signal is cached; and
- explicit refusal of predictive, betting, injury, deployment, and autonomous
  coaching claims.
