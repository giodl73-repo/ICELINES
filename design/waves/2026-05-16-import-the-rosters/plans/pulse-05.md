---
wave: import-the-rosters
pulse: 05
date: 2026-05-16
status: planned
governing_roles:
  - bench
  - wire
  - glass
---

# Pulse 05 - Docs, Regression Gates, and Closeout

## Goal

Close Import the Rosters with user-facing documentation, surface-parity truth,
backlog status, and regression gates updated.

## Owned Scope

- Update `README.md`, `COMMANDS.md`, `design/specs/surface-parity.md`, and
  `design/specs/data-sources.md` if import behavior changes the Yahoo CSV truth.
- Move the Tier 3 backlog row to a cleared section in `design/plans/INDEX.md`.
- Mark pulse and wave statuses complete in `WAVE.md`, `plans/pulse-05.md`, and
  `design/waves/PHASES.md`.
- Run focused gates from Pulses 02-04, proof, and whitespace checks.

## Non-goals

- No new runtime behavior.
- No broad unrelated documentation cleanup.

## Gates

- [ ] `cargo fmt --check`
- [ ] focused tests from Pulses 02-04
- [ ] proof on touched docs
- [ ] `git diff --check`
