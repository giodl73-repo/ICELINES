# Phase Coyotes

## Scope

Plan and execute the Web `/docs` route-row wording gate. The wave does not add
new docs behavior; it records the existing `GET /docs` route with scoped
`DocsView` wording and static-site non-claims.

## Entry Posture

- Phase Sabres already closed the docs/reference truth gate.
- `GET /docs` renders embedded `COMMANDS.md` through `DocsView`.
- The focused Web route test verifies the career fetch instruction remains
  visible in rendered docs output.
- Removed mkdocs/static-site CLI commands and `/site/*` are not active
  docs/reference surfaces.
- The route inventory still starts the `GET /docs` status with terse `done`
  wording.

## Goals

1. Inventory the `/docs` route row and evidence.
2. Validate focused `/docs` route evidence.
3. Tighten route-row wording to a scoped `DocsView` claim.
4. Preserve exact non-claims around `/site/*` and removed static-site
   publishing commands.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Coyotes goals | passed; see `COYOTES-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | `/docs` route evidence gate | passed; focused docs route test supports scoped wording, see `pulses/pulse-02.md` |
| 03 | `/docs` route wording gate | passed; route row now carries scoped `DocsView` wording, see `pulses/pulse-03.md` |
| 04 | Close Phase Coyotes | passed; final scoped claims and non-claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use the focused `/docs` Web route test.
- No live mkdocs or network dependency in tests.

## Closeout

Phase Coyotes is closed. The `GET /docs` route row now records that the route
renders embedded `COMMANDS.md` through `DocsView`, keeps career fetch
instructions visible, and remains the canonical Web docs route for
dashboard/menu handoffs.

The claim remains bounded. `/docs` is not a `/site/*` static-site mount and does
not revive removed mkdocs build/serve/deploy behavior.
