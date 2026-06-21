# Kraken Practice Focus Product-Copy Gate

## Decision

The existing practice-focus copy is sufficient for a bounded prepared-cache
practice focus report claim.

This does not promote mandatory drill plans or autonomous practice
prescriptions. It only allows Phase Kraken to claim that `/practice/focus` and
`/api/v1/practice/focus` are prepared-cache practice focus report surfaces when
a matching cache record exists.

## Accepted Claim

Practice focus may be described as:

- an active-context prepared-cache practice focus report surface;
- backed by `AnalyticsCacheConsumerView` as `PracticeFocusReport`;
- preserving source-state, quality, methodology, disclosures, non-claims, and
  metric evidence;
- rendering explicit unavailable state when the active cache record is missing;
- not recomputing analytics or fetching live data on read.

## Still Not Claimed

Practice focus must not be described as:

- mandatory drill plans;
- autonomous practice prescriptions;
- coaching, deployment, roster, matchup, betting, injury, or transaction advice;
- prediction certainty;
- live analytics recomputation;
- a replacement for postgame or agent workflow evidence.

## Evidence

- The shared analytics-cache template states that it reads a named cache record,
  preserves source state, quality, methodology, disclosures, and non-claims, and
  does not recompute analytics or fetch live data.
- The practice-focus JSON unavailable response tells users to build or restore
  the active practice-focus analytics cache before using the report.
- Ready-route L2 coverage checks the report title, active cache key, metric
  rendering, JSON twin link, consumer kind, non-claim copy, and absence of
  mandatory-drill-plan or autonomous-practice-prescription copy.
- Missing-route L2 coverage checks explicit unavailable state and no cache
  directory creation on GET.
