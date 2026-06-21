# Phase Maple Leafs - Career/cohort leaders gate

> Phase Maple Leafs decides whether Career/cohort leaders should remain a
> deliberate TUI handoff to canonical CLI/Web surfaces or become a dedicated TUI
> board.

**Created:** 2026-06-20
**Status:** Active - pulse 01 planned

---

## Frame

Career/cohort leaders are useful today through `CareerView`: the CLI
`query career` command, Web `/career`, JSON `/api/v1/career`, and dashboard
workspace summaries all route to the same cohort contract. The TUI surface is
different by design: it provides a command-bar handoff that names the exact CLI
and Web targets because the career-history store is local, optional, and not
bundled on cold install.

Phase Maple Leafs audits that boundary and only changes the matrix if a native
TUI board has evidence-backed value beyond duplicating the canonical table.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Maple Leafs Goal 1 - Career inventory** | Evidence is split across CLI, TUI command-bar, Web, docs, and dashboard routing. | A wave inventory names the surfaces, tests, and blockers. |
| 2 | **Maple Leafs Goal 2 - Evidence gate** | The handoff must be tested enough to remain an intentional partial. | Focused CLI/TUI/Web tests pass and cold-store guidance stays explicit. |
| 3 | **Maple Leafs Goal 3 - TUI board decision** | A native board should not be added just to duplicate the CLI/Web cohort table. | Decision recorded as promote, defer, or keep partial by design. |
| 4 | **Maple Leafs Goal 4 - Matrix closeout** | The surface matrix must distinguish deliberate handoff from missing evidence. | `design/specs/surface-parity.md` carries exact final wording. |

---

## Non-goals

- Do not add live career-history fetches to read surfaces.
- Do not imply `~/.icelines/career_history.json` exists on cold install.
- Do not add a native TUI board unless evidence proves new TUI-specific value.
- Do not promote Career/cohort leaders to fully done while TUI remains a
  handoff.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Run focused CLI/TUI/Web career tests.
3. **Pulse 03 - TUI board decision and matrix wording.** Keep partial by design
   or promote only if the evidence justifies it.
4. **Pulse 04 - Closeout.** Update wave, plan, and indexes.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused career tests across touched surfaces.
- Tests stay offline and fixture-backed.
- Child repo commit and push first; TRACKER records only the submodule pointer.
