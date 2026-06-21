# Phase Jets - Transactions route wording gate

> Phase Jets records Transactions route rows with precise scoped wording.

**Created:** 2026-06-21
**Status:** Closed - Phase Jets complete

---

## Frame

The Transactions family row already says CLI, TUI, Web HTML, and Web JSON row
projection build from `TransactionsView`, including the `LEAGUE` teamless
bucket. The route inventory still uses short project wording for the Web rows.

Phase Jets tightens the `/transactions` and `/api/v1/transactions` route rows
without changing route behavior or turning Transactions into a mutation surface.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Jets Goal 1 - Route inventory** | Transactions route rows should name the exact ViewModel, filters, and errors. | A wave inventory names route rows, evidence, and non-claims. |
| 2 | **Jets Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused Transactions route tests pass. |
| 3 | **Jets Goal 3 - Scoped route wording** | Existing wording hides filter metadata and unavailable-source errors. | Route rows name `TransactionsView`, kind/team filters, LEAGUE support, data/meta envelopes, and typed unavailable errors. |
| 4 | **Jets Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not change Transactions runtime behavior.
- Do not add transaction mutation, import, or editing behavior.
- Do not claim live source availability beyond bundled/snapshot fallback.
- Do not broaden Transactions into roster-state or fantasy transaction claims.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused Transactions route tests
   passed.
3. **Pulse 03 - Matrix wording.** Result: route rows now carry scoped
   Transactions route wording.
4. **Pulse 04 - Closeout.** Result: Phase Jets is closed with final route-row
   claims and non-claims recorded.

---

## Closeout

Phase Jets closed the Transactions route wording gate. The two route rows now
record `TransactionsView` projection, kind/team filters, the `LEAGUE` teamless
bucket, data/meta envelope metadata, and typed unavailable-source errors without
claiming mutation or live-source guarantees.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused Transactions Web route tests.
- No live network dependency in tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
