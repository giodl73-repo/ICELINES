# Phase Canadiens - Leaderboard route wording gate

> Phase Canadiens records Leaders and Goalies route rows with precise scoped
> wording.

**Created:** 2026-06-21
**Status:** Closed - Phase Canadiens complete

---

## Frame

The Leaders and Goalies route rows already name `LeadersView` and `GoaliesView`,
but the wording is uneven: Leaders records adapter/chart evidence while Goalies
records advanced workload metrics. Phase Canadiens tightens these rows as one
leaderboard gate without changing route behavior.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Canadiens Goal 1 - Route inventory** | Leaderboard route rows should name ViewModels, filters, charts, and envelopes. | A wave inventory names route rows, evidence, and non-claims. |
| 2 | **Canadiens Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused Leaders and Goalies route tests pass. |
| 3 | **Canadiens Goal 3 - Scoped route wording** | Existing wording hides query/filter and metric boundaries. | Route rows name `LeadersView`, `GoaliesView`, SVG chart evidence, JSON metadata, and goalie advanced metrics. |
| 4 | **Canadiens Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not change leaderboard runtime behavior.
- Do not merge skater and goalie leaderboard contracts.
- Do not add new leaderboard metrics or persistence behavior.
- Do not claim full browser interaction/visual QA beyond route tests.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused Leaders and Goalies route
   tests passed.
3. **Pulse 03 - Matrix wording.** Result: route rows now carry scoped
   leaderboard route wording.
4. **Pulse 04 - Closeout.** Result: Phase Canadiens is closed with final
   route-row claims and non-claims recorded.

---

## Closeout

Phase Canadiens closed the leaderboard route wording gate. The four route rows
now record `LeadersView`/`GoaliesView` projection, query/filter handling,
descriptive SVG chart evidence, JSON envelope metadata, and goalie advanced
workload metrics while keeping skater and goalie contracts distinct.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused Leaders and Goalies Web route tests.
- No live network dependency in tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
