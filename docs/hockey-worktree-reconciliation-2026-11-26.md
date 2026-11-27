# Hockey worktree reconciliation — 2026-11-26

Two older F: checkouts held uncommitted reusable IceLines changes. Their exact
tracked work was preserved on pushed recovery branches before integration.
Generated data, local source captures and research outputs were not committed.

| Checkout | Preservation commit | Disposition |
| --- | --- | --- |
| `F:/src/ICELINES` | `a49410e1`, `recovery/native-leader-tools-20261126` | Port native leaderboard tools onto current `master`. |
| `F:/src/ICELINES-worktrees/rinkcraft-sheet` | `1bdf851`, `recovery/rinkcraft-sheet-20261126` | Eligibility work is already present in newer upstream implementations; retain the snapshot without reverting them. |

## Rinkcraft sheet audit

The old sheet's pending work implements a common 10 GP feat minimum, saved-profile
reassessment, explicit GP eligibility labels, updated preview projection and
tests. Current `master` already has these behaviors. In particular,
`icelines-core/tests/rinkcraft_stat_thresholds.rs` is byte-equivalent after newline
normalization to the recovered working file. Current `refresh_profiles` also
selects task repertoires, which the old file predates. The current preview example
adds admitted observations and retained EDGE recovery on top of the saved-profile
refresh. Replacing these current files with their old versions would regress the
profile work merged in PRs #133, #135 and #137.

The existing Rust threshold tests cover 9/10 GP boundaries, missing versus zero,
goalie eligibility and agreement between saved-profile refresh and fresh evaluation.
The browser/native renderers already show `Needs 10 GP (observed/10)`.

## Recovered native leaderboard behavior

- Optional numeric bio bounds accept blank HTML form submissions. Calendar
  birth-year bounds are distinct from the existing January-31 season age.
- Explicit even-strength points mode changes counted points and points-based
  rankings; all-situation goals and assists retain their labels. EV goals and
  assists are additional observations. EV points per 60 remains unavailable
  without matching strength exposure.
- Inclusive rank ranges apply after filtering and sorting, with at most 500 rows
  per request. Shared table display, JSON and CSV preserve the rank offset.
- Global clean styling and neutral HTML/plain-text table copying remain local
  display tools. Clipboard writes require a button click.

The port retains the current shared query shell, multi-column sort, row selection,
optional bio columns, franchise and roster context, exports and applied-context
caption. It does not restore the retired parallel leaderboard table.

Recovered season-only controls are rejected on date-window requests instead of
being silently ignored. Date-window reports continue through their existing
pipeline. GitHub Actions remain manual-only; this reconciliation does not publish
or build a hosted release.

## Validation notes

The nine recovery regressions cover blank form inputs, calendar/age intersections,
dashboard cohorts, even-strength arithmetic, context captions, rank ranges, shared
sorting/CSV and rejection of unsupported date-window combinations. Ten static
asset tests and three JavaScript preference/clipboard tests also pass.

The first broader parallel compile exceeded the Windows paging-file limit
(`os error 1455`). The one-job retry of
`cargo test -p icelines-web -p icelines-query --locked -j 1` passes **1,292 tests**,
with two existing documentation examples ignored. The three JavaScript tests
also pass. Strict all-target Clippy for the changed crates passes with
`cargo clippy --no-deps -p icelines-web -p icelines-query --all-targets --locked -j 1 -- -D warnings`.
Strict dependency-inclusive
Clippy reports 30 findings in unchanged `icelines-core` files, including argument
counts, type complexity and unnecessary unwraps. Those source files have no diff
in this integration; that existing lint debt is not silently suppressed or
described as a passing strict workspace check.

## Local artifacts

The recovery commits preserve source changes and Git history. Ignored preview
sites, source captures and research artifacts still exist in the old directories;
they require separate preservation before any checkout is deleted. A clean Git
status alone does not establish that those directories are disposable.
