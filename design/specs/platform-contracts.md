# IceLines platform contracts

**Date**: 2026-05-09
**Status**: Accepted VTRACE baseline

This spec defines what "uniform" means for IceLines. A feature is not fully
platform-ready until it satisfies the data, query, ViewModel, surface, report,
and visual contracts that apply to it.

---

## Contract 1 - Data context

Every analytical result must carry enough context for the user and every
renderer to know what it means.

Required fields:

- `season`
- `season_type`
- view/query signature when filters, sorts, windows, or scopes affect the
  result
- `source_state`
- `generated_at` or equivalent snapshot/fetch timestamp when known
- source/data generation token when known and safe to expose
- completeness: `complete`, `partial`, `stale`, or `unavailable`
- provenance: bundled, installed bundle, snapshot, cache, or live fetch write
  path, represented as typed state rather than display text

Rules:

- Missing data is represented as state, not as silent zeroes.
- Partial source state must survive from `LoadOutcome.missing` into the
  rendered surface where it affects interpretation.
- Cache keys for analytical results include the active `(season, season_type)`,
  the typed query/filter/sort/window signature, and the source/data generation
  token where the repository exposes one.
- Stale and partial data must name the affected source or domain slice when
  known, for example roster, schedule, game logs, shifts, injuries, or fantasy
  import.

---

## Contract 2 - Query/filter intent

Every surface that asks for filtered or sorted hockey data must lower user
intent into one typed query/filter shape before execution.

Rules:

- CLI args, TUI cmdbar commands, web query params, and AI fallback output all
  validate through deterministic parser/planner code.
- Screen-local shortcut keys may exist, but their effect must be expressible as
  the same typed filter/sort state.
- Bad filters return typed errors with enough information for each renderer to
  show a useful message.
- Repeated filters, duplicate keys, aliases, and positional-vs-kv ordering have
  one shared behavior.

---

## Contract 3 - ViewModels

Core/query code produces typed ViewModels. Surfaces render ViewModels.

Initial ViewModel families:

- `LeadersView`
- `PlayerCardView`
- `SimilarPlayersView`
- `TeamDepthView`
- `GoaliesView`
- `ScheduleView`
- `ScoresView`
- `PlayoffsView`
- `TransactionsView`
- `FavoritesView`
- `DocsView`
- `PoachBoardView`
- `WatchRulesView`
- `PoachReportView`
- `ReportView`

Required ViewModel shape:

- `context`: season/type/source/completeness state
- `applied_filters`
- `sort`
- `rows` or structured domain sections
- `empty_state`
- `warnings`
- stable semantic status/color tokens, not renderer-specific colors
- typed numeric display policy for ranked/statistical values: unit, precision,
  and missing-value behavior

Rules:

- ViewModels contain hockey semantics, not terminal widths or HTML classes.
- Renderers may choose layout, truncation, and styling, but not recompute
  ranking, filtering, source state, or hockey classification.
- A ViewModel must be serializable enough for JSON tests or snapshot fixtures
  unless a plan records why not.

---

## Contract 4 - Surface parity

Every platform feature has a matrix row with:

- shared engine/ViewModel path
- CLI command
- TUI screen/action
- web HTML route
- web JSON route
- static site/export artifact where applicable
- status: `done`, `verify`, `partial`, `planned`, `stub`, `deferred`, or
  `n/a`
- tests present
- documented exceptions

Rules:

- "Done" requires the shared ViewModel path unless the feature is explicitly
  surface-only.
- Stubs are allowed, but must not be advertised as shipped.
- If a surface is deferred, the reason and phase owner are named.
- Web HTML pages and HTMX fragments surface active context and applied state
  without depending on hidden local storage or color-only cues.

---

## Contract 5 - Report generation

Reports are durable decision artifacts. They must be built from ViewModels or
report-specific ViewModel projections, never from a separate formatter-only data
path.

Required report fields:

- report kind
- generated_at
- season/type
- source/completeness state
- filters/sort/scoring scheme
- sections with stable IDs
- warnings and omissions
- machine-readable JSON equivalent where practical

Rules:

- Markdown, HTML, and JSON report variants share the same source ViewModel.
- Reports declare stale or partial source state near the top.
- A report must be reproducible for a fixed fixture and clock.
- Report text may explain a recommendation, but may not invent a fact not
  present in the ViewModel.

---

## Contract 6 - Visual language

Visual output has a shared semantic vocabulary.

Required token families:

- fit/classification
- game state: pre/live/final/OT/SO
- source state: complete/partial/stale/unavailable
- warning/error/info
- active filters/sort
- season/type context
- composition/aesthetic roles where useful, for example primary action,
  supporting evidence, quiet metadata, warning, and decision highlight

Rules:

- Color never carries meaning alone.
- Tokens must be renderable as text, glyph, badge, aria label, or report label;
  renderer color is only an enhancement.
- Semantic tokens should give CREST enough material to design hierarchy and
  rhythm without moving renderer-specific classes or layout into core.
- TUI/CLI/web may render tokens differently, but token names and semantics are
  shared.
- TUI glyphs require ASCII fallback.
- Web state is bookmarkable in the URL wherever state is user-controlled.

---

## Contract ownership

- Jennings: verifies the docs can name the contracts honestly.
- Campbell: creates the ViewModel layer and first contract fixtures.
- Selke: adds fantasy-poacher ViewModels, watch rules, and report surfaces.
- Messier: routes TUI filter/sort behavior through shared intent/ViewModels.
- Lester Patrick: renders CLI commands from ViewModels.
- Ted Lindsay: renders web HTML/JSON from ViewModels and owns surface matrix.
- Prince of Wales: owns visual token quality and ASPECT review.
- Jim Gregory: makes contract checks part of release discipline.
