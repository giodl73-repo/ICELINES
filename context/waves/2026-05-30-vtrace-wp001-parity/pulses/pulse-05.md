# Pulse 05: Leaders Result-State JSON Parity

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
| Parent requirements | `REQ-PARITY-001`; `REQ-QUERY-001`; `REQ-DATA-001`; `REQ-CODE-001` |
| Interfaces | `IF-VIEW-001`; `IF-QUERY-001`; `IF-WEB-001` |
| Design / code rigor | `DES-001`; `DES-002`; `DES-009`; `DES-014`; `CR-006`; `CR-007`; `CR-008`; `CR-010`; `CR-023`; `CR-032` |
| Validation / evidence | `VAL-004`; `EVID-VAL-004`; `EVID-CR-003`; `EVID-CR-018`; `EVID-CODE-001` |

## Objective

Expose selected leaders result-state metadata through the active CLI JSON leaders
row-array contract and compare it with the existing Web JSON meta result state
for the canonical bundled leaders fixture.

## Affected Surfaces

| Surface | Expected Change |
|---|---|
| CLI JSON | Additive `total`, `returned`, `top`, and `active_filters` fields on leaders JSON rows. |
| Web JSON | Existing `/api/v1/leaders` `meta.total`, `meta.returned`, `meta.top`, and `meta.active_filters` remain the reference surface. |
| Tests | Add an L2 fixture comparing CLI row result state and Web meta result state. |
| VTRACE docs | Record this as a result-state affected slice without closing full `WP-001`. |

## Allowed / Forbidden Scope

Allowed:

- Additive result-state fields on active leaders CLI JSON rows.
- Focused tests for CLI JSON row result state vs Web JSON meta result state.
- Documentation/evidence updates tied to this pulse.

Forbidden:

- Breaking the existing top-level CLI leaders JSON row array.
- Introducing a CLI JSON envelope without change control for a new version.
- Reworking empty-result, warning, TUI, report/export, or browser recovery flows.
- Claiming full `WP-001` closure.

## Validation Levels

| Level | Required Check | Status |
|---|---|---|
| L0 | CLI JSON export fixture proves result-state fields remain valid row additions. | passed |
| L1 | Formatting plus affected Web/CLI clippy checks or recorded blocker. | passed_with_risk |
| L2 | CLI subprocess JSON vs Web JSON result-state parity fixture. | passed |

## Evidence Log

| Evidence | Result |
|---|---|
| Inventory | Active Web leaders JSON already disclosed selected result-state metadata in `meta`; CLI leaders JSON rows did not expose matching fields while preserving the row-array contract. |
| L0 | `cargo test -p icelines-cli --test system_tests l2_cmd_query_leaders_json_export` passed and now asserts row result-state fields. |
| L1 | `cargo fmt --check`; `cargo clippy -p icelines-web --lib --no-deps -- -D warnings`; `cargo clippy -p icelines-cli --bin icelines --no-deps -- -D warnings` passed. Full workspace clippy remains blocked by unrelated `icelines-fetch/src/fletch.rs` `too_many_arguments`. |
| L2 | `cargo test -p icelines-cli --test goalies_web_cli_parity l2_query_leaders_cli_and_web_result_state_match` passed. |

## Gate

Gate is `passed_with_risk` for this pulse only. CLI and Web JSON leaders now
expose matching `total`, `returned`, `top`, and `active_filters` result state for
the bundled fixture, but empty-result behavior, warnings, TUI, report/export,
browser recovery states, and full `WP-001` parity remain open.
