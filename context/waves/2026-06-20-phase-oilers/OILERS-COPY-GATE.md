# Oilers Named Cache Report Product-Copy Gate

## Decision

The existing named analytics-cache report copy is sufficient for a bounded
generic prepared-cache inspection claim.

This does not promote any specific hockey workflow. It only allows Phase Oilers
to claim that `/reports/analytics-cache` and
`/api/v1/reports/analytics-cache` are named prepared-cache inspection surfaces
when a matching cache record and metric list are provided.

## Accepted Claim

Named analytics cache report may be described as:

- a generic prepared-cache inspection surface;
- requiring an explicit `cache_key` and metric list;
- backed by `AnalyticsCacheConsumerView`;
- preserving source-state, quality, methodology, disclosures, non-claims, and
  metric evidence;
- rendering explicit unavailable state when the named cache record is missing;
- not recomputing analytics or fetching live data on read.

## Still Not Claimed

Named analytics cache report must not be described as:

- a coaching, scouting, player, line, goalie, practice, postgame, or agent
  workflow;
- recommendation authority;
- prediction certainty;
- live analytics recomputation;
- autonomous coaching or agent behavior.

## Evidence

- The shared analytics-cache template states that it reads a named cache record,
  preserves source state, quality, methodology, disclosures, and non-claims, and
  does not recompute analytics or fetch live data.
- Ready-route L2 coverage checks title, metric rendering, source/disclosure
  preservation, non-claim copy, and no recompute/fetch-live copy.
- Missing-route L2 coverage checks explicit unavailable state and no cache
  directory creation on GET.
