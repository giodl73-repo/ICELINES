# Phase Capitals Pulse 05 - Closeout

**Date:** 2026-06-20
**Result:** Passed

## Closeout

Phase Capitals is closed.

The phase decided that Signals are not yet eligible for:

- WP-009 analytics-cache publication;
- `StatId` rows;
- `--filter` keys;
- catalog-driven sort keys;
- public cross-team Signal leaderboards.

Signals remain direct `PlayerSignalsView` inspection surfaces:

- `icelines signals <name>` text/JSON;
- `icelines export md signals --player <name>`;
- TUI player-card Signals block;
- Web `/player/:id/signals` and `/api/v1/player/:id/signals`;
- team-scoped `icelines signals-roster --team <ABBR>`.

## Future Promotion Prerequisites

Future promotion requires accepted cache metric keys, source-state,
invalidation, methodology versioning, unavailable-state fixtures, and bounded
catalog/leaderboard product copy that preserves scorer-bias and non-claim
language.

## Validation

- `git diff --check`

## Non-Claims

No implementation promotion was added in Capitals. No cache records, `StatId`
rows, filter keys, leaderboard routes, or ranking copy were introduced.
