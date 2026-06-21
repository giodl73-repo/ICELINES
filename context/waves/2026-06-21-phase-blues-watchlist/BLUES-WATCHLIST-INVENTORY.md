# Phase Blues Watchlist Inventory

## Purpose

Inventory Watchlist read route rows before tightening their route wording.

## Current Surface

| Area | Evidence | Blues Watchlist posture |
|---|---|---|
| HTML Watchlist | `GET /watchlist` | Keep read-only `WatchlistView` projection, watch-note metadata, recent alerts, and scoped player-rule create/toggle/delete form affordances. |
| JSON Watchlist | `GET /api/v1/watchlist` | Keep stable `watchlist.v1` payload with member counts, watch-note reason/source/update metadata, and recent alert rows. |
| Mutation boundary | POST-backed rule routes | Keep rule mutations on create/toggle/delete POST routes; do not imply GET mutation or JSON rule mutation from the read twin. |

## Risks to Avoid

- Claiming GET navigation mutates Watchlist or rule state.
- Claiming arbitrary team/deployment rule editing.
- Claiming JSON rule mutation through `/api/v1/watchlist`.
- Changing runtime behavior while performing a wording gate.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused Watchlist tests cover HTML shell,
   watch-note metadata, recent alerts, scoped rule forms, and JSON payload
   shape.
3. Matrix wording. Result: passed; Watchlist rows now carry scoped read-only
   wording.
4. Closeout. Result: passed; Phase Blues Watchlist is closed with final
   route-row claims and non-claims recorded.
