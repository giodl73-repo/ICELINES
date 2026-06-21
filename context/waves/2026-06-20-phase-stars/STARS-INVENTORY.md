# Phase Stars Inventory

## Purpose

Inventory the player evidence-card WP-009 analytics-cache route before deciding
whether it has enough evidence for a bounded prepared-cache player
evidence-card claim.

## Current Surface

| Item | Evidence | Stars posture |
|---|---|---|
| HTML route | `/player/evidence-card` | Candidate for bounded prepared-cache player evidence-card claim, not a full player research or deployment workflow. |
| JSON route | `/api/v1/player/evidence-card` | Candidate JSON twin for the same bounded claim. |
| Default cache key | `player_evidence_card:<season>:<type>` | Must stay active-context and prepared-cache only. |
| ViewModel | `AnalyticsCacheConsumerView` as `PlayerEvidenceCard` | Must preserve source state, quality, methodology, disclosures, non-claims, and metric evidence. |
| Tests | `l2_wp009_player_evidence_card_*` | Existing first-route coverage; promotion requires copy/workflow review. |

## Promotion Blockers

- Do not claim a full player-card research workflow.
- Do not claim deployment, transaction, betting, injury, matchup, or roster
  advice.
- Do not claim prediction certainty or autonomous coaching authority.
- Do not recompute analytics or fetch live data from the route.
- Do not create cache directories or records on GET when the cache is missing.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Product-copy gate. Result: passed for bounded prepared-cache player evidence-card claim.
3. Workflow evidence gate. Result: passed for bounded prepared-cache player evidence-card claim.
4. Surface-matrix closeout.
