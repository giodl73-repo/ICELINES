---
wave: clear-the-unblocks
pulse: 01
date: 2026-05-15
status: complete
governing_roles:
  - bench
  - glass
  - tape
  - wire
  - forge
---

# Pulse 01 - Small Unblock Inventory and Pulse Map

## Goal

Open the next wave after Guard the Operations by inventorying the Tier 2
small-unblock backlog and splitting it into safe follow-up pulses.

## Owned Scope

- Read the backlog entries in `design/plans/INDEX.md`.
- Inspect the relevant specs and existing code/tests for headshots,
  admin-overlay behavior, and shift-profile data.
- Produce `SMALL-UNBLOCKS-INVENTORY.md`.
- Create the follow-up pulse map and role review notes.

## Non-goals

- No runtime behavior changes.
- No test additions beyond discovery.
- No data bundling.

## Gates

- [x] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-15-clear-the-unblocks design\waves\PHASES.md --errors-only`

## Result

Opened Clear the Unblocks. The headshot and admin-overlay backlog items are
primarily spec/index drift because focused tests already exist; historical shift
bundling remains a real source/capability decision and gets its own pulse.
