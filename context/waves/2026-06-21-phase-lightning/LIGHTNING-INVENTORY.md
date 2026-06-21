# Phase Lightning Inventory

## Purpose

Inventory the Career/cohort route rows before converting their plain partial
wording into partial-by-design wording.

## Current Surface

| Area | Evidence | Lightning posture |
|---|---|---|
| Career HTML | `/career` | Keep Maple Leafs partial-by-design claim: templated Web HTML projects cohort rows from `CareerView` and returns an explicit fetch instruction when the local career-history store is missing. |
| Career JSON | `/api/v1/career` | Keep JSON twin with stable success/bad-request/error envelopes and the same missing-store fetch instruction as CLI. |
| TUI Career | command-bar handoff | Keep handoff-only by design unless a future dedicated TUI cohort board adds value beyond canonical CLI/Web cohort tables. |
| Local career-history store | `~/.icelines/career_history.json` | Keep optional and unbundled; read surfaces must not imply live fetch or bundled career-history availability. |

## Risks to Avoid

- Rewording route rows as a full cross-surface Career board.
- Claiming bundled career-history data availability.
- Claiming live fetch from `/career` or `/api/v1/career` reads.
- Creating local career-history state from read surfaces.
- Weakening the explicit `icelines fetch career --bundled-seasons 5`
  instruction for cold installs.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Run focused Career Web route tests.
3. Matrix wording. Convert the two route rows to partial-by-design wording if
   evidence passes.
4. Closeout. Record final claims and non-claims.
