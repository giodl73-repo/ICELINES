# Web Dashboard Spec Drift

**Date**: 2026-05-10
**Owner**: Ted Lindsay

`design/specs/web-dashboard.md` is a historical King Clancy design record, not
the current route source of truth. Current shipped route truth lives in
`design/specs/surface-parity.md`.

## Current Drift

- `/fantasy` is currently a coming-soon stub.
- `/fantasy/*` is not folded into the main dashboard.
- `/admin/snapshots` is not mounted.
- `/api/v1/reports` is not mounted.
- `/api/v1/admin/snapshots` is not mounted.
- The broad King route list includes several aspirational routes that should not
  be treated as shipped until the surface matrix says so.

## Rule

Do not use `web-dashboard.md` as the source for shipped-route claims. Use
`surface-parity.md` and `icelines-web/tests/ted_lindsay_route_inventory.rs`.

Ted Lindsay closeout should either update the historical spec in place or mark
the stale route sections as deferred once the encoding in that file is cleaned
up safely.
