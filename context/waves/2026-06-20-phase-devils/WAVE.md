# Phase Devils

## Scope

Plan and execute the dashboard visual QA gate left after Phase Islanders. The
wave focuses on repeatable browser captures, automated artifact checks, and
truthful promotion wording for `/dashboard` without claiming unsupported browser,
touch, focus, or accessibility breadth.

## Entry posture

- Phase Islanders is wrapped as of 2026-06-20.
- `scripts/web-dashboard-capture.ps1` builds `icelines-cli`, starts
  `icelines --no-live serve`, and captures four Edge/Chrome screenshots.
- Islanders pulse 04 recorded selected nonblank desktop/mobile browser evidence
  for leaders, poach, fantasy, and team-season workspaces.
- `design/specs/surface-parity.md` still fences full live-browser, touch/focus,
  and exhaustive responsive proof as future visual QA claims.

## Goals

1. Inventory the current dashboard capture harness and workspace coverage.
2. Expand or explicitly fence the dashboard capture matrix.
3. Add automated artifact checks for captured dashboard screenshots.
4. Decide whether responsive/focus checks are in scope for this wave or remain
   deferred.
5. Close the phase with an exact surface-matrix browser-proof claim.

## Pulse log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Devils goals | passed; see `DEVILS-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Dashboard capture matrix harness | passed with representative desktop/tablet/mobile workspace captures; see `scripts/web-dashboard-capture.ps1` and `pulses/pulse-02.md` |
| 03 | Dashboard artifact validation | passed with route readiness, dimension, and sampled nonblank checks; see `scripts/web-dashboard-capture.ps1` and `pulses/pulse-03.md` |

## Validation posture

- Planning/doc-only edits use `git diff --check`.
- Script or browser-gate changes run offline through `icelines --no-live serve`.
- No live network dependency in tests.
