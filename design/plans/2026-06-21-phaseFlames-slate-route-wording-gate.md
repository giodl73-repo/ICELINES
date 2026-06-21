# Phase Flames - Slate route wording gate

> Phase Flames records Scores, Schedule, and Playoffs route rows with precise
> scoped wording.

**Created:** 2026-06-21
**Status:** Closed - Phase Flames complete

---

## Frame

The Scores, Schedule, and Playoffs route rows already name the shared
ViewModels, but the wording is still terse and does not consistently spell out
the HTML/JSON route evidence or live-source failure boundary.

Phase Flames tightens those route rows without changing route behavior or
claiming live-network success.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Flames Goal 1 - Route inventory** | Slate route rows should name the exact ViewModels and evidence. | A wave inventory names route rows, evidence, and non-claims. |
| 2 | **Flames Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused Scores, Schedule, and Playoffs route tests pass. |
| 3 | **Flames Goal 3 - Scoped route wording** | Existing wording hides date/range/season query and `source_error` boundaries. | Route rows name ViewModels, query handling, envelope metadata, and live-source failure boundaries. |
| 4 | **Flames Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not change Scores, Schedule, or Playoffs runtime behavior.
- Do not add live-network success claims.
- Do not broaden Schedule into TUI-only season-team or matchup projections.
- Do not broaden Playoffs into prediction or bracket-editing behavior.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused slate route tests passed.
3. **Pulse 03 - Matrix wording.** Result: route rows now carry scoped slate
   route wording.
4. **Pulse 04 - Closeout.** Result: Phase Flames is closed with final
   route-row claims and non-claims recorded.

---

## Closeout

Phase Flames closed the Scores/Schedule/Playoffs route wording gate. The six
route rows now record shared ViewModel projection, accepted query parameters,
standard data/meta envelope metadata, and explicit `source_error` handling
without making live-network success claims.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused Scores, Schedule, and Playoffs Web route tests.
- No live network success dependency in tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
