---
skill: roles-check
topic: miser-player-query-read
date: 2026-10-18
roles_used: 6
p1_count: 0
verdict: APPROVED-WITH-CONDITIONS
---

# Borrowed Constraint evaluator review

One agent applying HART/FORGE/BENCH/PACE/KEEL/EDGE lenses; no independent reviewer
claim. Scope: PlayerRead, shared Constraint evaluator, pre-change oracle, integration
tests and CHG-123 / IF-QUERY-READ-001. Final local checks pass; hosted CI remains a merge gate.

| ID / role | Finding | Severity / state | Evidence | Resolution / condition |
|---|---|---|---|---|
| H1 HART | Provider lookups must use identity IDs rather than raw stat IDs | P2 resolved | unchanged identity.id accesses; explicit routing test | Preserve defensive legacy incoherent-view behavior |
| H2 HART | Historical identity objects cannot be deduplicated by ID | P3 verified | two distinct identity borrows sharing one ID | Keep caller/window-specific references |
| H3 HART | Position, goalie applicability and last-stint team are season facts | P3 verified | PlayerRead helpers, frozen matrix | No identity-position substitution |
| F1 FORGE | Legacy API must stay source-compatible | P3 verified | original matches signature delegates | PlayerView fields and repository reference types unchanged |
| F2 FORGE | Separate payload owners must work without constructing SeasonStats | P3 verified | Split reader has no SeasonStats field | Generic and dynamic interfaces return original typed borrows |
| F3 FORGE | Borrow doctest could pass for an unrelated fixture typo | P2 resolved | valid identity/stats builders; E0506 compile-fail test | Check the intended owner-mutation error |
| B1 BENCH | Mutual comparisons after delegation could hide changed evaluator rules | P2 resolved | clean original producer 0a82da65, frozen 813 x 288 observations | Retain immutable golden/custody; do not regenerate after edits |
| B2 BENCH | Generic, dynamic and legacy readers need the same full-IR outcomes | P3 verified | player_read full matrix | Cover all Constraint variants and nullable scalar/range/bio shapes |
| B3 BENCH | Goalie identity fallback needs explicit separate-reader coverage | P2 resolved | targeted integration test | Check absent identity data, payload fallback and identity precedence |
| P1 PACE | New API is not evidence of engine memory/latency gains | P3 documented | spec non-goals | No benchmark or browser budget claim |
| P2 PACE | Tolerances, applicability no-ops and career/window rules must remain | P3 verified | evaluator diff / original matrix | Only access paths and signatures change |
| P3 PACE | Original country matching accepts birth country OR nationality | P2 resolved | failed new expectation retained; corrected historical fixture | Preserve existing semantics; no production fix to force a test |
| K1 KEEL | All surfaces must still compile through existing entry point | P3 verified | native workspace and WASM checks | Full downstream compilation gate |
| K2 KEEL | Contract borrow retention does not add contract query predicates | P3 documented | contract pointer test / API docs | Carry existing optional reference unchanged |
| K3 KEEL | Full Constraint support is narrower than production sparse adoption | P3 tracked | spec / PR scope | Legacy helper/surface/repository migration stays separate |
| E1 EDGE | Boolean recursion must retain provider short-circuiting | P3 verified | targeted identity/provider test | False All / true Any must avoid later provider calls |
| E2 EDGE | Zero/missing, trades, goalie/skater and season/type combinations matter | P3 verified | 288 source cases / frozen results | Exact boolean outcome comparisons |
| E3 EDGE | New dependency/storage/Send changes would expand migration risk | P3 verified | unchanged manifests, locks, core owners, EvalCtx | No new dependencies, caches or marker changes |

Three amendments: freeze original outcomes before production edits; preserve
identity/contract references with explicit routing and precedence tests; name the
native/WASM/repository/resource gates without claiming a backend switch.
Five P2 resolved, zero P1/P2 open; 13 P3 verified/documented/tracked. Scoped verdict: APPROVED-WITH-CONDITIONS for additive evaluator compatibility, with hosted CI before merge. Final unique query suite: 657 tests/docs, plus 20 downstream tests; query all-target Clippy, native nine-crate workspace, WASM target and fmt pass. Targeted tests cover goalie fallback/identity precedence/absence, empty stints and provider short-circuiting. Failures and successful recovery are retained in miser-query-read-local.json and compressed logs. No backend adoption or resource acceptance is implied.
