# Phase Capitals Pulse 03 - Catalog, Filter, and Leaderboard Gate

**Date:** 2026-06-20
**Result:** Passed as not eligible yet

## Decision

Signals remain outside `StatId`, `--filter`, catalog-driven sorting, and public
cross-team leaderboards.

## Evidence

| Evidence | Result |
|---|---|
| `icelines-core/src/signal_metrics.rs` | Current Signals are descriptive composites with scorer-bias and context limitations; one is neutral polarity and all preserve missing-input gates. |
| `icelines-core/src/stats_catalog.rs` | `StatId` keys drive filter grammar and deterministic sort semantics across product surfaces. |
| `design/specs/icelines-signals.md` | Promotion requires product-copy review, unavailable/partial disclosure, parity evidence, and refusal of predictive/betting/injury/deployment/coaching claims. |
| `context/waves/2026-06-20-phase-capitals/CATALOG-LEADERBOARD-GATE.md` | Records the no-promotion decision and future bounded promotion requirements. |

## Validation

- `git diff --check`

## Residual Risk

Users still cannot filter or rank across the league by Signals. That is
intentional until a later phase proves a bounded subset can carry catalog and
leaderboard semantics without overstating the evidence.

## Next Pulse

Pulse 04 should implement the durable deferral posture in the public docs and
surface matrix, unless a new implementation contract is opened first.
