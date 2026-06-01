# Pulse 09: Leaders TUI Context and Source-State Presentation

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
| Parent requirements | `REQ-WB-002`; `REQ-DATA-001`; `REQ-PARITY-001`; `REQ-CODE-001` |
| Interfaces | `IF-DATA-001`; `IF-VIEW-001`; `IF-QUERY-001` |
| Design / code rigor | `DES-001`; `DES-002`; `DES-003`; `DES-009`; `DES-014`; `CR-003`; `CR-006`; `CR-007`; `CR-008`; `CR-011`; `CR-032` |
| Validation / evidence | `VAL-001`; `VAL-004`; `EVID-VAL-004`; `EVID-WP001-TUI-CONTEXT-L0`; `EVID-WP001-L1`; `EVID-CR-003`; `EVID-CR-011`; `EVID-CR-018`; `EVID-CODE-001` |

## Objective

Carry the selected leaders active season/type context and roster source-state into
the TUI Stats results `LeadersView`, then render a stable context/source line so
the TUI no longer hides the same source-honesty fields already carried by the
CLI/Web leaders slices.

## Affected Surfaces

| Surface | Expected Change |
|---|---|
| TUI Stats ViewModel adapter | `leaders_view_from_query_results` populates `ViewContext.source_state` with `complete` / `roster` for the selected bundled leaders result. |
| TUI Stats results panel | Results render `Context: 20242025 regular | source roster complete` above the leaders table. |
| Tests | Focused TUI unit/render tests assert ViewContext season/type/source state and the rendered context/source line. |
| VTRACE docs | Record this as a TUI affected-slice improvement without claiming full TUI parity, report/export parity, or full `WP-001` closure. |

## Allowed / Forbidden Scope

Allowed:

- Additive TUI leaders context/source display sourced from `LeadersView.context`.
- Focused unit/render tests for the selected TUI leaders result slice.
- Documentation/evidence updates tied to this pulse.

Forbidden:

- Claiming full interactive TUI parity or full `VAL-001` workbench validation.
- Claiming report/export, query planner, browser route/accessibility, or full
  `WP-001` closure.
- Changing CLI/Web JSON or browser HTML contracts from pulses 01 through 08.
- Reworking the query parser, app navigation, or season picker.

## Validation Levels

| Level | Required Check | Status |
|---|---|---|
| L0 | TUI leaders unit/render tests prove selected active context and source state survive into rendered Stats results. | passed |
| L1 | Formatting plus affected CLI bin clippy checks or recorded blocker. | passed_with_risk |
| L2 | Interactive CLI/TUI/Web/report parity rehearsal. | pending_overall; not required for this narrow TUI presentation pulse |

## Evidence Log

| Evidence | Result |
|---|---|
| Inventory | CLI/Web leaders JSON and Web HTML already carried selected source/context fields, but the TUI Stats leaders path built a `LeadersView` with an empty `source_state` and rendered no context/source line. |
| L0 | `cargo test -p icelines-cli tui::screens::queries::tests::l0_tui_leaders --bin icelines -- --nocapture` passed. |
| L1 | `cargo fmt --check`; `cargo clippy -p icelines-cli --bin icelines --no-deps -- -D warnings` passed. Full workspace clippy remains blocked by unrelated `icelines-fetch/src/fletch.rs` `too_many_arguments`. |

## Gate

Gate is `passed_with_risk` for this pulse only. TUI Stats now exposes the selected
leaders active season/type and complete roster source-state in the rendered
results panel. Full `WP-001`, interactive TUI parity, report/export parity, query
planner parity, broader provenance/context coverage, and full browser/accessibility
route proof remain open.
