# Phase Oilers Inventory

## Purpose

Inventory the named analytics-cache report before deciding whether it has enough
evidence for a bounded generic prepared-cache inspection claim.

## Current Surface

| Item | Evidence | Oilers posture |
|---|---|---|
| HTML route | `/reports/analytics-cache` | Candidate for bounded generic prepared-cache inspection, not a hockey workflow. |
| JSON route | `/api/v1/reports/analytics-cache` | Candidate JSON twin for the same bounded claim. |
| Required query | `cache_key`, `metrics` | Must stay explicit named-cache inspection. |
| ViewModel | `AnalyticsCacheConsumerView` | Must preserve source state, quality, methodology, disclosures, non-claims, and metric evidence. |
| Tests | `l2_wp009_analytics_cache_report_*` | Existing coverage for unavailable and ready behavior without recomputation. |

## Promotion Blockers

- Do not claim a coaching, scouting, player, line, goalie, practice, postgame,
  or agent workflow.
- Do not claim prediction certainty or recommendation authority.
- Do not recompute analytics or fetch live data from the route.
- Do not turn a missing cache into a created cache record on read.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Product-copy gate.
3. Workflow evidence gate.
4. Surface-matrix closeout.
