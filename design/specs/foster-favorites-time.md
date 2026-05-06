# Phase Foster — Favorites, Time-Travel, Unified Data Layer (SUPERSEDED)

**Trophy**: Foster Hewitt Memorial Award (broadcaster — "keeping you informed")
**Version**: 0.1 (draft, superseded 2026-05-06 by the four-doc split)
**Date**: 2026-05-06
**Status**: SUPERSEDED — kept for review-history reference only.

Replaced by:
- `foster-overview.md` — phase-level vision + sub-phase ordering
- `foster-data-architecture.md` — DataStore, manifest, capabilities, EntityRef
- `foster-favorites-dashboard.md` — FavoritesView, EventStream, per-night lines
- `foster-time-and-timeframes.md` — date axis + Day/Week/Month timeframes

Two rounds of role reviews (TAPE/FORGE/GLASS/WIRE + SCOUT/EDGE/BENCH/PACE)
surfaced 19 blocking findings. The split keeps each doc focused on one
concern and makes the cross-cutting items (capability matrix, time
axis, EventStream) live with their owning spec.

**Plan**: `design/plans/2026-05-06-phaseFoster-plan.md` (also superseded
by per-spec plans).

---

## Goal

Three intertwined product capabilities, plus the data-layer cleanup we've
been deferring since Hart:

1. **Favorites dashboard** — one page per surface (CLI / TUI / Web) that
   answers "what's happening with the people and teams I care about,
   right now (or on any past date)." Pulls scores, stat lines, recent
   transactions, milestones for each favorited entity.
2. **Time-travel** — every date-anchored surface (Scores, Schedule,
   Playoffs, Favorites) accepts a date selector and shows what was true
   on that day. NHL API serves arbitrary dates back to ≥2014 (probed).
3. **Timeframe views** — Day / Week / Month / Season selectors on the
   surfaces that benefit. Favorites starts with Day + Week.
4. **Unified data layer** — replace the *bundle / snapshot / `data
   install`* triad with one model: a per-install `DataStore` rooted at
   `~/.icelines/data/` with a `manifest.json` that tracks every dataset's
   freshness and source. First-run setup pulls defaults; lazy-load fills
   gaps.

The NHL API probe (2026-05-06) confirmed `/v1/score/{date}` and
`/v1/schedule/{date}` return real data for arbitrary past dates, so
time-travel is achievable without scraping or third-party sources.

---

## Surfaces

| Capability | CLI | TUI | Web |
|---|---|---|---|
| Favorites dashboard | `icelines favorites [--date D] [--week]` | New tab `f` | `/favorites?date=D&view=week` |
| Time-travel scores | `icelines tonight --date 2014-10-08` | Scores tab `d` opens date picker | `/scores?date=2014-10-08` |
| Time-travel schedule | `icelines schedule --start 2014-10-08` | Schedule tab `d` | `/schedule?start=2014-10-08` |
| Timeframe selector | `--week` / `--month` flags | Top-bar selector | `?view=week|month` query |
| Setup hook | `icelines setup` (also auto on first run) | First-run prompt overlay | n/a (server expects local data) |
| Data manifest | `icelines data status` | Settings overlay (`R`) | `/admin/data` (local-only) |

Surface coverage doctrine: **CLI ✅ TUI ✅ Web ✅** for every capability
that's date- or entity-anchored. Setup hook is CLI/TUI only — the web
server presumes data already exists locally.

---

## Data model upgrades (Foster.0)

Add **additively**, alongside existing types — don't refactor
StatsRepository. Pattern proven by Calder's parallel store.

### A. `EntityRef` enum (`icelines-core/src/entity.rs` — new module)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityRef {
    Player(PlayerId),
    Team(TeamAbbr),
    Game(GameId),
    // Coach, Conference, Division, DraftYear — defer; add when used.
}
```

Replaces today's ad-hoc discriminators (groups' `kind` column,
favorites' implicit player-only assumption, scattered `u32`/`String`
keys). Migration: `groups.group_members` schema gets a third column
`entity_kind` mirroring `kind` but using the EntityRef serialization.

### B. `Freshness` (`icelines-core/src/freshness.rs` — new module)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Freshness {
    pub fetched_at: DateTime<Utc>,
    pub source: FetchSource,        // Setup | Live | DataInstall | Manual
    pub ttl_hint: Option<Duration>, // None = never auto-refreshes
}

pub enum FetchSource { Setup, Live, DataInstall, Manual }
```

Carried as a sidecar in the manifest, not on every record. One
`Freshness` per (kind, season-or-date). Foster.4 sync reads this to
decide what's stale.

### C. `GameLog` / `BoxscoreDay` (`icelines-fetch/src/boxscore.rs` —
already exists; expand to per-day storage)

Per-game stat lines stored at `~/.icelines/data/boxscores/<YYYY-MM-DD>/<game_id>.json`
or in a per-day SQLite table. Foster.2 favorites dashboard reads from
this. Foster.3 populates it for favorited entities.

### D. `EventStream` (SQLite table — new)

```sql
CREATE TABLE events (
    date         TEXT NOT NULL,           -- YYYY-MM-DD
    entity_kind  TEXT NOT NULL,           -- 'player' | 'team' | 'game'
    entity_key   TEXT NOT NULL,           -- pid / abbrev / game_id
    event_kind   TEXT NOT NULL,           -- 'score' | 'trade' | 'milestone' | 'streak' | …
    payload      TEXT NOT NULL,           -- JSON blob
    created_at   TEXT NOT NULL,
    PRIMARY KEY (date, entity_kind, entity_key, event_kind, payload)
);
CREATE INDEX events_by_date ON events(date);
CREATE INDEX events_by_entity ON events(entity_kind, entity_key, date);
```

One row per (entity, event) on a given date. `payload` carries the
event-kind-specific blob (score line, trade description, milestone
text). Foster.2's "what's new for my favorites today" is one indexed
query.

Existing transactions feed pours into this on fetch. New score-row
insertions on each `tonight`/`fetch sync` run.

### E. Unified `DataStore` (`icelines-fetch/src/datastore.rs` — new)

Replaces direct `BUNDLED_*` and `SnapshotStore` access. Single entry
point:

```rust
impl DataStore {
    pub fn load_bios(&self, season: Season) -> Result<Vec<SkaterBio>, DataError>;
    pub fn load_stats(&self, season: Season, type_: SeasonType) -> Result<…>;
    pub fn load_career_history(&self, pid: PlayerId) -> Option<CareerHistory>;
    pub fn load_boxscore(&self, game_id: GameId) -> Result<…>;
    pub fn freshness(&self, kind: DataKind, key: DataKey) -> Option<Freshness>;
    pub fn list_seasons(&self) -> Vec<Season>;
    pub fn manifest(&self) -> &Manifest;
}
```

Underneath:
1. Look in `~/.icelines/data/` first (manifest-tracked).
2. Fall back to embedded **minimal bundle** (current season only —
   ~1.5 MB instead of 56 MB).
3. If `live_feeds_enabled()` and missing, lazy-fetch from NHL API,
   write to manifest, return.

`Manifest` JSON shape:

```json
{
  "schema_version": 1,
  "datasets": [
    {"kind": "bios", "season": "20252026", "fetched_at": "2026-05-06T00:00:00Z",
     "source": "setup", "ttl_hint_secs": 21600, "path": "seasons/20252026/bios.json"},
    {"kind": "career_history", "fetched_at": "...", "source": "manual", ...},
    {"kind": "boxscore", "game_id": 2025020001, "date": "2025-10-08", ...}
  ]
}
```

**Bundle reduction**: `BUNDLED_SEASONS` shrinks from 38 → 1 (current
season). Everything else moves to setup-time fetch. Binary drops from
56 MB → ~5 MB. Users who want offline-everything run `icelines fetch
all --bundled-seasons 38` once.

---

## Surface specs

### Favorites dashboard (Foster.2)

**CLI** — `icelines favorites [--date D] [--week] [--json]`

```
FAVORITES — 2026-01-15 (Mon)
══════════════════════════════════════════════════════════════
Players (3 favorited)
  Connor McDavid     EDM 7-3 W vs CGY    1G 2A 3P  TOI 22:14  +2  4 SOG
  Macklin Celebrini  SJS 2-4 L @ STL     0G 1A 1P  TOI 19:01  -1
  Brad Marchand      FLA — DNP (rest)
Teams (2 favorited)
  EDM                7-3 W vs CGY        Skinner 32 SV  Top: McDavid 3P
  TOR                — bye

Last 7 days for your favorites:
  McDavid: 7 GP · 5G 12A 17P · +8 · 13.7 SOG/g
  Celebrini: 7 GP · 3G 4A 7P · -3 · 4.0 SOG/g
  EDM: 5-2-0 · +9 GD · 3rd in Pacific
```

**TUI** — new tab `f` between Stats and Scores.

**Web** — `/favorites` HTML page with date picker + Day/Week toggle;
`/api/v1/favorites` JSON twin per WIRE convention.

### Time-travel on Scores (Foster.1)

CLI: `icelines tonight --date 2014-10-08` (re-uses the existing
command path; just plumbs a date through `fetch_today_schedule`).
TUI: `d` keybind on Scores tab opens a date picker overlay.
Web: `/scores?date=2014-10-08` query param.

### Setup hook (Foster.0)

```
$ icelines tui
First run detected. Download default dataset?
  [1] Minimal — current season + last 5 (recommended, ~30 MB, ~2 min)
  [2] Recent  — last 10 seasons (~60 MB, ~4 min)
  [3] Full    — all 38 seasons (~250 MB, ~15 min)
  [4] Skip    — lazy-load on demand
Choice [1]:
```

Choice writes `~/.icelines/data/` and updates manifest. Subsequent
runs skip the prompt unless the manifest is empty or `icelines
setup --force` is run.

---

## Out of scope

- **Live websocket pushes** — Foster polls. Notifications are an
  explicit non-goal in IceLines.md.
- **Multi-user accounts** — single-user local stays the v1 stance.
- **Cloud sync of favorites** — favorites live in the local SQLite db.
  Export/import via the existing `group export` / `group import` is
  the manual-sync surface.
- **Pre-2014 historical scores** — needs further API probing; defer
  if NHL endpoints don't reach back.

---

## Open questions

1. **EntityRef serialization shape** — discriminated union (`{"kind":
   "Player", "id": 8478402}`) or stringly-typed (`"player:8478402"`)?
   The latter is more compact in JSON envelopes; the former is
   self-documenting. Lean stringly-typed for envelopes, struct
   internally.
2. **EventStream insertion latency** — synchronous in `tonight` /
   `fetch sync`, or via a background job? Sync is simpler; background
   adds complexity that Foster.4 might want anyway.
3. **Timeframe aggregation source** — sum on the fly from per-game data
   (cheap, accurate), or pre-compute weekly tables (faster on big
   ranges, more storage). Lean on-the-fly for v1; pre-compute only if
   measurable on real data.
4. **Setup prompt UX in TUI** — block the alt-screen until the user
   chooses, or render the prompt in the alt-screen and route input
   through the normal event loop? Likely block on a stdin read before
   entering raw mode.
5. **Bundle deletion blast radius** — every existing test that calls
   `BUNDLED_SEASONS` becomes a `DataStore` test. Big mechanical
   refactor, but the tests get cleaner (no more `data install` vs
   bundle vs snapshot ceremony).

---

## Sub-phases

See `design/plans/2026-05-06-phaseFoster-plan.md`.
- **Foster.0** — Data-model groundwork (EntityRef, Freshness, DataStore,
  manifest) + bundle reduction. ~3-4 days.
- **Foster.1** — Time axis on Scores/Schedule/Playoffs. ~2 days.
- **Foster.2** — Favorites dashboard (CLI + TUI + Web). ~3 days.
- **Foster.3** — Per-game boxscore fetching for favorited entities. ~2 days.
- **Foster.4** — Sync layer (background freshness + `fetch sync`). ~2 days.
- **Foster.5** — Timeframe views (Day/Week/Month). ~2 days.
- **Foster.6** — Setup wizard polish + docs + persona pass. ~1 day.

Total: ~14-16 days. Largest phase yet, but it's also the data-model
capstone and shouldn't repeat for v1.

---

## Success criteria

1. `icelines favorites` works on a fresh install after `icelines setup`,
   shows real data for the user's favorited players + teams.
2. Time-travel: `icelines tonight --date 2014-10-08` renders that night's
   8 games (verified via API probe).
3. Binary size drops from ~56 MB to ~5 MB; `icelines setup` fills
   `~/.icelines/data/` to ~30 MB on the default choice.
4. `data install` continues to work as a power-user power-tool but is
   no longer the primary data path.
5. All workspace tests green, including ~20 new tests targeting the
   new data layer.
