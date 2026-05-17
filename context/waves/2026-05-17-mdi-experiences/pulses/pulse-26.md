# Pulse 26: TUI broad-suite cleanup

## Goal

Move the TUI cleanup pass from focused slices to the broad `tui` test suite and
fix stale expectations found there.

## Changes

- Updated the focused right-pane cycling app test to assert the shared binding
  catalog contract instead of a stale hard-coded next binding.
- Kept the workspace stability assertion and status-label assertion intact.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-cli --bin icelines tui`
- `git diff --check`

## Status

Done.
