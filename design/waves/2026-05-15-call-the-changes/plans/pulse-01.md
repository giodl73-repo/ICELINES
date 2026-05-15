# Pulse 01 - MDI Navigation Inventory and Contract

## Goal

Define the final MDI navigation contract before implementation. Inventory the
current TUI MDI dashboard, classic tab behavior, standalone launchers, web
dashboard workspace routing, command grammar, and docs. Produce an implementation
map for replacing tab-first/default command-recall navigation with an explicit
screen catalog/picker across TUI and web.

## Governing roles

- **keel**: one catalog identity must map cleanly to TUI screens, web workspace
  routes, and command aliases.
- **glass**: the catalog/picker must be discoverable and readable without
  memorized command syntax; tab strip removal must not strand users.
- **forge**: prefer typed catalog entries and existing screen/route enums over
  duplicated string tables.
- **wire**: picker/catalog navigation must not create GET-backed mutations, live
  fetches, or browser-only state that changes canonical route meaning.
- **bench**: inventory must name regression tests for active-screen selection,
  catalog completeness, keyboard access, and docs parity.

## Owned scope

1. Inspect current TUI MDI implementation, screen enum/surface launchers,
   command-bar routing, and `--classic` / `--standalone` behavior.
2. Inspect current web `/dashboard` workspace routing, panel-ready route table,
   command endpoint, and static dashboard assets.
3. Write a wave-local inventory/contract document covering:
   - main screen catalog entries and aliases;
   - TUI picker/catalog UX options and selected default;
   - web catalog/picker UX options and selected default;
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

`MDI-NAVIGATION-INVENTORY.md` in this wave folder.

## Initial implementation split to validate

| Pulse | Scope |
|---|---|
| 02 - Shared screen catalog | Add typed catalog entries and tests for labels, aliases, TUI targets, and web targets. |
| 03 - TUI full-MDI picker | Replace default MDI tab strip/cycling with picker/catalog selection while preserving `--classic`. |
| 04 - Web dashboard screen catalog | Add visible catalog/picker navigation to `/dashboard` that opens canonical workspace panels. |
| 05 - Docs, regression gates, and closeout | Update README/COMMANDS/surface parity, run gates, and close the wave. |

## Gates

- [ ] `cargo fmt --check`
- [ ] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-15-call-the-changes design\waves\PHASES.md --errors-only`
