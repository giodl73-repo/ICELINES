# Phase Canadiens Diagnostics

## Mission

Advance the Canadiens production-packaging roadmap by making freshness and
source-authority diagnostics available as a machine-readable CLI envelope.

## Scope

- Add JSON output to `icelines data-status`.
- Reuse the shared `DataStatusView` contract.
- Preserve the current text table by default.
- Document the scriptable diagnostic path.

## Pulses

| Pulse | Status | Evidence |
|---|---|---|
| 01 | passed | `data-status --json` implemented, documented, and covered by focused tests. |

## Closeout

Phase Canadiens Diagnostics is closed for the CLI data-status JSON slice.
Broader installer/update UX, seeded demo profiles, public API docs, and
automated freshness checks remain separate packaging and authority work.
