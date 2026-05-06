# Phase Foster — Time axis + Timeframes plan (Foster.1 + Foster.5)

**Spec**: `design/specs/foster-time-and-timeframes.md`
**Test budget**: 12 (F.1) + 9 (F.5) = **21 tests**

---

## Foster.1 — Date axis on Scores / Schedule / Playoffs

### F.1.1 — Frozen JSON fixtures

- Capture three NHL API responses to disk under
  `icelines-fetch/tests/fixtures/dates/`:
  - `score-2014-10-08.json` (~30 KB)
  - `schedule-2014-10-08.json` (~40 KB)
  - `score-2024-12-01.json`
- Used by httpmock-backed L1 tests for deterministic time-travel.

### F.1.2 — Verify `fetch_schedule_for_date` works for arbitrary dates

- Existing method at `nhl_api.rs:373` already accepts a date.
- Add doc comment: "verified for arbitrary past dates back to ≥2014".
- L1 mock test against the frozen fixtures (4 tests).

### F.1.3 — CLI `--date` flags

- `tonight --date YYYY-MM-DD` — already extends; add `--date` argument
- `schedule --date YYYY-MM-DD --range day|week|month` — replaces `--start`
  - Migrate `--start` to a deprecated alias of `--date`
- `playoffs --season YYYYZZZZ` already accepts a season (no change)
- **Tests (2 L2)**: `tonight --date 2014-10-08` exits 0 with games rendered; invalid date format → clean error

### F.1.4 — TUI date picker overlay

- Generalize `scores_picker_open` from `app.rs` (King.7 work) into a
  shared `DatePickerState` reusable by Scores / Schedule / Playoffs tabs
- Keybind: `Shift+D` (mirror `Shift+P` season-type, `Shift+M` docs-overlay
  precedent — GLASS B1)
- `d` lowercase stays as global Depth shortcut (don't break existing UX)
- Render: month grid, ←/→ day, ↑/↓ week, PgUp/PgDn month, Enter pick, Esc cancel
- **Tests (3 L1 web)**: `/scores?date=…`, `/schedule?date=…`, `/playoffs?season=…`

### F.1.5 — Web URL convention

- `?date=YYYY-MM-DD` everywhere (anchor)
- `?range=week|month` for span (default `day` is implicit)
- `/schedule?start=…` becomes a deprecated alias
- 400 on bad date format with helpful body
- **Tests (3 L0)**: valid date, invalid format, far-past date

## Foster.5 — Timeframes

### F.5.1 — `Timeframe` enum (icelines-core/src/timeframe.rs)

- `enum Timeframe { Day, Week, Month, Season }`
- `range(date) -> (NaiveDate, NaiveDate)` resolver
- Week starts Monday (ISO 8601, deterministic)
- Month: first → last day of date's month (handles 28/29/30/31)
- Season: nearest enclosing NHL season (Oct N → Jun N+1)
- **Tests (4 L0)**: Day, Week (Monday-anchored), Month (28/30/31), Season (Oct→Jun)

### F.5.2 — Timeframe × `--filter` grammar (EDGE B1)

- Extend `parse_filter_expr` in `stats_catalog.rs` to accept namespaced atoms:
  - `<atom> := <stat-key> [ '.' <window> ] <op> <value>`
  - `<window> := 'season' | 'week' | 'month' | 'day'`
- Bare alias (no namespace) binds to active CLI timeframe; defaults to `season`
- New `WindowedAtom` type alongside `StatFilter` and `FilterExpr`
- `apply_views` gains a `windowed_filters: Vec<WindowedAtom>` bucket; AND'd with the others
- Reject `--week`/`--month` on `query career` with clear error (EDGE B2)
- **Tests (3 L0 + 1 L2)**: namespaced atom parses, bare-alias-binds-to-active-window, `query career --week` rejects, query leaders mixed-window filter

### F.5.3 — TUI timeframe selector

- Keybind: `v` cycles Day → Week → Month → Season → Day (GLASS B2 — `t` is taken)
- Render active timeframe in **status bar** (chunks[2]) — append `· Week (Mon-Sun)` or similar (GLASS L8)
- Don't add a new top-bar widget (chunks[0] is tight)
- **Tests (1 L1 render smoke)**: timeframe label appears in status

### F.5.4 — DST + week-start edge cases (BENCH L1)

- DST transition Sunday-Monday boundary
- Year-boundary week (Dec 31 → Jan 1, ISO 8601 says belongs to year-of-Thursday)
- Leap-year February (Feb 29 only valid in leap years)
- **Tests (3 L0)**: covers all three

### F.5.5 — Timeframe applies to surfaces

Land on:
- `query leaders` (windowed grammar)
- `query goalies` (same)
- `tonight` / `schedule` (`--week` widens to 7-day grid)
- `favorites` (Foster.2)

Reject on:
- `query player`, `query compare`, `query career`, `playoffs`

## Files added/modified

```
icelines-core/src/timeframe.rs                       ~120 lines  (new)
icelines-core/src/stats_catalog.rs                   +60 lines for windowed-atom grammar
icelines-cli/src/commands/tonight.rs                 +30 lines for --date
icelines-cli/src/commands/schedule.rs                +30 lines for --date / --range
icelines-cli/src/commands/query.rs                   +40 lines for --week/--month plumbing
icelines-cli/src/tui/screens/datepicker.rs           ~100 lines  (new — extracted from scores_picker)
icelines-fetch/tests/fixtures/dates/                 3 frozen JSON files
icelines-cli/tests/foster_time.rs                    ~250 lines  (new)
```

## Acceptance for Foster.1

- `icelines tonight --date 2014-10-08` renders that night's 8 games
- `/scores?date=2014-10-08` and `/schedule?date=2014-10-08` both work
- TUI `Shift+D` opens date picker on Scores/Schedule/Playoffs; `d` (lowercase) still opens Depth
- `?start=` works as deprecated alias on `/schedule`
- 12 tests pass

## Acceptance for Foster.5

- `icelines query leaders --week --filter "g>=10"` interprets `g` as week-window goal count
- `icelines query career --week` rejects with the documented error
- `query leaders --filter "g.season>=10 AND g.week>=5"` works (mixed-window)
- TUI `v` cycles timeframes with status-bar indicator
- 9 tests pass
