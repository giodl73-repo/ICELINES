# Phase Americans Dashboard Group Docs

## Scope

Align `COMMANDS.md` and `/docs` with the dashboard group command copy from
Phase Maroons.

## Entry Posture

- Dashboard parser rejects group mutations as not GET-backed.
- `/favorites` exposes POST-backed group editor forms.
- `COMMANDS.md` still described `group create` as deferred in the command bar.

## Goals

1. Update command reference copy for dashboard group mutations.
2. Add `/docs` route coverage for the group command recovery text.
3. Preserve GET non-mutation and `/favorites` POST recovery boundaries.
4. Close the phase with validation evidence recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Command reference and docs route update | passed; group command docs now say not GET-backed |
| 02 | Close Phase Americans | passed; final docs truth claims recorded |

## Validation Posture

- `cargo test -p icelines-web --test l1_router docs`
- `cargo fmt --check`
- `git diff --check`

## Closeout

Phase Americans is closed. The embedded command reference and `/docs` route now
point dashboard group mutations to `/favorites` POST forms or `icelines group`,
without implying GET-backed mutation behavior.
