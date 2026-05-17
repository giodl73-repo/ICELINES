# Pulse 05: Launch-time room presets

## Goal

Make direct MDI launches into bound workspaces start with the same room preset
state as activity-rail and command-bar activation.

## Changes

- Added an `App::enable_mdi_dashboard` launch seam that attaches MDI state and
  immediately applies the bound workbench experience for the current start
  screen.
- Updated the TUI runner to use that seam instead of attaching raw default MDI
  state.
- Added launch guards for bound and unbound start screens.
- Documented that `icelines tui stats` and `icelines tui --start scores` apply
  matching MDI room presets before the first frame.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-core workbench`
- `cargo test -p icelines-cli --bin icelines tui::workbench`
- `cargo test -p icelines-cli --bin icelines tui::app::tests::l0_mdi_launch`
- `cargo test -p icelines-cli --bin icelines tui::screens::app_snapshot_tests::l0_mdi_active_room_field_summary_renders`
- `cargo test -p icelines-cli --bin icelines tui::app::tests::l0_call_the_changes`
- `git diff --check`

## Status

Done.
