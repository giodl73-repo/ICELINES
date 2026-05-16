---
wave: shape-the-rosters
pulse: 05
date: 2026-05-16
status: complete
governing_roles:
  - bench
  - glass
  - wire
---

# Pulse 05 - Docs, Regression Gates, and Closeout

## Goal

Close Shape the Rosters after the core contract, persistence/import wiring, and
surfaces are documented and verified.

## Owned Scope

- Update README, COMMANDS, surface parity, and backlog truth.
- Run focused gates from Pulses 02-04.
- Mark WAVE, PHASES, and Pulse 05 closed.

## Non-goals

- No release tag.
- No unrelated fantasy feature work.

## Gates

- [x] focused core/fetch/CLI/web roster-shape tests
- [x] `cargo fmt --check`
- [x] proof on touched docs
- [x] `git diff --check`

## Stop Conditions

- Stop if any prior pulse gate is unchecked.

## Result

Completed. README, COMMANDS, surface parity, backlog, wave, and phase-index
truth now document roster-shape setup and validation as shipped. The Shape the
Rosters wave is closed with CLI-backed mutation, TUI/web read handoffs, and no
GET-backed roster-state mutation.
