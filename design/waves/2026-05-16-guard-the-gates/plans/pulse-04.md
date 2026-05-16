---
wave: guard-the-gates
pulse: 04
date: 2026-05-16
status: planned
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

- [ ] proof on touched docs
- [ ] `cargo fmt --check`
- [ ] `git diff --check`

## Stop Conditions

- Stop if docs would claim cargo-audit is blocking before Pulse 02/03 gates prove
  it locally.
