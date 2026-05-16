---
wave: import-the-rosters
pulse: 02
date: 2026-05-16
status: complete
governing_roles:
  - forge
  - bench
  - glass
---

# Pulse 02 - Shared Import Contract

## Goal

Add a pure shared fantasy roster import preview/result contract so every surface
reports the same import summary and row diagnostics.

## Owned Scope

- Add core ViewModel/result types for a roster import preview/apply result.
- Represent league name, dry-run/apply mode, team summaries, row diagnostics,
  warnings, source state, and empty/error states.
- Add L0 tests for summary counts, diagnostics, warnings, deterministic ordering,
  and dry-run/apply labels.
- Export the contract from `icelines-core` for fetch/CLI/web/TUI adapters.

## Non-goals

- No CSV file I/O in core.
- No SQLite writes.
- No CLI or web surface wiring.
- No new scoring/poach/fantasy math.

## Gates

- [x] `cargo fmt --check`
- [x] `cargo test -p icelines-core fantasy_import --quiet`

## Stop Conditions

- Stop if the contract needs renderer-specific fields such as ANSI, CSS classes,
  or HTML.
- Stop if row status cannot distinguish imported, skipped, unresolved,
  duplicate, and error outcomes.

## Result

Added `FantasyImportView` in core with dry-run/apply mode labels, summary
counts, deterministic team/player rows, row diagnostics, source state, warnings,
and empty-state handling. L0 tests cover counts, ordering, team summaries,
mode labels, warnings, and empty input.
