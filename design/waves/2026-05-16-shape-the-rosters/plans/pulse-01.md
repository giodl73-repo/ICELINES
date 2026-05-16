---
wave: shape-the-rosters
pulse: 01
date: 2026-05-16
status: complete
governing_roles:
  - bench
  - forge
  - wire
  - glass
---

# Pulse 01 - Roster Shape Inventory and Pulse Map

## Goal

Open the fantasy roster-shape enforcement wave by mapping current storage,
import, ViewModel, and surface behavior.

## Owned Scope

- Inspect fantasy DB, import, scoring scheme, CLI docs, and backlog records.
- Produce `FANTASY-ROSTER-SHAPE-INVENTORY.md`.
- Create follow-up pulse plans and role-review notes.
- Mark the wave active in `design/waves/PHASES.md` and the backlog active in
  `design/plans/INDEX.md`.

## Non-goals

- No code behavior changes.
- No DB migration yet.
- No roster-shape command yet.

## Gates

- [x] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-16-shape-the-rosters design\waves\PHASES.md design\plans\INDEX.md --errors-only`
- [x] `git diff --check`

## Result

Opened Shape the Rosters. Current FantasyDb roster state is normalized-name only,
Yahoo CSV position hints are diagnostics only, and `Scheme` has scoring weights
but no roster-shape rules. Follow-up pulses split the pure core contract,
FantasyDb/import persistence, CLI/TUI/dashboard surfaces, and docs/closeout.
