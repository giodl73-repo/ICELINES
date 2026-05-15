# R2 Review - wire

## Findings

### F-01 - BLOCK: Web data install/remove cannot ship as casual admin buttons
File: `design/waves/2026-05-15-guard-the-operations/plans/pulse-03.md`
Finding: Data install is live/network release work and data remove is destructive filesystem mutation.
Consequence: Exposing either without scoped confirmation, fixture-backed tests, and clear errors could mutate user data or fail silently under network drift.
Fix: Pulse 03 must either design a safe POST-backed confirmation/dry-run/local-only contract or keep the operations explicitly deferred.

### F-02 - WARN: Preserve mutation method boundaries
File: `design/waves/2026-05-15-guard-the-operations/WAVE.md`
Finding: This wave touches dashboard, admin, favorites, watch, config, snapshots, and data operations.
Consequence: Mixing read navigation and mutation state would break bookmarkability and invite accidental writes.
Fix: Keep GET for navigation/read state only; route every write through existing POST handlers or typed TUI command intents.
