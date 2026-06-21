# Phase Red Wings Inventory

## Purpose

Inventory Favorites/watch/watch-rules before deciding whether the active
partial should remain a deliberate narrow contract or expand to richer
group/rule editing.

## Current Surface

| Item | Evidence | Red Wings posture |
|---|---|---|
| Favorites read | `/favorites`, `/api/v1/favorites` | Projects `FavoritesView`, supports read-only named groups, and must not create cache state on GET. |
| Favorites mutation | `/favorites/add`, `/favorites/remove`, JSON twins | POST-backed canonical `Favorites` add/remove through mutation views. |
| Dashboard groups | `favorites group=...`, `group show ...` | Opens read-only named group view; create/delete/rename/member edits remain rejected. |
| Watchlist read | `/watchlist`, `/api/v1/watchlist` | Projects `WatchlistView` with reason/event metadata. |
| Watch rules read | `/api/v1/watch-rules` | Projects `WatchRulesView` with default and persisted rules. |
| Watch rule mutation | `/watch-rules/create`, `/watch-rules/set-enabled`, `/watch-rules/delete`, JSON toggle | POST-backed player-rule create/toggle/delete through watch-rule mutation intents. |
| Unsupported rule edits | dashboard/TUI deployment and team-rule phrases | Must reject rather than silently creating player rules. |

## Promotion Blockers

- Do not mutate favorites, groups, or watch rules through GET navigation.
- Do not promote arbitrary group create/rename/delete/member editing until a
  shared group mutation contract exists.
- Do not promote arbitrary team/deployment watch-rule editing until the shared
  rule intent carries validated fields for those dimensions.
- Do not allow browser/dashboard commands to reinterpret unsupported edits as
  narrower player-rule mutations.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Run focused favorites/watch tests and record the supported
   read/mutation/refusal boundary. Result: passed; focused CLI/Web evidence
   supports the deliberate narrow partial.
3. Matrix wording. Tighten partial wording if evidence supports a deliberate
   narrow-contract claim.
4. Closeout. Record the final Favorites/watch/watch-rules decision.
