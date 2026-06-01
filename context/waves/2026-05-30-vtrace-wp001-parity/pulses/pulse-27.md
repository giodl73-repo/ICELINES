# Pulse 27: Leaders Web Active Position-Chip Accessibility State

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
| Parent requirements | `REQ-WEB-002`; `REQ-PARITY-001`; `REQ-QUERY-001`; `REQ-CODE-001` |
| Interfaces | `IF-WEB-001`; `IF-VIEW-001`; `IF-QUERY-001` |
| Design / code rigor | `DES-003`; `DES-007`; `DES-009`; `DES-014`; `CR-006`; `CR-010`; `CR-014`; `CR-027`; `CR-032` |
| Validation / evidence | `VAL-003`; `VAL-004`; `EVID-WP001-HTML-POS-ARIA-L0`; `EVID-WP001-L1`; `EVID-CR-006`; `EVID-CR-014`; `EVID-CR-018`; `EVID-CODE-001` |

## Objective

Expose the active Web leaders position-filter route state to browser/accessibility
semantics by marking the selected position chip with `aria-current="true"`.

## Affected Surfaces

| Surface | Expected Change |
|---|---|
| Web leaders HTML | Adds `aria-current="true"` to the active position chip, including the selected `G` goalie recovery chip. |
| Web leaders handler tests | Adds focused template-render evidence that only the selected chip carries current-route state. |
| Web JSON / CLI / TUI / Markdown / CSV | No contract change. |
| VTRACE docs | Record this as a selected Web HTML accessibility-state improvement without claiming full browser/accessibility or `WP-001` closure. |

## Allowed / Forbidden Scope

Allowed:

- Additive Web HTML accessibility/current-state metadata for the existing position
  chip strip.
- Focused Web template/unit checks for selected-chip current-state rendering.
- Documentation/evidence updates tied to this pulse.

Forbidden:

- Changing Web JSON, CLI, TUI, Markdown export, CSV, scoring, or data-loading
  contracts.
- Claiming full Web browser/accessibility proof, full route parity, or full
  `WP-001` closure.

## Validation Levels

| Level | Required Check | Status |
|---|---|---|
| L0 | Web template test proves the selected leaders position chip renders exactly one `aria-current="true"` marker and preserves the selected `G` route. | passed |
| L1 | Formatting plus affected Web lib clippy checks or recorded blocker. | passed_with_risk |
| L2 | Browser-route/accessibility parity remains pending overall; this pulse records L0 rendered-markup evidence only. | pending_overall |

## Evidence Log

| Evidence | Result |
|---|---|
| Inventory | Pulse 26 exposed the visible `G` chip, but active position state remained visual-only (`fit-solid`) in the rendered chip strip. |
| L0 | `cargo test -p icelines-web l0_web_leaders_active_position_chip_exposes_current_route_state --lib` passed. |
| L0 regression | `cargo test -p icelines-web l0_web_leaders_position_chips_include_goalie_recovery_filter --lib` passed before formatting. |
| L1 | `cargo fmt --check` passed after `cargo fmt`. `cargo clippy -p icelines-web --lib --no-deps -- -D warnings` passed. Full Web all-targets clippy and full workspace clippy remain blocked by unrelated existing lint debt outside this pulse. |
| Docs | `C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only` passed. |

## Gate

Gate is `passed_with_risk` for this pulse only. Web leaders now exposes selected
position-filter route state through `aria-current="true"` on the active chip while
preserving all existing query and ViewModel behavior. Full `WP-001`, browser
route/accessibility proof, broader Web parity, full interactive TUI parity,
broader source provenance, and WP-008 integration rehearsal remain open.
