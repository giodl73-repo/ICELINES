# Penguins Coach Dashboard Product-Copy Gate

## Decision

The existing coach dashboard copy is sufficient for a bounded
prepared-cache dashboard claim.

This does not promote a finished coaching workflow or dashboard suite. It only
allows Phase Penguins to claim that `/coach/dashboard` and
`/api/v1/coach/dashboard` are a prepared-cache coach dashboard surface when a
matching cache record exists.

## Accepted Claim

Coach dashboard may be described as:

- an active-context prepared-cache dashboard surface;
- backed by `AnalyticsCacheConsumerView`;
- preserving source-state, quality, methodology, disclosures, non-claims, and
  metric evidence;
- rendering explicit unavailable state when the active cache record is missing;
- not recomputing analytics or fetching live data on read.

## Still Not Claimed

Coach dashboard must not be described as:

- a finished multi-panel coaching suite;
- autonomous coaching authority;
- prediction accuracy;
- betting, injury, deployment, or player-grade advice;
- live analytics recomputation;
- a replacement for broader scout/player/line/goalie/practice/postgame/agent
  workflow evidence.

## Evidence

- The shared analytics-cache template states that it reads a named cache record,
  preserves source state, quality, methodology, disclosures, and non-claims, and
  does not recompute analytics or fetch live data.
- Ready-route L2 coverage checks the coach title, active cache key, metric
  rendering, JSON twin link, and non-claim copy.
- Missing-route L2 coverage checks explicit unavailable state and no cache
  directory creation on GET.
