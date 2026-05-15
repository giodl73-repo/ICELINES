# Pulse 05 - Docs, Regression Gates, and Closeout

## Goal

Close Call the Changes after the shared catalog, TUI workbench, and web
workbench are documented, validated, and pushed. The closeout must make the new
default MDI model discoverable in README, COMMANDS, TUI help text, and
surface-parity records.

## Governing roles

- **keel**: final docs must describe one workbench/cross-surface catalog, not two
  unrelated navigation systems.
- **glass**: user-facing docs must explain the zones and the new Tab behavior
  crisply.
- **forge**: final gates must compile and lint the touched crates.
- **wire**: docs must preserve mutation/read boundaries for web dashboard actions.
- **bench**: all pulse gates must be checked; final CI must pass before closeout.

## Owned scope

1. Update README and COMMANDS for the workbench model.
2. Update `design/specs/surface-parity.md` dashboard notes.
3. Update `WAVE.md` and `design/waves/PHASES.md` closeout records.
4. Run final code/docs/release gates.
5. Commit, push, and verify CI.

## Non-goals

- No new feature implementation beyond docs/tests needed for closeout.
- No unrelated cleanup.

## Gates

- [x] `cargo fmt --check`
- [x] `cargo test -p icelines-core --quiet`
- [x] `cargo test -p icelines-cli --quiet`
- [x] `cargo test -p icelines-web --quiet`
- [x] `cargo clippy -- -D warnings`
- [x] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-15-call-the-changes README.md COMMANDS.md design\specs\surface-parity.md --errors-only`
- [x] `cargo build --release -p icelines-cli`
