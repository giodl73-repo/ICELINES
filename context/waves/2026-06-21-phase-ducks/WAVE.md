# Phase Ducks

## Scope

Plan and execute the Favorites/watch route-row wording gate after Phase Red
Wings. The wave does not reopen the Red Wings product boundary; it records the
individual Favorites, Watchlist, and Watch rule route rows as scoped partials by
design.

## Entry Posture

- Phase Red Wings closed the Favorites/watch/watch-rules boundary gate.
- Feature rows already say favorites/groups and watch rules are partial by
  design.
- The individual route rows still begin with plain `partial -` wording, which
  makes intentional deferrals harder to distinguish from unresolved route drift.
- Read-only named group views, POST-backed canonical Favorites add/remove, and
  player watch-rule create/toggle/delete are supported and tested.
- Richer group create/rename/delete/member editing and arbitrary
  team/deployment watch-rule editing remain deferred until shared mutation
  contracts exist.

## Goals

1. Inventory the Favorites/watch route rows and the Red Wings evidence they
   depend on.
2. Validate focused Favorites/watch route evidence for the scoped route claims.
3. Tighten the Favorites, Watchlist, and Watch rule route rows so their partial
   status is explicit and by design.
4. Preserve exact non-claims around GET mutations, arbitrary group editing,
   arbitrary team/deployment watch-rule editing, and unsafe command
   reinterpretation.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Ducks goals | passed; see `DUCKS-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Favorites/watch route evidence gate | passed; focused route evidence supports scoped partial-by-design wording, see `pulses/pulse-02.md` |
| 03 | Favorites/watch route wording gate | passed; route rows now read as partial by design, see `pulses/pulse-03.md` |
| 04 | Close Phase Ducks | passed; phase closed with route rows recorded as scoped partials by design, see `pulses/pulse-04.md` |

## Closeout

Phase Ducks is closed. The Favorites/watch feature rows and route rows now carry
the same boundary: read-only named group views, POST-backed canonical
`Favorites` add/remove, watchlist/watch-rule reads, and player-rule
create/toggle/delete are supported and tested as a scoped partial by design.

Richer group create/rename/delete/member editing and arbitrary team/deployment
watch-rule editing remain deferred until shared mutation contracts carry
validated fields. Browser/dashboard/TUI commands must continue rejecting
unsupported edits rather than mutating through GET or reinterpreting the request
as a narrower player-rule action.

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused Favorites/watch route tests.
- No live network dependency in tests.
