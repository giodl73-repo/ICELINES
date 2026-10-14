---
skill: roles-check
topic: assertion-overhead-diagnostic
date: 2026-10-14
roles_used: [BENCH, WIRE, KEEL, FORGE, EDGE]
p1_count: 1
verdict: APPROVED-WITH-CONDITIONS
---

Single agent applying installed lenses. Approval for diagnostic retention only.
[Evidence](evidence/assertion-overhead-provenance-20261014.json).

| # | Role | Finding | Severity | Action |
|---|---|---|---|---|
| 1 | BENCH | Both full75/eight/two905/332/20/restart runs complete all sampled phases. | P3 | Preserve original workload. |
| 2 | BENCH | Complete result count/summary and inert=false remain checked; row and resident limits still asserted. | P3 | Do not substitute mounted-row count for full results. |
| 3 | BENCH | Changed polling/check timing affects allocation and GC timing; one pair is not causal attribution. | P2 | Keep both flags null; no latency/acceptance promotion. |
| 4 | WIRE | Exact current retained75e6ba97 artifact verifies before serving; no module/data transformations. | P3 | Keep artifacts identical in both modes. |
| 5 | WIRE | Package actions, archive inputs, network observer and restart stay unchanged. | P3 | Preserve source/recovery scope. |
| 6 | WIRE | No live relay requests or local-state changes. | P3 | No deployment/source-state claim. |
| 7 | KEEL | Official memory.mjs is unchanged against origin/master. | P3 | Do not quietly replace the accepted measurement path. |
| 8 | KEEL | Additional batched helper exists only under ignored target. | P3 | Keep methodology experiment separate from product code. |
| 9 | KEEL | Whole-browser includes automation; cumulative churn is not subtracted. | P2 | No budget waiver. |
| 10 | FORGE | Batched395.06/control393.29 MiB is not evidence of saving; baseline differs. | P3 | Do not adopt this as a memory optimization. |
| 11 | FORGE | Helpers dispose returned JS handles, preserving owned-browser cleanup. | P3 | Keep observer lifetimes bounded. |
| 12 | FORGE | No new allocator, actor, engine or dependency. | P3 | No Rust/build redesign. |
| 13 | EDGE | Mounted<=44 and residents<=8 assertions remain at committed-state snapshots. | P3 | Keep viewport/residency bounds. |
| 14 | EDGE | No forced GC, profiling or memory pressure; flags/sampler unchanged. | P3 | No hidden resource intervention. |
| 15 | EDGE |250 MiB and deferred Android remain incomplete. | P1 | Keep full validation open. |

Five roles;1 P1,2 P2,12 P3. BENCH/KEEL prohibit method/latency promotion and
automation subtraction. Amendments: explicitly flag changed polling, retain both
raw results/baselines and null acceptance, and leave official runner unchanged.
All satisfied. No PR or publication.
