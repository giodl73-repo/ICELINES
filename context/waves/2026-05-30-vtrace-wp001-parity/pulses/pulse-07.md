# Pulse 07: Leaders Browser HTML Recovery Parity

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
| Parent requirements | `REQ-PARITY-001`; `REQ-WEB-002`; `REQ-DATA-001`; `REQ-CODE-001` |
| Interfaces | `IF-VIEW-001`; `IF-WEB-001`; `IF-DATA-001` |
| Design / code rigor | `DES-001`; `DES-007`; `DES-009`; `DES-014`; `CR-006`; `CR-007`; `CR-008`; `CR-014`; `CR-015`; `CR-032` |
| Validation / evidence | `VAL-003`; `VAL-004`; `EVID-VAL-004`; `EVID-WP001-HTML-RECOVERY-L2`; `EVID-CR-003`; `EVID-CR-006`; `EVID-CR-018`; `EVID-CODE-001` |

## Objective

Carry the leaders goalie-filter empty/warning ViewModel recovery state into the
browser HTML route, then compare the rendered HTML recovery affordance with the
CLI JSON envelope metadata for the canonical bundled fixture.

## Affected Surfaces

| Surface | Expected Change |
|---|---|
| Web template model | `LeadersTemplate` accepts warning messages, empty-state text, and recovery links sourced from the ViewModel. |
| Web HTML | `/leaders?pos=G` renders the unsupported-filter warning, empty-state title/detail, and a recovery link to `/goalies` while rendering no leader rows. |
| CLI/Web parity | The L2 fixture compares CLI JSON envelope `empty_state` and `warnings` metadata with Web HTML recovery text and link semantics. |
| Tests | Add L0 template coverage and L2 CLI/Web HTML recovery parity coverage. |
| VTRACE docs | Record the browser HTML recovery slice and keep broader `WP-001` risk open. |

## Allowed / Forbidden Scope

Allowed:

- Additive Web leaders HTML warning and empty-state recovery rendering for the
  goalie-filter empty result.
- Additive template model fields needed to render ViewModel recovery metadata.
- Focused CLI JSON envelope versus Web HTML parity test for this one recovery
  state.
- Documentation/evidence updates tied to this pulse.

Forbidden:

- Claiming full browser route/accessibility coverage for `VAL-003`.
- Claiming TUI, report/export, query planner, or full `WP-001` closure.
- Changing the existing CLI `--json` row-array compatibility contract.
- Broad goalie leaders or query semantics rework.

## Validation Levels

| Level | Required Check | Status |
|---|---|---|
| L0 | Web leaders template fixture proves empty warning and recovery markup render. | passed |
| L1 | Formatting plus affected Web/CLI clippy checks or recorded blocker. | passed_with_risk |
| L2 | CLI subprocess envelope metadata matches Web leaders HTML empty/warning recovery text and link semantics. | passed |

## Evidence Log

| Evidence | Result |
|---|---|
| Inventory | Pulse 06 proved CLI/Web JSON empty/warning parity but left browser HTML recovery open; Web HTML had no selected recovery affordance for the leaders goalie-filter empty state. |
| L0 | `cargo test -p icelines-web --lib l0_web_leaders_template_renders_empty_warning_recovery -- --nocapture` passed. |
| L1 | `cargo fmt --check`; `cargo clippy -p icelines-web --lib --no-deps -- -D warnings`; `cargo clippy -p icelines-cli --test goalies_web_cli_parity --no-deps -- -D warnings` passed. Full workspace clippy remains blocked by unrelated `icelines-fetch/src/fletch.rs` `too_many_arguments`. |
| L2 | `cargo test -p icelines-cli --test goalies_web_cli_parity l2_query_leaders_cli_json_and_web_html_recovery_match -- --nocapture` passed. |

## Gate

Gate is `passed_with_risk` for this pulse only. Browser HTML now renders and tests
the selected leaders goalie-filter warning, empty-state text, and `/goalies`
recovery link in parity with the CLI JSON envelope. Full `WP-001`, TUI parity,
report/export parity, query planner parity, broader provenance/context coverage,
and full browser/accessibility route proof remain open.
