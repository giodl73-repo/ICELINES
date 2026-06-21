# Phase Oilers Admin Config - Admin config mutation route wording gate

> Phase Oilers Admin Config records runtime web-config mutation rows with
> precise in-memory and deferral boundaries.

**Created:** 2026-06-21
**Status:** Closed - Phase Oilers Admin Config complete

---

## Frame

The admin config mutation routes already mutate only runtime web config. Phase
Oilers Admin Config tightens the route matrix so the rows name
`ConfigMutationIntent`, allowed keys, validation, derived-key rejection,
`MutationResultView`, HTML redirects, and the persistent report-toggle deferral.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Oilers Admin Config Goal 1 - Route inventory** | Config mutation rows should name runtime-only key scope and deferrals. | A wave inventory names route rows, evidence, and non-claims. |
| 2 | **Oilers Admin Config Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused admin config mutation tests pass. |
| 3 | **Oilers Admin Config Goal 3 - Scoped route wording** | Existing rows are accurate but terse for config safety. | Rows name intent, validation, result/redirect behavior, and report-toggle non-claims. |
| 4 | **Oilers Admin Config Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not persist web config changes to `~/.icelines/config.toml`.
- Do not add persistent report-toggle writes to web admin.
- Do not allow derived `web.active_label` writes.
- Do not change runtime behavior.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused admin config mutation tests passed.
3. **Pulse 03 - Matrix wording.** Result: config mutation rows now carry scoped
   runtime-only wording.
4. **Pulse 04 - Closeout.** Result: Phase Oilers Admin Config is closed with
   final route-row claims and non-claims recorded.

---

## Closeout

Phase Oilers Admin Config closed the admin config mutation route wording gate.
The rows now record runtime-only `WebConfig` mutation, `ConfigMutationIntent`,
allowed keys, season/season-type validation, derived-key and report-toggle
rejection, `MutationResultView`, HTML redirects, and durable-config non-claims.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused admin config mutation route tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
