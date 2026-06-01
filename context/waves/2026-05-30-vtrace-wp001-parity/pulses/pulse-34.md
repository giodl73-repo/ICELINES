# Pulse 34: Leaders TUI Active Filter L2 Parity

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
| Design / code rigor | `DES-003`; `DES-009`; `DES-014`; `CR-003`; `CR-010`; `CR-011`; `CR-018`; `CR-032` |
| Validation / evidence | `VAL-001`; `VAL-004`; `EVID-WP001-TUI-ACTIVE-FILTER-L2`; `EVID-WP001-L1`; `EVID-CR-003`; `EVID-CR-018`; `EVID-CODE-001` |

## Objective

Close the selected TUI leaders Stats active-filter L2 gap by rendering the
existing TUI results panel through a non-interactive test backend and comparing
the visible `country=CAN` result metadata and rows against the canonical
`query leaders --json-envelope` output.

## Affected Surfaces

| Surface | Expected Change |
|---|---|
| TUI rendering seam | Adds a hidden non-interactive snapshot helper that renders the existing TUI Stats leaders results panel through `ratatui::backend::TestBackend` without entering raw terminal mode. |
| CLI validation seam | Adds a hidden `tui --render-leaders-active-filter-snapshot` flag for system-test evidence only. |
| CLI system tests | Adds L2 evidence comparing the hidden TUI snapshot with `query leaders --json-envelope` for `country=CAN`. |
| VTRACE docs | Record selected TUI active-filter L2 evidence without claiming full interactive TUI parity or full `WP-001` closure. |

## Allowed / Forbidden Scope

Allowed:

- Additive hidden validation seam for deterministic TUI snapshot evidence.
- Reuse of the existing TUI Stats `render_results` path and query filter
  parsing/evaluation helpers.
- Focused CLI subprocess L2 evidence.
- Documentation/evidence updates tied to this pulse.

Forbidden:

- Changing query grammar, filter semantics, ranking semantics, Web/CSV/export
  contracts, or public TUI behavior.
- Claiming full interactive TUI parity, full query-planner parity, or full
  `WP-001` closure.

## Validation Levels

| Level | Required Check | Status |
|---|---|---|
| L0 | Existing TUI render and CLI parser tests continue to pass with the hidden snapshot flag present. | passed |
| L1 | Formatting plus affected CLI system-test clippy checks or recorded blocker. | passed_with_risk |
| L2 | `tui --render-leaders-active-filter-snapshot` matches `query leaders --season 20242025 --sort goals --filter country=CAN --top 20 --json-envelope` result metadata and filtered rows; TUI snapshot excludes unfiltered non-CAN leader `Leon Draisaitl`. | passed |

## Evidence Log

| Evidence | Result |
|---|---|
| Inventory | Pulses 16 and 31 proved selected TUI result metadata and active-filter rendering at L0, but no subprocess fixture compared the TUI Stats output against the canonical query JSON envelope. |
| L0 | `cargo test -p icelines-cli cli::tests:: -- --nocapture` passed. `cargo test -p icelines-cli l0_tui -- --nocapture` passed. |
| L2 | `cargo test -p icelines-cli --test system_tests l2_cmd_tui_stats_active_filter_result_state_matches_query_envelope -- --nocapture` passed. |
| L1 | `cargo fmt --check` passed. Affected CLI system-test clippy passed. Full workspace clippy remains blocked by unrelated existing lint debt outside this pulse. |
| Docs | `C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only` passed. |

## Gate

Gate is `passed_with_risk` for this pulse only. The selected TUI leaders Stats
path now has L2 evidence that a free-form active filter is applied to rows,
surfaced in visible result metadata, and aligned with the query JSON envelope
for the same season/type window. Full interactive TUI parity, broader
`WP-001`, broader query-planner parity, broader source provenance, and WP-008
integration rehearsal remain open.
