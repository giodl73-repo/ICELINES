---
wave: score-the-day
pulse: 02
date: 2026-05-15
status: planned
governing_roles:
  - pace
  - bench
  - forge
---

# Pulse 02 - Core Daily-Delta ViewModel and Scoring Adapter

## Goal

Add the pure core contract for fantasy daily delta scoring.

## Owned Scope

- Add a shared daily-delta ViewModel in `icelines-core`.
- Add skater/goalie daily stat adapter logic that reuses `Scheme` weights.
- Define row ordering, team totals, player totals, warnings, and source-state
  semantics.
- Add L0 tests with manually calculated expected values.

## Non-goals

- No SQLite/cache reads.
- No CLI/web/TUI wiring.
- No live game fetch.

## Gates

- [ ] `cargo test -p icelines-core fantasy_daily --quiet`
- [ ] `cargo fmt --check`
