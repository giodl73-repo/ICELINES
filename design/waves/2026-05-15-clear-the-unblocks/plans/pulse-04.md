---
wave: clear-the-unblocks
pulse: 04
date: 2026-05-15
status: planned
governing_roles:
  - bench
  - wire
---

# Pulse 04 - Docs, Regression Gates, and Closeout

## Goal

Close Clear the Unblocks after spec drift is corrected and shift-data status is
truthful.

## Owned Scope

- Update `WAVE.md`, `PHASES.md`, and completed pulse plans.
- Update README/COMMANDS only if user-facing behavior changed.
- Run focused regression gates from Pulses 02-03 plus docs proof.

## Non-goals

- No new runtime behavior.
- No broad release-hardening pass beyond this wave's scope.

## Gates

- [ ] `cargo fmt --check`
- [ ] focused crate tests from Pulses 02-03
- [ ] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-15-clear-the-unblocks design\plans\INDEX.md design\specs\headshot-rendering.md design\specs\tui-admin-overlay.md design\specs\data-sources.md design\specs\foster-data-architecture.md --errors-only`
