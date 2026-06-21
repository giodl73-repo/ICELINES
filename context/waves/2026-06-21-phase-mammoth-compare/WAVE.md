# Phase Mammoth Compare

## Scope

Plan and execute the Compare route-row wording gate. The wave does not add
runtime behavior; it records existing `/compare` and `/api/v1/compare`
evidence.

## Entry Posture

- `/compare` and `/api/v1/compare` project `CompareView`.
- Similarity mode projects `SimilarPlayersView`.
- Selected-player mode preserves card row identity.
- HTML renders a career trend SVG when both players have enough loaded career
  rows.
- Bad input uses the shared error envelope.

## Goals

1. Inventory Compare route rows and evidence.
2. Validate focused Compare HTML/JSON route evidence.
3. Tighten route-row wording to read-only ViewModel projection, similarity
   rows, selected-card row identity, career SVG, shared envelopes, no-create
   behavior, and adjacent-route non-claims.
4. Preserve exact non-claims around scoring, streaks, records, fantasy, new
   comparison modes, career data creation, and runtime behavior changes.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Mammoth Compare goals | passed; see `MAMMOTH-COMPARE-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Compare route evidence gate | passed; focused route tests support scoped wording, see `pulses/pulse-02.md` |
| 03 | Compare route wording gate | passed; rows now carry scoped compare wording, see `pulses/pulse-03.md` |
| 04 | Close Phase Mammoth Compare | passed; final scoped claims and non-claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused compare Web route tests.
- No runtime behavior changes are part of this gate.

## Closeout

Phase Mammoth Compare is closed. Compare rows now record read-only
`CompareView` and `SimilarPlayersView` projection, selected-card row identity,
career trend SVG evidence, shared bad-input envelopes, no career-data creation,
and adjacent-route non-claims.

The claim remains bounded. The rows do not promote scoring, streaks, records,
fantasy, new comparison modes, career data creation, or runtime behavior
changes.
