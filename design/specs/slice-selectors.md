# SLICE prepared-row selectors

ICELINES has its own hockey query language and typed query IR. SLICE is only a
candidate for simple low-level row predicates that already look like portable
selectors.

## Boundary

- ICELINES owns player commands, stat IDs, aliases, windows, career aggregation,
  leaderboards, similarity search, ranking, percentiles, data requirements, and
  hockey-facing error messages.
- SLICE owns selector parsing, typed field catalogs, requirements, diagnostics,
  row predicate evaluation for adapter-projected rows, and fold-plan analysis for
  prepared SQLite predicates.
- SQLite schema, joins, query execution, ranking, and residual-row materializing
  remain ICELINES-owned.
- A SLICE selector should never replace ICELINES query UX. It can help when a
  prepared row already has fields such as `player.position`,
  `player.nationality`, and `stats.ppg`.

## Example

```text
player.position eq 'C' and player.nationality eq 'SWE' and stats.ppg ge 0.8
```

The runtime helper `icelines_query::select_prepared_player_rows` and the checked
test `icelines-query/tests/slice_simple_selector.rs` demonstrate that narrow
row-filter shape while keeping advanced ICELINES semantics in `icelines-query`.

The runtime helper `icelines_query::plan_prepared_player_sqlite_selector` lowers
the same prepared-row predicate shape into a SLICE `slice.fold.v1` plan. ICELINES
can attach the `players` predicate to its player table, the `stats` predicate to
its stat table, run the ICELINES-owned join, and then evaluate any residual
SLICE filter locally.

For simple ICELINES `QueryPlan` trees that contain only prepared-row-safe bio and
season-stat predicates, `prepared_player_slice_expr_for_query_plan` and
`plan_prepared_player_query_sqlite_selector` translate the ICELINES IR into the
same SLICE expression/fold path. Unsupported domain shapes, such as country
matching, team history, sliding windows, career aggregation, and league queries,
return `None` and stay in ICELINES.
