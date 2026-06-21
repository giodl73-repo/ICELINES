# Phase Stars Watch Rules

## Scope

Plan and execute the watch-rules JSON route-row wording gate. The wave does not
add runtime behavior; it records existing read-only watch-rule catalog evidence.

## Entry Posture

- `GET /api/v1/watch-rules` builds `WatchRulesView`.
- The route includes five default rule templates and persisted player rules.
- Persisted rules carry enabled state, trigger JSON, unsupported sources, and
  last-fired metadata.
- Mutation remains POST-backed through set-enabled/create/delete paths.

## Goals

1. Inventory the watch-rules JSON route row and evidence.
2. Validate focused watch-rules JSON route evidence.
3. Tighten route-row wording to scoped default/persisted catalog, metadata,
   typed error, and non-mutation claims.
4. Preserve exact non-claims around GET mutation, arbitrary team/deployment
   editing, event firing, and runtime behavior changes.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Stars Watch Rules goals | passed; see `STARS-WATCH-RULES-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Watch-rules JSON route evidence gate | passed; focused route tests support scoped wording, see `pulses/pulse-02.md` |
| 03 | Watch-rules JSON route wording gate | passed; row now carries scoped read catalog wording, see `pulses/pulse-03.md` |
| 04 | Close Phase Stars Watch Rules | passed; final scoped claims and non-claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused watch-rules JSON route tests.
- No runtime behavior changes are part of this gate.

## Closeout

Phase Stars Watch Rules is closed. The watch-rules JSON row now records
read-only catalog behavior through `WatchRulesView`, five default rules,
persisted player-rule merge, enabled state, trigger payloads, unsupported-source
markers, `last_fired` metadata, typed config errors, and non-claims around GET
mutation, arbitrary team/deployment editing, and event firing.

The claim remains bounded. The row does not promote runtime changes,
GET-backed mutation, arbitrary team/deployment editing, or event firing from
read navigation.
