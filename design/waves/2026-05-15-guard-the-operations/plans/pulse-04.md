---
wave: guard-the-operations
pulse: 04
date: 2026-05-15
status: planned
governing_roles:
  - keel
  - wire
  - bench
  - forge
  - glass
---

# Pulse 04 - Watch-Rule Editor Parity

## Goal

Deepen watch-rule editing where the existing ViewModels and mutation intents
already support it. Player-rule create/toggle/delete exists; this pulse decides
whether richer team/deployment dimensions can be edited safely in TUI and web or
must stay deferred.

## Owned Scope

- Inspect `WatchRulesView`, `WatchlistView`, `WatchRuleMutationIntent`, TUI
  cmdbar parsing, and web watchlist forms.
- Add editor support only for dimensions already represented by shared contracts.
- Preserve existing watch notes and fired-alert history.
- Keep all mutations POST-backed or TUI-command-backed through typed intents.

## Non-goals

- No full query-builder rewrite.
- No watch-rule schema migration unless it is narrowly scoped and tested.
- No surface-local rule semantics.

## Gates

- [ ] `cargo fmt --check`
- [ ] `cargo test -p icelines-core --quiet`
- [ ] `cargo test -p icelines-cli --quiet`
- [ ] `cargo test -p icelines-web --quiet`
- [ ] `cargo clippy -p icelines-core --no-deps -- -D warnings`
- [ ] `cargo clippy -p icelines-cli --no-deps -- -D warnings`
- [ ] `cargo clippy -p icelines-web --no-deps -- -D warnings`
