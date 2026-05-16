# BENCH Review - Import the Rosters

## Findings

- The wave is fully fixture-testable with temp CSV files and in-memory FantasyDb.
- The highest-risk bugs are dry-run mutating state, header drift being accepted
  silently, duplicate ownership overwriting another team, and diacritic rows
  failing normalization.
- System tests should isolate `HOME`/`USERPROFILE` so no real user fantasy DB is
  mutated.

## Required Pulse Constraints

- Add L0 tests for import ViewModel counts and diagnostics.
- Add L1 tests for parser/importer success, dry-run no mutation, missing columns,
  duplicate ownership, diacritics, and unresolved rows.
- Add L2 CLI tests only with isolated temp home and fixture CSV.
- No live Yahoo exports or network calls in tests.
