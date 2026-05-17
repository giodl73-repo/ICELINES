# Pulse 08: Workbench chrome labels

## Goal

Remove the remaining generic MDI chrome label for named workbench destinations.

## Changes

- Added explicit MDI chrome labels for Admin, Docs, Groups, and group-detail
  workbench destinations.
- Added a render guard proving the Admin room footer uses the named chrome label
  while still showing the active room field summary.
- Documented the named chrome behavior in the TUI user docs.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-core workbench`
- `cargo test -p icelines-cli --bin icelines tui::workbench`
- `cargo test -p icelines-cli --bin icelines tui::screens::app_snapshot_tests::l0_mdi_admin_room_uses_workbench_chrome_label`
- `cargo test -p icelines-cli --bin icelines tui::screens::app_snapshot_tests::l0_mdi_active_room_field_summary_renders`
- `git diff --check`

## Status

Done.
