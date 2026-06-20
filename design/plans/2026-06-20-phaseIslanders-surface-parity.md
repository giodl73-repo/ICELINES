# Phase Islanders - Surface parity cleanup

> Phase Islanders is the post-Rangers cleanup round for surface truth. It turns
> the remaining UX/admin/docs partials into either verified product surfaces or
> durable deferrals without overstating browser, TUI, cache, or docs coverage.

**Created:** 2026-06-20
**Status:** Active - pulse 03 admin/docs truth pass passed

---

## Frame

Phase Rangers wrapped the post-Hurricane analytics organization work. The
forward roadmap now points at the explicitly partial UX/admin/doc surfaces in
`design/specs/surface-parity.md`.

Islanders should not reopen Hurricane source claims or Rangers Signals cache
decisions. Its job is narrower: make the shipped surface matrix trustworthy,
close stale partial labels where evidence already exists, and fence the
remaining gaps with runnable checks or durable deferral text.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Islanders Goal 1 - Surface parity matrix refresh** | The matrix still carries old draft wording while many rows now have later evidence. | `design/specs/surface-parity.md` has a current status block and a compact list of active partials that matches the implementation and VTRACE posture. |
| 2 | **Islanders Goal 2 - Admin/docs truth pass** | Admin and docs routes are user-facing operational surfaces where over-claiming is risky. | `/admin`, admin JSON routes, `/docs`, and docs/menu references have explicit done/partial/deferred wording and focused route tests or documented deferrals. |
| 3 | **Islanders Goal 3 - Dashboard partial proof** | Dashboard workspace partials are central to browser UX but are still easy to overstate without capture evidence. | A repeatable command or script records selected desktop/mobile dashboard workspace proof, or explicitly keeps live visual capture deferred. |
| 4 | **Islanders Goal 4 - Cache-backed partial rollup** | WP-009 added many partial cache-backed Web/API surfaces. Users need a clear line between first evidence routes and broader workflow claims. | Cache-backed rows summarize which consumers are first-route evidence, which broader workflows remain partial, and what future gate promotes them. |
| 5 | **Islanders Goal 5 - Closeout snapshot** | The phase should leave no ambiguous "partial because stale docs" state. | A closeout updates the plan, wave, surface matrix, and validation notes with no active Islanders pulse remaining. |

---

## Non-goals

- Do not implement Signals cache/catalog/filter/leaderboard promotion.
- Do not perform lean CLI dependency surgery.
- Do not add MoneyPuck deployment columns, GSAx/high-danger save percentage, or
  team confidence bands without their missing source contracts.
- Do not claim full live-browser or interactive TUI coverage unless repeatable
  evidence is captured in this phase.

---

## Recommended pulse order

1. **Pulse 01 - Inventory and plan.** Record the current partial rows, route
   evidence, and stale documentation risks. Result: passed 2026-06-20.
2. **Pulse 02 - Surface matrix refresh.** Update `surface-parity.md` so current
   status and active partials are easy to audit. Result: passed 2026-06-20.
3. **Pulse 03 - Admin/docs truth pass.** Tighten docs/admin wording and tests or
   deferrals for user-facing operations. Result: passed 2026-06-20.
4. **Pulse 04 - Dashboard proof/fence.** Add repeatable selected capture proof
   or record the remaining visual-capture deferral.
5. **Pulse 05 - Cache partial rollup.** Summarize WP-009 first-route evidence
   versus broader workflow claims.
6. **Pulse 06 - Closeout.** Wrap Islanders with validation and tracker snapshot.

---

## Validation expectations

- VTRACE proof check for traceability edits.
- Focused route or script tests for changed web/admin/docs behavior.
- `git diff --check` before committing.
- Child repo commit and push first; TRACKER records only the submodule pointer.
