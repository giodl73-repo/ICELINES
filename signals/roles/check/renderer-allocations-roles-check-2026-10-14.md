---
skill: roles-check
topic: renderer-allocations
date: 2026-10-14
roles_used: [BENCH, KEEL, FORGE]
p1_count: 0
verdict: APPROVED-WITH-CONDITIONS
---

# Native allocation evidence review

Single-agent installed-lens review, not independent reviewers. BENCH reviews
trace/experimental fidelity, KEEL ownership boundaries and FORGE allocation
interpretation. Scope is diagnostic evidence only, not ship/acceptance approval.

| Role | Finding | Severity | Evidence / action |
|---|---|---|---|
| BENCH | All nine requested dumps present and matched. |P3|Exact GUID and begin/end interval, no numeric serialized-ID shortcut.|
| BENCH | Target timestamp PIDs correlate allocation records. |P3|All markers mapped; preserve raw traces.|
| BENCH | Full workload completes with extra observers. |P3|75/eight/two/20/restart verified; null acceptance flags.|
| BENCH | Summary failure is retained and corrected without rerun. |P3|Separate global graph caused duplicate PID events; local categories asserted disjoint.|
| BENCH | Archive compression is lossless. |P3|Every decompressed trace equals original and matches original report SHA.|
| KEEL | Global ownership graph is distinct from local allocator records. |P3|Exclude global entries from local summary rather than adding them to omnibox allocations.|
| KEEL | Hierarchical categories overlap. |P3|No sum/subtraction as a process partition.|
| KEEL | Targets sharing PID do not own separate process totals. |P3|One application renderer for page/worker/service worker.|
| KEEL | Browser UI/GPU remain inside whole-browser acceptance. |P3|No excluded allocations or launch flags.|
| KEEL | PR101 verifier repair is separate. |P3|Diagnostic executes retained public5ef artifact; no runtime change.|
| FORGE | ArrayBuffer category fluctuates. |P3|31.10/14.32/27.82/8.92 MiB is not a leak or ownership map.|
| FORGE | Blink capacity and live objects differ. |P3|33.5 capacity versus2.59 after queries does not establish removable memory.|
| FORGE | Named fragmentation/cache counter is narrow. |P3|Do not generalize0.035 MiB to all fragmentation.|
| FORGE | Temporary-buffer disposal needs ownership proof. |P3|Preserve canonical bytes and queued chunk aliases before any experiment.|
| FORGE | Completion remains unproven. |P3|250 MiB failed; physical Android explicitly deferred.|

Roles reviewed:3; P1:0; P2:0; P3:15.
Verdict: APPROVED-WITH-CONDITIONS for local diagnostic records only.
BENCH/KEEL/FORGE consensus: allocation categories narrow a hypothesis but do not
prove object ownership or a saving. Amendments incorporated: explicit global
graph exclusion, nonadditive counter interpretation and lossless/raw-hash archive.
