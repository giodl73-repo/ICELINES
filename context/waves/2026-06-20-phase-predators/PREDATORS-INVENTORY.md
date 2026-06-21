# Phase Predators Inventory

## Purpose

Inventory the postgame WP-009 analytics-cache routes before deciding whether
they have enough evidence for bounded prepared-cache postgame report claims.

## Current Surfaces

| Item | Evidence | Predators posture |
|---|---|---|
| Review HTML route | `/postgame/review` | Candidate for bounded prepared-cache postgame review report claim, not causal blame. |
| Review JSON route | `/api/v1/postgame/review` | Candidate JSON twin for the same bounded claim. |
| Adjustments HTML route | `/postgame/adjustments` | Candidate for bounded prepared-cache postgame adjustment review claim, not correction authority. |
| Adjustments JSON route | `/api/v1/postgame/adjustments` | Candidate JSON twin for the same bounded claim. |
| Default cache keys | `postgame_review:<season>:<type>`, `postgame_adjustments:<season>:<type>` | Must stay active-context and prepared-cache only. |
| ViewModel | `AnalyticsCacheConsumerView` as `PostgameReviewReport` | Must preserve source state, quality, methodology, disclosures, non-claims, and metric evidence. |
| Tests | `l2_wp009_postgame_review_*`, `l2_wp009_postgame_adjustments_*` | Existing first-route coverage; promotion requires copy/workflow review. |

## Promotion Blockers

- Do not claim causal blame assignment.
- Do not claim automatic correction plans.
- Do not claim coaching, deployment, roster, matchup, betting, injury, or
  transaction advice.
- Do not claim prediction certainty.
- Do not recompute analytics or fetch live data from the route.
- Do not create cache directories or records on GET when the cache is missing.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Product-copy gate. Result: passed for bounded prepared-cache postgame report claims.
3. Workflow evidence gate.
4. Surface-matrix closeout.
