# Phase Devils - Streaks route wording gate

> Phase Devils records Player and Team streak route rows with precise scoped
> wording.

**Created:** 2026-06-21
**Status:** Closed - Phase Devils complete

---

## Frame

Player and Team streak routes are cache-backed read surfaces. Their route rows
already name the right ViewModels and cache-state boundaries, but the wording
can be more explicit about boxscore/play-by-play sources, cache-load recovery,
shot metrics, shared envelopes, and no local cache creation.

Phase Devils tightens those rows without changing runtime behavior or promoting
scoring report and analytics-cache routes.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Devils Goal 1 - Route inventory** | Streak rows should name ViewModels, source rows, recovery, and no-cache-creation boundaries. | A wave inventory names route rows, evidence, and non-claims. |
| 2 | **Devils Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused streak route tests pass. |
| 3 | **Devils Goal 3 - Scoped route wording** | Existing wording hides shared envelope and shot-metric evidence. | Route rows name `PlayerStreaksView`, `TeamPlayerStreaksView`, cache recovery, source-state, and no-local-cache behavior. |
| 4 | **Devils Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not change streak runtime behavior.
- Do not include scoring report, game detail, or analytics-cache rows.
- Do not infer streaks from season totals.
- Do not create local cache state from GET navigation.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused streak route tests passed.
3. **Pulse 03 - Matrix wording.** Result: route rows now carry scoped streak
   route wording.
4. **Pulse 04 - Closeout.** Result: Phase Devils is closed with final route-row
   claims and non-claims recorded.

---

## Closeout

Phase Devils closed the Streaks route wording gate. The four route rows now
record `PlayerStreaksView` and `TeamPlayerStreaksView`, cached boxscore and
play-by-play source-state, shot metrics, cache-load recovery forms, shared JSON
envelopes, and no local cache creation from read navigation.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused streak route tests.
- No live network dependency in tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
