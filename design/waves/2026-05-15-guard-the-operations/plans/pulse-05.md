---
wave: guard-the-operations
pulse: 05
date: 2026-05-15
status: complete
governing_roles:
  - keel
  - wire
  - bench
  - forge
  - glass
---

# Pulse 05 - Favorites and Groups Parity

## Goal

Close the favorites/groups partial where the shared contracts already support
it. Favorites add/remove exists; this pulse inventories group-management UX
gaps and implements the smallest safe parity step across web/dashboard/TUI.

## Owned Scope

- Inspect `FavoritesView`, `FavoriteMutationIntent`, `GroupDb`, TUI favorites
  affordances, and web favorites/dashboard side-pane routes.
- Add group selection/create/remove affordances only if existing identity and
  persistence contracts can support them without ambiguity.
- Keep dashboard side panes honest: navigation/read state via GET, mutations via
  existing POST routes/intents.
- Update docs/parity notes for any intentionally handoff-only behavior.

## Non-goals

- No replacement of `GroupDb`.
- No schema migration unless directly required and fully tested.
- No favorite/watch mutation through dashboard GET parameters.

## Gates

- [x] `cargo fmt --check`
- [x] `cargo test -p icelines-core --quiet`
- [x] `cargo test -p icelines-cli --quiet`
- [x] `cargo test -p icelines-web --quiet`
- [x] clippy for touched crates with `--no-deps -- -D warnings`

## Result

Web `/favorites` now supports read-only selection of any SQLite group through
`?group=<name>`, while POST-backed add/remove controls remain limited to the
canonical `Favorites` group. The dashboard command bar opens named group reads
and explicitly rejects group create/delete/rename/member edits instead of
turning them into GET mutations. TUI `/fav add/remove` now accepts team
abbreviations, matching the CLI group identity contract.
