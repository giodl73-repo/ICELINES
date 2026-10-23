# Contributing

Keep ICELINES inspectable, source-aware, backward-compatible, and honest about
heuristics, projections, probabilities, and missing data.

Run checks locally before committing. Ordinary commits, pull requests and tag
pushes do not start repository-defined GitHub Actions. This hobby project keeps
hosted validation, publication and releases manual-only.

During development, choose a focused slice with
`powershell -File scripts/test-slice.ps1 list`. For broad Rust changes, run:

```powershell
$env:RUST_MIN_STACK = '16777216'
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Before a native release, also run
`powershell -ExecutionPolicy Bypass -File scripts/release-smoke.ps1` locally.
For browser changes, use `node scripts/build-browser.mjs --check` with the pinned
browser toolchain and dependencies installed.

The optional hosted `CI` workflow is one Linux job, capped at 45 minutes, via
`gh workflow run ci.yml`. It runs formatting, Clippy and workspace tests without
packaging, coverage, cross-platform matrices or FERRIS builds. It is not the
routine pre-commit check. Browser preview/publication and rollback remain manual
through `browser-pages.yml` and `browser-rollback.yml`.

To publish a native release, push the reviewed version tag, then explicitly run
`gh workflow run release.yml --ref vX.Y.Z` on that tag. Selecting a branch does
not build or publish a release. This explicit release still builds the three
supported native platforms.

Data workflows are manual and capped at 15 minutes. `data-bundle.yml` builds one
selected season, for example `gh workflow run data-bundle.yml -f season=20252026`.
Historical fetching defaults to that one season; use `scripts/fetch_history.py`
locally for large backfills. Existing published data releases remain available.
Dependabot groups version updates monthly, with one open PR per ecosystem.

Business logic belongs in the owning library crate, not the CLI adapter. Do not
commit credentials, private fantasy-league data, local databases, snapshots, or
generated build artifacts.

## Repository boundary

IceLines contains the reusable engine, schemas, CLI and web surfaces, bundled
league data, and the smallest deterministic fixtures required to verify those
contracts. It does not serve as a notebook for a maintainer's favorite teams or
fantasy roster.

- Keep current-team forecasts, trade proposals, roster what-ifs, generated
  scenario outputs, and personal fantasy reports in an external analysis
  workspace.
- Prefer synthetic players, teams, and leagues for new tests and examples.
- A real-team fixture is acceptable only when an upstream contract or a
  regression cannot be represented faithfully with neutral data; document why
  it is required and keep it minimal.
- Do not check in named personal fantasy teams, league exports, draft plans, or
  manager-specific recommendations.
- Historical all-league source data remains in scope when it is part of the
  documented bundled-data product rather than an individual team's scenario.
