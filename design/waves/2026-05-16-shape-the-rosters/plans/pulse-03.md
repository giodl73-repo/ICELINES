---
wave: shape-the-rosters
pulse: 03
date: 2026-05-16
status: planned
governing_roles:
  - forge
  - wire
  - bench
---

# Pulse 03 - FantasyDb Shape Persistence and Import Validation

## Goal

Persist league roster-shape settings and validate imports/manual roster state
through the core contract.

## Owned Scope

- Add a safe FantasyDb migration for roster-shape preset/config.
- Wire default shape behavior for existing leagues.
- Validate Yahoo import dry-run/apply output without treating position hints as
  authoritative player truth.
- Add fetch-layer tests for migration, defaults, import warnings, and no-mutation
  dry runs.

## Non-goals

- No CLI shape commands yet beyond helpers needed for tests.
- No remote Yahoo API.
- No live network tests.

## Gates

- [ ] `cargo test -p icelines-fetch roster_shape --quiet`
- [ ] `cargo test -p icelines-fetch fantasy_import --quiet`
- [ ] `cargo fmt --check`
- [ ] `git diff --check`

## Stop Conditions

- Stop if migration would make existing leagues unreadable or silently discard
  existing roster rows.
