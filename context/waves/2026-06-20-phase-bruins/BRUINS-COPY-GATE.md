# Bruins Opponent Scout Product-Copy Gate

## Decision

The existing opponent-scout copy is sufficient for a bounded prepared-cache
scout report claim.

This does not promote a full scouting suite or opponent game-plan workflow. It
only allows Phase Bruins to claim that `/scout/opponent` and
`/api/v1/scout/opponent` are prepared-cache opponent scout report surfaces when
a matching cache record exists.

## Accepted Claim

Opponent scout may be described as:

- an active-context prepared-cache scout report surface;
- backed by `AnalyticsCacheConsumerView` as `OpponentScoutReport`;
- preserving source-state, quality, methodology, disclosures, non-claims, and
  metric evidence;
- rendering explicit unavailable state when the active cache record is missing;
- not recomputing analytics or fetching live data on read.

## Still Not Claimed

Opponent scout must not be described as:

- a full scouting suite;
- an opponent game-plan workflow;
- autonomous coaching authority;
- prediction certainty;
- betting, injury, matchup, or deployment advice;
- live analytics recomputation;
- a replacement for player-card, line, goalie, practice, postgame, or agent
  workflow evidence.

## Evidence

- The shared analytics-cache template states that it reads a named cache record,
  preserves source state, quality, methodology, disclosures, and non-claims, and
  does not recompute analytics or fetch live data.
- The opponent-scout JSON unavailable response tells users to build or restore
  the active opponent-scout analytics cache before using the report.
- Ready-route L2 coverage checks the report title, active cache key, metric
  rendering, JSON twin link, consumer kind, and non-claim copy.
- Missing-route L2 coverage checks explicit unavailable state and no cache
  directory creation on GET.
