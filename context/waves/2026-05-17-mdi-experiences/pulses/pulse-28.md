# Pulse 28: TUI release warning cleanup

## Goal

Make the release build clean after the TUI cleanup pass exposed helpers that are
test-only in optimized builds.

## Changes

- Scoped the no-argument workbench catalog import to tests.
- Scoped MDI pane-model accessors and imports to tests.
- Kept the synthetic `Action::Back` variant explicitly allowed because it is a
  test/handler action, not a terminal event mapping.

## Validation

- `cargo fmt --check`
- `cargo build --release`
- `git diff --check`

## Status

Done.
