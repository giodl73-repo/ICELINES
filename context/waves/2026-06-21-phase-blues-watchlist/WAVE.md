# Phase Blues Watchlist

## Scope

Plan and execute the Watchlist read route-row wording gate. The wave does not
add runtime behavior; it records existing read-only Watchlist HTML and JSON
evidence.

## Entry Posture

- `/watchlist` reads `Watchlist` group members, watch notes, recent alert
  events, and persisted player-rule rows.
- `/api/v1/watchlist` returns a stable `watchlist.v1` payload with member
  counts, note metadata, and recent alerts.
- Rule mutations remain POST-backed through create/toggle/delete routes.
- Arbitrary team/deployment rule editing remains outside the shared contract.

## Goals

1. Inventory the Watchlist read route rows and evidence.
2. Validate focused Watchlist route evidence.
3. Tighten route-row wording to scoped read-only, note metadata, alert,
   player-rule form affordance, JSON shape, and non-mutation claims.
4. Preserve exact non-claims around GET mutation, JSON rule mutation, arbitrary
   team/deployment editing, and runtime behavior changes.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Blues Watchlist goals | passed; see `BLUES-WATCHLIST-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Watchlist read route evidence gate | passed; focused route tests support scoped wording, see `pulses/pulse-02.md` |
| 03 | Watchlist read route wording gate | passed; rows now carry scoped read wording, see `pulses/pulse-03.md` |
| 04 | Close Phase Blues Watchlist | passed; final scoped claims and non-claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused Watchlist route tests.
- No runtime behavior changes are part of this gate.

## Closeout

Phase Blues Watchlist is closed. Watchlist rows now record read-only group
projection through `WatchlistView`, watch-note reason/source/update metadata,
recent alert rows, scoped HTML player-rule create/toggle/delete forms, stable
`watchlist.v1` JSON, and non-claims around GET mutation, JSON rule mutation, and
arbitrary team/deployment editing.

The claim remains bounded. The rows do not promote runtime changes, GET-backed
mutation, arbitrary team/deployment rule editing, or JSON rule mutation.
