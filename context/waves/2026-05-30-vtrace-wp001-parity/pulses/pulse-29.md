# Pulse 29: Leaders Web Active Filter Route Evidence

## Pulse Control

| Field | Value |
|---|---|
| Wave | `2026-05-30-vtrace-wp001-parity` |
| Work package | `WP-001` |
| Status | closed_with_risk |
| Gate decision | passed_with_risk |
| Date | 2026-05-31 |

## VTRACE IDs

| Type | IDs |
|---|---|
| Parent requirements | `REQ-WEB-002`; `REQ-PARITY-001`; `REQ-QUERY-001`; `REQ-CODE-001` |
| Interfaces | `IF-WEB-001`; `IF-VIEW-001`; `IF-QUERY-001` |
| Design / code rigor | `DES-003`; `DES-007`; `DES-009`; `DES-014`; `CR-006`; `CR-010`; `CR-014`; `CR-027`; `CR-032` |
| Validation / evidence | `VAL-003`; `VAL-004`; `EVID-WP001-HTML-FILTER-ACTIVE-L2`; `EVID-WP001-L1`; `EVID-CR-006`; `EVID-CR-014`; `EVID-CR-018`; `EVID-CODE-001` |

## Objective

Promote the existing leaders Web HTML `data-result-active-filters` metadata to
route-level evidence by comparing the selected CLI JSON envelope active filter
state with the Web `/leaders?filter=goals%3E%3D1` HTML result metadata.

## Affected Surfaces

| Surface | Expected Change |
|---|---|
| CLI/Web parity tests | Adds a focused L2 fixture that dispatches the Web `/leaders` route with an encoded query filter and checks `data-result-*` metadata against the selected CLI JSON envelope state. |
| Web HTML / JSON / CLI output | No product contract change. |
| VTRACE docs | Record route-level evidence for selected active query-filter state without claiming full browser/accessibility, full query-planner parity, or `WP-001` closure. |

## Allowed / Forbidden Scope

Allowed:

- Additive parity evidence for existing Web result metadata and CLI JSON envelope
  metadata.
- Documentation/evidence updates tied to this pulse.

Forbidden:

- Changing Web JSON, CLI, TUI, Markdown export, CSV, scoring, parser/planner, or
  data-loading contracts.
- Claiming full Web browser/accessibility proof, full query grammar proof, full
  route parity, or full `WP-001` closure.

## Validation Levels

| Level | Required Check | Status |
|---|---|---|
| L0 | Covered by existing Web result metadata rendering evidence from Pulse 19. | inherited |
| L1 | Formatting plus affected CLI parity-test clippy checks or recorded blocker. | passed_with_risk |
| L2 | CLI JSON envelope selected `active_filters` matches Web route HTML `data-result-active-filters` for `/leaders?sort=goals&filter=goals%3E%3D1&top=5`, including matching total/returned/top/sort metadata. | passed |

## Evidence Log

| Evidence | Result |
|---|---|
| Inventory | Pulse 19 proved rendered result metadata, but route-level parity evidence for selected query-filter state was still pending. |
| L2 | `cargo test -p icelines-cli --test goalies_web_cli_parity l2_query_leaders_cli_json_and_web_html_active_filter_state_match -- --nocapture` passed. |
| L1 | `cargo fmt --check` passed. Affected CLI parity-test clippy passed. Full Web all-targets clippy and full workspace clippy remain blocked by unrelated existing lint debt outside this pulse. |
| Docs | `C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only` passed. |

## Gate

Gate is `passed_with_risk` for this pulse only. The selected Web leaders
`filter=goals>=1` route now has L2 route evidence that active query-filter state
and result metadata match the CLI JSON envelope. Full `WP-001`, broader
browser/accessibility proof, broader query-planner parity, full interactive TUI
parity, broader source provenance, and WP-008 integration rehearsal remain open.
