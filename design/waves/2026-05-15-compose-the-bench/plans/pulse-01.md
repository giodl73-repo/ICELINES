# Pulse 01 - Pane Composition Inventory and Contract

## Goal

Define the pane-composition contract before implementation. Inspect the shared
workbench catalog, existing TUI MDI side panes, web dashboard pane affordances,
and current ViewModel field coverage. Produce a wave-local inventory that says
which pane models can be composed now, which fields drive them, what state is
surface-local versus shared, and what later pulses must implement.

## Governing roles

- **keel**: one pane binding identity must map to TUI and web without route-local
  clones.
- **glass**: pane composition must keep the center workspace primary and make
  focus/selection visible.
- **forge**: inventory should lead to typed metadata, not free-form strings.
- **wire**: pane selection is read/navigation state; do not introduce GET-backed
  mutations or live-fetching panel paths.
- **bench**: inventory must name tests for every planned binding and control.

## Owned scope

1. Inspect `icelines-core/src/workbench.rs`.
2. Inspect TUI MDI state/render/event files and shared TUI adapter.
3. Inspect web dashboard handler/templates/static JS/CSS and shared web adapter.
4. Inspect ViewModel-bearing modules as needed to verify pane fields are backed
   by existing data contracts.
5. Produce `PANE-COMPOSITION-INVENTORY.md` with:
   - pane model binding taxonomy;
   - center/left/right/top/bottom placement rules;
   - candidate bound experiences and active field sets;
   - TUI implementation plan;
   - web implementation plan;
   - stop conditions and non-goals;
   - test and docs matrix.
6. Amend later pulse plans if the inventory reveals a better split.

## Non-goals

- No implementation code.
- No new analytics, projections, scoring formulas, or data fetchers.
- No new mutation routes.

## Gates

- [ ] `cargo fmt --check`
- [ ] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-15-compose-the-bench design\waves\PHASES.md --errors-only`
