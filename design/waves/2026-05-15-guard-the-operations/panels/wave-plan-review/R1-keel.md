# R1 Review - keel

## Findings

### F-01 - WARN: Do not make web admin state long-lived by accident
File: `design/waves/2026-05-15-guard-the-operations/plans/pulse-02.md`
Finding: Pulse 02 touches config/report state, where web request handlers are one-shot but CLI/TUI config can be persistent.
Consequence: A handler-local or process-local fix would appear to work in browser tests while diverging from CLI/TUI persistence.
Fix: Reuse the existing config contract for any durable report-toggle path, or label runtime-only web config explicitly and keep persistence deferred.

### F-02 - NOTE: Career TUI handoff should stay out of this wave
File: `design/waves/2026-05-15-guard-the-operations/OPERATIONS-PARITY-INVENTORY.md`
Finding: The career cohort TUI board is marked done/handoff-only.
Consequence: Pulling it into an operations wave would duplicate canonical CLI/web cohort rendering without new long-lived TUI state.
Fix: Keep the handoff decision unless a future phase adds new TUI-specific career fields.
