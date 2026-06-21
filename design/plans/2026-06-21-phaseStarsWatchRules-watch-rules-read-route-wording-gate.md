# Phase Stars Watch Rules - Watch-rules read route wording gate

> Phase Stars Watch Rules records the watch-rules JSON read route row with
> precise default/persisted rule catalog boundaries.

**Created:** 2026-06-21
**Status:** Closed - Phase Stars Watch Rules complete

---

## Frame

`GET /api/v1/watch-rules` already projects the watch-rule catalog through
`WatchRulesView`. Phase Stars Watch Rules tightens the route matrix so the row
names the five default rules, persisted player-rule merge, enabled state,
trigger payloads, unsupported-source markers, last-fired metadata, typed config
errors, and non-claims around mutation or arbitrary team/deployment editing.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Stars Watch Rules Goal 1 - Route inventory** | The watch-rules JSON row should name default/persisted catalog details and mutation boundaries. | A wave inventory names route row, evidence, and non-claims. |
| 2 | **Stars Watch Rules Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused watch-rules JSON tests pass. |
| 3 | **Stars Watch Rules Goal 3 - Scoped route wording** | Existing wording is accurate but too terse beside watchlist and mutation rows. | Route row names `WatchRulesView`, catalog fields, persisted metadata, errors, and non-claims. |
| 4 | **Stars Watch Rules Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not change watch-rules runtime behavior.
- Do not expose mutation through `GET /api/v1/watch-rules`.
- Do not add arbitrary team/deployment rule editing.
- Do not claim event firing from read navigation.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused watch-rules JSON tests passed.
3. **Pulse 03 - Matrix wording.** Result: route row now carries scoped read
   catalog wording.
4. **Pulse 04 - Closeout.** Result: Phase Stars Watch Rules is closed with
   final route-row claims and non-claims recorded.

---

## Closeout

Phase Stars Watch Rules closed the watch-rules JSON route wording gate. The row
now records read-only catalog behavior, default and persisted rules,
`WatchRulesView`, enabled state, trigger payloads, unsupported sources,
persisted `last_fired` metadata, typed config errors, and mutation/editing/event
non-claims.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused watch-rules JSON route tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
