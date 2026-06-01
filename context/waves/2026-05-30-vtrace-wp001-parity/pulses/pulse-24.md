# Pulse 24: Leaders Markdown Export Front Matter Empty/Warning State

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
| Parent requirements | `REQ-REPORT-001`; `REQ-QUERY-001`; `REQ-DATA-001`; `REQ-PARITY-001`; `REQ-CODE-001` |
| Interfaces | `IF-DATA-001`; `IF-VIEW-001`; `IF-REPORT-001`; `IF-QUERY-001` |
| Design / code rigor | `DES-001`; `DES-002`; `DES-003`; `DES-009`; `DES-014`; `CR-003`; `CR-007`; `CR-010`; `CR-011`; `CR-013`; `CR-016`; `CR-017`; `CR-032` |
| Validation / evidence | `VAL-002`; `VAL-004`; `EVID-WP001-L0`; `EVID-WP001-L1`; `EVID-WP001-EXPORT-FM-EMPTY-WARNING-L0`; `EVID-WP001-EXPORT-FM-EMPTY-WARNING-L2`; `EVID-CR-003`; `EVID-CR-007`; `EVID-CR-018`; `EVID-CODE-001` |

## Objective

Carry selected leaders empty-state, warning, detail, and recovery-route metadata
into Markdown export YAML front matter for the goalie-filter empty result, using
the same `LeadersView` state already carried by the CLI JSON envelope and report
body.

## Affected Surfaces

| Surface | Expected Change |
|---|---|
| `export md leaders` front matter | Adds a `state` block with `empty_state` and `warnings` metadata for selected empty results. |
| CLI query leaders JSON envelope | No contract change; existing opt-in envelope `meta.empty_state` and `meta.warnings` remain the comparison source. |
| Markdown export report body | No contract change; pulse 23 report-body sections remain intact. |
| CLI query text / CSV, TUI, Web | No contract change. |
| Tests | Focused export L0 renderer assertion plus L2 CLI subprocess export-vs-JSON-envelope assertion for selected front-matter empty/warning state. |
| VTRACE docs | Record this as a Markdown front-matter affected-slice improvement without claiming full report/export parity or `WP-001` closure. |

## Allowed / Forbidden Scope

Allowed:

- Additive Markdown front-matter rendering for existing `LeadersView.warnings`
  and `LeadersView.empty_state` fields.
- Focused export checks for the selected goalie-filter empty result.
- Documentation/evidence updates tied to this pulse.

Forbidden:

- Claiming full CLI, TUI, Web, report/export, query planner, browser route,
  accessibility, broader provenance, or full `WP-001` closure.
- Changing ranking, filtering, scoring, data loading, JSON, CSV, TUI, Web HTML,
  report-body, query grammar, or non-leaders front-matter contracts.

## Validation Levels

| Level | Required Check | Status |
|---|---|---|
| L0 | Markdown export helper test proves selected warning, empty-state kind/title/detail, and recovery route appear in YAML front matter from the ViewModel. | passed |
| L1 | Formatting plus affected CLI clippy checks or recorded blocker. | passed_with_risk |
| L2 | CLI subprocess Markdown export front matter is checked against the CLI JSON envelope empty/warning metadata. | passed |

## Evidence Log

| Evidence | Result |
|---|---|
| Inventory | CLI JSON envelope and Markdown report body carried selected leaders empty/warning/recovery metadata, but Markdown export front matter only exposed context/source/result metadata for the goalie-filter empty result. |
| L0 | `cargo test -p icelines-cli l0_export_leaders_front_matter_reports_empty_warning_state --bin icelines` passed. |
| L2 | `cargo test -p icelines-cli l2_cmd_export_md_leaders_front_matter_empty_warning_matches_query_envelope --test system_tests` passed. |
| L1 | `cargo fmt --check` passed after formatting. Affected CLI bin and system-test clippy passed. Full workspace clippy remains blocked by unrelated existing lint debt in `icelines-fetch`. |

## Gate

Gate is `passed_with_risk` for this pulse only. Markdown leaders export now
surfaces selected empty/warning detail and `/goalies` recovery guidance in YAML
front matter from the existing ViewModel state while preserving existing report
body, context, result, table, JSON, CSV, TUI, Web, and CLI text contracts. Full
`WP-001`, full CLI/TUI/Web/report parity, broader source provenance, full query
planner parity, and WP-008 integration rehearsal remain open.
