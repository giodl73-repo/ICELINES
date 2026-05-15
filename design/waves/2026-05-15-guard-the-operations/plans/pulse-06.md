---
wave: guard-the-operations
pulse: 06
date: 2026-05-15
status: planned
governing_roles:
  - tape
  - broadcast
  - wire
  - bench
---

# Pulse 06 - Docs, Regression Gates, and Closeout

## Goal

Close Guard the Operations after the selected operational partials are resolved,
deferred with durable rationale, and documented.

## Owned Scope

- Update README, COMMANDS, and `surface-parity.md` for changed operational
  behavior.
- Update `WAVE.md`, `PHASES.md`, and pulse plans.
- Run focused gates from Pulses 02-05 plus a final docs proof and release smoke
  or release build.
- Commit, push, and check CI status.

## Non-goals

- No new product behavior beyond closeout docs/tests.
- No unrelated formatting churn.

## Gates

- [ ] `cargo fmt --check`
- [ ] `cargo test -p icelines-core --quiet`
- [ ] `cargo test -p icelines-cli --quiet`
- [ ] `cargo test -p icelines-web --quiet`
- [ ] `cargo clippy -- -D warnings`
- [ ] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-15-guard-the-operations README.md COMMANDS.md design\specs\surface-parity.md --errors-only`
- [ ] `powershell -ExecutionPolicy Bypass -File scripts\release-smoke.ps1`
