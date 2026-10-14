---
skill: roles-check
topic: renderer-heaps
date: 2026-10-14
roles_used: [BENCH, WIRE, KEEL, FORGE, EDGE]
p1_count: 0
verdict: APPROVED-WITH-CONDITIONS
---

# Renderer heap diagnostic review

Single-agent installed-role review. Verdict covers diagnostic evidence only;
not ship approval or independent reviewers. BENCH covers measurement validity,
WIRE integrity contracts, KEEL process boundaries, FORGE allocation meaning and
EDGE acceptance/capability preservation.

| Role | Finding | Severity | Evidence / condition |
|---|---|---|---|
| BENCH | Nine checkpoints all complete with trace hashes verified. |P3|Recorder asserts all target maps and full workload counts.|
| BENCH | Repeated observers affect subsequent allocation timing. |P3|Null targets; no acceptance or exact saving claim.|
| BENCH | No forced GC. |P3|Natural collection is uncontrolled; do not label each fall as explicit reclamation.|
| WIRE | Duplicate digest is a hypothesis, not approval to skip integrity. |P3|Review authenticated-return contracts and context checks first.|
| WIRE | Existing download/raw/saved validation remains unchanged. |P3|No production source changes in this investigation.|
| WIRE | No live NHL reliability proof. |P3|Retained local artifact/fixture workload only.|
| KEEL | Page, worker and service worker share renderer. |P3|Per-target counters are not a process partition.|
| KEEL | Archive names differ from local trace filenames. |P3|Preserve raw report; provenance explicitly maps archive hashes.|
| KEEL | Restart creates fresh worker with smaller linear memory. |P3|Does not imply all renderer allocation is released.|
| FORGE | Linear memory stabilizes16.75 MiB here. |P3|Not exact Rust live allocation or resident RAM.|
| FORGE | Backing storage declines later. |P3|No unbounded-leak or exact removable-byte claim.|
| FORGE | Allocated versus used JS heap differs. |P3|Retained heap capacity alone is not a leak.|
| EDGE | Complete workload is retained. |P3|75/eight/two/20/restart counts verified; observations remain supplemental.|
| EDGE |250 MiB remains failed. |P3|No budget waiver, browser exclusion or reduced functionality.|
| EDGE | Android remains deferred. |P3|No physical-device claim.|

Roles reviewed:5; P1:0; P2:0; P3:15.
Verdict: APPROVED-WITH-CONDITIONS for local diagnostic records.
BENCH/FORGE consensus: counters narrow investigation, not reclaimable savings.
Amendments incorporated: observer/null-target scope; nonadditive counter caveat;
integrity-preserving next hypothesis with no current implementation approval.
