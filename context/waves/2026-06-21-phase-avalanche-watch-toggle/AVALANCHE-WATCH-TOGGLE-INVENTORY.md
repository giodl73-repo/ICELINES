# Phase Avalanche Watch Toggle Inventory

## Purpose

Inventory watch-rule toggle route rows before tightening their route wording.

## Current Surface

| Area | Evidence | Avalanche Watch Toggle posture |
|---|---|---|
| JSON toggle | `POST /api/v1/watch-rules/set-enabled` | Keep persisted player-rule `WatchRuleMutationIntent::set_enabled`, stored `enabled` update, and `MutationResultView`. |
| HTML toggle | `POST /watch-rules/set-enabled` | Keep `/watchlist` form twin, shared intent, stored `enabled` update, and `/watchlist` redirect. |

## Risks to Avoid

- Claiming default-rule mutation.
- Claiming arbitrary team/deployment rule editing.
- Claiming event firing from toggle routes.
- Changing runtime behavior while performing a wording gate.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused watch-rule toggle tests cover JSON
   result, HTML form rendering, HTML redirect, and persisted enabled updates.
3. Matrix wording. Result: passed; toggle rows now carry scoped persisted-rule
   wording.
4. Closeout. Result: passed; Phase Avalanche Watch Toggle is closed with final
   route-row claims and non-claims recorded.
