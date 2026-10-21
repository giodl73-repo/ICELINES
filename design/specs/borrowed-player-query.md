# Borrowed player query evaluation

Change: CHG-123; interface IF-QUERY-READ-001; requirements REQ-PARITY-001,
REQ-QUERY-001. Depends on the additive core interface in CHG-122 / PR #111.

`icelines_query::PlayerRead<'a, R: SeasonStatsRead + ?Sized>` borrows the existing
PlayerIdentity, one season-stat reader, and optional PlayerContract. It has no
owned season row or clone cache. Identity references remain specific to the
caller/window; sharing by player ID alone is not introduced. Contract facts are
carried unchanged; existing Constraint variants do not evaluate contract predicates.

`Constraint::matches_read` evaluates the entire existing constraint tree over this
view: Bio, SeasonStat, SlidingWindow, CareerAggregate, CareerLeague, All, Any, Not.
The existing `matches(&PlayerView, &EvalCtx)` signature remains and delegates to
the same implementation. Parser/planner/provider interfaces, strict/no-fetch and
clock injection, current unsupported variants, short-circuiting, nullable-value
handling, goalie fallback, applicability no-ops, tolerances, current-team and
career/lockout logic retain their original semantics. Provider routing uses
identity.id, including defensive compatibility for incoherent legacy views.

Only raw data access changes in the evaluator: goalie_bios and team_stints getters,
and the actual core catalog read interface. Domain formulas are not copied into
new readers. Position and goalie status retain per-season semantics; current team
is the last stint, including the empty-stint None case. Public PlayerView fields,
repository references, stored wire formats, ownership and marker traits remain.

Before production query edits, clean producer 0a82da65028efc03f21334b4b5b3e31765431365
captured the original evaluator at the PR111 core baseline. The frozen synthetic
matrix has 813 constraints x 288 rows (234,144 boolean outcomes): all 108 stats,
six scalar comparisons and range predicates, all 16 bio fields, four window types,
four career aggregators, three league shapes with/without totals, and boolean
compositions. Fixtures include absent/zero/fractional reports, traded rows,
skaters/goalies, regular/playoff and contextual provider data/misses.

Tests compare frozen outcomes with legacy PlayerView and generic inline, separate
boxed-payload and dynamic trait readers. The separate layout owns no SeasonStats.
Additional checks cover identity-based provider IDs, distinct historical identity
references sharing one ID, contract pointer preservation and borrowed-owner
mutation rejection. Existing full query tests remain regression gates. Exact
producer/fixture/executable/golden hashes are retained in query tests/fixtures.

This enables storage-independent Constraint evaluation. It does not replace the
production repository or migrate every legacy helper/surface to sparse ownership.
MISER mutable-reader integration, actual roster/report eviction, native/WASM
exports/revisions and unchanged browser/device resource tests remain separate
adoption gates. No memory/latency gain, release, deployment or budget waiver.
