# Phase Avalanche Watch Toggle

## Scope

Plan and execute the watch-rule toggle route-row wording gate. The wave does not
add runtime behavior; it records existing persisted player-rule toggle evidence.

## Entry Posture

- JSON and HTML toggle routes resolve through `WatchRuleMutationIntent`.
- Toggle scope is persisted player watch rules.
- The stored `enabled` flag is updated.
- JSON returns `MutationResultView`; HTML redirects back to `/watchlist`.

## Goals

1. Inventory watch-rule toggle route rows and evidence.
2. Validate focused watch-rule toggle route evidence.
3. Tighten route-row wording to persisted player-rule scope, intent, enabled
   flag update, result/redirect, and non-editing claims.
4. Preserve exact non-claims around default-rule mutation, arbitrary
   team/deployment editing, event firing, and runtime behavior changes.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Avalanche Watch Toggle goals | passed; see `AVALANCHE-WATCH-TOGGLE-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Watch-rule toggle route evidence gate | passed; focused route tests support scoped wording, see `pulses/pulse-02.md` |
| 03 | Watch-rule toggle route wording gate | passed; rows now carry scoped persisted-rule wording, see `pulses/pulse-03.md` |
| 04 | Close Phase Avalanche Watch Toggle | passed; final scoped claims and non-claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused watch-rule toggle route tests.
- No runtime behavior changes are part of this gate.

## Closeout

Phase Avalanche Watch Toggle is closed. Watch-rule toggle rows now record
persisted player-rule `enabled` updates through `WatchRuleMutationIntent`, JSON
`MutationResultView`, HTML `/watchlist` redirects, and default-rule,
team/deployment editing, and event-firing non-claims.

The claim remains bounded. The rows do not promote default-rule mutation,
arbitrary team/deployment editing, event firing, or runtime behavior changes.
