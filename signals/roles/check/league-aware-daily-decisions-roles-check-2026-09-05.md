---
skill: roles-check
topic: league-aware-daily-decisions
date: 2026-09-05
roles_used: 12
initial_p1_count: 9
p1_count: 0
verdict: APPROVED-WITH-CONDITIONS
---

# League-Aware Daily Decisions — Roles Check

## Artifact identification

- **Artifact**: `design/plans/2026-09-05-league-aware-daily-decisions.md`
- **Type**: product, architecture, interaction, and implementation plan
- **Domain**: fantasy decisions, local data assembly, matchup context,
  transactions, concurrency, CLI/TUI/Web parity, and PUCK interoperability

## Role selection

All twelve installed IceLines roles apply. The feature changes the decision
contract, storage joins, performance boundary, surface loading model, hockey
reasoning, failure behavior, and visual hierarchy.

## HART review

| # | Finding | Severity | Resolution |
|---:|---|---|---|
| H1 | Matchup and roster state can be joined incorrectly without full league/team/season/type/week/time axes. | P1 | The assembly request and snapshot selector require every axis; display names are not keys. |
| H2 | Missing opponent, category totals, or status cannot share a zero value. | P2 | Typed unavailable/stale/partial states and recovery commands survive into v2. |
| H3 | Yahoo eligibility must remain fantasy context rather than canonical player position. | P2 | Provider fields stay at the fantasy join boundary and outside `StatsRepository`. |

## KEEL review

| # | Finding | Severity | Resolution |
|---:|---|---|---|
| K1 | CLI-private assembly plus TUI/Web subprocesses creates three runtime architectures. | P1 | One synchronous `icelines-fetch` service becomes the sole assembly owner. |
| K2 | Moving orchestration wholesale into core would violate the pure-engine boundary. | P1 | Fetch owns local I/O; core owns only pure decision composition; renderers only project. |
| K3 | The migration must remain bisectable and build-green. | P2 | Nine ordered slices freeze v1 before extracting, migrating, and removing adapters. |

## TAPE review

| # | Finding | Severity | Resolution |
|---:|---|---|---|
| T1 | A combined recommendation can conceal authority and age of each input. | P1 | Evidence rows retain source scope, capture time, freshness, and rejected snapshot reasons. |
| T2 | Platform injury/status observations are not verified NHL medical facts. | P2 | Provider context is labeled and uncertainty limits action firmness. |
| T3 | PUCK and real Yahoo payloads could leak into fixtures or logs. | P2 | Only synthetic/public fixtures are allowed; personal data remains in PUCK. |

## FORGE review

| # | Finding | Severity | Resolution |
|---:|---|---|---|
| F1 | `StatsRepository` and SQLite values are `!Send` hazards across TUI/Web task boundaries. | P1 | They are created and dropped inside one synchronous call; only owned ViewModels cross boundaries. |
| F2 | Core must not open storage, read clocks, or discover environment paths. | P2 | The request carries explicit clock/context and fetch performs local I/O. |
| F3 | Stringly errors would force each surface to invent recovery behavior. | P2 | Typed assembly errors and recovery commands are contract requirements. |

## PACE review

| # | Finding | Severity | Resolution |
|---:|---|---|---|
| P1 | Reintroducing the global pickup search would destroy daily latency. | P1 | Default candidate work is bounded/cached and measured; deep search stays explicit. |
| P2 | A numeric cap chosen without evidence could be either slow or strategically weak. | P2 | Pulse 0 records baselines before committing the candidate/time budget. |
| P3 | Repeated repository and schedule loads waste most of the proposed consolidation. | P2 | The service loads each once per assembly and integration tests count them. |

## BENCH review

| # | Finding | Severity | Resolution |
|---:|---|---|---|
| B1 | A read-only claim needs database, sidecar, network, and subprocess evidence. | P2 | L2 proves byte equality, no sidecars, no network, and zero child processes. |
| B2 | Contract evolution can silently break current API consumers. | P2 | Independent v1/v2 goldens and a stable compatibility projection are required. |
| B3 | Surface screenshots alone cannot prove shared decisions. | P2 | One sealed fixture asserts semantic parity across CLI, TUI, Web, and JSON. |

## EDGE review

| # | Finding | Severity | Resolution |
|---:|---|---|---|
| E1 | A numerically good add/start can be illegal after lock, on waivers, or after acquisition exhaustion. | P1 | Only legal-at-evaluation actions may be primary; conditional choices remain labeled alternatives. |
| E2 | Future, wrong-week, cross-team, and partial snapshots are realistic failure modes. | P2 | Selector rejects and reports each case with deterministic fallback. |
| E3 | DST, no opponent, stale status, and already-met goalie minimum alter urgency. | P2 | Explicit clock/timezone semantics and boundary fixtures cover each case. |

## WIRE review

| # | Finding | Severity | Resolution |
|---:|---|---|---|
| W1 | Changing primary-action meaning under `fantasy_today.v1` would violate the published contract. | P1 | League-aware semantics ship as v2 while v1 remains a compatibility projection. |
| W2 | Reads must not silently refresh Yahoo or record a journal entry. | P2 | GET/default commands are cached-only and immutable; sync and journal are explicit operations. |
| W3 | Partial data must be machine-readable, not only prose. | P2 | Typed state, reason, freshness, and recovery fields remain in JSON. |

## SCOUT review

| # | Finding | Severity | Resolution |
|---:|---|---|---|
| S1 | Season totals cannot identify starts actually usable against this week's opponent. | P2 | Quiet-night usable starts, collisions, roster fit, and matchup impact drive the recommendation. |
| S2 | Uncertain goalie or injury evidence cannot support a firm move. | P2 | Evidence confidence constrains firmness and may promote refresh/recovery first. |
| S3 | Managers need alternatives when the first player is claimed or the move is too expensive. | P2 | The contract carries ordered legal alternatives and acquisition/waiver cost. |

## GLASS review

| # | Finding | Severity | Resolution |
|---:|---|---|---|
| G1 | A league-aware screen can become a dense dashboard instead of a decision tool. | P2 | Context, one action, deadline, and firmness precede matchup and evidence detail. |
| G2 | Refresh must be visible and predictable in a long-lived TUI. | P2 | Entry and `r` refresh owned state; freshness and errors remain on screen. |
| G3 | Narrow terminals and no-color use must preserve meaning. | P2 | 80/120-column and no-color interaction gates are explicit. |

## CREST review

| # | Finding | Severity | Resolution |
|---:|---|---|---|
| C1 | Equal-weight cards would obscure the move that matters before lock. | P2 | One primary decision and deadline own the hierarchy. |
| C2 | Empty, stale, partial, and unsupported-category states need deliberate layouts. | P2 | Each is a designed state with recovery, not a blank panel. |
| C3 | Sentimental Rangers/Kraken weighting would make the reusable surface dishonest. | P3 | IceLines stays neutral; PUCK may personalize alternatives downstream. |

## broadcast review

| # | Finding | Severity | Resolution |
|---:|---|---|---|
| BRC1 | An Axum handler cannot casually carry `!Send` repository state across an await. | P1 | A compile-tested blocking/local boundary contains all local state and returns an owned view. |
| BRC2 | Bookmarkable v1/v2 routes need explicit compatibility behavior. | P2 | `/api/v1` remains stable; `/api/v2` and HTML use v2 with visible context. |
| BRC3 | Mobile, keyboard, screen-reader, no-JS, loading, and error behavior are release behavior. | P2 | Surface convergence includes all states as acceptance gates. |

## Initial synthesis

```text
Roles reviewed: 12
P1 blockers: 9  |  P2 issues: 26  |  P3 notes: 1

Verdict: NEEDS-WORK
```

The roles agree that the value is orchestration: one legal, evidence-backed
answer from already implemented contracts. The largest risks are divergent
surface adapters, unsafe `!Send` boundaries, incoherent matchup selection,
unbounded pickup latency, and changing a published schema without a version.

## Required amendments

1. **Architecture and contract** — establish one in-process assembly owner,
   pure-core composition, full identity axes, safe concurrency, and explicit
   v1/v2 compatibility.
2. **Decision trust** — require coherent snapshot provenance, legal-at-time
   actions, separate scoring modes, bounded candidate work, and no implicit
   sync or journal mutation.
3. **Operator proof** — replace permanent TUI caching and subprocess adapters;
   require parity, degraded-state, accessibility, performance, and immutable
   read evidence.

## Post-amendment verification

All three amendments are now incorporated into the plan's decisions, delivery
slices, tests, acceptance gates, and amendment log.

```text
Roles reviewed: 12
Remaining P1 blockers: 0

Final verdict: APPROVED-WITH-CONDITIONS
```

Conditions: Pulse 0 must measure before fixing a candidate cap; the Web slice
must compile-prove its `!Send` boundary; categories remain unavailable until
real category components exist; and default reads may not write history or
invoke provider/network refresh.
