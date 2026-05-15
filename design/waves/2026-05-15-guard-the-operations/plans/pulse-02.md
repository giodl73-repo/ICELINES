---
wave: guard-the-operations
pulse: 02
date: 2026-05-15
status: done
governing_roles:
  - keel
  - wire
  - bench
  - forge
  - glass
---

# Pulse 02 - Persistent Config and Report Toggle Contract

## Goal

Resolve the config/report-toggle partial without making runtime web state look
durable. The web admin surface must either persist report toggles through the
same config contract used by CLI/TUI or explicitly label and fence what remains
runtime-only.

## Owned Scope

- Inspect existing `ConfigView`, `ConfigMutationIntent`, report toggle storage,
  and web admin forms/JSON.
- Implement the smallest safe persistent report-toggle path if the existing
  contract supports it.
- If persistence requires a broader config redesign, keep runtime web controls
  explicit and document a durable deferral.
- Update `surface-parity.md`, README/COMMANDS only if behavior changes.

## Non-goals

- No new config file format.
- No hidden long-lived web session state.
- No GET-backed config mutation.

## Gates

- [x] `cargo fmt --check`
- [x] `cargo test -p icelines-web --quiet`
- [x] `cargo clippy -p icelines-web --no-deps -- -D warnings`
- [x] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-15-guard-the-operations design\specs\surface-parity.md README.md COMMANDS.md --errors-only`

## Result

Persistent report-toggle writes remain deliberately deferred on web because the
durable report config contract still lives in the CLI/TUI `Config` type. The web
admin surface now labels active-season controls as runtime-only, renders an
explicit persistent-report-toggle deferral with the TUI recovery path, and emits
a `ConfigView.warnings` entry for JSON clients.
