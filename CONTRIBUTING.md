# Contributing

Keep ICELINES inspectable, source-aware, backward-compatible, and honest about
heuristics, projections, probabilities, and missing data.

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
powershell -ExecutionPolicy Bypass -File scripts/release-smoke.ps1
```

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
