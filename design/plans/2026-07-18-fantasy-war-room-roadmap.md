# Fantasy War Room — Product Roadmap

**Date**: 2026-07-18  
**Status**: Active  
**Parent plan**: [`2026-07-16-fantasy-draft-daily-assistant.md`](2026-07-16-fantasy-draft-daily-assistant.md)  
**Specification**: [`../specs/fantasy-draft-daily-assistant.md`](../specs/fantasy-draft-daily-assistant.md)

## Objective

Turn the implemented fantasy assistant foundations into a live decision war
room spanning weekly matchups, draft execution, multi-move streaming, goalies,
trade negotiation, injury contingencies, playoff schedules, roster archetypes,
decision review, and data readiness.

The first outcome is not another season-total leaderboard. It is a weekly
matchup strategist designed to reduce single-week playoff upset risk after
Dexter's Dawgs' authoritative 18-2, all-category-leading regular season ended
in a first-round loss.

## Existing Foundation

The roadmap reuses these implemented contracts:

- active-league skater and goalie scoring;
- the configured 2 C / 2 LW / 2 RW / 3 D / 1 UTIL / 2 G / 4 bench roster;
- daily multi-position lineup assignment;
- 2 IR and 2 IR+ placement;
- four Monday-Sunday acquisitions, same-day free agents, and two-day waivers;
- exact 2026-27 schedules, quiet slates, overlap, and equivalence classes;
- draft taken-player paste and platform eligibility;
- one-move weekly pickups, morning briefing, and sleeper discovery;
- seeded season/playoff stress simulation;
- trade evaluation, finder, readiness, pending offers, and atomic history.

No workstream may fork those rules or recompute them in a renderer.

## Product and Safety Rules

1. The active league contract owns scoring, roster shape, timezone, locks,
   acquisitions, waivers, and matchup mode.
2. Points and categories are separate competition modes. IceLines must never
   infer category rules from point weights or historical prose.
3. Missing live evidence is explicit. Estimated goalies, injuries, lines, and
   roles are never labeled confirmed.
4. Recommendations remain advisory. External fantasy-platform mutations require
   a separately authorized authenticated provider contract.
5. Every recommendation exposes baseline, delta, constraints, uncertainty, and
   the next-best legal alternative.
6. Playoff decisions optimize weekly win probability and downside risk, not only
   full-season expected value.
7. Deterministic core ViewModels precede CLI, TUI, Web, notifications, or agents.

## Delivery Order

| Wave | Capability | Primary contract | Dependency | Exit gate |
|---:|---|---|---|---|
| 14 | Weekly matchup strategist | `fantasy_matchup_strategy.v1` | league matchup mode + opponent snapshot | probabilities reconcile; no impossible category totals |
| 15 | Playoff schedule portfolio | `fantasy_playoff_portfolio.v1` | schedule edge + matchup strategist | playoff-week usable starts and collision deltas proven |
| 16 | Live draft room | `fantasy_draft_session.v1` | draft board + roster assignment | every pasted pick advances one durable session safely |
| 17 | Multi-move pickup planner | `fantasy_pickup_sequence.v1` | waiver ledger + matchup priorities | bounded sequences obey four-move and lock constraints |
| 18 | Injury contingency tree | `fantasy_injury_contingency.v1` | status evidence + pickup sequences | every branch is legal at its effective time |
| 19 | Goalie command center | `fantasy_goalie_plan.v1` | goalie scoring + evidence freshness | confirmed/estimated starts remain distinguishable |
| 20 | Trade negotiator | `fantasy_trade_negotiation.v1` | saved offers + matchup/playoff needs | counters help both rosters and stale offers are fenced |
| 21 | Category scarcity and archetypes | `fantasy_player_archetypes.v1` | category contract + replacement levels | labels derive from disclosed metrics and thresholds |
| 22 | Decision journal and retrospective | `fantasy_decision_journal.v1` | stable decision fingerprints | projections and outcomes join without hindsight mutation |
| 23 | Data-readiness dashboard | `fantasy_readiness.v1` | freshness contracts from all waves | one surface identifies every blocker and recovery action |

Readiness fields are added incrementally in every wave; Wave 23 consolidates
them into the complete operator surface.

## Wave 14 — Weekly Matchup Strategist

### User workflow

```powershell
icelines fantasy matchup-plan --week 2027-03-22
icelines fantasy matchup-plan --week 2027-03-22 --opponent "Hockey Nerds"
icelines fantasy matchup-plan --week 2027-03-22 --strategy floor
icelines fantasy matchup-plan --week 2027-03-22 --strategy upside --json
```

### Scope

- Persist `competition_mode = points | categories` on the league.
- For category mode, persist the exact scored categories, direction
  (`higher_wins` or `lower_wins`), minimum goalie appearances, and tie policy.
- Accept an optional pasted current-week matchup snapshot; otherwise label the
  report as a pre-week projection.
- Project daily legal lineups for both teams over the remaining week.
- Report point-margin distribution or per-category win/tie/loss probability.
- Classify categories as safe, press, volatile, or low-return/punt candidates.
- Produce floor, balanced, and upside lineup/pickup strategies.
- Stress one injury, one missed goalie start, and one poor goalie performance.
- Keep probabilities simulation estimates with trial count and seed, never
  betting odds.

### MVP boundary

The first release uses completed-season player rates, saved rosters, exact
schedule, and optional pasted matchup-to-date totals. Live fantasy scoring and
confirmed starting goalies wait for verified source adapters.

### Acceptance gates

- Category directions and ties work for counting and ratio categories.
- A player can contribute only when legally assigned on that date.
- Current matchup totals are not projected a second time.
- Same seed and inputs produce byte-stable decision rows.
- Floor strategy cannot be labeled safer unless its modeled loss tail improves.

## Wave 15 — Playoff Schedule Portfolio

### User workflow

```powershell
icelines fantasy playoff-portfolio --rounds 3
icelines fantasy playoff-portfolio --team "Dexter's Dawgs" --json
```

### Scope

- Rank teams and players over the configured fantasy playoff weeks.
- Separate scheduled games, usable starts, quiet-slate starts, and bench
  collisions.
- Measure marginal playoff fit against the actual roster, not a generic team.
- Expose first-round, semifinal, and championship-week value separately.
- Compare regular-season rank with playoff rank and flag schedule-driven risers.
- Feed playoff deltas into draft, pickup, and trade explanations without
  silently replacing their base score.

### Acceptance gates

- Changing playoff dates changes only playoff-derived fields.
- Exact-date overlap drives collision; weekly game counts alone are insufficient.
- A four-game player with two bench collisions can rank below a three-usable-start
  player, with the reason visible.

## Wave 16 — Live Draft Room

### User workflow

```powershell
icelines fantasy draft-session start --name "2026 Main Draft"
Get-Clipboard | icelines fantasy draft-session update --taken-file -
icelines fantasy draft-session recommend --top 3
icelines fantasy draft-session pick "Player Name"
```

### Scope

- Persist session, round, pick, draft position, taken pool, roster, and undoable
  local event history.
- Make repeated pasted taken lists idempotent and report newly resolved,
  duplicate, ambiguous, and unresolved names.
- Recommend best value, safest roster fit, and highest-upside schedule complement.
- Show positional tier cliffs and estimated availability before the next pick.
- Track starter gaps, UTIL flexibility, bench composition, schedule classes,
  playoff fit, and third-goalie opportunity cost.
- Support a dry-run hypothetical pick and an explicit confirmed local pick.

### Acceptance gates

- A pasted full list and a sequence of incremental pastes produce the same state.
- Undo restores the prior session without altering the fantasy league roster.
- Recommendations never exceed roster capacity or create an unfillable active
  shape when a legal alternative exists.
- Tier-run urgency is evidence, not a claim about other managers' future picks.

## Wave 17 — Multi-Move Pickup Planner

### User workflow

```powershell
icelines fantasy pickup-plan --week 2026-11-09 --moves 4
icelines fantasy pickup-plan --strategy matchup --reserve 1 --json
```

### Scope

- Search bounded add/drop sequences across the remaining Monday-Sunday dates.
- Recompute daily assignments, locks, waivers, and opponent/category priorities
  after every hypothetical move.
- Permit planned stream-and-drop sequences only after the player's final useful
  start and before the next add's lock.
- Price retained player value, waiver reacquisition risk, and move-budget cost.
- Preserve the configurable injury reserve and exceptional override policies.
- Return the best complete sequence plus robust alternatives if a player is
  claimed or a status changes.

### Acceptance gates

- No sequence exceeds remaining moves or uses a player while on waivers.
- Intermediate rosters remain legal; final-only legality is insufficient.
- Sequence value equals the sum of recomputed usable marginal starts/value, not
  independent one-move scores.
- The planner can recommend fewer than four moves when additional churn hurts.

## Wave 18 — Injury Contingency Tree

### User workflow

```powershell
icelines fantasy injury-tree --date 2026-11-12
icelines fantasy injury-tree "Player Name" --at 2026-11-12T17:30:00-08:00
```

### Scope

- Build explicit healthy, active, ruled-out, and unresolved branches.
- Assign IR before IR+, then choose legal replacement and fallback replacements.
- Include pickup-budget, waiver, game-lock, and later-week recovery consequences.
- Identify the last safe decision time for each branch.
- Attach evidence source, confidence, observed time, and refresh instruction.
- Embed the highest-priority branches into `fantasy morning`.

### Acceptance gates

- No uncertain status becomes a definitive drop or injury claim.
- Every branch is executable at its stated time under locks and move budget.
- A fallback player already claimed is replaced by the next legal alternative.
- Recovery never releases the wrong substitute after intervening roster moves.

## Wave 19 — Goalie Command Center

Status: evidence, matchup/rest adjustment, and stream portfolio foundation
implemented. Game-specific starter observations persist with provenance and
freshness; the weekly plan enforces goalie-only daily capacity, separates
expected starts from the confirmed minimum floor, exposes poor-start downside,
discounts unsourced back-to-back workload, indexes opponent offense, ranks
legal marginal streams, and compares the best conditional third goalie with
the current group. Verified opponent shot quality, deeper rest/workload feeds,
and automatic platform execution remain.

Morning integration is implemented: the 07:00 briefing carries the typed goalie
plan, suppresses generic schedule-only goalie starts, emits firm starts only for
fresh confirmed evidence, keeps reported/estimated states conditional, and
offers a legal stream plus fallback when coverage warrants it. Re-running with
`--at` provides the same-day evidence refresh path before lock.
Atomic CSV/stdin starter import and lock-aware urgency are also implemented.
Batch evidence rejects partial writes and duplicate player/date rows; planning
escalates from check-later to refresh-soon to refresh-now, and removes locked
games from remaining-start and stream value.
The morning v3 contract separates wall-clock generation from decision time,
publishes a compact next-refresh/next-lock checkpoint, and prioritizes confirmed
same-day streams while retaining the best unconfirmed option as a conditional
fallback.
Confirmed starters and backups now receive a final T−30 safety checkpoint, with
an explicit verify-now morning action once the window opens so late reversals
can supersede the earlier recommendation before lock.
Goalie streams now share transaction intent with weekly pickups: a final
proactive move produces choose-one alternatives, while an identical candidate
is deduplicated into one action carrying the optimizer's drop/value evidence.
Primary and fallback streams independently attach their legal weekly-optimizer
drop/value pairing. An unpaired stream remains capacity-gated until an open
roster spot is verified.

### User workflow

```powershell
icelines fantasy goalie-plan --week 2026-11-09
icelines fantasy goalie-plan --date 2026-11-12 --strategy floor
```

### Scope

- Combine league goalie weights/categories, schedule, opponent, workload, rest,
  and saved start evidence.
- Distinguish confirmed, reported, estimated, and unknown starter state.
- Model back-to-back allocation as a probability unless sourced confirmation exists.
- Respect minimum goalie appearances and show the risk of falling short.
- Compare start, bench, wait, stream, and third-goalie choices.
- Stress a poor start so ratio-category downside is visible.

### Acceptance gates

- A probable starter is never rendered as confirmed.
- Points and ratio-category decisions use their correct objective functions.
- Goalie starts cannot occupy UTIL or duplicate one goalie on the same date.
- Missing starter evidence produces a refresh action, not fabricated certainty.

## Wave 20 — Trade Negotiator

### User workflow

```powershell
icelines fantasy trade-recheck OFFER_ID
icelines fantasy trade-counter OFFER_ID --top 10
icelines fantasy trade-counter OFFER_ID --strategy playoff --json
```

### Scope

- Revalue saved offers using current rosters, injuries, remaining schedule,
  matchup needs, and playoff fit.
- Compare accept, reject, counter, and no-trade-plus-streaming baselines.
- Generate counters that improve user fit while remaining legal and non-negative
  for the counterpart's active-lineup fit.
- Explain each manager's positional/category need and schedule effect.
- Add buy-low/sell-high evidence using recent versus established rates with
  sample-size risk.
- Keep acceptance likelihood qualitative until actual league offer outcomes
  provide enough calibration data.

### Acceptance gates

- Stale saved packages cannot be recommended unchanged.
- Protected anchors remain excluded unless explicitly overridden.
- Counteroffers never worsen positional legality for either roster.
- Accepted status alone never mutates local or external rosters.

## Wave 21 — Category Scarcity and Player Archetypes

### User workflow

```powershell
icelines fantasy archetypes --top 50
icelines fantasy archetypes "Player Name" --json
icelines fantasy roster-balance --team "Dexter's Dawgs"
```

### Scope

- Produce non-exclusive archetypes: elite scorer, peripheral contributor,
  power-play specialist, blocks/hits defenseman, high-floor goalie, volatile
  upside goalie, schedule streamer, and multi-position glue player.
- Compute category scarcity and replacement level by platform eligibility.
- Report the player's marginal effect on the selected roster's balance.
- Keep raw league value, scarcity, archetype strength, flexibility, and schedule
  fit as separate disclosed components.

### Acceptance gates

- Archetypes derive from versioned thresholds and source-backed metrics.
- Multi-position eligibility changes flexibility, not canonical NHL position.
- Small samples widen uncertainty and cannot create an elite label alone.
- Aggregate scoring never hides a zero-category or ratio-risk weakness.

## Wave 22 — Decision Journal and Retrospective

### User workflow

```powershell
icelines fantasy decision-record --kind pickup --recommendation-id ID --chosen 1
icelines fantasy decision-review --week 2026-11-09
icelines fantasy decision-review --season 20262027 --json
```

### Scope

- Persist the recommendation fingerprint, alternatives, evidence timestamps,
  projected deltas, chosen action, and optional manager rationale.
- Later attach actual usable starts, league value, matchup/category result, and
  whether the held acquisition reserve was needed.
- Compare projection error by decision type without rewriting original evidence.
- Calibrate thresholds only from sufficiently comparable decisions.
- Keep “good process, bad outcome” distinct from a poor recommendation.

### Acceptance gates

- Original recommendation inputs and projections are immutable.
- Outcomes may be appended or corrected with an audit trail.
- Review separates decision quality, projection error, and random outcome.
- No private rationale appears in public exports unless explicitly requested.

## Wave 23 — Data-Readiness Dashboard

### User workflow

```powershell
icelines fantasy readiness
icelines fantasy readiness --workflow matchup
icelines fantasy readiness --json
```

### Scope

- Consolidate league rules, roster sync, eligibility, schedule, scoring,
  matchup snapshot, status freshness, goalie evidence, pickup budget, waiver
  state, and saved-offer validity.
- Grade each workflow `ready`, `provisional`, or `blocked`.
- Name the exact recovery command for every non-ready input.
- Include source/fetched/observed timestamps and decision-relevant age limits.
- Never fetch or mutate as a side effect of a readiness read.

### Acceptance gates

- Every war-room workflow declares its required and optional evidence.
- A missing optional source lowers confidence without becoming fabricated zero.
- A blocked workflow names at least one actionable recovery step.
- Text and `fantasy_readiness.v1` JSON agree on status and blockers.

## Cross-Cutting Architecture

### Persisted additions

- competition mode and category rules;
- optional matchup-to-date snapshots with source timestamps;
- draft sessions and local session events;
- decision journal and append-only outcomes;
- goalie/start observations through the existing evidence pattern.

Every migration requires in-memory round-trip, existing-database preservation,
foreign-key behavior, and failure rollback tests.

### Core optimization

- Daily assignment remains the only lineup legality engine.
- Matchup simulations use deterministic domain-separated randomness.
- Multi-move pickup search is bounded by remaining days, moves, candidate pool,
  and beam width; it must disclose truncation.
- Category ratios are modeled from numerator/denominator components rather than
  averaging percentages.
- Playoff, archetype, and readiness values are inputs to recommendations, never
  hidden score multipliers.

### Source authority

| Evidence | Initial authority | Claim boundary |
|---|---|---|
| NHL schedule/results | official cached NHL feeds | game/date/result only |
| Player rates | selected completed or current sealed stats sample | projection input, not guaranteed outcome |
| Fantasy rosters/eligibility | exact Yahoo CSV/clipboard sync | current only as of import time |
| Matchup-to-date totals | labeled user/platform paste | no live continuity claim |
| Injury/goalie/role state | saved sourced observations | confidence and freshness required |
| Other-manager behavior | saved league events when available | no calibrated probability before sample gate |

## Validation Matrix

| Area | Required proof |
|---|---|
| Matchups | points/category fixtures, ratio math, ties, current-total joins, deterministic simulation |
| Playoffs | configured week boundaries, exact-date collisions, round-specific deltas |
| Draft | idempotent paste, session persistence/undo, tier and legality golden tests |
| Pickups | bounded sequence optimality fixtures, intermediate legality, waiver/lock/budget fences |
| Injuries | branch completeness, evidence freshness, IR/IR+ and fallback timing |
| Goalies | start-confidence labels, minimum starts, back-to-back and ratio downside |
| Trades | stale recheck, counter legality, counterpart benefit, streaming baseline |
| Archetypes | versioned thresholds, replacement scarcity, small-sample fences |
| Journal | immutable recommendation, append-only outcome, retrospective attribution |
| Readiness | workflow dependency fixtures, recovery commands, read-only behavior |
| Surfaces | typed JSON/text parity first; TUI/Web only after contract stability |
| Regression | existing draft, morning, trade, import, simulation, and schedule tests remain green |

## Milestones

1. **Playoff defense ready** — Waves 14–15: matchup probabilities, category
   strategy, and playoff schedule value are actionable.
2. **Draft war room ready** — Wave 16: persistent live session and three-way
   recommendation work before every pick.
3. **Weekly operations ready** — Waves 17–19: multi-move streaming, injury
   contingencies, and goalies share locks, evidence, and acquisition state.
4. **Negotiation ready** — Waves 20–21: offers, counters, scarcity, and roster
   archetypes explain both sides of a move.
5. **Learning system ready** — Waves 22–23: outcomes improve calibration and one
   readiness surface protects every workflow from stale or missing evidence.

## Recommended First Implementation Slice

**Status (2026-07-19)**: Implemented for pre-week and in-progress points-mode
contracts, including legal remaining-date assignment, immutable manually
sourced matchup-to-date totals, expected/floor/upside bands, modeled win
probability, bench collisions, fresh saved status evidence, and the top legal
one-move swing. Pre-week category mode now persists exact category direction,
sum/ratio aggregation, tie tolerance/policy, and goalie minimums; it reports
category probabilities and safe/press/volatile/low-return classifications with
correct ratio component aggregation. In-progress category snapshots now retain
source-labeled observed numerator/denominator components and goalie appearances,
then project only dates after the snapshot. Confirmed goalie-start evidence
is now implemented with persistent sourced observations, atomic CSV import,
daily checklist export, evidence freshness, lock-aware refresh checkpoints,
late-reversal handling, confirmed-stream promotion, and two-versus-three goalie
portfolio advice.

Start Wave 14 with a deterministic pre-week points-mode `matchup-plan`:

1. resolve the selected week and saved opponent;
2. assign both rosters legally for every game date;
3. project remaining active-lineup value from the active scheme;
4. report expected margin, floor/upside bands, schedule/bench losses, and the
   largest legal lineup or one-move pickup swing;
5. emit `fantasy_matchup_strategy.v1` with fixture tests;
6. add category rules and pasted matchup-to-date state as the next slice.

This produces useful output immediately while establishing the shared contract
needed by playoff, pickup, goalie, trade, archetype, and journal waves.

## Exit Criteria

- All ten capabilities have typed contracts and deterministic core tests.
- Matchup and playoff decisions optimize weekly outcomes rather than season rank.
- Draft and pickup workflows respect the exact saved league rules at every step.
- Injury and goalie advice exposes evidence freshness and uncertainty.
- Trades compare counters with streaming and playoff alternatives.
- Decision review preserves original reasoning and distinguishes process from luck.
- Readiness blocks unsupported certainty and names recovery actions.
- No workflow mutates an external fantasy platform without a future explicit
  authenticated contract.
