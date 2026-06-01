# WP-001 VTRACE Wave: Leaders Parity Foundation

## Scope

Wave ID: `2026-05-30-vtrace-wp001-parity`

Active work package: `WP-001 Source-state and ViewModel parity foundation`.

This wave executes a narrow leaders parity slice. It does not attempt to close
all source-state, query, TUI, report/export, fetch, or browser evidence for
`WP-001`.

## Parent IDs

| Type | IDs |
|---|---|
| Requirements | `REQ-QUERY-001`; `REQ-PARITY-001`; `REQ-DATA-001`; `REQ-WB-002`; `REQ-CODE-001` |
| Interfaces | `IF-VIEW-001`; `IF-QUERY-001`; `IF-DATA-001`; `IF-WEB-001` |
| Validation | `VAL-004`; supporting `VAL-001`; `VAL-005`; `VAL-009` |
| Evidence | `EVID-VAL-004`; `EVID-CR-003`; `EVID-CR-011`; `EVID-CR-018`; `EVID-CODE-001` |

## Wave Boundaries

Allowed scope:

- Preserve stable leader row identity from shared `LeadersView` into active JSON
  adapters.
- Preserve typed leaders source/completeness state into active JSON adapters.
- Preserve active leaders context from shared `ViewContext` into active JSON
  adapters.
- Preserve selected leaders result-state metadata into active JSON adapters.
- Preserve selected leaders empty-state and warning metadata into an opt-in CLI
  JSON envelope and the active Web JSON envelope.
- Preserve selected leaders browser HTML empty-state and warning recovery metadata
  from the ViewModel into rendered HTML.
- Preserve leaders active season/type context into machine-readable browser HTML
  attributes.
- Preserve selected leaders active season/type and source/completeness context into
  the TUI Stats results panel.
- Preserve selected leaders active season/type and source/completeness context into
  the Markdown export report.
- Preserve selected leaders active season/type and source/completeness context into
  machine-readable Markdown export front matter.
- Preserve selected leaders result-state and query-intent metadata into the
  Markdown export report body.
- Preserve selected leaders result-state and query-intent metadata into
  machine-readable Markdown export front matter.
- Preserve selected leaders result-state and query-intent metadata into
  machine-readable browser HTML attributes.
- Preserve selected leaders source/completeness metadata into machine-readable
  browser HTML attributes.
- Preserve selected leaders empty-state and warning metadata into
  machine-readable browser HTML attributes.
- Preserve selected leaders empty-state, warning detail, and recovery guidance
  into default CLI text query output.
- Preserve selected leaders empty-state, warning detail, and recovery guidance
  into the Markdown export report body.
- Preserve selected leaders empty-state, warning detail, and recovery guidance
  into machine-readable Markdown export front matter.
- Preserve selected leaders empty-state, warning detail, and recovery guidance
  into the TUI Stats results panel.
- Preserve selected leaders free-form active-filter result state into the TUI
  Stats results panel.
- Add L2 evidence that selected leaders free-form active-filter result state is
  applied and surfaced in the TUI Stats results panel.
- Add L2 evidence that selected leaders free-form active-filter result state is
  applied and surfaced in default CLI text query output.
- Preserve selected leaders free-form active-filter result state into Markdown
  export reports and front matter.
- Expose the selected leaders goalie-filter recovery path as a visible Web HTML
  position chip.
- Expose selected leaders active position-filter route state as Web HTML
  accessibility/current-state metadata on the active position chip.
- Add route-level CLI/Web HTML evidence that the selected leaders active
  position chip matches the URL/CLI envelope position filter.
- Add route-level CLI/Web HTML evidence that selected leaders active query-filter
  state matches the URL/CLI envelope active filters.
- Add route-level CLI/Web HTML evidence that selected leaders active query-filter
  UI state matches the URL/CLI envelope active filters.
- Preserve selected leaders active season/type and source/completeness context into
  default CLI text query output.
- Preserve selected leaders stable row identity, active season/type context, and
  source/completeness context into CSV query output.
- Preserve selected leaders result-state and query-intent metadata into CSV query
  output.
- Preserve selected leaders result-state and query-intent metadata into default
  CLI text query output.
- Preserve selected leaders result-state and query-intent metadata into the TUI
  Stats results panel.
- Add a CLI/Web leaders parity fixture that compares stable row IDs and visible
  semantic fields, source/completeness state, active context, result state, or
  empty/warning state.
- Update VTRACE rows to record affected-slice evidence honestly.

Forbidden scope:

- No query grammar redesign.
- No broad TUI or report/export rewrites beyond the named affected slice.
- No fetch, dependency, lean-build, fantasy, or local-state changes.
- No full `WP-001` closure unless every named parent evidence row is satisfied.

## Pulses

| Pulse | Objective | Status | Gate |
|---|---|---|---|
| `pulse-01` | Leaders stable row identity through shared ViewModel, CLI JSON, and Web JSON. | closed_with_risk | passed_with_risk |
| `pulse-02` | Leaders stable row identity exposed on Web HTML rows and compared against CLI JSON. | closed_with_risk | passed_with_risk |
| `pulse-03` | Leaders source/completeness state exposed on CLI JSON rows and Web JSON meta. | closed_with_risk | passed_with_risk |
| `pulse-04` | Leaders active season/type context exposed on CLI JSON rows and compared against Web JSON meta. | closed_with_risk | passed_with_risk |
| `pulse-05` | Leaders result-state metadata exposed on CLI JSON rows and compared against Web JSON meta. | closed_with_risk | passed_with_risk |
| `pulse-06` | Leaders empty-state and warning metadata exposed through an opt-in CLI JSON envelope and compared against Web JSON meta. | closed_with_risk | passed_with_risk |
| `pulse-07` | Leaders goalie-filter empty/warning recovery rendered in Web HTML and compared against CLI JSON envelope metadata. | closed_with_risk | passed_with_risk |
| `pulse-08` | Leaders active season/type context exposed in Web HTML and compared against CLI JSON metadata. | closed_with_risk | passed_with_risk |
| `pulse-09` | Leaders active season/type and source/completeness state exposed in TUI Stats results. | closed_with_risk | passed_with_risk |
| `pulse-10` | Leaders active season/type and source/completeness state exposed in Markdown export reports. | closed_with_risk | passed_with_risk |
| `pulse-11` | Leaders active season/type and source/completeness state exposed in Markdown export front matter. | closed_with_risk | passed_with_risk |
| `pulse-12` | Leaders active season/type and source/completeness state exposed in default CLI text query output. | closed_with_risk | passed_with_risk |
| `pulse-13` | Leaders stable row identity, active season/type, and source/completeness state exposed in CSV query output. | closed_with_risk | passed_with_risk |
| `pulse-14` | Leaders result-state and query-intent metadata exposed in CSV query output. | closed_with_risk | passed_with_risk |
| `pulse-15` | Leaders result-state and query-intent metadata exposed in default CLI text query output. | closed_with_risk | passed_with_risk |
| `pulse-16` | Leaders result-state and query-intent metadata exposed in TUI Stats results. | closed_with_risk | passed_with_risk |
| `pulse-17` | Leaders result-state and query-intent metadata exposed in Markdown export reports. | closed_with_risk | passed_with_risk |
| `pulse-18` | Leaders result-state and query-intent metadata exposed in Markdown export front matter. | closed_with_risk | passed_with_risk |
| `pulse-19` | Leaders result-state and query-intent metadata exposed in Web HTML. | closed_with_risk | passed_with_risk |
| `pulse-20` | Leaders source/completeness metadata exposed in Web HTML. | closed_with_risk | passed_with_risk |
| `pulse-21` | Leaders empty-state and warning metadata exposed in Web HTML. | closed_with_risk | passed_with_risk |
| `pulse-22` | Leaders empty-state, warning detail, and recovery guidance exposed in default CLI text output. | closed_with_risk | passed_with_risk |
| `pulse-23` | Leaders empty-state, warning detail, and recovery guidance exposed in Markdown export reports. | closed_with_risk | passed_with_risk |
| `pulse-24` | Leaders empty-state, warning detail, and recovery guidance exposed in Markdown export front matter. | closed_with_risk | passed_with_risk |
| `pulse-25` | Leaders empty-state, warning detail, and recovery guidance exposed in TUI Stats results. | closed_with_risk | passed_with_risk |
| `pulse-26` | Leaders goalie-filter recovery path exposed as a visible Web HTML position chip. | closed_with_risk | passed_with_risk |
| `pulse-27` | Leaders active position chip exposes current route state for browser/accessibility semantics. | closed_with_risk | passed_with_risk |
| `pulse-28` | Leaders active position chip route state compared against CLI JSON envelope selection. | closed_with_risk | passed_with_risk |
| `pulse-29` | Leaders active query-filter state compared against CLI JSON envelope selection. | closed_with_risk | passed_with_risk |
| `pulse-30` | Leaders visible active query-filter UI state compared against CLI JSON envelope selection. | closed_with_risk | passed_with_risk |
| `pulse-31` | Leaders TUI active query-filter result state rendered from the applied filter. | closed_with_risk | passed_with_risk |
| `pulse-32` | Leaders CLI text active query-filter result state compared against JSON envelope rows. | closed_with_risk | passed_with_risk |
| `pulse-33` | Leaders Markdown export active query-filter result state compared against query JSON envelope rows. | closed_with_risk | passed_with_risk |
| `pulse-34` | Leaders TUI active query-filter result state compared against query JSON envelope rows. | closed_with_risk | passed_with_risk |

## Validation Strategy

| Level | Evidence |
|---|---|
| L0 | Focused core/Web/CLI adapter tests proving stable IDs and JSON projection. |
| L1 | `cargo fmt --check`, affected package tests, and affected clippy where practical; workspace clippy risk remains separately recorded if unchanged. |
| L2 | CLI subprocess, hidden TUI snapshot, and Web route parity fixture for canonical leaders rows. |

## Gate Status

Current gate: `passed_with_risk` for pulses 01 through 34 only.

Wave disposition: `WP-001` is `closed_with_risk` by the 2026-05-31 package
close review. This closes the selected leaders parity foundation slice and routes
remaining broad browser, report/export, source provenance, query-planner, and
integration-rehearsal risks to successor packages.

Closure rule: only the leaders identity/source-state/active-context/result-state,
empty/warning JSON parity, selected browser HTML recovery parity, and selected
browser HTML active-context parity, selected TUI leaders context/source, selected
Markdown export report context/source, selected Markdown export front-matter
metadata, selected default CLI text query context/source, selected CSV query
identity/context/source, selected CSV query-result metadata, selected default CLI
text query-result metadata, selected TUI query-result metadata, and selected
Markdown export report query-result metadata, and selected Markdown export
front-matter query-result metadata, selected browser HTML query-result metadata,
selected browser HTML source/completeness metadata, and selected browser HTML
empty-state/warning metadata, and selected default CLI text empty-state/warning
recovery, selected Markdown export empty-state/warning recovery, and selected
Markdown export front-matter empty-state/warning recovery, and selected TUI
empty-state/warning recovery, and selected Web HTML position-chip goalie
recovery, selected Web HTML active position-chip accessibility state, and selected
Web HTML active position-chip route evidence, selected Web HTML active
query-filter route evidence, selected Web HTML active query-filter UI
route evidence, selected TUI active-filter result evidence, selected TUI
active-filter L2 snapshot evidence, selected default CLI text active-filter
result evidence, and selected Markdown export active-filter result evidence
slices may move to `passed_with_risk`.
Broader `WP-001`, `VAL-004`, `VAL-005`, `EVID-CR-003`, `EVID-CR-011`,
`EVID-CR-018`, and `EVID-CODE-001` remain open unless later evidence covers
their full scope.
