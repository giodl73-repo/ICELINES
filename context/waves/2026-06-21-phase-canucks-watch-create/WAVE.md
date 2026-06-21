# Phase Canucks Watch Create

## Scope

Plan and execute the watch-rule create route-row wording gate. The wave does not
add runtime behavior; it records existing persisted player-rule creation
evidence.

## Entry Posture

- Watch-rule create is exposed as an HTML form mutation.
- The route resolves `WatchRuleMutationIntent::create`.
- The route stores promotion/availability trigger payloads with enabled state.
- Successful creation redirects to a safe caller target or `/watchlist`.
- Unsupported team/deployment edits remain rejected.

## Goals

1. Inventory the watch-rule create route row and evidence.
2. Validate focused watch-rule create and safe-return route evidence.
3. Tighten route-row wording to persisted player-rule scope, shared intent,
   trigger payload, enabled state, safe redirect, dashboard handoff, and
   unsupported-edit claims.
4. Preserve exact non-claims around arbitrary team/deployment editing, unsafe
   redirects, default-rule creation, and runtime behavior changes.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Canucks Watch Create goals | passed; see `CANUCKS-WATCH-CREATE-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Watch-rule create route evidence gate | passed; focused route tests support scoped wording, see `pulses/pulse-02.md` |
| 03 | Watch-rule create route wording gate | passed; row now carries scoped persisted-rule wording, see `pulses/pulse-03.md` |
| 04 | Close Phase Canucks Watch Create | passed; final scoped claims and non-claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused watch-rule create route tests.
- No runtime behavior changes are part of this gate.

## Closeout

Phase Canucks Watch Create is closed. The watch-rule create row now records
persisted player-rule creation through `WatchRuleMutationIntent`, submitted
player identifier resolution, promotion/availability trigger payloads, enabled
state, safe `return_to` redirects, dashboard command handoff behavior, and
unsupported team/deployment edit rejection.

The claim remains bounded. The row does not promote arbitrary team/deployment
editing, unsafe redirects, default-rule creation, or runtime behavior changes.
