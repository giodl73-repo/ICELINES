# Phase Flyers Career

## Scope

Plan and execute the Career cohort route-row wording gate. The wave does not add
runtime behavior; it records existing read-only local career-history store
projection evidence.

## Entry Posture

- `/career` and `/api/v1/career` validate `league` plus optional `season`,
  `sort`, and `top`.
- Successful routes project local career-history store rows through
  `CareerView`.
- HTML renders through the shared page shell; JSON returns data/meta envelopes.
- Missing local career-history store responses return the shared CLI fetch
  instruction.

## Goals

1. Inventory the Career cohort route rows and evidence.
2. Validate focused Career route evidence.
3. Tighten route-row wording to scoped read-only, query validation,
   local-store, shell/envelope, missing-store guidance, and non-creation claims.
4. Preserve exact non-claims around live fetch, local-store creation, bundled
   career-history availability, dedicated TUI cohort boards, and runtime
   behavior changes.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Flyers Career goals | passed; see `FLYERS-CAREER-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Career cohort route evidence gate | passed; focused route tests support scoped wording, see `pulses/pulse-02.md` |
| 03 | Career cohort route wording gate | passed; rows now carry scoped local-store wording, see `pulses/pulse-03.md` |
| 04 | Close Phase Flyers Career | passed; final scoped claims and non-claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused Career route tests.
- No runtime behavior changes are part of this gate.

## Closeout

Phase Flyers Career is closed. Career route rows now record read-only cohort
leaderboards, query validation, `top` capping, local career-history store
projection through `CareerView`, shared HTML shell and JSON envelopes,
missing-store fetch guidance, and no-live-fetch/no-store-create non-claims.

The claim remains bounded. The rows do not promote runtime changes, dedicated
TUI cohort boards, bundled career-history availability, live fetch, or local
store creation from read navigation.
