---
wave: shape-the-rosters
pulse: 04
date: 2026-05-16
status: complete
governing_roles:
  - glass
  - bench
  - wire
---

# Pulse 04 - CLI, TUI, and Dashboard Validation Surfaces

## Goal

Expose roster-shape validation through user-visible surfaces without GET-backed
mutation.

## Owned Scope

- Add CLI commands for showing/setting roster shape and validating a league/team.
- Add TUI command-bar handoffs for the CLI validation/setup commands.
- Add web/dashboard read-only validation display or explicit mutation deferral.
- Add L2/system or focused surface tests for command output and no-GET mutation.

## Non-goals

- No browser-side shape mutation unless it is POST-backed and already has a
  shared mutation contract.
- No lineup optimizer.
- No scoring changes.

## Gates

- [x] `cargo test -p icelines-cli roster_shape --quiet`
- [x] `cargo test -p icelines-web roster_shape --quiet`
- [x] `cargo test -p icelines-cli l2_cmd_fantasy_roster_shape --test system_tests --quiet`
- [x] `cargo fmt --check`
- [x] `git diff --check`

## Stop Conditions

- Stop if a proposed dashboard link would mutate roster state through GET.

## Result

Completed. CLI now exposes roster-shape show/set/validate commands, the TUI
command bar hands off to canonical CLI/API targets, and the web dashboard/API
surface validates persisted FantasyDb rosters through read-only GET routes while
explicitly deferring browser mutation.
