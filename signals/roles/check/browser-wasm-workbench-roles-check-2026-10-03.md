---
skill: roles-check
topic: browser-wasm-workbench
date: 2026-10-03
roles_used: 12
p1_count: 4
p2_count: 26
p3_count: 6
verdict: APPROVED-WITH-CONDITIONS
---

# Browser WASM Workbench — Roles Review

**Artifact:** [Implementation plan](../../../design/plans/2026-10-03-browser-wasm-workbench.md).
**Depth:** Standard, three findings per role, 36 findings total.
**Method:** One reviewer applying the installed role lenses; these are not
independent agent reviews. Findings assess the draft's delivery risks and missing
contract detail, not implemented code. No runtime checks are claimed by this review.

## Role selection

| Role | Reason selected |
|---|---|
| HART | Repository identity, season/type axes, dataset revision and worker ownership |
| KEEL | New surface, portable loading, persistence sources, VTRACE boundaries |
| TAPE | Release archives, live acquisition, provenance, observation timestamps |
| FORGE | WASM build, Rust ownership, dependency boundaries, hostile input errors |
| PACE | Memory/decompression/query budgets and ranking methodology |
| BENCH | Native/WASM equivalence and browser lifecycle evidence |
| EDGE | Refresh races, cross-tab writes, rollover and identity collisions |
| WIRE | CORS/relay feasibility, protocol evolution, upstream partial failures |
| SCOUT | Hockey context, era interpretation, roster versus historical statistics |
| GLASS | Visible active context, persistence affordances, accessible data states |
| CREST | IceLines visual hierarchy, density, coherent recovery states |
| broadcast | Pages routing, browser privacy/security, keyboard/mobile/offline behavior |

Read `.roles/ROLE.md`, the selected `lens.verify` lists, broadcast's Lane/Stance,
and CREST's verify/reject lists. KEEL context includes `design/ARCHITECTURE.md`
and `design/IceLines.md`; governing baseline inputs include VTRACE requirements,
interfaces, implementation plan, review, and change control. Some role prose
predates current code: native `send-sync` and retired site references must be
resolved against current code and the active baseline. This review does not
reintroduce obsolete per-surface restrictions as global requirements.

## HART

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| BW-H1 | Dataset context is described, but result caches lack an explicit active-revision key; a refreshed season can serve old derived results. | P1 | 4 | Include active dataset revision and query/context signature in derived cache keys; increment generation on activation and invalidate all dependent views. |
| BW-H2 | Portable loading could flatten trade rows or discriminate goalies by position instead of canonical goalie stats. | P2 | 3, 8 | Require shared normalization, sum-of-stints invariants, identity upsert semantics, and `is_goalie()` parity fixtures. |
| BW-H3 | Worker ownership is sound in intent, but replacement/cancellation ordering is underspecified. | P2 | 3, 4 | Apply a validated replacement between executions; identify every request/context generation and discard stale responses. |

## KEEL

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| BW-K1 | The portable loader extraction is the central convergence risk; a second browser loader would fork hockey semantics. | P1 | 3, 7 | Require a chosen dependency DAG and native delegation to the same portable functions before engine-slice closure. |
| BW-K2 | Snapshot choice is explicit but mixed-family results still need a reproducible data-state identity. | P2 | 4 | Fingerprint the ordered selected family revisions and preserve it with query results and exports. |
| BW-K3 | A new public browser surface changes the three-surface baseline and build policy. | P2 | 3, 9 | Record VTRACE change/interface rows before feature code; keep existing localhost policy and native capability status intact. |

## TAPE

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| BW-T1 | Release publication dates and small size do not establish complete/final season contents. | P2 | 2, 6 | Inspect archive contents, season/type row fields and required report coverage; record unknown observation timestamps as unknown. |
| BW-T2 | Legacy release manifests lack the proposed complete integrity envelope. | P2 | 6 | Pin upstream digest/asset identity, generate conversion digests, preserve upstream provenance, and label legacy optional absence explicitly. |
| BW-T3 | A refresh may replace one family while optional evidence belongs to an older generation. | P2 | 4, 5 | Track freshness/context per family; do not imply synchronized observations; mark incompatible inputs unavailable. |

## FORGE

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| BW-F1 | Arbitrary local archives enter the engine without specified expansion/path/entry limits. | P1 | 6, 8 | Cap compressed and expanded bytes and entry count, reject traversal/links/duplicate names, allowlist files, and stage before activation. |
| BW-F2 | Filesystem helpers, clock calls and feature unification may make the WASM build fail or merely compile unusable methods. | P2 | 2, 3 | Audit transitive features with cargo tree, inject browser clock, gate native I/O, pin matching binding crate/CLI versions. |
| BW-F3 | Worker errors and worker crash recovery lack a typed wire contract. | P2 | 3, 8 | Version request/response envelopes; enumerate recovery errors; rebuild from saved packages after crash and report lost session-only data. |

## PACE

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| BW-P1 | No initial memory/transfer bounds constrain a malicious or simply huge package. | P2 | 4, 8 | Set conservative configurable import limits; measure peak staging/decompression memory, not only compressed transfer size. |
| BW-P2 | Resident historical windows can cause global repository scans and memory pressure. | P2 | 3, 8 | Start with bounded resident windows, load query requirements explicitly, reject incomplete multi-season answers, measure representative latency. |
| BW-P3 | Browser tables could alter tiebreakers, rate floors or round numbers before sorting. | P3 | 1, 8 | Render shared sorted projections and echoed implicit filters; compare full precision before display formatting. |

## BENCH

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| BW-B1 | Broad parity language needs a fixed meaningful fixture set, not matching two copies of one implementation. | P2 | 7, 8 | Add hand-audited traded-player, GP=0/floor-boundary, accented-name, ambiguous-name, goalie and wrong-season fixtures. |
| BW-B2 | Persistence acceptance omits denied storage, interrupted transactions and old/new engine combinations. | P2 | 4, 8 | Test quota/denial, two tabs, migration rollback, unsupported schema, stale-response suppression, and interrupted save. |
| BW-B3 | Deployed live probe evidence could get confused with deterministic CI tests. | P2 | 5, 8 | Record browser/origin/date/endpoints separately; mock all CI network tests; require a real-browser release check for each live family. |

## EDGE

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| BW-E1 | An old-season fetch can finish after switching seasons and activate the wrong context. | P2 | 4, 5 | Context-scoped cancellation/generation checks must gate activation and UI updates, not just hide stale rendering. |
| BW-E2 | Two tabs can race saved-active pointers or schema migrations. | P2 | 4, 5 | Use IndexedDB transactional revision checks and cross-tab notifications; only one migration writer; preserve conflicts for explicit selection. |
| BW-E3 | October rollover and unfinished playoffs could silently use stale current-season defaults. | P2 | 2, 8 | Derive season from shared constants, detect catalog gaps, reject wrong `seasonId`, keep empty/unavailable playoffs explicit. |

## WIRE

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| BW-W1 | Live refresh remains an unresolved environment dependency; a Pages-only design cannot promise access to blocked upstreams. | P1 | 5, 7 | Keep browser endpoint proof as a mandatory release gate; if direct fails, allocate a constrained relay host/owner before implementation. |
| BW-W2 | “Bounded retries” lacks a concrete retry, cancellation and partial-activation policy. | P2 | 5 | Set request timeout and retry ceilings; respect Retry-After without tight loops; retry safe reads only; required-family partial results never replace good data. |
| BW-W3 | Import schema and worker protocol evolution have no explicit supported-version behavior. | P2 | 3, 4 | Refuse unsupported future major versions, validate required fields/units/context, and perform supported migrations transactionally with original bytes recoverable. |

## SCOUT

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| BW-S1 | Live current-roster facts could overwrite historical last-stint roster meaning. | P2 | 4, 5 | Preserve roster family/date authority independently; distinguish current membership from season statistics and all-stints history. |
| BW-S2 | Missing advanced inputs or early-season GP can produce apparently confident rankings. | P2 | 1, 8 | Preserve shared sample floors and unavailable evidence; no invented advanced zero values or browser-specific prediction claims. |
| BW-S3 | Historical cross-season comparisons need descriptive era/context disclosure. | P3 | 1, 8 | Preserve shared methodology and completeness disclosures in tables/details/exports; era adjustment remains a separate capability. |

## GLASS

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| BW-G1 | Persistence actions need a visible distinction between removing a saved copy and unloading memory. | P2 | 4 | Show both states/actions and dataset identity; report save failure as usable-but-unsaved; do not bury them in an admin console. |
| BW-G2 | Async loading/retry/error status lacks a keyboard/focus behavior contract. | P2 | 5, 8 | Use semantic controls, aria-live progress, stable focus, cancellable acquisition and accessible error recovery. |
| BW-G3 | Small screens can bury active season/type and make data tables unusable. | P3 | 1, 8 | Keep context visible and horizontally contain tables; target readable 360px and desktop layouts. |

## CREST

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| BW-C1 | A new shell could become a generic dashboard instead of IceLines. | P3 | 3, 7 | Reuse visual tokens and analytical density; lead with the tool and active context. |
| BW-C2 | Catalog/library/live controls can dominate the analysis screen. | P3 | 1, 4 | Make analysis primary, with a compact data-status summary and deliberate library drilldown. |
| BW-C3 | Recovery screens need the same composition quality as the happy path. | P3 | 7, 8 | Capture cold, loading, stale, partial, unavailable and saved/unsaved screenshots; review before release. |

## broadcast

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| BW-R1 | Pages project paths, bookmarked state and cache updates can fail independently of engine correctness. | P2 | 7, 8 | Use base-relative assets plus hash routes, URL allowlists, app/engine cache versioning, and reload/update tests. |
| BW-R2 | Same-origin IndexedDB is not isolated by repository path; private filenames/query contents can also leak through links. | P2 | 4, 8 | Namespace storage by application/deployment, avoid credentials/private contents in URLs, no analytics or personal-data upload, render imported text safely. |
| BW-R3 | Saved data does not ensure an offline application shell, and browser deletion can remove local saves. | P2 | 4, 8 | Implement a versioned shell cache for offline-open claim; expose backup exports and storage-loss recovery; persistent-storage grants are best effort. |

## Synthesis

Roles reviewed: **12**. Initial findings: **4 P1**, **26 P2**, **6 P3**.

**Verdict: APPROVED-WITH-CONDITIONS for implementation planning.** This is not a
ship approval. The amended plan specifies controls for the design omissions;
their implementation/evidence remains required by the work-package gates.

Top finding: **BW-W1 — live data access must be proven in the deployed browser
environment, or supplied by a constrained relay with an allocated host/owner.**
The plan deliberately leaves that environment outcome open; it must not be
reported as a working capability before WP-BW-04 evidence passes.

Cross-role consensus:

- HART/KEEL/TAPE: same normalized model and reproducible selected revisions;
  worker/UI/storage must not invent a second domain model.
- HART/EDGE/FORGE: activation must respect context generation, borrowed-view
  lifetime, cancellation and transactional replacement.
- WIRE/TAPE/BENCH: freshness and browser source reachability need observed
  evidence, not package timestamps or native request success.
- GLASS/CREST/broadcast: saved/temporary/unavailable state must be obvious,
  accessible and recoverable.

## Three amendments applied

1. **Shared engine and generation contract:** plan section 11A adds the exact
   portable-loader gate, worker envelopes, selected-revision identity, atomic
   activation and stale-response rejection. Addresses HART/KEEL/FORGE/EDGE.
2. **Bounded acquisition and durable library contract:** section 11B adds archive
   limits, transaction/migration rules, retries, namespace/privacy controls,
   partial-data policy and relay release conditions. Addresses TAPE/WIRE/EDGE,
   FORGE and PACE; BW-W1 remains a required environment gate.
3. **Evidence and product acceptance contract:** section 11C adds fixture cases,
   performance measurement targets, URL/offline/update behavior and screenshot
   acceptance. Addresses BENCH/PACE/SCOUT/GLASS/CREST/broadcast.

## Next executable step

Execute WP-BW-01: allocate baseline IDs, compile the isolated engine path,
document portable-loader extraction, probe live/browser source access, and
record the chosen source adapters. Do not implement the complete dashboard or
publish a website as part of this planning review.

## Planning clarification after foundation work

The plan now includes a decision summary mapping catalog loads, imports, live
refresh, local saves, optional saved updates, removal, unloading, and exports
to their session and persistence behavior. Its index distinguishes existing
implementation evidence from pending release acceptance.

Rechecking the installed role lenses against that planning contract preserves
the **APPROVED-WITH-CONDITIONS** planning verdict. HART/KEEL/FORGE require one
shared normalized engine and explicit revision ownership; TAPE/WIRE require
validated provenance and deployed-browser live access; BENCH/EDGE require
failure, parity, and lifecycle evidence; PACE requires measured resource bounds;
SCOUT requires preserved hockey context and methodology; GLASS/CREST/broadcast
require clear data states, accessible analysis, and deployment/offline evidence.
These are planning requirements, not a new audit of implementation correctness.

The original 36 findings and severity counts remain the initial review ledger;
they must not be interpreted as 36 currently open code defects or as resolved
solely by plan wording. Section 12 of the plan links implementation evidence.
Live access, offline reopening, representative mobile verification, and Pages
deployment/rollback remain acceptance gates. The original next-step paragraph
above describes the sequence at initial review; subsequent foundation work does
not change its requirement to establish source feasibility before release.
