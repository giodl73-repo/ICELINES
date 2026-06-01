# Pulse 28: Leaders Web Active Position-Chip Route Evidence

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
| Validation / evidence | `VAL-003`; `VAL-004`; `EVID-WP001-HTML-POS-ARIA-L2`; `EVID-WP001-L1`; `EVID-CR-006`; `EVID-CR-014`; `EVID-CR-018`; `EVID-CODE-001` |

## Objective

Promote Pulse 27's rendered-template active position-chip accessibility state to
route-level evidence by comparing the selected CLI JSON envelope position filter
with the Web `/leaders?pos=G` HTML active chip.

## Affected Surfaces

| Surface | Expected Change |
|---|---|
| CLI/Web parity tests | Adds a focused L2 fixture that dispatches the Web `/leaders` route and checks the active chip label, href position, top value, and single `aria-current="true"` marker against the selected CLI JSON envelope state. |
| Web HTML / JSON / CLI output | No product contract change. |
| VTRACE docs | Record route-level evidence for the selected Web active-chip accessibility state without claiming full browser/accessibility or `WP-001` closure. |

## Allowed / Forbidden Scope

Allowed:

- Additive parity evidence for the existing Web active position-chip markup.
- Documentation/evidence updates tied to this pulse.

Forbidden:

- Changing Web JSON, CLI, TUI, Markdown export, CSV, scoring, or data-loading
  contracts.
- Claiming full Web browser/accessibility proof, full route parity, or full
  `WP-001` closure.

## Validation Levels

| Level | Required Check | Status |
|---|---|---|
| L0 | Covered by Pulse 27 rendered-template evidence. | inherited |
| L1 | Formatting plus affected CLI parity-test clippy checks or recorded blocker. | passed_with_risk |
| L2 | CLI JSON envelope selected `position_filter` matches Web route HTML active chip state for `/leaders?sort=goals&pos=G&top=5`, including exactly one `aria-current="true"`. | passed |

## Evidence Log

| Evidence | Result |
|---|---|
| Inventory | Pulse 27 proved rendered-template `aria-current` behavior, but route-level parity evidence for the selected `pos=G` URL was still pending. |
| L2 | `cargo test -p icelines-cli l2_query_leaders_cli_json_and_web_html_active_position_chip_match --test goalies_web_cli_parity` passed. |
| L1 | `cargo fmt --check` passed after `cargo fmt`. Affected CLI parity-test clippy passed. Full Web all-targets clippy and full workspace clippy remain blocked by unrelated existing lint debt outside this pulse. |
| Docs | `C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only` passed. |

## Gate

Gate is `passed_with_risk` for this pulse only. The selected Web leaders
`pos=G` route now has L2 route evidence that the active position chip matches the
CLI JSON envelope selected position filter and exposes exactly one
`aria-current="true"` marker. Full `WP-001`, broader browser/accessibility proof,
broader Web parity, full interactive TUI parity, broader source provenance, and
WP-008 integration rehearsal remain open.
