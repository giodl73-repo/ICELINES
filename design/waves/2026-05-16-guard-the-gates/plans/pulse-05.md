---
wave: guard-the-gates
pulse: 05
date: 2026-05-16
status: complete
governing_roles:
  - bench
  - forge
  - wire
---

# Pulse 05 - Regression Gates and Closeout

## Goal

Close Guard the Gates after the CI audit path, advisory policy, docs, and backlog
truth are complete.

## Owned Scope

- Run focused gates from Pulses 02-04.
- Run proof on touched wave/docs records.
- Mark `WAVE.md`, `plans/pulse-05.md`, and `design/waves/PHASES.md` closed.
- Update session plan/todos.

## Non-goals

- No release tag.
- No new CI gates beyond the planned audit/fmt/clippy/release-smoke set.

## Gates

- [x] `powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 ci-audit`
- [x] `powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 ci-fmt`
- [x] proof on touched docs
- [x] `git diff --check`

## Stop Conditions

- Stop if cargo-audit cannot be reproduced locally.
- Stop if any pulse gates remain unchecked.

## Result

Closed Guard the Gates. The CI/local audit path, warning-class advisory ledger,
README/COMMANDS docs, release checklist, and backlog truth are complete. Closeout
reran the focused audit/fmt/proof/whitespace gates before marking the wave
closed.
