---
skill: roles-check
topic: wasm-browser-measurements
date: 2026-10-12
roles_used: [bench, forge, wire, keel, broadcast]
p1_count: 3
verdict: NEEDS-WORK
---

# Terminal browser acceptance and OS memory review

Artifact: PR83 browser runners, pinned development dependency, README, acceptance
ledger and dated offline/process-memory evidence. Code and evidence artifact;
domains: test reliability, resource accounting, browser storage and release claims.

Inventory: all 13 Markdown files in `.roles/` inspected as an inventory. Selected
BENCH for meaningful tests, FORGE for ownership/error handling and memory limits,
WIRE for input/network boundaries, KEEL for scope and release gates, broadcast
for browser/mobile behavior. Selected role instructions and verification lenses
were read. This is one agent applying five lenses, not five independent reviewers
or an independent release approval. Native Rust soundness and HTMX-only rules
remain native-surface constraints; no Rust/domain or production UI code changed.

## BENCH — cases and measurement coverage

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | The process-tree counters exceed the provisional 250 MiB target. | P1 open | Memory report | Investigate allocations; retain the failed target. Measurement completion cannot close resource acceptance. |
| 2 | Repeated archive imports could match the prior status before the new import completed. | P2 amended | memory.mjs imports | Now wait for season/type and the verified catalog's expected row count. Final run returns 905 regular and 332 playoff players. |
| 3 | The offline filter derives expected names from the initial table. | P3 constraint | offline-reopen.mjs | This proves restoration and fresh filtering, not native/WASM domain parity. Existing parity corpus remains its separate evidence. |

## FORGE — ownership and failure reporting

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 4 | Fast phases could finish between OS samples. | P2 amended | memory.mjs phase coverage | Hold startup/package phases until samples arrive; fail if any requested phase lacks samples. Final report has no missing phases. |
| 5 | Windows process discovery adds overhead and does not capture every instantaneous spike. | P3 constraint | sample-browser-memory.ps1 | Report actual intervals and instrument limits; do not present requested 100 ms as actual cadence. Final median/max are 185/755 ms. |
| 6 | Process resources and staging remain outside the Rust allocator figure. | P3 constraint | report metrics | Keep process counters distinct from prior WASM linear-memory evidence; do not infer an allocator leak from retained OS allocations. |

## WIRE — artifact and network boundaries

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 7 | Remote offline acceptance fetches public Pages assets. | P3 scope | runner --url | Keep opt-in environment evidence separate from mocked/unit CI; no NHL refresh is invoked. |
| 8 | Memory measurements identify exact retained artifact and archive bytes. | P3 provenance | artifact verifier/report | Keep full commit, shell identity and archive digest with each report; do not generalize to unrelated builds or archives. |
| 9 | Profiles and raw process samples are local artifacts. | P3 privacy | evidence publication | Public offline evidence omits its temporary profile path. Sampler emits only owned-process counters/IDs, not unrelated process command lines or user data. |

## KEEL — acceptance and architecture claims

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 10 | A local artifact/server-stopped test does not prove public rollback compatibility. | P1 open | rollback gate | Rehearse retained releases remotely with saved bytes and documentation preserved; approval remains pending. |
| 11 | Successful measurement exit could be mistaken for a passing acceptance target. | P2 amended | package scripts/README | Use `measure:memory`; document completed measurement versus the separate false target fields. No release completion claim. |
| 12 | This testing slice introduces no alternate stats engine. | P3 scope | architecture | Production worker/WASM and existing UI run unchanged; no blanket native-surface convergence promotion. |

## broadcast — actual browser/device behavior

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 13 | Headless workstation measurements are not physical phone evidence. | P1 open | mobile gate | Test touch, picker, exports and latency on a real device; retain the mobile gate. |
| 14 | Deployed offline reopening now has actual controlled-network browser evidence. | P3 scope | offline report | Close the selected desktop Chromium gate only: fresh process, saved restoration, fresh query/detail and failed uncached network probe. |
| 15 | Working-set sums include shared-page duplication; private bytes include nonresident allocations. | P3 constraint | memory claims | Name both metrics precisely and retain baseline/hardware; neither is an exact concurrent group physical-memory peak. |

Roles reviewed: 5. Findings: 15 (3 P1 open, 3 P2 amended, 9 P3 constraints).
Verdict: NEEDS-WORK for full release acceptance; tooling/evidence is reviewable
subject to current-head CI. Top finding: measured memory exceeds the provisional
target. BENCH/FORGE/KEEL agree that passing functional tests cannot waive this.

Amendments: (1) corrected import completion fences, (2) required measured-phase
coverage, (3) named measurement status separately from budget acceptance and
recorded the failed target. Next work: allocation attribution and representative
hardware, physical phone, remote rollback with saved-data checks.
