# Phase Canucks Watch Create Inventory

## Purpose

Inventory the watch-rule create route row before tightening its route wording.

## Current Surface

| Area | Evidence | Canucks Watch Create posture |
|---|---|---|
| HTML create | `POST /watch-rules/create` | Keep persisted player-rule `WatchRuleMutationIntent::create`, promotion/availability trigger payloads, enabled state, safe redirects, and unsupported team/deployment edit rejection. |
| Dashboard handoff | dashboard command route | Keep command handoff returning to dashboard workspace without broadening rule-edit scope. |

## Risks to Avoid

- Claiming arbitrary team/deployment rule editing.
- Allowing external or protocol-relative redirect targets.
- Claiming default-rule creation from the form route.
- Changing runtime behavior while performing a wording gate.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused watch-rule create tests cover
   persisted row creation, safe return handling, unsafe return rejection, and
   dashboard handoff behavior.
3. Matrix wording. Result: passed; create row now carries scoped persisted-rule
   wording.
4. Closeout. Result: passed; Phase Canucks Watch Create is closed with final
   route-row claims and non-claims recorded.
