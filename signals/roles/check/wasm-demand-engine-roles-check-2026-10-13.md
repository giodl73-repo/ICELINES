---
skill: roles-check
topic: wasm-demand-engine
date: 2026-10-13
roles_used: [bench, forge, wire, keel, edge]
p1_count: 2
verdict: NEEDS-WORK
---

# Deferred engine and memory evidence review

Artifact: EngineClient/main lifecycle changes, regression tests and dated memory
and offline evidence. All 13 role Markdown files were inventoried; selected
verification lenses were read. BENCH covers measurement, FORGE ownership, WIRE
failure boundaries, KEEL shared-engine convergence and EDGE cold-start races.
This is one agent applying five lenses, not independent reviewers or full release
approval. The full-release verdict remains separate from this scoped patch.

| Role / ID | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| BENCH B1 | Full workload still exceeds the retained 250 MiB limit | P1 open | Memory evidence | Continue optimization; do not claim acceptance from idle results |
| BENCH B2 | Early startup and varying blank baselines confounded the first comparison | P2 amended | Settled startup probe | Wait for readiness/offline preparation and repeat in reversed order; inventory workers afterward |
| BENCH B3 | Stats and schedule-only offline restoration use actual distributed WASM | P3 verified | Browser reports | Keep the distinct first-operation cases; unit doubles are insufficient alone |
| FORGE F1 | Starting a cold worker must not invalidate a caller's selected context | P2 amended | startWorker | Separate construction from restart's cancellation/generation change |
| FORGE F2 | Explicit restart retains termination, pending cancellation and epoch guards | P3 verified | EngineClient tests | Keep retired responses unable to satisfy new work |
| FORGE F3 | Main-thread business calculations and Rust domain code are unchanged | P3 verified | Source scope | Keep all data projection in the shared engine |
| WIRE W1 | Worker initialization errors now occur at first use | P3 verified | Failure tests | Preserve typed failure and bounded watchdog/recovery |
| WIRE W2 | Public/live request shapes, endpoints and save consent do not change | P3 verified | Diff scope | Do not imply relay or upstream improvements |
| WIRE W3 | Payload transfers still consume only an owned staging copy | P3 verified | Existing transfer tests | Preserve original bytes for saving/export/recovery |
| KEEL K1 | Catalog and saved-library inspection do not need Rust initialization | P3 verified | start | Remove the unused readiness request; activate Rust when restoration needs it |
| KEEL K2 | Saved stats and schedule-only restoration both automatically start the engine | P3 verified | Actual offline reports | Keep schema and local-record identities unchanged |
| KEEL K3 | Candidate and control share identical Windows WASM bytes | P3 verified | Artifact identities | Distinguish them from Linux CI builds and current public deployment |
| EDGE E1 | Cold available means a request can be accepted, not initialized bindings | P3 verified | Client state | Match prior queued-request semantics and test first-load generation preservation |
| EDGE E2 | A failed cold construction must not become an automatic retry loop | P3 verified | Constructor-failure test | Require explicit restart/recovery after failure |
| EDGE E3 | Physical Android acceptance is deferred by the user | P1 open | Acceptance scope | Do not substitute desktop emulation or close the release gate |

## Synthesis

Five lenses; two P1 release issues remain open, two P2 issues amended, eleven P3
findings verified. Full-release verdict: **NEEDS-WORK**. The deferred-start patch
is approved with conditions: required CI must pass; claims remain limited to
worker lifetime and observed idle memory. BENCH and KEEL agree that artifact
identities and first-operation browser evidence must stay distinct from a full
memory-budget claim.

Amendments: preserve generation during first construction; add cold-context,
concurrent/failure/restart regression cases; report replicated settled startup
separately from the unchanged failing full-workload target.
