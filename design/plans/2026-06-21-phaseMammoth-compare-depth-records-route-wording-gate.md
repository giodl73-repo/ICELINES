# Phase Mammoth - Compare/depth/records route wording gate

> Phase Mammoth records Compare, Depth, and Records route rows with precise
> scoped wording.

**Created:** 2026-06-21
**Status:** Closed - Phase Mammoth complete

---

## Frame

Compare, Depth, and Records are read-only route families backed by shared
ViewModels. Their route rows already name the right contracts, but some wording
still uses short `projects` phrasing and underspecifies similarity, envelope,
metric, and empty-state evidence.

Phase Mammoth tightens those rows without changing runtime behavior or pulling
in scoring/streak/cache route claims.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Mammoth Goal 1 - Route inventory** | Compare/depth/records rows should name ViewModels, metrics, charts, and envelopes. | A wave inventory names route rows, evidence, and non-claims. |
| 2 | **Mammoth Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused compare, depth, and records route tests pass. |
| 3 | **Mammoth Goal 3 - Scoped route wording** | Existing wording hides similarity, row-identity, metric, and empty-state boundaries. | Route rows name `CompareView`, `SimilarPlayersView`, `DepthLeagueView`, `PlayerRecordsView`, and `TeamRecordsView` evidence. |
| 4 | **Mammoth Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not change Compare, Depth, or Records runtime behavior.
- Do not include scoring, streak, analytics-cache, or fantasy rows.
- Do not add new records metrics.
- Do not promote markdown export behavior into Web route claims beyond existing
  `DepthLeagueView` alignment.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused compare, depth, and records
   route tests passed.
3. **Pulse 03 - Matrix wording.** Result: route rows now carry scoped
   compare/depth/records route wording.
4. **Pulse 04 - Closeout.** Result: Phase Mammoth is closed with final
   route-row claims and non-claims recorded.

---

## Closeout

Phase Mammoth closed the Compare/Depth/Records route wording gate. The eight
route rows now record Compare and similarity ViewModels, Depth row identity and
shared envelopes, Records metric selection, count charts, and empty-state JSON
handoffs while preserving adjacent route-family non-claims.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused compare, depth, and records Web route tests.
- No live network dependency in tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
