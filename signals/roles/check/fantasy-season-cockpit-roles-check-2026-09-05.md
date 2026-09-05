---
skill: roles-check
topic: fantasy-season-cockpit
date: 2026-09-05
roles_used: 12
initial_p1_count: 5
p1_count: 0
verdict: APPROVED-WITH-CONDITIONS
---

# Fantasy Season Cockpit — Roles Check

## Artifact identification

- **Artifact**: `design/plans/2026-09-05-fantasy-season-cockpit.md`
- **Type**: product, architecture, interaction, and implementation plan
- **Domain signals**: fantasy decisions, contract composition, source freshness,
  local persistence, CLI/TUI/Web parity, accessibility, and latency

## Role selection

All twelve roles are selected because this plan introduces a cross-surface
decision contract that touches domain state, hockey reasoning, performance,
data authority, API shape, tests, and interaction design.

## HART review

| # | Finding | Severity | Resolution |
|---:|---|---|---|
| H1 | A “today” view without league, team, season/type, local date/week, and evaluation instant can join incompatible state. | P1 | Complete identity and time axes are mandatory in the contract. |
| H2 | Missing matchup/status data must not share representation with a real zero or healthy state. | P2 | Optional sections plus typed readiness replace sentinel values. |
| H3 | Provider eligibility must not overwrite canonical NHL position or enter `StatsRepository`. | P2 | Platform context remains at the fantasy join boundary. |

## KEEL review

| # | Finding | Severity | Resolution |
|---:|---|---|---|
| K1 | Reimplementing morning, goalie, or pickup logic in the cockpit would create a second architecture. | P1 | `fantasy_today.v1` composes existing child ViewModels through one pure core builder. |
| K2 | Surface work could diverge if CLI text lands before a complete contract fixture. | P2 | Contract/fixture is the first build-green slice; renderers follow it. |
| K3 | Commit boundaries need to preserve a green workspace. | P3 | Eight independently compiling delivery slices are specified. |

## TAPE review

| # | Finding | Severity | Resolution |
|---:|---|---|---|
| T1 | A unified recommendation can obscure which source is authoritative for each claim. | P1 | Evidence rows name source family, authority scope, timestamps, freshness, and recovery. |
| T2 | Yahoo status is platform context, not verified NHL medical truth. | P2 | Injury/goalie observations retain source and confidence; stale/missing resolves unknown. |
| T3 | Private league payloads could leak into fixtures or logs. | P2 | Raw provider payloads, credentials, and team-specific fixtures are barred from source control/logs. |

## FORGE review

| # | Finding | Severity | Resolution |
|---:|---|---|---|
| F1 | The core builder must not open SQLite, read clocks, or acquire network data. | P1 | `FantasyTodayInput` carries assembled views and explicit instants; core remains pure. |
| F2 | Copying `run_morning` assembly into `run_today` would deepen an already large command module. | P2 | The CLI slice must extract and reuse common morning assembly. |
| F3 | Readiness and action states need exhaustive enums rather than string conventions. | P2 | Typed state, firmness, reason codes, and action kinds are contract requirements. |

## PACE review

| # | Finding | Severity | Resolution |
|---:|---|---|---|
| P1 | The observed near-100-second pickup path makes an unbounded default cockpit unusable. | P1 | Deep search is excluded from the default until bounded/measured; drill-down remains explicit. |
| P2 | Proposed latency numbers would be false claims without fixture and machine context. | P2 | They are labeled targets; Pulse 0 records cold/warm p50/p95 and request counts. |
| P3 | Schedule edge could dominate roster quality without disclosure. | P2 | Existing capped child scoring is reused and quiet-night evidence remains a separate component. |

## BENCH review

| # | Finding | Severity | Resolution |
|---:|---|---|---|
| B1 | A read-only cockpit needs proof that invoking it does not alter FantasyDb. | P2 | L2 asserts database equivalence before and after default/JSON runs. |
| B2 | Text-only tests cannot prevent JSON/surface semantic drift. | P2 | Byte-stable contract fixture plus cross-surface parity assertions are required. |
| B3 | Happy-path fixtures miss the cases that make daily advice dangerous. | P2 | Matrix covers locks, waivers, DST, ambiguity, GP=0, stale status, goalie minimums, and no legal action. |

## EDGE review

| # | Finding | Severity | Resolution |
|---:|---|---|---|
| E1 | A waiver delay or game lock can make a numerically attractive move illegal. | P2 | Primary actions require established legality at the evaluated instant. |
| E2 | No opponent/snapshot is not a 0-0 matchup, and no schedule is not an off night. | P2 | Both produce explicit unavailable/provisional states and recovery. |
| E3 | Future timestamps, DST transitions, exhausted moves, and already-met goalie minimums change urgency. | P2 | Explicit boundary fixtures and evaluated-at semantics are required. |

## WIRE review

| # | Finding | Severity | Resolution |
|---:|---|---|---|
| W1 | Consumers need a versioned envelope, not inference from optional fields. | P2 | The schema is fixed as `fantasy_today.v1` with typed readiness. |
| W2 | A partial API response must disclose unavailable sections and remediation. | P2 | Evidence/readiness rows survive JSON and surface projection. |
| W3 | Read routes must not hide fetch or mutation side effects. | P2 | CLI/Web cockpit reads are side-effect free; future refresh controls are separate operations. |

## SCOUT review

| # | Finding | Severity | Resolution |
|---:|---|---|---|
| S1 | Season-total value alone cannot answer a weekly lineup or quiet-night decision. | P2 | Legal daily assignments, bench collisions, and exact schedule remain the decision basis. |
| S2 | An estimated starter or uncertain injury cannot support firm advice. | P2 | Firmness is explicit and refresh-required actions precede conditional hockey moves. |
| S3 | The manager needs alternatives when the top waiver player is claimed or one move remains. | P2 | The primary decision retains legal alternatives and acquisition constraints. |

## GLASS review

| # | Finding | Severity | Resolution |
|---:|---|---|---|
| G1 | Combining every subsystem risks an unreadable status dump. | P2 | Text uses context → primary decision → checkpoints → context → alternatives. |
| G2 | Dense fantasy output must remain usable at 80 columns and without color. | P2 | 80-column/no-color L2 fences are acceptance requirements. |
| G3 | Users must always see which league, team, date, week, and freshness they are acting on. | P2 | Context is the first rendered region on every surface. |

## CREST review

| # | Finding | Severity | Resolution |
|---:|---|---|---|
| C1 | The cockpit needs one unmistakable decision rather than equal-weight panels. | P2 | One primary decision and deadline own the visual hierarchy. |
| C2 | Empty, stale, provisional, and blocked states require deliberate composition. | P2 | They are named product states with recovery copy, not blank panels. |
| C3 | Personal team branding or sentimental picks would weaken the reusable product language. | P3 | Personal preferences remain in PUCK; IceLines stays league-neutral. |

## broadcast review

| # | Finding | Severity | Resolution |
|---:|---|---|---|
| BRC1 | The Web cockpit must be bookmarkable and retain active context. | P2 | `/fantasy/today` and `/api/v1/fantasy/today` carry visible context. |
| BRC2 | Mobile, keyboard, screen-reader, and no-JavaScript behavior cannot be deferred as polish. | P2 | They are explicit Web-slice gates. |
| BRC3 | Loading and error states need recovery without turning a GET into mutation. | P2 | Read degradation is rendered; refresh/import actions remain explicit commands. |

## Initial synthesis

```text
Roles reviewed: 12
P1 blockers: 5  |  P2 issues: 29  |  P3 notes: 2

Verdict: NEEDS-WORK
```

Cross-role consensus: the valuable product is not a new scoring model. It is a
fast, honest composition layer over proven contracts. The default path must not
inherit the current unbounded pickup latency, and missing evidence must remain
visible all the way through JSON and every renderer.

## Required amendments

1. **Architectural convergence** — make the cockpit a pure core orchestration
   envelope over existing ViewModels, extract shared CLI assembly, and establish
   a golden contract before surface code.
2. **Decision trust** — include complete identity/time context, typed
   readiness/firmness, per-source authority/freshness, recovery actions,
   alternatives, and explicit no-mutation proof.
3. **Operator quality** — keep deep pickup search off the default path until
   bounded and measured; lock summary-first 80-column, accessible, responsive,
   bookmarkable surface behavior.

## Post-amendment verification

The plan was amended in place and now contains all three required amendments,
their associated test gates, and an amendment log.

```text
Roles reviewed: 12
Remaining P1 blockers: 0

Final verdict: APPROVED-WITH-CONDITIONS
```

Conditions: the first slice must prove composition and read-only behavior
without duplicating `fantasy_morning_briefing.v3`; the default CLI path may not
ship with unbounded candidate search. TUI/Web implementation follows the stable
contract fixture and must preserve the same decision semantics.

## Implementation re-review — 2026-09-05

The implemented vertical slice satisfies the release conditions:

- the pure `fantasy_today.v1` builder composes the existing daily, injury,
  matchup, goalie, and bench-coverage contracts without opening storage or
  reading the clock;
- the CLI reuses morning assembly, excludes deep pickup/sleeper searches from
  the default path, reads the schedule cache only, and opens FantasyDb through
  an immutable read-only connection;
- a stable decision golden and focused assertions cover ordering, fingerprints,
  missing-not-zero semantics, points/categories separation, quiet nights, and
  deadline choice;
- CLI, TUI, and Web consume the exact serialized contract, and Web missing-state
  tests prove that GET requests create no user database or SQLite sidecars;
- text is fenced at 80 columns, TUI is exercised at 80/120 columns, and Web is
  semantic, responsive, keyboard-readable, and independent of JavaScript;
- the measured warm release p95 is 239.7 ms, with the database hash unchanged.

Validation evidence: `cargo fmt --all -- --check`, strict workspace Clippy,
`cargo test --workspace -j 1`, the repository role checker, `git diff --check`,
and byte equality between the fantasy guide source and generated document all
pass. The unconstrained parallel test build exceeded local Windows compiler
memory; the identical suite passed serially without a code or assertion failure.

Two non-blocking follow-ups remain visible as P2 architectural work: replace
the local-process TUI/Web adapter with a shared local assembler, and enrich the
default cockpit from a saved matchup strategy when that input exists. Missing
matchup input is currently typed `unavailable`/`provisional`, never zero-filled.

```text
Roles re-reviewed: 12
Remaining P1 blockers: 0
Residual P2 follow-ups: 2

Implementation verdict: APPROVED-FOR-PARTIAL-SURFACE
```
