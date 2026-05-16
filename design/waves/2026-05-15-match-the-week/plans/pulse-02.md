---
wave: match-the-week
pulse: 02
date: 2026-05-15
status: complete
governing_roles:
  - pace
  - bench
  - forge
---

# Pulse 02 - Core Weekly Matchup ViewModel

## Goal

Add the pure core weekly matchup contract that all surfaces and builders will
render.

## Owned Scope

- Add a `FantasyMatchupWeekView` family under `icelines-core::view_model`.
- Model week range, league, scoring scheme, matchup rows, team totals, bye rows,
  outcome (`win`, `loss`, `tie`, `bye`, `pending`), source state, warnings, and
  completeness.
- Add input structs that can be built from daily team totals without I/O.
- Add L0 tests for ordering, win/loss/tie/bye behavior, missing schedule empty
  state, source-state propagation, and deterministic tie-breaks.

## Non-goals

- No SQLite reads/writes.
- No DataStore or cached boxscore reads.
- No CLI/web/TUI rendering.

## Gates

- [x] `cargo test -p icelines-core fantasy_matchup --quiet`
- [x] `cargo fmt --check`

## Result

Added `FantasyMatchupWeekView` and its input/row/outcome family in
`icelines-core::view_model::fantasy_matchup`. The pure ViewModel resolves
weekly matchup rows, team rankings, wins/losses/ties/byes/pending outcomes,
missing schedule empty state, and source-state completeness without any I/O.
