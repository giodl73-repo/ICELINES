# Pulse 01: Rangers plan and inventory

## Goal

Confirm Phase Rangers goals against existing ICELINES evidence before
implementation starts.

## Result

Status: passed.

The inventory found that two initial goals are already partially implemented by
earlier VTRACE packages:

- WP-002 already provides named workbench layout persistence with accepted risk.
- WP-009 already provides cache-backed evidence-card and analytics consumer
  envelopes for selected Web/API routes.

Phase Rangers therefore shifts from "build these from scratch" to "reuse,
harden, or bridge the existing contracts." The first implementation pulse should
be the NYR workflow proof because it ties the shipped Hurricane surfaces together
without requiring new source claims.

## Evidence

| Evidence | Result |
|---|---|
| `context/waves/2026-05-30-vtrace-wp002-layout/WAVE.md` inspection | WP-002 layout persistence is closed_with_risk, not unstarted. |
| `context/waves/2026-05-30-vtrace-wp002-layout/pulses/pulse-01.md` inspection | Shared layout schema/store, CLI management, TUI restore, and Web bookmark restore evidence exist. |
| `docs/vtrace/WORK_PACKAGES.md` inspection | WP-007 remains target-not-met; WP-009 remains partial with selected evidence-card/cache consumers; WP-010 remains partial with selected Signals surfaces. |
| `design/specs/surface-parity.md` inspection | Signals and player evidence-card surface rows already record current parity posture and residual limits. |
| `RANGERS-INVENTORY.md` | New inventory records Rangers starting posture, blockers, and recommended next pulse. |

## Next pulse

Pulse 02 should implement the NYR workflow proof as a script or
documentation-backed transcript over existing offline surfaces. It should not add
new MoneyPuck, goalie xGA, team-confidence, or Signals promotion claims.
