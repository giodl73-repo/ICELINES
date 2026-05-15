---
wave: guard-the-operations
pulse: 04
date: 2026-05-15
status: complete
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

- [x] `cargo fmt --check`
- [x] `cargo test -p icelines-core --quiet`
- [x] `cargo test -p icelines-cli --quiet`
- [x] `cargo test -p icelines-web --quiet`
- [x] `cargo clippy -p icelines-core --no-deps -- -D warnings`
- [x] `cargo clippy -p icelines-cli --no-deps -- -D warnings`
- [x] `cargo clippy -p icelines-web --no-deps -- -D warnings`

## Result

Richer arbitrary team/deployment editing remains deferred because the shared
`WatchRuleMutationIntent` only carries create/enable/disable/delete by rule id
and has no validated team/deployment fields. The pulse fenced the unsupported
phrases instead: TUI and web dashboard commands now reject team/deployment edit
attempts rather than reinterpreting them as player rules, while player-rule
create/toggle/delete paths remain POST-backed or TUI-command-backed.
