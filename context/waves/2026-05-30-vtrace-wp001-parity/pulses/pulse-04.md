# Pulse 04: Leaders Active Context JSON Parity

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
| Parent requirements | `REQ-WB-002`; `REQ-PARITY-001`; `REQ-DATA-001`; `REQ-QUERY-001`; `REQ-CODE-001` |
| Interfaces | `IF-DATA-001`; `IF-VIEW-001`; `IF-WEB-001`; `IF-QUERY-001` |
| Design / code rigor | `DES-001`; `DES-002`; `DES-009`; `DES-014`; `CR-006`; `CR-007`; `CR-008`; `CR-010`; `CR-023`; `CR-032` |
| Validation / evidence | `VAL-004`; `EVID-VAL-004`; `EVID-CR-003`; `EVID-CR-018`; `EVID-CODE-001` |

## Objective

Expose the leaders ViewContext active window through the active CLI JSON leaders
path and compare it with the existing Web JSON meta window for the canonical
bundled leaders fixture.

## Affected Surfaces

| Surface | Expected Change |
|---|---|
| CLI JSON | Additive `season` and `season_type` fields on leaders JSON rows. |
| Web JSON | Existing `/api/v1/leaders` `meta.season` and `meta.season_type` remain the reference surface. |
| Tests | Add an L2 fixture comparing CLI and Web leaders active context. |
| VTRACE docs | Record this as an active-context affected slice without closing full `WP-001`. |

## Allowed / Forbidden Scope

Allowed:

- Additive active-window fields on active leaders CLI JSON rows.
- Focused tests for CLI JSON row active context vs Web JSON meta active context.
- Documentation/evidence updates tied to this pulse.

Forbidden:

- Breaking the existing top-level CLI leaders JSON row array.
- Reworking Web config, active-season selection, route planning, TUI, or reports.
- Claiming full active-context parity across all surfaces.
- Claiming full `WP-001` closure.

## Validation Levels

| Level | Required Check | Status |
|---|---|---|
| L0 | Web projection test proves leaders ViewContext carries the expected active window. | passed |
| L1 | Formatting plus affected Web/CLI clippy checks or recorded blocker. | passed_with_risk |
| L2 | CLI subprocess JSON vs Web JSON active-context parity fixture. | passed |

## Evidence Log

| Evidence | Result |
|---|---|
| Inventory | Active Web leaders JSON already disclosed `meta.season` and `meta.season_type`; CLI leaders JSON rows did not expose the `ViewContext` window while preserving the row-array contract. |
| L0 | `cargo test -p icelines-web l0_web_leaders_view_round_trips_template_and_json_rows --lib` passed and now asserts the leaders ViewContext active window. |
| L1 | `cargo fmt --check`; `cargo clippy -p icelines-web --lib --no-deps -- -D warnings`; `cargo clippy -p icelines-cli --bin icelines --no-deps -- -D warnings` passed. Full workspace clippy remains blocked by unrelated `icelines-fetch/src/fletch.rs` `too_many_arguments`. |
| L2 | `cargo test -p icelines-cli --test goalies_web_cli_parity l2_query_leaders_cli_and_web_active_context_match` passed. |

## Gate

Gate is `passed_with_risk` for this pulse only. CLI and Web JSON leaders now
expose matching `20242025` / `regular` active context for the bundled fixture,
but broader active context across TUI, Web HTML, reports/exports, warnings,
empty/recovery states, and full `WP-001` parity remain open.
