# Phase Ducks Pulse 03 - Favorites/watch Route Wording Gate

**Date:** 2026-06-21
**Result:** Passed

## Work Completed

- Converted Favorites read/mutation route rows from plain `partial -` wording
  to `partial by design`.
- Converted Watchlist and Watch rule read/mutation route rows to matching
  `partial by design` wording.
- Preserved Red Wings non-claims: no GET-backed mutations, no arbitrary named
  group editing, no arbitrary team/deployment watch-rule editing, no unsafe
  dashboard/TUI command reinterpretation, and no live/cache-creating stat-line
  reads on GET.

## Validation

- `cargo test -p icelines-web --test l1_router favorites`
  - Result from Pulse 02: 10 passed, 0 failed, 156 filtered out.
- `cargo test -p icelines-web --test l1_router watch`
  - Result from Pulse 02: 18 passed, 0 failed, 148 filtered out.
- `cargo test -p icelines-cli watch`
  - Result from Pulse 02: 21 passed, 0 failed.
- `git diff --check`

## Next Pulse

Pulse 04 closes Phase Ducks with final route-row claims and non-claims.
