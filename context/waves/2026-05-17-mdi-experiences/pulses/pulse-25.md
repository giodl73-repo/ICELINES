# Pulse 25: TUI event scaffolding cleanup

## Goal

Continue the TUI cleanup pass by tightening stale scaffolding around the shared
event and screen-dispatch layers.

## Changes

- Removed broad dead-code suppression from the TUI `Action` enum.
- Removed the stale `Back` variant-specific dead-code suppression.
- Added focused event-map tests for MDI pane toggles, Shift-Tab, and key
  release filtering.
- Updated the screen dispatch module comment to reflect that `ScreenAction` and
  `AppContext` are wired while the `Screen` trait remains a migration seam.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-cli --bin icelines tui::event`
- `cargo test -p icelines-cli --bin icelines tui::screen`
- `git diff --check`

## Status

Done.
