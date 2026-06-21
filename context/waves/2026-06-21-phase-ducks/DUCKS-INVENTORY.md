# Phase Ducks Inventory

## Purpose

Inventory the Favorites/watch route rows before converting their plain partial
wording into explicit partial-by-design wording.

## Current Surface

| Area | Evidence | Ducks posture |
|---|---|---|
| Favorites HTML read | `GET /favorites` | Keep selected group membership through `FavoritesView`; `?group=` remains read-only. |
| Favorites JSON read | `GET /api/v1/favorites` | Keep stable `favorites.v1` read payload with optional named group selection. |
| Favorites mutations | `POST /favorites/add`, `POST /favorites/remove`, JSON twins | Keep canonical `Favorites` add/remove through `FavoriteMutationIntent` and `MutationResultView`. |
| Watchlist HTML read | `GET /watchlist` | Keep `WatchlistView` notes and rule controls. |
| Watchlist JSON read | `GET /api/v1/watchlist` | Keep stable `watchlist.v1` read payload with reason metadata. |
| Watch rules read | `GET /api/v1/watch-rules` | Keep default and persisted rules through `WatchRulesView`. |
| Watch rule mutations | `POST /watch-rules/create`, `POST /watch-rules/set-enabled`, `POST /watch-rules/delete`, JSON toggle | Keep player-rule create/toggle/delete through `WatchRuleMutationIntent`. |

## Risks to Avoid

- Rewording route rows as full group/watch-rule parity.
- Mutating favorites, groups, or watch rules through GET navigation.
- Promoting arbitrary group create/rename/delete/member editing.
- Promoting arbitrary team/deployment watch-rule editing.
- Allowing dashboard/TUI commands to reinterpret unsupported edits as narrower
  player-rule mutations.
- Claiming favorite stat-line reads fetch live data or create cache state on
  GET.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused Favorites/watch route tests cover
   scoped route claims and Red Wings deferrals.
3. Matrix wording. Convert route rows to explicit partial-by-design wording if
   evidence passes.
4. Closeout. Record final claims and non-claims.
