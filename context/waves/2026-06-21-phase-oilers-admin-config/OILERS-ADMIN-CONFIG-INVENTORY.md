# Phase Oilers Admin Config Inventory

## Purpose

Inventory admin config mutation route rows before tightening their route wording.

## Current Surface

| Area | Evidence | Oilers Admin Config posture |
|---|---|---|
| JSON set | `POST /api/v1/admin/config/set` | Keep runtime-only `ConfigMutationIntent::set`, key/value validation, report-toggle rejection, and `MutationResultView`. |
| JSON reset | `POST /api/v1/admin/config/reset` | Keep runtime-only `ConfigMutationIntent::reset`, scoped key reset, and applied/noop `MutationResultView`. |
| HTML set/reset | `POST /admin/config/set`, `POST /admin/config/reset` | Keep HTML twins with same validation, derived result, and `/admin` redirect. |

## Risks to Avoid

- Claiming durable writes to `~/.icelines/config.toml`.
- Claiming persistent report-toggle web writes.
- Claiming derived `web.active_label` can be set/reset directly.
- Changing runtime behavior while performing a wording gate.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused admin config tests cover JSON
   set/reset, HTML set/reset, and report-toggle rejection.
3. Matrix wording. Result: passed; config mutation rows now carry scoped
   runtime-only wording.
4. Closeout. Result: passed; Phase Oilers Admin Config is closed with final
   route-row claims and non-claims recorded.
