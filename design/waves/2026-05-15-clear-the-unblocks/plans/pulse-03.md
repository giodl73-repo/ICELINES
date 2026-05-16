---
wave: clear-the-unblocks
pulse: 03
date: 2026-05-15
status: planned
governing_roles:
  - tape
  - wire
  - forge
  - bench
---

# Pulse 03 - Shift-Data Bundle Decision

## Goal

Decide whether historical shift-data bundling is currently actionable, and make
the specs/backlog truthful either way.

## Owned Scope

- Inspect `icelines-fetch/src/shift_profile.rs`, shift-related commands, and the
  sync capability matrix.
- Inspect data bundle directories for actual shift artifacts.
- Update data/spec/backlog docs with a truthful decision: actionable pulse,
  blocked/parked rationale, or narrow test-only cleanup.

## Non-goals

- No live shiftchart fetching without fixtures.
- No capability flip from `shifts=off` without a separate source contract.
- No large data bundle additions without size/source validation.

## Gates

- [ ] `cargo test -p icelines-fetch shift_profile --quiet`
- [ ] `C:\src\proof\target\debug\proof.exe check design\specs\data-sources.md design\specs\foster-data-architecture.md design\plans\INDEX.md --errors-only`
