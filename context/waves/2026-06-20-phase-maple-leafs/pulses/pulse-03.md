# Phase Maple Leafs Pulse 03 - Matrix Wording

**Date:** 2026-06-20
**Result:** Passed

## Work Completed

- Updated `design/specs/surface-parity.md` to make Career/cohort leaders
  partial by design rather than vaguely partial.
- Recorded that TUI career cohorts remain command-bar handoff-only with tested
  CLI/Web target flashes.
- Preserved cold-store and read-surface boundaries: no bundled career-history
  claim and no live career-history fetch from read surfaces.

## Decision

Do not add a native TUI career cohort board in this phase. The canonical
`CareerView` cohort table already exists on CLI/Web/JSON/dashboard summaries,
and the tested TUI handoff is enough unless a future phase adds new
TUI-specific fields or workflows.

## Validation

- `cargo test -p icelines-cli career`
- `cargo test -p icelines-web --test l1_router career`
- `git diff --check`

## Next Pulse

Pulse 04 closes Phase Maple Leafs and records the deliberate partial as final.
