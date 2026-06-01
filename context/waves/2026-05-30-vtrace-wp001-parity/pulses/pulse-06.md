# Pulse 06: Leaders Empty/Warning JSON Envelope Parity

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
| Interfaces | `IF-VIEW-001`; `IF-QUERY-001`; `IF-WEB-001`; `IF-DATA-001` |
| Design / code rigor | `DES-001`; `DES-002`; `DES-009`; `DES-014`; `CR-006`; `CR-007`; `CR-008`; `CR-010`; `CR-023`; `CR-032` |
| Validation / evidence | `VAL-004`; `EVID-VAL-004`; `EVID-WP001-EMPTY-WARNING-L2`; `EVID-CR-003`; `EVID-CR-018`; `EVID-CODE-001` |

## Objective

Add an opt-in leaders CLI JSON envelope that can carry ViewModel `empty_state`
and `warnings`, then compare the empty goalie-filter leaders result against Web
JSON `meta` for the canonical bundled fixture.

## Affected Surfaces

| Surface | Expected Change |
|---|---|
| CLI command | `icelines query leaders --json-envelope` emits a Web-compatible `schema_version`, `route`, `data`, and `meta` envelope without changing existing `--json` row-array output. |
| CLI/Web JSON | Leaders goalie-filter empty results carry matching `meta.empty_state` and `meta.warnings` semantics. |
| Web JSON | Existing `/api/v1/leaders` v1 envelope gains additive `meta.empty_state` and `meta.warnings` fields. |
| Tests | Add L0/L2 fixtures for the envelope and empty/warning parity. |
| VTRACE docs | Record the envelope/versioning decision and the remaining scope risk. |

## Allowed / Forbidden Scope

Allowed:

- Additive CLI `--json-envelope` output for leaders only.
- Additive Web JSON meta fields for leaders empty/warning state.
- Focused empty goalie-filter fixture for CLI/Web JSON parity.
- Documentation/evidence updates tied to this pulse.

Forbidden:

- Breaking the existing CLI leaders `--json` top-level row array.
- Claiming TUI, report/export, browser HTML recovery, or full `WP-001` closure.
- Broad query planner or goalie leaders rework.

## Validation Levels

| Level | Required Check | Status |
|---|---|---|
| L0 | CLI envelope fixture and Web ViewModel empty/warning fixture prove the fields serialize. | passed |
| L1 | Formatting plus affected Web/CLI clippy checks or recorded blocker. | passed_with_risk |
| L2 | CLI subprocess envelope vs Web route JSON empty/warning parity fixture. | passed |

## Evidence Log

| Evidence | Result |
|---|---|
| Inventory | Existing CLI `--json` row-array compatibility could not carry empty-result metadata; an opt-in envelope was required to preserve compatibility while exposing `empty_state` and `warnings`. |
| L0 | `cargo test -p icelines-cli --test system_tests l2_cmd_query_leaders_json_envelope_empty_warning_export`; `cargo test -p icelines-web --lib l0_web_leaders_goalie_filter_sets_empty_warning_state` passed. |
| L1 | `cargo fmt --check`; `cargo clippy -p icelines-web --lib --no-deps -- -D warnings`; `cargo clippy -p icelines-cli --bin icelines --no-deps -- -D warnings` passed. Full workspace clippy remains blocked by unrelated `icelines-fetch/src/fletch.rs` `too_many_arguments`. |
| L2 | `cargo test -p icelines-cli --test goalies_web_cli_parity l2_query_leaders_cli_and_web_empty_warning_state_match` passed. |

## Gate

Gate is `passed_with_risk` for this pulse only. CLI and Web JSON leaders now
match on the empty goalie-filter result's `data`, result-state fields,
`empty_state`, and `warnings`, but TUI parity, report/export parity, browser HTML
recovery review, broader source provenance, and full `WP-001` remain open.
