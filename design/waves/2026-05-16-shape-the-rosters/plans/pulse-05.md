---
wave: shape-the-rosters
pulse: 05
date: 2026-05-16
status: planned
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

- [ ] focused core/fetch/CLI/web roster-shape tests
- [ ] `cargo fmt --check`
- [ ] proof on touched docs
- [ ] `git diff --check`

## Stop Conditions

- Stop if any prior pulse gate is unchecked.
