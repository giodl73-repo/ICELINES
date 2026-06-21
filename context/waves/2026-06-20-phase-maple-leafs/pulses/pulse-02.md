# Phase Maple Leafs Pulse 02 - Career Evidence Gate

**Date:** 2026-06-20
**Result:** Passed

## Work Completed

- Ran focused CLI/TUI/Web career evidence.
- Confirmed TUI command-bar career input flashes exact canonical CLI and Web
  targets instead of pretending to be a native cohort board.
- Confirmed CLI `query career` rows project from `CareerView`.
- Confirmed Web `/career`, `/api/v1/career`, dashboard `/career` workspace
  routing, and docs/cold-store guidance remain covered by focused L1 tests.

## Validation

- `cargo test -p icelines-cli career`
- `cargo test -p icelines-web --test l1_router career`
- `git diff -- Cargo.lock`

## Decision

Keep Career/cohort leaders partial by design. The current TUI behavior is an
intentional handoff to canonical CLI/Web cohort tables because the local
career-history store is optional and unbundled. A native TUI board should remain
out of scope unless a future phase adds new TUI-specific fields or workflows.

## Residual Risk

The broad CLI `career` filter runs more evidence than the minimum command-bar
handoff tests. It is useful here because it also proves adjacent career-table
and capability boundaries, but future evidence gates can split it if runtime
matters.

## Next Pulse

Pulse 03 tightens the surface matrix wording so the partial status reads as a
deliberate product boundary, not missing verification.
