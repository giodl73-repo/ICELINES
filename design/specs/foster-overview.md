# Phase Foster — Overview

**Trophy**: Foster Hewitt Memorial Award (broadcaster — "keeping you informed")
**Version**: 1.0 (post-review revision)
**Date**: 2026-05-06
**Status**: Spec — ready for implementation
**Predecessor**: `foster-favorites-time.md` (superseded after 8-role review)
**Detail specs**:
- `foster-data-architecture.md` — Foster.0 foundation
- `foster-favorites-dashboard.md` — Foster.2 / .3 / .4
- `foster-time-and-timeframes.md` — Foster.1 / .5
**Plan**: `design/plans/2026-05-06-phaseFoster-overview.md` (orchestrator)

---

## Vision in one paragraph

Add a Favorites dashboard that shows what's happening with the players
and teams the user cares about — tonight's stat lines, last night's
scores, recent transactions, milestones — for any date past or
present. Land it on top of a unified data layer that replaces today's
*bundle / snapshot / `data install`* triad with one cache, one
manifest, one set of read paths. NHL API serves arbitrary past dates
(probed: `/v1/score/2014-10-08` returns 200) so time-travel is
achievable without scraping.

## Locked decisions (carried into the detail specs)

| Decision | Choice | Source |
|---|---|---|
| Bundle size | **Keep 38 seasons (~56 MB)** — status quo | User, post-TAPE review |
| Architecture | Bundle = first cache layer; one DataStore reads it like everything else | User insight, post-PACE/TAPE |
| Default sync policy | **Eager** — auto-refresh on launch past TTL, **non-blocking** background task | User + PACE B1 |
| Banner | Summary line ("Refreshed 4 datasets · 2.1 s") via one-shot channel | User |
| Capability defaults | `transactions=favorites`, `boxscores=favorites`, `shifts=off`, `career_history=favorites`, `stats`/`scores_schedule=league` | User |
| Real shift parsing | **DEFERRED to a future phase** — `shifts` capability stays in the matrix as off-only; existing `shift_profile.rs` is per-game co-appearance, NOT per-shift, and would mislead. SCOUT-flagged. | SCOUT 6, deferred |
| Setup wizard | Alt-screen modal (mirror `render_reports_overlay`); not pre-raw-mode prompt | GLASS M5 |
| Season-transition | Prompt on detect (`/v1/standings/now` advances) | User |
| EntityRef encoding | **Stringly-typed everywhere** (`player:8478402`, `team:EDM`, `game:2025020001`); `Display`/`FromStr` round-trip; serde delegated; URL-safe | FORGE H1 + WIRE H3 |
| Manifest data | **Sharded by kind**, HashMap-indexed in `OnceLock`; `manifest/{bios,stats,boxscores,…}.json` | PACE B2 |

## Sub-phase ordering

```
Foster.0 ─── data architecture (foundation; everything depends on it)
                │
                ├─→ Foster.1 ─── time axis (Scores/Schedule/Playoffs)
                │
                └─→ Foster.2 ─── favorites dashboard
                       │
                       ├─→ Foster.3 ─── boxscore + EventStream population
                       │
                       └─→ Foster.4 ─── sync layer (background refresh)

Foster.5 ─── timeframe views (Day/Week/Month) — depends on .1 + .2
Foster.6 ─── setup wizard polish + docs + persona pass
```

Critical-path: F.0 → F.1 → F.2 → F.3. F.4 / F.5 can land in parallel
once F.3 is in.

## Out of scope (deferred to post-Foster phases)

- **Real per-shift parsing** — NHL HTML shift reports. Phase candidate:
  *Phase Maurice Richard* (skating speed already parked there). Surfaces
  as a `shift_profile.rs` rewrite that swaps per-game co-appearance
  for actual shift sequences. Foster's `shifts` capability is reserved
  in the matrix but cannot be set to anything but `off` until that
  phase lands.
- **Pre-2014 historical scores** — depends on further API probing; if
  the score endpoint stops working before some date, that becomes the
  hard floor.
- **Multi-user / cloud sync** — explicit IceLines.md non-goal; favorites
  stay local in `~/.icelines/icelines.db`.
- **Live websocket pushes** — Foster polls. Notifications opt-in only.
- **League-wide boxscore bulk-fetch** — capability matrix allows
  `boxscores=league` but the F.3 implementation only exposes
  `favorites` and `off`. League-wide is a follow-on (storage growth +
  rate-limit considerations).

## Surface coverage matrix

| Capability | CLI | TUI | Web |
|---|---|---|---|
| Favorites dashboard | `icelines favorites [--date D] [--week]` | New tab `Shift+F` (admin moves to `Shift+A`) | `/favorites?date=D&range=week` |
| Time-travel scores | `icelines tonight --date 2014-10-08` | `Shift+D` opens date picker | `/scores?date=2014-10-08` |
| Time-travel schedule | `icelines schedule --date 2014-10-08` | `Shift+D` | `/schedule?date=2014-10-08` |
| Time-travel playoffs | `icelines playoffs --season 19931994` (existing) | `Shift+D` jumps to season | `/playoffs?season=19931994` |
| Timeframe selector | `--week` / `--month` flags | `v` cycles Day↔Week↔Month↔Season; status-bar indicator | `?range=week|month` |
| Setup wizard | `icelines setup` (also auto on first run when manifest is empty) | First-run alt-screen modal | n/a (server expects local data) |
| Data manifest | `icelines data status` | Settings overlay (`R`) extended | `/admin/data` (local-only) |

Every capability ships on **CLI ✅ TUI ✅ Web ✅** by default. Setup
wizard is CLI/TUI only — server presumes data exists.

## Sub-phase summaries

### Foster.0 — Data architecture (~5 days, +50% over original estimate)

See `foster-data-architecture.md` for detail. Headline: DataStore
with sharded HashMap-indexed manifest in OnceLock, EntityRef as
stringly-typed enum, capability matrix in `[sync.capabilities]`,
setup wizard, snapshot read-shim.

**Test budget**: 35 + 24 capability matrix = **59 tests**.

### Foster.1 — Time axis (~3 days, +50%)

See `foster-time-and-timeframes.md` for detail. Headline: `Shift+D`
date picker overlay on Scores/Schedule, frozen JSON fixtures for 3
historical dates, query-param convention (`?date=` + `?range=`).

**Test budget**: 12 tests.

### Foster.2 — Favorites dashboard (~4 days)

See `foster-favorites-dashboard.md` for detail. Headline: separate
`SkaterNightLine` and `GoalieNightLine` schemas, mid-day trade
attribution rule, DNP/scratched/absent classification, heterogeneous
JSON envelope explicitly documented.

**Test budget**: 18 tests + 6 personas.

### Foster.3 — Boxscores + EventStream (~3 days, +50%)

See `foster-favorites-dashboard.md` for detail. Headline: per-game
boxscore fetch keyed on `(date, game_id)`, EventStream with
caller-supplied `event_id` + `payload_version` per event_kind,
frozen schemas for `score`/`trade`/`milestone`/`streak`.

**Test budget**: 12 tests.

### Foster.4 — Sync layer (~3 days, +50%)

See `foster-data-architecture.md` §"Sync engine" for detail.
Headline: **non-blocking** background refresh via tokio::spawn +
one-shot channel, `MockClock` injected for test determinism,
`ICELINES_TEST_MODE=1` gates auto-sync in test runs.

**Test budget**: 15 tests.

### Foster.5 — Timeframe views (~2 days)

See `foster-time-and-timeframes.md` for detail. Headline:
`Timeframe::{Day,Week,Month,Season}` with `range(date) → (start, end)`;
bare filter aliases (`g`, `p`) bind to active timeframe when
`--week`/`--month` set; rejected on `query career`.

**Test budget**: 9 tests.

### Foster.6 — Setup wizard polish + docs + persona pass (~2 days, +100%)

Headline: docs refresh (COMMANDS.md, README, CLAUDE.md), 30 persona
scenarios mirroring `persona_wave3.rs` density.

**Test budget**: 4 unit + 30 personas = 34 tests.

## Total budget

**~16 working days** (4-week-aware), **~159 tests** (~3× the original
~59). Largest phase yet — also the data-model capstone, expected not
to repeat for v1.

## Pre-flight checklist

- [x] NHL API probe — `/v1/score/{date}` returns 200 for arbitrary past dates
- [x] Groups+teams (`MemberKind`) committed (`ab8903bf`) — Foster.0
      collapses this into `EntityRef` via migration 006
- [x] Two rounds of 8-role reviews complete; 19 blockers captured + addressed
- [ ] Final lint pass on all four specs by the user
- [ ] Foster.0 starts (data architecture)

## Cross-cutting open items

These don't block Foster.0 but need decisions before the relevant
sub-phase:

1. **Time-travel URL convention** — locked: `?date=YYYY-MM-DD` for
   anchor, `?range=week|month` for span. Drop the existing `?start=`
   on `/schedule` for consistency.
2. **`/api/v1/favorites` envelope shape** — `data` is heterogeneous
   (`{players: [], teams: [], events: []}`), explicitly breaking the
   homogeneous-array convention. Documented in
   `foster-favorites-dashboard.md`.
3. **Clock injection for tests** — `Freshness::now()` reads from a
   `Clock` trait; production uses `SystemClock`, tests use
   `MockClock`. `ICELINES_TEST_MODE=1` env var disables auto-sync
   regardless of policy. Documented in `foster-data-architecture.md`.
