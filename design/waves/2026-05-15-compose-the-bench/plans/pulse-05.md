# Pulse 05 - Docs, Regression Gates, and Closeout

## Goal

Close Compose the Bench after pane composition is implemented, documented, and
validated. User-facing docs must explain how center workspaces, side panes,
bound experiences, and active fields compose in both TUI and web.

## Governing roles

- **keel**: docs must describe one shared pane-composition system.
- **glass**: keybinds and web controls must be discoverable and concise.
- **forge**: final gates must compile and lint touched crates.
- **wire**: docs must preserve read/navigation versus POST mutation boundaries.
- **bench**: all pulse gates must be checked before closeout.

## Owned scope

1. Update README and COMMANDS for pane composition controls.
2. Update `design/specs/surface-parity.md` with pane-composition parity notes.
3. Update `WAVE.md` and `design/waves/PHASES.md` closeout records.
4. Run final code/docs/release gates.
5. Commit, push, and verify CI.

## Non-goals

- No new implementation beyond docs/tests needed for closeout.
- No unrelated cleanup.

## Gates

- [x] `cargo fmt --check`
- [x] `cargo test -p icelines-core --quiet`
- [x] `cargo test -p icelines-cli --quiet`
- [x] `cargo test -p icelines-web --quiet`
- [x] `cargo clippy -- -D warnings`
- [x] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-15-compose-the-bench README.md COMMANDS.md design\specs\surface-parity.md --errors-only`
- [x] `cargo build --release -p icelines-cli`

## Result

Closed Compose the Bench after documenting shared pane composition in README,
COMMANDS, and the surface-parity matrix. The closeout gates passed, including
the release CLI build.
