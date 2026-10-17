---
skill: roles-check
topic: buffer-heap-bridge
date: 2026-10-17
roles_used: [bench, keel, forge, wire, edge]
p1_count: 0
verdict: APPROVED-WITH-CONDITIONS
---

Single-agent review, diagnostic/causal interpretation only.

| Role | Finding | Severity | Recommendation |
|---|---|---|---|
| BENCH | Diagnostic pair adds repeated native/trace observers. | P3 | Null acceptance, never replace normal four runs. |
| BENCH | Main heap sampled committed maxima differ by about6.2 MiB. | P3 | Keep one-pair/sparse-checkpoint limits. |
| BENCH | Named class totals and ownership joins match exact main allocated bytes. | P3 | Preserve assertions and raw traces. |
| KEEL | Lower heap maximum does not establish whole-browser saving. | P3 | Keep original250 MiB failed gate. |
| KEEL | Marker-mapped application renderer includes page/worker/service worker. | P3 | Count PID once, not once per target. |
| KEEL | Diagnostic tracing-service variation materially affects totals. | P3 | Do not relabel it as normal application overhead. |
| FORGE | No application or official benchmark code changed. | P3 | No runtime/build/deployment claim. |
| FORGE | Parser verifies SHA, exact GUID interval, PID and duplicate categories. | P3 | Fail on ambiguity. |
| FORGE | Free-list buckets selected as disjoint leaves, never added to pooled/used totals. | P3 | Preserve overlapping-category warning. |
| WIRE | Eighteen trace archives round trip exactly. | P3 | Preserve compressed and expanded hashes. |
| WIRE | Runtime-relative trace names have condition/label archive mapping. | P3 | Keep raw reports lossless without unresolved flat references. |
| WIRE | Four original normal runs remain raw/unchanged. | P3 | Retain source artifact and sampler evidence. |
| EDGE | OS sample/native dump timing differs. | P3 | No exact residual attribution. |
| EDGE | Collection timing and unsampled spikes remain confounders. | P3 | No universal saving/leak or impossibility claim. |
| EDGE | Optional skill runbook missing; Android deferred. | P3 | State manual review honestly and leave device gate open. |

Roles reviewed:5; P1:0; P2:0; P3:15. Verdict: APPROVED-WITH-CONDITIONS for the
review record. BENCH/KEEL agree only end-to-end original measurements can close
the resource gate. Amendments applied: observer-process confounding identified;
scope/overlap/timing limits; unavailable skill runbook explicitly distinguished.
