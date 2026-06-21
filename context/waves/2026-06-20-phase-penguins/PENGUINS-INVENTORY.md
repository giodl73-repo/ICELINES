# Phase Penguins Inventory

## Purpose

Inventory the WP-009 analytics-cache route families before deciding whether any
family has enough evidence for broader workflow promotion.

## Current Consumer Families

| Family | First-route evidence | Penguins posture |
|---|---|---|
| Named cache report | `/reports/analytics-cache`, `/api/v1/reports/analytics-cache`, `l2_wp009_analytics_cache_report_*`. | Keep as generic prepared-cache inspection unless a specific workflow is selected. |
| Coach dashboard | `/coach/dashboard`, `/api/v1/coach/dashboard`, `l2_wp009_coach_dashboard_*`. | Pulse 02 selected this as the promotion-lane candidate for stricter copy/workflow evidence; no broad promotion yet. |
| Opponent scout | `/scout/opponent`, `/api/v1/scout/opponent`, `l2_wp009_opponent_scout_*`. | Candidate lane, but must not imply a full game-plan workflow. |
| Player evidence card | `/player/evidence-card`, `/api/v1/player/evidence-card`, `l2_wp009_player_evidence_card_*`. | Candidate lane, but must not imply full research, deployment, or transaction workflow. |
| Line combinations | `/lines/explorer`, `/api/v1/lines/explorer`, `l2_wp009_line_combination_explorer_*`. | Keep cautious; deployment advice and line-chemistry causality remain blocked. |
| Goalie readiness | `/goalies/readiness`, `/api/v1/goalies/readiness`, `l2_wp009_goalie_readiness_*`. | Keep cautious; injury certainty and start/sit authority remain blocked. |
| Practice focus | `/practice/focus`, `/api/v1/practice/focus`, `l2_wp009_practice_focus_*`. | Keep cautious; mandatory drill plans and autonomous prescriptions remain blocked. |
| Postgame review | `/postgame/review`, `/api/v1/postgame/review`, `l2_wp009_postgame_review_*`. | Keep cautious; causal blame and automatic correction remain blocked. |
| Postgame adjustments | `/postgame/adjustments`, `/api/v1/postgame/adjustments`, `l2_wp009_postgame_adjustments_*`. | Keep cautious; correction authority remains blocked. |
| Agent evidence | `/agents/evidence`, `/api/v1/agents/evidence`, `l2_wp009_agent_evidence_*`. | Keep cautious; autonomous agent action remains blocked. |

## Cross-Family Requirements

- Preserve prepared-cache consumption through `AnalyticsCacheConsumerView`.
- Preserve explicit unavailable states for missing cache records.
- Preserve no cache storage creation on GET.
- Preserve methodology, source-state, quality, disclosure, and non-claim fields.
- Avoid live analytics recomputation in route handlers.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Promotion lane selection. Result: passed; coach dashboard selected as candidate lane.
3. Product-copy gate.
4. Workflow evidence gate.
5. Closeout and surface-matrix claim.
