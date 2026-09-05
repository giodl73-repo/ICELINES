# League-Aware Daily Decisions

**Date**: 2026-09-05

**Status**: Complete — implemented and role-reviewed

**Parents**:
[`Fantasy Season Cockpit`](2026-09-05-fantasy-season-cockpit.md) and
[`Fantasy War Room`](2026-07-18-fantasy-war-room-roadmap.md)

**Role review**:
[`../../signals/roles/check/league-aware-daily-decisions-roles-check-2026-09-05.md`](../../signals/roles/check/league-aware-daily-decisions-roles-check-2026-09-05.md)

**Implementation review**:
[`../../signals/roles/check/league-aware-daily-decisions-implementation-roles-check-2026-09-05.md`](../../signals/roles/check/league-aware-daily-decisions-implementation-roles-check-2026-09-05.md)

**Contracts**: `fantasy_daily_decisions.v1`, `fantasy_today.v2`

## Outcome

Turn the implemented season cockpit into a league-aware daily operator that
answers, in order:

1. What legal action matters before the next lock?
2. How does that action affect this week's actual opponent and objective?
3. Which bench player can create a usable start on a quiet night?
4. Is a goalie start required, optional, or actively harmful?
5. If a transaction is justified, what is the best bounded add/drop pair and
   how many weekly acquisitions remain?

The answer is assembled once from local evidence and projected unchanged into
CLI, TUI, Web, and JSON. It is not a new optimizer and does not put personal
PUCK preferences into IceLines.

## Product boundary

- IceLines owns reusable league rules, legal-action checks, hockey evidence,
  schedule fit, matchup context, and explainable decision contracts.
- PUCK owns Gio's private league rosters, sentimental weights, watch lists,
  season notes, and longitudinal management history.
- The default command is read-only, cached-only, and network-free. It never
  syncs, adds, drops, starts, benches, waives, trades, or records history.
- Provider observations describe platform state only. Official cached NHL
  sources remain authoritative for statistics, schedules, and game results.
- Points and categories remain different objective functions. Missing category
  components never become a synthetic points matchup or a row of zeroes.

## Decisions already made

### One in-process assembly owner

Add a synchronous local assembly service to `icelines-fetch`, provisionally
`fantasy_today_service.rs`. CLI, TUI, and Web call it directly. The service:

- opens `FantasyDb` through its immutable read-only path;
- loads the applicable stats repository and cached schedule once per assembly;
- composes the existing morning, injury, lineup, goalie, acquisition,
  bench-coverage, and matchup builders;
- constructs and drops all `StatsRepository` and SQLite state inside the call;
- returns only owned core ViewModels and typed diagnostics;
- performs no live API call and no persistence.

`icelines-core` remains pure. It receives assembled child views plus explicit
identity, clock, rules, and evidence inputs. Renderers receive the final owned
contract and contain no fantasy calculations.

### Complete request identity

The assembly request carries, or resolves unambiguously:

```text
FantasyTodayAssemblyRequest
  league_id
  user_team_id
  season
  season_type
  evaluated_at_utc
  local_date
  week_start
  timezone (IANA)
  competition_mode
  data_root / database path
  freshness policy
  candidate policy
```

League names are display labels, never cache or join keys. A request that cannot
resolve one league and one user team fails with a typed recovery action.

### Saved matchup selection

The service selects the newest coherent saved platform snapshot whose league,
team, opponent, week, competition mode, and capture time match the request.
Future snapshots, wrong-week rows, cross-team rows, and incomplete identities
are rejected and disclosed.

- Points mode may compose the existing points matchup strategy from saved point
  totals and goalie appearances.
- Categories mode requires actual category components. The current platform
  snapshot does not store them, so categories remain typed `unavailable` with a
  recovery command until that provider/storage contract is implemented.
- No opponent is `unavailable`, not a 0-0 matchup.
- Snapshot provenance and capture time survive into evidence rows.

### Contract evolution

`fantasy_today.v1` remains a compatibility projection at
`/api/v1/fantasy/today`. League-aware prioritization changes primary-action
semantics, so the new default contract is `fantasy_today.v2` at:

```powershell
icelines fantasy today --json
icelines tui fantasy
GET /fantasy/today
GET /api/v2/fantasy/today
```

`fantasy_daily_decisions.v1` is the new pure child contract. It contains:

- one primary legal decision and deadline;
- ordered legal alternatives;
- matchup impact and firmness;
- quiet-night usable-start evidence;
- goalie-minimum posture;
- acquisition budget and waiver/lock state;
- an optional bounded transaction candidate;
- typed unavailable/stale/partial states and recovery commands;
- the evidence timestamps and material fingerprint used to reach the answer.

The v1 compatibility projection keeps its current meanings. It must not expose
v2 decisions under a v1 schema label.

### Bounded transaction advice

The default daily path does not invoke the known-slow global pickup search.
Transaction advice is available only from a caller-supplied or cached bounded
candidate set with disclosed population, truncation, freshness, and elapsed
time. Pulse 0 measures the existing path and sets the supported candidate/time
budget before a numeric cap is committed.

If no qualifying bounded set exists, the transaction section is provisional
and links to the explicit deeper pickup command. Lineup, injury, matchup,
goalie, and quiet-night decisions remain usable.

### Refresh and concurrency

- CLI builds once and exits.
- TUI replaces the process-wide `OnceLock` result with owned load state,
  refreshes on screen entry or `r`, and invalidates on context/fingerprint
  change. It never keeps `StatsRepository` in a spawned task.
- Web runs the synchronous service inside a blocking boundary only if all
  `!Send` repository/database values are created and dropped inside that
  boundary and the returned ViewModel is owned. A compile-focused proof decides
  whether `spawn_blocking` is sufficient or a dedicated local worker is needed.
- No `StatsRepository`, `rusqlite::Connection`, or borrowed row crosses an
  await, thread, or surface boundary.

### Decision history

Default reads do not secretly create recommendation history. A later explicit
Wave 22 operation may record a `fantasy_today.v2` material fingerprint and the
manager's eventual choice in an append-only journal. PUCK may consume exported
JSON for personal season analysis. Automatic logging is out of scope here.

## Delivery slices

Each slice ends build-green and reviewable.

### 0. Measure and freeze current truth

- Capture current cold/warm p50/p95 for CLI, TUI adapter, Web adapter, matchup
  assembly, and pickup search on named public fixtures.
- Freeze `fantasy_today.v1` JSON and current missing-matchup behavior.
- Add a before/after database hash and SQLite-sidecar inventory.
- Record the current subprocess count and stats/schedule load count.

### 1. Assembly request and typed errors

- Add the complete request axes and explicit clock/freshness policy.
- Define errors for missing/ambiguous league or team, missing rules, missing
  cache, stale/partial snapshot, unsupported categories, and invalid timezone.
- Return recovery commands without invoking them.
- Prove immutable open and no network at L1/L2.

### 2. Shared local assembly service

- Extract CLI-private injury, goalie, bench-coverage, and matchup assembly into
  `icelines-fetch` without moving calculations into the I/O layer.
- Load stats and schedule once, then reuse the resulting local evidence.
- Delete duplicate orchestration from the CLI after parity is green.
- Keep the service synchronous and return owned core inputs/views.

### 3. Saved matchup composition

- Select the newest coherent points snapshot using full identity/time axes.
- Compose `fantasy_matchup_strategy.v1` into the daily decision input.
- Expose capture time, authority, freshness, rejected-candidate reasons, and
  goalie-appearance provenance.
- Keep unsupported category evidence honest and actionable.

### 4. Daily decisions v1 and Today v2

- Add the pure `fantasy_daily_decisions.v1` builder in `icelines-core`.
- Prioritize only actions legal at `evaluated_at_utc`; conditional actions may
  be alternatives but cannot be presented as firm.
- Add `fantasy_today.v2` and a stable v1 compatibility projection.
- Freeze deterministic JSON fixtures and ordering/fingerprint rules.

### 5. Bounded transaction candidate

- Benchmark and select a measured candidate policy.
- Reuse the existing pickup scoring contract; do not introduce a new score.
- Disclose candidates considered, truncation, freshness, legal roster fit,
  acquisition cost, waiver timing, and quiet-night usable starts.
- Degrade to a recovery command when the budget or evidence is insufficient.

### 6. Surface convergence

- Point CLI directly at the service.
- Replace TUI subprocess/`OnceLock` loading with refreshable owned state.
- Replace Web subprocess invocation with the validated in-process boundary.
- Add `/api/v2/fantasy/today`; preserve v1 compatibility and semantic no-script
  HTML.
- Render context, primary action, deadline, firmness, evidence age, and recovery
  before secondary detail at 80 and 120 columns and on narrow mobile Web.

### 7. PUCK handoff and explicit journal seam

- Document the stable v2 JSON fields PUCK can ingest without copying IceLines
  internals or private data back into this repository.
- Expose the material fingerprint and evidence timestamps needed by the future
  Wave 22 journal.
- Do not implement automatic writes, remote sync, or PUCK-specific scoring.

### 8. Closeout

- Run focused and workspace validation, parity fixtures, performance gates,
  role re-review, and documentation drift checks.
- Remove the two cockpit P2 limitations only when saved matchup composition and
  all in-process surface adapters are proven.
- Update the parent plans and surface truth table with measured evidence.

## Decision ordering

The pure builder applies this precedence, with stable tie-breakers:

1. blocked or expiring legality/freshness recovery;
2. legal roster/IR correction before the next lock;
3. action required to satisfy the weekly goalie minimum;
4. matchup-positive start/bench change with a usable start;
5. bounded transaction whose modeled benefit clears the configured threshold;
6. optional quiet-night improvement;
7. no action.

An action may move upward only when its evidence is at least as firm as the
action it displaces. Sentimental preference never changes this generic order;
PUCK may use it when presenting alternatives to Gio.

## Tests and evidence

- **L0 core**: deterministic order/fingerprint; v1 projection; points versus
  categories; missing is not zero; legal/conditional/blocked; tie-breaks;
  acquisition exhaustion; goalie minimum; no-action state.
- **L1 service**: temporary read-only FantasyDb; one exact snapshot selected;
  wrong league/team/week/future/partial rejected; one stats/schedule load;
  manual league before Yahoo; stale provider; bounded candidate disclosure.
- **L2 binary**: help/parsing, 80-column text, v1/v2 JSON schema, actionable
  errors, zero child-process invocations, and byte-identical DB plus sidecars.
- **Parity**: one sealed public fixture produces the same primary decision,
  alternatives, deadline, firmness, evidence, and fingerprint in CLI/TUI/Web.
- **Interaction**: TUI entry/refresh/context change; Web keyboard, screen
  reader, no-JavaScript, mobile, empty, stale, partial, and error states.
- **Performance**: named cold/warm p50/p95 gates for no-candidate and bounded
  candidate modes; no undocumented regression from the current release
  baseline.

No fixture contains a private league payload, credential, or personal PUCK
preference. Tests perform no live network access.

## Acceptance gates

- CLI, TUI, and Web invoke one in-process assembly owner and spawn no IceLines
  subprocess for the daily view.
- A saved points matchup is selected only on complete identity/time axes and is
  visible in the default decision.
- Unsupported or missing category evidence is explicit and never zero-filled.
- The primary decision is legal at the evaluation instant and explains matchup
  impact, deadline, firmness, and evidence age.
- Default reads are immutable, network-free, and leave the database and its
  sidecars byte-for-byte unchanged.
- Candidate work is bounded, measured, disclosed, and reusable; global search
  stays an explicit drill-down.
- v1 compatibility and v2 semantics have independent golden fixtures.
- 80/120-column TUI, 80-column CLI, and accessible mobile/no-script Web preserve
  the same decision hierarchy.
- IceLines remains league-neutral; PUCK integration consumes a contract and
  does not leak personal data into this repository.

## Amendment log

The 2026-09-05 `.roles` review required three amendments, now incorporated:

1. **Architecture and contract** — one `icelines-fetch` in-process assembly
   owner, a pure core decision contract, complete axes, explicit `!Send`
   boundaries, and honest v1/v2 evolution.
2. **Decision trust** — coherent snapshot selection, field-level authority,
   legal-at-time actions, points/categories separation, bounded candidate work,
   and an explicit rather than automatic journal.
3. **Operator proof** — refreshable TUI state, safe Web execution, deterministic
   parity fixtures, no-mutation/network/process evidence, accessible degraded
   states, and measured performance gates.

## Completion record

Completed on 2026-09-05 with the shared read-only `icelines-fetch` assembly
service, pure `fantasy_daily_decisions.v1` and `fantasy_today.v2` core
contracts, exact v1 compatibility projection, and direct CLI, TUI, Web, and
JSON consumption. The implementation also adds coherent saved-matchup
selection, explicit readiness and recovery evidence, legal-at-evaluation
decisions, and a disclosed 12-candidate transaction screen with a 250 ms
budget. The global pickup search remains an intentional drill-down.

Validation evidence is recorded in the implementation role review and
[`../../signals/performance/fantasy-today-2026-09-05.md`](../../signals/performance/fantasy-today-2026-09-05.md).
The database and sidecars remained byte-identical during the read-only smoke,
and no daily surface invokes an IceLines subprocess. The completion audit added
a compact sealed public surface fixture consumed by CLI, TUI, and Web, carried
the requested season type through the stats load, made missing rules/cache
failures typed and actionable, rejected stale/partial saved matchups, and made
all surfaces render the same v2 alternatives and fingerprint. Field-triggered
async TUI loading remains an optional follow-up because it does not change the
decision contract and measured warm assembly remains below the interaction
budget.

## Requirement audit closure

| Requirement family | Closing evidence |
|---|---|
| Request identity and typed recovery | `FantasyTodayAssemblyRequest` carries season type; service tests cover missing database, rules, schedule cache, invalid inputs, and exact league/team resolution paths. |
| Saved matchup trust | Selection tests cover newest coherent, reverse orientation, wrong team, future, stale, and partial snapshots; category components remain explicitly unavailable. |
| Pure contracts and compatibility | Independent v1/v2 goldens, deterministic fingerprints, exact `v1_projection()`, and bounded-candidate disclosure tests. |
| Surface convergence | CLI, TUI, and Web call the one service and consume `FantasyTodaySurfaceDecision`; the shared sealed fixture pins decision, alternatives, deadline, firmness, evidence age, matchup impact, and fingerprint. |
| Interaction and accessibility | CLI wrapping, 80/120-column TUI degradation and refresh entry points, plus semantic responsive no-script Web and typed empty/error routes. |
| Performance and immutability | Named release cold/warm and bounded-candidate measurements live in `signals/performance/fantasy-today-2026-09-05.md`; database hash/sidecar and zero-subprocess evidence are recorded there. |
| PUCK boundary | `docs/contracts/fantasy-today-v2.md` documents the generic handoff; no private roster, credential, sentimental weight, or automatic journal is stored here. |
