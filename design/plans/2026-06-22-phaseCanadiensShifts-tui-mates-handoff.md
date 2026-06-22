# Phase Canadiens Shifts - TUI mates handoff

Status: Closed

## Intent

Keep the TUI command bar honest when users ask for `:mates`. The handoff should
describe the current roster fallback and locked shift capability, not imply
shift-backed linemate/deployment analysis is available.

## Scope

- Change the `:mates` command-bar handoff copy to mention roster fallback and
  shifts locked off.
- Add a focused TUI command test that rejects the old `linemates/deployment`
  wording.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-cli tui::command::tests::l0_mates_cmdbar_handoff_reports_shift_lock`
- `git diff --check`
