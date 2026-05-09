# IceLines Review Roles

Eleven core perspectives on the IceLines NHL analytics + fantasy app, named
after hockey and rink concepts. Each role has a pointed view and pulls against
at least one other. Additional specialist roles may exist for one surface or
delivery channel.

## Core Roles

```
HART     Domain Model Invariant Keeper ─── Post-Hart canonical shape, primary key axis
KEEL     System Architecture Coherence ─── 4-surface convergence, persistence chain
TAPE     Data Accuracy Analyst         ─── NHL API fidelity, identity flow, source-of-truth
FORGE    Rust Engineer                 ─── Ownership model, error handling, crate boundaries
PACE     Methodology Statistician      ─── Pace formulas, complexity claims, render budget
BENCH    Test Engineer                 ─── L0/L1/L2 coverage, fixture discipline, golden snapshots
EDGE     Edge Case Specialist          ─── GP=0, mid-season trades, accents, !Send boundaries
WIRE     API & Schema Pipeline         ─── Schema validation, version evolution, ESPN backup
SCOUT    Hockey Domain Expert          ─── Line assignments, positional fit, hockey sense
GLASS    Visualization & UX Critic     ─── Color semantics, terminal clarity, ratatui rendering
```

Additional core role added after the original ten:

```
CREST    Visual Design Aesthetic       --- Product taste, composition, polish, identity
```

## Specialist Roles

```
broadcast Web-view Perspective         --- Browser UX, HTMX, accessibility, sticky URLs
```

## Tiebreaker Ranking

When roles conflict, earlier roles govern:

1. **HART**   — domain model rules everything; if the canonical shape is wrong, every other claim is undefined
2. **KEEL**   — system architecture must converge; mismatched surfaces produce silent wrong-output
3. **TAPE**   — bad source data invalidates every downstream result
4. **FORGE**  — unsound Rust is a correctness liability before features matter
5. **PACE**   — an undocumented assumption in the scoring formula or complexity claim propagates silently
6. **BENCH**  — if we can't verify the algorithm, we can't trust the output
7. **EDGE**   — a failure mode that isn't enumerated will eventually fire in production
8. **WIRE**   — an API integration without graceful degradation fails at the worst time
9. **SCOUT**  — hockey sense is the final reasonableness check, not the foundation
10. **GLASS** — visualization quality matters, but only after correctness

11. **CREST** --- visual taste matters after correctness, convergence, and readability

## Core Tensions

| Pulls           | Against | Because |
|-----------------|---------|---------|
| HART            | TAPE    | TAPE asks "is this row right"; HART asks "does this row fit the model." Diverge on shape questions (cache key, season-type axis). |
| HART            | FORGE   | FORGE owns Rust marker traits (`!Send`); HART owns their domain rationale. They collaborate; HART decides why, FORGE decides how. |
| KEEL            | HART    | HART is the type view ("is the model coherent"); KEEL is the system view ("do all four surfaces and five sources agree"). Both must hold. |
| KEEL            | WIRE    | WIRE owns external API contracts; KEEL owns the convergence of those contracts across the 4 internal surfaces. |
| KEEL            | GLASS   | GLASS owns per-screen UX; KEEL owns cross-screen consistency. The depth chart in TUI must match the CLI must match the site. |
| FORGE           | GLASS   | soundness before features — GLASS wants new columns, FORGE wants compile-time guarantees |
| PACE            | SCOUT   | statistical rigour vs. hockey intuition — a threshold derived from data can conflict with what scouts know |
| SCOUT           | PACE    | raw hockey context (line chemistry, deployment) can make a pace-adjusted number misleading |
| GLASS           | PACE    | methodology wants every assumption explicit; GLASS wants the card clean and readable |
| CREST           | GLASS   | GLASS asks "is it readable and accessible"; CREST asks "does it look intentional and beautiful." |
| CREST           | KEEL    | KEEL wants cross-surface convergence; CREST wants related surfaces that still honor their medium. |
| EDGE            | WIRE    | EDGE finds failure modes; WIRE decides whether to degrade gracefully or error hard |
| TAPE            | everyone | every role depends on the data being right |
| BENCH           | FORGE   | BENCH wants test coverage; FORGE wants the test not to use `unwrap` |

## Usage

Invoke any role by name when reviewing code, data pipeline outputs, scoring
results, lineup cards, or spec decisions. Each role file contains its
orientation, lens questions, expertise domains, and tensions.

**When to invoke a specific role:**

- **HART** — any time the data model shape is touched. New `SeasonStats` field, new cache key, new view accessor, new (season, type)-coupled state. "Does this fit the canonical post-Hart shape?"
- **KEEL** — any time more than one surface is affected, any new persistence tier, any architecture diagram. "Do all four surfaces and five sources converge here?"
- **TAPE** — any time external NHL data is ingested or transformed. "Does this row identity flow correctly through the loader?"
- **FORGE** — any time Rust code is written or reviewed. "Is the ownership model clean? Is every `?` in the right place?"
- **PACE** — any time a formula, complexity claim, threshold, or render-budget assertion is made. "Is this number actually measured, or estimated?"
- **BENCH** — before any merge. "What test would catch a regression here?"
- **EDGE** — after any feature is implemented. "What new failure modes did we just introduce?"
- **WIRE** — any time an external API client or schema is touched. "What happens when the network is down? When the schema drifts?"
- **SCOUT** — any time a line assignment, fit class, or hockey-domain claim is questioned. "Does it make hockey sense?"
- **GLASS** — any time a screen, terminal table, or site page is designed. "Is the layout scannable, the color contract honored?"

- **CREST** --- any time visual polish, product identity, screenshot quality, or aesthetic direction is at stake. "Does this feel intentionally designed and worth using?"
- **broadcast** --- any time web HTML, HTMX fragments, browser behavior, or bookmarkable page state is touched. "Does this work for a user opening the browser cold?"

## Role System Notes

Roles are not personas — they are lenses. You can invoke multiple roles on the
same question and let the tensions surface real design tradeoffs. FORGE and
GLASS arguing about a new player-card field is not a bug in the role system; it
is the role system working. The tiebreaker ranking resolves the impasse when it
cannot be dissolved.

The pitfalls collection in `design/PITFALLS.md` is EDGE's institutional memory.
It grows every session and never shrinks.

The architecture spec at `design/ARCHITECTURE.md` and the app plan at
`design/IceLines.md` are KEEL's institutional context. Reading them is a
prerequisite for any KEEL review.

## Adding a Role

A new role is justified when an entire class of defects is going uncaught.
HART and KEEL were added 2026-05-01 after second-round review on the v1.0
architecture surfaced 3 BLOCKERs that no existing role flagged on first
pass: a fictional "5-tier fallback chain" diagram (KEEL), a `schedule_team_cache`
keyed only on team while data was `(team, season)`-shaped (HART + KEEL),
and a build-green-invariant violation in the 3-commit migration plan (KEEL).
TAPE and FORGE caught these on second-round review, but the right framing was
post-Hart system architecture, which neither role explicitly owned.

Don't add a role for a single defect class — add it for a recurring lens that
the existing roles consistently miss. Spell out the tiebreaker placement and
tensions before adding.
