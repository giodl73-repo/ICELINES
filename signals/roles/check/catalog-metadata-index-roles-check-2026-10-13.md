---
skill: roles-check
topic: catalog-metadata-index
date: 2026-10-13
roles_used: 5
p1_count: 0
verdict: APPROVED-WITH-CONDITIONS
---

# Digest-keyed catalog metadata

One agent applies BENCH, WIRE, KEEL, FORGE and EDGE lenses for measurement,
integrity, source boundaries, generation/ownership and unusual input. This scoped
implementation review does not close250 MiB or physical Android acceptance.

| Role / ID | Finding | Severity | Evidence | Recommendation |
|---|---|---|---|---|
| BENCH B1 | Opposite-order raw private reductions are2.69/5.18 MiB | P3 | Four normal workload reports | Keep modest local scope and baselines explicit |
| BENCH B2 | All75 generated headers match independent full parses;160 units/typecheck pass | P3 | Unit log and corpus test | Preserve byte-digest identity and caller-mutation check |
| BENCH B3 | Reviewed-head CI remains pending | P2 | Local candidate only | Validate exact CI bytes before merge |
| WIRE W1 | Index lookup occurs only after wire/raw size/hash authentication | P3 | loadCatalog/downloadPackage | Never trust an advertised digest without checking bytes |
| WIRE W2 | Six fresh/cached forged context cases reject while preserving previous export/query | P3 | Context report | Keep refusal before activation |
| WIRE W3 | Unknown digests keep full fatal UTF-8/JSON parsing, including malformed bodies | P3 | Fallback tests | Do not replace it with an approximate prefix parser |
| KEEL K1 | No WASM API, normalization, revision, save policy or resident-limit changes | P3 | Diff and unchanged WASM/catalog | Keep projection in browser acquisition |
| KEEL K2 | Metadata is derived from whole authenticated UTF-8 JSON during build | P3 | Build checks | Fail on size/hash/context/UTF-8 or marker mismatch |
| KEEL K3 | Final generator hardening emits identical measured frontend/engine/catalog bytes | P3 | Final artifact comparison | Distinguish measured source from later source identities |
| FORGE F1 | Returned headers are fresh shallow copies of valid primitive metadata | P3 | Caller mutation test | Keep immutable index private |
| FORGE F2 | Function replacement preserves literal dollar tokens, quotes and Unicode | P3 | Executed generated-module unit | Do not use replacement-string expansion on source data |
| FORGE F3 | Source marker occurs exactly once;27 shell assets include the generated module | P3 | Builder assertion and artifact verification | Preserve complete offline asset inventory |
| EDGE E1 | Reordered Unicode unknown headers and duplicate-key JSON behavior remain unchanged | P3 | Fallback tests | Preserve existing refusal/normalization order |
| EDGE E2 | Viewport/export/keyboard/print and offline process reopening pass | P3 | Browser/offline reports | Require CI artifact verification too |
| EDGE E3 | Corrupt gzip preserves prior context/result/raw export; retry returns905 | P3 | Refusal report | Never infer acceptance from count-only checks |

Roles:5; P1:0; P2:1; P3:14. APPROVED-WITH-CONDITIONS for this implementation.
Top finding: the small observed saving still leaves peaks395–399 MiB, above250.
Consensus: WIRE/KEEL/EDGE require authenticated identity plus unchanged full
validation; BENCH/FORGE require honest code/artifact/generation provenance.

Amendments:
1. Finish reviewed-head CI and retained CI artifact checks before merge.
2. Preserve unknown-digest parsing and context/export/refusal regressions.
3. Keep the original workload/flags/sampler and full memory/Android gates open.
