---
skill: roles-check
topic: blink-layout-types
date: 2026-10-17
roles_used: [bench, keel, forge, wire, edge]
p1_count: 0
verdict: APPROVED-WITH-CONDITIONS
---

Single-agent review using installed lenses, diagnostic interpretation only.

| Role | Finding | Severity | Recommendation |
|---|---|---|---|
| BENCH | Nine raw trace hashes match original checkpoint descriptors. | P3 | Retain lossless traces and verify exact dump GUID/time interval. |
| BENCH | Named class byte totals equal main allocated-object totals at every checkpoint. | P3 | Preserve equality assertion; do not add both views. |
| BENCH | Every nonempty class maps by GUID edge to an equal-size anonymous suballocation. | P3 | Retain all joins, not only a top-class screenshot. |
| KEEL | Historical extra-observer artifact differs from current release. | P3 | Preserve original identity and null acceptance. |
| KEEL | Main layout classes dominate part of loaded8 Blink allocations. | P3 | Prioritize layout investigation without claiming OS savings. |
| KEEL | Original250 MiB and physical Android gates remain open. | P3 | Do not mark the validation goal complete. |
| FORGE | Reanalysis changes no Rust/browser source. | P3 | Do not run unrelated builds/tests. |
| FORGE | Parser rejects duplicate local categories and ambiguous page PID mapping. | P3 | Fail rather than select a convenient allocator event. |
| FORGE | Current upstream provider documents matching named-type/ownership semantics. | P3 | Do not claim current HEAD is the installed binary source. |
| WIRE | Compressed archives round trip to original trace bytes. | P3 | Preserve compressed and expanded hashes. |
| WIRE | Runtime-relative trace filenames need explicit archive mapping. | P3 | Store original report compressed and map checkpoint labels. |
| WIRE | Analysis provides no new live/storage/deployment proof. | P3 | Keep its scope on native allocation interpretation. |
| EDGE | Class counts include objects that may await collection. | P3 | No permanent-leak or reachability claim. |
| EDGE | Application and automation execute within the mapped renderer. | P3 | Do not assign all layout allocations to application JavaScript. |
| EDGE | Fixed-layout changes could overlap long names or break print/row geometry. | P3 | Require full visibility and viewport/keyboard/print proof before adoption. |

Roles reviewed:5; P1:0; P2:0; P3:15. Verdict: APPROVED-WITH-CONDITIONS for
diagnostic interpretation. BENCH/KEEL agree class views cannot be added to OS
bytes or converted into a saving claim. Amendments applied: exact ownership joins;
historical/null/automation scope; explicit future layout visibility gates.
