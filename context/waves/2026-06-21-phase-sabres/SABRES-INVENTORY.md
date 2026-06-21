# Phase Sabres Inventory

## Purpose

Inventory the docs/reference surfaces before replacing the remaining
docs/reference placeholder wording in the active partial rollup.

## Current Surface

| Area | Evidence | Sabres posture |
|---|---|---|
| Core docs contract | `DocsView` carries source metadata and rendered markdown. | Keep as the shared docs/reference contract. |
| CLI docs | `icelines docs` prints embedded `COMMANDS.md`. | Keep as the offline command reference surface. |
| TUI docs overlay | `show_docs` overlay renders `COMMANDS.md` through `DocsView` and supports scroll/close behavior. | Keep as the in-TUI docs surface. |
| Web docs route | `GET /docs` renders `COMMANDS.md` through `DocsView` and includes current operational guidance. | Keep as the web docs/reference route. |
| Dashboard/menu docs entry points | Dashboard/workbench/menu docs links resolve to `/docs` or embedded docs output. | Keep as handoffs to canonical docs surfaces. |
| Removed mkdocs CLI | `build`, `deploy`, and `site` are absent; `serve --help` labels the static-site CLI surface as removed. | Keep removed; do not revive as active docs/reference surface. |
| Removed `/site/*` mount | Web router comments record the removed mount. | Keep removed unless a future scoped static-site contract reintroduces it. |
| Supporting artifacts | `icelines-site`, `docs/`, and `mkdocs.yml` remain for generated/supporting artifacts. | Keep as supporting artifacts, not active docs/reference user surfaces. |

## Risks to Avoid

- Re-advertising `icelines site serve`, `icelines build`, or `icelines deploy`
  as active commands.
- Treating the supporting `icelines-site` crate as an active CLI/web surface.
- Claiming `/site/*` is mounted.
- Turning the docs/reference row into a static-site publishing claim.
- Weakening the existing `/docs`, TUI overlay, or CLI docs source-of-truth
  relationship with `COMMANDS.md`.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Run focused docs/reference tests.
3. Matrix wording. Replace placeholder rollup wording and preserve the
   deferred static-site artifact row.
4. Closeout. Record final claims and non-claims.
