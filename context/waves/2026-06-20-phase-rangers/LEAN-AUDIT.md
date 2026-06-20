# Rangers lean CLI audit

Date: 2026-06-20

## Decision

Phase Rangers does not claim lean or standalone CLI support. The current repo
still has dependency and feature-boundary blockers that belong in a future
dependency surgery wave.

## Current blockers

| Blocker | Current evidence | Disposition |
|---|---|---|
| FLETCH dependency seam | Root `Cargo.toml` has workspace `fletch-core` as a git dependency from `giodl73-repo/FLETCH.git`; `icelines-fetch` consumes it through `fletch-core.workspace = true`. | Target not met. Future wave must choose replacement, refusal, shim, or rollback for FLETCH-backed commands. |
| SLICE dependency seam | Root `Cargo.toml` has workspace `slice-core` as a git dependency at rev `353564781f6cad53fc5a934178a7927824824e3e`; `icelines-query` consumes it through `slice-core.workspace = true`. | Target not met. Future wave must decide whether selector support remains, moves, or gains a compatibility boundary. |
| FLETCH command surfaces | `icelines-cli/src/cli.rs` still exposes `fetch fletch-sources`, `fetch fletch-partitions`, `fetch fletch-quivers`, and `fetch fletch-cache-index`. | Do not remove silently. Each command needs replacement/refusal/shim/rollback evidence. |
| SLICE selector surface | `icelines-query/src/slice_selectors.rs` still uses `slice_core`. | Do not delete without selector compatibility evidence. |
| Lean feature boundary | No root or `icelines-cli` `cli` feature exists. | `cargo build --no-default-features --features cli` remains an invalid claim path. |

## Reproducible check

Run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\rangers-lean-audit.ps1
powershell -ExecutionPolicy Bypass -File scripts\rangers-lean-audit.ps1 -Json
```

The script intentionally verifies the blocker state instead of trying to force a
lean build. It should fail only if the blocker inventory changes and this audit
needs to be updated.

## Next gate

A future lean implementation wave must define:

- package features for `cli`, `tui`, `web`, `net`, and `reports`;
- command-by-command FLETCH replacement/refusal/shim/rollback decisions;
- SLICE selector replacement or compatibility evidence;
- a passing `cargo build --no-default-features --features cli` command;
- an offline CLI smoke test that does not rely on Web, TUI, or network crates.
