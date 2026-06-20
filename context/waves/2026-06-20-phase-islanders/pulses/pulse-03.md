# Phase Islanders Pulse 03 - Admin/Docs Truth Pass

## Result

Passed. Admin and docs truth now preserves the existing safe operational
deferrals and removes stale static-site wording from active docs surfaces.

## Work completed

- Kept `/admin` operational behavior unchanged: safe runtime config, data
  verify, snapshot activate/delete, and game-cache warmer paths remain
  POST-backed; web data install/remove and persistent report-toggle writes stay
  deferred.
- Updated `design/specs/surface-parity.md` so Docs reference covers the active
  `DocsView` surfaces only, while the old mkdocs/static-site artifact row is
  explicitly deferred.
- Updated `serve --help` wording so it no longer advertises the removed
  `icelines site serve` command.
- Added a CLI system regression that checks `serve --help` names the active web
  dashboard, labels the static-site CLI surface as removed, and does not mention
  `icelines site serve`.

## Validation

```powershell
cargo test -p icelines-cli --test system_tests l2_cmd_serve_help_does_not_advertise_removed_site_serve
cargo test -p icelines-web --test l1_router l1_admin_html_renders_operational_viewmodels
cargo test -p icelines-web --test l1_router l1_docs_route_includes_career_fetch_instruction
git diff --check
```

## Next pulse

Pulse 04 should prove or explicitly fence dashboard workspace partial/browser
capture claims.
