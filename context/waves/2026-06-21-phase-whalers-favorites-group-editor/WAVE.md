# Phase Whalers Favorites Group Editor

## Mission

Add POST-backed web and JSON mutation routes for named Favorites group editing
while preserving GET-read-only navigation and canonical Favorites protections.

## Scope

- Create, rename, and delete local SQLite groups from `/favorites`.
- Add/remove members for the selected group from `/favorites?group=<name>`.
- Return `MutationResultView` from JSON mutation routes.
- Reject rename/delete for canonical `Favorites`.
- Keep dashboard command group edits deferred to avoid GET mutation semantics.

## Pulses

| Pulse | Status | Evidence |
|---|---|---|
| 01 | passed | Plan and inventory created. |
| 02 | passed | Group mutation helper and routes implemented. |
| 03 | passed | Browser template controls and router tests added. |
| 04 | passed | Surface matrix, command docs, and closeout updated. |

## Closeout

Phase Whalers is complete. Named Favorites group editing is now available through
POST-backed browser and JSON routes, with GET and dashboard command boundaries
unchanged.
