# IceLines Specification Baseline

## Scope

This baseline groups the current IceLines VTRACE requirements into controlled
specification surfaces. It keeps product evidence honest: query, workbench,
web, report, fantasy, data-depth, cache, and signal claims must remain tied to
source state, validation evidence, and explicit limits.

## Specification Rows

| Spec ID | Source IDs | Surface | Baseline Rule | Verification Method | Status |
|---|---|---|---|---|---|
| SPEC-ICE-001 | REQ-WB-001 / REQ-WB-002 / REQ-WB-003 | Workbench state and layouts | The workbench keeps active season context visible and preserves named layout state without hiding hockey semantics in renderer-local storage. | TUI/Web layout and active-context evidence | accepted_with_risk |
| SPEC-ICE-002 | REQ-QUERY-001 / REQ-PARITY-001 / REQ-DATA-001 | Query and ViewModel parity | CLI, TUI, Web, JSON, and export surfaces lower through shared query/ViewModel semantics and expose source/completeness state. | parity fixtures and route/export evidence | accepted_with_risk |
| SPEC-ICE-003 | REQ-STAT-001 / REQ-STAT-002 / REQ-REPORT-001 | Stat-in-perspective and reports | Public stat and report outputs disclose scope, historical-ranking limits, completeness, and unsupported claim boundaries. | report/export snapshots and edge-case fixtures | accepted_with_risk |
| SPEC-ICE-004 | REQ-WEB-001 / REQ-WEB-002 | Web dashboard safety | Web routes preserve no-JS readability, active context, read-only GET behavior, recovery, viewport, and bind warnings. | route/browser/shell evidence | accepted_with_risk |
| SPEC-ICE-005 | REQ-OFFLINE-001 / REQ-DATA-DEPTH-001 / REQ-FRESH-001 | Source state and data-depth reliability | Offline, fetch, snapshot, freshness, schema-drift, and unavailable-source states fail visibly instead of producing silent zeroes. | source-state tests and command transcripts | accepted_with_risk |
| SPEC-ICE-006 | REQ-FANTASY-001 / REQ-CACHE-001 / REQ-CACHE-002 / REQ-CACHE-003 / REQ-CACHE-004 / REQ-SIGNAL-001 | Fantasy, analytics cache, and signals | Product read models and derived signals carry evidence tier, methodology, limitation, and unavailable/missing-source behavior. | fantasy/cache/signal fixtures | partial |
| SPEC-ICE-007 | REQ-DEP-001 / REQ-LEAN-001 / REQ-CODE-001 | Build and dependency posture | Default code rigor remains tracked while standalone/lean dependency claims stay target-not-met until verified. | workspace gates and dependency inspection | mixed |

## Non-Goals

- IceLines does not claim betting, prediction, injury certainty, era-adjusted
  normalization, autonomous coaching authority, or complete-world truth from
  the current baseline.
- `icelines-site` remains deferred/historical for this VTRACE baseline.
