# Phase Blue Jackets Team Depth Inventory

## Purpose

Inventory team-depth read route rows before tightening their route wording.

## Current Surface

| Area | Evidence | Blue Jackets Team Depth posture |
|---|---|---|
| HTML team depth | `GET /team/:abbrev` | Keep `TeamDepthView` projection, skater/goalie slots, active-roster context, and Pts/82 SVG chart when finite positive rates exist. |
| JSON team depth | `GET /api/v1/team/:abbrev` | Keep stable success/error envelopes, skater/goalie row identity, unknown-team errors, and bad-active-season errors. |

## Risks to Avoid

- Claiming TUI-only `TeamDepthChartView` as a Web route contract.
- Claiming team-season, scoring, or streak behavior in team-depth rows.
- Claiming live fetch or local-store creation from team-depth reads.
- Changing runtime behavior while performing a wording gate.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused team-depth tests cover HTML chart,
   JSON row identity, and shared error envelopes.
3. Matrix wording. Result: passed; team-depth rows now carry scoped wording.
4. Closeout. Result: passed; Phase Blue Jackets Team Depth is closed with final
   route-row claims and non-claims recorded.
