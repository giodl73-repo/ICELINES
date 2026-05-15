# Pulse 05 - Wave Closeout

## Goal

Close Sim the Spark with an honest validation record, final wave status updates,
and handoff notes for any residual Rocket Richard scoring-intelligence work.

## Governing roles

- **pace**: confirm final documentation names every formula and avoids predictive
  overclaiming.
- **scout**: confirm user-facing copy stays descriptive and does not imply
  certainty, odds, or guaranteed finishes.
- **wire**: confirm GET surfaces remain cache/read-only and missing sources are
  explicit.
- **bench**: confirm all pulse gates are checked, CI passes, and any residual
  gaps become future wave/pulse notes rather than untracked debt.

## Owned scope

1. Review Pulses 01-04 and summarize completed deliverables.
2. Run final docs/code gates appropriate to the wave.
3. Mark the wave closed in `WAVE.md` and `design/waves/PHASES.md`.
4. Add residual follow-up notes only if they are outside this wave's scoped
   descriptive outlook contract.
5. Commit, push, and verify CI.

## Non-goals

- No new scoring features.
- No new web/API/CLI/TUI surface wiring.
- No cleanup outside this wave.

## Gates

- [x] `cargo fmt --check`
- [x] `cargo test -p icelines-core --quiet`
- [x] `cargo test -p icelines-fetch --quiet`
- [x] `cargo test -p icelines-web --quiet`
- [x] `cargo clippy -- -D warnings`
- [x] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-15-sim-the-spark README.md COMMANDS.md --errors-only`
- [x] `cargo build --release -p icelines-cli`
