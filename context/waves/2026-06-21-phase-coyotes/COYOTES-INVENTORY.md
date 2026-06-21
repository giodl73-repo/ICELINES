# Phase Coyotes Inventory

## Purpose

Inventory the Web `/docs` route row before converting terse `done` wording into
scoped `DocsView` route evidence.

## Current Surface

| Area | Evidence | Coyotes posture |
|---|---|---|
| Web docs route | `GET /docs` | Keep embedded `COMMANDS.md` rendering through `DocsView`. |
| Career fetch instruction | `l1_docs_route_includes_career_fetch_instruction` | Keep route evidence that the docs include the career fetch command and `/career` guidance. |
| Shared docs contract | `DocsView` | Keep source metadata/body alignment with CLI docs and the TUI docs overlay. |
| Dashboard/menu handoffs | `/docs` links and handoffs | Keep handoffs to the canonical Web docs route. |
| Removed static site | `/site/*`, mkdocs build/serve/deploy | Keep removed; do not turn `/docs` into a static-site publishing claim. |

## Risks to Avoid

- Re-advertising removed mkdocs/static-site commands.
- Treating `/docs` as a `/site/*` mount.
- Claiming live static-site build/serve/deploy behavior.
- Weakening the shared `COMMANDS.md`/`DocsView` source-of-truth relationship.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused `/docs` route test supports scoped
   `DocsView` wording.
3. Matrix wording. Result: passed; the route row now carries scoped
   docs-reference wording without reviving static-site claims.
4. Closeout. Result: passed; Phase Coyotes is closed with final route-row
   claims and non-claims recorded.
