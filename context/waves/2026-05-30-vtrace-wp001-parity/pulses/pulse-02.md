# Pulse 02: Leaders Web HTML Identity Parity

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
| Parent requirements | `REQ-QUERY-001`; `REQ-PARITY-001`; `REQ-DATA-001`; `REQ-WB-002`; `REQ-CODE-001` |
| Interfaces | `IF-VIEW-001`; `IF-QUERY-001`; `IF-DATA-001`; `IF-WEB-001` |
| Design / code rigor | `DES-001`; `DES-002`; `DES-009`; `DES-014`; `CR-006`; `CR-007`; `CR-008`; `CR-010`; `CR-011`; `CR-023`; `CR-024`; `CR-032` |
| Validation / evidence | `VAL-004`; `EVID-VAL-004`; `EVID-CR-003`; `EVID-CR-011`; `EVID-CR-018`; `EVID-CODE-001` |

## Objective

Expose the stable leaders row identity already proven in CLI JSON and Web JSON
on the `/leaders` HTML table rows, then compare Web HTML row identity against
the canonical CLI leaders JSON fixture.

## Affected Surfaces

| Surface | Expected Change |
|---|---|
| Web HTML | Add machine-readable stable row identity attributes sourced from `LeadersView`-projected rows. |
| Tests | Add an L2 fixture comparing CLI JSON to Web HTML row identity and visible semantic fields. |
| VTRACE docs | Record this as a Web HTML affected slice without closing full `WP-001`. |

## Allowed / Forbidden Scope

Allowed:

- Additive HTML `data-*` attributes for row identity and fixture-visible metrics.
- Focused tests for CLI JSON vs Web HTML leaders identity parity.
- Documentation/evidence updates tied to this pulse.

Forbidden:

- Rewriting the Web leaders query/filter planner.
- Changing visible table layout or existing JSON field names.
- Reworking TUI, report/export, fetch, or dependency boundaries.
- Claiming full `WP-001` closure.

## Validation Levels

| Level | Required Check | Status |
|---|---|---|
| L0 | Web template/projection test proves stable row values reach HTML-ready rows. | passed |
| L1 | Formatting plus affected Web/CLI clippy checks or recorded blocker. | passed_with_risk |
| L2 | CLI subprocess JSON vs Web HTML route parity fixture. | passed |

## Evidence Log

| Evidence | Result |
|---|---|
| Inventory | `/leaders` HTML already renders player links from `row.nhl_id`, but table rows do not expose stable identity for parity fixtures. |
| L0 | `cargo test -p icelines-web l0_web_leaders_view_round_trips_template_and_json_rows --lib` passed and now asserts stable HTML `data-*` identity attributes. |
| L1 | `cargo fmt --check`; `cargo clippy -p icelines-web --lib --no-deps -- -D warnings`; `cargo clippy -p icelines-cli --bin icelines --no-deps -- -D warnings` passed. Full workspace clippy remains blocked by unrelated `icelines-fetch/src/fletch.rs` `too_many_arguments`. |
| L2 | `cargo test -p icelines-cli --test goalies_web_cli_parity l2_query_leaders_cli_and_web_html_stable_identity_match` passed. |

## Gate

Gate is `passed_with_risk` for this pulse only. Web HTML rows now expose stable
leaders identity for parity fixtures, but warnings, source/completeness state,
active context, TUI, report/export, and full `WP-001` parity remain open.
