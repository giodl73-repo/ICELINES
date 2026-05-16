---
wave: score-the-day
pulse: 05
date: 2026-05-15
status: planned
governing_roles:
  - bench
  - wire
---

# Pulse 05 - Docs, Regression Gates, and Closeout

## Goal

Close Score the Day with docs, parity records, and regression gates updated.

## Owned Scope

- Update README, COMMANDS, and `surface-parity.md` for any new public surfaces.
- Update `design/plans/INDEX.md` backlog status.
- Run focused gates from Pulses 02-04 plus proof.
- Close `WAVE.md` and `PHASES.md`.

## Non-goals

- No new feature work beyond closeout fixes.
- No broad release hardening unless earlier pulses changed release-critical
  behavior.

## Gates

- [ ] `cargo fmt --check`
- [ ] focused tests from Pulses 02-04
- [ ] proof on touched docs
