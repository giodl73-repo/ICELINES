# Phase Campbell - platform contracts and ViewModels

**Date**: 2026-05-09
**Status**: Draft - planned after Jennings, before Messier
**Trophy**: Clarence S. Campbell Bowl. Fit: conference architecture between the data engine and the presentation surfaces. Campbell creates the common ice every later surface plays on.
**Spec**: `design/specs/platform-contracts.md`
**Estimated**: 2-4 sub-phases

---

## Why

IceLines has a good data spine and a good query ambition, but uniformity still
depends too much on discipline at each renderer. The same feature can become a
TUI state shape, a CLI formatter, a web handler payload, and a JSON route with
slightly different assumptions.

Campbell introduces the missing middle layer:

```text
StatsRepository + Query IR -> typed ViewModel -> CLI/TUI/web renderers
```

This keeps hockey logic, data provenance, filters, source state, and row
identity in one place while still letting each surface have its own layout.

---

## Role review gates

| Role | Gate |
|---|---|
| HART | Every ViewModel context carries `(season, season_type)` and cache keys include both. |
| KEEL | Each ViewModel names the CLI/TUI/web/static-export surfaces expected to render it. |
| TAPE | `LoadOutcome.missing` and source freshness survive into ViewModel warnings/source state. |
| FORGE | ViewModels are typed structs with no stringly-typed domain state where an enum fits. |
| PACE | Numeric fields carry units/precision policy; no renderer invents rounding. |
| BENCH | ViewModel fixtures/goldens prove CLI/TUI/web can render the same source shape. |
| EDGE | Empty, partial, stale, bad-filter, duplicate-name, and no-data states are modeled explicitly. |
| WIRE | JSON payloads can serialize ViewModels or documented DTO projections without losing contract fields. |
| SCOUT | ViewModels preserve hockey semantics: line, pair, goalie role, game state, series state. |
| GLASS | ViewModels expose visual/status tokens, but not renderer-specific colors/classes. |
| broadcast | Web HTML and HTMX fragments keep active context, applied state, accessibility labels, and bookmarkable URLs. |
| CREST | ViewModel tokens give later visual design enough semantic material to feel intentional without renderer-specific styling. |

---

## Platform contracts

Campbell owns the first implemented version of:

- Data context contract
- Query/filter intent contract
- ViewModel contract
- Surface parity contract
- Visual language contract
- Report generation contract

Source of truth: `design/specs/platform-contracts.md`.

---

## Sub-phase ordering

```text
Campbell.1  Contract spec and current-surface inventory
Campbell.2  Core ViewModel crate/module skeleton
Campbell.3  First three ViewModels: Leaders, TeamDepth, Goalies
Campbell.4  Contract fixtures and renderer adapters
Campbell.5  Docs handoff to Messier/Lester/Ted/Prince/Jim
```

---

## Campbell.1 - Contract spec and inventory

Create or update:

- `design/specs/platform-contracts.md`
- `design/specs/viewmodels.md`
- `design/specs/surface-parity.md`

Acceptance:

- Every forward roadmap plan names the platform contracts it consumes.
- The matrix records which features still bypass ViewModels.
- Specs index links the Campbell contract docs.

Closeout:

- Added `design/specs/platform-contracts.md`.
- Added `design/specs/viewmodels.md`.
- Added `design/specs/surface-parity.md`.
- Linked all three from `design/specs/INDEX.md`.
- Surface matrix is intentionally conservative: older web/portfolio claims are
  marked `verify` or `partial` until Ted Lindsay validates them against the
  running router and tests.
- Role review hardened the specs around static/export surfaces, source
  provenance, cache identity, metric precision, recovery actions, and renderer
  parity tests. Review note:
  `design/notes/2026-05-09-campbell-specs-roles-review.md`.

---

## Campbell.2 - Core ViewModel skeleton

Add a small shared module, likely in `icelines-core` unless dependency direction
requires a new crate:

```text
icelines-core/src/view_model/mod.rs
icelines-core/src/view_model/context.rs
icelines-core/src/view_model/tokens.rs
icelines-core/src/view_model/leaders.rs
icelines-core/src/view_model/team_depth.rs
icelines-core/src/view_model/goalies.rs
```

Initial shared types:

- `ViewContext`
- `ViewWindow`
- `SourceState`
- `SourceProvenance`
- `Completeness`
- `AppliedFilter`
- `SortState`
- `MetricCell`
- `SemanticToken`
- `EmptyState`
- `ViewWarning`
- `RecoveryAction`
- `ReportContext`

Acceptance:

- Types compile with no renderer dependency.
- No HTML class, ratatui style, or comfy-table width leaks into core ViewModel
  structs.
- Extension point is clear for product ViewModels such as Selke's
  `PoachBoardView`, `WatchRulesView`, and `PoachReportView`.
- Report ViewModels can carry generated-at, source state, sections, warnings,
  and reproducible fixture metadata without depending on markdown/HTML output.

Closeout:

- Added `icelines-core/src/view_model/` with `context`, `tokens`, `leaders`,
  `team_depth`, and `goalies` modules.
- Exported shared ViewModel types from `icelines-core`.
- Added serializable contracts for `ViewWindow`, `ViewContext`,
  `SourceState`, `SourceProvenance`, `MetricCell`, `EmptyState`,
  `ViewWarning`, `RecoveryAction`, first ViewModel shells, and
  `ReportContext`.
- Added core tests that assert source context, metric precision/tokens, and
  report context survive JSON projection.
- Verified with `cargo check -p icelines-core` and
  `cargo test -p icelines-core view_model -- --nocapture`.

---

## Campbell.3 - First ViewModels

Build the first three because later plans depend on them most:

- `LeadersView`
- `TeamDepthView`
- `GoaliesView`

Acceptance:

- Existing CLI/TUI/web code can be adapted incrementally.
- ViewModels include context, filters, sort, rows, empty state, warnings, and
  semantic tokens.
- Row identity uses stable player/team IDs, not display names.
- Metric/stat cells include unit and precision policy.
- Depth rows distinguish actual, estimated, and unknown deployment evidence.
- Source state maps missing, partial, stale, and unavailable repository inputs
  into typed context/warnings.

Closeout:

- Added first repository-backed builders:
  - `LeadersView::skater_pace`
  - `GoaliesView::from_repository`
  - `TeamDepthView::from_repository`
- Builders use `StatsRepository`/`PlayerView` and preserve `(season,
  season_type)` via `ViewContext`.
- Leader and goalie rows carry stable `PlayerId`, team, metrics, precision
  policy, and semantic tokens.
- Team depth rows carry estimated deployment evidence and preserve line/pair
  slot identity.
- Added fixture-backed tests for the three first builders.
- Remaining Campbell.3 work: adapt CLI/TUI/web renderers to consume these
  ViewModels instead of their existing local formatter shapes.

---

## Campbell.4 - Fixtures and adapters

Add test fixtures that build each ViewModel from a canonical repository fixture.

Acceptance:

- One fixture can be rendered by at least two surfaces without recomputing
  hockey logic.
- JSON/snapshot tests assert context and source state are present.
- JSON/snapshot tests assert schema version, warnings, empty state, and
  data/source generation survive projection where available.
- Renderers are allowed to format/truncate, but tests fail if they recompute
  classification/filtering differently.

Closeout so far:

- `query goalies` now builds a `GoaliesView` after its existing
  load/filter/sort/top pipeline, then projects the ViewModel back to the
  stable CLI JSON/CSV/table row shape.
- The TUI goalies screen now builds a `GoaliesView` from its active season,
  season type, sort cycle, and min-GP filter before rendering the leaderboard
  rows.
- Web `/goalies` and `/api/v1/goalies` now build a `GoaliesView` before
  projecting the existing HTML and JSON DTO rows.
- CLI `team <ABBR>` now builds `TeamDepthView` and renders the terminal depth
  chart from that ViewModel while keeping the legacy `DepthChart` renderer
  available for existing callers.
- Markdown `export md team` now builds `TeamDepthView` from its filtered roster
  and emits a ViewModel-backed estimated-lineup section before the existing
  all-skaters table.
- Markdown `export md leaders` now renders its default Pts/82 table from
  `LeadersView`; custom `--columns` remains on the stat-catalog path.
- `query leaders` text tables now build a `LeadersView` after the existing
  load/filter/sort/top pipeline; JSON/CSV remain on the stable legacy output
  contracts.
- Web `/api/v1/leaders` now projects its existing stable envelope rows from a
  `LeadersView`; HTML `/leaders` remains on the current template row path.
- `LeadersView::from_player_views_with_primary` supports sort-specific primary
  metrics without leaking renderer formatting into the default pace builder.
- `GoaliesView::from_player_views` lets renderer adapters consume already
  prepared player rows without duplicating goalie metric semantics.
- Verified the adapter with `cargo test -p icelines-cli --test system_tests
  query_goalies -- --nocapture`, `cargo test -p icelines-cli
  tui::screens::goalies -- --nocapture`, `cargo check -p icelines-cli`,
  `cargo check -p icelines-web`, `cargo test -p icelines-web goalies --
  --nocapture`, `cargo test -p icelines-cli team -- --nocapture`,
  `cargo test -p icelines-cli l0_export_team_card_lists_only_target_team --
  --nocapture`, `cargo test -p icelines-cli l0_export_leaders --
  --nocapture`, `cargo test -p icelines-cli l2_cmd_query_leaders_exits_zero
  -- --nocapture`, `cargo test -p icelines-cli
  l2_cmd_query_leaders_percentiles_flag -- --nocapture`, `cargo test -p
  icelines-cli l2_cmd_query_leaders_json_export -- --nocapture`, `cargo check
  -p icelines-web`, `cargo test -p icelines-web --test persona_wave19 --
  --nocapture`, `cargo test -p icelines-web --test persona_wave22b_envelope
  -- --nocapture`, `cargo test -p icelines-web --test persona_wave21_parity
  -- --nocapture`, and the `viewmodel` test slice.

---

## Campbell.5 - Handoff

Update:

- Selke plan: fantasy poacher renders `PoachBoardView` and report ViewModels.
- Messier plan: TUI filters mutate shared intent/ViewModels.
- Lester Patrick plan: CLI commands render ViewModels.
- Ted Lindsay plan: web routes render ViewModels and JSON envelope projections.
- Prince of Wales plan: visual tokens come from ViewModels.
- Jim Gregory plan: release gates include contract fixture checks.

Acceptance:

- Campbell can close before every surface is migrated, but the migration path
  is explicit and tracked.
- Messier starts with ViewModel contracts available.

---

## Out of scope

- Full migration of every surface.
- Visual redesign.
- New hockey analytics.
- Public API stabilization beyond the local JSON envelope seed.
