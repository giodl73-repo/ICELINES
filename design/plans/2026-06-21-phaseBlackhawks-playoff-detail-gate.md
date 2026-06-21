# Phase Blackhawks - Playoff bracket/detail gate

> Phase Blackhawks decides whether Playoff bracket/detail can promote from a
> partial row to a bounded `PlayoffsView` detail/export claim.

**Created:** 2026-06-21
**Status:** Active - pulse 02 evidence passed

---

## Frame

Main playoff surfaces are already strong: CLI `playoffs`, TUI Playoffs,
`/playoffs`, and `/api/v1/playoffs` project through `PlayoffsView`. The
remaining partial is the separate detail/export row for TUI series detail and
Markdown `export md series`.

Phase Blackhawks audits that boundary and only promotes the row if focused
evidence supports a bounded claim over bundled playoff game-log detail.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Blackhawks Goal 1 - Detail/export inventory** | Evidence is split across CLI, TUI, Web, and Markdown export. | A wave inventory names supported surfaces and blockers. |
| 2 | **Blackhawks Goal 2 - Evidence gate** | Promotion needs proof that series detail/export render from shared playoff data. | Focused playoff tests pass. |
| 3 | **Blackhawks Goal 3 - Claim boundary** | The claim must not imply live data, predictions, or exhaustive browser drilldown. | Non-claims are recorded before matrix promotion. |
| 4 | **Blackhawks Goal 4 - Matrix closeout** | The surface matrix should reflect the bounded detail/export claim precisely. | `design/specs/surface-parity.md` carries exact final wording. |

---

## Non-goals

- Do not add live playoff fetch/recompute behavior.
- Do not add new Web series-drilldown routes.
- Do not claim predictive playoff momentum or betting analysis.
- Do not infer missing game logs from aggregates.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused CLI/TUI/Web/export evidence
   supports a bounded `PlayoffsView` detail/export claim.
3. **Pulse 03 - Matrix wording.** Promote to bounded detail/export only if
   evidence supports it.
4. **Pulse 04 - Closeout.** Update wave, plan, and indexes.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused playoff tests across touched surfaces.
- Tests stay offline and fixture-backed.
- Child repo commit and push first; TRACKER records only the submodule pointer.
