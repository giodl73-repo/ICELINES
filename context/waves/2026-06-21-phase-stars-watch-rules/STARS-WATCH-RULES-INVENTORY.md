# Phase Stars Watch Rules Inventory

## Purpose

Inventory the watch-rules JSON read row before tightening its route wording.

## Current Surface

| Area | Evidence | Stars Watch Rules posture |
|---|---|---|
| JSON catalog | `GET /api/v1/watch-rules` | Keep read-only `WatchRulesView` wording with five default rule templates, persisted player-rule merge, enabled state, trigger payloads, unsupported-source markers, and typed bad-config errors. |
| Persisted metadata | `watch_rules` plus `watch_rule_events` | Keep persisted `last_fired` wording for read projection only. |
| Mutation boundary | POST routes | Keep set-enabled/create/delete mutations POST-backed and outside this read row. |

## Risks to Avoid

- Claiming GET mutation.
- Claiming arbitrary team/deployment rule editing.
- Claiming watch-rule event firing from read navigation.
- Changing runtime behavior while performing a wording gate.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused watch-rules JSON tests cover default
   catalog shape, typed config errors, and persisted rule metadata.
3. Matrix wording. Result: passed; watch-rules JSON row now carries scoped read
   catalog wording.
4. Closeout. Result: passed; Phase Stars Watch Rules is closed with final
   route-row claims and non-claims recorded.
