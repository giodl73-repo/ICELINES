---
wave: shape-the-rosters
pulse: 04
date: 2026-05-16
status: planned
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

- [ ] `cargo test -p icelines-cli roster_shape --quiet`
- [ ] `cargo test -p icelines-web roster_shape --quiet`
- [ ] `cargo test -p icelines-cli l2_cmd_fantasy_roster_shape --test system_tests --quiet`
- [ ] `cargo fmt --check`
- [ ] `git diff --check`

## Stop Conditions

- Stop if a proposed dashboard link would mutate roster state through GET.
