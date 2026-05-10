# IceLines ViewModels

**Date**: 2026-05-09
**Status**: Draft - Campbell owns first implementation

ViewModels are the typed boundary between hockey computation and presentation.
Core/query code builds them. CLI, TUI, web, markdown, and JSON render them.

```text
StatsRepository + Query/Feature Intent -> ViewModel -> Renderer
```

The goal is surface uniformity without forcing every surface to look the same.

---

## Rules

1. ViewModels carry hockey semantics, not renderer layout.
2. ViewModels include context, source state, filters, sort, warnings, and empty
   state.
3. Renderers may truncate, align, paginate, style, and choose density.
4. Renderers may not recompute ranking, classification, source state, or
   recommendation logic.
5. Row identity uses stable IDs, never display names.
6. Every ViewModel is fixture-testable with a fixed repository and clock.
7. Display labels are allowed, but canonical domain state stays typed when an
   enum or ID exists.
8. Numeric/stat cells carry unit and precision policy so renderers do not
   invent rounding or missing-value behavior.

---

## Shared types

Initial module target:

```text
icelines-core/src/view_model/mod.rs
icelines-core/src/view_model/context.rs
icelines-core/src/view_model/tokens.rs
icelines-core/src/view_model/leaders.rs
icelines-core/src/view_model/team_depth.rs
icelines-core/src/view_model/goalies.rs
```

Shared structs/enums:

```rust
pub struct ViewWindow {
    pub season: Season,
    pub season_type: SeasonType,
}

pub struct ViewContext {
    pub window: ViewWindow,
    pub generated_at: Option<DateTime<Utc>>,
    pub source_state: Vec<SourceState>,
    pub completeness: Completeness,
    pub data_generation: Option<String>,
}

pub enum Completeness {
    Complete,
    Partial,
    Stale,
    Unavailable,
}

pub struct SourceState {
    pub source: SourceKind,
    pub state: Completeness,
    pub provenance: Option<SourceProvenance>,
    pub fetched_at: Option<DateTime<Utc>>,
    pub stale_reason: Option<String>,
    pub message: Option<String>,
}

pub struct AppliedFilter {
    pub key: FilterKey,
    pub op: Option<FilterOp>,
    pub value: String,
    pub label: String,
}

pub struct SortState {
    pub key: SortKey,
    pub label: String,
    pub direction: SortDirection,
}

pub struct MetricCell {
    pub key: StatKey,
    pub label: String,
    pub value: MetricValue,
    pub unit: MetricUnit,
    pub precision: ValuePrecision,
    pub token: Option<SemanticToken>,
}

pub struct EmptyState {
    pub kind: EmptyKind,
    pub title: String,
    pub detail: Option<String>,
    pub recovery: Vec<RecoveryAction>,
}

pub struct ViewWarning {
    pub kind: WarningKind,
    pub source: Option<SourceKind>,
    pub message: String,
    pub recovery: Vec<RecoveryAction>,
}

pub enum SemanticToken {
    FitElite,
    FitSolid,
    FitBuried,
    FitStretch,
    SourceComplete,
    SourcePartial,
    SourceStale,
    SourceUnavailable,
    Rising,
    Stash,
    Stream,
    CategoryFit,
    ScheduleEdge,
    Risk,
}
```

Exact Rust names can change during implementation, but the concepts are
load-bearing.

### Identity and provenance

- View rows carry stable domain IDs plus display labels.
- Duplicate player or team names are disambiguated in the ViewModel, not left to
  one renderer.
- `SourceState` is the typed projection of repository load state, including
  `LoadOutcome.missing` where applicable.
- Cacheable builders use `(ViewWindow, query/filter/sort signature,
  data_generation)` as the logical cache key.
- JSON projections preserve `context`, `warnings`, `empty_state`, and source
  state, even when they flatten rows for API convenience.

---

## Initial ViewModels

### `LeadersView`

Purpose: one ranked list of skaters or goalies.

Required:

- `context`
- `applied_filters`
- `sort`
- `rows: Vec<LeaderRow>`
- `empty_state`
- `warnings`

`LeaderRow` includes:

- `player_id`
- display name
- team
- position
- primary stat cell
- secondary stat cells
- rank
- semantic tokens

### `TeamDepthView`

Purpose: one team depth chart plus summary.

Required:

- `context`
- `team`
- forward lines
- defense pairs
- goalies section if available
- unplaced/extra players
- warnings
- source state

Rows/slots must preserve:

- player ID
- line/pair/slot
- position
- fit/classification token
- visible stat summary
- whether the assignment came from actual deployment, estimated roster shape, or
  unknown evidence

Depth builders must distinguish last-stint/current-active roster claims from
all-stints historical depth. Surfaces may summarize either view, but the
ViewModel must not collapse them into one unnamed truth.

### `GoaliesView`

Purpose: goalie board and goalie role filtering.

Required:

- `context`
- `applied_filters`
- `sort`
- `role_filter`
- `rows`
- `warnings`
- `empty_state`

Goalie rows include:

- player ID
- team
- GP/starts if available
- save metrics
- role signal: actual, estimated, or unknown where applicable
- semantic tokens

---

## Product ViewModels

### `PoachBoardView`

Owned by Phase Selke.

Required:

- `context`
- scoring scheme/window
- query/filter intent
- `rows: Vec<PoachPlayerRow>`
- source state
- warnings
- confidence summary

Every row includes score components and explanation. A row with only a magic
number is invalid.

Current Selke surfaces:

- CLI `icelines poach`
- TUI Poach screen
- web `/poach`
- JSON `/api/v1/poach`

### `WatchRulesView`

Owned by Phase Selke.

Required:

- rules
- enabled/disabled state
- last fired state where known
- unsupported source warnings

Current Selke surfaces:

- CLI `icelines watch rules`
- CLI `icelines watch player|deployment --save`
- CLI `icelines watch enable|disable|fire|history`
- TUI Watchlist workspace rule/history summary
- JSON `/api/v1/watch-rules`

### `PoachReportView`

Owned by Phase Selke, uses the report contract from `platform-contracts.md`.

Required:

- report context
- sections with stable IDs
- source state
- warnings/omissions
- rows/structured recommendations

Current Selke surfaces:

- CLI `icelines report poach`
- CLI `icelines report weekly`
- markdown report rendering
- JSON report output via CLI `--json`
- web `/reports/poach`
- web `/reports/weekly`

---

## Renderer responsibilities

| Renderer | May do | Must not do |
|---|---|---|
| CLI | choose columns, align, truncate, emit JSON/CSV | recompute rank/filter/source state |
| TUI | paginate, style, choose panes, handle selection | mutate ViewModel rows in render path |
| Web HTML | choose semantic markup, CSS classes, HTMX fragments | invent route-specific data semantics |
| Web JSON | serialize envelope/projection with schema version | drop context/source/warnings |
| Markdown report | format sections and prose | invent facts absent from ViewModel |

---

## Test strategy

Campbell fixtures:

- build a canonical repository fixture;
- build `LeadersView`, `TeamDepthView`, and `GoaliesView`;
- render at least two surfaces from the same ViewModel fixture;
- assert context/source state survives serialization;
- assert row identity is stable by player/team IDs.
- assert metric units/precision are supplied by the ViewModel;
- assert duplicate names, empty results, partial source state, stale source
  state, and bad filters have typed warnings or empty states;
- assert renderer adapters do not recompute ranking/classification by comparing
  the same fixture across CLI/TUI/web JSON or snapshot projections.

Later phases add product fixtures, especially Selke poacher rows.
