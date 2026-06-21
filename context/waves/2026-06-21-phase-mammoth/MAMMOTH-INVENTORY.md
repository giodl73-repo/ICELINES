# Phase Mammoth Inventory

## Purpose

Inventory Compare, Depth, and Records route rows before tightening their route
wording.

## Current Surface

| Area | Evidence | Mammoth posture |
|---|---|---|
| Compare HTML | `/compare` | Keep `CompareView` rendering, similarity mode through `SimilarPlayersView`, and career trend SVG when both cards have enough career rows. |
| Compare JSON | `/api/v1/compare` | Keep stable data/meta envelopes, card row identity, similarity rows, and shared bad-input error envelopes. |
| Depth HTML | `/depth` | Keep `DepthLeagueView` cross-team depth rendering and dashboard workspace embedding. |
| Depth JSON | `/api/v1/depth` | Keep stable data/meta envelopes, row identity with `DepthLeagueView`, and shared error envelopes. |
| Player records | `/records/player/:id`, `/api/v1/records/player/:id` | Keep metric-aware `PlayerRecordsView` with supported metric query selection. |
| Team records | `/records/team/:abbrev`, `/api/v1/records/team/:abbrev` | Keep metric-aware `TeamRecordsView` with empty-state JSON handoff and cache-load recovery link. |

## Risks to Avoid

- Pulling scoring, streak, analytics-cache, or fantasy route claims into this
  gate.
- Claiming new records metrics.
- Treating markdown export behavior as new Web route behavior.
- Changing runtime behavior while performing a wording gate.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused compare, depth, and records tests
   cover envelopes, row identity, charts, metric selection, and empty-state
   boundaries.
3. Matrix wording. Result: passed; eight route rows now carry scoped wording.
4. Closeout. Result: passed; Phase Mammoth is closed with final route-row claims
   and non-claims recorded.
