# Pulse 24: TUI MDI Tab cleanup

## Goal

Continue the TUI cleanup pass by removing obsolete MDI Tab commentary and
making the workbench test helper's scope explicit.

## Changes

- Marked the no-argument workbench mapping helper as test-only.
- Updated the legacy Tab branch comment to point at the current MDI focus
  traversal path instead of the old no-op stub.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-cli --bin icelines tui::workbench`
- `cargo test -p icelines-cli --bin icelines l0_call_the_changes_mdi_tab_moves_focus_not_workspace`
- `git diff --check`

## Status

Done.
