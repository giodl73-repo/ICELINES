# Phase Foster — Time Axis + Timeframes (Foster.1 + Foster.5)

**Parent**: `foster-overview.md`
**Plan**: `design/plans/2026-05-06-phaseFoster-time.md`
**Status**: Spec — ready for implementation

---

## Goal

Two cross-cutting upgrades that don't fit cleanly in any single
surface but show up everywhere date-anchored:

1. **Foster.1 — date axis** on Scores, Schedule, Playoffs. Every
   surface gains a date selector; the user can navigate to arbitrary
   past or future dates.
2. **Foster.5 — timeframe views** Day / Week / Month / Season for
   surfaces that aggregate (Favorites, Scores-summary, Stats over
   range).

NHL API supports arbitrary date queries: `/v1/score/2014-10-08`
returns 200 with full data (probed 2026-05-06).

## Foster.1 — Date axis

### URL convention (WIRE M5 — locked)

- `?date=YYYY-MM-DD` — anchor for any date-anchored surface
- `?range=week|month` — span starting at `date`
- Drop the existing `?start=` on `/schedule` for consistency

Every web surface accepting a date uses these param names.

### CLI surface

```
icelines tonight   --date 2014-10-08      # past date
icelines schedule  --date 2026-01-15      # default range = day
icelines schedule  --date 2026-01-15 --range week
icelines playoffs  --season 19931994      # already exists; no change
```

### TUI surface

**Date picker overlay** — keybind `Shift+D` (GLASS B1; `d` is taken
globally for Depth; uppercase `D` mirrors `Shift+P` season-type and
`Shift+M` docs-overlay precedent).

`Shift+D` works on Scores, Schedule, Playoffs tabs. Reuses the
existing `scores_picker_open` overlay state machine in `app.rs`
(already implemented as part of King.7); generalizes it to be
shared across the three time-anchored tabs.

```
                  ┌─── Date picker ────────────┐
                  │                              │
                  │    January 2026              │
                  │  Su Mo Tu We Th Fr Sa        │
                  │              1  2  3         │
                  │   4  5  6  7  8  9 10        │
                  │  11 12 13 14[15]16 17        │
                  │  18 19 20 21 22 23 24        │
                  │  25 26 27 28 29 30 31        │
                  │                              │
                  │  ←/→ day · ↑/↓ week          │
                  │  PgUp/PgDn month             │
                  │  Enter pick · Esc cancel     │
                  └──────────────────────────────┘
```

`Shift+D` on Playoffs tab opens a season-list picker (the season
*is* the date axis for playoffs).

### Backend reuse

`NhlApiClient::fetch_schedule_for_date(date)` already exists at
`nhl_api.rs:373`. Verify it works for arbitrary past dates (probe
showed yes); document it as the single date-fetch entry point.

## Foster.5 — Timeframes

### `Timeframe` enum (icelines-core/src/timeframe.rs — new)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Timeframe {
    Day,
    Week,
    Month,
    Season,
}

impl Timeframe {
    /// Resolve the timeframe to a date range anchored at `date`.
    /// Week starts Monday (ISO 8601, deterministic across locales).
    pub fn range(self, date: NaiveDate) -> (NaiveDate, NaiveDate) {
        match self {
            Self::Day => (date, date),
            Self::Week => {
                let monday = date - Duration::days(date.weekday().num_days_from_monday() as i64);
                (monday, monday + Duration::days(6))
            }
            Self::Month => {
                let first = date.with_day(1).unwrap();
                let last = first.checked_add_months(Months::new(1)).unwrap()
                    .pred_opt().unwrap();   // last day of month
                (first, last)
            }
            Self::Season => {
                /* NHL season: October 1 of `season_year` through June of `season_year + 1` */
                /* Inferred from the date — Oct/Nov/Dec → start = Oct 1 of date.year() */
            }
        }
    }
}
```

### Timeframe × `--filter` ambiguity (EDGE B1 — locked resolution)

When `--week` or `--month` is set, **bare filter aliases bind to the
active timeframe**, not season totals:

```bash
# Without a timeframe flag: season totals (existing behavior)
icelines query leaders --filter "g>=10"

# With --week: "10+ goals THIS WEEK"
icelines query leaders --week --filter "g>=10"

# Explicit override: namespaced atom
icelines query leaders --filter "g.season>=10"        # always season
icelines query leaders --filter "g.week>=10"          # always week
```

Grammar extension (icelines-core/src/stats_catalog.rs):

```
<atom>     := <stat-key> [ '.' <window> ] <op> <value>
<window>   := 'season' | 'week' | 'month' | 'day'
```

When `<window>` is omitted, resolves to the active CLI timeframe
flag, defaulting to `season` if no flag.

`apply_views` gains a third bucket: `windowed_filters: Vec<WindowedAtom>`
alongside `stat_filters` and `expr_filters`. AND'd with the others
(EDGE L1).

### Timeframe rejection on `query career` (EDGE B2)

`query career --week` rejects with a clear error:

```
error: --week / --month not supported on `query career` (junior seasons
       aren't aligned with NHL week boundaries). Use --season instead.
```

L2 test required.

### CLI `--week` / `--month` flags

Land on:
- `query leaders` (EDGE B1 grammar fix)
- `query goalies` (same)
- `tonight` (single-day; `--week` widens to 7-day score grid)
- `schedule` (already has `--days`; `--week` is sugar for `--days 7`)
- `favorites` (Foster.2)

Reject on:
- `query player`, `query compare`, `query career`, `playoffs` (no
  meaningful window semantics)

### TUI surface

**Timeframe selector key** — `v` (GLASS B2 — `t` already used by
Scores/Schedule/Transactions for "today" / team filter). `v`
cycles Day → Week → Month → Season → Day.

Active timeframe rendered in the **status bar** (chunks[2], existing
status string), not a new top-bar widget — chunks[0] is nav-tabs and
already tight (GLASS L8):

```
[L]eague | [D]epth | … | [F]avorites · 25-26 Regular · Week (Mon-Sun) · refresh
```

`v` on the Favorites tab cycles its date range. `v` on Scores cycles
the score-summary aggregation window.

### Web URL convention

```
/scores?date=2014-10-08&range=day        # default (range=day implicit)
/scores?date=2014-10-08&range=week        # week starting 2014-10-06 (Mon)
/favorites?date=2026-01-15&range=week
```

`range=day` is the default and may be omitted. `range=season` is
valid only on routes where a single season makes sense (avoids
deriving from a date).

## Test plan

**Foster.1 = 12 tests**:

- L0 date parsing (3): valid YYYY-MM-DD, invalid format, far-past
  date (2014-01-01)
- L1 mock NHL API for past date (4): mounted `/v1/score/2014-10-08`
  fixture, mounted `/v1/schedule/2014-10-08` fixture, mounted
  `/v1/score/2024-12-01`, mounted `/v1/schedule/2026-01-15`. Three
  frozen JSON fixtures committed under `icelines-fetch/tests/fixtures/dates/`
  (~30 KB each)
- L1 web routes (3): `/scores?date=2014-10-08`, `/schedule?date=…`,
  `/playoffs?season=19931994` round-trip
- L2 CLI (2): `tonight --date 2014-10-08` exits 0 with games rendered;
  invalid date format surfaces clean error

**Foster.5 = 9 tests**:

- L0 Timeframe range computation (4): Day, Week (Monday-anchored),
  Month (28/30/31-day boundaries), Season (Oct→Jun)
- L0 DST + week-start edge cases (3) (BENCH L1): DST transition
  Sunday-Monday boundary, year-boundary week (Dec 31 → Jan 1),
  leap-year February
- L1 web `/favorites?range=week` (1): renders 7 days
- L2 CLI (1): `query leaders --week --filter "g>=5"` rejects when
  --week not yet supported on that subcommand (or accepts when it
  is — flip when implementation lands)

**Foster.1 + .5 total = 21 tests.**

## Files added

```
icelines-core/src/timeframe.rs                 (~120 lines)
icelines-fetch/tests/fixtures/dates/            (3 frozen JSON files)
icelines-cli/src/commands/tonight.rs (extended) — already exists; +30 lines for --date
icelines-cli/src/commands/schedule.rs (extended) — +30 lines for --date / --range
icelines-cli/src/tui/screens/datepicker.rs     (~100 lines, generalize from scores_picker)
icelines-cli/tests/foster_time.rs              (~250 lines)
```

## Open items

1. **`Timeframe::Season` semantics for an offseason date** — if user
   passes `--date 2026-07-15` (July, no NHL season active), what's
   the "season" range? Recommend: nearest enclosing season (Oct 2025
   → Jun 2026). Document in COMMANDS.md.
2. **Cross-year week boundaries** — week containing Dec 31 → Jan 1.
   ISO 8601 says it belongs to the year of its Thursday. Confirm
   `chrono::NaiveDate::iso_week()` behavior in tests.
3. **Future-date queries** — what does `--date 2027-10-08` return
   when the API has nothing scheduled? NHL API returns an empty
   `gameWeek`. Recommend: render "No games scheduled" (matches
   existing tonight-empty-night behavior).
