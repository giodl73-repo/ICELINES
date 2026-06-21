# Phase Predators Snapshots Inventory

## Purpose

Inventory admin snapshot mutation route rows before tightening their route
wording.

## Current Surface

| Area | Evidence | Predators Snapshots posture |
|---|---|---|
| JSON activate | `POST /api/v1/admin/snapshots/activate` | Keep sealed-only `SnapshotMutationIntent::activate`, active snapshot pointer update, and `MutationResultView`. |
| HTML activate | `POST /admin/snapshots/activate` | Keep activate controls limited to sealed inactive rows, shared intent, `/admin` redirect, and no creation/sealing claim. |
| JSON delete | `POST /api/v1/admin/snapshots/delete` | Keep inactive-only `SnapshotMutationIntent::delete`, active-snapshot rejection, and `MutationResultView`. |
| HTML delete | `POST /admin/snapshots/delete` | Keep delete controls limited to inactive rows, shared intent, `/admin` redirect, and no broad maintenance claim. |

## Risks to Avoid

- Claiming browser snapshot creation, sealing, or broad maintenance.
- Claiming active snapshot deletion.
- Bypassing shared mutation intents in wording.
- Changing runtime behavior while performing a wording gate.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused admin snapshot tests cover JSON and
   HTML activation/deletion plus active snapshot rejection.
3. Matrix wording. Result: passed; snapshot mutation rows now carry scoped
   sealed/inactive wording.
4. Closeout. Result: passed; Phase Predators Snapshots is closed with final
   route-row claims and non-claims recorded.
