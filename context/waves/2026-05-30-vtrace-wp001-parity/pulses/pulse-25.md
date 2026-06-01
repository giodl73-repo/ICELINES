# Pulse 25: Leaders TUI Empty/Warning Recovery State

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
| Parent requirements | `REQ-WB-002`; `REQ-QUERY-001`; `REQ-DATA-001`; `REQ-PARITY-001`; `REQ-CODE-001` |
| Interfaces | `IF-DATA-001`; `IF-VIEW-001`; `IF-QUERY-001` |
| Design / code rigor | `DES-001`; `DES-002`; `DES-003`; `DES-009`; `DES-014`; `CR-003`; `CR-007`; `CR-010`; `CR-011`; `CR-032` |
| Validation / evidence | `VAL-001`; `VAL-004`; `EVID-WP001-L0`; `EVID-WP001-L1`; `EVID-WP001-TUI-EMPTY-WARNING-L0`; `EVID-CR-003`; `EVID-CR-018`; `EVID-CODE-001` |

## Objective

Carry selected leaders empty-state, warning, detail, and recovery-route metadata
into the TUI Stats results panel for the goalie-filter empty result, using the
same `LeadersView` state already carried by CLI text, Markdown export, and Web
renderers.

## Affected Surfaces

| Surface | Expected Change |
|---|---|
| TUI Stats leaders results | Adds a goalie position option that renders `unsupported_filter`, `no_rows`, detail text, and `/goalies` recovery guidance when selected. |
| TUI query filtering | Applies the goalie position filter explicitly so the skater-only leaders view has an empty result instead of silently ignoring `G`. |
| CLI/Web/Markdown/CSV | No contract change. |
| Tests | Focused TUI L0 state and render assertions for selected empty/warning recovery state. |
| VTRACE docs | Record this as a TUI affected-slice improvement without claiming full interactive TUI parity or `WP-001` closure. |

## Allowed / Forbidden Scope

Allowed:

- Additive TUI rendering for existing `LeadersView.warnings` and
  `LeadersView.empty_state` fields.
- Focused TUI checks for the selected goalie-filter empty result.
- Documentation/evidence updates tied to this pulse.

Forbidden:

- Claiming full CLI, TUI, Web, report/export, query planner, browser route,
  accessibility, broader provenance, or full `WP-001` closure.
- Changing CLI, JSON, CSV, Web, Markdown export, scoring, data loading, or
  non-leaders TUI contracts.

## Validation Levels

| Level | Required Check | Status |
|---|---|---|
| L0 | TUI helper and render tests prove selected warning, empty-state kind/title/detail, and recovery route appear in the Stats results panel from ViewModel state. | passed |
| L1 | Formatting plus affected CLI clippy checks or recorded blocker. | passed_with_risk |
| L2 | Interactive TUI parity remains pending overall; this pulse records L0 render evidence only. | pending_overall |

## Evidence Log

| Evidence | Result |
|---|---|
| Inventory | TUI Stats leaders already rendered active context/source and result metadata, but did not render the selected goalie-filter warning, empty-state detail, or recovery guidance carried by other surfaces. |
| L0 | `cargo test -p icelines-cli l0_tui_leaders_goalie_filter_reports_empty_warning_recovery_state --bin icelines` passed. |
| L0 render | `cargo test -p icelines-cli l0_tui_leaders_results_render_goalie_filter_empty_warning_recovery_state --bin icelines` passed. |
| L1 | `cargo fmt --check` passed after formatting. `cargo clippy -p icelines-cli --bin icelines --no-deps -- -D warnings` passed. Full workspace clippy remains blocked by unrelated existing lint debt in `icelines-fetch`. |
| Docs | `C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only` passed. |

## Gate

Gate is `passed_with_risk` for this pulse only. TUI Stats leaders now surfaces
selected empty/warning detail and `/goalies` recovery guidance from the existing
ViewModel state while preserving existing CLI, JSON, CSV, Web, Markdown export,
and non-empty TUI row contracts. Full `WP-001`, full interactive TUI parity,
broader source provenance, full query planner parity, and WP-008 integration
rehearsal remain open.
