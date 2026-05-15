---
wave: guard-the-operations
pulse: 01
date: 2026-05-15
status: done
governing_roles:
  - keel
  - wire
  - bench
  - forge
  - glass
---

# Pulse 01 - Operations Parity Inventory and Pulse Map

## Mission

Create a durable inventory of the remaining operational/product-UX partials and
convert them into executable pulses. This is a planning pulse: no product code
changes unless a broken doc link blocks the inventory.

## Discovery Scope

- `design/specs/surface-parity.md`
- `design/plans/INDEX.md`
- `design/plans/2026-05-09-phaseJennings-stabilization-truth.md`
- `design/waves/2026-05-13-backcheck-the-phases/ADMIN-OPERATIONS-INVENTORY.md`
- `design/waves/2026-05-13-backcheck-the-phases/CAREER-DOCS-INVENTORY.md`
- `README.md`
- `COMMANDS.md`
- Related web/TUI/admin/watch/favorites source files as needed for file ownership.

## Deliverables

- `OPERATIONS-PARITY-INVENTORY.md` with residuals, decisions, and pulse mapping.
- Pulse plans for Pulses 02-06.
- Role review panel under `panels/wave-plan-review/`.
- Updated `WAVE.md` and `design/waves/PHASES.md`.

## Role Lenses

- **keel** - Does each pulse preserve shared ViewModel/mutation-intent
  convergence across CLI/TUI/web?
- **wire** - Are GET/read navigation and POST/write mutations clearly separated?
- **bench** - Does each pulse have a focused regression gate that would catch the
  intended bug class?
- **forge** - Are crate boundaries and handler responsibilities scoped enough to
  implement safely?
- **glass** - Will the resulting UI make state, persistence, and deferrals clear
  to users?

## Gates

- [x] Inventory names every `partial`, `planned`, or `deferred` operational/product
      surface selected for this wave.
- [x] Each residual item has one status: `pulse`, `defer`, `done`, or `watch`.
- [x] Each `pulse` item maps to a pulse number, owner surface, and gates.
- [x] Role review files exist for the governing panel.
- [x] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-15-guard-the-operations design\waves\PHASES.md --errors-only`

## Suggested Commands

```powershell
rg -n "partial|planned|deferred|handoff|runtime|destructive|watch rules|Favorites/groups" design\specs\surface-parity.md
rg -n "ConfigView|MutationResultView|DataMutationIntent|SnapshotMutationIntent|WatchRuleMutationIntent|FavoritesView|GroupDb" icelines-core icelines-cli icelines-web
```

## Result

Opened Guard the Operations, produced the operations parity inventory, mapped
Pulses 02-06, and wrote the governing role review panel.
