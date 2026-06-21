# Phase Flyers Pulse 03 - Persistent Report-Toggle Decision

**Date:** 2026-06-20
**Result:** Passed

## Decision

Persistent report-toggle writes remain deferred to the TUI Reports overlay and
CLI config path.

The web runtime config slice intentionally covers only `web.active_season`,
`web.active_season_type`, and derived `web.active_label`. Durable report toggles
still live in the CLI/TUI `Config` type that writes `~/.icelines/config.toml`.
Adding those writes to web admin would either move the durable config contract
to a shared crate or add an explicit persistence bridge; both are larger than
this admin safety gate.

The existing admin surface keeps the handoff and rejection fences:

- `/admin` labels persistent report toggles as managed by the TUI Reports
  overlay.
- `GET /api/v1/admin/config` returns a warning that persistent report toggles
  are deferred on web admin.
- `POST /api/v1/admin/config/set` rejects report-toggle keys such as
  `reports.realtime` as unknown web config keys.
- Runtime web config set/reset remains limited to active season context.

## Validation

- `cargo test -p icelines-web --test l1_router l1_admin_report_toggle_json_write_is_rejected_as_deferred`
- `git diff --check`

## Residual Risk

Admin operations remain partial by design. The final phase wording still needs
to distinguish implemented runtime web config from deferred durable report
config.

## Next Pulse

Pulse 04 runs the focused admin safety regression gate across the chosen
deferrals and safe mutation paths.
