---
wave: guard-the-operations
pulse: 05
date: 2026-05-15
status: planned
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

- [ ] `cargo fmt --check`
- [ ] `cargo test -p icelines-core --quiet`
- [ ] `cargo test -p icelines-cli --quiet`
- [ ] `cargo test -p icelines-web --quiet`
- [ ] clippy for touched crates with `--no-deps -- -D warnings`
