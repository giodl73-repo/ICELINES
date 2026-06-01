# VTRACE WP-008 Integration Rehearsal

## Scope

Run the final VTRACE integration and validation rehearsal after WP-001 through
WP-006 are closed with risk and WP-007 is explicitly target-not-met dispositioned.

## Entry posture

- WP-001 through WP-006 have repo-local pulse evidence and closeout dispositions.
- WP-007 has dependency/lean inventory evidence, an owner, and a revisit trigger.
- `docs/vtrace/` is the controlling specification baseline.

## Exit posture

- Broad workspace format, test, and lint gates pass in the final implementation
  state.
- The stale Lindsay L3 golden-output mismatch is corrected by regenerating
  fixture outputs that now include source-state and result-state context lines.
- The Foster invalid-date/no-live regression exposed by the broad test gate is
  fixed by validating date arguments before live-feed short-circuits.
- The Wave 6 fetch-boxscore dry-run/no-live regression exposed by the broad
  test gate is fixed by preserving plain dry-run output before live-feed refusal
  while keeping `--for-favorites` refusal before cache writes.
- The Web fantasy read-only regression exposed by the broad route gate is fixed
  by checkpointing FantasyDb WAL writes aggressively enough for immutable
  read-only GET routes to see seeded/imported rows without creating sidecars.
- VTRACE proof and diff checks pass after closeout documentation updates.
- Release/readiness remains `closed_with_risk`, not fully passed, because
  dependency/lean support is still target-not-met and selected browser/TUI/report
  matrix breadth remains accepted residual risk.

## Pulse log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Integration rehearsal and VTRACE closeout | passed_with_risk |
