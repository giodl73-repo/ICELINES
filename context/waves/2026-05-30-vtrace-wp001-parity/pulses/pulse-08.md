# Pulse 08: Leaders Browser HTML Active-Context Parity

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
| Parent requirements | `REQ-WB-002`; `REQ-PARITY-001`; `REQ-WEB-001`; `REQ-CODE-001` |
| Interfaces | `IF-VIEW-001`; `IF-WEB-001`; `IF-DATA-001` |
| Design / code rigor | `DES-001`; `DES-007`; `DES-009`; `DES-014`; `CR-006`; `CR-007`; `CR-008`; `CR-014`; `CR-015`; `CR-032` |
| Validation / evidence | `VAL-003`; `VAL-004`; `EVID-VAL-004`; `EVID-WP001-HTML-CONTEXT-L2`; `EVID-CR-003`; `EVID-CR-006`; `EVID-CR-018`; `EVID-CODE-001` |

## Objective

Carry the leaders active season/type context into machine-readable browser HTML
markup, then compare that HTML context with the CLI JSON active-context metadata
for the canonical bundled fixture.

## Affected Surfaces

| Surface | Expected Change |
|---|---|
| Web template model | `LeadersTemplate` accepts active season and active season-type fields alongside the existing visible label. |
| Web HTML | `/leaders` renders `data-active-season` and `data-active-season-type` on the leaders meta line without changing the visible layout. |
| CLI/Web parity | The L2 fixture compares CLI JSON active context with Web HTML active-context attributes. |
| Tests | Extend L0 template coverage and add L2 CLI/Web HTML active-context parity coverage. |
| VTRACE docs | Record the browser HTML active-context slice and keep broader `WP-001` risk open. |

## Allowed / Forbidden Scope

Allowed:

- Additive Web leaders HTML active-context attributes sourced from route state.
- Additive template model fields needed to render active season/type metadata.
- Focused CLI JSON versus Web HTML parity test for this one context state.
- Documentation/evidence updates tied to this pulse.

Forbidden:

- Claiming full browser route/accessibility coverage for `VAL-003`.
- Claiming TUI, report/export, query planner, broader provenance, or full
  `WP-001` closure.
- Changing the existing CLI `--json` row-array compatibility contract.
- Reworking season selection or dashboard/global navigation behavior.

## Validation Levels

| Level | Required Check | Status |
|---|---|---|
| L0 | Web leaders template fixture proves active season/type attributes render. | passed |
| L1 | Formatting plus affected Web/CLI clippy checks or recorded blocker. | passed_with_risk |
| L2 | CLI subprocess JSON active context matches Web leaders HTML active-context attributes. | passed |

## Evidence Log

| Evidence | Result |
|---|---|
| Inventory | Pulse 04 proved CLI/Web JSON active-context parity, but browser HTML only carried the formatted label and did not expose canonical season/type values for parity fixtures. |
| L0 | `cargo test -p icelines-web --lib l0_web_leaders_view_round_trips_template_and_json_rows -- --nocapture` passed. |
| L1 | `cargo fmt --check`; `cargo clippy -p icelines-web --lib --no-deps -- -D warnings`; `cargo clippy -p icelines-cli --test goalies_web_cli_parity --no-deps -- -D warnings` passed. Full workspace clippy remains blocked by unrelated `icelines-fetch/src/fletch.rs` `too_many_arguments`. |
| L2 | `cargo test -p icelines-cli --test goalies_web_cli_parity l2_query_leaders_cli_json_and_web_html_active_context_match -- --nocapture` passed. |

## Gate

Gate is `passed_with_risk` for this pulse only. Browser HTML now exposes and tests
the selected leaders active season/type context in parity with the CLI JSON row
metadata. Full `WP-001`, TUI parity, report/export parity, query planner parity,
broader provenance/context coverage, and full browser/accessibility route proof
remain open.
