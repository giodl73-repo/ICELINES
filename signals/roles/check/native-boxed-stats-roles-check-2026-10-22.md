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

## Completed bounded native validation — 2026-10-23

Measured implementation: unchanged commit 4d5764244dd7a2239a2496961397e715cf10a50c.
[EXP038 results and raw custody](https://github.com/giodl73-repo/MISER/blob/0bf41dff256af2ea07c3e7e42fdd70d88af936f5/evidence/EXP-038.md)
close the planned native loader/career and shared-query evaluation. Twelve fresh
release processes retain complete row/query/roster parity. Both variants' rich
lifecycle tests and inline engine tests were checked again locally to preserve
complete receipts; no additional hosted matrix ran.

HART/FORGE/BENCH/EDGE findings remain valid: representation code is unchanged,
and full parity, tests, unchanged allocator forwarding and send-sync/WASM checks
are retained. PACE finds lower requested retention (27.88%/24.99%/37.27%) and
favorable paired private-byte deltas, with extra allocation calls and mixed RSS.
No repeatable >10% query regression occurs in both pairs, but 18/42 series flag
one pair; equivalent latency is unproven and the all-pairs default screen fails.
KEEL accepts the evidence for explicit native opt-in use only: it measures shared
query operations, not every surface's rendering or every possible career cohort.

Verdict: validation is complete for review of the default-off feature. Keep the
native default inline; no browser/Android acceptance, default activation,
deployment or follow-on measurement campaign is implied. The small MISER Linux
lab workflow passed separately; it is not Linux execution of this native workload.
