# VTRACE WP-010 IceLines Signals

## Scope

Start the IceLines Signals metric family as a core-only methodology slice. This
wave introduces descriptive derived metrics and typed evidence coverage without
promoting new stable `StatId` values, public leaderboards, reports, cache metric
families, prediction, betting, injury certainty, deployment authority, or
autonomous coaching claims.

## Entry posture

- WP-009 remains partial for major analytics cache surfaces.
- Signals are new methodology work and therefore execute as WP-010 rather than a
  continuation of WP-009 cache route expansion.
- The first slice must stay inside `icelines-core` and use existing
  `PlayerView`/season stat inputs.

## Implementation sequence

1. Define a core signal descriptor/evidence API with stable metric IDs, labels,
   keys, formulas, required inputs, units, polarity, methodology, and
   limitations.
2. Implement the first safe descriptive signal set:
   Physical Engagement Rate, Puck Management Differential, and Penalty Drag Rate.
3. Prove missing realtime, missing/tiny TOI, and below-threshold sample size do
   not zero-fill.
4. Add an internal ViewModel boundary before any shipped surface consumes the
   signals.
5. Document the product semantics and promotion rule before any surface consumes
   the signals.

## Pulse log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Core signal descriptor, evidence, and formula contract | core_signals_partial_passed |
| 02 | Internal Signals ViewModel boundary | core_signals_viewmodel_partial_passed |
| 03 | `icelines signals` CLI + `signals.v1` JSON surface (Phase Hurricane) | signals_cli_surface_passed |
| 04 | TUI player-card Signals block + Web HTML/API parity | signals_tui_web_surface_passed |

## Residual risk

- The first signal set is descriptive and scorer-biased where it uses realtime
  rink-recorded events.
- Signals now have CLI text, `signals.v1` JSON, a TUI player-card block, and Web
  HTML/API surfaces. They are still not exposed through reports, exports, the
  `--filter` catalog, leaderboards, `StatId`, or the analytics cache.
- Broader signal families such as special-teams leverage, creation pressure, or
  evidence-card integrations require their own methodology and copy review.
