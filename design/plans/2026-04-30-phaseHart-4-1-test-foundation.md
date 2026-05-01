# Phase Hart.4.1 — Test Foundation Before Hart.5

**Status**: Spec / plan v0.2 — incorporates bench / forge / tape review punch list
**Date**: 2026-04-30
**Trophy**: Hart (sub-phase of the normalization effort)
**Predecessor**: design/plans/2026-04-30-phaseHart-normalization.md
**Replaces**: nothing — additive

---

## Goal

Land the missing test coverage and canonical scenario builders **before** Hart.5
migrates 30+ consumers off the legacy `Player`/`Goalie`/`PlayerRepository` types.
The new normalized model has solid unit-level coverage but several integration-
level gaps that would let bugs slip through during consumer migration.

If a consumer migration breaks something at the data layer, we want the test
suite to catch it at the L0 / L1 boundary — not at the consumer L2 / TUI layer
where the failure surface is harder to localize.

## v0.2 review revisions

Three-role review (bench / forge / tape) of the v0.1 spec flagged 4 BLOCKERs
and ~16 FIXITs. Notable changes from v0.1:

- **Gap D (LRU proptest)** — switched from "compute expected resident set
  via the same algorithm as production" (which would have made it a
  change-detector, not an invariant test) to **invariant assertions**:
  `resident_windows() ≤ cap`, stats↔window_lru bidirectional bijection,
  touch-then-evict ordering. (FORGE #3, TAPE #7)
- **Gap E (roster sum proptest)** — strategy now feeds **raw possibly-duplicate**
  `Vec<TeamAbbr>` directly to `index_rosters` (the production code dedups
  on insert) so the same-team-twice regression Hart.2.1 fixed actually
  surfaces in the proptest. v0.1 would have pre-deduped and silently
  missed it. (BENCH #2)
- **Gap F (mid-playoff goalie trade L1)** — moved to L0 inside
  `stats_loader.rs` instead of L1 in `tests/`. Avoids pub-ing
  `build_goalie_season_stats` (a public-API expansion FORGE flagged as
  the wrong idiom). (FORGE #2)
- **Gap G (fixture builders)** — every builder must enforce the TAPE
  invariants by construction (sum-equals across stints, monotonic
  stint ordering, post-upsert roster sum-equals). v0.1 said "L0 self-test
  ensures valid SeasonStats" — too vague. Now explicit. (TAPE #11)
- **Gap C (snapshot-populated)** — must use `tempfile::tempdir()`-rooted
  `SnapshotStore`, never `Config::load()` defaults. v0.1 didn't specify;
  a misuse would pollute `~/.icelines/snapshots/`. (FORGE landmine)
- **Gap H added** — error-path L1 coverage. The Framework Discipline
  criterion #2 says "every LoadError variant has L1 coverage" but v0.1
  didn't enumerate which variants are uncovered. (BENCH #7)
- **`drafted_prospect` builder dropped** from Gap G — premature
  scaffolding (no Hart.5 consumer named that needs it). (BENCH #6)
- **Three new builders** added to Gap G: `emergency_backup_goalie`,
  `partial_tier_player`, `goalie_zero_toi`. (TAPE #12)
- **Trade builders** are now parameterized on `from`/`to` team from day
  one, avoiding the post-hoc `traded_skater_atl_to_nsh` proliferation
  trap. (BENCH #5)
- **Multi-load test-helper** must live in a single shared file with a
  doc-comment naming it as draft scaffolding, plus an escape-clause:
  if ≥3 Hart.5 sub-phases need the orchestration, promote to a public
  `StatsRepository::merge_load_outcome` API. (FORGE #1)

## Why now

Hart.5 is a 30+ file refactor. Every CLI command, TUI screen, fantasy / scheme
module, and mkdocs builder gets touched. Without a robust test foundation:

- A subtle mapper bug (e.g. accessor returns 0 when it should return None)
  surfaces as a TUI rendering glitch rather than a broken assertion in a
  named test.
- A regression in `merge_with` triggered by Hart.5's new multi-season-load
  call pattern goes uncaught (no L1 covers cross-load merge).
- A roster-index integrity break sneaks past because the existing tests use
  `cold_store()` and the bug only surfaces with a populated snapshot.
- An eviction bug that orphans stats rows passes the existing LRU case
  tests but fails the bidirectional-consistency invariant.

The cost to land the foundation is ~2h (revised up from 1.5h after review).
The cost to debug a Hart.5 bug that escapes into the consumer layer is much
higher.

## Non-goals

- TUI rendering tests for Hart.5 surfaces — those land alongside the
  consumer migrations themselves.
- Performance benchmarks (Pace's deferred TeamAbbr-interning ticket).
- Doctests on PlayerView accessors / SeasonStatsBuilder (cosmetic).
- A full property-test sweep of every type — only the invariants where
  randomized input adds real signal.
- A multi-season public API (`merge_load_outcome`) — only the test-helper
  form. Public API waits for Hart.5 to demonstrate it's needed at ≥3 sites.

---

## Coverage gaps to close

### A — Multi-season cross-load identity merge (L1)

**Gap**: `load_into_repo` builds a fresh `StatsRepository` each call. The
loader path therefore does NOT exercise `PlayerIdentity::merge_with` within
a single load (the bios-dedup means each player_id upserts exactly once).
Hart.5 callers will load multiple seasons into the same repo for time-travel
queries — that's where merge fires through the loader. Currently uncovered.

**Test-helper home**: `icelines-fetch/tests/common/multi_load.rs` (new file).
Doc-comment names it explicitly as "Hart.4.1 draft scaffolding — promote
to `StatsRepository::merge_load_outcome` if ≥3 Hart.5 sub-phases reference
this orchestration." The helper takes a `LoadOutcome` and merges its
identities/stats/contracts into an existing repo.

**Tests** (3 in this gap):

1. `l1_load_two_seasons_merges_identities_through_loader`
   - Load 20232024 Regular → merge into repo
   - Load 20242025 Regular → merge into same repo
   - For McDavid (player_id 8478402): assert identity present once,
     both season's stats present, bio fields stable across loads (TAPE
     immutability — birth_date doesn't drift, draft_year doesn't drift).

2. `l1_load_two_seasons_cross_team_change_preserves_identity`
   - Pick a player who changed teams between 23-24 and 24-25 (dynamic
     selection: filter for `team_stints.last().team` differing across
     loads; hardcode the discovered player_id as a `const` after the
     selection so the test is deterministic on rerun).
   - Assert: `repo.identity(pid)` is unique; the per-(season, type)
     stats rows have different last-stint teams; bio fields match.
   - **Hard assert** (TAPE #4): if no candidate found in bundled data,
     panic with "TAPE invariant violated: bundled data missing expected
     cross-season-trade coverage."

3. `l1_load_two_seasons_reissue_across_seasons_rejects_and_preserves_state`
   - Use a synthetic `LoadOutcome` (or fabricate one via the test-helper)
     where the second load has a player_id with a different `rookie_season`
     than the first.
   - Snapshot the repo state before the second load: `repo.identities_len()`,
     `repo.stats_len()`, `repo.contracts_len()`, plus
     `repo.identity(pid).cloned()` for the conflicting pid.
   - Attempt the second load; assert `RepoError::IdentityMerge(LikelyIdReissue { .. })`.
   - **Re-snapshot**: every counter equals pre-state; the conflicting
     identity is byte-identical. (TAPE #1 — rejected merges leave repo
     unchanged at the integration level, not just the unit level.)

### B — Real-data PlayerView accessor smoke (L1)

**Gap**: every PlayerView accessor (`team_display`, `was_traded_in_window`,
`hits`, `xg`, `pace_score`, `contract_*`, etc.) is L0-tested with fixtures.
No L1 test confirms the accessors return sensible values against real
bundled data.

**Test**: `l1_player_view_accessors_against_real_bundled_data`

- Load 20242025 Regular via `load_into_repo`.
- Pick McDavid by **name match + property filter**, not pure hardcode:
  ```rust
  const MCDAVID_ID: PlayerId = PlayerId(8478402); // stable forever
  let view = repo.view(MCDAVID_ID, ...).expect("McDavid in 24-25 bundled data");
  ```
- Assert (BENCH #1, FORGE #8 — relational + presence, never absolute counters):
  - `full_name().to_lowercase() == "connor mcdavid"`
  - `full_name().contains("McDavid")` (regression fence on case-flip drift)
  - `position().is_forward()`
  - `team_display()` is a 3-letter team string (length == 3, all uppercase ASCII)
  - `gp() > 0`, `goals() > 0`
  - **Relational invariant**: `points() == goals() + assists()`
  - `pace_score().is_some()` — verified with property check first: `gp() >= MIN_GP`. If the chosen player doesn't meet MIN_GP this season, fall back to "first eligible C from the league iter" (TAPE #3 — dynamic-with-skipping, never silently pass).
  - `was_traded_in_window() == false` (Connor is single-team)
  - **Cold-start Option assertions**: `hits().is_none()`, `xg().is_none()`, `contract.is_none()`. Locks the cold-start contract.
- **Diacritic round-trip** (TAPE #13): pick a player with non-ASCII characters in their name (e.g. Slafkovský, Pastrňák) — verify `full_name()` round-trips byte-identical. Cheap to add; locks NFC/NFD-corruption-on-load to a regression fence.

**Traded-player check** (TAPE #4 — hard assertion):

- Iterate `repo.skaters(season, Regular)`; find any view with `was_traded_in_window() == true`.
- Hard-assert at least one exists. If zero, panic: "TAPE invariant violated: bundled 24-25 has no multi-stint skater. Either bundle drift or test bug."
- For the found view: `team_stints.len() >= 2`; `team_display()` matches `team_stints.last().team.as_str()`.

### C — Snapshot-populated load path (L1)

**Gap**: every L1 test uses `cold_store()` — bundled fallback only. The
snapshot-first path where realtime / moneypuck / contracts JSON files
ARE present is completely untested.

**Critical isolation requirement** (FORGE landmine): every test MUST
construct a fresh `SnapshotStore` via `SnapshotStore::new(tempfile::tempdir().unwrap().path())`.
NEVER use `Config::load()` defaults, which would pollute
`~/.icelines/snapshots/`. The plan's helper `cold_store()` already does
this; the populated-snapshot helper follows the same pattern.

**Tests** (4 in this gap):

1. `l1_load_into_repo_with_populated_snapshot_realtime`
   - Use `SnapshotStore::create + write_file + seal` (the canonical public
     write path verified in `snapshot.rs:233-307`).
   - Stage one realtime row referencing a `player_id` known to exist in
     the bundled bios (e.g. McDavid 8478402).
   - **Precondition assertion** (TAPE #5): `assert!(repo.identity(pid).is_some(), "synthetic realtime row references pid not in bundled bios — fixture drift")` BEFORE asserting on accessors.
   - Assert: `outcome.missing` does NOT contain `MissingSource::Realtime`;
     `view.hits()` returns the synthesized value (Some-arm of Option-at-leaf, BENCH #10);
     a different bundled player not in the synthetic realtime returns `view.hits().is_none()` (None-arm).

2. `l1_load_into_repo_with_populated_snapshot_moneypuck` — same shape for moneypuck.

3. `l1_load_into_repo_with_populated_snapshot_contracts` — same shape for contracts.

4. `l1_load_into_repo_orphan_realtime_row_skipped_gracefully` (TAPE #6)
   - Stage realtime.json with two rows: one for a real bundled pid, one
     for `player_id: 99_999_999` (no bios).
   - Assert: load succeeds, no panic; `repo.identity(PlayerId(99_999_999)).is_none()` (orphan didn't accidentally create a phantom identity).

### D — LRU invariant proptest (L0)

**Gap**: BENCH-deferred from Hart.2.1 review. Hart.2 has cap=2 / cap=4
case tests. No proptest spans the invariant.

**Strategy** (revised per BENCH #3 + FORGE #3):

```rust
proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        // Override via `PROPTEST_CASES` env in CI if runtime regresses.
        ..ProptestConfig::default()
    })]

    #[test]
    fn lru_invariant_proptest(
        cap in 1usize..=10,
        ops in prop::collection::vec(
            // Alphabet of 6 windows (3 seasons × 2 types). Length 0..50.
            // Birthday-paradox at length 50 over 6 distinct values
            // gives ~30% repeat rate, hitting the touch-then-promote
            // path consistently.
            (0u32..3, prop_oneof![Just(SeasonType::Regular), Just(SeasonType::Playoff)])
                .prop_map(|(idx, t)| (Season(20212022 + idx * 10001), t)),
            0..50,
        ),
    ) { /* see assertions below */ }
}
```

**Assertions** (TAPE #7 — bidirectional bijection, NOT change-detector
per FORGE #3):

After running every op (one upsert per `(s, t)`):
1. **Cap bound**: `repo.resident_windows() <= cap`.
2. **Stats → window_lru**: for every `(pid, s, t)` in `repo.stats`,
   `repo.has_window(s, t) == true`.
3. **window_lru → stats**: for every `(s, t)` in `repo.window_lru`,
   there's at least one stats row keyed to that window.
4. **Touch promotion**: the most-recently-upserted window is always
   resident (`has_window(last_op_window) == true`), regardless of the
   sequence before it.

A separate handcrafted test `l0_hart4_1_lru_cap_one_pure_churn`
covers the `cap = 1` edge: every upsert evicts the prior. (BENCH #3)

### E — Roster sum invariant proptest (L0)

**Gap**: same Hart.2.1 deferral.

**Strategy** (revised per BENCH #2 + FORGE #4):

```rust
proptest! {
    #[test]
    fn roster_sum_invariant_proptest(
        // Each player has 1..=4 stints. Stints CAN repeat the same team
        // (Hart.2.1's same-team-twice fix). Strategy doesn't pre-dedup —
        // it feeds raw possibly-duplicate stints into index_rosters.
        players in prop::collection::vec(
            (any::<u32>(), prop::collection::vec(
                "[A-Z]{3}".prop_map(|s| TeamAbbr(s)),
                1..=4, // non-empty per builder invariant
            )),
            1..=20,
        ),
    ) { /* … */ }
}
```

**Assertions** (TAPE #8 — also covers `rosters_last_stint`):

- For every player, build a `SeasonStats` with the raw input stints
  (not deduped). Upsert into a fresh repo with `season=Season(20232024)`,
  `season_type=Regular`.
- **rosters_all_stints sum** (Hart.2.1 regression fence):
  `sum over teams of repo.team_roster_all_stints(team, ...).len() ==
  count of distinct (player, team) pairs across the input`.
  Computed reference: `players.iter().flat_map(|(pid, teams)| teams.iter().map(|t| (pid, t.clone()))).collect::<HashSet<_>>().len()`.
- **rosters_last_stint sum** (TAPE #8):
  `sum over teams of repo.team_roster(team, ...).len() == players.len()`
  (each player has exactly one last-stint team).

**Plus a case-test for the replace-path** (TAPE #9 — proptest only fires
`index_rosters`, not `unindex_rosters_for`):

`l0_hart4_1_replace_stats_unindexes_old_roster_entries`
- Upsert player(1) with stints `[BOS, NYR]` → assert BOS and NYR contain pid 1.
- Replace with stints `[TOR]` → assert BOS and NYR no longer contain pid 1; TOR does.

### F — Mid-playoff goalie trade L0 synthetic (BENCH-mandated)

**Decision (FORGE #2)**: this lands as L0 inside `icelines-fetch/src/stats_loader.rs`
under `#[cfg(test)] mod tests`. Avoids pub-ing `build_goalie_season_stats`.

**Test**: `l0_hart4_1_mid_playoff_goalie_trade_synthetic_l1_repeat`

- Construct `GoalieStats` row: `team_abbrevs = "BOS,FLA"`, `games_played = 10`,
  `wins = 5`, `losses = 5`, `saves = 252`, `goals_against = 28`, etc.
- Run through `build_goalie_season_stats` (in-module, no pub change).
- Assert:
  - `stats.team_stints.len() == 2`
  - `stats.team_stints[0].team.as_str() == "BOS"` (Hart.3.1 monotonic-date
    fix — insertion order survives the builder sort)
  - `stats.team_stints[1].team.as_str() == "FLA"`
  - **Sum-equals**: sum of stint gp/goals/assists/points across the 2
    stints == aggregate totals
  - `stats.goalie.is_some()` with games_started == 10, wins == 5
  - `stats.is_goalie() == true`

**Plus L1 follow-up** in `tests/stats_loader.rs`:

`l1_hart4_1_goalie_trade_identity_survives_minimal_bio` (TAPE #10)

- Construct an identity-only repo via the same loader path or a
  hand-staged snapshot with a synthetic GoalieStats row.
- Upsert through the full loader pipeline.
- Assert: `repo.identity(pid).unwrap().full_name == "<synthesized>"` and
  `repo.identity(pid).unwrap().bio.shoots_catches.is_some()`. Locks
  goalie identities don't get clobbered when the only data source has
  minimal bio fields.

### G — Canonical scenario builders for Hart.5

**Builders** in `icelines-core/src/fixtures.rs` (revised list):

```rust
/// A skater traded mid-window with two TeamStints. Defaults to STL → NYR
/// (matches the Tarasenko 2022-23 worked example) but the team pair is
/// parameterizable from day one to avoid post-hoc proliferation
/// (BENCH #5).
pub fn traded_skater(
    player_id: u32,
    season: u32,
    from: TeamAbbr,
    to: TeamAbbr,
) -> StatsFixture { /* … */ }

/// Convenience default — STL → NYR.
pub fn traded_skater_default(player_id: u32, season: u32) -> StatsFixture {
    traded_skater(player_id, season, TeamAbbr("STL".into()), TeamAbbr("NYR".into()))
}

/// A goalie traded mid-playoff with two TeamStints both carrying
/// GoalieStintStats. Parameterized by team pair.
pub fn goalie_mid_playoff_trade(
    player_id: u32,
    season: u32,
    from: TeamAbbr,
    to: TeamAbbr,
) -> StatsFixture { /* … */ }

pub fn goalie_mid_playoff_trade_default(player_id: u32, season: u32) -> StatsFixture {
    goalie_mid_playoff_trade(player_id, season, TeamAbbr("BOS".into()), TeamAbbr("FLA".into()))
}

/// A goalie that played for one team only (default: Linus Ullmark @ OTT).
pub fn solo_goalie(player_id: u32, season: u32, team: TeamAbbr) -> StatsFixture { /* … */ }

/// Hart's emergency-backup-goalie scenario: a `Position::Goalie` row for
/// a player whose career is otherwise skater. Locks the per-row
/// `is_goalie()` design (TAPE #12).
pub fn emergency_backup_goalie(player_id: u32, season: u32) -> StatsFixture { /* … */ }

/// A player with bios + realtime tier present, but no MoneyPuck and no
/// contracts. The realistic mid-season case — exercises the partial-tier
/// view-accessor path (TAPE #12, BENCH #10).
pub fn partial_tier_player(player_id: u32, season: u32) -> StatsFixture { /* … */ }

/// Goalie with games_played > 0 but time_on_ice_sec == 0. Data-quality
/// edge case; PACE divides by TOI in some leaderboard formulas
/// (TAPE #12).
pub fn goalie_zero_toi(player_id: u32, season: u32) -> StatsFixture { /* … */ }
```

**Each builder MUST have an L0 self-test that asserts** (TAPE #11 — invariants
by construction, not aspirational):

1. The fixture's `SeasonStats` builds successfully (passes `SeasonStatsBuilder::build()`'s `assert!(!team_stints.is_empty())`).
2. **Sum-equals** across stints: `sum(stints[i].gp) == totals.gp`,
   same for goals/assists/points.
3. For goalie variants: `sum(stints[i].goalie.games_started) == goalie.games_started`,
   same for wins/losses.
4. **Stint ordering** is monotonic (chronological by `started`, with
   the Hart.3.1 synthetic-date prefix sorting before any real date —
   see "shared invariants" below).
5. **Post-upsert roster sum-equals** holds: build a fresh `StatsRepository`,
   upsert identity + the fixture, assert
   `sum over teams of team_roster_all_stints(team).len() == count of
   distinct teams in the fixture`.

**Shared synthetic-date prefix** (FORGE additional landmine):

The `"AAAA-{:02}"` synthetic date prefix that production
`build_goalie_season_stats` uses to preserve insertion order is
extracted to:

```rust
// icelines-core/src/season_stats.rs
pub(crate) const SYNTHETIC_DATE_PREFIX: &str = "AAAA";
```

Both production loader code and fixture builders reference this const.
Doc-comment notes the invariant: "any string starting with `AAAA-`
sorts before any ISO-8601 date `YYYY-MM-DD` for `YYYY >= 1900`."

### H — Error-path L1 audit (BENCH #7)

**Gap**: Framework Discipline criterion #2 ("every LoadError variant
reachable from at least one L1 test") was claimed in v0.1 but never
audited explicitly.

**Variants and current L1 coverage** (verify during implementation):

| Variant | L1 covered? | Where / TODO |
|---|---|---|
| `SeasonNotBundled` | ✅ | `l1_load_into_repo_unknown_season_returns_season_not_bundled` |
| `MissingBundle` (Playoff) | ✅ | `l1_load_into_repo_playoff_returns_missing_bundle` |
| `BundleSchemaUnknown` | ✅ | `l1_load_into_repo_rejects_future_bundle_schema_version` |
| `RepoVersionUnknown` | ✅ | `l1_load_into_repo_rejects_future_repository_version` |
| `Repo(IdentityMerge(LikelyIdReissue))` | ⚠️ added by Gap A #3 | new |
| `Repo(StatsWithoutIdentity)` | ❌ | **add `l1_hart4_1_load_stats_without_identity_errors`** |
| `Bundle { source: BundleError::Io }` | ❌ | dead code path today (FORGE #1 from Hart.3 review) — keep or drop? |
| `Bundle { source: BundleError::Parse }` | ❌ | same |

**Action**: add the `StatsWithoutIdentity` L1 (synthesize via a malformed
fixture upsert sequence). For the `Bundle` variants — the Hart.3 forge
review flagged these as dead code. Hart.4.1 must EITHER:
- (a) Wire bundle reads through `BundleError` so the variants are
  reachable (then add L1 coverage); or
- (b) Drop `LoadError::Bundle` and the `BundleError` enum until a real
  call site exists.

**Decision**: **(b) drop**. Defer reintroduction to whenever a real
load path needs distinct I/O-vs-Parse-vs-NotBundled error fidelity.
Dead code shouldn't have tests; it should be deleted. Reverts a small
piece of Hart.3 spec.

### Framework discipline criteria (revised)

1. **Every consumer-visible behavior has a fence at L0 or L1**. Hart.5
   should not introduce a runtime bug that no existing test would catch.
2. **Every load-time error variant is exercised**. Audit table in Gap H.
3. **Every `merge_with` policy clause has a fixture**. Sanity floors,
   reissue, immutable fields, none-incoming all have named tests
   (Hart.1 + Hart.1.1 + Gap A).
4. **Every roster-index mutation path has a sum-equals assertion**.
   Both `index_rosters` AND `unindex_rosters_for` (proptest E + case
   test in same gap).
5. **Every Option-returning PlayerView accessor has both Some-case AND
   None-case exercised on real data** (BENCH #10). Hits/xg None-case
   from cold-start; Some-case from Gap C populated-snapshot tests.
6. **No test depends on HashMap iteration order** unless the iteration
   order is part of the documented contract.
7. **No test relies on a wall-clock or random source without a seed**.
   `proptest-regressions/` files commit to git so flakes are
   reproducible (TAPE #14).
8. **Test isolation: every test that touches a snapshot store uses a
   fresh `tempfile::tempdir()`-rooted `SnapshotStore`**. Never
   `Config::load()`. (FORGE landmine.)
9. **No silent-skip soft assertions in TAPE territory**. If bundled
   data is expected to contain a specific shape (e.g. ≥1 traded
   skater), the test panics with "TAPE invariant violated: ..." rather
   than passing vacuously. (TAPE #4)

---

## Test counts after Hart.4.1

| Tier | Now (Hart.4) | After Hart.4.1 | Delta |
|---|---|---|---|
| L0 | 694 | ~707 | +13 (proptest scaffolding, fixture self-tests, replace-path case, cap=1 case, mid-playoff-goalie L0) |
| L1 | 135 | ~145 | +10 (multi-load × 3, real-data accessors × 2, populated-snapshot × 4, goalie-identity-survival × 1) |
| L2 | 140 | 140 | 0 |
| Total prefixed | 969 | ~992 | +23 |
| Workspace total | 1018 | ~1041 | +23 |

The proptest count is one named test per invariant but each runs ~256
cases by default — so in practice the LRU and roster-sum proptests
exercise tens of thousands of synthetic inputs.

---

## Ordering

This is one self-contained commit. No dependency on Hart.5 consumer
work; can land before any Hart.5 sub-phase begins.

After this commit, Hart.5 starts. Hart.5 sub-phases each touch one
consumer; they can rely on the new fixture builders and the proptest
invariants to catch regressions quickly.

---

## Risks

1. **Multi-season cross-load needs a stable API eventually**. v0.2 caps
   the test-helper approach with an explicit escape clause: if ≥3
   Hart.5 sub-phases reference the orchestration, promote to public
   `StatsRepository::merge_load_outcome`. Without that tripwire, ad-hoc
   helpers proliferate.

2. **Proptest flakes / slow tests**. Default 256 cases × moderate
   strategy. Mitigation: explicit `proptest_config` block; CI can
   override via `PROPTEST_CASES` env. Commit `proptest-regressions/`
   files to git.

3. **Synthetic snapshot-write for path C** is brittle if the snapshot
   tier directory layout changes. Mitigation: tests write through the
   public `SnapshotStore::create + write_file + seal` flow. Tempdir
   isolation requirement is in framework criterion #8.

4. **Hart.5 might want different fixture shapes**. Bounded by
   parameterization-from-day-one (BENCH #5). All trade builders take
   `from`/`to` `TeamAbbr` arguments. Adding a new scenario is additive
   (new fn), not modificative (changing a default).

5. **Dropping `LoadError::Bundle`** (Gap H decision) is a small
   reversal of Hart.3 spec. Defensible because the variant is
   currently dead code. Document in commit message that reintroduction
   is permitted whenever a real call site emerges.

---

## Resolved questions (post-review)

| Q | A |
|---|---|
| Multi-season load: new public API or test-helper merge? | Test-helper merge in `icelines-fetch/tests/common/multi_load.rs`. Escape clause: ≥3 Hart.5 sites → promote to public API. |
| Synthetic goalie trade: L0 in `stats_loader.rs` or L1 in `tests/`? | **L0 in `stats_loader.rs`** (FORGE #2). Avoids pub-ing `build_goalie_season_stats`. Companion L1 covers identity survival. |
| Pub `build_goalie_season_stats`? | **No.** Stays `pub(crate)` (private module-scope is fine since the test is in-module). |
| Proptest case count? | Default 256 via explicit `#![proptest_config]`. CI can override via `PROPTEST_CASES` env. Commit `proptest-regressions/` to git. |
| Where do scenario builders live? | `icelines-core/src/fixtures.rs`. Self-tests inline. Each builder enforces sum-equals + monotonic-stints + post-upsert roster sum-equals invariants by construction. |
| `drafted_prospect` builder? | **Dropped** (BENCH #6). Add when Hart.5 first consumer demonstrably needs it. |
| Trade fixture team pairs? | **Parameterized** from day one (BENCH #5). Default convenience constructors (`*_default`) for the common case. |
| `LoadError::Bundle` and `BundleError`? | **Drop** (Gap H decision). Reintroduce when a real call site emerges. |
| Synthetic date prefix shared between fixtures and production? | Extract to `pub(crate) const SYNTHETIC_DATE_PREFIX: &str = "AAAA"` in `season_stats.rs`. Both sites reference it. |
| Tempdir isolation required for all snapshot-touching tests? | **Yes.** Framework criterion #8. (FORGE landmine.) |
| Soft "if data has one" assertions? | **No.** Hard panic with "TAPE invariant violated" message. (TAPE #4.) |
| McDavid hardcoded name string assertion? | **Case-insensitive + relational.** `to_lowercase() == "connor mcdavid"` + `points == goals + assists`. (FORGE #8, BENCH #1.) |
| Diacritic round-trip test? | **Yes** in Gap B — pick a player with non-ASCII characters in their name; verify `full_name()` byte-identical. (TAPE #13.) |

---

## What "great test coverage" means here (the framework discipline)

See "Framework discipline criteria (revised)" above. The Hart.4.1
additions plug the gaps where the current suite fails one of those
nine criteria.
