---
skill: roles-check
topic: verified-shell-body
date: 2026-10-14
roles_used: [BENCH, WIRE, KEEL, FORGE, EDGE]
p1_count: 1
verdict: NEEDS-WORK
---

# Verified offline-shell body experiment

Single-agent review using installed role lenses; no independent reviewer claim.
Artifact: service-worker.template.js, offline.test.mjs and verified-shell-body-*
evidence. This branch is local and unadopted. Published source remains5ef015c4.

Selected BENCH for regression/measurement validity, WIRE for decoded transport
and integrity, KEEL for offline ownership/update boundaries, FORGE for engine
and allocation scope, EDGE for visible offline and table behavior.

| Role | Finding | Severity | Evidence / action |
|---|---|---|---|
| BENCH | Whole-browser benefit is absent in the measured pair. | P1 | Candidate396.36 versus control389.02 MiB; block adoption as an optimization. |
| BENCH | Normal workload and observer are retained. | P3 | Both completed75 packages, eight residents, two imports,20 queries/restart; no missing sampled phases or forced GC. Keep original runner. |
| BENCH | Failed observations remain distinguishable from passing results. | P3 | Retain stale-dist unit failure and incorrect HTTP-gzip harness report;161 rebuilt unit tests pass. |
| WIRE | SHA and byte-count verification precedes caching decoded bytes. | P3 | Same manifest checks; existing corrupt/missing asset and cache-repair tests pass. Preserve fail-closed behavior. |
| WIRE | Headers must describe the decoded cached body. | P3 | New test checks MIME/etag and removed encoding/replaced length; real HTTP-gzip shell test passes. |
| WIRE | Canonical gzip data is a separate boundary. | P3 | Initial outer-gzip test changed wire size and triggered advertised-byte refusal. Corrected helper leaves canonical packages unchanged. No downloader change. |
| KEEL | Shell and data ownership stay separate. | P3 | Dataset routes still uncached; saved data remains IndexedDB-owned. No new tier or fallback. |
| KEEL | Synthetic response loses original response URL. | P3 | Browser worker/module loading and process-close/offline-reopen pass at the existing subpath. No broader browser claim. |
| KEEL | Update/repair consent boundary remains intact. | P3 | Existing full-shell repair and consent/refusal tests pass; public same-profile recovery not performed for this rejected experiment. |
| FORGE | Rust engine and frontend modules are byte-identical to control. | P3 | Recorder asserts WASM and every src/*.js SHA; no ownership, repository or native change. |
| FORGE | Fewer conceptual copies do not prove lower process memory. | P3 | Response construction/cache storage may still copy; retain measured negative result. |
| FORGE | No additional dependencies or compiler-profile changes. | P3 | Source diff limited to service-worker template and regression test. Keep release unchanged. |
| EDGE | Player readability and complete results remain verified. | P3 |933 names/ranks,≤44 mounted, keyboard boundaries, last detail,835 filtered and77 goalies pass. |
| EDGE | Offline use survives actual process close/reopen. | P3 | HTTP-gzip shell plus network-blocked reopened saved data/fresh query/detail pass. |
| EDGE | Physical mobile and250 MiB acceptance remain open. | P3 | No phone attached; both target flags false. Do not imply readiness from narrow390px checks. |

Roles reviewed:5. P1 blockers:1; P2 issues:0; P3 notes:14.
Verdict: NEEDS-WORK for adoption as a memory optimization.
BENCH/FORGE consensus: the measured benefit must govern adoption, rather than
the apparent reduction in copies. Startup279.21/278.58 MiB also offers no
observed improvement in this pair. Blank baselines235.52/231.70 differ; this is
not a causal estimate of an exact7.34 MiB regression.

Amendments:
1. Keep the implementation off master/publication; retain this branch and evidence.
2. Preserve original workload, byte limits and failed harness report when testing another hypothesis.
3. Investigate cold-start allocation ownership before another change; keep250 MiB failed and Android deferred.
