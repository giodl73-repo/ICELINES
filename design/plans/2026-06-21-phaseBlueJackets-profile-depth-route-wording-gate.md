# Phase Blue Jackets - Profile/depth route wording gate

> Phase Blue Jackets records Player card, Team depth, and Team season route rows
> with precise scoped wording.

**Created:** 2026-06-21
**Status:** Closed - Phase Blue Jackets complete

---

## Frame

Player card, Team depth, and Team season rows already project shared ViewModels,
but several route rows still use short `projects` wording. Phase Blue Jackets
tightens those core profile/depth rows without changing runtime behavior or
promoting adjacent scoring, streak, signals, or fantasy surfaces.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Blue Jackets Goal 1 - Route inventory** | Core profile/depth rows should name ViewModels, charts, envelopes, and boundaries. | A wave inventory names route rows, evidence, and non-claims. |
| 2 | **Blue Jackets Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused player and team route tests pass. |
| 3 | **Blue Jackets Goal 3 - Scoped route wording** | Existing wording hides row-identity, error-envelope, chart, and team-season metric boundaries. | Route rows name `PlayerCardView`, `TeamDepthView`, `TeamSeasonView`, SVG evidence, and focused tests. |
| 4 | **Blue Jackets Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not change player or team runtime behavior.
- Do not include scoring, streaks, signals, scouting, compare, or fantasy rows.
- Do not promote TUI-only `TeamDepthChartView` behavior into Web route claims.
- Do not add historical standings/cache persistence claims for Team season.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused player and team route tests
   passed.
3. **Pulse 03 - Matrix wording.** Result: route rows now carry scoped
   profile/depth route wording.
4. **Pulse 04 - Closeout.** Result: Phase Blue Jackets is closed with final
   route-row claims and non-claims recorded.

---

## Closeout

Phase Blue Jackets closed the profile/depth route wording gate. The six route
rows now record `PlayerCardView`, `TeamDepthView`, and `TeamSeasonView`
projection, row-identity and error-envelope evidence, SVG chart evidence, and
the Team season metric scope while preserving adjacent surface non-claims.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused player and team Web route tests.
- No live network dependency in tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
