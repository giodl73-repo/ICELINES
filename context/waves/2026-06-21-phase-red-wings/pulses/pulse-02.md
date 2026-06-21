# Phase Red Wings Pulse 02 - Favorites/Watch Evidence Gate

**Date:** 2026-06-21
**Result:** Passed

## Work Completed

- Ran focused Web favorites evidence for read-only named groups, canonical
  favorites mutations, and GET cache-read boundaries.
- Ran focused Web watch evidence for watchlist reads, watch-rule contract
  reads, POST-backed create/toggle/delete paths, safe return targets, and
  deployment-watch refusal.
- Ran focused CLI/TUI favorites evidence covering favorites commands, TUI
  affordances, persistence, and empty/error states.
- Ran focused CLI/TUI watch evidence covering watch-rule persistence,
  enable/disable behavior, alert history, command-bar handoff, and watchlist
  rendering.

## Validation

- `cargo test -p icelines-web --test l1_router favorites`
- `cargo test -p icelines-web --test l1_router watch`
- `cargo test -p icelines-cli favorites`
- `cargo test -p icelines-cli watch`
- `git diff -- Cargo.lock`

## Decision

Keep Favorites/watch/watch-rules partial by design. The supported boundary is:
read-only named group views, POST-backed canonical `Favorites` add/remove,
watchlist/watch-rule reads, and POST-backed player-rule create/toggle/delete.
Richer group create/rename/delete/member edits and arbitrary team/deployment
watch-rule editing remain deferred until shared mutation contracts carry
validated fields for those dimensions.

## Residual Risk

The focused filters are broad enough to include adjacent favorites/watch tests.
That is acceptable for this gate because these surfaces share persistence and
dashboard command paths.

## Next Pulse

Pulse 03 tightens the surface matrix so the partial is clearly a deliberate
narrow-contract boundary.
