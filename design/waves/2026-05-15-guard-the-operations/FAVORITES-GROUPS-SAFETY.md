# Favorites and Groups Safety

Pulse 05 closes the safe favorites/groups slice without widening identity or
persistence contracts.

## Supported now

- Web `/favorites` still mutates only the canonical `Favorites` group through
  existing POST routes.
- Web `/favorites?group=<name>` and `/api/v1/favorites?group=<name>` can inspect
  any SQLite group through `FavoritesView`.
- The web dashboard command bar opens named group reads with
  `favorites group=<name>` or `group show <name>`.
- TUI cmdbar `/fav add/remove` now accepts NHL team abbreviations as team
  members, matching the CLI `icelines group add Favorites EDM` behavior.

## Fenced intentionally

Create, rename, delete, and arbitrary group membership edits are not added to web
dashboard GET state. Those edits remain on `icelines group ...` and the TUI
Groups screen until IceLines has a shared `GroupMutationIntent`-style contract.

This avoids inventing web-local group semantics, avoids schema changes, and keeps
dashboard navigation read-only.
