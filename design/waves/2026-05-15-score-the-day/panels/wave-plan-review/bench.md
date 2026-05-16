# BENCH Review - Score the Day

## Findings

- The wave is testable without live network because FantasyDb has in-memory
  tests and game-night schemas can be fixture-built.
- The riskiest regressions are zero-shaped success for missing cache and counting
  NHL live-game default zeros as real physical stats.

## Required Pulse Constraints

- Add L0 tests for core scoring and ordering.
- Add fixture-based data-path tests for missing cache, unfinalized game, skater
  row, goalie row, and team total ordering.
- Add surface tests only after surfaces project the shared ViewModel.
