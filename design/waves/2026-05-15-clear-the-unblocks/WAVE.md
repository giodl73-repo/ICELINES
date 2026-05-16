---
wave: clear-the-unblocks
date_open: 2026-05-15
status: closed
source: Tier 2 backlog and spec drift after Guard the Operations closeout
---

# Clear the Unblocks

## Mission

Resolve the small backlog items that already have implementation paths, tests, or
explicit blockers. This wave is not a broad product phase; it clears stale
documentation, missing focused tests, and data-path decisions that keep future
feature work from starting cleanly.

## Award Fit

This is a defensive truth-and-unblock wave in the Jennings/Jim Gregory lane:
small fixes that prevent false backlog signals, stale specs, and accidental
scope creep.

## Scope

| Track | Target | Non-goal |
|---|---|---|
| Spec truth | Reconcile specs/index entries that claim tests are missing when tests now exist. | Rewrite historical specs for style. |
| Focused test gaps | Add only small L0/L1 tests where the code path already exists and the spec names the gap. | Introduce new renderers, APIs, or live-network tests. |
| Shift-data decision | Determine whether historical shift bundling is currently actionable. | Start a live data-ingestion project without fixtures and bundle policy. |
| Closeout | Keep README/COMMANDS/spec indexes honest if behavior or status changes. | Hide deferred work by marking blocked items done. |

## Operating Rules

- Prefer truth updates over feature work when code already outpaced the spec.
- Do not add live network tests.
- Do not bundle new data unless source, size, and fallback contracts are explicit.
- Do not change query/scoring behavior while clearing test/doc unblocks.

## Pulse Status

| Pulse | Status | Evidence |
|---|---|---|
| 01 - Small unblock inventory and pulse map | complete | `SMALL-UNBLOCKS-INVENTORY.md`; `plans/pulse-01.md`; `panels/wave-plan-review/` |
| 02 - Headshot and admin-overlay spec truth | complete | `design/specs/headshot-rendering.md`; `design/specs/tui-admin-overlay.md`; `design/plans/INDEX.md`; `plans/pulse-02.md` |
| 03 - Shift-data bundle decision | complete | `design/specs/data-sources.md`; `design/specs/foster-data-architecture.md`; `design/specs/query-engine.md`; `design/plans/INDEX.md`; `icelines-cli/src/commands/mates.rs`; `icelines-cli/src/commands/scouting.rs`; `plans/pulse-03.md` |
| 04 - Docs, regression gates, and closeout | complete | `design/waves/PHASES.md`; `WAVE.md`; `plans/pulse-04.md`; closeout gates |

## Role Notes

- **bench**: distinguish missing tests from stale specs; every real gap needs the
  right tier and no live network.
- **glass**: headshot/admin-overlay docs must match what users see in the TUI.
- **tape**: shift-data claims must not imply bundled coverage that does not
  exist.
- **wire**: shifts remain disabled unless data-source and capability contracts are
  ready.
- **forge**: keep this wave to small, compile-safe changes.

## Current Result

Clear the Unblocks is closed. Pulse 02 corrected stale headshot/admin-overlay
test-coverage docs, Pulse 03 parked historical shift-data bundles behind a future
source/capability contract, and Pulse 04 ran the focused regression/doc gates.

## Next

No active pulse remains in this wave.
