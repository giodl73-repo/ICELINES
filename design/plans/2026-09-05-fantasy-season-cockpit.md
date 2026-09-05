# Fantasy Season Cockpit

**Date**: 2026-09-05

**Status**: Active — reviewed; cross-surface vertical slice implemented

**Parent**: [`2026-07-18-fantasy-war-room-roadmap.md`](2026-07-18-fantasy-war-room-roadmap.md)

**Role review**: [`../../signals/roles/check/fantasy-season-cockpit-roles-check-2026-09-05.md`](../../signals/roles/check/fantasy-season-cockpit-roles-check-2026-09-05.md)
**Primary contract**: `fantasy_today.v1`

## Outcome

Give a fantasy manager one calm, fast answer to “what needs my attention now?”
without making them stitch together the existing morning, matchup, goalie,
injury, pickup, sleeper, bench-coverage, and readiness commands.

```powershell
icelines fantasy today
icelines fantasy today --json
icelines tui fantasy
GET /fantasy/today
GET /api/v1/fantasy/today
```

The first vertical slice is CLI text plus JSON. TUI and Web consume the same
core-owned contract after its CLI fixtures stabilize.

## Product boundary

- IceLines owns reusable, league-neutral hockey and fantasy decision logic.
- League rules, saved rosters, provider observations, and local decision state
  remain under the IceLines user-data root.
- Personal team preferences, sentimental targets, private league rosters, and
  season-management notes belong in PUCK, not this repository.
- The cockpit is advisory and read-only. It never fetches, adds, drops, starts,
  benches, waives, or trades as a side effect.
- Yahoo may provide league context when connected; official cached NHL sources
  remain authoritative for hockey statistics and schedules.

## Existing contracts reused

`fantasy_today.v1` is an orchestration envelope, not a second optimizer. It
reuses:

- `fantasy_morning_briefing.v3` for legal daily lineup, IR/IR+, goalie,
  acquisition-budget, pickup, sleeper, action ordering, and evidence freshness;
- `fantasy_matchup_strategy.v1` for points/category matchup state and modeled
  margin or category posture;
- `fantasy_bench_coverage.v1` and exact schedule assignment for quiet-night and
  collision evidence;
- `fantasy_provider_status.v1` and the Wave 23 readiness vocabulary for source
  readiness and recovery;
- the active league contract for scoring mode, roster shape, timezone, locks,
  acquisition limits, waivers, and goalie minimums.

No renderer may recalculate any of those meanings.

## Core contract

The pure builder lives in `icelines-core` and accepts fully assembled child
views plus explicit context. It performs deterministic prioritization and
readiness reduction only; it does no I/O, clock reads, database access, or
network work.

```text
FantasyTodayInput
  schema inputs + league/team identity + season/type
  generated_at + evaluated_at + local date/week + IANA timezone
  competition_mode
  morning: FantasyMorningBriefingView
  matchup: Option<FantasyMatchupStrategyView>
  readiness: Vec<FantasyTodayReadinessInput>
  evidence: Vec<FantasyTodayEvidenceInput>

FantasyTodayView (`fantasy_today.v1`)
  context
  state: ready | provisional | blocked
  primary_decision
  actions[]
  matchup_summary?
  lineup_summary
  goalie_summary?
  acquisition_summary
  quiet_night_summary?
  readiness[]
  evidence[]
  next_decision_deadline_utc?
  alternatives[]
  material_fingerprint
  warnings[]
```

Every recommendation row carries a stable ID, action kind, firmness
(`firm | conditional | refresh_required`), explanation, constraint summary,
and optional player key. The primary decision is the first legal actionable row
after deterministic ordering; alternatives remain visible rather than being
discarded.

The initial CLI slice may omit matchup and quiet-night summaries when no saved
input exists, but must render them as unavailable/provisional with recovery
guidance. Absence is never serialized as a zero margin, zero opportunity, or
healthy player.

## Identity and time invariants

- Hockey rows remain keyed by `(player_id, season, season_type)`.
- Fantasy context adds league ID and fantasy team ID; provider IDs never enter
  `StatsRepository`.
- Multi-position eligibility is platform context and does not replace canonical
  NHL position.
- `generated_at` records construction time; `evaluated_at` drives locks,
  freshness, waivers, and recommendations.
- The view includes local date, Monday-Sunday week bounds, IANA timezone,
  NHL stats season, and season type.
- Fingerprints exclude wall-clock generation time and warning prose, but include
  every decision-bearing child fingerprint/input.

## Source and trust contract

Each evidence row names its source family, authority scope, observed/fetched
time when known, freshness state, and recovery action. At minimum the cockpit
distinguishes:

| Evidence | Authority | Missing/stale behavior |
|---|---|---|
| NHL schedule and game state | cached official NHL source | affected schedule decisions provisional or blocked |
| Player rates | selected sealed NHL sample | projection unavailable; never zero-filled |
| Fantasy rules, roster, eligibility | saved local state or read-only Yahoo sync | name the import/sync recovery command |
| Matchup totals/opponent | labeled platform or user snapshot | pre-week projection or unavailable, never “tied 0-0” |
| Injury and goalie status | saved sourced observations | stale/future/missing becomes unknown plus refresh action |
| Acquisition and waiver state | saved league ledger | pickup action blocked if legality cannot be established |

Private raw provider payloads, OAuth credentials, and team-specific fixtures do
not enter source control or normal logs.

## Readiness and degradation

The cockpit remains useful under partial evidence:

- `ready`: required evidence for the displayed action is current;
- `provisional`: useful output exists but an optional or refreshable input is
  stale/missing;
- `blocked`: the primary workflow cannot establish legality or meaning.

Each non-ready row includes a machine-readable reason code and a concrete
recovery command. Explicit cases include no active league, no user team,
missing/illegal roster, no schedule, no opponent or matchup snapshot, stale
injury evidence, unknown goalie starter, minimum appearances already met/not
met, exhausted acquisitions, waiver delay crossing a lock, locked players,
ambiguous player identity, GP=0/missing rates, categories without category
rules, points without a scoring scheme, and no positive legal action.

## CLI interaction

Default text follows a five-second scan order:

1. league/team/date/week and overall readiness;
2. one primary decision and its deadline;
3. today’s lineup/IR/goalie checkpoints;
4. matchup and acquisition context;
5. alternatives, quiet-night opportunities, and recovery warnings.

The core summary fits 80 columns without color and does not use color as the
only state signal. `--json` emits the complete contract. Existing
`fantasy morning` remains supported; `fantasy today` becomes the preferred
operator entry point and initially composes the same proven briefing pipeline.

## TUI and Web interaction

- TUI fantasy opens on the cockpit and provides drill-downs to lineup, matchup,
  goalie, pickups, and readiness without recomputation.
- Web exposes bookmarkable `/fantasy/today` and
  `/api/v1/fantasy/today`; active league/team/date and freshness remain visible.
- HTML supports narrow/mobile layouts, keyboard navigation, semantic headings,
  non-color status labels, no-JavaScript reading, and designed empty/stale/error
  states.
- Loading/fetch controls, if later added, are explicit mutations separate from
  the read route.

## Performance budget

The current weekly-pickup candidate path has been observed near 100 seconds for
a small top-five request, so it cannot sit invisibly on the default daily path.
Pulse 0 records cold/warm timings and request counts before changing defaults.

Initial targets (targets, not current claims):

- core composition: p95 below 10 ms on fixtures;
- `fantasy today` from already-local inputs, excluding deep candidate search:
  warm p95 below 2 seconds;
- bounded pickup enrichment: warm p95 below 10 seconds with disclosed candidate
  count/truncation;
- default cockpit never performs network I/O and never waits on an unbounded
  search.

Until measured, the default reuses cached/saved or bounded recommendation
inputs and marks deeper pickup analysis as a drill-down. Performance evidence
records fixture size, machine, build profile, cold/warm state, and p50/p95.

## Delivery slices

Every commit compiles and passes its focused tests independently.

1. **Contract and fixtures** — add pure `FantasyTodayView`, typed states,
   deterministic builder, JSON fixture, and L0 invariants.
2. **CLI vertical slice** — add `fantasy today`, refactor common morning input
   assembly rather than copy it, and prove text/JSON output and no mutation.
3. **Readiness and matchup** — compose saved matchup/provider/readiness state,
   explicit recovery rows, and deadline selection.
4. **Measured pickup boundary** — benchmark the current candidate path, add a
   bounded/cached fast path, and disclose truncation.
5. **TUI parity** — render the contract with summary-first drill-downs and
   80/120-column golden fences.
6. **Web parity** — add HTML/JSON routes, accessibility/mobile/no-JS states, and
   route-level degradation tests.
7. **Docs and readiness** — make this the preferred daily workflow, reconcile
   generated/source fantasy-guide drift, and update surface truth tables.
8. **Release closeout** — workspace tests, clippy, format, diff check, role
   re-review, and measured acceptance evidence.

## Test matrix

- **L0 core**: deterministic ordering/fingerprint; firm versus conditional;
  missing is not zero; dates/timezones/DST; no actions; points/categories;
  ambiguous/multi-position/GP=0; locks/waivers/acquisition exhaustion; child
  warning deduplication; deadline selection.
- **L1 integration**: temporary FantasyDb; manual roster before Yahoo; partial
  provider state; stale/future status; no opponent; goalie minimum boundary;
  bounded pickup timeout/truncation; byte-stable JSON fixture.
- **L2 binary**: help/parsing, text at 80 columns, `--json` schema, actionable
  missing-input errors, and read-only database equivalence before/after.
- **Parity**: one sealed fixture produces equivalent decisions, firmness,
  deadlines, readiness, and warnings in CLI/TUI/Web.
- **Performance**: recorded cold/warm p50/p95 for no-pickup, bounded-pickup, and
  worst supported local fixture; regressions beyond the accepted budget block
  the relevant slice.

No test uses live network access, private league payloads, or real credentials.

## Acceptance gates

- One command exposes the highest-priority legal decision and next deadline.
- The contract composes existing child ViewModels; renderers contain no hockey
  or fantasy calculations.
- The same inputs produce byte-stable decision fields and ordering.
- Missing/stale evidence changes readiness and wording, never silently becomes
  negative or zero evidence.
- Points and categories keep separate objective functions.
- Default execution stays within the measured daily-use budget or explicitly
  degrades to a bounded view with a drill-down recovery command.
- CLI, TUI, and Web agree on primary decision, alternatives, readiness, and
  deadlines before the plan closes.
- All behavior remains league-neutral and all external mutations remain out of
  scope.

## Amendment log

The 2026-09-05 `.roles` review required the following plan changes before code:

1. Reuse `fantasy_morning_briefing.v3` and other child contracts explicitly,
   with one pure orchestration builder and no renderer logic.
2. Add complete identity/time axes, typed readiness/firmness, missing-data
   behavior, field-level authority, recovery commands, and no-mutation proof.
3. Turn latency into measured targets, keep deep pickup search off the default
   path until bounded, and add parity/accessibility/performance gates.

## Implementation checkpoint — 2026-09-05

The first cross-surface vertical slice is implemented on
`feat/fantasy-season-cockpit`:

- added the pure `fantasy_today.v1` contract with full action collection,
  stable action IDs/order, points/category matchup projections, quiet-night
  projection, typed evidence/readiness, deadline reduction, warning deduplication,
  and a generation-time-independent material fingerprint;
- added `icelines fantasy today [--json]` using the existing morning assembly,
  cached-only schedule reads, an immutable SQLite connection, and no morning
  fingerprint write;
- derives one-week bench coverage from the already-built legal lineup and
  cached schedule, keeping the known-slow deep pickup/sleeper searches behind
  explicit drill-down commands;
- added a versioned JSON decision golden, focused core/CLI tests, an 80-column
  text fence, and before/after database hash evidence;
- made `icelines tui fantasy` open the cockpit and added 80/120-column designed
  degradation tests;
- added semantic no-script `/fantasy/today` HTML and complete
  `/api/v1/fantasy/today` JSON, plus typed missing-state route tests and a real
  loopback 200 smoke;
- recorded a 233.7 ms warm release p50 and 239.7 ms warm release p95 on the
  local 16-team fixture.

Remaining before promotion from `partial` to `done`: compose the newest saved
matchup strategy into the default view without invoking the slow pickup search,
and replace the TUI/Web local-process adapter with a shared `icelines-fetch`
local assembly service. These limitations are explicit in the surface-parity
matrix rather than hidden behind a completion claim.

## Validation checkpoint — 2026-09-05

- `cargo fmt --all -- --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- `cargo test --workspace -j 1`: pass (the initial unconstrained parallel run
  exhausted local Windows compiler memory before test execution)
- `py C:/src/tracker/repos/standards-protocols/roles/tools/check_roles.py .`:
  pass with pre-existing role-frontmatter warnings
- `git diff --check`: pass
- fantasy guide source/generated byte comparison: pass
- loopback CLI/Web smoke and read-only database hash proof: pass, recorded in
  `signals/performance/fantasy-today-2026-09-05.md`
