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
