---
wave: shape-the-rosters
pulse: 02
date: 2026-05-16
status: complete
governing_roles:
  - forge
  - bench
---

# Pulse 02 - Core Roster Shape Contract

## Goal

Add pure `icelines-core` roster-shape types and validation logic.

## Owned Scope

- Define roster shape rules and validation result/ViewModel types in core.
- Support legal, underfilled, overfilled, unknown-player, and goalie/skater
  mismatch cases.
- Keep scoring math unchanged.
- Add L0 tests alongside the core module.

## Non-goals

- No SQLite migration.
- No CLI/web/TUI rendering.
- No Yahoo CSV parsing changes.

## Gates

- [x] `cargo test -p icelines-core roster_shape --quiet`
- [x] `cargo fmt --check`
- [x] `git diff --check`

## Stop Conditions

- Stop if validation needs data not available from existing player views/import
  rows without inventing a new source of truth.

## Result

Added `icelines-core/src/roster_shape.rs` and exported the pure roster-shape
contract from `icelines-core`. The contract validates `RosterShapePlayerInput`
canonical position rows against `RosterShape` rules and returns a
`RosterShapeValidationView` with slot rows, summary counts, and player issues.

Focused L0 coverage includes legal rosters, missing required slots, position and
total over-cap rosters, unknown players, duplicate rows without double-counting,
and goalie/skater mismatch through an ineligible shape.
