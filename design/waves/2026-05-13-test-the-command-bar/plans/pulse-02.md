---
wave: test-the-command-bar
pulse: 02
date: 2026-05-13
status: planned
depends_on: [pulse-01]
governing_roles:
  - glass
  - bench
  - edge
---

# Pulse 02 - Tabbing and Focus Regression Harness

## Mission

Prove the keyboard focus model is predictable before bringing in testers:
`Tab`, `Shift+Tab`, `:`, `/`, `Esc`, command submission, and pane toggles must
have visible, test-backed outcomes.

## Likely Files

- `icelines-cli/src/tui/event.rs`
- `icelines-cli/src/tui/command.rs`
- `icelines-cli/src/tui/persona_jack_adams.rs`
- `icelines-cli/src/tui/screens/mod.rs`
- `COMMANDS.md`

## Gates

- [ ] `cargo test -p icelines-cli --bin icelines persona_jack_adams`
- [ ] Focused parser/focus tests added or identified by exact name.
- [ ] Help/cheat-sheet text names the focus behavior testers need.

## Stop Conditions

- Stop if a focus fix requires changing screen state ownership outside the MDI
  dashboard scope.
- Stop if a command mutates data without going through an existing mutation
  intent or POST-backed web path.
