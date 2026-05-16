---
wave: guard-the-gates
pulse: 04
date: 2026-05-16
status: complete
governing_roles:
  - bench
  - glass
  - wire
---

# Pulse 04 - Release Docs and Backlog Truth

## Goal

Make user/developer docs and backlog status match the final gate behavior.

## Owned Scope

- Update `README.md`, `COMMANDS.md`, and `design/release-checklist.md` with the
  final local/CI audit gate and any advisory policy.
- Move the Tier 3 backlog row from active/backlog to cleared in
  `design/plans/INDEX.md`.
- Keep docs concise and command-copyable.

## Non-goals

- No CI behavior changes.
- No release tagging.
- No unrelated README cleanup.

## Gates

- [x] proof on touched docs
- [x] `cargo fmt --check`
- [x] `git diff --check`

## Stop Conditions

- Stop if docs would claim cargo-audit is blocking before Pulse 02/03 gates prove
  it locally.

## Result

Updated README and COMMANDS with `ci-audit` as the command-copyable local
dependency vulnerability gate. Clarified that the release `ci` slice includes
the audit gate, moved the Tier 3 backlog row into "Cleared in Guard the Gates",
and kept the warning-class advisory ledger anchored in the release checklist.
