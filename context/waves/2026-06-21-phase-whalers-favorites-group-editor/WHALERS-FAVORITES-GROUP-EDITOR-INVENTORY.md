# Phase Whalers Favorites Group Editor Inventory

## Surfaces

| Surface | File | Result |
|---|---|---|
| Web handlers | `icelines-web/src/handlers/favorites.rs` | Added form and JSON endpoints for group create/rename/delete/member add/member remove. |
| Mutation boundary | `icelines-web/src/handlers/favorites_data.rs` | Added local SQLite group mutation helper with group-name validation and Favorites rename/delete guards. |
| Router | `icelines-web/src/lib.rs` | Mounted POST HTML and JSON group mutation routes. |
| Template | `icelines-web/templates/favorites.html` | Added group create/rename/delete controls and selected-group member add/remove controls. |
| Tests | `icelines-web/tests/l1_router.rs` | Added JSON mutation round-trip and Favorites delete rejection tests; updated HTML named-group editing evidence. |
| Docs/specs | `COMMANDS.md`, `design/specs/surface-parity.md` | Replaced old read-only/deferred claims with the POST-backed group editor contract. |

## Non-Claims

- Dashboard command text does not create, rename, delete, or edit groups.
- GET `/favorites` and `/api/v1/favorites` remain read-only.
- `Favorites` cannot be renamed or deleted.
- Watch-rule team/deployment editing remains outside this phase.

## Validation

1. `cargo test -p icelines-web --test l1_router favorites`
