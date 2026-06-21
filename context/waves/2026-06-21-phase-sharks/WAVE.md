# Phase Sharks

## Scope

Plan and execute the analytics-cache player evidence-card and opponent-scout
route-row wording gate. The wave does not reopen the Stars or Bruins promotion
decisions; it records the four individual route rows as bounded prepared-cache
claims instead of plain partial rows.

## Entry Posture

- Phase Bruins promoted opponent scout to a bounded prepared-cache scout report
  claim.
- Phase Stars promoted player evidence card to a bounded prepared-cache player
  evidence-card claim.
- The active rollup already names both promoted workflow families.
- The route inventory still starts `/player/evidence-card`,
  `/api/v1/player/evidence-card`, `/scout/opponent`, and
  `/api/v1/scout/opponent` with plain `partial -` wording.

## Goals

1. Inventory the player evidence-card and opponent-scout route rows and their
   Stars/Bruins evidence.
2. Validate focused analytics-cache evidence for those route pairs.
3. Tighten route-row wording to bounded prepared-cache claims, not broad
   workflow claims.
4. Preserve exact non-claims around live recomputation, cache creation on GET,
   deployment/game-plan authority, prediction certainty, and autonomous coaching.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Sharks goals | passed; see `SHARKS-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Analytics-cache route evidence gate | passed; focused route evidence supports bounded prepared-cache wording, see `pulses/pulse-02.md` |
| 03 | Analytics-cache route wording gate | passed; route rows now carry bounded prepared-cache claims, see `pulses/pulse-03.md` |
| 04 | Close Phase Sharks | passed; final scoped claims and non-claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused analytics-cache Web route tests.
- No live network dependency in tests.

## Closeout

Phase Sharks is closed. The player evidence-card and opponent-scout route rows
now match their Stars/Bruins feature-level promotions: bounded prepared-cache
player evidence-card and scout report claims over active-context cache reads,
ready/unavailable HTML and JSON, preserved source/quality/methodology/disclosure
and non-claim copy, and no cache creation on missing GET reads.

The claim remains bounded. It does not include full player research, scouting
suite, deployment, transaction, opponent game-plan, prediction certainty,
matchup advice, live recomputation, live fetch, or autonomous coaching behavior.
