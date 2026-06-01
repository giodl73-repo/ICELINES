# Pulse 18: Leaders Markdown Export Front-Matter Query-Result Metadata

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
| Parent requirements | `REQ-WB-002`; `REQ-DATA-001`; `REQ-PARITY-001`; `REQ-QUERY-001`; `REQ-REPORT-001`; `REQ-CODE-001` |
| Interfaces | `IF-DATA-001`; `IF-VIEW-001`; `IF-QUERY-001`; `IF-REPORT-001` |
| Design / code rigor | `DES-001`; `DES-002`; `DES-003`; `DES-008`; `DES-009`; `DES-014`; `CR-003`; `CR-006`; `CR-007`; `CR-010`; `CR-011`; `CR-016`; `CR-017`; `CR-032` |
| Validation / evidence | `VAL-002`; `VAL-004`; `EVID-WP001-EXPORT-FM-RESULT-L0`; `EVID-WP001-EXPORT-FM-RESULT-L2`; `EVID-WP001-L1`; `EVID-CR-003`; `EVID-CR-007`; `EVID-CR-013`; `EVID-CR-018`; `EVID-CODE-001` |

## Objective

Carry selected leaders query-result metadata into Markdown leaders export front
matter so machine-readable report metadata discloses the same result window and
query intent already visible in the Markdown report body: total matched rows,
returned rows, requested top count, sort key, and active filters.

## Affected Surfaces

| Surface | Expected Change |
|---|---|
| Markdown leaders export front matter | Adds a `result` object with `total`, `returned`, `top`, `sort`, and `active_filters`. |
| Markdown leaders export report body | Preserves the existing `## Context` and `## Result` sections. |
| CLI query leaders JSON/CSV/text/Web/TUI | No contract change. |
| Tests | Focused L0 export render test and existing L2 subprocess export test assert selected front-matter result metadata. |
| VTRACE docs | Record this as a report/export affected-slice improvement without claiming full report/export, full query planner, browser, TUI, or `WP-001` closure. |

## Allowed / Forbidden Scope

Allowed:

- Additive Markdown front-matter result metadata sourced from selected leaders
  export execution state.
- Focused export unit and subprocess checks for selected machine-readable
  query-result metadata disclosure.
- Documentation/evidence updates tied to this pulse.

Forbidden:

- Claiming full query planner parity, full report/export validation, broader
  browser route/accessibility proof, broader provenance/context coverage, full
  TUI validation, or full `WP-001` closure.
- Changing ranking, filtering, scoring, data loading, JSON/CSV/text/Web/TUI
  contracts, report-body semantics, or the query grammar.

## Validation Levels

| Level | Required Check | Status |
|---|---|---|
| L0 | Markdown export render tests prove selected query-result metadata is in leaders front matter. | passed |
| L1 | Formatting plus affected CLI bin clippy checks or recorded blocker. | passed_with_risk |
| L2 | CLI subprocess Markdown export stdout fixture proves selected result metadata is emitted in front matter. | passed |

## Evidence Log

| Evidence | Result |
|---|---|
| Inventory | Markdown leaders export body carried selected result/query metadata, but YAML front matter only exposed context/source metadata. |
| L0 | `cargo test -p icelines-cli l0_export_leaders_has_required_front_matter` passed. |
| L0 | `cargo test -p icelines-cli l0_export_leaders_reports_result_and_query_intent` passed. |
| L2 | `cargo test -p icelines-cli l2_cmd_export_md_leaders_to_stdout --test system_tests` passed. |
| L1 | `cargo fmt --check`; `cargo clippy -p icelines-cli --bin icelines --no-deps -- -D warnings` passed. Full workspace clippy remains blocked by unrelated `icelines-fetch/src/fletch.rs` `too_many_arguments`. |

## Gate

Gate is `passed_with_risk` for this pulse only. Markdown leaders exports now
expose selected query-result metadata in both machine-readable front matter and
the visible `## Result` section while preserving existing table columns,
ranking, and query behavior. Full `WP-001`, full query planner parity, broader
provenance, full report/export validation, full TUI validation, and full
browser/accessibility route proof remain open.
