# Pulse 04: Shared inspector pane cycling

## Goal

Make MDI side-pane cycling reach the shared workbench inspector catalog without
showing web-only dead ends in the TUI.

## Changes

- Marked shared groups, player, goalie, game, scoring trend, outlook, fantasy
  simulation, records, and career pane bindings as TUI-safe.
- Kept native TUI panes native while rendering web-derived inspectors as compact
  field and command summaries.
- Documented side-pane cycling as a full shared inspector-catalog affordance.
- Added a render guard proving a cycled web-derived inspector pane appears in
  the MDI dashboard.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-core workbench`
- `cargo test -p icelines-cli --bin icelines tui::workbench`
- `cargo test -p icelines-cli --bin icelines tui::screens::app_snapshot_tests::l0_mdi_side_pane_cycles_web_catalog_summaries`
- `cargo test -p icelines-cli --bin icelines tui::app::tests::l0_call_the_changes`
- `git diff --check`

## Status

Done.
