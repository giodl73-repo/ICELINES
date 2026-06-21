# Phase Blue Jackets

## Scope

Plan and execute the Player card, Team depth, and Team season route-row wording
gate. The wave does not add runtime behavior; it records existing route evidence
for core profile/depth surfaces.

## Entry Posture

- `/player/:id` and `/api/v1/player/:id` project `PlayerCardView`.
- `/team/:abbrev` and `/api/v1/team/:abbrev` project `TeamDepthView`.
- `/team/:abbrev/season` and `/api/v1/team/:abbrev/season` project
  `TeamSeasonView`.
- Existing tests cover player row identity, shared error envelopes, headshot and
  signals links, player career trend SVGs, team row identity, team error
  envelopes, and team Pts/82 SVG charts.
- Team season route wording should remain distinct from roster/depth and raw
  schedule list claims.

## Goals

1. Inventory the profile/depth route rows and evidence.
2. Validate focused player and team route evidence.
3. Tighten route-row wording to scoped ViewModel, chart, envelope, and metric
   claims.
4. Preserve exact non-claims around adjacent scoring/streak/signals/fantasy
   surfaces, TUI-only `TeamDepthChartView`, and historical Team season
   persistence.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Blue Jackets goals | passed; see `BLUE-JACKETS-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Profile/depth route evidence gate | passed; focused route tests support scoped wording, see `pulses/pulse-02.md` |
| 03 | Profile/depth route wording gate | passed; rows now carry scoped ViewModel wording, see `pulses/pulse-03.md` |
| 04 | Close Phase Blue Jackets | passed; final scoped claims and non-claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused player and team Web route tests.
- No live network dependency in tests.

## Closeout

Phase Blue Jackets is closed. Player card, Team depth, and Team season route rows
now record shared ViewModel projection, chart evidence, row identity, envelope
metadata, and error-envelope boundaries.

The claim remains bounded. The rows do not promote adjacent scoring, streaks,
signals, scouting, compare, fantasy, TUI-only chart, or historical Team season
persistence behavior.
