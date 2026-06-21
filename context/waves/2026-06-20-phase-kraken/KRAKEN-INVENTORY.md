# Phase Kraken Inventory

## Purpose

Inventory the practice-focus WP-009 analytics-cache route before deciding
whether it has enough evidence for a bounded prepared-cache practice focus
report claim.

## Current Surface

| Item | Evidence | Kraken posture |
|---|---|---|
| HTML route | `/practice/focus` | Candidate for bounded prepared-cache practice focus report claim, not mandatory drill planning. |
| JSON route | `/api/v1/practice/focus` | Candidate JSON twin for the same bounded claim. |
| Default cache key | `practice_focus:<season>:<type>` | Must stay active-context and prepared-cache only. |
| ViewModel | `AnalyticsCacheConsumerView` as `PracticeFocusReport` | Must preserve source state, quality, methodology, disclosures, non-claims, and metric evidence. |
| Tests | `l2_wp009_practice_focus_*` | Existing first-route coverage; promotion requires copy/workflow review. |

## Promotion Blockers

- Do not claim mandatory drill plans.
- Do not claim autonomous practice prescriptions.
- Do not claim coaching, deployment, roster, matchup, betting, injury, or
  transaction advice.
- Do not claim prediction certainty.
- Do not recompute analytics or fetch live data from the route.
- Do not create cache directories or records on GET when the cache is missing.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Product-copy gate. Result: passed for bounded prepared-cache practice focus report claim.
3. Workflow evidence gate. Result: passed for bounded prepared-cache practice focus report claim.
4. Surface-matrix closeout. Result: passed; Phase Kraken closed.
