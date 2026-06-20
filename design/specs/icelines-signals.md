# IceLines Signals

## Scope

IceLines Signals are descriptive derived metrics built from existing stats. They
are not new official NHL stats, predictive model outputs, betting edges, injury
signals, deployment recommendations, or autonomous coaching decisions.

Signals remain separate from `StatId` until methodology, consumer fit, and
surface copy are reviewed across the product. The first core slices are exposed
through `icelines-core::signal_metrics` and the internal
`icelines-core::view_model::signals` boundary only.

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

`PlayerSignalsView` is the canonical internal consumer shape for future
renderers. It carries player identity, active window, one row per Signal, typed
evidence tiers, missing-input labels, methodology, limitations, disclosures, and
non-claim copy so renderers do not recompute signal meaning locally.

## Surface status

- **CLI + JSON (live, Phase Hurricane / WP-010 pulse-03):** `icelines signals
  "<player>"` renders `PlayerSignalsView` as a text table and, with `--json`, a
  frozen `signals.v1` envelope. Both encodings carry the evidence tier, missing
  inputs, methodology, limitations, disclosures, and non-claim copy; missing or
  partial evidence renders as `unavailable`/`null`, never zero-fill. Product copy
  for this surface is reviewed; Signals remain **out of** `StatId`, the `--filter`
  catalog, and leaderboards.
- **TUI + Web (live, Phase Hurricane / WP-010 pulse-04):** the player-card TUI
  block renders the same `PlayerSignalsView` rows and links to the Web surface.
  Web HTML lives at `/player/:id/signals`, and Web JSON lives at
  `/api/v1/player/:id/signals` with the `player-signals` data/meta envelope. L0
  and L1 fences prove unavailable evidence remains `unavailable`/`null`, not
  zero-filled.
- **Report/export (live, Phase Hurricane / WP-010 pulse-05):** `export md
  signals --player "<player>"` renders a disclosure-first Markdown packet from
  the same `PlayerSignalsView`, including evidence tiers, missing inputs,
  methodology, limitations, disclosures, and non-claim copy. Signals still remain
  out of `StatId`, the `--filter` catalog, leaderboards, and analytics cache
  publication.

## Promotion rule

Before any signal becomes a stable `StatId`, leaderboard field, additional
report/export field beyond the selected `export md signals` packet, cache metric
family, or additional Web/CLI/TUI surface, a later pulse must
add:

- product-copy review for the target surface;
- source/completeness disclosure for unavailable and partial evidence;
- parity evidence if more than one surface renders the signal;
- cache-envelope methodology if the signal is cached; and
- explicit refusal of predictive, betting, injury, deployment, and autonomous
  coaching claims.

## Phase Rangers discovery gate

Phase Rangers pulse 03 accepts one narrow discovery lane: a team-scoped roster
matrix that helps users find which player Signals cards deserve inspection.
Pulse 04 ships that lane as `icelines signals-roster --team <ABBR>` with a
`signals-roster.v1` JSON twin. This is not a leaderboard and does not promote
Signals into `StatId`, `--filter`, analytics cache, or public cross-team ranking
surfaces.

Any implementation of this matrix must preserve evidence tiers, missing-input
summaries, unavailable values, and non-claim copy. Missing or partial evidence
must render as unavailable/missing evidence, never as `0.0` player truth.

Phase Rangers pulse 05 keeps the roster matrix outside the analytics cache
envelope. A future bridge to `AnalyticsCacheConsumerView` requires accepted
Signal cache metric keys or an accepted cache-contract extension, plus
source-state, invalidation, methodology-version, and non-claim evidence.
