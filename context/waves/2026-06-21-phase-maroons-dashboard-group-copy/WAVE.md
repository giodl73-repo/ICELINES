# Phase Maroons Dashboard Group Copy

## Scope

Align dashboard command unsupported group-mutation copy with the current
Favorites group editor routes.

## Entry Posture

- `/favorites` exposes POST-backed group create/rename/delete and member edit
  forms.
- Dashboard command `group show` navigates to read views.
- Dashboard command `group create/delete/rename/member` must remain non-mutating
  because command execution is GET/navigation oriented.

## Goals

1. Update unsupported group mutation recovery copy.
2. Preserve parser rejection for GET-backed group mutations.
3. Add parser test coverage for the `/favorites` recovery target.
4. Close the phase with validation evidence recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Dashboard group copy update | passed; parser recovery text now points to `/favorites` POST forms |
| 02 | Close Phase Maroons | passed; parser safety and copy claims recorded |

## Validation Posture

- `cargo test -p icelines-web dashboard_command_group`
- `cargo fmt --check`
- `git diff --check`

## Closeout

Phase Maroons is closed. Dashboard group mutation commands still reject GET
mutation attempts, but recovery copy now matches the active `/favorites` POST
editor and CLI alternatives.
