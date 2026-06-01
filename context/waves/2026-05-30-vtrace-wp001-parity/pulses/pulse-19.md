# Pulse 19: Leaders Web HTML Query-Result Metadata

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
| Parent requirements | `REQ-WB-002`; `REQ-DATA-001`; `REQ-PARITY-001`; `REQ-QUERY-001`; `REQ-WEB-001`; `REQ-WEB-002`; `REQ-CODE-001` |
| Interfaces | `IF-DATA-001`; `IF-VIEW-001`; `IF-QUERY-001`; `IF-WEB-001` |
| Design / code rigor | `DES-001`; `DES-002`; `DES-003`; `DES-007`; `DES-009`; `DES-014`; `CR-003`; `CR-006`; `CR-007`; `CR-010`; `CR-011`; `CR-014`; `CR-015`; `CR-032` |
| Validation / evidence | `VAL-003`; `VAL-004`; `EVID-WP001-L0`; `EVID-WP001-L1`; `EVID-WP001-HTML-RESULT-L2`; `EVID-CR-003`; `EVID-CR-006`; `EVID-CR-018`; `EVID-CODE-001` |

## Objective

Carry selected leaders result-state and query-intent metadata into the Web HTML
leaders meta line so browser-visible markup exposes the same total, returned,
top, sort, and active-filter state already carried by CLI JSON and Web JSON.

## Affected Surfaces

| Surface | Expected Change |
|---|---|
| Web leaders HTML | Adds `data-result-total`, `data-result-returned`, `data-result-top`, `data-result-sort`, and `data-result-active-filters` attributes to the existing leaders meta line. |
| Web leaders JSON | No contract change; existing `meta` result-state fields remain the JSON parity source. |
| CLI query leaders JSON | No contract change; existing row result-state fields remain the CLI parity source. |
| Tests | Focused Web L0 render assertion plus L2 CLI JSON / Web HTML parity fixture for selected result metadata. |
| VTRACE docs | Record this as a browser HTML affected-slice improvement without claiming full browser route/accessibility, full query planner, full TUI, report/export, or `WP-001` closure. |

## Allowed / Forbidden Scope

Allowed:

- Additive Web HTML `data-*` result/query metadata on the existing leaders meta
  line.
- Focused route/render parity checks for selected leaders result/query metadata.
- Documentation/evidence updates tied to this pulse.

Forbidden:

- Claiming full browser route/accessibility proof, full query planner parity, full
  report/export validation, broader provenance/context coverage, full TUI
  validation, or full `WP-001` closure.
- Changing ranking, filtering, scoring, data loading, Web JSON, CLI JSON, CSV,
  text, TUI, Markdown export contracts, or the query grammar.

## Validation Levels

| Level | Required Check | Status |
|---|---|---|
| L0 | Web leaders render test proves selected query-result metadata is in HTML attributes. | passed |
| L1 | Formatting plus affected Web/CLI clippy checks or recorded blocker. | passed_with_risk |
| L2 | CLI subprocess JSON and Web HTML route fixture prove selected result metadata parity. | passed |

## Evidence Log

| Evidence | Result |
|---|---|
| Inventory | Web JSON carried selected leaders result/query metadata, but Web HTML only exposed active season/type attributes on the meta line. |
| L0 | `cargo test -p icelines-web l0_web_leaders_view_round_trips_template_and_json_rows --lib` passed. |
| L2 | `cargo test -p icelines-cli l2_query_leaders_cli_json_and_web_html_result_state_match --test goalies_web_cli_parity` passed. |
| Regression | `cargo test -p icelines-cli l2_query_leaders_cli_and_web_result_state_match --test goalies_web_cli_parity` passed. |
| L1 | `cargo fmt --check`; `cargo clippy -p icelines-web --lib --no-deps -- -D warnings`; `cargo clippy -p icelines-cli --test goalies_web_cli_parity --no-deps -- -D warnings` passed. `cargo clippy -p icelines-web --all-targets --no-deps -- -D warnings` remains blocked by unrelated existing lints in `icelines-web/tests/l1_router.rs`. |

## Gate

Gate is `passed_with_risk` for this pulse only. Web leaders HTML now exposes
selected query-result metadata in machine-readable attributes while preserving
existing visible layout, route behavior, and JSON/CLI contracts. Full `WP-001`,
full browser/accessibility route proof, broader source provenance, full query
planner parity, full TUI validation, full report/export validation, and WP-008
integration rehearsal remain open.
