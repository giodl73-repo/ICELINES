# Pulse 22: Leaders CLI Text Empty/Warning Recovery

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
| Parent requirements | `REQ-QUERY-001`; `REQ-DATA-001`; `REQ-PARITY-001`; `REQ-CODE-001` |
| Interfaces | `IF-DATA-001`; `IF-VIEW-001`; `IF-QUERY-001` |
| Design / code rigor | `DES-001`; `DES-002`; `DES-003`; `DES-009`; `DES-014`; `CR-003`; `CR-006`; `CR-007`; `CR-010`; `CR-011`; `CR-014`; `CR-015`; `CR-032` |
| Validation / evidence | `VAL-004`; `EVID-WP001-L0`; `EVID-WP001-L1`; `EVID-WP001-QUERY-TEXT-EMPTY-WARNING-L0`; `EVID-WP001-QUERY-TEXT-EMPTY-WARNING-L2`; `EVID-CR-003`; `EVID-CR-006`; `EVID-CR-018`; `EVID-CODE-001` |

## Objective

Carry selected leaders empty-state, warning, detail, and recovery-route metadata
into default CLI text output for the goalie-filter empty result, using the same
`LeadersView` state already carried by the CLI JSON envelope.

## Affected Surfaces

| Surface | Expected Change |
|---|---|
| CLI `query leaders` text | Adds a warning/empty/recovery block after existing `Context:` and `Result:` lines for selected empty results. |
| CLI query leaders JSON envelope | No contract change; existing opt-in envelope `meta.empty_state` and `meta.warnings` remain the source of truth. |
| CLI query leaders CSV | No contract change. |
| TUI/Web/Markdown export | No contract change. |
| Tests | Focused CLI L0 renderer assertion plus L2 CLI subprocess text-vs-JSON-envelope assertion for selected empty/warning recovery. |
| VTRACE docs | Record this as a default CLI text affected-slice improvement without claiming full CLI/TUI/Web/report parity or `WP-001` closure. |

## Allowed / Forbidden Scope

Allowed:

- Additive default CLI text rendering for existing `LeadersView.warnings` and
  `LeadersView.empty_state` fields.
- Focused CLI text recovery checks for the selected goalie-filter empty result.
- Documentation/evidence updates tied to this pulse.

Forbidden:

- Claiming full CLI, TUI, Web, report/export, query planner, browser route,
  accessibility, broader provenance, or full `WP-001` closure.
- Changing ranking, filtering, scoring, data loading, JSON, CSV, TUI, Web HTML,
  Markdown export, or query grammar contracts.

## Validation Levels

| Level | Required Check | Status |
|---|---|---|
| L0 | CLI text helper test proves selected warning, empty-state kind/title/detail, and recovery route render from the ViewModel. | passed |
| L1 | Formatting plus affected CLI clippy checks or recorded blocker. | passed_with_risk |
| L2 | CLI subprocess default text output is checked against the CLI JSON envelope empty/warning metadata. | passed |

## Evidence Log

| Evidence | Result |
|---|---|
| Inventory | CLI JSON envelope carried selected leaders empty/warning/recovery metadata, but default CLI text only exposed context/result/table framing for the goalie-filter empty result. |
| L0 | `cargo test -p icelines-cli l0_query_leaders_warning_empty_lines_report_recovery --bin icelines` passed. |
| L2 | `cargo test -p icelines-cli l2_query_leaders_cli_text_renders_empty_warning_recovery_state --test goalies_web_cli_parity` passed. |
| L1 | `cargo fmt --check` passed after formatting. Affected CLI clippy passed in the final validation run. Full workspace clippy remains blocked by unrelated existing lint debt in `icelines-fetch`. |

## Gate

Gate is `passed_with_risk` for this pulse only. Default CLI leaders text now
surfaces selected empty/warning detail and `/goalies` recovery guidance from the
existing ViewModel state while preserving existing context, result, table,
JSON, CSV, TUI, Web, and Markdown export contracts. Full `WP-001`, full CLI/TUI/
Web/report parity, broader source provenance, full query planner parity, and
WP-008 integration rehearsal remain open.
