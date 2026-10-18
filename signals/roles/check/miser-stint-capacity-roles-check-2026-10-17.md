---
skill: roles-check
topic: miser-stint-capacity
date: 2026-10-17
roles_used: [HART, FORGE, BENCH, PACE, KEEL, EDGE]
p1_count: 0
verdict: APPROVED-WITH-CONDITIONS
---

# miser-stint-capacity review

One agent applied the named repo-local lenses; no independent reviewer claim.
Artifact: shared builder optimization, regression test, actual native/WASM/browser
measurements, functional validation and trace records. Selection and findings follow.
Measured consumer: c4232db05168c8f6c14853e84de2ac39ddb52953. The final production
portion of season_stats.rs matches that point exactly (SHA-256
b02da454b9163244148fe04da213f1b8e9b6fa4fe8e9688cdd44cc85bc70fdeb).
Later changes correct a test fixture's totals and add documentation; focused test rerun passes.

## HART: model invariants

| ID | Finding | Severity | Section | Recommendation / disposition |
|---|---|---|---|---|
| H1 | Compaction occurs before repository insertion and borrowed views | P3 | try_build | Keep ownership/public Vec type; no aliases or key changes. |
| H2 | Nonempty validation and canonical sorting precede shrinking | P3 | builder / existing tests | Preserve missing-stint errors, date/tie ordering and goalie rules. |
| H3 | Initial single-stint regression totals did not reflect its own stint | P2 resolved | finalized_stints regression | Compute fixture totals from stints; focused final test passes. |

## FORGE: Rust and allocation costs

| ID | Finding | Severity | Section | Recommendation / disposition |
|---|---|---|---|---|
| F1 | Safe std Vec operation adds no new I/O, dependency or unsafe boundary | P3 | season_stats.rs | Retain crate boundaries and existing Send/Sync contracts. |
| F2 | Live bytes fall while realloc calls and cumulative bytes rise | P3 | pulse / raw counters | Report allocation traffic; do not claim universal speedup. |
| F3 | Measured revision precedes final test/docs edits | P2 resolved | runtime prefix SHA | Verified identical production portion; final focused test rerun passes. |

## BENCH: validation completeness

| ID | Finding | Severity | Section | Recommendation / disposition |
|---|---|---|---|---|
| B1 | Parity needs richer real data than basic totals | P3 | 44 loader tests / 270 fresh cases | Fresh native/WASM full-row checks and complete exports match. |
| B2 | Concurrent Windows compiler attempts failed internally | P3 | retained logs | Record both failures; successful serialized full suite is the local verification command. |
| B3 | Browser behavior must survive smaller storage | P3 | 163 units / offline / viewport | All-row export, scrolling, AX/print, keyboard, detail and retry pass. |

## PACE: measurement and claims

| ID | Finding | Severity | Section | Recommendation / disposition |
|---|---|---|---|---|
| P1 | Native requested bytes, WASM capacity and browser process counters are different resources | P2 resolved | pulse tables | Report separately; no baseline subtraction or physical-memory equivalence. |
| P2 | Two browser pairs show lower candidate peaks but are a small sample | P3 | whole browser | State 7.7031 / 3.6367 MiB observations; no statistical generalization. |
| P3 | Non-consumer dependency graph could confound native comparison | P3 | locked sidecar receipt | All 100 non-consumer records match; runtime control source revisions also match. |

## KEEL: cross-surface coherence

| ID | Finding | Severity | Section | Recommendation / disposition |
|---|---|---|---|---|
| K1 | Actual shared builder replaces the lab-only candidate path | P3 | core / data / WASM | One production finalization rule serves native and browser loaders. |
| K2 | Exports/revisions/source values must remain stable | P3 | 75-package semantics / 270 cases | Canonical exports and complete query results match; no schema version change. |
| K3 | Capacity optimization must not weaken window limits or previous-good safety | P3 | actual engine / existing tests | Eight windows, stage-before-trim, refresh/retry and resource rejection pass. |

## EDGE: edge cases and acceptance limits

| ID | Finding | Severity | Section | Recommendation / disposition |
|---|---|---|---|---|
| E1 | Empty builder, trades, goalie null rates and order are sensitive | P3 | existing core/data/loader tests | Existing cases pass; finalization keeps the same typed values. |
| E2 | Resource win must not be created by altered measurement conditions | P2 resolved | four browser traces | Same 75 packages, archive 905/332 rows, 20 queries, restart, flags and sampler; all phases sampled. |
| E3 | Desktop observations do not complete phone or 250 MiB acceptance | P3 | resource table | All four budget comparisons fail; Android remains deferred. |

## Synthesis

Six roles; 18 findings. Open P1: 0; open P2: 0; resolved P2: 4; P3 notes: 14.
Verdict: APPROVED-WITH-CONDITIONS for review of this scoped performance change.
The code and measured engine gains support a scoped PR. Native and WASM savings are established; the broader browser budget and external release gates remain separate.

Three amendments applied:
1. Strengthened semantic/serialization checks and corrected the final regression fixture.
2. Recorded source/lock provenance and separate native, WASM and whole-browser metrics.
3. Preserved every run and compiler failure; kept CI, phone and overall resource gates open.

Validation: selected native core/data/WASM suites pass 1,237 tests/doc-tests;
44 native-loader integration tests pass; fresh native/distributed-WASM parity passes
270 cases (33,613 full rows, 90 empty results); 163 browser units, artifact checks,
offline reopening and viewport/export/keyboard/retry pass. Final focused fixture test
passes; affected production library clippy denies warnings. Three native pairs,
five WASM pairs and two browser pairs are retained. No runtime launch/sampler flags,
workload, forced GC or budget changes. The initial parallel compiler failures are
retained; serialized compilation passes without asserting a compiler root cause.

Hosted CI is not claimed passed. The 250 MiB whole-browser gate fails all four
browser runs. Physical Android stays deferred. Approval covers a reviewable local
optimization, not completed full-resource or production-origin acceptance.
