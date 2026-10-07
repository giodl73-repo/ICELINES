---
skill: roles-check
topic: browser-package-resource-validation
date: 2026-10-06
roles_used: [keel, wire, bench, forge, edge]
p1_count: 0
p2_count: 2
p3_count: 13
verdict: APPROVED-WITH-CONDITIONS
---

# Package resource validation review

Code and desktop memory regression reviewed through five installed role lenses.
These are one agent's lens checks, not five independent approvals. Scope is the
reproduced oversized-text amplification fix; full release remains unaccepted.

KEEL checks boundary ownership/convergence; WIRE checks input and failure policy;
BENCH checks regression evidence; FORGE checks Rust allocation/error safety;
EDGE checks worst-case input, boundaries and retained state. Their lens.verify
sections were read; unrelated native screen/network checks do not become passed
by this browser-only change.

| # | Role | Finding | Severity | Recommendation |
|---|---|---|---|---|
| 1 | KEEL | Preflight belongs to browser package transport; shared normalization is reused. | P3 | Keep hockey calculations out of the guard. |
| 2 | KEEL | Linear-memory reduction does not prove total/staging peak under 250 MiB. | P2 | Keep the broader resource gate open and measure whole-browser staging. |
| 3 | KEEL | Native policies and persistence semantics stay separate; new custom-import limits matter. | P3 | Document the browser-specific contract and refusal behavior. |
| 4 | WIRE | Encoded token scan rejects huge raw or escaped text before owned normalization. | P3 | Retain encoded and decoded Unicode boundary tests. |
| 5 | WIRE | Rejection preserves the active DTO and resident order, and never truncates input. | P3 | Preserve atomic activation for every future resource refusal. |
| 6 | WIRE | Old saved/custom packages exceeding new limits can be refused. | P3 | Keep limits and recovery expectations visible in import documentation. |
| 7 | BENCH | All 75 catalog packages load/query through actual guarded WASM. | P3 | Retain catalog metrics with the build verification. |
| 8 | BENCH | 17 Rust tests cover resource boundaries and active-state retention. | P3 | Keep expectations tied to stated limits and identity invariants. |
| 9 | BENCH | Browser after-evidence is a disclosed dirty local build; no clean current CI artifact yet. | P2 | Verify a clean committed build and current CI before merging/publishing. |
| 10 | FORGE | JSON visitor borrows text and holds only bounded counters; no full Value tree. | P3 | Keep preflight before owned decoding and repository cloning. |
| 11 | FORGE | Bounds stop counters well before usize overflow; serde's depth/error checks remain. | P3 | Retain malformed/trailing and nested-node tests. |
| 12 | FORGE | Safe Rust and typed engine errors preserve schema/context checks after preflight. | P3 | Do not let resource checks stand in for semantic validation. |
| 13 | EDGE | 60 MiB name input formerly reached 372.625 MiB WASM; guard now rejects at 67 MiB. | P3 | Retain the before/after reproduction and prior-query comparison. |
| 14 | EDGE | Whitespace-padded archives shrink before normalization and cannot prove model memory. | P3 | Keep archive and resident-model evidence distinct. |
| 15 | EDGE | Aggregate text and nested small arrays can amplify despite per-field/array limits. | P3 | Keep aggregate/node enforcement and future model-density stress. |

Roles reviewed: 5. P1 blockers: 0; P2 issues: 2; P3 notes: 13.
Verdict: APPROVED-WITH-CONDITIONS for the scoped mitigation.
Top finding: the reproduced defect is mitigated; whole-browser resource acceptance
is still unproven. KEEL and BENCH agree on evidence scope and clean-build gates.

Amendments: document explicit input limits and compatibility refusal (README);
retain boundary/state regressions plus before/after/catalog evidence (implemented);
verify clean committed WASM/current CI and continue whole-browser/device gates
(pending). BW-02 records the failure in design/PITFALLS.md.