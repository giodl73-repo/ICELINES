# Borrowed season-stat read interface

Status: additive core interface; production repository ownership unchanged.
Change control: CHG-122. Interface: IF-CORE-READ-001. Requirement: REQ-PARITY-001.

## Contract

`icelines_core::SeasonStatsRead` describes one coherent player/season/type row.
It exposes original-width scalar metadata, `&StatTotals`, `&[TeamStint]` and
`Option<&T>` for all eight existing typed report payloads. Missing payloads and
nullable fields retain their existing meaning. Getters borrow the owner and do
not require an assembled `SeasonStats`. Storage implementors own coherent data
access, not hockey formulas. The crate-private blanket extension owns the shared
PlayerView computations and cannot be overridden by downstream implementations.

Implementations for `SeasonStats` and existing `PlayerView` use their current
owners. PlayerView's identity-based id behavior is retained; valid repository
views have matching identity/row IDs. No field, key, serialization, repository
return type, marker trait, LRU, roster, contract or source-export policy changes.
`PlayerView.stats` remains `&SeasonStats`; `StatsRepository.season`/`iter_stats`
retain their existing references. No memory or latency improvement is asserted
from introducing the interface.

The additive catalog entry points are `StatId::read_from`,
`StatId::sort_read_cmp` and `FilterExpr::matches_read`. They accept statically
known readers or `dyn SeasonStatsRead`. Existing `read`, `sort_cmp` and `matches`
remain usable. The existing read and boolean-filter entry points delegate to
one implementation of all catalog formulas, nullable-value handling, TOI floors,
trade guards, applicability/no-op rules, short-circuit semantics and tolerances.
The canonical `sort_values_cmp` still owns ordering and identity tie-breaking.

## Verification

A legacy oracle was captured before modifying production source, from clean
fixture-tool commit 9af58a5dc45e456e8c3a9df2638447901504dc2b atop production
663c4f2909905c9e1d3258e26478d36355d30f4a. The producer, executable, fixture
source and golden hashes are retained in
`icelines-core/tests/fixtures/season-read-legacy-custody.json`.
`season_read_oracle` emits observations; it never automatically overwrites the
retained golden. Source changes require review, not routine golden regeneration.

The synthetic matrix contains 288 rows spanning GP 0/9/10, absent/0/299/300 TOI,
one/two stints, skater/goalie, regular/playoff, report absence/presence/zero and
fractional values, and both historical/current season metadata. All 108 catalog
values are retained as optional exact f64 bits; every stat's complete sorted ID
sequence and eight boolean-filter outcome vectors are retained independently.
These are boundary fixtures, not measured production density or acquired NHL data.

Tests compare that pre-refactor oracle with legacy PlayerView, inline readers,
PlayerView readers and a non-contiguous implementation owning independently boxed
payloads with no SeasonStats field. Typed accessor checks cover every report and
metadata field. Explicit guards test the 300-second floor, current summary-TOI
precedence, traded on-ice exclusion/deployment exception and position applicability.
A compile-fail doctest checks that a returned borrow prevents owner mutation.

Run core unit/integration/doc tests and core all-target Clippy, downstream query/
data/WASM tests, native workspace checks and WASM target checks. Existing catalog
and query suites remain regression gates. This does not migrate the entire query
executor, production storage or browser engine to sparse ownership; connecting
those backends requires their own full native/WASM/export/revision and resource
measurements. Existing browser/device criteria are unchanged.
