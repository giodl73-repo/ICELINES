# Phase Sharks Dashboard Command Inventory

## Purpose

Inventory the dashboard command route row before tightening its route wording.

## Current Surface

| Area | Evidence | Sharks Dashboard Command posture |
|---|---|---|
| Read redirects | `POST /dashboard/command` read commands | Keep allowlisted workspace redirects for TUI-shaped read commands. |
| Pane/report state | dashboard pane/report commands | Keep URL-state preservation for pane/report command redirects. |
| Errors | unknown commands | Keep explicit error labels without redirecting. |
| Mutations | favorite/watch command paths | Keep delegation to existing POST handlers/intents; reject unsupported deployment-watch before persistence. |

## Risks to Avoid

- Claiming new command parsing behavior.
- Claiming persistence for unsupported deployment-watch commands.
- Broadening workspace redirect allowlists.
- Changing runtime behavior while performing a wording gate.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused dashboard command tests cover read
   redirects, report redirects, unknown errors, watch delegation, watch toggle
   delegation, and deployment-watch rejection.
3. Matrix wording. Result: passed; dashboard command row now carries scoped
   allowlist/delegation wording.
4. Closeout. Result: passed; Phase Sharks Dashboard Command is closed with final
   route-row claims and non-claims recorded.
