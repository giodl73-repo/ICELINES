# Pulse 33: Leaders Markdown Export Active Filter Parity

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
| Parent requirements | `REQ-REPORT-001`; `REQ-QUERY-001`; `REQ-PARITY-001`; `REQ-WB-002`; `REQ-CODE-001` |
| Interfaces | `IF-VIEW-001`; `IF-QUERY-001`; `IF-DATA-001`; `IF-REPORT-001` |
| Design / code rigor | `DES-003`; `DES-008`; `DES-009`; `DES-014`; `CR-003`; `CR-007`; `CR-010`; `CR-018`; `CR-032` |
| Validation / evidence | `VAL-002`; `VAL-004`; `EVID-WP001-EXPORT-ACTIVE-FILTER-L0`; `EVID-WP001-EXPORT-ACTIVE-FILTER-L2`; `EVID-WP001-L0`; `EVID-WP001-L1`; `EVID-CR-003`; `EVID-CR-007`; `EVID-CR-018`; `EVID-CODE-001` |

## Objective

Close the selected Markdown leaders export active-filter gap by letting
`export md leaders` accept the same free-form `--filter` intent and explicit
season/type window as `query leaders`, then prove `country=CAN` export output
matches the query JSON envelope rows and result metadata.

## Affected Surfaces

| Surface | Expected Change |
|---|---|
| CLI command surface | `export md leaders` now accepts repeatable `--filter`, plus explicit `--season` and `--type`, so report exports can align with query evidence windows. |
| Markdown export runtime | Export filtering reuses the query filter dispatch/evaluation helpers and keeps context/source/result/front-matter metadata intact. |
| CLI system tests | Adds L2 evidence comparing `query leaders --json-envelope` with `export md leaders --out -` for `country=CAN`. |
| VTRACE docs | Record selected Markdown export active-filter application evidence without claiming full report/export parity or full `WP-001` closure. |

## Allowed / Forbidden Scope

Allowed:

- Additive `export md leaders` arguments for active filter and explicit context.
- Reuse of existing query filter parsing/evaluation helpers.
- Focused Markdown export L0 and CLI subprocess L2 evidence.
- Documentation/evidence updates tied to this pulse.

Forbidden:

- Changing query grammar, filter semantics, ranking semantics, Web/TUI/CSV
  contracts, or unrelated report shapes.
- Claiming full report/export parity, full query-planner parity, or full
  `WP-001` closure.

## Validation Levels

| Level | Required Check | Status |
|---|---|---|
| L0 | Markdown export unit evidence applies `gp>=60`, reports the active filter, keeps matching rows, and excludes filtered-out rows. | passed |
| L1 | Formatting plus affected CLI system-test clippy checks or recorded blocker. | passed_with_risk |
| L2 | `query leaders --season 20242025 --filter country=CAN --top 5 --json-envelope` matches `export md leaders --season 20242025 --filter country=CAN --top 5 --out -` active-filter metadata and filtered rows; export excludes unfiltered non-CAN leader `Leon Draisaitl`. | passed |

## Evidence Log

| Evidence | Result |
|---|---|
| Inventory | Pulses 17/18 proved Markdown export result metadata and pulse 32 proved CLI text active-filter rows, but `export md leaders` did not accept free-form `--filter` or explicit `--season`/`--type` context, so report exports could not prove active-filter parity against query envelopes. |
| L0 | `cargo test -p icelines-cli export::tests::l0_export_leaders_free_form_filter_reports_and_filters_rows` passed. |
| L2 | `cargo test -p icelines-cli --test system_tests l2_cmd_export_md_leaders_active_filter_result_state_matches_query_envelope -- --nocapture` passed. |
| L1 | `cargo fmt --check` passed. Affected CLI system-test clippy passed. Full workspace clippy remains blocked by unrelated existing lint debt outside this pulse. |
| Docs | `C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only` passed. |

## Gate

Gate is `passed_with_risk` for this pulse only. The selected Markdown leaders
export path now has L0/L2 evidence that a free-form active filter is applied to
rows, surfaced in result metadata and front matter, and aligned with the query
JSON envelope for the same season/type window. Full `WP-001`, broader
report/export parity, broader query-planner parity, broader source provenance,
and WP-008 integration rehearsal remain open.
