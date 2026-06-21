# Phase Blackhawks

## Scope

Plan and execute the playoff bracket/detail boundary gate. The wave decides
whether the separate Playoff bracket/detail row should remain partial or be
promoted to a bounded detail/export claim.

## Entry Posture

- Main playoff CLI/TUI/Web/JSON surfaces already project through
  `PlayoffsView`.
- The separate `Playoff bracket/detail` matrix row remains partial even though
  TUI series detail and Markdown `export md series` both render bundled
  game-log detail from `PlayoffsView`.
- Existing evidence covers CLI series commands, TUI series detail rendering,
  Web `/playoffs`, and Markdown series export with game-margin SVG.

## Goals

1. Inventory playoff bracket/detail evidence, supported surfaces, and blockers.
2. Validate focused CLI/TUI/Web/export evidence for the bounded detail/export
   claim.
3. Decide whether this row can be promoted from partial to bounded
   `PlayoffsView` detail/export.
4. Tighten surface-matrix wording to avoid implying live playoff data,
   exhaustive web series-drilldown, or prediction/analysis claims.
5. Close the phase with exact final wording.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Blackhawks goals | passed; see `BLACKHAWKS-INVENTORY.md` and `pulses/pulse-01.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused playoff CLI/TUI/Web/export tests.
- No live network dependency in tests.
