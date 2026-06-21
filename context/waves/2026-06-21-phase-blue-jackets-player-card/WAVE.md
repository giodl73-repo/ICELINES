# Phase Blue Jackets Player Card

## Scope

Plan and execute the player-card read route-row wording gate. The wave does not
add runtime behavior; it records existing `/player/:id` and
`/api/v1/player/:id` evidence.

## Entry Posture

- HTML and JSON player-card routes project `PlayerCardView`.
- HTML renders headshot/fallback URLs, a Signals handoff, aligned profile
  fields, and a career trend SVG when enough loaded career rows exist.
- JSON preserves row identity and shared success/error envelopes.
- JSON reads do not mutate shared repository career windows.
- Signals, scoring, streaks, and scouting remain separate route contracts.

## Goals

1. Inventory player-card route rows and evidence.
2. Validate focused player-card HTML/JSON route evidence.
3. Tighten route-row wording to `PlayerCardView`, headshot/fallback, Signals
   handoff, career chart, row identity, shared envelopes, no-mutation behavior,
   and adjacent-route non-claims.
4. Preserve exact non-claims around adjacent routes, live fetch, local-store
   creation, shared repository mutation, and runtime behavior changes.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Blue Jackets Player Card goals | passed; see `BLUE-JACKETS-PLAYER-CARD-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Player-card route evidence gate | passed; focused route tests support scoped wording, see `pulses/pulse-02.md` |
| 03 | Player-card route wording gate | passed; rows now carry scoped player-card wording, see `pulses/pulse-03.md` |
| 04 | Close Phase Blue Jackets Player Card | passed; final scoped claims and non-claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused player-card Web route tests.
- No runtime behavior changes are part of this gate.

## Closeout

Phase Blue Jackets Player Card is closed. Player-card rows now record
`PlayerCardView` HTML/JSON projection, headshot fallback, Signals handoff,
career metric/chart evidence, row identity, shared error envelopes, no
shared-repo career-window mutation on read, and adjacent-route non-claims.

The claim remains bounded. The rows do not promote Signals, scoring, streaks,
scouting, live fetch, local-store creation, shared repository mutation, or
runtime behavior changes.
