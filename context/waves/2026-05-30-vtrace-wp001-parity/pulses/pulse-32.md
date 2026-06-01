# Pulse 32: Leaders CLI Text Active Filter L2 Evidence

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
| Parent requirements | `REQ-QUERY-001`; `REQ-PARITY-001`; `REQ-WB-002`; `REQ-CODE-001` |
| Interfaces | `IF-VIEW-001`; `IF-QUERY-001`; `IF-DATA-001` |
| Design / code rigor | `DES-003`; `DES-009`; `DES-014`; `CR-003`; `CR-010`; `CR-018`; `CR-032` |
| Validation / evidence | `VAL-004`; `EVID-WP001-QUERY-TEXT-ACTIVE-FILTER-L2`; `EVID-WP001-L0`; `EVID-WP001-L1`; `EVID-CR-003`; `EVID-CR-018`; `EVID-CODE-001` |

## Objective

Add L2 subprocess proof that default CLI leaders text output both surfaces and
applies a selected free-form active filter by comparing `country=CAN` text output
against the same command's JSON envelope metadata and rows.

## Affected Surfaces

| Surface | Expected Change |
|---|---|
| CLI system tests | Adds a focused L2 fixture that runs `query leaders --season 20242025 --sort goals --filter country=CAN --top 5` as default text and as `--json-envelope`. |
| CLI runtime behavior | No product contract change; the pulse records evidence for existing text rendering and filter execution behavior. |
| VTRACE docs | Record selected CLI text active-filter application evidence without claiming full query-planner parity or full `WP-001` closure. |

## Allowed / Forbidden Scope

Allowed:

- Additive CLI subprocess evidence for existing leaders free-form filter behavior.
- Documentation/evidence updates tied to this pulse.

Forbidden:

- Changing query grammar, parser semantics, data loading, Web/TUI/CSV/export
  contracts, or CLI output shape.
- Claiming full query-planner parity, full surface parity, or full `WP-001`
  closure.

## Validation Levels

| Level | Required Check | Status |
|---|---|---|
| L0 | Existing text renderer/result metadata and query fixtures remain in force. | passed |
| L1 | Formatting plus affected CLI system-test clippy checks or recorded blocker. | passed_with_risk |
| L2 | CLI text output for `country=CAN` matches JSON-envelope `total`, `returned`, `top`, `sort`, and `active_filters`; includes each filtered JSON row; excludes unfiltered non-CAN leader `Leon Draisaitl`. | passed |

## Evidence Log

| Evidence | Result |
|---|---|
| Inventory | Pulse 15 proved default CLI text result metadata, but active-filter L2 evidence did not prove the rendered text rows matched the applied free-form filter and excluded an unfiltered nonmatching leader. |
| L2 | `cargo test -p icelines-cli --test system_tests l2_cmd_query_leaders_text_active_filter_result_state_matches_json_envelope -- --nocapture` passed. |
| L1 | `cargo fmt --check` passed. Affected CLI system-test clippy passed. Full workspace clippy remains blocked by unrelated existing lint debt outside this pulse. |
| Docs | `C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only` passed. |

## Gate

Gate is `passed_with_risk` for this pulse only. The selected default CLI leaders
text path now has L2 evidence that a free-form active filter is both applied to
rows and surfaced in the visible result metadata. Full `WP-001`, broader
query-planner parity, broader source provenance, broader cross-surface parity,
and WP-008 integration rehearsal remain open.
