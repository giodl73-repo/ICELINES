---
wave: import-the-rosters
pulse: 03
date: 2026-05-16
status: complete
governing_roles:
  - tape
  - wire
  - forge
  - bench
---

# Pulse 03 - CSV Parser and FantasyDb Importer

## Goal

Implement the fixture-driven Yahoo roster CSV parser and FantasyDb import path
behind the shared import contract.

## Owned Scope

- Extend or add an `icelines-fetch` CSV parser for roster ownership with header
  aliases, BOM handling, flexible rows, and explicit missing-column diagnostics.
- Add a FantasyDb bulk preview/apply operation that can create/find teams, set the
  optional user team, and converge normalized roster rows idempotently.
- Preserve dry-run behavior by exercising the same parser/validation path without
  SQLite mutation.
- Add L1 tests using temp files and in-memory SQLite for successful import,
  dry-run no mutation, missing columns, diacritics, duplicate ownership, and
  unresolved/skipped rows.

## Non-goals

- No live Yahoo API or real user files.
- No stat import from Yahoo columns.
- No schedule, keeper, salary, or waiver import.
- No browser route.

## Gates

- [x] `cargo fmt --check`
- [x] `cargo test -p icelines-fetch fantasy_import --quiet`

## Stop Conditions

- Stop if duplicate roster ownership would be accepted silently.
- Stop if missing fantasy-team data would be mapped to a synthetic default team.
- Stop if parser errors do not identify the offending header or row.

## Result

Added `icelines_fetch::fantasy_import` with Yahoo roster CSV parsing,
header-alias validation, UTF-8 BOM stripping, dry-run/apply import over
FantasyDb, optional known-player validation, duplicate ownership diagnostics,
and L1 tests for success, no-mutation dry-run, missing columns, diacritics,
duplicates, unresolved rows, and skipped same-team duplicates.
