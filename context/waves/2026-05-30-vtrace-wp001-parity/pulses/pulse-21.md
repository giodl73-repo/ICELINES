# Pulse 21: Leaders Web HTML Empty/Warning Metadata

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
| Parent requirements | `REQ-WB-002`; `REQ-DATA-001`; `REQ-PARITY-001`; `REQ-WEB-001`; `REQ-WEB-002`; `REQ-CODE-001` |
| Interfaces | `IF-DATA-001`; `IF-VIEW-001`; `IF-WEB-001` |
| Design / code rigor | `DES-001`; `DES-002`; `DES-003`; `DES-007`; `DES-009`; `DES-014`; `CR-003`; `CR-006`; `CR-007`; `CR-010`; `CR-011`; `CR-014`; `CR-015`; `CR-032` |
| Validation / evidence | `VAL-003`; `VAL-004`; `EVID-WP001-L0`; `EVID-WP001-L1`; `EVID-WP001-HTML-EMPTY-WARNING-L2`; `EVID-CR-003`; `EVID-CR-006`; `EVID-CR-018`; `EVID-CODE-001` |

## Objective

Carry selected leaders empty-state and warning metadata into Web HTML so browser
markup exposes the same `no_rows` / `unsupported_filter` state already carried by
the CLI JSON envelope and Web JSON meta for the goalie-filter empty result.

## Affected Surfaces

| Surface | Expected Change |
|---|---|
| Web leaders HTML | Adds `data-empty-kind`, `data-warning-count`, `data-warning-kinds`, and per-warning `data-warning-kind` attributes while preserving visible recovery text and `/goalies` link. |
| Web leaders JSON | No contract change; existing `meta.empty_state` and `meta.warnings` fields remain the JSON parity source. |
| CLI query leaders JSON envelope | No contract change; existing opt-in envelope `meta.empty_state` and `meta.warnings` remain the CLI parity source. |
| Tests | Focused Web L0 render assertions plus L2 CLI JSON envelope / Web HTML parity fixture for selected empty/warning metadata. |
| VTRACE docs | Record this as a browser HTML affected-slice improvement without claiming full browser route/accessibility, broader provenance, full TUI, report/export, or `WP-001` closure. |

## Allowed / Forbidden Scope

Allowed:

- Additive Web HTML `data-*` empty/warning metadata on the existing leaders meta
  line, empty-state section, and warning item.
- Focused route/render parity checks for selected leaders empty/warning metadata.
- Documentation/evidence updates tied to this pulse.

Forbidden:

- Claiming full browser route/accessibility proof, broader provenance/context
  coverage, full query planner parity, full TUI validation, full report/export
  validation, or full `WP-001` closure.
- Changing ranking, filtering, scoring, data loading, Web JSON, CLI JSON, CSV,
  text, TUI, Markdown export contracts, or the query grammar.

## Validation Levels

| Level | Required Check | Status |
|---|---|---|
| L0 | Web leaders render tests prove selected empty/warning metadata is in HTML attributes. | passed |
| L1 | Formatting plus affected Web/CLI clippy checks or recorded blocker. | passed_with_risk |
| L2 | CLI subprocess JSON envelope and Web HTML route fixture prove selected empty/warning metadata parity. | passed |

## Evidence Log

| Evidence | Result |
|---|---|
| Inventory | CLI JSON envelope and Web JSON meta carried selected leaders empty/warning metadata, and Web HTML rendered visible recovery, but Web HTML did not expose the empty/warning kinds as machine-readable metadata. |
| L0 | `cargo test -p icelines-web l0_web_leaders_view_round_trips_template_and_json_rows --lib` and `cargo test -p icelines-web l0_web_leaders_template_renders_empty_warning_recovery --lib` passed. |
| L2 | `cargo test -p icelines-cli l2_query_leaders_cli_json_and_web_html_empty_warning_metadata_match --test goalies_web_cli_parity` passed. |
| L1 | `cargo fmt --check`; `cargo clippy -p icelines-web --lib --no-deps -- -D warnings`; `cargo clippy -p icelines-cli --test goalies_web_cli_parity --no-deps -- -D warnings` passed. Full workspace clippy remains blocked by unrelated existing lint debt in `icelines-fetch`. |

## Gate

Gate is `passed_with_risk` for this pulse only. Web leaders HTML now exposes
selected empty/warning metadata in machine-readable attributes while preserving
existing visible layout, route behavior, recovery content, and JSON/CLI contracts.
Full `WP-001`, full browser/accessibility route proof, broader source
provenance, full query planner parity, full TUI validation, full report/export
validation, and WP-008 integration rehearsal remain open.
