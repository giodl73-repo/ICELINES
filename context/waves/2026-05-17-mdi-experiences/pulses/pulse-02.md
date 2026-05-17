# Pulse 02: Activity rail room labels

## Goal

Make the final MDI room presets discoverable before a user activates them.

## Changes

- Added a TUI workbench helper that resolves the bound experience for a catalog
  workspace.
- Reused that helper for activity-rail activation so render and dispatch share
  one experience lookup.
- Rendered compact room labels in the activity rail for Tonight, Scoring, Team,
  Fantasy, and Admin presets.
- Added a render guard for the rail labels.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-core workbench`
- `cargo test -p icelines-cli --bin icelines tui::workbench`
- `cargo test -p icelines-cli --bin icelines tui::app::tests::l0_call_the_changes`
- `cargo test -p icelines-cli --bin icelines tui::screens::app_snapshot_tests::l0_mdi_activity_rail_surfaces_bound_experience_labels`
- `git diff --check`

## Status

Done.
