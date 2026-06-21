# Phase Predators Snapshots

## Scope

Plan and execute the admin snapshot mutation route-row wording gate. The wave
does not add runtime behavior; it records existing snapshot activation and
deletion evidence.

## Entry Posture

- Snapshot activate/delete routes resolve through `SnapshotMutationIntent`.
- Activation is scoped to sealed snapshots.
- Deletion is scoped to inactive snapshots and rejects the active snapshot.
- JSON returns `MutationResultView`; HTML forms redirect back to `/admin`.

## Goals

1. Inventory admin snapshot mutation route rows and evidence.
2. Validate focused admin snapshot mutation route evidence.
3. Tighten route-row wording to scoped sealed activation, inactive deletion,
   result/redirect, and guard claims.
4. Preserve exact non-claims around browser snapshot creation, sealing,
   arbitrary maintenance, and active snapshot deletion.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Predators Snapshots goals | passed; see `PREDATORS-SNAPSHOTS-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Admin snapshot mutation route evidence gate | passed; focused route tests support scoped wording, see `pulses/pulse-02.md` |
| 03 | Admin snapshot mutation route wording gate | passed; rows now carry scoped sealed/inactive wording, see `pulses/pulse-03.md` |
| 04 | Close Phase Predators Snapshots | passed; final scoped claims and non-claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused admin snapshot mutation route tests.
- No runtime behavior changes are part of this gate.

## Closeout

Phase Predators Snapshots is closed. Admin snapshot mutation rows now record
sealed-only activation, inactive-only deletion, shared `SnapshotMutationIntent`,
active-snapshot delete rejection, JSON `MutationResultView`, HTML redirects, and
browser-maintenance non-claims.

The claim remains bounded. The rows do not promote browser snapshot creation,
sealing, broad maintenance, or active snapshot deletion.
