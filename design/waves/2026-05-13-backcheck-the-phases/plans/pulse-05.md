---
wave: backcheck-the-phases
pulse: 05
date: 2026-05-13
status: planned
depends_on: [pulse-01]
governing_roles:
  - scout
  - tape
  - forge
  - wire
---

# Pulse 05 - Scenario Harness Classification

## Mission

Sort the large scenario corpus into what is still applicable, what is already
covered by tests, what should become a test, and what should be retired.

## Deliverables

- Scenario inventory with counts by product surface.
- Classification: `test-backed`, `needs-test`, `docs-only`, `obsolete`,
  `future-product`.
- A recommended next batch of scenario-to-test conversions.
- Links from scenario groups to existing focused test slices.

## Discovery Scope

- `src/**/persona*.rs`
- `src/**/scenario*.rs`
- `icelines-cli/tests/`
- `icelines-web/tests/`
- `design/notes/*scenario*`
- `README.md`
- `COMMANDS.md`

## Gates

- [ ] Inventory command records the scenario count and source paths.
- [ ] Every scenario bucket has at least one example.
- [ ] `needs-test` scenarios map to a crate/surface and suggested test level.
- [ ] Obsolete scenarios explain the superseding ViewModel or surface.
- [ ] Recommended next conversions name the exact test file or new test target.

## Stop Conditions

- Stop if scenario files are generated artifacts without stable IDs; first
  create an ID/stability pulse.
