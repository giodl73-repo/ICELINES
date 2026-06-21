# Phase Bruins Inventory

## Purpose

Inventory the opponent-scout WP-009 analytics-cache route before deciding
whether it has enough evidence for a bounded prepared-cache scout report claim.

## Current Surface

| Item | Evidence | Bruins posture |
|---|---|---|
| HTML route | `/scout/opponent` | Candidate for bounded prepared-cache scout report claim, not a full scouting suite. |
| JSON route | `/api/v1/scout/opponent` | Candidate JSON twin for the same bounded claim. |
| Default cache key | `opponent_scout:<season>:<type>` | Must stay active-context and prepared-cache only. |
| ViewModel | `AnalyticsCacheConsumerView` as `OpponentScoutReport` | Must preserve source state, quality, methodology, disclosures, non-claims, and metric evidence. |
| Tests | `l2_wp009_opponent_scout_*` | Existing first-route coverage; promotion requires copy/workflow review. |

## Promotion Blockers

- Do not claim a full scouting suite.
- Do not claim an opponent game-plan workflow.
- Do not claim prediction certainty, matchup advice, betting advice, injury
  advice, or autonomous coaching authority.
- Do not recompute analytics or fetch live data from the route.
- Do not create cache directories or records on GET when the cache is missing.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Product-copy gate. Result: passed for bounded prepared-cache scout report claim.
3. Workflow evidence gate. Result: passed for bounded prepared-cache scout report claim.
4. Surface-matrix closeout.
