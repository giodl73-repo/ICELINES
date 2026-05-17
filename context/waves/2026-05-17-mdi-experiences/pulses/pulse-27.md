# Pulse 27: TUI broad-suite hardening

## Goal

Finish cleaning stale MDI Tab test language and harden the broad TUI suite so
parallel command persistence tests do not race on process-wide home variables.

## Changes

- Renamed legacy MDI Tab tests away from no-op wording.
- Added focus assertions for Tab and Shift+Tab in MDI mode.
- Kept the screen-stability assertions so SDI tab cycling stays fenced off.
- Added a thread-local test home override for `GroupDb::open`.
- Moved the command watch-rule persistence test off process-wide HOME mutation.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-cli --bin icelines l0_adams_mdi_tab`
- `cargo test -p icelines-cli --bin icelines tui`
- `git diff --check`

## Status

Done.
