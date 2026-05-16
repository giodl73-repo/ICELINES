---
wave: shape-the-rosters
pulse: 03
date: 2026-05-16
status: complete
governing_roles:
  - forge
  - wire
  - bench
---

# Pulse 03 - FantasyDb Shape Persistence and Import Validation

## Goal

Persist league roster-shape settings and validate imports/manual roster state
through the core contract.

## Owned Scope

- Add a safe FantasyDb migration for roster-shape preset/config.
- Wire default shape behavior for existing leagues.
- Validate Yahoo import dry-run/apply output without treating position hints as
  authoritative player truth.
- Add fetch-layer tests for migration, defaults, import warnings, and no-mutation
  dry runs.

## Non-goals

- No CLI shape commands yet beyond helpers needed for tests.
- No remote Yahoo API.
- No live network tests.

## Gates

- [x] `cargo test -p icelines-fetch roster_shape --quiet`
- [x] `cargo test -p icelines-fetch fantasy_import --quiet`
- [x] `cargo fmt --check`
- [x] `git diff --check`

## Stop Conditions

- Stop if migration would make existing leagues unreadable or silently discard
  existing roster rows.

## Result

Completed in Pulse 03:

- Added the built-in roster-shape resolver around the core `RosterShape`
  contract.
- Added a safe `fl_leagues.roster_shape` migration/default and snapshot
  propagation in FantasyDb.
- Added fetch-layer roster-shape validation helpers for persisted team rosters.
- Wired Yahoo CSV import validation through canonical player positions when
  available, while keeping CSV position hints non-authoritative.
- Added L1 coverage for default persistence, unknown preset rejection, persisted
  roster validation, import dry-run validation warnings, and existing-roster
  unknown-player diagnostics.
