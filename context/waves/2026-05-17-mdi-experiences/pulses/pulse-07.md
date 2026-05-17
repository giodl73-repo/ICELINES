# Pulse 07: Activity rail viewport

## Goal

Keep the selected MDI activity-rail entry visible when shorter terminals cannot
show the full shared workbench catalog.

## Changes

- Made the activity rail render a selected-entry window instead of always
  painting from the top of the catalog.
- Added a short-terminal render guard for the lower Admin room preset.
- Documented that the rail scrolls to keep selected rooms visible.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-core workbench`
- `cargo test -p icelines-cli --bin icelines tui::workbench`
- `cargo test -p icelines-cli --bin icelines tui::screens::app_snapshot_tests::l0_mdi_activity_rail_scrolls_selected_room_into_view`
- `cargo test -p icelines-cli --bin icelines tui::screens::app_snapshot_tests::l0_mdi_activity_rail_surfaces_bound_experience_labels`
- `git diff --check`

## Status

Done.
