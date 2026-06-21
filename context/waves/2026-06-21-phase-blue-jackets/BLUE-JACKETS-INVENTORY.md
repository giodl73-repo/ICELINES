# Phase Blue Jackets Inventory

## Purpose

Inventory core Player card, Team depth, and Team season route rows before
tightening their route wording.

## Current Surface

| Area | Evidence | Blue Jackets posture |
|---|---|---|
| Player HTML | `/player/:id` | Keep `PlayerCardView` page rendering with headshot fallback, signals link, and career trend SVG when enough career rows exist. |
| Player JSON | `/api/v1/player/:id` | Keep stable success/error envelopes and row identity with `PlayerCardView`. |
| Team depth HTML | `/team/:abbrev` | Keep `TeamDepthView` roster/depth rendering with active-roster skater Pts/82 SVG chart when finite positive rates exist. |
| Team depth JSON | `/api/v1/team/:abbrev` | Keep stable success/error envelopes and row identity with `TeamDepthView`. |
| Team season HTML | `/team/:abbrev/season` | Keep `TeamSeasonView` season-performance rendering distinct from roster/depth and raw schedule list surfaces. |
| Team season JSON | `/api/v1/team/:abbrev/season` | Keep standard data/meta envelope with standings, schedule-strength, and quality-ledger fields. |

## Risks to Avoid

- Pulling scoring, streak, signals, scouting, compare, or fantasy route claims
  into this gate.
- Treating TUI-only `TeamDepthChartView` as a Web route contract.
- Claiming new historical standings/cache persistence for Team season.
- Changing runtime behavior while performing a wording gate.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused player and team route tests cover row
   identity, envelopes, charts, and HTML route affordances.
3. Matrix wording. Result: passed; six route rows now carry scoped wording.
4. Closeout. Result: passed; Phase Blue Jackets is closed with final route-row
   claims and non-claims recorded.
