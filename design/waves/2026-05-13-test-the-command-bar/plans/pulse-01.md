---
wave: test-the-command-bar
pulse: 01
date: 2026-05-13
status: done
depends_on: []
governing_roles:
  - glass
  - crest
  - bench
  - edge
  - wire
---

# Pulse 01 - Usability Protocol and Command Inventory

## Mission

Create a reproducible user-testing protocol for the MDI command bar, tabbing,
and subcommand handoffs.

## Deliverables

- Inventory the documented command-bar vocabulary.
- Define participant setup, task script, success scale, and issue taxonomy.
- Seed follow-up pulses for focus behavior, discoverability, and moderated
  findings.

## Gates

- [x] `COMMANDS.md` command-bar section reviewed.
- [x] Existing parser/persona test surfaces identified:
      `cargo test -p icelines-cli --bin icelines l0_adams_parse` ran 41
      matching tests; `cargo test -p icelines-cli --bin icelines
      persona_jack_adams -- --list` found 100 persona scenarios.
- [x] `USER-TESTING-PROTOCOL.md` created.

## Notes

The protocol is intentionally observational. Grammar or UI changes should land
in later pulses only after a task failure is classified.
