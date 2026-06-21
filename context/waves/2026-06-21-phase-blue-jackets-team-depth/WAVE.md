# Phase Blue Jackets Team Depth

## Scope

Plan and execute the Team Depth read route-row wording gate. The wave does not
add runtime behavior; it records existing `/team/:abbrev` and
`/api/v1/team/:abbrev` evidence.

## Entry Posture

- HTML and JSON team-depth routes project `TeamDepthView`.
- HTML renders skater/goalie slots and an active-roster Pts/82 SVG chart when
  finite positive rates exist.
- JSON preserves skater/goalie row identity and shared success/error envelopes.
- Unknown-team and bad-active-season errors are shared envelope shapes.
- TUI chart, team-season, scoring, and streak behavior remain separate
  contracts.

## Goals

1. Inventory team-depth route rows and evidence.
2. Validate focused team-depth HTML/JSON route evidence.
3. Tighten route-row wording to `TeamDepthView`, skater/goalie row identity,
   active-roster chart, shared envelopes, error shapes, and adjacent-route
   non-claims.
4. Preserve exact non-claims around TUI chart, team-season/scoring/streak
   routes, live fetch, local-store creation, and runtime behavior changes.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Blue Jackets Team Depth goals | passed; see `BLUE-JACKETS-TEAM-DEPTH-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Team-depth route evidence gate | passed; focused route tests support scoped wording, see `pulses/pulse-02.md` |
| 03 | Team-depth route wording gate | passed; rows now carry scoped team-depth wording, see `pulses/pulse-03.md` |
| 04 | Close Phase Blue Jackets Team Depth | passed; final scoped claims and non-claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused team-depth Web route tests.
- No runtime behavior changes are part of this gate.

## Closeout

Phase Blue Jackets Team Depth is closed. Team-depth rows now record
`TeamDepthView` HTML/JSON projection, skater/goalie row identity,
active-roster Pts/82 SVG evidence, shared success/error envelopes, unknown-team
and bad-active-season errors, and TUI chart/team-season/scoring/streak
non-claims.

The claim remains bounded. The rows do not promote TUI-only chart, team-season,
scoring, streak, live fetch, local-store creation, or runtime behavior changes.
