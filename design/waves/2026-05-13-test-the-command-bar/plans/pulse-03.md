---
wave: test-the-command-bar
pulse: 03
date: 2026-05-13
status: planned
depends_on: [pulse-01]
governing_roles:
  - edge
  - bench
  - keel
  - wire
---

# Pulse 03 - Command Vocabulary and Subcommand Discoverability

## Mission

Compare documented command examples against parser behavior and web/TUI
handoffs, then fix mismatches or make intentional handoffs clearer.

## Likely Files

- `COMMANDS.md`
- `README.md`
- `icelines-cli/src/tui/command.rs`
- `icelines-web/src/handlers/dashboard.rs`
- `icelines-web/tests/l1_router.rs`

## Gates

- [ ] `cargo test -p icelines-cli --bin icelines l0_adams_parse`
- [ ] `cargo test -p icelines-cli --bin icelines l0_adams_exec`
- [ ] `cargo test -p icelines-web dashboard_command`
- [ ] Every command in the protocol is classified: opens workspace, flashes
      CLI/web target, mutates through safe intent, or intentionally unsupported.

## Stop Conditions

- Stop if a discoverability fix would require a new ViewModel not already
  exposed.
- Stop if web command parsing would introduce GET-backed mutations.
