---
wave: backcheck-the-phases
pulse: 01
date: 2026-05-13
status: done
governing_roles:
  - bench
  - forge
  - keel
  - tape
  - wire
---

# Pulse 01 - Backfill Inventory and Pulse Map

## Mission

Create the first honest inventory of previous-phase residual work and convert it
into pulse-sized packets. This is a planning pulse: no product code changes
unless a broken doc link or typo blocks the inventory.

## Discovery Scope

- `design/phases.md`
- `design/plans/`
- `design/specs/surface-parity.md`
- `design/ARCHITECTURE.md`
- `design/INVARIANTS.md`
- `design/PITFALLS.md`
- `README.md`
- `COMMANDS.md`

## Deliverables

- `design/waves/2026-05-13-backcheck-the-phases/BACKFILL-INVENTORY.md`
  with phase-by-phase residuals.
- New or amended pulse plans for the highest-value residuals.
- A short deferred/deleted list for obsolete plans.
- Updated `WAVE.md` pulse status table.

## Role Lenses

- `bench` - does this help a real user find the next useful surface?
- `forge` - are pulse boundaries implementable without hidden coupling?
- `keel` - does the inventory preserve architecture and data flow?
- `tape` - are tests/gates attached to each residual?
- `wire` - are CLI/TUI/web/report parity gaps called out explicitly?

## Gates

- [x] Inventory names every currently implemented trophy phase in
      `design/phases.md`.
- [x] Each residual item has one status: `pulse`, `defer`, `delete`, or `done`.
- [x] Each `pulse` item maps to a pulse number and owner surface.
- [x] Each pulse has tests/gates and affected files or discovery scope.
- [x] No residual requires reading chat history to understand the ask.

## Suggested Commands

```powershell
rg -n "partial|defer|TODO|remaining|carry-forward|implemented|planned" design README.md COMMANDS.md
rg -n "Jack Adams|Prince|Campbell|Presidents|Selke|Messier" design
```
