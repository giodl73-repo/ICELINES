# Phase Seals Admin Install/Remove Inventory

| Area | Evidence | Seals posture |
|---|---|---|
| Admin HTML | `GET /admin` | Renders install/remove forms with exact confirmation placeholders while preserving read-only data-status rendering and existing admin controls. |
| JSON install | `POST /api/v1/admin/data/install` | Validates bundled YYYYZZZZ seasons, rejects bad confirmation before mutation, writes embedded bundle files plus manifest, returns `MutationResultView`. |
| HTML install | `POST /admin/data/install` | Reuses the JSON install contract and redirects back to a safe admin return path. |
| JSON remove | `POST /api/v1/admin/data/remove` | Validates YYYYZZZZ season ids, rejects path traversal before mutation, requires exact confirmation, removes only `~/.icelines/seasons/<season>`, returns `MutationResultView`. |
| HTML remove | `POST /admin/data/remove` | Reuses the JSON remove contract and redirects back to a safe admin return path. |
| Non-claims | Route/docs wording | No live source fetch from web install, no non-bundled browser install, no arbitrary filesystem remove, no persistent report-toggle writes. |

## Acceptance

1. Install/remove routes are mounted for JSON and HTML.
2. Bad confirmation rejects before creating installed season state.
3. Confirmed install writes `bios.json`, `stats.json`, `goalie-stats.json`, and `manifest.json` under `bundle-<season>`.
4. Confirmed remove deletes only the requested installed season directory.
5. Path traversal input is rejected before filesystem mutation.
6. Surface matrix and command docs no longer describe install/remove as unmounted.
