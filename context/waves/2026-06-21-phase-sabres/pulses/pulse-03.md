# Phase Sabres Pulse 03 - Docs/static-site Matrix Wording Gate

**Date:** 2026-06-21
**Result:** Passed

## Work Completed

- Replaced the docs/reference rollup placeholder with final Phase Sabres
  wording.
- Marked active docs/reference surfaces as done: CLI `docs`, TUI docs overlay,
  Web `/docs`, and dashboard/menu handoffs to the same command reference.
- Kept static-site publishing deferred by design: removed mkdocs/static-site CLI
  commands and `/site/*` remain outside active user-facing docs/reference
  claims.
- Clarified that `icelines-site`, `docs/`, and `mkdocs.yml` remain supporting
  or generated artifacts, not active docs/reference surfaces.

## Validation

- `cargo test -p icelines-core docs_viewmodel_carries_source_metadata_and_rendered_body`
- `cargo test -p icelines-web --test l1_router docs`
- `cargo test -p icelines-cli lp_docs_overlay`
- `cargo test -p icelines-cli --test system_tests site`
- `git diff --check`

## Next Pulse

Pulse 04 closes Phase Sabres with final docs/reference claims and static-site
non-claims.
