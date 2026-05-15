# Pulse 01 - MDI Workbench Inventory and Zone Contract

## Goal

Define the final MDI navigation contract before implementation. Inventory the
current TUI MDI dashboard, classic tab behavior, standalone launchers, web
dashboard workspace routing, command grammar, and docs. Produce an implementation
map for replacing tab-first/default command-recall navigation with a true
workbench model across TUI and web: activity/catalog rail, center workspace,
left/right context panes, top live ribbon, bottom command/status surface, and
temporary overlays.

## Governing roles

- **keel**: one catalog identity and one zone taxonomy must map cleanly to TUI
  screens, web workspace routes, and command aliases.
- **glass**: the catalog/picker and pane layout must be discoverable and readable
  without memorized command syntax; tab strip removal must not strand users.
- **forge**: prefer typed catalog entries and existing screen/route enums over
  duplicated string tables.
- **wire**: picker/catalog navigation must not create GET-backed mutations, live
  fetches, or browser-only state that changes canonical route meaning.
- **bench**: inventory must name regression tests for active-screen selection,
  catalog completeness, ViewModel-backed pane fields/models, keyboard access,
  and docs parity.

## Owned scope

1. Inspect current TUI MDI implementation, screen enum/surface launchers,
   command-bar routing, and `--classic` / `--standalone` behavior.
2. Inspect current web `/dashboard` workspace routing, panel-ready route table,
   command endpoint, and static dashboard assets.
3. Write a wave-local inventory/contract document covering:
   - main screen catalog entries and aliases;
   - workbench zones and placement rules:
     - activity/catalog rail;
     - center workspace;
     - left context pane;
     - right context pane;
     - top live ribbon;
     - bottom command/status surface;
     - overlays;
   - context-pane option bank sourced from existing ViewModels, including pane
     models, shared fields, filter dimensions, summaries, timelines,
     comparisons, queues, source-state panes, and action/status panes;
   - TUI workbench UX options and selected default;
   - web workbench UX options and selected default;
   - tab removal/compatibility rules for default MDI and `--classic`;
   - keyboard and accessibility expectations;
   - tests and docs required before closeout.
4. Amend later pulse plans if the inventory reveals a better split.

## Non-goals

- No implementation code in this pulse.
- No new analytics screens or ViewModels.
- No route-local data logic, browser-only mutations, or live-fetching GET paths.
- No removal of command bars; commands remain shortcuts.

## Candidate deliverable

`MDI-WORKBENCH-INVENTORY.md` in this wave folder.

## Initial implementation split to validate

| Pulse | Scope |
|---|---|
| 02 - Shared workbench catalog, fields, and pane models | Add typed catalog entries and tests for labels, aliases, default zone, TUI targets, web targets, ViewModel-backed fields, pane model capabilities, and bound experience tabs. |
| 03 - TUI full-MDI workbench shell | Replace default TUI MDI tab-first navigation with activity/catalog selection, center workspace, context panes, live ribbon, and command/status surface while preserving `--classic`. |
| 04 - Web dashboard workbench catalog | Add visible catalog/zone navigation to `/dashboard` that opens canonical workspace panels and keeps side-pane semantics aligned with TUI. |
| 05 - Docs, regression gates, and closeout | Update README/COMMANDS/surface parity, run gates, and close the wave. |

## Gates

- [x] `cargo fmt --check`
- [x] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-15-call-the-changes design\waves\PHASES.md --errors-only`
