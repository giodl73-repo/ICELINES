# Phase Sharks Watch Inventory

## Purpose

Inventory the watch-rule delete route row before tightening its route wording.

## Current Surface

| Area | Evidence | Sharks Watch posture |
|---|---|---|
| HTML delete | `POST /watch-rules/delete` | Keep form-only persisted player rule deletion wording with `WatchRuleMutationIntent::delete`, blank/unknown id rejection, single-row `watch_rules` removal, and `/watchlist` redirect. |
| Create/toggle siblings | `POST /watch-rules/create`, `POST /watch-rules/set-enabled`, `POST /api/v1/watch-rules/set-enabled` | Use as surrounding context for shared intent wording without expanding delete semantics. |

## Risks to Avoid

- Claiming JSON delete or bulk delete support.
- Expanding destructive behavior beyond persisted player rule ids.
- Claiming arbitrary team/deployment rule editing.
- Changing runtime behavior while performing a wording gate.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused watch-rule delete route test covers
   redirect and row removal.
3. Matrix wording. Result: passed; delete row now carries scoped
   destructive-boundary wording.
4. Closeout. Result: passed; Phase Sharks Watch is closed with final route-row
   claims and non-claims recorded.
