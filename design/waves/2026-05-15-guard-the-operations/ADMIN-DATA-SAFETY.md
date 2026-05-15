# Admin Data Safety

Pulse 03 re-audited web admin data operations after the config/report-toggle
contract slice.

## Decision Table

| Operation | Web status | Safety decision |
|---|---|---|
| Data status/list | Exposed as `GET /admin` and `GET /api/v1/admin/data-status` | Read-only `DataStatusView` projection. |
| Data verify | Exposed as `POST /admin/data/verify` and `POST /api/v1/admin/data/verify` | Safe POST-backed mutation through `DataMutationIntent::Verify`; unknown targets are rejected. |
| Game-cache load | Exposed as `POST /admin/game-cache/load` and JSON twin | Cache warmer only. It may fetch official game data for requested teams, but it does not install release bundles or remove local data. Invalid requests are rejected before network work. |
| Favorites game-cache load | Exposed as `POST /admin/game-cache/load-favorites` and JSON twin | Cache warmer only. It uses Favorites membership to warm game-line/scoring-event cache rows. Invalid season input is rejected before network work. |
| Data install | Not exposed on web | Deferred. It performs live/network release downloads and needs a separate local-only/dry-run contract before browser exposure. Use the CLI intentionally. |
| Data remove | Not exposed on web | Deferred. It is destructive filesystem mutation and needs a scoped confirmation contract before browser exposure. |

## Regression Fences

- `/admin` labels game-cache controls as POST-backed cache warmers, not install or
  remove operations.
- `/admin` renders explicit data install/remove deferral copy.
- `/admin/data/install`, `/admin/data/remove`, and JSON twins remain unmounted.
- Game-cache JSON rejects invalid requests before network work.
- Data verify remains POST-backed and rejects unknown targets.
