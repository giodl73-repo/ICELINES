---
wave: guard-the-gates
date_open: 2026-05-16
status: active
source: Tier 3 backlog - CI cargo fmt + cargo audit gates
---

# Guard the Gates

## Mission

Make the release/CI quality gates truthful and enforceable: keep the existing
format gate visible, add a documented cargo-audit path, and avoid surprise
security-gate failures by recording any advisory policy explicitly.

## Award Fit

This is a Jim Gregory / Jennings operations-hardening wave. It does not add a
product surface; it makes the runway safer by turning release-gate intent into
repeatable CI/local checks.

## Scope

| Track | Target | Non-goal |
|---|---|---|
| Inventory | Reconcile existing CI/slice/release docs with the backlog row. | Rewrite the whole CI matrix. |
| Audit gate | Add a cargo-audit check to CI and a local slice only after install/config policy is explicit. | Hide advisories or make network-heavy release builds slower without cache/installer discipline. |
| Advisory policy | Add a documented place for ignored advisories with rationale and expiry if needed. | Ignore RUSTSEC findings without explanation. |
| Docs | Update README/COMMANDS/release checklist/backlog truth for the final gate set. | Broad release-process redesign. |
| Closeout | Run focused CI docs/script gates and mark the wave closed. | Cut a release tag. |

## Operating Rules

- `cargo fmt --check` is already enforced in `.github/workflows/ci.yml` and
  `scripts/test-slice.ps1`; do not duplicate it under a second name.
- Any cargo-audit ignore must include advisory ID, reason, owner, and expiry or
  removal condition.
- CI changes must be reproducible locally through `scripts/test-slice.ps1`.
- Do not introduce live product/network tests. Installing audit tooling in CI is
  allowed; fetching NHL/Yahoo/etc. data is not.
- Keep release docs honest: if audit remains advisory for a pulse, say so.

## Pulse Status

| Pulse | Status | Evidence |
|---|---|---|
| 01 - CI gate inventory and pulse map | complete | `CI-GATES-INVENTORY.md`; `plans/pulse-01.md`; `panels/wave-plan-review/` |
| 02 - Cargo audit CI and local slice | complete | `.github/workflows/ci.yml`; `scripts/test-slice.ps1`; cargo-audit baseline exits 0 with three warning-class advisories |
| 03 - Advisory policy and failure messaging | planned | depends on Pulse 02 |
| 04 - Release docs and backlog truth | planned | depends on Pulses 02-03 |
| 05 - Regression gates and closeout | planned | depends on Pulses 02-04 |

## Role Notes

- **bench**: every gate needs a local reproduction path and should fail for real
  regressions, not for missing tool bootstrap.
- **forge**: CI should install tools through maintained actions or cargo
  install with cache discipline; do not add brittle shell shortcuts.
- **wire**: advisory metadata must be explicit because dependency vulnerability
  feeds change outside the repo.
- **pace**: account for runtime cost; audit should not balloon routine test
  feedback without a named reason.

## Current Result

Pulse 01 opened the wave and found that the backlog row is partly stale:
formatting is already a blocking CI/local slice, while cargo-audit was
documented as advisory and had no CI/local slice yet.

Pulse 02 added `cargo audit` to the CI quality matrix through
`taiki-e/install-action@cargo-audit`, cached the Cargo advisory DB, and added a
local `ci-audit` slice that installs `cargo-audit --locked` when missing. The
baseline exits 0 but reports three warning-class advisories for Pulse 03 policy:
`RUSTSEC-2025-0052` (`async-std` via `httpmock`), `RUSTSEC-2024-0436` (`paste`
via `ratatui`), and `RUSTSEC-2026-0002` (`lru` via `ratatui`).

## Next

Execute Pulse 03: decide and document warning-class advisory handling.
