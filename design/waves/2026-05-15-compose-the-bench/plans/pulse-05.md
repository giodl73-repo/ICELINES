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

- [ ] `cargo fmt --check`
- [ ] `cargo test -p icelines-core --quiet`
- [ ] `cargo test -p icelines-cli --quiet`
- [ ] `cargo test -p icelines-web --quiet`
- [ ] `cargo clippy -- -D warnings`
- [ ] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-15-compose-the-bench README.md COMMANDS.md design\specs\surface-parity.md --errors-only`
- [ ] `cargo build --release -p icelines-cli`
