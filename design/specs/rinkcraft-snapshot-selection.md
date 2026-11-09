# Explicit numerical source selection — CHG-136

IF-RINKCRAFT-NUMBERS-001. Research authority: Rinkcraft decision 0016 and snapshot
selection protocol 0.1. Existing numerical/annual/assessment contracts continue.

`NumericalHistory::snapshot(pid, first, last, exact)` returns `SnapshotResult`:
optional unchanged EvidenceCard and selection metadata (selected/newest supplied/
requested season, state, note). Filter before selecting. Default latest player
record; explicitly label older fallback. Exact requests never substitute years.
`catalog_view(season)` returns one catalog and sorted supplied seasons; default
newest supplied catalog. Missing exact source is an error. Existing comparison
checks reject absent/incompatible players. `parse_source` accepts bounded legacy
single catalog or archive and parses original text, avoiding a JSON Value roundtrip.

Optional profile `numerical_snapshot` is absent for nonregular/no-source views.
Native prefers HISTORY over NUMBERS and respects requested first/last and exact
`season`. Numbers JSON/comparison/page accept `season=YYYYZZZZ`; omitted means
newest supplied source. HTML has a source-season selector and pinned compare links.
WASM methods `numerical_catalog` and `numerical_snapshot` are stateless; matching
worker operations carry their results. Browser primary records use the active
prepared season as ceiling; source selection has no network/provider side effects.
Browser comparisons have their own explicit source selector, preserve unavailable
selected players and guard asynchronous stale results. Card actions carry year.

Full-window measured summaries are independent of older primary fallback. No
tracking forward fill, class inference, new source, pooled rank or data mutation.
Legacy single-catalog methods/assets remain compatible. Actual all-player parity,
missing/exact/bounds/context and offline/no-JS/mobile acceptance evidence is in
`docs/rinkcraft-snapshot-validation-2026-11-08.md`.
