# Pulse 06: Activity rail selection sync

## Goal

Keep the MDI activity rail's selected catalog entry aligned with workspace
changes that happen outside direct rail activation.

## Changes

- Added a `MdiLayout::select_workbench_id` helper for catalog selection by
  shared workbench ID.
- Synced launch-time MDI initialization with the active start workspace before
  applying bound room presets.
- Synced command-bar workspace swaps with the activity rail selection while
  preserving bound-room activation and stale-room clearing.
- Documented that launch and command-bar swaps keep the rail positioned on the
  active workspace.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-core workbench`
- `cargo test -p icelines-cli --bin icelines tui::workbench`
- `cargo test -p icelines-cli --bin icelines tui::app::tests::l0_mdi_launch`
- `cargo test -p icelines-cli --bin icelines tui::command::tests::l0_mdi_exec_workspace_swap_applies_bound_experience`
- `cargo test -p icelines-cli --bin icelines tui::app::tests::l0_call_the_changes`
- `git diff --check`

## Status

Done.
