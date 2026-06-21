# Phase Whalers - Favorites group editor

> Promote named Favorites group editing from CLI/TUI-only to POST-backed web
> forms and JSON mutation routes without turning dashboard commands or GET
> navigation into mutations.

**Status:** Implemented - Phase Whalers complete

## Goals

| Goal | Why | Result |
|---|---|---|
| 1. Web group management | `/favorites?group=<name>` was read-only despite local SQLite group contracts already existing. | `/favorites` now exposes group create, rename, delete, and selected-group member add/remove forms. |
| 2. JSON mutation twins | API consumers need stable mutation results matching other web mutations. | `/api/v1/favorites/groups/*` routes return `MutationResultView`. |
| 3. Guard canonical Favorites | The default group anchors dashboard panes, cache warmers, and shortcut mutations. | Rename/delete for `Favorites` is rejected. |
| 4. Preserve GET safety | Group edits must not become command-bar redirects or GET side effects. | Dashboard group commands still open read views; all edits are POST-only. |

## Validation

- `cargo test -p icelines-web --test l1_router favorites`

## Closeout

Phase Whalers closes the named Favorites group editing gap for browser surfaces.
It does not add watch-rule team/deployment editing, does not make dashboard
command text mutating, and does not change canonical Favorites add/remove career
augmentation behavior.
