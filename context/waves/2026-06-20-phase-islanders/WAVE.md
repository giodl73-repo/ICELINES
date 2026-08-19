# Phase Islanders

## Scope

Plan and execute the post-Rangers surface-parity cleanup round for ICELINES.
This wave focuses on UX/admin/docs truth, dashboard partial proof, and
cache-backed partial rollups without promoting new analytics claims.

## Entry posture

- Phase Rangers is wrapped as of 2026-06-20.
- `design/specs/surface-parity.md` is still the source-of-truth matrix, but its
  header and several partial rows predate later VTRACE and route evidence.
- Admin and docs routes are mounted and tested, with some operations
  deliberately deferred.
- Dashboard workspace partials have focused route tests; full live visual
  capture remains a separate claim unless this phase records it.
- WP-009 cache-backed surfaces remain partial first-route evidence, not broad
  coach/scout/player/line/goalie/practice/postgame/agent workflow completion.

## Goals

1. Refresh the surface parity matrix so current status and active partials are
   audit-friendly.
2. Tighten admin/docs route truth for done, partial, and deferred operations.
3. Prove or fence selected dashboard workspace partial/browser capture claims.
4. Roll up cache-backed partials without overstating broader workflows.
5. Close the phase with no ambiguous active Islanders pulse remaining.

## Pulse log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Inventory and plan Phase Islanders goals | passed; see `ISLANDERS-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Surface parity matrix refresh | passed; see `design/specs/surface-parity.md` and `pulses/pulse-02.md` |
| 03 | Admin/docs truth pass | passed; see `design/specs/surface-parity.md`, `icelines serve --help`, and `pulses/pulse-03.md` |
| 04 | Dashboard selected capture proof/fence | passed with selected capture evidence; see `scripts/web-dashboard-capture.ps1`, `dist/web-dashboard-captures/`, and `pulses/pulse-04.md` |
| 05 | Cache-backed partial rollup | passed with WP-009 first-route evidence fenced from broader workflow claims; see `design/specs/surface-parity.md` and `pulses/pulse-05.md` |
| 06 | Phase closeout | passed; see `design/archive/plans/2026-06/2026-06-20-phaseIslanders-surface-parity.md`, `design/specs/surface-parity.md`, and `pulses/pulse-06.md` |

## Validation posture

- Planning/doc-only edits use VTRACE proof check when VTRACE files change and
  always use `git diff --check`.
- Implementation pulses add focused tests or scripts for changed behavior.
- No live network dependency in tests.

## Phase Islanders closeout (2026-06-20)

Phase Islanders is wrapped. The phase delivered the planned post-Rangers surface
truth cleanup: a refreshed active partial rollup, explicit admin/docs deferrals,
selected dashboard desktop/mobile capture evidence, a WP-009 cache-backed route
rollup that keeps first-route evidence separate from workflow completion, and a
closeout record with no active Islanders pulse remaining.

No new analytics source claim was promoted. Future work requires new waves:

- Full live-browser, touch/focus, and exhaustive responsive dashboard proof
  remains a visual QA wave.
- Web admin data install/remove and persistent report-toggle writes require a
  scoped confirmation/persistence contract.
- WP-009 cache-backed coach, scout, player, line, goalie, practice, postgame,
  and agent surfaces remain partial until workflow evidence and product-copy
  review promote each family.
- Signals cache/catalog/filter/leaderboard promotion remains outside Islanders
  and requires a separate Signals cache-promotion gate.
