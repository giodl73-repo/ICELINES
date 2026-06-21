# Phase Barons Admin Docs Truth

## Scope

Align the command reference and `/docs` route with the current admin
install/remove contract.

## Entry Posture

- Phase Seals mounted bounded admin install/remove routes.
- `/docs` embeds `COMMANDS.md`.
- One active command reference block still said web install/remove remained
  deferred.

## Goals

1. Replace stale deferred install/remove copy in `COMMANDS.md`.
2. Add a `/docs` route test for the exact confirmation contract.
3. Preserve Seals non-claims around live fetches, arbitrary removal, and report
   toggles.
4. Close the phase with validation evidence recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Admin docs truth cleanup | passed; command reference and `/docs` route test updated |
| 02 | Close Phase Barons | passed; final docs truth claims recorded |

## Validation Posture

- `cargo test -p icelines-web --test l1_router docs`
- `cargo fmt --check`
- `git diff --check`

## Closeout

Phase Barons is closed. The embedded command reference no longer claims web data
install/remove are deferred; it names the bounded `INSTALL <season>` and
`REMOVE <season>` confirmation contract.
