# Pulse 15: Leaders CLI Text Query-Result Metadata

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
| Parent requirements | `REQ-WB-002`; `REQ-DATA-001`; `REQ-PARITY-001`; `REQ-QUERY-001`; `REQ-CODE-001` |
| Interfaces | `IF-DATA-001`; `IF-VIEW-001`; `IF-QUERY-001` |
| Design / code rigor | `DES-001`; `DES-002`; `DES-003`; `DES-009`; `DES-014`; `CR-003`; `CR-006`; `CR-007`; `CR-010`; `CR-011`; `CR-032` |
| Validation / evidence | `VAL-004`; `EVID-WP001-QUERY-TEXT-RESULT-L0`; `EVID-WP001-QUERY-TEXT-RESULT-L2`; `EVID-WP001-L1`; `EVID-CR-003`; `EVID-CR-018`; `EVID-CODE-001` |

## Objective

Carry the selected leaders query-result metadata already present in JSON and CSV
surfaces into the default `query leaders` text adapter so interactive CLI output
discloses the same result window and query intent: total matched rows, returned
rows, requested top count, sort key, and active stat filters.

## Affected Surfaces

| Surface | Expected Change |
|---|---|
| CLI query leaders text | Adds a `Result:` line after the existing `Context:` line with `total`, `returned`, `top`, `sort`, and `active_filters`. |
| CLI query leaders JSON/CSV/Web/TUI/Markdown | No contract change. |
| Tests | Focused unit and CLI subprocess tests assert the text query-result metadata line. |
| VTRACE docs | Record this as a query/text affected-slice improvement without claiming full query planner, TUI, report/export, browser, or `WP-001` closure. |

## Allowed / Forbidden Scope

Allowed:

- Additive text output line sourced from selected leaders query execution state.
- Focused unit/subprocess tests for selected default text query-result metadata
  disclosure.
- Documentation/evidence updates tied to this pulse.

Forbidden:

- Claiming full query planner parity, full interactive TUI parity, broader browser
  route/accessibility proof, broader provenance/context coverage, full
  report/export parity, or full `WP-001` closure.
- Reworking ranking, filtering, scoring, data loading, JSON/CSV contracts, or the
  query grammar.

## Validation Levels

| Level | Required Check | Status |
|---|---|---|
| L0 | CLI text renderer unit test proves selected query-result metadata is part of the default text contract. | passed |
| L1 | Formatting plus affected CLI bin clippy checks or recorded blocker. | passed_with_risk |
| L2 | CLI subprocess query test proves default text output exposes result state plus sort and active filter metadata. | passed |

## Evidence Log

| Evidence | Result |
|---|---|
| Inventory | JSON and CSV carried selected result/query metadata, but default `query leaders` text output only exposed context/source-state plus a loose summary. |
| L0 | `cargo test -p icelines-cli commands::query::tests::l0_query_leaders_result_line_reports_query_result_metadata --bin icelines -- --nocapture` passed. |
| L2 | `cargo test -p icelines-cli --test system_tests l2_cmd_query_leaders_exits_zero -- --nocapture` passed. |
| L1 | `cargo fmt --check`; `cargo clippy -p icelines-cli --bin icelines --no-deps -- -D warnings` passed. Full workspace clippy remains blocked by unrelated `icelines-fetch/src/fletch.rs` `too_many_arguments`. |

## Gate

Gate is `passed_with_risk` for this pulse only. Default leaders CLI text output
now exposes selected query-result metadata as an additive `Result:` line while
preserving existing context/source-state disclosure and the leaders table. Full
`WP-001`, full query planner parity, broader provenance, full TUI parity, full
report/export parity, and full browser/accessibility route proof remain open.
