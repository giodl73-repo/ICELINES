# Phase Blue Jackets Team Depth - Team depth route wording gate

> Phase Blue Jackets Team Depth records the Team Depth HTML and JSON routes with
> precise ViewModel, row-identity, chart, envelope, and adjacent-route
> boundaries.

**Created:** 2026-06-21
**Status:** Closed - Phase Blue Jackets Team Depth complete

---

## Frame

The team-depth routes already project `TeamDepthView`. Phase Blue Jackets Team
Depth tightens the route matrix so the rows name skater/goalie slots,
active-roster context, Pts/82 SVG evidence, JSON row identity, shared
success/error envelopes, unknown-team and bad-active-season errors, and
non-claims around TUI-only `TeamDepthChartView`, team-season, scoring, and
streak routes.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Blue Jackets Team Depth Goal 1 - Route inventory** | Team-depth rows should name ViewModel evidence and adjacent-route boundaries. | A wave inventory names route rows, evidence, and non-claims. |
| 2 | **Blue Jackets Team Depth Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused team HTML/JSON tests pass. |
| 3 | **Blue Jackets Team Depth Goal 3 - Scoped route wording** | Existing rows are accurate but terse for row identity, chart, and error-shape claims. | Rows name `TeamDepthView`, skater/goalie identity, chart evidence, and shared envelopes. |
| 4 | **Blue Jackets Team Depth Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not change team-depth runtime behavior.
- Do not promote TUI-only `TeamDepthChartView` into Web route claims.
- Do not pull team-season, scoring, or streak behavior into team-depth rows.
- Do not add live fetch or local-store creation from team-depth reads.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused team-depth tests passed.
3. **Pulse 03 - Matrix wording.** Result: team-depth rows now carry scoped wording.
4. **Pulse 04 - Closeout.** Result: Phase Blue Jackets Team Depth is closed with final route-row claims and non-claims recorded.

---

## Closeout

Phase Blue Jackets Team Depth closed the team-depth route wording gate. The rows
now record `TeamDepthView` HTML/JSON projection, skater/goalie row identity,
active-roster Pts/82 SVG evidence, shared success/error envelopes, unknown-team
and bad-active-season errors, and TUI chart/team-season/scoring/streak
non-claims.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused team-depth Web route tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
