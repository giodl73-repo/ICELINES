# R4 Review - wire

## Findings

### F-01 - WARN: Action/status panes must stay POST-backed for mutations
File: `design/waves/2026-05-15-call-the-changes/MDI-WORKBENCH-INVENTORY.md`
Finding: The pane vocabulary includes action/status panes and mutation result
fields, while the compatibility rules forbid GET mutations.
Consequence: If an experience tab or pane selector encodes an action as a GET
link, dashboard navigation could mutate favorites, watch rules, caches, or
config.
Fix: Pulse 04 should keep catalog/tab/pane GET links read-only and delegate any
mutation to existing POST-backed intents with explicit result rendering.

### F-02 - NOTE: Dashboard URL invariants are preserved
File: `design/waves/2026-05-15-call-the-changes/plans/pulse-04.md`
Finding: The web plan preserves `/dashboard?workspace=...`, workspace partials,
canonical full routes, and local side-pane state.
Consequence: Experience tabs can enhance the dashboard without turning it into
a browser-only SPA state model.
Fix: Add route/static tests for URL invariants and no-JS rendering.
