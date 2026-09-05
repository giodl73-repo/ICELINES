---
skill: roles-check
topic: weekly-operations-planner
date: 2026-09-05
roles_used: [hart, keel, tape, forge, pace, bench, edge, wire, scout, glass, crest, broadcast]
p1_count: 0
verdict: APPROVED-WITH-CONDITIONS
---

# Weekly Operations Planner — `.roles` Review

Implementation closeout: the P2 conditions were incorporated into the shared
core/fetch implementation and verified on 2026-09-05. The retained P3 items are
ongoing hardening signals rather than release blockers; bounded search and
provisional evidence remain visibly disclosed.

## Artifact identification

- **Artifact**: `design/plans/2026-09-05-weekly-operations-planner.md`
- **Type**: architecture and implementation plan
- **Domains**: fantasy hockey optimization, time/legality, SQLite persistence,
  immutable audit history, CLI/TUI/Web parity, accessibility, performance

## Role selection

All installed roles are material. HART owns the planning key and player axes;
KEEL the shared engine and surfaces; TAPE source fidelity; FORGE Rust boundaries;
PACE optimizer claims; BENCH proof; EDGE temporal failure modes; WIRE schemas and
degradation; SCOUT hockey sense; GLASS and CREST decision presentation; and
broadcast the browser contract.

## Findings

### HART — domain model invariant keeper

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| H1 | A date-only move is not sufficiently keyed: locks, waivers, and same-day eligibility change within a date. | P2 | Domain model | Add exact UTC effective/lock instants plus league timezone and local date. |
| H2 | Platform player keys can collide with or drift from NHL identity. | P2 | Planner input | Carry league-scoped platform key and optional canonical `PlayerId`; never join by display name. |
| H3 | Schedule values are season/type-shaped although the proposed schedule map key names only team. | P3 | Planner input | State that the complete plan context owns season/type and invalidates the map as one immutable input. |

### KEEL — system architecture coherence

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| K1 | The plan says the daily cockpit may consume a saved or fresh plan but does not name one assembly authority. | P2 | Daily cockpit | Add one `icelines-fetch` assembly service shared by CLI, TUI, Web, and today. |
| K2 | Journal persistence and planner read assembly could become intertwined and make reads mutate. | P2 | Decision journal | Separate pure plan assembly from explicit journal commands and prove DB hash stability. |
| K3 | Static-site parity is not discussed. | P3 | Surface contract | Declare the static site out of scope because plans are private, time-sensitive local state. |

### TAPE — data accuracy analyst

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| T1 | Per-game rates lack source season/type and observed/fetched timestamps. | P2 | Planner input | Attach evidence IDs and age to every decision-bearing rate. |
| T2 | Schedule and ownership completeness are asserted but not given authority grades. | P2 | Existing foundation | Require complete-week schedule and complete league ownership snapshots or degrade. |
| T3 | Display names may change after capture. | P3 | Decision journal | Preserve both stable keys and the exact displayed label in projection bytes. |

### FORGE — Rust engineer

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| F1 | The plan names invalid inputs but no typed core error surface. | P2 | Core contract | Define a `FantasyPickupSequenceError` enum; adapters add context without parsing strings. |
| F2 | Floating-point NaN/Infinity could break total ordering, fingerprints, or JSON. | P2 | Search and scoring | Validate all numeric inputs as finite before search. |
| F3 | Search cancellation and timing could leak wall-clock nondeterminism into output. | P3 | Performance | Use deterministic state caps for contract output; elapsed time is observability-only and excluded from fingerprint. |

### PACE — methodology statistician

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| P1 | “Realized active value” does not provide the exact sequence objective. | P2 | Search and scoring | Freeze the points formula and each penalty/bonus term with units. |
| P2 | Beam search is not globally optimal, so “highest-valued legal complete state” overclaims. | P2 | Planner output | Say best evaluated bounded state and disclose caps/truncation. |
| P3 | Alternative materiality and score tie precision are undefined. | P3 | Planner output | Define canonical sequence key and epsilon-free `total_cmp` ordering over validated finite values. |

### BENCH — test engineer

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| B1 | Example-based tests alone will miss illegal intermediate rosters and repeated-player transitions. | P2 | Verification | Add property tests asserting legality after every prefix and budget monotonicity. |
| B2 | Cross-surface parity needs a sealed shared projection fixture, not three independent goldens. | P2 | Parity | Add one core fixture consumed by CLI, TUI, and Web tests. |
| B3 | “DB unchanged” needs a concrete proof. | P3 | L1/L2 | Hash the main database plus WAL/SHM-relevant state before and after read commands. |

### EDGE — edge-case specialist

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| E1 | Multiple acquisitions at the same instant need deterministic ordering; a drop can otherwise free a slot ambiguously. | P2 | Waivers and locks | Define ordered atomic transitions and reject duplicate effective ordinals. |
| E2 | A dropped player, failed waiver, claimed fallback, or newly locked player can invalidate later prefixes. | P2 | Contingencies | Re-simulate each fallback from the exact pre-move state and expose conditional branches. |
| E3 | DST, week rollover, past dates, Sunday reserve release, open roster slots, IR occupancy, and exhausted budgets need explicit behavior. | P2 | Verification | Add typed cases and tests for every boundary. |

### WIRE — API and schema pipeline

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| W1 | New JSON contracts need compatibility rules and unknown-field behavior. | P2 | Core contract | Version schemas, deny unknown fields at import boundaries, and preserve old journal rows as opaque projection bytes. |
| W2 | Missing/stale provider state needs machine-readable recovery, not warnings alone. | P2 | Planner output | Reuse typed readiness/evidence/recovery rows from `fantasy_today.v2`. |
| W3 | A future Yahoo sync must not silently replace manual state mid-plan. | P3 | Product boundary | Fingerprint ownership/eligibility snapshots and require explicit refresh/replan. |

### SCOUT — hockey domain expert

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| S1 | Historical per-game value can overrate a player whose current deployment or injury role changed. | P2 | Search and scoring | Apply an explicit uncertainty/evidence discount; label stale role evidence conditional. |
| S2 | Goalie streams are not interchangeable with skater adds and can be needed solely to reach the weekly minimum. | P2 | Domain model | Keep goalie capacity/minimum logic typed and separate inside the shared objective. |
| S3 | Defense coverage must show actual usable D substitution rather than team off-night correlation. | P3 | Quiet nights | Preserve the named D-slot coverage requirement and test a three-D collision fixture. |

### GLASS — visualization and UX critic

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| G1 | A four-move plan plus seven-day table can overflow the five-second scan path. | P2 | Surface contract | Lead with hold/add/drop/deadline; collapse daily evidence behind concise rows. |
| G2 | Firm, conditional, and blocked states cannot rely on color. | P3 | TUI/Web | Always render text labels and recovery verbs. |
| G3 | An 80-column fence needs explicit truncation rules for long player names. | P3 | CLI | Preserve dates/action/value first; truncate labels deterministically with full JSON unchanged. |

### CREST — visual design aesthetic

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| C1 | Rendering the plan as one undifferentiated table will feel like optimizer debug output. | P3 | Surface contract | Compose a calm primary-plan block, restrained alternatives, then evidence. |
| C2 | Hockey identity should come from week/date/slot rhythm, not decorative graphics. | P3 | TUI/Web | Use a seven-day strip and transaction timeline; avoid ornamental panels. |
| C3 | Empty/no-move state needs intentional language. | P3 | Planner output | Render “Hold — your current roster is the best evaluated plan” with the saved move reserve. |

### broadcast — web-view perspective

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| BR1 | The bookmarkable route does not define query parameters or invalid-week recovery. | P2 | Web | Freeze `league`, `team`, and ISO Monday `week`; canonicalize or return a typed 400 with recovery. |
| BR2 | A time-sensitive plan must not be browser/proxy cached as if durable. | P2 | Web | Emit `Cache-Control: no-store` and visible evaluated/evidence timestamps. |
| BR3 | The daily table needs mobile containment and semantic row headers. | P3 | Web | Use semantic ordered moves and a horizontally contained coverage table with a no-JS fallback. |

## Synthesis

```text
Roles reviewed: 12
P1 blockers: 0  |  P2 issues: 22  |  P3 notes: 14

Verdict: APPROVED-WITH-CONDITIONS

Top finding: legality must be evaluated at exact instants against each
intermediate roster, not inferred from dates or independent move rankings.

Cross-role consensus: HART, EDGE, PACE, BENCH, and WIRE require one typed,
deterministic temporal state machine with explicit evidence and prefix tests.
KEEL, GLASS, CREST, and broadcast require every surface to consume its one
projection without recalculation.
```

## Amendments required before implementation

1. Add an explicit temporal transition model: UTC effective/lock instants,
   stable transition ordinals, pre/post roster fingerprints, typed errors, and
   exact fallback re-simulation.
2. Freeze the objective and honesty boundary: finite numeric validation, exact
   points components, separate goalie/category terms, canonical tiebreakers,
   “best bounded state” wording, state caps, and fingerprint exclusions.
3. Harden authority and delivery: shared fetch-owned assembly, typed readiness
   and recovery, immutable versioned journal bytes, no-store Web semantics,
   one cross-surface fixture, prefix/property tests, and read-only DB proof.

The conditions are plan amendments and implementation evidence. They do not
require a new architecture direction.
