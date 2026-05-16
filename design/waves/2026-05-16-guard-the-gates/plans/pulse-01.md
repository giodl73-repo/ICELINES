---
wave: guard-the-gates
pulse: 01
date: 2026-05-16
status: complete
governing_roles:
  - bench
  - forge
  - wire
  - pace
---

# Pulse 01 - CI Gate Inventory and Pulse Map

## Goal

Open the next wave by reconciling the Tier 3 `cargo fmt + cargo audit` backlog
row against current CI, local scripts, and release docs.

## Owned Scope

- Inspect `.github/workflows/ci.yml`, `.github/workflows/release.yml`,
  `scripts/test-slice.ps1`, `design/release-checklist.md`, README/COMMANDS, and
  the backlog row.
- Produce `CI-GATES-INVENTORY.md`.
- Create follow-up pulse plans and role-review notes.
- Mark the wave active in `design/waves/PHASES.md` and the backlog active in
  `design/plans/INDEX.md`.

## Non-goals

- No CI runtime behavior changes.
- No cargo-audit install or ignore policy yet.
- No release tag.

## Gates

- [x] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-16-guard-the-gates design\waves\PHASES.md design\plans\INDEX.md --errors-only`

## Result

Opened Guard the Gates. `cargo fmt --check` is already a blocking CI quality
matrix entry and local `ci-fmt` slice; cargo-audit remains advisory in the
release checklist and needs a reproducible CI/local gate plus policy for any
accepted advisories.
