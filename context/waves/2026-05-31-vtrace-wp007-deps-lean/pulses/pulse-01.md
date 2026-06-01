# WP-007 Pulse 01 - Dependency and Lean-Feature Inventory

## Scope

Dependency graph, FLETCH/SLICE command-surface inventory, and lean CLI target
disposition.

## Findings

- `fletch-core` remains a path dependency through `icelines-fetch`.
- `slice-core` remains a direct git dependency through `icelines-query`.
- `slice-core` is also present at a second git rev through `fletch-core`.
- FLETCH command surfaces remain active:
  `fetch fletch-sources`, `fetch fletch-partitions`, `fetch fletch-quivers`, and
  `fetch fletch-cache-index`.
- SLICE selector surfaces remain active in
  `icelines-query/src/slice_selectors.rs`.
- The documented lean CLI command fails before compilation because the workspace
  does not expose a `cli` feature.

## Evidence

```powershell
cargo tree -i fletch-core
cargo tree -i 'git+https://github.com/giodl73-repo/SLICE?rev=353564781f6cad53fc5a934178a7927824824e3e#slice-core@0.1.0'
cargo tree -i 'git+https://github.com/giodl73-repo/SLICE?rev=50b63a2eefc66916e9a015a915c845c28d80773c#slice-core@0.1.0'
cargo build --no-default-features --features cli
C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only
git diff --check
```

Result: dependency inventory passed 2026-05-31; lean build target remains
target-not-met with this Cargo error:

```text
error: none of the selected packages contains this feature: cli
selected packages: icelines-core, icelines-query, icelines-fetch, icelines-site, icelines-web, icelines-cli
```

## Review

This pulse does not remove dependencies or promote lean build status. It records
the exact blockers and keeps standalone/lean claims target-not-met until a future
manifest/feature wave provides command replacement/refusal/shim/rollback evidence
and a passing lean build.

## Status

`target-not-met_dispositioned`; `WP-007` remains an explicit release/revisit
item and no standalone/lean claim is made.

