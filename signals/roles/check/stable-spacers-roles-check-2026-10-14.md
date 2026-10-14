---
skill: roles-check
topic: stable-spacers
date: 2026-10-14
roles_used: [bench, keel, forge, wire, edge]
p1_count: 2
verdict: NEEDS-WORK
---

Single-agent review of candidate f56c50bc and retained decision/evidence, using
the installed role lenses. The candidate remains local and unpublished.

| Role | Finding | Severity | Recommendation |
|---|---|---|---|
| BENCH | Opposite-order pairs do not establish repeatable memory savings. | P1 | Do not adopt as a memory optimization. |
| BENCH | Original implementation fails the new row insertion regression. | P3 | Retain the failing control log alongside passing candidate evidence. |
| BENCH | Full workload and original sampler are preserved in all four memory runs. | P3 | Preserve raw reports rather than selecting the best run. |
| KEEL | Both candidate peaks exceed the 250 MiB whole-browser requirement. | P1 | Keep desktop acceptance false. |
| KEEL | Frontend-only build explicitly reuses retained Windows WASM. | P3 | Require a fresh clean CI build before any future adoption. |
| KEEL | Published release remains 75e6ba97; this branch changes no deployment. | P3 | Keep release claims separate from trial results. |
| FORGE | Optional end marker preserves existing PlayerTable callers. | P3 | Keep insertion regression if revisiting this implementation. |
| FORGE | Virtual redraw retains spacers while nonvirtual/empty paths remove them. | P3 | Preserve terminal and mode transition browser checks. |
| FORGE | Original min-height guard remains despite reduced detach operations. | P3 | Do not remove the guard without independent layout/focus evidence. |
| WIRE | Corrupt gzip refusal and recovery preserve prior data/export. | P3 | Retain refusal report; do not infer new transport behavior. |
| WIRE | Context mismatch checks cover fresh/cached fixtures, not every offline path. | P3 | State their scope explicitly. |
| WIRE | Actual process close/reopen offline query/detail passed separately. | P3 | Retain separate offline evidence. |
| EDGE | Candidate mutation observer records zero spacer detachments; control records one. | P3 | Treat this as DOM behavior proof, not memory acceptance. |
| EDGE | Full viewport, keyboard, accessibility names, print transitions and 390px checks passed. | P3 | Keep original full-result assertions. |
| EDGE | Android is untested and PDF extraction is not physical printer proof. | P3 | Keep device gates open; phone testing is deferred by user. |

Roles reviewed: 5. P1 blockers: 2; P2 issues: 0; P3 notes: 13.
Verdict: NEEDS-WORK for adoption. BENCH and KEEL agree that fewer DOM operations
do not establish the requested whole-browser saving.

Amendments applied: retain all four raw measurements; record frontend-only build
provenance and observer scope; leave desktop and physical Android gates open.
