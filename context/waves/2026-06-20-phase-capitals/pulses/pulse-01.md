# Phase Capitals Pulse 01 - Plan and Inventory

**Date:** 2026-06-20
**Result:** Passed

## Work Completed

- Created the plan now archived at
  `design/archive/plans/2026-06/2026-06-20-phaseCapitals-signals-cache-promotion.md`.
- Created `context/waves/2026-06-20-phase-capitals/WAVE.md`.
- Created `context/waves/2026-06-20-phase-capitals/CAPITALS-INVENTORY.md`.
- Recorded the current Signals surfaces, inherited Hurricane/Rangers
  non-promotions, and the promotion decisions Capitals must make.
- Added Phase Capitals to the plan and wave indexes.

## Decision

Capitals starts as a promotion gate, not an implementation promise. The default
posture is no cache, catalog, filter, or leaderboard promotion unless a later
pulse accepts the necessary source-state, invalidation, methodology, unavailable
state, and product-copy contracts.

## Validation

- `git diff --check`

## Residual Risk

This pulse is planning only. It does not add cache metric keys, `StatId` rows,
filter keys, leaderboards, routes, or CLI commands.

## Next Pulse

Pulse 02 decides whether Signals are eligible for WP-009 analytics cache metric
publication or should remain uncached `PlayerSignalsView` projections.
