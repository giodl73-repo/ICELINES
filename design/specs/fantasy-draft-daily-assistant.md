# Fantasy Draft and Daily Assistant — Specification

**Version**: 0.2  
**Date**: 2026-07-18  
**Status**: Implemented (partial) — draft, daily, morning, roster, trade, and simulation foundations live
**Owner domain**: fantasy decision support
**Plan**: [`../plans/2026-07-16-fantasy-draft-daily-assistant.md`](../plans/2026-07-16-fantasy-draft-daily-assistant.md)
**Expansion roadmap**: [`../plans/2026-07-18-fantasy-war-room-roadmap.md`](../plans/2026-07-18-fantasy-war-room-roadmap.md)

---

## Purpose

Build a league-specific fantasy hockey copilot that helps with four connected
decisions:

1. draft the strongest legal roster without creating avoidable schedule
   collisions;
2. set the highest-value legal lineup every day;
3. use at most four weekly acquisitions on meaningful improvements; and
4. detect injury replacements and under-the-radar opportunity risers before
   their season totals make them obvious.

The assistant consumes the scoring scheme already stored on the active fantasy
league. Generic NHL rankings are evidence, not the final ranking authority.

This feature extends `fantasy-leagues.md`, `fantasy-scheme.md`,
`fantasy-poacher.md`, and the implemented `fantasy schedule-edge` foundation.

---

## Settled League Rules

### Standard roster

| Slot | Count | Eligibility |
|---|---:|---|
| C | 2 | C |
| LW | 2 | LW |
| RW | 2 | RW |
| D | 3 | D |
| UTIL | 1 | Any skater; goalies excluded |
| G | 2 | G |
| Bench | 4 | Any player, including a third goalie |

There are 12 active slots and four unrestricted bench slots, for 16 standard
roster players. The manager may change active/bench assignments every day.

### Injury reserve

| Slot | Count | Eligible statuses |
|---|---:|---|
| IR | 2 | Confirmed IR or LTIR |
| IR+ | 2 | DTD, OUT, IR, or LTIR |

IR and IR+ players do not consume one of the 16 standard roster positions while
properly placed. The optimizer assigns strict IR-eligible players to IR before
using flexible IR+ space when that preserves room for DTD/OUT players.

### Transactions

- Four acquisitions are allowed per Monday-Sunday fantasy week.
- A free-agent add is usable on the same day if the player's game has not
  locked.
- A dropped player enters waivers for two days.
- A waiver player is unavailable until the recorded clearance time.
- A waiver acquisition counts against the four weekly acquisitions by default;
  the rule remains configurable for leagues with different accounting.
- Moving a rostered player between active, bench, IR, and IR+ does not consume
  an acquisition.
- The product recommends moves; it does not execute external fantasy-platform
  transactions without a future explicit authenticated mutation contract.

### Time and lock semantics

- Fantasy weeks run Monday through Sunday.
- The default operating timezone is `America/Los_Angeles`.
- The morning briefing defaults to 07:00 local time and is configurable.
- Each player locks independently at the scheduled start of that player's NHL
  game.
- The optimizer may rearrange only unlocked players and slots.
- A same-day free agent is useful only if the add can complete before game lock.

---

## Product Workflows

### Draft assistant

Before each draft turn the manager can paste or import the players already
taken. IceLines removes those players from the available pool, reads the marked
roster, and recommends the best legal next pick.

Required inputs:

- active fantasy league and persisted scoring scheme;
- current user roster;
- taken/rostered player pool from FantasyDb, CSV, clipboard/stdin, or a future
  provider connector;
- platform eligibility such as `C/LW`, `C/RW`, or `LW/RW` when available;
- current NHL team and official season schedule; and
- optional position/category priorities.

Required output:

- best overall available player;
- best C, LW, RW, D, G, and multi-position value;
- current starter gaps and remaining bench capacity;
- projected slot assignment after the pick;
- marginal league-scored value;
- marginal usable starts and collision cost;
- schedule equivalence class and low-overlap benefit;
- alternative recommendation if the preferred player is selected; and
- an explanation of why need, quality, flexibility, and schedule produced the
  ranking.

Example input paths:

```powershell
Get-Clipboard | icelines fantasy draft-board --taken-file -
icelines fantasy draft-board --taken-file taken.txt
icelines fantasy draft-board --eligibility-file yahoo-player-pool.csv
```

The parser accepts one player per line and common CSV columns. Unresolved rows
remain visible as diagnostics and are never silently treated as available.

### Daily lineup assistant

For each date, choose the highest-value legal active lineup from rostered
players whose NHL teams play that day. Players without games remain on the
bench and do not compete for active slots.

The daily response includes:

- active assignments by slot;
- bench players and the reason each is benched;
- unused active slots;
- locked players that cannot move;
- same-night collisions;
- provisional injury/game-time-decision warnings; and
- legal fallback assignments.

Multi-position players are assigned once. A `C/LW` player may fill C, LW, UTIL,
or bench, but cannot count toward multiple simultaneous slots.

### Weekly acquisition assistant

The weekly planner evaluates the remaining Monday-Sunday horizon rather than
raw season totals. It tracks `acquisitions_used` and refuses to recommend a plan
that exceeds four additions.

Each candidate recommendation includes:

- recommended add and drop;
- first usable date and game-lock deadline;
- incremental playable starts after daily slot optimization;
- projected league-scored contribution;
- category/position gap effect;
- quiet-slate and schedule-class effect;
- whether the move is a one-day stream, multi-day stream, injury replacement,
  stash, or rest-of-season upgrade;
- pickup budget before and after the move; and
- the opportunity cost of losing the dropped player to two-day waivers.

The initial planner may recommend one move at a time and recompute after each
confirmed transaction. A later bounded search may propose a full sequence of
up to four moves, but it must honor game locks and waiver clearance at every
step.

### Morning and pregame briefing

The default briefing runs at 07:00 Pacific and is explicitly provisional. The
same analysis can be refreshed:

- 90 minutes before a roster player's game;
- 30 minutes before puck drop for lineup/goalie confirmation; and
- on a material injury, scratch, starter, line, pair, or power-play-role change.

Alerts are material-only. A refresh with no decision-changing evidence should
not create noise.

The assistant provides conditional actions:

```text
Start Player A at LW.
If ruled OUT, move Player B from bench to LW.
If Player B is unavailable, add Player C and drop Player D.
Pickup budget after move: 2 of 4 remaining.
```

### Secret-finds board

The board is designed to identify a Darren Raddysh-style pickup: a player whose
role and league-specific opportunity become valuable before name recognition or
season totals catch up.

Candidate kinds:

- emerging role;
- emerging defense;
- injury replacement;
- schedule unlock;
- category specialist;
- goalie streamer;
- short-term streamer; and
- longer-term breakout.

Signals include:

- PP1/PP2 or top-line/top-pair promotion;
- recent TOI, shots, attempts, blocks, hits, starts, and deployment trend;
- opportunity created by injury, trade, or scratch;
- per-minute or per-game scoring under the active league scheme;
- multi-position eligibility;
- quiet-slate games and incremental playable starts;
- local/imported availability; and
- sample-size, role-stability, and injury risk.

One-game point spikes without a supporting role or opportunity change receive a
risk discount and an explicit explanation.

---

## Position and Eligibility Authority

Canonical NHL position and fantasy-platform eligibility are separate facts.

```text
PlayerEligibility {
  player_id,
  canonical_position,
  platform_positions,
  source,
  fetched_at,
}
```

- NHL roster/boxscore position remains canonical hockey identity.
- Yahoo or manually imported eligibility controls fantasy slot legality.
- Platform eligibility may contain multiple positions.
- Missing platform eligibility falls back to canonical position with a visible
  warning and lower confidence.
- CSV position hints may become authoritative for the selected fantasy league
  only when the import is explicitly labeled as platform eligibility. They do
  not rewrite canonical NHL position.

---

## Optimization Model

### Daily assignment

Daily lineup selection is a maximum-weight bipartite assignment:

- left nodes: unlocked roster players with a game on the date;
- right nodes: C, LW, RW, D, UTIL, and G slot instances;
- edge: player is platform-eligible for the slot;
- edge value: projected league-scored contribution for that game, with stable
  deterministic tie-breaks;
- constraints: one player per slot and one slot per player.

The solver must maximize, in order:

1. legal filled active slots when projected value is available;
2. projected league-scored contribution;
3. category/position need fit;
4. preservation of flexible eligibility for unresolved slots; and
5. deterministic player/slot ordering.

Goalies cannot occupy UTIL. Bench and injury reserve are state, not active
scoring slots.

### Draft marginal value

Draft ranking starts with player quality under the persisted league scheme and
then evaluates the candidate's marginal effect:

```text
DraftValue =
  league_scored_quality
+ starter_gap_value
+ positional_scarcity
+ multi_position_flexibility
+ incremental_usable_starts
+ quiet_slate_value
+ schedule_diversity
- collision_cost
- injury_or_role_risk
```

Player quality remains the largest component. Calendar fit may break close
decisions but must not make a replacement-level player outrank an elite player
without a clearly disclosed, league-specific reason.

### Weekly marginal value

The weekly engine simulates daily assignment before and after an add/drop. Raw
scheduled games are not counted as usable starts when the player would remain
blocked on the bench.

```text
WeeklyMoveValue =
  projected_points_from_incremental_starts
+ category_gap_delta
+ future_schedule_option_value
- dropped_player_rest_of_week_value
- waiver_reacquisition_cost
- pickup_budget_cost
- uncertainty_discount
```

Schedule equivalence classes remain a useful draft explanation, while exact
date-by-date overlap is authoritative for lineup and pickup decisions.

---

## Data and Persistence

### Existing inputs

- `fl_leagues`: active league, scoring scheme, roster shape;
- `fl_teams` / `fl_roster`: ownership and marked user roster;
- official NHL rosters and 2026-27 schedule cache;
- season, recent-window, goalie, realtime, and optional MoneyPuck stats;
- fantasy import and poacher availability state; and
- watch rules and notes.

### Required new state

The implementation plan may normalize names, but the durable contract must
represent:

- platform eligibility per player/league;
- acquisition ledger with effective time, add/drop/claim kind, and fantasy
  week;
- waiver state and clearance timestamp;
- player injury/status observations with source and freshness;
- daily slot locks and confirmed lineup decisions;
- league roster configuration including active, bench, IR, and IR+ counts; and
- briefing preferences such as timezone and morning time.

All migrations must preserve existing FantasyDb data and support read-only Web
GET behavior without creating databases or WAL/SHM files.

---

## Source State and Injury Safety

Injury and deployment information can change minutes before puck drop. Every
observation carries source, status, observed/fetched time, and confidence.

```text
PlayerAvailabilityStatus =
  Healthy
  DayToDay
  GameTimeDecision
  Out
  InjuredReserve
  LongTermInjuredReserve
  Suspended
  Personal
  Unknown
```

Rules:

- `Unknown` is not healthy.
- A stale injury observation cannot support a definitive start recommendation.
- Estimated lines or goalies are labeled estimated.
- Confirmed scratches, starters, and IR transactions name their evidence.
- Missing injury/deployment feeds lower confidence rather than silently scoring
  the player as unavailable.
- The morning briefing identifies which recommendations need a pregame refresh.

No unverified scraping source is promoted to authority without an explicit
source/schema/freshness review.

---

## ViewModels and Commands

Planned typed views:

- `FantasyDraftBoardView`
- `FantasyRosterConstructionView`
- `FantasyDailyLineupView`
- `FantasyWeeklyPickupView`
- `FantasyInjuryResponseView`
- `FantasyMorningBriefingView`
- `FantasySleeperBoardView`
- `FantasySeasonSimView`

Planned CLI:

```text
icelines fantasy draft-board [--taken-file PATH|-] [--eligibility-file PATH]
icelines fantasy draft-fit PLAYER [--date YYYY-MM-DD]
icelines fantasy lineup --date YYYY-MM-DD
icelines fantasy weekly-pickups [--date YYYY-MM-DD] [--pickups-used N]
icelines fantasy sleepers [--positions C,LW,RW,D] [--top N] [--json]
icelines fantasy injury-plan [--date YYYY-MM-DD]
icelines fantasy morning [--date YYYY-MM-DD] [--at RFC3339] [--material-only] [--json]
icelines fantasy sleepers [--position D] [--window 5]
icelines fantasy season-sim [--team NAME] [--trials N] [--seed N] [--injury-rate P] [--trade-probability P] [--json]
icelines fantasy season-sim --scenario-matrix [--team NAME] [--trials N] [--json]
icelines fantasy season-sim [--opponent-pickup-accuracy P]
icelines fantasy season-sim --manager-matrix [--trials N] [--json]
```

All views support deterministic JSON. CLI/TUI/Web renderers consume the shared
views and do not recompute assignment or recommendation math.

Roster, draft, morning, and trade card projections follow
[`ui-neutral-card-system.md`](ui-neutral-card-system.md). Those projections
reuse the active league and feature ViewModels; the card grammar does not gain
its own eligibility, assignment, waiver, pickup-budget, scoring, or trade
logic.

---

## Non-Claims and Safety Boundaries

- IceLines does not guarantee that a player will play or score.
- A recommendation is not a confirmed injury, line, pair, PP, or goalie report
  unless the evidence is explicitly labeled confirmed.
- The assistant does not autonomously add, drop, claim, start, bench, or move a
  player to IR on an external platform.
- Schedule classes do not measure player quality.
- Generic roster percentage is not availability in the user's league.
- A locally imported available player may have been acquired elsewhere since
  the last sync; freshness is always shown.
- Morning output is provisional until relevant game-time decisions and lineup
  locks resolve.

---

## Acceptance Criteria

1. A 16-player roster plus two IR and two IR+ slots validates against the
   settled configuration.
2. Multi-position players are assigned to at most one active slot per day.
3. UTIL accepts skaters and rejects goalies.
4. Bench accepts any position, including a third goalie.
5. Daily assignment respects player-specific game locks.
6. The same-day free-agent path counts the player's game only when the move can
   complete before lock.
7. No weekly plan exceeds four acquisitions.
8. A dropped player is excluded until the two-day waiver clearance time.
9. Strict IR assignment preserves IR+ capacity when possible.
10. Draft imports never silently treat unresolved taken players as available.
11. Recommendations use the active league's persisted scoring scheme.
12. Weekly value counts optimized playable starts, not raw scheduled games.
13. Injury/deployment recommendations carry source and freshness.
14. A missing or stale source produces a warning and confidence downgrade.
15. CLI text and JSON share one typed ViewModel result.
16. Tests cover DST/timezone, Monday-Sunday boundaries, game locks, same-day
    adds, waiver clearance, pickup budget, multi-position matching, IR/IR+,
    goalie/UTIL exclusion, third-goalie benching, taken-list parsing, and
    deterministic ranking.
17. A seeded full-season stress run is deterministic and includes injuries,
    recoveries, IR/IR+ replacements, weekly pickups, and roster trades without
    mutating the saved league.
18. Six playoff qualifiers use the final three Monday-Sunday fantasy weeks as
    quarterfinal, semifinal, and championship windows, with byes for seeds one
    and two.
19. `season-sim --team` locks every resolved player from a partial or complete
    saved roster, preserves active-slot legality, reports unresolved players,
    and fills only the remaining standard-roster spots synthetically.
20. Regular-season standings use seeded Monday-Sunday head-to-head matchups and
    expose average W-L-T records, average seed, and first-place probability,
    with fantasy points as the standings tiebreak.
21. Playoff probability equals the mutually exclusive sum of first-round,
    semifinal, final, and championship outcomes for every simulated team.
22. `season-sim --scenario-matrix` holds roster, seed, schedule, scoring, and
    trials constant across clean, baseline, and high-chaos environments and
    reports each selected-team outcome against the baseline.
23. Manager advantage is an explicit opponent pickup-accuracy input: team one
    chooses the top projected add, while opponent misses select the second- or
    third-ranked add. The neutral default is full opponent accuracy; no points
    multiplier may stand in for transaction quality.
24. Pickup, trade, injury, and scoring randomness use independent deterministic
    domains; weekly and player-level events are keyed by trial/team/week/date so
    changing manager accuracy does not silently rewrite unrelated luck.
25. `--manager-matrix` compares 100%, 85%, and 70% opponent accuracy with parity
    as the delta reference and refuses combination with `--scenario-matrix`.
26. Every simulated pickup and trade leaves each affected standard roster able
    to fill all configured active slots under platform multi-position eligibility.
    Complete locked and synthetic drafted rosters that fail this invariant are
    rejected before trials; a full lock bypasses irrelevant temporary drafting.
27. An injured IR/IR+ player cannot be synthetically dropped or traded. When a
    temporary replacement is later swapped, replacement ownership follows the
    incoming player and recovery releases that current substitute exactly once.
28. Recovery and substitute release occur before pickups and trades on the same
    date, including a Monday acquisition-budget reset.
29. The simulator opens a rotating proactive pickup opportunity every day, but
    proactive moves and injury replacements share one hard Monday-Sunday
    four-add limit.
30. Every drop and released injury substitute enters the configured waiver
    window and is excluded from all pickup rounds until its exact clearance date.
31. A pickup pays an explicit three-game retention cost for any positive
    league-scored per-game gap favoring the drop. Quiet-week stars remain
    protected while comparable-player schedule streams remain legal.
32. Team one can reserve acquisitions from proactive streaming without lowering
    the hard weekly limit available to injury replacements. The default reserve
    is one of four moves through Friday, releases Saturday if unused, and can be
    disabled for paired stress comparisons.
33. Season output reports long-injury replacement attempts blocked specifically
    by an exhausted weekly acquisition budget, separately from starts lost.
34. Weekly-budget, weekly-pickup, JSON, and morning surfaces distinguish the
    platform hard limit from safe proactive capacity. A current IR/IR+ opening
    may consume the reserve, and an unused reserve releases Saturday.
35. A healthy-roster exceptional override requires at least +6.0 projected net
    value and +3.0 usable starts. Any status requiring pregame refresh disables
    the exception; the morning action must identify when the final move is spent.
36. A paired reserve-policy matrix compares all-in, strict, and adaptive policies
    without changing roster, seed, schedule, scoring, injuries, or performance
    luck. The simulation documents scheduled-game gain as its usable-start proxy.
37. Trade evaluation supports one-to-three-player packages containing skaters or
    goalies, projects league points per game across 2026-27 remaining games,
    reports multi-position roster legality for both teams, and has text/JSON
    parity. Execution atomically commits legal one-to-three-player packages and
    refuses stale membership or any positionally illegal result.
38. Trade finding searches one opponent or the league for fair one- and
    two-player packages, rejects positional or capacity regressions for either
    team, ranks only positive user fits by rest-of-season value and schedule
    diversification, discloses counterpart schedule effects, and labels partial
    roster results provisional.
39. Trade finding protects the user's highest-value anchor by default, accepts
    additional named protections, and ranks offers with separate active-lineup,
    user-fit, counterpart-fit, and mutual-fit values. Anchor inclusion requires
    an explicit override.
40. A complete Yahoo roster export can replace included saved rosters after a
    dry-run preflight. Exact synchronization permits legitimate player movement,
    removes stale memberships atomically, and refuses to mutate when any source
    row is skipped, unresolved, duplicated, or invalid.
41. Trade readiness reports roster size, missing active slots, unresolved
    position eligibility, and missing scoring data for each checked team with
    text/JSON parity. Strict
    trade finding refuses provisional output unless the user and every searched
    opponent are complete and legally fillable.
42. Yahoo roster synchronization accepts the same CSV through a file or stdin,
    including BOM-prefixed clipboard text, so a complete league can be pasted
    immediately before strict trade analysis without an intermediate file.
43. Every locally executed trade records both canonical packages and both teams
    in the same transaction as the roster swap. History is newest-first in text
    or typed JSON, and failed trades create no audit entry.
44. A legal trade evaluation can be saved as a pending offer without mutating
    rosters. Pending offers have an immutable terminal close status; accepting
    one does not pretend that the external fantasy platform has changed.
45. Saved offers revalidate both packages against current roster ownership when
    listed. Stale offers disclose the missing membership and are excluded by an
    explicit actionable-only filter without silently changing lifecycle status.
46. Matchup strategy supports explicit points and category competition modes,
    legal daily assignments, remaining-week distributions, and floor/balanced/
    upside alternatives without treating simulation probabilities as betting odds.
47. Playoff portfolio value separates each configured round, usable starts,
    quiet slates, and exact-date bench collisions from raw scheduled games.
48. A persistent draft session accepts idempotent full or incremental taken
    pastes, retains unresolved diagnostics, supports local undo, and explains
    best-value, safest-fit, and schedule-upside recommendations.
49. Multi-move pickup planning validates every intermediate roster, lock,
    waiver, and acquisition count and may recommend fewer than four moves.
50. Injury contingencies provide evidence-labeled, time-valid healthy/out/
    unresolved branches with legal IR/IR+ and fallback actions.
51. Goalie planning distinguishes confirmed, reported, estimated, and unknown
    starts while modeling league scoring, minimum appearances, rest, and
    downside without allowing a goalie in UTIL.
52. Trade negotiation revalues saved offers, fences stale packages, compares
    streaming/no-trade baselines, and generates legal counters with disclosed
    counterpart effects.
53. Player archetypes and category scarcity use versioned disclosed thresholds,
    platform replacement levels, and small-sample uncertainty rather than hidden
    aggregate labels.
54. The decision journal preserves immutable recommendation evidence and adds
    auditable outcomes that separate decision quality, projection error, and luck.
55. Workflow readiness reports ready/provisional/blocked state, source freshness,
    blockers, and recovery commands without fetching or mutating as a read side effect.
56. In-progress points matchup planning accepts both platform totals through one
    disclosed date, fixes those totals as observed history, and projects only
    later legal lineup dates. Fresh saved non-healthy status evidence affects
    the current matchup window; missing, stale, and future-week evidence remains
    an explicit refresh requirement rather than a fabricated health claim.
57. Competition mode and exact category direction, aggregation, tie tolerance,
    tie policy, and goalie minimum persist independently from point weights.
    Pre-week category projections aggregate ratio numerators/denominators before
    division, enforce legal daily assignments and goalie minimum outcomes, and
    expose per-category probabilities and strategy classifications.
58. An in-progress category snapshot supplies source-labeled numerator and
    denominator components for every configured category plus goalie appearances
    through one date. Those observations remain fixed, elapsed dates are not
    projected twice, and output separates current, remaining, and final values.
59. Goalie starter observations persist per player and game date with source,
    timestamps, and distinct confirmed/reported/estimated/backup/unknown state.
    Stale or missing evidence becomes an explicit refresh; weekly planning uses
    goalie slots only, separates expected appearances from the confirmed
    minimum floor, and discloses poor-start points and ratio downside. Unsourced
    back-to-back allocation remains probabilistic. Opponent adjustment uses a
    disclosed relative offense index, and free-agent streams count only marginal
    usable starts after slot collisions, waiver timing, and reserved move budget;
    the output compares the current goalie group with its best legal third-goalie
    alternative.
60. The morning briefing embeds the goalie plan and never emits a generic
    schedule-only goalie start. Fresh confirmed starter evidence may produce a
    firm start; reported, estimated, stale, or missing evidence produces a
    same-day refresh. A stream action remains confirmation-gated and includes a
    legal fallback when available, while sharing the same waiver and reserved
    acquisition budget as all other morning pickup advice.
61. Same-day goalie evidence supports an atomic CSV/stdin batch with strict
    player/date uniqueness, source provenance, and row-level observed times.
    Game-start timestamps drive check-later, refresh-soon, refresh-now, and
    locked urgency; locked games cannot contribute a remaining appearance or
    free-agent stream recommendation.
62. A daily goalie checklist exports the exact batch-import schema for every
    rostered goalie playing that day and the best legal same-day streams. The
    plan exposes its next evidence refresh, next game lock, checks due now, and
    unresolved rostered-goalie count. Latest evidence wins on late reversals,
    while a freshly confirmed free-agent starter is promoted explicitly in the
    morning briefing.
63. Morning output separates generation time from its decision-evaluation
    instant. Evidence freshness, game locks, and same-day stream eligibility use
    the requested `--at` value. The briefing exposes one next-refresh/next-lock
    summary, promotes a confirmed same-day stream over an unconfirmed
    higher-volume candidate, and retains that candidate as a conditional
    fallback.
64. Fresh confirmed starter and backup evidence receives a final T−30 safety
    checkpoint rather than remaining silently trusted through lock. When that
    window opens, the morning briefing emits a verify-now action; a newer
    starter/backup observation deterministically replaces the earlier decision.
65. Morning goalie streams and weekly pickups share the proactive acquisition
    budget. With one move left, distinct recommendations are explicitly
    mutually exclusive. When both surfaces select the same player, the briefing
    emits one combined action retaining the optimizer's drop and value evidence.
66. Every primary and fallback goalie stream independently carries its weekly
    optimizer add/drop pairing and projected value when one exists. Without a
    legal pairing, execution is explicitly gated on verifying an open roster
    spot; a roster-full add is never presented as a complete transaction.

---

## Resolved Decisions

- Roster: 2 C, 2 LW, 2 RW, 3 D, 1 skater UTIL, 2 G, 4 unrestricted bench.
- Injury reserve: 2 IR plus 2 IR+.
- Lineups may change daily.
- Weekly acquisitions: four.
- Free-agent adds are same-day before lock.
- Drops create a two-day waiver period.
- Scoring authority: active league scheme stored in IceLines.
- Historical calibration benchmark: Dexter's Dawgs finished 18-2 and first in
  every tracked scoring category, then lost in the first playoff round. This is
  authoritative user-supplied outcome context; the available January workbook
  is an incomplete roster/transaction snapshot and must not overwrite it.
- Morning briefing default: 07:00 `America/Los_Angeles`, with pregame refreshes.
- Availability may be pasted/imported until a verified provider connector is
  implemented.
