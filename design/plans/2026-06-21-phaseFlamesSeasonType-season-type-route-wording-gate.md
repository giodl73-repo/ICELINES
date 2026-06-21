# Phase Flames Season Type - Season type route wording gate

> Phase Flames Season Type records the global web season-type toggle with
> precise runtime-config, redirect, and durable-config boundaries.

**Created:** 2026-06-21
**Status:** Closed - Phase Flames Season Type complete

---

## Frame

The season-type toggle route already changes only runtime web state. Phase
Flames Season Type tightens the route matrix so the row names
`WebConfig.active_season_type`, whitelisted regular/playoff normalization,
unknown-kind fallback, safe referer redirects, GET read-only behavior, global-nav
affordance, and durable-config/report-toggle non-claims.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Flames Season Type Goal 1 - Route inventory** | The route row should name runtime-only config scope and redirect guards. | A wave inventory names route row, evidence, and boundaries. |
| 2 | **Flames Season Type Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused season-type route tests pass. |
| 3 | **Flames Season Type Goal 3 - Scoped route wording** | Existing row is accurate but terse for config safety. | Row names runtime config, normalization, redirect behavior, GET behavior, nav affordance, and non-claims. |
| 4 | **Flames Season Type Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not persist changes to `~/.icelines/config.toml`.
- Do not add report-toggle writes.
- Do not allow unsafe off-site redirects.
- Do not change runtime behavior.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused season-type tests passed.
3. **Pulse 03 - Matrix wording.** Result: route row now carries scoped
   runtime-toggle wording.
4. **Pulse 04 - Closeout.** Result: Phase Flames Season Type is closed with
   final route-row claims and non-claims recorded.

---

## Closeout

Phase Flames Season Type closed the season-type route wording gate. The row now
records runtime-only web `active_season_type` mutation, whitelisted regular and
playoff normalization, unknown-kind fallback, safe referer redirects, GET
read-only behavior, global-nav affordance, and durable-config/report-toggle
non-claims.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused season-type route tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
