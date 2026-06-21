# Phase Blue Jackets Player Card Inventory

## Purpose

Inventory player-card read route rows before tightening their route wording.

## Current Surface

| Area | Evidence | Blue Jackets Player Card posture |
|---|---|---|
| HTML player card | `GET /player/:id` | Keep `PlayerCardView` projection, headshot fallback, Signals link, aligned profile fields, and career trend SVG when loaded career rows exist. |
| JSON player card | `GET /api/v1/player/:id` | Keep stable success/error envelopes, row identity, bad-active-season/missing-player shapes, and no shared-repo career-window mutation. |

## Risks to Avoid

- Claiming Signals/scoring/streaks/scouting route behavior in player-card rows.
- Claiming live fetch or local-store creation from player-card reads.
- Claiming shared repository mutation from JSON reads.
- Changing runtime behavior while performing a wording gate.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused player-card tests cover HTML headshot,
   Signals link, career SVG, JSON envelope, row identity, error envelopes, and
   no shared-repo career-window mutation.
3. Matrix wording. Result: passed; player-card rows now carry scoped wording.
4. Closeout. Result: passed; Phase Blue Jackets Player Card is closed with final
   route-row claims and non-claims recorded.
