# Pulse 23: TUI MDI scaffolding cleanup

## Goal

Start the TUI cleanup pass by removing stale MDI scaffolding that no longer
matches the finished room/pane experience.

## Changes

- Removed the unused MDI pane stub renderer.
- Dropped stale dead-code allowances from wired TUI workbench helpers.
- Refreshed MDI layout comments to describe the current data-first TUI chrome.
- Updated pane-cycle tests to assert the current shared binding catalog contract
  instead of stale hard-coded neighbor bindings.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-cli --bin icelines tui::mdi`
- `cargo test -p icelines-cli --bin icelines tui::workbench`
- `cargo test -p icelines-cli --bin icelines l0_adams_render_does_not_panic_at_any_width`
- `git diff --check`

## Status

Done.
