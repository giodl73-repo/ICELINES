---
skill: roles-check
topic: frontend-bundle-build-candidate
date: 2026-10-14
roles_used: [BENCH, WIRE, KEEL, FORGE, EDGE]
p1_count: 3
verdict: APPROVED-WITH-CONDITIONS
---

Single agent applying installed lenses; no independent/delegated review claimed.
Artifact: ed60eedc clean local integrated candidate, draft PR105.
Evidence: [provenance](evidence/frontend-bundle-build-provenance-20261014.json).
Verdict is limited to a draft candidate; it does not approve final acceptance.

| # | Role | Finding | Severity | Action |
|---|---|---|---|---|
| 1 | BENCH | Earlier opposite-order transformed pairs are promising but have varying baselines. | P1 | Measure the clean CI artifact before adopting a memory improvement claim. |
| 2 | BENCH | 165 units include meaningful metadata ordering refusal and Unicode/relative worker URL behavior after removing the separate metadata module. | P3 | Preserve these build boundary regressions. |
| 3 | BENCH | Shared local build check passes:270 native-loader/WASM cases,163 Rust tests across nonempty groups,34 Python tests and165 browser units. CI still runs. | P3 | Preserve the CI pending gate. |
| 4 | WIRE | Bundling occurs after authenticated generated metadata injection, before actual asset sealing. | P3 | Keep this ordering enforced by the helper. |
| 5 | WIRE | Corrupted gzip preserves prior full results and byte-exact package export; retry returns905 rows. | P3 | Retain actual bundled-browser evidence. |
| 6 | WIRE | All six fresh/cached wrong season/type/source cases preserve933 prior rows, exact export and requery. | P3 | Retain unsigned fixture scope; not a complete offline/live test. |
| 7 | KEEL | CHG-121 records candidate dependency/build posture with DESIGN/TRACE updates; requirement/budget meaning is unchanged. | P3 | Promote only when named gates are proven. |
| 8 | KEEL | Worker/WASM, schemas and paths remain separate/unchanged; saved and unsaved handling passes offline process reopening. | P3 | Keep persistence semantics. |
| 9 | KEEL | No merge/publication has occurred; clean CI runtime correctness and publication identity remain unproved. | P1 | Keep PR draft until required CI and artifact checks pass. |
| 10 | FORGE | esbuild0.28.2 is exact-pinned in development dependencies and recorded in build-info; install with ignored scripts works locally. | P3 | Verify the same installation path in CI. |
| 11 | FORGE | The actual executing bundle hash equals the prototype; actual WASM bytecode matches the local artifact. | P3 | Repeat against CI Linux artifact; debugger observer is not part of normal memory runs. |
| 12 | FORGE | Separate decoder/metadata modules do not execute; individual module tests remain useful but cannot prove inlined executing identity. | P3 | Use main-bundle fingerprint rather than falsely claiming separate scripts execute. |
| 13 | EDGE | 933 players,120 forward/reverse tab steps, terminal boundaries,77 goalies,390px width and print transitions pass. | P3 | Preserve complete result parity beyond44 mounted rows. |
| 14 | EDGE | Actual24-page PDF has all933 ordered Unicode names/ranks with two parsers; inactive modules are still cached. | P3 | Retain exact PDF/reference and state no cache saving. |
| 15 | EDGE |250 MiB whole-browser acceptance is unmet; Android is explicitly deferred. | P1 | Keep the full validation goal open. |

Roles reviewed:5. P1 blockers:3; P2 issues:0; P3 notes:12.
BENCH/KEEL agree that clean CI validation remains required; BENCH/EDGE agree
there is no resource waiver or final acceptance. Amendments: enforce injection
ordering (implemented/tested), verify actual executing bundle instead of separate
modules (local pass, CI pending), retain scoped evidence and draft status
(implemented). Native Rust semantics and live relay behavior are unchanged.
