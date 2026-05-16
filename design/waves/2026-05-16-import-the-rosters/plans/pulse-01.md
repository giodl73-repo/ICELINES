---
wave: import-the-rosters
pulse: 01
date: 2026-05-16
status: complete
governing_roles:
  - tape
  - wire
  - forge
  - bench
  - glass
---

# Pulse 01 - CSV Import Inventory and Pulse Map

## Goal

Open the Import the Rosters wave and define an executable pulse map for Yahoo
fantasy roster CSV import.

## Owned Scope

- Inspect current fantasy persistence, CLI surfaces, CSV parsing, source-truth
  specs, and surface parity rails.
- Create `FANTASY-CSV-IMPORT-INVENTORY.md`.
- Create pulse plans and role-review panels.
- Add the wave to `design/waves/PHASES.md`.
- Mark the Tier 3 backlog item as active.

## Non-goals

- No runtime behavior.
- No database migration yet.
- No web upload/import form.
- No Yahoo API integration or live Yahoo tests.

## Gates

- [x] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-16-import-the-rosters design\waves\PHASES.md design\plans\INDEX.md --errors-only`

## Result

Opened Import the Rosters with Yahoo CSV scoped to local fantasy roster
membership import, not player/stat ground truth.
