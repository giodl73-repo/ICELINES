# Phase Oilers Admin Config

## Scope

Plan and execute the admin config mutation route-row wording gate. The wave does
not add runtime behavior; it records existing runtime-only config mutation
evidence.

## Entry Posture

- Admin config set/reset routes resolve through `ConfigMutationIntent`.
- Only `web.active_season` and `web.active_season_type` are mutable.
- `web.active_label` is derived and persistent report toggles are rejected.
- JSON returns `MutationResultView`; HTML forms redirect back to `/admin`.

## Goals

1. Inventory admin config mutation route rows and evidence.
2. Validate focused admin config mutation route evidence.
3. Tighten route-row wording to scoped runtime-only mutation, validation,
   result/redirect, and deferral claims.
4. Preserve exact non-claims around durable config writes, persistent
   report-toggle writes, derived-key mutation, and runtime behavior changes.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Oilers Admin Config goals | passed; see `OILERS-ADMIN-CONFIG-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Admin config mutation route evidence gate | passed; focused route tests support scoped wording, see `pulses/pulse-02.md` |
| 03 | Admin config mutation route wording gate | passed; rows now carry scoped runtime-only wording, see `pulses/pulse-03.md` |
| 04 | Close Phase Oilers Admin Config | passed; final scoped claims and non-claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused admin config mutation route tests.
- No runtime behavior changes are part of this gate.

## Closeout

Phase Oilers Admin Config is closed. Admin config mutation rows now record
runtime-only `WebConfig` set/reset behavior, `ConfigMutationIntent`, allowed
keys, validation, derived-key and report-toggle rejection, JSON
`MutationResultView`, HTML redirects, and durable-config non-claims.

The claim remains bounded. The rows do not promote runtime changes,
`~/.icelines/config.toml` writes, persistent report-toggle mutation, or derived
`web.active_label` writes.
