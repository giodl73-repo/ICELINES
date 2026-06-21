# Phase Mammoth Compare - Compare route wording gate

> Phase Mammoth Compare records the compare HTML and JSON routes with precise
> ViewModel, similarity, chart, envelope, and adjacent-route boundaries.

**Created:** 2026-06-21
**Status:** Closed - Phase Mammoth Compare complete

---

## Frame

The compare routes already project read-only comparison views. Phase Mammoth
Compare tightens the route matrix so the rows name `CompareView`,
`SimilarPlayersView`, selected-card row identity, career trend SVG evidence,
shared bad-input envelopes, no career-data creation, and adjacent scoring,
streak, records, and fantasy non-claims.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Mammoth Compare Goal 1 - Route inventory** | Compare rows should name read-only ViewModels and adjacent-route boundaries. | A wave inventory names route rows, evidence, and non-claims. |
| 2 | **Mammoth Compare Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused compare HTML/JSON tests pass. |
| 3 | **Mammoth Compare Goal 3 - Scoped route wording** | Existing rows are accurate but terse for similarity, chart, row identity, and error-shape claims. | Rows name `CompareView`, `SimilarPlayersView`, career SVG, row identity, and bad-input envelopes. |
| 4 | **Mammoth Compare Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not change compare runtime behavior.
- Do not create career data from compare reads.
- Do not promote scoring, streak, records, or fantasy comparison behavior.
- Do not add new comparison modes.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused compare tests passed.
3. **Pulse 03 - Matrix wording.** Result: compare rows now carry scoped wording.
4. **Pulse 04 - Closeout.** Result: Phase Mammoth Compare is closed with final route-row claims and non-claims recorded.

---

## Closeout

Phase Mammoth Compare closed the compare route wording gate. The rows now record
read-only `CompareView` and `SimilarPlayersView` projection, selected-card row
identity, career trend SVG evidence, shared bad-input envelopes, no career-data
creation, and adjacent scoring/streak/records/fantasy non-claims.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused compare Web route tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
