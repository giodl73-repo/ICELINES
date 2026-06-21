# Phase Red Wings

## Scope

Plan and execute the Favorites/watch/watch-rules boundary gate. The wave
decides whether the current partial remains a deliberate product boundary or
whether shared contracts are ready for richer group/rule editing.

## Entry Posture

- `design/specs/surface-parity.md` marks Favorites/watch/watch-rules partial:
  read/mutation paths are useful and tested, while richer group/rule dimensions
  remain intentionally narrow.
- Guard the Operations already closed safe favorites/groups and watch-rule
  slices: named group reads are read-only on web, canonical `Favorites` mutation
  remains POST-backed, player watch rules can be created/toggled/deleted, and
  unsupported group/team/deployment edits are rejected instead of being
  reinterpreted as safe GET navigation.
- The active matrix still needs a phase-level closeout that makes this partial
  read as intentional, not under-verified.

## Goals

1. Inventory favorites/groups and watch-rule evidence, supported mutations, and
   deliberate deferrals.
2. Validate focused CLI/Web/dashboard evidence for read paths, POST-backed
   mutations, and unsupported edit refusals.
3. Decide whether richer group/rule dimensions should remain deferred until a
   shared mutation contract exists.
4. Tighten surface-matrix wording to name the final boundary.
5. Close the phase with exact non-claims.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Red Wings goals | passed; see `RED-WINGS-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Favorites/watch evidence gate | passed; focused CLI/Web evidence supports deliberate narrow partial, see `pulses/pulse-02.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused favorites/watch tests across CLI and Web.
- No live network dependency in tests.
