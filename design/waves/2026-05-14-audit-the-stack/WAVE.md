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

## Pulse map

| Pulse | Status | Findings | Goal |
|---|---|---|---|
| [Pulse 01 - Web silent fallback hardening](plans/pulse-01.md) | closed | R1 F-02, R3 F-01, R5 F-02 | Replaced success-shaped web fallbacks with typed errors for invalid seasons and missing transaction sources. |
| [Pulse 02 - Identity resolution hardening](plans/pulse-02.md) | closed | R3 F-02, R4 F-02 | Stopped ambiguous player-name matches from creating confident wrong player links. |
| [Pulse 03 - Records ownership integrity](plans/pulse-03.md) | planned | R4 F-01, R5 F-02 | Make malformed play-by-play team ownership explicit instead of grouping under a blank team key. |
| [Pulse 04 - Player route cache ownership](plans/pulse-04.md) | planned | R1 F-01 | Keep web player-card requests from mutating the shared active-season repository. |
| [Pulse 05 - TUI storage error surfacing](plans/pulse-05.md) | planned | R6 F-01 | Surface Favorites storage/view-construction failures instead of rendering them as empty state. |
