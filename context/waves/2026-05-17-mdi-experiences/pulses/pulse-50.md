# Pulse 50: TUI orbit key parity

## Goal

Catch the TUI up to the browser dashboard orbit model by mapping the same
center-stable shortcut keys onto existing TUI MDI pane behavior.

## Changes

- Added browser-parity TUI MDI shortcuts: `[` cycles the left pane binding,
  `]` cycles the right pane binding, and `\` swaps focus between the center
  workspace and the active side pane.
- Kept center workspace state stable while cycling pane bindings.
- Updated the TUI MDI command-bar hint row to advertise the new orbit keys.
- Added App-level tests for pane cycling and center/side focus swapping.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-cli tui::event::tests::l0_mdi_ctrl_h_and_ctrl_l_map_to_side_pane_toggles`
- `cargo test -p icelines-cli tui::app::tests::l0_mdi_browser_parity`
- `cargo test -p icelines-cli tui::mdi::tests`
- `git diff --check`
- `cargo build --release`

## Status

Done.
