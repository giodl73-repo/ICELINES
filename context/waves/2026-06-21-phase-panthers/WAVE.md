# Phase Panthers

## Scope

Plan and execute the Player Signals surface-row wording gate. The wave does not
promote Signals into analytics cache, `StatId`, filters, catalog sorting, or
leaderboards; it records the existing direct inspection surfaces as partial by
design to match Phase Capitals.

## Entry Posture

- Phase Hurricane shipped Signals to CLI, TUI player-card, Web HTML/JSON, and
  Markdown export surfaces.
- Phase Rangers shipped `signals-roster` as a team-scoped inspection matrix.
- Phase Capitals kept Signals out of analytics cache, `StatId`, filters,
  catalog-driven sorting, and public cross-team leaderboards until future
  source-state, cache-key, invalidation, and bounded ranking-copy contracts
  exist.
- The surface matrix still starts the Player Signals row with plain `partial,`
  wording.

## Goals

1. Inventory the Player Signals row and Capitals evidence.
2. Validate focused Signals Web route evidence.
3. Tighten the surface-row wording to partial by design, not a promotion claim.
4. Preserve exact non-claims around cache/catalog/filter/leaderboard promotion,
   ranking, betting, injury, deployment, player-grade, and coaching authority.
5. Close the phase with final surface-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Panthers goals | passed; see `PANTHERS-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Signals route evidence gate | passed; focused Signals route tests support partial-by-design wording, see `pulses/pulse-02.md` |
| 03 | Signals surface-row wording gate | passed; surface row now carries partial-by-design wording, see `pulses/pulse-03.md` |
| 04 | Close Phase Panthers | pending |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused Signals Web route tests.
- No live network dependency in tests.
