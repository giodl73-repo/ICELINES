# Pulse 16: Leaders TUI Query-Result Metadata

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
| Design / code rigor | `DES-001`; `DES-002`; `DES-003`; `DES-006`; `DES-009`; `DES-014`; `CR-003`; `CR-006`; `CR-007`; `CR-010`; `CR-011`; `CR-012`; `CR-013`; `CR-032` |
| Validation / evidence | `VAL-001`; `VAL-004`; `EVID-WP001-TUI-RESULT-L0`; `EVID-WP001-L1`; `EVID-CR-003`; `EVID-CR-005`; `EVID-CR-018`; `EVID-CODE-001` |

## Objective

Carry selected leaders query-result metadata into the TUI Stats leaders results
panel so the interactive workbench surface discloses the same result window and
query intent already present in JSON, CSV, and default CLI text output: total
matched rows, returned rows, requested top count, sort key, and active filters.

## Affected Surfaces

| Surface | Expected Change |
|---|---|
| TUI Stats leaders results | Adds a `Result:` line after the existing `Context:` line with `total`, `returned`, `top`, `sort`, and `active_filters`. |
| CLI query leaders JSON/CSV/text/Web/Markdown | No contract change. |
| Tests | Focused TUI unit/render tests assert active filter labeling and the rendered result metadata line. |
| VTRACE docs | Record this as a TUI affected-slice improvement without claiming full interactive TUI, query planner, report/export, browser, or `WP-001` closure. |

## Allowed / Forbidden Scope

Allowed:

- Additive TUI result metadata line sourced from selected leaders query execution
  state.
- Focused TUI unit/render tests for selected query-result metadata disclosure.
- Documentation/evidence updates tied to this pulse.

Forbidden:

- Claiming full query planner parity, full interactive TUI validation, broader
  browser route/accessibility proof, broader provenance/context coverage, full
  report/export parity, or full `WP-001` closure.
- Reworking ranking, filtering, scoring, data loading, JSON/CSV/text/Web/Markdown
  contracts, or the query grammar.

## Validation Levels

| Level | Required Check | Status |
|---|---|---|
| L0 | TUI unit/render tests prove selected query-result metadata is part of the Stats leaders results panel. | passed |
| L1 | Formatting plus affected CLI bin clippy checks or recorded blocker. | passed_with_risk |
| L2 | Cross-surface or interactive TUI rehearsal. | pending_overall; not required for this narrow TUI metadata pulse |

## Evidence Log

| Evidence | Result |
|---|---|
| Inventory | JSON, CSV, and default CLI text carried selected result/query metadata, but the TUI Stats leaders results panel only exposed active context and source-state metadata. |
| L0 | `cargo test -p icelines-cli tui::screens::queries::tests::l0_tui_leaders_active_filters_label_reports_query_intent --bin icelines -- --nocapture` passed. |
| L0 | `cargo test -p icelines-cli tui::screens::queries::tests::l0_tui_leaders_results_render_active_context_and_source_state --bin icelines -- --nocapture` passed. |
| L1 | `cargo fmt --check`; `cargo clippy -p icelines-cli --bin icelines --no-deps -- -D warnings` passed. Full workspace clippy remains blocked by unrelated `icelines-fetch/src/fletch.rs` `too_many_arguments`. |

## Gate

Gate is `passed_with_risk` for this pulse only. The TUI Stats leaders results
panel now exposes selected query-result metadata as an additive `Result:` line
while preserving existing context/source-state disclosure and row rendering. Full
`WP-001`, full query planner parity, broader provenance, full TUI validation,
full report/export parity, and full browser/accessibility route proof remain
open.
