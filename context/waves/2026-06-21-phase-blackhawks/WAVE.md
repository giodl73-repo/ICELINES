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
| 02 | Playoff detail/export evidence gate | passed; focused evidence supports bounded `PlayoffsView` detail/export claim, see `pulses/pulse-02.md` |
| 03 | Playoff detail/export matrix wording | passed; surface matrix promotes bounded detail/export claim, see `pulses/pulse-03.md` |
| 04 | Close Phase Blackhawks | passed; phase closed with Playoff bracket/detail promoted to bounded detail/export, see `pulses/pulse-04.md` |

## Closeout

Phase Blackhawks is closed. Playoff bracket/detail is promoted to bounded
`PlayoffsView` detail/export: CLI bracket and series detail, TUI bracket and
series detail, Web `/playoffs` and `/api/v1/playoffs`, and Markdown
`export md series` render bundled playoff bracket and game-log rows from the
shared playoff contract.

The claim remains bounded. It does not include live playoff fetch/recompute
behavior, predictive momentum, betting analysis, causal series analysis,
inferred missing game logs, or new Web series-drilldown routes.

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused playoff CLI/TUI/Web/export tests.
- No live network dependency in tests.
