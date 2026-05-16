---
wave: guard-the-gates
pulse: 03
date: 2026-05-16
status: complete
governing_roles:
  - wire
  - forge
  - bench
---

# Pulse 03 - Advisory Policy and Failure Messaging

## Goal

Make cargo-audit failures actionable and document any accepted advisory risk.

## Owned Scope

- Decide warning-class policy for the Pulse 02 baseline advisories:
  `RUSTSEC-2025-0052` (`async-std` via `httpmock`), `RUSTSEC-2024-0436`
  (`paste` via `ratatui`), and `RUSTSEC-2026-0002` (`lru` via `ratatui`).
- Either fix warning-class advisories or document why the gate remains
  vulnerability-blocking while warnings are tracked separately.
- If an ignore/config file is introduced, include advisory ID, rationale, owner,
  and expiry/removal condition.
- Update release docs to explain how to handle new RustSec advisories.
- Keep policy scoped to dependency security advisories; do not add unrelated
  lint or benchmark policy.

## Non-goals

- No dependency upgrades unrelated to audit findings.
- No permanent blanket ignores.
- No product code changes unless directly required to remove an advisory.

## Gates

- [x] `powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 ci-audit`
- [x] `cargo fmt --check`
- [x] proof on touched docs
- [x] `git diff --check`

## Stop Conditions

- Stop if an advisory affects a runtime dependency and cannot be fixed or
  time-boxed with an explicit risk owner.

## Result

Made the policy explicit: `cargo audit` is a blocking CI/local gate for
vulnerability advisories, while warning-class advisories remain visible in audit
output and are tracked in `design/release-checklist.md` rather than hidden in an
ignore config. The current warning ledger records owner, rationale, and removal
condition for `RUSTSEC-2025-0052` (`async-std` via test-only `httpmock`),
`RUSTSEC-2024-0436` (`paste` via `ratatui`), and `RUSTSEC-2026-0002` (`lru` via
`ratatui`).

The local `ci-audit` slice now prints remediation guidance before exiting
nonzero, and CI runs the same slice after installing `cargo-audit`.
