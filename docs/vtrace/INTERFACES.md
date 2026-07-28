# Interfaces

## Scope

Repo or feature: `icelines` repo-baseline VTRACE adoption.

This file freezes the interfaces that the CONOPS depends on, especially the
data-state vocabulary called out by R2.

## Interface Inventory

| ID | Interface | Type | Owner | Consumers | Compatibility Rule | Verification |
|---|---|---|---|---|---|---|
| IF-DATA-001 | Source/completeness state vocabulary | schema / ViewModel | HART / Campbell | CLI, TUI, Web, JSON API, exports | Typed state is authoritative; renderer text may vary but semantics may not. | REQ-DATA-001; VER-REQ-DATA-001 |
| IF-VIEW-001 | ViewModel envelope | schema / internal API | Campbell | Surface adapters and report/export writers | Additive fields allowed; row identity and context fields remain stable. | REQ-PARITY-001 |
| IF-SIGNAL-001 | IceLines signal metric descriptor and evidence API | internal API | PACE / HART | Core signal consumers, future cache/report/surface builders | Signals expose descriptors, required inputs, evidence tiers, units, polarity, methodology, and limitations; missing evidence returns unavailable instead of numeric zero. | REQ-SIGNAL-001 |
| IF-QUERY-001 | Art Ross filter/query intent | CLI / TUI / API grammar | Art Ross | CLI, TUI cmdbar, web params, AI fallback | User-facing grammar changes require explicit version/review notes. | REQ-QUERY-001 |
| IF-WEB-001 | Dashboard routes and URL state | API / route / URL | Ted Lindsay | Browser users, bookmarks, automation | GET routes are read-only; user-controlled state is URL-visible and allowlisted. Selected WP-003 evidence now covers POST-only season-type mutation, cache-read-only `/favorites` stat-line rendering, missing-cache `/player/:id/streaks`, `/team/:abbrev/streaks`, and `/api/v1/team/:abbrev/streaks` rendering, selected scoring/outlook/tonight-intel GET/JSON rendering, selected Admin data-status GET/JSON rendering without creating local data state, selected no-JS/viewport/recovery shell route evidence, and serve URL-before-open / `--no-open` / LAN bind warning evidence. Selected WP-006 evidence now covers fantasy JSON missing-DB and missing-cache reads without creating missing local SQLite or data-cache state, existing-FantasyDb and poach imported-availability reads without creating SQLite WAL/SHM sidecar state, and dashboard fantasy import / roster-shape mutation deferrals instead of GET-backed mutation. | REQ-WEB-001; REQ-WEB-002 |
| IF-LAYOUT-001 | Named workbench layout state | config / state schema | Jack Adams / GLASS | TUI, Web, local config, bookmarks | Durable layouts are versioned records; renderer-local state may not be the semantic source of truth. | REQ-WB-003 |
| IF-REPORT-001 | Report/export envelope | file / schema | Jim Gregory / SCOUT | Markdown, JSON, CSV, public posts | Reports are ViewModel-backed and include disclosure near the top; selected Markdown exports now emit explicit public-copy methodology-limit and completeness/skeleton-state sections before report content and preserve explicit ViewModel season windows in front matter. | REQ-REPORT-001 |
| IF-FETCH-001 | Snapshot/manifest external data boundary | file / API / cache | WIRE / Foster | DataStore, StatsRepository, fetch commands | Integrity, schema version, and missing-source states fail or degrade explicitly. | REQ-FRESH-001 |
| IF-BUILD-001 | Cargo feature/dependency boundary | build interface | FORGE / KEEL | Maintainers, release scripts | Standalone/lean claims require no cross-repo deps and a passing lean build. | REQ-DEP-001; REQ-LEAN-001 |

## Interface Details

### IF-SIGNAL-001: IceLines signal metric descriptor and evidence API

Purpose: let future hockey-facing surfaces use descriptive composite metrics
without hiding scorer bias, missing inputs, tiny-TOI problems, or sample-size
limits.

Current outputs:

- Stable signal IDs for Physical Engagement Rate, Puck Management Differential,
  and Penalty Drag Rate.
- Descriptors with label, short label, CLI-safe key, unit, polarity, required
  inputs, methodology, and limitations.
- `SignalEvidenceTier` plus missing-input details before a value is rendered.
- `None` for unavailable values; missing realtime, missing/tiny TOI, and
  below-threshold sample size are not numeric zeroes.
- `PlayerSignalsView` rows that preserve player identity, active window, signal
  value, evidence tier, missing inputs, methodology, limitations, disclosures,
  and non-claim copy for future renderers.

Compatibility rule: future consumers may add signals or descriptor fields
additively, but any surface promotion must preserve evidence coverage and
non-claim limitations from `design/specs/icelines-signals.md`.

### IF-DATA-001: Source/completeness state vocabulary

Purpose: give every renderer the same truth state so degraded data is honest and
surface parity can compare more than row values.

Inputs:

- Active `season` and `season_type`.
- Query/filter/sort/window signature when it affects the result.
- Source kind, for example bundled stats, installed manifest, snapshot,
  realtime, MoneyPuck, contracts, roster, schedule, game logs, shifts, injuries,
  fantasy import, or upstream API.
- Load outcome from the data spine, including `LoadOutcome.missing`.
- Fetch or generated timestamp when known.

Outputs:

- `complete`: source has the expected data for the requested scope.
- `partial`: source has some data, but the affected range, domain, or source kind
  must be named.
- `stale`: source exists but freshness is outside the expected window or came
  from an older snapshot/cache.
- `unavailable`: source cannot provide the requested domain in this context.
- `MissingSource`: a typed unavailable state for an absent silo or offline
  live-only source; it is not a numeric zero and not an empty success.
- Active leaders CLI JSON exposes `source_completeness` and `source_state` as
  additive row fields for the current row-array contract.
- Active leaders CLI JSON exposes `season` and `season_type` as additive row
  fields for the current row-array contract, matching the active Web JSON meta
  window.
- Active leaders CLI JSON exposes `total`, `returned`, `top`, and
  `active_filters` as additive row fields for the current row-array contract,
  matching the active Web JSON meta result state where rows exist.
- Active leaders CLI `--json-envelope` exposes a Web-compatible `schema_version`,
  `route`, `data`, and `meta` shape for result-state, `empty_state`, and
  `warnings` where the legacy row-array contract cannot carry empty-result
  metadata.
- Active Web leaders JSON exposes `meta.completeness` and `meta.source_state` in
  the existing v1 route envelope, plus `meta.total`, `meta.returned`, `meta.top`,
  `meta.active_filters`, `meta.empty_state`, and `meta.warnings` for result-state
  and recovery disclosure.
- Active Web leaders HTML renders ViewModel-provided empty-state text, warning
  text, and recovery actions for the selected goalie-filter empty result without
  recomputing the unsupported-filter meaning in the template.
- Active Web leaders HTML exposes the selected goalie-filter recovery path as a
  visible `G` position chip that links to the existing `pos=G` empty/warning
  recovery state.
- Active Web leaders HTML marks the selected position chip with
  `aria-current="true"` so active route state is exposed beyond visual styling.
- Active Web leaders HTML active position-chip label/href and single
  `aria-current="true"` marker are compared against the CLI JSON envelope
  selected `position_filter` for the same leaders route.
- Active Web leaders HTML exposes active `season` and `season_type` as additive
  `data-active-season` and `data-active-season-type` attributes on the leaders
  meta line, matching the active CLI JSON row context.
- Active Web leaders HTML exposes selected query-result metadata (`total`,
  `returned`, `top`, `sort`, and `active_filters`) as additive `data-result-*`
  attributes on the leaders meta line, matching the active CLI JSON row and Web
  JSON meta result state where rows exist.
- Active Web leaders HTML route-level `data-result-active-filters` state is
  compared against the CLI JSON envelope selected `active_filters` for the same
  leaders route.
- Active Web leaders HTML route-level visible active-filter token, preserved
  filter input, and clear-link behavior are compared against the CLI JSON
  envelope selected `active_filters` for the same leaders route.
- Active Web leaders HTML exposes selected source-state metadata (`kind` and
  `completeness`) as additive `data-source-*` attributes on the leaders meta line,
  matching the active CLI JSON row and Web JSON meta source state where rows
  exist.
- Active Web leaders HTML exposes selected empty-state and warning metadata
  (`empty_kind`, warning count, warning kinds, and per-warning kind) as additive
  `data-empty-*` and `data-warning-*` attributes, matching the CLI JSON envelope
  and Web JSON meta empty/warning state for the selected goalie-filter empty
  result.
- Active Markdown leaders exports expose selected active `season`, `season_type`,
  and `sources` front-matter metadata, matching the visible report context and
  the source state carried by `LeadersView.context`.
- Active Markdown leaders exports expose selected query-result metadata (`total`,
  `returned`, `top`, `sort`, and `active_filters`) in the report body after the
  visible context/source-state section.
- Active Markdown leaders exports expose selected empty-state/warning detail and
  `/goalies` recovery guidance in the report body for the goalie-filter empty
  result.
- Active Markdown leaders exports expose selected empty-state/warning detail and
  `/goalies` recovery guidance in front-matter `state` metadata for the
  goalie-filter empty result.
- Active default CLI leaders text output exposes selected active season/type and
  `roster complete` source-state context before the leaders table. It also
  exposes selected query-result metadata (`total`, `returned`, `top`, `sort`,
  and `active_filters`) for filtered text fixtures, plus selected
  empty-state/warning detail and `/goalies` recovery guidance for the
  goalie-filter empty result.
- Active TUI Stats leaders output exposes selected query-result metadata
  (`total`, `returned`, `top`, `sort`, and `active_filters`) in the results panel
  while preserving the existing active context and `roster complete` source-state
  presentation. It also exposes selected empty-state/warning detail and
  `/goalies` recovery guidance for the goalie-filter empty result.
- Active CSV leaders output appends selected stable row identity, active
  season/type, and `roster complete` source-state metadata while preserving the
  existing leading metric columns and row order. It also appends selected
  query-result metadata (`total`, `returned`, `top`, `sort`, and
  `active_filters`) for filtered CSV fixtures.
- `bundled`: provenance for data embedded in the binary.
- `installed bundle`: provenance for operator-installed local season artifacts.
- `snapshot`: provenance for cached/fetched serialized data.
- `cache`: provenance for session/local cache reuse.
- `live fetch write path`: provenance for opt-in fresh data written to local
  state.

Errors:

- Schema drift is a hard deserialization failure.
- Integrity-hash mismatch is a hard read failure.
- A snapshot or bundle schema from a newer binary is refused with an upgrade
  message.
- Locked shift-level tracking returns an explicit refusal, not a silent no-op.

Versioning or compatibility:

- `source_state` semantics are additive-only for `v1` envelopes.
- Renderers may translate state into badges, labels, glyphs, aria labels, or text,
  but may not recompute completeness locally.
- Color must never be the only carrier of source state.

Evidence:

- `design/specs/platform-contracts.md` Contract 1.
- `design/specs/viewmodels.md` `ViewContext` and `SourceState`.
- `VALIDATION.md` VAL-002, VAL-005, VAL-008.

### IF-VIEW-001: ViewModel envelope

Purpose: make CLI, TUI, Web, JSON API, and report/export renderers compare the
same hockey result.

Inputs: typed domain ViewModels such as `LeadersView`, `PlayerCardView`,
`ScoresView`, `FantasyRosterGapView`, `ReportView`, and related projections.

Outputs: `context`, `applied_filters`, `sort`, stable row IDs, structured rows or
sections, `empty_state`, `warnings`, and semantic tokens. For active leaders JSON
surfaces, stable player identity is exposed as additive `nhl_id`, with
`team_abbrev` mirroring the canonical team abbreviation. Leaders CLI/Web JSON
also preserve typed source/completeness state and active season/type context for
the bundled roster fixture, plus selected result-state and empty/warning
metadata: CLI keeps the existing top-level row array for `--json`, adds row
fields where rows exist, and uses `--json-envelope` for empty-result metadata,
while Web places the same state/context/result/empty/warning state in route
`meta`. For active Web HTML leaders rows, additive `data-nhl-id`,
`data-team-abbrev`, and visible metric attributes expose the same row identity
for parity fixtures without changing the visible layout. Active Web HTML also
renders selected empty-state and warning recovery metadata from the ViewModel,
including the `/goalies` recovery link for the leaders goalie-filter empty
result.
- Active Web HTML also exposes selected active-context metadata as additive
  `data-active-season` and `data-active-season-type` attributes on the leaders
  meta line for parity fixtures without changing the visible layout.
- Active Web HTML also exposes selected result-state and query-intent metadata as
  additive `data-result-total`, `data-result-returned`, `data-result-top`,
  `data-result-sort`, and `data-result-active-filters` attributes on the leaders
  meta line for parity fixtures without changing the visible layout.
- Active Web HTML also exposes selected source-state metadata as additive
  `data-source-kind` and `data-source-completeness` attributes on the leaders
  meta line for parity fixtures without changing the visible layout.
- Active Web HTML also exposes selected empty-state and warning metadata as
  additive `data-empty-kind`, `data-warning-count`, `data-warning-kinds`, and
  per-warning `data-warning-kind` attributes for parity fixtures without changing
  the visible layout.
- Active Web HTML also exposes the selected goalie-filter recovery path as a
  visible `G` position chip while preserving existing route behavior and
  ViewModel-backed empty/warning rendering.
- Active Web HTML also exposes the selected position-chip route state with
  `aria-current="true"` while preserving existing query links and chip ordering.
- Active Web HTML active position-chip state also has route-level parity evidence
  against the CLI JSON envelope selected `position_filter`.
- Active Web HTML active query-filter state also has route-level parity evidence
  against the CLI JSON envelope selected `active_filters`.
- Active default CLI leaders text output renders selected active context and
  source-state disclosure from `LeadersView.context` before the leaders table.
- Active default CLI leaders text output renders selected result-state and
  query-intent metadata from the leaders query execution state before the leaders
  table.
- Active TUI Stats leaders results render selected result-state and query-intent
  metadata from the leaders query execution state after the existing context line.
- Active TUI Stats leaders results render selected empty-state/warning detail and
  `/goalies` recovery guidance from `LeadersView` when the goalie-filter empty
  result is selected.
- Active Markdown leaders exports render selected result-state and query-intent
  metadata from the export execution state after the visible context/source-state
  section.
- Active Markdown leaders exports also expose selected result-state and
  query-intent metadata in front matter under `result.total`,
  `result.returned`, `result.top`, `result.sort`, and
  `result.active_filters`.
- Active CSV leaders output appends selected stable row identity, active context,
  and source-state metadata from `LeadersView.context` while preserving the
  existing leading metric columns. It also appends selected result-state and
  query-intent metadata from the leaders query execution state.
- Active player and team-player streak ViewModels expose selected
  `current_status` labels as additive row fields: `ongoing` when the current
  streak is nonzero and `inactive` when a loaded game has broken the streak. CLI
  streak output and the TUI player-streaks table render the label without
  recomputing streak meaning locally.

Errors: renderer-local recomputation of ranking, filtering, source state, or
classification is a defect.

Versioning or compatibility: JSON envelopes are `v1` and additive-only unless a
new version is explicitly introduced. CHG-002 is an additive WP-001 identity
field change; existing leaders JSON fields remain available. CHG-003 is an
additive Web HTML attribute change; existing table markup and visible columns
remain available. CHG-004 is an additive source-state disclosure change, and
CHG-005 is an additive active-context disclosure change. CHG-006 is an additive
result-state disclosure change; CHG-007 adds an opt-in CLI JSON envelope and Web
meta fields for empty/warning parity while the CLI leaders `--json` row-array
shape and Web leaders v1 envelope remain available. CHG-008 adds selected Web
HTML empty/warning recovery rendering while preserving the existing leaders route
and table markup for non-empty results. CHG-009 adds selected Web HTML
active-context attributes while preserving visible leaders layout and existing
route behavior. CHG-010 adds selected TUI leaders context/source-state
presentation while preserving the existing Stats results table behavior. CHG-011
adds selected Markdown leaders export context/source-state presentation while
preserving the existing leaders table columns. CHG-012 adds selected Markdown
leaders export front-matter context/source-state metadata while preserving the
visible report body and table columns. CHG-013 adds selected default CLI leaders
text context/source-state presentation while preserving JSON, CSV, Web, TUI, and
Markdown export contracts. CHG-014 adds selected CSV leaders identity,
active-context, and source-state metadata while preserving existing leading
metric columns and row order.
CHG-015 adds selected CSV leaders result-state and query-intent metadata while
preserving existing leading metric columns and row order. CHG-016 adds selected
default CLI leaders text result-state and query-intent metadata while preserving
existing context/source disclosure and table semantics. CHG-017 adds selected TUI
leaders result-state and query-intent metadata while preserving existing
context/source disclosure and row semantics. CHG-018 adds selected Markdown
leaders export report-body result-state and query-intent metadata while preserving
existing front matter, context/source disclosure, and table semantics. CHG-019
adds selected Markdown leaders export front-matter result-state and query-intent
metadata while preserving existing report-body disclosure and table semantics. CHG-020 adds selected Web
HTML leaders result-state and query-intent attributes while preserving existing
route behavior, visible layout, and JSON contracts. CHG-021 adds selected Web
HTML leaders source-state attributes while preserving existing route behavior,
visible layout, and JSON contracts. CHG-022 adds selected Web HTML leaders
empty-state and warning metadata attributes while preserving existing route
behavior, visible recovery layout, and JSON/CLI contracts.
CHG-023 adds selected default CLI leaders text empty-state, warning detail, and
recovery guidance while preserving existing JSON, CSV, TUI, Web, Markdown export,
and non-empty text table contracts.
CHG-024 adds selected Markdown leaders export report-body empty-state, warning
detail, and recovery guidance while preserving existing front matter, JSON, CSV,
TUI, Web, CLI text, context/result, and non-empty table contracts.
CHG-025 adds selected Markdown leaders export front-matter empty-state, warning
detail, and recovery guidance while preserving existing report body, JSON, CSV,
TUI, Web, CLI text, context/result, and non-empty table contracts.
CHG-026 adds selected TUI Stats leaders empty-state, warning detail, and
recovery guidance while preserving existing JSON, CSV, Web, CLI text, Markdown
export, context/result, and non-empty TUI table contracts.
CHG-027 adds a selected Web HTML leaders goalie position chip for the existing
goalie-filter recovery path while preserving existing JSON, CLI, TUI, Markdown
export, CSV, route, and ViewModel contracts.
CHG-028 adds selected Web HTML leaders active position-chip accessibility state
while preserving existing JSON, CLI, TUI, Markdown export, CSV, route, and
ViewModel contracts. CHG-029 adds route-level CLI JSON/Web HTML parity evidence
for that active-chip state without changing the interface contract. CHG-030 adds
route-level CLI JSON/Web HTML parity evidence for selected active query-filter
state without changing the interface contract. CHG-031 adds route-level CLI JSON/Web HTML parity evidence for selected active
query-filter UI state without changing the interface contract. CHG-032 adds
selected TUI active-filter result evidence and CHG-033 adds selected default CLI
text active-filter result evidence without changing the shared query/filter
contract. CHG-034 adds repeatable `--filter`, `--season`, and `--type` controls
to `export md leaders` so selected Markdown reports can use the same query
intent/context window as CLI query evidence. CHG-050 adds selected streak
`current_status` labels as additive ViewModel/output fields for CLI and TUI
streak surfaces.

Evidence: `design/specs/viewmodels.md`; `design/specs/surface-parity.md`;
`CHG-002`; `CHG-003`; `CHG-004`; `CHG-005`; `CHG-006`; `CHG-007`; `CHG-008`; `CHG-009`; `CHG-010`; `CHG-011`; `CHG-012`; `CHG-013`; `CHG-014`; `CHG-015`; `CHG-016`; `CHG-017`; `CHG-018`; `CHG-019`; `CHG-020`; `CHG-021`; `CHG-022`; `CHG-023`; `CHG-024`; `CHG-025`; `CHG-026`; `CHG-027`; `CHG-028`; `CHG-029`; `CHG-030`; `CHG-031`; `CHG-032`; `CHG-033`; `CHG-034`; `EVID-WP001-L0`;
`EVID-WP001-L2`; `EVID-WP001-HTML-L2`; `EVID-WP001-SOURCE-L2`;
`EVID-WP001-CONTEXT-L2`; `EVID-WP001-RESULT-L2`;
`EVID-WP001-EMPTY-WARNING-L2`; `EVID-WP001-HTML-RECOVERY-L2`;
`EVID-WP001-HTML-CONTEXT-L2`; `EVID-WP001-HTML-RESULT-L2`;
`EVID-WP001-HTML-SOURCE-L2`; `EVID-WP001-HTML-EMPTY-WARNING-L2`;
`EVID-WP001-HTML-POS-CHIP-L0`; `EVID-WP001-HTML-POS-ARIA-L0`;
`EVID-WP001-HTML-POS-ARIA-L2`; `EVID-WP001-HTML-FILTER-ACTIVE-L2`;
`EVID-WP001-HTML-FILTER-UI-L2`;
`EVID-WP001-TUI-CONTEXT-L0`;
`EVID-WP001-EXPORT-CONTEXT-L0`; `EVID-WP001-EXPORT-CONTEXT-L2`;
`EVID-WP001-EXPORT-METADATA-L0`; `EVID-WP001-EXPORT-METADATA-L2`;
`EVID-WP001-QUERY-TEXT-CONTEXT-L0`;
`EVID-WP001-QUERY-TEXT-CONTEXT-L2`;
`EVID-WP001-QUERY-CSV-METADATA-L0`;
`EVID-WP001-QUERY-CSV-METADATA-L2`;
`EVID-WP001-QUERY-CSV-RESULT-L0`;
`EVID-WP001-QUERY-CSV-RESULT-L2`;
`EVID-WP001-QUERY-TEXT-RESULT-L0`;
`EVID-WP001-QUERY-TEXT-RESULT-L2`;
`EVID-WP001-QUERY-TEXT-EMPTY-WARNING-L0`;
`EVID-WP001-QUERY-TEXT-EMPTY-WARNING-L2`;
`EVID-WP001-TUI-RESULT-L0`; `EVID-WP001-TUI-EMPTY-WARNING-L0`;
`EVID-WP001-EXPORT-RESULT-L0`;
`EVID-WP001-EXPORT-RESULT-L2`; `EVID-WP001-EXPORT-FM-RESULT-L0`;
`EVID-WP001-EXPORT-FM-RESULT-L2`;
`EVID-WP001-EXPORT-EMPTY-WARNING-L0`;
`EVID-WP001-EXPORT-EMPTY-WARNING-L2`;
`EVID-WP001-EXPORT-FM-EMPTY-WARNING-L0`;
`EVID-WP001-EXPORT-FM-EMPTY-WARNING-L2`; `CHG-050`;
`EVID-WP004-STREAK-STATUS-L0`.

### IF-QUERY-001: Art Ross filter/query intent

Purpose: preserve one query meaning across command-line, command bar, web params,
and future AI fallback.

Inputs: filter strings, typed flags, route params, and command-bar phrases.

Outputs: deterministic parser/planner output with typed errors, spans, hints, and
data-provider requirements. Default leaders CLI text output also discloses the
selected active context and source-state carried by the resulting `LeadersView`,
plus selected result-state and query-intent metadata (`total`, `returned`, `top`,
`sort`, and `active_filters`) for parity fixtures, plus selected empty-state,
warning, and recovery guidance for the goalie-filter empty result. Selected leaders CSV output
discloses the same result-state and query-intent metadata for filtered fixtures.
Selected Web HTML leaders output discloses selected active-filter query-intent
metadata through route-level `data-result-active-filters` parity evidence and
selected visible active-filter UI parity evidence.
Selected TUI Stats leaders output discloses the same result-state and query-intent
metadata plus selected empty-state, warning, and recovery guidance for focused
render fixtures. Selected Markdown leaders exports disclose the same result-state and query-intent
metadata in report body and front matter, apply selected free-form active filters
through the shared query filter evaluator, and disclose selected empty-state,
warning, and recovery guidance in report body and front matter for focused export
fixtures.

Errors: bad filters return typed parse/planner errors with recovery hints rather
than panics or empty success.

Versioning or compatibility: grammar is user-facing; breaking changes require a
new compatibility note and validation scenario update.

Evidence: `COMMANDS.md` filter grammar; CON-001; CON-004; CHG-013; CHG-015; CHG-016; CHG-017; CHG-018; CHG-019; CHG-026; CHG-030; CHG-031; CHG-032; CHG-033; CHG-034;
EVID-WP001-QUERY-TEXT-CONTEXT-L0; EVID-WP001-QUERY-TEXT-CONTEXT-L2;
EVID-WP001-QUERY-CSV-RESULT-L0; EVID-WP001-QUERY-CSV-RESULT-L2;
EVID-WP001-QUERY-TEXT-RESULT-L0; EVID-WP001-QUERY-TEXT-RESULT-L2;
EVID-WP001-TUI-RESULT-L0; EVID-WP001-TUI-EMPTY-WARNING-L0;
EVID-WP001-EXPORT-RESULT-L0;
EVID-WP001-HTML-FILTER-ACTIVE-L2; EVID-WP001-HTML-FILTER-UI-L2; EVID-WP001-QUERY-TEXT-ACTIVE-FILTER-L2;
EVID-WP001-EXPORT-RESULT-L2; EVID-WP001-EXPORT-FM-RESULT-L0;
EVID-WP001-EXPORT-FM-RESULT-L2; EVID-WP001-EXPORT-ACTIVE-FILTER-L0;
EVID-WP001-EXPORT-ACTIVE-FILTER-L2.

### IF-WEB-001: Dashboard routes and URL state

Purpose: keep browser navigation safe, discoverable, and bookmarkable.

Inputs: `/dashboard`, `/leaders`, `/scores`, `/player/:id`, `/playoffs`, JSON
twins, POST-backed mutation controls such as `/season-type/:kind`, and
allowlisted query state such as `workspace`, `left`, `right`, `experience`, and
the additive dashboard `layout` restore parameter.

Outputs: no-JS-readable HTML, active context, recovery pages, JSON twins where
available, read-only GET behavior, selected shell-level viewport/skip-link/no-JS
guidance, selected serve URL/no-open/LAN warning behavior, and selected
ViewModel-backed empty/warning recovery markup such as leaders `/goalies`
recovery for unsupported goalie filters. The leaders route also carries selected
active-context attributes, selected query-result active-filter attributes, and
selected visible active-filter UI state for parity fixtures.

Errors: unknown routes recover; overconstrained filters show empty-state recovery;
mutation through GET is rejected or routed to a POST-backed path. The selected
season-type route rejects GET with method-not-allowed and preserves active state.

Versioning or compatibility: bookmarkable query params are user-visible and
should be additive or backwards compatible.

Evidence: CON-003; VAL-003; `CHG-001`; `CHG-008`; `CHG-009`; `CHG-027`;
`CHG-028`; `CHG-030`; `CHG-031`; `CHG-037`; `CHG-047`; `CHG-048`; `WP-002`;
`WP-001` pulses 07-08 and 26-30; `WP-003` pulses 01-07.

### IF-LAYOUT-001: Named workbench layout state

Purpose: make personalized workbench layouts durable and portable without turning
TUI or Web renderer state into hidden hockey semantics.

Inputs: layout name, center workbench slug, left/right pane binding slugs,
optional experience slug, active context policy, and compatibility version.

Outputs: restorable layout records in `~/.icelines/layouts.json` for TUI and Web
where supported. The current schema is store version `1` with record version `1`;
TUI restore uses `icelines tui --layout <name>`, and Web restore uses
`/dashboard?layout=<name>`.

Errors: corrupt, unsupported, or semantically incomplete layout records must
surface a recovery path instead of silently falling back to a misleading default.

Versioning or compatibility: named layouts require an explicit schema version.
Unsupported store or record versions, corrupt JSON, unknown workbench slugs,
unsupported panes, and mismatched experiences are refused rather than silently
falling back to misleading defaults. Writes are atomic replace operations in the
existing user config/cache root and do not mutate snapshots, FantasyDb,
favorites, watch rules, reports, or cache state.

Evidence: MISSION Success Criteria; CON-001; CON-003; REQ-WB-003; VAL-010;
`CHG-001`; `WP-002`; `EVID-WP002-L0`.

### IF-REPORT-001: Report/export envelope

Purpose: make public artifacts reproducible and safe to quote.

Inputs: ViewModels or report-specific ViewModel projections plus fixed fixture
and clock for reproducibility tests.

Outputs: Markdown/JSON/CSV artifacts with report kind, generated time,
season/type, source/completeness state, filters/sort/scoring scheme, stable
sections, warnings, omissions, public-copy methodology limitations, and
machine-readable equivalents where practical.

Errors: reports must not invent facts outside the ViewModel and must not imply
predictive, betting, era-adjusted, injury, special-teams, deployment-adjusted,
line-chemistry, or linemate-analysis meaning.

Versioning or compatibility: exported JSON envelopes use additive `v1` fields.
Markdown leaders exports add a visible `## Context` section before the existing
table under CHG-011, additive `season_type`/`sources` front-matter metadata under
CHG-012, a visible `## Result` section under CHG-018, and additive
`result.total`/`result.returned`/`result.top`/`result.sort`/
`result.active_filters` front-matter metadata under CHG-019, visible
empty-state/warning recovery sections under CHG-024, additive
`state.empty_state`/`state.warnings` front-matter metadata under CHG-025, and
free-form active-filter application plus explicit `--season`/`--type` context
controls under CHG-034; under CHG-055, selected Markdown leaders exports use
the active ViewModel context for front-matter `season` when one is available.
Existing table columns remain stable.
Under CHG-049, selected Markdown exports add a visible `## Disclosure` section
immediately after front matter and before report content; existing table columns
remain stable.

Evidence: Contract 5; REQ-REPORT-001; VAL-002; CHG-011; CHG-012; CHG-018; CHG-019; CHG-024; CHG-025; CHG-034; CHG-049; CHG-055;
EVID-WP001-EXPORT-CONTEXT-L0; EVID-WP001-EXPORT-CONTEXT-L2;
EVID-WP001-EXPORT-METADATA-L0; EVID-WP001-EXPORT-METADATA-L2;
EVID-WP001-EXPORT-RESULT-L0; EVID-WP001-EXPORT-RESULT-L2;
EVID-WP001-EXPORT-FM-RESULT-L0; EVID-WP001-EXPORT-FM-RESULT-L2;
EVID-WP001-EXPORT-EMPTY-WARNING-L0; EVID-WP001-EXPORT-EMPTY-WARNING-L2;
EVID-WP001-EXPORT-FM-EMPTY-WARNING-L0; EVID-WP001-EXPORT-FM-EMPTY-WARNING-L2;
EVID-WP001-EXPORT-ACTIVE-FILTER-L0; EVID-WP001-EXPORT-ACTIVE-FILTER-L2;
EVID-WP004-DISCLOSURE-L0; EVID-WP004-SEASON-WINDOW-L0.

### IF-FETCH-001: Snapshot/manifest external data boundary

Purpose: keep optional fresh data from silently corrupting analytical results.

Inputs: public NHL API, optional MoneyPuck, ESPN transactions, local snapshots,
manifests, integrity hashes, and schema versions.

Outputs: snapshots/manifests with provenance, freshness, integrity status, and
missing-source records.

Errors: 429 honors `Retry-After`; 503 returns clear unavailable state; schema
drift fails loud; integrity mismatch is hard failure; unknown team abbreviation
maps season-aware or falls back to `LEAGUE` with warning.

Versioning or compatibility: newer snapshot schema is refused by older binaries.

Evidence: CON-008; VAL-008.

### IF-CACHE-001: Major analytics cache contract

Purpose: provide one canonical analytics evidence layer for future hockey
decision surfaces instead of letting each dashboard/report/card recompute its own
metrics, freshness, quality, or source-state meaning.

Inputs: explicit cache build/read request with season, season type, source
window, source manifest or snapshot generation, metric family, entity keys
(team/player/game/line/goalie where applicable), consumer contract version, and
requested output family.

Outputs: versioned cache records or consumer envelopes containing metric values,
canonical ordering/rank where applicable, methodology version, provenance,
freshness/staleness, source-window, quality/completeness, warnings, omissions,
invalidation keys, generated/source timestamps, and disclosure text. Envelopes
are consumable by Coach Game-Day Dashboard, Opponent Scout Report, Player
Evidence Card, Line Combination Explorer, Goalie Readiness & Workload View,
Practice Focus Report, Postgame Review Report, and agent-facing summaries.

Errors: missing source, stale source generation, partial source window,
unsupported metric, invalid entity key, cache schema mismatch, consumer contract
mismatch, or invalidation mismatch returns typed unavailable/stale/partial/refusal
state. Cache reads do not call live APIs, do not zero-fill missing hockey facts,
and do not silently drop unknown required fields.

Consumer rule: consumers may format, paginate, filter visually, and add local
navigation, but may not recompute canonical analytics, ranking, confidence,
freshness, quality/completeness, or source-state meaning. Consumers must preserve
disclosures and must not claim autonomous coaching authority, prediction
accuracy, betting value, injury certainty, line-chemistry causality, or
complete-world truth unless a later controlled requirement and validation row
authorizes that claim.

Versioning or compatibility: cache records carry `cache_schema_version`,
`producer_version`, `methodology_version`, and `consumer_contract_version`.
Breaking record or consumer-contract changes require change control and
compatibility/refusal fixtures. Additive fields are allowed only when older
consumers can ignore them without losing required provenance/freshness/disclosure
meaning.

Evidence: CON-010; REQ-CACHE-001; REQ-CACHE-002; REQ-CACHE-003;
REQ-CACHE-004; VAL-011; CHG-072; WP-009; `icelines-core::analytics_cache`
initial schema/consumer contract; `icelines-fetch::analytics_cache_store`
strict store/read/invalidation contract;
`icelines-core::view_model::analytics_cache_consumer` internal consumer
ViewModel fixture; `icelines-web::handlers::analytics_cache_report` named-cache
HTML/JSON report fixture, coach dashboard route fixture, opponent scout route
fixture, player evidence-card route fixture, line-combination explorer route
fixture, goalie readiness route fixture, practice focus route fixture, postgame
review route fixture, postgame adjustment-review route fixture, and agent
evidence summary route fixture. The first store-backed consumer-envelope and
ViewModel fixtures now feed a narrow named-cache report, active-context coach
dashboard route, active-context opponent scout route, active-context player
evidence-card route, active-context line-combination explorer route,
active-context goalie readiness route, active-context practice focus route, and
active-context postgame review route plus a second active-context postgame
adjustment-review route plus an active-context agent evidence summary route;
broader practice, postgame, and agent workflows remain pending.

### IF-BUILD-001: Cargo feature/dependency boundary

Purpose: prevent standalone and lean-build targets from being claimed before they
are true.

Inputs: `Cargo.toml`, workspace features, and release/build commands.

Outputs: default all-surface binary; target lean offline CLI binary with
`--no-default-features --features cli`.

Errors: cross-repo FLETCH/SLICE dependencies, ungated web/TUI/network crates, or
missing replacement command behavior block standalone/lean claims.

Versioning or compatibility: removing FLETCH/SLICE command surfaces requires
replacement, refusal, or rollback notes.

Evidence: CON-009; REQ-DEP-001; REQ-LEAN-001.

## IF-WINDOW-001 — profile observation and registry

Inputs: typed upstream IceLines authorities plus a registered profile descriptor.

Outputs: `organization_profile_observation.v1` with raw value/unit, method,
organization/time axes, normalized score/rank, confidence, coverage, evidence,
limitations, and fingerprints.

Errors: unknown method, duplicate observation, non-finite value, identity or
season mismatch, unsupported source schema, and insufficient cohort.

Versioning: profile key plus method version is immutable.

## IF-WINDOW-002 — Frame manifest

Inputs: `organization_window_manifest.v1` JSON/TOML using registered profiles.

Outputs: a validated canonical manifest and SHA-256 fingerprint.

Errors: cycles, duplicates, unknown profiles, mismatched methods, negative or
non-finite weights, weights outside tolerance, all-zero budgets, and family-cap
violations.

## IF-WINDOW-003 — board, history, and scenario documents

Outputs: `organization_window_board.v1`,
`organization_window_history.v1`, `organization_window_movement.v1`,
`organization_window_scenario_impact.v1`, and
`organization_window_bridge.v1`. Focused output retains the complete-board
fingerprint. History rejects incomparable contexts; scenarios retain baseline
and upstream scenario identity. A bridge seals exact source/target manifests,
complete one-to-one profile mappings, affine raw transforms, rationale, and
evidence. Rebase reruns the canonical scorer and never rewrites the source.

Errors: bridge schema or fingerprint mismatch, incomplete/duplicate mappings,
non-finite or zero-scale transforms, unknown source/target methods, cohort or
horizon mismatch, and tampered source/target boards fail closed.

Typed scenario authorities carry kind, source schema/fingerprint,
organization scope, profile methods, and rationale. First-party adapters cover
team-season trade/injury/return/goalie/form events, training camp, and line
combinations. Direct raw/evidence changes require organization-scoped
attribution; normalized cohort effects require same-profile authority; an
overall-only or otherwise unattributed change fails closed.

Versioning: bridge mappings and fingerprints are immutable. Movement fields
for source manifest, bridge, and rebased checkpoint are additive within v1.

## IF-WINDOW-004 — user surfaces

CLI/TUI/Web/API/report/card adapters accept sealed core documents. Web state is
bookmarkable by season, as-of, horizon, view, and registered Frame ID. No
surface calculates hockey values.

## Open Questions

- Storage location and migration policy for named workbench layouts.
- Exact Cargo feature names and crate boundaries for the lean CLI target.
- Minimum non-noisy completeness disclosure for historical perspective answers
  over skeleton seasons.
- Exact persisted storage path and rebuild command shape for the major analytics
  cache implementation wave; the first in-core coach-dashboard consumer fixture
  exists, but production downstream surfaces remain pending.
