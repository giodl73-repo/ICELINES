# Canucks Agent Evidence Product-Copy Gate

## Decision

The existing agent-evidence copy is sufficient for a bounded prepared-cache
agent evidence summary claim.

This does not promote autonomous agent action, execute-recommendation behavior,
or broader agent workflow completion. It only allows Phase Canucks to claim that
`/agents/evidence` and `/api/v1/agents/evidence` are prepared-cache agent
evidence summary surfaces when a matching cache record exists.

## Accepted Claim

Agent evidence may be described as:

- an active-context prepared-cache agent evidence summary surface;
- backed by `AnalyticsCacheConsumerView` as `AgentEvidence`;
- preserving source-state, quality, methodology, disclosures, non-claims, and
  metric evidence;
- rendering explicit unavailable state when the active cache record is missing;
- not recomputing analytics or fetching live data on read.

## Still Not Claimed

Agent evidence must not be described as:

- autonomous agent action;
- execute-recommendation behavior or recommendation authority;
- transaction, deployment, roster, matchup, betting, injury, or coaching advice;
- prediction certainty;
- live analytics recomputation;
- broader agent workflow completion.

## Evidence

- The shared analytics-cache template states that it reads a named cache record,
  preserves source state, quality, methodology, disclosures, and non-claims, and
  does not recompute analytics or fetch live data.
- The agent-evidence JSON unavailable response tells users to build or restore
  the active agent-evidence analytics cache before using the summary.
- Ready-route L2 coverage checks the report title, active cache key, metric
  rendering, JSON twin link, consumer kind, non-claim copy, and absence of
  execute-recommendation or autonomous-action copy.
- Missing-route L2 coverage checks explicit unavailable state and no cache
  directory creation on GET.
