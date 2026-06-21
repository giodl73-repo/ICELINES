# Phase Canadiens Inventory

## Purpose

Inventory the Leaders and Goalies route rows before tightening their route
wording.

## Current Surface

| Area | Evidence | Canadiens posture |
|---|---|---|
| Leaders HTML | `/leaders` | Keep `LeadersView` template rows, query state, dashboard workspace embedding, and Pts/82 SVG chart. |
| Leaders JSON | `/api/v1/leaders` | Keep JSON rows from `LeadersView` with tested success and bad-filter envelope behavior. |
| Goalies HTML | `/goalies` | Keep `GoaliesView` rows with goalie workload metrics, include-below-threshold handling, and SV% SVG chart. |
| Goalies JSON | `/api/v1/goalies` | Keep stable data/meta envelope with sort/top/gp_min metadata and nullable advanced workload fields. |

## Risks to Avoid

- Merging skater and goalie leaderboard contracts.
- Claiming new metrics or persistence behavior.
- Claiming full browser interaction or visual QA beyond route tests.
- Treating dashboard embedding as a separate dashboard behavior change.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused Leaders and Goalies route tests cover
   chart, query, dashboard embedding, envelope, and goalie workload boundaries.
3. Matrix wording. Result: passed; four route rows now carry scoped wording.
4. Closeout. Result: passed; Phase Canadiens is closed with final route-row
   claims and non-claims recorded.
