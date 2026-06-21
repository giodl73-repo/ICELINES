# Phase Red Wings Pulse 03 - Matrix Wording

**Date:** 2026-06-21
**Result:** Passed

## Work Completed

- Updated `design/specs/surface-parity.md` to make
  Favorites/watch/watch-rules partial by design rather than vaguely partial.
- Recorded supported paths: read-only named group views, POST-backed canonical
  Favorites mutations, watchlist/watch-rule reads, and POST-backed player-rule
  create/toggle/delete.
- Preserved explicit blockers for richer group and arbitrary team/deployment
  watch-rule editing.

## Decision

Do not promote richer group/rule editing in this phase. The current shared
contracts support the narrow paths. Future promotion needs shared mutation
contracts with validated fields for group edits and team/deployment watch-rule
dimensions.

## Validation

- `cargo test -p icelines-web --test l1_router favorites`
- `cargo test -p icelines-web --test l1_router watch`
- `cargo test -p icelines-cli favorites`
- `cargo test -p icelines-cli watch`
- `git diff --check`

## Next Pulse

Pulse 04 closes Phase Red Wings and records the deliberate partial as final.
