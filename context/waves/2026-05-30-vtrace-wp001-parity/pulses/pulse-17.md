# Pulse 17: Leaders Markdown Export Query-Result Metadata

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
| Validation / evidence | `VAL-002`; `VAL-004`; `EVID-WP001-EXPORT-RESULT-L0`; `EVID-WP001-EXPORT-RESULT-L2`; `EVID-WP001-L1`; `EVID-CR-003`; `EVID-CR-007`; `EVID-CR-013`; `EVID-CR-018`; `EVID-CODE-001` |

## Objective

Carry selected leaders query-result metadata into the Markdown leaders export body
so the report discloses the same result window and query intent already present in
JSON, CSV, default CLI text, and TUI output: total matched rows, returned rows,
requested top count, sort key, and active filters.

## Affected Surfaces

| Surface | Expected Change |
|---|---|
| Markdown leaders export body | Adds a `## Result` section after the existing `## Context` section with `Total`, `Returned`, `Top`, `Sort`, and `Active filters`. |
| Markdown leaders export front matter | No contract change in this pulse. |
| CLI query leaders JSON/CSV/text/Web/TUI | No contract change. |
| Tests | Focused L0 export render test and existing L2 subprocess export test assert selected result metadata. |
| VTRACE docs | Record this as a report/export affected-slice improvement without claiming full report/export, full query planner, browser, TUI, or `WP-001` closure. |

## Allowed / Forbidden Scope

Allowed:

- Additive Markdown report-body result metadata sourced from selected leaders
  export execution state.
- Focused export unit and subprocess checks for selected query-result metadata
  disclosure.
- Documentation/evidence updates tied to this pulse.

Forbidden:

- Claiming full query planner parity, full report/export validation, broader
  browser route/accessibility proof, broader provenance/context coverage, full
  TUI validation, or full `WP-001` closure.
- Changing Markdown front-matter result metadata, ranking, filtering, scoring,
  data loading, JSON/CSV/text/Web/TUI contracts, or the query grammar.

## Validation Levels

| Level | Required Check | Status |
|---|---|---|
| L0 | Markdown export render test proves selected query-result metadata is in the leaders report body. | passed |
| L1 | Formatting plus affected CLI bin clippy checks or recorded blocker. | passed_with_risk |
| L2 | CLI subprocess Markdown export stdout fixture proves selected result metadata is emitted with the report. | passed |

## Evidence Log

| Evidence | Result |
|---|---|
| Inventory | JSON, CSV, default CLI text, and TUI carried selected result/query metadata, but Markdown leaders export only exposed context/source-state body disclosure and front-matter context/source metadata. |
| L0 | `cargo test -p icelines-cli l0_export_leaders_reports_result_and_query_intent` passed. |
| L2 | `cargo test -p icelines-cli l2_cmd_export_md_leaders_to_stdout --test system_tests` passed. |
| L1 | `cargo fmt --check`; `cargo clippy -p icelines-cli --bin icelines --no-deps -- -D warnings` passed. Full workspace clippy remains blocked by unrelated `icelines-fetch/src/fletch.rs` `too_many_arguments`. |

## Gate

Gate is `passed_with_risk` for this pulse only. Markdown leaders exports now
expose selected query-result metadata as an additive `## Result` section after
context/source-state disclosure while preserving existing front matter, table
columns, ranking, and query behavior. Full `WP-001`, full query planner parity,
broader provenance, full report/export validation, full TUI validation, and full
browser/accessibility route proof remain open.
