# Capitals Catalog, Filter, and Leaderboard Gate

## Decision

Signals are not eligible for `StatId`, `--filter`, or public cross-team
leaderboard promotion yet.

`signals-roster` remains the accepted discovery lane: a team-scoped inspection
matrix that helps users find player Signals cards worth opening. It is not a
ranking surface.

## Why

The current Signal set is useful as descriptive evidence, but not stable enough
to promote into catalog/filter/leaderboard semantics:

- `physical-engagement-rate` is neutral polarity and depends on hit/block
  recording that carries rink scorer bias.
- `puck-management-differential` depends on takeaway/giveaway recording that is
  scorer-dependent and context-light.
- `penalty-drag-rate` mixes penalty types and does not isolate avoidable team
  harm.
- Missing realtime, time-on-ice, or sample-size evidence returns unavailable
  values, not zeros.

`StatId`, filter grammar, and public leaderboards imply stable comparable stats
with deterministic ordering and broad surface reuse. Promoting Signals there now
would make the product read like it is ranking player quality or deployment
fitness, which the Signal copy explicitly rejects.

## Required Future Gate

A later bounded promotion can proceed only if it adds:

- an accepted subset of Signal keys and polarities;
- explicit `StatId` naming and category placement, if catalog promotion is
  chosen;
- filter grammar tests proving unavailable Signal evidence does not behave like
  zero;
- sort/leaderboard tests proving missing evidence sorts last and copy identifies
  the surface as descriptive;
- product-copy review preserving scorer-bias, missing-input, non-prediction,
  non-betting, non-injury, non-deployment, and non-coaching language;
- surface parity decisions for CLI/TUI/Web/Markdown consumers.

## Non-Claims

This decision does not add:

- `StatId` rows;
- `--filter` keys;
- catalog-driven sort keys;
- public cross-team Signal leaderboards;
- player-quality grades;
- deployment recommendations;
- prediction, betting, injury, or autonomous coaching claims.
