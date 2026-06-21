# Phase Flyers Pulse 02 - Data Install/Remove Decision

**Date:** 2026-06-20
**Result:** Passed

## Decision

Web data install and web data remove remain deferred and unmounted.

`icelines data install` can perform live release downloads, so browser promotion
needs a dry-run or local-only contract before it is safe to expose. `icelines
data remove` is destructive filesystem mutation, so browser promotion needs a
scoped confirmation contract and target fencing before any route is added.

The existing admin surface keeps the user-facing handoff copy and the route
fence:

- `/admin` labels data install and data remove as deferred operations.
- `/admin/data/install`, `/admin/data/remove`,
  `/api/v1/admin/data/install`, and `/api/v1/admin/data/remove` remain
  unmounted.
- Safe admin operations stay limited to read status, data verify, game-cache
  warmers, snapshot activate/delete, and runtime web config.

## Validation

- `cargo test -p icelines-web --test l1_router l1_admin_data_install_remove_routes_remain_unmounted`
- `git diff --check`

## Residual Risk

This does not promote the surface matrix row. Admin operations remain partial
until persistent report toggles are decided and the final closeout wording is
recorded.

## Next Pulse

Pulse 03 decides whether persistent report-toggle writes remain a CLI/TUI handoff
or receive a shared durable config contract.
