# BENCH Review - Match the Week

## Findings

- The wave is fixture-testable because FantasyDb has in-memory tests and Score
  the Day can build missing/final/unfinalized daily results without live data.
- The riskiest regressions are silently treating missing schedule/cache as
  successful zeroes and failing to test tie/bye behavior.

## Required Pulse Constraints

- Add L0 tests for weekly ViewModel ordering, win/loss/tie/bye outcomes, and
  source-state aggregation.
- Add L1 tests for FantasyDb schedule persistence and weekly builder behavior.
- Add surface tests only after surfaces project the shared ViewModel.
- No live NHL or Yahoo tests.
