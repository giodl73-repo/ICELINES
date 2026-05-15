# Pulse 03 - TUI Pane Composition Controls

## Goal

Wire the shared pane binding contract into default TUI MDI. Users should be able
to change pane composition and apply bound experiences without memorizing command
syntax, while `--classic` and `--standalone` keep their compatibility behavior.

## Governing roles

- **keel**: TUI state should store shared pane binding IDs, not duplicate labels.
- **glass**: focus, selected pane, and active composition must be visible in a
  terminal layout without crowding the center workspace.
- **forge**: preserve typed event handling and avoid brittle string matching.
- **wire**: pane controls must not trigger live fetches or mutations.
- **bench**: add tests for focus traversal, pane cycling/selection, bound
  experience application, and compatibility modes.

## Owned scope

1. Extend default MDI state with active bound experience and left/right pane
   composition selections as defined by Pulse 02.
2. Render pane composition labels and affordances in the TUI shell.
3. Add keyboard behavior for pane selection/cycling that respects focus zones.
4. Apply bound experiences to center workspace + panes when possible.
5. Preserve existing command bar shortcuts, `--classic`, and `--standalone`.

## Non-goals

- No web dashboard changes.
- No persistent pane preferences.
- No new hockey data.

## Gates

- [ ] `cargo fmt --check`
- [ ] `cargo test -p icelines-cli --quiet`
- [ ] `cargo clippy -p icelines-cli --no-deps -- -D warnings`
