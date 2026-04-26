# IceLines Test Strategy

**Version**: 0.1  
**Date**: 2026-04-25  
**Role reference**: `.roles/bench.md` — BENCH is the authoritative voice on test quality

---

## Test Levels

### L0 — Unit Tests

**What**: Pure-function tests in-module (`#[cfg(test)]` inside `src/`).  
**Scope**: `icelines-core` exclusively — the computation layer has no I/O.  
**Speed**: Must complete in < 2 seconds total across all L0 tests.  
**Network**: Zero. No filesystem access beyond compile-time constants.  
**Coverage target**: ≥ 95% line coverage on `icelines-core`.

L0 tests verify:
- Every scoring formula with documented expected values and calculation comments
- Every classification boundary (at threshold − ε, at threshold, at threshold + ε)
- Every normalization function (Unicode, diacritics, empty string, all-whitespace)
- Every data model constraint (GP=0, GP < MIN_GP, GP = MIN_GP exactly)
- Every enum conversion (positionCode → Position, API team abbrev → TeamAbbr)
- Every error variant is constructible and has a meaningful Display impl
- All arithmetic uses f32/f64 consistently — no silent integer truncation

**Rule**: Every hardcoded expected value carries a calculation comment explaining the derivation.
A magic number in a test is a bug waiting to happen.

```rust
#[test]
fn pace_score_mcdavid_2023_24() {
    // 100 pts in 75 GP → pace = 100/75 × 82 = 109.333...
    let score = compute_pace_score(100, 75).unwrap();
    assert!((score.pace_82 - 109.333).abs() < 0.001);
}

#[test]
fn pace_score_none_below_min_gp() {
    // GP=9 is below MIN_GP=10 — must return None, not 0
    assert!(compute_pace_score(50, 9).is_none());
}

#[test]
fn pace_score_some_at_exactly_min_gp() {
    // GP=10 is exactly MIN_GP — must return Some
    assert!(compute_pace_score(10, 10).is_some());
}
```

---

### L1 — Integration Tests

**What**: Multi-component tests in `tests/` directory, with controlled external I/O.  
**Scope**: Cross-crate pipelines — CSV → scoring → depth chart, scheme → fantasy pts, etc.  
**Speed**: Must complete in < 30 seconds. No live network.  
**Mocking**:
- HTTP: `httpmock` crate — record real responses once, replay from fixtures
- Database: in-memory SQLite (`rusqlite::Connection::open_in_memory()`)
- Filesystem: `tempfile` crate for temporary directories

L1 tests verify:
- Full pipeline: load fixtures → fetch (mocked) → score → classify → depth chart
- Scheme scoring: known CSV fixture + known weights → exact fantasy point totals
- Position profile: fixture boxscores → correct primary position + multi-eligible list
- PlayerFilter: all filter dimensions (age, nationality, position, ppg, etc.) in combination
- Dashboard TOML loading → correct QuerySpec deserialization
- PlayerGroup persistence round-trip (create → add → show → delete in in-memory DB)
- Name resolution: Slafkovský normalizes correctly, Sebastian Aho disambiguation
- CSV parsing: BOM, empty numeric fields, missing columns, accented names

**Fixture catalog** (in `tests/fixtures/`):

| File | Contents |
|------|----------|
| `sample_skaters.csv` | 9 player archetypes (see BENCH role) |
| `api/bios_page_1.json` | NHL bios API response, 5 players |
| `api/stats_page_1.json` | NHL summary stats API response |
| `api/roster_SEA.json` | Seattle Kraken roster |
| `api/roster_COL.json` | Colorado Avalanche roster |
| `api/boxscore_2025020001.json` | Sample boxscore with position data |
| `api/player_8478402_landing.json` | McDavid player landing (career stats) |
| `schemes/yahoo_standard.toml` | Yahoo standard scheme |
| `dashboards/u23_centers.toml` | Sample dashboard definition |

**The 9 BENCH archetypes** (every test fixture must include all 9):

| Player | GP | G | A | Position | Role |
|--------|----|---|---|----------|------|
| Elite  | 82 | 50| 90| C        | McDavid-tier — always Elite |
| Solid  | 74 | 28| 40| LW       | Solid/Fit range |
| Buried | 68 | 15| 22| C        | Good but low line slot |
| Stretch| 55 | 4 | 8 | RW       | Playing above talent |
| Traded | 40 | 12| 18| D        | Mid-season trade, split rows |
| Rookie | 22 | 5 | 7 | C        | < MIN_GP threshold at season start |
| Injured| 10 | 3 | 5 | LW       | Exactly MIN_GP — included |
| Absent | 0  | 0 | 0 | RW       | GP=0 — always excluded |
| Multi  | 78 | 20| 35| C,LW     | Multi-position eligible |

---

### L2 — System Tests

**What**: Invoke the compiled `icelines` binary as a subprocess.  
**Scope**: Every CLI command at least one smoke test.  
**Speed**: Must complete in < 120 seconds using cached fixture data (no live network).  
**Binary**: Built in `--release` mode before L2 suite runs.

L2 tests verify:
- Binary exists and `--version` / `--help` exit 0
- `icelines team COL` exits 0 and stdout contains "Colorado Avalanche"
- `icelines rank --top 10` exits 0 and stdout has exactly 10 data rows
- `icelines fetch --dry-run` exits 0 (no API calls made)
- `icelines scheme from-csv tests/fixtures/sample_skaters.csv` creates a TOML file
- `icelines scheme list` shows the created scheme
- `icelines players --pos C --age-max 23` exits 0 and all rows are centers ≤ 23
- `icelines class 2022` exits 0 and output contains 2022 draft picks
- `icelines build --no-site` exits 0 and creates docs/teams/COL.md
- `icelines project "Elite Player"` exits 0 (uses fixture data)

**Real API tests** (opt-in, not in CI):
```bash
cargo test --features integration   # hits live NHL API
```
These are gated behind `#[cfg(feature = "integration")]` and run manually before releases.

---

## Coverage Targets by Crate

| Crate | L0 Target | L1 Target | L2 |
|-------|-----------|-----------|-----|
| `icelines-core` | **≥ 95%** line | All public functions via L1 pipeline | Via L2 binary |
| `icelines-fetch` | ≥ 70% (I/O code) | All endpoints mocked, all error paths | `--dry-run` L2 |
| `icelines-site` | ≥ 60% | Template rendering + dashboard TOML | `build --no-site` |
| `icelines-cli` | ≥ 50% (thin wrappers) | N/A (covered by L2) | All commands |

**Measuring coverage**: `cargo llvm-cov --workspace` (requires `cargo-llvm-cov` installed).
CI fails if `icelines-core` drops below 95%.

---

## CI Test Matrix

Every pull request runs:

```yaml
steps:
  - name: L0 — Unit tests
    run: cargo test -p icelines-core

  - name: L1 — Integration tests
    run: cargo test --tests

  - name: Clippy (zero warnings)
    run: cargo clippy -- -D warnings

  - name: Format check
    run: cargo fmt --check

  - name: L2 — System tests
    run: |
      cargo build --release
      cargo test --features system-tests

  - name: Coverage gate (icelines-core ≥ 95%)
    run: |
      cargo llvm-cov -p icelines-core --fail-under-lines 95
```

---

## Test Naming Convention

```
{level}_{module}_{scenario}_{expected_outcome}

l0_scoring_pace_below_min_gp_returns_none
l0_scoring_fit_at_elite_threshold_is_elite
l0_name_slafkovsky_normalizes_to_ascii
l1_pipeline_full_csv_to_depth_chart_col
l1_scheme_yahoo_standard_beniers_total_179
l2_cmd_team_col_exits_zero
l2_cmd_rank_top10_has_ten_rows
```

---

## What BENCH Checks on Every Review

From `.roles/bench.md`:

1. Does every hardcoded expected value have a calculation comment?
2. Is there a test for each classification boundary (at − ε, at, at + ε)?
3. Is there a test that would have caught today's bug if written yesterday?
4. Are API calls mocked at the HTTP boundary, not at the function boundary?
5. Are there tests for the GP=0, GP < MIN_GP, GP = MIN_GP trio?
6. Are property-based tests (`proptest`) used for scoring boundaries?
7. Does L2 cover every CLI command with at least one smoke test?
