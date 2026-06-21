# Phase Wild Inventory

## Purpose

Inventory the line-combination WP-009 analytics-cache route before deciding
whether it has enough evidence for a bounded prepared-cache line-combination
explorer claim.

## Current Surface

| Item | Evidence | Wild posture |
|---|---|---|
| HTML route | `/lines/explorer` | Candidate for bounded prepared-cache line-combination explorer claim, not deployment advice. |
| JSON route | `/api/v1/lines/explorer` | Candidate JSON twin for the same bounded claim. |
| Default cache key | `line_combination_explorer:<season>:<type>` | Must stay active-context and prepared-cache only. |
| ViewModel | `AnalyticsCacheConsumerView` as `LineCombinationExplorer` | Must preserve source state, quality, methodology, disclosures, non-claims, and metric evidence. |
| Tests | `l2_wp009_line_combination_explorer_*` | Existing first-route coverage; promotion requires copy/workflow review. |

## Promotion Blockers

- Do not claim deployment advice or roster authority.
- Do not claim line-chemistry causality.
- Do not claim betting, injury, matchup, or transaction advice.
- Do not claim prediction certainty or autonomous coaching authority.
- Do not recompute analytics or fetch live data from the route.
- Do not create cache directories or records on GET when the cache is missing.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Product-copy gate.
3. Workflow evidence gate.
4. Surface-matrix closeout.
