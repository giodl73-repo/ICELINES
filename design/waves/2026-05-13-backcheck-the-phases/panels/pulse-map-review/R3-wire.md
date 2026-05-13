# R3 Review - wire

## Findings

### F-01 - BLOCK: Pulse 07 must split safe admin UX from live network install/remove
File: `design/waves/2026-05-13-backcheck-the-phases/plans/pulse-07.md`
Finding: The mission names destructive web install/remove as possible parity work. Data install/remove can involve filesystem mutation and, for install, live/download behavior unless routed through an existing fixture-backed local path.
Consequence: A web admin pulse could accidentally introduce a network-dependent test, a long-running web mutation, or an unsafe operation exposed through HTML.
Fix: Start Pulse 07 with an inventory/decision table. Implement only operations that already have typed intent/result contracts and fixture-backed tests. Explicitly defer live install/remove web execution unless the pulse first adds a safe dry-run/local-only contract and POST-backed tests.

### F-02 - WARN: Dashboard/watch mutations must stay POST-backed
File: `design/waves/2026-05-13-backcheck-the-phases/plans/pulse-06.md`
Finding: Pulse 06 touches TUI watch UX and lists dashboard/web watch files as likely files. The plan has a stop condition against GET mutations, but the gates do not explicitly assert it.
Consequence: A UI shortcut could regress the mutation boundary by turning a watch add/toggle into a link-like GET route.
Fix: Include an existing or new route/parser test proving watch/favorite commands resolve to POST-backed handlers or mutation intents, not GET workspace URLs.
