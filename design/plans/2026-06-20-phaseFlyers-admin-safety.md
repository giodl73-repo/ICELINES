# Phase Flyers - Admin operation safety gate

> Phase Flyers owns the post-Devils admin operation safety gate. It decides
> whether any web admin deferrals can be promoted, or whether the current
> install/remove and persistent report-toggle boundaries should remain durable.

**Created:** 2026-06-20
**Status:** Active - pulse 01 planning opened

---

## Frame

The active surface matrix still marks Admin operations partial. Safe runtime web
config, data verify, snapshot activate/delete, and game-cache warmer paths are
POST-backed and tested. Web data install/remove and persistent report-toggle
writes remain deferred because they cross live-network, destructive filesystem,
or durable config-contract boundaries.

Phase Flyers should not make admin mutation broader just to reduce the count of
partial rows. Its job is to audit the remaining deferrals and either promote a
small safe contract with tests or close the gate with explicit durable
deferrals.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Flyers Goal 1 - Admin inventory refresh** | Existing admin decisions live across the surface matrix and older operation-safety docs. | A wave inventory summarizes implemented, deferred, and blocked admin operations from current code/tests. |
| 2 | **Flyers Goal 2 - Install/remove decision** | Web data install/remove are risky because install can fetch live release data and remove is destructive. | The phase either defines a scoped dry-run/confirmation contract with tests or keeps install/remove unmounted with current deferral copy. |
| 3 | **Flyers Goal 3 - Persistent report-toggle decision** | Web runtime config is not the same as the CLI/TUI durable report config. | The phase either defines a shared persistent config contract with tests or keeps report toggles deferred to TUI/CLI. |
| 4 | **Flyers Goal 4 - Admin safety regression gate** | Existing admin safety depends on many narrow route fences. | Focused route tests prove dangerous routes stay unmounted or new safe paths are explicitly covered. |
| 5 | **Flyers Goal 5 - Closeout matrix claim** | The surface matrix should distinguish safe admin partials from intentionally deferred dangerous operations. | `design/specs/surface-parity.md` has exact final wording and no ambiguous stale admin claim. |

---

## Non-goals

- Do not add browser data install/remove without a scoped confirmation or
  dry-run contract.
- Do not write persistent report toggles from web unless the config contract is
  shared with CLI/TUI and fixture-backed.
- Do not weaken existing POST-only and unknown-target rejection fences.
- Do not promote unrelated dashboard, WP-009 workflow, or Signals cache claims.

---

## Recommended pulse order

1. **Pulse 01 - Plan and inventory.** Record current admin routes, tests,
   implemented safe mutations, and remaining deferrals.
2. **Pulse 02 - Install/remove decision.** Decide whether to keep web data
   install/remove deferred or define a small safe contract.
3. **Pulse 03 - Persistent report-toggle decision.** Decide whether web can
   share durable report config or should keep using TUI/CLI handoff.
4. **Pulse 04 - Regression gate.** Run or add focused admin safety checks for
   the chosen decisions.
5. **Pulse 05 - Closeout.** Update the wave, plan, and surface matrix.

---

## Validation expectations

- Planning/doc-only edits use `git diff --check`.
- Admin behavior changes require focused `icelines-web --test l1_router` route
  tests.
- No live network dependency in tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
