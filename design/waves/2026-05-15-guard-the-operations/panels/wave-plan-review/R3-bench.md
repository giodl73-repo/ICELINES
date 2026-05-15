# R3 Review - bench

## Findings

### F-01 - WARN: Each moved partial needs a test that names the bug class
File: `design/waves/2026-05-15-guard-the-operations/OPERATIONS-PARITY-INVENTORY.md`
Finding: The wave covers several partials whose failure modes differ: runtime-vs-persistent config, destructive data remove, active snapshot deletion, and rule editing.
Consequence: A broad route smoke test would not prove the specific safety property that made the partial risky.
Fix: Each implementation pulse should add or name focused tests such as persistence round-trip, active-delete rejection, unknown-target rejection, and POST-only mutation coverage.

### F-02 - NOTE: No live network tests
File: `design/waves/2026-05-15-guard-the-operations/plans/pulse-03.md`
Finding: Pulse 03 explicitly prohibits live release download tests.
Consequence: The plan stays compatible with IceLines fixture discipline.
Fix: Use tempdir stores, bundled data, and mocked/dry-run contracts for any data-operation tests.
