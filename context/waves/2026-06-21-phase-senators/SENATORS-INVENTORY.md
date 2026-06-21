# Phase Senators Inventory

## Purpose

Inventory the individual admin operation rows before converting their plain
partial wording into explicit safe partial-by-design wording.

## Current Surface

| Area | Flyers evidence | Senators posture |
|---|---|---|
| Data status/list | `DataStatusView` renders through `/admin` and `/api/v1/admin/data-status` without creating local cache state when manifest state is missing. | Keep implemented read surface. |
| Data verify | HTML and JSON POST routes resolve `DataMutationIntent::Verify`, reject unknown targets, and return/derive `MutationResultView`. | Keep safe scoped mutation. |
| Game-cache warmers | HTML and JSON POST routes warm game-cache artifacts and reject invalid requests before network work. | Keep as cache warmers only, not release bundle install/remove. |
| Snapshot activate/delete | HTML and JSON POST routes use `SnapshotMutationIntent`; activate requires sealed snapshots and delete rejects active snapshots. | Keep safe scoped mutations. |
| Runtime web config | HTML and JSON POST routes set/reset runtime `web.active_season` and `web.active_season_type` through `ConfigMutationIntent`. | Keep runtime-only web config. |
| Web data install/remove | Routes remain unmounted because install can fetch release data and remove is destructive filesystem mutation. | Keep deferred until a scoped confirmation/dry-run contract exists. |
| Persistent report toggles | Web exposes warning/deferred behavior; CLI/TUI own durable `~/.icelines/config.toml` writes. | Keep deferred until a shared durable config contract exists. |

## Risks to Avoid

- Rewording partial rows as full parity.
- Claiming web data install/remove support while routes remain unmounted.
- Treating runtime web config as durable user config.
- Weakening snapshot delete active-snapshot guards.
- Adding or implying GET-backed mutations.
- Treating game-cache warmers as release bundle install/remove operations.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused `l1_admin_` route tests cover the
   scoped safe-operation claims and durable deferrals.
3. Matrix wording. Convert the individual admin rows to explicit partial by
   design wording if evidence passes.
4. Closeout. Record final claims and non-claims.
