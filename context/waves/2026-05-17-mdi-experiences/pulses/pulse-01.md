# Pulse 01: TUI-bound workbench rooms

## Goal

Make the final named MDI room presets usable from the TUI activity rail.

## Changes

- Marked Scoring room, Team room, Fantasy room, and Admin room as TUI-supported
  shared workbench experiences.
- Promoted their side-pane bindings to TUI-safe bindings.
- Added compact side-pane summary rendering for non-native TUI panes so MDI
  rooms show useful field and command context instead of unavailable stubs.
- Updated user docs to name the available TUI workbench room presets.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-core workbench`
- `cargo test -p icelines-cli --bin icelines tui::workbench`
- `cargo test -p icelines-cli --bin icelines tui::app::tests::l0_call_the_changes`
- `cargo test -p icelines-cli --bin icelines commands::data`
- `git diff --check`

## Status

Done.
