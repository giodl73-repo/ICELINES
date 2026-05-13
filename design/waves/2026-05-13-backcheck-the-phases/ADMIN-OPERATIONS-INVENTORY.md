# Pulse 07 Admin Operations Inventory

This inventory records the Pulse 07 WIRE decision table for admin data, snapshot,
config, and report-toggle parity. The rule is conservative: web admin may expose
only typed, POST-backed mutations that already return or derive
`MutationResultView` and have fixture-backed tests. Live network installs and
unscoped destructive data removal remain deferred.

## Decision table

| Capability | CLI | TUI | Web HTML | Web JSON | Pulse 07 decision |
|---|---|---|---|---|---|
| Data status/list | `icelines data list` plus `DataStatusView` contract for manifest rows | Admin overlay and cmdbar handoff to terminal command | `/admin` renders `DataStatusView` | `GET /api/v1/admin/data-status` returns `DataStatusView` | Implemented read surface. |
| Data verify | `icelines data verify [--all]` resolves `DataMutationIntent::Verify` | Cmdbar handoff; no long-running TUI mutation | `POST /admin/data/verify` derives `MutationResultView` and redirects | `POST /api/v1/admin/data/verify` returns `MutationResultView` | Implemented safe POST mutation; unknown targets rejected. |
| Data install | `icelines data install` resolves `DataMutationIntent::Install` and downloads release data | TUI install status exists for explicit local install flow | Not exposed | Not exposed | Deferred for web: live/network install work needs a separate dry-run/local-only contract and fixture-backed tests. |
| Data remove | `icelines data remove` resolves `DataMutationIntent::Remove` | Cmdbar handoff only | Not exposed | Not exposed | Deferred for web: destructive filesystem removal needs an explicit scoped confirmation contract. |
| Snapshot list/show | `icelines snapshot list/show` project through `SnapshotView` | Admin overlay and cmdbar handoff | `/admin` renders `SnapshotView` | `GET /api/v1/admin/snapshots` returns `SnapshotView` | Implemented read surface. |
| Snapshot activate | `icelines snapshot use` resolves `SnapshotMutationIntent::Activate`; store requires sealed snapshot | Cmdbar handoff only | `POST /admin/snapshots/activate` redirects | `POST /api/v1/admin/snapshots/activate` returns `MutationResultView` | Implemented safe POST mutation; store rejects unsealed snapshots. |
| Snapshot delete | `icelines snapshot delete` resolves `SnapshotMutationIntent::Remove` | Cmdbar handoff only | `POST /admin/snapshots/delete` is rendered only for inactive snapshots | `POST /api/v1/admin/snapshots/delete` returns `MutationResultView` | Implemented POST mutation with backend active-snapshot guard; active delete is rejected. |
| Runtime web config | `icelines config get/set/list/reset` projects through `ConfigView`/`ConfigMutationIntent` for persistent CLI config | Reports/admin overlays plus cmdbar handoff | `POST /admin/config/set` and `/admin/config/reset` update runtime web season/season-type config | JSON twins return `MutationResultView` | Implemented for runtime web keys only: `web.active_season`, `web.active_season_type`; `web.active_label` is derived. |
| Persistent report toggles | `icelines config set/reset` and TUI Reports overlay persist `~/.icelines/config.toml` | `R` Reports overlay persists report-source toggles | Not exposed | Not exposed | Deferred for web: persistent report-toggle UI needs a broader config contract and tests proving parity with TUI/CLI. |

## Focused safety checks

- `l1_admin_html_renders_data_verify_form_for_manifest_rows` asserts the admin
  page renders verify forms but does not expose `/admin/data/install` or
  `/admin/data/remove`.
- `l1_admin_snapshot_delete_json_rejects_active_snapshot` asserts the JSON
  delete mutation preserves the backend active-snapshot guard.
- Existing `admin` route tests cover POST-backed config, snapshot activate/delete,
  and data verify JSON/HTML mutations with temp-home fixtures and no live network.
