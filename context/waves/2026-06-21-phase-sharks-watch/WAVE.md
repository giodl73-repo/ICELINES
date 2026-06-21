# Phase Sharks Watch

## Scope

Plan and execute the watch-rule delete route-row wording gate. The wave does not
add runtime behavior; it records existing POST-backed persisted-rule deletion
evidence.

## Entry Posture

- Watch-rule delete is exposed as an HTML form mutation.
- The route resolves `WatchRuleMutationIntent::delete` from the submitted
  persisted rule id.
- The route rejects blank/unknown ids and removes only the matching
  `watch_rules` row.
- Successful deletion redirects back to `/watchlist`.

## Goals

1. Inventory the watch-rule delete route row and evidence.
2. Validate focused watch-rule delete route evidence.
3. Tighten route-row wording to scoped form-only, shared-intent, id validation,
   single-row deletion, redirect, and destructive-boundary claims.
4. Preserve exact non-claims around JSON delete, bulk delete, arbitrary
   team/deployment editing, and runtime behavior changes.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Sharks Watch goals | passed; see `SHARKS-WATCH-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Watch-rule delete route evidence gate | passed; focused route test supports scoped wording, see `pulses/pulse-02.md` |
| 03 | Watch-rule delete route wording gate | passed; row now carries scoped destructive-boundary wording, see `pulses/pulse-03.md` |
| 04 | Close Phase Sharks Watch | passed; final scoped claims and non-claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused watch-rule delete route tests.
- No runtime behavior changes are part of this gate.

## Closeout

Phase Sharks Watch is closed. The watch-rule delete row now records form-only
persisted player watch-rule deletion, `WatchRuleMutationIntent::delete`, blank
and unknown id rejection, single-row removal from `watch_rules`, `/watchlist`
redirect behavior, and arbitrary destructive rule-dimension non-claims.

The claim remains bounded. The row does not promote runtime changes, JSON
delete, bulk delete, or arbitrary team/deployment rule editing.
