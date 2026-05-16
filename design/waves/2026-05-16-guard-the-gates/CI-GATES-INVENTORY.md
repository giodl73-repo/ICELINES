# CI Gates Inventory

## Source backlog

`design/plans/INDEX.md` listed `CI: cargo fmt + cargo audit gates` as a Tier 3
future feature with no blocker.

## Current state

| Gate | Existing status | Evidence | Gap |
|---|---|---|---|
| `cargo fmt --check` | Blocking CI and local slice already exist | `.github/workflows/ci.yml` quality matrix area `fmt`; `scripts/test-slice.ps1 ci-fmt`; `design/release-checklist.md` required local gates | Backlog wording is stale for fmt. |
| `cargo clippy -- -D warnings` | Blocking CI and local slice already exist | `.github/workflows/ci.yml` quality matrix area `clippy`; `scripts/test-slice.ps1 ci-clippy` | No action except preserve. |
| `cargo audit` | Advisory only | `design/release-checklist.md` says cargo audit is not yet blocking | Need CI job/slice plus advisory-ignore policy. |
| Release smoke | Blocking CI release job and local script exist | `.github/workflows/ci.yml` release job; `scripts/release-smoke.ps1`; `scripts/test-slice.ps1 ci-release` | No action except document final relation to audit. |

## Risks to resolve

- Cargo audit depends on the RustSec advisory database, so failures can appear
  without source changes. This is acceptable only with a documented ignore/expiry
  policy for known accepted risk.
- CI must bootstrap `cargo-audit` deterministically. Prefer a maintained install
  action or cached `cargo install`; do not assume the tool is preinstalled on
  `windows-latest`.
- A local slice must mirror CI so developers can reproduce failures before
  pushing.
- If existing dependencies trigger advisories, Pulse 03 must decide whether to
  fix, ignore with expiry, or leave audit advisory until the risk is addressed.

## Pulse map

1. **Pulse 01 - CI gate inventory and pulse map**: open this wave, record current
   gate truth, and split follow-up work.
2. **Pulse 02 - Cargo audit CI and local slice**: add an audit quality job and a
   `scripts/test-slice.ps1` slice that installs/runs the same check.
3. **Pulse 03 - Advisory policy and failure messaging**: add/document cargo-audit
   ignore policy if needed and make failure output actionable.
4. **Pulse 04 - Release docs and backlog truth**: update README/COMMANDS/release
   checklist/plan index to match the final blocking gates.
5. **Pulse 05 - Regression gates and closeout**: run focused CI/script/doc gates
   and close the wave.

## Non-goals

- No release tag.
- No broad migration away from the existing split CI matrix.
- No live product API tests.
