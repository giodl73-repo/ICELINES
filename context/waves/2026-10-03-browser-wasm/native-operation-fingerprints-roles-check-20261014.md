---
skill: roles-check
topic: native-operation-fingerprints
date: 2026-10-14
roles_used: [BENCH, WIRE, KEEL, FORGE, EDGE]
p1_count: 1
verdict: APPROVED-WITH-CONDITIONS
---

Single agent applying installed lenses; diagnostic retention only.
[Evidence](evidence/native-operation-fingerprints-provenance-20261014.json).

| # | Role | Finding | Severity | Action |
|---|---|---|---|---|
| 1 | BENCH | Known operation results verify byte counts,5000 DOM nodes and post-profile gzip SHA. | P3 | Preserve operation scope, not full workload acceptance. |
| 2 | BENCH | Prefix categories account for all272 historical samples once per depth. | P3 | Keep depth partitions separate. |
| 3 | BENCH | Sampling intervals differ4096/32768; weights/counts are not exact memory. | P2 | No cross-run byte/OS/latency subtraction. |
| 4 | WIRE | No-op, buffers, clone, DOM and gzip use owned localhost fixtures; no live NHL or relay. | P3 | Keep no source/public data effects. |
| 5 | WIRE | Exact normalized debug UUID plus RVA avoids ASLR/mismatched-module comparisons. | P3 | Enforce UUID/image-size checks when present. |
| 6 | WIRE | Control coverage is absent; this does not prove no allocations occurred. | P3 | Preserve null mapping/zero samples explicitly. |
| 7 | KEEL | Shared typed-array/gzip signatures identify a generic call path, not specific object ownership. | P2 | Do not label unused buffers or leaks. |
| 8 | KEEL | App, engine, data, official runner and persistent schemas stay unchanged. | P3 | No implementation adoption. |
| 9 | KEEL | Only13 historical samples match at depth8;259 remain unmatched. | P3 | Preserve unresolved native attribution. |
| 10 | FORGE | Exact prefix matching uses module-relative PCs, not nearby exports. | P3 | No invented function names. |
| 11 | FORGE | Snapshot sampling may omit transient/freed allocations and unsampled blocks. | P2 | No complete allocator census. |
| 12 | FORGE | All owned sessions/browsers/server close in finally; no GC/pressure commands. | P3 | Keep lifecycle bounded. |
| 13 | EDGE | Initial overstrict control guard failure is retained; corrected run is separate. | P3 | No erased failure or production fix inference. |
| 14 | EDGE | Raw corrected report is losslessly compressed with exact recipes/summary. | P3 | Verify archival/Git/checkout custody. |
| 15 | EDGE |250 MiB and physical Android remain unproved. | P1 | Keep validation goal open. |

Five roles;1 P1,3 P2,11 P3. BENCH/KEEL/FORGE prohibit turning correlations into
object ownership, leaks or removable bytes. Amendments: preserve initial failure,
record missing control coverage and keep shared/unmatched paths explicit. All
implemented. No PR, publication, resource waiver or completion claim.
