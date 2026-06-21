# Phase Sabres

## Scope

Plan and execute the docs/reference truth gate after Phase Senators. The wave
turns the remaining docs/reference rollup placeholder into exact matrix wording
without reviving removed mkdocs/static-site CLI or `/site/*` claims.

## Entry Posture

- `/docs` renders embedded `COMMANDS.md` through `DocsView`.
- The TUI docs overlay renders the same embedded command reference.
- `icelines docs` prints the embedded command reference from the binary.
- The old mkdocs/static-site CLI surface and `/site/*` mount were removed.
- `icelines-site`, `docs/`, and `mkdocs.yml` remain supporting artifacts, not
  active user-facing docs/reference surfaces.
- The active partial rollup still has placeholder language for a docs/reference
  pulse instead of a final Sabres disposition.

## Goals

1. Inventory current docs/reference surfaces and removed static-site claims.
2. Validate focused docs/reference evidence across core, CLI/TUI, and web.
3. Tighten the docs/reference rollup and static-site artifact wording so the
   matrix cannot be read as advertising the removed mkdocs surface.
4. Close the phase with exact final wording and non-claims.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Sabres goals | passed; see `SABRES-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Docs/reference evidence gate | pending |
| 03 | Docs/static-site matrix wording gate | pending |
| 04 | Close Phase Sabres | pending |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused docs/reference tests.
- No live mkdocs or network dependency in tests.
