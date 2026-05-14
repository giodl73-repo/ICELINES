---
wave: audit-the-stack
date_open: 2026-05-14
status: active
source: user request for whole-codebase bug detection and architecture review
---

# Audit the Stack

## Mission

Run a role-based bug-detection pass over IceLines after the player-profile wave,
looking for issues like the web Favorites mismatch where one entity type had a
first-class link/highlight and the parallel entity type looked like plain text.

## Scope

| Track | Target | Non-goal |
|---|---|---|
| Surface parity | CLI, TUI, web/API paths expose the same concepts with consistent affordances. | Redesign every screen in one pass. |
| Architecture boundaries | Confirm shared ViewModels/providers are used instead of renderer-local logic. | Rewrite crate structure without a specific finding. |
| Cache/data seams | Look for stale-state, missing-source, and fallback-chain mismatches. | Run live network fetches. |
| Test coverage | Identify missing regression tests for detected bug classes. | Add broad speculative tests without a bug class. |

## Review panel

- **keel**: cross-surface convergence and cache ownership.
- **forge**: crate boundaries, error handling, and Rust safety.
- **wire**: web/API contracts, graceful degradation, and entity link consistency.
- **glass**: user-visible affordances, discoverability, and visual parity.
- **bench**: regression test coverage for found bug classes.
- **edge**: edge cases around identities, dates, caches, and missing data.

## Outputs

Findings live under `panels/whole-codebase-bug-pass/` in the IceLines review
format. Each finding must be actionable and grounded in inspected files.
