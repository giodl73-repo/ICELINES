# Phase Flyers Career Inventory

## Purpose

Inventory Career cohort route rows before tightening their route wording.

## Current Surface

| Area | Evidence | Flyers Career posture |
|---|---|---|
| HTML cohort leaderboard | `GET /career` | Keep read-only `league` plus optional `season`/`sort`/`top` query wording, `CareerView` projection, shared shell rendering, 400 guidance, and missing-store fetch instruction. |
| JSON cohort leaderboard | `GET /api/v1/career` | Keep JSON twin wording with data/meta success envelopes, bad-request error envelopes, league/season/sort/count/total metadata, and CLI-aligned missing-store fetch instruction. |
| Local store boundary | `~/.icelines/career_history.json` | Keep optional local-store source wording; GET routes must not fetch live data or create local store state. |

## Risks to Avoid

- Claiming live career-history fetches from GET navigation.
- Claiming local-store creation from read navigation.
- Claiming bundled career-history availability.
- Claiming a dedicated TUI cohort board.
- Changing runtime behavior while performing a wording gate.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused Career route tests cover HTML shell,
   JSON envelopes, missing-store guidance, and row projection from `CareerView`.
3. Matrix wording. Result: passed; Career rows now carry scoped local-store
   cohort wording.
4. Closeout. Result: passed; Phase Flyers Career is closed with final route-row
   claims and non-claims recorded.
