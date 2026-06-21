# Phase Lightning

## Scope

Plan and execute the Career/cohort route-row wording gate. The wave does not add
a dedicated TUI career board or change local career-history behavior; it records
the two individual Career route rows as partial by design to match Phase Maple
Leafs.

## Entry Posture

- Phase Maple Leafs closed the Career/cohort leaders gate.
- The surface family row already says Career/cohort leaders are partial by
  design: CLI, Web HTML/JSON, and dashboard summaries use `CareerView`; TUI is a
  tested command-bar handoff to canonical CLI/Web cohort tables.
- The route inventory still starts `/career` and `/api/v1/career` with plain
  `partial -` wording.

## Goals

1. Inventory the Career route rows and Maple Leafs evidence.
2. Validate focused Career Web route evidence.
3. Tighten route-row wording to partial by design, not a full TUI or bundled
   career-history claim.
4. Preserve exact non-claims around dedicated TUI boards, bundled career-history
   availability, live fetch from read surfaces, and implicit local-store
   creation.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Lightning goals | passed; see `LIGHTNING-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Career route evidence gate | passed; focused Career route tests support partial-by-design wording, see `pulses/pulse-02.md` |
| 03 | Career route wording gate | passed; route rows now carry partial-by-design claims, see `pulses/pulse-03.md` |
| 04 | Close Phase Lightning | pending |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused Career Web route tests.
- No live network dependency in tests.
