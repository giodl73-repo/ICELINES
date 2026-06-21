# Phase Canucks Inventory

## Purpose

Inventory the agent-evidence WP-009 analytics-cache route before deciding
whether it has enough evidence for a bounded prepared-cache agent evidence
summary claim.

## Current Surface

| Item | Evidence | Canucks posture |
|---|---|---|
| HTML route | `/agents/evidence` | Candidate for bounded prepared-cache agent evidence summary claim, not autonomous action. |
| JSON route | `/api/v1/agents/evidence` | Candidate JSON twin for the same bounded claim. |
| Default cache key | `agent_evidence:<season>:<type>` | Must stay active-context and prepared-cache only. |
| ViewModel | `AnalyticsCacheConsumerView` as `AgentEvidence` | Must preserve source state, quality, methodology, disclosures, non-claims, and metric evidence. |
| Tests | `l2_wp009_agent_evidence_*` | Existing first-route coverage; promotion requires copy/workflow review. |

## Promotion Blockers

- Do not claim autonomous agent action.
- Do not claim recommendation, transaction, deployment, roster, matchup,
  betting, injury, or coaching authority.
- Do not claim prediction certainty or broader agent workflow completion.
- Do not recompute analytics or fetch live data from the route.
- Do not create cache directories or records on GET when the cache is missing.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Product-copy gate. Result: passed for bounded prepared-cache agent evidence summary claim.
3. Workflow evidence gate. Result: passed for bounded prepared-cache agent evidence summary claim.
4. Surface-matrix closeout. Result: passed; Phase Canucks closed.
