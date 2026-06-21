# Phase Avalanche Inventory

## Purpose

Inventory the goalie-readiness WP-009 analytics-cache route before deciding
whether it has enough evidence for a bounded prepared-cache goalie readiness
workload claim.

## Current Surface

| Item | Evidence | Avalanche posture |
|---|---|---|
| HTML route | `/goalies/readiness` | Candidate for bounded prepared-cache goalie readiness workload claim, not start/sit authority. |
| JSON route | `/api/v1/goalies/readiness` | Candidate JSON twin for the same bounded claim. |
| Default cache key | `goalie_readiness:<season>:<type>` | Must stay active-context and prepared-cache only. |
| ViewModel | `AnalyticsCacheConsumerView` as `GoalieReadiness` | Must preserve source state, quality, methodology, disclosures, non-claims, and metric evidence. |
| Tests | `l2_wp009_goalie_readiness_*` | Existing first-route coverage; promotion requires copy/workflow review. |

## Promotion Blockers

- Do not claim injury certainty or medical advice.
- Do not claim goalie start/sit authority.
- Do not claim betting, matchup, deployment, or roster advice.
- Do not claim prediction certainty or autonomous coaching authority.
- Do not recompute analytics or fetch live data from the route.
- Do not create cache directories or records on GET when the cache is missing.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Product-copy gate. Result: passed for bounded prepared-cache goalie readiness workload claim.
3. Workflow evidence gate.
4. Surface-matrix closeout.
