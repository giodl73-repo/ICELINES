# Phase Flames Season Type

## Scope

Plan and execute the season-type route-row wording gate. The wave does not add
runtime behavior; it records existing runtime-only season-type toggle evidence.

## Entry Posture

- `POST /season-type/:kind` updates runtime `WebConfig.active_season_type`.
- `regular`, `playoff`, and plural `playoffs` are normalized.
- Unknown kinds fall back safely.
- Redirects honor safe relative/local referers and drop off-site referers.
- GET is method-not-allowed and read-only.

## Goals

1. Inventory the season-type route row and evidence.
2. Validate focused season-type route evidence.
3. Tighten route-row wording to runtime-only config scope, whitelisted
   normalization, safe redirects, GET behavior, global-nav affordance, and
   durable-config non-claims.
4. Preserve exact non-claims around persistent config writes, report-toggle
   writes, unsafe redirects, and runtime behavior changes.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Flames Season Type goals | passed; see `FLAMES-SEASON-TYPE-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Season-type route evidence gate | passed; focused route tests support scoped wording, see `pulses/pulse-02.md` |
| 03 | Season-type route wording gate | passed; row now carries scoped runtime-toggle wording, see `pulses/pulse-03.md` |
| 04 | Close Phase Flames Season Type | passed; final scoped claims and non-claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused season-type route tests.
- No runtime behavior changes are part of this gate.

## Closeout

Phase Flames Season Type is closed. The season-type route row now records
runtime-only web `active_season_type` mutation, whitelisted regular/playoff
normalization, unknown-kind fallback, safe redirects, GET read-only behavior,
global-nav affordance, and durable-config/report-toggle non-claims.

The claim remains bounded. The row does not promote persistent config writes,
report-toggle writes, unsafe redirects, or runtime behavior changes.
