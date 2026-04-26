# IceLines Review Roles

Eight perspectives on NHL analytics and lineup visualization, named after hockey and rink concepts.
Each role has a pointed view and pulls against at least one other.

## The Eight Roles

```
SCOUT    Hockey Domain Expert          ─── Line assignments, positional fit, hockey sense
TAPE     Data Accuracy Analyst         ─── CSV fidelity, GP matching, player-team currency
FORGE    Rust Engineer                 ─── Ownership model, error handling, crate boundaries
EDGE     Edge Case Specialist          ─── GP=0, mid-season trades, accented names, API failures
BENCH    Test Engineer                 ─── Scoring rules tested, fit thresholds property-tested
GLASS    Visualization & UX Critic     ─── Color semantics, lineup card scannability, terminal clarity
PACE     Methodology Statistician      ─── PPG formula, MIN_GP threshold, fit boundaries principled
WIRE     API & Data Pipeline           ─── Schema validation, cache-first, partial data, CSV drift
```

## Tiebreaker Ranking

When roles conflict, earlier roles govern:

1. **TAPE**   — bad source data invalidates every downstream result
2. **FORGE**  — unsound Rust is a correctness liability before features matter
3. **PACE**   — an undocumented assumption in the scoring formula propagates silently
4. **BENCH**  — if we can't verify the algorithm, we can't trust the output
5. **EDGE**   — a failure mode that isn't enumerated will eventually fire in production
6. **WIRE**   — an API integration without graceful degradation fails at the worst time
7. **SCOUT**  — hockey sense is the final reasonableness check, not the foundation
8. **GLASS**  — visualization quality matters, but only after correctness

## Core Tensions

| Pulls           | Against | Because |
|-----------------|---------|---------|
| FORGE           | GLASS   | soundness before features — GLASS wants new columns, FORGE wants compile-time guarantees |
| PACE            | SCOUT   | statistical rigour vs. hockey intuition — a threshold derived from data can conflict with what scouts know |
| SCOUT           | PACE    | raw hockey context (line chemistry, deployment) can make a pace-adjusted number misleading |
| GLASS           | PACE    | methodology wants every assumption explicit; GLASS wants the card clean and readable |
| EDGE            | WIRE    | EDGE finds failure modes; WIRE decides whether to degrade gracefully or error hard |
| TAPE            | everyone | every role depends on the data being right |
| BENCH           | FORGE   | BENCH wants test coverage; FORGE wants the test not to use `unwrap` |

## Usage

Invoke any role by name when reviewing code, data pipeline outputs, scoring results, lineup cards,
or spec decisions. Each role file contains its orientation, lens questions, expertise domains, and
tensions.

**When to invoke a specific role:**

- **SCOUT** — any time a line assignment or positional fit classification is questioned. "Does it make hockey sense?"
- **TAPE** — any time player data is ingested. "Does this CSV row match the current roster?"
- **FORGE** — any time Rust code is written or reviewed. "Is the ownership model clean? Is every `?` in the right place?"
- **EDGE** — after any feature is implemented. "What new failure modes did we just introduce?"
- **BENCH** — before any merge. "What test would catch a regression in this scoring rule?"
- **GLASS** — any time a lineup card, terminal table, or site page is designed or changed.
- **PACE** — any time a formula, threshold, or classification boundary is defined or changed.
- **WIRE** — any time the NHL API client or CSV loader is touched. "What happens when the network is down?"

## Role System Notes

Roles are not personas — they are lenses. You can invoke multiple roles on the same question and
let the tensions surface real design tradeoffs. FORGE and GLASS arguing about a new player-card
field is not a bug in the role system; it is the role system working. The tiebreaker ranking
resolves the impasse when it cannot be dissolved.

The pitfalls collection in `design/pitfalls/` is EDGE's institutional memory. It grows every
session and never shrinks.
