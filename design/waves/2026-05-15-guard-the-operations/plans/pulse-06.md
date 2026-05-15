---
wave: guard-the-operations
pulse: 06
date: 2026-05-15
status: complete
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

- [x] `cargo fmt --check`
- [x] `cargo test -p icelines-core --quiet`
- [x] `cargo test -p icelines-cli --quiet`
- [x] `cargo test -p icelines-web --quiet`
- [x] `cargo clippy -- -D warnings`
- [x] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-15-guard-the-operations README.md COMMANDS.md design\specs\surface-parity.md --errors-only`
- [x] `powershell -ExecutionPolicy Bypass -File scripts\release-smoke.ps1`

## Result

Closed Guard the Operations after documenting every moved or intentionally
deferred operational partial. The closeout gates passed, including full clippy,
proof on the wave/docs/parity surface, and release smoke against the release CLI
binary.
