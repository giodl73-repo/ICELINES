---
skill: roles-check
topic: wasm-catalog-experiment
date: 2026-10-13
roles_used: [bench, forge, keel, wire, edge]
p1_count: 2
verdict: NEEDS-WORK
---

# Catalog context and cache interaction experiment review

Scope: archived runtime prototypes 7e069abe and 47e7ab29, tests, actual-browser
fault injection and original whole-browser measurements. One agent applied five
installed role lenses; this is not independent multi-agent review. The prototypes
are not proposed for merge and do not change the stable merged PR #89 runtime.

| Role / ID | Finding | Severity | Evidence / disposition |
|---|---|---|---|
| BENCH B1 | The 250 MiB limit remains exceeded | P1 open | All full-workload prototype/control peaks exceed target; no waiver |
| BENCH B2 | Removing a redundant decode does not establish lower total memory | P2 open for promotion | Standalone/combined comparisons show no useful saving; neither adopted |
| BENCH B3 | Context binding changes WASM bytes and must not be described as a fixed-binary comparison | P3 verified | d91f75da candidate versus 4f99584f Windows control identities recorded |
| FORGE F1 | Fresh-context validation must precede repository replacement | P3 verified | Native mismatch cases preserve full query output and resident order |
| FORGE F2 | Cache hits must validate before selecting an existing resident | P3 verified | Native active/resident cases and actual-WASM resident rejection preserve order |
| FORGE F3 | Owned staging bytes must be transferred without consuming backup/export bytes | P3 verified | Nested-payload transfer regression; no original-buffer transfer |
| KEEL K1 | Domain normalization, filtering and sorting remain in the shared Rust engine | P3 verified | Adapter-only validation changes; no business projection moved to JS |
| KEEL K2 | Generic imports retain revision and data parity | P3 verified | Actual WASM verifies catalog load and generic import on all 75 packages |
| KEEL K3 | Prototype results must be distinct from merged/public identities | P3 verified | PR89 merged e0741daa; public identity remains 0e34ce88; prototypes separate |
| WIRE W1 | A context rejection must remain recoverable and preserve the previous dataset | P3 verified | Typed engine_error; actual browser retains 933 rows/one window and runs fresh 835-row query/detail |
| WIRE W2 | Metadata acquisition does not permit changed paths, size or checksum rules | P3 verified | Existing bounded read, SHA and exact package-path checks retained |
| WIRE W3 | UI/worker/bindings must stay in one verified shell | P3 verified | Actual immutable-artifact offline checks pass; no prototype deployed |
| EDGE E1 | Malformed or unknown expected fields must reject without state mutation | P3 verified | Native invalid JSON, missing/extra fields and source/type/season cases |
| EDGE E2 | Page-only cached shortcut rejection lacks an actual-browser negative case | P2 open for promotion | Fresh browser fault and native cache cases are not a substitute for this separate path; add before any future promotion |
| EDGE E3 | Android acceptance is deferred by the user | P1 open | No mobile claim from desktop tests |

Five lenses, 15 findings: two P1 release gates open, two P2 conditions remain
unsatisfied for promotion, eleven P3 findings verified. Verdict: **NEEDS-WORK**;
the runtime experiments remain unadopted. BENCH and KEEL agree that functional
correctness and removing apparent redundancy cannot substitute for evidence of
lower peak allocations. The paired memory results, rather than code neatness,
determine the decision to retain the stable implementation.
