# Pulse 09: Hidden pane focus recovery

## Goal

Prevent MDI keyboard focus from staying on a side pane after that pane is hidden.

## Changes

- Added shared MDI side-pane visibility helpers that also recover focus to the
  central workspace when the focused pane is hidden.
- Routed Ctrl+H/Ctrl+L pane toggles and `/hide`/`/show` commands through those
  helpers.
- Added focused guards for keyboard toggles and command-bar hide behavior.
- Documented that hiding a focused side pane returns focus to the workspace.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-core workbench`
- `cargo test -p icelines-cli --bin icelines tui::app::tests::l0_mdi_hiding_focused_side_pane_returns_focus_to_workspace`
- `cargo test -p icelines-cli --bin icelines tui::command::tests::l0_mdi_hide_command_moves_focus_off_hidden_pane`
- `cargo test -p icelines-cli --bin icelines tui::app::tests::l0_adams_ctrl`
- `git diff --check`

## Status

Done.
