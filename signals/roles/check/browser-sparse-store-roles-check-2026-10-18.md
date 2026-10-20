---
skill: roles-check
topic: browser-sparse-store
date: 2026-10-18
roles_used: 6
p1_count: 0
verdict: APPROVED-WITH-CONDITIONS
---

# Prepared browser sparse ownership review

One agent applies HART/FORGE/BENCH/PACE/KEEL/EDGE lenses. Scope: CHG124,
separate SparseSeasonRepository, shared borrowed views/comparators/serialization,
opt-in browser backend and actual native/WASM verification. No independent panel.

| ID / role | Finding | Severity / state | Evidence | Resolution / condition |
|---|---|---|---|---|
| H1 HART | This owner is narrower than full StatsRepository | P2 resolved | prepared-store API / spec | Explicitly no roster overlays/Tier2/LRU; BrowserEngine owns windows |
| H2 HART | Identity versions must remain scoped by prepared window | P3 verified | ActivePackage ownership | No cross-package identity sharing; original merge rules reused |
| H3 HART | Widths, absent/zero values and trade stints must survive | P3 verified | original catalog/typed/JSON matrix | All eight payload types retained; no stat narrowing |
| F1 FORGE | Generic default inference could break mutable/Box reference coercions | P2 resolved | concrete PlayerView alias and coercion test | Separate ReadPlayerView; retain old constructors/field types/Copy behavior |
| F2 FORGE | Slot width and swapped ownership can break mutation correctness | P3 verified | checked indices /1200 mixed operations | Precheck before mutation; repair moved slots/owners; validate invariants |
| F3 FORGE | Borrowed reads must prohibit mutation of their owner | P3 verified | E0502 doctest / marker assertion | No fabricated row reference or clone cache; sparse owner !Send/!Sync |
| B1 BENCH | Shared formulas alone could conceal migration errors | P2 resolved | native original/default/sparse replay | Nine native samples match all 78 states; actual WASM directly matches 1,560 query strings/51,435 details/78 exports |
| B2 BENCH | Player detail serialization cannot lose payload fields | P2 resolved |15-field borrowed serialization / exact original JSON checks | No owned SeasonStats reconstruction in production |
| B3 BENCH | Both feature modes and full callers must compile | P3 verified | core/query/browser tests and native/WASM checks | Opt-in feature, current full repository remains available |
| P1 PACE | Omitting unused indexes contributes to storage savings | P3 documented | spec / native allocation scope | Attribute full prepared-owner result, not solely payload packing |
| P2 PACE | Wider handles may increase query scratch | P3 verified | separate20-query requested allocations | Query peak unchanged; first-eight cumulative bytes +0.76%, final sweep -1.86%; retained bytes zero |
| P3 PACE | Native bytes are not WASM/whole-browser acceptance | P3 documented | platform scopes / fixed budget | Separate actual WASM replay and full browser/device gates |
| K1 KEEL | Source exports and revisions must remain original package facts | P3 verified | retained SeasonPackage / per-window hashes | Never export normalized reconstructed packages |
| K2 KEEL | Refresh must remain staged and window limits unchanged | P3 verified | original activation/LRU code; invalid refresh/import/reselection replay | No new activation or eviction policy |
| K3 KEEL | Default deployment must not select unaccepted candidate | P3 verified | default=[] / sparse-storage feature | No artifact publication or merge authorized by this experiment |
| E1 EDGE | Missing goalie values and ranking ties have special ordering | P3 verified | unchanged comparator expressions / golden stat order | Legacy wrappers delegate to one read comparator |
| E2 EDGE | Replacements must release old payloads rather than append forever | P3 verified | dense owner release/pack/swap repair | Whole-window browser replacement also drops the old owner |
| E3 EDGE | Production API compatibility needs downstream evidence | P2 resolved | workspace and all feature gates | Nine-crate workspace and 1,921 production test/doc executions pass; default WASM check and sparse release build pass; failures/recovery retained |

Three amendments: keep concrete legacy view/coercion and comparator signatures;
name the narrower prepared-data owner and opt-in boundary; require original-engine
parity plus measured owner/query/platform tradeoffs. All five P2 resolved.
Native/WASM checks and actual replay pass; scoped approval covers this opt-in
candidate. See context/waves/2026-10-03-browser-wasm/evidence/miser-browser-sparse-local.json. Browser 250 MiB
and Android acceptance remain separate, with no budget waiver or publication claim.


Whole-browser follow-through (EXP011): four fresh complete ABBA runs pass all 75
package/two archive/20 warm-query/restart checks. Pair directions are mixed
(-2.09 MiB and +14.36 MiB sparse private peaks). All normal 250 MiB comparisons fail;
this is a failed resource gate, not full adoption approval. The opt-in build and
faithful evidence retain scoped approval. Standard feature build, artifact identity,
TypeScript and 163 browser tests pass. Separate GC attribution and raw cadence/source
checks are retained; physical Android stays deferred. See the process-memory JSON
beside the original native/WASM evidence. MISER-011 tracks next renderer/GPU/JS work.


2026-10-20 validation addendum: BENCH/EDGE find the retained sparse artifact now
passes offline process reopen, viewport/keyboard/full export/AX, six fresh/cached
and three persisted context-refusal cases, correct no-network saved retry, and
24-page/two-parser print checks. KEEL/PACE require the successful hosted default
artifact to remain distinct from sparse evidence: merge53a4df14 and headcdac8098
have identical trees, but CI features are empty. The acceptance ledger and receipt
state that distinction. No executable changes or change in scoped approval; failed
250 MiB acceptance and deferred Android remain open. See
context/waves/2026-10-03-browser-wasm/evidence/miser-sparse-browser-validation-20261020.json.


2026-10-20 hosted/diagnostic follow-through: BENCH verifies actual sparse dispatch
39484530829,270 independent native-loader/WASM cases and exact retained-artifact
functional reports. Native tests receive the feature; hosted scope remains selected
browser/native checks, not a full workspace matrix. KEEL verifies build-info/feature
and distinct sparse artifact identity, publication rejection before checkout, and
skipped deploy. PACE verifies both rendering diagnostic prefixes remain failed250MiB
observations, the five-sample qualification stays false, and no specific CSS cause
or positive memory saving is claimed. Generic budget-scope tests now cover rendering
as well as GC. No Rust/application-source change after the hosted revision; follow-up
files are E2E/tests/evidence only. MISER six-lens review approves EXP012's faithful
reporting with conditions. Original scoped approval remains; memory/Android and
full repository/rich-density adoption stay open. See both20261020 evidence receipts.


EXP013 addendum: BENCH/KEEL verify fresh original and both candidate storage-mode
resident probes, candidate/restored browser suites, exact restored main/engine
bytes and actual standard build. PACE rejects a whole-browser gain from DOM reuse:
private-peak pair directions are mixed and the declared repeated10MiB criterion
fails. Main.ts is reverted; only the optional functional probe and evidence remain.
The hosted trial success is explicitly separate from restored-source equivalence
to earlierda22174c. MISER six-lens review approves faithful rejection/restoration
reporting, with all250MiB/Android/private billing gates retained. No scoped approval
of the rejected UI optimization or whole-goal completion. See EXP013 public receipt.


EXP014 addendum: BENCH/KEEL verify two direct frame/worker/target-to-PID mappings,
no pre-load engine worker, first-season/detail/restart behavior and unchanged app
bytes. PACE requires tracing overhead, sequential snapshot bounds, unknown renderer
costs and the unchanged normal250MiB failure to remain explicit. Existing Playwright
attachment and40s running status do not prove a causal lifetime mechanism. Six
MISER lenses approve diagnostic reporting with conditions; shared-renderer allocator
attribution remains next. New Rust streaming guards run in all-target tests (12 local
passes), without a hosted-private success claim. See the EXP014 bounded receipt.


EXP015 addendum: BENCH verifies ten successful detailed nondeterministic dumps,
original/native/export ID linkage and16 local guards plus exact legacy replay.
PACE/KEEL require provider units/ownership, two ID namespaces and diagnostic scope
to stay explicit; no roots/children sum or equation with OS private bytes. Growing
compositor accounting is not a proven CSS cause or leak. Six MISER lenses approve
evidence/tooling with conditions. No application changes or normal250MiB/mobile/
private-hosted acceptance. Full-workload compositor/native lifetimes remain next.


EXP016 evidence addendum (2026-10-20): two failed initial captures and two complete
full75 diagnostics retained. All18 final requests pass strict linkage and no-loss
checks;20 lab tests and exact legacy replays pass. Separate global namespaces and
failed-request omission guards preserve measurement boundaries. Nonmonotonic cc
refutes steady growth at observed checkpoints; transient native loading/eviction
lifetimes remain next. Six MISER role lenses approve evidence scope with conditions.
No product change, optimization saving, normal250MiB acceptance or mobile claim.


EXP017 addendum: two complete full75 digest-buffer observations, independent
catalog call/byte parity and22 local tests pass.11.07MiB by8 and84.06MiB cumulative
through imports select a view-based digest trial. Weak cohort collection does not
certify liveness/leaks or exact OS savings. Source/custody guards and six MISER
lenses approve evidence scope only; future candidate must preserve offset/shared/
snapshot/transfer semantics and pass normal-workload comparisons. Original gates
remain open. No application change in this evidence update.


EXP018 addendum: reject/revert the digest view application trial. Final semantic
checks pass (173 units/ten browser cases/four suites), but normal private pairs
regress1.88/37.39MiB and miss repeated10MiB improvement. Preserve initial subclass/
RAB failures and all six normal runs. Restored9bae39fd has exact original application
assets,172 units/ten browser cases/four suites pass; nine semantic regressions stay.
Six MISER lenses approve negative evidence/guidance scope, not adoption.250MiB,
Android and private-hosted gates stay open; no causal collection claim.


EXP019 addendum: no handoff trial selected. Two explicitly primed worker diagnostics
show no catalog input cohort observable by75;26 lab tests/strictClippy/identity/
source/OS audits pass. Preserve recovery/borrowed semantics. Six MISER lenses approve
evidence scope only: priming/evaluation affect timing, termination is unavailable,
not-collected is not liveness, and native/compressed/main storage is outside this
census.18 private archives/four public raw files retained. Original gates stay open;
next normal process mapping keeps instrumentation outside the measured interval.


EXP020 addendum: two complete normal prefixes with attribution after sampler exit.
Source/boundary/marker/target/type/birth and all-byte accounting checks pass with33
lab tests. Full private peaks428.08/387.61MiB still fail250MiB. Keep73.72MiB transient
PID unknown/counted and renderer without target unbound; final engine cannot label
terminated old workers. Six MISER lenses approve qualified evidence only, with no
independent-floor/thread-cost/causal leak inference. Nine raw public/30 private
archives retained. Next transient OS metadata; application and original gates remain.
