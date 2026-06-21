# Phase Utah - Scouting/game route wording gate

> Phase Utah records Scouting and Game detail route rows with precise scoped
> wording.

**Created:** 2026-06-21
**Status:** Closed - Phase Utah complete

---

## Frame

Scouting and Game detail routes are read surfaces backed by `ReportView` and
`GameView`. Their route rows are currently accurate but terse. Phase Utah
tightens those rows without changing runtime behavior or pulling in scoring
report routes.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Utah Goal 1 - Route inventory** | Scouting/game rows should name ViewModels, HTML/JSON shape, and source-error boundaries. | A wave inventory names route rows, evidence, and non-claims. |
| 2 | **Utah Goal 2 - Evidence gate** | Wording changes need current route and handler proof. | Focused scouting and game tests pass. |
| 3 | **Utah Goal 3 - Scoped route wording** | Existing wording hides ReportView metadata and GameView fetch-error handling. | Route rows name `ReportView`, player-card backing, `GameView`, and `meta.source_error`. |
| 4 | **Utah Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not change Scouting or Game detail runtime behavior.
- Do not include `/game/:id/scoring` or other scoring report routes.
- Do not claim live game fetch success; failures remain `meta.source_error` or
  rendered error pages.
- Do not add scouting sections or new player-card fields.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused scouting and game tests passed.
3. **Pulse 03 - Matrix wording.** Result: route rows now carry scoped
   scouting/game detail wording.
4. **Pulse 04 - Closeout.** Result: Phase Utah is closed with final route-row
   claims and non-claims recorded.

---

## Closeout

Phase Utah closed the Scouting/Game detail route wording gate. The four route
rows now record player-card-backed `ReportView` scouting output, scouting JSON
metadata, `GameView` boxscore detail rendering, and game JSON `meta.source_error`
handling while preserving scoring-route and live-fetch non-claims.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused scouting and game tests.
- No live network success dependency in tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
