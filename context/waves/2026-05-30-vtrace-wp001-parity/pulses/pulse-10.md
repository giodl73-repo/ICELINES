# Pulse 10: Leaders Markdown Export Context and Source-State Presentation

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
| Parent requirements | `REQ-WB-002`; `REQ-DATA-001`; `REQ-PARITY-001`; `REQ-REPORT-001`; `REQ-CODE-001` |
| Interfaces | `IF-DATA-001`; `IF-VIEW-001`; `IF-REPORT-001` |
| Design / code rigor | `DES-001`; `DES-002`; `DES-003`; `DES-009`; `DES-014`; `CR-003`; `CR-006`; `CR-007`; `CR-011`; `CR-016`; `CR-017`; `CR-032` |
| Validation / evidence | `VAL-002`; `VAL-004`; `EVID-VAL-002C`; `EVID-VAL-004`; `EVID-WP001-EXPORT-CONTEXT-L0`; `EVID-WP001-EXPORT-CONTEXT-L2`; `EVID-WP001-L1`; `EVID-CR-003`; `EVID-CR-007`; `EVID-CR-013`; `EVID-CR-018`; `EVID-CODE-001` |

## Objective

Carry the selected leaders active season/type context and roster source-state into
`export md leaders`, then render a stable top-of-report context/source section so
Markdown reports no longer hide the same source-honesty fields already carried by
the CLI/Web/TUI leaders slices.

## Affected Surfaces

| Surface | Expected Change |
|---|---|
| CLI Markdown export ViewModel adapter | `render_leaders_from_views` populates `ViewContext.source_state` with `complete` / `roster` for the selected bundled leaders result. |
| Markdown leaders report | Report body renders `## Context`, active season/type, and `roster: complete` before the leaders table. |
| Tests | Focused unit tests and the CLI subprocess export test assert the context/source section. |
| VTRACE docs | Record this as a report/export affected-slice improvement without claiming full report/export parity, public historical reporting, or full `WP-001` closure. |

## Allowed / Forbidden Scope

Allowed:

- Additive Markdown leaders context/source display sourced from `LeadersView.context`.
- Focused unit/subprocess tests for the selected Markdown leaders report slice.
- Documentation/evidence updates tied to this pulse.

Forbidden:

- Claiming full report/export parity, full `VAL-002`, or full `WP-004` closure.
- Claiming full interactive TUI parity, broader browser route/accessibility proof,
  query planner parity, broader provenance/context coverage, or full `WP-001`
  closure.
- Changing CLI/Web JSON or browser HTML contracts from pulses 01 through 09.
- Reworking the query parser, scoring methodology, or historical report fixtures.

## Validation Levels

| Level | Required Check | Status |
|---|---|---|
| L0 | Markdown leaders unit tests prove selected active context and source state survive into the rendered report before the table. | passed |
| L1 | Formatting plus affected CLI bin clippy checks or recorded blocker. | passed_with_risk |
| L2 | CLI subprocess export test proves stdout report includes the context/source section. | passed |

## Evidence Log

| Evidence | Result |
|---|---|
| Inventory | CLI/Web/TUI leaders slices already carried selected source/context fields, but `export md leaders` built a `LeadersView` with an empty `source_state` and emitted no top-of-report context/source section. |
| L0 | `cargo test -p icelines-cli commands::export::tests::l0_export_leaders --bin icelines -- --nocapture` passed. |
| L2 | `cargo test -p icelines-cli --test system_tests l2_cmd_export_md_leaders_to_stdout -- --nocapture` passed. |
| L1 | `cargo fmt --check`; `cargo clippy -p icelines-cli --bin icelines --no-deps -- -D warnings` passed. Full workspace clippy remains blocked by unrelated `icelines-fetch/src/fletch.rs` `too_many_arguments`. |

## Gate

Gate is `passed_with_risk` for this pulse only. Markdown leaders export now exposes
the selected active season/type and complete roster source-state before the table.
Full `WP-001`, full report/export parity, historical report/public-copy evidence,
query planner parity, broader provenance/context coverage, full TUI parity, and
full browser/accessibility route proof remain open.
