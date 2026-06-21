# Phase Blues Watchlist - Watchlist read route wording gate

> Phase Blues Watchlist records the Watchlist HTML/JSON read route rows with
> precise read-only note, alert, and rule-control boundaries.

**Created:** 2026-06-21
**Status:** Closed - Phase Blues Watchlist complete

---

## Frame

The Watchlist HTML route and JSON twin already project `Watchlist` group members
and notes through `WatchlistView`. Phase Blues Watchlist tightens the route
matrix so those rows name read-only group projection, watch-note metadata,
recent alerts, scoped player-rule form affordances, and non-claims around GET
mutation and arbitrary team/deployment rule editing.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Blues Watchlist Goal 1 - Route inventory** | Watchlist read rows should name notes, alerts, counts, and rule-control boundaries. | A wave inventory names route rows, evidence, and non-claims. |
| 2 | **Blues Watchlist Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused watchlist route tests pass. |
| 3 | **Blues Watchlist Goal 3 - Scoped route wording** | Existing rows are accurate but too terse beside watch-rule mutation rows. | Route rows name read-only Watchlist projection, metadata, alerts, scoped forms, and non-claims. |
| 4 | **Blues Watchlist Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not change Watchlist runtime behavior.
- Do not make GET navigation mutating.
- Do not add arbitrary team/deployment rule editing.
- Do not expose rule mutation through the JSON read twin.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused watchlist tests passed.
3. **Pulse 03 - Matrix wording.** Result: route rows now carry scoped read
   wording.
4. **Pulse 04 - Closeout.** Result: Phase Blues Watchlist is closed with final
   route-row claims and non-claims recorded.

---

## Closeout

Phase Blues Watchlist closed the Watchlist read route wording gate. The rows now
record read-only Watchlist group projection, watch-note metadata, recent alert
rows, scoped HTML player-rule controls, stable `watchlist.v1` JSON shape, and
GET-mutation/team-deployment editing non-claims.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused watchlist route tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
