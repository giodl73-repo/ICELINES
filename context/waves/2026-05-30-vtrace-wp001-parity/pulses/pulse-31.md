# Pulse 31: Leaders TUI Active Filter Result Evidence

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
| Validation / evidence | `VAL-001`; `VAL-004`; `EVID-WP001-TUI-ACTIVE-FILTER-L0`; `EVID-WP001-L0`; `EVID-WP001-L1`; `EVID-CR-003`; `EVID-CR-018`; `EVID-CODE-001` |

## Objective

Promote the selected TUI leaders free-form active-filter path from helper coverage
to rendered-results evidence by proving the TUI Stats results panel both applies a
parsed `country=CAN` filter to the result set and displays the active filter in
the visible query-result metadata line.

## Affected Surfaces

| Surface | Expected Change |
|---|---|
| TUI Stats tests | Adds a focused L0 render fixture with two rows, one matching and one filtered out, then asserts the rendered result line shows `active_filters country=CAN` and only the matching row remains visible. |
| TUI runtime behavior | No product contract change; the pulse records evidence for existing active-filter execution/rendering behavior. |
| VTRACE docs | Record selected TUI active-filter render evidence without claiming full interactive TUI parity or full `WP-001` closure. |

## Allowed / Forbidden Scope

Allowed:

- Additive TUI render evidence for existing leaders free-form filter behavior.
- Documentation/evidence updates tied to this pulse.

Forbidden:

- Changing query grammar, parser semantics, data loading, Web/CLI/CSV/export
  contracts, or TUI interaction flow.
- Claiming full interactive TUI parity, full query-planner parity, or full
  `WP-001` closure.

## Validation Levels

| Level | Required Check | Status |
|---|---|---|
| L0 | TUI Stats leaders rendered-results fixture applies `country=CAN`, reports `total 1`, `returned 1`, `sort goals`, and `active_filters country=CAN`, and hides the non-matching row. | passed |
| L1 | Formatting plus affected CLI bin clippy checks or recorded blocker. | passed_with_risk |
| L2 | Not claimed for this pulse; full interactive TUI parity remains pending overall. | pending_overall |

## Evidence Log

| Evidence | Result |
|---|---|
| Inventory | Pulse 16 proved selected TUI query-result metadata rendering and Pulse 25 proved selected TUI empty/warning recovery rendering, but rendered evidence that a free-form active filter affects visible TUI results while surfacing active-filter metadata was still pending. |
| L0 | `cargo test -p icelines-cli tui::screens::queries::tests::l0_tui_leaders_results_render_active_filter_result_state --bin icelines -- --nocapture` passed. |
| L1 | `cargo fmt --check` passed. Affected CLI bin clippy passed. Full workspace clippy remains blocked by unrelated existing lint debt outside this pulse. |
| Docs | `C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only` passed. |

## Gate

Gate is `passed_with_risk` for this pulse only. The selected TUI leaders Stats
rendered-results path now has L0 evidence that a free-form active filter is both
applied to rows and surfaced in the visible result metadata. Full `WP-001`, full
interactive TUI parity, broader query-planner parity, broader source provenance,
and WP-008 integration rehearsal remain open.
