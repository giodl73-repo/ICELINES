# Pulse 03: Leaders Source State JSON Parity

## Pulse Control

| Field | Value |
|---|---|
| Wave | `2026-05-30-vtrace-wp001-parity` |
| Work package | `WP-001` |
| Status | closed_with_risk |
| Gate decision | passed_with_risk |
| Date | 2026-05-30 |

## VTRACE IDs

| Type | IDs |
|---|---|
| Parent requirements | `REQ-DATA-001`; `REQ-PARITY-001`; `REQ-QUERY-001`; `REQ-WB-002`; `REQ-CODE-001` |
| Interfaces | `IF-DATA-001`; `IF-VIEW-001`; `IF-QUERY-001`; `IF-WEB-001` |
| Design / code rigor | `DES-001`; `DES-002`; `DES-004`; `DES-009`; `DES-014`; `CR-006`; `CR-007`; `CR-008`; `CR-010`; `CR-011`; `CR-023`; `CR-024`; `CR-032` |
| Validation / evidence | `VAL-004`; `VAL-005`; `EVID-VAL-004`; `EVID-VAL-005`; `EVID-CR-003`; `EVID-CR-011`; `EVID-CR-018`; `EVID-CODE-001` |

## Objective

Expose the leaders ViewContext source/completeness state through the active CLI
JSON and Web JSON leaders paths, then compare both surfaces for the canonical
bundled leaders fixture.

## Affected Surfaces

| Surface | Expected Change |
|---|---|
| CLI JSON | Additive `source_completeness` and `source_state` fields on leaders JSON rows. |
| Web JSON | Additive `meta.completeness` and `meta.source_state` fields on `/api/v1/leaders`. |
| Tests | Add an L2 fixture comparing CLI and Web leaders source/completeness state. |
| VTRACE docs | Record this as a source-state affected slice without closing full `WP-001`. |

## Allowed / Forbidden Scope

Allowed:

- Additive source/completeness fields on active leaders JSON surfaces.
- Shared `ViewContext` source-state construction for the leaders JSON slice.
- Focused tests for CLI JSON vs Web JSON source/completeness parity.
- Documentation/evidence updates tied to this pulse.

Forbidden:

- Breaking the existing top-level CLI leaders JSON row array.
- Rewriting the Web leaders query/filter planner.
- Claiming source provenance for live, snapshot, cache, TUI, or report/export
  surfaces not exercised by this pulse.
- Claiming full `WP-001` closure.

## Validation Levels

| Level | Required Check | Status |
|---|---|---|
| L0 | Web projection test proves leaders ViewContext carries complete roster source state. | passed |
| L1 | Formatting plus affected Web/CLI clippy checks or recorded blocker. | passed_with_risk |
| L2 | CLI subprocess JSON vs Web JSON source/completeness parity fixture. | passed |

## Evidence Log

| Evidence | Result |
|---|---|
| Inventory | Active leaders JSON rows did not disclose source/completeness state, while `ViewContext` already defines the typed vocabulary required by `IF-DATA-001`. |
| L0 | `cargo test -p icelines-web l0_web_leaders_view_round_trips_template_and_json_rows --lib` passed and now asserts complete roster source state on the leaders ViewContext. |
| L1 | `cargo fmt --check`; `cargo clippy -p icelines-web --lib --no-deps -- -D warnings`; `cargo clippy -p icelines-cli --bin icelines --no-deps -- -D warnings` passed. Full workspace clippy remains blocked by unrelated `icelines-fetch/src/fletch.rs` `too_many_arguments`. |
| L2 | `cargo test -p icelines-cli --test goalies_web_cli_parity l2_query_leaders_cli_and_web_source_state_match` passed. |

## Gate

Gate is `passed_with_risk` for this pulse only. CLI and Web JSON leaders now
expose matching `complete` / `roster` source state for the bundled fixture, but
broader source provenance, warnings, active context, TUI, report/export, Web
HTML, and full `WP-001` parity remain open.
