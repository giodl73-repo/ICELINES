# Pulse 01: Leaders Stable Identity Parity

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
| Design / code rigor | `DES-001`; `DES-002`; `DES-003`; `DES-009`; `DES-014`; `CR-006`; `CR-007`; `CR-008`; `CR-010`; `CR-011`; `CR-023`; `CR-024`; `CR-032` |
| Validation / evidence | `VAL-004`; `EVID-VAL-004`; `EVID-CR-003`; `EVID-CR-011`; `EVID-CR-018`; `EVID-CODE-001` |

## Objective

Prove that leaders row identity is not renderer-local by preserving the shared
`LeadersView.rows[].player_id` through CLI JSON and Web JSON, then validating a
CLI/Web leaders fixture against stable IDs and visible semantic fields.

## Affected Surfaces

| Surface | Expected Change |
|---|---|
| Core ViewModel | No semantic rewrite; existing `LeadersView` stable `player_id` is the source of truth. |
| CLI JSON | Include stable `nhl_id` and team abbreviation fields from `LeadersView`. |
| Web JSON | Include stable `nhl_id` and team abbreviation fields from `LeadersView`. |
| Tests | Add or extend affected-slice tests for ViewModel-to-JSON and CLI/Web parity. |
| VTRACE docs | Record evidence and keep broad WP-001 rows pending unless fully proven. |

## Allowed / Forbidden Scope

Allowed:

- Additive JSON identity fields.
- Focused tests for leaders stable identity parity.
- Documentation/evidence updates tied to this pulse.

Forbidden:

- Breaking existing JSON field names.
- Changing query result ordering semantics outside the selected fixture.
- Reworking TUI, reports, export, fetch, or dependency boundaries.
- Claiming full `WP-001` closure.

## Validation Levels

| Level | Required Check | Status |
|---|---|---|
| L0 | Focused Web/CLI JSON projection tests and existing core `LeadersView` contract fixture. | passed |
| L1 | Formatting plus affected package/test/clippy checks or recorded blocker. | passed_with_risk |
| L2 | CLI subprocess vs Web route leaders parity fixture. | passed |

## Evidence Log

| Evidence | Result |
|---|---|
| Inventory | Shared `LeadersView` already serializes `player_id`; active JSON adapter rows need stable identity fields for parity evidence. |
| L0 | `cargo test -p icelines-web l0_web_leaders_view_round_trips_template_and_json_rows --lib`; `cargo test -p icelines-cli --test system_tests l2_cmd_query_leaders_json_export`; `cargo test -p icelines-cli --test system_tests l2_cmd_query_leaders_json_csv_row_identity_match` passed. |
| L1 | `cargo fmt --check`; `cargo clippy -p icelines-web --lib --no-deps -- -D warnings`; `cargo clippy -p icelines-cli --bin icelines --no-deps -- -D warnings` passed. Full workspace clippy remains blocked by unrelated `icelines-fetch/src/fletch.rs` `too_many_arguments`. |
| L2 | `cargo test -p icelines-cli --test goalies_web_cli_parity l2_query_leaders_cli_and_web_stable_identity_match` passed. |

## Gate

Gate is `passed_with_risk` for this pulse only. Stable leaders identity now flows
through CLI JSON and Web JSON, but full workspace clippy and broader `WP-001`
source-state/query/ViewModel parity remain open.
