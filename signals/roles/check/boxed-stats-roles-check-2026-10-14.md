---
skill: roles-check
topic: boxed-stats
date: 2026-10-14
roles_used: [HART, BENCH, KEEL, FORGE, EDGE]
p1_count: 0
p2_count: 0
verdict: APPROVED
---

# wasm32 stats-slot storage review

Artifact: stats_repository.rs, source d825f9d01d937c8878b961528346486238bd8885.
Single-agent review through five installed role lenses; no independent-agent claim.
Scope: incremental memory optimization, not full browser acceptance. Native rows
remain inline; wasm32 rows use unique Box ownership. Public getters still borrow
SeasonStats/PlayerView, and key shape, normalization and resident bounds are unchanged.
No dependencies, features, public schemas or engine-worker messages change.
This changes internal allocation, not crate/actor ownership boundaries or acceptance
methods, so no new controlled requirement/interface change is proposed.

HART covers canonical data and lifetimes; BENCH measurements and parity; KEEL
surface/target scope; FORGE safe ownership/code rigor; EDGE failures and eviction.

| Role | Finding | Severity | Evidence / recommendation |
|---|---|---|---|
| HART | Keys remain player/season/type; boxing does not merge windows or records. | P3 | Original key construction, roster indexing and LRU bookkeeping remain unchanged. |
| HART | Public borrowed types and ownership constraints remain intact. | P3 | Native1,182 tests and repo_swap compile-fail pass; wasm32 compiles with the same getters. |
| HART | Normalization, traded-player stints and goalie discriminator are unchanged. | P3 | Actual engine matches270 native-loader cases/33,613 rows; no normalization code changes. |
| BENCH | Two opposite-order full-workload pairs show lower candidate peaks. | P3 |15.91/5.41 MiB raw saving, with all baselines reported; no universal saving claim. |
| BENCH | Existing Windows evidence does not prove the exact CI/master artifact. | P3 | Fulfilled: PR/master29 checks and exact retained artifact/browser/PDF/resource validation; all failures/qualifications retained. |
| BENCH |250 MiB still fails; query timing remains within the local target. | P3 | Every candidate/control report retains failed budget flags; all warm maxima <96 ms. |
| KEEL | Native storage remains inline; change is wasm32-only. | P3 | Target-specific alias/constructor, with shared public borrowed types. |
| KEEL | UI/catalog bytes and shell asset set are unchanged. | P3 | Byte comparison passes;27 assets/182 files, only WASM/build metadata differ. |
| KEEL | Public update/rollback/restore need evidence for the changed engine. | P3 | Fulfilled: direct same-profile transitions,273-file verification in each phase, saved bytes/settings and executing engine fingerprints pass. |
| FORGE | Box provides unique ownership with no unsafe code. | P3 | Existing repository owner and !Send/!Sync marker policy remain; no shared mutable state added. |
| FORGE | Spare hash buckets hold small pointers rather than full696-byte rows. | P3 | Compiled wasm32 layout probe measures712-byte inline tuple vs16-byte boxed tuple; not an allocator/RAM prediction. |
| FORGE | Native noop-borrow warning is fixed explicitly. | P3 | Final native/all-target and wasm32 strict Clippy pass; final WASM bytes match checked initial engine. |
| EDGE | Eviction and replacement retain existing atomic load boundaries. | P3 |75 real package loads/eight residents/two imports/restart complete; no early eviction or validation removal. |
| EDGE | Corrupt transfer and forged catalog context preserve previous good data. | P3 | Actual refusal/exact export/retry and six fresh/cached context cases pass. Context fixture is unsigned and invokes hidden retry handler; not offline/UI-flow proof. |
| EDGE | Saved-data offline restoration and complete-result navigation remain functional. | P3 | Process-close offline fresh query/detail; all933 names/ranks,120 forward/reverse Tab steps, terminal boundaries,77 goalies and print transitions pass. No physical-phone/screen-reader/printer claim. |

Roles reviewed:5. P1 blockers:0. P2 adoption conditions:0. P3 notes:15.
Verdict: APPROVED for an incremental optimization only.
Top finding: preserve borrowed/model invariants while measuring whole-browser
effects. HART and FORGE agree ownership remains unique; BENCH and KEEL require
exact retained CI/public evidence. Amendments applied: wasm32-only storage;
explicit cross-target borrowing; retain full workload and failed250 MiB flags.
Raw evidence and hashes: boxed-stats-provenance-20261014.json.

Final reconciliation: merged5ef015c4, master395.54 MiB against237.10 blank with
97.05 ms warm max. First CI100.81 ms is retained alongside99.40 repeat; CI controls
are qualified as shared, not independent.250 MiB remains failed; Android deferred.
Publication38551551343, rollback38552043788 and restore38552342417 succeed without
rebuild. Same original owned profile/95 identical docs preserved; actual executing
WASM bytecode matches the new/old/new retained binaries online and offline.
See boxed-stats-master/publication provenance. This closes the two incremental
adoption conditions, not the full resource/mobile objective.
