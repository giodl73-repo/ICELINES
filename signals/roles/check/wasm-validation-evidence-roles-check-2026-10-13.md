---
skill: roles-check
topic: wasm-validation-evidence
date: 2026-10-13
roles_used: [bench, forge, keel, wire, edge]
p1_count: 2
verdict: NEEDS-WORK
---

# Validation evidence and completion-barrier review

Scope: acceptance/memory ledgers, retained positive and incomplete reports, and
the memory runner's query-commit barrier. Installed role files were previously
inventoried; one agent applies the selected lenses. This is not independent
review or full release acceptance. No runtime prototype is included in this diff.

| Role / ID | Finding | Severity | Evidence / disposition |
|---|---|---|---|
| BENCH B1 | The 250 MiB whole-browser target still fails | P1 open | Latest corrected controls remain around 544–548 MiB; limit unchanged |
| BENCH B2 | Same row count and enabled form could represent a pending prior view | P2 amended | Wait for existing tbody.inert marker to clear; full candidate/control workloads pass stronger barrier |
| BENCH B3 | Evidence flags alone are insufficient for broad acceptance | P3 verified | Structural audit checks 35 JSON files, 11 complete memory workloads, input counts/residency/peak arithmetic and current public storage/renderer assertions; scope remains qualified |
| FORGE F1 | No experimental runtime or new Rust API is included | P3 verified | Runtime source diff against master is empty; only e2e readiness and evidence/docs differ |
| FORGE F2 | Measurement must preserve owned-process accounting and workload | P3 verified | Original sampler, flags, 75 packages, two imports, 20 queries and restart retained |
| FORGE F3 | Complete metadata-only package identity does not prove executing UI version | P3 verified | Actual pending state and DOM mutation behavior distinguish new/legacy renderer during public transitions |
| KEEL K1 | Merged source, public source and local prototypes must stay distinct | P3 verified | Current public e0741daa, exact CI provenance and unadopted runtime branches recorded |
| KEEL K2 | Local two-tab evidence must not become a public two-tab claim | P3 verified | Ledger/provenance explicitly separate localhost two-tab consent from public one-tab transitions |
| KEEL K3 | Validation publication is distinct from accepted memory/mobile completion | P3 verified | Current release explicitly labeled validation build; full goal remains incomplete |
| WIRE W1 | Saved test records must survive rollback and forward restore | P3 verified | Exact hashes/revisions/metadata and recorded settings values compare equal across public transitions |
| WIRE W2 | Retained artifact/deployment identity and documentation must be checked independently | P3 verified | Exact e0741daa artifact104/package75; final195 served hashes/MIME including94 docs; complete upload tar guard includes hidden files |
| WIRE W3 | Offline execution is not established by static byte verification | P3 verified | Separate actual process-close offline network failure, saved restoration, fresh query/detail and renderer checks |
| EDGE E1 | Notice timeouts cannot be erased or described as a code fix | P3 verified | Incomplete probe retained; repeated same-deployment pass recorded; cause unproven |
| EDGE E2 | Correct prototype behavior does not establish lower memory | P3 verified | Cache, context, handler and opacity trials remain unadopted with separate raw reports |
| EDGE E3 | Physical Android remains untested and deferred | P1 open | No emulated/desktop substitute; user deferral retained |

Five lenses, 15 findings: two P1 full-goal gaps open, one P2 amended and twelve
P3 findings verified. Full-goal verdict: **NEEDS-WORK**. Scoped validation patch:
**APPROVED-WITH-CONDITIONS** on required CI and preservation of bounded claims.
BENCH and KEEL agree that completed validation execution must not be presented
as a passed 250 MiB requirement. Source/schema/URL behavior and user data are
unchanged by this patch. The evidence audit is consistency checking, not proof
of every possible dataset, device, input or browser behavior.
