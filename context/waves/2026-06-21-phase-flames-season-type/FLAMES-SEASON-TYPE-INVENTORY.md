# Phase Flames Season Type Inventory

## Purpose

Inventory the season-type toggle route row before tightening its route wording.

## Current Surface

| Area | Evidence | Flames Season Type posture |
|---|---|---|
| Runtime toggle | `POST /season-type/:kind` | Keep runtime-only `WebConfig.active_season_type` mutation for regular/playoff values. |
| Redirect safety | referer handling tests | Keep safe relative/local redirects and off-site fallback behavior. |
| Read-only GET | method test | Keep GET method-not-allowed and non-mutating. |
| Global nav | base shell test | Keep visible global nav toggle affordance. |

## Risks to Avoid

- Claiming durable writes to `~/.icelines/config.toml`.
- Claiming persistent report-toggle writes.
- Claiming unsafe off-site redirects.
- Changing runtime behavior while performing a wording gate.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused season-type tests cover POST
   mutation, normalization, fallback, redirect safety, GET read-only behavior,
   and global-nav affordance.
3. Matrix wording. Result: passed; route row now carries scoped runtime-toggle
   wording.
4. Closeout. Result: passed; Phase Flames Season Type is closed with final
   route-row claims and non-claims recorded.
