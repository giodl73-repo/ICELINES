# Phase Sabres Pulse 02 - Docs/reference Evidence Gate

**Date:** 2026-06-21
**Result:** Passed

## Work Completed

- Validated the core `DocsView` contract carries source metadata and rendered
  body content.
- Validated Web `/docs` still renders current `COMMANDS.md` guidance through
  the docs route.
- Validated the TUI docs overlay render, open, close, scroll, and quit behavior.
- Validated `serve --help` does not advertise the removed `icelines site serve`
  path and labels the mkdocs/static-site CLI surface as removed.
- Restored incidental Cargo lockfile churn from the test run.

## Validation

- `cargo test -p icelines-core docs_viewmodel_carries_source_metadata_and_rendered_body`
  - Result: 1 passed, 0 failed.
- `cargo test -p icelines-web --test l1_router docs`
  - Result: 1 passed, 0 failed, 165 filtered out.
- `cargo test -p icelines-cli lp_docs_overlay`
  - Result: 6 passed, 0 failed.
- `cargo test -p icelines-cli --test system_tests site`
  - Result: 1 passed, 0 failed, 230 filtered out.

## Next Pulse

Pulse 03 updates the docs/reference rollup and static-site row wording without
reviving removed mkdocs/static-site claims.
