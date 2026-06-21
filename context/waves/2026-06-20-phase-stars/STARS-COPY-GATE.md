# Stars Player Evidence-Card Product-Copy Gate

## Decision

The existing player evidence-card copy is sufficient for a bounded
prepared-cache player evidence-card claim.

This does not promote a full player research, deployment, or transaction
workflow. It only allows Phase Stars to claim that `/player/evidence-card` and
`/api/v1/player/evidence-card` are prepared-cache player evidence-card surfaces
when a matching cache record exists.

## Accepted Claim

Player evidence card may be described as:

- an active-context prepared-cache player evidence-card surface;
- backed by `AnalyticsCacheConsumerView` as `PlayerEvidenceCard`;
- preserving source-state, quality, methodology, disclosures, non-claims, and
  metric evidence;
- rendering explicit unavailable state when the active cache record is missing;
- not recomputing analytics or fetching live data on read.

## Still Not Claimed

Player evidence card must not be described as:

- a full player-card research workflow;
- transaction, roster, deployment, matchup, betting, or injury advice;
- autonomous coaching authority;
- prediction certainty;
- live analytics recomputation;
- a replacement for line, goalie, practice, postgame, or agent workflow
  evidence.

## Evidence

- The shared analytics-cache template states that it reads a named cache record,
  preserves source state, quality, methodology, disclosures, and non-claims, and
  does not recompute analytics or fetch live data.
- The player evidence-card JSON unavailable response tells users to build or
  restore the active player-evidence-card analytics cache before using the card.
- Ready-route L2 coverage checks the report title, active cache key, metric
  rendering, JSON twin link, consumer kind, and non-claim copy.
- Missing-route L2 coverage checks explicit unavailable state and no cache
  directory creation on GET.
