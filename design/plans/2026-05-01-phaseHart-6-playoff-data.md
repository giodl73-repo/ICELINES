# Phase Hart.6 — Playoff Per-Player Data (v0.2, post-review)

**Status**: v0.2 — incorporates 4-role review (forge / tape / wire / bench).
Ready to implement.
**Date**: 2026-05-01
**Trophy**: Hart (final sub-phase — completes the original Hart goal)
**Predecessor**: design/plans/2026-04-30-phaseHart-normalization.md (master),
design/plans/2026-05-01-phaseHart-5c-final-cleanup.md (v0.2)
**Replaces**: nothing — additive

---

## v0.1 → v0.2 changelog

Four parallel reviews returned **8 BLOCKERs + 14 FIXITs + 10 NITs**.
Punch list applied below; spec body updated inline.

### BLOCKERs resolved

- **Tape B1** — `SkaterStats` (`schema.rs:57-76`) does NOT carry
  `skater_full_name`, `team_abbrev`, or `position_code`. The "synthesize
  identity from playoff stats rows" approach in v0.1 D2 is impossible
  without schema drift. Resolution: **fetch and bundle `playoff-bios.json`**
  for each season. Drop `load_playoff_bios_synthetic` entirely. Cost:
  one extra ~50KB JSON per season. The `gameTypeId=3` bios endpoint is
  free once D1 lands. Forge F2 (newtype `SyntheticSkaterBio`) is moot.

- **Tape B2** — Risk #2 (cross-season API leakage) cannot be defended
  without a schema addition. `SkaterStats` and `SkaterBio` (`schema.rs:30,
  :57`) currently do **not** deserialize `seasonId` (only `GoalieStats`
  does, `schema.rs:154`). Resolution: **Hart.6.1 adds `season_id: u32`
  to `SkaterStats` and `SkaterBio`** as additive serde fields (the API
  returns `seasonId` in every row; we just throw it away today).
  **Hart.6.4 loader rejects rows where `season_id != requested_season`**
  with new `LoadError::SeasonIdMismatch { expected, found, count }`.
  **Hart.6.3 bundled-data authoring fails-loud** on any mismatched row.

- **Tape B3** — `cross_team::compute_all_views` and friends
  (`cross_team.rs:217, 222, 309`) take `&[PlayerView<'_>]` with no
  type-homogeneity guard. The repo key is correct
  (`(PlayerId, Season, SeasonType)` per `stats_repository.rs:66`), but
  the cross-team API trusts callers. Footgun: a non-TUI consumer that
  builds a mixed-type Vec corrupts CF%/team-strength aggregates.
  Resolution: **add `debug_assert!` for type-homogeneity** in every
  cross-team entry point (new D8 below). Document precondition in
  rustdoc.

- **Forge B1** — `ChunkedManifest` (`snapshot.rs:115-122`) has no
  season-type axis. `fetch stats --type playoff --chunked` would
  silently corrupt regular-season hash tables. Resolution: extend
  `ChunkedManifest` with `playoff_bios: Option<HashMap<u32, String>>`
  + `playoff_stats: Option<HashMap<u32, String>>` as `#[serde(default)]`
  fields. Old chunked snapshots deserialize cleanly (None for both).
  No version bump on `ChunkedManifest::version` because all additions
  are `Option`. The chunked read/write path branches on `season_type`
  the same way the legacy file-per-tier path does. Detail in D3.

- **Forge B2** — `find_snapshot_for_tier` (`snapshot.rs:311`) walks the
  parent chain on tier-dir existence only, with no season filter.
  Today's flat layout is implicitly regular-season-only so the
  ambiguity is invisible; Hart.6 makes it visible. Resolution: **co-locate**
  playoff stats with the same-season regular snapshot inside the
  existing tier dir (`stats/playoff-stats.json` adjacent to
  `stats/stats.json`). Same-season co-location means the parent-chain
  walker continues to do the right thing — the alternative (parent
  walker takes `season_type`) would require API changes to every
  tier-read call site. Detail in D3.

- **Bench B1** — Test impact table is missing the L1 rewrite for 6.4.
  `tests/stats_loader.rs:169` has `l1_load_into_repo_playoff_returns_missing_bundle`
  pinning the early-return that 6.4 deletes. Resolution: 6.4 explicitly
  REPLACES that test with paired
  `l1_load_into_repo_playoff_succeeds_for_bundled_season` +
  `l1_load_into_repo_playoff_returns_missing_bundle_for_unbundled_season`.
  Test impact table updated.

- **Bench B2** — Risk #2 mitigation has no required test. Resolution:
  **required L1 test**:
  `l1_playoff_load_rejects_rows_with_wrong_seasonid` feeds the loader
  a synthetic `playoff-stats.json` with cross-season rows; asserts
  `LoadError::SeasonIdMismatch`.

- **Bench B3** — 6.5 L2 system test as v0.1 wrote it doesn't round-trip.
  Resolution: 6.5 system test must (a) `fetch stats --type playoff`,
  (b) verify file at exact spec-named path, (c) `query leaders --season
  ... --type playoff` returns non-empty rows. The query step proves
  the read path uses the same path as the write.

### FIXITs applied inline

- **Forge F1** — `tests/mock_nhl_api.rs` URL matchers hardcode
  `gameTypeId%3D2` (~5 call sites, lines 519/545/575/604/1441). Hart.6.1
  must update them to dispatch on `gameTypeId` so playoff variants land
  in the same fixture. Added to 6.1 deliverables.

- **Forge F3** — `--type` default is `regular`; **add `--type both`**
  (or sibling `--include-playoff` flag) on `fetch all` so season-end
  snapshots are one invocation, not two. Doc in 6.7 clarifies.

- **Forge F4** — Empty-bios on playoff path must surface as
  `LoadError::MissingBundle { season, season_type: Playoff }`, not
  `SeasonNotBundled { season }` (no season_type field). Pinned in D4.

- **Forge F5** — D6 wording: "set `MissingSource::MoneyPuck { reason: ... }`"
  not "Add a `MissingSource::MoneyPuck` reason". The variant exists.

- **Wire F1** — D3 spec text needs to say *why* the snapshot store is
  exempt from migration: tier directory names (`stats/`, `realtime/`,
  `goalie-stats/`) are unchanged; the change is purely additive
  filenames within existing tier dirs. `SnapshotMeta.integrity` is
  per-file (`snapshot.rs:107`); old snapshots simply have no integrity
  entry for new files; `read_tier` returns `NotFound` cleanly. Added
  to D3.

- **Wire F2** — D7 hedged `CURRENT_BUNDLE_SCHEMA_VERSION` bump with
  "tape confirm". Wire owns this constant (`snapshot.rs:946`).
  Resolution: **do NOT bump.** New files don't add fields to existing
  bundled types. Pinned in D7.

- **Wire F3** — Hart.6 does not advance `MAX_KNOWN_BUNDLE_SCHEMA`
  (`stats_loader.rs:38`); explicitly state the migrator-on-read TODO
  at line 140 is unaffected. Added to D7.

- **Bench F1** — TUI snapshot harness from 5c.6 must accept
  `season_type` as a parameter for 6.6 to extend it. **Pre-condition
  on Hart.5c.6**: harness signature is
  `render_screen(repo, season, season_type, screen) -> Buffer`. Added
  to "Pre-conditions" section. Surfacing as a 5c.6 spec amendment.

- **Bench F2** — L1 mock NHL API gets a third test:
  `l1_mock_nhl_api_loader_mixed_types_into_one_repo` — load
  gameTypeId=2 first, then =3 for same season; verify identity reuse
  and `(season, type)` keying.

- **Bench F3** — Cold-start playoff-only test (formerly "synthetic
  bios" path; now "real but type-only" cold-start):
  `l1_playoff_only_cold_start_uses_playoff_bios` — fresh repo, load
  only `SeasonType::Playoff`, assert `view.full_name()` populated and
  `view.identity.bio.draft_year.is_some()` (because we now fetch real
  playoff bios, draft year IS populated).

- **Bench F4** — 6.4 extends the 5c.4-rewritten
  `tests/integration_phase2.rs` with **playoff Beniers known-value
  asserts** (or whichever player has clean playoff numbers in the
  bundle). Anchors known-value coverage on the playoff axis the same
  way 5c.4 anchors regular.

- **Tape F1** — D3 snapshot path consistency: bundled layout
  (`data/seasons/<id>/playoff-goalie-stats.json` flat) and snapshot
  layout (`stats/goalie-stats/playoff-goalie-stats.json` nested) are
  intentionally different. 6.7 docs spell this out.

- **Tape F2** — 6.3 authoring procedure too loose. Replace "validate
  the JSON structure matches `stats.json` shape" with explicit:
  `serde_json::from_slice::<Vec<SkaterStats>>(&playoff_bytes)?` plus
  the seasonId filter from B2. Added to D7 step 6.3.

- **Tape F3** — D5 `fetch all --type playoff` explicit behavior:
  runs `bios + stats + goalies` for `gameTypeId=3`, skips `realtime
  + moneypuck + contracts`. `fetch realtime --type playoff` errors
  cleanly with `Error: realtime is regular-season-only`.

### NITs applied / deferred

- **Forge N1** (asymmetric `load_playoff_*` naming): defer. Single-fn-with-type-param
  is cleaner long-term, but the asymmetry mirrors the existing
  bundled/snapshot file naming (`stats.json` vs `playoff-stats.json`).
  Revisit in a hypothetical Hart.7 cleanup if the asymmetry actually
  bites.
- **Forge N2** (6.1 unit test scope): URL builder is private. Resolution:
  6.1 asserts via mock NHL API in the same commit (no separate URL-builder
  test).
- **Forge N3** (Risk #2 defensive filter is forge concern): subsumed
  by Tape B2 — schema addition + filter, not just filter.
- **Forge N4** (retitle "Decisions to make" → "Decisions made"): done
  in v0.2.
- **Wire N1** (default-flag): no action.
- **Wire N2** (`get_playoff_stats_installed`): added one bullet under D2.
- **Wire N3** (shape parity check): added to D7 step 6.3 (Tape F2).
- **Bench N1** (drop `_when_no_data` suffix): renamed
  `l0_playoff_returns_missing_bundle_for_unbundled_season`.
- **Tape N1** (bundle size estimate): corrected to ~500KB across all
  five seasons, not 2MB. Risk #1 updated.
- **Tape N2** (`realtime --type playoff` errors not silent): added to
  D5.

---

## Goal

Populate `StatsRepository` with per-player **playoff** stats for the
five bundled seasons (and live playoff fetch for the current season),
so a TUI/CLI season-type toggle from "Regular" to "Playoff" returns
real numbers instead of `LoadError::MissingBundle`.

The original motivation for Phase Hart was making `(season, season_type)`
a first-class axis. Hart.5c completes the **consumer migration**.
Hart.6 completes the **data half** — without it, the entire Phase
Hart investment is structurally complete but functionally unused for
playoffs.

## Pre-conditions

- **Hart.5c lands first.** After 5c the model layer accepts playoff stats
  natively and every consumer reads through `(season, season_type)`-aware
  accessors. Hart.6 only needs to populate; no consumer changes.
- `SeasonType::Playoff` exists (`icelines-core/src/season_stats.rs:34`).
- `App::active_type` is owned by App (Hart.5c.6).
- L1 fixture builders already produce playoff `SeasonStats` (Hart.4.1).
- **Hart.5c.6 TUI snapshot harness signature** (Bench F1): the harness
  introduced as a required deliverable in 5c.6 must take
  `(repo, season, season_type, screen)` so 6.6 can extend it without
  rewriting. If 5c.6 lands without `season_type` in the signature, file
  a Hart.5c.6.1 fix before starting Hart.6.6. **Action**: amend
  `design/plans/2026-05-01-phaseHart-5c-final-cleanup.md` v0.3 to pin
  this signature.

## Why this is a separate phase

Hart.5c is a 8-step consumer migration that must land independently
green. Hart.6 is a data-source addition that touches `nhl_api.rs`,
`bundled.rs`, `snapshot.rs`, `stats_loader.rs`, and the `fetch` CLI.
Largely orthogonal codepaths; bundling would inflate review surface
for no shared dependency.

---

## Current state (what already exists)

**API client** (`icelines-fetch/src/nhl_api.rs`):
- `fetch_all_bios(season)` — hardcodes `gameTypeId=2`, line 117
- `fetch_all_stats(season)` — hardcodes `gameTypeId=2`, line 126
- `fetch_all_realtime(season)` — hardcodes `gameTypeId=2`, line 158
- `fetch_all_goalies(season)` — hardcodes `gameTypeId=2`, line 169
- `fetch_playoff_bracket(year)` — exists, returns bracket/series data
  (NOT per-player stats)

**Schema** (`icelines-fetch/src/schema.rs`):
- `SkaterBio` (line 30) — does NOT deserialize `seasonId` ← Hart.6.1 adds
- `SkaterStats` (line 57) — does NOT deserialize `seasonId` ← Hart.6.1 adds
- `GoalieStats` (line 154) — DOES carry `season_id: u32`

**Bundled data** (`icelines-fetch/src/bundled.rs`):
- `BIOS_<season>` / `STATS_<season>` / `GOALIES_<season>` — five seasons
  embedded, all regular-season only
- `BUNDLED_PLAYOFFS` — series/bracket data only (1993-94 fixture)
- No per-player playoff stats embedded

**Loader** (`icelines-fetch/src/stats_loader.rs`):
- `load_into_repo(season, season_type, store)` — line 120
- Lines 127-133: explicit early-return `LoadError::MissingBundle` for
  `season_type == SeasonType::Playoff`
- Test `l0_hart3_playoff_returns_missing_bundle_for_now` (line 636)

**Snapshot tier layout** (`icelines-fetch/src/snapshot.rs`):
- File-per-tier: `stats/bios.json`, `stats/stats.json`,
  `realtime/realtime.json`, etc. Implicitly regular-season-only.
- `ChunkedManifest` (line 115) — flat `bios: HashMap<u32, String>` /
  `stats: HashMap<u32, String>`. No type axis. ← Hart.6.2 adds Option fields.
- `find_snapshot_for_tier` (line 311) — walks parent chain on tier-dir
  existence only. ← Hart.6 co-locates instead of changing this.

**Cross-team API** (`icelines-core/src/cross_team.rs`):
- `compute_all_views`, `compute_all_views_with_mode`,
  `compute_team_strength_views` (lines 217, 222, 309) — take
  `&[PlayerView<'_>]` with no type-homogeneity guard ← Hart.6 adds
  `debug_assert!`.

**MoneyPuck** (`icelines-fetch/src/moneypuck.rs`):
- `MoneyPuckStats` (lines 36-43) — no `gameTypeId` / `season_type` field
- `csv_url()` (lines 108-113) — hardcodes `/regular/skaters.csv`
- → MoneyPuck cannot do playoff in v1; D6 confirmed.

**Fetch CLI** (`icelines-cli/src/commands/fetch.rs`):
- `FetchSubcommand::{Rosters, Stats, Realtime, Goalies, Contracts,
  Transactions, MoneyPuck, Positions, All}` — all assume gameTypeId=2.

## Scope

In:
- NHL API client — playoff URL variants for bios, stats, goalies
- **Schema** — add `season_id: u32` to `SkaterBio` and `SkaterStats`
- Bundled data — author + embed `playoff-bios.json`,
  `playoff-stats.json`, `playoff-goalie-stats.json` for the five
  bundled seasons
- Snapshot store — type-aware filenames in existing tier dirs
  (co-located); chunked manifest extended with `Option` playoff fields
- `load_into_repo` — remove early bail, route by season_type, reject
  cross-season rows
- Cross-team API — `debug_assert!` for type-homogeneity
- Fetch CLI — `--type {regular|playoff|both}` flag on stats / goalies
  / rosters / all
- Tests — L0 fixture, L1 mock NHL API for playoff path, L2 for the
  fetch + load round-trip
- TUI hookup — confirm end-to-end via extended L2 snapshot from 5c.6

Out:
- Realtime stats for playoffs (no separate dataset; live games update
  through the existing realtime endpoint)
- Per-game playoff stats (totals only, same shape as regular-season)
- MoneyPuck playoff stats (verified impossible without endpoint
  discovery — D6)
- Contracts (single source per player; not type-keyed)
- Pre-1991 historical playoff data (NHL API coverage thins out;
  see Risk #4)

---

## Decisions made

### D1 — URL parameterization

**Problem**: API client URL-builders hardcode `gameTypeId=2`.

**Decision**: take `season_type: SeasonType` as a parameter on each
fetch fn. New signatures:

```rust
impl NhlApiClient {
    pub async fn fetch_all_bios(&self, season: &str, ty: SeasonType)
        -> Result<Vec<SkaterBio>, FetchError>;
    pub async fn fetch_all_stats(&self, season: &str, ty: SeasonType)
        -> Result<Vec<SkaterStats>, FetchError>;
    pub async fn fetch_all_goalies(&self, season: &str, ty: SeasonType)
        -> Result<Vec<GoalieStats>, FetchError>;
    // realtime stays gameTypeId=2-only.
}

fn game_type_param(ty: SeasonType) -> u8 {
    match ty { SeasonType::Regular => 2, SeasonType::Playoff => 3 }
}
```

URLs change to `gameTypeId%3D{}` interpolation. Every existing call site
passes `SeasonType::Regular` to keep behavior; new playoff path passes
`SeasonType::Playoff`. **Forge F1**: `tests/mock_nhl_api.rs` URL matchers
must update to dispatch on `gameTypeId` in the same commit — without
this, 6.1 lands red.

### D2 — Bundled file layout (FETCH BIOS, NO SYNTHESIS)

**Problem**: `data/seasons/<id>/` currently contains regular-season files
only.

**Decision**: parallel naming convention with **real bios**:
```
data/seasons/<id>/
  bios.json                         # gameTypeId=2 (existing)
  stats.json                        # gameTypeId=2 (existing)
  goalie-stats.json                 # gameTypeId=2 (existing)
  playoff-bios.json                 # NEW — gameTypeId=3 bios
  playoff-stats.json                # NEW — skater playoff stats
  playoff-goalie-stats.json         # NEW — goalie playoff stats
  playoffs.json                     # bracket/series (existing, unchanged)
  transactions.json                 # unchanged
```

**Tape B1**: `SkaterStats` (`schema.rs:57-76`) does not carry
name/team/position. Synthesizing identity from stats rows in v0.1 was
impossible. Resolution: **fetch real playoff bios** at gameTypeId=3
(free once D1 lands). Cost: ~50KB extra per season; ~250 players ship
playoff bios. Identity upsert is idempotent on `player_id`, so a
regular-season-then-playoff load is a no-op for shared players.

**Wire N2**: also expose `bundled::get_playoff_stats_installed` and
`bundled::get_playoff_bios_installed` mirroring the existing `_installed`
helpers, so installed bundles work the same way.

### D3 — Snapshot tier paths (CO-LOCATED)

**Problem**: snapshot tier dirs are implicitly regular-season-only.
Need to add playoff data without breaking the parent-chain walker
(`find_snapshot_for_tier`, `snapshot.rs:311`).

**Decision**: **co-locate** playoff files inside existing tier dirs:
```
~/.icelines/snapshots/<id>/
  stats/
    bios.json                       # regular (existing)
    stats.json                      # regular (existing)
    chunked.json                    # regular + playoff (extended manifest)
    playoff-bios.json               # NEW
    playoff-stats.json              # NEW
  realtime/
    realtime.json                   # regular only
  goalie-stats/
    goalie-stats.json               # regular (existing)
    playoff-goalie-stats.json       # NEW
  contracts/                        # type-agnostic (existing)
```

**Why co-located, not separate `playoff/` subdirs (Forge B2)**: a
parent-chain walker that finds `playoff-stats.json` in an unrelated
parent snapshot for the same tier would silently return wrong-season
rows. Co-locating with the same-season regular snapshot means the
walker continues to do the right thing — playoff data lives or dies
with its regular-season sibling, in the same `<snapshot-id>` directory.

**ChunkedManifest extension (Forge B1)**: `ChunkedManifest`
(`snapshot.rs:115-122`) gains two `Option` fields:
```rust
#[derive(Serialize, Deserialize)]
pub struct ChunkedManifest {
    pub version: u32,                                       // unchanged: 1
    pub bios:    HashMap<u32, String>,                      // existing (regular)
    pub stats:   HashMap<u32, String>,                      // existing (regular)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub playoff_bios:  Option<HashMap<u32, String>>,        // NEW
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub playoff_stats: Option<HashMap<u32, String>>,        // NEW
}
```
Old chunked snapshots deserialize cleanly (None for both new fields).
Both serializer and deserializer handle Option transparently. No
`ChunkedManifest::version` bump because additions are `Option`.
`read_chunked_active(season_type)` branches on type to read the right
HashMap.

**Wire F1 reasoning** (why no migration is needed): tier directory
names (`stats/`, `realtime/`, `goalie-stats/`) are unchanged. The
change is purely additive filenames within existing tier dirs.
`SnapshotMeta.integrity` is per-file (`snapshot.rs:107`); old snapshots
simply have no integrity entry for new files; `read_tier` returns
`NotFound` cleanly (`snapshot.rs:385`). The active-snapshot lookup
chain is unchanged.

### D4 — `load_into_repo` dispatch

**Problem**: line 128 short-circuits with `MissingBundle` when
`season_type == SeasonType::Playoff`.

**Decision**: replace early-return with type-keyed source selection;
add seasonId rejection (Tape B2):

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
            bundled::load_playoff_bios_with_fallback(&season_str, store)?,
            bundled::load_playoff_stats_with_fallback(&season_str, store).unwrap_or_default(),
            bundled::load_playoff_goalies_with_fallback(&season_str, store).unwrap_or_default(),
        ),
    };

    // Forge F4: empty-bios on playoff → MissingBundle (carries season_type),
    // not SeasonNotBundled (no type field).
    if bios.is_empty() {
        return Err(match season_type {
            SeasonType::Regular => LoadError::SeasonNotBundled { season: season_str.clone() },
            SeasonType::Playoff => LoadError::MissingBundle { season: season_str.clone(), season_type },
        });
    }

    // Tape B2: reject cross-season rows.
    let expected = season.0;
    let count_mismatched_skater_stats = stats.iter().filter(|s| s.season_id != expected).count();
    let count_mismatched_skater_bios  = bios.iter().filter(|b| b.season_id != expected).count();
    let count_mismatched_goalies      = goalie_stats.iter().filter(|g| g.season_id != expected).count();
    let total_mismatched = count_mismatched_skater_stats + count_mismatched_skater_bios + count_mismatched_goalies;
    if total_mismatched > 0 {
        return Err(LoadError::SeasonIdMismatch {
            expected,
            found: stats.iter().chain(bios.iter().map(|b| /* coerce */)).next()
                       .map(|x| x.season_id).unwrap_or(0),
            count: total_mismatched,
        });
    }

    let realtime: Vec<SkaterRealtime> = match season_type {
        SeasonType::Regular => /* existing read */,
        SeasonType::Playoff => Vec::new(),
    };

    let moneypuck: Vec<MoneyPuckStats> = match season_type {
        SeasonType::Regular => /* existing read */,
        SeasonType::Playoff => {
            // Forge F5 + Tape Q2 verified: set reason on existing variant.
            missing.push(MissingSource::MoneyPuck {
                season: season_str.clone(),
                reason: "advanced stats not populated for playoff season_type — Hart.6 v1 limitation".into(),
            });
            Vec::new()
        }
    };

    // contracts: unchanged (type-agnostic).
    // … rest of the function (upserts) is type-agnostic …
}
```

New `LoadError::SeasonIdMismatch` variant added. Existing
`#[non_exhaustive]` annotation makes this safe.

### D5 — Fetch CLI surface

**Problem**: `FetchSubcommand::Stats { season, refresh }` etc don't
take a season-type.

**Decision**: add `--type {regular|playoff|both}` flag, defaulting to
`regular` (Forge F3):

```
icelines fetch stats --season 20242025                    # regular (default)
icelines fetch stats --season 20242025 --type regular     # explicit
icelines fetch stats --season 20242025 --type playoff     # writes
                                                          #   stats/playoff-stats.json
icelines fetch stats --season 20242025 --type both        # both, single invocation
icelines fetch goalies --season 20242025 --type playoff
icelines fetch all --season 20242025 --type playoff       # bios+stats+goalies (skip realtime+moneypuck)
icelines fetch all --season 20242025 --type both          # full season-end snapshot
icelines fetch all --season 20242025                      # regular (default)
icelines fetch realtime --type playoff                    # ERROR: realtime is regular-season-only
icelines fetch moneypuck --type playoff                   # ERROR: moneypuck is regular-season-only
```

`fetch all --type playoff` (Tape F3) explicitly runs `bios + stats +
goalies` for `gameTypeId=3`; skips `realtime + moneypuck + contracts`
(contracts are type-agnostic; they run once on `--type both` or
`--type regular`).

`fetch all --type both` runs the regular-season set first (full pipe
including realtime, moneypuck, contracts), then the playoff trio.
Order matters: regular-season identities populate before playoff
upsert sees them, so the no-op deduplication path is exercised.

### D6 — MoneyPuck on playoff path (verified skip)

**Problem**: MoneyPuck's per-row schema and source URL.

**Decision**: skip MoneyPuck on playoff path for v1.
**Verified from source** (Tape Q2):
- `MoneyPuckStats` (`moneypuck.rs:36-43`) has no `gameTypeId`,
  `season_type`, or `season` field. There is no way to disambiguate row
  provenance from the persisted shape.
- `csv_url()` (`moneypuck.rs:108-113`) **hardcodes
  `/regular/skaters.csv`** in the URL path.
- "All situations" / "5on5" splits are situation-keyed, not
  game-type-keyed. There is no playoff variant of this endpoint that
  the existing parse path can consume.

`AdvancedStats` (xG, CF%, FF%) on playoff `SeasonStats` is `None` for
v1. Loader sets `MissingSource::MoneyPuck { reason: "advanced stats
not populated for playoff season_type — Hart.6 v1 limitation" }` on
playoff loads. (Forge F5: this is the existing variant; we're setting
the reason field, not adding a new variant.)

A future Hart.7 (or separate phase) could add MoneyPuck playoff
support. Out of scope here.

### D7 — Sub-phase ordering (8 sub-phases now, was 7)

Each is a separate commit. L2 system tests gate every commit.

1. **Hart.6.1** — Schema additions + NHL API client.
   - Schema: add `season_id: u32` to `SkaterBio` (`schema.rs:30`) and
     `SkaterStats` (`schema.rs:57`) as additive serde fields. The API
     returns `seasonId` in every row.
   - API client: add `season_type` parameter to `fetch_all_bios`,
     `fetch_all_stats`, `fetch_all_goalies`. Update all call sites
     to pass `SeasonType::Regular`. Switch URL templates to
     `gameTypeId%3D{}` interpolation.
   - **`tests/mock_nhl_api.rs` URL matchers** (Forge F1): update ~5
     call sites at lines 519/545/575/604/1441 to dispatch on
     `gameTypeId` so playoff variants land in the same fixture.
   - Forge N2 + N3: defensive `season_id` filter in the loader (lands
     in 6.4) is the partner of this schema addition.

2. **Hart.6.2** — Snapshot tier paths.
   - Extend `ChunkedManifest` with `Option` `playoff_bios` /
     `playoff_stats` fields.
   - `read_chunked_active` and `read_chunked_stats` take `season_type`
     parameter.
   - Add `bundled::load_playoff_bios_with_fallback`,
     `load_playoff_stats_with_fallback`,
     `load_playoff_goalies_with_fallback` mirror chains.
   - Add `bundled::get_playoff_stats_installed`,
     `bundled::get_playoff_bios_installed`.
   - Stub bundled data with empty arrays for now — actual bundled data
     in 6.3.

3. **Hart.6.3** — Bundled data authoring.
   - Author `playoff-bios.json`, `playoff-stats.json`,
     `playoff-goalie-stats.json` for the five bundled seasons:
     `20212022`, `20222023`, `20232024`, `20242025`, `20252026`.
     The 2025-26 file ships as `[]` (cup not yet contested) and the
     load surfaces `MissingBundle` cleanly.
   - **Authoring procedure** (Tape F2): for each season, run
     `icelines fetch all --type playoff --season <id> --write-bundle`,
     then `serde_json::from_slice::<Vec<SkaterStats>>(&playoff_bytes)?`
     to validate parse, then assert `all rows.season_id == season.0`
     before committing. Each season is its own commit so review can
     validate per-file.

4. **Hart.6.4** — Loader dispatch.
   - Replace early-return at `stats_loader.rs:128` with type-keyed
     source selection (D4 sketch).
   - Add `LoadError::SeasonIdMismatch { expected, found, count }`
     variant.
   - Add seasonId rejection on every loaded row.
   - **Replace** `tests/stats_loader.rs:169`
     `l1_load_into_repo_playoff_returns_missing_bundle` (Bench B1)
     with paired:
     - `l1_load_into_repo_playoff_succeeds_for_bundled_season`
     - `l1_load_into_repo_playoff_returns_missing_bundle_for_unbundled_season`
   - **Add** `l1_playoff_load_rejects_rows_with_wrong_seasonid`
     (Bench B2).
   - **Add** `l1_playoff_only_cold_start_uses_playoff_bios` (Bench
     F3).

5. **Hart.6.5** — Fetch CLI.
   - Add `--type {regular|playoff|both}` flag (D5).
   - Reject `realtime --type playoff` and `moneypuck --type playoff`
     with clean errors.
   - **L2 round-trip test** (Bench B3): `icelines fetch stats --season
     20242025 --type playoff` writes to
     `~/.icelines/snapshots/<active>/stats/playoff-stats.json`; THEN
     `icelines query leaders --season 20242025 --type playoff` returns
     non-empty rows. The query step proves the read path uses the
     same path as the write.

6. **Hart.6.6** — Cross-team type-homogeneity guard + TUI verification.
   - Add `debug_assert!` for type-homogeneity to
     `cross_team::compute_all_views`,
     `compute_all_views_with_mode`,
     `compute_team_strength_views` (lines 217, 222, 309). Document
     the precondition in rustdoc (Tape B3):
     ```rust
     debug_assert!(
         views.iter().all(|v| v.season_type() == views[0].season_type()),
         "cross_team views must be type-homogeneous"
     );
     ```
   - **Extend** `tests/tui_snapshot.rs` (the 5c.6 deliverable) with
     a playoff-toggle case. Pre-condition (Bench F1): the harness
     accepts `season_type` as a parameter — see "Pre-conditions" above.

7. **Hart.6.7** — Documentation + version assessment.
   - Update `docs/guides/04-data.md` with playoff fetch instructions.
   - Update `design/specs/season-data.md` to document type-keyed
     filenames (and the bundled-vs-snapshot directory layout
     asymmetry — Tape F1).
   - **Wire F2**: do **not** bump `CURRENT_BUNDLE_SCHEMA_VERSION`.
     Hart.6 adds new files but no fields to existing types; the
     version constant gates field-addition compatibility. Document
     this decision in the commit message.
   - **Wire F3**: `MAX_KNOWN_BUNDLE_SCHEMA` is unchanged; the
     migrator-on-read TODO at `stats_loader.rs:140` is unaffected.

8. **Hart.6.8** (NEW) — Hart.5c.6 amendment ("test-amendment"
   commit).
   - File a v0.3 amendment to Hart.5c spec to pin the TUI snapshot
     harness signature: `render_screen(repo, season, season_type,
     screen) -> Buffer`. Without this, Hart.6.6 cannot extend the
     harness without rewriting it.
   - This is a documentation-only commit (spec text) but it has a
     real-code dependency: 5c.6 must land with the parameterized
     signature. If 5c.6 lands without it, Hart.6.8 grows code to fix
     the harness.

---

## Test impact (corrected v0.2)

| File | Change | Sub-phase |
|---|---|---|
| `nhl_api.rs` test mod (L0) | new — playoff URL parameterization assertions; mock_nhl_api fixture verifies URLs land at gameTypeId=3 | 6.1 |
| `tests/mock_nhl_api.rs` | UPDATE URL matchers (Forge F1) at lines 519/545/575/604/1441 to dispatch on gameTypeId | 6.1 |
| `tests/mock_nhl_api_loader.rs` (Hart.5c.7) | extend — playoff fetch round-trip; **add** `l1_mock_nhl_api_loader_mixed_types_into_one_repo` (Bench F2) | 6.1 + 6.4 |
| `bundled.rs` test mod (L0) | new — `get_playoff_stats / load_playoff_stats_with_fallback`; ChunkedManifest deserializes old snapshots cleanly | 6.2 |
| `stats_loader.rs` test mod (L0) | rename `l0_hart3_playoff_returns_missing_bundle_for_now` → `l0_playoff_returns_missing_bundle_for_unbundled_season` | 6.4 |
| `tests/stats_loader.rs` (L1) | REPLACE `l1_load_into_repo_playoff_returns_missing_bundle` (line 169) with paired tests (Bench B1); ADD `l1_playoff_load_rejects_rows_with_wrong_seasonid` (Bench B2); ADD `l1_playoff_only_cold_start_uses_playoff_bios` (Bench F3) | 6.4 |
| `tests/integration_phase2.rs` (L1, rewritten in 5c.4) | EXTEND with playoff Beniers known-value asserts (Bench F4) | 6.4 |
| `tests/system_tests.rs` (L2) | ADD round-trip test (Bench B3): fetch+verify file+query+assert non-empty rows | 6.5 |
| `cross_team.rs` test mod (L0) | new — `debug_assert!` panics on mixed-type Vec<PlayerView> (Tape B3) | 6.6 |
| `tests/tui_snapshot.rs` (L2, from 5c.6) | EXTEND — playoff toggle frame (Bench F1 precondition) | 6.6 |

---

## Risks (updated v0.2)

1. **Bundled file size growth** — corrected (Tape N1): 5 seasons × 3
   new files × ~50KB ≈ ~750KB total bundle growth (not the 2MB v0.1
   estimated). Negligible for binary size budget.

2. **Cross-season API leakage** — RESOLVED via schema addition + filter:
   `season_id: u32` is now in every persisted row (Tape B2); the loader
   rejects mismatches with `LoadError::SeasonIdMismatch`. Authoring
   procedure validates before commit.

3. **Identity drift across types** — handled by the repo's
   `(player_id, season, season_type)` triple key
   (`stats_repository.rs:66`). Cross-team metrics need type-homogeneous
   views: now enforced by `debug_assert!` in 6.6 (Tape B3).

4. **Pre-1991 historical playoff data** — coverage gap. Hart.6 ships
   five bundled seasons (post-1991). Older seasons added via
   `installed bundle` if someone authors them externally; this phase
   doesn't.

5. ~~**Cold-start "degraded bios"**~~ — REMOVED. Hart.6 fetches real
   playoff bios; cold-start playoff-only loads get fully-populated
   identities (draft year, headshot, etc).

6. **MoneyPuck mismatch on type-toggled view** — `view.advanced` is
   `Some` on regular and `None` on playoff. Existing UI handles
   `advanced=None` (cold-start path already produces it). Verified in
   6.6 TUI snapshot test.

7. **`debug_assert!` is debug-only** — Tape B3's type-homogeneity guard
   compiles out in release. This is intentional (the assertion is a
   programmer-error fence; release builds shouldn't crash on it). If
   a future bug bypasses the guard at runtime, it's silent. Mitigation:
   the guard is also documented in rustdoc as a precondition; consumer
   contract is "don't pass mixed-type Vecs".

---

## What I'm asking the reviewers (round 2)

This v0.2 punch list is large. Re-review focus:

**forge** — confirm B1 + B2 fixes (chunked-manifest extension via
`Option` fields; co-located filenames). Verify the `LoadError::
SeasonIdMismatch` shape is sound (`#[non_exhaustive]` is already
there).

**tape** — confirm the schema additions to `SkaterBio` and `SkaterStats`
land cleanly; spot-check that `seasonId` is actually returned by the
NHL API for all relevant endpoints. Confirm cross-team `debug_assert!`
is the right mechanism (vs. a runtime check).

**wire** — confirm the `ChunkedManifest` Option-fields approach does
NOT need a `version` bump (additive Option fields are forward-compat
per established convention).

**bench** — confirm the 5-test additions (replace + 4 new L1s + 1 new
L2 + 1 new L0) cover the surface; sign off on the precondition
amendment to 5c.6.

---

## What's NOT in this spec

- Per-game playoff stats (only totals).
- Pre-1991 historical playoff data (not bundled here).
- MoneyPuck playoff advanced stats (verified impossible without
  endpoint discovery — D6).
- Realtime playoff stats (no separate dataset).
- A "compare regular vs playoff" feature view.
- TUI redesign for type toggle — `y` season picker handles it
  post-5c.6.

## Next step

If approved as v0.2:
1. Implement Hart.6.1 (schema + API client parameterization).
2. forge / wire review on the 6.1 implementation diff.
3. Implement 6.2 → 6.7 in order. (6.8 is the 5c spec amendment, not
   code.)
4. Phase Hart ships: `(season, season_type)` first-class through
   model, consumers, AND data. Functional playoff toggle in TUI/CLI.
