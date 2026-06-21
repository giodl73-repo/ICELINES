# Avalanche Goalie Readiness Product-Copy Gate

## Decision

The existing goalie-readiness copy is sufficient for a bounded prepared-cache
goalie readiness workload claim.

This does not promote injury certainty, medical advice, or start/sit authority.
It only allows Phase Avalanche to claim that `/goalies/readiness` and
`/api/v1/goalies/readiness` are prepared-cache goalie readiness workload
surfaces when a matching cache record exists.

## Accepted Claim

Goalie readiness may be described as:

- an active-context prepared-cache goalie readiness workload surface;
- backed by `AnalyticsCacheConsumerView` as `GoalieReadiness`;
- preserving source-state, quality, methodology, disclosures, non-claims, and
  metric evidence;
- rendering explicit unavailable state when the active cache record is missing;
- not recomputing analytics or fetching live data on read.

## Still Not Claimed

Goalie readiness must not be described as:

- injury certainty or medical advice;
- goalie start/sit authority;
- deployment, roster, matchup, betting, or transaction advice;
- autonomous coaching authority;
- prediction certainty;
- live analytics recomputation;
- a replacement for practice, postgame, or agent workflow evidence.

## Evidence

- The shared analytics-cache template states that it reads a named cache record,
  preserves source state, quality, methodology, disclosures, and non-claims, and
  does not recompute analytics or fetch live data.
- The goalie-readiness JSON unavailable response tells users to build or restore
  the active goalie-readiness analytics cache before using the workload view.
- Ready-route L2 coverage checks the report title, active cache key, metric
  rendering, JSON twin link, consumer kind, non-claim copy, and absence of
  injury-certainty or deployment-recommendation copy.
- Missing-route L2 coverage checks explicit unavailable state and no cache
  directory creation on GET.
