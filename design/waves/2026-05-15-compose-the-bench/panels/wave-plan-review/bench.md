# BENCH review - Compose the Bench plan

## Findings

- The wave is testable if each pulse asserts table integrity and surface
  behavior instead of relying on screenshots or manual inspection.
- Add tests at the level where bugs would occur: core table references in L0,
  TUI focus/application behavior in CLI tests, web route/template behavior in
  web tests.
- Closeout should not mark complete until pulse files have checked gates and
  proof validates the new docs.

## Required checks

- Pulse 01 inventory should name exact regression tests for each planned pane
  control.
- Pulse 05 should run the full listed gates after docs updates, not before.
