# FORGE review - Compose the Bench plan

## Findings

- The correct implementation shape is typed IDs and static metadata tables in
  core, plus small adapter structs in TUI/web. Avoid stringly pane names in
  event handlers or templates.
- `icelines-core` must remain pure. No route generation, terminal styling, HTML,
  config paths, or I/O belongs in the shared contract.
- Keep ownership cheap. Pane metadata should be copied by ID/reference, not
  cloned from large ViewModels.

## Required checks

- Pulse 02 should include compile-time-ish L0 table integrity tests.
- Clippy gates should target each touched crate before broader closeout.
