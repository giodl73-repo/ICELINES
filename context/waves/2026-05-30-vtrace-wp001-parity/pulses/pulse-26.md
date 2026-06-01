# Pulse 26: Leaders Web Position-Chip Goalie Recovery Affordance

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
| Validation / evidence | `VAL-003`; `VAL-004`; `EVID-WP001-HTML-POS-CHIP-L0`; `EVID-WP001-L1`; `EVID-CR-006`; `EVID-CR-014`; `EVID-CR-018`; `EVID-CODE-001` |

## Objective

Expose the existing Web leaders goalie-filter recovery path through the visible
position-chip strip by adding a `G` chip that activates the already-supported
`pos=G` empty/warning/recovery state.

## Affected Surfaces

| Surface | Expected Change |
|---|---|
| Web leaders HTML | Adds a visible `G` position chip after the existing skater/forward/defense chips. |
| Web leaders handler | Centralizes position-chip construction so the goalie chip and active state are testable. |
| Web JSON / CLI / TUI / Markdown / CSV | No contract change. |
| Tests | Focused Web L0 assertion that the chip strip includes `G`, preserves order, and marks it active for `pos=G`. |
| VTRACE docs | Record this as a Web HTML recovery-affordance improvement without claiming full browser/accessibility or `WP-001` closure. |

## Allowed / Forbidden Scope

Allowed:

- Additive Web HTML affordance for the existing goalie-filter recovery path.
- Focused Web handler/unit checks for chip presence, ordering, and active state.
- Documentation/evidence updates tied to this pulse.

Forbidden:

- Changing Web JSON, CLI, TUI, Markdown export, CSV, scoring, or data-loading
  contracts.
- Claiming full Web browser/accessibility proof, full route parity, or full
  `WP-001` closure.

## Validation Levels

| Level | Required Check | Status |
|---|---|---|
| L0 | Web handler test proves the leaders position-chip list includes `G`, preserves chip order, and marks `G` active when selected. | passed |
| L1 | Formatting plus affected Web lib clippy checks or recorded blocker. | passed_with_risk |
| L2 | Browser-route/accessibility parity remains pending overall; this pulse records L0 affordance evidence only. | pending_overall |

## Evidence Log

| Evidence | Result |
|---|---|
| Inventory | The Web leaders route already accepted `pos=G` and rendered the ViewModel empty/warning/recovery state when supplied in the URL, but the visible position-chip strip exposed only `All`, `C`, `LW`, `RW`, `F`, and `D`. |
| L0 | `cargo test -p icelines-web l0_web_leaders_position_chips_include_goalie_recovery_filter --lib` passed. |
| L1 | `cargo fmt --check` passed. `cargo clippy -p icelines-web --lib --no-deps -- -D warnings` passed. Full Web all-targets clippy and full workspace clippy remain blocked by unrelated existing lint debt outside this pulse. |
| Docs | `C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only` passed. |

## Gate

Gate is `passed_with_risk` for this pulse only. Web leaders now exposes the
goalie recovery filter as a visible position chip while preserving the existing
`pos=G` ViewModel-backed empty/warning/recovery behavior and all non-Web
contracts. Full `WP-001`, browser route/accessibility proof, broader Web parity,
full interactive TUI parity, broader source provenance, and WP-008 integration
rehearsal remain open.
