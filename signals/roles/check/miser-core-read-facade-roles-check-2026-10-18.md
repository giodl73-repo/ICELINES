---
skill: roles-check
topic: miser-core-read-facade
date: 2026-10-18
roles_used: 6
p1_count: 0
verdict: APPROVED-WITH-CONDITIONS
---

# Additive core season-read interface review

Scope: SeasonStatsRead, shared catalog read/sort/filter and legacy helper delegation,
pre-refactor oracle, split-storage witness, docs/trace and local validation logs.
One agent applies HART, FORGE, BENCH, PACE, KEEL and EDGE as repository lenses;
these are not independent reviewers or production sparse-backend acceptance.

Selection: HART for model/meaning, FORGE for ownership/public Rust API, BENCH for
independent regression evidence, PACE for numeric guards, KEEL for convergence and
EDGE for missing/zero/borrow boundaries. KEEL architecture/app context was read.

| ID / role | Finding | Severity / state | Reference | Remedy / remaining condition |
|---|---|---|---|---|
| H1 HART | Read abstraction must retain original keys, widths and all optional payloads | P2 resolved | season_read.rs / typed accessor test | Original types and borrowed slices/options; canonical row contract; identity-based PlayerView id preserved |
| H2 HART | Domain computation must not become adapter-overridable | P2 resolved | private SeasonStatsReadExt blanket impl | Public trait supplies data only; core-owned helpers and one catalog formula implementation |
| H3 HART | Production ownership is a separate migration | P3 tracked | interface spec / MISER-010 | Existing SeasonStats, PlayerView.stats and repository return types remain; no backend adoption claim |
| F1 FORGE | Independently stored payloads cannot safely fabricate &SeasonStats | P2 resolved | SplitRow witness / raw trait getters | Real separately allocated payload references, no whole SeasonStats field or fake Deref |
| F2 FORGE | Returned references must prevent owner mutation while live | P2 resolved | compile-fail season_read doctest | Owner-tied reference signatures, correct fixture construction and post-mutation use; no unsafe code added |
| F3 FORGE | Static/dynamic readers must compile without changing Send/Sync or dependencies | P3 verified | integration / workspace/WASM checks | ?Sized/dyn path tested, marker/dependency definitions unchanged; no ABI/layout promise |
| B1 BENCH | Comparing new entry points to delegated legacy paths alone is tautological | P2 resolved | producer 9af58a5d / retained oracle | Capture clean unmodified-production observations before refactoring; never rebaseline after production changes |
| B2 BENCH | Count correctness must include ordering, missingness and boolean semantics | P2 resolved | 108 x 288 oracle and tests | Exact optional f64 bits, complete sorted IDs, eight filter vectors across four reader paths |
| B3 BENCH | Local selected tests are weaker than all hosted surface checks | P3 tracked | local verification JSON | State exact 1,430-test scope; hosted CI still required before merge |
| PA1 PACE | Numeric re-derivation could alter precision or guard precedence | P3 verified | shared helpers / oracle | Move existing computations; preserve f32/f64 widths, saturating operations and exact baseline outputs |
| PA2 PACE | TOI floors, trade exception and position/no-op guards require explicit boundaries | P3 verified | explicit integration guard test | 299/300 floor, summary-TOI precedence, traded on-ice exclusion/deployment exception, center applicability |
| PA3 PACE | An interface enabler is not a memory/latency improvement | P3 documented | spec / change control | No percentage, browser budget, monomorphization or dispatch-speed claim |
| K1 KEEL | All surfaces must keep one data/computation contract | P3 verified | legacy delegation / shared catalog | Existing read/filter paths delegate; canonical sort comparator retained; no new formula copy |
| K2 KEEL | Additive facade does not migrate identity/contract-aware executor paths | P3 tracked | unchanged query interfaces / next step | Full executor/backend/native-WASM/export/revision remains separate acceptance |
| K3 KEEL | Cross-crate and WASM compatibility need real compilation | P3 verified | native workspace and wasm32 logs | All nine crates and browser target check pass; no production release implied |
| E1 EDGE | None, zero, negative, fractional, GP=0 and season/type cases must survive | P3 verified | synthetic matrix / typed getters | Boundary shapes cover GP 0/9/10, missing/zero/TOI, trade, goalie/type and metadata; not production-density evidence |
| E2 EDGE | Existing NOT-missing and non-applicable-atom semantics must remain | P3 verified | oracle filters / applicability check | Preserve old short-circuit/tolerance logic; no unrequested behavior correction |
| E3 EDGE | Disk recovery could hide a failed validation or change resource criteria | P3 documented | failed raw log / local profile | Retain disk error, clear only verified task cache, record debug-symbol-free validation; no runtime resource change |

Roles reviewed: 6. P1: 0. P2: 6 resolved, 0 open. P3: 12 verified,
documented or tracked. Verdict APPROVED-WITH-CONDITIONS for the additive core API;
normal hosted CI remains a merge condition. Top finding: keep one core computation
implementation and an independent pre-change oracle while separating read access
from production storage. HART/FORGE agree raw data contracts must not override
formulas; BENCH/PACE agree exact frozen observations are needed beyond delegated
self-comparison. KEEL/EDGE agree full backend/browser acceptance is not established.

Three amendments incorporated:
1. Freeze legacy observations before production edits and compare full bits/orders/
   filters with a genuine separately owned payload reader and dynamic dispatch.
2. Keep computed helpers crate-private with a blanket implementation, exposing
   only borrowed data access and preserving all old PlayerView/repository APIs.
3. Retain guard/borrow tests, native and WASM checks, failed disk recovery and exact
   local validation scope; defer resource/backend claims to measured adoption.
