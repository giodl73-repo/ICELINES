---
wave: guard-the-gates
pulse: 02
date: 2026-05-16
status: complete
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

- [x] `powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 list`
- [x] `powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 ci-audit`
- [x] `cargo fmt --check`
- [x] `git diff --check`

## Stop Conditions

- Stop if the first local audit run finds advisories that require a policy/fix
  decision before the gate can be blocking.
- Stop if tool installation requires credentials or external services beyond
  public Cargo/RustSec access.

## Result

Added a CI quality-matrix `audit` entry that installs `cargo-audit` with
`taiki-e/install-action@cargo-audit`, runs `cargo audit`, and caches
`~/.cargo/advisory-db` with the existing Cargo cache. Added a matching local
`ci-audit` slice to `scripts/test-slice.ps1`, including missing-tool bootstrap
through `cargo install cargo-audit --locked`, and included it in the serial
`ci` slice after doc tests.

The baseline exits 0. It reports three warning-class advisories for Pulse 03 to
handle explicitly: `RUSTSEC-2025-0052` (`async-std` via `httpmock`),
`RUSTSEC-2024-0436` (`paste` via `ratatui`), and `RUSTSEC-2026-0002` (`lru` via
`ratatui`).
