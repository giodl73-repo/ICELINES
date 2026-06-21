# Phase Maple Leafs

## Scope

Plan and execute the career/cohort leaders promotion gate. The wave decides
whether cross-league career cohorts should remain a deliberate TUI handoff to
canonical CLI/Web surfaces or become a dedicated TUI board.

## Entry Posture

- `design/specs/surface-parity.md` marks Career/cohort leaders as partial
  because CLI, Web HTML/JSON, and dashboard summaries use `CareerView`, while
  TUI remains intentionally handoff-only.
- Prior operations inventory records that the handoff is deliberate because
  local career-history data is unbundled.
- Existing evidence covers TUI command-bar career handoff, CLI projection from
  `CareerView`, Web HTML shell rendering, Web JSON envelope shape, missing-store
  guidance, and dashboard `/career` workspace summary routing.

## Goals

1. Inventory career/cohort route evidence, command handoff behavior, and
   cold-store guidance.
2. Decide whether a dedicated TUI career cohort board adds value beyond the
   canonical CLI/Web table.
3. Preserve explicit non-claims: no bundled career-history availability on cold
   install, no live fetch from read routes, and no implied TUI-native board.
4. Validate focused CLI/Web/TUI handoff evidence.
5. Close the phase with exact surface-matrix wording.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Maple Leafs goals | passed; see `MAPLE-LEAFS-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Career evidence gate | passed; focused CLI/TUI/Web evidence supports deliberate TUI handoff, see `pulses/pulse-02.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused career CLI/TUI/Web tests.
- No live network dependency in tests.
