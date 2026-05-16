---
wave: guard-the-gates
pulse: 02
date: 2026-05-16
status: planned
governing_roles:
  - bench
  - forge
  - wire
---

# Pulse 02 - Cargo Audit CI and Local Slice

## Goal

Add a reproducible cargo-audit gate to CI and `scripts/test-slice.ps1`.

## Owned Scope

- Add a quality/audit CI job or matrix entry to `.github/workflows/ci.yml`.
- Bootstrap `cargo-audit` through a maintained action or documented cargo install
  path with cache behavior.
- Add a local `ci-audit` slice to `scripts/test-slice.ps1` and include it in the
  `ci` sequence if the baseline is green.
- Update command/help text inside the slice script.

## Non-goals

- No broad CI matrix rewrite.
- No ignored advisories without Pulse 03 policy.
- No release workflow changes unless CI install decisions require shared setup.

## Gates

- [ ] `powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 list`
- [ ] `powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 ci-audit`
- [ ] `cargo fmt --check`
- [ ] `git diff --check`

## Stop Conditions

- Stop if the first local audit run finds advisories that require a policy/fix
  decision before the gate can be blocking.
- Stop if tool installation requires credentials or external services beyond
  public Cargo/RustSec access.
