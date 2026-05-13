# Phase Presidents Trophy - team season performance

**Date**: 2026-05-12
**Status**: Active
**Trophy lineage**: Presidents Trophy as the regular-season team-performance
phase. This is distinct from the old folded "Presidents" season-type plan:
this phase is about team standings truth, schedule context, and whether a
club's record is as strong as it looks.
**Depends on**: Foster schedule data, Lester Patrick schedule CLI, Ted Lindsay
web parity, Campbell ViewModels, Jack Adams Web dashboard workspace.

---

## Why

IceLines has a good roster/depth team view, but that answers "who is on the
team?" It does not answer "how good has this team actually been?"

A serious NHL team page needs a season-performance surface:

- where they sit in the standings;
- how far they are from a playoff or wild-card spot;
- whether their record came against strong or weak opponents;
- whether they are winning the games they should win;
- whether home/away, division, travel, and remaining schedule change the story.

This should be a new shared product surface, not more route-local analysis
inside `/team/:abbrev` or `/schedule?team=...`.

---

## Product Shape

Keep roster and season performance separate:

| Question | Surface |
|---|---|
| Who are they? | `/team/:abbrev` via `TeamDepthView` |
| What have they done? | `/team/:abbrev/season` or `/team-season?team=ABBR` via `TeamSeasonView` |
| What games are on the calendar? | `/schedule?team=ABBR` via `ScheduleTeamView` |

The new Team Season surface should feel like an NHL ops standings room:

```text
EDM Team Season - 2025-26 Regular

Record       42-23-6   90 pts   .634 pts%   +28 goal diff
Standing     Pacific 2nd   West 4th   +6 over WC2   11 GR
Split        Home 24-9-3   Away 18-14-3
Schedule     Faced .543 opp pts%   Remaining .518 opp pts%
Ledger       14 quality wins   7 bad losses   +3 schedule-adjusted points
Form         Last 10: 7-2-1   Opp strength: hard   Goals: +11
```

---

## ViewModel Contract

Add `TeamSeasonView` in `icelines-core::view_model`.

Suggested fields:

```rust
pub struct TeamSeasonView {
    pub context: ViewContext,
    pub season: String,
    pub season_pretty: String,
    pub team: String,
    pub headline: TeamSeasonHeadline,
    pub standings: Option<TeamStandingsContext>,
    pub splits: TeamSeasonSplits,
    pub schedule_strength: TeamScheduleStrength,
    pub quality_ledger: TeamQualityLedger,
    pub form: TeamRecentForm,
    pub remaining: TeamRemainingSchedule,
    pub rows: Vec<TeamSeasonGameRow>,
    pub warnings: Vec<ViewWarning>,
    pub empty_state: Option<EmptyState>,
}
```

Core invariants:

- The ViewModel owns all win/loss/OTL, points, points percentage, splits, and
  strength-of-schedule math.
- Web, CLI, TUI, and JSON adapters only project fields.
- Missing standings data degrades gracefully: schedule-derived record/splits
  still render, while standings/playoff-distance fields become source warnings.
- Opponent-strength metrics must name their basis: current standings points
  percentage, final-season points percentage for historical seasons, or
  unknown.
- Playoff distance must distinguish division rank, conference rank, and wild
  card rank. No single "playoff spot" number without context.

---

## Data Strategy

### Existing Data We Can Use Now

From `ScheduleTeamView` / `ScheduleGameRow`:

- full team schedule;
- home/away;
- final/live/scheduled;
- team and opponent score;
- OT/SO result;
- playoff vs regular marker;
- remaining games.

This supports a first slice with:

- record;
- home/away splits;
- goal differential;
- last 5/10;
- upcoming/remaining count;
- opponent list;
- division/opponent split if team metadata is available.

### New Data Needed

For real standings and strength-of-schedule:

- standings snapshot by team:
  - points;
  - regulation wins / ROW if available;
  - games played;
  - division/conference;
  - rank;
  - points percentage;
  - wildcard position;
- team metadata:
  - division;
  - conference;
- standings history basis:
  - current standings for active season;
  - final standings for completed historical seasons, if bundled.

Preferred implementation:

- Add an `icelines-fetch` standings client/cache rather than route-local fetches.
- Add a small core input DTO, e.g. `TeamStandingInput`, so core remains fetch-free.
- Store source provenance in `ViewContext.source_state`.

---

## Metrics

### Headline

- wins, losses, overtime losses;
- points;
- games played and games remaining;
- points percentage;
- goals for, goals against, goal differential;
- current streak if source supports it.

### Standings Context

- division rank;
- conference rank;
- wild-card rank/status;
- points above/below playoff cut line;
- games in hand versus nearest cutoff teams;
- possible warning when standings data is stale or unavailable.

### Home/Away And Situation Splits

- home record;
- away record;
- home/away goal differential;
- division record;
- conference record;
- record after OT/SO;
- one-goal games.

### Strength Of Schedule

- opponent points percentage faced;
- opponent points percentage remaining;
- record vs top third / middle third / bottom third;
- record vs current playoff teams;
- record vs non-playoff teams;
- back-to-back and road-trip burden if schedule dates are reliable.

### Quality Ledger

Classify each final game:

- `quality_win`: win vs strong opponent;
- `expected_win`: win vs weak opponent;
- `bad_loss`: regulation loss vs weak opponent;
- `missed_point`: OTL/SOL vs weak opponent or blown available point;
- `schedule_tax`: loss in a heavy context, if travel/back-to-back data exists;
- `statement_game`: win by multiple goals against strong opponent.

The classification thresholds should be configurable constants in core tests,
not hidden in web templates.

---

## Surface Plan

### Core

- `TeamSeasonView`
- `TeamSeasonGameRow`
- `TeamStandingsView` or standings input DTOs if needed
- pure unit tests for:
  - W-L-OTL;
  - home/away splits;
  - points percentage;
  - quality win / bad loss classification;
  - missing standings source warnings.

### CLI

Add:

```text
icelines team-season EDM
icelines team-season EDM --json
icelines team-season EDM --top-games 10
```

Or fold under the existing command if clap shape permits:

```text
icelines team EDM --season-view
```

Preferred: new explicit `team-season` command. It keeps roster/depth output
stable and makes scripts clearer.

### TUI

- Add a team season detail screen or mode reachable from Team and Schedule.
- Command bar:
  - `team EDM season` opens Team Season, not the raw schedule list.
  - `schedule EDM` can remain the game list.
- Show compact:
  - standings line;
  - home/away splits;
  - SOS card;
  - quality ledger;
  - recent/upcoming game table.

### Web

Add:

```text
GET /team/:abbrev/season
GET /api/v1/team/:abbrev/season
```

Dashboard:

- `team EDM season` opens `/team/EDM/season` in the workspace.
- Team roster page links to "Season performance".
- Schedule team page links to "Season analysis".

### Reports

Add a markdown/report section later:

```text
icelines export md team-season EDM
```

This is useful for snapshotting "why are they really 2nd in the division?"

---

## Sub-Phase Order

```text
PT.1  Contract and fixtures (started)
PT.2  Schedule-derived TeamSeasonView (started)
PT.3  Standings data source and playoff-distance model
PT.4  Strength-of-schedule and quality ledger
PT.5  CLI/TUI/web/JSON surfaces
PT.6  Dashboard integration and docs
PT.7  Role review, tests, closeout
```

### PT.1 - Contract And Fixtures

- Add spec examples for one strong team, one bubble team, one weak team.
- Create deterministic fixture games:
  - home win;
  - road loss;
  - OT loss;
  - win vs strong opponent;
  - loss vs weak opponent;
  - scheduled remaining game.
- Add standings fixture rows with division/conference ranks.

Exit:

- ViewModel field contract is documented.
- Fixture math is hand-checkable in tests.

### PT.2 - Schedule-Derived TeamSeasonView

- Build record, points, points percentage, goal differential.
- Build home/away splits.
- Build last 5/10 and remaining count.
- Add source warnings when no standings are supplied.

Exit:

- Core unit tests pass.
- View can render meaningful output without standings data.

Progress:

- 2026-05-13: Added the first core `TeamSeasonView` builder from schedule
  games. It projects record, points, points percentage, goal differential,
  home/away and one-goal splits, last 5/10 form, remaining home/away counts,
  next opponents, per-game rows, and explicit partial-source warnings while
  standings are unavailable.

### PT.3 - Standings Data Source

- Add fetch/cache path for standings.
- Normalize standings rows into core input DTOs.
- Compute division/conference/wild-card context.
- Compute points above/below playoff cut line and games-in-hand context.

Exit:

- Active season standings can be loaded offline after fetch.
- Missing/stale standings renders explicit source warnings.

Progress:

- 2026-05-13: Added core `TeamStandingInput` and
  `TeamStandingsContext`; `TeamSeasonView::from_games_and_standings`
  now marks standings complete when supplied and projects conference,
  division, ranks, points, points percentage, regulation wins, goal
  differential, playoff cut points, and above/behind cutline context.
- 2026-05-13: Added `icelines-fetch` standings client/parser for
  `/v1/standings/now` and `/v1/standings/{date}` with defensive NHL field
  parsing and conversion into the core standings input DTO.
- 2026-05-13: Wired CLI and web team-season surfaces to fetch current
  standings opportunistically. If standings fail, surfaces still render the
  schedule-derived view with the existing source warning.

### PT.4 - SOS And Quality Ledger

- Compute faced/remaining opponent strength.
- Add top/middle/bottom opponent buckets.
- Classify quality wins, expected wins, bad losses, missed points.
- Record basis and threshold labels in the ViewModel.

Exit:

- Tests prove opponent strength uses standings input, not route-local ranking.
- Ledger counts reconcile with final games.

### PT.5 - Surfaces

- CLI `team-season`.
- TUI Team Season screen.
- Web `/team/:abbrev/season` and JSON twin.
- Keep `/team/:abbrev` roster/depth unchanged.

Exit:

- CLI/TUI/web/JSON row identity/parity tests.
- Surface parity matrix updated.

Progress:

- 2026-05-13: Added the first web HTML and JSON route plan target in code:
  `/team/:abbrev/season` and `/api/v1/team/:abbrev/season` project the
  schedule-derived `TeamSeasonView`. The roster/depth route remains unchanged,
  and `team <ABBR> season` in the web dashboard command parser now opens the
  season-performance route while `team <ABBR> schedule` keeps the raw schedule
  list.
- 2026-05-13: Added CLI `icelines team-season <ABBR>` and `--json`. The
  command fetches the same club season schedule as the web route, builds the
  shared `TeamSeasonView`, renders a compact text summary/table, and emits the
  raw viewmodel for scripts.
- 2026-05-13: Updated the TUI `team <ABBR> season` screen to render from
  `TeamSeasonView` rather than `ScheduleTeamView`, adding points, points
  percentage, goal differential, home/away/one-goal splits, recent form,
  remaining schedule, next opponents, and explicit standings/SOS warning
  context.

### PT.6 - Dashboard Integration

- Update Jack Adams Web command parser:
  - `team EDM season` -> `/team/EDM/season`.
- Add dashboard workspace card/links where appropriate.
- Add route inventory and no-JS dashboard checks.

Exit:

- Command opens Team Season in dashboard workspace.
- Full page remains usable without JavaScript.

### PT.7 - Closeout

- Role review with `.roles`, including visual/aesthetic review.
- Update `COMMANDS.md`, README route docs, and `surface-parity.md`.
- Run focused gates:
  - core team-season tests;
  - CLI team-season tests;
  - TUI team-season tests;
  - web route/template/JSON tests;
  - dashboard command tests.

---

## Risks And Guardrails

| Risk | Guardrail |
|---|---|
| Confusing roster view with season performance | Keep `/team/:abbrev` and `/team/:abbrev/season` separate. |
| Route-local standings math | All calculations live in `TeamSeasonView` core builders. |
| SOS overclaims precision | Every SOS metric carries basis and warning state. |
| Current standings distort early season | Label sample size and games played; avoid "true team quality" language. |
| Historical seasons lack standings snapshots | Render schedule-derived view with source warnings. |
| Playoff distance ambiguity | Separate division, conference, wild-card, and games-in-hand context. |

---

## Definition Of Done

Phase Presidents Trophy is done when IceLines has a shared, tested
`TeamSeasonView` that explains team performance beyond roster depth:
record, standings context, playoff distance, home/away splits,
strength-of-schedule, quality wins/bad losses, recent form, and remaining
schedule pressure, consistently available across CLI, TUI, web HTML, web JSON,
and the Jack Adams dashboard.
