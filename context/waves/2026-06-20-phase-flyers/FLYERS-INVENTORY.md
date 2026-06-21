# Phase Flyers Inventory

## Current admin safety posture

| Area | Current state | Flyers disposition |
|---|---|---|
| Data status/list | `GET /admin` and `GET /api/v1/admin/data-status` project `DataStatusView` and do not create local data cache state on missing manifest. | Keep implemented read surface. |
| Data verify | HTML and JSON POST routes use `DataMutationIntent::Verify`, return/derive `MutationResultView`, and reject unknown targets. | Keep implemented safe mutation. |
| Game-cache warmers | HTML and JSON POST routes warm game-cache artifacts and reject invalid requests before network work. | Keep as cache warmers only, not release bundle install/remove. |
| Snapshot activate/delete | HTML and JSON POST routes use `SnapshotMutationIntent`; activate requires sealed snapshots and delete rejects active snapshots. | Keep implemented safe mutations. |
| Runtime web config | HTML and JSON POST routes set/reset `web.active_season` and `web.active_season_type` for the running server only. | Keep runtime-only copy unless a durable config contract is added. |
| Data install | CLI exists, but web install routes are unmounted and `/admin` labels install deferred because it can perform live/network release downloads. | Pulse 02 keeps install deferred and unmounted; no browser contract until a dry-run/local-only design exists. |
| Data remove | CLI exists, but web remove routes are unmounted and `/admin` labels remove deferred because it is destructive filesystem mutation. | Pulse 02 keeps remove deferred and unmounted; no browser contract until scoped confirmation and target fencing exist. |
| Persistent report toggles | TUI Reports overlay and CLI config persist `~/.icelines/config.toml`; web admin exposes warning copy and rejects unknown report-toggle keys. | Decide whether web should remain a handoff or share a durable config contract. |

## Existing focused checks

- `l1_admin_html_renders_operational_viewmodels`
- `l1_admin_html_renders_data_verify_form_for_manifest_rows`
- `l1_admin_data_install_remove_routes_remain_unmounted`
- `l1_admin_report_toggle_json_write_is_rejected_as_deferred`
- `l1_admin_snapshot_delete_json_rejects_active_snapshot`
- `l1_admin_game_cache_json_rejects_invalid_request_before_network`
- `l1_admin_favorites_game_cache_json_rejects_invalid_season_before_network`

## Risks to avoid

- Exposing live/network install behind a casual browser button.
- Exposing destructive data remove without a scoped confirmation contract.
- Treating runtime web config as persistent user config.
- Weakening active-snapshot delete guards.
- Allowing GET-backed admin mutations.

## Pulse map

1. Plan and inventory.
2. Data install/remove decision. Result: passed; install/remove stay deferred and unmounted.
3. Persistent report-toggle decision.
4. Admin safety regression gate.
5. Closeout and surface-matrix claim.
