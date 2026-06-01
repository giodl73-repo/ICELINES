# Pulse 12: Leaders CLI Text Context and Source-State Disclosure

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
| Parent requirements | `REQ-WB-002`; `REQ-DATA-001`; `REQ-PARITY-001`; `REQ-QUERY-001`; `REQ-CODE-001` |
| Interfaces | `IF-DATA-001`; `IF-VIEW-001`; `IF-QUERY-001` |
| Design / code rigor | `DES-001`; `DES-002`; `DES-003`; `DES-009`; `DES-014`; `CR-003`; `CR-006`; `CR-007`; `CR-010`; `CR-011`; `CR-032` |
| Validation / evidence | `VAL-004`; `EVID-WP001-QUERY-TEXT-CONTEXT-L0`; `EVID-WP001-QUERY-TEXT-CONTEXT-L2`; `EVID-WP001-L1`; `EVID-CR-003`; `EVID-CR-018`; `EVID-CODE-001` |

## Objective

Carry the selected leaders active season/type context and roster source-state into
the default human-readable `query leaders` CLI text output, so the text table is
not the last active leaders surface without visible context/source disclosure.

## Affected Surfaces

| Surface | Expected Change |
|---|---|
| CLI query leaders text | Prints `Context: <season> <season_type> | source <kind> <completeness>` before the leaders table. |
| CLI query leaders improvement text | Prints the same context line before the improvement table, because it shares the leaders result path. |
| Tests | Focused unit and CLI subprocess tests assert the context/source-state text output. |
| VTRACE docs | Record this as a query/text affected-slice improvement without claiming full query planner, TUI, report/export, browser, or `WP-001` closure. |

## Allowed / Forbidden Scope

Allowed:

- Additive text output line sourced from `LeadersView.context`.
- Focused unit/subprocess tests for the selected CLI text leaders context/source-state disclosure.
- Documentation/evidence updates tied to this pulse.

Forbidden:

- Claiming full query planner parity, full interactive TUI parity, broader browser
  route/accessibility proof, broader provenance/context coverage, full
  report/export parity, or full `WP-001` closure.
- Changing CLI JSON, CLI CSV, Web JSON/HTML, TUI, or Markdown export contracts
  from pulses 01 through 11.
- Reworking ranking, filtering, scoring, data loading, or the query grammar.

## Validation Levels

| Level | Required Check | Status |
|---|---|---|
| L0 | CLI text renderer helper unit test proves selected active context and source state render from `LeadersView.context`. | passed |
| L1 | Formatting plus affected CLI bin clippy checks or recorded blocker. | passed_with_risk |
| L2 | CLI subprocess query test proves default text stdout includes context and source-state disclosure before the table. | passed |

## Evidence Log

| Evidence | Result |
|---|---|
| Inventory | JSON, Web HTML, TUI, Markdown body, and Markdown front matter carried selected leaders context/source-state, but default `query leaders` text output jumped directly to the leaders table. |
| L0 | `cargo test -p icelines-cli commands::query::tests::l0_query_leaders_context_line_reports_context_and_source_state --bin icelines -- --nocapture` passed. |
| L2 | `cargo test -p icelines-cli --test system_tests l2_cmd_query_leaders_exits_zero -- --nocapture` passed. |
| L1 | `cargo fmt --check`; `cargo clippy -p icelines-cli --bin icelines --no-deps -- -D warnings` passed. Full workspace clippy remains blocked by unrelated `icelines-fetch/src/fletch.rs` `too_many_arguments`. |

## Gate

Gate is `passed_with_risk` for this pulse only. Default leaders CLI text output
now exposes the selected active season/type and complete roster source-state
before the table. Full `WP-001`, full query planner parity, broader provenance,
full TUI parity, full report/export parity, and full browser/accessibility route
proof remain open.
