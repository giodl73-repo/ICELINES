---
skill: roles-check
topic: query-json-response-candidate
date: 2026-10-14
roles_used: [BENCH, WIRE, KEEL, FORGE, EDGE]
p1_count: 3
verdict: APPROVED-WITH-CONDITIONS
---

Single agent applying installed lenses; approval for local experiment, not release.
Diagnostic: [allocation sampling](evidence/js-allocation-sampling-provenance-20261014.json).
Candidate plan: CHG-122 and IF-BROWSER-001 optional response encoding.

| # | Role | Finding | Severity | Action |
|---|---|---|---|---|
| 1 | BENCH | Sampling measures cumulative churn, including collected objects; most page allocations have no app URL. | P2 | Do not claim retained leaks or removable79 MiB; internal/automation frames remain unattributed. |
| 2 | BENCH | Known code builds worker query objects before cloning them to the page. | P3 | Test the allocation hypothesis with full-workload controls, not assumed savings. |
| 3 | BENCH | 167 unit tests pass, including negotiated/legacy values and malformed/cancelled encoding. | P3 | Real worker/browser compatibility and exact artifact checks still required. |
| 4 | WIRE | Only queries may request JSON text; packages, metadata, details and schedules retain existing values. | P3 | Keep the experiment scoped to query transfer. |
| 5 | WIRE | New worker must return objects to old clients; new client must accept objects from old workers. | P1 | Exercise both mixed-version directions in real browsers before adoption. |
| 6 | WIRE | Invalid encoded values/JSON/unknown encoding fail protocol recovery. | P3 | Keep typed failure and generation/epoch rules. |
| 7 | KEEL | CHG-122 records additive negotiated schema1 capability; no local-state schema or domain changes. | P3 | Keep interface/trace status candidate until gates pass. |
| 8 | KEEL | WASM scoring, normalization, resident engine ownership and all8 windows remain unchanged. | P3 | Use exact matching engine bytes in initial comparisons. |
| 9 | KEEL | Clean CI/full artifact/public transition checks are pending. | P1 | No merge/publication based only on local units. |
| 10 | FORGE | Page parses the same WASM JSON once; worker bypasses parse only after explicit capability request. | P3 | Preserve every field, null, Unicode string and row order. |
| 11 | FORGE | Deserialization moves to the page; cost may affect responsiveness despite avoiding worker object trees. | P2 | Measure query completion and retain slow observations; Android is deferred. |
| 12 | FORGE | No Rust code, allocator, dependency or extra actor is introduced. | P3 | Keep the engine/source/data matching controls. |
| 13 | EDGE | Cancelled/retired responses must not poison the next selected context. | P3 | New regression plus existing restart/generation tests cover this. |
| 14 | EDGE | Full933-player viewport/export/keyboard/PDF/offline and corruption/context checks are pending for the candidate. | P3 | Run the actual artifact checks, including complete results beyond mounted rows. |
| 15 | EDGE |250 MiB target remains failed; phone explicitly deferred. | P1 | Keep full acceptance open; no reduced workload or baseline-subtracted pass. |

Five roles;3 P1,2 P2,10 P3. BENCH/FORGE require measured rather than inferred
benefit. Amendments: explicit request capability with legacy fallback, strict
encoding recovery/cancelled-response handling, and full artifact/interoperability
checks before adoption. First two implemented; runtime/memory/CI gates pending.

Final local disposition: not adopted. [Evidence](evidence/query-json-response-provenance-20261014.json)
contains167 passing units, all3 real-worker compatibility directions with full
payload/detail parity, corruption/context, offline,933-player viewport and24-page
PDF checks. First interop fixture used invalid goalie points sort; failed output
and corrected wins-sort result remain separate. Opposite-order raw reductions
1.078/8.613 MiB track baseline differences0.902/8.320 MiB. No convincing memory
benefit is established; no causal subtraction or budget waiver. Normal CI Rust
rebuild/public transitions are not claimed for this rejected local experiment.
The full250 MiB/Android acceptance remains open; production is unchanged.
