---
skill: roles-check
topic: js-context-attribution
date: 2026-10-14
roles_used: [BENCH, WIRE, KEEL, FORGE, EDGE]
p1_count: 1
verdict: APPROVED-WITH-CONDITIONS
---

Single agent applying installed lenses; approval for diagnostic retention only.
[Provenance](evidence/js-context-attribution-provenance-20261014.json).

| # | Role | Finding | Severity | Action |
|---|---|---|---|---|
| 1 | BENCH |75 packages/eight residents/two905/332 imports/20 committed queries/restart complete, with null acceptance flags. | P3 | Keep full workload and diagnostic scope. |
| 2 | BENCH | Direct selfSize is counted once per unique profile node, with all sample node IDs validated. | P3 | Avoid inclusive stack double counting. |
| 3 | BENCH | Cumulative sampled churn includes collected objects; metadata/profilers perturb the process. | P2 | No live-memory, causal saving or acceptance interpretation. |
| 4 | WIRE | Retained75e6ba97 artifact verifies unchanged; actual script hashes match9 page app modules. | P3 | Preserve artifact versus observer identity. |
| 5 | WIRE |104 sampled page sources are unavailable; script metadata still maps contexts where known. | P3 | Preserve source errors/unmapped buckets without retrying the whole workload. |
| 6 | WIRE | No new live API reads, package format, engine data or local schema. | P3 | No source/publication changes. |
| 7 | KEEL | Named Playwright isolated world is explicit Runtime context metadata. | P3 | Distinguish it from the default page world. |
| 8 | KEEL | One runtime whole-script SHA exactly matches the installed1.63.0 UtilityScript wrapper. | P3 | Separate exact match from remaining marker heuristics. |
| 9 | KEEL | Automation allocation samples remain inside original whole-browser acceptance scope. | P2 | No baseline or automation subtraction/waiver. |
| 10 | FORGE | Native/unmapped nodes inherit closest known script context only as a stack heuristic. | P2 | Do not claim exact native allocation ownership. |
| 11 | FORGE | Utility matching evaluates only a regex-constrained quoted string, not the installed module/code body. | P3 | Keep reconstruction bounded and source-hash based. |
| 12 | FORGE | Unmapped/internal23.11% and remaining marker/default buckets stay explicit. | P3 | Do not assign unknown bytes to app or automation. |
| 13 | EDGE | No collectGarbage, heap snapshot, memory pressure or changed browser flags. | P3 | Keep no-explicit-GC/pressure qualification. |
| 14 | EDGE | Full raw report is losslessly compressed, and exact recipes/summary are retained. | P3 | Verify archive SHA/Git/checkout custody. |
| 15 | EDGE |250 MiB and deferred physical Android still lack completion evidence. | P1 | Keep full validation open. |

Five roles;1 P1,3 P2,11 P3. BENCH/FORGE agree on cumulative/heuristic limitations;
BENCH/KEEL prohibit using these categories to discount acceptance. Amendments:
exactly match utility source instead of relying only on markers, keep unknown
allocations explicit, and retain null/nonacceptance scope. All implemented.
