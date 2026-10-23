# Native boxed stats adoption review

Scope: optional `icelines-core/boxed-stats`, based on MISER EXP037 at consumer
base `393304d72518453a25371c4675356ceb3bee6dee`. Six review lenses applied by
one agent; this is not six independent reviewers.

- **HART — NOTE:** Full `SeasonStats`, keys, season-type axis, borrowed views,
  indexes and eviction algorithm are preserved. Only private map values change.
- **FORGE — NOTE:** Reuses the existing safe WASM representation. No unsafe
  code, dependency, public row type, lifetime or `send-sync` change. Feature
  unification and per-row allocation cost are documented.
- **BENCH — NOTE:** Reuses the 288-row rich boundary corpus for full typed and
  serialized comparisons, payload replacement, roster removal/restoration,
  missing-identity rejection and LRU eviction. Run against both native feature
  configurations alongside existing engine tests. Validation results belong in
  the PR description; these requirements alone do not establish a pass.
- **EDGE — NOTE:** Fixtures cover GP 0/9/10, missing versus zero payloads,
  goalies/skaters, trades, two seasons and regular/playoff rows. The failed
  insert requests a fresh window, checking that rejection does not evict data.
- **PACE — WARN:** EXP037 measured native BrowserEngine allocator-requested
  bytes; it did not measure query latency, allocator metadata, RSS or native
  CLI/server workloads. Keep the feature default off and avoid speed claims.
- **KEEL — WARN:** BrowserEngine uses per-season repositories; CLI/server
  occupancy and lifetimes differ. WASM already boxes rows. This proposal does
  not close browser memory acceptance or activate any production surface.

Verdict: suitable for an opt-in draft proposal after local checks. Global
default adoption remains gated on representative native workload and query
performance evidence. No deployment or browser research follows automatically.

Local validation: Rust 1.95.0 on Windows; both native configurations pass
1,183 core unit tests, five read-contract/lifecycle tests and 18 engine tests,
plus strict all-target Clippy for core/engine. The final team-transfer test
refinement was rerun with strict Clippy in both configurations. Combined
`boxed-stats,send-sync`, default `wasm32-unknown-unknown` engine compilation,
formatting and diff checks pass. Hosted Linux and live browser testing were
not run for this proposal.
