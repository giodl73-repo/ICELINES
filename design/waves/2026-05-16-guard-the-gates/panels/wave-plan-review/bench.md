# BENCH Review - Guard the Gates

## Findings

- The backlog item is stale for formatting: `cargo fmt --check` already exists in
  both CI and `scripts/test-slice.ps1 ci-fmt`.
- Cargo-audit should not become a CI-only surprise. Pulse 02 must add a local
  `ci-audit` slice before or with the CI job.
- A clean audit run is the preferred baseline. If advisories exist, Pulse 03 must
  either fix them or add a documented, time-boxed ignore policy.

- bench
