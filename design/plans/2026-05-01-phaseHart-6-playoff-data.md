# Phase Hart.6 — Playoff Per-Player Data (v0.1, pre-review)

**Status**: Draft v0.1 — needs forge / tape / wire / bench review
**Date**: 2026-05-01
**Trophy**: Hart (final sub-phase — completes the original Hart goal)
**Predecessor**: design/plans/2026-04-30-phaseHart-normalization.md (master),
design/plans/2026-05-01-phaseHart-5c-final-cleanup.md
**Replaces**: nothing — additive

---

## Goal

Populate `StatsRepository` with per-player **playoff** stats for the
five bundled seasons (and live playoff fetch for the current season),
so a TUI/CLI season-type toggle from "Regular" to "Playoff" returns
real numbers instead of `LoadError::MissingBundle`.

The original motivation for Phase Hart was making `(season, season_type)`
a first-class axis. Hart.5c completes the **consumer migration**.
Hart.6 completes the **data half** — without it, the entire Phase Hart
investment is structurally complete but functionally unused for playoffs.

## Pre-conditions

- Hart.5c lands first. After 5c the model layer accepts playoff stats
  natively and every consumer reads through `(season, season_type)`-aware
  accessors. Hart.6 only needs to populate; no consumer changes.
- `SeasonType::Playoff` exists (`icelines-core/src/season_stats.rs:34`).
- `App::active_type` is owned by App (Hart.5c.6).
- L1 fixture builders already produce playoff `SeasonStats` (Hart.4.1).

## Why this is a separate phase

Hart.5c is a 8-step consumer migration that must land independently
green. Hart.6 is a data-source addition that touches `nhl_api.rs`,
`bundled.rs`, `snapshot.rs`, `stats_loader.rs`, and the `fetch` CLI.
These are largely orthogonal codepaths from Hart.5c. Bundling them
would inflate review surface area for no shared dependency.

---

## Current state (what already exists)

**API client** (`icelines-fetch/src/nhl_api.rs`):
- `fetch_all_bios(season)` — hardcodes `gameTypeId=2`, line 117
- `fetch_all_stats(season)` — hardcodes `gameTypeId=2`, line 126
- `fetch_all_realtime(season)` — hardcodes `gameTypeId=2`, line 158
- `fetch_all_goalies(season)` — hardcodes `gameTypeId=2`, line 169
- `fetch_playoff_bracket(year)` — exists, returns bracket/series data
  (NOT per-player stats)

**Bundled data** (`icelines-fetch/src/bundled.rs`):
- `BIOS_<season>` / `STATS_<season>` / `GOALIES_<season>` static byte
  arrays — five seasons embedded, all regular-season only
- `BUNDLED_PLAYOFFS` — series/bracket data only (1993-94 fixture for now)
- No per-player playoff stats embedded

**Loader** (`icelines-fetch/src/stats_loader.rs`):
- `load_into_repo(season, season_type, store)` — line 120
- Lines 127-133: explicit early-return `LoadError::MissingBundle` for
  `season_type == SeasonType::Playoff`
- Test `l0_hart3_playoff_returns_missing_bundle_for_now` (line 636) pins
  this behavior

**Snapshot tier layout** (`icelines-fetch/src/snapshot.rs`):
- File-per-tier: `stats/bios.json`, `stats/stats.json`,
  `realtime/realtime.json`, etc. Implicitly regular-season only.

**Fetch CLI** (`icelines-cli/src/commands/fetch.rs`):
- `FetchSubcommand::{Rosters, Stats, Realtime, Goalies, Contracts,
  Transactions, MoneyPuck, Positions, All}`
- All assume gameTypeId=2; no playoff variant.

## Scope

In:
- NHL API client — playoff URL variants for bios, stats, goalies
- Bundled data — author + embed `playoff-stats.json`,
  `playoff-goalie-stats.json` for the five bundled seasons
- Snapshot store — type-aware tier paths (e.g.
  `stats/playoff-stats.json` or `stats/playoff/stats.json`)
- `load_into_repo` — remove early bail, route by season_type
- Fetch CLI — `--type playoff` flag (or a new subcommand) on stats /
  goalies / rosters
- Tests — L0 fixture, L1 mock NHL API for playoff path, L2 for the
  fetch + load round-trip
- TUI hookup — `y` season picker continues to handle type toggle
  (already wired by Hart.5c.6); confirm end-to-end

Out:
- Realtime stats for playoffs (the realtime endpoint is regular-season
  game-day data; playoff games update through the same endpoint live,
  but historical playoff "realtime" doesn't exist as a separate dataset)
- Per-game playoff stats (this phase ships totals only — same shape as
  regular-season stats)
- MoneyPuck playoff stats (separate decision below — see "Open question")
- Contracts (single source per player; not type-keyed)
- Pre-1991 historical playoff data (NHL API coverage thins out;
  see Risk #4)

---

## Decisions to make

### D1 — URL parameterization

**Problem**: API client URL-builders hardcode `gameTypeId=2`.

**Recommendation**: take `season_type: SeasonType` as a parameter on
each fetch fn. New signatures:

```rust
impl NhlApiClient {
    pub async fn fetch_all_bios(&self, season: &str, ty: SeasonType)
        -> Result<Vec<SkaterBio>, FetchError>;
    pub async fn fetch_all_stats(&self, season: &str, ty: SeasonType)
        -> Result<Vec<SkaterStats>, FetchError>;
    pub async fn fetch_all_goalies(&self, season: &str, ty: SeasonType)
        -> Result<Vec<GoalieStats>, FetchError>;
    // realtime stays gameTypeId=2-only — playoff games ship through it
    // live but historical playoff realtime is not its own dataset.
}

fn game_type_param(ty: SeasonType) -> u8 {
    match ty { SeasonType::Regular => 2, SeasonType::Playoff => 3 }
}
```

URLs change to `gameTypeId%3D{}` interpolation. Every existing call
site passes `SeasonType::Regular` to keep behavior; new playoff path
passes `SeasonType::Playoff`.

### D2 — Bundled file layout

**Problem**: `data/seasons/<id>/` currently contains:
```
bios.json
stats.json
goalie-stats.json
transactions.json
playoffs.json   ← bracket/series only
```

**Recommendation**: parallel naming convention:
```
data/seasons/<id>/
  bios.json                         # gameTypeId=2 (existing)
  stats.json                        # gameTypeId=2 (existing)
  goalie-stats.json                 # gameTypeId=2 (existing)
  playoff-stats.json                # NEW — skater playoff stats (gameTypeId=3)
  playoff-goalie-stats.json         # NEW — goalie playoff stats (gameTypeId=3)
  playoffs.json                     # bracket/series (existing, unchanged)
  transactions.json                 # unchanged
```

Why prefix `playoff-` instead of subdir `playoff/`:
- Consistent with existing flat layout
- `playoffs.json` (bracket) already lives at the top level; prefixing
  keeps it adjacent to its sibling stats files
- Smaller diff to `bundled.rs` macros (one new pair of static byte
  arrays per season instead of a directory walker)

**Identity sourcing**: playoff loads upsert identities from
`playoff-stats.json` rows (which include `skater_full_name` and
`team_abbrev`). Most players' identities will already exist from a
prior regular-season load on the same `StatsRepository`; upsert is
idempotent. Cold-start playoff-only loads (rare — operator running
`fetch stats --type playoff` without a prior regular-season fetch) get
identities populated from playoff bios alone.

**Open question**: do we need `playoff-bios.json`, or can
`playoff-stats.json` carry name+team+position for cold-start identity
construction? The `SkaterStats` schema today doesn't carry full bio
fields (no birth date, draft year, headshot canonicalization). My
read: skip `playoff-bios.json`. Cold-start playoff-only loads get
"degraded" identities (full_name + position from stats; bio fields
empty). Operators running `fetch all` get full identities from the
regular-season bios path; the playoff-only cold-start is rare enough
to accept degraded bios. Forge / tape: confirm.

### D3 — Snapshot tier paths

**Problem**: snapshot store uses flat filenames (`stats.json`,
`goalie-stats.json`, `bios.json`, `realtime.json`). No room for type.

**Recommendation**: prefix-based naming, parallel to bundled layout:
```
~/.icelines/snapshots/<id>/
  stats/
    bios.json                       # regular (existing)
    stats.json                      # regular (existing)
    chunked.json                    # regular (existing chunked path)
    playoff-bios.json               # NEW (or skip per D2)
    playoff-stats.json              # NEW
  realtime/
    realtime.json                   # regular only (no playoff variant)
  goalie-stats/
    goalie-stats.json               # regular (existing)
    playoff-goalie-stats.json       # NEW
  moneypuck/                        # regular only (per Open question)
  contracts/                        # type-agnostic (existing)
```

**Resolution chain** (mirror of regular-season today):
```
load_playoff_stats_with_fallback(season, store):
  1. Active snapshot — `stats/playoff-stats.json`
  2. Bundled — `bundled::get_playoff_stats(season)`
  3. Installed bundle — `~/.icelines/seasons/<id>/playoff-stats.json`
```

Same shape as `load_stats_with_fallback`; just different filenames.

### D4 — `load_into_repo` dispatch

**Problem**: line 128 short-circuits with `MissingBundle` when
`season_type == SeasonType::Playoff`.

**Recommendation**: replace the early-return with type-keyed source
selection:

```rust
pub fn load_into_repo(
    season: Season,
    season_type: SeasonType,
    store: &SnapshotStore,
) -> Result<LoadOutcome, LoadError> {
    let season_str = season.as_str();
    // … schema-version gate unchanged …

    let (bios, stats, goalie_stats) = match season_type {
        SeasonType::Regular => (
            bundled::load_bios_with_fallback(&season_str, store)?,
            bundled::load_stats_with_fallback(&season_str, store).unwrap_or_default(),
            bundled::load_goalies_with_fallback(&season_str, store).unwrap_or_default(),
        ),
        SeasonType::Playoff => (
            // D2: playoff identity comes from playoff-stats.json, not bios.
            // Synthesize SkaterBio from the stats row's name+team+position.
            bundled::load_playoff_bios_synthetic(&season_str, store)?,
            bundled::load_playoff_stats_with_fallback(&season_str, store).unwrap_or_default(),
            bundled::load_playoff_goalies_with_fallback(&season_str, store).unwrap_or_default(),
        ),
    };

    if bios.is_empty() {
        return Err(LoadError::MissingBundle {
            season: season_str.clone(),
            season_type,
        });
    }

    // … rest of the function (realtime/moneypuck/contracts/upserts)
    // is type-agnostic; realtime is skipped on playoff path.
    let realtime: Vec<SkaterRealtime> = match season_type {
        SeasonType::Regular => /* existing read */,
        SeasonType::Playoff => Vec::new(),
    };

    // … moneypuck: skip on playoff per D6 unless reviewer disagrees …
}
```

`load_playoff_bios_synthetic` is the cold-start "degraded bios" helper:
reads `playoff-stats.json` rows and produces `SkaterBio` shells with
just `player_id`, `skater_full_name`, `team_abbrev`, `position_code`
populated (other fields default). When a regular-season load has
already populated identities for the same `(season, player_id)`, the
upsert is a no-op — fully populated bios already in the repo are not
overwritten.

The empty-bios early-return becomes the playoff-equivalent of
`SeasonNotBundled`. Operators get a clean error directing them to
`icelines fetch stats --type playoff --season <id>`.

### D5 — Fetch CLI surface

**Problem**: `FetchSubcommand::Stats { season, refresh }` etc don't
take a season-type.

**Recommendation**: add `--type {regular|playoff}` flag, defaulting
to regular for backwards compat:

```
icelines fetch stats --season 20242025 --type regular   # existing behavior
icelines fetch stats --season 20242025 --type playoff   # new — writes
                                                        #   stats/playoff-stats.json
icelines fetch goalies --season 20242025 --type playoff
icelines fetch all --season 20242025 --type playoff     # bios+stats+goalies (skip realtime)
icelines fetch all --season 20242025                    # regular (default)
```

**Open question**: should `fetch all --type playoff` imply also fetching
regular-season data first if missing, on the theory that playoff stats
without regular-season data is operator error? My read: no.
Independent operations. The loader handles the mixed-presence case
cleanly (regular available, playoff missing → playoff load returns
`MissingBundle`; regular missing, playoff available → regular load
returns `SeasonNotBundled` separately).

### D6 — MoneyPuck on playoff path

**Problem**: MoneyPuck data ships per-season but combines regular-season
and playoff stats inline (per-row gameTypeId column or separate
endpoints depending on extract). Today the loader reads moneypuck.json
without knowing which game type the rows are.

**Recommendation**: skip MoneyPuck on the playoff-stats path for v1.
`AdvancedStats` (xG, CF%, FF%) on playoff `SeasonStats` will be `None`
for now. Add a `MissingSource::MoneyPuck` reason "advanced stats not
populated for playoff season_type — Hart.6 v1 limitation".

If a future feature needs playoff advanced stats, the resolution is:
either extend `MoneyPuckStats` with a `gameTypeId` column and split,
or use a separate playoff-only MoneyPuck endpoint. Tape: confirm the
data structure isn't already split (I'm working from memory here,
should verify against `moneypuck.rs`).

### D7 — Sub-phase ordering

Hart.6 needs to land in this order (each is a separate commit):

1. **Hart.6.1** — NHL API client. Add `season_type` parameter to
   `fetch_all_bios`, `fetch_all_stats`, `fetch_all_goalies`. Update
   all call sites to pass `SeasonType::Regular`. Add unit tests
   asserting the URL contains `gameTypeId%3D3` for playoff.

2. **Hart.6.2** — Snapshot tier paths. Extend
   `SnapshotStore::read_tier` / `write_tier` to accept type-prefixed
   filenames. Add `bundled::load_playoff_*_with_fallback` helpers
   (mirror of regular-season chain). Stub bundled data with empty
   arrays for now — actual bundled data lands in 6.3.

3. **Hart.6.3** — Bundled data. Author `playoff-stats.json` +
   `playoff-goalie-stats.json` for the five bundled seasons:
   `20212022`, `20222023`, `20232024`, `20242025`, `20252026` (the
   2025-26 file is empty / cup-final-not-played — the bundle ships
   it as `[]` and the load surfaces `MissingBundle` cleanly).
   Authoring procedure: run `icelines fetch all --type playoff
   --season <id> --write-bundle`, validate the JSON structure
   matches `data/seasons/<id>/stats.json` shape, commit.
   Each season is its own commit so review can validate per-file.

4. **Hart.6.4** — Loader dispatch. Replace the early-return at
   line 128 with the type-keyed source-selection in D4. Update the
   pinning unit test `l0_hart3_playoff_returns_missing_bundle_for_now`
   to assert on a season WITH playoff bundle (success) and one
   WITHOUT (clean `MissingBundle`).

5. **Hart.6.5** — Fetch CLI. Add `--type` flag per D5. Wire to the
   D1 API parameter. Add L2 system test:
   `icelines fetch stats --season 20242025 --type playoff` writes
   to `~/.icelines/snapshots/<active>/stats/playoff-stats.json`.

6. **Hart.6.6** — End-to-end TUI. Verify `y` season-picker overlay
   handles type toggle correctly: switching to playoff calls
   `App::reload_for_season(season, SeasonType::Playoff)` which
   invokes the now-functional load path. Add an L2 snapshot test
   (extending Hart.5c.6's `tests/tui_snapshot.rs`) that switches
   to playoff stats and asserts the rendered frame includes the
   "PO" type badge.

7. **Hart.6.7** — Documentation + release notes. Update
   `docs/guides/04-data.md` with playoff fetch instructions. Update
   `design/specs/season-data.md` to document type-keyed snapshot
   paths. Bump `CURRENT_BUNDLE_SCHEMA_VERSION` if the addition of
   `playoff-stats.json` to bundled tarballs is a breaking change
   for users on a prior version (it isn't — older binaries just
   won't see the file; tape confirm).

---

## Test impact

| File | Change | Sub-phase |
|---|---|---|
| `nhl_api.rs` test mod | new — playoff URL parameterization assertions | 6.1 |
| `tests/mock_nhl_api_loader.rs` (Hart.5c.7) | extend — playoff fetch round-trip | 6.1 + 6.4 |
| `bundled.rs` test mod | new — `get_playoff_stats / load_playoff_stats_with_fallback` | 6.2 |
| `stats_loader.rs` test mod | rewrite the playoff pinning test | 6.4 |
| `tests/stats_loader.rs` (L1) | extend — playoff load on a synthetic fixture | 6.4 |
| `tests/system_tests.rs` (L2) | add — `fetch stats --type playoff` round-trip | 6.5 |
| `tests/tui_snapshot.rs` (L2, from 5c.6) | add — playoff toggle frame | 6.6 |

---

## Risks

1. **Bundled file size growth**. `stats.json` for a single season is
   ~2 MB; playoff stats are ~10× smaller (only ~250 players play
   playoff games). Adding two playoff JSONs per season ≈ +2 MB across
   five seasons. Mitigation: monitor binary size in CI; if it crosses
   a meaningful threshold, switch to chunked layout.

2. **NHL API gameTypeId=3 returns empty mid-season**. During the
   regular season (October–April), `gameTypeId=3` returns last
   season's playoffs. During / after playoffs (April–June+), it
   returns the current playoff. The fetch logic is the same — just
   the response varies. Live operator running `fetch stats --type
   playoff --season 20252026` in February will get prior-year data
   incorrectly mapped to 20252026. Mitigation: API responses include
   `seasonId` in each row; loader rejects rows whose `seasonId` ≠
   the requested season. Forge confirm.

3. **Identity drift across types**. A player is on team EDM during
   the regular season and gets dealt to FLA at the deadline; their
   regular-season `SeasonStats.team_abbr` differs from their playoff
   `SeasonStats.team_abbr`. The repo handles this correctly because
   stats are keyed by `(player_id, season, season_type)` — both rows
   coexist. Cross-team-metrics views need to read from the
   appropriate-type stats; verify Hart.5b2c's `compute_cross_team_*`
   functions take `season_type` as a parameter (they should — confirm).

4. **Pre-1991 historical playoff data**. NHL API coverage of
   pre-modern-era playoffs is incomplete or absent. Hart.6 ships
   with the 5 bundled seasons (all post-1991). Older seasons can be
   added via `installed bundle` (~/.icelines/seasons/) authored
   externally; this phase doesn't author them.

5. **Cold-start "degraded bios"**. Cold-start playoff-only load
   (operator runs `fetch stats --type playoff` first, no regular
   data fetched) produces identities with empty bio fields. Risk:
   downstream consumers display blanks for birth date / draft year /
   headshot. Mitigation: documented behavior; operators following
   the documented happy path (`fetch all --type regular` first,
   then `fetch all --type playoff`) get full identities.

6. **MoneyPuck mismatch on type-toggled view**. Skipping MoneyPuck
   on playoff path means a player's `view.advanced` returns `Some`
   on regular and `None` on playoff. UI must handle the discrepancy
   gracefully; today's code already handles `advanced=None` because
   cold-start without MoneyPuck snapshot already produces it.
   Verify in 6.6 the TUI doesn't panic on missing-advanced for
   playoff stats.

---

## What I'm asking the reviewers

**forge** (Rust soundness):
- D1 — adding `season_type: SeasonType` to fetch fn signatures is a
  breaking API change to `NhlApiClient`. Today there's only one consumer
  (the fetch CLI). Confirm there's no need to keep a `_v2` parallel API.
- D4 — `load_playoff_bios_synthetic` returns `Vec<SkaterBio>` with
  most fields empty. Should the synthetic path produce a different
  type, or is reusing `SkaterBio` with `Default` fields the right call?
- D5 — `--type` flag default behavior: keep `regular` as default
  (silent no-flag = current behavior), or require explicit on every
  invocation post-Hart.6? My read: keep default to avoid breaking
  scripts.

**tape** (data integrity):
- D2 — skip `playoff-bios.json`, synthesize identity from playoff
  stats rows. Risk: a player who played ONLY in the playoffs (call-up)
  gets a degraded identity even after a normal `fetch all` because
  their regular-season bios row never existed. Frequency: low (a few
  per season). Acceptable?
- D6 — MoneyPuck split. Verify `MoneyPuckStats` schema; my
  recommendation skips it for playoff. Is there a `gameTypeId` column
  I'm missing, or a separate playoff endpoint?
- Risk #2 — seasonId enforcement on API responses. Worth a defensive
  filter in the loader, or trust the API?
- Risk #3 — verify `compute_cross_team_metrics_views` takes
  `season_type` as a parameter post-Hart.5c.

**wire** (API contracts):
- D3 — snapshot tier filename change from `stats.json` to
  `playoff-stats.json` for the playoff variant. Existing snapshots
  stay regular-season — no migration needed. Confirm we don't need
  a snapshot schema version bump.
- D7 — `CURRENT_BUNDLE_SCHEMA_VERSION` does NOT bump just because
  a new file is added (an additive file is forward-compatible —
  older binaries skip files they don't read). Confirm.
- 1993-94 playoffs.json: this is a different file (bracket data),
  unaffected by Hart.6. Confirm naming doesn't collide with the new
  `playoff-*.json` files.

**bench** (test discipline):
- 6.4 pinning test rewrite: today's
  `l0_hart3_playoff_returns_missing_bundle_for_now` becomes
  `l0_playoff_returns_missing_bundle_when_no_data` (a season WITHOUT
  bundled playoff data). Add `l0_playoff_load_succeeds_for_bundled_season`.
  Both required.
- 6.6 TUI snapshot — extending the Hart.5c.6 deliverable. Verify the
  snapshot harness can take a season-type as a parameter.
- L1 mock NHL API test: should it cover both `gameTypeId=2` and =3
  in a single test (round-trip both types into the same repo) or
  separate tests? My read: separate (smaller failure surface).

---

## What's NOT in this spec

- Per-game playoff stats (only totals, same shape as regular-season).
- Pre-1991 historical playoff data (not bundled in this phase).
- MoneyPuck playoff advanced stats (deferred per D6).
- Realtime playoff stats (no separate dataset; live games update
  through the existing realtime endpoint).
- A "compare regular vs playoff" feature view — that's a feature
  built on top of Hart.6's data, specced separately when scheduled.
- TUI redesign for type toggle — `y` season picker already handles
  it post-5c.6; Hart.6 is just data fill.

## Next step after this spec is reviewed

If approved as v0.2:
1. Implement Hart.6.1 (API client parameterization).
2. forge / wire review on the implementation diff.
3. Implement Hart.6.2 → 6.7 in order.
4. Phase Hart ships: `(season, season_type)` first-class through
   model, consumers, AND data. Functional playoff toggle in TUI/CLI.
