# Pulse 13: Leaders CSV Identity, Context, and Source-State Metadata

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
| Validation / evidence | `VAL-004`; `EVID-WP001-QUERY-CSV-METADATA-L0`; `EVID-WP001-QUERY-CSV-METADATA-L2`; `EVID-WP001-L1`; `EVID-CR-003`; `EVID-CR-018`; `EVID-CODE-001` |

## Objective

Carry the selected leaders stable row identity, active season/type context, and
roster source-state into the `query leaders --csv` adapter so CSV consumers can
trace rows to the same `LeadersView` semantics used by JSON, Web, TUI, Markdown,
and default text output.

## Affected Surfaces

| Surface | Expected Change |
|---|---|
| CLI query leaders CSV | Appends `nhl_id`, `season`, `season_type`, `source_kind`, and `source_completeness` columns after the existing leading metric columns. |
| CLI query leaders text/JSON/Web/TUI/Markdown | No contract change beyond shared helper reuse for source-state labels. |
| Tests | Focused unit and CLI subprocess tests assert the CSV header and row metadata. |
| VTRACE docs | Record this as a query/CSV affected-slice improvement without claiming full query planner, TUI, report/export, browser, or `WP-001` closure. |

## Allowed / Forbidden Scope

Allowed:

- Additive CSV columns sourced from `LeadersView` row identity and context.
- Focused unit/subprocess tests for the selected CSV leaders metadata disclosure.
- Documentation/evidence updates tied to this pulse.

Forbidden:

- Claiming full query planner parity, full interactive TUI parity, broader browser
  route/accessibility proof, broader provenance/context coverage, full
  report/export parity, or full `WP-001` closure.
- Reordering or removing existing leading CSV columns.
- Reworking ranking, filtering, scoring, data loading, or the query grammar.

## Validation Levels

| Level | Required Check | Status |
|---|---|---|
| L0 | CLI CSV header unit test proves selected identity/context/source-state metadata columns are part of the renderer contract. | passed |
| L1 | Formatting plus affected CLI bin clippy checks or recorded blocker. | passed_with_risk |
| L2 | CLI subprocess query test proves CSV rows expose `nhl_id`, active season/type, and `roster complete` source-state metadata. | passed |

## Evidence Log

| Evidence | Result |
|---|---|
| Inventory | JSON, Web HTML, TUI, Markdown, and default text carried selected leaders identity/context/source-state semantics, but `query leaders --csv` did not expose stable `nhl_id` or selected context/source metadata. |
| L0 | `cargo test -p icelines-cli commands::query::tests::l0_query_leaders_csv_header_reports_identity_context_and_source_state --bin icelines -- --nocapture` passed. |
| L2 | `cargo test -p icelines-cli --test system_tests l2_cmd_query_leaders -- --nocapture` passed, including `l2_cmd_query_leaders_csv_export` and `l2_cmd_query_leaders_json_csv_row_identity_match`. |
| L1 | `cargo fmt --check`; `cargo clippy -p icelines-cli --bin icelines --no-deps -- -D warnings` passed. Full workspace clippy remains blocked by unrelated `icelines-fetch/src/fletch.rs` `too_many_arguments`. |

## Gate

Gate is `passed_with_risk` for this pulse only. Leaders CSV output now exposes
the selected stable row identity, active season/type, and complete roster
source-state as additive trailing columns while preserving existing leading CSV
columns. Full `WP-001`, full query planner parity, broader provenance, full TUI
parity, full report/export parity, and full browser/accessibility route proof
remain open.
