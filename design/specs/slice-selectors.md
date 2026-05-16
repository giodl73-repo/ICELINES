# SLICE selector examples

ICELINES has its own hockey query language and typed query IR. SLICE is only a
candidate for simple low-level row predicates that already look like portable
selectors.

## Boundary

- ICELINES owns player commands, stat IDs, aliases, windows, career aggregation,
  leaderboards, similarity search, ranking, percentiles, data requirements, and
  hockey-facing error messages.
- SLICE owns selector parsing, typed field catalogs, requirements, diagnostics,
  and row predicate evaluation for adapter-projected rows.
- A SLICE selector should never replace ICELINES query UX. It can help when a
  prepared row already has fields such as `player.position`,
  `player.nationality`, and `stats.ppg`.

## Example

```text
player.position eq 'C' and player.nationality eq 'SWE' and stats.ppg ge 0.8
```

The checked test `icelines-query/tests/slice_simple_selector.rs` demonstrates
that narrow row-filter shape while keeping advanced ICELINES semantics in
`icelines-query`.
