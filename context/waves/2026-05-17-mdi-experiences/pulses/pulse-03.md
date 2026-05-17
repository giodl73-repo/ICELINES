# Pulse 03: Active room field strip

## Goal

Make active MDI room context visible after a preset is opened.

## Changes

- Added an active room field summary to the MDI screen-keybind strip.
- Reused the shared workbench field catalog so field labels stay aligned with
  the TUI and web workbench model.
- Routed command-bar workspace swaps through the same bound-room activation path
  as the activity rail, clearing stale room context for unbound screens.
- Added a render guard proving the Scoring room advertises its field scope in
  dashboard chrome.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-core workbench`
- `cargo test -p icelines-cli --bin icelines tui::workbench`
- `cargo test -p icelines-cli --bin icelines tui::command::tests::l0_mdi_exec_workspace_swap_applies_bound_experience`
- `cargo test -p icelines-cli --bin icelines tui::app::tests::l0_call_the_changes`
- `cargo test -p icelines-cli --bin icelines tui::screens::app_snapshot_tests::l0_mdi_active_room_field_summary_renders`
- `git diff --check`

## Status

Done.
