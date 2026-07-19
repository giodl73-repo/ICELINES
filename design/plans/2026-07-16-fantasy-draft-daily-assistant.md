# Fantasy Draft and Daily Assistant — Implementation Plan

**Date**: 2026-07-16  
**Status**: Active  
**Specification**: [`../specs/fantasy-draft-daily-assistant.md`](../specs/fantasy-draft-daily-assistant.md)

**Expansion roadmap**: [`2026-07-18-fantasy-war-room-roadmap.md`](2026-07-18-fantasy-war-room-roadmap.md)

## Objective

Deliver a league-specific assistant that progresses from draft-day roster
construction into daily lineup, injury, waiver, pickup, and sleeper decisions
without forking the existing scoring, schedule, FantasyDb, or poacher contracts.

## Entry State

- Active-league scoring schemes, user teams, rosters, Yahoo roster import,
  roster-shape validation, daily scoring, weekly matchups, and poacher views
  already exist.
- The current worktree adds the official 2026-27 schedule cache and
  `fantasy schedule-edge`: 1,344 deduplicated games, Monday-Sunday volume,
  quiet-slate value, exact-date overlap, equivalence classes, and roster
  calendar fit.
- Existing roster-shape validation understands multiple canonical positions but
  does not perform daily one-player/one-slot assignment.
- Platform eligibility, injury reserve, waiver timers, transaction budgets,
  game locks, and morning briefing state are not yet durable contracts.

## Delivery Principles

- Core owns optimization and typed ViewModels; renderers do not recompute.
- The active league's persisted scheme owns fantasy value.
- Platform eligibility never overwrites canonical NHL position.
- Missing evidence is state, not zero.
- Read-only Web routes remain non-mutating.
- Recommendations are advisory until an authenticated provider mutation phase
  is separately authorized.
- Each pulse is independently testable and leaves existing fantasy commands
  working.

## Progress Snapshot — 2026-07-18

The original entry-state gaps have now been substantially closed:

| Scope | State |
|---|---|
| Pulses 00–05 | Implemented: exact schedule edge/classes, saved league rules, platform eligibility, legal daily assignment, pasted taken pool, and roster-aware draft ranking |
| Pulses 06–10 | Implemented: acquisition/waiver ledger, one-move weekly optimizer, sourced status/IR planning, morning briefing, reserve policy, and sleeper discovery |
| Pulses 11–12 | Partial: canonical CLI/JSON and documentation exist; richer TUI/Web views, recurring operation, and configured notification delivery remain |
| Pulse 13 | Implemented: deterministic season/playoff stress simulation with injuries, recoveries, pickups, trades, waivers, reserve matrices, and playoff exits |
| Trade extension | Implemented: package evaluation/finder/readiness, exact roster sync, atomic execution/history, pending offers, lifecycle status, and stale-offer detection |

The next expansion is intentionally maintained in the linked Fantasy War Room
roadmap so this foundation plan remains auditable rather than continually
renumbering its completed pulses.

## Pulse Plan

| Pulse | Scope | Deliverable | Gate |
|---:|---|---|---|
| 00 | Schedule foundation | Finish and harden `fantasy_schedule.v1`, cache persistence, weekly leaders, equivalence classes, roster multiplicity | 1,344 live games; fixture tests; no incomplete cache activation |
| 01 | League-rule contract | Persist 2C/2LW/2RW/3D/1UTIL/2G/4BN/2IR/2IR+ plus timezone, four-add budget, two-day waivers | Migration preservation and round-trip tests |
| 02 | Platform eligibility | Import/store `C/LW`, `C/RW`, `LW/RW`, etc. separately from canonical position | Multi-position source/freshness and fallback tests |
| 03 | Daily assignment engine | Maximum-weight legal assignment with UTIL skater-only, locks, bench, IR, and IR+ | Matching invariants and deterministic golden fixtures |
| 04 | Draft board ingestion | stdin/clipboard/text/CSV taken pool, availability resolution, unresolved diagnostics | Parser fixtures; unresolved names never leak into available pool |
| 05 | Draft recommendation | Best overall/by-position ranking using scheme value, gaps, flexibility, scarcity, exact-date usable starts, and classes | Elite-vs-fit guardrails; explanation fixtures |
| 06 | Acquisition and waiver ledger | Add/drop/claim records, Monday-Sunday four-move budget, same-day effective time, two-day waiver clearance | Boundary/timezone/DST and budget tests |
| 07 | Weekly pickup optimizer | One-move marginal add/drop ranking, recomputed daily usable starts, optional bounded four-move sequence | Never exceeds budget; never counts locked/unusable starts |
| 08 | Injury and status state | Typed DTD/GTD/OUT/IR/LTIR observations, source/freshness, strict IR then IR+ placement | Stale/unknown/confirmed behavior fixtures |
| 09 | Morning briefing | 07:00 Pacific briefing plus on-demand/pregame refresh and conditional fallback plan | Material-change suppression and lock-safe actions |
| 10 | Secret-finds board | Emerging defense/role, injury replacement, category specialist, schedule unlock, goalie stream, breakout | Raddysh-style role-change fixture; one-game-spike risk fence |
| 11 | Surface integration | CLI and deterministic JSON first; then TUI/Web read views and report/export parity | Shared ViewModel parity; GET non-mutation tests |
| 12 | Operational automation | Document Windows scheduling/recurring runner and notification handoff without external mutations | Idempotent run, observable failures, no duplicate alerts |

## Detailed Work

### Pulse 00 — schedule foundation

- Complete current `fantasy schedule-edge` implementation.
- Preserve all 32 team feeds as one deduplicated season schedule.
- Validate team/game cardinality and official schedule date range.
- Keep quiet-slate threshold configurable.
- Retain exact pair overlap in JSON; classes remain explanatory.
- Add cached/offline and partial-fetch failure tests.

### Pulses 01–03 — legal roster engine

- Add a versioned league-rules value rather than hard-coding the user's rules
  into generic `yahoo-standard` behavior.
- Introduce platform eligibility persistence with provenance.
- Model active slot instances, unrestricted bench, strict IR, flexible IR+,
  player game locks, and roster overflow.
- Implement a small deterministic maximum-weight matching/DP solver. Prove:
  one player occupies at most one slot; a flexible player does not strand a
  scarcer slot when an equivalent assignment exists; goalies never occupy UTIL.
- Produce `FantasyRosterConstructionView` and `FantasyDailyLineupView`.

### Pulses 04–05 — draft loop

- Accept `--taken-file -` stdin, newline lists, and common CSV player columns.
- Accept platform eligibility from a labeled Yahoo/player-pool CSV or explicit
  override.
- Reconcile names through canonical identity; retain ambiguous/unresolved rows.
- Combine active scheme value, positional replacement level, open starter slots,
  bench room, multi-position option value, exact daily usable starts, quiet
  slates, schedule overlap, and risk.
- Render best overall plus C/LW/RW/D/G/flexible alternatives and a fallback pick.
- Add a dry-run hypothetical pick path; roster mutation remains explicit.

### Pulses 06–07 — in-season streaming

- Add migration-backed acquisition and waiver state.
- Compute fantasy-week boundaries in league timezone.
- Track used/remaining acquisitions and effective timestamps.
- Simulate one candidate add against every legal drop, using daily lineup
  optimization for the remaining week.
- Rank incremental playable starts and league-scored value, not games scheduled.
- Preserve the dropped player's future value and waiver reacquisition cost.
- Reserve one of four weekly acquisitions from ordinary streaming through Friday
  so a later injury can still receive a same-week replacement; release it
  Saturday if unused and keep the policy configurable.
- Permit an explicit exceptional-value override only for a verified healthy
  roster and a move worth at least +6.0 net value and +3.0 usable starts.
- Calibrate that override with paired all-in, strict-reserve, and adaptive-reserve
  whole-season runs that preserve every non-policy random domain.
- Add a bounded multi-move search only after one-move recommendations are stable.

### Pulses 08–10 — injuries, mornings, and sleepers

- Define verified status sources before enabling definitive injury claims.
- Add observation freshness and confidence.
- Optimize IR/IR+ placement before recommending a drop.
- Generate a morning baseline and conditional pregame fallbacks.
- Suppress alerts without a material decision change.
- Reuse PoachScore evidence/explanation patterns for emerging-role and
  emerging-defense recommendations; add exact usable-start and roster-need
  components.

### Pulses 11–12 — product surfaces and operations

- Ship canonical CLI/JSON first.
- Add TUI/Web read-only views after ViewModel contracts stabilize.
- Keep all external fantasy-platform moves as handoffs until authenticated,
  confirmed mutation contracts exist.
- Provide a repeatable 07:00 Pacific task-runner example and pregame refresh
  command. Do not claim push notification delivery without a configured channel.

### Pulse 13 — season stress simulation

- Seed a synthetic league under the saved scoring, roster, bench, IR/IR+, and
  weekly acquisition rules when imported opponent rosters are incomplete.
- Simulate exact-date daily assignments, schedule-driven pickups, injuries,
  replacements, recoveries, missed starts, fair-value trades, and roster churn.
- Emit deterministic Monte Carlo standings and a typed sample event ledger.
- Separate Monday-Sunday regular-season standings from a seeded weekly playoff
  bracket; for six qualifiers, the top two seeds receive first-round byes.
- Report mutually exclusive first-round, semifinal, final, and championship
  outcomes so regular-season dominance and single-week upset risk stay distinct.
- Add a same-seed clean/baseline/high-chaos matrix so injury/trade sensitivity
  is inspectable without three manually coordinated commands.
- Model superior pickup decisions through explicit opponent selection accuracy,
  never through a hidden scoring multiplier.
- Keep the run non-mutating and label it as a stress model, not a calibrated
  injury or championship forecast.
- Allow a marked or explicitly selected partial roster to seed team one; rotate
  its draft-seat filler across trials and lock imported players before the
  season begins.
- Use Dexter's Dawgs' authoritative 18-2, all-category-leading regular season
  followed by a first-round loss as the historical stress case. Treat the
  January workbook as an incomplete snapshot, not the final record or roster path.

## Validation Matrix

Reserve calibration on Dexter's Dawgs uses paired seed `20262027`. At 120
trials, all-in produced a 5.73 average seed and 63.3% playoff rate; strict
Friday reserve produced 5.67 and 65.8%; adaptive +6 value/+3 games produced
5.76 and 65.0%. This supports +6/+3 as a deliberately rare review threshold,
while strict reserve remains the safer default recommendation when evidence is
uncertain. Championship percentages remain too noisy to tune this policy.

| Area | Required proof |
|---|---|
| Assignment | L0 property/invariant tests and fixed multi-position fixtures |
| Persistence | In-memory SQLite migration/round-trip and existing-DB preservation |
| Schedule | Offline cache fixture plus live cardinality smoke kept outside deterministic CI |
| Time | Monday/Sunday, same-day lock, waiver +2 days, DST transitions, Pacific timezone |
| Transactions | Four-add hard fence, claim accounting configuration, drop waiver state |
| Injuries | confirmed/stale/unknown source-state fixtures; IR before IR+ preservation |
| Draft import | newline/CSV/stdin, duplicates, ambiguous names, unresolved safety |
| Scoring | built-in and custom active-league scheme fixtures; skater and goalie paths |
| Recommendations | deterministic ranking and explanation golden tests |
| Trades | one-for-one/package fixtures, goalie and multi-position legality, fair-offer search, schedule-class deltas, text/JSON parity |
| Surfaces | CLI/JSON parity, then TUI/Web shared-ViewModel tests |
| Safety | no external mutation, GET non-mutation, no missing-DB side effects |

## Milestones

1. **Draft-ready**: pulses 00–05. Taken-player paste, legal roster construction,
   and best-available recommendations work before draft day.
2. **Week-ready**: pulses 06–07. Acquisition budget, waivers, and daily usable
   starts drive pickup recommendations.
3. **Morning-ready**: pulses 08–10. Injury-aware briefing, fallbacks, and secret
   finds work with explicit freshness.
4. **Product-ready**: pulses 11–12. Read surfaces and repeatable morning operation
   are documented and validated.

## Exit Criteria

- The acceptance criteria in the linked spec pass.
- The active league's scoring scheme is used end-to-end.
- A pasted taken pool produces safe best-available draft recommendations.
- A multi-position roster is legally optimized each day.
- Weekly recommendations respect four acquisitions, same-day free agency,
  player locks, and two-day waivers.
- Injury recommendations preserve 2 IR and 2 IR+ semantics with freshness.
- Morning output includes legal fallback actions and does not overclaim status.
- Secret finds explain opportunity, league fit, usable starts, and risk.
